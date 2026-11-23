use std::mem;

use asrbox_platform_api::{DockMenuAction, DockMenuState, PixelPoint, TrayAction, TrayMenuEntry};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, LoadIconW, PostMessageW, SetForegroundWindow,
    TrackPopupMenuEx, HMENU, IDI_APPLICATION, MF_CHECKED, MF_GRAYED, MF_SEPARATOR, MF_STRING,
    TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_APP, WM_NULL,
};

use crate::shell::encode_wide_str;

pub(crate) const WM_TRAYICON: u32 = WM_APP + 10;

const CMD_EDGE_LEFT: u32 = 101;
const CMD_EDGE_RIGHT: u32 = 102;
const CMD_EDGE_TOP: u32 = 103;
const CMD_EDGE_BOTTOM: u32 = 104;
const CMD_ALIGN_CENTER: u32 = 110;
const CMD_OFFSET_MINUS: u32 = 111;
const CMD_OFFSET_PLUS: u32 = 112;
const CMD_SIZE_SMALL: u32 = 120;
const CMD_SIZE_MEDIUM: u32 = 121;
const CMD_SIZE_LARGE: u32 = 122;
const CMD_TOGGLE_ZOOM: u32 = 130;
const CMD_CHOOSE_ICON: u32 = 140;
const CMD_RESET_ICON: u32 = 141;
const CMD_REMOVE_ITEM: u32 = 142;
const CMD_ADD_SEPARATOR: u32 = 145;
const CMD_TOGGLE_SETTINGS_BTN: u32 = 146;
const CMD_OPEN_DOCK_CFG: u32 = 147;
const CMD_RELOAD_DOCK_CFG: u32 = 148;
const CMD_ADD_DOCK: u32 = 150;
const CMD_REMOVE_DOCK: u32 = 151;

const TRAY_ADD_DOCK: u32 = 1001;
const TRAY_EXIT: u32 = 1002;
const TRAY_TOGGLE_CALENDAR: u32 = 1003;
const TRAY_OPEN_WIDGET_CFG: u32 = 1004;
const TRAY_OPEN_DOCK_CFG: u32 = 1005;
const TRAY_OPEN_DEBUG: u32 = 1006;
const TRAY_OPEN_ABOUT: u32 = 1007;
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

pub(crate) fn show_dock_context_menu(
    hwnd: HWND,
    screen_pt: PixelPoint,
    state: &DockMenuState,
) -> Result<Option<DockMenuAction>, String> {
    // SAFETY: Popup menu is created, tracked, and destroyed synchronously on the UI thread.
    unsafe {
        let menu = CreatePopupMenu().map_err(|err| format!("CreatePopupMenu failed: {err}"))?;
        populate_dock_menu(menu, state);
        let _ = SetForegroundWindow(hwnd);
        let flags = (TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON).0;
        let raw = TrackPopupMenuEx(menu, flags, screen_pt.x, screen_pt.y, hwnd, None);
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        Ok(decode_dock_command(raw.0 as u32, state.hovered_item_id))
    }
}

unsafe fn populate_dock_menu(menu: HMENU, state: &DockMenuState) {
    if state.hovered_item_id.is_some() {
        if state.hovered_is_separator {
            add_item(menu, CMD_REMOVE_ITEM, "Удалить разделитель", false, true);
        } else {
            add_item(menu, CMD_CHOOSE_ICON, "Сменить иконку...", false, true);
            add_item(menu, CMD_RESET_ICON, "Авто-иконка Windows", false, true);
            add_item(menu, CMD_REMOVE_ITEM, "Удалить значок", false, true);
        }
        add_sep(menu);
    }
    add_item(menu, CMD_ADD_SEPARATOR, "Добавить разделитель", false, true);
    add_item(
        menu,
        CMD_TOGGLE_SETTINGS_BTN,
        "Кнопка настроек на панели",
        state.show_settings_button,
        true,
    );
    add_item(
        menu,
        CMD_OPEN_DOCK_CFG,
        "Открыть настройки (docks.cfg)",
        false,
        true,
    );
    add_item(
        menu,
        CMD_RELOAD_DOCK_CFG,
        "Перечитать настройки",
        false,
        true,
    );
    add_sep(menu);
    add_item(
        menu,
        CMD_EDGE_BOTTOM,
        "Край: Снизу",
        state.edge_code == 3,
        true,
    );
    add_item(
        menu,
        CMD_EDGE_TOP,
        "Край: Сверху",
        state.edge_code == 2,
        true,
    );
    add_item(
        menu,
        CMD_EDGE_LEFT,
        "Край: Слева",
        state.edge_code == 0,
        true,
    );
    add_item(
        menu,
        CMD_EDGE_RIGHT,
        "Край: Справа",
        state.edge_code == 1,
        true,
    );
    add_sep(menu);
    add_item(menu, CMD_ALIGN_CENTER, "По центру", state.is_centered, true);
    add_item(menu, CMD_OFFSET_MINUS, "Сдвинуть (-80 px)", false, true);
    add_item(menu, CMD_OFFSET_PLUS, "Сдвинуть (+80 px)", false, true);
    add_sep(menu);
    add_item(
        menu,
        CMD_SIZE_SMALL,
        "Значки: 40 px",
        state.icon_size_px <= 42,
        true,
    );
    let md = (43..=56).contains(&state.icon_size_px);
    add_item(menu, CMD_SIZE_MEDIUM, "Значки: 48 px", md, true);
    add_item(
        menu,
        CMD_SIZE_LARGE,
        "Значки: 64 px",
        state.icon_size_px >= 57,
        true,
    );
    add_item(
        menu,
        CMD_TOGGLE_ZOOM,
        "Увеличение иконки",
        state.hover_zoom_enabled,
        true,
    );
    add_sep(menu);
    add_item(menu, CMD_ADD_DOCK, "Добавить док-панель", false, true);
    add_item(
        menu,
        CMD_REMOVE_DOCK,
        "Удалить эту панель",
        false,
        state.can_remove_dock,
    );
}

fn decode_dock_command(cmd: u32, hovered: Option<u64>) -> Option<DockMenuAction> {
    match cmd {
        CMD_EDGE_LEFT => Some(DockMenuAction::SetEdgeLeft),
        CMD_EDGE_RIGHT => Some(DockMenuAction::SetEdgeRight),
        CMD_EDGE_TOP => Some(DockMenuAction::SetEdgeTop),
        CMD_EDGE_BOTTOM => Some(DockMenuAction::SetEdgeBottom),
        CMD_ALIGN_CENTER => Some(DockMenuAction::AlignCenter),
        CMD_OFFSET_MINUS => Some(DockMenuAction::OffsetMinus),
        CMD_OFFSET_PLUS => Some(DockMenuAction::OffsetPlus),
        CMD_SIZE_SMALL => Some(DockMenuAction::IconSizeSmall),
        CMD_SIZE_MEDIUM => Some(DockMenuAction::IconSizeMedium),
        CMD_SIZE_LARGE => Some(DockMenuAction::IconSizeLarge),
        CMD_TOGGLE_ZOOM => Some(DockMenuAction::ToggleHoverZoom),
        CMD_CHOOSE_ICON => hovered.map(DockMenuAction::ChooseItemIcon),
        CMD_RESET_ICON => hovered.map(DockMenuAction::ResetItemIcon),
        CMD_REMOVE_ITEM => hovered.map(DockMenuAction::RemoveItem),
        CMD_ADD_SEPARATOR => Some(DockMenuAction::AddSeparator),
        CMD_TOGGLE_SETTINGS_BTN => Some(DockMenuAction::ToggleSettingsButton),
        CMD_OPEN_DOCK_CFG => Some(DockMenuAction::OpenDockConfig),
        CMD_RELOAD_DOCK_CFG => Some(DockMenuAction::ReloadDockConfig),
        CMD_ADD_DOCK => Some(DockMenuAction::AddNewDock),
        CMD_REMOVE_DOCK => Some(DockMenuAction::RemoveDock),
        _ => None,
    }
}

pub(crate) fn show_tray_context_menu(
    hwnd: HWND,
    screen_pt: PixelPoint,
    docks: &[TrayMenuEntry],
    calendar_visible: bool,
) -> Result<Option<TrayAction>, String> {
    // SAFETY: Creating, tracking, and destroying the tray popup menu on the UI thread.
    unsafe {
        let menu = CreatePopupMenu().map_err(|err| format!("CreatePopupMenu failed: {err}"))?;
        add_item(menu, TRAY_ADD_DOCK, "Добавить док-панель", false, true);
        add_item(
            menu,
            TRAY_OPEN_DOCK_CFG,
            "Настройки док-панелей (docks.cfg)",
            false,
            true,
        );
        add_sep(menu);
        add_item(
            menu,
            TRAY_TOGGLE_CALENDAR,
            "Виджет: Google Календарь",
            calendar_visible,
            true,
        );
        add_item(
            menu,
            TRAY_OPEN_WIDGET_CFG,
            "Настройки виджетов (widgets.cfg)",
            false,
            true,
        );
        if !docks.is_empty() {
            add_sep(menu);
        }
        for (idx, dock) in docks.iter().take(50).enumerate() {
            let id = TRAY_TOGGLE_BASE + u32::try_from(idx).unwrap_or(0);
            add_item(
                menu,
                id,
                &format!("Показать: {}", dock.title),
                dock.enabled,
                true,
            );
        }
        if docks.len() > 1 {
            add_sep(menu);
            for (idx, dock) in docks.iter().take(50).enumerate() {
                let id = TRAY_REMOVE_BASE + u32::try_from(idx).unwrap_or(0);
                add_item(menu, id, &format!("Удалить: {}", dock.title), false, true);
            }
        }
        add_sep(menu);
        add_item(menu, TRAY_OPEN_DEBUG, "Диагностика и логи...", false, true);
        add_item(menu, TRAY_OPEN_ABOUT, "О программе...", false, true);
        add_sep(menu);
        add_item(menu, TRAY_EXIT, "Выход из asrbox", false, true);

        let _ = SetForegroundWindow(hwnd);
        let flags = (TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON).0;
        let raw = TrackPopupMenuEx(menu, flags, screen_pt.x, screen_pt.y, hwnd, None);
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        Ok(decode_tray_command(raw.0 as u32, docks))
    }
}

fn decode_tray_command(cmd: u32, docks: &[TrayMenuEntry]) -> Option<TrayAction> {
    match cmd {
        TRAY_ADD_DOCK => Some(TrayAction::AddDock),
        TRAY_OPEN_DOCK_CFG => Some(TrayAction::OpenDockConfig),
        TRAY_TOGGLE_CALENDAR => Some(TrayAction::ToggleCalendarWidget),
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
