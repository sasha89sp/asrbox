use std::cell::RefCell;
use std::collections::HashSet;
use std::io::{self, BufRead, BufReader, Write};
use std::rc::Rc;
use std::thread;
use std::time::Duration;

use asrbox_mod_about::AppPaths;
use asrbox_mod_settings::ui_state_store::{load_expanded_sections, save_expanded_sections};
use slint::{ComponentHandle, SharedString, Timer, TimerMode, Weak};

use crate::generated::ui::SettingsWindow;
use crate::ipc_proto::{decode_runtime_status, encode_ui_cmd, escape_field, UiRuntimeStatus};
use crate::settings_state::{apply_settings_state, SettingsViewContext};
use crate::vibrancy::{
    apply_settings_chrome, center_window_in_work_area, focus_window, minimize_window,
    select_winit_backend, start_window_drag, start_window_resize, toggle_maximize_window,
};
use crate::win_os::primary_work_area;

struct LocalSettingsState {
    paths: AppPaths,
    selected_dock: usize,
    selected_dock_item: Option<u64>,
    expanded: HashSet<String>,
    search: String,
    runtime: UiRuntimeStatus,
    last_dark: bool,
}

impl LocalSettingsState {
    fn sync_ui(&mut self, ui: &SettingsWindow, refresh_inputs: bool) {
        let ctx = SettingsViewContext {
            paths: &self.paths,
            selected_dock: self.selected_dock,
            selected_dock_item: self.selected_dock_item,
            expanded: &self.expanded,
            search: &self.search,
            runtime: &self.runtime,
            refresh_inputs,
        };
        let (dark, dock_count) = apply_settings_state(ui, &ctx);
        if self.selected_dock >= dock_count {
            self.selected_dock = dock_count.saturating_sub(1);
        }
        if dark != self.last_dark {
            self.last_dark = dark;
            apply_settings_chrome(ui.window(), dark);
        }
    }
}

pub fn run_settings_window(
    paths: AppPaths,
    app_version: &str,
    initial_tab: i32,
    initial_widget: i32,
) -> Result<(), String> {
    select_winit_backend()?;
    let ui = SettingsWindow::new().map_err(|e| format!("create SettingsWindow: {e}"))?;
    ui.set_app_version(SharedString::from(app_version));
    ui.set_active_tab(initial_tab.clamp(0, 5));
    ui.set_selected_widget(initial_widget.clamp(0, 3));

    let expanded = load_expanded_sections(&paths.settings_ui_cfg);
    let state = Rc::new(RefCell::new(LocalSettingsState {
        paths,
        selected_dock: 0,
        selected_dock_item: None,
        expanded,
        search: String::new(),
        runtime: UiRuntimeStatus::default(),
        last_dark: true,
    }));
    state.borrow_mut().sync_ui(&ui, true);
    let initial_dark = state.borrow().last_dark;

    wire_window_chrome(&ui);
    wire_settings_callbacks(&ui, &state);
    spawn_stdin_listener(ui.as_weak(), &state);

    let weak_vib = ui.as_weak();
    Timer::single_shot(Duration::from_millis(24), move || {
        if let Some(win) = weak_vib.upgrade() {
            let wa = primary_work_area();
            center_window_in_work_area(win.window(), wa, 860, 560, 360, 460);
            apply_settings_chrome(win.window(), initial_dark);
        }
    });

    let diag_timer = Timer::default();
    let weak_diag = ui.as_weak();
    diag_timer.start(
        TimerMode::Repeated,
        Duration::from_millis(1500),
        move || {
            if let Some(win) = weak_diag.upgrade() {
                if win.get_active_tab() == 4 && win.get_diag_monitor_enabled() {
                    emit_cmd("refresh_diag", "");
                }
            }
        },
    );

    ui.run().map_err(|e| format!("run SettingsWindow: {e}"))
}

fn wire_window_chrome(ui: &SettingsWindow) {
    let w = ui.as_weak();
    ui.on_drag_window(move || {
        if let Some(u) = w.upgrade() {
            start_window_drag(u.window());
        }
    });
    let w = ui.as_weak();
    ui.on_resize_window(move |dir| {
        if let Some(u) = w.upgrade() {
            start_window_resize(u.window(), dir);
        }
    });
    let w = ui.as_weak();
    ui.on_minimize_window(move || {
        if let Some(u) = w.upgrade() {
            minimize_window(u.window());
        }
    });
    let w = ui.as_weak();
    ui.on_maximize_window(move || {
        if let Some(u) = w.upgrade() {
            toggle_maximize_window(u.window());
        }
    });
    ui.on_close_window(|| {
        emit_cmd("settings_closed", "");
        let _ = slint::quit_event_loop();
    });
}

fn wire_settings_callbacks(ui: &SettingsWindow, state: &Rc<RefCell<LocalSettingsState>>) {
    let w = ui.as_weak();
    let st = Rc::clone(state);
    ui.on_search_changed(move |q| {
        st.borrow_mut().search = q.to_string();
        if let Some(u) = w.upgrade() {
            st.borrow_mut().sync_ui(&u, false);
        }
    });

    let w = ui.as_weak();
    let st = Rc::clone(state);
    ui.on_dispatch(move |action, arg| {
        let act = action.to_string();
        let val = arg.to_string();
        if handle_local_action(&st, &w, &act, &val) {
            return;
        }
        let dock_idx = st.borrow().selected_dock;
        if act.starts_with("dock_") || act == "remove_dock" {
            emit_cmd(&act, &format!("{dock_idx}|{val}"));
        } else {
            emit_cmd(&act, &val);
        }
    });

    ui.on_save_webdav_server(move |prov, url, user, pass, dir| {
        let payload = format!(
            "{prov}|{}|{}|{}|{}",
            escape_field(url.as_str()),
            escape_field(user.as_str()),
            escape_field(pass.as_str()),
            escape_field(dir.as_str()),
        );
        emit_cmd("save_webdav_server", &payload);
    });

    ui.on_save_gh_settings(move |repo, branch, token| {
        let payload = format!(
            "{}|{}|{}",
            escape_field(repo.as_str()),
            escape_field(branch.as_str()),
            escape_field(token.as_str()),
        );
        emit_cmd("save_gh_settings", &payload);
    });
}

fn handle_local_action(
    state: &Rc<RefCell<LocalSettingsState>>,
    weak: &Weak<SettingsWindow>,
    act: &str,
    val: &str,
) -> bool {
    match act {
        "toggle_section" => {
            let mut st = state.borrow_mut();
            if !st.expanded.remove(val) {
                st.expanded.insert(val.to_owned());
            }
            let _ = save_expanded_sections(&st.paths.settings_ui_cfg, &st.expanded);
            if let Some(u) = weak.upgrade() {
                st.sync_ui(&u, false);
            }
            true
        }
        "select_dock" => {
            if let Ok(idx) = val.parse::<usize>() {
                let mut st = state.borrow_mut();
                st.selected_dock = idx;
                st.selected_dock_item = None;
                if let Some(u) = weak.upgrade() {
                    u.set_dock_selected_item_id(-1);
                    st.sync_ui(&u, false);
                }
            }
            true
        }
        "dock_select_item" => {
            if let Ok(id) = val.parse::<u64>() {
                let mut st = state.borrow_mut();
                st.selected_dock_item = Some(id);
                if let Some(u) = weak.upgrade() {
                    st.sync_ui(&u, false);
                }
            }
            true
        }
        _ => false,
    }
}

fn spawn_stdin_listener(weak: Weak<SettingsWindow>, state: &Rc<RefCell<LocalSettingsState>>) {
    SETTINGS_STATE_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Rc::clone(state));
    });
    thread::spawn(move || {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());
        for line in reader.lines().map_while(Result::ok) {
            let trimmed = line.trim().to_owned();
            if trimmed == "SHUTDOWN" {
                let _ = slint::invoke_from_event_loop(|| {
                    let _ = slint::quit_event_loop();
                });
                return;
            }
            let w = weak.clone();
            let _ = slint::invoke_from_event_loop(move || {
                apply_host_line(&w, &trimmed);
            });
        }
        let _ = slint::invoke_from_event_loop(|| {
            let _ = slint::quit_event_loop();
        });
    });
}

thread_local! {
    static SETTINGS_STATE_HOOK: RefCell<Option<Rc<RefCell<LocalSettingsState>>>> =
        const { RefCell::new(None) };
}

fn apply_host_line(weak: &Weak<SettingsWindow>, line: &str) {
    let Some(ui) = weak.upgrade() else {
        return;
    };
    SETTINGS_STATE_HOOK.with(|slot| {
        let Some(rc) = slot.borrow().as_ref().cloned() else {
            return;
        };
        if let Some(st) = decode_runtime_status(line) {
            rc.borrow_mut().runtime = st;
            rc.borrow_mut().sync_ui(&ui, true);
        } else if line == "REFRESH" {
            rc.borrow_mut().sync_ui(&ui, true);
        } else if let Some(rest) = line.strip_prefix("TAB|") {
            let mut p = rest.splitn(2, '|');
            if let Some(tab) = p.next().and_then(|s| s.parse::<i32>().ok()) {
                ui.set_active_tab(tab.clamp(0, 5));
            }
            if let Some(sub) = p.next().and_then(|s| s.parse::<i32>().ok()) {
                ui.set_selected_widget(sub.clamp(0, 3));
            }
            ui.invoke_reset_scroll();
            focus_window(ui.window());
            rc.borrow_mut().sync_ui(&ui, true);
        }
    });
}

fn emit_cmd(action: &str, arg: &str) {
    let mut out = io::stdout().lock();
    let _ = writeln!(out, "{}", encode_ui_cmd(action, arg));
    let _ = out.flush();
}
