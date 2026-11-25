use std::cell::RefCell;
use std::ffi::c_void;

use asrbox_platform_api::{
    BgraFrame, FluentPalette, NotesInputRects, PixelRect, WidgetFrame, WidgetLayer,
};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, InvalidateRect, HBRUSH, HFONT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetWindowTextLengthW, GetWindowTextW, KillTimer, MoveWindow,
    SendMessageW, SetLayeredWindowAttributes, SetWindowLongPtrW, ShowWindow, ES_AUTOHSCROLL,
    ES_AUTOVSCROLL, ES_MULTILINE, ES_WANTRETURN, GWLP_WNDPROC, HMENU, LWA_ALPHA, SW_HIDE, SW_SHOW,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_SETFONT, WS_BORDER, WS_CHILD, WS_CLIPCHILDREN, WS_EX_LAYERED,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VSCROLL,
};

use crate::app_icon::apply_window_icon;
use crate::font_engine::set_active_font_renderer;
use crate::notes_proc::{inline_edit_wnd_proc, NOTES_SAVE_TIMER_ID};
use crate::settings_helpers::{apply_dwm_dark_mode, rgba_to_colorref};
use crate::shell::encode_wide_str;
use crate::tools_win::{focus_window, set_window_text};
use crate::widget_proc::apply_widget_layer;
use crate::widget_text::composite_widget_frame;

pub(crate) const NOTES_EDIT_SLOTS: usize = 4;
pub(crate) const NOTES_EDIT_ID_BASE: usize = 800;
const DWMWA_CORNER_PREF: DWMWINDOWATTRIBUTE = DWMWINDOWATTRIBUTE(33);
const DWMWCP_ROUND: i32 = 2;

pub(crate) struct NotesWinHandle {
    pub hwnd: Option<HWND>,
    pub edits: [Option<HWND>; NOTES_EDIT_SLOTS],
    pub edit_rects: [Option<PixelRect>; NOTES_EDIT_SLOTS],
    pub inline_orig_proc: isize,
    pub edit_brush: Option<HBRUSH>,
    pub edit_bg: COLORREF,
    pub edit_fg: COLORREF,
    pub layer: WidgetLayer,
    pub suppress_change: bool,
    pub frame: Option<BgraFrame>,
}

impl Default for NotesWinHandle {
    fn default() -> Self {
        Self {
            hwnd: None,
            edits: [None; NOTES_EDIT_SLOTS],
            edit_rects: [None; NOTES_EDIT_SLOTS],
            inline_orig_proc: 0,
            edit_brush: None,
            edit_bg: COLORREF(0x0024_2424),
            edit_fg: COLORREF(0x00FF_FFFF),
            layer: WidgetLayer::Normal,
            suppress_change: false,
            frame: None,
        }
    }
}

thread_local! {
    pub(crate) static NOTES_UI: RefCell<NotesWinHandle> = RefCell::new(NotesWinHandle::default());
}

pub(crate) fn notes_hwnd_or(fallback: HWND) -> HWND {
    NOTES_UI.with(|ui| ui.borrow().hwnd.unwrap_or(fallback))
}

pub(crate) fn open_notes_win(
    hinstance: HINSTANCE,
    title: &str,
    rect: PixelRect,
    layer: WidgetLayer,
    opacity_percent: u8,
    ui_font: HFONT,
    palette: FluentPalette,
) -> Result<(), String> {
    if let Some(hwnd) = NOTES_UI.with(|ui| ui.borrow().hwnd) {
        focus_window(hwnd);
        return Ok(());
    }
    let tw = encode_wide_str(title);
    let mut ex = WS_EX_LAYERED | WS_EX_TOOLWINDOW;
    if layer == WidgetLayer::Topmost {
        ex |= WS_EX_TOPMOST;
    }
    let w = i32::try_from(rect.width).unwrap_or(780).max(300);
    let h = i32::try_from(rect.height).unwrap_or(520).max(320);

    // SAFETY: Creating layered notes popup window and child EDIT controls on the UI thread.
    unsafe {
        let hwnd = CreateWindowExW(
            ex,
            w!("AsrboxNotesWindow"),
            PCWSTR(tw.as_ptr()),
            WS_POPUP | WS_CLIPCHILDREN,
            rect.x,
            rect.y,
            w,
            h,
            None,
            None,
            Some(hinstance),
            None,
        )
        .map_err(|e| format!("CreateWindowExW(notes) failed: {e}"))?;

        apply_window_icon(hwnd);
        apply_dwm_dark_mode(hwnd, palette.is_dark);
        let corner = DWMWCP_ROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_CORNER_PREF,
            (&corner as *const i32).cast::<c_void>(),
            std::mem::size_of::<i32>() as u32,
        );
        apply_alpha(hwnd, opacity_percent);
        apply_widget_layer(hwnd, layer);

        let e0 = make_notes_edit(hwnd, hinstance, ui_font, false, NOTES_EDIT_ID_BASE);
        let e1 = make_notes_edit(hwnd, hinstance, ui_font, false, NOTES_EDIT_ID_BASE + 1);
        let e2 = make_notes_edit(hwnd, hinstance, ui_font, true, NOTES_EDIT_ID_BASE + 2);
        let e3 = make_notes_edit(hwnd, hinstance, ui_font, false, NOTES_EDIT_ID_BASE + 3);
        let orig_proc = e3.map_or(0, |eh| {
            SetWindowLongPtrW(eh, GWLP_WNDPROC, inline_edit_wnd_proc as *const () as isize)
        });

        let bg = rgba_to_colorref(palette.input_bg);
        let fg = rgba_to_colorref(palette.text_primary);
        let brush = CreateSolidBrush(bg);
        NOTES_UI.with(|ui| {
            let mut st = ui.borrow_mut();
            if let Some(old) = st.edit_brush.take() {
                let _ = DeleteObject(old.into());
            }
            st.hwnd = Some(hwnd);
            st.edits = [e0, e1, e2, e3];
            st.edit_rects = [None; NOTES_EDIT_SLOTS];
            st.inline_orig_proc = orig_proc;
            st.edit_brush = Some(brush);
            st.edit_bg = bg;
            st.edit_fg = fg;
            st.layer = layer;
            st.frame = None;
        });
        let _ = ShowWindow(hwnd, SW_SHOW);
    }
    Ok(())
}

pub(crate) fn close_notes_win() {
    let (hwnd, brush) = NOTES_UI.with(|ui| {
        let Ok(mut st) = ui.try_borrow_mut() else {
            return (None, None);
        };
        st.edits = [None; NOTES_EDIT_SLOTS];
        st.edit_rects = [None; NOTES_EDIT_SLOTS];
        st.inline_orig_proc = 0;
        st.frame = None;
        (st.hwnd.take(), st.edit_brush.take())
    });
    // SAFETY: Destroying owned notes window, debounce timer, and GDI brush after releasing RefCell borrow.
    unsafe {
        if let Some(h) = hwnd {
            let _ = KillTimer(Some(h), NOTES_SAVE_TIMER_ID);
            let _ = DestroyWindow(h);
        }
        if let Some(br) = brush {
            let _ = DeleteObject(br.into());
        }
    }
}

pub(crate) fn present_notes_win(frame: &WidgetFrame, inputs: NotesInputRects) {
    let Ok(composited) = composite_widget_frame(frame) else {
        return;
    };
    let rects = [inputs.search, inputs.title, inputs.body, inputs.inline_edit];
    let (hwnd, updates) = NOTES_UI.with(|ui| {
        let mut st = ui.borrow_mut();
        st.frame = Some(composited);
        let mut updates: [(Option<HWND>, Option<PixelRect>, bool); NOTES_EDIT_SLOTS] =
            [(None, None, false); NOTES_EDIT_SLOTS];
        for i in 0..NOTES_EDIT_SLOTS {
            let changed = st.edit_rects[i] != rects[i];
            st.edit_rects[i] = rects[i];
            updates[i] = (st.edits[i], rects[i], changed);
        }
        (st.hwnd, updates)
    });
    for (edit_opt, rect_opt, changed) in updates {
        if changed {
            if let Some(eh) = edit_opt {
                position_notes_edit(eh, rect_opt);
            }
        }
    }
    if let Some(h) = hwnd {
        // SAFETY: Invalidating owned notes HWND client area (WS_CLIPCHILDREN protects child EDITs).
        unsafe {
            let _ = InvalidateRect(Some(h), None, false);
        }
    }
}

pub(crate) fn read_notes_input(slot: u8) -> String {
    let edit = NOTES_UI.with(|ui| ui.borrow().edits.get(usize::from(slot)).copied().flatten());
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
        String::from_utf16_lossy(&buf[..copied as usize]).replace("\r\n", "\n")
    }
}

pub(crate) fn write_notes_input(slot: u8, text: &str) {
    let edit = NOTES_UI.with(|ui| {
        let mut st = ui.borrow_mut();
        st.suppress_change = true;
        st.edits.get(usize::from(slot)).copied().flatten()
    });
    if let Some(eh) = edit {
        let win_text = if slot == 2 {
            text.replace('\r', "").replace('\n', "\r\n")
        } else {
            text.to_owned()
        };
        set_window_text(eh, &win_text);
    }
    NOTES_UI.with(|ui| ui.borrow_mut().suppress_change = false);
}

pub(crate) fn focus_notes_input_win(slot: u8) {
    let edit = NOTES_UI.with(|ui| ui.borrow().edits.get(usize::from(slot)).copied().flatten());
    if let Some(eh) = edit {
        // SAFETY: Focusing an owned child EDIT control on the UI thread.
        unsafe {
            let _ = SetFocus(Some(eh));
        }
    }
}

pub(crate) fn apply_notes_win_theme(palette: FluentPalette) {
    set_active_font_renderer(palette.font_renderer);
    let bg = rgba_to_colorref(palette.input_bg);
    let fg = rgba_to_colorref(palette.text_primary);
    // SAFETY: Creating a solid brush for child EDIT controls.
    let new_brush = unsafe { CreateSolidBrush(bg) };
    let (old_brush, hwnd, edits) = NOTES_UI.with(|ui| {
        let mut st = ui.borrow_mut();
        st.edit_bg = bg;
        st.edit_fg = fg;
        let old = st.edit_brush.replace(new_brush);
        (old, st.hwnd, st.edits)
    });
    // SAFETY: Deleting old GDI brush and updating DWM/child EDITs.
    unsafe {
        if let Some(old) = old_brush {
            let _ = DeleteObject(old.into());
        }
        if let Some(h) = hwnd {
            apply_dwm_dark_mode(h, palette.is_dark);
            for eh in edits.into_iter().flatten() {
                let _ = InvalidateRect(Some(eh), None, true);
            }
        }
    }
}

pub(crate) fn set_notes_win_layer(layer: WidgetLayer) {
    let hwnd = NOTES_UI.with(|ui| {
        let mut st = ui.borrow_mut();
        st.layer = layer;
        st.hwnd
    });
    if let Some(h) = hwnd {
        apply_widget_layer(h, layer);
    }
}

pub(crate) fn set_notes_win_opacity(opacity_percent: u8) {
    if let Some(h) = NOTES_UI.with(|ui| ui.borrow().hwnd) {
        // SAFETY: Updating layered alpha attribute on owned notes HWND.
        unsafe {
            apply_alpha(h, opacity_percent);
        }
    }
}

unsafe fn apply_alpha(hwnd: HWND, opacity_percent: u8) {
    let alpha = ((u32::from(opacity_percent.clamp(35, 100)) * 255) / 100) as u8;
    let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);
}

unsafe fn make_notes_edit(
    parent: HWND,
    hinstance: HINSTANCE,
    ui_font: HFONT,
    multiline: bool,
    ctrl_id: usize,
) -> Option<HWND> {
    let raw = if multiline {
        (WS_CHILD | WS_BORDER | WS_VSCROLL).0
            | (ES_MULTILINE as u32)
            | (ES_WANTRETURN as u32)
            | (ES_AUTOVSCROLL as u32)
    } else {
        (WS_CHILD | WS_BORDER).0 | (ES_AUTOHSCROLL as u32)
    };
    let eh = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("EDIT"),
        w!(""),
        WINDOW_STYLE(raw),
        0,
        0,
        100,
        24,
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

fn position_notes_edit(eh: HWND, rect_opt: Option<PixelRect>) {
    // SAFETY: Positioning or hiding child EDIT control owned by the notes window.
    unsafe {
        if let Some(r) = rect_opt {
            let _ = MoveWindow(eh, r.x, r.y, r.width as i32, r.height as i32, true);
            let _ = ShowWindow(eh, SW_SHOW);
        } else {
            let _ = ShowWindow(eh, SW_HIDE);
        }
    }
}
