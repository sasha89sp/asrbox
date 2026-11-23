pub mod config;
pub mod layout;
mod layout_ops;
pub mod render;
mod render_ops;
pub mod state;
pub mod storage;

use std::path::PathBuf;

use asrbox_platform_api::{
    DockMenuAction, DockMenuState, IconRequest, PixelPoint, PixelRect, WidgetLayer,
};

pub use config::{
    DockAlignment, DockConfig, DockConfigError, DockEdge, DockId, DockItem, DockItemKind,
    DockRegistry, HoverZoomConfig, HoverZoomMode, IconSource, DEFAULT_DOCK_OPACITY,
    DRAG_HANDLE_SLOT_ID, MAX_DOCK_OPACITY, MIN_DOCK_OPACITY, SETTINGS_SLOT_ID,
};
pub use layout::{
    apply_dragged_window_pos, compute_layout, drop_insert_index, hit_test_item, hit_test_slot,
    DockLayout, DockSlotLayout, HoverVisualState, Point, Rect, SlotKind,
};
pub use render::{render_dock_frame, DockRenderOverlay};
pub use state::{DockEvent, DockInstance, DockMetrics, DockStepEffect};
pub use storage::{load_docks_from_file, parse_docks, save_docks_to_file, serialize_docks};

pub fn icon_request_for_item(item: &DockItem) -> IconRequest {
    match &item.icon {
        IconSource::AutoShell => IconRequest::AutoShell(item.target.clone()),
        IconSource::CustomFile(path) => IconRequest::CustomFile(path.clone()),
        IconSource::ExeResource { path, index } => IconRequest::ExeResource {
            path: path.clone(),
            index: *index,
        },
    }
}

pub fn build_menu_state(dock: &DockInstance, pt: Point, can_remove: bool) -> DockMenuState {
    let cfg = dock.config();
    let edge_code = match cfg.edge {
        DockEdge::Left => 0,
        DockEdge::Right => 1,
        DockEdge::Top => 2,
        DockEdge::Bottom => 3,
    };
    let (hovered_item_id, hovered_is_separator) = match hit_test_slot(dock.layout(), pt) {
        Some(slot) if slot.kind == SlotKind::Separator => (Some(slot.item_id), true),
        Some(slot) if slot.kind == SlotKind::Shortcut => (Some(slot.item_id), false),
        _ => (None, false),
    };
    DockMenuState {
        edge_code,
        layer: cfg.layer,
        is_centered: matches!(cfg.alignment, DockAlignment::Center) && cfg.free_pos.is_none(),
        icon_size_px: cfg.icon_size_px,
        opacity_percent: cfg.opacity_percent,
        locked: cfg.locked,
        hover_zoom_enabled: cfg.hover_zoom.enabled,
        show_running_indicator: cfg.show_running_indicator,
        focus_running_window: cfg.focus_running_window,
        hovered_item_id,
        hovered_is_separator,
        show_settings_button: cfg.show_settings_button,
        can_remove_dock: can_remove,
    }
}

pub fn apply_menu_action(
    cfg: &mut DockConfig,
    action: DockMenuAction,
    picked_icon: Option<PathBuf>,
) -> bool {
    match action {
        DockMenuAction::SetEdgeLeft => {
            cfg.edge = DockEdge::Left;
            cfg.free_pos = None;
        }
        DockMenuAction::SetEdgeRight => {
            cfg.edge = DockEdge::Right;
            cfg.free_pos = None;
        }
        DockMenuAction::SetEdgeTop => {
            cfg.edge = DockEdge::Top;
            cfg.free_pos = None;
        }
        DockMenuAction::SetEdgeBottom => {
            cfg.edge = DockEdge::Bottom;
            cfg.free_pos = None;
        }
        DockMenuAction::SetLayerWallpaper => cfg.layer = WidgetLayer::Wallpaper,
        DockMenuAction::SetLayerNormal => cfg.layer = WidgetLayer::Normal,
        DockMenuAction::SetLayerTopmost => cfg.layer = WidgetLayer::Topmost,
        DockMenuAction::AlignCenter => {
            cfg.alignment = DockAlignment::Center;
            cfg.free_pos = None;
        }
        DockMenuAction::ToggleLock => cfg.locked = !cfg.locked,
        DockMenuAction::SetOpacity(pct) => {
            cfg.opacity_percent = pct.clamp(MIN_DOCK_OPACITY, MAX_DOCK_OPACITY);
        }
        DockMenuAction::IconSizeSmall => cfg.icon_size_px = 40,
        DockMenuAction::IconSizeMedium => cfg.icon_size_px = 48,
        DockMenuAction::IconSizeLarge => cfg.icon_size_px = 64,
        DockMenuAction::ToggleHoverZoom => cfg.hover_zoom.enabled = !cfg.hover_zoom.enabled,
        DockMenuAction::ToggleRunningIndicator => {
            cfg.show_running_indicator = !cfg.show_running_indicator;
        }
        DockMenuAction::ToggleFocusRunning => {
            cfg.focus_running_window = !cfg.focus_running_window;
        }
        DockMenuAction::ChooseItemIcon(id) => {
            let Some(path) = picked_icon else {
                return false;
            };
            if let Some(item) = cfg.items.iter_mut().find(|i| i.id == id) {
                item.icon = IconSource::CustomFile(path);
            }
        }
        DockMenuAction::ResetItemIcon(id) => {
            if let Some(item) = cfg.items.iter_mut().find(|i| i.id == id) {
                item.icon = IconSource::AutoShell;
            }
        }
        DockMenuAction::RemoveItem(id) => cfg.items.retain(|i| i.id != id),
        DockMenuAction::AddSeparator => {
            let id = cfg.next_item_id();
            cfg.items.push(DockItem::separator(id));
        }
        DockMenuAction::ToggleSettingsButton => {
            cfg.show_settings_button = !cfg.show_settings_button;
        }
        DockMenuAction::OpenSettings
        | DockMenuAction::OpenDockConfig
        | DockMenuAction::ReloadDockConfig
        | DockMenuAction::AddNewDock
        | DockMenuAction::RemoveDock => return false,
    }
    true
}

pub fn to_dock_rect(r: PixelRect) -> Rect {
    Rect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
    }
}

pub fn to_pixel_rect(r: Rect) -> PixelRect {
    PixelRect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
    }
}

pub fn to_dock_point(p: PixelPoint) -> Point {
    Point { x: p.x, y: p.y }
}
