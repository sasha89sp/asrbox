use std::collections::{HashMap, HashSet};

use asrbox_platform_api::{BgraFrame, RgbaImage};

use crate::config::{DockConfig, DockEdge, MAX_DOCK_OPACITY, MIN_DOCK_OPACITY};
use crate::layout::{DockLayout, DockSlotLayout, Rect, SlotKind};
use crate::render_ops::{blend_pixel, blit_scaled_icon, rounded_rect_sdf};

#[derive(Debug, Clone, Copy, Default)]
pub struct DockRenderOverlay {
    pub dragged_item_id: Option<u64>,
    pub insert_before_index: Option<usize>,
}

pub fn render_dock_frame(
    config: &DockConfig,
    layout: &DockLayout,
    icons: &HashMap<u64, RgbaImage>,
    running_items: &HashSet<u64>,
    overlay: DockRenderOverlay,
) -> BgraFrame {
    let mut frame = BgraFrame::transparent(layout.window_rect.width, layout.window_rect.height);
    let radius = f32::from(config.corner_radius_px);
    let opacity_scale = f32::from(
        config
            .opacity_percent
            .clamp(MIN_DOCK_OPACITY, MAX_DOCK_OPACITY),
    ) / 100.0;

    draw_capsule_shadow(&mut frame, layout.bar_rect, radius, opacity_scale);
    draw_glass_capsule(&mut frame, layout.bar_rect, radius, opacity_scale);

    if layout.slots.is_empty() {
        draw_empty_plus(&mut frame, layout.bar_rect);
        return frame;
    }

    for pass_zoomed in [false, true] {
        for slot in &layout.slots {
            let is_zoomed = slot.scale > 1.001;
            if is_zoomed != pass_zoomed {
                continue;
            }
            let dimmed = overlay.dragged_item_id == Some(slot.item_id);
            let running = config.show_running_indicator
                && slot.kind == SlotKind::Shortcut
                && running_items.contains(&slot.item_id);
            draw_slot(
                &mut frame,
                *slot,
                icons.get(&slot.item_id),
                config.edge,
                dimmed,
                running,
            );
        }
    }
    if let Some(ins_idx) = overlay.insert_before_index {
        draw_insert_marker(&mut frame, layout, config.edge, ins_idx);
    }
    frame
}

fn draw_capsule_shadow(frame: &mut BgraFrame, bar: Rect, radius: f32, opacity: f32) {
    let spread = 4;
    let x0 = bar.x.saturating_sub(spread).max(0);
    let y0 = bar.y.saturating_sub(spread).max(0);
    let x1 = bar
        .x
        .saturating_add(i32::try_from(bar.width).unwrap_or(0))
        .saturating_add(spread)
        .min(i32::try_from(frame.width).unwrap_or(0));
    let y1 = bar
        .y
        .saturating_add(i32::try_from(bar.height).unwrap_or(0))
        .saturating_add(spread)
        .min(i32::try_from(frame.height).unwrap_or(0));

    for py in y0..y1 {
        for px in x0..x1 {
            let dist = rounded_rect_sdf(px as f32 + 0.5, py as f32 + 0.5, bar, radius);
            if dist > 0.0 && dist < spread as f32 {
                let falloff = 1.0 - (dist / spread as f32);
                let alpha = (falloff * falloff * 78.0 * opacity).round() as u8;
                blend_pixel(frame, px, py, [8, 10, 14, alpha]);
            }
        }
    }
}

fn draw_glass_capsule(frame: &mut BgraFrame, bar: Rect, radius: f32, opacity: f32) {
    let x0 = bar.x.max(0);
    let y0 = bar.y.max(0);
    let x1 = bar
        .x
        .saturating_add(i32::try_from(bar.width).unwrap_or(0))
        .min(i32::try_from(frame.width).unwrap_or(0));
    let y1 = bar
        .y
        .saturating_add(i32::try_from(bar.height).unwrap_or(0))
        .min(i32::try_from(frame.height).unwrap_or(0));
    let h = (bar.height.max(1)) as f32;
    let max_body = 248.0 * opacity;
    let max_rim = (76.0 * opacity).max(24.0);

    for py in y0..y1 {
        let t = ((py - bar.y) as f32 / h).clamp(0.0, 1.0);
        let boost = ((1.0 - t) * 18.0).round() as u8;
        let (r, g, b) = (26 + boost, 29 + boost, 36 + boost);
        for px in x0..x1 {
            let dist = rounded_rect_sdf(px as f32 + 0.5, py as f32 + 0.5, bar, radius);
            if dist <= 0.75 {
                let cov = (0.75 - dist).clamp(0.0, 1.0);
                let body_a = (max_body * cov).round() as u8;
                blend_pixel(frame, px, py, [r, g, b, body_a]);
                if dist > -1.35 {
                    let rim = (1.0 - ((dist + 0.35).abs() / 1.0)).clamp(0.0, 1.0);
                    let rim_a = (max_rim * rim * cov).round() as u8;
                    blend_pixel(frame, px, py, [255, 255, 255, rim_a]);
                }
            }
        }
    }
}

fn draw_empty_plus(frame: &mut BgraFrame, bar: Rect) {
    let cx = bar.x + i32::try_from(bar.width / 2).unwrap_or(0);
    let cy = bar.y + i32::try_from(bar.height / 2).unwrap_or(0);
    let arm = (i32::try_from(bar.width.min(bar.height)).unwrap_or(32) / 5).max(6);
    let color = [215, 224, 240, 180];
    for d in -arm..=arm {
        for o in -1..=1 {
            blend_pixel(frame, cx + d, cy + o, color);
            blend_pixel(frame, cx + o, cy + d, color);
        }
    }
}

fn draw_slot(
    frame: &mut BgraFrame,
    slot: DockSlotLayout,
    icon: Option<&RgbaImage>,
    edge: DockEdge,
    dimmed: bool,
    running: bool,
) {
    match slot.kind {
        SlotKind::DragHandle => draw_drag_handle(frame, slot.base_rect, edge),
        SlotKind::Separator => draw_separator(frame, slot.base_rect, edge, dimmed),
        SlotKind::SettingsButton => draw_settings_button(frame, slot.visual_rect),
        SlotKind::Shortcut => {
            match icon {
                Some(image) if image.width > 0 && image.height > 0 => {
                    blit_scaled_icon(
                        frame,
                        slot.visual_rect,
                        image,
                        if dimmed { 0.35 } else { 1.0 },
                    );
                }
                _ => draw_fallback_tile(frame, slot.visual_rect, if dimmed { 0.35 } else { 1.0 }),
            }
            if running {
                draw_running_dot(frame, slot.base_rect, edge);
            }
        }
    }
}

fn draw_drag_handle(frame: &mut BgraFrame, rect: Rect, edge: DockEdge) {
    let cx = rect.x + i32::try_from(rect.width / 2).unwrap_or(0);
    let cy = rect.y + i32::try_from(rect.height / 2).unwrap_or(0);
    let dot = [225, 232, 245, 145];
    let offsets = if edge.is_horizontal() {
        [(-2, -6), (2, -6), (-2, 0), (2, 0), (-2, 6), (2, 6)]
    } else {
        [(-6, -2), (-6, 2), (0, -2), (0, 2), (6, -2), (6, 2)]
    };
    for (ox, oy) in offsets {
        for dy in 0..=1 {
            for dx in 0..=1 {
                blend_pixel(frame, cx + ox + dx, cy + oy + dy, dot);
            }
        }
    }
}

fn draw_running_dot(frame: &mut BgraFrame, base: Rect, edge: DockEdge) {
    let (cx, cy) = match edge {
        DockEdge::Bottom => (
            base.x + i32::try_from(base.width / 2).unwrap_or(0),
            base.y + i32::try_from(base.height).unwrap_or(0) + 5,
        ),
        DockEdge::Top => (
            base.x + i32::try_from(base.width / 2).unwrap_or(0),
            base.y - 5,
        ),
        DockEdge::Left => (
            base.x - 5,
            base.y + i32::try_from(base.height / 2).unwrap_or(0),
        ),
        DockEdge::Right => (
            base.x + i32::try_from(base.width).unwrap_or(0) + 5,
            base.y + i32::try_from(base.height / 2).unwrap_or(0),
        ),
    };
    for dy in -3..=3 {
        for dx in -3..=3 {
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            if d <= 2.4 {
                let cov = (2.4 - d).clamp(0.0, 1.0);
                let a = (240.0 * cov).round() as u8;
                blend_pixel(frame, cx + dx, cy + dy, [225, 240, 255, a]);
            }
        }
    }
}

fn draw_insert_marker(frame: &mut BgraFrame, layout: &DockLayout, edge: DockEdge, idx: usize) {
    let items: Vec<DockSlotLayout> = layout
        .slots
        .iter()
        .copied()
        .filter(|s| matches!(s.kind, SlotKind::Shortcut | SlotKind::Separator))
        .collect();
    let accent = [10, 132, 255, 235];
    if edge.is_horizontal() {
        let x = match items.get(idx) {
            Some(s) => s.base_rect.x - 4,
            None => items.last().map_or(layout.bar_rect.x + 12, |s| {
                s.base_rect.x + i32::try_from(s.base_rect.width).unwrap_or(0) + 4
            }),
        };
        let y0 = layout.bar_rect.y + 8;
        let y1 = layout.bar_rect.y + i32::try_from(layout.bar_rect.height).unwrap_or(48) - 8;
        for py in y0..y1 {
            blend_pixel(frame, x, py, accent);
            blend_pixel(frame, x + 1, py, accent);
        }
    } else {
        let y = match items.get(idx) {
            Some(s) => s.base_rect.y - 4,
            None => items.last().map_or(layout.bar_rect.y + 12, |s| {
                s.base_rect.y + i32::try_from(s.base_rect.height).unwrap_or(0) + 4
            }),
        };
        let x0 = layout.bar_rect.x + 8;
        let x1 = layout.bar_rect.x + i32::try_from(layout.bar_rect.width).unwrap_or(48) - 8;
        for px in x0..x1 {
            blend_pixel(frame, px, y, accent);
            blend_pixel(frame, px, y + 1, accent);
        }
    }
}

fn draw_separator(frame: &mut BgraFrame, rect: Rect, edge: DockEdge, dimmed: bool) {
    let cx = rect.x + i32::try_from(rect.width / 2).unwrap_or(0);
    let cy = rect.y + i32::try_from(rect.height / 2).unwrap_or(0);
    let light = [255, 255, 255, if dimmed { 30 } else { 82 }];
    let shadow = [10, 12, 18, if dimmed { 36 } else { 98 }];
    if edge.is_horizontal() {
        let m = (i32::try_from(rect.height).unwrap_or(40) / 6).max(4);
        let y0 = rect.y.saturating_add(m);
        let y1 = rect
            .y
            .saturating_add(i32::try_from(rect.height).unwrap_or(40))
            .saturating_sub(m);
        for py in y0..y1 {
            blend_pixel(frame, cx - 1, py, shadow);
            blend_pixel(frame, cx, py, light);
        }
    } else {
        let m = (i32::try_from(rect.width).unwrap_or(40) / 6).max(4);
        let x0 = rect.x.saturating_add(m);
        let x1 = rect
            .x
            .saturating_add(i32::try_from(rect.width).unwrap_or(40))
            .saturating_sub(m);
        for px in x0..x1 {
            blend_pixel(frame, px, cy - 1, shadow);
            blend_pixel(frame, px, cy, light);
        }
    }
}

fn draw_settings_button(frame: &mut BgraFrame, rect: Rect) {
    let radius = (rect.width.min(rect.height) as f32) * 0.24;
    let x0 = rect.x.max(0);
    let y0 = rect.y.max(0);
    let x1 = (rect.x + i32::try_from(rect.width).unwrap_or(0))
        .min(i32::try_from(frame.width).unwrap_or(0));
    let y1 = (rect.y + i32::try_from(rect.height).unwrap_or(0))
        .min(i32::try_from(frame.height).unwrap_or(0));
    let cx = rect.x as f32 + (rect.width as f32) * 0.5;
    let cy = rect.y as f32 + (rect.height as f32) * 0.5;
    let unit = (rect.width.min(rect.height) as f32) * 0.5;

    for py in y0..y1 {
        let t = ((py - rect.y) as f32 / (rect.height.max(1) as f32)).clamp(0.0, 1.0);
        let bg = (74.0 - 24.0 * t).round() as u8;
        for px in x0..x1 {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let dist = rounded_rect_sdf(fx, fy, rect, radius);
            if dist <= 0.6 {
                let cov = (0.6 - dist).clamp(0.0, 1.0);
                blend_pixel(frame, px, py, [bg, bg + 4, bg + 12, (238.0 * cov) as u8]);
                if dist > -1.3 {
                    blend_pixel(frame, px, py, [255, 255, 255, (68.0 * cov) as u8]);
                }
                let gear_a = sample_gear_alpha(fx - cx, fy - cy, unit);
                if gear_a > 0 {
                    blend_pixel(frame, px, py, [232, 238, 248, gear_a]);
                }
            }
        }
    }
}

fn sample_gear_alpha(dx: f32, dy: f32, unit: f32) -> u8 {
    let r = (dx * dx + dy * dy).sqrt() / unit.max(1.0);
    let tooth = (dy.atan2(dx) * 8.0).cos();
    let outer = if tooth > 0.2 { 0.66 } else { 0.52 };
    let inner = 0.22;
    if r <= outer && r >= inner {
        let eo = ((outer - r) * unit).clamp(0.0, 1.0);
        let ei = ((r - inner) * unit).clamp(0.0, 1.0);
        (242.0 * eo.min(ei)).round() as u8
    } else {
        0
    }
}

fn draw_fallback_tile(frame: &mut BgraFrame, rect: Rect, alpha_mul: f32) {
    let radius = (rect.width.min(rect.height) as f32) * 0.24;
    let x0 = rect.x.max(0);
    let y0 = rect.y.max(0);
    let x1 = (rect.x + i32::try_from(rect.width).unwrap_or(0))
        .min(i32::try_from(frame.width).unwrap_or(0));
    let y1 = (rect.y + i32::try_from(rect.height).unwrap_or(0))
        .min(i32::try_from(frame.height).unwrap_or(0));

    for py in y0..y1 {
        let t = ((py - rect.y) as f32 / (rect.height.max(1) as f32)).clamp(0.0, 1.0);
        let r = (66.0 - 18.0 * t).round() as u8;
        let g = (142.0 - 28.0 * t).round() as u8;
        let b = (242.0 - 22.0 * t).round() as u8;
        for px in x0..x1 {
            let dist = rounded_rect_sdf(px as f32 + 0.5, py as f32 + 0.5, rect, radius);
            if dist <= 0.6 {
                let cov = (0.6 - dist).clamp(0.0, 1.0);
                let a = (238.0 * cov * alpha_mul).round() as u8;
                blend_pixel(frame, px, py, [r, g, b, a]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::PathBuf;

    use crate::config::DockItem;
    use crate::layout::{compute_layout, HoverVisualState};

    use super::*;

    #[test]
    fn renders_opacity_running_dot_and_drag_handle() -> Result<(), Box<dyn Error>> {
        let mut config = DockConfig::new(1, "Main");
        config.locked = false;
        config.opacity_percent = 50;
        config
            .items
            .push(DockItem::from_dropped_path(1, PathBuf::from("App.exe"))?);
        config.items.push(DockItem::separator(2));
        let area = Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let layout = compute_layout(&config, area, HoverVisualState::default());
        let running = HashSet::from([1u64]);
        let overlay = DockRenderOverlay {
            dragged_item_id: None,
            insert_before_index: Some(1),
        };
        let frame = render_dock_frame(&config, &layout, &HashMap::new(), &running, overlay);
        assert_eq!(frame.width, layout.window_rect.width);
        assert_eq!(frame.pixels[3], 0);
        Ok(())
    }
}
