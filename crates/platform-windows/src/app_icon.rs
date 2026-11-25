use std::cell::RefCell;
use std::ffi::c_void;
use std::mem;

use windows::core::PCWSTR;
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
    BITMAPINFO, BITMAPV5HEADER, BI_BITFIELDS, DIB_RGB_COLORS,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, DestroyIcon, GetSystemMetrics, LoadImageW, SendMessageW, HICON, ICONINFO,
    ICON_BIG, ICON_SMALL, IMAGE_ICON, LR_DEFAULTCOLOR, LR_SHARED, SM_CXICON, SM_CXSMICON,
    WM_SETICON,
};

const APP_ICON_RES_ID: usize = 1;

#[derive(Default)]
struct CachedIconSlot {
    handle: Option<HICON>,
    shared: bool,
}

#[derive(Default)]
struct CachedIcons {
    small: CachedIconSlot,
    large: CachedIconSlot,
}

thread_local! {
    static APP_ICONS: RefCell<CachedIcons> = RefCell::new(CachedIcons::default());
}

pub(crate) fn small_app_icon() -> HICON {
    // SAFETY: Querying standard Windows small icon DPI metric.
    let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.max(16) as u32;
    APP_ICONS.with(|cell| {
        let mut st = cell.borrow_mut();
        get_or_init_icon(&mut st.small, size)
    })
}

pub(crate) fn large_app_icon() -> HICON {
    // SAFETY: Querying standard Windows large icon DPI metric.
    let size = unsafe { GetSystemMetrics(SM_CXICON) }.max(32) as u32;
    APP_ICONS.with(|cell| {
        let mut st = cell.borrow_mut();
        get_or_init_icon(&mut st.large, size)
    })
}

fn get_or_init_icon(slot: &mut CachedIconSlot, size: u32) -> HICON {
    if let Some(icon) = slot.handle {
        return icon;
    }
    if let Some(res_icon) = load_embedded_resource_icon(size) {
        slot.handle = Some(res_icon);
        slot.shared = true;
        return res_icon;
    }
    let fallback = create_fluent_hicon(size).unwrap_or_default();
    if !fallback.is_invalid() {
        slot.handle = Some(fallback);
        slot.shared = false;
    }
    fallback
}

fn load_embedded_resource_icon(size: u32) -> Option<HICON> {
    let side = i32::try_from(size).ok()?;
    // SAFETY: Loading RT_GROUP_ICON #1 embedded in the current executable via LoadImageW.
    unsafe {
        let hmod = GetModuleHandleW(None).ok()?;
        let hinst = HINSTANCE::from(hmod);
        let handle = LoadImageW(
            Some(hinst),
            PCWSTR(APP_ICON_RES_ID as *const u16),
            IMAGE_ICON,
            side,
            side,
            LR_DEFAULTCOLOR | LR_SHARED,
        )
        .ok()?;
        let icon = HICON(handle.0);
        if icon.is_invalid() {
            None
        } else {
            Some(icon)
        }
    }
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
        // SAFETY: Destroying only non-shared runtime HICON handles during host teardown.
        unsafe {
            if let Some(h) = st.small.handle.take() {
                if !st.small.shared {
                    let _ = DestroyIcon(h);
                }
            }
            if let Some(h) = st.large.handle.take() {
                if !st.large.shared {
                    let _ = DestroyIcon(h);
                }
            }
        }
        st.small.shared = false;
        st.large.shared = false;
    });
}

fn create_fluent_hicon(size: u32) -> Option<HICON> {
    let top_down = render_fluent_icon_bgra(size);
    let side = i32::try_from(size).ok()?;
    let v5 = BITMAPV5HEADER {
        bV5Size: u32::try_from(mem::size_of::<BITMAPV5HEADER>()).ok()?,
        bV5Width: side,
        bV5Height: side, // Bottom-up DIB required for reliable Win32 CreateIconIndirect
        bV5Planes: 1,
        bV5BitCount: 32,
        bV5Compression: BI_BITFIELDS,
        bV5RedMask: 0x00FF_0000,
        bV5GreenMask: 0x0000_FF00,
        bV5BlueMask: 0x0000_00FF,
        bV5AlphaMask: 0xFF00_0000,
        ..Default::default()
    };
    let mut bits: *mut c_void = std::ptr::null_mut();

    // SAFETY: Creating bottom-up 32-bit BGRA DIB and aligned 1-bpp AND mask for CreateIconIndirect.
    unsafe {
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let bmi_ptr = (&v5 as *const BITMAPV5HEADER).cast::<BITMAPINFO>();
        let hcolor =
            match CreateDIBSection(Some(hdc_mem), bmi_ptr, DIB_RGB_COLORS, &mut bits, None, 0) {
                Ok(b) => b,
                Err(_) => {
                    let _ = DeleteDC(hdc_mem);
                    let _ = ReleaseDC(None, hdc_screen);
                    return None;
                }
            };
        if !bits.is_null() {
            copy_bottom_up_bgra(&top_down, size as usize, bits.cast::<u8>());
        }
        let mask_bytes = build_and_mask(&top_down, size as usize);
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

unsafe fn copy_bottom_up_bgra(top_down: &[u8], size: usize, dst: *mut u8) {
    let row_bytes = size * 4;
    for y in 0..size {
        let src_row = (size - 1 - y) * row_bytes;
        std::ptr::copy_nonoverlapping(
            top_down.as_ptr().add(src_row),
            dst.add(y * row_bytes),
            row_bytes,
        );
    }
}

fn build_and_mask(top_down: &[u8], size: usize) -> Vec<u8> {
    let stride = size.div_ceil(16) * 2;
    let mut mask = vec![0u8; stride * size];
    for y in 0..size {
        for x in 0..size {
            if top_down[(y * size + x) * 4 + 3] == 0 {
                mask[y * stride + x / 8] |= 1u8 << (7 - (x & 7));
            }
        }
    }
    mask
}

fn render_fluent_icon_bgra(size: u32) -> Vec<u8> {
    let mut frame = asrbox_platform_api::BgraFrame::transparent(size, size);
    asrbox_platform_api::draw_asr_glass_logo(
        &mut frame,
        0,
        0,
        i32::try_from(size).unwrap_or(32),
        true,
        false,
    );
    frame.pixels
}
