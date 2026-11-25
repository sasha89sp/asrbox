#![cfg(windows)]

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use asrbox_core::{SupervisorCmd, WorkerMsg};
use asrbox_mod_widgets::{
    CalendarWidget, NotesWidget, SysMonitorWidget, WeatherWidget, CALENDAR_WIDGET_ID,
    SYS_WIDGET_ID, WEATHER_WIDGET_ID,
};
use asrbox_platform_api::{
    load_appearance_config, load_menu_config, DockPlatform, PlatformEvent, WidgetPlatform,
};
use asrbox_platform_windows::{is_process_alive, WindowsDockHost};

use crate::tools::resolve_app_paths;
use crate::worker::{drain_cmds, emit_log, emit_msg, WorkerStats};

const WEATHER_REQ_BASE: u64 = 100_000;

struct WorkerWidgets {
    cal: CalendarWidget,
    sys: SysMonitorWidget,
    weather: WeatherWidget,
    notes: NotesWidget,
}

impl WorkerWidgets {
    fn load(
        host: &mut WindowsDockHost,
        safe_mode: bool,
    ) -> Result<(Self, std::path::PathBuf, std::path::PathBuf), String> {
        let paths = resolve_app_paths();
        let pal = load_appearance_config(&paths.appearance_cfg).palette();
        let vis = load_menu_config(&paths.menus_cfg).widget;
        WidgetPlatform::set_theme(host, pal);

        let mut cal = CalendarWidget::load(paths.widgets_cfg, paths.cache_dir);
        let mut sys = SysMonitorWidget::load(paths.sysmon_cfg);
        let mut weather = WeatherWidget::load(paths.weather_cfg);
        let mut notes = NotesWidget::load(paths.notes_cfg, paths.notes_dir.join("notes_db.cfg"));

        cal.set_menu_visibility(vis);
        sys.set_menu_visibility(vis);
        weather.set_menu_visibility(vis);
        notes.set_menu_visibility(vis);
        cal.set_palette(host, pal);
        sys.set_palette(host, pal);
        weather.set_palette(host, pal);
        notes.set_palette(host, pal);

        cal.sync_window(host, safe_mode)?;
        sys.sync_window(host, safe_mode)?;
        weather.sync_window(host, safe_mode)?;
        notes.sync_window(host, safe_mode)?;
        Ok((
            Self {
                cal,
                sys,
                weather,
                notes,
            },
            paths.appearance_cfg,
            paths.menus_cfg,
        ))
    }

    fn summary(&self) -> String {
        format!(
            "Календарь [{}], Монитор [{}], Погода [{}], Заметки [{}]",
            self.cal.status_summary(),
            self.sys.status_summary(),
            self.weather.status_summary(),
            self.notes.status_summary(),
        )
    }
}

pub(crate) fn run_widgets_worker(
    host: &mut WindowsDockHost,
    cmds: Arc<Mutex<VecDeque<SupervisorCmd>>>,
    parent_pid: u32,
    mut safe_mode: bool,
) -> Result<(), String> {
    let (mut widgets, appearance_cfg, menus_cfg) = WorkerWidgets::load(host, safe_mode)?;
    emit_log(
        "INFO",
        "widgets",
        format!("Виджеты (worker): {}", widgets.summary()),
    );
    let mut stats = WorkerStats::new("widgets");

    host.run_event_loop(|platform, ev| {
        if let PlatformEvent::SupervisorTick = ev {
            if parent_pid > 0 && !is_process_alive(parent_pid) {
                platform.request_quit();
                return;
            }
            for cmd in drain_cmds(&cmds) {
                handle_supervisor_cmd(
                    platform,
                    &mut widgets,
                    &mut safe_mode,
                    &appearance_cfg,
                    &menus_cfg,
                    cmd,
                );
            }
            stats.maybe_emit(platform);
            return;
        }
        handle_widget_event(platform, &mut widgets, &mut stats, ev);
    });
    Ok(())
}

fn handle_supervisor_cmd(
    platform: &mut WindowsDockHost,
    w: &mut WorkerWidgets,
    safe_mode: &mut bool,
    appearance_cfg: &Path,
    menus_cfg: &Path,
    cmd: SupervisorCmd,
) {
    match cmd {
        SupervisorCmd::ReloadConfig => {
            let pal = load_appearance_config(appearance_cfg).palette();
            let vis = load_menu_config(menus_cfg).widget;
            WidgetPlatform::set_theme(platform, pal);
            w.cal.set_menu_visibility(vis);
            w.sys.set_menu_visibility(vis);
            w.weather.set_menu_visibility(vis);
            w.notes.set_menu_visibility(vis);
            w.cal.set_palette(platform, pal);
            w.sys.set_palette(platform, pal);
            w.weather.set_palette(platform, pal);
            w.notes.set_palette(platform, pal);
            let _ = w.cal.reload_config_and_refresh(platform);
            let _ = w.cal.sync_window(platform, *safe_mode);
            let _ = w.sys.reload_config(platform, *safe_mode);
            let _ = w.weather.reload_config(platform, *safe_mode);
            let _ = w.notes.reload_config(platform, *safe_mode);
            emit_log(
                "INFO",
                "widgets",
                format!("Перечитаны конфиги виджетов: {}", w.summary()),
            );
        }
        SupervisorCmd::ToggleCalendar => {
            *safe_mode = false;
            w.cal.toggle_visible(platform, false);
            emit_changed();
        }
        SupervisorCmd::RefreshCalendar => {
            w.cal.on_refresh_tick(platform);
            emit_log("INFO", "http", "Ручное обновление .ics".to_owned());
        }
        SupervisorCmd::ToggleSysMonitor => {
            *safe_mode = false;
            w.sys.toggle_visible(platform, false);
            emit_changed();
        }
        SupervisorCmd::RefreshSysMonitor => w.sys.on_refresh_tick(platform),
        SupervisorCmd::ToggleWeather => {
            *safe_mode = false;
            w.weather.toggle_visible(platform, false);
            emit_changed();
        }
        SupervisorCmd::RefreshWeather => {
            w.weather.on_refresh_tick(platform);
            emit_log(
                "INFO",
                "http",
                "Ручное обновление прогноза погоды".to_owned(),
            );
        }
        SupervisorCmd::ToggleNotes => {
            *safe_mode = false;
            w.notes.toggle_visible(platform, false);
            emit_changed();
        }
        SupervisorCmd::Shutdown => platform.request_quit(),
        _ => {}
    }
}

fn handle_widget_event(
    host: &mut WindowsDockHost,
    w: &mut WorkerWidgets,
    stats: &mut WorkerStats,
    ev: PlatformEvent,
) {
    let t0 = Instant::now();
    stats.events += 1;
    match ev {
        PlatformEvent::WidgetPointerMove { widget_id, point } => match widget_id {
            CALENDAR_WIDGET_ID => w.cal.on_pointer_move(host, point),
            SYS_WIDGET_ID => w.sys.on_pointer_move(host, point),
            WEATHER_WIDGET_ID => w.weather.on_pointer_move(host, point),
            _ => {}
        },
        PlatformEvent::WidgetPointerLeave { widget_id } => match widget_id {
            CALENDAR_WIDGET_ID => w.cal.on_pointer_leave(host),
            SYS_WIDGET_ID => w.sys.on_pointer_leave(host),
            WEATHER_WIDGET_ID => w.weather.on_pointer_leave(host),
            _ => {}
        },
        PlatformEvent::WidgetLeftClick { widget_id, point } => {
            let msg = match widget_id {
                CALENDAR_WIDGET_ID => w.cal.on_left_click(host, point),
                SYS_WIDGET_ID => w.sys.on_left_click(host, point),
                WEATHER_WIDGET_ID => w.weather.on_left_click(host, point),
                _ => String::new(),
            };
            route_widget_msg(msg);
            stats.record_frame(t0);
        }
        PlatformEvent::WidgetRightClick {
            widget_id,
            screen_point,
        } => {
            let msg = match widget_id {
                CALENDAR_WIDGET_ID => w.cal.on_right_click(host, screen_point),
                SYS_WIDGET_ID => w.sys.on_right_click(host, screen_point),
                WEATHER_WIDGET_ID => w.weather.on_right_click(host, screen_point),
                _ => String::new(),
            };
            route_widget_msg(msg);
        }
        PlatformEvent::WidgetWheel { widget_id, delta } => {
            if widget_id == CALENDAR_WIDGET_ID {
                w.cal.on_wheel(host, delta);
                stats.record_frame(t0);
            }
        }
        PlatformEvent::WidgetMoved { widget_id, x, y } => match widget_id {
            CALENDAR_WIDGET_ID => w.cal.on_moved(x, y),
            SYS_WIDGET_ID => w.sys.on_moved(x, y),
            WEATHER_WIDGET_ID => w.weather.on_moved(x, y),
            _ => {}
        },
        PlatformEvent::WidgetResized {
            widget_id,
            x,
            y,
            width,
            height,
            finished,
        } => {
            match widget_id {
                CALENDAR_WIDGET_ID => w.cal.on_resized(host, x, y, width, height, finished),
                SYS_WIDGET_ID => w.sys.on_resized(host, x, y, width, height, finished),
                WEATHER_WIDGET_ID => w.weather.on_resized(host, x, y, width, height, finished),
                _ => {}
            }
            if finished {
                emit_changed();
            }
            stats.record_frame(t0);
        }
        PlatformEvent::WidgetMinuteTick { widget_id } => {
            if widget_id == CALENDAR_WIDGET_ID {
                w.cal.on_minute_tick(host);
            }
        }
        PlatformEvent::WidgetRefreshTick { widget_id } => match widget_id {
            CALENDAR_WIDGET_ID => w.cal.on_refresh_tick(host),
            SYS_WIDGET_ID => w.sys.on_refresh_tick(host),
            WEATHER_WIDGET_ID => w.weather.on_refresh_tick(host),
            _ => {}
        },
        PlatformEvent::SysTelemetryReady { snapshot } => {
            w.sys.on_telemetry_ready(host, *snapshot);
            stats.record_frame(t0);
        }
        PlatformEvent::HttpFetchDone { request_id, result } => {
            let outcome = if request_id >= WEATHER_REQ_BASE {
                w.weather.on_http_done(host, request_id, result)
            } else {
                w.cal.on_http_done(host, request_id, result)
            };
            if let Some((is_err, msg)) = outcome {
                emit_log(if is_err { "ERROR" } else { "INFO" }, "http", msg);
            }
            stats.record_frame(t0);
        }
        PlatformEvent::NotesEvent(nev) => {
            let msg = w.notes.handle_event(host, nev);
            route_widget_msg(msg);
            stats.record_frame(t0);
        }
        _ => {}
    }
}

fn emit_changed() {
    emit_msg(&WorkerMsg::ConfigChanged {
        module: "widgets".to_owned(),
    });
}

fn route_widget_msg(msg: String) {
    let tab = match msg.as_str() {
        "open_settings" => Some("calendar"),
        "open_settings_sysmon" => Some("sysmon"),
        "open_settings_weather" => Some("weather"),
        "open_settings_notes" => Some("notes"),
        _ => None,
    };
    if let Some(tab) = tab {
        emit_msg(&WorkerMsg::OpenSettings {
            tab: tab.to_owned(),
        });
    } else if !msg.is_empty() {
        let lvl = if msg.starts_with("error:") {
            "ERROR"
        } else {
            "INFO"
        };
        emit_log(lvl, "widgets", msg);
        emit_changed();
    }
}
