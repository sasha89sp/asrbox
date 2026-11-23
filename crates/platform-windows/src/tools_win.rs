use std::path::Path;

use asrbox_platform_api::{ProcessMetrics, ToolsPlatform};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{GlobalFree, HANDLE, HWND};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, DeleteObject, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
    DEFAULT_PITCH, FF_DONTCARE, FW_NORMAL, HFONT, OUT_DEFAULT_PRECIS,
};
use windows::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::UI::WindowsAndMessaging::{DestroyWindow, KillTimer};

use crate::shell::{encode_wide_str, open_in_explorer, open_in_notepad};
use crate::tools_proc::{
    create_about_window, create_debug_window, focus_window, scroll_edit_to_end, set_window_text,
    AboutControls, DebugControls, ABOUT_UI, DEBUG_TICK_TIMER_ID, DEBUG_UI,
};
use crate::window::WindowsDockHost;

const CF_UNICODETEXT: u32 = 13;

pub(crate) struct ToolsWinState {
    ui_font: Option<HFONT>,
    mono_font: Option<HFONT>,
    last_summary: String,
    last_logs: String,
    last_verbose: Option<bool>,
    last_file: Option<bool>,
    last_filter: String,
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
        DEBUG_UI.with(|ui| {
            let mut st = ui.borrow_mut();
            if let Some(hwnd) = st.hwnd.take() {
                // SAFETY: Destroying owned debug window on UI teardown.
                unsafe {
                    let _ = KillTimer(Some(hwnd), DEBUG_TICK_TIMER_ID);
                    let _ = DestroyWindow(hwnd);
                }
            }
            *st = DebugControls::default();
        });
        ABOUT_UI.with(|ui| {
            let mut st = ui.borrow_mut();
            if let Some(hwnd) = st.hwnd.take() {
                // SAFETY: Destroying owned about window on UI teardown.
                unsafe {
                    let _ = DestroyWindow(hwnd);
                }
            }
            *st = AboutControls::default();
        });
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

    fn ensure_fonts(&mut self) -> (HFONT, HFONT) {
        if self.ui_font.is_none() {
            self.ui_font = Some(create_font("Segoe UI", 15));
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

impl ToolsPlatform for WindowsDockHost {
    fn open_debug_window(&mut self, title: &str) -> Result<(), String> {
        let existing = DEBUG_UI.with(|ui| ui.borrow().hwnd);
        if let Some(hwnd) = existing {
            focus_window(hwnd);
            return Ok(());
        }
        let (ui_font, mono_font) = self.tools.ensure_fonts();
        self.tools.last_summary.clear();
        self.tools.last_logs.clear();
        self.tools.last_verbose = None;
        self.tools.last_file = None;
        self.tools.last_filter.clear();
        create_debug_window(self.hinstance, title, ui_font, mono_font)
    }

    fn update_debug_window(
        &mut self,
        summary: &str,
        logs: &str,
        verbose: bool,
        file_log: bool,
        filter_label: &str,
    ) {
        DEBUG_UI.with(|ui| {
            let st = ui.borrow();
            if st.hwnd.is_none() {
                return;
            }
            if self.tools.last_verbose != Some(verbose) {
                self.tools.last_verbose = Some(verbose);
                let txt = if verbose {
                    "TRACE: ВКЛ"
                } else {
                    "TRACE: ВЫКЛ"
                };
                if let Some(h) = st.btn_verbose {
                    set_window_text(h, txt);
                }
            }
            if self.tools.last_file != Some(file_log) {
                self.tools.last_file = Some(file_log);
                let txt = if file_log {
                    "Файл: ВКЛ"
                } else {
                    "Файл: ВЫКЛ"
                };
                if let Some(h) = st.btn_file {
                    set_window_text(h, txt);
                }
            }
            if self.tools.last_filter != filter_label {
                self.tools.last_filter = filter_label.to_owned();
                if let Some(h) = st.btn_filter {
                    set_window_text(h, &format!("Модуль: {filter_label}"));
                }
            }
            if self.tools.last_summary != summary {
                self.tools.last_summary = summary.to_owned();
                if let Some(h) = st.summary_edit {
                    set_window_text(h, summary);
                }
            }
            if self.tools.last_logs != logs {
                self.tools.last_logs = logs.to_owned();
                if let Some(h) = st.logs_edit {
                    set_window_text(h, logs);
                    scroll_edit_to_end(h);
                }
            }
        });
    }

    fn is_debug_window_open(&self) -> bool {
        DEBUG_UI.with(|ui| ui.borrow().hwnd.is_some())
    }

    fn open_about_window(&mut self, title: &str, content: &str) -> Result<(), String> {
        let existing = ABOUT_UI.with(|ui| ui.borrow().hwnd);
        if let Some(hwnd) = existing {
            ABOUT_UI.with(|ui| {
                if let Some(edit) = ui.borrow().text_edit {
                    set_window_text(edit, content);
                }
            });
            focus_window(hwnd);
            return Ok(());
        }
        let (ui_font, mono_font) = self.tools.ensure_fonts();
        create_about_window(self.hinstance, title, content, ui_font, mono_font)
    }

    fn query_process_metrics(&mut self) -> ProcessMetrics {
        self.metrics.sample()
    }

    fn copy_to_clipboard(&self, text: &str) -> Result<(), String> {
        copy_text_to_clipboard(self.host_hwnd, text)
    }

    fn open_folder(&self, path: &Path) -> Result<(), String> {
        open_in_explorer(path)
    }

    fn open_text_file(&self, path: &Path) -> Result<(), String> {
        open_in_notepad(path)
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

fn copy_text_to_clipboard(owner: HWND, text: &str) -> Result<(), String> {
    let wide = encode_wide_str(text);
    let byte_len = wide.len().saturating_mul(2);
    // SAFETY: Standard Win32 clipboard sequence; global memory transfers ownership on success.
    unsafe {
        OpenClipboard(Some(owner)).map_err(|e| format!("OpenClipboard failed: {e}"))?;
        let _ = EmptyClipboard();
        let Ok(hmem) = GlobalAlloc(GMEM_MOVEABLE, byte_len) else {
            let _ = CloseClipboard();
            return Err("GlobalAlloc failed".to_owned());
        };
        let ptr = GlobalLock(hmem);
        if ptr.is_null() {
            let _ = GlobalFree(Some(hmem));
            let _ = CloseClipboard();
            return Err("GlobalLock failed".to_owned());
        }
        std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr.cast::<u16>(), wide.len());
        let _ = GlobalUnlock(hmem);
        let set = SetClipboardData(CF_UNICODETEXT, Some(HANDLE(hmem.0)));
        let _ = CloseClipboard();
        if set.is_err() {
            let _ = GlobalFree(Some(hmem));
            return Err("SetClipboardData failed".to_owned());
        }
    }
    Ok(())
}
