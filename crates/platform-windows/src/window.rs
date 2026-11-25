use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::ffi::c_void;
use std::mem;

use asrbox_platform_api::{AccentPreset, FluentPalette, PixelPoint, PlatformEvent};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{COLOR_WINDOW, HBRUSH};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW,
    KillTimer, LoadCursorW, PostMessageW, RegisterClassExW, SetTimer, TranslateMessage, IDC_ARROW,
    MSG, WM_APP, WM_DISPLAYCHANGE, WM_LBUTTONUP, WM_RBUTTONUP, WM_TIMER, WNDCLASSEXW,
    WS_EX_TOOLWINDOW, WS_POPUP,
};

use crate::app_icon::{cleanup_app_icons, large_app_icon, small_app_icon};
use crate::dock_proc::dock_wnd_proc;
use crate::dwrite_text::cleanup_dwrite_cache;
use crate::fluent_menu::fluent_menu_wnd_proc;
use crate::font_engine::cleanup_font_cache;
use crate::github_http::{drain_github_results, WM_GITHUB_DONE};
use crate::http::{drain_http_results, WM_HTTP_DONE};
use crate::metrics::ProcessMetricsSampler;
use crate::notes_proc::notes_wnd_proc;
use crate::notes_win::close_notes_win;
use crate::overlay_telemetry::{drain_overlay_results, WM_OVERLAY_DONE};
use crate::query_primary_work_area;
use crate::settings_proc::settings_wnd_proc;
use crate::sys_telemetry::{drain_telemetry_results, WM_TELEMETRY_DONE};
use crate::tools_proc::{about_wnd_proc, debug_wnd_proc};
use crate::tools_win::ToolsWinState;
use crate::tray::{TrayIconHandle, WM_TRAYICON};
use crate::webdav_http::{drain_webdav_results, WM_WEBDAV_DONE};
use crate::widget_proc::widget_wnd_proc;

pub(crate) const ANIMATION_TIMER_ID: usize = 1;
pub(crate) const ANIMATION_TICK_MS: u32 = 16;
pub(crate) const RUNNING_POLL_TIMER_ID: usize = 2;
pub(crate) const RUNNING_POLL_MS: u32 = 2000;
const SUPERVISOR_TICK_TIMER_ID: usize = 3;
const SUPERVISOR_TICK_MS: u32 = 500;
pub(crate) const WM_MOUSELEAVE: u32 = 0x02A3;
const WM_WAKE_EVENTS: u32 = WM_APP + 12;
const WM_IPC_WAKE: u32 = WM_APP + 13;

thread_local! {
    static EVENT_QUEUE: RefCell<VecDeque<PlatformEvent>> = const { RefCell::new(VecDeque::new()) };
    static HOST_HWND_RAW: Cell<usize> = const { Cell::new(0) };
    static WAKE_POSTED: Cell<bool> = const { Cell::new(false) };
}

pub struct WindowsDockHost {
    pub(crate) hinstance: HINSTANCE,
    pub(crate) host_hwnd: HWND,
    _tray: Option<TrayIconHandle>,
    pub(crate) windows: HashMap<u64, HWND>,
    pub(crate) widget_windows: HashMap<u64, HWND>,
    pub(crate) tooltip_hwnd: Option<HWND>,
    pub(crate) tools: ToolsWinState,
    pub(crate) metrics: ProcessMetricsSampler,
    pub(crate) theme: FluentPalette,
}

impl WindowsDockHost {
    pub fn new(tooltip: &str) -> Result<Self, String> {
        Self::new_internal(Some(tooltip))
    }

    pub fn new_worker() -> Result<Self, String> {
        Self::new_internal(None)
    }

    fn new_internal(tray_tooltip: Option<&str>) -> Result<Self, String> {
        // SAFETY: Registering window classes and creating hidden host window on UI thread.
        unsafe {
            let hmod =
                GetModuleHandleW(None).map_err(|err| format!("GetModuleHandleW failed: {err}"))?;
            let hinstance = HINSTANCE::from(hmod);
            let bg = HBRUSH((COLOR_WINDOW.0 + 1) as isize as *mut c_void);
            register_class(
                hinstance,
                w!("AsrboxHostWindow"),
                Some(host_wnd_proc),
                HBRUSH::default(),
            )?;
            register_class(
                hinstance,
                w!("AsrboxDockWindow"),
                Some(dock_wnd_proc),
                HBRUSH::default(),
            )?;
            register_class(
                hinstance,
                w!("AsrboxWidgetWindow"),
                Some(widget_wnd_proc),
                HBRUSH::default(),
            )?;
            register_class(
                hinstance,
                w!("AsrboxFluentMenuWindow"),
                Some(fluent_menu_wnd_proc),
                HBRUSH::default(),
            )?;
            register_class(hinstance, w!("AsrboxDebugWindow"), Some(debug_wnd_proc), bg)?;
            register_class(hinstance, w!("AsrboxAboutWindow"), Some(about_wnd_proc), bg)?;
            register_class(
                hinstance,
                w!("AsrboxSettingsWindow"),
                Some(settings_wnd_proc),
                HBRUSH::default(),
            )?;
            register_class(
                hinstance,
                w!("AsrboxNotesWindow"),
                Some(notes_wnd_proc),
                HBRUSH::default(),
            )?;

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

            HOST_HWND_RAW.with(|cell| cell.set(host_hwnd.0 as usize));
            WAKE_POSTED.with(|cell| cell.set(false));
            let tray = match tray_tooltip {
                Some(tip) => Some(TrayIconHandle::create(host_hwnd, tip)?),
                None => None,
            };
            Ok(Self {
                hinstance,
                host_hwnd,
                _tray: tray,
                windows: HashMap::new(),
                widget_windows: HashMap::new(),
                tooltip_hwnd: None,
                tools: ToolsWinState::new(),
                metrics: ProcessMetricsSampler::new(),
                theme: FluentPalette::dark(AccentPreset::Blue),
            })
        }
    }

    pub fn host_hwnd_raw(&self) -> usize {
        self.host_hwnd.0 as usize
    }

    pub fn set_supervisor_timer(&mut self, active: bool) {
        // SAFETY: Configuring supervisor heartbeat timer on owned host_hwnd.
        unsafe {
            if active {
                let _ = SetTimer(
                    Some(self.host_hwnd),
                    SUPERVISOR_TICK_TIMER_ID,
                    SUPERVISOR_TICK_MS,
                    None,
                );
            } else {
                let _ = KillTimer(Some(self.host_hwnd), SUPERVISOR_TICK_TIMER_ID);
            }
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
            WAKE_POSTED.with(|cell| cell.set(false));
            while let Some(event) = pop_event() {
                on_event(self, event);
            }
        }
    }
}

pub fn wake_host_window(hwnd_raw: usize) {
    if hwnd_raw == 0 {
        return;
    }
    let hwnd = HWND(hwnd_raw as *mut c_void);
    // SAFETY: PostMessageW is thread-safe and safely wakes GetMessageW with a custom WM_APP message.
    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_IPC_WAKE, WPARAM(0), LPARAM(0));
    }
}

impl Drop for WindowsDockHost {
    fn drop(&mut self) {
        close_notes_win();
        self.tools.cleanup();
        let dock_hwnds: Vec<HWND> = self.windows.values().copied().collect();
        let widget_hwnds: Vec<HWND> = self.widget_windows.values().copied().collect();
        self.windows.clear();
        self.widget_windows.clear();
        // SAFETY: Destroying owned HWNDs on the UI thread during teardown.
        unsafe {
            let _ = KillTimer(Some(self.host_hwnd), RUNNING_POLL_TIMER_ID);
            let _ = KillTimer(Some(self.host_hwnd), SUPERVISOR_TICK_TIMER_ID);
            for hwnd in dock_hwnds.into_iter().chain(widget_hwnds) {
                let _ = DestroyWindow(hwnd);
            }
            if let Some(tip) = self.tooltip_hwnd.take() {
                let _ = DestroyWindow(tip);
            }
            let _ = DestroyWindow(self.host_hwnd);
        }
        cleanup_app_icons();
        cleanup_font_cache();
        cleanup_dwrite_cache();
        HOST_HWND_RAW.with(|cell| cell.set(0));
        WAKE_POSTED.with(|cell| cell.set(false));
    }
}

unsafe fn register_class(
    hinstance: HINSTANCE,
    class_name: PCWSTR,
    wnd_proc: Option<unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT>,
    hbr_background: HBRUSH,
) -> Result<(), String> {
    let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
    let wc = WNDCLASSEXW {
        cbSize: u32::try_from(mem::size_of::<WNDCLASSEXW>()).unwrap_or(0),
        lpfnWndProc: wnd_proc,
        hInstance: hinstance,
        hIcon: large_app_icon(),
        hIconSm: small_app_icon(),
        hCursor: cursor,
        hbrBackground: hbr_background,
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
            if sub == WM_RBUTTONUP || sub == WM_LBUTTONUP {
                let mut pt = POINT::default();
                let _ = GetCursorPos(&mut pt);
                push_event(PlatformEvent::TrayContextMenu {
                    screen_point: PixelPoint { x: pt.x, y: pt.y },
                });
            }
            LRESULT(0)
        }
        WM_WAKE_EVENTS => LRESULT(0),
        WM_IPC_WAKE => {
            push_event(PlatformEvent::SupervisorTick);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == RUNNING_POLL_TIMER_ID => {
            push_event(PlatformEvent::RunningPollTick);
            LRESULT(0)
        }
        WM_TIMER if wparam.0 == SUPERVISOR_TICK_TIMER_ID => {
            push_event(PlatformEvent::SupervisorTick);
            LRESULT(0)
        }
        WM_HTTP_DONE => {
            for (request_id, result) in drain_http_results() {
                push_event(PlatformEvent::HttpFetchDone { request_id, result });
            }
            LRESULT(0)
        }
        WM_TELEMETRY_DONE => {
            for snap in drain_telemetry_results() {
                push_event(PlatformEvent::SysTelemetryReady {
                    snapshot: Box::new(snap),
                });
            }
            LRESULT(0)
        }
        WM_OVERLAY_DONE => {
            for snap in drain_overlay_results() {
                push_event(PlatformEvent::OverlayTelemetryReady {
                    snapshot: Box::new(snap),
                });
            }
            LRESULT(0)
        }
        WM_WEBDAV_DONE => {
            for res in drain_webdav_results() {
                push_event(PlatformEvent::WebDavDone(res));
            }
            LRESULT(0)
        }
        WM_GITHUB_DONE => {
            for res in drain_github_results() {
                push_event(PlatformEvent::GitHubFetchDone(res));
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
    let raw = HOST_HWND_RAW.with(Cell::get);
    let already = WAKE_POSTED.with(|cell| cell.replace(true));
    if raw != 0 && !already {
        let hwnd = HWND(raw as *mut c_void);
        // SAFETY: Posting a no-op wake message unblocks GetMessageW after SendMessage callbacks.
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_WAKE_EVENTS, WPARAM(0), LPARAM(0));
        }
    }
}

fn pop_event() -> Option<PlatformEvent> {
    EVENT_QUEUE.with(|queue| queue.borrow_mut().pop_front())
}
