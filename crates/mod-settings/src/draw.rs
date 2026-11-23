use asrbox_platform_api::{BgraFrame, PixelRect, TextAlign, WidgetTextCmd};

pub fn fill_rect(frame: &mut BgraFrame, rect: PixelRect, rgba: [u8; 4]) {
    let x0 = rect.x.max(0);
    let y0 = rect.y.max(0);
    let x1 = rect
        .x
        .saturating_add(rect.width as i32)
        .min(frame.width as i32);
    let y1 = rect
        .y
        .saturating_add(rect.height as i32)
        .min(frame.height as i32);
    for y in y0..y1 {
        for x in x0..x1 {
            blend_px(frame, x as u32, y as u32, rgba);
        }
    }
}

pub fn fill_rounded_rect(frame: &mut BgraFrame, rect: PixelRect, radius: i32, rgba: [u8; 4]) {
    if rect.width == 0 || rect.height == 0 {
        return;
    }
    let r = radius
        .min((rect.width / 2) as i32)
        .min((rect.height / 2) as i32)
        .max(0);
    let x0 = rect.x.max(0);
    let y0 = rect.y.max(0);
    let x1 = rect
        .x
        .saturating_add(rect.width as i32)
        .min(frame.width as i32);
    let y1 = rect
        .y
        .saturating_add(rect.height as i32)
        .min(frame.height as i32);
    let r_f = r as f32;
    for y in y0..y1 {
        for x in x0..x1 {
            let cov = rounded_coverage(
                x - rect.x,
                y - rect.y,
                rect.width as i32,
                rect.height as i32,
                r_f,
            );
            if cov > 0.0 {
                let alpha = (f32::from(rgba[3]) * cov).round().clamp(0.0, 255.0) as u8;
                blend_px(
                    frame,
                    x as u32,
                    y as u32,
                    [rgba[0], rgba[1], rgba[2], alpha],
                );
            }
        }
    }
}

pub fn stroke_rounded_rect(frame: &mut BgraFrame, rect: PixelRect, radius: i32, rgba: [u8; 4]) {
    if rect.width < 2 || rect.height < 2 {
        return;
    }
    let r = radius
        .min((rect.width / 2) as i32)
        .min((rect.height / 2) as i32)
        .max(0) as f32;
    let x0 = rect.x.max(0);
    let y0 = rect.y.max(0);
    let x1 = rect
        .x
        .saturating_add(rect.width as i32)
        .min(frame.width as i32);
    let y1 = rect
        .y
        .saturating_add(rect.height as i32)
        .min(frame.height as i32);
    for y in y0..y1 {
        for x in x0..x1 {
            let lx = x - rect.x;
            let ly = y - rect.y;
            let outer = rounded_coverage(lx, ly, rect.width as i32, rect.height as i32, r);
            if outer <= 0.0 {
                continue;
            }
            let inner = rounded_coverage(
                lx - 1,
                ly - 1,
                rect.width as i32 - 2,
                rect.height as i32 - 2,
                (r - 1.0).max(0.0),
            );
            let ring = (outer - inner).clamp(0.0, 1.0);
            if ring > 0.05 {
                let a = (f32::from(rgba[3]) * ring).round().clamp(0.0, 255.0) as u8;
                blend_px(frame, x as u32, y as u32, [rgba[0], rgba[1], rgba[2], a]);
            }
        }
    }
}

pub fn draw_toggle(frame: &mut BgraFrame, rect: PixelRect, on: bool, hovered: bool) {
    let track = if on {
        if hovered {
            [56, 224, 98, 255]
        } else {
            [48, 209, 88, 255]
        }
    } else if hovered {
        [96, 96, 102, 255]
    } else {
        [72, 72, 78, 255]
    };
    let radius = (rect.height / 2) as i32;
    fill_rounded_rect(frame, rect, radius, track);
    let knob = rect.height.saturating_sub(4).max(8);
    let kx = if on {
        rect.x + rect.width as i32 - knob as i32 - 2
    } else {
        rect.x + 2
    };
    let knob_rect = PixelRect {
        x: kx,
        y: rect.y + 2,
        width: knob,
        height: knob,
    };
    fill_rounded_rect(frame, knob_rect, (knob / 2) as i32, [255, 255, 255, 255]);
}

pub fn draw_slider(frame: &mut BgraFrame, rect: PixelRect, ratio: f32, hovered: bool) {
    let clamped = ratio.clamp(0.0, 1.0);
    let track_h = 4u32;
    let track_y = rect.y + (rect.height as i32 - track_h as i32) / 2;
    let track = PixelRect {
        x: rect.x,
        y: track_y,
        width: rect.width,
        height: track_h,
    };
    fill_rounded_rect(frame, track, 2, [68, 68, 74, 255]);
    let fill_w = ((rect.width as f32) * clamped).round() as u32;
    if fill_w > 0 {
        let filled = PixelRect {
            x: rect.x,
            y: track_y,
            width: fill_w.min(rect.width),
            height: track_h,
        };
        fill_rounded_rect(frame, filled, 2, [10, 132, 255, 255]);
    }
    let knob = if hovered { 18u32 } else { 16u32 };
    let travel = rect.width.saturating_sub(knob) as f32;
    let kx = rect.x + (travel * clamped).round() as i32;
    let ky = rect.y + (rect.height as i32 - knob as i32) / 2;
    let knob_rect = PixelRect {
        x: kx,
        y: ky,
        width: knob,
        height: knob,
    };
    fill_rounded_rect(frame, knob_rect, (knob / 2) as i32, [255, 255, 255, 255]);
    stroke_rounded_rect(frame, knob_rect, (knob / 2) as i32, [20, 20, 24, 90]);
}

pub fn push_text(
    texts: &mut Vec<WidgetTextCmd>,
    rect: PixelRect,
    text: impl Into<String>,
    font_size_px: i32,
    bold: bool,
    color_rgba: [u8; 4],
    align: TextAlign,
) {
    texts.push(WidgetTextCmd {
        x: rect.x,
        y: rect.y,
        width: rect.width as i32,
        height: rect.height as i32,
        text: text.into(),
        font_size_px,
        bold,
        color_rgba,
        align,
    });
}

fn rounded_coverage(lx: i32, ly: i32, w: i32, h: i32, r: f32) -> f32 {
    if lx < 0 || ly < 0 || lx >= w || ly >= h {
        return 0.0;
    }
    if r <= 0.5 {
        return 1.0;
    }
    let px = lx as f32 + 0.5;
    let py = ly as f32 + 0.5;
    let cx = px.clamp(r, (w as f32 - r).max(r));
    let cy = py.clamp(r, (h as f32 - r).max(r));
    let dist = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
    (r + 0.5 - dist).clamp(0.0, 1.0)
}

fn blend_px(frame: &mut BgraFrame, x: u32, y: u32, rgba: [u8; 4]) {
    let sa = u32::from(rgba[3]);
    if sa == 0 {
        return;
    }
    let idx = ((y as usize) * (frame.width as usize) + (x as usize)) * 4;
    if idx + 3 >= frame.pixels.len() {
        return;
    }
    if sa == 255 {
        frame.pixels[idx] = rgba[2];
        frame.pixels[idx + 1] = rgba[1];
        frame.pixels[idx + 2] = rgba[0];
        frame.pixels[idx + 3] = 255;
        return;
    }
    let inv = 255 - sa;
    let db = u32::from(frame.pixels[idx]);
    let dg = u32::from(frame.pixels[idx + 1]);
    let dr = u32::from(frame.pixels[idx + 2]);
    frame.pixels[idx] = ((u32::from(rgba[2]) * sa + db * inv) / 255) as u8;
    frame.pixels[idx + 1] = ((u32::from(rgba[1]) * sa + dg * inv) / 255) as u8;
    frame.pixels[idx + 2] = ((u32::from(rgba[0]) * sa + dr * inv) / 255) as u8;
    frame.pixels[idx + 3] = 255;
}
