use std::cell::RefCell;
use std::ffi::c_void;

use asrbox_platform_api::{AboutUiCommand, DebugUiCommand, PlatformEvent};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateSolidBrush, DeleteObject, FillRect, SetBkColor, SetTextColor, HBRUSH, HDC, HFONT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetClientRect, KillTimer, MoveWindow,
    SendMessageW, SetForegroundWindow, SetTimer, ShowWindow, BS_PUSHBUTTON, ES_AUTOHSCROLL,
    ES_AUTOVSCROLL, ES_MULTILINE, ES_READONLY, HMENU, SW_SHOW, WINDOW_STYLE, WM_CLOSE, WM_COMMAND,
    WM_CTLCOLOREDIT, WM_CTLCOLORSTATIC, WM_ERASEBKGND, WM_SETFONT, WM_SIZE, WM_TIMER, WS_BORDER,
    WS_CHILD, WS_HSCROLL, WS_OVERLAPPEDWINDOW, WS_VISIBLE, WS_VSCROLL,
};

use crate::app_icon::apply_window_icon;
use crate::shell::encode_wide_str;
use crate::tools_win::set_window_text;
use crate::window::push_event;

pub(crate) const DEBUG_TICK_TIMER_ID: usize = 201;

// NotePlan dark theme BGR COLORREFs: canvas #1C1D22, card/edit #23252C, text #EAECEF
const NP_CANVAS_COLOR: COLORREF = COLORREF(0x0022_1D1C);
const NP_CARD_COLOR: COLORREF = COLORREF(0x002C_2523);
const NP_TEXT_COLOR: COLORREF = COLORREF(0x00EF_ECEA);

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
    static TOOL_BRUSHES: RefCell<[Option<HBRUSH>; 2]> = const { RefCell::new([None, None]) };
}

fn ensure_brushes() -> (HBRUSH, HBRUSH) {
    TOOL_BRUSHES.with(|cell| {
        let mut b = cell.borrow_mut();
        // SAFETY: Creating solid GDI brushes freed once in cleanup_tool_brushes.
        unsafe {
            let c = *b[0].get_or_insert_with(|| CreateSolidBrush(NP_CANVAS_COLOR));
            let k = *b[1].get_or_insert_with(|| CreateSolidBrush(NP_CARD_COLOR));
            (c, k)
        }
    })
}

pub(crate) fn cleanup_tool_brushes() {
    TOOL_BRUSHES.with(|cell| {
        let mut b = cell.borrow_mut();
        // SAFETY: Freeing cached GDI brushes once during teardown.
        unsafe {
            for slot in &mut *b {
                if let Some(h) = slot.take() {
                    let _ = DeleteObject(h.into());
                }
            }
        }
    });
}

unsafe fn paint_dark_bg(hwnd: HWND, wparam: WPARAM) -> LRESULT {
    let hdc = HDC(wparam.0 as *mut c_void);
    let mut rc = RECT::default();
    if GetClientRect(hwnd, &mut rc).is_ok() {
        let _ = FillRect(hdc, &rc, ensure_brushes().0);
    }
    LRESULT(1)
}

unsafe fn style_dark_edit(wparam: WPARAM) -> LRESULT {
    let hdc = HDC(wparam.0 as *mut c_void);
    SetTextColor(hdc, NP_TEXT_COLOR);
    SetBkColor(hdc, NP_CARD_COLOR);
    LRESULT(ensure_brushes().1 .0 as isize)
}

pub(crate) unsafe extern "system" fn debug_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_ERASEBKGND => paint_dark_bg(hwnd, wparam),
        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => style_dark_edit(wparam),
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
        WM_ERASEBKGND => paint_dark_bg(hwnd, wparam),
        WM_CTLCOLOREDIT | WM_CTLCOLORSTATIC => style_dark_edit(wparam),
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

unsafe fn create_tool_window(
    hinst: HINSTANCE,
    class: PCWSTR,
    title: &str,
    r: (i32, i32, i32, i32),
) -> Result<HWND, String> {
    let w_title = encode_wide_str(title);
    let hwnd = CreateWindowExW(
        Default::default(),
        class,
        PCWSTR(w_title.as_ptr()),
        WS_OVERLAPPEDWINDOW,
        r.0,
        r.1,
        r.2,
        r.3,
        None,
        None,
        Some(hinst),
        None,
    )
    .map_err(|e| format!("CreateWindowExW failed: {e}"))?;
    apply_window_icon(hwnd);
    Ok(hwnd)
}

pub(crate) fn create_debug_window(
    hinst: HINSTANCE,
    title: &str,
    ui_font: HFONT,
    mono_font: HFONT,
) -> Result<(), String> {
    // SAFETY: Creating standard Win32 diagnostics window and child controls on UI thread.
    unsafe {
        let hwnd = create_tool_window(hinst, w!("AsrboxDebugWindow"), title, (120, 80, 880, 600))?;
        let b1 = make_btn(
            hwnd,
            hinst,
            (ID_BTN_VERBOSE, 8, 118),
            "TRACE: ВЫКЛ",
            ui_font,
        );
        let b2 = make_btn(
            hwnd,
            hinst,
            (ID_BTN_FILELOG, 132, 112),
            "Файл: ВЫКЛ",
            ui_font,
        );
        let b3 = make_btn(
            hwnd,
            hinst,
            (ID_BTN_FILTER, 250, 130),
            "Модуль: Все",
            ui_font,
        );
        for (spec, label) in [
            ((ID_BTN_COPY_REPORT, 386, 140), "Копировать отчёт"),
            ((ID_BTN_SAVE_REPORT, 532, 136), "Сохранить отчёт"),
            ((ID_BTN_CLEAR, 674, 92), "Очистить"),
        ] {
            let _ = make_btn(hwnd, hinst, spec, label, ui_font);
        }
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
    // SAFETY: Creating standard Win32 About window and child controls on UI thread.
    unsafe {
        let hwnd = create_tool_window(hinst, w!("AsrboxAboutWindow"), title, (160, 110, 740, 560))?;
        for (spec, label) in [
            ((ID_BTN_OPEN_DATA, 8, 175), "Открыть папку данных"),
            ((ID_BTN_COPY_PATHS, 190, 145), "Копировать пути"),
        ] {
            let _ = make_btn(hwnd, hinst, spec, label, ui_font);
        }
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

unsafe fn set_ctrl_font(hwnd: HWND, font: HFONT) {
    let _ = SendMessageW(
        hwnd,
        WM_SETFONT,
        Some(WPARAM(font.0 as usize)),
        Some(LPARAM(1)),
    );
}

unsafe fn make_btn(
    parent: HWND,
    hinst: HINSTANCE,
    (id, x, w_px): (usize, i32, i32),
    label: &str,
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
    set_ctrl_font(hwnd, font);
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
    set_ctrl_font(hwnd, font);
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
