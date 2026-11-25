use crate::BgraFrame;

/// Draws the «Стеклянный блок ASR» (Glass Stack) brand logo into a `BgraFrame`.
/// Two overlapping squircles (22% corner radius): back layer `#0284C7 -> #38BDF8`,
/// front layer frosted glass with 1px luminous border, centered bold `ASR` (`#0F172A`).
pub fn draw_asr_glass_logo(
    frame: &mut BgraFrame,
    x: i32,
    y: i32,
    size: i32,
    dark_bg: bool,
    premultiplied: bool,
) {
    if size < 8 {
        return;
    }
    let s = size as f32;
    let sq = s * 0.78;
    let rad = sq * 0.22;
    let back_x = x as f32 + s * 0.05;
    let back_y = y as f32 + s * 0.05;
    let front_x = x as f32 + s * 0.17;
    let front_y = y as f32 + s * 0.17;

    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + size).min(i32::try_from(frame.width).unwrap_or(0));
    let y1 = (y + size).min(i32::try_from(frame.height).unwrap_or(0));

    for py in y0..y1 {
        let fy = py as f32 + 0.5;
        for px in x0..x1 {
            let fx = px as f32 + 0.5;
            // Back squircle: diagonal gradient #0284C7 -> #38BDF8 with crisp 1px cyan rim
            let d_back = sdf_round_rect(fx, fy, back_x, back_y, sq, sq, rad);
            if d_back < 0.8 {
                let cov = (0.5 - d_back).clamp(0.0, 1.0);
                let t = (((fx - back_x) / sq) * 0.55 + ((fy - back_y) / sq) * 0.45).clamp(0.0, 1.0);
                let r = lerp_f(2.0, 56.0, t);
                let g = lerp_f(132.0, 189.0, t);
                let b = lerp_f(199.0, 248.0, t);
                let alpha = (cov * 0.88 * 255.0).round().clamp(0.0, 255.0) as u8;
                blend_bgra(
                    frame,
                    px,
                    py,
                    [r as u8, g as u8, b as u8, alpha],
                    premultiplied,
                );
                if d_back > -1.35 {
                    let edge = ((d_back + 1.35) / 1.35).clamp(0.0, 1.0) * cov;
                    let ea = (edge * 0.42 * 255.0).round().clamp(0.0, 255.0) as u8;
                    blend_bgra(frame, px, py, [186, 234, 255, ea], premultiplied);
                }
            }

            // Front frosted glass squircle
            let d_front = sdf_round_rect(fx, fy, front_x, front_y, sq, sq, rad);
            if d_front < 0.8 {
                let cov = (0.5 - d_front).clamp(0.0, 1.0);
                let t = (((fx - front_x) / sq) * 0.5 + ((fy - front_y) / sq) * 0.5).clamp(0.0, 1.0);
                let (r, g, b, base_a) = if dark_bg {
                    (
                        lerp_f(178.0, 96.0, t),
                        lerp_f(236.0, 198.0, t),
                        lerp_f(255.0, 252.0, t),
                        0.66,
                    )
                } else {
                    (
                        lerp_f(236.0, 152.0, t),
                        lerp_f(248.0, 224.0, t),
                        lerp_f(255.0, 255.0, t),
                        0.76,
                    )
                };
                let alpha = (cov * base_a * 255.0).round().clamp(0.0, 255.0) as u8;
                blend_bgra(
                    frame,
                    px,
                    py,
                    [r as u8, g as u8, b as u8, alpha],
                    premultiplied,
                );

                // Luminous 1px glass border
                if d_front > -1.4 {
                    let edge = ((d_front + 1.4) / 1.4).clamp(0.0, 1.0) * cov;
                    let ea = (edge * 0.58 * 255.0).round().clamp(0.0, 255.0) as u8;
                    blend_bgra(frame, px, py, [255, 255, 255, ea], premultiplied);
                }

                // Centered bold geometric "ASR" glyph (#0F172A)
                if size >= 20 {
                    let u = (fx - front_x) / sq;
                    let v = (fy - front_y) / sq;
                    let glyph_cov = sample_asr_glyph(u, v, sq);
                    if glyph_cov > 0.01 {
                        let ga = (glyph_cov * cov * 255.0).round().clamp(0.0, 255.0) as u8;
                        blend_bgra(frame, px, py, [15, 23, 42, ga], premultiplied);
                    }
                }
            }
        }
    }
}

/// Evaluates anti-aliased signed-distance coverage for bold geometric `ASR` inside `[0,1]x[0,1]`.
pub fn sample_asr_glyph(u: f32, v: f32, sq_px: f32) -> f32 {
    if !(0.10..=0.90).contains(&u) || !(0.26..=0.74).contains(&v) {
        return 0.0;
    }
    let stroke = (0.044_f32).max(1.15 / sq_px.max(16.0));
    let mut d = 1.0_f32;

    // 'A' in u: 0.14..0.36, v: 0.34..0.66
    d = d.min(dist_seg(u, v, 0.15, 0.65, 0.25, 0.35));
    d = d.min(dist_seg(u, v, 0.25, 0.35, 0.35, 0.65));
    d = d.min(dist_seg(u, v, 0.19, 0.55, 0.31, 0.55));

    // 'S' in u: 0.40..0.59, v: 0.34..0.66
    d = d.min(dist_seg(u, v, 0.57, 0.37, 0.44, 0.37));
    d = d.min(dist_seg(u, v, 0.44, 0.37, 0.42, 0.43));
    d = d.min(dist_seg(u, v, 0.42, 0.43, 0.44, 0.49));
    d = d.min(dist_seg(u, v, 0.44, 0.49, 0.55, 0.51));
    d = d.min(dist_seg(u, v, 0.55, 0.51, 0.58, 0.57));
    d = d.min(dist_seg(u, v, 0.58, 0.57, 0.55, 0.64));
    d = d.min(dist_seg(u, v, 0.55, 0.64, 0.41, 0.64));

    // 'R' in u: 0.65..0.85, v: 0.34..0.66
    d = d.min(dist_seg(u, v, 0.66, 0.35, 0.66, 0.65));
    d = d.min(dist_seg(u, v, 0.66, 0.36, 0.78, 0.36));
    d = d.min(dist_seg(u, v, 0.78, 0.36, 0.83, 0.42));
    d = d.min(dist_seg(u, v, 0.83, 0.42, 0.78, 0.50));
    d = d.min(dist_seg(u, v, 0.78, 0.50, 0.66, 0.50));
    d = d.min(dist_seg(u, v, 0.74, 0.50, 0.84, 0.65));

    let px_dist = (d - stroke) * sq_px;
    (0.65 - px_dist).clamp(0.0, 1.0)
}

fn sdf_round_rect(px: f32, py: f32, x: f32, y: f32, w: f32, h: f32, r: f32) -> f32 {
    let cx = x + w * 0.5;
    let cy = y + h * 0.5;
    let qx = (px - cx).abs() - (w * 0.5 - r);
    let qy = (py - cy).abs() - (h * 0.5 - r);
    let out = (qx.max(0.0).powi(2) + qy.max(0.0).powi(2)).sqrt();
    let inside = qx.max(qy).min(0.0);
    out + inside - r
}

fn dist_seg(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let abx = bx - ax;
    let aby = by - ay;
    let apx = px - ax;
    let apy = py - ay;
    let denom = abx * abx + aby * aby;
    let t = if denom > 1e-6 {
        ((apx * abx + apy * aby) / denom).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let dx = apx - abx * t;
    let dy = apy - aby * t;
    (dx * dx + dy * dy).sqrt()
}

fn lerp_f(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn blend_bgra(frame: &mut BgraFrame, x: i32, y: i32, rgba: [u8; 4], premultiplied: bool) {
    let sa = f32::from(rgba[3]) / 255.0;
    if sa <= 0.0 {
        return;
    }
    let idx = ((y as usize) * (frame.width as usize) + (x as usize)) * 4;
    if idx + 3 >= frame.pixels.len() {
        return;
    }
    if premultiplied {
        let inv = 1.0 - sa;
        let db = f32::from(frame.pixels[idx]);
        let dg = f32::from(frame.pixels[idx + 1]);
        let dr = f32::from(frame.pixels[idx + 2]);
        let da = f32::from(frame.pixels[idx + 3]);
        frame.pixels[idx] = (f32::from(rgba[2]) * sa + db * inv)
            .round()
            .clamp(0.0, 255.0) as u8;
        frame.pixels[idx + 1] = (f32::from(rgba[1]) * sa + dg * inv)
            .round()
            .clamp(0.0, 255.0) as u8;
        frame.pixels[idx + 2] = (f32::from(rgba[0]) * sa + dr * inv)
            .round()
            .clamp(0.0, 255.0) as u8;
        frame.pixels[idx + 3] = (f32::from(rgba[3]) + da * inv).round().clamp(0.0, 255.0) as u8;
    } else {
        let inv = 1.0 - sa;
        frame.pixels[idx] = (f32::from(rgba[2]) * sa + f32::from(frame.pixels[idx]) * inv)
            .round()
            .clamp(0.0, 255.0) as u8;
        frame.pixels[idx + 1] = (f32::from(rgba[1]) * sa + f32::from(frame.pixels[idx + 1]) * inv)
            .round()
            .clamp(0.0, 255.0) as u8;
        frame.pixels[idx + 2] = (f32::from(rgba[0]) * sa + f32::from(frame.pixels[idx + 2]) * inv)
            .round()
            .clamp(0.0, 255.0) as u8;
        frame.pixels[idx + 3] = 255;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glass_logo_draws_visible_pixels() {
        let mut frame = BgraFrame::transparent(64, 64);
        draw_asr_glass_logo(&mut frame, 6, 6, 52, true, true);
        let non_zero = (3..frame.pixels.len())
            .step_by(4)
            .filter(|&idx| frame.pixels[idx] > 0)
            .count();
        assert!(non_zero > 1200);
    }
}
