use std::ffi::c_void;

use asrbox_platform_api::{BgraFrame, LocalDateTime, TextAlign, WidgetFrame, WidgetTextCmd};
use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, CreateFontIndirectW, DeleteDC, DeleteObject, DrawTextW,
    GetDC, ReleaseDC, SelectObject, SetBkMode, SetTextColor, ANTIALIASED_QUALITY, DEFAULT_CHARSET,
    DIB_RGB_COLORS, DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE,
    DT_VCENTER, FW_NORMAL, FW_SEMIBOLD, LOGFONTW, TRANSPARENT,
};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Time::{GetTimeZoneInformation, TIME_ZONE_INFORMATION};

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
            for cmd in &frame.texts {
                draw_and_blend_cmd(hdc_mem, scratch, &mut out, cmd);
            }
        }

        let _ = SelectObject(hdc_mem, prev_bmp);
        let _ = DeleteObject(hbmp.into());
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);
    }
    Ok(out)
}

unsafe fn draw_and_blend_cmd(
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
    let font = create_segoe_font(cmd.font_size_px, cmd.bold);
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
    let _ = DeleteObject(font.into());

    blend_text_mask(scratch, out, x0, y0, x1, y1, cmd.color_rgba);
}

unsafe fn create_segoe_font(size_px: i32, bold: bool) -> windows::Win32::Graphics::Gdi::HFONT {
    let mut lf = LOGFONTW {
        lfHeight: -size_px.max(8),
        lfWeight: if bold {
            FW_SEMIBOLD.0 as i32
        } else {
            FW_NORMAL.0 as i32
        },
        lfCharSet: DEFAULT_CHARSET,
        lfQuality: ANTIALIASED_QUALITY,
        ..Default::default()
    };
    for (dst, src) in lf.lfFaceName.iter_mut().zip("Segoe UI".encode_utf16()) {
        *dst = src;
    }
    CreateFontIndirectW(&lf)
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

fn blend_text_mask(
    scratch: &[u8],
    out: &mut BgraFrame,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    rgba: [u8; 4],
) {
    let stride = out.width as usize;
    let base_alpha = f32::from(rgba[3]) / 255.0;
    for y in y0..y1 {
        for x in x0..x1 {
            let idx = ((y as usize) * stride + (x as usize)) * 4;
            if idx + 3 >= scratch.len() || idx + 3 >= out.pixels.len() {
                continue;
            }
            let lum = scratch[idx].max(scratch[idx + 1]).max(scratch[idx + 2]);
            if lum == 0 {
                continue;
            }
            let sa = (f32::from(lum) / 255.0) * base_alpha;
            let inv = 1.0 - sa;
            let sr = f32::from(rgba[0]) * sa;
            let sg = f32::from(rgba[1]) * sa;
            let sb = f32::from(rgba[2]) * sa;

            let db = f32::from(out.pixels[idx]);
            let dg = f32::from(out.pixels[idx + 1]);
            let dr = f32::from(out.pixels[idx + 2]);
            let da = f32::from(out.pixels[idx + 3]);

            out.pixels[idx] = (sb + db * inv).round().clamp(0.0, 255.0) as u8;
            out.pixels[idx + 1] = (sg + dg * inv).round().clamp(0.0, 255.0) as u8;
            out.pixels[idx + 2] = (sr + dr * inv).round().clamp(0.0, 255.0) as u8;
            out.pixels[idx + 3] = (sa * 255.0 + da * inv).round().clamp(0.0, 255.0) as u8;
        }
    }
}

pub(crate) fn render_tooltip_frame(text: &str) -> Result<BgraFrame, String> {
    let lines: Vec<&str> = text.lines().take(5).collect();
    let max_chars = lines.iter().map(|l| l.chars().count()).max().unwrap_or(8);
    let width = ((max_chars as u32) * 7 + 24).clamp(140, 320);
    let height = (lines.len().max(1) as u32) * 18 + 14;
    let mut bgra = BgraFrame::transparent(width, height);
    fill_tooltip_bg(&mut bgra);
    let mut texts = Vec::with_capacity(lines.len());
    for (idx, line) in lines.iter().enumerate() {
        texts.push(WidgetTextCmd {
            x: 10,
            y: 7 + (idx as i32) * 18,
            width: (width as i32) - 20,
            height: 18,
            text: (*line).to_owned(),
            font_size_px: 12,
            bold: idx == 0,
            color_rgba: if idx == 0 {
                [243, 244, 246, 255]
            } else {
                [196, 202, 214, 255]
            },
            align: TextAlign::Left,
        });
    }
    composite_widget_frame(&WidgetFrame { bgra, texts })
}

fn fill_tooltip_bg(frame: &mut BgraFrame) {
    let w = frame.width as usize;
    let h = frame.height as usize;
    for y in 0..h {
        for x in 0..w {
            let border = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
            let idx = (y * w + x) * 4;
            if border {
                frame.pixels[idx] = 74;
                frame.pixels[idx + 1] = 68;
                frame.pixels[idx + 2] = 64;
                frame.pixels[idx + 3] = 245;
            } else {
                frame.pixels[idx] = 34;
                frame.pixels[idx + 1] = 28;
                frame.pixels[idx + 2] = 26;
                frame.pixels[idx + 3] = 242;
            }
        }
    }
}
