use asrbox_platform_api::{BgraFrame, RgbaImage};

use crate::layout::Rect;

pub(crate) fn blit_scaled_icon(frame: &mut BgraFrame, dst: Rect, src: &RgbaImage, alpha_mul: f32) {
    let (dst_w, dst_h) = (dst.width.max(1) as f32, dst.height.max(1) as f32);
    let (x0, y0) = (dst.x.max(0), dst.y.max(0));
    let x1 = (dst.x + i32::try_from(dst.width).unwrap_or(0))
        .min(i32::try_from(frame.width).unwrap_or(0));
    let y1 = (dst.y + i32::try_from(dst.height).unwrap_or(0))
        .min(i32::try_from(frame.height).unwrap_or(0));

    for py in y0..y1 {
        let v = (((py - dst.y) as f32) + 0.5) / dst_h;
        for px in x0..x1 {
            let u = (((px - dst.x) as f32) + 0.5) / dst_w;
            let mut rgba = sample_bilinear(src, u, v);
            if rgba[3] > 0 {
                rgba[3] = ((rgba[3] as f32) * alpha_mul).round().clamp(0.0, 255.0) as u8;
                blend_pixel(frame, px, py, rgba);
            }
        }
    }
}

fn sample_bilinear(src: &RgbaImage, u: f32, v: f32) -> [u8; 4] {
    let fx = (u * src.width as f32 - 0.5).clamp(0.0, (src.width - 1) as f32);
    let fy = (v * src.height as f32 - 0.5).clamp(0.0, (src.height - 1) as f32);
    let (x0, y0) = (fx.floor() as u32, fy.floor() as u32);
    let (x1, y1) = ((x0 + 1).min(src.width - 1), (y0 + 1).min(src.height - 1));
    let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
    let (p00, p10) = (read_rgba(src, x0, y0), read_rgba(src, x1, y0));
    let (p01, p11) = (read_rgba(src, x0, y1), read_rgba(src, x1, y1));
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
        Some(s) => [s[0], s[1], s[2], s[3]],
        None => [0, 0, 0, 0],
    }
}

pub(crate) fn rounded_rect_sdf(px: f32, py: f32, rect: Rect, radius: f32) -> f32 {
    let (hw, hh) = ((rect.width as f32) * 0.5, (rect.height as f32) * 0.5);
    let r = radius.min(hw).min(hh).max(0.0);
    let (qx, qy) = (
        (px - (rect.x as f32 + hw)).abs() - (hw - r),
        (py - (rect.y as f32 + hh)).abs() - (hh - r),
    );
    (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt() + qx.max(qy).min(0.0) - r
}

pub(crate) fn blend_pixel(frame: &mut BgraFrame, x: i32, y: i32, src: [u8; 4]) {
    if x < 0 || y < 0 || src[3] == 0 {
        return;
    }
    let (ux, uy) = (x as u32, y as u32);
    if ux >= frame.width || uy >= frame.height {
        return;
    }
    let idx = ((uy as usize) * (frame.width as usize) + (ux as usize)) * 4;
    let Some(dst) = frame.pixels.get_mut(idx..idx + 4) else {
        return;
    };
    let sa = src[3] as f32 / 255.0;
    let inv = 1.0 - sa;
    dst[0] = ((src[2] as f32) * sa + (dst[0] as f32) * inv)
        .round()
        .clamp(0.0, 255.0) as u8;
    dst[1] = ((src[1] as f32) * sa + (dst[1] as f32) * inv)
        .round()
        .clamp(0.0, 255.0) as u8;
    dst[2] = ((src[0] as f32) * sa + (dst[2] as f32) * inv)
        .round()
        .clamp(0.0, 255.0) as u8;
    dst[3] = (src[3] as f32 + (dst[3] as f32) * inv)
        .round()
        .clamp(0.0, 255.0) as u8;
}

pub(crate) fn draw_icon_tile(
    frame: &mut BgraFrame,
    rect: Rect,
    hovered: bool,
    dimmed: bool,
    pal: &asrbox_platform_api::FluentPalette,
) {
    let radius = ((rect.width.min(rect.height) as f32) * 0.22).clamp(6.0, 12.0);
    let alpha_mul = if dimmed { 0.35 } else { 1.0 };
    let (fill_rgb, fill_a, rim_rgb, rim_a) = if pal.is_dark {
        if hovered {
            ([255u8, 255, 255], 34.0, [255u8, 255, 255], 54.0)
        } else {
            ([255u8, 255, 255], 16.0, [255u8, 255, 255], 28.0)
        }
    } else if hovered {
        ([15u8, 23, 42], 24.0, [15u8, 23, 42], 38.0)
    } else {
        ([255u8, 255, 255], 150.0, [15u8, 23, 42], 24.0)
    };
    let x0 = rect.x.max(0);
    let y0 = rect.y.max(0);
    let x1 = (rect.x + i32::try_from(rect.width).unwrap_or(0))
        .min(i32::try_from(frame.width).unwrap_or(0));
    let y1 = (rect.y + i32::try_from(rect.height).unwrap_or(0))
        .min(i32::try_from(frame.height).unwrap_or(0));
    for py in y0..y1 {
        for px in x0..x1 {
            let dist = rounded_rect_sdf(px as f32 + 0.5, py as f32 + 0.5, rect, radius);
            if dist <= 0.6 {
                let cov = (0.6 - dist).clamp(0.0, 1.0) * alpha_mul;
                blend_pixel(
                    frame,
                    px,
                    py,
                    [fill_rgb[0], fill_rgb[1], fill_rgb[2], (fill_a * cov) as u8],
                );
                if dist > -1.2 {
                    blend_pixel(
                        frame,
                        px,
                        py,
                        [rim_rgb[0], rim_rgb[1], rim_rgb[2], (rim_a * cov) as u8],
                    );
                }
            }
        }
    }
}

pub(crate) fn draw_settings_button(
    frame: &mut BgraFrame,
    rect: Rect,
    hovered: bool,
    pal: &asrbox_platform_api::FluentPalette,
) {
    if pal.design_theme.is_glass() {
        draw_icon_tile(frame, rect, hovered, false, pal);
        let pad = (rect.width.min(rect.height) as i32 / 10).max(2);
        let size = (rect.width.min(rect.height) as i32 - pad * 2).max(16);
        asrbox_platform_api::draw_asr_glass_logo(
            frame,
            rect.x + pad,
            rect.y + pad,
            size,
            pal.is_dark,
            true,
        );
        return;
    }
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
        let bg = (56.0 - 16.0 * t).round() as u8;
        for px in x0..x1 {
            let (fx, fy) = (px as f32 + 0.5, py as f32 + 0.5);
            let dist = rounded_rect_sdf(fx, fy, rect, radius);
            if dist <= 0.6 {
                let cov = (0.6 - dist).clamp(0.0, 1.0);
                blend_pixel(frame, px, py, [bg, bg + 2, bg + 7, (240.0 * cov) as u8]);
                if dist > -1.3 {
                    blend_pixel(frame, px, py, [232, 122, 72, (92.0 * cov) as u8]);
                }
                let gear_a = sample_gear_alpha(fx - cx, fy - cy, unit);
                if gear_a > 0 {
                    blend_pixel(frame, px, py, [242, 238, 234, gear_a]);
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

pub(crate) fn draw_fallback_tile(frame: &mut BgraFrame, rect: Rect, alpha_mul: f32) {
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

pub(crate) fn draw_folder_badge(
    frame: &mut BgraFrame,
    tile: Rect,
    alpha_mul: f32,
    pal: &asrbox_platform_api::FluentPalette,
) {
    let bw = (tile.width / 3).clamp(11, 18);
    let bh = (bw * 3 / 4).max(9);
    let badge = Rect {
        x: tile.x + tile.width as i32 - bw as i32 - 2,
        y: tile.y + tile.height as i32 - bh as i32 - 2,
        width: bw,
        height: bh,
    };
    let tab = Rect {
        x: badge.x,
        y: badge.y - 2,
        width: (bw / 2).max(5),
        height: 4,
    };
    let fill = pal.accent;
    let rim = if pal.is_dark {
        [248, 250, 252]
    } else {
        [15, 23, 42]
    };
    for rect in [tab, badge] {
        let x0 = rect.x.max(0);
        let y0 = rect.y.max(0);
        let x1 = (rect.x + rect.width as i32).min(frame.width as i32);
        let y1 = (rect.y + rect.height as i32).min(frame.height as i32);
        for py in y0..y1 {
            for px in x0..x1 {
                let dist = rounded_rect_sdf(px as f32 + 0.5, py as f32 + 0.5, rect, 2.5);
                if dist <= 0.5 {
                    let cov = (0.5 - dist).clamp(0.0, 1.0) * alpha_mul;
                    blend_pixel(
                        frame,
                        px,
                        py,
                        [fill[0], fill[1], fill[2], (235.0 * cov) as u8],
                    );
                    if dist > -1.0 {
                        blend_pixel(frame, px, py, [rim[0], rim[1], rim[2], (110.0 * cov) as u8]);
                    }
                }
            }
        }
    }
}
