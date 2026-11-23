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
