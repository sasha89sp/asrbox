use asrbox_platform_api::BgraFrame;

const CARD_RADIUS: f32 = 14.0;

pub fn draw_calendar_badge(frame: &mut BgraFrame, x: i32, y: i32) {
    fill_rounded_rect(frame, x, y, 22, 22, 5.0, [229, 231, 235, 255]);
    fill_rounded_rect(frame, x, y, 22, 7, 4.0, [239, 68, 68, 255]);
    fill_rect(frame, x, y + 4, 22, 4, [239, 68, 68, 255]);
    let dot = [55, 65, 81, 255];
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
    fill_rect(frame, 10, y, frame.width as i32 - 20, 1, rgba);
}

pub fn fill_rounded_card(frame: &mut BgraFrame, fill: [u8; 4], border: [u8; 4]) {
    let w = frame.width as i32;
    let h = frame.height as i32;
    for y in 0..h {
        for x in 0..w {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let d = rounded_rect_sdf(px, py, w as f32, h as f32, CARD_RADIUS);
            let cov = (0.5 - d).clamp(0.0, 1.0);
            if cov <= 0.0 {
                continue;
            }
            let a = (f32::from(fill[3]) * cov).round() as u8;
            blend_pixel(frame, x, y, [fill[0], fill[1], fill[2], a]);
            if d > -1.3 && d < 0.4 {
                let ba = (f32::from(border[3]) * cov).round() as u8;
                blend_pixel(frame, x, y, [border[0], border[1], border[2], ba]);
            }
        }
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
