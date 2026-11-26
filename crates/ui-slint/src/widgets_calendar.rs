use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use asrbox_mod_widgets::format::{
    event_day_url, event_matches_period, format_event_subtitle, pick_bar_color,
};
use asrbox_mod_widgets::{
    load_cached_ics, load_widget_config, parse_ics_bytes, rows_for_height, save_cached_ics,
    save_widget_config, widget_height, CalendarEvent, CalendarSource, CalendarWidgetConfig,
    ParseOptions, PeriodFilter,
};
use asrbox_platform_api::{AppearanceConfig, WidgetMenuVisibility};
use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};

use crate::generated::ui::{CalendarWindow, SlintCalEventRow};
use crate::settings_state::apply_theme_to_window;
use crate::vibrancy::{
    apply_acrylic_vibrancy, configure_desktop_window, install_bounds_tracker, resize_window_to,
    set_window_layer, set_window_visible, start_resize_se, start_window_drag,
};
use crate::win_os::{
    emit_config_changed, emit_open_settings, emit_worker_log, http_get, idx_to_layer, layer_to_idx,
    local_time_now, open_text_file, open_url,
};

thread_local! {
    static CALENDAR_SLOT: RefCell<Option<SharedCalendar>> = const { RefCell::new(None) };
}

type CalFetchOutput = (HashMap<String, Vec<CalendarEvent>>, Option<String>);

pub struct CalendarSlintState {
    pub ui: CalendarWindow,
    pub cfg_path: PathBuf,
    pub cache_dir: PathBuf,
    pub cfg: CalendarWidgetConfig,
    pub events_by_src: HashMap<String, Vec<CalendarEvent>>,
    pub error: Option<String>,
    pub loading: bool,
    pub vis: WidgetMenuVisibility,
    pub dark: bool,
    pub window_shown: bool,
    pub sync_timer: Timer,
}

pub type SharedCalendar = Rc<RefCell<CalendarSlintState>>;

pub fn create_calendar_controller(
    cfg_path: PathBuf,
    cache_dir: PathBuf,
    appearance: AppearanceConfig,
    vis: WidgetMenuVisibility,
    safe_mode: bool,
) -> Result<SharedCalendar, String> {
    let ui = CalendarWindow::new().map_err(|e| format!("create CalendarWindow: {e}"))?;
    let dark = apply_theme_to_window(&ui, appearance);
    let cfg = load_widget_config(&cfg_path).unwrap_or_default();
    let state = Rc::new(RefCell::new(CalendarSlintState {
        ui,
        cfg_path,
        cache_dir,
        cfg,
        events_by_src: HashMap::new(),
        error: None,
        loading: false,
        vis,
        dark,
        window_shown: false,
        sync_timer: Timer::default(),
    }));
    CALENDAR_SLOT.with(|slot| *slot.borrow_mut() = Some(Rc::clone(&state)));
    load_all_cached_calendars(&state);
    wire_calendar_callbacks(&state);
    apply_calendar_window_state(&state, safe_mode, true);
    Ok(state)
}

pub fn clear_calendar_slot() {
    CALENDAR_SLOT.with(|slot| *slot.borrow_mut() = None);
}

pub fn reload_calendar(
    state: &SharedCalendar,
    appearance: AppearanceConfig,
    vis: WidgetMenuVisibility,
    safe_mode: bool,
) {
    let mut s = state.borrow_mut();
    if let Ok(fresh) = load_widget_config(&s.cfg_path) {
        s.cfg = fresh;
    }
    s.vis = vis;
    let dark = apply_theme_to_window(&s.ui, appearance);
    if dark != s.dark {
        s.dark = dark;
        apply_acrylic_vibrancy(s.ui.window(), dark);
    }
    drop(s);
    load_all_cached_calendars(state);
    apply_calendar_window_state(state, safe_mode, true);
}

pub fn toggle_calendar_visible(state: &SharedCalendar) {
    let mut s = state.borrow_mut();
    s.cfg.visible = !s.cfg.visible;
    let _ = save_widget_config(&s.cfg_path, &s.cfg);
    drop(s);
    apply_calendar_window_state(state, false, false);
    emit_config_changed("widgets");
}

pub fn trigger_calendar_refresh(state: &SharedCalendar) {
    let (sources, lookahead, cache_dir) = {
        let mut s = state.borrow_mut();
        if !s.window_shown || s.cfg.sources.is_empty() {
            sync_calendar_ui(&s);
            return;
        }
        s.loading = true;
        s.ui.set_loading(true);
        (
            s.cfg.sources.clone(),
            s.cfg.lookahead_days,
            s.cache_dir.clone(),
        )
    };
    thread::spawn(move || {
        let (per_source, error) = fetch_sources_blocking(&sources, lookahead, &cache_dir);
        let _ = slint::invoke_from_event_loop(move || {
            CALENDAR_SLOT.with(|slot| {
                if let Some(rc) = slot.borrow().as_ref().cloned() {
                    let mut s = rc.borrow_mut();
                    s.loading = false;
                    s.events_by_src.extend(per_source);
                    s.error = error;
                    sync_calendar_ui(&s);
                }
            });
        });
    });
}

fn load_all_cached_calendars(state: &SharedCalendar) {
    let mut s = state.borrow_mut();
    let opts = make_parse_opts(local_time_now(), s.cfg.lookahead_days);
    s.events_by_src.clear();
    let sources = s.cfg.sources.clone();
    for src in &sources {
        if let Some(bytes) = load_cached_ics(&s.cache_dir, &src.id) {
            if let Ok(parsed) = parse_ics_bytes(&bytes, &src.id, src.color_rgb, &opts) {
                s.events_by_src.insert(src.id.clone(), parsed.events);
            }
        }
    }
}

fn apply_calendar_window_state(state: &SharedCalendar, safe_mode: bool, trigger_fetch: bool) {
    let (should_show, interval_sec) = {
        let mut s = state.borrow_mut();
        let show = !safe_mode && s.cfg.visible;
        let h = widget_height(s.cfg.visible_rows);
        if show {
            if !s.window_shown {
                let _ = s.ui.show();
                let (x, y, w, layer, dark) = (s.cfg.x, s.cfg.y, s.cfg.width, s.cfg.layer, s.dark);
                configure_desktop_window(s.ui.window(), x, y, w, h, layer, dark);
                s.window_shown = true;
            } else {
                set_window_visible(s.ui.window(), true);
                set_window_layer(s.ui.window(), s.cfg.layer);
                resize_window_to(s.ui.window(), s.cfg.width, h);
            }
            sync_calendar_ui(&s);
        } else if s.window_shown {
            set_window_visible(s.ui.window(), false);
            s.window_shown = false;
            s.sync_timer.stop();
        }
        (show, s.cfg.refresh_interval_sec)
    };
    if should_show {
        arm_calendar_timer(state, interval_sec);
        if trigger_fetch {
            trigger_calendar_refresh(state);
        }
    }
}

fn arm_calendar_timer(state: &SharedCalendar, interval_sec: u32) {
    let secs = u64::from(interval_sec.clamp(60, 86_400));
    let st_weak = Rc::downgrade(state);
    state
        .borrow()
        .sync_timer
        .start(TimerMode::Repeated, Duration::from_secs(secs), move || {
            if let Some(rc) = st_weak.upgrade() {
                trigger_calendar_refresh(&rc);
            }
        });
}

fn wire_calendar_callbacks(state: &SharedCalendar) {
    let ui = &state.borrow().ui;
    let (w1, w2) = (ui.as_weak(), ui.as_weak());
    ui.on_drag_window(move || {
        if let Some(u) = w1.upgrade() {
            start_window_drag(u.window());
        }
    });
    ui.on_start_resize(move || {
        if let Some(u) = w2.upgrade() {
            start_resize_se(u.window());
        }
    });
    ui.on_open_settings(|| emit_open_settings("calendar"));
    let st = Rc::clone(state);
    ui.on_refresh_now(move || trigger_calendar_refresh(&st));
    ui.on_open_event_url(|url| {
        if !url.is_empty() {
            let _ = open_url(url.as_str());
        }
    });
    let st = Rc::clone(state);
    ui.on_dismiss_error(move || {
        let mut s = st.borrow_mut();
        s.error = None;
        s.ui.set_error_text(SharedString::from(""));
    });
    wire_calendar_filter_and_menu(state);
    let (st_m, st_r) = (Rc::clone(state), Rc::clone(state));
    install_bounds_tracker(
        state.borrow().ui.window(),
        move |x, y| {
            let mut s = st_m.borrow_mut();
            if s.window_shown && (s.cfg.x != x || s.cfg.y != y) {
                s.cfg.x = x;
                s.cfg.y = y;
                let _ = save_widget_config(&s.cfg_path, &s.cfg);
            }
        },
        move |w, h| {
            let mut s = st_r.borrow_mut();
            let (cw, rows) = (w.clamp(280, 860), rows_for_height(h));
            if s.window_shown && (s.cfg.width != cw || s.cfg.visible_rows != rows) {
                s.cfg.width = cw;
                s.cfg.visible_rows = rows;
                let _ = save_widget_config(&s.cfg_path, &s.cfg);
            }
        },
    );
}

fn update_cal_cfg(state: &SharedCalendar, notify: bool, f: impl FnOnce(&mut CalendarSlintState)) {
    let mut s = state.borrow_mut();
    f(&mut s);
    let _ = save_widget_config(&s.cfg_path, &s.cfg);
    sync_calendar_ui(&s);
    if notify {
        emit_config_changed("widgets");
    }
}

fn wire_calendar_filter_and_menu(state: &SharedCalendar) {
    let ui = &state.borrow().ui;
    let st = Rc::clone(state);
    ui.on_select_period(move |idx| {
        update_cal_cfg(&st, true, |s| {
            s.cfg.period = match idx {
                0 => PeriodFilter::Today,
                1 => PeriodFilter::Days7,
                3 => PeriodFilter::All,
                _ => PeriodFilter::Days30,
            };
        });
    });
    let st = Rc::clone(state);
    ui.on_cycle_date_format(move || {
        update_cal_cfg(&st, false, |s| s.cfg.date_format = s.cfg.date_format.next());
    });
    let st = Rc::clone(state);
    ui.on_set_layer(move |idx| {
        update_cal_cfg(&st, true, |s| {
            s.cfg.layer = idx_to_layer(idx);
            set_window_layer(s.ui.window(), s.cfg.layer);
        });
    });
    let st = Rc::clone(state);
    ui.on_set_opacity(move |pct| {
        update_cal_cfg(&st, true, |s| {
            s.cfg.opacity_percent = pct.clamp(20, 100) as u8
        });
    });
    let st = Rc::clone(state);
    ui.on_set_sync_interval(move |sec| {
        let clamped = (sec.max(60) as u32).clamp(60, 86_400);
        update_cal_cfg(&st, true, |s| s.cfg.refresh_interval_sec = clamped);
        arm_calendar_timer(&st, clamped);
    });
    let st = Rc::clone(state);
    ui.on_open_config(move || drop(open_text_file(&st.borrow().cfg_path)));
    let st = Rc::clone(state);
    ui.on_hide_widget(move || toggle_calendar_visible(&st));
}

fn fetch_sources_blocking(
    sources: &[CalendarSource],
    lookahead_days: u16,
    cache_dir: &Path,
) -> CalFetchOutput {
    let opts = make_parse_opts(local_time_now(), lookahead_days);
    let mut per_source = HashMap::new();
    let mut error = None;
    for src in sources {
        let res = http_get(&src.url, &[]);
        if let Some(err) = res.error {
            emit_worker_log("ERROR", "http", format!("Календарь {}: {err}", src.id));
            error = Some(format!("Нет сети: {err}"));
        } else if res.status_code != 200 {
            error = Some(format!("HTTP {}", res.status_code));
        } else {
            let _ = save_cached_ics(cache_dir, &src.id, &res.body);
            match parse_ics_bytes(&res.body, &src.id, src.color_rgb, &opts) {
                Ok(p) => {
                    let msg = format!("Календарь {}: событий {}", src.id, p.events.len());
                    emit_worker_log("INFO", "http", msg);
                    per_source.insert(src.id.clone(), p.events);
                }
                Err(e) => error = Some(format!("ICS: {e}")),
            }
        }
    }
    (per_source, error)
}

fn make_parse_opts(now: asrbox_platform_api::LocalDateTime, lookahead_days: u16) -> ParseOptions {
    let today = asrbox_mod_widgets::rrule::days_from_civil(now.year, now.month, now.day);
    let (ey, em, ed) =
        asrbox_mod_widgets::rrule::civil_from_days(today + i64::from(lookahead_days.max(7)));
    ParseOptions {
        window_start: asrbox_mod_widgets::IcsDateTime::date_only(now.year, now.month, now.day),
        window_end: asrbox_mod_widgets::IcsDateTime::date_only(ey, em, ed),
        local_utc_offset_min: now.utc_offset_min,
        max_events: 400,
    }
}

fn sync_calendar_ui(s: &CalendarSlintState) {
    let ui = &s.ui;
    let now = local_time_now();
    let today = asrbox_mod_widgets::rrule::days_from_civil(now.year, now.month, now.day);
    ui.set_widget_title(SharedString::from(s.cfg.title.as_str()));
    ui.set_period_index(match s.cfg.period {
        PeriodFilter::Today => 0,
        PeriodFilter::Days7 => 1,
        PeriodFilter::Days30 => 2,
        PeriodFilter::All => 3,
    });
    ui.set_date_format_label(SharedString::from(s.cfg.date_format.label()));
    ui.set_loading(s.loading);
    ui.set_error_text(SharedString::from(s.error.as_deref().unwrap_or("")));
    ui.set_layer_index(layer_to_idx(s.cfg.layer));
    ui.set_opacity_pct(i32::from(s.cfg.opacity_percent));
    ui.set_sync_interval_sec(s.cfg.refresh_interval_sec as i32);
    ui.set_vis_refresh(s.vis.show_refresh);
    ui.set_vis_layer(s.vis.show_layer);
    ui.set_vis_opacity(s.vis.show_opacity);
    ui.set_vis_interval(s.vis.show_interval);
    ui.set_vis_open_config(s.vis.show_open_config);
    ui.set_vis_hide(s.vis.show_hide);

    let mut all: Vec<&CalendarEvent> = s
        .events_by_src
        .values()
        .flatten()
        .filter(|ev| event_matches_period(ev, now, s.cfg.period, s.cfg.lookahead_days))
        .collect();
    all.sort_by_key(|ev| ev.start.to_epoch_min());
    let rows: Vec<SlintCalEventRow> = all
        .into_iter()
        .take(40)
        .map(|ev| {
            let rgb = pick_bar_color(ev, now);
            SlintCalEventRow {
                summary: SharedString::from(ev.summary.as_str()),
                subtitle: SharedString::from(format_event_subtitle(ev, now, s.cfg.date_format)),
                accent_r: i32::from(rgb[0]),
                accent_g: i32::from(rgb[1]),
                accent_b: i32::from(rgb[2]),
                url: SharedString::from(event_day_url(ev)),
                is_today: ev.start.days_since_epoch() <= today,
            }
        })
        .collect();
    let empty = if s.cfg.sources.is_empty() {
        "Добавьте .ics календарь через ⚙ Настройки"
    } else {
        "Нет событий за выбранный период"
    };
    ui.set_empty_text(SharedString::from(empty));
    ui.set_events(ModelRc::from(Rc::new(VecModel::from(rows))));
}
