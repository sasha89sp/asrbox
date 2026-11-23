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
mod http;
#[cfg(windows)]
mod icons;
#[cfg(windows)]
mod job;
#[cfg(windows)]
mod metrics;
#[cfg(windows)]
mod running;
#[cfg(windows)]
mod settings_proc;
#[cfg(windows)]
mod shell;
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
mod tools_win;
#[cfg(windows)]
mod tray;
#[cfg(windows)]
mod webdav_http;
#[cfg(windows)]
mod widget_menu;
#[cfg(windows)]
mod widget_proc;
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
pub use window::{wake_host_window, WindowsDockHost};

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
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    AC_SRC_ALPHA, AC_SRC_OVER, BLENDFUNCTION, DIB_RGB_COLORS,
};
#[cfg(windows)]
use windows::Win32::System::Threading::CreateMutexW;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{
    SystemParametersInfoW, UpdateLayeredWindow, SPI_GETWORKAREA,
    SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, ULW_ALPHA,
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
