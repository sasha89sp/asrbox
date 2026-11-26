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
    configure_popup_window, force_window_repaint, install_focus_lost_hook, move_window_to,
    resize_window_to, select_winit_backend,
};
use crate::win_os::{is_parent_alive, primary_work_area};

const TRAY_MENU_W: u32 = 284;

struct TrayHostState {
    ui: TrayMenuWindow,
    paths: AppPaths,
    visible: bool,
    anchor_x: i32,
    anchor_y: i32,
    base_height: u32,
    dock_count: usize,
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
            anchor_x: 0,
            anchor_y: 0,
            base_height: 360,
            dock_count: 1,
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
    ui.on_section_changed(resize_tray_for_section);
    install_focus_lost_hook(ui.window(), || {
        let should_hide = TRAY_HOST.with(|slot| {
            slot.borrow().as_ref().is_some_and(|s| {
                s.visible && s.last_shown_at.elapsed() > Duration::from_millis(220)
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

fn compute_base_height(p: &TrayShowPayload) -> u32 {
    let mv = &p.flags.menu_vis;
    let mut rows = 2u32; // Settings + Exit
    if mv.show_docks {
        rows += 1;
    }
    if mv.show_widgets {
        rows += 1;
    }
    if mv.show_overlay {
        rows += 1;
    }
    if mv.show_backup {
        rows += 1;
    }
    if mv.show_autostart {
        rows += 1;
    }
    if mv.show_debug {
        rows += 1;
    }
    if mv.show_about {
        rows += 1;
    }
    // 16px pad + 36px header + 3 dividers + rows * 33px + bottom breathing room
    (74 + rows * 33).clamp(210, 420)
}

fn compute_popup_rect(anchor_x: i32, anchor_y: i32, h: u32) -> (i32, i32) {
    let wa = primary_work_area();
    let max_x = (wa.x + wa.width as i32 - TRAY_MENU_W as i32 - 8).max(wa.x + 8);
    let max_y = (wa.y + wa.height as i32 - h as i32 - 8).max(wa.y + 8);
    let x = (anchor_x - TRAY_MENU_W as i32 + 24).clamp(wa.x + 8, max_x);
    let y = (anchor_y - h as i32 - 8).clamp(wa.y + 8, max_y);
    (x, y)
}

fn resize_tray_for_section(section: i32) {
    TRAY_HOST.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(s) = guard.as_mut() else {
            return;
        };
        if !s.visible {
            return;
        }
        let extra = match section {
            1 => (3 + s.dock_count as u32) * 28,
            2 => 5 * 28,
            3 => 3 * 28,
            4 => 3 * 28,
            _ => 0,
        };
        let target_h = (s.base_height + extra).min(520);
        let (x, y) = compute_popup_rect(s.anchor_x, s.anchor_y, target_h);
        resize_window_to(s.ui.window(), TRAY_MENU_W, target_h);
        move_window_to(s.ui.window(), x, y);
        force_window_repaint(s.ui.window());
    });
}

fn show_tray_popup(payload: TrayShowPayload) {
    let weak = TRAY_HOST.with(|slot| {
        let mut guard = slot.borrow_mut();
        let s = guard.as_mut()?;
        if s.visible {
            let _ = s.ui.hide();
        }
        let appearance = load_appearance_config(&s.paths.appearance_cfg);
        let dark = apply_theme_to_window(&s.ui, appearance);
        apply_tray_payload(&s.ui, &payload);

        let base_h = compute_base_height(&payload);
        s.anchor_x = payload.x;
        s.anchor_y = payload.y;
        s.base_height = base_h;
        s.dock_count = payload.docks.len();
        let (x, y) = compute_popup_rect(payload.x, payload.y, base_h);

        let _ = s.ui.show();
        configure_popup_window(s.ui.window(), x, y, TRAY_MENU_W, base_h, dark);
        s.visible = true;
        s.last_shown_at = Instant::now();
        Some(s.ui.as_weak())
    });

    // Guarantee a full software-renderer present once Windows finishes mapping the popup.
    if let Some(w) = weak {
        Timer::single_shot(Duration::from_millis(20), move || {
            if let Some(u) = w.upgrade() {
                force_window_repaint(u.window());
            }
        });
    }
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
                let _ = s.ui.hide();
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
