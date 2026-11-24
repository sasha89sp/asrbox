use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use asrbox_platform_api::{BgraFrame, FontRenderer};
use windows::Win32::Foundation::LPARAM;
use windows::Win32::Graphics::Gdi::{
    CreateFontIndirectW, DeleteObject, EnumFontFamiliesExW, GetDC, ReleaseDC,
    CLEARTYPE_NATURAL_QUALITY, DEFAULT_CHARSET, FW_NORMAL, FW_SEMIBOLD, HFONT, LOGFONTW,
    TEXTMETRICW,
};

const SEGOE_VAR_TEXT: &str = "Segoe UI Variable Text";
const SEGOE_VAR_DISPLAY: &str = "Segoe UI Variable Display";
const SEGOE_VAR_SMALL: &str = "Segoe UI Variable Small";
const SEGOE_FALLBACK: &str = "Segoe UI";

thread_local! {
    static ACTIVE_RENDERER: Cell<FontRenderer> = const { Cell::new(FontRenderer::GdiClearType) };
    static HAS_SEGOE_VAR: Cell<Option<bool>> = const { Cell::new(None) };
    static FONT_CACHE: RefCell<HashMap<(i32, bool), HFONT>> = RefCell::new(HashMap::new());
}

pub(crate) fn set_active_font_renderer(renderer: FontRenderer) {
    ACTIVE_RENDERER.with(|cell| cell.set(renderer));
}

pub(crate) fn active_font_renderer() -> FontRenderer {
    ACTIVE_RENDERER.with(Cell::get)
}

pub(crate) fn preferred_ui_font_face() -> &'static str {
    if has_segoe_ui_variable() {
        SEGOE_VAR_TEXT
    } else {
        SEGOE_FALLBACK
    }
}

pub(crate) fn font_face_for_size(size_px: i32) -> &'static str {
    if !has_segoe_ui_variable() {
        return SEGOE_FALLBACK;
    }
    if size_px >= 18 {
        SEGOE_VAR_DISPLAY
    } else if size_px <= 12 {
        SEGOE_VAR_SMALL
    } else {
        SEGOE_VAR_TEXT
    }
}

pub(crate) fn has_segoe_ui_variable() -> bool {
    HAS_SEGOE_VAR.with(|cell| {
        if let Some(cached) = cell.get() {
            return cached;
        }
        let detected = probe_font_family(SEGOE_VAR_TEXT);
        cell.set(Some(detected));
        detected
    })
}

fn probe_font_family(face: &str) -> bool {
    let mut lf = LOGFONTW {
        lfCharSet: DEFAULT_CHARSET,
        ..Default::default()
    };
    for (dst, src) in lf.lfFaceName.iter_mut().zip(face.encode_utf16()) {
        *dst = src;
    }
    let mut found = false;
    // SAFETY: EnumFontFamiliesExW synchronously invokes `font_probe_cb` with a valid stack pointer.
    unsafe {
        let hdc = GetDC(None);
        if !hdc.is_invalid() {
            let lparam = LPARAM((&mut found as *mut bool) as isize);
            let _ = EnumFontFamiliesExW(hdc, &lf, Some(font_probe_cb), lparam, 0);
            let _ = ReleaseDC(None, hdc);
        }
    }
    found
}

unsafe extern "system" fn font_probe_cb(
    _lpelfe: *const LOGFONTW,
    _lpntme: *const TEXTMETRICW,
    _font_type: u32,
    lparam: LPARAM,
) -> i32 {
    if lparam.0 != 0 {
        let found = &mut *(lparam.0 as *mut bool);
        *found = true;
    }
    0
}

pub(crate) fn get_or_create_ui_font(size_px: i32, bold: bool) -> HFONT {
    let clamped = size_px.clamp(8, 72);
    FONT_CACHE.with(|cache| {
        if let Some(&font) = cache.borrow().get(&(clamped, bold)) {
            return font;
        }
        let font = create_ui_font_handle(clamped, bold);
        if !font.is_invalid() {
            cache.borrow_mut().insert((clamped, bold), font);
        }
        font
    })
}

fn create_ui_font_handle(size_px: i32, bold: bool) -> HFONT {
    let face = font_face_for_size(size_px);
    let mut lf = LOGFONTW {
        lfHeight: -size_px,
        lfWeight: if bold {
            FW_SEMIBOLD.0 as i32
        } else {
            FW_NORMAL.0 as i32
        },
        lfCharSet: DEFAULT_CHARSET,
        lfQuality: CLEARTYPE_NATURAL_QUALITY,
        ..Default::default()
    };
    for (dst, src) in lf.lfFaceName.iter_mut().zip(face.encode_utf16()) {
        *dst = src;
    }
    // SAFETY: Creating a GDI logical font stored in FONT_CACHE and released in cleanup_font_cache.
    unsafe { CreateFontIndirectW(&lf) }
}

pub(crate) fn cleanup_font_cache() {
    FONT_CACHE.with(|cache| {
        let mut map = cache.borrow_mut();
        for (_, font) in map.drain() {
            if !font.is_invalid() {
                // SAFETY: Deleting owned cached GDI font handle on UI teardown.
                unsafe {
                    let _ = DeleteObject(font.into());
                }
            }
        }
    });
}

#[inline]
fn tune_coverage(raw: u8, is_light_text: bool) -> f32 {
    if raw == 0 {
        return 0.0;
    }
    if raw == 255 {
        return 1.0;
    }
    let t = f32::from(raw) / 255.0;
    if is_light_text {
        t * (0.94 + 0.06 * t)
    } else {
        t * (1.28 - 0.28 * t)
    }
}

/// Blends a ClearType white-on-black BGRA coverage buffer onto `out`.
/// `src_origin` maps `(dst_x, dst_y)` to `(dst_x - src_origin.0, dst_y - src_origin.1)` in `scratch`.
#[allow(clippy::too_many_arguments)]
pub(crate) fn blend_cleartype_mask(
    scratch: &[u8],
    src_stride_bytes: usize,
    src_height: usize,
    src_top_down: bool,
    src_offset_x: i32,
    src_offset_y: i32,
    out: &mut BgraFrame,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    rgba: [u8; 4],
) {
    let dst_stride = out.width as usize;
    let base_alpha = f32::from(rgba[3]) / 255.0;
    let luma = (0.299 * f32::from(rgba[0]) + 0.587 * f32::from(rgba[1]) + 0.114 * f32::from(rgba[2]))
        / 255.0;
    let is_light = luma >= 0.5;
    let (sr_c, sg_c, sb_c) = (f32::from(rgba[0]), f32::from(rgba[1]), f32::from(rgba[2]));

    for y in y0..y1 {
        let sy = y - src_offset_y;
        if sy < 0 || (sy as usize) >= src_height {
            continue;
        }
        let src_row = if src_top_down {
            sy as usize
        } else {
            src_height - 1 - (sy as usize)
        };
        let row_base = src_row * src_stride_bytes;
        for x in x0..x1 {
            let sx = x - src_offset_x;
            if sx < 0 {
                continue;
            }
            let s_idx = row_base + (sx as usize) * 4;
            let d_idx = ((y as usize) * dst_stride + (x as usize)) * 4;
            if s_idx + 2 >= scratch.len() || d_idx + 3 >= out.pixels.len() {
                continue;
            }
            let (mb, mg, mr) = (scratch[s_idx], scratch[s_idx + 1], scratch[s_idx + 2]);
            if (mb | mg | mr) == 0 {
                continue;
            }
            let cr = tune_coverage(mr, is_light) * base_alpha;
            let cg = tune_coverage(mg, is_light) * base_alpha;
            let cb = tune_coverage(mb, is_light) * base_alpha;
            let da = out.pixels[d_idx + 3];
            if da == 255 {
                let db = f32::from(out.pixels[d_idx]);
                let dg = f32::from(out.pixels[d_idx + 1]);
                let dr = f32::from(out.pixels[d_idx + 2]);
                out.pixels[d_idx] = (sb_c * cb + db * (1.0 - cb)).round().clamp(0.0, 255.0) as u8;
                out.pixels[d_idx + 1] =
                    (sg_c * cg + dg * (1.0 - cg)).round().clamp(0.0, 255.0) as u8;
                out.pixels[d_idx + 2] =
                    (sr_c * cr + dr * (1.0 - cr)).round().clamp(0.0, 255.0) as u8;
            } else {
                let sa = (0.299 * cr + 0.587 * cg + 0.114 * cb).clamp(0.0, 1.0);
                let inv = 1.0 - sa;
                let db = f32::from(out.pixels[d_idx]);
                let dg = f32::from(out.pixels[d_idx + 1]);
                let dr = f32::from(out.pixels[d_idx + 2]);
                out.pixels[d_idx] = (sb_c * sa + db * inv).round().clamp(0.0, 255.0) as u8;
                out.pixels[d_idx + 1] = (sg_c * sa + dg * inv).round().clamp(0.0, 255.0) as u8;
                out.pixels[d_idx + 2] = (sr_c * sa + dr * inv).round().clamp(0.0, 255.0) as u8;
                out.pixels[d_idx + 3] = (sa * 255.0 + f32::from(da) * inv)
                    .round()
                    .clamp(0.0, 255.0) as u8;
            }
        }
    }
}
