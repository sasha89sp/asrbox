#![cfg(windows)]

use std::path::PathBuf;

use asrbox_core::ModuleId;
use asrbox_mod_debug::LogLevel;
use asrbox_mod_dock::{load_docks_from_file, save_docks_to_file, DockConfig, DockItem};
use asrbox_mod_settings::SettingsAction;
use asrbox_platform_api::{DockPlatform, IconProvider, ToolsPlatform, WidgetPlatform};

use crate::dock_shortcuts::{localize_shortcut_path, shortcuts_dir_for_config};
use crate::slint_dispatch_tabs::idx_to_layer;
use crate::{AppController, APP_VERSION};

impl<I: IconProvider> AppController<I> {
    pub(crate) fn dispatch_dock_cmd<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        act: &str,
        arg: &str,
    ) -> Option<(bool, bool)> {
        if act == "add_dock" {
            self.apply_settings_action(host, SettingsAction::AddDock);
            return Some((true, false));
        }
        if act == "open_docks_cfg" {
            self.apply_settings_action(host, SettingsAction::OpenDocksConfig);
            return Some((false, false));
        }
        if !act.starts_with("dock_") && act != "remove_dock" {
            return None;
        }
        let (idx_s, rest) = arg.split_once('|').unwrap_or((arg, ""));
        let dock_idx = idx_s.parse::<usize>().unwrap_or(0);
        let docks = load_docks_from_file(&self.paths.docks_cfg).unwrap_or_default();
        let dock_id = docks
            .get(dock_idx)
            .or_else(|| docks.first())
            .map_or_else(|| "1".to_owned(), |d| d.id.to_string());
        if self.dispatch_dock_direct(host, act, &dock_id, rest) {
            return Some((true, false));
        }
        let action = decode_dock_action(act, dock_id, rest)?;
        self.apply_settings_action(host, action);
        Some((true, false))
    }

    fn dispatch_dock_direct<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        act: &str,
        dock_id: &str,
        rest: &str,
    ) -> bool {
        match act {
            "dock_add_files" => {
                let picked = host.pick_sync_files().unwrap_or_default();
                self.append_dock_paths(host, dock_id, picked);
                true
            }
            "dock_add_folder" => {
                let picked: Vec<PathBuf> =
                    host.pick_sync_folder().ok().flatten().into_iter().collect();
                self.append_dock_paths(host, dock_id, picked);
                true
            }
            "dock_rename_item" => {
                if let Some((id_s, label)) = rest.split_once('|') {
                    if let Ok(item_id) = id_s.parse::<u64>() {
                        self.rename_dock_item_direct(host, dock_id, item_id, label);
                    }
                }
                true
            }
            "dock_icon_frame" => {
                if let Ok(px) = rest.parse::<u16>() {
                    self.set_dock_icon_frame_direct(host, dock_id, px.min(24));
                }
                true
            }
            "dock_menu_vis" => {
                if let Some((m_idx, val_s)) = rest.split_once('|') {
                    let on = val_s == "1";
                    let mut vis = self.menus_cfg.dock;
                    match m_idx {
                        "0" => vis.show_item_actions = on,
                        "1" => vis.show_add_separator = on,
                        "2" => vis.show_edge_and_layer = on,
                        "3" => vis.show_dock_management = on,
                        _ => {}
                    }
                    self.apply_settings_action(host, SettingsAction::SetDockMenuVis(vis));
                }
                true
            }
            _ => false,
        }
    }

    fn append_dock_paths<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        dock_id: &str,
        picked: Vec<PathBuf>,
    ) {
        if picked.is_empty() {
            return;
        }
        let Ok(id) = dock_id.parse::<u64>() else {
            return;
        };
        let mut cfgs = load_docks_from_file(&self.paths.docks_cfg).unwrap_or_default();
        if cfgs.is_empty() {
            cfgs.push(DockConfig::new(1, "Док 1"));
        }
        let sc_dir = shortcuts_dir_for_config(&self.paths.docks_cfg);
        let mut added = 0usize;
        if let Some(c) = cfgs.iter_mut().find(|d| d.id == id) {
            for p in picked {
                let target = localize_shortcut_path(&p, &sc_dir);
                let nid = c.next_item_id();
                if let Ok(item) = DockItem::from_dropped_path(nid, target) {
                    c.items.push(item);
                    added += 1;
                }
            }
        }
        if added > 0 && save_docks_to_file(&self.paths.docks_cfg, &cfgs, APP_VERSION).is_ok() {
            self.reload_module_config(host, ModuleId::Dock);
            self.debug.log(
                host.local_time_now(),
                LogLevel::Info,
                "dock",
                format!("Добавлено элементов в док {id}: {added}"),
            );
        }
    }

    fn rename_dock_item_direct<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        dock_id: &str,
        item_id: u64,
        new_label: &str,
    ) {
        let trimmed = new_label.trim();
        let Ok(id) = dock_id.parse::<u64>() else {
            return;
        };
        if trimmed.is_empty() {
            return;
        }
        let mut cfgs = load_docks_from_file(&self.paths.docks_cfg).unwrap_or_default();
        if let Some(it) = cfgs
            .iter_mut()
            .find(|c| c.id == id)
            .and_then(|c| c.items.iter_mut().find(|i| i.id == item_id))
        {
            it.label = trimmed.to_owned();
            if save_docks_to_file(&self.paths.docks_cfg, &cfgs, APP_VERSION).is_ok() {
                self.reload_module_config(host, ModuleId::Dock);
            }
        }
    }

    fn set_dock_icon_frame_direct<H: DockPlatform + WidgetPlatform + ToolsPlatform>(
        &mut self,
        host: &mut H,
        dock_id: &str,
        frame_px: u16,
    ) {
        let Ok(id) = dock_id.parse::<u64>() else {
            return;
        };
        let mut docks = load_docks_from_file(&self.paths.docks_cfg).unwrap_or_default();
        if let Some(dock) = docks.iter_mut().find(|d| d.id == id) {
            dock.icon_frame_px = frame_px.min(24);
            if save_docks_to_file(&self.paths.docks_cfg, &docks, APP_VERSION).is_ok() {
                self.reload_module_config(host, ModuleId::Dock);
            }
        }
    }
}

fn decode_dock_action(act: &str, dock_id: String, rest: &str) -> Option<SettingsAction> {
    Some(match act {
        "remove_dock" => SettingsAction::RemoveDock(dock_id),
        "dock_edge" => SettingsAction::SetDockEdge {
            dock_id,
            edge_index: rest.parse::<u8>().unwrap_or(0),
        },
        "dock_layer" => SettingsAction::SetDockLayer {
            dock_id,
            layer: idx_to_layer(rest),
        },
        "dock_label_mode" => SettingsAction::SetDockLabelMode {
            dock_id,
            mode_index: rest.parse::<u8>().unwrap_or(0),
        },
        "dock_icon_size" => SettingsAction::SetDockIconSize {
            dock_id,
            size_px: rest.parse::<u32>().unwrap_or(44),
        },
        "dock_opacity" => SettingsAction::SetDockOpacity {
            dock_id,
            opacity: rest.parse::<u8>().unwrap_or(82),
        },
        "dock_edge_margin" => SettingsAction::SetDockEdgeMargin {
            dock_id,
            margin_px: rest.parse::<u16>().unwrap_or(10),
        },
        "dock_corner_radius" => SettingsAction::SetDockCornerRadius {
            dock_id,
            radius_px: rest.parse::<u16>().unwrap_or(18),
        },
        "dock_blur" => SettingsAction::SetDockBlur {
            dock_id,
            enabled: rest == "1",
        },
        "dock_locked" => SettingsAction::SetDockLocked {
            dock_id,
            locked: rest == "1",
        },
        "dock_lock_items" => SettingsAction::SetDockLockItems {
            dock_id,
            locked: rest == "1",
        },
        "dock_show_running" => SettingsAction::SetDockShowRunning {
            dock_id,
            enabled: rest == "1",
        },
        "dock_focus_running" => SettingsAction::SetDockFocusRunning {
            dock_id,
            enabled: rest == "1",
        },
        "dock_launch_anim" => SettingsAction::SetDockLaunchAnimation {
            dock_id,
            enabled: rest == "1",
        },
        "dock_add_sep" => SettingsAction::AddDockSeparator(dock_id),
        "dock_reset_item" => SettingsAction::ResetDockItemLabel {
            dock_id,
            item_id: rest.parse::<u64>().unwrap_or(0),
        },
        "dock_move_item" => {
            let (id_s, d_s) = rest.split_once('|')?;
            SettingsAction::MoveDockItem {
                dock_id,
                item_id: id_s.parse::<u64>().ok()?,
                delta: d_s.parse::<i8>().ok()?,
            }
        }
        "dock_open_item_folder" => SettingsAction::OpenDockItemFolder {
            dock_id,
            item_id: rest.parse::<u64>().unwrap_or(0),
        },
        "dock_remove_item" => SettingsAction::RemoveDockItem {
            dock_id,
            item_id: rest.parse::<u64>().unwrap_or(0),
        },
        _ => return None,
    })
}
