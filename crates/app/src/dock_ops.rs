#![cfg(windows)]

use std::collections::{HashMap, HashSet};

use asrbox_mod_dock::{
    apply_menu_action, icon_request_for_item, render_dock_widget_frame, save_docks_to_file,
    to_pixel_rect, DockConfig, DockInstance, DockRenderOverlay, Rect, SlotKind,
};
use asrbox_platform_api::{
    DockMenuAction, DockPlatform, FluentPalette, IconProvider, RgbaImage, TrayMenuEntry,
};

use crate::dock_shortcuts::{cleanup_unused_shortcuts, shortcuts_dir_for_config};
use crate::docks::{DockManager, DockRuntime};

impl<I: IconProvider> DockManager<I> {
    pub(crate) fn handle_slot_click(
        &mut self,
        host: &mut dyn DockPlatform,
        id: u64,
        kind: SlotKind,
        item_id: u64,
    ) -> Option<String> {
        let (target, label, focus_running, tile_screen_rect) = {
            let dock = self
                .docks
                .iter_mut()
                .find(|d| d.instance.config().id == id)?;
            match kind {
                SlotKind::SettingsButton => return Some("open_settings".to_owned()),
                SlotKind::Separator | SlotKind::DragHandle => return None,
                SlotKind::Shortcut => {
                    let cfg = dock.instance.config();
                    let item = cfg.items.iter().find(|i| i.id == item_id)?;
                    let layout = dock.instance.layout();
                    let tile_rect = layout
                        .slots
                        .iter()
                        .find(|s| s.item_id == item_id)
                        .map(|s| asrbox_platform_api::PixelRect {
                            x: layout.window_rect.x + s.base_rect.x,
                            y: layout.window_rect.y + s.base_rect.y,
                            width: s.base_rect.width,
                            height: s.base_rect.height,
                        })
                        .unwrap_or_else(|| to_pixel_rect(layout.window_rect));
                    let info = (
                        item.target.clone(),
                        item.label.clone(),
                        cfg.focus_running_window,
                        tile_rect,
                    );
                    let eff = dock.instance.trigger_launch_bounce(item_id);
                    if dock.window_open && eff.needs_redraw {
                        if eff.animation_active && !dock.timer_active {
                            dock.timer_active = true;
                            let _ = host.set_animation_timer(id, true);
                        }
                        let _ = present_dock(host, dock, self.drag.overlay_for(dock), self.palette);
                    }
                    info
                }
            }
        };
        let is_folder = target.is_dir()
            && !target.extension().is_some_and(|e| {
                e.eq_ignore_ascii_case("exe")
                    || e.eq_ignore_ascii_case("lnk")
                    || e.eq_ignore_ascii_case("url")
            });
        let launch_target = if is_folder {
            match host.show_dock_folder_popup(id, &target, tile_screen_rect) {
                Ok(Some(chosen)) => chosen,
                Ok(None) => return None,
                Err(_) => target,
            }
        } else {
            target
        };
        match host.activate_or_launch_target(&launch_target, focus_running) {
            Ok(true) => Some(format!("dock[{id}] focus: {label}")),
            Ok(false) => Some(format!("dock[{id}] launch: {label}")),
            Err(err) => Some(format!("error: dock[{id}] launch {label}: {err}")),
        }
    }

    pub(crate) fn redraw_open_docks(&mut self, host: &mut dyn DockPlatform) {
        for dock in &self.docks {
            if dock.window_open {
                let _ = present_dock(host, dock, self.drag.overlay_for(dock), self.palette);
            }
        }
    }

    pub(crate) fn sync_running_timer(&mut self, host: &mut dyn DockPlatform) {
        let want = self
            .docks
            .iter()
            .any(|d| d.window_open && d.instance.config().show_running_indicator);
        if want != self.running_timer_on {
            self.running_timer_on = want;
            let _ = host.set_running_poll_timer(want);
        }
    }

    pub(crate) fn apply_dock_action(
        &mut self,
        host: &mut dyn DockPlatform,
        id: u64,
        action: DockMenuAction,
        safe_mode: &mut bool,
    ) -> String {
        match action {
            DockMenuAction::OpenSettings => return "open_settings".to_owned(),
            DockMenuAction::AddNewDock => {
                self.add_new_dock(host, safe_mode);
                return "menu: add new dock".to_owned();
            }
            DockMenuAction::RemoveDock => {
                self.remove_dock(host, id);
                return format!("menu: remove dock {id}");
            }
            DockMenuAction::OpenDockConfig => {
                return self.open_config_file(host).map_or_else(
                    |e| format!("error: open docks.cfg: {e}"),
                    |()| "menu: open docks.cfg".to_owned(),
                );
            }
            DockMenuAction::OpenItemFolder(item_id) => {
                let target = self
                    .docks
                    .iter()
                    .find(|d| d.instance.config().id == id)
                    .and_then(|d| {
                        d.instance
                            .config()
                            .items
                            .iter()
                            .find(|i| i.id == item_id && !i.is_separator())
                            .map(|i| i.target.clone())
                    });
                return match target {
                    Some(path) => host.open_item_location(&path).map_or_else(
                        |e| format!("error: dock[{id}] open folder: {e}"),
                        |()| format!("dock[{id}] open folder: {}", path.display()),
                    ),
                    None => format!("dock[{id}] open folder: item #{item_id} not found"),
                };
            }
            DockMenuAction::ReloadDockConfig => {
                return self.reload_from_disk(host, *safe_mode).map_or_else(
                    |e| format!("error: reload docks.cfg: {e}"),
                    |n| format!("menu: reload docks.cfg ({n})"),
                );
            }
            _ => {}
        }
        let picked = matches!(action, DockMenuAction::ChooseItemIcon(_))
            .then(|| host.pick_icon_file().ok().flatten())
            .flatten();
        let removed_item = matches!(action, DockMenuAction::RemoveItem(_));
        if let Some(dock) = self.docks.iter_mut().find(|d| d.instance.config().id == id) {
            let mut cfg = dock.instance.config().clone();
            if apply_menu_action(&mut cfg, action, picked)
                && update_runtime_in_place(dock, cfg, self.work_area, &self.icon_provider, host)
                    .is_ok()
            {
                if dock.timer_active {
                    dock.timer_active = false;
                    let _ = host.set_animation_timer(id, false);
                }
                if dock.window_open {
                    let layer = dock.instance.config().layer;
                    if dock.applied_layer != Some(layer) {
                        let _ = host.set_dock_layer(id, layer);
                        dock.applied_layer = Some(layer);
                    }
                    let _ = present_dock(host, dock, self.drag.overlay_for(dock), self.palette);
                }
                self.persist();
                if removed_item {
                    self.cleanup_orphaned_shortcuts();
                }
                self.sync_running_timer(host);
                self.refresh_running_apps(host);
            }
        }
        format!("dock[{id}] menu: {action:?}")
    }

    pub(crate) fn cleanup_orphaned_shortcuts(&self) {
        let cfgs: Vec<DockConfig> = self
            .docks
            .iter()
            .map(|d| d.instance.config().clone())
            .collect();
        let dir = shortcuts_dir_for_config(self.config_path());
        cleanup_unused_shortcuts(&cfgs, &dir);
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

    pub fn persist(&self) {
        let cfgs: Vec<DockConfig> = self
            .docks
            .iter()
            .map(|d| d.instance.config().clone())
            .collect();
        let _ = save_docks_to_file(self.config_path(), &cfgs, self.app_version);
    }
}

/// Updates an existing `DockRuntime` in place without destroying its HWND or re-extracting
/// unchanged item icons, preventing any visual flicker during live slider drag in Settings.
pub(crate) fn update_runtime_in_place<I: IconProvider>(
    dock: &mut DockRuntime,
    cfg: DockConfig,
    work_area: Rect,
    provider: &I,
    host: &dyn DockPlatform,
) -> Result<(), String> {
    let old_items: HashMap<u64, _> = dock
        .instance
        .config()
        .items
        .iter()
        .filter(|i| !i.is_separator())
        .map(|i| (i.id, (i.target.clone(), i.icon.clone())))
        .collect();
    let rebuilt = DockInstance::new(cfg, work_area).map_err(|e| e.to_string())?;
    let mut next_icons = HashMap::new();
    let mut next_exes = HashMap::new();
    let mut next_running = HashSet::new();
    for item in &rebuilt.config().items {
        if item.is_separator() {
            continue;
        }
        let same_sig = old_items
            .get(&item.id)
            .is_some_and(|(t, c)| t == &item.target && c == &item.icon);
        if same_sig {
            if let Some(img) = dock.icons.remove(&item.id) {
                next_icons.insert(item.id, img);
            } else if let Ok(img) = provider.load_icon(&icon_request_for_item(item), 128) {
                next_icons.insert(item.id, img);
            }
            if let Some(exe) = dock.resolved_exes.remove(&item.id) {
                next_exes.insert(item.id, exe);
            }
            if dock.running_items.contains(&item.id) {
                next_running.insert(item.id);
            }
        } else {
            if let Ok(img) = provider.load_icon(&icon_request_for_item(item), 128) {
                next_icons.insert(item.id, img);
            }
            if let Some(exe) = host.resolve_target_exe_name(&item.target) {
                next_exes.insert(item.id, exe);
            }
        }
    }
    dock.icons = next_icons;
    dock.resolved_exes = next_exes;
    dock.running_items = next_running;
    dock.instance = rebuilt;
    Ok(())
}

pub(crate) fn reload_runtimes_in_place<I: IconProvider>(
    existing: Vec<DockRuntime>,
    configs: Vec<DockConfig>,
    work_area: Rect,
    icons: &I,
    host: &mut dyn DockPlatform,
) -> Result<Vec<DockRuntime>, String> {
    let mut old_by_id: HashMap<u64, DockRuntime> = existing
        .into_iter()
        .map(|d| (d.instance.config().id, d))
        .collect();
    let mut next = Vec::with_capacity(configs.len());
    for cfg in configs {
        if let Some(mut dock) = old_by_id.remove(&cfg.id) {
            let dock_id = cfg.id;
            update_runtime_in_place(&mut dock, cfg, work_area, icons, host)?;
            if dock.timer_active {
                dock.timer_active = false;
                let _ = host.set_animation_timer(dock_id, false);
            }
            next.push(dock);
        } else {
            let instance = DockInstance::new(cfg, work_area).map_err(|e| e.to_string())?;
            next.push(DockRuntime {
                icons: load_dock_icons(icons, instance.config()),
                resolved_exes: resolve_dock_exes(host, instance.config()),
                running_items: HashSet::new(),
                instance,
                window_open: false,
                timer_active: false,
                applied_layer: None,
            });
        }
    }
    for (_, removed) in old_by_id {
        if removed.window_open {
            let _ = host.destroy_dock_window(removed.instance.config().id);
        }
    }
    Ok(next)
}

pub(crate) fn build_runtimes<I: IconProvider>(
    configs: Vec<DockConfig>,
    work_area: Rect,
    icons: &I,
    host: &dyn DockPlatform,
) -> Result<Vec<DockRuntime>, String> {
    configs
        .into_iter()
        .map(|cfg| {
            let instance = DockInstance::new(cfg, work_area).map_err(|e| e.to_string())?;
            Ok(DockRuntime {
                icons: load_dock_icons(icons, instance.config()),
                resolved_exes: resolve_dock_exes(host, instance.config()),
                running_items: HashSet::new(),
                instance,
                window_open: false,
                timer_active: false,
                applied_layer: None,
            })
        })
        .collect()
}

pub(crate) fn resolve_dock_exes(host: &dyn DockPlatform, cfg: &DockConfig) -> HashMap<u64, String> {
    cfg.items
        .iter()
        .filter(|i| !i.is_separator())
        .filter_map(|i| host.resolve_target_exe_name(&i.target).map(|e| (i.id, e)))
        .collect()
}

pub(crate) fn load_dock_icons<I: IconProvider>(
    provider: &I,
    cfg: &DockConfig,
) -> HashMap<u64, RgbaImage> {
    cfg.items
        .iter()
        .filter(|i| !i.is_separator())
        .filter_map(|i| {
            provider
                .load_icon(&icon_request_for_item(i), 128)
                .ok()
                .map(|img| (i.id, img))
        })
        .collect()
}

pub(crate) fn present_dock(
    host: &mut dyn DockPlatform,
    dock: &DockRuntime,
    mut overlay: DockRenderOverlay,
    palette: FluentPalette,
) -> Result<(), String> {
    overlay.palette = palette;
    if overlay.dragged_item_id.is_none() {
        overlay.hovered_item_id = dock.instance.hovered_item();
    }
    let layout = dock.instance.layout();
    let frame = render_dock_widget_frame(
        dock.instance.config(),
        layout,
        &dock.icons,
        &dock.running_items,
        overlay,
    );
    host.present_dock_widget_frame(
        dock.instance.config().id,
        to_pixel_rect(layout.window_rect),
        &frame,
    )
}
