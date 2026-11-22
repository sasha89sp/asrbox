#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use asrbox_mod_dock::{
    DockConfig, DockEdge, DockEvent, DockInstance, Point, Rect, apply_menu_action,
    build_menu_state, hit_test_item, icon_request_for_item, load_docks_from_file, parse_docks,
    render_dock_frame, save_docks_to_file, serialize_docks, to_dock_point, to_dock_rect,
    to_pixel_rect,
};
use asrbox_platform_api::{
    DockMenuAction, DockPlatform, IconProvider, PixelPoint, PixelRect, PlatformEvent, RgbaImage,
    TrayAction, TrayMenuEntry,
};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "--version") {
        println!("asrbox {APP_VERSION}");
        return ExitCode::SUCCESS;
    }
    if args.iter().any(|arg| arg == "--selftest") {
        return match run_selftest() {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => ExitCode::FAILURE,
        };
    }

    let safe_mode = args.iter().any(|arg| arg == "--safe-mode");
    match run_app(safe_mode) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

fn run_selftest() -> Result<(), String> {
    let cfg = DockConfig::new(1, "Selftest Dock");
    let area = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let dock = DockInstance::new(cfg.clone(), area).map_err(|e| e.to_string())?;
    let frame = render_dock_frame(dock.config(), dock.layout(), &HashMap::new());
    if frame.pixels.is_empty() {
        return Err("rendered frame is empty".to_owned());
    }
    let parsed = parse_docks(&serialize_docks(&[cfg], APP_VERSION))?;
    if parsed.len() != 1 {
        return Err("config roundtrip mismatch".to_owned());
    }
    Ok(())
}

#[cfg(not(windows))]
fn run_app(_safe_mode: bool) -> Result<(), String> {
    Err("asrbox UI runs on Windows only".to_owned())
}

#[cfg(windows)]
fn run_app(safe_mode: bool) -> Result<(), String> {
    use asrbox_platform_windows::{SingleInstanceGuard, WindowsDockHost, WindowsIconProvider};

    let Some(_guard) = SingleInstanceGuard::acquire("Local\\asrbox_single_instance_v1")? else {
        return Ok(());
    };
    let tooltip = format!("asrbox v{APP_VERSION}");
    let mut host = WindowsDockHost::new(&tooltip)?;
    let work_area = to_dock_rect(host.primary_work_area()?);
    let config_path = resolve_config_path();
    let icons = WindowsIconProvider::new();
    let mut ctrl = AppController::load(config_path, work_area, icons, safe_mode)?;
    ctrl.sync_all_windows(&mut host)?;

    host.run_event_loop(|platform, event| ctrl.handle_event(platform, event));
    Ok(())
}

#[cfg(windows)]
struct DockRuntime {
    instance: DockInstance,
    icons: HashMap<u64, RgbaImage>,
    window_open: bool,
}

#[cfg(windows)]
struct AppController<I: IconProvider> {
    config_path: PathBuf,
    work_area: Rect,
    icon_provider: I,
    docks: Vec<DockRuntime>,
    next_dock_id: u64,
    safe_mode: bool,
}

#[cfg(windows)]
impl<I: IconProvider> AppController<I> {
    fn load(
        config_path: PathBuf,
        work_area: Rect,
        icon_provider: I,
        safe_mode: bool,
    ) -> Result<Self, String> {
        let mut configs = load_docks_from_file(&config_path).unwrap_or_default();
        if configs.is_empty() {
            configs.push(DockConfig::new(1, "Док 1"));
        }
        let max_id = configs.iter().map(|c| c.id).max().unwrap_or(0);
        let mut docks = Vec::with_capacity(configs.len());
        for cfg in configs {
            let instance = DockInstance::new(cfg, work_area).map_err(|e| e.to_string())?;
            let icons = load_dock_icons(&icon_provider, instance.config());
            docks.push(DockRuntime {
                instance,
                icons,
                window_open: false,
            });
        }
        Ok(Self {
            config_path,
            work_area,
            icon_provider,
            docks,
            next_dock_id: max_id.saturating_add(1),
            safe_mode,
        })
    }

    fn sync_all_windows(&mut self, host: &mut dyn DockPlatform) -> Result<(), String> {
        for dock in &mut self.docks {
            let show = !self.safe_mode && dock.instance.config().enabled;
            let id = dock.instance.config().id;
            if show {
                let rect = to_pixel_rect(dock.instance.layout().window_rect);
                if !dock.window_open {
                    host.create_dock_window(id, &dock.instance.config().name, rect)?;
                    dock.window_open = true;
                }
                present_dock(host, dock)?;
            } else if dock.window_open {
                let _ = host.destroy_dock_window(id);
                dock.window_open = false;
            }
        }
        Ok(())
    }

    fn handle_event(&mut self, host: &mut dyn DockPlatform, event: PlatformEvent) {
        match event {
            PlatformEvent::PointerMove { dock_id, point } => {
                self.step_dock(host, dock_id, DockEvent::PointerMove(to_dock_point(point)));
            }
            PlatformEvent::PointerLeave { dock_id } => {
                self.step_dock(host, dock_id, DockEvent::PointerLeave);
            }
            PlatformEvent::AnimationTick { dock_id, dt_ms } => {
                self.step_dock(host, dock_id, DockEvent::AnimationTick { dt_ms });
            }
            PlatformEvent::DropFiles {
                dock_id,
                paths,
                point,
            } => self.on_drop(host, dock_id, paths, to_dock_point(point)),
            PlatformEvent::LeftClick { dock_id, point } => {
                self.on_left_click(host, dock_id, to_dock_point(point));
            }
            PlatformEvent::RightClick {
                dock_id,
                point,
                screen_point,
            } => self.on_right_click(host, dock_id, to_dock_point(point), screen_point),
            PlatformEvent::DisplayChanged { work_area } => self.on_display(host, work_area),
            PlatformEvent::TrayContextMenu { screen_point } => {
                self.on_tray_menu(host, screen_point);
            }
        }
    }

    fn step_dock(&mut self, host: &mut dyn DockPlatform, id: u64, ev: DockEvent) {
        let Some(dock) = self.docks.iter_mut().find(|d| d.instance.config().id == id) else {
            return;
        };
        let effect = dock.instance.handle_event(ev);
        let _ = host.set_animation_timer(id, effect.animation_active);
        if effect.needs_redraw && dock.window_open {
            let _ = present_dock(host, dock);
        }
    }

    fn on_display(&mut self, host: &mut dyn DockPlatform, area: PixelRect) {
        self.work_area = to_dock_rect(area);
        for dock in &mut self.docks {
            let ev = DockEvent::WorkAreaChanged(self.work_area);
            let effect = dock.instance.handle_event(ev);
            if effect.needs_redraw && dock.window_open {
                let _ = present_dock(host, dock);
            }
        }
    }

    fn on_drop(&mut self, host: &mut dyn DockPlatform, id: u64, paths: Vec<PathBuf>, pt: Point) {
        let Some(dock) = self.docks.iter_mut().find(|d| d.instance.config().id == id) else {
            return;
        };
        let ev = DockEvent::DropFiles {
            paths,
            local_point: pt,
        };
        let effect = dock.instance.handle_event(ev);
        if effect.added_item_ids.is_empty() {
            return;
        }
        for item_id in &effect.added_item_ids {
            let items = &dock.instance.config().items;
            if let Some(item) = items.iter().find(|i| i.id == *item_id) {
                let req = icon_request_for_item(item);
                if let Ok(img) = self.icon_provider.load_icon(&req, 128) {
                    dock.icons.insert(*item_id, img);
                }
            }
        }
        if effect.needs_redraw && dock.window_open {
            let _ = present_dock(host, dock);
        }
        self.persist();
    }

    fn on_left_click(&self, host: &dyn DockPlatform, id: u64, pt: Point) {
        let Some(dock) = self.docks.iter().find(|d| d.instance.config().id == id) else {
            return;
        };
        if let Some(item_id) = hit_test_item(dock.instance.layout(), pt) {
            let items = &dock.instance.config().items;
            if let Some(item) = items.iter().find(|i| i.id == item_id) {
                let _ = host.launch_target(&item.target);
            }
        }
    }

    fn on_right_click(
        &mut self,
        host: &mut dyn DockPlatform,
        id: u64,
        pt: Point,
        screen: PixelPoint,
    ) {
        let can_del = self.docks.len() > 1;
        let state = self
            .docks
            .iter()
            .find(|d| d.instance.config().id == id)
            .map(|d| build_menu_state(&d.instance, pt, can_del));
        if let Some(state) = state {
            if let Ok(Some(action)) = host.show_dock_menu(id, screen, &state) {
                self.apply_dock_action(host, id, action);
            }
        }
    }

    fn apply_dock_action(&mut self, host: &mut dyn DockPlatform, id: u64, action: DockMenuAction) {
        match action {
            DockMenuAction::AddNewDock => return self.add_new_dock(host),
            DockMenuAction::RemoveDock => return self.remove_dock(host, id),
            _ => {}
        }
        let picked = match action {
            DockMenuAction::ChooseItemIcon(_) => host.pick_icon_file().ok().flatten(),
            _ => None,
        };
        if let Some(dock) = self.docks.iter_mut().find(|d| d.instance.config().id == id) {
            let mut cfg = dock.instance.config().clone();
            if apply_menu_action(&mut cfg, action, picked) {
                if let Ok(rebuilt) = DockInstance::new(cfg, self.work_area) {
                    dock.icons = load_dock_icons(&self.icon_provider, rebuilt.config());
                    dock.instance = rebuilt;
                    let _ = host.set_animation_timer(id, false);
                    if dock.window_open {
                        let _ = present_dock(host, dock);
                    }
                    self.persist();
                }
            }
        }
    }

    fn on_tray_menu(&mut self, host: &mut dyn DockPlatform, screen: PixelPoint) {
        let entries: Vec<TrayMenuEntry> = self
            .docks
            .iter()
            .map(|d| TrayMenuEntry {
                dock_id: d.instance.config().id,
                title: d.instance.config().name.clone(),
                enabled: d.instance.config().enabled,
            })
            .collect();
        let Ok(Some(action)) = host.show_tray_menu(screen, &entries) else {
            return;
        };
        match action {
            TrayAction::AddDock => self.add_new_dock(host),
            TrayAction::ToggleDock(id) => self.toggle_dock(host, id),
            TrayAction::RemoveDock(id) => self.remove_dock(host, id),
            TrayAction::Exit => {
                self.persist();
                host.request_quit();
            }
        }
    }

    fn add_new_dock(&mut self, host: &mut dyn DockPlatform) {
        let id = self.next_dock_id;
        self.next_dock_id = self.next_dock_id.saturating_add(1);
        let mut cfg = DockConfig::new(id, format!("Док {id}"));
        cfg.edge = match self.docks.len() % 4 {
            0 => DockEdge::Bottom,
            1 => DockEdge::Left,
            2 => DockEdge::Right,
            _ => DockEdge::Top,
        };
        if let Ok(instance) = DockInstance::new(cfg, self.work_area) {
            self.docks.push(DockRuntime {
                instance,
                icons: HashMap::new(),
                window_open: false,
            });
            self.safe_mode = false;
            let _ = self.sync_all_windows(host);
            self.persist();
        }
    }

    fn toggle_dock(&mut self, host: &mut dyn DockPlatform, id: u64) {
        if let Some(dock) = self.docks.iter_mut().find(|d| d.instance.config().id == id) {
            let mut cfg = dock.instance.config().clone();
            cfg.enabled = !cfg.enabled;
            if let Ok(rebuilt) = DockInstance::new(cfg, self.work_area) {
                dock.instance = rebuilt;
            }
        }
        let _ = self.sync_all_windows(host);
        self.persist();
    }

    fn remove_dock(&mut self, host: &mut dyn DockPlatform, id: u64) {
        if self.docks.len() <= 1 {
            return;
        }
        if let Some(pos) = self.docks.iter().position(|d| d.instance.config().id == id) {
            let removed = self.docks.remove(pos);
            if removed.window_open {
                let _ = host.destroy_dock_window(id);
            }
            self.persist();
        }
    }

    fn persist(&self) {
        let cfgs: Vec<DockConfig> = self
            .docks
            .iter()
            .map(|d| d.instance.config().clone())
            .collect();
        let _ = save_docks_to_file(&self.config_path, &cfgs, APP_VERSION);
    }
}

#[cfg(windows)]
fn load_dock_icons<I: IconProvider>(provider: &I, cfg: &DockConfig) -> HashMap<u64, RgbaImage> {
    let mut map = HashMap::new();
    for item in &cfg.items {
        let req = icon_request_for_item(item);
        if let Ok(img) = provider.load_icon(&req, 128) {
            map.insert(item.id, img);
        }
    }
    map
}

#[cfg(windows)]
fn present_dock(host: &mut dyn DockPlatform, dock: &DockRuntime) -> Result<(), String> {
    let layout = dock.instance.layout();
    let frame = render_dock_frame(dock.instance.config(), layout, &dock.icons);
    let rect = to_pixel_rect(layout.window_rect);
    host.present_dock_frame(dock.instance.config().id, rect, &frame)
}

#[cfg(windows)]
fn resolve_config_path() -> PathBuf {
    let base = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("data").join("config").join("docks.cfg")
}
