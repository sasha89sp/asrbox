use std::collections::{HashMap, HashSet};

use asrbox_platform_api::{BgraFrame, RgbaImage, WidgetFrame, WidgetTextCmd};

use crate::config::{DockConfig, DockEdge, MAX_DOCK_OPACITY, MIN_DOCK_OPACITY};
use crate::labels::{
    draw_labels_only_pill, icon_and_label_rects, push_icon_label_text, DockLabelMode,
};
use crate::layout::{DockLayout, DockSlotLayout, Rect, SlotKind};
use crate::render_ops::{
    blend_pixel, blit_scaled_icon, draw_fallback_tile, draw_settings_button, rounded_rect_sdf,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct DockRenderOverlay {
    pub dragged_item_id: Option<u64>,
    pub insert_before_index: Option<usize>,
    pub hovered_item_id: Option<u64>,
}

pub fn render_dock_frame(
    config: &DockConfig,
    layout: &DockLayout,
    icons: &HashMap<u64, RgbaImage>,
    running_items: &HashSet<u64>,
    overlay: DockRenderOverlay,
) -> BgraFrame {
    render_dock_widget_frame(config, layout, icons, running_items, overlay).bgra
}

pub fn render_dock_widget_frame(
    config: &DockConfig,
    layout: &DockLayout,
    icons: &HashMap<u64, RgbaImage>,
    running_items: &HashSet<u64>,
    overlay: DockRenderOverlay,
) -> WidgetFrame {
    let mut frame = BgraFrame::transparent(layout.window_rect.width, layout.window_rect.height);
    let mut texts = Vec::new();
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
        return WidgetFrame { bgra: frame, texts };
    }

    for pass_zoomed in [false, true] {
        for slot in &layout.slots {
            let is_zoomed = slot.scale > 1.001;
            if is_zoomed != pass_zoomed {
                continue;
            }
            let dimmed = overlay.dragged_item_id == Some(slot.item_id);
            let hovered = overlay.hovered_item_id == Some(slot.item_id);
            let running = config.show_running_indicator
                && slot.kind == SlotKind::Shortcut
                && running_items.contains(&slot.item_id);
            let label = config
                .items
                .iter()
                .find(|i| i.id == slot.item_id)
                .map(|i| i.label.as_str())
                .unwrap_or("");
            draw_slot(
                &mut frame,
                &mut texts,
                *slot,
                icons.get(&slot.item_id),
                label,
                config,
                hovered,
                dimmed,
                running,
            );
        }
    }
    if let Some(ins_idx) = overlay.insert_before_index {
        draw_insert_marker(&mut frame, layout, config.edge, ins_idx);
    }
    WidgetFrame { bgra: frame, texts }
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
        let boost = ((1.0 - t) * 12.0).round() as u8;
        let (r, g, b) = (26 + boost, 27 + boost, 32 + boost);
        for px in x0..x1 {
            let dist = rounded_rect_sdf(px as f32 + 0.5, py as f32 + 0.5, bar, radius);
            if dist <= 0.75 {
                let cov = (0.75 - dist).clamp(0.0, 1.0);
                let body_a = (max_body * cov).round() as u8;
                blend_pixel(frame, px, py, [r, g, b, body_a]);
                if dist > -1.35 {
                    let rim = (1.0 - ((dist + 0.35).abs() / 1.0)).clamp(0.0, 1.0);
                    let rim_a = (max_rim * rim * cov).round() as u8;
                    blend_pixel(frame, px, py, [238, 232, 226, rim_a]);
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

#[allow(clippy::too_many_arguments)]
fn draw_slot(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    slot: DockSlotLayout,
    icon: Option<&RgbaImage>,
    label: &str,
    config: &DockConfig,
    hovered: bool,
    dimmed: bool,
    running: bool,
) {
    match slot.kind {
        SlotKind::DragHandle => draw_drag_handle(frame, slot.base_rect, config.edge),
        SlotKind::Separator => draw_separator(frame, slot.base_rect, config.edge, dimmed),
        SlotKind::SettingsButton => {
            let r = square_centered_rect(slot.visual_rect);
            draw_settings_button(frame, r);
        }
        SlotKind::Shortcut => match config.label_mode {
            DockLabelMode::LabelsOnly => {
                draw_labels_only_pill(
                    frame,
                    texts,
                    slot.base_rect,
                    label,
                    hovered,
                    dimmed,
                    running,
                );
            }
            DockLabelMode::IconAndLabel => {
                let (icon_r, text_r) =
                    icon_and_label_rects(slot, config.edge, u32::from(config.icon_size_px));
                draw_icon_or_fallback(frame, icon_r, icon, dimmed);
                push_icon_label_text(texts, text_r, label, config.edge, dimmed);
                if running {
                    draw_running_dot(frame, slot.base_rect, config.edge);
                }
            }
            DockLabelMode::IconsOnly | DockLabelMode::HoverTooltip => {
                draw_icon_or_fallback(frame, slot.visual_rect, icon, dimmed);
                if running {
                    draw_running_dot(frame, slot.base_rect, config.edge);
                }
            }
        },
    }
}

fn draw_icon_or_fallback(
    frame: &mut BgraFrame,
    rect: Rect,
    icon: Option<&RgbaImage>,
    dimmed: bool,
) {
    let alpha = if dimmed { 0.35 } else { 1.0 };
    match icon {
        Some(image) if image.width > 0 && image.height > 0 => {
            blit_scaled_icon(frame, rect, image, alpha);
        }
        _ => draw_fallback_tile(frame, rect, alpha),
    }
}

fn square_centered_rect(r: Rect) -> Rect {
    let side = r.width.min(r.height);
    Rect {
        x: r.x + (r.width as i32 - side as i32) / 2,
        y: r.y + (r.height as i32 - side as i32) / 2,
        width: side,
        height: side,
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
                let a = (242.0 * cov).round() as u8;
                blend_pixel(frame, cx + dx, cy + dy, [236, 134, 86, a]);
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
    let accent = [232, 122, 72, 240];
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
            hovered_item_id: None,
        };
        let frame = render_dock_frame(&config, &layout, &HashMap::new(), &running, overlay);
        assert_eq!(frame.width, layout.window_rect.width);
        assert_eq!(frame.pixels[3], 0);
        config.label_mode = DockLabelMode::IconAndLabel;
        let wf = render_dock_widget_frame(&config, &layout, &HashMap::new(), &running, overlay);
        assert!(!wf.texts.is_empty());
        Ok(())
    }
}
