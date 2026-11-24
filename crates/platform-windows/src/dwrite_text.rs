use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;
use std::mem;

use asrbox_platform_api::{BgraFrame, TextAlign, WidgetTextCmd};
use windows::core::{implement, PCWSTR, Ref};
use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteBitmapRenderTarget, IDWriteFactory, IDWriteGdiInterop,
    IDWritePixelSnapping_Impl, IDWriteRenderingParams, IDWriteTextFormat, IDWriteTextRenderer,
    IDWriteTextRenderer_Impl, DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL,
    DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD,
    DWRITE_GLYPH_RUN, DWRITE_GLYPH_RUN_DESCRIPTION, DWRITE_MATRIX, DWRITE_MEASURING_MODE,
    DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_PIXEL_GEOMETRY_RGB,
    DWRITE_RENDERING_MODE_CLEARTYPE_NATURAL_SYMMETRIC, DWRITE_STRIKETHROUGH,
    DWRITE_TEXT_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_LEADING, DWRITE_TEXT_ALIGNMENT_TRAILING,
    DWRITE_TRIMMING, DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_UNDERLINE,
    DWRITE_WORD_WRAPPING_NO_WRAP,
};
use windows::Win32::Graphics::Gdi::{GetCurrentObject, GetObjectW, DIBSECTION, OBJ_BITMAP};
use windows::Win32::UI::WindowsAndMessaging::FillRect;

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
        render_with_context(ctx, out, cmd, x0, y0, x1, y1).is_ok()
    })
}

fn init_dwrite_context() -> Result<DWriteContext, String> {
    // SAFETY: Creating a shared DirectWrite factory and GDI interop on the UI thread.
    unsafe {
        let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)
            .map_err(|e| format!("DWriteCreateFactory failed: {e}"))?;
        let gdi_interop = factory
            .GetGdiInterop()
            .map_err(|e| format!("GetGdiInterop failed: {e}"))?;
        let base_params = factory
            .CreateRenderingParams()
            .map_err(|e| format!("CreateRenderingParams failed: {e}"))?;
        let params = factory
            .CreateCustomRenderingParams(
                base_params.GetGamma(),
                base_params.GetEnhancedContrast(),
                base_params.GetClearTypeLevel(),
                DWRITE_PIXEL_GEOMETRY_RGB,
                DWRITE_RENDERING_MODE_CLEARTYPE_NATURAL_SYMMETRIC,
            )
            .unwrap_or(base_params);
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
            .map_err(|e| format!("CreateTextFormat failed: {e}"))?;
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
            .map_err(|e| format!("CreateBitmapRenderTarget failed: {e}"))?;
        let _ = rt.SetPixelsPerDip(1.0);
        ctx.target = Some((new_w, new_h, rt.clone()));
        Ok(rt)
    }
}

fn render_with_context(
    ctx: &mut DWriteContext,
    out: &mut BgraFrame,
    cmd: &WidgetTextCmd,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) -> Result<(), String> {
    let bw = (x1 - x0) as u32;
    let bh = (y1 - y0) as u32;
    let fmt = get_or_create_format(ctx, cmd.font_size_px, cmd.bold, cmd.align)?;
    let target = ensure_render_target(ctx, bw, bh)?;
    let wide: Vec<u16> = cmd.text.encode_utf16().collect();
    // SAFETY: Drawing white DirectWrite glyphs onto cleared black DIB target and blending into `out`.
    unsafe {
        let layout = ctx
            .factory
            .CreateTextLayout(&wide, &fmt, bw as f32, bh as f32)
            .map_err(|e| format!("CreateTextLayout failed: {e}"))?;
        if let Ok(sign) = ctx.factory.CreateEllipsisTrimmingSign(&fmt) {
            let trim = DWRITE_TRIMMING {
                granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                delimiter: 0,
                delimiterCount: 0,
            };
            let _ = layout.SetTrimming(&trim, Some(&sign));
        }
        let hdc = target.GetMemoryDC();
        let hbmp = GetCurrentObject(hdc, OBJ_BITMAP);
        let mut ds = DIBSECTION::default();
        let got = GetObjectW(
            hbmp,
            mem::size_of::<DIBSECTION>() as i32,
            Some((&mut ds as *mut DIBSECTION).cast::<c_void>()),
        );
        if got == 0 || ds.dsBm.bmBits.is_null() {
            return Err("DWrite target DIB unavailable".to_owned());
        }
        let clear_rc = RECT {
            left: 0,
            top: 0,
            right: bw as i32,
            bottom: bh as i32,
        };
        let _ = FillRect(
            hdc,
            &clear_rc,
            windows::Win32::Graphics::Gdi::HBRUSH(
                windows::Win32::Graphics::Gdi::GetStockObject(
                    windows::Win32::Graphics::Gdi::BLACK_BRUSH,
                )
                .0,
            ),
        );
        let renderer: IDWriteTextRenderer = GlyphBlitRenderer {
            target: target.clone(),
            params: ctx.params.clone(),
        }
        .into();
        layout
            .Draw(None, &renderer, 0.0, 0.0)
            .map_err(|e| format!("IDWriteTextLayout::Draw failed: {e}"))?;

        let src_h = ds.dsBm.bmHeight.unsigned_abs() as usize;
        let stride = ds.dsBm.bmWidthBytes.unsigned_abs() as usize;
        let top_down = ds.dsBmih.biHeight < 0;
        let total = stride.saturating_mul(src_h);
        let scratch = std::slice::from_raw_parts(ds.dsBm.bmBits.cast::<u8>(), total);
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

#[implement(IDWriteTextRenderer)]
struct GlyphBlitRenderer {
    target: IDWriteBitmapRenderTarget,
    params: IDWriteRenderingParams,
}

impl IDWritePixelSnapping_Impl for GlyphBlitRenderer_Impl {
    fn IsPixelSnappingDisabled(&self, _clientdrawingcontext: *const c_void) -> windows::core::Result<windows::core::BOOL> {
        Ok(windows::core::BOOL(0))
    }

    fn GetCurrentTransform(&self, _clientdrawingcontext: *const c_void) -> windows::core::Result<DWRITE_MATRIX> {
        Ok(DWRITE_MATRIX {
            m11: 1.0,
            m12: 0.0,
            m21: 0.0,
            m22: 1.0,
            dx: 0.0,
            dy: 0.0,
        })
    }

    fn GetPixelsPerDip(&self, _clientdrawingcontext: *const c_void) -> windows::core::Result<f32> {
        Ok(1.0)
    }
}

impl IDWriteTextRenderer_Impl for GlyphBlitRenderer_Impl {
    fn DrawGlyphRun(
        &self,
        _clientdrawingcontext: *const c_void,
        baselineoriginx: f32,
        baselineoriginy: f32,
        measuringmode: DWRITE_MEASURING_MODE,
        glyphrun: *const DWRITE_GLYPH_RUN,
        _glyphrundescription: *const DWRITE_GLYPH_RUN_DESCRIPTION,
        _clientdrawingeffect: Ref<'_, windows::core::IUnknown>,
    ) -> windows::core::Result<()> {
        // SAFETY: Forwarding valid DirectWrite glyph run onto our owned bitmap render target.
        unsafe {
            let _ = self.target.DrawGlyphRun(
                baselineoriginx,
                baselineoriginy,
                measuringmode,
                glyphrun,
                &self.params,
                COLORREF(0x00FF_FFFF),
                None,
            )?;
        }
        Ok(())
    }

    fn DrawUnderline(
        &self,
        _clientdrawingcontext: *const c_void,
        _baselineoriginx: f32,
        _baselineoriginy: f32,
        _underline: *const DWRITE_UNDERLINE,
        _clientdrawingeffect: Ref<'_, windows::core::IUnknown>,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn DrawStrikethrough(
        &self,
        _clientdrawingcontext: *const c_void,
        _baselineoriginx: f32,
        _baselineoriginy: f32,
        _strikethrough: *const DWRITE_STRIKETHROUGH,
        _clientdrawingeffect: Ref<'_, windows::core::IUnknown>,
    ) -> windows::core::Result<()> {
        Ok(())
    }

    fn DrawInlineObject(
        &self,
        _clientdrawingcontext: *const c_void,
        _originx: f32,
        _originy: f32,
        _inlineobject: Ref<'_, windows::Win32::Graphics::DirectWrite::IDWriteInlineObject>,
        _issideways: windows::core::BOOL,
        _isrighttoleft: windows::core::BOOL,
        _clientdrawingeffect: Ref<'_, windows::core::IUnknown>,
    ) -> windows::core::Result<()> {
        Ok(())
    }
}
