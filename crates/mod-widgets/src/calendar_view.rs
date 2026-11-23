use std::collections::HashMap;

use asrbox_platform_api::LocalDateTime;

use crate::format::{
    event_matches_period, format_event_subtitle, format_event_tooltip, local_now_as_ics,
    pick_bar_color,
};
use crate::ics::{parse_ics_bytes, CalendarEvent, ParseOptions};
use crate::render::{DisplayRow, WidgetHitTarget};
use crate::storage::CalendarWidgetConfig;

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

pub fn build_footer_status(
    status_error: Option<&str>,
    busy_warning: bool,
    has_sources: bool,
    last_updated: Option<&str>,
    scroll_offset: usize,
    total: usize,
    shown: usize,
) -> (String, bool) {
    if let Some(err) = status_error {
        return (err.to_owned(), true);
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
