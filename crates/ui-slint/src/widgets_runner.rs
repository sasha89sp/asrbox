use std::cell::RefCell;
use std::io::{self, BufRead};
use std::thread;
use std::time::Duration;

use asrbox_core::{decode_supervisor_cmd, SupervisorCmd, WorkerMsg};
use asrbox_mod_about::AppPaths;
use asrbox_platform_api::{load_appearance_config, load_menu_config};
use slint::{Timer, TimerMode};

use crate::vibrancy::select_winit_backend;
use crate::widgets_calendar::{
    clear_calendar_slot, create_calendar_controller, reload_calendar, toggle_calendar_visible,
    trigger_calendar_refresh, SharedCalendar,
};
use crate::widgets_sysmon::{
    clear_sysmon_slot, create_sysmon_controller, kick_sysmon_sample, reload_sysmon,
    stop_sysmon_sampler, toggle_sysmon_visible, SharedSysMon,
};
use crate::widgets_weather::{
    clear_weather_slot, create_weather_controller, reload_weather, toggle_weather_visible,
    trigger_weather_refresh, SharedWeather,
};
use crate::win_os::{emit_worker_log, emit_worker_msg, is_parent_alive};

struct WidgetsHostCtx {
    paths: AppPaths,
    safe_mode: bool,
    weather: SharedWeather,
    sysmon: SharedSysMon,
    calendar: SharedCalendar,
}

thread_local! {
    static HOST_CTX: RefCell<Option<WidgetsHostCtx>> = const { RefCell::new(None) };
}

pub fn run_widgets_host(
    paths: AppPaths,
    _app_version: &str,
    parent_pid: u32,
    safe_mode: bool,
) -> Result<(), String> {
    select_winit_backend()?;
    emit_worker_msg(&WorkerMsg::Ready {
        module: "widgets".to_owned(),
        pid: std::process::id(),
    });
    let appearance = load_appearance_config(&paths.appearance_cfg);
    let vis = load_menu_config(&paths.menus_cfg).widget;

    let weather = create_weather_controller(paths.weather_cfg.clone(), appearance, vis, safe_mode)?;
    let sysmon = create_sysmon_controller(paths.sysmon_cfg.clone(), appearance, vis, safe_mode)?;
    let calendar = create_calendar_controller(
        paths.widgets_cfg.clone(),
        paths.cache_dir.clone(),
        appearance,
        vis,
        safe_mode,
    )?;

    HOST_CTX.with(|slot| {
        *slot.borrow_mut() = Some(WidgetsHostCtx {
            paths,
            safe_mode,
            weather,
            sysmon,
            calendar,
        });
    });
    emit_worker_log("INFO", "widgets", "Slint-воркер виджетов запущен");

    spawn_stdin_listener();
    let watchdog = Timer::default();
    if parent_pid > 0 {
        watchdog.start(TimerMode::Repeated, Duration::from_secs(2), move || {
            if !is_parent_alive(parent_pid) {
                shutdown_widgets_host();
            }
        });
    }

    let res = slint::run_event_loop_until_quit().map_err(|e| format!("widgets event loop: {e}"));
    cleanup_widgets_host();
    res
}

fn spawn_stdin_listener() {
    thread::spawn(|| {
        let stdin = io::stdin();
        for line in stdin.lock().lines().map_while(Result::ok) {
            if let Some(cmd) = decode_supervisor_cmd(&line) {
                let _ = slint::invoke_from_event_loop(move || handle_supervisor_cmd(cmd));
            }
        }
        let _ = slint::invoke_from_event_loop(shutdown_widgets_host);
    });
}

fn handle_supervisor_cmd(cmd: SupervisorCmd) {
    HOST_CTX.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(ctx) = guard.as_mut() else {
            return;
        };
        match cmd {
            SupervisorCmd::ReloadConfig => {
                let appearance = load_appearance_config(&ctx.paths.appearance_cfg);
                let vis = load_menu_config(&ctx.paths.menus_cfg).widget;
                reload_weather(&ctx.weather, appearance, vis, ctx.safe_mode);
                reload_sysmon(&ctx.sysmon, appearance, vis, ctx.safe_mode);
                reload_calendar(&ctx.calendar, appearance, vis, ctx.safe_mode);
            }
            SupervisorCmd::ToggleWeather => {
                ctx.safe_mode = false;
                toggle_weather_visible(&ctx.weather);
            }
            SupervisorCmd::RefreshWeather => trigger_weather_refresh(&ctx.weather, true),
            SupervisorCmd::ToggleSysMonitor => {
                ctx.safe_mode = false;
                toggle_sysmon_visible(&ctx.sysmon);
            }
            SupervisorCmd::RefreshSysMonitor => kick_sysmon_sample(&ctx.sysmon),
            SupervisorCmd::ToggleCalendar => {
                ctx.safe_mode = false;
                toggle_calendar_visible(&ctx.calendar);
            }
            SupervisorCmd::RefreshCalendar => trigger_calendar_refresh(&ctx.calendar),
            SupervisorCmd::Shutdown => {
                drop(guard);
                shutdown_widgets_host();
            }
            _ => {}
        }
    });
}

fn shutdown_widgets_host() {
    cleanup_widgets_host();
    let _ = slint::quit_event_loop();
}

fn cleanup_widgets_host() {
    HOST_CTX.with(|slot| {
        if let Some(ctx) = slot.borrow_mut().take() {
            stop_sysmon_sampler(&ctx.sysmon);
        }
    });
    clear_weather_slot();
    clear_sysmon_slot();
    clear_calendar_slot();
}
