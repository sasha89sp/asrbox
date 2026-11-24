use std::mem;

use asrbox_platform_api::{FluentPalette, PixelPoint, TrayAction, TrayMenuEntry, TrayModuleFlags};
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::WM_APP;

use crate::app_icon::small_app_icon;
use crate::fluent_menu::show_fluent_menu;
use crate::tray_menu::{build_tray_menu_spec, decode_tray_command};

pub(crate) const WM_TRAYICON: u32 = WM_APP + 10;

pub(crate) struct TrayIconHandle {
    hwnd: HWND,
    active: bool,
}

impl TrayIconHandle {
    pub(crate) fn create(hwnd: HWND, tooltip: &str) -> Result<Self, String> {
        let mut nid = NOTIFYICONDATAW {
            cbSize: u32::try_from(mem::size_of::<NOTIFYICONDATAW>()).unwrap_or(0),
            hWnd: hwnd,
            uID: 1,
            uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            ..Default::default()
        };
        let tip_wide: Vec<u16> = tooltip.encode_utf16().collect();
        let max_len = nid.szTip.len().saturating_sub(1);
        for (dst, src) in nid.szTip.iter_mut().take(max_len).zip(tip_wide.iter()) {
            *dst = *src;
        }
        // SAFETY: `hwnd` is a valid UI-thread window; `small_app_icon()` returns a cached HICON.
        unsafe {
            let icon = small_app_icon();
            if !icon.is_invalid() {
                nid.hIcon = icon;
            }
            if !Shell_NotifyIconW(NIM_ADD, &nid).as_bool() {
                return Err("Shell_NotifyIconW(NIM_ADD) failed".to_owned());
            }
        }
        Ok(Self { hwnd, active: true })
    }
}

impl Drop for TrayIconHandle {
    fn drop(&mut self) {
        if !self.active {
            return;
        }
        let nid = NOTIFYICONDATAW {
            cbSize: u32::try_from(mem::size_of::<NOTIFYICONDATAW>()).unwrap_or(0),
            hWnd: self.hwnd,
            uID: 1,
            ..Default::default()
        };
        // SAFETY: Removing the tray icon previously registered on `self.hwnd`.
        unsafe {
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        }
        self.active = false;
    }
}

pub(crate) fn show_tray_context_menu(
    hinstance: HINSTANCE,
    hwnd: HWND,
    screen_pt: PixelPoint,
    docks: &[TrayMenuEntry],
    flags: TrayModuleFlags,
    palette: FluentPalette,
) -> Result<Option<TrayAction>, String> {
    let spec = build_tray_menu_spec(docks, flags);
    let chosen = show_fluent_menu(hinstance, hwnd, screen_pt, &spec, palette)?;
    Ok(chosen.and_then(|cmd| decode_tray_command(cmd, docks)))
}
