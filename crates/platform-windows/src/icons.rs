use std::ffi::c_void;
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use asrbox_platform_api::{IconProvider, IconRequest, RgbaImage};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{GENERIC_READ, SIZE};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetDIBits, GetObjectW,
    ReleaseDC, SelectObject, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, HBITMAP,
};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppRGBA, IWICImagingFactory,
    WICBitmapDitherTypeNone, WICBitmapPaletteTypeMedianCut, WICDecodeMetadataCacheOnLoad,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{
    ExtractIconExW, IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK,
    SIIGBF_ICONONLY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, DrawIconEx, PrivateExtractIconsW, DI_NORMAL, HICON,
};

use crate::icons_lnk::{
    bgra_to_rgba, has_explorer_frame_box, has_ext, is_executable_ext, is_image_ext,
    load_shgetfileinfo_icon, recover_alpha_from_black_white, resolve_lnk_icon_info,
};

pub struct WindowsIconProvider;

impl WindowsIconProvider {
    pub fn new() -> Self {
        // SAFETY: COM apartment init on the UI thread is idempotent.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        Self
    }
}

impl Default for WindowsIconProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl IconProvider for WindowsIconProvider {
    fn load_icon(&self, request: &IconRequest, target_px: u32) -> Result<RgbaImage, String> {
        let size = target_px.clamp(32, 256);
        match request {
            IconRequest::AutoShell(path) => load_auto_shell_icon(path, size),
            IconRequest::CustomFile(path) => {
                if is_executable_ext(path) {
                    load_exe_resource_icon(path, 0, size)
                        .or_else(|_| load_shell_icon_unboxed(path, size))
                        .or_else(|_| load_shgetfileinfo_icon(path, size))
                } else {
                    load_wic_image(path, size)
                        .or_else(|_| load_shell_icon_unboxed(path, size))
                        .or_else(|_| load_shgetfileinfo_icon(path, size))
                }
            }
            IconRequest::ExeResource { path, index } => {
                let idx = i32::try_from(*index).unwrap_or(0);
                load_exe_resource_icon(path, idx, size)
                    .or_else(|_| load_shell_icon_unboxed(path, size))
                    .or_else(|_| load_shgetfileinfo_icon(path, size))
            }
        }
    }
}

fn load_auto_shell_icon(path: &Path, size: u32) -> Result<RgbaImage, String> {
    if has_ext(path, "lnk") {
        if let Some(res) = load_lnk_icon(path, size) {
            return Ok(res);
        }
    }
    if has_ext(path, "ico") {
        if let Ok(img) = load_wic_image(path, size) {
            return Ok(img);
        }
    }
    if is_executable_ext(path) {
        if let Ok(img) = load_shell_icon_unboxed(path, size) {
            return Ok(img);
        }
        if let Ok(img) = load_exe_resource_icon(path, 0, size) {
            return Ok(img);
        }
        return load_shgetfileinfo_icon(path, size);
    }
    load_shell_icon_unboxed(path, size)
        .or_else(|_| load_shgetfileinfo_icon(path, size))
        .or_else(|_| load_exe_resource_icon(path, 0, size))
}

fn load_lnk_icon(lnk_path: &Path, size: u32) -> Option<RgbaImage> {
    let info = resolve_lnk_icon_info(lnk_path)?;
    if let Some(icon_path) = info.icon_path.as_deref() {
        if has_ext(icon_path, "ico") || is_image_ext(icon_path) {
            if let Ok(img) = load_wic_image(icon_path, size) {
                return Some(img);
            }
        }
        if let Ok(img) = load_exe_resource_icon(icon_path, info.icon_index, size) {
            return Some(img);
        }
    }
    if let Some(target) = info.target_path.as_deref() {
        if is_executable_ext(target) {
            if let Ok(img) = load_shell_icon_unboxed(target, size) {
                return Some(img);
            }
            if let Ok(img) = load_exe_resource_icon(target, 0, size) {
                return Some(img);
            }
        }
    }
    None
}

pub(crate) fn make_top_down_bmi(width: i32, height: i32) -> BITMAPINFO {
    BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: u32::try_from(mem::size_of::<BITMAPINFOHEADER>()).unwrap_or(40),
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub(crate) fn to_wide(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn load_shell_icon_unboxed(path: &Path, size_px: u32) -> Result<RgbaImage, String> {
    let img = load_shell_icon(path, size_px)?;
    if has_explorer_frame_box(&img) {
        return Err("shell icon is wrapped in explorer frame box".to_owned());
    }
    Ok(img)
}

fn load_shell_icon(path: &Path, size_px: u32) -> Result<RgbaImage, String> {
    let wide = to_wide(path);
    // SAFETY: `wide` is NUL-terminated UTF-16; returned COM object and HBITMAP are freed.
    unsafe {
        let factory: IShellItemImageFactory =
            SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None)
                .map_err(|err| format!("SHCreateItemFromParsingName failed: {err}"))?;
        let side = i32::try_from(size_px).unwrap_or(256);
        let size = SIZE { cx: side, cy: side };
        let flags = SIIGBF_BIGGERSIZEOK | SIIGBF_ICONONLY;
        let hbitmap = factory
            .GetImage(size, flags)
            .map_err(|err| format!("IShellItemImageFactory::GetImage failed: {err}"))?;
        let image = hbitmap_to_rgba(hbitmap);
        let _ = DeleteObject(hbitmap.into());
        image
    }
}

fn load_wic_image(path: &Path, target_px: u32) -> Result<RgbaImage, String> {
    let wide = to_wide(path);
    // SAFETY: COM is initialized; `wide` is NUL-terminated; WIC interfaces are RAII-freed.
    unsafe {
        let factory: IWICImagingFactory =
            CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER)
                .map_err(|err| format!("WIC factory failed: {err}"))?;
        let decoder = factory
            .CreateDecoderFromFilename(
                PCWSTR(wide.as_ptr()),
                None,
                GENERIC_READ,
                WICDecodeMetadataCacheOnLoad,
            )
            .map_err(|err| format!("WIC decode failed: {err}"))?;
        let frame_idx = pick_best_wic_frame(&decoder, target_px);
        let frame = decoder
            .GetFrame(frame_idx)
            .map_err(|err| format!("WIC GetFrame failed: {err}"))?;
        let converter = factory
            .CreateFormatConverter()
            .map_err(|err| format!("WIC converter failed: {err}"))?;
        converter
            .Initialize(
                &frame,
                &GUID_WICPixelFormat32bppRGBA,
                WICBitmapDitherTypeNone,
                None,
                0.0,
                WICBitmapPaletteTypeMedianCut,
            )
            .map_err(|err| format!("WIC convert failed: {err}"))?;

        let (mut w, mut h) = (0u32, 0u32);
        converter
            .GetSize(&mut w, &mut h)
            .map_err(|err| format!("WIC GetSize failed: {err}"))?;
        let stride = w
            .checked_mul(4)
            .ok_or_else(|| "WIC stride overflow".to_owned())?;
        let len = (stride as usize)
            .checked_mul(h as usize)
            .ok_or_else(|| "WIC buffer overflow".to_owned())?;
        let mut pixels = vec![0u8; len];
        converter
            .CopyPixels(std::ptr::null(), stride, &mut pixels)
            .map_err(|err| format!("WIC CopyPixels failed: {err}"))?;
        RgbaImage::new(w, h, pixels)
    }
}

unsafe fn pick_best_wic_frame(
    decoder: &windows::Win32::Graphics::Imaging::IWICBitmapDecoder,
    target_px: u32,
) -> u32 {
    let count = decoder.GetFrameCount().unwrap_or(1);
    if count <= 1 {
        return 0;
    }
    let mut best_idx = 0u32;
    let mut best_score = 0u32;
    for i in 0..count {
        if let Ok(frame) = decoder.GetFrame(i) {
            let (mut w, mut h) = (0u32, 0u32);
            if frame.GetSize(&mut w, &mut h).is_ok() {
                let side = w.min(h);
                let score = if side >= target_px {
                    10_000u32.saturating_sub(side.saturating_sub(target_px))
                } else {
                    side
                };
                if score > best_score {
                    best_score = score;
                    best_idx = i;
                }
            }
        }
    }
    best_idx
}

fn load_exe_resource_icon(path: &Path, index: i32, size_px: u32) -> Result<RgbaImage, String> {
    let wide = to_wide(path);
    let side = i32::try_from(size_px).unwrap_or(128);
    let mut icons = [HICON::default()];
    let mut icon_id = 0u32;
    let count = if wide.len() <= 260 {
        let mut name_buf = [0u16; 260];
        name_buf[..wide.len()].copy_from_slice(&wide);
        // SAFETY: `name_buf` is NUL-terminated fixed [u16; 260]; HICON is destroyed below.
        unsafe {
            PrivateExtractIconsW(
                &name_buf,
                index,
                side,
                side,
                Some(&mut icons),
                Some(&mut icon_id as *mut u32),
                0,
            )
        }
    } else {
        0
    };
    let hicon = if count > 0 && count != u32::MAX && !icons[0].is_invalid() {
        icons[0]
    } else {
        let mut large = HICON::default();
        // SAFETY: `wide` is NUL-terminated; `large` is valid storage and destroyed below.
        let extracted =
            unsafe { ExtractIconExW(PCWSTR(wide.as_ptr()), index, Some(&mut large), None, 1) };
        if extracted == 0 || large.is_invalid() {
            return Err("No icon resource found".to_owned());
        }
        large
    };
    let result = hicon_to_rgba(hicon, size_px);
    // SAFETY: `hicon` was created above and has not yet been destroyed.
    unsafe {
        let _ = DestroyIcon(hicon);
    }
    result
}

pub(crate) fn hicon_to_rgba(hicon: HICON, size_px: u32) -> Result<RgbaImage, String> {
    let mut raw_black = rasterize_hicon_on_bg(hicon, size_px, 0x00)?;
    let has_alpha = raw_black.as_chunks::<4>().0.iter().any(|px| px[3] != 0);
    if !has_alpha {
        let raw_white = rasterize_hicon_on_bg(hicon, size_px, 0xFF)?;
        recover_alpha_from_black_white(&mut raw_black, &raw_white);
    }
    for px in raw_black.as_chunks_mut::<4>().0 {
        px.swap(0, 2);
    }
    RgbaImage::new(size_px, size_px, raw_black)
}

fn rasterize_hicon_on_bg(hicon: HICON, size_px: u32, fill_byte: u8) -> Result<Vec<u8>, String> {
    let side = i32::try_from(size_px).unwrap_or(64);
    let bmi = make_top_down_bmi(side, side);
    let mut bits: *mut c_void = std::ptr::null_mut();

    // SAFETY: All GDI handles (DCs and DIBSection) are released before returning.
    unsafe {
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let dib = CreateDIBSection(Some(hdc_mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0);
        let hbitmap = match dib {
            Ok(bmp) => bmp,
            Err(err) => {
                let _ = DeleteDC(hdc_mem);
                let _ = ReleaseDC(None, hdc_screen);
                return Err(format!("CreateDIBSection failed: {err}"));
            }
        };
        let prev = SelectObject(hdc_mem, hbitmap.into());
        let byte_len = (size_px as usize) * (size_px as usize) * 4;
        if !bits.is_null() {
            std::ptr::write_bytes(bits as *mut u8, fill_byte, byte_len);
        }
        let _ = DrawIconEx(hdc_mem, 0, 0, hicon, side, side, 0, None, DI_NORMAL);
        let mut raw = vec![0u8; byte_len];
        if !bits.is_null() {
            std::ptr::copy_nonoverlapping(bits as *const u8, raw.as_mut_ptr(), byte_len);
        }
        let _ = SelectObject(hdc_mem, prev);
        let _ = DeleteObject(hbitmap.into());
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);
        Ok(raw)
    }
}

fn hbitmap_to_rgba(hbitmap: HBITMAP) -> Result<RgbaImage, String> {
    let mut bmp = BITMAP::default();
    let bmp_size = i32::try_from(mem::size_of::<BITMAP>()).unwrap_or(0);
    // SAFETY: `bmp` matches `bmp_size`; the screen DC is released before returning.
    unsafe {
        let written = GetObjectW(
            hbitmap.into(),
            bmp_size,
            Some(&mut bmp as *mut BITMAP as *mut c_void),
        );
        if written == 0 || bmp.bmWidth <= 0 || bmp.bmHeight <= 0 {
            return Err("GetObjectW failed for icon bitmap".to_owned());
        }
        let w = u32::try_from(bmp.bmWidth).map_err(|_| "negative width".to_owned())?;
        let h = u32::try_from(bmp.bmHeight).map_err(|_| "negative height".to_owned())?;
        let mut bmi = make_top_down_bmi(bmp.bmWidth, bmp.bmHeight);
        let byte_len = (w as usize) * (h as usize) * 4;
        let mut raw = vec![0u8; byte_len];

        let hdc = GetDC(None);
        let lines = GetDIBits(
            hdc,
            hbitmap,
            0,
            h,
            Some(raw.as_mut_ptr() as *mut c_void),
            &mut bmi,
            DIB_RGB_COLORS,
        );
        let _ = ReleaseDC(None, hdc);
        if lines == 0 {
            return Err("GetDIBits failed for icon bitmap".to_owned());
        }
        bgra_to_rgba(&mut raw);
        RgbaImage::new(w, h, raw)
    }
}
