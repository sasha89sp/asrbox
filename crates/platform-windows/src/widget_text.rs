use std::ffi::c_void;

use asrbox_platform_api::{
    BgraFrame, FontRenderer, LocalDateTime, TextAlign, WidgetFrame, WidgetTextCmd,
};
use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, DrawTextW, GetDC, ReleaseDC,
    SelectObject, SetBkMode, SetTextColor, DIB_RGB_COLORS, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT,
    DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER, TRANSPARENT,
};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Time::{GetTimeZoneInformation, TIME_ZONE_INFORMATION};

use crate::dwrite_text::try_render_dwrite_cmd;
use crate::font_engine::{active_font_renderer, blend_cleartype_mask, get_or_create_ui_font};
use crate::icons::make_top_down_bmi;

pub(crate) fn query_local_time() -> LocalDateTime {
    // SAFETY: GetLocalTime and GetTimeZoneInformation write to valid stack structs.
    unsafe {
        let st = GetLocalTime();
        let mut tzi = TIME_ZONE_INFORMATION::default();
        let tz_id = GetTimeZoneInformation(&mut tzi);
        let extra = match tz_id {
            1 => tzi.StandardBias,
            2 => tzi.DaylightBias,
            _ => 0,
        };
        let utc_offset_min = -(tzi.Bias + extra);
        LocalDateTime {
            year: i32::from(st.wYear),
            month: u8::try_from(st.wMonth).unwrap_or(1),
            day: u8::try_from(st.wDay).unwrap_or(1),
            hour: u8::try_from(st.wHour).unwrap_or(0),
            minute: u8::try_from(st.wMinute).unwrap_or(0),
            second: u8::try_from(st.wSecond).unwrap_or(0),
            utc_offset_min,
        }
    }
}

pub(crate) fn composite_widget_frame(frame: &WidgetFrame) -> Result<BgraFrame, String> {
    let mut out = frame.bgra.clone();
    if frame.texts.is_empty() || out.width == 0 || out.height == 0 {
        return Ok(out);
    }
    let use_dwrite = active_font_renderer() == FontRenderer::DirectWrite;
    let mut pending_gdi: Vec<&WidgetTextCmd> = Vec::new();
    if use_dwrite {
        for cmd in &frame.texts {
            if !try_render_dwrite_cmd(&mut out, cmd) {
                pending_gdi.push(cmd);
            }
        }
        if pending_gdi.is_empty() {
            return Ok(out);
        }
    } else {
        pending_gdi.extend(frame.texts.iter());
    }
    composite_gdi_texts(&mut out, &pending_gdi)?;
    Ok(out)
}

fn composite_gdi_texts(out: &mut BgraFrame, cmds: &[&WidgetTextCmd]) -> Result<(), String> {
    let w = i32::try_from(out.width).map_err(|_| "widget width overflow".to_owned())?;
    let h = i32::try_from(out.height).map_err(|_| "widget height overflow".to_owned())?;
    let bmi = make_top_down_bmi(w, h);
    let mut bits: *mut c_void = std::ptr::null_mut();

    // SAFETY: Scratch DIBSection and GDI handles are owned locally and freed before return.
    unsafe {
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let dib = CreateDIBSection(Some(hdc_mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0);
        let hbmp = match dib {
            Ok(b) => b,
            Err(err) => {
                let _ = DeleteDC(hdc_mem);
                let _ = ReleaseDC(None, hdc_screen);
                return Err(format!("CreateDIBSection(text) failed: {err}"));
            }
        };
        let prev_bmp = SelectObject(hdc_mem, hbmp.into());
        SetBkMode(hdc_mem, TRANSPARENT);
        SetTextColor(hdc_mem, COLORREF(0x00FF_FFFF));

        if !bits.is_null() {
            let total_bytes = (out.width as usize) * (out.height as usize) * 4;
            let scratch = std::slice::from_raw_parts_mut(bits as *mut u8, total_bytes);
            for cmd in cmds {
                draw_and_blend_gdi_cmd(hdc_mem, scratch, out, cmd);
            }
        }

        let _ = SelectObject(hdc_mem, prev_bmp);
        let _ = DeleteObject(hbmp.into());
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);
    }
    Ok(())
}

unsafe fn draw_and_blend_gdi_cmd(
    hdc: windows::Win32::Graphics::Gdi::HDC,
    scratch: &mut [u8],
    out: &mut BgraFrame,
    cmd: &WidgetTextCmd,
) {
    let x0 = cmd.x.clamp(0, out.width as i32);
    let y0 = cmd.y.clamp(0, out.height as i32);
    let x1 = (cmd.x + cmd.width).clamp(x0, out.width as i32);
    let y1 = (cmd.y + cmd.height).clamp(y0, out.height as i32);
    if x1 <= x0 || y1 <= y0 || cmd.text.is_empty() {
        return;
    }

    clear_scratch_rect(scratch, out.width as usize, x0, y0, x1, y1);
    let font = get_or_create_ui_font(cmd.font_size_px, cmd.bold);
    let prev_font = SelectObject(hdc, font.into());

    let mut rect = RECT {
        left: x0,
        top: y0,
        right: x1,
        bottom: y1,
    };
    let align_flag = match cmd.align {
        TextAlign::Left => DT_LEFT,
        TextAlign::Center => DT_CENTER,
        TextAlign::Right => DT_RIGHT,
    };
    let flags = DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_NOPREFIX | align_flag;
    let mut wide: Vec<u16> = cmd.text.encode_utf16().collect();
    let _ = DrawTextW(hdc, &mut wide, &mut rect, flags);
    let _ = SelectObject(hdc, prev_font);

    let stride_bytes = (out.width as usize) * 4;
    blend_cleartype_mask(
        scratch,
        stride_bytes,
        out.height as usize,
        true,
        0,
        0,
        out,
        x0,
        y0,
        x1,
        y1,
        cmd.color_rgba,
        true,
    );
}

fn clear_scratch_rect(scratch: &mut [u8], stride_px: usize, x0: i32, y0: i32, x1: i32, y1: i32) {
    for y in y0..y1 {
        let row_start = ((y as usize) * stride_px + (x0 as usize)) * 4;
        let row_end = ((y as usize) * stride_px + (x1 as usize)) * 4;
        if row_end <= scratch.len() {
            scratch[row_start..row_end].fill(0);
        }
    }
}

pub(crate) fn render_tooltip_frame(
    text: &str,
    palette: &asrbox_platform_api::FluentPalette,
) -> Result<BgraFrame, String> {
    const SHADOW: i32 = 4;
    let lines: Vec<&str> = text.lines().take(5).collect();
    let single = lines.len() <= 1;
    let max_chars = lines.iter().map(|l| l.chars().count()).max().unwrap_or(4);
    let inner_w = if single {
        ((max_chars as u32) * 7 + 22).clamp(64, 300)
    } else {
        ((max_chars as u32) * 7 + 24).clamp(140, 320)
    };
    let inner_h = if single {
        26
    } else {
        (lines.len() as u32) * 18 + 10
    };
    let total_w = inner_w + (SHADOW as u32) * 2;
    let total_h = inner_h + (SHADOW as u32) * 2;
    let mut bgra = BgraFrame::transparent(total_w, total_h);
    fill_tooltip_capsule(&mut bgra, SHADOW, inner_w as i32, inner_h as i32, palette);
    let mut texts = Vec::with_capacity(lines.len());
    for (idx, line) in lines.iter().enumerate() {
        let y = if single {
            SHADOW
        } else {
            SHADOW + 5 + (idx as i32) * 18
        };
        texts.push(WidgetTextCmd {
            x: SHADOW + 10,
            y,
            width: (inner_w as i32) - 20,
            height: if single { inner_h as i32 } else { 18 },
            text: (*line).to_owned(),
            font_size_px: 12,
            bold: idx == 0 && !single,
            color_rgba: if idx == 0 {
                palette.text_primary
            } else {
                palette.text_secondary
            },
            align: if single {
                TextAlign::Center
            } else {
                TextAlign::Left
            },
        });
    }
    composite_widget_frame(&WidgetFrame { bgra, texts })
}

fn fill_tooltip_capsule(
    frame: &mut BgraFrame,
    pad: i32,
    pw: i32,
    ph: i32,
    palette: &asrbox_platform_api::FluentPalette,
) {
    let w = frame.width as i32;
    let h = frame.height as i32;
    let cx = pad as f32 + pw as f32 * 0.5;
    let cy = pad as f32 + ph as f32 * 0.5;
    let hx = pw as f32 * 0.5;
    let hy = ph as f32 * 0.5;
    let rad = 6.0f32;
    let shadow_a = if palette.is_dark { 72.0f32 } else { 36.0f32 };
    let top_spec = if palette.is_dark {
        [255u8, 255, 255, 36]
    } else {
        [255u8, 255, 255, 200]
    };
    for y in 0..h {
        let is_top = y == pad + 1;
        for x in 0..w {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let d_sh = tip_rounded_sdf(fx - cx, fy - (cy + 1.5), hx, hy, rad);
            if d_sh > 0.0 && d_sh < pad as f32 {
                let t = 1.0 - (d_sh / pad as f32);
                tip_blend_px(frame, x, y, [0, 0, 0, (t * t * shadow_a).round() as u8]);
            }
            let d = tip_rounded_sdf(fx - cx, fy - cy, hx, hy, rad);
            if d <= 0.5 {
                let cov = (0.5 - d).clamp(0.0, 1.0);
                let mut col = if d >= -1.0 {
                    palette.menu_border
                } else {
                    palette.menu_bg
                };
                col[3] = (f32::from(col[3]) * cov).round().clamp(0.0, 255.0) as u8;
                tip_blend_px(frame, x, y, col);
                if is_top && d < -1.0 {
                    tip_blend_px(frame, x, y, top_spec);
                }
            }
        }
    }
}

fn tip_rounded_sdf(px: f32, py: f32, hx: f32, hy: f32, r: f32) -> f32 {
    let qx = px.abs() - (hx - r);
    let qy = py.abs() - (hy - r);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

fn tip_blend_px(bgra: &mut BgraFrame, x: i32, y: i32, rgba: [u8; 4]) {
    if x < 0 || y < 0 || x >= bgra.width as i32 || y >= bgra.height as i32 || rgba[3] == 0 {
        return;
    }
    let idx = ((y as usize) * (bgra.width as usize) + (x as usize)) * 4;
    let sa = f32::from(rgba[3]) / 255.0;
    let inv = 1.0 - sa;
    bgra.pixels[idx] =
        (f32::from(rgba[2]) * sa + f32::from(bgra.pixels[idx]) * inv).clamp(0.0, 255.0) as u8;
    bgra.pixels[idx + 1] =
        (f32::from(rgba[1]) * sa + f32::from(bgra.pixels[idx + 1]) * inv).clamp(0.0, 255.0) as u8;
    bgra.pixels[idx + 2] =
        (f32::from(rgba[0]) * sa + f32::from(bgra.pixels[idx + 2]) * inv).clamp(0.0, 255.0) as u8;
    bgra.pixels[idx + 3] =
        (sa * 255.0 + f32::from(bgra.pixels[idx + 3]) * inv).clamp(0.0, 255.0) as u8;
}

pub(crate) fn compute_tooltip_rect(
    cursor_x: i32,
    cursor_y: i32,
    width: u32,
    height: u32,
    area: asrbox_platform_api::PixelRect,
) -> asrbox_platform_api::PixelRect {
    let min_x = area.x + 8;
    let max_x = (area.x + area.width as i32 - width as i32 - 8).max(min_x);
    let x = (cursor_x - (width as i32) / 2).clamp(min_x, max_x);
    let above = cursor_y - height as i32 - 12;
    let max_y = (area.y + area.height as i32 - height as i32 - 8).max(area.y + 8);
    let y = if above >= area.y + 8 {
        above
    } else {
        (cursor_y + 20).clamp(area.y + 8, max_y)
    };
    asrbox_platform_api::PixelRect {
        x,
        y,
        width,
        height,
    }
}
