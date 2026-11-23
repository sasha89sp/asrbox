use std::ffi::{c_void, OsString};
use std::mem;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;

use asrbox_platform_api::{PixelPoint, PlatformEvent};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::UI::Input::KeyboardAndMouse::{TrackMouseEvent, TME_LEAVE, TRACKMOUSEEVENT};
use windows::Win32::UI::Shell::{DragFinish, DragQueryFileW, DragQueryPoint, HDROP};
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, GetWindowLongPtrW, GWLP_USERDATA, WM_DROPFILES, WM_LBUTTONUP, WM_MOUSEMOVE,
    WM_RBUTTONUP, WM_TIMER,
};

use crate::window::{
    lparam_point, push_event, ANIMATION_TICK_MS, ANIMATION_TIMER_ID, WM_MOUSELEAVE,
};

pub(crate) unsafe extern "system" fn dock_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let dock_id = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as u64;
    match msg {
        WM_MOUSEMOVE => on_dock_mouse_move(hwnd, dock_id, lparam),
        WM_MOUSELEAVE => {
            push_event(PlatformEvent::PointerLeave { dock_id });
            LRESULT(0)
        }
        WM_LBUTTONUP => {
            push_event(PlatformEvent::LeftClick {
                dock_id,
                point: lparam_point(lparam),
            });
            LRESULT(0)
        }
        WM_RBUTTONUP => on_dock_rbutton_up(hwnd, dock_id, lparam),
        WM_TIMER if wparam.0 == ANIMATION_TIMER_ID => {
            push_event(PlatformEvent::AnimationTick {
                dock_id,
                dt_ms: u16::try_from(ANIMATION_TICK_MS).unwrap_or(16),
            });
            LRESULT(0)
        }
        WM_DROPFILES => {
            handle_drop_files(dock_id, HDROP(wparam.0 as *mut c_void));
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn on_dock_mouse_move(hwnd: HWND, dock_id: u64, lparam: LPARAM) -> LRESULT {
    let mut tme = TRACKMOUSEEVENT {
        cbSize: u32::try_from(mem::size_of::<TRACKMOUSEEVENT>()).unwrap_or(0),
        dwFlags: TME_LEAVE,
        hwndTrack: hwnd,
        dwHoverTime: 0,
    };
    let _ = TrackMouseEvent(&mut tme);
    push_event(PlatformEvent::PointerMove {
        dock_id,
        point: lparam_point(lparam),
    });
    LRESULT(0)
}

unsafe fn on_dock_rbutton_up(hwnd: HWND, dock_id: u64, lparam: LPARAM) -> LRESULT {
    let point = lparam_point(lparam);
    let mut screen = POINT {
        x: point.x,
        y: point.y,
    };
    let _ = ClientToScreen(hwnd, &mut screen);
    push_event(PlatformEvent::RightClick {
        dock_id,
        point,
        screen_point: PixelPoint {
            x: screen.x,
            y: screen.y,
        },
    });
    LRESULT(0)
}

unsafe fn handle_drop_files(dock_id: u64, hdrop: HDROP) {
    let mut pt = POINT::default();
    let _ = DragQueryPoint(hdrop, &mut pt);
    let count = DragQueryFileW(hdrop, u32::MAX, None);
    let mut paths = Vec::with_capacity(count as usize);

    for idx in 0..count {
        let len = DragQueryFileW(hdrop, idx, None);
        if len == 0 {
            continue;
        }
        let mut buf = vec![0u16; (len as usize) + 1];
        let copied = DragQueryFileW(hdrop, idx, Some(&mut buf));
        if copied > 0 {
            let text = OsString::from_wide(&buf[..copied as usize]);
            paths.push(PathBuf::from(text));
        }
    }
    DragFinish(hdrop);

    if !paths.is_empty() {
        push_event(PlatformEvent::DropFiles {
            dock_id,
            paths,
            point: PixelPoint { x: pt.x, y: pt.y },
        });
    }
}
