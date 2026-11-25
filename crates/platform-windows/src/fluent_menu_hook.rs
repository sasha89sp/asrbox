use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use asrbox_platform_api::PixelRect;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetForegroundWindow, GetWindowThreadProcessId, PostMessageW,
    SetForegroundWindow, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, MSLLHOOKSTRUCT,
    WH_MOUSE_LL, WM_APP, WM_LBUTTONDOWN, WM_MBUTTONDOWN, WM_NCLBUTTONDOWN, WM_NCRBUTTONDOWN,
    WM_RBUTTONDOWN,
};

pub(crate) const WM_MENU_DISMISS: u32 = WM_APP + 41;

static MENU_OPEN: AtomicBool = AtomicBool::new(false);

#[derive(Clone, Copy, Default)]
struct DismissTarget {
    hwnd_raw: usize,
    root_rect: PixelRect,
    sub_rect: Option<PixelRect>,
}

static DISMISS_TARGET: Mutex<DismissTarget> = Mutex::new(DismissTarget {
    hwnd_raw: 0,
    root_rect: PixelRect {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    },
    sub_rect: None,
});

pub(crate) fn is_fluent_menu_open() -> bool {
    MENU_OPEN.load(Ordering::Acquire)
}

pub(crate) struct MenuOutsideHookGuard {
    hook: Option<HHOOK>,
}

impl MenuOutsideHookGuard {
    pub(crate) fn install(root_hwnd: HWND, root_rect: PixelRect) -> Self {
        MENU_OPEN.store(true, Ordering::Release);
        if let Ok(mut target) = DISMISS_TARGET.lock() {
            target.hwnd_raw = root_hwnd.0 as usize;
            target.root_rect = root_rect;
            target.sub_rect = None;
        }
        // SAFETY: Installing a scoped WH_MOUSE_LL hook with static callback; unhooked on Drop.
        let hook = unsafe { SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_ll_proc), None, 0).ok() };
        Self { hook }
    }

    pub(crate) fn update_rects(root_rect: PixelRect, sub_rect: Option<PixelRect>) {
        if let Ok(mut target) = DISMISS_TARGET.lock() {
            target.root_rect = root_rect;
            target.sub_rect = sub_rect;
        }
    }
}

impl Drop for MenuOutsideHookGuard {
    fn drop(&mut self) {
        if let Some(h) = self.hook.take() {
            // SAFETY: Unhooking the HHOOK created by SetWindowsHookExW in install().
            unsafe {
                let _ = UnhookWindowsHookEx(h);
            }
        }
        if let Ok(mut target) = DISMISS_TARGET.lock() {
            target.hwnd_raw = 0;
            target.sub_rect = None;
        }
        MENU_OPEN.store(false, Ordering::Release);
    }
}

pub(crate) fn force_foreground(hwnd: HWND) {
    // SAFETY: Temporarily attaches to foreground window thread so SetForegroundWindow succeeds
    // even when called from non-activating widget/dock worker processes, then detaches.
    unsafe {
        let fg = GetForegroundWindow();
        let fg_tid = if !fg.is_invalid() {
            GetWindowThreadProcessId(fg, None)
        } else {
            0
        };
        let cur_tid = GetCurrentThreadId();
        let attached =
            fg_tid != 0 && fg_tid != cur_tid && AttachThreadInput(cur_tid, fg_tid, true).as_bool();
        let _ = SetForegroundWindow(hwnd);
        if attached {
            let _ = AttachThreadInput(cur_tid, fg_tid, false);
        }
    }
}

fn contains_pt(r: PixelRect, x: i32, y: i32) -> bool {
    x >= r.x && y >= r.y && x < r.x + r.width as i32 && y < r.y + r.height as i32
}

unsafe extern "system" fn mouse_ll_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && lparam.0 != 0 {
        let msg = wparam.0 as u32;
        let is_click = matches!(
            msg,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_NCLBUTTONDOWN | WM_NCRBUTTONDOWN
        );
        if is_click {
            let info = &*(lparam.0 as *const MSLLHOOKSTRUCT);
            let dismiss_hwnd = DISMISS_TARGET.try_lock().ok().and_then(|t| {
                if t.hwnd_raw == 0 {
                    return None;
                }
                let in_root = contains_pt(t.root_rect, info.pt.x, info.pt.y);
                let in_sub = t
                    .sub_rect
                    .is_some_and(|sr| contains_pt(sr, info.pt.x, info.pt.y));
                if !in_root && !in_sub {
                    Some(t.hwnd_raw)
                } else {
                    None
                }
            });
            if let Some(raw) = dismiss_hwnd {
                let hwnd = HWND(raw as *mut c_void);
                let _ = PostMessageW(Some(hwnd), WM_MENU_DISMISS, WPARAM(0), LPARAM(0));
            }
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}
