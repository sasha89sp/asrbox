use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use asrbox_core::{decode_supervisor_cmd, SupervisorCmd, WorkerMsg};
use asrbox_mod_about::AppPaths;
use asrbox_mod_dock::{
    load_docks_from_file, save_docks_to_file, DockConfig, DockEdge, DockId, DockItem, IconSource,
};
use asrbox_platform_api::{
    load_appearance_config, load_menu_config, AppearanceConfig, DockMenuVisibility,
};
use slint::{ComponentHandle, Timer, TimerMode};

use crate::dock_view::{apply_dock_mutation, compute_dock_rect, sync_dock_window, DockIconCache};
use crate::generated::ui::DockWindow;
use crate::settings_state::apply_theme_to_window;
use crate::vibrancy::{
    apply_acrylic_vibrancy, configure_desktop_window, install_dock_window_hooks, move_window_to,
    resize_window_to, select_winit_backend, set_window_layer, set_window_visible,
    start_window_drag,
};
use crate::win_os::{
    activate_or_launch, emit_config_changed, emit_open_settings, emit_worker_log, emit_worker_msg,
    is_parent_alive, open_item_location, pick_icon_file, primary_work_area, running_exe_names,
};

struct DockHostState {
    paths: AppPaths,
    app_version: String,
    safe_mode: bool,
    appearance: AppearanceConfig,
    dark: bool,
    vis: DockMenuVisibility,
    configs: Vec<DockConfig>,
    windows: HashMap<DockId, DockWindow>,
    icons: DockIconCache,
    running_exes: HashSet<String>,
    suppress_move_save: bool,
}

impl DockHostState {
    fn persist(&self) {
        let _ = save_docks_to_file(&self.paths.docks_cfg, &self.configs, &self.app_version);
    }
}

thread_local! {
    static DOCK_HOST: RefCell<Option<DockHostState>> = const { RefCell::new(None) };
}

pub fn run_dock_host(
    paths: AppPaths,
    app_version: &str,
    parent_pid: u32,
    safe_mode: bool,
) -> Result<(), String> {
    select_winit_backend()?;
    emit_worker_msg(&WorkerMsg::Ready {
        module: "dock".to_owned(),
        pid: std::process::id(),
    });
    let appearance = load_appearance_config(&paths.appearance_cfg);
    let vis = load_menu_config(&paths.menus_cfg).dock;
    let mut configs = load_docks_from_file(&paths.docks_cfg).unwrap_or_default();
    if configs.is_empty() {
        configs.push(DockConfig::new(1, "Док 1"));
    }
    let state = DockHostState {
        paths,
        app_version: app_version.to_owned(),
        safe_mode,
        appearance,
        dark: true,
        vis,
        configs,
        windows: HashMap::new(),
        icons: DockIconCache::new(),
        running_exes: running_exe_names(),
        suppress_move_save: false,
    };
    DOCK_HOST.with(|slot| *slot.borrow_mut() = Some(state));
    sync_all_docks();
    emit_worker_log("INFO", "dock", "Slint-воркер док-панелей запущен");

    spawn_stdin_listener();
    let poll_timer = Timer::default();
    poll_timer.start(
        TimerMode::Repeated,
        Duration::from_millis(2500),
        refresh_running_dots,
    );

    let watchdog = Timer::default();
    if parent_pid > 0 {
        watchdog.start(TimerMode::Repeated, Duration::from_secs(2), move || {
            if !is_parent_alive(parent_pid) {
                shutdown_dock_host();
            }
        });
    }

    let res = slint::run_event_loop_until_quit().map_err(|e| format!("dock event loop: {e}"));
    DOCK_HOST.with(|slot| *slot.borrow_mut() = None);
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
        let _ = slint::invoke_from_event_loop(shutdown_dock_host);
    });
}

fn handle_supervisor_cmd(cmd: SupervisorCmd) {
    match cmd {
        SupervisorCmd::ReloadConfig => {
            DOCK_HOST.with(|slot| {
                if let Some(s) = slot.borrow_mut().as_mut() {
                    s.appearance = load_appearance_config(&s.paths.appearance_cfg);
                    s.vis = load_menu_config(&s.paths.menus_cfg).dock;
                    if let Ok(fresh) = load_docks_from_file(&s.paths.docks_cfg) {
                        if !fresh.is_empty() {
                            s.configs = fresh;
                        }
                    }
                }
            });
            sync_all_docks();
        }
        SupervisorCmd::AddDock => add_new_dock(),
        SupervisorCmd::ToggleDock(id) => toggle_dock_enabled(id),
        SupervisorCmd::RemoveDock(id) => remove_dock_by_id(id),
        SupervisorCmd::Shutdown => shutdown_dock_host(),
        _ => {}
    }
}

fn shutdown_dock_host() {
    DOCK_HOST.with(|slot| {
        if let Some(s) = slot.borrow_mut().take() {
            s.persist();
        }
    });
    let _ = slint::quit_event_loop();
}

fn sync_all_docks() {
    DOCK_HOST.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(s) = guard.as_mut() else {
            return;
        };
        s.suppress_move_save = true;
        let work_area = primary_work_area();
        let can_remove = s.configs.len() > 1;
        let active_ids: HashSet<DockId> = s
            .configs
            .iter()
            .filter(|c| !s.safe_mode && c.enabled)
            .map(|c| c.id)
            .collect();
        s.windows.retain(|id, win| {
            if active_ids.contains(id) {
                true
            } else {
                set_window_visible(win.window(), false);
                false
            }
        });
        let configs = s.configs.clone();
        for cfg in &configs {
            if s.safe_mode || !cfg.enabled {
                continue;
            }
            let (x, y, w, h) = compute_dock_rect(cfg, work_area);
            if let Some(win) = s.windows.get(&cfg.id) {
                let dark = apply_theme_to_window(win, s.appearance);
                if dark != s.dark {
                    s.dark = dark;
                    apply_acrylic_vibrancy(win.window(), dark);
                }
                sync_dock_window(win, cfg, s.vis, &s.running_exes, &mut s.icons, can_remove);
                set_window_layer(win.window(), cfg.layer);
                resize_window_to(win.window(), w, h);
                move_window_to(win.window(), x, y);
                set_window_visible(win.window(), true);
            } else if let Ok(win) = DockWindow::new() {
                let dark = apply_theme_to_window(&win, s.appearance);
                s.dark = dark;
                sync_dock_window(&win, cfg, s.vis, &s.running_exes, &mut s.icons, can_remove);
                wire_dock_window(&win, cfg.id);
                let _ = win.show();
                configure_desktop_window(win.window(), x, y, w, h, cfg.layer, dark);
                s.windows.insert(cfg.id, win);
            }
        }
        s.suppress_move_save = false;
    });
}

fn wire_dock_window(win: &DockWindow, dock_id: DockId) {
    let w = win.as_weak();
    win.on_drag_window(move || {
        if let Some(u) = w.upgrade() {
            start_window_drag(u.window());
        }
    });
    win.on_open_settings(|| emit_open_settings("dock"));
    win.on_launch_item(move |item_id| handle_launch_item(dock_id, item_id as u64));
    win.on_menu_action(move |action, arg| handle_menu_action(dock_id, action.as_str(), arg));
    install_dock_window_hooks(
        win.window(),
        move |x, y| handle_dock_moved(dock_id, x, y),
        move |path| handle_file_dropped(dock_id, path),
    );
}

fn handle_dock_moved(dock_id: DockId, x: i32, y: i32) {
    DOCK_HOST.with(|slot| {
        let mut guard = slot.borrow_mut();
        let Some(s) = guard.as_mut() else {
            return;
        };
        if s.suppress_move_save {
            return;
        }
        let mut dirty = false;
        if let Some(cfg) = s.configs.iter_mut().find(|c| c.id == dock_id) {
            if !cfg.locked && cfg.free_pos != Some((x, y)) {
                cfg.free_pos = Some((x, y));
                dirty = true;
            }
        }
        if dirty {
            s.persist();
        }
    });
}

fn mutate_docks(f: impl FnOnce(&mut DockHostState) -> bool) {
    let changed = DOCK_HOST.with(|slot| {
        let mut g = slot.borrow_mut();
        let Some(s) = g.as_mut() else {
            return false;
        };
        let ok = f(s);
        if ok {
            s.persist();
        }
        ok
    });
    if changed {
        sync_all_docks();
        emit_config_changed("dock");
    }
}

fn handle_file_dropped(dock_id: DockId, path: PathBuf) {
    mutate_docks(|s| {
        let Some(cfg) = s.configs.iter_mut().find(|c| c.id == dock_id) else {
            return false;
        };
        if cfg.lock_items {
            emit_worker_log(
                "INFO",
                "dock",
                "Защита значков активна: добавление заблокировано",
            );
            return false;
        }
        let next_id = cfg.next_item_id();
        if let Ok(item) = DockItem::from_dropped_path(next_id, path) {
            cfg.items.push(item);
            return true;
        }
        false
    });
}

fn handle_launch_item(dock_id: DockId, item_id: u64) {
    let target = DOCK_HOST.with(|slot| {
        let g = slot.borrow();
        let cfg = g.as_ref()?.configs.iter().find(|c| c.id == dock_id)?;
        let it = cfg
            .items
            .iter()
            .find(|i| i.id == item_id && !i.is_separator())?;
        Some((it.target.clone(), cfg.focus_running_window))
    });
    if let Some((path, focus)) = target {
        if let Err(err) = activate_or_launch(&path, focus) {
            emit_worker_log("ERROR", "dock", format!("Запуск {}: {err}", path.display()));
        }
    }
}

fn handle_menu_action(dock_id: DockId, action: &str, arg: i32) {
    match action {
        "add_dock" => add_new_dock(),
        "remove_dock" => remove_dock_by_id(dock_id),
        "open_item_folder" => open_dock_item_folder(dock_id, arg as u64),
        "choose_icon" => choose_custom_icon(dock_id, arg as u64),
        _ => mutate_docks(|s| {
            let icons = &mut s.icons;
            s.configs
                .iter_mut()
                .find(|c| c.id == dock_id)
                .is_some_and(|cfg| apply_dock_mutation(cfg, icons, action, arg))
        }),
    }
}

fn open_dock_item_folder(dock_id: DockId, item_id: u64) {
    let target = DOCK_HOST.with(|slot| {
        let g = slot.borrow();
        let cfg = g.as_ref()?.configs.iter().find(|c| c.id == dock_id)?;
        cfg.items
            .iter()
            .find(|i| i.id == item_id)
            .map(|i| i.target.clone())
    });
    if let Some(p) = target {
        let _ = open_item_location(&p);
    }
}

fn choose_custom_icon(dock_id: DockId, item_id: u64) {
    let Ok(Some(picked)) = pick_icon_file() else {
        return;
    };
    mutate_docks(|s| {
        let Some(it) = s
            .configs
            .iter_mut()
            .find(|c| c.id == dock_id)
            .and_then(|c| c.items.iter_mut().find(|i| i.id == item_id))
        else {
            return false;
        };
        it.icon = IconSource::CustomFile(picked);
        s.icons.invalidate_item(it);
        true
    });
}

fn add_new_dock() {
    mutate_docks(|s| {
        let next_id = s.configs.iter().map(|c| c.id).max().unwrap_or(0) + 1;
        let mut cfg = DockConfig::new(next_id, format!("Док {next_id}"));
        cfg.edge = match next_id % 4 {
            0 => DockEdge::Right,
            2 => DockEdge::Top,
            3 => DockEdge::Left,
            _ => DockEdge::Bottom,
        };
        s.configs.push(cfg);
        s.safe_mode = false;
        true
    });
}

fn toggle_dock_enabled(dock_id: DockId) {
    mutate_docks(|s| {
        let Some(cfg) = s.configs.iter_mut().find(|c| c.id == dock_id) else {
            return false;
        };
        cfg.enabled = !cfg.enabled;
        true
    });
}

fn remove_dock_by_id(dock_id: DockId) {
    mutate_docks(|s| {
        if s.configs.len() <= 1 {
            return false;
        }
        s.configs.retain(|c| c.id != dock_id);
        true
    });
}

fn refresh_running_dots() {
    let fresh = running_exe_names();
    DOCK_HOST.with(|slot| {
        let mut g = slot.borrow_mut();
        let Some(s) = g.as_mut() else {
            return;
        };
        if s.running_exes == fresh {
            return;
        }
        s.running_exes = fresh;
        let can_remove = s.configs.len() > 1;
        let configs = s.configs.clone();
        for cfg in &configs {
            if let Some(win) = s.windows.get(&cfg.id) {
                sync_dock_window(win, cfg, s.vis, &s.running_exes, &mut s.icons, can_remove);
            }
        }
    });
}
