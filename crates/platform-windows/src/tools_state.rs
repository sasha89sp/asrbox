use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, DeleteObject, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
    DEFAULT_PITCH, FF_DONTCARE, FW_NORMAL, HFONT, OUT_DEFAULT_PRECIS,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyWindow, KillTimer, SendMessageW, SetForegroundWindow, SetWindowTextW, ShowWindow,
    SW_RESTORE,
};

use crate::font_engine::preferred_ui_font_face;
use crate::settings_proc::cleanup_settings_window;
use crate::shell::encode_wide_str;
use crate::tools_proc::{
    cleanup_tool_brushes, AboutControls, DebugControls, ABOUT_UI, DEBUG_TICK_TIMER_ID, DEBUG_UI,
};

pub(crate) struct ToolsWinState {
    ui_font: Option<HFONT>,
    mono_font: Option<HFONT>,
    pub(crate) last_summary: String,
    pub(crate) last_logs: String,
    pub(crate) last_verbose: Option<bool>,
    pub(crate) last_file: Option<bool>,
    pub(crate) last_filter: String,
}

impl ToolsWinState {
    pub(crate) fn new() -> Self {
        Self {
            ui_font: None,
            mono_font: None,
            last_summary: String::new(),
            last_logs: String::new(),
            last_verbose: None,
            last_file: None,
            last_filter: String::new(),
        }
    }

    pub(crate) fn cleanup(&mut self) {
        cleanup_settings_window();
        let debug_hwnd = DEBUG_UI.with(|ui| {
            let Ok(mut st) = ui.try_borrow_mut() else {
                return None;
            };
            let h = st.hwnd.take();
            *st = DebugControls::default();
            h
        });
        if let Some(hwnd) = debug_hwnd {
            // SAFETY: Destroying owned debug window after releasing DEBUG_UI borrow.
            unsafe {
                let _ = KillTimer(Some(hwnd), DEBUG_TICK_TIMER_ID);
                let _ = DestroyWindow(hwnd);
            }
        }
        let about_hwnd = ABOUT_UI.with(|ui| {
            let Ok(mut st) = ui.try_borrow_mut() else {
                return None;
            };
            let h = st.hwnd.take();
            *st = AboutControls::default();
            h
        });
        if let Some(hwnd) = about_hwnd {
            // SAFETY: Destroying owned about window after releasing ABOUT_UI borrow.
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
        cleanup_tool_brushes();
        // SAFETY: Deleting owned GDI font handles once on teardown.
        unsafe {
            if let Some(f) = self.ui_font.take() {
                let _ = DeleteObject(f.into());
            }
            if let Some(f) = self.mono_font.take() {
                let _ = DeleteObject(f.into());
            }
        }
    }

    pub(crate) fn ensure_fonts(&mut self) -> (HFONT, HFONT) {
        if self.ui_font.is_none() {
            self.ui_font = Some(create_font(preferred_ui_font_face(), 15));
        }
        if self.mono_font.is_none() {
            self.mono_font = Some(create_font("Consolas", 14));
        }
        (
            self.ui_font.unwrap_or_default(),
            self.mono_font.unwrap_or_default(),
        )
    }
}

fn create_font(face: &str, height_px: i32) -> HFONT {
    let w = encode_wide_str(face);
    // SAFETY: Creating a GDI font handle freed in ToolsWinState::cleanup.
    unsafe {
        CreateFontW(
            -height_px,
            0,
            0,
            0,
            FW_NORMAL.0 as i32,
            0,
            0,
            0,
            DEFAULT_CHARSET,
            OUT_DEFAULT_PRECIS,
            CLIP_DEFAULT_PRECIS,
            CLEARTYPE_QUALITY,
            (DEFAULT_PITCH.0 | FF_DONTCARE.0) as u32,
            PCWSTR(w.as_ptr()),
        )
    }
}

pub(crate) fn focus_window(hwnd: HWND) {
    // SAFETY: Restoring and focusing an owned tool window on UI thread.
    unsafe {
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
    }
}

pub(crate) fn set_window_text(hwnd: HWND, text: &str) {
    let w = encode_wide_str(text);
    // SAFETY: `w` is NUL-terminated UTF-16 valid during synchronous SetWindowTextW.
    unsafe {
        let _ = SetWindowTextW(hwnd, PCWSTR(w.as_ptr()));
    }
}

pub(crate) fn scroll_edit_to_end(hwnd: HWND) {
    const EM_SETSEL: u32 = 0x00B1;
    const EM_SCROLLCARET: u32 = 0x00B7;
    // SAFETY: Standard Win32 EDIT scroll-to-end messages on an owned control.
    unsafe {
        let _ = SendMessageW(hwnd, EM_SETSEL, Some(WPARAM(usize::MAX)), Some(LPARAM(-1)));
        let _ = SendMessageW(hwnd, EM_SCROLLCARET, Some(WPARAM(0)), Some(LPARAM(0)));
    }
}
