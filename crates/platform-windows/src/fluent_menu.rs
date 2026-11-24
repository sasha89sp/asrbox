use std::cell::RefCell;
use std::ffi::c_void;

use asrbox_platform_api::{
    BgraFrame, FluentMenuItem, FluentMenuSpec, FluentPalette, PixelPoint, PixelRect,
};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::VK_ESCAPE;
use windows::Win32::UI::WindowsAndMessaging::{
    DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, KillTimer, SetForegroundWindow,
    SetTimer, ShowWindow, TranslateMessage, MSG, SW_SHOW, WA_INACTIVE, WM_ACTIVATE, WM_KEYDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_RBUTTONUP, WM_TIMER,
};

use crate::fluent_menu_render::{clamp_menu_origin, measure_menu, render_fluent_menu, MenuLayout};
use crate::fluent_menu_win::{
    create_menu_hwnd, hit_quick, hit_row, monitor_work_area_at, present_with_alpha, repaint_window,
    sync_submenu_for_row,
};
use crate::window::{lparam_point, WM_MOUSELEAVE};

const FADE_TIMER_ID: usize = 51;
const FADE_TICK_MS: u32 = 16;

pub(crate) struct MenuWindow {
    pub hwnd: HWND,
    pub spec: FluentMenuSpec,
    pub layout: MenuLayout,
    pub origin: PixelPoint,
    pub hovered_row: Option<usize>,
    pub hovered_quick: Option<usize>,
    pub alpha: u8,
    pub frame: Option<BgraFrame>,
}

pub(crate) struct ActiveMenuSession {
    pub hinstance: HINSTANCE,
    pub work_area: PixelRect,
    pub palette: FluentPalette,
    pub root: MenuWindow,
    pub sub: Option<MenuWindow>,
    pub open_sub_row: Option<usize>,
    pub result: Option<u32>,
    pub done: bool,
}

thread_local! {
    static MENU_SESSION: RefCell<Option<ActiveMenuSession>> = const { RefCell::new(None) };
}

pub(crate) fn show_fluent_menu(
    hinstance: HINSTANCE,
    _owner_hwnd: HWND,
    screen_pt: PixelPoint,
    spec: &FluentMenuSpec,
    palette: FluentPalette,
) -> Result<Option<u32>, String> {
    let work_area = monitor_work_area_at(screen_pt);
    let layout = measure_menu(spec);
    let origin = clamp_menu_origin(screen_pt, &layout, work_area);
    let hwnd = create_menu_hwnd(hinstance, origin, &layout, false)?;
    let frame = render_fluent_menu(spec, &layout, &palette, None, None, None)?;
    present_with_alpha(hwnd, origin, &frame, 48)?;

    let root = MenuWindow {
        hwnd,
        spec: spec.clone(),
        layout,
        origin,
        hovered_row: None,
        hovered_quick: None,
        alpha: 48,
        frame: Some(frame),
    };
    MENU_SESSION.with(|cell| {
        *cell.borrow_mut() = Some(ActiveMenuSession {
            hinstance,
            work_area,
            palette,
            root,
            sub: None,
            open_sub_row: None,
            result: None,
            done: false,
        });
    });

    // SAFETY: Showing our layered popup menu, activating it for outside-click dismissal, and starting fade timer.
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
        let _ = SetTimer(Some(hwnd), FADE_TIMER_ID, FADE_TICK_MS, None);
    }

    let mut msg = MSG::default();
    loop {
        let finished = MENU_SESSION.with(|c| c.borrow().as_ref().is_none_or(|s| s.done));
        if finished {
            break;
        }
        // SAFETY: Standard modal Win32 message dispatch while the popup menu is open.
        let status = unsafe { GetMessageW(&mut msg, None, 0, 0).0 };
        if status <= 0 {
            break;
        }
        // SAFETY: Translating and dispatching valid MSG.
        unsafe {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    let chosen = MENU_SESSION.with(|cell| {
        let mut guard = cell.borrow_mut();
        if let Some(session) = guard.take() {
            // SAFETY: Destroying owned popup menu HWNDs cleanly upon dismissal.
            unsafe {
                let _ = KillTimer(Some(session.root.hwnd), FADE_TIMER_ID);
                if let Some(sub) = session.sub {
                    let _ = DestroyWindow(sub.hwnd);
                }
                let _ = DestroyWindow(session.root.hwnd);
            }
            session.result
        } else {
            None
        }
    });
    Ok(chosen)
}

pub(crate) unsafe extern "system" fn fluent_menu_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TIMER if wparam.0 == FADE_TIMER_ID => {
            advance_fade_animation(hwnd);
            LRESULT(0)
        }
        WM_MOUSEMOVE => {
            let pt = lparam_point(lparam);
            handle_mouse_move(hwnd, pt);
            LRESULT(0)
        }
        WM_MOUSELEAVE => {
            handle_mouse_leave(hwnd);
            LRESULT(0)
        }
        WM_LBUTTONUP | WM_RBUTTONUP => {
            let pt = lparam_point(lparam);
            handle_mouse_click(hwnd, pt);
            LRESULT(0)
        }
        WM_MOUSEWHEEL => {
            close_session(None);
            LRESULT(0)
        }
        WM_KEYDOWN if wparam.0 as u16 == VK_ESCAPE.0 => {
            close_session(None);
            LRESULT(0)
        }
        WM_ACTIVATE => {
            let state = (wparam.0 & 0xFFFF) as u32;
            if state == WA_INACTIVE {
                let other = HWND(lparam.0 as *mut c_void);
                let is_child = MENU_SESSION.with(|c| {
                    c.borrow()
                        .as_ref()
                        .is_some_and(|s| s.sub.as_ref().is_some_and(|sub| sub.hwnd == other))
                });
                if !is_child {
                    close_session(None);
                }
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn close_session(chosen: Option<u32>) {
    MENU_SESSION.with(|c| {
        if let Some(s) = c.borrow_mut().as_mut() {
            s.result = chosen;
            s.done = true;
        }
    });
}

fn advance_fade_animation(hwnd: HWND) {
    MENU_SESSION.with(|c| {
        let mut guard = c.borrow_mut();
        let Some(s) = guard.as_mut() else {
            return;
        };
        let next = s.root.alpha.saturating_add(44);
        s.root.alpha = next;
        if let Some(ref frame) = s.root.frame {
            let _ = present_with_alpha(s.root.hwnd, s.root.origin, frame, next);
        }
        if next == 255 {
            // SAFETY: Killing fade-in timer once fully opaque so idle CPU is 0%.
            unsafe {
                let _ = KillTimer(Some(hwnd), FADE_TIMER_ID);
            }
        }
    });
}

fn handle_mouse_move(hwnd: HWND, pt: PixelPoint) {
    MENU_SESSION.with(|c| {
        let mut guard = c.borrow_mut();
        let Some(s) = guard.as_mut() else {
            return;
        };
        if s.root.hwnd == hwnd {
            let new_q = hit_quick(&s.root.layout, pt);
            let new_r = hit_row(&s.root.layout, pt);
            let changed = s.root.hovered_quick != new_q || s.root.hovered_row != new_r;
            s.root.hovered_quick = new_q;
            s.root.hovered_row = new_r;
            if changed {
                sync_submenu_for_row(s, new_r);
                repaint_window(&mut s.root, &s.palette, s.open_sub_row);
            }
        } else if let Some(ref mut sub) = s.sub {
            if sub.hwnd == hwnd {
                let new_r = hit_row(&sub.layout, pt);
                if sub.hovered_row != new_r {
                    sub.hovered_row = new_r;
                    repaint_window(sub, &s.palette, None);
                }
            }
        }
    });
}

fn handle_mouse_leave(hwnd: HWND) {
    MENU_SESSION.with(|c| {
        let mut guard = c.borrow_mut();
        let Some(s) = guard.as_mut() else {
            return;
        };
        if s.root.hwnd == hwnd && (s.root.hovered_row.is_some() || s.root.hovered_quick.is_some()) {
            s.root.hovered_row = None;
            s.root.hovered_quick = None;
            repaint_window(&mut s.root, &s.palette, s.open_sub_row);
        } else if let Some(ref mut sub) = s.sub {
            if sub.hwnd == hwnd && sub.hovered_row.is_some() {
                sub.hovered_row = None;
                repaint_window(sub, &s.palette, None);
            }
        }
    });
}

fn handle_mouse_click(hwnd: HWND, pt: PixelPoint) {
    let mut chosen: Option<u32> = None;
    let mut dismiss = false;
    MENU_SESSION.with(|c| {
        let guard = c.borrow();
        let Some(s) = guard.as_ref() else {
            return;
        };
        if s.root.hwnd == hwnd {
            if let Some(q_idx) = hit_quick(&s.root.layout, pt) {
                if let Some(qb) = s.root.layout.quick_btns.get(q_idx) {
                    if qb.enabled {
                        chosen = Some(qb.id);
                        dismiss = true;
                    }
                }
                return;
            }
            if let Some(r_idx) = hit_row(&s.root.layout, pt) {
                if let Some(FluentMenuItem::Action { id, enabled, .. }) =
                    s.root.spec.items.get(r_idx)
                {
                    if *enabled {
                        chosen = Some(*id);
                        dismiss = true;
                    }
                }
            } else {
                dismiss = true;
            }
        } else if let Some(ref sub) = s.sub {
            if sub.hwnd == hwnd {
                if let Some(r_idx) = hit_row(&sub.layout, pt) {
                    if let Some(FluentMenuItem::Action { id, enabled, .. }) =
                        sub.spec.items.get(r_idx)
                    {
                        if *enabled {
                            chosen = Some(*id);
                            dismiss = true;
                        }
                    }
                }
            }
        }
    });
    if dismiss {
        close_session(chosen);
    }
}
