#[cfg(windows)]
mod app_icon;
#[cfg(windows)]
mod autostart;
#[cfg(windows)]
mod dock_menu;
#[cfg(windows)]
mod dock_proc;
#[cfg(windows)]
mod dock_win;
#[cfg(windows)]
mod dwrite_text;
#[cfg(windows)]
mod dwrite_vtable;
#[cfg(windows)]
mod fluent_menu;
#[cfg(windows)]
mod fluent_menu_hook;
#[cfg(windows)]
mod fluent_menu_layout;
#[cfg(windows)]
mod fluent_menu_render;
#[cfg(windows)]
mod fluent_menu_win;
#[cfg(windows)]
mod folder_popup;
#[cfg(windows)]
mod folder_popup_render;
#[cfg(windows)]
mod font_engine;
#[cfg(windows)]
mod github_http;
#[cfg(windows)]
mod http;
#[cfg(windows)]
mod icons;
#[cfg(windows)]
mod icons_lnk;
#[cfg(windows)]
mod job;
#[cfg(windows)]
mod metrics;
#[cfg(windows)]
mod notes_proc;
#[cfg(windows)]
mod notes_win;
#[cfg(windows)]
mod overlay_game;
#[cfg(windows)]
mod overlay_telemetry;
#[cfg(windows)]
mod running;
#[cfg(windows)]
mod settings_helpers;
#[cfg(windows)]
mod settings_proc;
#[cfg(windows)]
mod shell;
#[cfg(windows)]
mod slint_host;
#[cfg(windows)]
mod sys_gpu;
#[cfg(windows)]
mod sys_net;
#[cfg(windows)]
mod sys_procs;
#[cfg(windows)]
mod sys_telemetry;
#[cfg(windows)]
mod tools_proc;
#[cfg(windows)]
mod tools_state;
#[cfg(windows)]
mod tools_win;
#[cfg(windows)]
mod tray;
#[cfg(windows)]
mod tray_menu;
#[cfg(windows)]
mod webdav_http;
#[cfg(windows)]
mod webdav_sync;
#[cfg(windows)]
mod webdav_wire;
#[cfg(windows)]
mod webdav_xml;
#[cfg(windows)]
mod widget_menu;
#[cfg(windows)]
mod widget_proc;
#[cfg(windows)]
mod widget_resize;
#[cfg(windows)]
mod widget_text;
#[cfg(windows)]
mod widget_win;
#[cfg(windows)]
mod window;

#[cfg(windows)]
pub use icons::WindowsIconProvider;
#[cfg(windows)]
pub use job::{is_process_alive, JobObject};
#[cfg(windows)]
pub use shell::pick_icon_file;
#[cfg(windows)]
pub use slint_host::{
    apply_window_app_icon, cursor_screen_pos, move_window_phys, set_window_skip_taskbar,
    show_dock_native_menu, show_folder_native_popup, show_native_fluent_menu,
};
#[cfg(windows)]
pub use sys_telemetry::SysTelemetryCollector;
#[cfg(windows)]
pub use window::{wake_host_window, WindowsDockHost};

#[cfg(windows)]
pub fn cred_load(target: &str) -> Result<Option<(String, String)>, String> {
    autostart::cred_load(target)
}

#[cfg(windows)]
pub fn winhttp_fetch(
    url: &str,
    headers: &[(String, String)],
) -> asrbox_platform_api::HttpFetchResult {
    http::fetch_blocking(url, headers)
}

#[cfg(windows)]
pub fn activate_or_launch_target(
    path: &std::path::Path,
    focus_running: bool,
) -> Result<bool, String> {
    running::activate_or_launch_target(path, focus_running)
}

#[cfg(windows)]
pub fn query_running_exe_names() -> std::collections::HashSet<String> {
    running::query_running_exe_names()
}

#[cfg(windows)]
pub fn resolve_target_exe_name(path: &std::path::Path) -> Option<String> {
    running::resolve_target_exe_name(path)
}

#[cfg(windows)]
pub fn launch_system_tool(cmd: &str, args: &str) -> Result<(), String> {
    shell::launch_system_tool(cmd, args)
}

#[cfg(windows)]
pub fn open_in_explorer(path: &std::path::Path) -> Result<(), String> {
    shell::open_in_explorer(path)
}

#[cfg(windows)]
pub fn open_item_location(path: &std::path::Path) -> Result<(), String> {
    shell::open_item_location(path)
}

#[cfg(windows)]
pub fn open_in_notepad(path: &std::path::Path) -> Result<(), String> {
    shell::open_in_notepad(path)
}

#[cfg(windows)]
pub fn open_url_str(url: &str) -> Result<(), String> {
    shell::open_url_str(url)
}

#[cfg(windows)]
pub fn query_local_time() -> asrbox_platform_api::LocalDateTime {
    widget_text::query_local_time()
}

#[cfg(windows)]
pub fn primary_work_area() -> Result<PixelRect, String> {
    query_primary_work_area()
}

#[cfg(windows)]
pub fn apply_dwm_frameless_style(hwnd_raw: isize, dark: bool, round_small: bool) -> bool {
    if hwnd_raw == 0 {
        return false;
    }
    let hwnd = HWND(hwnd_raw as *mut c_void);
    let dark_val: i32 = if dark { 1 } else { 0 };
    let corner_val: i32 = if round_small { 3 } else { 2 };
    let border_none: u32 = 0xFFFF_FFFE;
    // SAFETY: `hwnd_raw` is a live Win32 HWND obtained from winit's HasWindowHandle.
    unsafe {
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            (&dark_val as *const i32).cast::<c_void>(),
            std::mem::size_of::<i32>() as u32,
        );
        let rounded = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(33),
            (&corner_val as *const i32).cast::<c_void>(),
            std::mem::size_of::<i32>() as u32,
        )
        .is_ok();
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWINDOWATTRIBUTE(34),
            (&border_none as *const u32).cast::<c_void>(),
            std::mem::size_of::<u32>() as u32,
        );
        rounded
    }
}

#[cfg(windows)]
pub fn set_window_opacity_alpha(hwnd_raw: isize, opacity_percent: u8) {
    if hwnd_raw == 0 {
        return;
    }
    let hwnd = HWND(hwnd_raw as *mut c_void);
    let pct = opacity_percent.clamp(25, 100);
    // SAFETY: `hwnd_raw` is a valid Win32 HWND; toggling WS_EX_LAYERED + LWA_ALPHA is safe.
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let layered_bit = WS_EX_LAYERED.0 as isize;
        if pct >= 100 {
            if (ex & layered_bit) != 0 {
                let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex & !layered_bit);
            }
        } else {
            if (ex & layered_bit) == 0 {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex | layered_bit);
            }
            let alpha = ((u32::from(pct) * 255) / 100) as u8;
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), alpha, LWA_ALPHA);
        }
    }
}

#[cfg(windows)]
pub fn force_redraw_hwnd(hwnd_raw: isize) {
    if hwnd_raw == 0 {
        return;
    }
    let hwnd = HWND(hwnd_raw as *mut c_void);
    // SAFETY: `hwnd_raw` is a live Win32 HWND; InvalidateRect schedules a full client repaint.
    unsafe {
        let _ = InvalidateRect(Some(hwnd), None, false);
    }
}

#[cfg(windows)]
use std::ffi::c_void;

#[cfg(windows)]
use asrbox_platform_api::{BgraFrame, PixelRect};
#[cfg(windows)]
use windows::core::PCWSTR;
#[cfg(windows)]
use windows::Win32::Foundation::{
    CloseHandle, GetLastError, COLORREF, ERROR_ALREADY_EXISTS, HANDLE, HWND, POINT, RECT, SIZE,
};
#[cfg(windows)]
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE, DWMWINDOWATTRIBUTE,
};
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, InvalidateRect, ReleaseDC,
    SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, DIB_RGB_COLORS,
};
#[cfg(windows)]
use windows::Win32::System::Threading::CreateMutexW;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, SetLayeredWindowAttributes, SetWindowLongPtrW, SystemParametersInfoW,
    UpdateLayeredWindow, GWL_EXSTYLE, LWA_ALPHA, SPI_GETWORKAREA,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, ULW_ALPHA, WS_EX_LAYERED,
};

#[cfg(windows)]
pub struct SingleInstanceGuard {
    handle: HANDLE,
}

#[cfg(windows)]
impl SingleInstanceGuard {
    pub fn acquire(name: &str) -> Result<Option<Self>, String> {
        let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: `wide` is NUL-terminated UTF-16; the returned mutex is closed on Drop.
        unsafe {
            let handle = CreateMutexW(None, false, PCWSTR(wide.as_ptr()))
                .map_err(|err| format!("CreateMutexW failed: {err}"))?;
            if GetLastError() == ERROR_ALREADY_EXISTS {
                let _ = CloseHandle(handle);
                return Ok(None);
            }
            Ok(Some(Self { handle }))
        }
    }
}

#[cfg(windows)]
impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            // SAFETY: `self.handle` is owned exclusively by this guard and closed once.
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

#[cfg(windows)]
pub(crate) fn present_layered_bgra(
    hwnd: HWND,
    rect: PixelRect,
    frame: &BgraFrame,
) -> Result<(), String> {
    let width = i32::try_from(frame.width).map_err(|_| "frame width overflow".to_owned())?;
    let height = i32::try_from(frame.height).map_err(|_| "frame height overflow".to_owned())?;
    let bmi = icons::make_top_down_bmi(width, height);
    let mut bits: *mut c_void = std::ptr::null_mut();

    // SAFETY: All GDI handles are freed before returning; copy size matches DIB buffer.
    unsafe {
        let hdc_screen = GetDC(None);
        let hdc_mem = CreateCompatibleDC(Some(hdc_screen));
        let dib = CreateDIBSection(Some(hdc_mem), &bmi, DIB_RGB_COLORS, &mut bits, None, 0);
        let hbitmap = match dib {
            Ok(bmp) => bmp,
            Err(err) => {
                let _ = DeleteDC(hdc_mem);
                let _ = ReleaseDC(None, hdc_screen);
                return Err(format!("CreateDIBSection failed: {err}"));
            }
        };
        if !bits.is_null() {
            let expected = (frame.width as usize) * (frame.height as usize) * 4;
            let count = expected.min(frame.pixels.len());
            std::ptr::copy_nonoverlapping(frame.pixels.as_ptr(), bits as *mut u8, count);
        }
        let prev = SelectObject(hdc_mem, hbitmap.into());
        let (x, y) = (rect.x, rect.y);
        let pt_dst = POINT { x, y };
        let (cx, cy) = (width, height);
        let size = SIZE { cx, cy };
        let pt_src = POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: AC_SRC_OVER as u8,
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: AC_SRC_ALPHA as u8,
        };
        let res = UpdateLayeredWindow(
            hwnd,
            Some(hdc_screen),
            Some(&pt_dst),
            Some(&size),
            Some(hdc_mem),
            Some(&pt_src),
            COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );
        let _ = SelectObject(hdc_mem, prev);
        let _ = DeleteObject(hbitmap.into());
        let _ = DeleteDC(hdc_mem);
        let _ = ReleaseDC(None, hdc_screen);
        res.map_err(|err| format!("UpdateLayeredWindow failed: {err}"))
    }
}

#[cfg(windows)]
pub(crate) fn query_primary_work_area() -> Result<PixelRect, String> {
    let mut rect = RECT::default();
    // SAFETY: `rect` is valid writable storage matching SPI_GETWORKAREA.
    unsafe {
        SystemParametersInfoW(
            SPI_GETWORKAREA,
            0,
            Some(&mut rect as *mut RECT as *mut c_void),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .map_err(|err| format!("SystemParametersInfoW(SPI_GETWORKAREA) failed: {err}"))?;
    }
    let width = u32::try_from(rect.right.saturating_sub(rect.left).max(1)).unwrap_or(1920);
    let height = u32::try_from(rect.bottom.saturating_sub(rect.top).max(1)).unwrap_or(1080);
    Ok(PixelRect {
        x: rect.left,
        y: rect.top,
        width,
        height,
    })
}
