use std::collections::{HashMap, HashSet};

use asrbox_platform_api::{BgraFrame, FluentPalette, RgbaImage, WidgetFrame, WidgetTextCmd};

use crate::config::{DockConfig, MAX_DOCK_OPACITY, MIN_DOCK_OPACITY};
use crate::labels::{
    draw_labels_only_pill, icon_and_label_rects, push_icon_label_text, DockLabelMode,
};
use crate::layout::{DockLayout, DockSlotLayout, Rect, SlotKind};
use crate::render_marks::{draw_drag_handle, draw_insert_marker, draw_running_dot, draw_separator};
use crate::render_ops::{
    blend_pixel, blit_scaled_icon, draw_fallback_tile, draw_folder_badge, draw_icon_tile,
    draw_settings_button, rounded_rect_sdf,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct DockRenderOverlay {
    pub dragged_item_id: Option<u64>,
    pub insert_before_index: Option<usize>,
    pub hovered_item_id: Option<u64>,
    pub pressed_item_id: Option<u64>,
    pub palette: FluentPalette,
}

#[derive(Clone, Copy)]
struct SlotFlags {
    hovered: bool,
    pressed: bool,
    dimmed: bool,
    running: bool,
    is_folder: bool,
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
    let pal = &overlay.palette;

    draw_capsule_shadow(&mut frame, layout.bar_rect, radius, opacity_scale, pal);
    draw_glass_capsule(&mut frame, layout.bar_rect, radius, opacity_scale, pal);

    if layout.slots.is_empty() {
        draw_empty_plus(&mut frame, layout.bar_rect, pal);
        return WidgetFrame { bgra: frame, texts };
    }

    for pass_zoomed in [false, true] {
        for slot in &layout.slots {
            let is_zoomed = slot.scale > 1.001;
            if is_zoomed != pass_zoomed {
                continue;
            }
            let item = config.items.iter().find(|i| i.id == slot.item_id);
            let label = item.map(|i| i.label.as_str()).unwrap_or("");
            let is_folder =
                slot.kind == SlotKind::Shortcut && item.is_some_and(|i| path_is_folder(&i.target));
            let flags = SlotFlags {
                hovered: overlay.hovered_item_id == Some(slot.item_id),
                pressed: overlay.pressed_item_id == Some(slot.item_id),
                dimmed: overlay.dragged_item_id == Some(slot.item_id),
                running: config.show_running_indicator
                    && slot.kind == SlotKind::Shortcut
                    && running_items.contains(&slot.item_id),
                is_folder,
            };
            draw_slot(
                &mut frame,
                &mut texts,
                *slot,
                icons.get(&slot.item_id),
                label,
                config,
                flags,
                pal,
            );
        }
    }
    if let Some(ins_idx) = overlay.insert_before_index {
        draw_insert_marker(&mut frame, layout, config.edge, ins_idx, pal);
    }
    WidgetFrame { bgra: frame, texts }
}

fn path_is_folder(path: &std::path::Path) -> bool {
    let skip_ext = path.extension().is_some_and(|e| {
        e.eq_ignore_ascii_case("exe")
            || e.eq_ignore_ascii_case("lnk")
            || e.eq_ignore_ascii_case("bat")
            || e.eq_ignore_ascii_case("cmd")
            || e.eq_ignore_ascii_case("msi")
            || e.eq_ignore_ascii_case("url")
    });
    !skip_ext && path.is_dir()
}

fn draw_capsule_shadow(
    frame: &mut BgraFrame,
    bar: Rect,
    radius: f32,
    opacity: f32,
    pal: &FluentPalette,
) {
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
    let strength = if pal.is_dark { 78.0 } else { 52.0 };

    for py in y0..y1 {
        for px in x0..x1 {
            let dist = rounded_rect_sdf(px as f32 + 0.5, py as f32 + 0.5, bar, radius);
            if dist > 0.0 && dist < spread as f32 {
                let falloff = 1.0 - (dist / spread as f32);
                let alpha = (falloff * falloff * strength * opacity).round() as u8;
                blend_pixel(frame, px, py, [8, 10, 14, alpha]);
            }
        }
    }
}

fn draw_glass_capsule(
    frame: &mut BgraFrame,
    bar: Rect,
    radius: f32,
    opacity: f32,
    pal: &FluentPalette,
) {
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
    let max_rim = if pal.is_dark {
        (110.0 * opacity).max(38.0)
    } else {
        (210.0 * opacity).max(80.0)
    };
    let bg = pal.menu_bg;
    let rim_col = pal.card_border;

    for py in y0..y1 {
        let t = ((py - bar.y) as f32 / h).clamp(0.0, 1.0);
        let boost = if pal.is_dark {
            ((1.0 - t) * 7.0).round() as u8
        } else {
            ((1.0 - t) * 4.0).round() as u8
        };
        let r = bg[0].saturating_add(boost);
        let g = bg[1].saturating_add(boost);
        let b = bg[2].saturating_add(boost);
        for px in x0..x1 {
            let dist = rounded_rect_sdf(px as f32 + 0.5, py as f32 + 0.5, bar, radius);
            if dist <= 0.75 {
                let cov = (0.75 - dist).clamp(0.0, 1.0);
                let body_a = (max_body * cov).round() as u8;
                blend_pixel(frame, px, py, [r, g, b, body_a]);
                if dist > -1.25 {
                    let rim = (1.0 - ((dist + 0.25).abs() / 1.0)).clamp(0.0, 1.0);
                    let rim_a = (max_rim * rim * cov).round() as u8;
                    blend_pixel(frame, px, py, [rim_col[0], rim_col[1], rim_col[2], rim_a]);
                }
            }
        }
    }
}

fn draw_empty_plus(frame: &mut BgraFrame, bar: Rect, pal: &FluentPalette) {
    let cx = bar.x + i32::try_from(bar.width / 2).unwrap_or(0);
    let cy = bar.y + i32::try_from(bar.height / 2).unwrap_or(0);
    let arm = (i32::try_from(bar.width.min(bar.height)).unwrap_or(32) / 5).max(6);
    let c = pal.text_muted;
    let color = [c[0], c[1], c[2], 190];
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
    flags: SlotFlags,
    pal: &FluentPalette,
) {
    let vis = if flags.pressed {
        scale_centered_rect(slot.visual_rect, 0.95)
    } else {
        slot.visual_rect
    };
    match slot.kind {
        SlotKind::DragHandle => draw_drag_handle(frame, slot.base_rect, config.edge, pal),
        SlotKind::Separator => {
            draw_separator(frame, slot.base_rect, config.edge, flags.dimmed, pal)
        }
        SlotKind::SettingsButton => {
            let r = square_centered_rect(vis);
            draw_settings_button(frame, r, flags.hovered, pal);
        }
        SlotKind::Shortcut => match config.label_mode {
            DockLabelMode::LabelsOnly => {
                draw_labels_only_pill(
                    frame,
                    texts,
                    slot.base_rect,
                    label,
                    flags.hovered,
                    flags.dimmed,
                    flags.running,
                    pal,
                );
            }
            DockLabelMode::IconAndLabel => {
                let (icon_r, text_r) =
                    icon_and_label_rects(slot, config.edge, u32::from(config.icon_size_px));
                let draw_r = if flags.pressed {
                    scale_centered_rect(icon_r, 0.95)
                } else {
                    icon_r
                };
                draw_icon_or_fallback(frame, draw_r, icon, flags, pal);
                push_icon_label_text(texts, text_r, label, config.edge, flags.dimmed, pal);
                if flags.running {
                    draw_running_dot(frame, slot.base_rect, config.edge, pal);
                }
            }
            DockLabelMode::IconsOnly | DockLabelMode::HoverTooltip => {
                draw_icon_or_fallback(frame, vis, icon, flags, pal);
                if flags.running {
                    draw_running_dot(frame, slot.base_rect, config.edge, pal);
                }
            }
        },
    }
}

fn draw_icon_or_fallback(
    frame: &mut BgraFrame,
    rect: Rect,
    icon: Option<&RgbaImage>,
    flags: SlotFlags,
    pal: &FluentPalette,
) {
    let alpha = if flags.dimmed { 0.35 } else { 1.0 };
    let dst = if pal.design_theme.is_glass() {
        draw_icon_tile(frame, rect, flags.hovered, flags.dimmed, pal);
        let pad = (rect.width.min(rect.height) as i32 / 7).clamp(4, 9);
        Rect {
            x: rect.x + pad,
            y: rect.y + pad,
            width: rect.width.saturating_sub((pad * 2) as u32).max(8),
            height: rect.height.saturating_sub((pad * 2) as u32).max(8),
        }
    } else {
        rect
    };
    match icon {
        Some(image) if image.width > 0 && image.height > 0 => {
            blit_scaled_icon(frame, dst, image, alpha);
        }
        _ => draw_fallback_tile(frame, dst, alpha),
    }
    if flags.is_folder {
        draw_folder_badge(frame, rect, alpha, pal);
    }
}

fn scale_centered_rect(r: Rect, factor: f32) -> Rect {
    let nw = ((r.width as f32) * factor).round().max(8.0) as u32;
    let nh = ((r.height as f32) * factor).round().max(8.0) as u32;
    Rect {
        x: r.x + (r.width as i32 - nw as i32) / 2,
        y: r.y + (r.height as i32 - nh as i32) / 2,
        width: nw,
        height: nh,
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
            insert_before_index: Some(1),
            ..DockRenderOverlay::default()
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
