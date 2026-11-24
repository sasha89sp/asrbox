use asrbox_platform_api::{BgraFrame, FluentPalette, TextAlign, WidgetTextCmd};

pub const CARD_RADIUS: f32 = 10.0;

pub fn fill_overlay_card(
    frame: &mut BgraFrame,
    opacity_percent: u8,
    setup_mode: bool,
    pal: &FluentPalette,
) {
    let w = frame.width as i32;
    let h = frame.height as i32;
    if w <= 0 || h <= 0 {
        return;
    }
    let pct = u16::from(opacity_percent.clamp(20, 100));
    let base_alpha = ((pct * 238) / 100).clamp(44, 245) as u8;
    let rim_alpha = if setup_mode {
        235u8
    } else {
        ((u16::from(base_alpha) * 160) / 255).clamp(40, 190) as u8
    };
    let border_w = if setup_mode { 2.0 } else { 1.1 };
    let acc = pal.accent;
    let brd = pal.card_border;

    for y in 0..h {
        let fy = (y as f32) / (h.max(1) as f32);
        // High-contrast HUD dark surface (#202020 -> #191919) for crisp readability over games.
        let r = (32.0 - fy * 7.0) as u8;
        let g = (32.0 - fy * 7.0) as u8;
        let b = (34.0 - fy * 7.0) as u8;
        for x in 0..w {
            let sdf = rounded_rect_sdf(
                x as f32 + 0.5,
                y as f32 + 0.5,
                w as f32,
                h as f32,
                CARD_RADIUS,
            );
            if sdf > 0.75 {
                continue;
            }
            let cov = (0.75 - sdf).clamp(0.0, 1.0);
            let a = (f32::from(base_alpha) * cov).round() as u8;
            blend_pixel(frame, x, y, [r, g, b, a]);
            if sdf > -border_w {
                let rim = if setup_mode {
                    [
                        acc[0],
                        acc[1],
                        acc[2],
                        (f32::from(rim_alpha) * cov).round() as u8,
                    ]
                } else {
                    [
                        brd[0],
                        brd[1],
                        brd[2],
                        (f32::from(rim_alpha) * cov).round() as u8,
                    ]
                };
                blend_pixel(frame, x, y, rim);
            }
        }
    }
    if setup_mode {
        draw_resize_grip(frame, acc);
    }
}

pub fn fill_rounded_rect(
    frame: &mut BgraFrame,
    rect: (i32, i32, i32, i32),
    radius: f32,
    rgba: [u8; 4],
) {
    let (rx, ry, rw, rh) = rect;
    if rw <= 0 || rh <= 0 || rgba[3] == 0 {
        return;
    }
    let rad = radius.min(rw as f32 * 0.5).min(rh as f32 * 0.5);
    for dy in 0..rh {
        for dx in 0..rw {
            let sdf = rounded_rect_sdf(dx as f32 + 0.5, dy as f32 + 0.5, rw as f32, rh as f32, rad);
            if sdf <= 0.6 {
                let cov = (0.6 - sdf).clamp(0.0, 1.0);
                let a = (f32::from(rgba[3]) * cov).round() as u8;
                blend_pixel(frame, rx + dx, ry + dy, [rgba[0], rgba[1], rgba[2], a]);
            }
        }
    }
}

pub fn fill_progress_bar(
    frame: &mut BgraFrame,
    rect: (i32, i32, i32, i32),
    pct: u8,
    fill_rgba: [u8; 4],
    track_rgba: [u8; 4],
) {
    let (rx, ry, rw, rh) = rect;
    let rad = (rh as f32) * 0.5;
    fill_rounded_rect(frame, (rx, ry, rw, rh), rad, track_rgba);
    let fill_w = ((rw as f32) * (f32::from(pct.min(100)) / 100.0)).round() as i32;
    if fill_w >= rh.max(2) {
        fill_rounded_rect(frame, (rx, ry, fill_w.min(rw), rh), rad, fill_rgba);
    } else if fill_w > 0 {
        fill_circle(
            frame,
            rx + (rh / 2).max(1),
            ry + (rh / 2),
            (rh as f32 * 0.5).max(1.5),
            fill_rgba,
        );
    }
}

pub fn fill_circle(frame: &mut BgraFrame, cx: i32, cy: i32, radius: f32, rgba: [u8; 4]) {
    let r_i = radius.ceil() as i32 + 1;
    for dy in -r_i..=r_i {
        for dx in -r_i..=r_i {
            let d = (dx as f32).hypot(dy as f32) - radius;
            if d <= 0.6 {
                let cov = (0.6 - d).clamp(0.0, 1.0);
                let a = (f32::from(rgba[3]) * cov).round() as u8;
                blend_pixel(frame, cx + dx, cy + dy, [rgba[0], rgba[1], rgba[2], a]);
            }
        }
    }
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
        width: r.2.max(1),
        height: r.3.max(1),
        text: text.to_owned(),
        font_size_px: font.0.max(9),
        bold: font.1,
        color_rgba,
        align,
    });
}

fn draw_resize_grip(frame: &mut BgraFrame, accent: [u8; 4]) {
    let w = frame.width as i32;
    let h = frame.height as i32;
    let col = [accent[0], accent[1], accent[2], 220];
    for step in [5, 9, 13] {
        for i in 0..step {
            blend_pixel(frame, w - 4 - i, h - 4 - (step - i), col);
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
