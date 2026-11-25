use std::ffi::c_void;

use asrbox_platform_api::{NotesUiEvent, PlatformEvent, WidgetLayer};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, ClientToScreen, EndPaint, ScreenToClient, SetBkColor, SetDIBitsToDevice,
    SetTextColor, DIB_RGB_COLORS, HBRUSH, HDC, PAINTSTRUCT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, ReleaseCapture, SetCapture, VK_CONTROL, VK_ESCAPE, VK_RETURN,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallWindowProcW, DefWindowProcW, GetWindowRect, KillTimer, SetTimer, HTBOTTOM, HTBOTTOMLEFT,
    HTBOTTOMRIGHT, HTCAPTION, HTCLIENT, HTLEFT, HTRIGHT, HTTOP, HTTOPLEFT, HTTOPRIGHT, HWND_BOTTOM,
    MINMAXINFO, WINDOWPOS, WM_CHAR, WM_CLOSE, WM_COMMAND, WM_CTLCOLOREDIT, WM_ERASEBKGND,
    WM_EXITSIZEMOVE, WM_GETMINMAXINFO, WM_KEYDOWN, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_MOUSEWHEEL, WM_MOVE, WM_NCHITTEST, WM_PAINT, WM_RBUTTONUP, WM_SIZE, WM_TIMER,
    WM_WINDOWPOSCHANGING, WNDPROC,
};

use crate::icons::make_top_down_bmi;
use crate::notes_win::{
    close_notes_win, read_notes_input, NOTES_EDIT_ID_BASE, NOTES_EDIT_SLOTS, NOTES_UI,
};
use crate::window::{lparam_point, push_event};

const NOTES_MIN_W: i32 = 300;
const NOTES_MIN_H: i32 = 320;
const RESIZE_EDGE_PX: i32 = 6;
const EN_CHANGE: u32 = 0x0300;
pub(crate) const NOTES_SAVE_TIMER_ID: usize = 73;
const NOTES_SAVE_DEBOUNCE_MS: u32 = 450;

pub(crate) unsafe extern "system" fn notes_wnd_proc(
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
                (*mmi).ptMinTrackSize.x = NOTES_MIN_W;
                (*mmi).ptMinTrackSize.y = NOTES_MIN_H;
            }
            LRESULT(0)
        }
        WM_WINDOWPOSCHANGING => {
            let is_wallpaper = NOTES_UI.with(|ui| {
                ui.try_borrow()
                    .is_ok_and(|st| st.layer == WidgetLayer::Wallpaper)
            });
            if is_wallpaper && lparam.0 != 0 {
                let wp = lparam.0 as *mut WINDOWPOS;
                (*wp).hwndInsertAfter = HWND_BOTTOM;
            }
            LRESULT(0)
        }
        WM_NCHITTEST => hit_test_notes(hwnd, lparam),
        WM_SIZE => {
            emit_notes_rect(hwnd, false);
            LRESULT(0)
        }
        WM_EXITSIZEMOVE => {
            emit_notes_rect(hwnd, true);
            LRESULT(0)
        }
        WM_MOVE => {
            let mut rc = RECT::default();
            if GetWindowRect(hwnd, &mut rc).is_ok() {
                push_event(PlatformEvent::NotesEvent(NotesUiEvent::Moved {
                    x: rc.left,
                    y: rc.top,
                }));
            }
            LRESULT(0)
        }
        WM_KEYDOWN => {
            if GetKeyState(i32::from(VK_CONTROL.0)) < 0 {
                match wparam.0 {
                    0x31 => push_event(PlatformEvent::NotesEvent(NotesUiEvent::TogglePane(0))),
                    0x32 => push_event(PlatformEvent::NotesEvent(NotesUiEvent::TogglePane(1))),
                    0x33 => push_event(PlatformEvent::NotesEvent(NotesUiEvent::TogglePane(2))),
                    _ => {}
                }
            }
            LRESULT(0)
        }
        WM_COMMAND => handle_notes_command(hwnd, wparam),
        WM_TIMER if wparam.0 == NOTES_SAVE_TIMER_ID => {
            let _ = KillTimer(Some(hwnd), NOTES_SAVE_TIMER_ID);
            push_event(PlatformEvent::NotesEvent(NotesUiEvent::FlushSaveTick));
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let pt = lparam_point(lparam);
            push_event(PlatformEvent::NotesEvent(NotesUiEvent::PointerMove {
                x: pt.x,
                y: pt.y,
            }));
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let _ = SetCapture(hwnd);
            let pt = lparam_point(lparam);
            push_event(PlatformEvent::NotesEvent(NotesUiEvent::MouseDown {
                x: pt.x,
                y: pt.y,
            }));
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            let _ = ReleaseCapture();
            let pt = lparam_point(lparam);
            push_event(PlatformEvent::NotesEvent(NotesUiEvent::MouseUp {
                x: pt.x,
                y: pt.y,
            }));
            LRESULT(0)
        }
        WM_RBUTTONUP => {
            let pt = lparam_point(lparam);
            let mut scr = POINT { x: pt.x, y: pt.y };
            let _ = ClientToScreen(hwnd, &mut scr);
            push_event(PlatformEvent::NotesEvent(NotesUiEvent::RightClick {
                x: pt.x,
                y: pt.y,
                screen_x: scr.x,
                screen_y: scr.y,
            }));
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            let delta = ((wparam.0 >> 16) & 0xFFFF) as i16;
            let scr = lparam_point(lparam);
            let mut cli = POINT { x: scr.x, y: scr.y };
            let _ = ScreenToClient(hwnd, &mut cli);
            push_event(PlatformEvent::NotesEvent(NotesUiEvent::Wheel {
                x: cli.x,
                y: cli.y,
                delta,
            }));
            LRESULT(0)
        }
        WM_CTLCOLOREDIT => handle_ctlcolor(wparam),
        WM_PAINT => paint_notes(hwnd),
        WM_CLOSE => {
            close_notes_win();
            push_event(PlatformEvent::NotesEvent(NotesUiEvent::Closed));
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub(crate) unsafe extern "system" fn inline_edit_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_KEYDOWN if wparam.0 == usize::from(VK_RETURN.0) => {
            push_event(PlatformEvent::NotesEvent(NotesUiEvent::InlineSubmit));
            return LRESULT(0);
        }
        WM_KEYDOWN if wparam.0 == usize::from(VK_ESCAPE.0) => {
            push_event(PlatformEvent::NotesEvent(NotesUiEvent::InlineCancel));
            return LRESULT(0);
        }
        WM_CHAR if wparam.0 == 0x0D || wparam.0 == 0x1B => {
            return LRESULT(0);
        }
        _ => {}
    }
    let orig = NOTES_UI.with(|ui| ui.try_borrow().map_or(0, |st| st.inline_orig_proc));
    if orig != 0 {
        let proc_fn: WNDPROC = std::mem::transmute::<isize, WNDPROC>(orig);
        return CallWindowProcW(proc_fn, hwnd, msg, wparam, lparam);
    }
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

unsafe fn hit_test_notes(hwnd: HWND, lparam: LPARAM) -> LRESULT {
    let scr = lparam_point(lparam);
    let mut rc = RECT::default();
    if GetWindowRect(hwnd, &mut rc).is_err() {
        return LRESULT(HTCLIENT as isize);
    }
    let cx = scr.x - rc.left;
    let cy = scr.y - rc.top;
    let w = rc.right - rc.left;
    let h = rc.bottom - rc.top;
    let left = cx < RESIZE_EDGE_PX;
    let right = cx >= w - RESIZE_EDGE_PX;
    let top = cy < RESIZE_EDGE_PX;
    let bottom = cy >= h - RESIZE_EDGE_PX;
    let hit = match (left, right, top, bottom) {
        (true, _, true, _) => HTTOPLEFT,
        (_, true, true, _) => HTTOPRIGHT,
        (true, _, _, true) => HTBOTTOMLEFT,
        (_, true, _, true) => HTBOTTOMRIGHT,
        (true, _, _, _) => HTLEFT,
        (_, true, _, _) => HTRIGHT,
        (_, _, true, _) => HTTOP,
        (_, _, _, true) => HTBOTTOM,
        _ if cy < 36 && cx < w - 180 => HTCAPTION,
        _ => HTCLIENT,
    };
    LRESULT(hit as isize)
}

unsafe fn emit_notes_rect(hwnd: HWND, finished: bool) {
    let mut rc = RECT::default();
    if GetWindowRect(hwnd, &mut rc).is_ok() {
        let w = u32::try_from((rc.right - rc.left).max(NOTES_MIN_W)).unwrap_or(780);
        let h = u32::try_from((rc.bottom - rc.top).max(NOTES_MIN_H)).unwrap_or(520);
        push_event(PlatformEvent::NotesEvent(NotesUiEvent::Resized {
            x: rc.left,
            y: rc.top,
            width: w,
            height: h,
            finished,
        }));
    }
}

unsafe fn handle_notes_command(hwnd: HWND, wparam: WPARAM) -> LRESULT {
    let notify = ((wparam.0 >> 16) & 0xFFFF) as u32;
    let ctrl_id = wparam.0 & 0xFFFF;
    if notify == EN_CHANGE
        && (NOTES_EDIT_ID_BASE..NOTES_EDIT_ID_BASE + NOTES_EDIT_SLOTS).contains(&ctrl_id)
    {
        let suppressed = NOTES_UI.with(|ui| ui.try_borrow().map_or(true, |st| st.suppress_change));
        if !suppressed {
            let slot = (ctrl_id - NOTES_EDIT_ID_BASE) as u8;
            let text = read_notes_input(slot);
            if slot == 1 || slot == 2 {
                let _ = SetTimer(
                    Some(hwnd),
                    NOTES_SAVE_TIMER_ID,
                    NOTES_SAVE_DEBOUNCE_MS,
                    None,
                );
            }
            push_event(PlatformEvent::NotesEvent(NotesUiEvent::InputChanged {
                slot,
                text,
            }));
        }
    }
    LRESULT(0)
}

unsafe fn handle_ctlcolor(wparam: WPARAM) -> LRESULT {
    let hdc = HDC(wparam.0 as *mut c_void);
    let (fg, bg, br) = NOTES_UI.with(|ui| {
        ui.try_borrow().map_or(
            (
                COLORREF(0x00FF_FFFF),
                COLORREF(0x0024_2424),
                HBRUSH::default(),
            ),
            |st| (st.edit_fg, st.edit_bg, st.edit_brush.unwrap_or_default()),
        )
    });
    SetTextColor(hdc, fg);
    SetBkColor(hdc, bg);
    LRESULT(br.0 as isize)
}

unsafe fn paint_notes(hwnd: HWND) -> LRESULT {
    let mut ps = PAINTSTRUCT::default();
    let hdc = BeginPaint(hwnd, &mut ps);
    NOTES_UI.with(|ui| {
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
