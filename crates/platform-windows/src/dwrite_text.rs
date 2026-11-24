use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;

use asrbox_platform_api::{BgraFrame, TextAlign, WidgetTextCmd};
use windows::core::{Interface, PCWSTR};
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteBitmapRenderTarget, IDWriteFactory, IDWriteGdiInterop,
    IDWriteRenderingParams, IDWriteTextFormat, IDWriteTextRenderer, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL,
    DWRITE_FONT_WEIGHT_SEMI_BOLD, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_PIXEL_GEOMETRY_RGB,
    DWRITE_RENDERING_MODE_CLEARTYPE_NATURAL_SYMMETRIC, DWRITE_TEXT_ALIGNMENT_CENTER,
    DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING, DWRITE_TRIMMING,
    DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP,
};
use windows::Win32::Graphics::Gdi::{
    FillRect, GetCurrentObject, GetObjectW, GetStockObject, BLACK_BRUSH, DIBSECTION, HBRUSH,
    OBJ_BITMAP,
};

use crate::dwrite_vtable::{static_renderer_raw, DrawCtx};
use crate::font_engine::{blend_cleartype_mask, font_face_for_size};
use crate::shell::encode_wide_str;

struct DWriteContext {
    factory: IDWriteFactory,
    gdi_interop: IDWriteGdiInterop,
    params: IDWriteRenderingParams,
    formats: HashMap<(i32, bool, u8), IDWriteTextFormat>,
    target: Option<(u32, u32, IDWriteBitmapRenderTarget)>,
}

thread_local! {
    static DWRITE_CTX: RefCell<Option<DWriteContext>> = const { RefCell::new(None) };
    static DWRITE_FAILED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(crate) fn cleanup_dwrite_cache() {
    DWRITE_CTX.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

pub(crate) fn try_render_dwrite_cmd(out: &mut BgraFrame, cmd: &WidgetTextCmd) -> bool {
    if DWRITE_FAILED.with(std::cell::Cell::get) {
        return false;
    }
    let x0 = cmd.x.clamp(0, out.width as i32);
    let y0 = cmd.y.clamp(0, out.height as i32);
    let x1 = (cmd.x + cmd.width).clamp(x0, out.width as i32);
    let y1 = (cmd.y + cmd.height).clamp(y0, out.height as i32);
    if x1 <= x0 || y1 <= y0 || cmd.text.is_empty() {
        return true;
    }
    DWRITE_CTX.with(|cell| {
        let mut guard = cell.borrow_mut();
        if guard.is_none() {
            match init_dwrite_context() {
                Ok(ctx) => *guard = Some(ctx),
                Err(_) => {
                    DWRITE_FAILED.with(|f| f.set(true));
                    return false;
                }
            }
        }
        let Some(ctx) = guard.as_mut() else {
            return false;
        };
        render_with_context(ctx, out, cmd, (x0, y0, x1, y1)).is_ok()
    })
}

fn init_dwrite_context() -> Result<DWriteContext, String> {
    // SAFETY: Creating a shared DirectWrite factory and GDI interop on the UI thread.
    unsafe {
        let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)
            .map_err(|e| format!("DWriteCreateFactory: {e}"))?;
        let gdi_interop = factory
            .GetGdiInterop()
            .map_err(|e| format!("GetGdiInterop: {e}"))?;
        let base = factory
            .CreateRenderingParams()
            .map_err(|e| format!("RenderingParams: {e}"))?;
        let params = factory
            .CreateCustomRenderingParams(
                base.GetGamma(),
                base.GetEnhancedContrast(),
                base.GetClearTypeLevel(),
                DWRITE_PIXEL_GEOMETRY_RGB,
                DWRITE_RENDERING_MODE_CLEARTYPE_NATURAL_SYMMETRIC,
            )
            .unwrap_or(base);
        Ok(DWriteContext {
            factory,
            gdi_interop,
            params,
            formats: HashMap::new(),
            target: None,
        })
    }
}

fn get_or_create_format(
    ctx: &mut DWriteContext,
    size_px: i32,
    bold: bool,
    align: TextAlign,
) -> Result<IDWriteTextFormat, String> {
    let clamped = size_px.clamp(8, 72);
    let align_key = match align {
        TextAlign::Left => 0u8,
        TextAlign::Center => 1u8,
        TextAlign::Right => 2u8,
    };
    if let Some(fmt) = ctx.formats.get(&(clamped, bold, align_key)) {
        return Ok(fmt.clone());
    }
    let face_w = encode_wide_str(font_face_for_size(clamped));
    let locale_w = encode_wide_str("");
    let weight = if bold {
        DWRITE_FONT_WEIGHT_SEMI_BOLD
    } else {
        DWRITE_FONT_WEIGHT_NORMAL
    };
    // SAFETY: Creating a DirectWrite text format with valid NUL-terminated UTF-16 strings.
    unsafe {
        let fmt = ctx
            .factory
            .CreateTextFormat(
                PCWSTR(face_w.as_ptr()),
                None,
                weight,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                clamped as f32,
                PCWSTR(locale_w.as_ptr()),
            )
            .map_err(|e| format!("CreateTextFormat: {e}"))?;
        let dw_align = match align {
            TextAlign::Left => DWRITE_TEXT_ALIGNMENT_LEADING,
            TextAlign::Center => DWRITE_TEXT_ALIGNMENT_CENTER,
            TextAlign::Right => DWRITE_TEXT_ALIGNMENT_TRAILING,
        };
        let _ = fmt.SetTextAlignment(dw_align);
        let _ = fmt.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER);
        let _ = fmt.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP);
        ctx.formats.insert((clamped, bold, align_key), fmt.clone());
        Ok(fmt)
    }
}

fn ensure_render_target(
    ctx: &mut DWriteContext,
    req_w: u32,
    req_h: u32,
) -> Result<IDWriteBitmapRenderTarget, String> {
    if let Some((tw, th, ref rt)) = ctx.target {
        if tw >= req_w && th >= req_h {
            return Ok(rt.clone());
        }
    }
    let new_w = req_w.max(256).next_power_of_two().min(2048).max(req_w);
    let new_h = req_h.max(48).next_power_of_two().min(512).max(req_h);
    // SAFETY: Creating a DirectWrite GDI-backed bitmap render target on UI thread.
    unsafe {
        let rt = ctx
            .gdi_interop
            .CreateBitmapRenderTarget(None, new_w, new_h)
            .map_err(|e| format!("CreateBitmapRenderTarget: {e}"))?;
        let _ = rt.SetPixelsPerDip(1.0);
        ctx.target = Some((new_w, new_h, rt.clone()));
        Ok(rt)
    }
}

fn render_with_context(
    ctx: &mut DWriteContext,
    out: &mut BgraFrame,
    cmd: &WidgetTextCmd,
    (x0, y0, x1, y1): (i32, i32, i32, i32),
) -> Result<(), String> {
    let (bw, bh) = ((x1 - x0) as u32, (y1 - y0) as u32);
    let fmt = get_or_create_format(ctx, cmd.font_size_px, cmd.bold, cmd.align)?;
    let target = ensure_render_target(ctx, bw, bh)?;
    let wide: Vec<u16> = cmd.text.encode_utf16().collect();
    // SAFETY: Drawing white DirectWrite glyphs onto cleared black DIB target and blending into `out`.
    unsafe {
        let layout = ctx
            .factory
            .CreateTextLayout(&wide, &fmt, bw as f32, bh as f32)
            .map_err(|e| format!("CreateTextLayout: {e}"))?;
        if let Ok(sign) = ctx.factory.CreateEllipsisTrimmingSign(&fmt) {
            let trim = DWRITE_TRIMMING {
                granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                delimiter: 0,
                delimiterCount: 0,
            };
            let _ = layout.SetTrimming(&trim, Some(&sign));
        }
        let ds = clear_target_dib(&target, bw, bh)?;
        let draw_ctx = DrawCtx {
            target: &target,
            params: &ctx.params,
        };
        let raw_ptr = static_renderer_raw();
        let renderer = IDWriteTextRenderer::from_raw_borrowed(&raw_ptr)
            .ok_or_else(|| "static DWrite renderer unavailable".to_owned())?;
        layout
            .Draw(
                Some((&draw_ctx as *const DrawCtx).cast()),
                renderer,
                0.0,
                0.0,
            )
            .map_err(|e| format!("IDWriteTextLayout::Draw: {e}"))?;
        let src_h = ds.dsBm.bmHeight.unsigned_abs() as usize;
        let stride = ds.dsBm.bmWidthBytes.unsigned_abs() as usize;
        let scratch = std::slice::from_raw_parts(ds.dsBm.bmBits.cast::<u8>(), stride * src_h);
        let top_down = ds.dsBmih.biHeight < 0;
        blend_cleartype_mask(
            scratch,
            stride,
            src_h,
            top_down,
            x0,
            y0,
            out,
            x0,
            y0,
            x1,
            y1,
            cmd.color_rgba,
        );
    }
    Ok(())
}

unsafe fn clear_target_dib(
    target: &IDWriteBitmapRenderTarget,
    bw: u32,
    bh: u32,
) -> Result<DIBSECTION, String> {
    let hdc = target.GetMemoryDC();
    let hbmp = GetCurrentObject(hdc, OBJ_BITMAP);
    let mut ds = DIBSECTION::default();
    let sz = mem::size_of::<DIBSECTION>() as i32;
    let got = GetObjectW(hbmp, sz, Some((&mut ds as *mut DIBSECTION).cast()));
    if got == 0 || ds.dsBm.bmBits.is_null() {
        return Err("DWrite target DIB unavailable".to_owned());
    }
    let rc = RECT {
        left: 0,
        top: 0,
        right: bw as i32,
        bottom: bh as i32,
    };
    let _ = FillRect(hdc, &rc, HBRUSH(GetStockObject(BLACK_BRUSH).0));
    Ok(ds)
}
