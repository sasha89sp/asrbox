use std::collections::HashMap;

use asrbox_platform_api::{BgraFrame, RgbaImage};

use crate::config::{DockConfig, DockEdge};
use crate::layout::{DockLayout, DockSlotLayout, Rect, SlotKind};

pub fn render_dock_frame(
    config: &DockConfig,
    layout: &DockLayout,
    icons: &HashMap<u64, RgbaImage>,
) -> BgraFrame {
    let mut frame = BgraFrame::transparent(layout.window_rect.width, layout.window_rect.height);
    let radius = f32::from(config.corner_radius_px);

    draw_capsule_shadow(&mut frame, layout.bar_rect, radius);
    draw_glass_capsule(&mut frame, layout.bar_rect, radius);

    if layout.slots.is_empty() {
        draw_empty_plus(&mut frame, layout.bar_rect);
        return frame;
    }

    for slot in &layout.slots {
        if slot.scale <= 1.001 {
            draw_slot(&mut frame, *slot, icons.get(&slot.item_id), config.edge);
        }
    }
    for slot in &layout.slots {
        if slot.scale > 1.001 {
            draw_slot(&mut frame, *slot, icons.get(&slot.item_id), config.edge);
        }
    }
    frame
}

fn draw_capsule_shadow(frame: &mut BgraFrame, bar: Rect, radius: f32) {
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
                let alpha = (falloff * falloff * 78.0).round() as u8;
                blend_pixel(frame, px, py, [8, 10, 14, alpha]);
            }
        }
    }
}

fn draw_glass_capsule(frame: &mut BgraFrame, bar: Rect, radius: f32) {
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

    for py in y0..y1 {
        let t = ((py - bar.y) as f32 / h).clamp(0.0, 1.0);
        let top_boost = ((1.0 - t) * 18.0).round() as u8;
        let base_r = 26u8.saturating_add(top_boost);
        let base_g = 29u8.saturating_add(top_boost);
        let base_b = 36u8.saturating_add(top_boost);

        for px in x0..x1 {
            let dist = rounded_rect_sdf(px as f32 + 0.5, py as f32 + 0.5, bar, radius);
            if dist <= 0.75 {
                let cov = (0.75 - dist).clamp(0.0, 1.0);
                let body_a = (212.0 * cov).round() as u8;
                blend_pixel(frame, px, py, [base_r, base_g, base_b, body_a]);
                if dist > -1.35 {
                    let rim = (1.0 - ((dist + 0.35).abs() / 1.0)).clamp(0.0, 1.0);
                    let rim_a = (72.0 * rim * cov).round() as u8;
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

    for dx in -arm..=arm {
        blend_pixel(frame, cx + dx, cy - 1, color);
        blend_pixel(frame, cx + dx, cy, color);
        blend_pixel(frame, cx + dx, cy + 1, color);
    }
    for dy in -arm..=arm {
        if (-1..=1).contains(&dy) {
            continue;
        }
        blend_pixel(frame, cx - 1, cy + dy, color);
        blend_pixel(frame, cx, cy + dy, color);
        blend_pixel(frame, cx + 1, cy + dy, color);
    }
}

fn draw_slot(
    frame: &mut BgraFrame,
    slot: DockSlotLayout,
    icon: Option<&RgbaImage>,
    edge: DockEdge,
) {
    match slot.kind {
        SlotKind::Separator => draw_separator(frame, slot.base_rect, edge),
        SlotKind::SettingsButton => draw_settings_button(frame, slot.visual_rect),
        SlotKind::Shortcut => match icon {
            Some(image) if image.width > 0 && image.height > 0 => {
                blit_scaled_icon(frame, slot.visual_rect, image);
            }
            _ => draw_fallback_tile(frame, slot.visual_rect),
        },
    }
}

fn draw_separator(frame: &mut BgraFrame, rect: Rect, edge: DockEdge) {
    let cx = rect.x + i32::try_from(rect.width / 2).unwrap_or(0);
    let cy = rect.y + i32::try_from(rect.height / 2).unwrap_or(0);
    let light = [255, 255, 255, 76];
    let shadow = [10, 12, 18, 95];
    if edge.is_horizontal() {
        let margin = (i32::try_from(rect.height).unwrap_or(40) / 6).max(4);
        let y0 = rect.y.saturating_add(margin);
        let y1 = rect
            .y
            .saturating_add(i32::try_from(rect.height).unwrap_or(40))
            .saturating_sub(margin);
        for py in y0..y1 {
            blend_pixel(frame, cx - 1, py, shadow);
            blend_pixel(frame, cx, py, light);
        }
    } else {
        let margin = (i32::try_from(rect.width).unwrap_or(40) / 6).max(4);
        let x0 = rect.x.saturating_add(margin);
        let x1 = rect
            .x
            .saturating_add(i32::try_from(rect.width).unwrap_or(40))
            .saturating_sub(margin);
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
    let x1 = rect
        .x
        .saturating_add(i32::try_from(rect.width).unwrap_or(0))
        .min(i32::try_from(frame.width).unwrap_or(0));
    let y1 = rect
        .y
        .saturating_add(i32::try_from(rect.height).unwrap_or(0))
        .min(i32::try_from(frame.height).unwrap_or(0));
    let cx = rect.x as f32 + (rect.width as f32) * 0.5;
    let cy = rect.y as f32 + (rect.height as f32) * 0.5;
    let unit = (rect.width.min(rect.height) as f32) * 0.5;

    for py in y0..y1 {
        let t = ((py - rect.y) as f32 / (rect.height.max(1) as f32)).clamp(0.0, 1.0);
        let bg = (74.0 - 24.0 * t).round() as u8;
        for px in x0..x1 {
            let fx = px as f32 + 0.5;
            let fy = py as f32 + 0.5;
            let dist = rounded_rect_sdf(fx, fy, rect, radius);
            if dist <= 0.6 {
                let cov = (0.6 - dist).clamp(0.0, 1.0);
                let a = (238.0 * cov).round() as u8;
                blend_pixel(frame, px, py, [bg, bg + 4, bg + 12, a]);
                if dist > -1.3 {
                    let rim_a = (68.0 * cov).round() as u8;
                    blend_pixel(frame, px, py, [255, 255, 255, rim_a]);
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
    let angle = dy.atan2(dx);
    let tooth = (angle * 8.0).cos();
    let outer = if tooth > 0.2 { 0.66 } else { 0.52 };
    let inner = 0.22;
    if r <= outer && r >= inner {
        let edge_out = ((outer - r) * unit).clamp(0.0, 1.0);
        let edge_in = ((r - inner) * unit).clamp(0.0, 1.0);
        (242.0 * edge_out.min(edge_in)).round() as u8
    } else {
        0
    }
}

fn draw_fallback_tile(frame: &mut BgraFrame, rect: Rect) {
    let radius = (rect.width.min(rect.height) as f32) * 0.24;
    let x0 = rect.x.max(0);
    let y0 = rect.y.max(0);
    let x1 = rect
        .x
        .saturating_add(i32::try_from(rect.width).unwrap_or(0))
        .min(i32::try_from(frame.width).unwrap_or(0));
    let y1 = rect
        .y
        .saturating_add(i32::try_from(rect.height).unwrap_or(0))
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
                let a = (238.0 * cov).round() as u8;
                blend_pixel(frame, px, py, [r, g, b, a]);
            }
        }
    }
}

fn blit_scaled_icon(frame: &mut BgraFrame, dst: Rect, src: &RgbaImage) {
    let dst_w = dst.width.max(1) as f32;
    let dst_h = dst.height.max(1) as f32;
    let x0 = dst.x.max(0);
    let y0 = dst.y.max(0);
    let x1 = dst
        .x
        .saturating_add(i32::try_from(dst.width).unwrap_or(0))
        .min(i32::try_from(frame.width).unwrap_or(0));
    let y1 = dst
        .y
        .saturating_add(i32::try_from(dst.height).unwrap_or(0))
        .min(i32::try_from(frame.height).unwrap_or(0));

    for py in y0..y1 {
        let v = (((py - dst.y) as f32) + 0.5) / dst_h;
        for px in x0..x1 {
            let u = (((px - dst.x) as f32) + 0.5) / dst_w;
            let rgba = sample_bilinear(src, u, v);
            if rgba[3] > 0 {
                blend_pixel(frame, px, py, rgba);
            }
        }
    }
}

fn sample_bilinear(src: &RgbaImage, u: f32, v: f32) -> [u8; 4] {
    let fx = (u * src.width as f32 - 0.5).clamp(0.0, (src.width - 1) as f32);
    let fy = (v * src.height as f32 - 0.5).clamp(0.0, (src.height - 1) as f32);
    let x0 = fx.floor() as u32;
    let y0 = fy.floor() as u32;
    let x1 = (x0 + 1).min(src.width - 1);
    let y1 = (y0 + 1).min(src.height - 1);
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;

    let p00 = read_rgba(src, x0, y0);
    let p10 = read_rgba(src, x1, y0);
    let p01 = read_rgba(src, x0, y1);
    let p11 = read_rgba(src, x1, y1);

    let mut out = [0u8; 4];
    for ch in 0..4 {
        let top = p00[ch] as f32 * (1.0 - tx) + p10[ch] as f32 * tx;
        let bot = p01[ch] as f32 * (1.0 - tx) + p11[ch] as f32 * tx;
        out[ch] = (top * (1.0 - ty) + bot * ty).round().clamp(0.0, 255.0) as u8;
    }
    out
}

fn read_rgba(src: &RgbaImage, x: u32, y: u32) -> [u8; 4] {
    let idx = ((y as usize) * (src.width as usize) + (x as usize)) * 4;
    match src.pixels.get(idx..idx + 4) {
        Some(slice) => [slice[0], slice[1], slice[2], slice[3]],
        None => [0, 0, 0, 0],
    }
}

fn rounded_rect_sdf(px: f32, py: f32, rect: Rect, radius: f32) -> f32 {
    let half_w = (rect.width as f32) * 0.5;
    let half_h = (rect.height as f32) * 0.5;
    let r = radius.min(half_w).min(half_h).max(0.0);
    let cx = (rect.x as f32) + half_w;
    let cy = (rect.y as f32) + half_h;
    let qx = (px - cx).abs() - (half_w - r);
    let qy = (py - cy).abs() - (half_h - r);
    let outside = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    let inside = qx.max(qy).min(0.0);
    outside + inside - r
}

fn blend_pixel(frame: &mut BgraFrame, x: i32, y: i32, src: [u8; 4]) {
    if x < 0 || y < 0 {
        return;
    }
    let ux = x as u32;
    let uy = y as u32;
    if ux >= frame.width || uy >= frame.height || src[3] == 0 {
        return;
    }
    let idx = ((uy as usize) * (frame.width as usize) + (ux as usize)) * 4;
    let Some(dst) = frame.pixels.get_mut(idx..idx + 4) else {
        return;
    };

    let sa = src[3] as f32 / 255.0;
    let inv_sa = 1.0 - sa;
    let sb = (src[2] as f32) * sa;
    let sg = (src[1] as f32) * sa;
    let sr = (src[0] as f32) * sa;

    dst[0] = (sb + (dst[0] as f32) * inv_sa).round().clamp(0.0, 255.0) as u8;
    dst[1] = (sg + (dst[1] as f32) * inv_sa).round().clamp(0.0, 255.0) as u8;
    dst[2] = (sr + (dst[2] as f32) * inv_sa).round().clamp(0.0, 255.0) as u8;
    let out_a = (src[3] as f32 + (dst[3] as f32) * inv_sa).round();
    dst[3] = out_a.clamp(0.0, 255.0) as u8;
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::path::PathBuf;

    use crate::config::DockItem;
    use crate::layout::{compute_layout, HoverVisualState};

    use super::*;

    fn area() -> Rect {
        Rect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        }
    }

    #[test]
    fn renders_separator_and_settings_button() -> Result<(), Box<dyn Error>> {
        let mut config = DockConfig::new(1, "Main");
        config
            .items
            .push(DockItem::from_dropped_path(1, PathBuf::from("App.exe"))?);
        config.items.push(DockItem::separator(2));
        let layout = compute_layout(&config, area(), HoverVisualState::default());
        let frame = render_dock_frame(&config, &layout, &HashMap::new());
        assert_eq!(frame.width, layout.window_rect.width);
        assert_eq!(frame.pixels[3], 0);
        let gear = layout.slots.last().copied().ok_or("missing gear")?;
        let px = (gear.visual_rect.x as usize) + (gear.visual_rect.width as usize / 2);
        let py = (gear.visual_rect.y as usize) + (gear.visual_rect.height as usize / 2);
        assert!(frame.pixels[(py * (frame.width as usize) + px) * 4 + 3] > 200);
        Ok(())
    }
}
