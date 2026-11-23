#![cfg(windows)]

use std::collections::{HashMap, HashSet};

use asrbox_mod_dock::{
    apply_menu_action, icon_request_for_item, render_dock_frame, to_pixel_rect, DockConfig,
    DockInstance, DockRenderOverlay, Rect, SlotKind,
};
use asrbox_platform_api::{DockMenuAction, DockPlatform, IconProvider, RgbaImage};

use crate::docks::{DockManager, DockRuntime};

impl<I: IconProvider> DockManager<I> {
    pub(crate) fn handle_slot_click(
        &self,
        host: &dyn DockPlatform,
        id: u64,
        kind: SlotKind,
        item_id: u64,
    ) -> Option<String> {
        let dock = self.docks.iter().find(|d| d.instance.config().id == id)?;
        match kind {
            SlotKind::SettingsButton => Some("open_settings".to_owned()),
            SlotKind::Shortcut => {
                let cfg = dock.instance.config();
                let item = cfg.items.iter().find(|i| i.id == item_id)?;
                match host.activate_or_launch_target(&item.target, cfg.focus_running_window) {
                    Ok(true) => Some(format!("dock[{id}] focus: {}", item.label)),
                    Ok(false) => Some(format!("dock[{id}] launch: {}", item.label)),
                    Err(err) => Some(format!("error: dock[{id}] launch {}: {err}", item.label)),
                }
            }
            SlotKind::Separator | SlotKind::DragHandle => None,
        }
    }

    pub(crate) fn redraw_open_docks(&mut self, host: &mut dyn DockPlatform) {
        for dock in &self.docks {
            if dock.window_open {
                let _ = present_dock(host, dock, self.drag.overlay_for(dock));
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
        if let Some(dock) = self.docks.iter_mut().find(|d| d.instance.config().id == id) {
            let mut cfg = dock.instance.config().clone();
            if apply_menu_action(&mut cfg, action, picked) {
                if let Ok(rebuilt) = DockInstance::new(cfg, self.work_area) {
                    dock.icons = load_dock_icons(&self.icon_provider, rebuilt.config());
                    dock.resolved_exes = resolve_dock_exes(host, rebuilt.config());
                    dock.instance = rebuilt;
                    if dock.timer_active {
                        dock.timer_active = false;
                        let _ = host.set_animation_timer(id, false);
                    }
                    if dock.window_open {
                        let _ = present_dock(host, dock, self.drag.overlay_for(dock));
                    }
                    self.persist();
                    self.sync_running_timer(host);
                    self.refresh_running_apps(host);
                }
            }
        }
        format!("dock[{id}] menu: {action:?}")
    }
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
    overlay: DockRenderOverlay,
) -> Result<(), String> {
    let layout = dock.instance.layout();
    let frame = render_dock_frame(
        dock.instance.config(),
        layout,
        &dock.icons,
        &dock.running_items,
        overlay,
    );
    host.present_dock_frame(
        dock.instance.config().id,
        to_pixel_rect(layout.window_rect),
        &frame,
    )
}
