use crate::config::{DockAlignment, DockConfig, DockEdge};

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
        let right = self.x.saturating_add(w);
        let bottom = self.y.saturating_add(h);
        point.x >= self.x && point.x < right && point.y >= self.y && point.y < bottom
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HoverVisualState {
    pub item_id: Option<u64>,
    pub progress: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockSlotLayout {
    pub item_id: u64,
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
    let count = u32::try_from(config.items.len()).unwrap_or(u32::MAX);

    let main_items_len = if count == 0 {
        icon
    } else {
        count
            .saturating_mul(icon)
            .saturating_add(count.saturating_sub(1).saturating_mul(spacing))
    };
    let bar_main = main_items_len.saturating_add(padding.saturating_mul(2));
    let bar_cross = icon.saturating_add(padding.saturating_mul(2));

    let max_scale = config.hover_zoom.effective_scale();
    let raw_extra = (icon as f32) * (max_scale - 1.0);
    let zoom_extra = raw_extra.ceil().max(0.0) as u32;
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

pub fn hit_test_item(layout: &DockLayout, local_point: Point) -> Option<u64> {
    layout
        .slots
        .iter()
        .find(|slot| slot.base_rect.contains(local_point))
        .map(|slot| slot.item_id)
}

pub fn drop_insert_index(layout: &DockLayout, edge: DockEdge, local_point: Point) -> usize {
    let horizontal = edge.is_horizontal();
    for (idx, slot) in layout.slots.iter().enumerate() {
        let midpoint = if horizontal {
            let half = i32::try_from(slot.base_rect.width / 2).unwrap_or(0);
            slot.base_rect.x.saturating_add(half)
        } else {
            let half = i32::try_from(slot.base_rect.height / 2).unwrap_or(0);
            slot.base_rect.y.saturating_add(half)
        };
        let coord = if horizontal {
            local_point.x
        } else {
            local_point.y
        };
        if coord < midpoint {
            return idx;
        }
    }
    layout.slots.len()
}

fn build_window_and_bar(
    edge: DockEdge,
    bar_main: u32,
    bar_cross: u32,
    zoom_extra: u32,
    side_pad: u32,
) -> (u32, u32, Rect) {
    let side_pad_i32 = i32::try_from(side_pad).unwrap_or(0);
    let zoom_i32 = i32::try_from(zoom_extra).unwrap_or(0);
    match edge {
        DockEdge::Bottom => {
            let win_w = bar_main.saturating_add(side_pad.saturating_mul(2));
            let win_h = bar_cross.saturating_add(zoom_extra);
            let bar = Rect {
                x: side_pad_i32,
                y: zoom_i32,
                width: bar_main,
                height: bar_cross,
            };
            (win_w, win_h, bar)
        }
        DockEdge::Top => {
            let win_w = bar_main.saturating_add(side_pad.saturating_mul(2));
            let win_h = bar_cross.saturating_add(zoom_extra);
            let bar = Rect {
                x: side_pad_i32,
                y: 0,
                width: bar_main,
                height: bar_cross,
            };
            (win_w, win_h, bar)
        }
        DockEdge::Left => {
            let win_w = bar_cross.saturating_add(zoom_extra);
            let win_h = bar_main.saturating_add(side_pad.saturating_mul(2));
            let bar = Rect {
                x: 0,
                y: side_pad_i32,
                width: bar_cross,
                height: bar_main,
            };
            (win_w, win_h, bar)
        }
        DockEdge::Right => {
            let win_w = bar_cross.saturating_add(zoom_extra);
            let win_h = bar_main.saturating_add(side_pad.saturating_mul(2));
            let bar = Rect {
                x: zoom_i32,
                y: side_pad_i32,
                width: bar_cross,
                height: bar_main,
            };
            (win_w, win_h, bar)
        }
    }
}

fn place_window(config: &DockConfig, work_area: Rect, win_w: u32, win_h: u32) -> Rect {
    let margin = i32::from(config.edge_margin_px);
    let area_w = i32::try_from(work_area.width).unwrap_or(i32::MAX);
    let area_h = i32::try_from(work_area.height).unwrap_or(i32::MAX);
    let w_i32 = i32::try_from(win_w).unwrap_or(i32::MAX);
    let h_i32 = i32::try_from(win_h).unwrap_or(i32::MAX);

    let (x, y) = match config.edge {
        DockEdge::Bottom => {
            let x = align_axis(work_area.x, area_w, w_i32, config.alignment);
            let bottom = work_area.y.saturating_add(area_h);
            let y = bottom.saturating_sub(h_i32).saturating_sub(margin);
            (x, y)
        }
        DockEdge::Top => {
            let x = align_axis(work_area.x, area_w, w_i32, config.alignment);
            let y = work_area.y.saturating_add(margin);
            (x, y)
        }
        DockEdge::Left => {
            let x = work_area.x.saturating_add(margin);
            let y = align_axis(work_area.y, area_h, h_i32, config.alignment);
            (x, y)
        }
        DockEdge::Right => {
            let right = work_area.x.saturating_add(area_w);
            let x = right.saturating_sub(w_i32).saturating_sub(margin);
            let y = align_axis(work_area.y, area_h, h_i32, config.alignment);
            (x, y)
        }
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

fn build_slots(
    config: &DockConfig,
    bar_rect: Rect,
    max_scale: f32,
    hover: HoverVisualState,
) -> Vec<DockSlotLayout> {
    let icon = u32::from(config.icon_size_px);
    let icon_i32 = i32::from(config.icon_size_px);
    let spacing_i32 = i32::from(config.item_spacing_px);
    let step = icon_i32.saturating_add(spacing_i32);
    let pad = i32::from(config.padding_px);
    let progress = hover.progress.clamp(0.0, 1.0);
    let horizontal = config.edge.is_horizontal();

    let mut slots = Vec::with_capacity(config.items.len());
    for (idx, item) in config.items.iter().enumerate() {
        let offset = i32::try_from(idx).unwrap_or(0).saturating_mul(step);
        let base_rect = if horizontal {
            Rect {
                x: bar_rect.x.saturating_add(pad).saturating_add(offset),
                y: bar_rect.y.saturating_add(pad),
                width: icon,
                height: icon,
            }
        } else {
            Rect {
                x: bar_rect.x.saturating_add(pad),
                y: bar_rect.y.saturating_add(pad).saturating_add(offset),
                width: icon,
                height: icon,
            }
        };

        let is_hovered = hover.item_id == Some(item.id) && progress > 0.0 && max_scale > 1.0;
        let scale = if is_hovered {
            1.0 + (max_scale - 1.0) * progress
        } else {
            1.0
        };
        let visual_rect = scale_slot_rect(base_rect, scale, config.edge);

        slots.push(DockSlotLayout {
            item_id: item.id,
            base_rect,
            visual_rect,
            scale,
        });
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

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::PathBuf;

    use crate::config::DockItem;

    use super::*;

    fn sample_work_area() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }
    }

    fn dock_with_three_items(edge: DockEdge) -> Result<DockConfig, Box<dyn Error>> {
        let mut dock = DockConfig::new(1, "Test");
        dock.edge = edge;
        for id in 1..=3 {
            let path = PathBuf::from(format!("app{id}.exe"));
            dock.items.push(DockItem::from_dropped_path(id, path)?);
        }
        Ok(dock)
    }

    #[test]
    fn centers_dock_on_all_four_edges() -> Result<(), Box<dyn Error>> {
        let area = sample_work_area();
        let edges = [
            DockEdge::Bottom,
            DockEdge::Top,
            DockEdge::Left,
            DockEdge::Right,
        ];

        for edge in edges {
            let dock = dock_with_three_items(edge)?;
            let layout = compute_layout(&dock, area, HoverVisualState::default());
            let w = layout.window_rect;
            match edge {
                DockEdge::Bottom => {
                    assert_eq!(w.x, (1920 - w.width as i32) / 2);
                    assert_eq!(w.y + w.height as i32, 1080 - 8);
                }
                DockEdge::Top => {
                    assert_eq!(w.x, (1920 - w.width as i32) / 2);
                    assert_eq!(w.y, 8);
                }
                DockEdge::Left => {
                    assert_eq!(w.x, 8);
                    assert_eq!(w.y, (1080 - w.height as i32) / 2);
                }
                DockEdge::Right => {
                    assert_eq!(w.x + w.width as i32, 1920 - 8);
                    assert_eq!(w.y, (1080 - w.height as i32) / 2);
                }
            }
        }
        Ok(())
    }

    #[test]
    fn hover_zoom_scales_only_single_active_icon() -> Result<(), Box<dyn Error>> {
        let dock = dock_with_three_items(DockEdge::Bottom)?;
        let hover = HoverVisualState {
            item_id: Some(2),
            progress: 1.0,
        };
        let layout = compute_layout(&dock, sample_work_area(), hover);

        let first = layout.slots.first().ok_or("slot 0 missing")?;
        let second = layout.slots.get(1).ok_or("slot 1 missing")?;
        let third = layout.slots.get(2).ok_or("slot 2 missing")?;

        assert_eq!(first.visual_rect, first.base_rect);
        assert_eq!(third.visual_rect, third.base_rect);
        assert!(second.visual_rect.width > second.base_rect.width);
        assert!(second.visual_rect.y < second.base_rect.y);
        assert!((second.scale - 1.35).abs() < 1e-4);
        assert!((first.scale - 1.0).abs() < 1e-4);
        Ok(())
    }

    #[test]
    fn drop_insert_index_respects_icon_midpoints() -> Result<(), Box<dyn Error>> {
        let dock = dock_with_three_items(DockEdge::Bottom)?;
        let layout = compute_layout(&dock, sample_work_area(), HoverVisualState::default());
        let first = layout.slots.first().ok_or("slot 0 missing")?;
        let second = layout.slots.get(1).ok_or("slot 1 missing")?;

        let left = Point {
            x: 0,
            y: first.base_rect.y,
        };
        let middle = Point {
            x: second.base_rect.x + 2,
            y: second.base_rect.y,
        };
        let far_right = Point {
            x: 9999,
            y: second.base_rect.y,
        };

        assert_eq!(drop_insert_index(&layout, DockEdge::Bottom, left), 0);
        assert_eq!(drop_insert_index(&layout, DockEdge::Bottom, middle), 1);
        assert_eq!(drop_insert_index(&layout, DockEdge::Bottom, far_right), 3);
        Ok(())
    }
}
