use std::cell::RefCell;
use std::ffi::c_void;

use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
    DIB_RGB_COLORS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, DestroyIcon, SendMessageW, HICON, ICONINFO, ICON_BIG, ICON_SMALL,
    WM_SETICON,
};

use crate::icons::make_top_down_bmi;

#[derive(Default)]
struct CachedIcons {
    small: Option<HICON>,
    large: Option<HICON>,
}

thread_local! {
    static APP_ICONS: RefCell<CachedIcons> = RefCell::new(CachedIcons::default());
}

pub(crate) fn small_app_icon() -> HICON {
    APP_ICONS.with(|cell| {
        let mut st = cell.borrow_mut();
        if let Some(icon) = st.small {
            return icon;
        }
        let icon = create_fluent_hicon(16).unwrap_or_default();
        if !icon.is_invalid() {
            st.small = Some(icon);
        }
        icon
    })
}

pub(crate) fn large_app_icon() -> HICON {
    APP_ICONS.with(|cell| {
        let mut st = cell.borrow_mut();
        if let Some(icon) = st.large {
            return icon;
        }
        let icon = create_fluent_hicon(32).unwrap_or_default();
        if !icon.is_invalid() {
            st.large = Some(icon);
        }
        icon
    })
}

pub(crate) fn apply_window_icon(hwnd: HWND) {
    let big = large_app_icon();
    let small = small_app_icon();
    // SAFETY: Sending WM_SETICON with cached HICON handles to an owned HWND on UI thread.
    unsafe {
        if !big.is_invalid() {
            let _ = SendMessageW(
                hwnd,
                WM_SETICON,
                Some(WPARAM(ICON_BIG as usize)),
                Some(LPARAM(big.0 as isize)),
            );
        }
        if !small.is_invalid() {
            let _ = SendMessageW(
                hwnd,
                WM_SETICON,
                Some(WPARAM(ICON_SMALL as usize)),
                Some(LPARAM(small.0 as isize)),
            );
        }
    }
}

pub(crate) fn cleanup_app_icons() {
    APP_ICONS.with(|cell| {
        let mut st = cell.borrow_mut();
        // SAFETY: Destroying cached HICON handles once during host teardown.
        unsafe {
            if let Some(h) = st.small.take() {
                let _ = DestroyIcon(h);
            }
            if let Some(h) = st.large.take() {
                let _ = DestroyIcon(h);
            }
        }
    });
}

fn create_fluent_hicon(size: u32) -> Option<HICON> {
    let pixels = render_fluent_icon_bgra(size);
    let side = i32::try_from(size).ok()?;
    let bmi = make_top_down_bmi(side, side);
    let mut bits: *mut c_void = std::ptr::null_mut();

    // SAFETY: Creating color DIB and 1-bpp mask bitmap, turning into HICON, then freeing bitmaps.
    unsafe {
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let hcolor = match CreateDIBSection(Some(hdc_mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0)
        {
            Ok(b) => b,
            Err(_) => {
                let _ = DeleteDC(hdc_mem);
                let _ = ReleaseDC(None, hdc_screen);
                return None;
            }
        };
        if !bits.is_null() {
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits.cast::<u8>(), pixels.len());
        }
        let mask_stride = (size as usize).div_ceil(8).max(2);
        let mask_bytes = vec![0u8; mask_stride * (size as usize)];
        let hmask = CreateBitmap(side, side, 1, 1, Some(mask_bytes.as_ptr().cast()));
        let info = ICONINFO {
            fIcon: true.into(),
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: hmask,
            hbmColor: hcolor,
        };
        let icon = CreateIconIndirect(&info).ok();
        let _ = DeleteObject(hmask.into());
        let _ = DeleteObject(hcolor.into());
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);
        icon
    }
}

/// Renders a Windows 11 Fluent Design rounded squircle icon with Fluent Blue gradient
/// and crisp modular tile grid representing asrbox (32-bit BGRA for Win32 HICON).
fn render_fluent_icon_bgra(size: u32) -> Vec<u8> {
    let mut buf = vec![0u8; (size as usize) * (size as usize) * 4];
    let s = size as f32;
    let margin = s * 0.06;
    let body = s - margin * 2.0;
    let rad = s * 0.24;

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let d = rounded_sdf(fx - s * 0.5, fy - s * 0.5, body * 0.5, body * 0.5, rad);
            let cov = (0.6 - d).clamp(0.0, 1.0);
            if cov <= 0.0 {
                continue;
            }
            let ux = ((fx - margin) / body).clamp(0.0, 1.0);
            let uy = ((fy - margin) / body).clamp(0.0, 1.0);
            let mut rgb = sample_fluent_surface(ux, uy);
            if d > -1.2 {
                let rim = ((d + 1.2) / 1.2).clamp(0.0, 1.0) * 0.30;
                rgb[0] = lerp_u8(rgb[0], 190, rim);
                rgb[1] = lerp_u8(rgb[1], 232, rim);
                rgb[2] = lerp_u8(rgb[2], 255, rim);
            }
            let a = (cov * 255.0).round().clamp(0.0, 255.0) as u8;
            let idx = ((y as usize) * (size as usize) + (x as usize)) * 4;
            buf[idx] = rgb[2];
            buf[idx + 1] = rgb[1];
            buf[idx + 2] = rgb[0];
            buf[idx + 3] = a;
        }
    }
    buf
}

fn sample_fluent_surface(ux: f32, uy: f32) -> [u8; 3] {
    // Windows 11 Fluent Blue gradient (#0067C0 -> #0A4D96) with subtle top-left sheen
    let diag = (ux * 0.45 + uy * 0.55).clamp(0.0, 1.0);
    let bg = [
        lerp_u8(22, 0, diag),
        lerp_u8(136, 84, diag),
        lerp_u8(224, 174, diag),
    ];

    // 4 crisp modular Fluent tiles (2x2 asymmetric bento layout)
    let tile_top_left = in_box(ux, uy, 0.17, 0.17, 0.46, 0.46, 0.07);
    let tile_top_right = in_box(ux, uy, 0.54, 0.17, 0.83, 0.46, 0.07);
    let tile_bottom = in_box(ux, uy, 0.17, 0.54, 0.83, 0.83, 0.07);

    if tile_top_left {
        return [255, 255, 255];
    }
    if tile_top_right {
        return [140, 224, 255];
    }
    if tile_bottom {
        return [230, 244, 255];
    }
    bg
}

fn in_box(ux: f32, uy: f32, x0: f32, y0: f32, x1: f32, y1: f32, r: f32) -> bool {
    let cx = (x0 + x1) * 0.5;
    let cy = (y0 + y1) * 0.5;
    let hx = (x1 - x0) * 0.5;
    let hy = (y1 - y0) * 0.5;
    rounded_sdf(ux - cx, uy - cy, hx, hy, r) <= 0.0
}

fn rounded_sdf(px: f32, py: f32, hx: f32, hy: f32, r: f32) -> f32 {
    let qx = px.abs() - (hx - r);
    let qy = py.abs() - (hy - r);
    qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (f32::from(a) + (f32::from(b) - f32::from(a)) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}
