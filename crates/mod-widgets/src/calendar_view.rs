use std::collections::HashMap;
use std::path::Path;

use asrbox_platform_api::{HttpFetchResult, LocalDateTime};

use crate::format::{
    event_matches_period, format_event_subtitle, format_event_tooltip, local_now_as_ics,
    pick_bar_color,
};
use crate::ics::{parse_ics_bytes, CalendarEvent, ParseOptions};
use crate::render::{render_calendar_widget, DisplayRow, RenderView, WidgetHitTarget};
use crate::storage::{
    load_cached_ics, parse_calendar_source, parse_widget_config, save_widget_config,
    CalendarWidgetConfig,
};

pub fn format_status_summary(
    cfg: &CalendarWidgetConfig,
    events: usize,
    pending: usize,
    err: Option<&str>,
) -> String {
    format!(
        "visible={}, sources={}, events={}, pending={}, opacity={}%, err={}",
        cfg.visible,
        cfg.sources.len(),
        events,
        pending,
        cfg.opacity_percent,
        err.unwrap_or("none")
    )
}

pub struct CalendarState {
    pub all_events: Vec<CalendarEvent>,
    pub cached_by_source: HashMap<String, Vec<u8>>,
    pub pending_requests: HashMap<u64, String>,
    pub next_request_id: u64,
    pub scroll_offset: usize,
    pub shown_row_count: usize,
    pub hovered: WidgetHitTarget,
    pub last_tooltip: Option<String>,
    pub last_updated: Option<String>,
    pub status_error: Option<String>,
    pub busy_warning: bool,
}

impl Default for CalendarState {
    fn default() -> Self {
        Self {
            all_events: Vec::new(),
            cached_by_source: HashMap::new(),
            pending_requests: HashMap::new(),
            next_request_id: 1,
            scroll_offset: 0,
            shown_row_count: 0,
            hovered: WidgetHitTarget::None,
            last_tooltip: None,
            last_updated: None,
            status_error: None,
            busy_warning: false,
        }
    }
}

impl CalendarState {
    pub fn reparse(&mut self, cfg: &CalendarWidgetConfig, now: LocalDateTime) {
        let (merged, any_busy) = parse_all_sources(cfg, &self.cached_by_source, now);
        self.all_events = merged;
        self.busy_warning = any_busy;
    }

    pub fn load_caches(&mut self, cfg: &CalendarWidgetConfig, cache_dir: &Path) {
        for src in &cfg.sources {
            if let Some(bytes) = load_cached_ics(cache_dir, &src.id) {
                self.cached_by_source.insert(src.id.clone(), bytes);
            }
        }
    }
}

pub fn build_calendar_frame(
    cfg: &CalendarWidgetConfig,
    now: LocalDateTime,
    st: &CalendarState,
) -> (asrbox_platform_api::WidgetFrame, usize, usize) {
    let total = filter_events(&st.all_events, cfg, now).len();
    let max_rows = usize::from(cfg.visible_rows);
    let scroll_offset = st.scroll_offset.min(total.saturating_sub(max_rows));
    let rows = build_display_rows(&st.all_events, cfg, now, scroll_offset);
    let is_loading = !st.pending_requests.is_empty();
    let effective_err = st.status_error.as_deref().or(cfg
        .commented_calendar_hint
        .then_some("Уберите # перед calendar= или задайте URL в Настройках"));
    let (footer_text, footer_error) = build_footer_status(
        effective_err,
        st.busy_warning,
        !cfg.sources.is_empty(),
        is_loading,
        st.last_updated.as_deref(),
        scroll_offset,
        total,
        rows.len(),
    );
    let view = RenderView {
        width: cfg.width,
        visible_rows: cfg.visible_rows,
        opacity_percent: cfg.opacity_percent,
        title: &cfg.title,
        period: cfg.period,
        date_format: cfg.date_format,
        rows: &rows,
        hovered: st.hovered,
        footer_text: &footer_text,
        footer_error,
        can_scroll_up: scroll_offset > 0,
        can_scroll_down: scroll_offset + max_rows < total,
        is_loading,
    };
    (render_calendar_widget(&view), scroll_offset, rows.len())
}

pub enum HttpEval {
    Ok(Vec<u8>),
    Err { status_err: String, log_msg: String },
}

pub fn evaluate_http_response(source_id: &str, result: HttpFetchResult) -> HttpEval {
    let code = result.status_code;
    let bytes_len = result.body.len();
    if let Some(err) = result.error {
        return HttpEval::Err {
            status_err: format!("Нет сети: {err}"),
            log_msg: format!("http[{source_id}] error: {err}"),
        };
    }
    if code == 404 || code == 403 || code == 401 {
        return HttpEval::Err {
            status_err: format!("HTTP {code}: проверьте ссылку .ics"),
            log_msg: format!("http[{source_id}] status={code}"),
        };
    }
    if code != 200 {
        return HttpEval::Err {
            status_err: format!("Ошибка HTTP {code}"),
            log_msg: format!("http[{source_id}] status={code}"),
        };
    }
    if !contains_vcalendar(&result.body) {
        return HttpEval::Err {
            status_err: "Ответ не является .ics файлом".to_owned(),
            log_msg: format!("http[{source_id}] invalid ics ({bytes_len} B)"),
        };
    }
    HttpEval::Ok(result.body)
}

pub fn clamp_scroll_offset(current: usize, delta: isize, total: usize, visible_rows: u8) -> usize {
    let max_off = total.saturating_sub(usize::from(visible_rows));
    if delta < 0 {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current.saturating_add(delta as usize).min(max_off)
    }
}

pub fn parse_all_sources(
    cfg: &CalendarWidgetConfig,
    cached_by_source: &HashMap<String, Vec<u8>>,
    now: LocalDateTime,
) -> (Vec<CalendarEvent>, bool) {
    let now_ics = local_now_as_ics(now);
    let opts = ParseOptions {
        window_start_min: now_ics.to_epoch_min() - 1440,
        window_end_min: now_ics.to_epoch_min() + i64::from(cfg.lookahead_days) * 1440,
        local_utc_offset_min: now.utc_offset_min,
    };
    let mut merged = Vec::new();
    let mut any_busy_only = false;
    for src in &cfg.sources {
        if let Some(bytes) = cached_by_source.get(&src.id) {
            let parsed = parse_ics_bytes(bytes, &src.id, src.color_rgb, opts);
            if parsed.busy_only_warning {
                any_busy_only = true;
            }
            merged.extend(parsed.events);
        }
    }
    merged.sort_by_key(|e| (e.start.to_epoch_min(), !e.all_day, e.summary.clone()));
    (merged, any_busy_only)
}

pub fn filter_events<'a>(
    all: &'a [CalendarEvent],
    cfg: &CalendarWidgetConfig,
    now: LocalDateTime,
) -> Vec<&'a CalendarEvent> {
    all.iter()
        .filter(|e| event_matches_period(e, now, cfg.period, cfg.lookahead_days))
        .collect()
}

pub fn build_display_rows(
    all: &[CalendarEvent],
    cfg: &CalendarWidgetConfig,
    now: LocalDateTime,
    scroll_offset: usize,
) -> Vec<DisplayRow> {
    if cfg.sources.is_empty() {
        return setup_hint_rows();
    }
    let filtered = filter_events(all, cfg, now);
    let max_rows = usize::from(cfg.visible_rows);
    let off = scroll_offset.min(filtered.len().saturating_sub(max_rows));
    filtered
        .iter()
        .skip(off)
        .take(max_rows)
        .map(|ev| DisplayRow {
            title: ev.summary.clone(),
            subtitle: format_event_subtitle(ev, now, cfg.date_format),
            bar_rgb: pick_bar_color(ev, now),
            tooltip: format_event_tooltip(ev, now),
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
pub fn build_footer_status(
    status_error: Option<&str>,
    busy_warning: bool,
    has_sources: bool,
    is_loading: bool,
    last_updated: Option<&str>,
    scroll_offset: usize,
    total: usize,
    shown: usize,
) -> (String, bool) {
    if let Some(err) = status_error {
        return (err.to_owned(), true);
    }
    if is_loading {
        return ("Загрузка календаря...".to_owned(), false);
    }
    if busy_warning {
        return ("Только Busy: откройте доступ к названиям".to_owned(), true);
    }
    if !has_sources {
        return ("Нет подключённых календарей".to_owned(), false);
    }
    let upd = last_updated.unwrap_or("кэш");
    if total == 0 {
        (format!("Обн. {upd} · нет событий"), false)
    } else {
        let from = scroll_offset + 1;
        let to = (scroll_offset + shown).min(total);
        (format!("Обн. {upd} · {from}–{to} из {total}"), false)
    }
}

pub fn tooltip_for_target(target: WidgetHitTarget, rows: &[DisplayRow]) -> Option<String> {
    match target {
        WidgetHitTarget::EventRow(idx) => rows.get(idx).map(|r| r.tooltip.clone()),
        WidgetHitTarget::AddEventBtn => Some("Создать новое событие".to_owned()),
        WidgetHitTarget::SettingsBtn => Some("Открыть настройки виджета".to_owned()),
        WidgetHitTarget::RefreshBtn => Some("Обновить календарь сейчас".to_owned()),
        WidgetHitTarget::FormatCycleBtn => Some("Переключить формат времени".to_owned()),
        _ => None,
    }
}

pub fn contains_vcalendar(bytes: &[u8]) -> bool {
    let head = &bytes[..bytes.len().min(2048)];
    String::from_utf8_lossy(head).contains("BEGIN:VCALENDAR")
}

fn setup_hint_rows() -> Vec<DisplayRow> {
    vec![
        DisplayRow {
            title: "Нажмите ⚙ для настройки .ics".to_owned(),
            subtitle: "Вставьте секретную ссылку Google Календаря".to_owned(),
            bar_rgb: [78, 161, 255],
            tooltip: "Откройте widgets.cfg и добавьте строку calendar=main|auto|URL".to_owned(),
        },
        DisplayRow {
            title: "После сохранения нажмите ↻".to_owned(),
            subtitle: "Кэш сохранится локально в data/cache".to_owned(),
            bar_rgb: [52, 211, 153],
            tooltip: "Расписание подгрузится в фоне без блокировки UI".to_owned(),
        },
    ]
}

pub fn uncomment_calendar_sources(path: &Path) -> Result<usize, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| format!("read widgets.cfg: {e}"))?;
    let mut cfg = parse_widget_config(&raw);
    let mut added = 0usize;
    for line in raw.trim_start_matches('\u{FEFF}').lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') && !trimmed.starts_with(';') {
            continue;
        }
        let clean = trimmed.trim_start_matches(['#', ';', ' ', '\t']);
        let Some((k, v)) = clean.split_once('=') else {
            continue;
        };
        let key = k.trim();
        if (key == "calendar" || key == "url") && !v.contains("...") {
            let idx = cfg.sources.len().saturating_add(1);
            if let Some(src) = parse_calendar_source(v.trim(), idx) {
                if !cfg.sources.iter().any(|s| s.url == src.url) {
                    cfg.sources.push(src);
                    added += 1;
                }
            }
        }
    }
    if added > 0 {
        cfg.commented_calendar_hint = false;
        save_widget_config(path, &cfg)?;
    }
    Ok(added)
}
