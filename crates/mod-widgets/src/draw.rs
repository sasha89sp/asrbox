use asrbox_platform_api::{BgraFrame, FluentPalette, TextAlign, WidgetTextCmd};

pub const CARD_RADIUS: f32 = 10.0;

pub fn fill_macos_glass_card(frame: &mut BgraFrame, opacity_percent: u8, pal: &FluentPalette) {
    let w = frame.width as i32;
    let h = frame.height as i32;
    if w <= 0 || h <= 0 {
        return;
    }
    let pct = u16::from(opacity_percent.clamp(20, 100));
    let base_alpha = ((pct * 246) / 100).clamp(52, 250) as u8;
    let rim_alpha = if pal.is_dark {
        ((u16::from(base_alpha) * 195) / 255).clamp(48, 220) as u8
    } else {
        ((u16::from(base_alpha) * 235) / 255).clamp(90, 245) as u8
    };
    let bg = pal.window_bg;
    let border = pal.card_border;

    let card_rad = if pal.design_theme.is_glass() {
        12.0
    } else {
        CARD_RADIUS
    };
    let wf = w as f32;
    let hf = h as f32;
    for y in 0..h {
        let t = (y as f32) / hf.max(1.0);
        let shift = if pal.is_dark {
            ((1.0 - t) * 4.0) as u8
        } else {
            ((1.0 - t) * 3.0) as u8
        };
        let r = bg[0].saturating_add(shift);
        let g = bg[1].saturating_add(shift);
        let b = bg[2].saturating_add(shift);
        for x in 0..w {
            let d = rounded_rect_sdf(x as f32 + 0.5, y as f32 + 0.5, wf, hf, card_rad);
            let cov = (0.5 - d).clamp(0.0, 1.0);
            if cov <= 0.0 {
                continue;
            }
            let a = (f32::from(base_alpha) * cov).round() as u8;
            blend_pixel(frame, x, y, [r, g, b, a]);
            if d > -1.25 && d < 0.45 {
                let ba = (f32::from(rim_alpha) * cov).round() as u8;
                blend_pixel(frame, x, y, [border[0], border[1], border[2], ba]);
            }
        }
    }
}

pub fn fill_glow_bar(
    frame: &mut BgraFrame,
    rx: i32,
    ry: i32,
    rw: i32,
    rh: i32,
    pct: u8,
    pal: &FluentPalette,
) {
    if rw < 10 || rh < 4 {
        return;
    }
    let rad = (rh as f32) * 0.5;
    let track = if pal.is_dark {
        [255, 255, 255, 34]
    } else {
        [0, 0, 0, 24]
    };
    fill_rounded_rect(frame, rx, ry, rw, rh, rad, track);
    let fill_w = ((i32::from(pct.min(100)) * rw) / 100).max(if pct > 0 { rh.min(rw) } else { 0 });
    if fill_w <= 0 {
        return;
    }
    let acc = pal.accent;
    let hov = pal.accent_hover;
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
            let cr = (f32::from(acc[0]) + (f32::from(hov[0]) - f32::from(acc[0])) * t) as u8;
            let cg = (f32::from(acc[1]) + (f32::from(hov[1]) - f32::from(acc[1])) * t) as u8;
            let cb = (f32::from(acc[2]) + (f32::from(hov[2]) - f32::from(acc[2])) * t) as u8;
            let ca = (245.0 * cov).round() as u8;
            blend_pixel(frame, rx + dx, ry + dy, [cr, cg, cb, ca]);
        }
    }
}

pub fn fill_pill_badge(
    frame: &mut BgraFrame,
    rx: i32,
    ry: i32,
    rw: i32,
    rh: i32,
    pal: &FluentPalette,
) {
    if rw < 8 || rh < 8 {
        return;
    }
    let rad = 6.0f32.min((rh as f32) * 0.45);
    let wf = rw as f32;
    let hf = rh as f32;
    let acc = pal.accent;
    for dy in 0..rh {
        for dx in 0..rw {
            let d = rounded_rect_sdf(dx as f32 + 0.5, dy as f32 + 0.5, wf, hf, rad);
            let cov = (0.5 - d).clamp(0.0, 1.0);
            if cov <= 0.0 {
                continue;
            }
            let bg_a = ((if pal.is_dark { 46.0 } else { 28.0 }) * cov).round() as u8;
            blend_pixel(frame, rx + dx, ry + dy, [acc[0], acc[1], acc[2], bg_a]);
            if d > -1.15 && d < 0.4 {
                let rim_a = (110.0 * cov).round() as u8;
                blend_pixel(frame, rx + dx, ry + dy, [acc[0], acc[1], acc[2], rim_a]);
            }
        }
    }
}

pub fn draw_resize_grip(frame: &mut BgraFrame, pal: &FluentPalette) {
    let w = frame.width as i32;
    let h = frame.height as i32;
    let m = pal.text_muted;
    let col = [m[0], m[1], m[2], 125];
    for i in 0..3 {
        let off = 7 + i * 4;
        fill_rect(frame, w - off, h - 7, 2, 2, col);
        if i < 2 {
            fill_rect(frame, w - off, h - 11, 2, 2, col);
        }
    }
    fill_rect(frame, w - 7, h - 15, 2, 2, col);
}

pub fn draw_calendar_badge(frame: &mut BgraFrame, x: i32, y: i32, pal: &FluentPalette) {
    let body = if pal.is_dark {
        [244, 246, 250, 250]
    } else {
        [255, 255, 255, 255]
    };
    fill_rounded_rect(frame, x, y, 22, 22, 6.0, body);
    fill_rounded_rect(frame, x, y, 22, 7, 4.0, pal.accent);
    fill_rect(frame, x, y + 4, 22, 4, pal.accent);
    let dot = [64, 68, 78, 255];
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

pub fn push_txt(
    texts: &mut Vec<WidgetTextCmd>,
    r: (i32, i32, i32, i32),
    text: &str,
    font: (i32, bool),
    color_rgba: [u8; 4],
    align: TextAlign,
) {
    texts.push(WidgetTextCmd {
        x: r.0,
        y: r.1,
        width: r.2,
        height: r.3,
        text: text.to_owned(),
        font_size_px: font.0,
        bold: font.1,
        color_rgba,
        align,
    });
}

pub fn arrow_color(enabled: bool, hovered: bool, pal: &FluentPalette) -> [u8; 4] {
    match (enabled, hovered) {
        (false, _) => [pal.text_muted[0], pal.text_muted[1], pal.text_muted[2], 110],
        (true, true) => pal.text_primary,
        (true, false) => pal.text_secondary,
    }
}
