use std::mem;

use asrbox_platform_api::PixelRect;
use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HWND, RECT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible,
};

const NON_GAME_PROCS: &[&str] = &[
    "asrbox.exe",
    "explorer.exe",
    "shellexperiencehost.exe",
    "searchhost.exe",
    "startmenuexperiencehost.exe",
    "applicationframehost.exe",
    "lockapp.exe",
    "dwm.exe",
    "taskmgr.exe",
    "chrome.exe",
    "msedge.exe",
    "firefox.exe",
    "opera.exe",
    "brave.exe",
    "vivaldi.exe",
    "browser.exe",
    "yandex.exe",
    "code.exe",
    "devenv.exe",
    "notepad.exe",
    "notepad++.exe",
    "windowsterminal.exe",
    "wt.exe",
    "cmd.exe",
    "powershell.exe",
    "pwsh.exe",
    "telegram.exe",
    "discord.exe",
    "teams.exe",
    "zoom.exe",
    "skype.exe",
    "vlc.exe",
    "mpv.exe",
    "potplayermini64.exe",
    "mpc-hc64.exe",
    "obs64.exe",
    "mstsc.exe",
];

const NON_GAME_CLASSES: &[&str] = &[
    "progman",
    "workerw",
    "shell_traywnd",
    "cabinetwclass",
    "chrome_widgetwin_1",
    "mozillawindowclass",
    "applicationframewindow",
];

pub(crate) struct GameDetectResult {
    pub active: bool,
    pub process_name: String,
    pub monitor_rect: Option<PixelRect>,
}

pub(crate) fn detect_foreground_game(extra_games: &[String]) -> GameDetectResult {
    // SAFETY: Read-only Win32 foreground/monitor queries on current desktop session.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.0.is_null() || !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return GameDetectResult {
                active: false,
                process_name: String::new(),
                monitor_rect: None,
            };
        }

        let monitor_rect = query_window_monitor(hwnd);
        let proc_name = query_hwnd_process_name(hwnd);
        let lower_proc = proc_name.to_ascii_lowercase();
        let lower_class = query_hwnd_class(hwnd).to_ascii_lowercase();

        if lower_proc == "asrbox.exe" || NON_GAME_CLASSES.contains(&lower_class.as_str()) {
            return GameDetectResult {
                active: false,
                process_name: proc_name,
                monitor_rect,
            };
        }

        if !lower_proc.is_empty() && extra_games.iter().any(|g| g == &lower_proc) {
            return GameDetectResult {
                active: true,
                process_name: proc_name,
                monitor_rect,
            };
        }

        if let Ok(state) = SHQueryUserNotificationState() {
            if state == QUNS_RUNNING_D3D_FULL_SCREEN {
                return GameDetectResult {
                    active: true,
                    process_name: proc_name,
                    monitor_rect,
                };
            }
        }

        if NON_GAME_PROCS.contains(&lower_proc.as_str()) {
            return GameDetectResult {
                active: false,
                process_name: proc_name,
                monitor_rect,
            };
        }

        let covers_screen = monitor_rect.is_some_and(|mon| window_covers_monitor(hwnd, mon));
        GameDetectResult {
            active: covers_screen,
            process_name: proc_name,
            monitor_rect,
        }
    }
}

unsafe fn query_window_monitor(hwnd: HWND) -> Option<PixelRect> {
    let hmon = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
    if hmon.is_invalid() {
        return None;
    }
    let mut info = MONITORINFO {
        cbSize: u32::try_from(mem::size_of::<MONITORINFO>()).unwrap_or(0),
        ..Default::default()
    };
    if !GetMonitorInfoW(hmon, &mut info).as_bool() {
        return None;
    }
    let r = info.rcMonitor;
    let w = u32::try_from(r.right.saturating_sub(r.left).max(1)).unwrap_or(1920);
    let h = u32::try_from(r.bottom.saturating_sub(r.top).max(1)).unwrap_or(1080);
    Some(PixelRect {
        x: r.left,
        y: r.top,
        width: w,
        height: h,
    })
}

unsafe fn window_covers_monitor(hwnd: HWND, mon: PixelRect) -> bool {
    let mut wr = RECT::default();
    if GetWindowRect(hwnd, &mut wr).is_err() {
        return false;
    }
    let ww = wr.right.saturating_sub(wr.left);
    let wh = wr.bottom.saturating_sub(wr.top);
    let mw = i32::try_from(mon.width).unwrap_or(1920);
    let mh = i32::try_from(mon.height).unwrap_or(1080);
    if mw <= 0 || mh <= 0 || ww < (mw * 98) / 100 || wh < (mh * 98) / 100 {
        return false;
    }
    (wr.left - mon.x).abs() <= 8 && (wr.top - mon.y).abs() <= 8
}

unsafe fn query_hwnd_class(hwnd: HWND) -> String {
    let mut buf = [0u16; 128];
    let len = GetClassNameW(hwnd, &mut buf) as usize;
    String::from_utf16_lossy(&buf[..len.min(buf.len())])
}

unsafe fn query_hwnd_process_name(hwnd: HWND) -> String {
    let mut pid = 0u32;
    let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
    if pid == 0 {
        return String::new();
    }
    let Ok(hproc) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else {
        return String::new();
    };
    let mut buf = [0u16; 512];
    let mut len = buf.len() as u32;
    let ok =
        QueryFullProcessImageNameW(hproc, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len)
            .is_ok();
    let _ = CloseHandle(hproc);
    if !ok || len == 0 {
        return String::new();
    }
    let full = String::from_utf16_lossy(&buf[..(len as usize).min(buf.len())]);
    full.rsplit(['\\', '/']).next().unwrap_or(&full).to_owned()
}
