use std::mem;
use std::path::{Path, PathBuf};

use asrbox_platform_api::RgbaImage;
use windows::core::{Interface, PCWSTR};
use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
use windows::Win32::System::Com::{
    CoCreateInstance, IPersistFile, CLSCTX_INPROC_SERVER, STGM_READ,
};
use windows::Win32::System::Environment::ExpandEnvironmentStringsW;
use windows::Win32::UI::Shell::{
    IShellLinkW, SHGetFileInfoW, ShellLink, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SLGP_RAWPATH,
};
use windows::Win32::UI::WindowsAndMessaging::DestroyIcon;

use crate::icons::{hicon_to_rgba, to_wide};

#[derive(Debug, Default)]
pub(crate) struct LnkIconInfo {
    pub icon_path: Option<PathBuf>,
    pub icon_index: i32,
    pub target_path: Option<PathBuf>,
}

pub(crate) fn resolve_lnk_icon_info(lnk_path: &Path) -> Option<LnkIconInfo> {
    let wide = to_wide(lnk_path);
    // SAFETY: COM is initialized on UI thread; COM interfaces are RAII-released.
    unsafe {
        let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER).ok()?;
        let persist: IPersistFile = link.cast().ok()?;
        persist.Load(PCWSTR(wide.as_ptr()), STGM_READ).ok()?;

        let mut icon_buf = [0u16; 520];
        let mut icon_index = 0i32;
        let icon_path = if link
            .GetIconLocation(&mut icon_buf, &mut icon_index as *mut i32)
            .is_ok()
        {
            extract_expanded_path(&icon_buf)
        } else {
            None
        };

        let mut target_buf = [0u16; 520];
        let target_path = if link
            .GetPath(&mut target_buf, std::ptr::null_mut(), SLGP_RAWPATH.0 as u32)
            .is_ok()
        {
            extract_expanded_path(&target_buf)
        } else {
            None
        };

        Some(LnkIconInfo {
            icon_path,
            icon_index,
            target_path,
        })
    }
}

pub(crate) fn load_shgetfileinfo_icon(path: &Path, size_px: u32) -> Result<RgbaImage, String> {
    let wide = to_wide(path);
    let mut info = SHFILEINFOW::default();
    let cb = u32::try_from(mem::size_of::<SHFILEINFOW>()).unwrap_or(0);
    // SAFETY: `wide` is NUL-terminated; returned HICON is destroyed after rasterization.
    let mut ok = unsafe {
        SHGetFileInfoW(
            PCWSTR(wide.as_ptr()),
            FILE_FLAGS_AND_ATTRIBUTES(0),
            Some(&mut info as *mut SHFILEINFOW),
            cb,
            SHGFI_ICON | SHGFI_LARGEICON,
        )
    };
    if ok == 0 || info.hIcon.is_invalid() {
        let is_dir_like = path.extension().is_none() || path.is_dir();
        let attrs = FILE_FLAGS_AND_ATTRIBUTES(if is_dir_like { 0x0010 } else { 0x0080 });
        let flags =
            SHGFI_ICON | SHGFI_LARGEICON | windows::Win32::UI::Shell::SHGFI_FLAGS(0x0000_0010);
        // SAFETY: `wide` is NUL-terminated; fallback uses SHGFI_USEFILEATTRIBUTES.
        ok = unsafe {
            SHGetFileInfoW(
                PCWSTR(wide.as_ptr()),
                attrs,
                Some(&mut info as *mut SHFILEINFOW),
                cb,
                flags,
            )
        };
    }
    if ok == 0 || info.hIcon.is_invalid() {
        return Err("SHGetFileInfoW returned no icon".to_owned());
    }
    let res = hicon_to_rgba(info.hIcon, size_px);
    // SAFETY: `info.hIcon` was created by SHGetFileInfoW and is destroyed once.
    unsafe {
        let _ = DestroyIcon(info.hIcon);
    }
    res
}

/// Detects the hollow gray border box Windows Explorer draws around legacy 32px icons
/// when `IShellItemImageFactory::GetImage(128x128, SIIGBF_ICONONLY)` is called.
pub(crate) fn has_explorer_frame_box(img: &RgbaImage) -> bool {
    let (w, h) = (img.width as usize, img.height as usize);
    if w < 48 || h < 48 || img.pixels.len() < w * h * 4 {
        return false;
    }
    if !perimeter_looks_like_gray_border(img, w, h) {
        return false;
    }
    inner_moat_is_empty(img, w, h)
}

fn perimeter_looks_like_gray_border(img: &RgbaImage, w: usize, h: usize) -> bool {
    let mut total = 0usize;
    let mut gray_edge = 0usize;
    for x in 0..w {
        for y in [0, h - 1] {
            total += 1;
            if is_low_chroma_border_pixel(pixel_at(img, w, x, y)) {
                gray_edge += 1;
            }
        }
    }
    for y in 1..h.saturating_sub(1) {
        for x in [0, w - 1] {
            total += 1;
            if is_low_chroma_border_pixel(pixel_at(img, w, x, y)) {
                gray_edge += 1;
            }
        }
    }
    total > 0 && gray_edge * 100 >= total * 78
}

fn inner_moat_is_empty(img: &RgbaImage, w: usize, h: usize) -> bool {
    let m0 = (w.min(h) / 14).clamp(4, 10);
    let m1 = (m0 + 4).min(w.min(h) / 4);
    let mut total = 0usize;
    let mut sparse = 0usize;
    let sample = pixel_at(img, w, m0, m0);

    for d in m0..m1 {
        for x in d..w.saturating_sub(d) {
            for y in [d, h - 1 - d] {
                total += 1;
                if is_moat_pixel(pixel_at(img, w, x, y), sample) {
                    sparse += 1;
                }
            }
        }
        for y in (d + 1)..h.saturating_sub(d + 1) {
            for x in [d, w - 1 - d] {
                total += 1;
                if is_moat_pixel(pixel_at(img, w, x, y), sample) {
                    sparse += 1;
                }
            }
        }
    }
    total > 0 && sparse * 100 >= total * 80
}

fn pixel_at(img: &RgbaImage, w: usize, x: usize, y: usize) -> [u8; 4] {
    let i = (y * w + x) * 4;
    [
        img.pixels[i],
        img.pixels[i + 1],
        img.pixels[i + 2],
        img.pixels[i + 3],
    ]
}

fn is_low_chroma_border_pixel(px: [u8; 4]) -> bool {
    if px[3] < 24 {
        return false;
    }
    let hi = px[0].max(px[1]).max(px[2]);
    let lo = px[0].min(px[1]).min(px[2]);
    hi.saturating_sub(lo) <= 36
}

fn is_moat_pixel(px: [u8; 4], sample: [u8; 4]) -> bool {
    if px[3] <= 42 {
        return true;
    }
    let hi = px[0].max(px[1]).max(px[2]);
    let lo = px[0].min(px[1]).min(px[2]);
    if hi.saturating_sub(lo) > 22 || hi > 92 {
        return false;
    }
    let diff = (i16::from(px[0]) - i16::from(sample[0])).abs()
        + (i16::from(px[1]) - i16::from(sample[1])).abs()
        + (i16::from(px[2]) - i16::from(sample[2])).abs()
        + (i16::from(px[3]) - i16::from(sample[3])).abs();
    diff <= 32
}

fn extract_expanded_path(buf: &[u16]) -> Option<PathBuf> {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    if end == 0 {
        return None;
    }
    let raw = String::from_utf16_lossy(&buf[..end]);
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let expanded = expand_env_vars(trimmed);
    Some(PathBuf::from(expanded))
}

fn expand_env_vars(raw: &str) -> String {
    if !raw.contains('%') {
        return raw.to_owned();
    }
    let wide: Vec<u16> = raw.encode_utf16().chain(std::iter::once(0)).collect();
    let mut out = [0u16; 1024];
    // SAFETY: `wide` is NUL-terminated; `out` buffer length matches slice capacity.
    let len = unsafe { ExpandEnvironmentStringsW(PCWSTR(wide.as_ptr()), Some(&mut out)) };
    if len == 0 || (len as usize) > out.len() {
        return raw.to_owned();
    }
    let valid = (len as usize).saturating_sub(1);
    String::from_utf16_lossy(&out[..valid])
}

pub(crate) fn has_ext(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case(expected))
        .unwrap_or(false)
}

pub(crate) fn is_executable_ext(path: &Path) -> bool {
    has_ext(path, "exe") || has_ext(path, "dll") || has_ext(path, "icl")
}

pub(crate) fn is_image_ext(path: &Path) -> bool {
    has_ext(path, "png") || has_ext(path, "jpg") || has_ext(path, "jpeg") || has_ext(path, "bmp")
}

pub(crate) fn recover_alpha_from_black_white(black: &mut [u8], white: &[u8]) {
    for (b_px, w_px) in black
        .as_chunks_mut::<4>()
        .0
        .iter_mut()
        .zip(white.as_chunks::<4>().0.iter())
    {
        let diff = w_px[0]
            .saturating_sub(b_px[0])
            .max(w_px[1].saturating_sub(b_px[1]))
            .max(w_px[2].saturating_sub(b_px[2]));
        b_px[3] = 255u8.saturating_sub(diff);
    }
}

pub(crate) fn bgra_to_rgba(pixels: &mut [u8]) {
    let mut any_alpha = false;
    let mut any_color = false;
    for chunk in pixels.as_chunks_mut::<4>().0 {
        chunk.swap(0, 2);
        if chunk[3] != 0 {
            any_alpha = true;
        }
        if chunk[0] != 0 || chunk[1] != 0 || chunk[2] != 0 {
            any_color = true;
        }
    }
    if !any_alpha && any_color {
        for chunk in pixels.as_chunks_mut::<4>().0 {
            chunk[3] = 255;
        }
    }
}
