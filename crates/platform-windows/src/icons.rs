use std::ffi::c_void;
use std::mem;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use asrbox_platform_api::{IconProvider, IconRequest, RgbaImage};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{GENERIC_READ, SIZE};
use windows::Win32::Graphics::Gdi::{
    BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CreateCompatibleDC, CreateDIBSection,
    DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, GetDIBits, GetObjectW, HBITMAP, ReleaseDC,
    SelectObject,
};
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppRGBA, IWICImagingFactory,
    WICBitmapDitherTypeNone, WICBitmapPaletteTypeMedianCut, WICDecodeMetadataCacheOnLoad,
};
use windows::Win32::System::Com::{
    CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
};
use windows::Win32::UI::Shell::{
    ExtractIconExW, IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK,
    SIIGBF_ICONONLY,
};
use windows::Win32::UI::WindowsAndMessaging::{DI_NORMAL, DestroyIcon, DrawIconEx, HICON};

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
            IconRequest::AutoShell(path) => {
                load_shell_icon(path, size).or_else(|_| load_exe_resource_icon(path, 0, size))
            }
            IconRequest::CustomFile(path) => {
                if is_executable_ext(path) {
                    load_exe_resource_icon(path, 0, size).or_else(|_| load_shell_icon(path, size))
                } else {
                    load_wic_image(path).or_else(|_| load_shell_icon(path, size))
                }
            }
            IconRequest::ExeResource { path, index } => {
                load_exe_resource_icon(path, *index, size).or_else(|_| load_shell_icon(path, size))
            }
        }
    }
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

fn is_executable_ext(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.eq_ignore_ascii_case("exe") || ext.eq_ignore_ascii_case("dll"))
        .unwrap_or(false)
}

fn to_wide(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
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

fn load_wic_image(path: &Path) -> Result<RgbaImage, String> {
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
        let frame = decoder
            .GetFrame(0)
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

        let mut w = 0u32;
        let mut h = 0u32;
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

fn load_exe_resource_icon(path: &Path, index: u32, size_px: u32) -> Result<RgbaImage, String> {
    let wide = to_wide(path);
    let mut large = HICON::default();
    let idx = i32::try_from(index).unwrap_or(0);
    // SAFETY: `wide` is NUL-terminated; `large` is valid storage and destroyed below.
    let extracted =
        unsafe { ExtractIconExW(PCWSTR(wide.as_ptr()), idx, Some(&mut large), None, 1) };
    if extracted == 0 || large.is_invalid() {
        return Err("ExtractIconExW returned no icon".to_owned());
    }
    let result = hicon_to_rgba(large, size_px);
    // SAFETY: `large` was returned by ExtractIconExW and has not yet been destroyed.
    unsafe {
        let _ = DestroyIcon(large);
    }
    result
}

fn hicon_to_rgba(hicon: HICON, size_px: u32) -> Result<RgbaImage, String> {
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
            std::ptr::write_bytes(bits as *mut u8, 0, byte_len);
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

        bgra_to_rgba(&mut raw);
        RgbaImage::new(size_px, size_px, raw)
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

fn bgra_to_rgba(pixels: &mut [u8]) {
    let mut any_alpha = false;
    let mut any_color = false;
    for chunk in pixels.chunks_exact_mut(4) {
        chunk.swap(0, 2);
        if chunk[3] != 0 {
            any_alpha = true;
        }
        if chunk[0] != 0 || chunk[1] != 0 || chunk[2] != 0 {
            any_color = true;
        }
    }
    if !any_alpha && any_color {
        for chunk in pixels.chunks_exact_mut(4) {
            chunk[3] = 255;
        }
    }
}
