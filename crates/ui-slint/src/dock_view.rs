use std::collections::HashSet;
use std::rc::Rc;

use asrbox_mod_dock::{DockAlignment, DockConfig, DockEdge, DockItem, DockLabelMode, IconSource};
use asrbox_platform_api::{
    DockMenuAction, DockMenuState, DockMenuVisibility, PixelRect, WidgetLayer,
};
use slint::{ModelRc, SharedString, VecModel};

use crate::dock_icons::{is_folder_target, resolve_dock_target, DockIconCache};
use crate::generated::ui::{DockWindow, SlintDockIconRow};
use crate::win_os::{layer_to_idx, resolve_exe_name};

fn compute_dock_size(cfg: &DockConfig) -> (u32, u32) {
    let icon = u32::from(cfg.icon_size_px.clamp(24, 128));
    let frame2 = u32::from(cfg.icon_frame_px.min(24)) * 2;
    let pad = u32::from(cfg.padding_px.clamp(4, 48));
    let gap = u32::from(cfg.item_spacing_px.min(48));
    let lm = cfg.label_mode.to_index();
    let tile_w = if lm == 2 {
        icon + frame2 + 88
    } else {
        icon + frame2
    };
    let tile_h = if lm == 1 {
        icon + frame2 + 14
    } else {
        icon + frame2
    };
    let horiz = cfg.edge.is_horizontal();
    let mut main_len = pad * 2;
    let mut count = 0u32;
    if !cfg.locked {
        main_len += 16;
        count += 1;
    }
    for it in &cfg.items {
        main_len += if it.is_separator() {
            10
        } else if horiz {
            tile_w
        } else {
            tile_h
        };
        count += 1;
    }
    if cfg.show_settings_button {
        main_len += icon + frame2;
        count += 1;
    }
    if count > 1 {
        main_len += gap * (count - 1);
    }
    let cross_len = (if horiz { tile_h } else { tile_w }) + pad * 2;
    if horiz {
        (main_len.max(88), cross_len.max(56))
    } else {
        (cross_len.max(56), main_len.max(88))
    }
}

pub fn compute_dock_rect(cfg: &DockConfig, work_area: PixelRect) -> (i32, i32, u32, u32) {
    let (w, h) = compute_dock_size(cfg);
    if let Some((fx, fy)) = cfg.free_pos {
        let min_x = work_area.x - (w as i32 / 2);
        let max_x = work_area.x + work_area.width as i32 - 32;
        let min_y = work_area.y;
        let max_y = work_area.y + work_area.height as i32 - 32;
        return (fx.clamp(min_x, max_x), fy.clamp(min_y, max_y), w, h);
    }
    let margin = i32::from(cfg.edge_margin_px);
    let offset = match cfg.alignment {
        DockAlignment::Center => 0,
        DockAlignment::Offset(o) => o,
    };
    let (x, y) = match cfg.edge {
        DockEdge::Bottom => (
            work_area.x + (work_area.width as i32 - w as i32) / 2 + offset,
            work_area.y + work_area.height as i32 - h as i32 - margin,
        ),
        DockEdge::Top => (
            work_area.x + (work_area.width as i32 - w as i32) / 2 + offset,
            work_area.y + margin,
        ),
        DockEdge::Left => (
            work_area.x + margin,
            work_area.y + (work_area.height as i32 - h as i32) / 2 + offset,
        ),
        DockEdge::Right => (
            work_area.x + work_area.width as i32 - w as i32 - margin,
            work_area.y + (work_area.height as i32 - h as i32) / 2 + offset,
        ),
    };
    (x, y, w, h)
}

pub fn sync_dock_window(
    ui: &DockWindow,
    cfg: &DockConfig,
    running_exes: &HashSet<String>,
    icons: &mut DockIconCache,
    can_remove: bool,
) {
    ui.set_dock_name(SharedString::from(cfg.name.as_str()));
    ui.set_is_horizontal(cfg.edge.is_horizontal());
    ui.set_icon_size(f32::from(cfg.icon_size_px.clamp(24, 128)));
    ui.set_icon_frame_px(f32::from(cfg.icon_frame_px.min(24)));
    ui.set_item_spacing(f32::from(cfg.item_spacing_px.min(48)));
    ui.set_dock_padding(f32::from(cfg.padding_px.clamp(4, 48)));
    ui.set_corner_radius(f32::from(cfg.corner_radius_px.clamp(4, 32)));
    ui.set_opacity_pct(i32::from(cfg.opacity_percent));
    ui.set_edge_index(i32::from(edge_to_code(cfg.edge)));
    ui.set_layer_index(layer_to_idx(cfg.layer));
    ui.set_label_mode(i32::from(cfg.label_mode.to_index()));
    ui.set_locked(cfg.locked);
    ui.set_lock_items(cfg.lock_items);
    ui.set_hover_zoom(cfg.hover_zoom.enabled);
    ui.set_show_running_dot(cfg.show_running_indicator);
    ui.set_focus_running(cfg.focus_running_window);
    ui.set_launch_anim(cfg.launch_animation_enabled);
    ui.set_show_settings_button(cfg.show_settings_button);
    ui.set_can_remove_dock(can_remove);
    let rows = build_dock_icon_rows(cfg, running_exes, icons);
    ui.set_items(ModelRc::from(Rc::new(VecModel::from(rows))));
}

fn edge_to_code(edge: DockEdge) -> u8 {
    match edge {
        DockEdge::Left => 0,
        DockEdge::Right => 1,
        DockEdge::Top => 2,
        DockEdge::Bottom => 3,
    }
}

pub fn build_dock_menu_state(
    cfg: &DockConfig,
    vis: DockMenuVisibility,
    hovered_item_id: Option<u64>,
    hovered_is_separator: bool,
    can_remove_dock: bool,
) -> DockMenuState {
    DockMenuState {
        edge_code: edge_to_code(cfg.edge),
        layer: cfg.layer,
        label_mode_code: cfg.label_mode.to_index(),
        is_centered: cfg.free_pos.is_none() && matches!(cfg.alignment, DockAlignment::Center),
        icon_size_px: cfg.icon_size_px,
        opacity_percent: cfg.opacity_percent,
        locked: cfg.locked,
        lock_items: cfg.lock_items,
        hover_zoom_enabled: cfg.hover_zoom.enabled,
        show_running_indicator: cfg.show_running_indicator,
        focus_running_window: cfg.focus_running_window,
        launch_animation_enabled: cfg.launch_animation_enabled,
        hovered_item_id,
        hovered_is_separator,
        show_settings_button: cfg.show_settings_button,
        can_remove_dock,
        menu_vis: vis,
    }
}

fn build_dock_icon_rows(
    cfg: &DockConfig,
    running_exes: &HashSet<String>,
    icons: &mut DockIconCache,
) -> Vec<SlintDockIconRow> {
    let base_dir = icons.base_dir().to_path_buf();
    cfg.items
        .iter()
        .map(|it| {
            let resolved = resolve_dock_target(&it.target, &base_dir);
            let img = icons.get_or_load(it);
            let is_running = !it.is_separator()
                && cfg.show_running_indicator
                && resolve_exe_name(&resolved).is_some_and(|exe| running_exes.contains(&exe));
            SlintDockIconRow {
                id: it.id as i32,
                label: SharedString::from(it.label.as_str()),
                is_separator: it.is_separator(),
                is_folder: !it.is_separator() && is_folder_target(&resolved),
                is_running,
                has_icon: img.is_some(),
                icon_img: img.unwrap_or_default(),
            }
        })
        .collect()
}

pub fn apply_native_dock_action(
    cfg: &mut DockConfig,
    icons: &mut DockIconCache,
    action: DockMenuAction,
) -> bool {
    match action {
        DockMenuAction::SetEdgeLeft => set_edge(cfg, DockEdge::Left),
        DockMenuAction::SetEdgeRight => set_edge(cfg, DockEdge::Right),
        DockMenuAction::SetEdgeTop => set_edge(cfg, DockEdge::Top),
        DockMenuAction::SetEdgeBottom => set_edge(cfg, DockEdge::Bottom),
        DockMenuAction::AlignCenter => {
            cfg.free_pos = None;
            cfg.alignment = DockAlignment::Center;
            true
        }
        DockMenuAction::SetLayerWallpaper => set_layer(cfg, WidgetLayer::Wallpaper),
        DockMenuAction::SetLayerNormal => set_layer(cfg, WidgetLayer::Normal),
        DockMenuAction::SetLayerTopmost => set_layer(cfg, WidgetLayer::Topmost),
        DockMenuAction::SetLabelMode(idx) => {
            cfg.label_mode = DockLabelMode::from_index(idx);
            true
        }
        DockMenuAction::SetOpacity(pct) => {
            cfg.opacity_percent = pct.clamp(20, 100);
            true
        }
        DockMenuAction::IconSizeSmall => set_icon_size(cfg, 40),
        DockMenuAction::IconSizeMedium => set_icon_size(cfg, 48),
        DockMenuAction::IconSizeLarge => set_icon_size(cfg, 64),
        DockMenuAction::ToggleLock => {
            cfg.locked = !cfg.locked;
            true
        }
        DockMenuAction::ToggleLockItems => {
            cfg.lock_items = !cfg.lock_items;
            true
        }
        DockMenuAction::ToggleHoverZoom => {
            cfg.hover_zoom.enabled = !cfg.hover_zoom.enabled;
            true
        }
        DockMenuAction::ToggleRunningIndicator => {
            cfg.show_running_indicator = !cfg.show_running_indicator;
            true
        }
        DockMenuAction::ToggleFocusRunning => {
            cfg.focus_running_window = !cfg.focus_running_window;
            true
        }
        DockMenuAction::ToggleLaunchAnimation => {
            cfg.launch_animation_enabled = !cfg.launch_animation_enabled;
            true
        }
        DockMenuAction::ToggleSettingsButton => {
            cfg.show_settings_button = !cfg.show_settings_button;
            true
        }
        DockMenuAction::ResetItemIcon(id) => reset_item_icon(cfg, icons, id),
        DockMenuAction::RemoveItem(id) if !cfg.lock_items => {
            let prev = cfg.items.len();
            cfg.items.retain(|i| i.id != id);
            cfg.items.len() != prev
        }
        DockMenuAction::AddSeparator if !cfg.lock_items => {
            let nid = cfg.next_item_id();
            cfg.items.push(DockItem::separator(nid));
            true
        }
        _ => false,
    }
}

fn set_edge(cfg: &mut DockConfig, edge: DockEdge) -> bool {
    cfg.edge = edge;
    cfg.free_pos = None;
    true
}

fn set_layer(cfg: &mut DockConfig, layer: WidgetLayer) -> bool {
    cfg.layer = layer;
    true
}

fn set_icon_size(cfg: &mut DockConfig, px: u16) -> bool {
    cfg.icon_size_px = px;
    true
}

fn reset_item_icon(cfg: &mut DockConfig, icons: &mut DockIconCache, id: u64) -> bool {
    let Some(it) = cfg.items.iter_mut().find(|i| i.id == id) else {
        return false;
    };
    it.icon = IconSource::AutoShell;
    icons.invalidate_item(it);
    true
}

pub fn add_dock_config(configs: &mut Vec<DockConfig>) {
    let next_id = configs.iter().map(|c| c.id).max().unwrap_or(0) + 1;
    let mut cfg = DockConfig::new(next_id, format!("Док {next_id}"));
    cfg.edge = match next_id % 4 {
        0 => DockEdge::Right,
        2 => DockEdge::Top,
        3 => DockEdge::Left,
        _ => DockEdge::Bottom,
    };
    configs.push(cfg);
}

pub fn toggle_dock_config(configs: &mut [DockConfig], dock_id: u64) -> bool {
    configs
        .iter_mut()
        .find(|c| c.id == dock_id)
        .is_some_and(|cfg| {
            cfg.enabled = !cfg.enabled;
            true
        })
}

pub fn remove_dock_config(configs: &mut Vec<DockConfig>, dock_id: u64) -> bool {
    if configs.len() <= 1 {
        return false;
    }
    configs.retain(|c| c.id != dock_id);
    true
}

pub fn set_custom_icon_config(
    configs: &mut [DockConfig],
    icons: &mut DockIconCache,
    dock_id: u64,
    item_id: u64,
    picked: std::path::PathBuf,
) -> bool {
    let Some(it) = configs
        .iter_mut()
        .find(|c| c.id == dock_id)
        .and_then(|c| c.items.iter_mut().find(|i| i.id == item_id))
    else {
        return false;
    };
    it.icon = IconSource::CustomFile(picked);
    icons.invalidate_item(it);
    true
}
