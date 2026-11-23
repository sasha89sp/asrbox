use std::mem;

use asrbox_platform_api::{PixelPoint, TrayAction, TrayMenuEntry, TrayModuleFlags};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, LoadIconW, PostMessageW, SetForegroundWindow,
    TrackPopupMenuEx, HMENU, IDI_APPLICATION, MF_CHECKED, MF_GRAYED, MF_POPUP, MF_SEPARATOR,
    MF_STRING, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_NULL,
};

use crate::shell::encode_wide_str;

pub(crate) const WM_TRAYICON: u32 = WM_APP + 10;

const TRAY_OPEN_SETTINGS: u32 = 1000;
const TRAY_ADD_DOCK: u32 = 1001;
const TRAY_EXIT: u32 = 1002;
const TRAY_TOGGLE_CALENDAR: u32 = 1003;
const TRAY_OPEN_WIDGET_CFG: u32 = 1004;
const TRAY_OPEN_DOCK_CFG: u32 = 1005;
const TRAY_OPEN_DEBUG: u32 = 1006;
const TRAY_OPEN_ABOUT: u32 = 1007;
const TRAY_TOGGLE_DOCK_MOD: u32 = 1008;
const TRAY_TOGGLE_WIDGETS_MOD: u32 = 1009;
const TRAY_REFRESH_CALENDAR: u32 = 1010;
const TRAY_TOGGLE_BASE: u32 = 2000;
const TRAY_REMOVE_BASE: u32 = 3000;

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
        // SAFETY: `hwnd` is a valid UI-thread window; IDI_APPLICATION is a system icon.
        unsafe {
            if let Ok(icon) = LoadIconW(None, IDI_APPLICATION) {
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
    hwnd: HWND,
    screen_pt: PixelPoint,
    docks: &[TrayMenuEntry],
    flags: TrayModuleFlags,
) -> Result<Option<TrayAction>, String> {
    // SAFETY: Creating, tracking, and destroying grouped tray popup menu on UI thread.
    unsafe {
        let menu = CreatePopupMenu().map_err(|err| format!("CreatePopupMenu failed: {err}"))?;
        add_item(menu, TRAY_OPEN_SETTINGS, "Настройки asrbox...", false, true);
        add_sep(menu);

        if let Ok(dock_sub) = CreatePopupMenu() {
            add_item(
                dock_sub,
                TRAY_TOGGLE_DOCK_MOD,
                "Модуль включён",
                flags.dock_module_enabled,
                true,
            );
            add_sep(dock_sub);
            add_item(
                dock_sub,
                TRAY_ADD_DOCK,
                "Добавить док-панель",
                false,
                flags.dock_module_enabled,
            );
            for (idx, dock) in docks.iter().take(50).enumerate() {
                let id = TRAY_TOGGLE_BASE + u32::try_from(idx).unwrap_or(0);
                add_item(
                    dock_sub,
                    id,
                    &format!("Показать: {}", dock.title),
                    dock.enabled,
                    flags.dock_module_enabled,
                );
            }
            if docks.len() > 1 {
                add_sep(dock_sub);
                for (idx, dock) in docks.iter().take(50).enumerate() {
                    let id = TRAY_REMOVE_BASE + u32::try_from(idx).unwrap_or(0);
                    add_item(
                        dock_sub,
                        id,
                        &format!("Удалить: {}", dock.title),
                        false,
                        flags.dock_module_enabled,
                    );
                }
            }
            add_sep(dock_sub);
            add_item(
                dock_sub,
                TRAY_OPEN_DOCK_CFG,
                "Открыть docks.cfg",
                false,
                true,
            );
            add_submenu(menu, dock_sub, "Модуль: Док-панели");
        }

        if let Ok(cal_sub) = CreatePopupMenu() {
            add_item(
                cal_sub,
                TRAY_TOGGLE_WIDGETS_MOD,
                "Модуль включён",
                flags.widgets_module_enabled,
                true,
            );
            add_sep(cal_sub);
            add_item(
                cal_sub,
                TRAY_TOGGLE_CALENDAR,
                "Показывать виджет",
                flags.calendar_visible,
                flags.widgets_module_enabled,
            );
            add_item(
                cal_sub,
                TRAY_REFRESH_CALENDAR,
                "Обновить календарь сейчас",
                false,
                flags.widgets_module_enabled,
            );
            add_sep(cal_sub);
            add_item(
                cal_sub,
                TRAY_OPEN_WIDGET_CFG,
                "Открыть widgets.cfg",
                false,
                true,
            );
            add_submenu(menu, cal_sub, "Модуль: Google Календарь");
        }

        add_sep(menu);
        add_item(
            menu,
            TRAY_OPEN_DEBUG,
            "Модуль: Диагностика и логи...",
            false,
            true,
        );
        add_item(menu, TRAY_OPEN_ABOUT, "Модуль: О программе...", false, true);
        add_sep(menu);
        add_item(menu, TRAY_EXIT, "Выход из asrbox", false, true);

        let _ = SetForegroundWindow(hwnd);
        let tflags = (TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON).0;
        let raw = TrackPopupMenuEx(menu, tflags, screen_pt.x, screen_pt.y, hwnd, None);
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        Ok(decode_tray_command(raw.0 as u32, docks))
    }
}

fn decode_tray_command(cmd: u32, docks: &[TrayMenuEntry]) -> Option<TrayAction> {
    match cmd {
        TRAY_OPEN_SETTINGS => Some(TrayAction::OpenSettings),
        TRAY_TOGGLE_DOCK_MOD => Some(TrayAction::ToggleDockModule),
        TRAY_ADD_DOCK => Some(TrayAction::AddDock),
        TRAY_OPEN_DOCK_CFG => Some(TrayAction::OpenDockConfig),
        TRAY_TOGGLE_WIDGETS_MOD => Some(TrayAction::ToggleWidgetsModule),
        TRAY_TOGGLE_CALENDAR => Some(TrayAction::ToggleCalendarWidget),
        TRAY_REFRESH_CALENDAR => Some(TrayAction::RefreshCalendarWidget),
        TRAY_OPEN_WIDGET_CFG => Some(TrayAction::OpenWidgetConfig),
        TRAY_OPEN_DEBUG => Some(TrayAction::OpenDebugWindow),
        TRAY_OPEN_ABOUT => Some(TrayAction::OpenAboutWindow),
        TRAY_EXIT => Some(TrayAction::Exit),
        c if (TRAY_TOGGLE_BASE..TRAY_REMOVE_BASE).contains(&c) => docks
            .get((c - TRAY_TOGGLE_BASE) as usize)
            .map(|d| TrayAction::ToggleDock(d.dock_id)),
        c if c >= TRAY_REMOVE_BASE => docks
            .get((c - TRAY_REMOVE_BASE) as usize)
            .map(|d| TrayAction::RemoveDock(d.dock_id)),
        _ => None,
    }
}

pub(crate) unsafe fn add_submenu(parent: HMENU, sub: HMENU, text: &str) {
    let wide = encode_wide_str(text);
    // SAFETY: Ownership of `sub` transfers to `parent`, which frees it on DestroyMenu(parent).
    let _ = AppendMenuW(parent, MF_POPUP, sub.0 as usize, PCWSTR(wide.as_ptr()));
}

pub(crate) unsafe fn add_item(menu: HMENU, id: u32, text: &str, checked: bool, enabled: bool) {
    let mut flags = MF_STRING;
    if checked {
        flags |= MF_CHECKED;
    }
    if !enabled {
        flags |= MF_GRAYED;
    }
    let wide = encode_wide_str(text);
    // SAFETY: `wide` is NUL-terminated UTF-16 valid for the synchronous AppendMenuW call.
    let _ = AppendMenuW(menu, flags, id as usize, PCWSTR(wide.as_ptr()));
}

pub(crate) unsafe fn add_sep(menu: HMENU) {
    // SAFETY: Appending a separator does not dereference the string pointer.
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
}
