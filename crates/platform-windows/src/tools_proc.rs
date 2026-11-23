use std::cell::RefCell;
use std::ffi::c_void;

use asrbox_platform_api::{AboutUiCommand, DebugUiCommand, PlatformEvent};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::HFONT;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, KillTimer, MoveWindow,
    SendMessageW, SetForegroundWindow, SetTimer, SetWindowTextW, ShowWindow, BS_PUSHBUTTON,
    ES_AUTOHSCROLL, ES_AUTOVSCROLL, ES_MULTILINE, ES_READONLY, HMENU, SW_RESTORE, SW_SHOW,
    WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_SETFONT, WM_SIZE, WM_TIMER, WS_BORDER, WS_CHILD,
    WS_HSCROLL, WS_OVERLAPPEDWINDOW, WS_VISIBLE, WS_VSCROLL,
};

use crate::shell::encode_wide_str;
use crate::window::push_event;

pub(crate) const DEBUG_TICK_TIMER_ID: usize = 201;
const EM_SETSEL: u32 = 0x00B1;
const EM_SCROLLCARET: u32 = 0x00B7;

const ID_BTN_VERBOSE: usize = 501;
const ID_BTN_FILELOG: usize = 502;
const ID_BTN_FILTER: usize = 503;
const ID_BTN_COPY_REPORT: usize = 504;
const ID_BTN_SAVE_REPORT: usize = 505;
const ID_BTN_CLEAR: usize = 506;
const ID_BTN_OPEN_DATA: usize = 601;
const ID_BTN_COPY_PATHS: usize = 602;

#[derive(Default)]
pub(crate) struct DebugControls {
    pub(crate) hwnd: Option<HWND>,
    pub(crate) btn_verbose: Option<HWND>,
    pub(crate) btn_file: Option<HWND>,
    pub(crate) btn_filter: Option<HWND>,
    pub(crate) summary_edit: Option<HWND>,
    pub(crate) logs_edit: Option<HWND>,
}

#[derive(Default)]
pub(crate) struct AboutControls {
    pub(crate) hwnd: Option<HWND>,
    pub(crate) text_edit: Option<HWND>,
}

thread_local! {
    pub(crate) static DEBUG_UI: RefCell<DebugControls> = RefCell::new(DebugControls::default());
    pub(crate) static ABOUT_UI: RefCell<AboutControls> = RefCell::new(AboutControls::default());
}

pub(crate) unsafe extern "system" fn debug_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let cmd = match wparam.0 & 0xFFFF {
                ID_BTN_VERBOSE => Some(DebugUiCommand::ToggleVerbose),
                ID_BTN_FILELOG => Some(DebugUiCommand::ToggleFileLog),
                ID_BTN_FILTER => Some(DebugUiCommand::CycleFilter),
                ID_BTN_COPY_REPORT => Some(DebugUiCommand::CopyReport),
                ID_BTN_SAVE_REPORT => Some(DebugUiCommand::SaveReport),
                ID_BTN_CLEAR => Some(DebugUiCommand::ClearLogs),
                _ => None,
            };
            if let Some(c) = cmd {
                push_event(PlatformEvent::DebugCommand(c));
            }
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == DEBUG_TICK_TIMER_ID => {
            push_event(PlatformEvent::DebugWindowTick);
            LRESULT(0)
        }
        WM_SIZE => {
            layout_debug_controls(hwnd);
            LRESULT(0)
        }
        WM_CLOSE => {
            let _ = KillTimer(Some(hwnd), DEBUG_TICK_TIMER_ID);
            DEBUG_UI.with(|ui| *ui.borrow_mut() = DebugControls::default());
            let _ = DestroyWindow(hwnd);
            push_event(PlatformEvent::DebugCommand(DebugUiCommand::WindowClosed));
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub(crate) unsafe extern "system" fn about_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            match wparam.0 & 0xFFFF {
                ID_BTN_OPEN_DATA => {
                    push_event(PlatformEvent::AboutCommand(AboutUiCommand::OpenDataFolder));
                }
                ID_BTN_COPY_PATHS => {
                    push_event(PlatformEvent::AboutCommand(AboutUiCommand::CopyPaths));
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_SIZE => {
            layout_about_controls(hwnd);
            LRESULT(0)
        }
        WM_CLOSE => {
            ABOUT_UI.with(|ui| *ui.borrow_mut() = AboutControls::default());
            let _ = DestroyWindow(hwnd);
            push_event(PlatformEvent::AboutCommand(AboutUiCommand::WindowClosed));
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub(crate) fn create_debug_window(
    hinst: HINSTANCE,
    title: &str,
    ui_font: HFONT,
    mono_font: HFONT,
) -> Result<(), String> {
    let title_w = encode_wide_str(title);
    // SAFETY: Creating standard Win32 diagnostics window and child controls on UI thread.
    unsafe {
        let hwnd = CreateWindowExW(
            Default::default(),
            w!("AsrboxDebugWindow"),
            PCWSTR(title_w.as_ptr()),
            WS_OVERLAPPEDWINDOW,
            120,
            80,
            880,
            600,
            None,
            None,
            Some(hinst),
            None,
        )
        .map_err(|e| format!("CreateWindowExW(debug) failed: {e}"))?;

        let b1 = make_btn(hwnd, hinst, ID_BTN_VERBOSE, "TRACE: ВЫКЛ", 8, 118, ui_font);
        let b2 = make_btn(hwnd, hinst, ID_BTN_FILELOG, "Файл: ВЫКЛ", 132, 112, ui_font);
        let b3 = make_btn(hwnd, hinst, ID_BTN_FILTER, "Модуль: Все", 250, 130, ui_font);
        let _ = make_btn(
            hwnd,
            hinst,
            ID_BTN_COPY_REPORT,
            "Копировать отчёт",
            386,
            140,
            ui_font,
        );
        let _ = make_btn(
            hwnd,
            hinst,
            ID_BTN_SAVE_REPORT,
            "Сохранить отчёт",
            532,
            136,
            ui_font,
        );
        let _ = make_btn(hwnd, hinst, ID_BTN_CLEAR, "Очистить", 674, 92, ui_font);

        let summary_edit = make_edit(hwnd, hinst, false, mono_font);
        let logs_edit = make_edit(hwnd, hinst, true, mono_font);
        DEBUG_UI.with(|ui| {
            *ui.borrow_mut() = DebugControls {
                hwnd: Some(hwnd),
                btn_verbose: b1,
                btn_file: b2,
                btn_filter: b3,
                summary_edit,
                logs_edit,
            };
        });
        layout_debug_controls(hwnd);
        let _ = SetTimer(Some(hwnd), DEBUG_TICK_TIMER_ID, 1000, None);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
    Ok(())
}

pub(crate) fn create_about_window(
    hinst: HINSTANCE,
    title: &str,
    content: &str,
    ui_font: HFONT,
    mono_font: HFONT,
) -> Result<(), String> {
    let title_w = encode_wide_str(title);
    // SAFETY: Creating standard Win32 About window and child controls on UI thread.
    unsafe {
        let hwnd = CreateWindowExW(
            Default::default(),
            w!("AsrboxAboutWindow"),
            PCWSTR(title_w.as_ptr()),
            WS_OVERLAPPEDWINDOW,
            160,
            110,
            740,
            560,
            None,
            None,
            Some(hinst),
            None,
        )
        .map_err(|e| format!("CreateWindowExW(about) failed: {e}"))?;

        let _ = make_btn(
            hwnd,
            hinst,
            ID_BTN_OPEN_DATA,
            "Открыть папку данных",
            8,
            175,
            ui_font,
        );
        let _ = make_btn(
            hwnd,
            hinst,
            ID_BTN_COPY_PATHS,
            "Копировать пути",
            190,
            145,
            ui_font,
        );
        let text_edit = make_edit(hwnd, hinst, false, mono_font);
        if let Some(h) = text_edit {
            set_window_text(h, content);
        }
        ABOUT_UI.with(|ui| {
            *ui.borrow_mut() = AboutControls {
                hwnd: Some(hwnd),
                text_edit,
            };
        });
        layout_about_controls(hwnd);
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
    Ok(())
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
    // SAFETY: Standard Win32 EDIT scroll-to-end messages on an owned control.
    unsafe {
        let _ = SendMessageW(hwnd, EM_SETSEL, Some(WPARAM(usize::MAX)), Some(LPARAM(-1)));
        let _ = SendMessageW(hwnd, EM_SCROLLCARET, Some(WPARAM(0)), Some(LPARAM(0)));
    }
}

unsafe fn make_btn(
    parent: HWND,
    hinst: HINSTANCE,
    id: usize,
    label: &str,
    x: i32,
    w_px: i32,
    font: HFONT,
) -> Option<HWND> {
    let text_w = encode_wide_str(label);
    let style = WS_CHILD | WS_VISIBLE | WINDOW_STYLE(BS_PUSHBUTTON as u32);
    let hwnd = CreateWindowExW(
        Default::default(),
        w!("BUTTON"),
        PCWSTR(text_w.as_ptr()),
        style,
        x,
        8,
        w_px,
        28,
        Some(parent),
        Some(HMENU(id as *mut c_void)),
        Some(hinst),
        None,
    )
    .ok()?;
    let _ = SendMessageW(
        hwnd,
        WM_SETFONT,
        Some(WPARAM(font.0 as usize)),
        Some(LPARAM(1)),
    );
    Some(hwnd)
}

unsafe fn make_edit(parent: HWND, hinst: HINSTANCE, hscroll: bool, font: HFONT) -> Option<HWND> {
    let mut raw = (WS_CHILD | WS_VISIBLE | WS_BORDER | WS_VSCROLL).0
        | (ES_MULTILINE as u32)
        | (ES_READONLY as u32)
        | (ES_AUTOVSCROLL as u32);
    if hscroll {
        raw |= WS_HSCROLL.0 | (ES_AUTOHSCROLL as u32);
    }
    let hwnd = CreateWindowExW(
        Default::default(),
        w!("EDIT"),
        w!(""),
        WINDOW_STYLE(raw),
        8,
        44,
        600,
        200,
        Some(parent),
        None,
        Some(hinst),
        None,
    )
    .ok()?;
    let _ = SendMessageW(
        hwnd,
        WM_SETFONT,
        Some(WPARAM(font.0 as usize)),
        Some(LPARAM(1)),
    );
    Some(hwnd)
}

unsafe fn layout_debug_controls(hwnd: HWND) {
    let mut rc = RECT::default();
    if GetClientRect(hwnd, &mut rc).is_err() {
        return;
    }
    let w = (rc.right - rc.left).max(200);
    let h = (rc.bottom - rc.top).max(260);
    let sum_h = 152.min((h - 60) / 2);
    let log_y = 44 + sum_h + 6;
    let log_h = (h - log_y - 8).max(80);
    DEBUG_UI.with(|ui| {
        let st = ui.borrow();
        if let Some(e) = st.summary_edit {
            let _ = MoveWindow(e, 8, 44, w - 16, sum_h, true);
        }
        if let Some(e) = st.logs_edit {
            let _ = MoveWindow(e, 8, log_y, w - 16, log_h, true);
        }
    });
}

unsafe fn layout_about_controls(hwnd: HWND) {
    let mut rc = RECT::default();
    if GetClientRect(hwnd, &mut rc).is_err() {
        return;
    }
    let w = (rc.right - rc.left).max(200);
    let h = (rc.bottom - rc.top).max(160);
    ABOUT_UI.with(|ui| {
        if let Some(e) = ui.borrow().text_edit {
            let _ = MoveWindow(e, 8, 44, w - 16, (h - 52).max(80), true);
        }
    });
}
