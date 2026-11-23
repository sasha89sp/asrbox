#![cfg(windows)]

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use asrbox_core::{SupervisorCmd, WorkerMsg};
use asrbox_mod_widgets::{CalendarWidget, SysMonitorWidget, CALENDAR_WIDGET_ID, SYS_WIDGET_ID};
use asrbox_platform_api::{DockPlatform, PlatformEvent};
use asrbox_platform_windows::{is_process_alive, WindowsDockHost};

use crate::tools::resolve_app_paths;
use crate::worker::{drain_cmds, emit_log, emit_msg, WorkerStats};

pub(crate) fn run_widgets_worker(
    host: &mut WindowsDockHost,
    cmds: Arc<Mutex<VecDeque<SupervisorCmd>>>,
    parent_pid: u32,
    mut safe_mode: bool,
) -> Result<(), String> {
    let paths = resolve_app_paths();
    let mut cal = CalendarWidget::load(paths.widgets_cfg, paths.cache_dir);
    let mut sys = SysMonitorWidget::load(paths.sysmon_cfg);
    cal.sync_window(host, safe_mode)?;
    sys.sync_window(host, safe_mode)?;
    emit_log(
        "INFO",
        "widgets",
        format!(
            "Виджеты (worker): Календарь [{}], Монитор [{}]",
            cal.status_summary(),
            sys.status_summary()
        ),
    );
    let mut stats = WorkerStats::new("widgets");

    host.run_event_loop(|platform, ev| {
        if let PlatformEvent::SupervisorTick = ev {
            if parent_pid > 0 && !is_process_alive(parent_pid) {
                platform.request_quit();
                return;
            }
            for cmd in drain_cmds(&cmds) {
                handle_supervisor_cmd(platform, &mut cal, &mut sys, &mut safe_mode, cmd);
            }
            stats.maybe_emit(platform);
            return;
        }
        handle_widget_event(platform, &mut cal, &mut sys, &mut stats, ev);
    });
    Ok(())
}

fn handle_supervisor_cmd(
    platform: &mut WindowsDockHost,
    cal: &mut CalendarWidget,
    sys: &mut SysMonitorWidget,
    safe_mode: &mut bool,
    cmd: SupervisorCmd,
) {
    match cmd {
        SupervisorCmd::ReloadConfig => {
            let _ = cal.reload_config_and_refresh(platform);
            let _ = cal.sync_window(platform, *safe_mode);
            let _ = sys.reload_config(platform, *safe_mode);
            emit_log(
                "INFO",
                "widgets",
                format!(
                    "Перечитаны конфиги виджетов: Календарь [{}], Монитор [{}]",
                    cal.status_summary(),
                    sys.status_summary()
                ),
            );
        }
        SupervisorCmd::ToggleCalendar => {
            *safe_mode = false;
            cal.toggle_visible(platform, false);
            emit_msg(&WorkerMsg::ConfigChanged {
                module: "widgets".to_owned(),
            });
        }
        SupervisorCmd::RefreshCalendar => {
            cal.on_refresh_tick(platform);
            emit_log("INFO", "http", "Ручное обновление .ics".to_owned());
        }
        SupervisorCmd::ToggleSysMonitor => {
            *safe_mode = false;
            sys.toggle_visible(platform, false);
            emit_msg(&WorkerMsg::ConfigChanged {
                module: "widgets".to_owned(),
            });
        }
        SupervisorCmd::RefreshSysMonitor => {
            sys.on_refresh_tick(platform);
        }
        SupervisorCmd::Shutdown => platform.request_quit(),
        _ => {}
    }
}

fn handle_widget_event(
    host: &mut WindowsDockHost,
    cal: &mut CalendarWidget,
    sys: &mut SysMonitorWidget,
    stats: &mut WorkerStats,
    ev: PlatformEvent,
) {
    let t0 = Instant::now();
    stats.events += 1;
    match ev {
        PlatformEvent::WidgetPointerMove { widget_id, point } => match widget_id {
            CALENDAR_WIDGET_ID => cal.on_pointer_move(host, point),
            SYS_WIDGET_ID => sys.on_pointer_move(host, point),
            _ => {}
        },
        PlatformEvent::WidgetPointerLeave { widget_id } => match widget_id {
            CALENDAR_WIDGET_ID => cal.on_pointer_leave(host),
            SYS_WIDGET_ID => sys.on_pointer_leave(host),
            _ => {}
        },
        PlatformEvent::WidgetLeftClick { widget_id, point } => {
            let msg = match widget_id {
                CALENDAR_WIDGET_ID => cal.on_left_click(host, point),
                SYS_WIDGET_ID => sys.on_left_click(host, point),
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
                CALENDAR_WIDGET_ID => cal.on_right_click(host, screen_point),
                SYS_WIDGET_ID => sys.on_right_click(host, screen_point),
                _ => String::new(),
            };
            route_widget_msg(msg);
        }
        PlatformEvent::WidgetWheel { widget_id, delta } => {
            if widget_id == CALENDAR_WIDGET_ID {
                cal.on_wheel(host, delta);
                stats.record_frame(t0);
            }
        }
        PlatformEvent::WidgetMoved { widget_id, x, y } => match widget_id {
            CALENDAR_WIDGET_ID => cal.on_moved(x, y),
            SYS_WIDGET_ID => sys.on_moved(x, y),
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
                CALENDAR_WIDGET_ID => cal.on_resized(host, x, y, width, height, finished),
                SYS_WIDGET_ID => sys.on_resized(host, x, y, width, height, finished),
                _ => {}
            }
            if finished {
                emit_msg(&WorkerMsg::ConfigChanged {
                    module: "widgets".to_owned(),
                });
            }
            stats.record_frame(t0);
        }
        PlatformEvent::WidgetMinuteTick { widget_id } => {
            if widget_id == CALENDAR_WIDGET_ID {
                cal.on_minute_tick(host);
            }
        }
        PlatformEvent::WidgetRefreshTick { widget_id } => match widget_id {
            CALENDAR_WIDGET_ID => cal.on_refresh_tick(host),
            SYS_WIDGET_ID => sys.on_refresh_tick(host),
            _ => {}
        },
        PlatformEvent::SysTelemetryReady { snapshot } => {
            sys.on_telemetry_ready(host, *snapshot);
            stats.record_frame(t0);
        }
        PlatformEvent::HttpFetchDone { request_id, result } => {
            if let Some((is_err, msg)) = cal.on_http_done(host, request_id, result) {
                emit_log(if is_err { "ERROR" } else { "INFO" }, "http", msg);
            }
            stats.record_frame(t0);
        }
        _ => {}
    }
}

fn route_widget_msg(msg: String) {
    if msg == "open_settings" {
        emit_msg(&WorkerMsg::OpenSettings {
            tab: "calendar".to_owned(),
        });
    } else if msg == "open_settings_sysmon" {
        emit_msg(&WorkerMsg::OpenSettings {
            tab: "sysmon".to_owned(),
        });
    } else if !msg.is_empty() {
        let lvl = if msg.starts_with("error:") {
            "ERROR"
        } else {
            "INFO"
        };
        emit_log(lvl, "widgets", msg);
        emit_msg(&WorkerMsg::ConfigChanged {
            module: "widgets".to_owned(),
        });
    }
}
