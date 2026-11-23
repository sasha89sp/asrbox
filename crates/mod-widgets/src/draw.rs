use asrbox_platform_api::BgraFrame;

pub const CARD_RADIUS: f32 = 16.0;

pub fn fill_macos_glass_card(frame: &mut BgraFrame, opacity_percent: u8) {
    let w = frame.width as i32;
    let h = frame.height as i32;
    if w <= 0 || h <= 0 {
        return;
    }
    let pct = u16::from(opacity_percent.clamp(20, 100));
    let base_alpha = ((pct * 244) / 100).clamp(48, 248) as u8;
    let rim_alpha = ((u16::from(base_alpha) * 46) / 255).clamp(22, 60) as u8;
    let top_glow = ((u16::from(base_alpha) * 26) / 255).clamp(12, 34) as u8;

    let wf = w as f32;
    let hf = h as f32;
    for y in 0..h {
        let t = (y as f32) / hf.max(1.0);
        let r = (19.0 + t * 8.0) as u8;
        let g = (25.0 + t * 9.0) as u8;
        let b = (32.0 + t * 10.0) as u8;
        for x in 0..w {
            let d = rounded_rect_sdf(x as f32 + 0.5, y as f32 + 0.5, wf, hf, CARD_RADIUS);
            let cov = (0.5 - d).clamp(0.0, 1.0);
            if cov <= 0.0 {
                continue;
            }
            let a = (f32::from(base_alpha) * cov).round() as u8;
            blend_pixel(frame, x, y, [r, g, b, a]);
            if d > -1.35 && d < 0.45 {
                let extra = if y < 22 { top_glow / 2 } else { 0 };
                let ba = (f32::from(rim_alpha.saturating_add(extra)) * cov).round() as u8;
                blend_pixel(frame, x, y, [255, 255, 255, ba]);
            }
        }
    }
}

pub fn fill_glow_bar(frame: &mut BgraFrame, rx: i32, ry: i32, rw: i32, rh: i32, pct: u8) {
    if rw < 10 || rh < 4 {
        return;
    }
    let rad = (rh as f32) * 0.5;
    fill_rounded_rect(frame, rx, ry, rw, rh, rad, [255, 255, 255, 32]);
    let fill_w = ((i32::from(pct.min(100)) * rw) / 100).max(if pct > 0 { rh.min(rw) } else { 0 });
    if fill_w <= 0 {
        return;
    }
    // Soft cyan-blue outer halo
    fill_rounded_rect(
        frame,
        rx - 1,
        ry - 1,
        fill_w + 2,
        rh + 2,
        rad + 1.0,
        [88, 206, 255, 58],
    );
    // Crisp horizontal gradient pill fill
    let fw = fill_w as f32;
    let fh = rh as f32;
    for dy in 0..rh {
        for dx in 0..fill_w {
            let d = rounded_rect_sdf(dx as f32 + 0.5, dy as f32 + 0.5, fw, fh, rad);
            let cov = (0.5 - d).clamp(0.0, 1.0);
            if cov <= 0.0 {
                continue;
            }
            let t = (dx as f32) / fw.max(1.0);
            let cr = (70.0 + t * 82.0) as u8;
            let cg = (186.0 + t * 48.0) as u8;
            let cb = 255u8;
            let ca = (245.0 * cov).round() as u8;
            blend_pixel(frame, rx + dx, ry + dy, [cr, cg, cb, ca]);
        }
    }
}

pub fn fill_pill_badge(frame: &mut BgraFrame, rx: i32, ry: i32, rw: i32, rh: i32) {
    if rw < 8 || rh < 8 {
        return;
    }
    let rad = 7.0f32.min((rh as f32) * 0.45);
    let wf = rw as f32;
    let hf = rh as f32;
    for dy in 0..rh {
        for dx in 0..rw {
            let d = rounded_rect_sdf(dx as f32 + 0.5, dy as f32 + 0.5, wf, hf, rad);
            let cov = (0.5 - d).clamp(0.0, 1.0);
            if cov <= 0.0 {
                continue;
            }
            let bg_a = (30.0 * cov).round() as u8;
            blend_pixel(frame, rx + dx, ry + dy, [220, 236, 252, bg_a]);
            if d > -1.2 && d < 0.4 {
                let rim_a = (48.0 * cov).round() as u8;
                blend_pixel(frame, rx + dx, ry + dy, [255, 255, 255, rim_a]);
            }
        }
    }
}

pub fn draw_resize_grip(frame: &mut BgraFrame) {
    let w = frame.width as i32;
    let h = frame.height as i32;
    let col = [210, 224, 240, 82];
    for i in 0..3 {
        let off = 7 + i * 4;
        fill_rect(frame, w - off, h - 7, 2, 2, col);
        if i < 2 {
            fill_rect(frame, w - off, h - 11, 2, 2, col);
        }
    }
    fill_rect(frame, w - 7, h - 15, 2, 2, col);
}

pub fn draw_calendar_badge(frame: &mut BgraFrame, x: i32, y: i32) {
    fill_rounded_rect(frame, x, y, 22, 22, 6.0, [232, 238, 246, 250]);
    fill_rounded_rect(frame, x, y, 22, 7, 4.0, [78, 178, 255, 255]);
    fill_rect(frame, x, y + 4, 22, 4, [78, 178, 255, 255]);
    let dot = [45, 58, 74, 255];
    fill_rect(frame, x + 5, y + 11, 4, 3, dot);
    fill_rect(frame, x + 13, y + 11, 4, 3, dot);
    fill_rect(frame, x + 5, y + 16, 4, 3, dot);
    fill_rect(frame, x + 13, y + 16, 4, 3, dot);
}

pub fn draw_triangle_up(frame: &mut BgraFrame, cx: i32, top_y: i32, rgba: [u8; 4]) {
    for row in 0..6 {
        fill_rect(frame, cx - row, top_y + row, row * 2 + 1, 1, rgba);
    }
}

pub fn draw_triangle_down(frame: &mut BgraFrame, cx: i32, top_y: i32, rgba: [u8; 4]) {
    for row in 0..6 {
        let half = 5 - row;
        fill_rect(frame, cx - half, top_y + row, half * 2 + 1, 1, rgba);
    }
}

pub fn draw_hline(frame: &mut BgraFrame, y: i32, rgba: [u8; 4]) {
    fill_rect(frame, 14, y, frame.width as i32 - 28, 1, rgba);
}

pub fn draw_vline(frame: &mut BgraFrame, x: i32, y0: i32, y1: i32, rgba: [u8; 4]) {
    if y1 > y0 {
        fill_rect(frame, x, y0, 1, y1 - y0, rgba);
    }
}

pub fn fill_rounded_rect(
    frame: &mut BgraFrame,
    rx: i32,
    ry: i32,
    rw: i32,
    rh: i32,
    rad: f32,
    rgba: [u8; 4],
) {
    for dy in 0..rh {
        for dx in 0..rw {
            let px = dx as f32 + 0.5;
            let py = dy as f32 + 0.5;
            let d = rounded_rect_sdf(px, py, rw as f32, rh as f32, rad);
            let cov = (0.5 - d).clamp(0.0, 1.0);
            if cov > 0.0 {
                let a = (f32::from(rgba[3]) * cov).round() as u8;
                blend_pixel(frame, rx + dx, ry + dy, [rgba[0], rgba[1], rgba[2], a]);
            }
        }
    }
}

pub fn fill_rect(frame: &mut BgraFrame, rx: i32, ry: i32, rw: i32, rh: i32, rgba: [u8; 4]) {
    for dy in 0..rh {
        for dx in 0..rw {
            blend_pixel(frame, rx + dx, ry + dy, rgba);
        }
    }
}

fn rounded_rect_sdf(px: f32, py: f32, w: f32, h: f32, r: f32) -> f32 {
    let qx = (px - w * 0.5).abs() - (w * 0.5 - r);
    let qy = (py - h * 0.5).abs() - (h * 0.5 - r);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

fn blend_pixel(frame: &mut BgraFrame, x: i32, y: i32, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x >= frame.width as i32 || y >= frame.height as i32 {
        return;
    }
    let idx = ((y as usize) * (frame.width as usize) + (x as usize)) * 4;
    let sa = f32::from(rgba[3]) / 255.0;
    let inv = 1.0 - sa;
    let sr = f32::from(rgba[0]) * sa;
    let sg = f32::from(rgba[1]) * sa;
    let sb = f32::from(rgba[2]) * sa;

    let db = f32::from(frame.pixels[idx]);
    let dg = f32::from(frame.pixels[idx + 1]);
    let dr = f32::from(frame.pixels[idx + 2]);
    let da = f32::from(frame.pixels[idx + 3]);

    frame.pixels[idx] = (sb + db * inv).round().clamp(0.0, 255.0) as u8;
    frame.pixels[idx + 1] = (sg + dg * inv).round().clamp(0.0, 255.0) as u8;
    frame.pixels[idx + 2] = (sr + dr * inv).round().clamp(0.0, 255.0) as u8;
    frame.pixels[idx + 3] = (sa * 255.0 + da * inv).round().clamp(0.0, 255.0) as u8;
}
