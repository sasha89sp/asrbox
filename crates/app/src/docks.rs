#![cfg(windows)]

use std::collections::HashMap;
use std::path::PathBuf;

use asrbox_mod_dock::{
    apply_menu_action, build_menu_state, hit_test_item, icon_request_for_item,
    load_docks_from_file, render_dock_frame, save_docks_to_file, to_dock_rect, to_pixel_rect,
    DockConfig, DockEdge, DockEvent, DockInstance, Point, Rect,
};
use asrbox_platform_api::{
    DockMenuAction, DockPlatform, IconProvider, PixelPoint, PixelRect, RgbaImage, TrayMenuEntry,
};

pub struct DockRuntime {
    pub instance: DockInstance,
    pub icons: HashMap<u64, RgbaImage>,
    pub window_open: bool,
}

pub struct DockManager<I: IconProvider> {
    config_path: PathBuf,
    app_version: &'static str,
    work_area: Rect,
    icon_provider: I,
    docks: Vec<DockRuntime>,
    next_dock_id: u64,
}

impl<I: IconProvider> DockManager<I> {
    pub fn load(
        config_path: PathBuf,
        app_version: &'static str,
        work_area: Rect,
        icon_provider: I,
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
            app_version,
            work_area,
            icon_provider,
            docks,
            next_dock_id: max_id.saturating_add(1),
        })
    }

    pub fn sync_windows(
        &mut self,
        host: &mut dyn DockPlatform,
        safe_mode: bool,
    ) -> Result<(), String> {
        for dock in &mut self.docks {
            let show = !safe_mode && dock.instance.config().enabled;
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

    pub fn step_dock(&mut self, host: &mut dyn DockPlatform, id: u64, ev: DockEvent) {
        let Some(dock) = self.docks.iter_mut().find(|d| d.instance.config().id == id) else {
            return;
        };
        let effect = dock.instance.handle_event(ev);
        let _ = host.set_animation_timer(id, effect.animation_active);
        if effect.needs_redraw && dock.window_open {
            let _ = present_dock(host, dock);
        }
    }

    pub fn on_display(&mut self, host: &mut dyn DockPlatform, area: PixelRect) {
        self.work_area = to_dock_rect(area);
        for dock in &mut self.docks {
            let ev = DockEvent::WorkAreaChanged(self.work_area);
            let effect = dock.instance.handle_event(ev);
            if effect.needs_redraw && dock.window_open {
                let _ = present_dock(host, dock);
            }
        }
    }

    pub fn on_drop(
        &mut self,
        host: &mut dyn DockPlatform,
        id: u64,
        paths: Vec<PathBuf>,
        pt: Point,
    ) {
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

    pub fn on_left_click(&self, host: &dyn DockPlatform, id: u64, pt: Point) {
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

    pub fn on_right_click(
        &mut self,
        host: &mut dyn DockPlatform,
        id: u64,
        pt: Point,
        screen: PixelPoint,
        safe_mode: &mut bool,
    ) {
        let can_del = self.docks.len() > 1;
        let state = self
            .docks
            .iter()
            .find(|d| d.instance.config().id == id)
            .map(|d| build_menu_state(&d.instance, pt, can_del));
        if let Some(state) = state {
            if let Ok(Some(action)) = host.show_dock_menu(id, screen, &state) {
                self.apply_dock_action(host, id, action, safe_mode);
            }
        }
    }

    pub fn tray_entries(&self) -> Vec<TrayMenuEntry> {
        self.docks
            .iter()
            .map(|d| TrayMenuEntry {
                dock_id: d.instance.config().id,
                title: d.instance.config().name.clone(),
                enabled: d.instance.config().enabled,
            })
            .collect()
    }

    pub fn add_new_dock(&mut self, host: &mut dyn DockPlatform, safe_mode: &mut bool) {
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
            *safe_mode = false;
            let _ = self.sync_windows(host, false);
            self.persist();
        }
    }

    pub fn toggle_dock(&mut self, host: &mut dyn DockPlatform, id: u64, safe_mode: bool) {
        if let Some(dock) = self.docks.iter_mut().find(|d| d.instance.config().id == id) {
            let mut cfg = dock.instance.config().clone();
            cfg.enabled = !cfg.enabled;
            if let Ok(rebuilt) = DockInstance::new(cfg, self.work_area) {
                dock.instance = rebuilt;
            }
        }
        let _ = self.sync_windows(host, safe_mode);
        self.persist();
    }

    pub fn remove_dock(&mut self, host: &mut dyn DockPlatform, id: u64) {
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

    pub fn persist(&self) {
        let cfgs: Vec<DockConfig> = self
            .docks
            .iter()
            .map(|d| d.instance.config().clone())
            .collect();
        let _ = save_docks_to_file(&self.config_path, &cfgs, self.app_version);
    }

    fn apply_dock_action(
        &mut self,
        host: &mut dyn DockPlatform,
        id: u64,
        action: DockMenuAction,
        safe_mode: &mut bool,
    ) {
        match action {
            DockMenuAction::AddNewDock => return self.add_new_dock(host, safe_mode),
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
}

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

fn present_dock(host: &mut dyn DockPlatform, dock: &DockRuntime) -> Result<(), String> {
    let layout = dock.instance.layout();
    let frame = render_dock_frame(dock.instance.config(), layout, &dock.icons);
    let rect = to_pixel_rect(layout.window_rect);
    host.present_dock_frame(dock.instance.config().id, rect, &frame)
}
