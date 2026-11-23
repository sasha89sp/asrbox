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
        let icon = create_noteplan_hicon(16).unwrap_or_default();
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
        let icon = create_noteplan_hicon(32).unwrap_or_default();
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

fn create_noteplan_hicon(size: u32) -> Option<HICON> {
    let pixels = render_noteplan_icon_bgra(size);
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

/// Renders a NotePlan-inspired macOS dark squircle icon with coral-tangerine header band
/// and minimalist warm-cream task/calendar lines (32-bit BGRA, straight alpha for Win32 HICON).
fn render_noteplan_icon_bgra(size: u32) -> Vec<u8> {
    let mut buf = vec![0u8; (size as usize) * (size as usize) * 4];
    let s = size as f32;
    let margin = s * 0.06;
    let body_w = s - margin * 2.0;
    let rad = s * 0.24;

    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let d = rounded_sdf(fx - s * 0.5, fy - s * 0.5, body_w * 0.5, body_w * 0.5, rad);
            let cov = (0.6 - d).clamp(0.0, 1.0);
            if cov <= 0.0 {
                continue;
            }
            let ty = ((fy - margin) / body_w).clamp(0.0, 1.0);
            let mut rgb = sample_noteplan_surface(fx, fy, s, margin, body_w, ty, d);
            // Subtle warm rim highlight on squircle border
            if d > -1.25 {
                let rim = ((d + 1.25) / 1.25).clamp(0.0, 1.0) * 0.28;
                rgb[0] = lerp_u8(rgb[0], 246, rim);
                rgb[1] = lerp_u8(rgb[1], 228, rim);
                rgb[2] = lerp_u8(rgb[2], 216, rim);
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

fn sample_noteplan_surface(
    fx: f32,
    fy: f32,
    s: f32,
    margin: f32,
    body_w: f32,
    ty: f32,
    _d: f32,
) -> [u8; 3] {
    let header_end = margin + body_w * 0.30;
    if fy <= header_end {
        // Signature NotePlan warm coral-tangerine header gradient
        let ht = ((fx - margin) / body_w).clamp(0.0, 1.0);
        let r = (228.0 + ht * 16.0) as u8;
        let g = (112.0 + ht * 24.0) as u8;
        let b = (66.0 + ht * 14.0) as u8;
        return [r, g, b];
    }

    // Warm obsidian card body
    let mut rgb = [
        (36.0 - ty * 12.0) as u8,
        (38.0 - ty * 13.0) as u8,
        (46.0 - ty * 14.0) as u8,
    ];

    // Minimalist NotePlan checklist / planner rows inside lower body
    let ux = (fx - margin) / body_w;
    let uy = (fy - header_end) / (body_w * 0.70);
    let line_h = if s <= 18.0 { 0.14 } else { 0.11 };
    let row_centers = [0.27f32, 0.54, 0.80];
    let row_ends = [0.84f32, 0.72, 0.60];
    for (i, (&cy, &rx_end)) in row_centers.iter().zip(row_ends.iter()).enumerate() {
        if (uy - cy).abs() <= line_h * 0.55 {
            // Coral bullet dot on the left
            if (0.16..=0.28).contains(&ux) {
                rgb = if i == 0 {
                    [238, 134, 84]
                } else {
                    [104, 199, 148]
                };
            } else if ux >= 0.35 && ux <= rx_end {
                // Warm cream note line
                rgb = if i == 0 {
                    [242, 240, 236]
                } else {
                    [186, 192, 204]
                };
            }
        }
    }
    rgb
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
