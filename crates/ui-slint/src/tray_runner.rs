use std::cell::RefCell;
use std::io::{self, BufRead, Write};
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};

use asrbox_mod_about::AppPaths;
use asrbox_platform_api::load_appearance_config;
use slint::{ComponentHandle, ModelRc, SharedString, Timer, TimerMode, VecModel};

use crate::generated::ui::{SlintTrayDockRow, TrayMenuWindow};
use crate::ipc_proto::{decode_tray_show, encode_ui_cmd, TrayShowPayload};
use crate::settings_state::apply_theme_to_window;
use crate::vibrancy::{
    configure_popup_window, install_focus_lost_hook, select_winit_backend, set_window_visible,
};
use crate::win_os::{is_parent_alive, primary_work_area};

const TRAY_MENU_W: u32 = 284;
const TRAY_MENU_H: u32 = 410;

struct TrayHostState {
    ui: TrayMenuWindow,
    paths: AppPaths,
    visible: bool,
    last_shown_at: Instant,
}

thread_local! {
    static TRAY_HOST: RefCell<Option<TrayHostState>> = const { RefCell::new(None) };
}

pub fn run_tray_menu_host(
    paths: AppPaths,
    app_version: &str,
    parent_pid: u32,
) -> Result<(), String> {
    select_winit_backend()?;
    let ui = TrayMenuWindow::new().map_err(|e| format!("create TrayMenuWindow: {e}"))?;
    ui.set_app_version(SharedString::from(app_version));
    let appearance = load_appearance_config(&paths.appearance_cfg);
    apply_theme_to_window(&ui, appearance);
    wire_tray_callbacks(&ui);

    TRAY_HOST.with(|slot| {
        *slot.borrow_mut() = Some(TrayHostState {
            ui,
            paths,
            visible: false,
            last_shown_at: Instant::now(),
        });
    });

    spawn_stdin_listener();
    let watchdog = Timer::default();
    if parent_pid > 0 {
        watchdog.start(TimerMode::Repeated, Duration::from_secs(2), move || {
            if !is_parent_alive(parent_pid) {
                let _ = slint::quit_event_loop();
            }
        });
    }

    let res = slint::run_event_loop_until_quit().map_err(|e| format!("tray event loop: {e}"));
    TRAY_HOST.with(|slot| *slot.borrow_mut() = None);
    res
}

fn wire_tray_callbacks(ui: &TrayMenuWindow) {
    ui.on_hide_menu(hide_tray_popup);
    ui.on_toggle_theme(|| {
        emit_ui_cmd("tray_toggle_theme", "");
        hide_tray_popup();
    });
    ui.on_trigger_action(|act, id| {
        hide_tray_popup();
        emit_ui_cmd("tray_action", &format!("{}|{id}", act.as_str()));
    });
    install_focus_lost_hook(ui.window(), || {
        let should_hide = TRAY_HOST.with(|slot| {
            slot.borrow().as_ref().is_some_and(|s| {
                s.visible && s.last_shown_at.elapsed() > Duration::from_millis(180)
            })
        });
        if should_hide {
            hide_tray_popup();
        }
    });
}

fn spawn_stdin_listener() {
    thread::spawn(|| {
        let stdin = io::stdin();
        for line in stdin.lock().lines().map_while(Result::ok) {
            let trimmed = line.trim().to_owned();
            if trimmed == "SHUTDOWN" {
                let _ = slint::invoke_from_event_loop(|| {
                    let _ = slint::quit_event_loop();
                });
                break;
            }
            if trimmed == "HIDE" {
                let _ = slint::invoke_from_event_loop(hide_tray_popup);
                continue;
            }
            if let Some(payload) = decode_tray_show(&trimmed) {
                let _ = slint::invoke_from_event_loop(move || show_tray_popup(payload));
            }
        }
        let _ = slint::invoke_from_event_loop(|| {
            let _ = slint::quit_event_loop();
        });
    });
}

fn show_tray_popup(payload: TrayShowPayload) {
    TRAY_HOST.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(s) = guard.as_mut() else {
            return;
        };
        let appearance = load_appearance_config(&s.paths.appearance_cfg);
        let dark = apply_theme_to_window(&s.ui, appearance);
        apply_tray_payload(&s.ui, &payload);

        let wa = primary_work_area();
        let x = (payload.x - TRAY_MENU_W as i32 + 24)
            .clamp(wa.x + 6, wa.x + wa.width as i32 - TRAY_MENU_W as i32 - 6);
        let y = (payload.y - TRAY_MENU_H as i32 - 8)
            .clamp(wa.y + 6, wa.y + wa.height as i32 - TRAY_MENU_H as i32 - 6);

        if !s.visible {
            let _ = s.ui.show();
        }
        configure_popup_window(s.ui.window(), x, y, TRAY_MENU_W, TRAY_MENU_H, dark);
        s.visible = true;
        s.last_shown_at = Instant::now();
    });
}

fn apply_tray_payload(ui: &TrayMenuWindow, p: &TrayShowPayload) {
    let f = &p.flags;
    let mv = &f.menu_vis;
    ui.set_expanded_section(0);
    ui.set_dock_module_enabled(f.dock_module_enabled);
    ui.set_widgets_module_enabled(f.widgets_module_enabled);
    ui.set_overlay_module_enabled(f.overlay_module_enabled);
    ui.set_overlay_setup_mode(f.overlay_setup_mode);
    ui.set_calendar_visible(f.calendar_visible);
    ui.set_sysmon_visible(f.sys_monitor_visible);
    ui.set_weather_visible(f.weather_visible);
    ui.set_notes_visible(f.notes_visible);
    ui.set_autostart_enabled(f.autostart_enabled);
    ui.set_vis_autostart(mv.show_autostart);
    ui.set_vis_docks(mv.show_docks);
    ui.set_vis_widgets(mv.show_widgets);
    ui.set_vis_overlay(mv.show_overlay);
    ui.set_vis_backup(mv.show_backup);
    ui.set_vis_debug(mv.show_debug);
    ui.set_vis_about(mv.show_about);
    let rows: Vec<SlintTrayDockRow> = p
        .docks
        .iter()
        .map(|d| SlintTrayDockRow {
            id: d.dock_id as i32,
            title: SharedString::from(d.title.as_str()),
            enabled: d.enabled,
        })
        .collect();
    ui.set_docks(ModelRc::from(Rc::new(VecModel::from(rows))));
}

fn hide_tray_popup() {
    TRAY_HOST.with(|slot| {
        if let Some(s) = slot.borrow_mut().as_mut() {
            if s.visible {
                set_window_visible(s.ui.window(), false);
                s.visible = false;
            }
        }
    });
}

fn emit_ui_cmd(action: &str, arg: &str) {
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{}", encode_ui_cmd(action, arg));
    let _ = out.flush();
}
