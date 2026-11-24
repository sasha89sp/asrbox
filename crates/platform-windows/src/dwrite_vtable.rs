use std::ffi::c_void;

use windows::core::{IUnknown, IUnknown_Vtbl, Interface, BOOL, GUID, HRESULT};
use windows::Win32::Foundation::{COLORREF, E_NOINTERFACE, S_OK};
use windows::Win32::Graphics::DirectWrite::{
    IDWriteBitmapRenderTarget, IDWritePixelSnapping, IDWritePixelSnapping_Vtbl,
    IDWriteRenderingParams, IDWriteTextRenderer, IDWriteTextRenderer_Vtbl, DWRITE_GLYPH_RUN,
    DWRITE_GLYPH_RUN_DESCRIPTION, DWRITE_MATRIX, DWRITE_MEASURING_MODE, DWRITE_STRIKETHROUGH,
    DWRITE_UNDERLINE,
};

pub(crate) struct DrawCtx<'a> {
    pub(crate) target: &'a IDWriteBitmapRenderTarget,
    pub(crate) params: &'a IDWriteRenderingParams,
}

#[repr(C)]
struct StaticRendererObj {
    vtable: *const IDWriteTextRenderer_Vtbl,
}

// SAFETY: Static COM singleton holds only an immutable vtable pointer; per-call state travels in clientdrawingcontext.
unsafe impl Sync for StaticRendererObj {}

static STATIC_RENDERER_VTBL: IDWriteTextRenderer_Vtbl = IDWriteTextRenderer_Vtbl {
    base__: IDWritePixelSnapping_Vtbl {
        base__: IUnknown_Vtbl {
            QueryInterface: renderer_qi,
            AddRef: renderer_refcount,
            Release: renderer_refcount,
        },
        IsPixelSnappingDisabled: renderer_is_pixel_snapping_disabled,
        GetCurrentTransform: renderer_get_current_transform,
        GetPixelsPerDip: renderer_get_pixels_per_dip,
    },
    DrawGlyphRun: renderer_draw_glyph_run,
    DrawUnderline: renderer_draw_underline,
    DrawStrikethrough: renderer_draw_strikethrough,
    DrawInlineObject: renderer_draw_inline_object,
};

static STATIC_RENDERER_OBJ: StaticRendererObj = StaticRendererObj {
    vtable: &STATIC_RENDERER_VTBL,
};

pub(crate) fn static_renderer_raw() -> *mut c_void {
    (&STATIC_RENDERER_OBJ as *const StaticRendererObj)
        .cast_mut()
        .cast::<c_void>()
}

unsafe extern "system" fn renderer_qi(
    this: *mut c_void,
    iid: *const GUID,
    out: *mut *mut c_void,
) -> HRESULT {
    if out.is_null() || iid.is_null() {
        return E_NOINTERFACE;
    }
    // SAFETY: Caller passed valid pointers per COM QueryInterface contract.
    unsafe {
        let g = *iid;
        if g == IUnknown::IID || g == IDWritePixelSnapping::IID || g == IDWriteTextRenderer::IID {
            *out = this;
            S_OK
        } else {
            *out = std::ptr::null_mut();
            E_NOINTERFACE
        }
    }
}

unsafe extern "system" fn renderer_refcount(_this: *mut c_void) -> u32 {
    1
}

unsafe extern "system" fn renderer_is_pixel_snapping_disabled(
    _this: *mut c_void,
    _ctx: *const c_void,
    is_disabled: *mut BOOL,
) -> HRESULT {
    if !is_disabled.is_null() {
        // SAFETY: DirectWrite supplies a valid writable BOOL out-pointer.
        unsafe { *is_disabled = BOOL(0) };
    }
    S_OK
}

unsafe extern "system" fn renderer_get_current_transform(
    _this: *mut c_void,
    _ctx: *const c_void,
    transform: *mut DWRITE_MATRIX,
) -> HRESULT {
    if !transform.is_null() {
        // SAFETY: DirectWrite supplies a valid writable DWRITE_MATRIX out-pointer.
        unsafe {
            *transform = DWRITE_MATRIX {
                m11: 1.0,
                m12: 0.0,
                m21: 0.0,
                m22: 1.0,
                dx: 0.0,
                dy: 0.0,
            };
        }
    }
    S_OK
}

unsafe extern "system" fn renderer_get_pixels_per_dip(
    _this: *mut c_void,
    _ctx: *const c_void,
    ppd: *mut f32,
) -> HRESULT {
    if !ppd.is_null() {
        // SAFETY: DirectWrite supplies a valid writable f32 out-pointer.
        unsafe { *ppd = 1.0 };
    }
    S_OK
}

unsafe extern "system" fn renderer_draw_glyph_run(
    _this: *mut c_void,
    ctx_ptr: *const c_void,
    bx: f32,
    by: f32,
    mode: DWRITE_MEASURING_MODE,
    run: *const DWRITE_GLYPH_RUN,
    _desc: *const DWRITE_GLYPH_RUN_DESCRIPTION,
    _effect: *mut c_void,
) -> HRESULT {
    if ctx_ptr.is_null() || run.is_null() {
        return S_OK;
    }
    // SAFETY: `ctx_ptr` points to our stack-pinned `DrawCtx` for the synchronous `Draw` call.
    unsafe {
        let c = &*ctx_ptr.cast::<DrawCtx<'_>>();
        match c
            .target
            .DrawGlyphRun(bx, by, mode, run, c.params, COLORREF(0x00FF_FFFF), None)
        {
            Ok(_) => S_OK,
            Err(e) => e.code(),
        }
    }
}

unsafe extern "system" fn renderer_draw_underline(
    _: *mut c_void,
    _: *const c_void,
    _: f32,
    _: f32,
    _: *const DWRITE_UNDERLINE,
    _: *mut c_void,
) -> HRESULT {
    S_OK
}

unsafe extern "system" fn renderer_draw_strikethrough(
    _: *mut c_void,
    _: *const c_void,
    _: f32,
    _: f32,
    _: *const DWRITE_STRIKETHROUGH,
    _: *mut c_void,
) -> HRESULT {
    S_OK
}

unsafe extern "system" fn renderer_draw_inline_object(
    _: *mut c_void,
    _: *const c_void,
    _: f32,
    _: f32,
    _: *mut c_void,
    _: BOOL,
    _: BOOL,
    _: *mut c_void,
) -> HRESULT {
    S_OK
}
