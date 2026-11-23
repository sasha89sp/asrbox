use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::mem;
use std::path::{Path, PathBuf};

use asrbox_platform_api::{
    BgraFrame, DockMenuAction, DockMenuState, DockPlatform, PixelPoint, PixelRect, PlatformEvent,
    TrayAction, TrayMenuEntry,
};
use windows::core::{PCWSTR, w};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::DragAcceptFiles;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetCursorPos,
    GetMessageW, IDC_ARROW, KillTimer, LoadCursorW, MSG, PostQuitMessage, RegisterClassExW,
    SW_SHOWNOACTIVATE, SetTimer, SetWindowLongPtrW, ShowWindow, TranslateMessage, WM_CONTEXTMENU,
    WM_DISPLAYCHANGE, WM_LBUTTONUP, WM_RBUTTONUP, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_TOOLWINDOW,
    WS_EX_TOPMOST, WS_POPUP,
};

use crate::dock_proc::dock_wnd_proc;
use crate::http::{WM_HTTP_DONE, drain_http_results};
use crate::present_layered_bgra;
use crate::query_primary_work_area;
use crate::tray::{
    TrayIconHandle, WM_TRAYICON, launch_path, pick_icon_dialog, show_dock_context_menu,
    show_tray_context_menu,
};
use crate::widget_proc::widget_wnd_proc;

pub(crate) const ANIMATION_TIMER_ID: usize = 1;
pub(crate) const ANIMATION_TICK_MS: u32 = 16;

thread_local! {
    static EVENT_QUEUE: RefCell<VecDeque<PlatformEvent>> = const { RefCell::new(VecDeque::new()) };
}

pub struct WindowsDockHost {
    pub(crate) hinstance: HINSTANCE,
    pub(crate) host_hwnd: HWND,
    _tray: TrayIconHandle,
    windows: HashMap<u64, HWND>,
    pub(crate) widget_windows: HashMap<u64, HWND>,
    pub(crate) tooltip_hwnd: Option<HWND>,
}

impl WindowsDockHost {
    pub fn new(tooltip: &str) -> Result<Self, String> {
        // SAFETY: Registering window classes and creating hidden host window on UI thread.
        unsafe {
            let hmod = GetModuleHandleW(None)
                .map_err(|err| format!("GetModuleHandleW failed: {err}"))?;
            let hinstance = HINSTANCE::from(hmod);
            register_class(hinstance, w!("AsrboxHostWindow"), Some(host_wnd_proc))?;
            register_class(hinstance, w!("AsrboxDockWindow"), Some(dock_wnd_proc))?;
            register_class(hinstance, w!("AsrboxWidgetWindow"), Some(widget_wnd_proc))?;

            let host_hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                w!("AsrboxHostWindow"),
                w!("asrbox"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(hinstance),
                None,
            )
            .map_err(|err| format!("CreateWindowExW(host) failed: {err}"))?;

            let tray = TrayIconHandle::create(host_hwnd, tooltip)?;
            Ok(Self {
                hinstance,
                host_hwnd,
                _tray: tray,
                windows: HashMap::new(),
                widget_windows: HashMap::new(),
                tooltip_hwnd: None,
            })
        }
    }

    pub fn run_event_loop<F>(&mut self, mut on_event: F)
    where
        F: FnMut(&mut Self, PlatformEvent),
    {
        let mut msg = MSG::default();
        loop {
            // SAFETY: Standard Win32 message loop on the UI thread.
            let status = unsafe { GetMessageW(&mut msg, None, 0, 0).0 };
            if status <= 0 {
                break;
            }
            // SAFETY: Translating and dispatching a valid MSG returned by GetMessageW.
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            while let Some(event) = pop_event() {
                on_event(self, event);
            }
        }
    }
}

impl Drop for WindowsDockHost {
    fn drop(&mut self) {
        let dock_hwnds: Vec<HWND> = self.windows.values().copied().collect();
        let widget_hwnds: Vec<HWND> = self.widget_windows.values().copied().collect();
        self.windows.clear();
        self.widget_windows.clear();
        // SAFETY: Destroying owned HWNDs on the UI thread during teardown.
        unsafe {
            for hwnd in dock_hwnds.into_iter().chain(widget_hwnds) {
                let _ = DestroyWindow(hwnd);
            }
            if let Some(tip) = self.tooltip_hwnd.take() {
                let _ = DestroyWindow(tip);
            }
            let _ = DestroyWindow(self.host_hwnd);
        }
    }
}

impl DockPlatform for WindowsDockHost {
    fn primary_work_area(&self) -> Result<PixelRect, String> {
        query_primary_work_area()
    }

    fn create_dock_window(
        &mut self,
        dock_id: u64,
        title: &str,
        rect: PixelRect,
    ) -> Result<(), String> {
        if self.windows.contains_key(&dock_id) {
            return Ok(());
        }
        let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
        let ex_style = WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW;
        // SAFETY: Creating a top-level layered window with a registered class on UI thread.
        unsafe {
            let hwnd = CreateWindowExW(
                ex_style,
                w!("AsrboxDockWindow"),
                PCWSTR(title_wide.as_ptr()),
                WS_POPUP,
                rect.x,
                rect.y,
                i32::try_from(rect.width).unwrap_or(100),
                i32::try_from(rect.height).unwrap_or(64),
                None,
                None,
                Some(self.hinstance),
                None,
            )
            .map_err(|err| format!("CreateWindowExW(dock) failed: {err}"))?;

            SetWindowLongPtrW(hwnd, GWLP_USERDATA, dock_id as isize);
            DragAcceptFiles(hwnd, true);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
            self.windows.insert(dock_id, hwnd);
        }
        Ok(())
    }

    fn destroy_dock_window(&mut self, dock_id: u64) -> Result<(), String> {
        if let Some(hwnd) = self.windows.remove(&dock_id) {
            // SAFETY: `hwnd` belongs to this thread and is removed from the map.
            unsafe {
                let _ = KillTimer(Some(hwnd), ANIMATION_TIMER_ID);
                let _ = DestroyWindow(hwnd);
            }
        }
        Ok(())
    }

    fn present_dock_frame(
        &mut self,
        dock_id: u64,
        rect: PixelRect,
        frame: &BgraFrame,
    ) -> Result<(), String> {
        let hwnd = self
            .windows
            .get(&dock_id)
            .copied()
            .ok_or_else(|| format!("unknown dock window {dock_id}"))?;
        present_layered_bgra(hwnd, rect, frame)
    }

    fn set_animation_timer(&mut self, dock_id: u64, active: bool) -> Result<(), String> {
        let Some(&hwnd) = self.windows.get(&dock_id) else {
            return Ok(());
        };
        // SAFETY: Starting/stopping a Win32 timer for an owned HWND on UI thread.
        unsafe {
            if active {
                let _ = SetTimer(Some(hwnd), ANIMATION_TIMER_ID, ANIMATION_TICK_MS, None);
            } else {
                let _ = KillTimer(Some(hwnd), ANIMATION_TIMER_ID);
            }
        }
        Ok(())
    }

    fn show_dock_menu(
        &mut self,
        dock_id: u64,
        screen_point: PixelPoint,
        state: &DockMenuState,
    ) -> Result<Option<DockMenuAction>, String> {
        let hwnd = self.windows.get(&dock_id).copied().unwrap_or(self.host_hwnd);
        show_dock_context_menu(hwnd, screen_point, state)
    }

    fn show_tray_menu(
        &mut self,
        screen_point: PixelPoint,
        docks: &[TrayMenuEntry],
        calendar_visible: bool,
    ) -> Result<Option<TrayAction>, String> {
        show_tray_context_menu(self.host_hwnd, screen_point, docks, calendar_visible)
    }

    fn pick_icon_file(&mut self) -> Result<Option<PathBuf>, String> {
        pick_icon_dialog(self.host_hwnd)
    }

    fn launch_target(&self, path: &Path) -> Result<(), String> {
        launch_path(path)
    }

    fn request_quit(&mut self) {
        // SAFETY: Posting WM_QUIT terminates the UI message loop cleanly.
        unsafe {
            PostQuitMessage(0);
        }
    }
}

unsafe fn register_class(
    hinstance: HINSTANCE,
    class_name: PCWSTR,
    wnd_proc: Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT>,
) -> Result<(), String> {
    let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
    let wc = WNDCLASSEXW {
        cbSize: u32::try_from(mem::size_of::<WNDCLASSEXW>()).unwrap_or(0),
        lpfnWndProc: wnd_proc,
        hInstance: hinstance,
        hCursor: cursor,
        lpszClassName: class_name,
        ..Default::default()
    };
    if RegisterClassExW(&wc) == 0 {
        return Err("RegisterClassExW failed".to_owned());
    }
    Ok(())
}

unsafe extern "system" fn host_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            let sub = (lparam.0 as usize & 0xFFFF) as u32;
            if sub == WM_RBUTTONUP || sub == WM_CONTEXTMENU || sub == WM_LBUTTONUP {
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                push_event(PlatformEvent::TrayContextMenu {
                    screen_point: PixelPoint { x: pt.x, y: pt.y },
                });
            }
            LRESULT(0)
        }
        WM_HTTP_DONE => {
            for (request_id, result) in drain_http_results() {
                push_event(PlatformEvent::HttpFetchDone { request_id, result });
            }
            LRESULT(0)
        }
        WM_DISPLAYCHANGE => {
            if let Ok(work_area) = query_primary_work_area() {
                push_event(PlatformEvent::DisplayChanged { work_area });
            }
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

pub(crate) fn lparam_point(lparam: LPARAM) -> PixelPoint {
    let raw = lparam.0 as usize;
    let x = i32::from((raw & 0xFFFF) as i16);
    let y = i32::from(((raw >> 16) & 0xFFFF) as i16);
    PixelPoint { x, y }
}

pub(crate) fn push_event(event: PlatformEvent) {
    EVENT_QUEUE.with(|queue| queue.borrow_mut().push_back(event));
}

fn pop_event() -> Option<PlatformEvent> {
    EVENT_QUEUE.with(|queue| queue.borrow_mut().pop_front())
}
