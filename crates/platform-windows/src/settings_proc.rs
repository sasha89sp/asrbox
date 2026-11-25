use std::cell::RefCell;
use std::ffi::c_void;

use asrbox_platform_api::{
    BgraFrame, DesignTheme, FluentPalette, PixelRect, PlatformEvent, SettingsInputRects,
    SettingsUiEvent, WidgetFrame,
};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, InvalidateRect, SetBkColor, SetTextColor, HBRUSH, HDC, HFONT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, KillTimer,
    SetTimer, ShowWindow, MINMAXINFO, SW_SHOW, WINDOW_EX_STYLE, WM_CLOSE, WM_COMMAND,
    WM_CTLCOLOREDIT, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_PAINT, WM_SIZE, WM_TIMER, WS_CLIPCHILDREN, WS_OVERLAPPEDWINDOW,
};

use crate::app_icon::apply_window_icon;
use crate::font_engine::set_active_font_renderer;
use crate::settings_helpers::{
    apply_dwm_backdrop, apply_dwm_dark_mode, make_child_edit, paint_settings, position_child_edit,
    rgba_to_colorref,
};
pub(crate) use crate::settings_helpers::{
    read_settings_input_text, settings_client_to_screen, write_settings_input_text,
};
use crate::shell::encode_wide_str;
use crate::tools_win::focus_window;
use crate::widget_text::composite_widget_frame;
use crate::window::{lparam_point, push_event};

const MIN_CLIENT_W: i32 = 720;
const MIN_CLIENT_H: i32 = 460;
const EDIT_SLOTS: usize = 9;
const EDIT_ID_BASE: usize = 700;
const EN_CHANGE: u32 = 0x0300;
const SETTINGS_ANIM_TIMER_ID: usize = 61;
const SETTINGS_ANIM_TICK_MS: u32 = 16;

pub(crate) struct SettingsWinHandle {
    pub hwnd: Option<HWND>,
    pub edits: [Option<HWND>; EDIT_SLOTS],
    pub edit_rects: [Option<PixelRect>; EDIT_SLOTS],
    pub edit_brush: Option<HBRUSH>,
    pub edit_bg: COLORREF,
    pub edit_fg: COLORREF,
    pub dark_title: bool,
    pub suppress_change: bool,
    pub frame: Option<BgraFrame>,
}

impl Default for SettingsWinHandle {
    fn default() -> Self {
        Self {
            hwnd: None,
            edits: [None; EDIT_SLOTS],
            edit_rects: [None; EDIT_SLOTS],
            edit_brush: None,
            edit_bg: COLORREF(0x002B_2B2B),
            edit_fg: COLORREF(0x00FF_FFFF),
            dark_title: true,
            suppress_change: false,
            frame: None,
        }
    }
}

thread_local! {
    pub(crate) static SETTINGS_UI: RefCell<SettingsWinHandle> = RefCell::new(SettingsWinHandle::default());
}

pub(crate) fn settings_hwnd_or(fallback: HWND) -> HWND {
    SETTINGS_UI.with(|ui| ui.borrow().hwnd.unwrap_or(fallback))
}

pub(crate) fn apply_settings_window_theme(palette: FluentPalette) {
    set_active_font_renderer(palette.font_renderer);
    let bg = rgba_to_colorref(palette.input_bg);
    let fg = rgba_to_colorref(palette.text_primary);
    // SAFETY: Creating a GDI solid brush owned by SETTINGS_UI.
    let new_brush = unsafe { CreateSolidBrush(bg) };
    let (old_brush, hwnd, edits) = SETTINGS_UI.with(|ui| {
        let mut st = ui.borrow_mut();
        st.edit_bg = bg;
        st.edit_fg = fg;
        st.dark_title = palette.is_dark;
        let old = st.edit_brush.replace(new_brush);
        (old, st.hwnd, st.edits)
    });
    // SAFETY: Deleting old GDI brush and updating DWM/child edits after releasing SETTINGS_UI borrow.
    unsafe {
        if let Some(old) = old_brush {
            let _ = DeleteObject(old.into());
        }
        if let Some(hwnd) = hwnd {
            apply_dwm_dark_mode(hwnd, palette.is_dark);
            apply_dwm_backdrop(hwnd, palette.design_theme == DesignTheme::AsrGlass);
            for eh in edits.into_iter().flatten() {
                let _ = InvalidateRect(Some(eh), None, true);
            }
        }
    }
}

pub(crate) fn set_settings_window_anim_timer(active: bool) {
    let hwnd = SETTINGS_UI.with(|ui| ui.borrow().hwnd);
    let Some(h) = hwnd else {
        return;
    };
    // SAFETY: Starting or stopping the smooth theme-transition timer on owned settings HWND.
    unsafe {
        if active {
            let _ = SetTimer(Some(h), SETTINGS_ANIM_TIMER_ID, SETTINGS_ANIM_TICK_MS, None);
        } else {
            let _ = KillTimer(Some(h), SETTINGS_ANIM_TIMER_ID);
        }
    }
}

pub(crate) fn open_or_focus_settings_window(
    hinstance: HINSTANCE,
    title: &str,
    width: u32,
    height: u32,
    ui_font: HFONT,
    palette: FluentPalette,
) -> Result<(), String> {
    let existing = SETTINGS_UI.with(|ui| ui.borrow().hwnd);
    if let Some(hwnd) = existing {
        focus_window(hwnd);
        return Ok(());
    }
    let tw = encode_wide_str(title);
    let style = WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN;
    let mut wr = RECT {
        left: 0,
        top: 0,
        right: i32::try_from(width).unwrap_or(920),
        bottom: i32::try_from(height).unwrap_or(620),
    };
    // SAFETY: Creating resizable top-level settings window and hidden child EDIT controls on UI thread.
    unsafe {
        let _ = AdjustWindowRectEx(&mut wr, style, false, WINDOW_EX_STYLE(0));
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            w!("AsrboxSettingsWindow"),
            PCWSTR(tw.as_ptr()),
            style,
            120,
            70,
            wr.right - wr.left,
            wr.bottom - wr.top,
            None,
            None,
            Some(hinstance),
            None,
        )
        .map_err(|e| format!("CreateWindowExW(settings) failed: {e}"))?;

        apply_window_icon(hwnd);
        apply_dwm_dark_mode(hwnd, palette.is_dark);
        apply_dwm_backdrop(hwnd, palette.design_theme == DesignTheme::AsrGlass);
        let edits = [
            make_child_edit(hwnd, hinstance, ui_font, false, EDIT_ID_BASE),
            make_child_edit(hwnd, hinstance, ui_font, true, EDIT_ID_BASE + 1),
            make_child_edit(hwnd, hinstance, ui_font, false, EDIT_ID_BASE + 2),
            make_child_edit(hwnd, hinstance, ui_font, false, EDIT_ID_BASE + 3),
            make_child_edit(hwnd, hinstance, ui_font, false, EDIT_ID_BASE + 4),
            make_child_edit(hwnd, hinstance, ui_font, true, EDIT_ID_BASE + 5),
            make_child_edit(hwnd, hinstance, ui_font, false, EDIT_ID_BASE + 6),
            make_child_edit(hwnd, hinstance, ui_font, false, EDIT_ID_BASE + 7),
            make_child_edit(hwnd, hinstance, ui_font, true, EDIT_ID_BASE + 8),
        ];
        let bg = rgba_to_colorref(palette.input_bg);
        let fg = rgba_to_colorref(palette.text_primary);
        let brush = CreateSolidBrush(bg);
        SETTINGS_UI.with(|ui| {
            let mut st = ui.borrow_mut();
            if let Some(old) = st.edit_brush.take() {
                let _ = DeleteObject(old.into());
            }
            st.hwnd = Some(hwnd);
            st.edits = edits;
            st.edit_rects = [None; EDIT_SLOTS];
            st.edit_brush = Some(brush);
            st.edit_bg = bg;
            st.edit_fg = fg;
            st.dark_title = palette.is_dark;
            st.frame = None;
        });
        let _ = ShowWindow(hwnd, SW_SHOW);
        focus_window(hwnd);
    }
    Ok(())
}

pub(crate) fn present_settings(frame: &WidgetFrame, inputs: SettingsInputRects) {
    let Ok(composited) = composite_widget_frame(frame) else {
        return;
    };
    let rects = [
        inputs.primary,
        inputs.secret,
        inputs.extra,
        inputs.search,
        inputs.aux,
        inputs.zip_secret,
        inputs.gh_repo,
        inputs.gh_branch,
        inputs.gh_token,
    ];
    let (hwnd, updates) = SETTINGS_UI.with(|ui| {
        let mut st = ui.borrow_mut();
        st.frame = Some(composited);
        let mut updates: [(Option<HWND>, Option<PixelRect>, bool); EDIT_SLOTS] =
            [(None, None, false); EDIT_SLOTS];
        for i in 0..EDIT_SLOTS {
            let changed = st.edit_rects[i] != rects[i];
            st.edit_rects[i] = rects[i];
            updates[i] = (st.edits[i], rects[i], changed);
        }
        (st.hwnd, updates)
    });
    for (edit_opt, rect_opt, changed) in updates {
        if changed {
            if let Some(eh) = edit_opt {
                position_child_edit(eh, rect_opt);
            }
        }
    }
    if let Some(h) = hwnd {
        // SAFETY: Requesting redraw of owned settings window without erasing background.
        unsafe {
            let _ = InvalidateRect(Some(h), None, false);
        }
    }
}

pub(crate) fn is_settings_open() -> bool {
    SETTINGS_UI.with(|ui| ui.borrow().hwnd.is_some())
}

pub(crate) fn cleanup_settings_window() {
    let (hwnd, brush) = SETTINGS_UI.with(|ui| {
        let Ok(mut st) = ui.try_borrow_mut() else {
            return (None, None);
        };
        st.edits = [None; EDIT_SLOTS];
        st.edit_rects = [None; EDIT_SLOTS];
        st.frame = None;
        (st.hwnd.take(), st.edit_brush.take())
    });
    // SAFETY: Destroying owned settings HWND, animation timer, and GDI brush after releasing RefCell borrow.
    unsafe {
        if let Some(hwnd) = hwnd {
            let _ = KillTimer(Some(hwnd), SETTINGS_ANIM_TIMER_ID);
            let _ = DestroyWindow(hwnd);
        }
        if let Some(br) = brush {
            let _ = DeleteObject(br.into());
        }
    }
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
                let w = u32::try_from(rc.right.saturating_sub(rc.left).max(1)).unwrap_or(760);
                let h = u32::try_from(rc.bottom.saturating_sub(rc.top).max(1)).unwrap_or(520);
                push_event(PlatformEvent::SettingsEvent(SettingsUiEvent::Resized {
                    width: w,
                    height: h,
                }));
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == SETTINGS_ANIM_TIMER_ID => {
            push_event(PlatformEvent::SettingsEvent(SettingsUiEvent::AnimTick {
                dt_ms: SETTINGS_ANIM_TICK_MS as u16,
            }));
            LRESULT(0)
        }
        WM_COMMAND => {
            let notify = ((wparam.0 >> 16) & 0xFFFF) as u32;
            let ctrl_id = wparam.0 & 0xFFFF;
            if notify == EN_CHANGE && (EDIT_ID_BASE..EDIT_ID_BASE + EDIT_SLOTS).contains(&ctrl_id) {
                let suppressed =
                    SETTINGS_UI.with(|ui| ui.try_borrow().map_or(true, |st| st.suppress_change));
                if !suppressed {
                    let slot = (ctrl_id - EDIT_ID_BASE) as u8;
                    let text = read_settings_input_text(slot);
                    push_event(PlatformEvent::SettingsEvent(
                        SettingsUiEvent::InputChanged { slot, text },
                    ));
                }
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
            let (fg, bg, br) = SETTINGS_UI.with(|ui| {
                ui.try_borrow().map_or(
                    (
                        COLORREF(0x00FF_FFFF),
                        COLORREF(0x002B_2B2B),
                        HBRUSH::default(),
                    ),
                    |st| (st.edit_fg, st.edit_bg, st.edit_brush.unwrap_or_default()),
                )
            });
            SetTextColor(hdc, fg);
            SetBkColor(hdc, bg);
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
