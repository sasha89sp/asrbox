use std::ffi::c_void;

use asrbox_platform_api::{PixelPoint, PixelRect};
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWINDOWATTRIBUTE,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, ClientToScreen, EndPaint, SetDIBitsToDevice, DIB_RGB_COLORS, HFONT, PAINTSTRUCT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, GetWindowTextLengthW, GetWindowTextW, MoveWindow, SendMessageW, ShowWindow,
    ES_AUTOHSCROLL, ES_PASSWORD, HMENU, SW_HIDE, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE,
    WM_SETFONT, WS_BORDER, WS_CHILD,
};

use crate::icons::make_top_down_bmi;
use crate::settings_proc::SETTINGS_UI;
use crate::tools_win::set_window_text;

const DWMWA_SYSTEMBACKDROP_TYPE_ATTR: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(38);
const DWMSBT_NONE: i32 = 1;
const DWMSBT_MAINWINDOW: i32 = 2;

pub(crate) fn rgba_to_colorref(c: [u8; 4]) -> COLORREF {
    COLORREF(u32::from(c[0]) | (u32::from(c[1]) << 8) | (u32::from(c[2]) << 16))
}

pub(crate) unsafe fn apply_dwm_dark_mode(hwnd: HWND, dark: bool) {
    let flag: i32 = if dark { 1 } else { 0 };
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_USE_IMMERSIVE_DARK_MODE,
        (&flag as *const i32).cast::<c_void>(),
        std::mem::size_of::<i32>() as u32,
    );
}

pub(crate) unsafe fn apply_dwm_backdrop(hwnd: HWND, glass: bool) {
    let backdrop: i32 = if glass {
        DWMSBT_MAINWINDOW
    } else {
        DWMSBT_NONE
    };
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_SYSTEMBACKDROP_TYPE_ATTR,
        (&backdrop as *const i32).cast::<c_void>(),
        std::mem::size_of::<i32>() as u32,
    );
}

pub(crate) fn settings_client_to_screen(pt: PixelPoint) -> PixelPoint {
    let hwnd = SETTINGS_UI.with(|ui| ui.borrow().hwnd);
    let Some(h) = hwnd else {
        return pt;
    };
    let mut p = POINT { x: pt.x, y: pt.y };
    // SAFETY: Converting client coordinates to screen coordinates on an owned HWND.
    unsafe {
        let _ = ClientToScreen(h, &mut p);
    }
    PixelPoint { x: p.x, y: p.y }
}

pub(crate) unsafe fn make_child_edit(
    parent: HWND,
    hinstance: HINSTANCE,
    ui_font: HFONT,
    password: bool,
    ctrl_id: usize,
) -> Option<HWND> {
    let mut raw = (WS_CHILD | WS_BORDER).0 | (ES_AUTOHSCROLL as u32);
    if password {
        raw |= ES_PASSWORD as u32;
    }
    let eh = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("EDIT"),
        w!(""),
        WINDOW_STYLE(raw),
        0,
        0,
        100,
        28,
        Some(parent),
        Some(HMENU(ctrl_id as *mut c_void)),
        Some(hinstance),
        None,
    )
    .ok()?;
    let _ = SendMessageW(
        eh,
        WM_SETFONT,
        Some(WPARAM(ui_font.0 as usize)),
        Some(LPARAM(1)),
    );
    let _ = ShowWindow(eh, SW_HIDE);
    Some(eh)
}

pub(crate) fn position_child_edit(eh: HWND, rect_opt: Option<PixelRect>) {
    // SAFETY: Positioning or hiding child EDIT control owned by the settings window.
    unsafe {
        if let Some(r) = rect_opt {
            let _ = MoveWindow(eh, r.x, r.y, r.width as i32, r.height as i32, true);
            let _ = ShowWindow(eh, SW_SHOW);
        } else {
            let _ = ShowWindow(eh, SW_HIDE);
        }
    }
}

pub(crate) fn read_settings_input_text(slot: u8) -> String {
    let edit = SETTINGS_UI.with(|ui| ui.borrow().edits.get(usize::from(slot)).copied().flatten());
    let Some(eh) = edit else {
        return String::new();
    };
    // SAFETY: Reading UTF-16 text from owned child EDIT control.
    unsafe {
        let len = GetWindowTextLengthW(eh);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; (len as usize) + 1];
        let copied = GetWindowTextW(eh, &mut buf);
        String::from_utf16_lossy(&buf[..copied as usize])
    }
}

pub(crate) fn write_settings_input_text(slot: u8, text: &str) {
    let edit = SETTINGS_UI.with(|ui| {
        let mut st = ui.borrow_mut();
        st.suppress_change = true;
        st.edits.get(usize::from(slot)).copied().flatten()
    });
    if let Some(eh) = edit {
        set_window_text(eh, text);
    }
    SETTINGS_UI.with(|ui| ui.borrow_mut().suppress_change = false);
}

pub(crate) unsafe fn paint_settings(hwnd: HWND) -> LRESULT {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    SETTINGS_UI.with(|ui| {
        let Ok(st) = ui.try_borrow() else {
            return;
        };
        if let Some(frame) = &st.frame {
            let w = i32::try_from(frame.width).unwrap_or(0);
            let h = i32::try_from(frame.height).unwrap_or(0);
            if w > 0 && h > 0 && frame.pixels.len() >= (w as usize) * (h as usize) * 4 {
                let bmi = make_top_down_bmi(w, h);
                let _ = SetDIBitsToDevice(
                    hdc,
                    0,
                    0,
                    frame.width,
                    frame.height,
                    0,
                    0,
                    0,
                    frame.height,
                    frame.pixels.as_ptr().cast::<c_void>(),
                    &bmi,
                    DIB_RGB_COLORS,
                );
            }
        }
    });
    let _ = EndPaint(hwnd, &ps);
    LRESULT(0)
}
