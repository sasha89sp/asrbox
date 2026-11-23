use std::cell::RefCell;
use std::ffi::c_void;

use asrbox_platform_api::{
    BgraFrame, PixelRect, PlatformEvent, SettingsInputRects, SettingsUiEvent, WidgetFrame,
};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, InvalidateRect, SetBkColor,
    SetDIBitsToDevice, SetTextColor, DIB_RGB_COLORS, HBRUSH, HDC, HFONT, PAINTSTRUCT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect,
    GetWindowTextLengthW, GetWindowTextW, MoveWindow, SendMessageW, ShowWindow, ES_AUTOHSCROLL,
    ES_PASSWORD, MINMAXINFO, SW_HIDE, SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE,
    WM_CTLCOLOREDIT, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_PAINT, WM_SETFONT, WM_SIZE, WS_BORDER, WS_CHILD, WS_OVERLAPPEDWINDOW,
};

use crate::app_icon::apply_window_icon;
use crate::icons::make_top_down_bmi;
use crate::shell::encode_wide_str;
use crate::tools_win::{focus_window, set_window_text};
use crate::widget_text::composite_widget_frame;
use crate::window::{lparam_point, push_event};

const MIN_CLIENT_W: i32 = 680;
const MIN_CLIENT_H: i32 = 480;
// NotePlan dark input surface (#18191E BGR) and crisp light text (#ECEEF2 BGR)
const EDIT_BG_COLOR: COLORREF = COLORREF(0x001E_1918);
const EDIT_TEXT_COLOR: COLORREF = COLORREF(0x00F2_EEEC);

#[derive(Default)]
pub(crate) struct SettingsWinHandle {
    pub hwnd: Option<HWND>,
    pub edits: [Option<HWND>; 3],
    pub edit_brush: Option<HBRUSH>,
    pub frame: Option<BgraFrame>,
}

thread_local! {
    pub(crate) static SETTINGS_UI: RefCell<SettingsWinHandle> = RefCell::new(SettingsWinHandle::default());
}

pub(crate) fn open_or_focus_settings_window(
    hinstance: HINSTANCE,
    title: &str,
    width: u32,
    height: u32,
    ui_font: HFONT,
) -> Result<(), String> {
    let existing = SETTINGS_UI.with(|ui| ui.borrow().hwnd);
    if let Some(hwnd) = existing {
        focus_window(hwnd);
        return Ok(());
    }
    let tw = encode_wide_str(title);
    let style = WS_OVERLAPPEDWINDOW;
    let mut wr = RECT {
        left: 0,
        top: 0,
        right: i32::try_from(width).unwrap_or(840),
        bottom: i32::try_from(height).unwrap_or(580),
    };
    // SAFETY: Creating resizable top-level settings window and hidden child EDIT controls on UI thread.
    unsafe {
        let _ = AdjustWindowRectEx(&mut wr, style, false, WINDOW_EX_STYLE(0));
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("AsrboxSettingsWindow"),
            PCWSTR(tw.as_ptr()),
            style,
            140,
            90,
            wr.right - wr.left,
            wr.bottom - wr.top,
            None,
            None,
            Some(hinstance),
            None,
        )
        .map_err(|e| format!("CreateWindowExW(settings) failed: {e}"))?;

        apply_window_icon(hwnd);
        let e0 = make_child_edit(hwnd, hinstance, ui_font, false);
        let e1 = make_child_edit(hwnd, hinstance, ui_font, true);
        let e2 = make_child_edit(hwnd, hinstance, ui_font, false);
        let brush = CreateSolidBrush(EDIT_BG_COLOR);
        SETTINGS_UI.with(|ui| {
            let mut st = ui.borrow_mut();
            st.hwnd = Some(hwnd);
            st.edits = [e0, e1, e2];
            st.edit_brush = Some(brush);
            st.frame = None;
        });
        let _ = ShowWindow(hwnd, SW_SHOW);
        focus_window(hwnd);
    }
    Ok(())
}

unsafe fn make_child_edit(
    parent: HWND,
    hinstance: HINSTANCE,
    ui_font: HFONT,
    password: bool,
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
        26,
        Some(parent),
        None,
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

pub(crate) fn present_settings(frame: &WidgetFrame, inputs: SettingsInputRects) {
    let Ok(composited) = composite_widget_frame(frame) else {
        return;
    };
    let (hwnd, edits) = SETTINGS_UI.with(|ui| {
        let mut st = ui.borrow_mut();
        st.frame = Some(composited);
        (st.hwnd, st.edits)
    });
    let rects = [inputs.primary, inputs.secret, inputs.extra];
    for (edit_opt, rect_opt) in edits.into_iter().zip(rects) {
        if let Some(eh) = edit_opt {
            position_child_edit(eh, rect_opt);
        }
    }
    if let Some(h) = hwnd {
        // SAFETY: Requesting redraw of owned settings window without erasing background.
        unsafe {
            let _ = InvalidateRect(Some(h), None, false);
        }
    }
}

fn position_child_edit(eh: HWND, rect_opt: Option<PixelRect>) {
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
    let edit = SETTINGS_UI.with(|ui| ui.borrow().edits.get(usize::from(slot)).copied().flatten());
    if let Some(eh) = edit {
        set_window_text(eh, text);
    }
}

pub(crate) fn is_settings_open() -> bool {
    SETTINGS_UI.with(|ui| ui.borrow().hwnd.is_some())
}

pub(crate) fn cleanup_settings_window() {
    SETTINGS_UI.with(|ui| {
        let mut st = ui.borrow_mut();
        // SAFETY: Destroying owned settings HWND and GDI brush on teardown.
        unsafe {
            if let Some(hwnd) = st.hwnd.take() {
                let _ = DestroyWindow(hwnd);
            }
            if let Some(br) = st.edit_brush.take() {
                let _ = DeleteObject(br.into());
            }
        }
        st.edits = [None, None, None];
        st.frame = None;
    });
}

pub(crate) unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => LRESULT(1),
        WM_GETMINMAXINFO => {
            if lparam.0 != 0 {
                let mmi = lparam.0 as *mut MINMAXINFO;
                (*mmi).ptMinTrackSize.x = MIN_CLIENT_W + 16;
                (*mmi).ptMinTrackSize.y = MIN_CLIENT_H + 39;
            }
            LRESULT(0)
        }
        WM_SIZE => {
            let mut rc = RECT::default();
            if GetClientRect(hwnd, &mut rc).is_ok() {
                let w = u32::try_from(rc.right.saturating_sub(rc.left).max(1)).unwrap_or(680);
                let h = u32::try_from(rc.bottom.saturating_sub(rc.top).max(1)).unwrap_or(480);
                push_event(PlatformEvent::SettingsEvent(SettingsUiEvent::Resized {
                    width: w,
                    height: h,
                }));
            }
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let pt = lparam_point(lparam);
            push_event(PlatformEvent::SettingsEvent(SettingsUiEvent::PointerMove {
                x: pt.x,
                y: pt.y,
            }));
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let _ = SetCapture(hwnd);
            let pt = lparam_point(lparam);
            push_event(PlatformEvent::SettingsEvent(SettingsUiEvent::MouseDown {
                x: pt.x,
                y: pt.y,
            }));
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let _ = ReleaseCapture();
            let pt = lparam_point(lparam);
            push_event(PlatformEvent::SettingsEvent(SettingsUiEvent::MouseUp {
                x: pt.x,
                y: pt.y,
            }));
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) & 0xFFFF) as i16;
            push_event(PlatformEvent::SettingsEvent(SettingsUiEvent::Wheel {
                delta,
            }));
            LRESULT(0)
        }
        WM_CTLCOLOREDIT => {
            let hdc = HDC(wparam.0 as *mut c_void);
            SetTextColor(hdc, EDIT_TEXT_COLOR);
            SetBkColor(hdc, EDIT_BG_COLOR);
            let br = SETTINGS_UI.with(|ui| ui.borrow().edit_brush.unwrap_or_default());
            LRESULT(br.0 as isize)
        }
        WM_PAINT => paint_settings(hwnd),
        WM_CLOSE => {
            cleanup_settings_window();
            push_event(PlatformEvent::SettingsEvent(SettingsUiEvent::Closed));
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn paint_settings(hwnd: HWND) -> LRESULT {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    SETTINGS_UI.with(|ui| {
        let st = ui.borrow();
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
