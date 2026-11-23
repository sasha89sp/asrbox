use std::ffi::OsString;
use std::mem;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

use asrbox_platform_api::{DockMenuAction, DockMenuState, PixelPoint, TrayAction, TrayMenuEntry};
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, ShellExecuteW,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, HMENU, IDI_APPLICATION, LoadIconW, MF_CHECKED,
    MF_GRAYED, MF_SEPARATOR, MF_STRING, SW_SHOWNORMAL, SetForegroundWindow, TPM_NONOTIFY,
    TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenuEx, WM_APP,
};

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
const CMD_ADD_DOCK: u32 = 150;
const CMD_REMOVE_DOCK: u32 = 151;

const TRAY_ADD_DOCK: u32 = 1001;
const TRAY_EXIT: u32 = 1002;
const TRAY_TOGGLE_CALENDAR: u32 = 1003;
const TRAY_OPEN_WIDGET_CFG: u32 = 1004;
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
        let _ = DestroyMenu(menu);
        Ok(decode_dock_command(raw.0 as u32, state.hovered_item_id))
    }
}

unsafe fn populate_dock_menu(menu: HMENU, state: &DockMenuState) {
    if state.hovered_item_id.is_some() {
        add_item(menu, CMD_CHOOSE_ICON, "Сменить иконку...", false, true);
        add_item(menu, CMD_RESET_ICON, "Авто-иконка Windows", false, true);
        add_item(menu, CMD_REMOVE_ITEM, "Удалить значок", false, true);
        add_sep(menu);
    }
    let bot = state.edge_code == 3;
    let top = state.edge_code == 2;
    let left = state.edge_code == 0;
    let right = state.edge_code == 1;
    add_item(menu, CMD_EDGE_BOTTOM, "Край: Снизу", bot, true);
    add_item(menu, CMD_EDGE_TOP, "Край: Сверху", top, true);
    add_item(menu, CMD_EDGE_LEFT, "Край: Слева", left, true);
    add_item(menu, CMD_EDGE_RIGHT, "Край: Справа", right, true);
    add_sep(menu);
    let centered = state.is_centered;
    add_item(menu, CMD_ALIGN_CENTER, "По центру", centered, true);
    add_item(menu, CMD_OFFSET_MINUS, "Сдвинуть (-80 px)", false, true);
    add_item(menu, CMD_OFFSET_PLUS, "Сдвинуть (+80 px)", false, true);
    add_sep(menu);
    let sm = state.icon_size_px <= 42;
    let md = (43..=56).contains(&state.icon_size_px);
    let lg = state.icon_size_px >= 57;
    add_item(menu, CMD_SIZE_SMALL, "Значки: 40 px", sm, true);
    add_item(menu, CMD_SIZE_MEDIUM, "Значки: 48 px", md, true);
    add_item(menu, CMD_SIZE_LARGE, "Значки: 64 px", lg, true);
    let zoom = state.hover_zoom_enabled;
    add_item(menu, CMD_TOGGLE_ZOOM, "Увеличение иконки", zoom, true);
    add_sep(menu);
    let can_del = state.can_remove_dock;
    add_item(menu, CMD_ADD_DOCK, "Добавить док-панель", false, true);
    add_item(menu, CMD_REMOVE_DOCK, "Удалить эту панель", false, can_del);
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
            let toggle_id = TRAY_TOGGLE_BASE + u32::try_from(idx).unwrap_or(0);
            let label = format!("Показать: {}", dock.title);
            add_item(menu, toggle_id, &label, dock.enabled, true);
        }
        if docks.len() > 1 {
            add_sep(menu);
            for (idx, dock) in docks.iter().take(50).enumerate() {
                let remove_id = TRAY_REMOVE_BASE + u32::try_from(idx).unwrap_or(0);
                let label = format!("Удалить: {}", dock.title);
                add_item(menu, remove_id, &label, false, true);
            }
        }
        add_sep(menu);
        add_item(menu, TRAY_EXIT, "Выход из asrbox", false, true);

        let _ = SetForegroundWindow(hwnd);
        let flags = (TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON).0;
        let raw = TrackPopupMenuEx(menu, flags, screen_pt.x, screen_pt.y, hwnd, None);
        let _ = DestroyMenu(menu);
        Ok(decode_tray_command(raw.0 as u32, docks))
    }
}

fn decode_tray_command(cmd: u32, docks: &[TrayMenuEntry]) -> Option<TrayAction> {
    if cmd == TRAY_ADD_DOCK {
        return Some(TrayAction::AddDock);
    }
    if cmd == TRAY_TOGGLE_CALENDAR {
        return Some(TrayAction::ToggleCalendarWidget);
    }
    if cmd == TRAY_OPEN_WIDGET_CFG {
        return Some(TrayAction::OpenWidgetConfig);
    }
    if cmd == TRAY_EXIT {
        return Some(TrayAction::Exit);
    }
    if (TRAY_TOGGLE_BASE..TRAY_REMOVE_BASE).contains(&cmd) {
        let idx = (cmd - TRAY_TOGGLE_BASE) as usize;
        return docks.get(idx).map(|d| TrayAction::ToggleDock(d.dock_id));
    }
    if cmd >= TRAY_REMOVE_BASE {
        let idx = (cmd - TRAY_REMOVE_BASE) as usize;
        return docks.get(idx).map(|d| TrayAction::RemoveDock(d.dock_id));
    }
    None
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

pub(crate) fn pick_icon_dialog(owner: HWND) -> Result<Option<PathBuf>, String> {
    let filter = encode_wide_str(concat!(
        "Иконки (*.ico;*.png;*.exe;*.dll)\0",
        "*.ico;*.png;*.exe;*.dll\0",
        "Все файлы (*.*)\0*.*\0",
    ));
    let mut file_buf = vec![0u16; 1024];
    let mut ofn = OPENFILENAMEW {
        lStructSize: u32::try_from(mem::size_of::<OPENFILENAMEW>()).unwrap_or(0),
        hwndOwner: owner,
        lpstrFilter: PCWSTR(filter.as_ptr()),
        lpstrFile: PWSTR(file_buf.as_mut_ptr()),
        nMaxFile: u32::try_from(file_buf.len()).unwrap_or(1024),
        Flags: OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR,
        ..Default::default()
    };

    // SAFETY: `filter` and `file_buf` stay valid during the modal GetOpenFileNameW call.
    let accepted = unsafe { GetOpenFileNameW(&mut ofn).as_bool() };
    if !accepted {
        return Ok(None);
    }
    let len = file_buf
        .iter()
        .position(|&ch| ch == 0)
        .unwrap_or(file_buf.len());
    if len == 0 {
        return Ok(None);
    }
    Ok(Some(PathBuf::from(OsString::from_wide(&file_buf[..len]))))
}

pub(crate) fn launch_path(path: &Path) -> Result<(), String> {
    let verb = encode_wide_str("open");
    let target: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    shell_open(&verb, &target)
}

pub(crate) fn open_url_str(url: &str) -> Result<(), String> {
    let verb = encode_wide_str("open");
    let target = encode_wide_str(url);
    shell_open(&verb, &target)
}

fn shell_open(verb: &[u16], target: &[u16]) -> Result<(), String> {
    // SAFETY: `verb` and `target` are NUL-terminated UTF-16 strings valid for ShellExecuteW.
    let res = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    if (res.0 as usize) <= 32 {
        return Err(format!("ShellExecuteW failed with code {}", res.0 as usize));
    }
    Ok(())
}

fn encode_wide_str(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
