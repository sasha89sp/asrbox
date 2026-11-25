#![cfg(windows)]

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use asrbox_mod_dock::{
    build_menu_state, icon_request_for_item, load_docks_from_file, save_docks_to_file,
    to_dock_rect, to_pixel_rect, DockConfig, DockEdge, DockEvent, DockInstance, Point, Rect,
};
use asrbox_platform_api::{
    DockMenuVisibility, DockPlatform, FluentPalette, IconProvider, PixelPoint, PixelRect,
    RgbaImage, WidgetLayer,
};

use crate::dock_drag::{DockDragState, DragReleaseOutcome};
use crate::dock_ops::{build_runtimes, present_dock, reload_runtimes_in_place};
use crate::dock_shortcuts::{
    localize_existing_docks, localize_shortcut_path, shortcuts_dir_for_config,
};

pub struct DockRuntime {
    pub instance: DockInstance,
    pub icons: HashMap<u64, RgbaImage>,
    pub resolved_exes: HashMap<u64, String>,
    pub running_items: HashSet<u64>,
    pub window_open: bool,
    pub timer_active: bool,
    pub applied_layer: Option<WidgetLayer>,
}

pub struct DockManager<I: IconProvider> {
    config_path: PathBuf,
    pub(crate) app_version: &'static str,
    pub(crate) work_area: Rect,
    pub(crate) icon_provider: I,
    pub(crate) docks: Vec<DockRuntime>,
    next_dock_id: u64,
    pub(crate) drag: DockDragState,
    pub(crate) running_timer_on: bool,
    pub(crate) palette: FluentPalette,
    pub(crate) menu_vis: DockMenuVisibility,
}

impl<I: IconProvider> DockManager<I> {
    pub fn load(
        config_path: PathBuf,
        app_version: &'static str,
        work_area: Rect,
        icon_provider: I,
        host: &dyn DockPlatform,
    ) -> Result<Self, String> {
        let mut configs = load_docks_from_file(&config_path).unwrap_or_default();
        if configs.is_empty() {
            configs.push(DockConfig::new(1, "Док 1"));
        }
        let sc_dir = shortcuts_dir_for_config(&config_path);
        if localize_existing_docks(&mut configs, &sc_dir) {
            let _ = save_docks_to_file(&config_path, &configs, app_version);
        }
        let max_id = configs.iter().map(|c| c.id).max().unwrap_or(0);
        let docks = build_runtimes(configs, work_area, &icon_provider, host)?;
        Ok(Self {
            config_path,
            app_version,
            work_area,
            icon_provider,
            docks,
            next_dock_id: max_id.saturating_add(1),
            drag: DockDragState::default(),
            running_timer_on: false,
            palette: FluentPalette::default(),
            menu_vis: DockMenuVisibility::default(),
        })
    }

    pub(crate) fn config_path(&self) -> &std::path::Path {
        &self.config_path
    }

    pub fn set_palette(&mut self, host: &mut dyn DockPlatform, palette: FluentPalette) {
        self.palette = palette;
        self.redraw_open_docks(host);
    }

    pub fn dock_count(&self) -> usize {
        self.docks.len()
    }

    pub fn destroy_all_windows(&mut self, host: &mut dyn DockPlatform) {
        for dock in &mut self.docks {
            if dock.window_open {
                let _ = host.destroy_dock_window(dock.instance.config().id);
                dock.window_open = false;
                dock.timer_active = false;
            }
        }
        let _ = host.set_dock_tooltip(0, None);
        if self.running_timer_on {
            self.running_timer_on = false;
            let _ = host.set_running_poll_timer(false);
        }
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
                    dock.applied_layer = None;
                }
                let layer = dock.instance.config().layer;
                if dock.applied_layer != Some(layer) {
                    let _ = host.set_dock_layer(id, layer);
                    dock.applied_layer = Some(layer);
                }
                present_dock(host, dock, self.drag.overlay_for(dock), self.palette)?;
            } else if dock.window_open {
                let _ = host.destroy_dock_window(id);
                dock.window_open = false;
                dock.timer_active = false;
                dock.applied_layer = None;
            }
        }
        self.sync_running_timer(host);
        self.refresh_running_apps(host);
        Ok(())
    }

    pub fn open_config_file(&self, host: &dyn DockPlatform) -> Result<(), String> {
        if !self.config_path.exists() {
            self.persist();
        }
        host.open_text_file(&self.config_path)
    }

    pub fn reload_from_disk(
        &mut self,
        host: &mut dyn DockPlatform,
        safe_mode: bool,
    ) -> Result<usize, String> {
        let mut configs = load_docks_from_file(&self.config_path)?;
        if configs.is_empty() {
            configs.push(DockConfig::new(1, "Док 1"));
        }
        let max_id = configs.iter().map(|c| c.id).max().unwrap_or(0);
        let existing = std::mem::take(&mut self.docks);
        self.docks =
            reload_runtimes_in_place(existing, configs, self.work_area, &self.icon_provider, host)?;
        self.next_dock_id = max_id.saturating_add(1);
        self.cleanup_orphaned_shortcuts();
        self.sync_windows(host, safe_mode)?;
        Ok(self.docks.len())
    }

    pub fn refresh_running_apps(&mut self, host: &mut dyn DockPlatform) -> bool {
        if !self
            .docks
            .iter()
            .any(|d| d.window_open && d.instance.config().show_running_indicator)
        {
            return false;
        }
        let active = host.query_running_exe_names();
        let mut changed = false;
        for dock in &mut self.docks {
            if !dock.window_open || !dock.instance.config().show_running_indicator {
                continue;
            }
            let next: HashSet<u64> = dock
                .resolved_exes
                .iter()
                .filter(|(_, exe)| active.contains(*exe))
                .map(|(id, _)| *id)
                .collect();
            if next != dock.running_items {
                dock.running_items = next;
                let _ = present_dock(host, dock, self.drag.overlay_for(dock), self.palette);
                changed = true;
            }
        }
        changed
    }

    pub fn step_dock(&mut self, host: &mut dyn DockPlatform, id: u64, ev: DockEvent) -> bool {
        let drag_cancelled = matches!(ev, DockEvent::PointerLeave) && self.drag.cancel();
        let update_tip = matches!(ev, DockEvent::PointerMove(_) | DockEvent::PointerLeave);
        let Some(dock) = self.docks.iter_mut().find(|d| d.instance.config().id == id) else {
            return false;
        };
        let effect = dock.instance.handle_event(ev);
        if update_tip && dock.window_open {
            let ov = self.drag.overlay_for(dock);
            let tip = ov
                .dragged_item_id
                .is_none()
                .then(|| dock.instance.hovered_tooltip())
                .flatten();
            let _ = host.set_dock_tooltip(id, tip.as_deref());
        }
        if dock.timer_active != effect.animation_active {
            dock.timer_active = effect.animation_active;
            let _ = host.set_animation_timer(id, effect.animation_active);
        }
        if (effect.needs_redraw || drag_cancelled) && dock.window_open {
            let _ = present_dock(host, dock, self.drag.overlay_for(dock), self.palette);
            return true;
        }
        false
    }

    pub fn on_left_down(&mut self, id: u64, local_pt: Point, screen_pt: PixelPoint) {
        self.drag.on_left_down(&self.docks, id, local_pt, screen_pt);
    }

    pub fn on_drag_move(&mut self, host: &mut dyn DockPlatform, screen_pt: PixelPoint) {
        if self
            .drag
            .on_drag_move(&mut self.docks, self.work_area, screen_pt)
        {
            let _ = host.set_dock_tooltip(0, None);
            self.redraw_open_docks(host);
        }
    }

    pub fn on_left_up(
        &mut self,
        host: &mut dyn DockPlatform,
        screen_pt: PixelPoint,
    ) -> Option<String> {
        match self.drag.on_left_up(&mut self.docks, host, screen_pt) {
            DragReleaseOutcome::None => {
                self.redraw_open_docks(host);
                None
            }
            DragReleaseOutcome::Mutated(msg) => {
                self.redraw_open_docks(host);
                self.persist();
                self.cleanup_orphaned_shortcuts();
                self.refresh_running_apps(host);
                Some(msg)
            }
            DragReleaseOutcome::ClickSlot {
                dock_id,
                slot_kind,
                item_id,
            } => self.handle_slot_click(host, dock_id, slot_kind, item_id),
        }
    }

    pub fn on_display(&mut self, host: &mut dyn DockPlatform, area: PixelRect) {
        self.work_area = to_dock_rect(area);
        for dock in &mut self.docks {
            let effect = dock
                .instance
                .handle_event(DockEvent::WorkAreaChanged(self.work_area));
            if effect.needs_redraw && dock.window_open {
                let _ = present_dock(host, dock, self.drag.overlay_for(dock), self.palette);
            }
        }
    }

    pub fn on_drop(
        &mut self,
        host: &mut dyn DockPlatform,
        id: u64,
        paths: Vec<PathBuf>,
        pt: Point,
    ) -> usize {
        let sc_dir = shortcuts_dir_for_config(&self.config_path);
        let localized: Vec<PathBuf> = paths
            .into_iter()
            .map(|p| localize_shortcut_path(&p, &sc_dir))
            .collect();
        let Some(dock) = self.docks.iter_mut().find(|d| d.instance.config().id == id) else {
            return 0;
        };
        let effect = dock.instance.handle_event(DockEvent::DropFiles {
            paths: localized,
            local_point: pt,
        });
        let added = effect.added_item_ids.len();
        if added == 0 {
            return 0;
        }
        for item_id in &effect.added_item_ids {
            if let Some(item) = dock
                .instance
                .config()
                .items
                .iter()
                .find(|i| i.id == *item_id && !i.is_separator())
            {
                if let Ok(img) = self
                    .icon_provider
                    .load_icon(&icon_request_for_item(item), 128)
                {
                    dock.icons.insert(*item_id, img);
                }
                if let Some(exe) = host.resolve_target_exe_name(&item.target) {
                    dock.resolved_exes.insert(*item_id, exe);
                }
            }
        }
        if effect.needs_redraw && dock.window_open {
            let _ = present_dock(host, dock, self.drag.overlay_for(dock), self.palette);
        }
        self.persist();
        self.refresh_running_apps(host);
        added
    }

    pub fn on_right_click(
        &mut self,
        host: &mut dyn DockPlatform,
        id: u64,
        pt: Point,
        screen: PixelPoint,
        safe_mode: &mut bool,
    ) -> Option<String> {
        let can_del = self.docks.len() > 1;
        let menu_vis = self.menu_vis;
        let state = self
            .docks
            .iter()
            .find(|d| d.instance.config().id == id)
            .map(|d| build_menu_state(&d.instance, pt, can_del, menu_vis))?;
        let _ = host.set_dock_tooltip(id, None);
        let action = host.show_dock_menu(id, screen, &state).ok().flatten()?;
        Some(self.apply_dock_action(host, id, action, safe_mode))
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
                resolved_exes: HashMap::new(),
                running_items: HashSet::new(),
                window_open: false,
                timer_active: false,
                applied_layer: None,
            });
            *safe_mode = false;
            let _ = self.sync_windows(host, false);
            self.persist();
        }
    }

    pub fn toggle_dock(&mut self, host: &mut dyn DockPlatform, id: u64, safe_mode: bool) {
        if let Some(dock) = self.docks.iter_mut().find(|d| d.instance.config().id == id) {
            dock.instance.config_mut().enabled = !dock.instance.config().enabled;
            dock.instance.refresh_layout();
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
            self.cleanup_orphaned_shortcuts();
            self.sync_running_timer(host);
        }
    }
}
