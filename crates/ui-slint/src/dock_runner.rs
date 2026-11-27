use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{self, BufRead};
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use asrbox_core::{decode_supervisor_cmd, SupervisorCmd, WorkerMsg};
use asrbox_mod_about::AppPaths;
use asrbox_mod_dock::{load_docks_from_file, save_docks_to_file, DockConfig, DockId, DockItem};
use asrbox_platform_api::{
    load_appearance_config, load_menu_config, AppearanceConfig, DockMenuAction, DockMenuVisibility,
    PixelRect,
};
use slint::{ComponentHandle, Timer, TimerMode};

use crate::dock_icons::{is_folder_target, resolve_dock_target, DockIconCache};
use crate::dock_view::{
    add_dock_config, apply_native_dock_action, build_dock_menu_state, compute_dock_rect,
    remove_dock_config, set_custom_icon_config, sync_dock_window, toggle_dock_config,
};
use crate::generated::ui::DockWindow;
use crate::settings_state::apply_theme_to_window;
use crate::vibrancy::{
    apply_acrylic_vibrancy, apply_window_opacity, configure_desktop_window, enforce_skip_taskbar,
    install_dock_window_hooks, move_window_to, resize_window_to, select_winit_backend,
    set_window_layer, set_window_visible, start_window_drag, window_hwnd_raw, DesktopWindowSpec,
};
use crate::win_os::{
    activate_or_launch, emit_config_changed, emit_open_settings, emit_worker_log, emit_worker_msg,
    is_parent_alive, open_item_location, open_text_file, pick_icon_file, primary_work_area,
    running_exe_names, show_dock_native_menu, show_folder_native_popup,
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
    ignore_bounds_until: Instant,
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
    let base_dir = paths
        .data_dir
        .parent()
        .map_or_else(|| paths.data_dir.clone(), PathBuf::from);
    let state = DockHostState {
        paths,
        app_version: app_version.to_owned(),
        safe_mode,
        appearance,
        dark: true,
        vis,
        configs,
        windows: HashMap::new(),
        icons: DockIconCache::new(base_dir),
        running_exes: running_exe_names(),
        ignore_bounds_until: Instant::now() + Duration::from_millis(800),
    };
    DOCK_HOST.with(|slot| *slot.borrow_mut() = Some(state));
    sync_all_docks();
    emit_worker_log("INFO", "dock", "Slint-воркер док-панелей запущен");
    spawn_stdin_listener();
    let poll = Timer::default();
    poll.start(
        TimerMode::Repeated,
        Duration::from_millis(2500),
        refresh_running_dots,
    );
    let wd = Timer::default();
    if parent_pid > 0 {
        wd.start(TimerMode::Repeated, Duration::from_secs(2), move || {
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
        for line in io::stdin().lock().lines().map_while(Result::ok) {
            if let Some(cmd) = decode_supervisor_cmd(&line) {
                let _ = slint::invoke_from_event_loop(move || handle_supervisor_cmd(cmd));
            }
        }
        let _ = slint::invoke_from_event_loop(shutdown_dock_host);
    });
}

fn reload_docks_from_disk() {
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

fn handle_supervisor_cmd(cmd: SupervisorCmd) {
    match cmd {
        SupervisorCmd::ReloadConfig => reload_docks_from_disk(),
        SupervisorCmd::AddDock => mutate_docks(|s| {
            add_dock_config(&mut s.configs);
            s.safe_mode = false;
            true
        }),
        SupervisorCmd::ToggleDock(id) => mutate_docks(|s| toggle_dock_config(&mut s.configs, id)),
        SupervisorCmd::RemoveDock(id) => mutate_docks(|s| remove_dock_config(&mut s.configs, id)),
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
        s.ignore_bounds_until = Instant::now() + Duration::from_millis(700);
        let work_area = primary_work_area();
        let can_remove = s.configs.len() > 1;
        let active_ids: HashSet<DockId> = s
            .configs
            .iter()
            .filter(|c| !s.safe_mode && c.enabled)
            .map(|c| c.id)
            .collect();
        s.windows.retain(|id, win| {
            let keep = active_ids.contains(id);
            if !keep {
                set_window_visible(win.window(), false);
            }
            keep
        });
        let configs = s.configs.clone();
        for cfg in &configs {
            if !s.safe_mode && cfg.enabled {
                sync_single_dock(s, cfg, work_area, can_remove);
            }
        }
    });
}

fn sync_single_dock(
    s: &mut DockHostState,
    cfg: &DockConfig,
    work_area: PixelRect,
    can_remove: bool,
) {
    let (x, y, width, height) = compute_dock_rect(cfg, work_area);
    if let Some(win) = s.windows.get(&cfg.id) {
        let dark = apply_theme_to_window(win, s.appearance);
        if dark != s.dark {
            s.dark = dark;
            apply_acrylic_vibrancy(win.window(), dark);
        }
        sync_dock_window(win, cfg, &s.running_exes, &mut s.icons, can_remove);
        set_window_layer(win.window(), cfg.layer);
        apply_window_opacity(win.window(), cfg.opacity_percent);
        resize_window_to(win.window(), width, height);
        move_window_to(win.window(), x, y);
        set_window_visible(win.window(), true);
    } else if let Ok(win) = DockWindow::new() {
        let dark = apply_theme_to_window(&win, s.appearance);
        s.dark = dark;
        sync_dock_window(&win, cfg, &s.running_exes, &mut s.icons, can_remove);
        wire_dock_window(&win, cfg.id);
        let _ = win.show();
        configure_desktop_window(
            win.window(),
            DesktopWindowSpec {
                x,
                y,
                width,
                height,
                min_w: 88,
                min_h: 56,
                resizable: false,
                layer: cfg.layer,
                dark,
                opacity_percent: cfg.opacity_percent,
            },
        );
        let weak = win.as_weak();
        Timer::single_shot(Duration::from_millis(60), move || {
            if let Some(u) = weak.upgrade() {
                resize_window_to(u.window(), width, height);
                move_window_to(u.window(), x, y);
                enforce_skip_taskbar(u.window());
            }
        });
        s.windows.insert(cfg.id, win);
    }
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
    win.on_open_context_menu(move |item_id, is_sep| {
        handle_dock_context_menu(dock_id, item_id, is_sep);
    });
    install_dock_window_hooks(
        win.window(),
        move |x, y| handle_dock_moved(dock_id, x, y),
        move |path| handle_file_dropped(dock_id, path),
    );
}

fn handle_dock_moved(dock_id: DockId, x: i32, y: i32) {
    DOCK_HOST.with(|slot| {
        let mut g = slot.borrow_mut();
        let Some(s) = g
            .as_mut()
            .filter(|s| Instant::now() >= s.ignore_bounds_until)
        else {
            return;
        };
        if let Some(cfg) = s.configs.iter_mut().find(|c| c.id == dock_id) {
            if !cfg.locked && cfg.free_pos != Some((x, y)) {
                cfg.free_pos = Some((x, y));
                s.persist();
            }
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
        DockItem::from_dropped_path(next_id, path).is_ok_and(|item| {
            cfg.items.push(item);
            true
        })
    });
}

fn handle_launch_item(dock_id: DockId, item_id: u64) {
    let info = DOCK_HOST.with(|slot| {
        let g = slot.borrow();
        let s = g.as_ref()?;
        let cfg = s.configs.iter().find(|c| c.id == dock_id)?;
        let it = cfg
            .items
            .iter()
            .find(|i| i.id == item_id && !i.is_separator())?;
        let path = resolve_dock_target(&it.target, s.icons.base_dir());
        Some((path, cfg.focus_running_window, s.appearance))
    });
    let Some((path, focus, appearance)) = info else {
        return;
    };
    if is_folder_target(&path) {
        if let Ok(Some(chosen)) = show_folder_native_popup(&path, appearance) {
            if let Err(err) = activate_or_launch(&chosen, focus) {
                emit_worker_log(
                    "ERROR",
                    "dock",
                    format!("Запуск {}: {err}", chosen.display()),
                );
            }
        }
        return;
    }
    if let Err(err) = activate_or_launch(&path, focus) {
        emit_worker_log("ERROR", "dock", format!("Запуск {}: {err}", path.display()));
    }
}

fn handle_dock_context_menu(dock_id: DockId, item_id: i32, is_sep: bool) {
    let hovered = (item_id >= 0).then_some(item_id as u64);
    let ctx = DOCK_HOST.with(|slot| {
        let g = slot.borrow();
        let s = g.as_ref()?;
        let cfg = s.configs.iter().find(|c| c.id == dock_id)?;
        let hwnd = s
            .windows
            .get(&dock_id)
            .map_or(0, |w| window_hwnd_raw(w.window()));
        let menu_state = build_dock_menu_state(cfg, s.vis, hovered, is_sep, s.configs.len() > 1);
        Some((hwnd, menu_state, s.appearance, s.paths.docks_cfg.clone()))
    });
    let Some((hwnd, menu_state, appearance, docks_cfg)) = ctx else {
        return;
    };
    let Ok(Some(action)) = show_dock_native_menu(hwnd, &menu_state, appearance) else {
        return;
    };
    match action {
        DockMenuAction::OpenSettings => emit_open_settings("dock"),
        DockMenuAction::OpenDockConfig => {
            let _ = open_text_file(&docks_cfg);
        }
        DockMenuAction::ReloadDockConfig => reload_docks_from_disk(),
        DockMenuAction::AddNewDock => mutate_docks(|s| {
            add_dock_config(&mut s.configs);
            s.safe_mode = false;
            true
        }),
        DockMenuAction::RemoveDock => mutate_docks(|s| remove_dock_config(&mut s.configs, dock_id)),
        DockMenuAction::OpenItemFolder(id) => open_dock_item_folder(dock_id, id),
        DockMenuAction::ChooseItemIcon(id) => {
            if let Ok(Some(picked)) = pick_icon_file() {
                mutate_docks(|s| {
                    set_custom_icon_config(&mut s.configs, &mut s.icons, dock_id, id, picked)
                });
            }
        }
        other => mutate_docks(|s| {
            let icons = &mut s.icons;
            s.configs
                .iter_mut()
                .find(|c| c.id == dock_id)
                .is_some_and(|cfg| apply_native_dock_action(cfg, icons, other))
        }),
    }
}

fn open_dock_item_folder(dock_id: DockId, item_id: u64) {
    let target = DOCK_HOST.with(|slot| {
        let g = slot.borrow();
        let s = g.as_ref()?;
        let cfg = s.configs.iter().find(|c| c.id == dock_id)?;
        let item = cfg.items.iter().find(|i| i.id == item_id)?;
        Some(resolve_dock_target(&item.target, s.icons.base_dir()))
    });
    if let Some(p) = target {
        let _ = open_item_location(&p);
    }
}

fn refresh_running_dots() {
    let fresh = running_exe_names();
    DOCK_HOST.with(|slot| {
        let mut g = slot.borrow_mut();
        let Some(s) = g.as_mut().filter(|s| s.running_exes != fresh) else {
            return;
        };
        s.running_exes = fresh;
        let can_remove = s.configs.len() > 1;
        let configs = s.configs.clone();
        for cfg in &configs {
            if let Some(win) = s.windows.get(&cfg.id) {
                sync_dock_window(win, cfg, &s.running_exes, &mut s.icons, can_remove);
            }
        }
    });
}
