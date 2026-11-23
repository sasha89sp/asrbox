use crate::config::{DockAlignment, DockConfig, DockEdge, DRAG_HANDLE_SLOT_ID, SETTINGS_SLOT_ID};

pub const SEPARATOR_THICKNESS_PX: u32 = 14;
pub const DRAG_HANDLE_THICKNESS_PX: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn contains(self, point: Point) -> bool {
        let w = i32::try_from(self.width).unwrap_or(i32::MAX);
        let h = i32::try_from(self.height).unwrap_or(i32::MAX);
        point.x >= self.x
            && point.x < self.x.saturating_add(w)
            && point.y >= self.y
            && point.y < self.y.saturating_add(h)
    }

    pub fn expanded(self, pad: i32) -> Self {
        let extra = u32::try_from(pad.max(0)).unwrap_or(0).saturating_mul(2);
        Self {
            x: self.x.saturating_sub(pad),
            y: self.y.saturating_sub(pad),
            width: self.width.saturating_add(extra),
            height: self.height.saturating_add(extra),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotKind {
    DragHandle,
    Shortcut,
    Separator,
    SettingsButton,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HoverVisualState {
    pub item_id: Option<u64>,
    pub progress: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockSlotLayout {
    pub item_id: u64,
    pub kind: SlotKind,
    pub base_rect: Rect,
    pub visual_rect: Rect,
    pub scale: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockLayout {
    pub window_rect: Rect,
    pub bar_rect: Rect,
    pub slots: Vec<DockSlotLayout>,
}

pub fn compute_layout(config: &DockConfig, work_area: Rect, hover: HoverVisualState) -> DockLayout {
    let icon = u32::from(config.icon_size_px);
    let spacing = u32::from(config.item_spacing_px);
    let padding = u32::from(config.padding_px);

    let main_items_len = compute_main_items_len(config, icon, spacing);
    let bar_main = main_items_len.saturating_add(padding.saturating_mul(2));
    let bar_cross = icon.saturating_add(padding.saturating_mul(2));

    let max_scale = config.hover_zoom.effective_scale();
    let zoom_extra = ((icon as f32) * (max_scale - 1.0)).ceil().max(0.0) as u32;
    let side_pad = zoom_extra / 2;

    let (win_w, win_h, bar_rect) =
        build_window_and_bar(config.edge, bar_main, bar_cross, zoom_extra, side_pad);
    let window_rect = place_window(config, work_area, win_w, win_h);
    let slots = build_slots(config, bar_rect, max_scale, hover);
    DockLayout {
        window_rect,
        bar_rect,
        slots,
    }
}

pub fn hit_test_slot(layout: &DockLayout, local_point: Point) -> Option<DockSlotLayout> {
    layout
        .slots
        .iter()
        .copied()
        .find(|slot| slot.base_rect.contains(local_point))
}

pub fn hit_test_item(layout: &DockLayout, local_point: Point) -> Option<u64> {
    hit_test_slot(layout, local_point).map(|slot| slot.item_id)
}

pub use crate::layout_ops::{apply_dragged_window_pos, drop_insert_index};

fn compute_main_items_len(config: &DockConfig, icon: u32, spacing: u32) -> u32 {
    let has_handle = usize::from(!config.locked);
    let has_gear = usize::from(config.show_settings_button);
    let total_slots = has_handle + config.items.len() + has_gear;
    if total_slots == 0 {
        return icon;
    }
    let mut sum = if !config.locked {
        DRAG_HANDLE_THICKNESS_PX
    } else {
        0
    };
    for item in &config.items {
        let len = if item.is_separator() {
            SEPARATOR_THICKNESS_PX
        } else {
            icon
        };
        sum = sum.saturating_add(len);
    }
    if config.show_settings_button {
        sum = sum.saturating_add(icon);
    }
    let gaps = u32::try_from(total_slots.saturating_sub(1)).unwrap_or(0);
    sum.saturating_add(gaps.saturating_mul(spacing))
}

fn build_window_and_bar(
    edge: DockEdge,
    bar_main: u32,
    bar_cross: u32,
    zoom_extra: u32,
    side_pad: u32,
) -> (u32, u32, Rect) {
    let side = i32::try_from(side_pad).unwrap_or(0);
    let zoom = i32::try_from(zoom_extra).unwrap_or(0);
    let win_main = bar_main.saturating_add(side_pad.saturating_mul(2));
    let win_cross = bar_cross.saturating_add(zoom_extra);
    let (w, h, x, y, bw, bh) = match edge {
        DockEdge::Bottom => (win_main, win_cross, side, zoom, bar_main, bar_cross),
        DockEdge::Top => (win_main, win_cross, side, 0, bar_main, bar_cross),
        DockEdge::Left => (win_cross, win_main, 0, side, bar_cross, bar_main),
        DockEdge::Right => (win_cross, win_main, zoom, side, bar_cross, bar_main),
    };
    (
        w,
        h,
        Rect {
            x,
            y,
            width: bw,
            height: bh,
        },
    )
}

fn place_window(config: &DockConfig, work_area: Rect, win_w: u32, win_h: u32) -> Rect {
    let area_w = i32::try_from(work_area.width).unwrap_or(i32::MAX);
    let area_h = i32::try_from(work_area.height).unwrap_or(i32::MAX);
    let w_i32 = i32::try_from(win_w).unwrap_or(i32::MAX);
    let h_i32 = i32::try_from(win_h).unwrap_or(i32::MAX);
    if let Some((fx, fy)) = config.free_pos {
        let max_x = work_area
            .x
            .saturating_add(area_w.saturating_sub(w_i32).max(0));
        let max_y = work_area
            .y
            .saturating_add(area_h.saturating_sub(h_i32).max(0));
        return Rect {
            x: fx.clamp(work_area.x, max_x),
            y: fy.clamp(work_area.y, max_y),
            width: win_w,
            height: win_h,
        };
    }
    let margin = i32::from(config.edge_margin_px);
    let ax_x = align_axis(work_area.x, area_w, w_i32, config.alignment);
    let ax_y = align_axis(work_area.y, area_h, h_i32, config.alignment);
    let (x, y) = match config.edge {
        DockEdge::Bottom => (
            ax_x,
            work_area
                .y
                .saturating_add(area_h)
                .saturating_sub(h_i32)
                .saturating_sub(margin),
        ),
        DockEdge::Top => (ax_x, work_area.y.saturating_add(margin)),
        DockEdge::Left => (work_area.x.saturating_add(margin), ax_y),
        DockEdge::Right => (
            work_area
                .x
                .saturating_add(area_w)
                .saturating_sub(w_i32)
                .saturating_sub(margin),
            ax_y,
        ),
    };
    Rect {
        x,
        y,
        width: win_w,
        height: win_h,
    }
}

fn align_axis(origin: i32, area_len: i32, win_len: i32, alignment: DockAlignment) -> i32 {
    let max_offset = area_len.saturating_sub(win_len).max(0);
    let rel = match alignment {
        DockAlignment::Center => max_offset / 2,
        DockAlignment::Offset(px) => px.clamp(0, max_offset),
    };
    origin.saturating_add(rel)
}

struct SlotCtx {
    bar_rect: Rect,
    pad: i32,
    cross_len: u32,
    edge: DockEdge,
    max_scale: f32,
    hover: HoverVisualState,
    progress: f32,
}

impl SlotCtx {
    fn make(&self, item_id: u64, kind: SlotKind, main_len: u32, cursor: i32) -> DockSlotLayout {
        let base_rect = if self.edge.is_horizontal() {
            Rect {
                x: self
                    .bar_rect
                    .x
                    .saturating_add(self.pad)
                    .saturating_add(cursor),
                y: self.bar_rect.y.saturating_add(self.pad),
                width: main_len,
                height: self.cross_len,
            }
        } else {
            Rect {
                x: self.bar_rect.x.saturating_add(self.pad),
                y: self
                    .bar_rect
                    .y
                    .saturating_add(self.pad)
                    .saturating_add(cursor),
                width: self.cross_len,
                height: main_len,
            }
        };
        let can_zoom = !matches!(kind, SlotKind::Separator | SlotKind::DragHandle);
        let hovered = can_zoom
            && self.hover.item_id == Some(item_id)
            && self.progress > 0.0
            && self.max_scale > 1.0;
        let scale = if hovered {
            1.0 + (self.max_scale - 1.0) * self.progress
        } else {
            1.0
        };
        DockSlotLayout {
            item_id,
            kind,
            base_rect,
            visual_rect: scale_slot_rect(base_rect, scale, self.edge),
            scale,
        }
    }
}

fn build_slots(
    config: &DockConfig,
    bar_rect: Rect,
    max_scale: f32,
    hover: HoverVisualState,
) -> Vec<DockSlotLayout> {
    let icon = u32::from(config.icon_size_px);
    let spacing = i32::from(config.item_spacing_px);
    let ctx = SlotCtx {
        bar_rect,
        pad: i32::from(config.padding_px),
        cross_len: icon,
        edge: config.edge,
        max_scale,
        hover,
        progress: hover.progress.clamp(0.0, 1.0),
    };
    let cap = config.items.len() + 2;
    let mut slots = Vec::with_capacity(cap);
    let mut cursor = 0i32;

    if !config.locked {
        slots.push(ctx.make(
            DRAG_HANDLE_SLOT_ID,
            SlotKind::DragHandle,
            DRAG_HANDLE_THICKNESS_PX,
            cursor,
        ));
        cursor = cursor
            .saturating_add(i32::try_from(DRAG_HANDLE_THICKNESS_PX).unwrap_or(16))
            .saturating_add(spacing);
    }
    for item in &config.items {
        let (kind, len) = if item.is_separator() {
            (SlotKind::Separator, SEPARATOR_THICKNESS_PX)
        } else {
            (SlotKind::Shortcut, icon)
        };
        slots.push(ctx.make(item.id, kind, len, cursor));
        cursor = cursor
            .saturating_add(i32::try_from(len).unwrap_or(0))
            .saturating_add(spacing);
    }
    if config.show_settings_button {
        slots.push(ctx.make(SETTINGS_SLOT_ID, SlotKind::SettingsButton, icon, cursor));
    }
    slots
}

fn scale_slot_rect(base: Rect, scale: f32, edge: DockEdge) -> Rect {
    if scale <= 1.0 {
        return base;
    }
    let scaled_w = ((base.width as f32) * scale).round().max(1.0) as u32;
    let scaled_h = ((base.height as f32) * scale).round().max(1.0) as u32;
    let dw = i32::try_from(scaled_w.saturating_sub(base.width)).unwrap_or(0);
    let dh = i32::try_from(scaled_h.saturating_sub(base.height)).unwrap_or(0);
    let (x, y) = match edge {
        DockEdge::Bottom => (base.x.saturating_sub(dw / 2), base.y.saturating_sub(dh)),
        DockEdge::Top => (base.x.saturating_sub(dw / 2), base.y),
        DockEdge::Left => (base.x, base.y.saturating_sub(dh / 2)),
        DockEdge::Right => (base.x.saturating_sub(dw), base.y.saturating_sub(dh / 2)),
    };
    Rect {
        x,
        y,
        width: scaled_w,
        height: scaled_h,
    }
}
