use asrbox_platform_api::{DockMenuAction, DockMenuState, PixelPoint};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, PostMessageW, SetForegroundWindow, TrackPopupMenuEx, HMENU,
    TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_NULL,
};

use crate::tray::{add_item, add_sep, add_submenu};

const CMD_EDGE_LEFT: u32 = 101;
const CMD_EDGE_RIGHT: u32 = 102;
const CMD_EDGE_TOP: u32 = 103;
const CMD_EDGE_BOTTOM: u32 = 104;
const CMD_ALIGN_CENTER: u32 = 110;
const CMD_TOGGLE_LOCK: u32 = 113;
const CMD_OPACITY_100: u32 = 114;
const CMD_OPACITY_85: u32 = 115;
const CMD_OPACITY_70: u32 = 116;
const CMD_OPACITY_50: u32 = 117;
const CMD_OPACITY_35: u32 = 118;
const CMD_SIZE_SMALL: u32 = 120;
const CMD_SIZE_MEDIUM: u32 = 121;
const CMD_SIZE_LARGE: u32 = 122;
const CMD_TOGGLE_ZOOM: u32 = 130;
const CMD_TOGGLE_RUNNING: u32 = 131;
const CMD_TOGGLE_FOCUS_RUN: u32 = 132;
const CMD_CHOOSE_ICON: u32 = 140;
const CMD_RESET_ICON: u32 = 141;
const CMD_REMOVE_ITEM: u32 = 142;
const CMD_ADD_SEPARATOR: u32 = 145;
const CMD_TOGGLE_SETTINGS_BTN: u32 = 146;
const CMD_OPEN_DOCK_CFG: u32 = 147;
const CMD_RELOAD_DOCK_CFG: u32 = 148;
const CMD_OPEN_SETTINGS: u32 = 149;
const CMD_ADD_DOCK: u32 = 150;
const CMD_REMOVE_DOCK: u32 = 151;

pub(crate) fn show_dock_context_menu(
    hwnd: HWND,
    screen_pt: PixelPoint,
    state: &DockMenuState,
) -> Result<Option<DockMenuAction>, String> {
    // SAFETY: Popup menu and its submenus are created, tracked, and destroyed synchronously.
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
    add_item(menu, CMD_OPEN_SETTINGS, "Настройки asrbox...", false, true);
    add_item(menu, CMD_ADD_SEPARATOR, "Добавить разделитель", false, true);
    add_item(
        menu,
        CMD_TOGGLE_LOCK,
        "Закрепить панель",
        state.locked,
        true,
    );
    add_sep(menu);
    populate_dock_submenus(menu, state);
    add_sep(menu);
    add_item(menu, CMD_OPEN_DOCK_CFG, "Открыть docks.cfg", false, true);
    add_item(
        menu,
        CMD_RELOAD_DOCK_CFG,
        "Перечитать docks.cfg",
        false,
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

unsafe fn populate_dock_submenus(menu: HMENU, state: &DockMenuState) {
    if let Ok(edge_sub) = CreatePopupMenu() {
        add_item(
            edge_sub,
            CMD_EDGE_BOTTOM,
            "Снизу",
            state.edge_code == 3,
            true,
        );
        add_item(edge_sub, CMD_EDGE_TOP, "Сверху", state.edge_code == 2, true);
        add_item(edge_sub, CMD_EDGE_LEFT, "Слева", state.edge_code == 0, true);
        add_item(
            edge_sub,
            CMD_EDGE_RIGHT,
            "Справа",
            state.edge_code == 1,
            true,
        );
        add_sep(edge_sub);
        add_item(
            edge_sub,
            CMD_ALIGN_CENTER,
            "По центру",
            state.is_centered,
            true,
        );
        add_submenu(menu, edge_sub, "Положение на экране");
    }
    if let Ok(op_sub) = CreatePopupMenu() {
        let p = closest_opacity(state.opacity_percent);
        add_item(op_sub, CMD_OPACITY_100, "100% (плотный)", p == 100, true);
        add_item(op_sub, CMD_OPACITY_85, "85%", p == 85, true);
        add_item(op_sub, CMD_OPACITY_70, "70%", p == 70, true);
        add_item(op_sub, CMD_OPACITY_50, "50%", p == 50, true);
        add_item(op_sub, CMD_OPACITY_35, "35%", p == 35, true);
        add_submenu(menu, op_sub, "Прозрачность панели");
    }
    if let Ok(beh_sub) = CreatePopupMenu() {
        add_item(
            beh_sub,
            CMD_SIZE_SMALL,
            "Значки: 40 px",
            state.icon_size_px <= 42,
            true,
        );
        let md = (43..=56).contains(&state.icon_size_px);
        add_item(beh_sub, CMD_SIZE_MEDIUM, "Значки: 48 px", md, true);
        add_item(
            beh_sub,
            CMD_SIZE_LARGE,
            "Значки: 64 px",
            state.icon_size_px >= 57,
            true,
        );
        add_sep(beh_sub);
        add_item(
            beh_sub,
            CMD_TOGGLE_ZOOM,
            "Увеличение при наведении",
            state.hover_zoom_enabled,
            true,
        );
        add_item(
            beh_sub,
            CMD_TOGGLE_RUNNING,
            "Индикатор запущенных",
            state.show_running_indicator,
            true,
        );
        add_item(
            beh_sub,
            CMD_TOGGLE_FOCUS_RUN,
            "Активировать открытое окно",
            state.focus_running_window,
            true,
        );
        add_item(
            beh_sub,
            CMD_TOGGLE_SETTINGS_BTN,
            "Кнопка ⚙ на панели",
            state.show_settings_button,
            true,
        );
        add_submenu(menu, beh_sub, "Значки и поведение");
    }
}

fn decode_dock_command(cmd: u32, hovered: Option<u64>) -> Option<DockMenuAction> {
    match cmd {
        CMD_EDGE_LEFT => Some(DockMenuAction::SetEdgeLeft),
        CMD_EDGE_RIGHT => Some(DockMenuAction::SetEdgeRight),
        CMD_EDGE_TOP => Some(DockMenuAction::SetEdgeTop),
        CMD_EDGE_BOTTOM => Some(DockMenuAction::SetEdgeBottom),
        CMD_ALIGN_CENTER => Some(DockMenuAction::AlignCenter),
        CMD_TOGGLE_LOCK => Some(DockMenuAction::ToggleLock),
        CMD_OPACITY_100 => Some(DockMenuAction::SetOpacity(100)),
        CMD_OPACITY_85 => Some(DockMenuAction::SetOpacity(85)),
        CMD_OPACITY_70 => Some(DockMenuAction::SetOpacity(70)),
        CMD_OPACITY_50 => Some(DockMenuAction::SetOpacity(50)),
        CMD_OPACITY_35 => Some(DockMenuAction::SetOpacity(35)),
        CMD_SIZE_SMALL => Some(DockMenuAction::IconSizeSmall),
        CMD_SIZE_MEDIUM => Some(DockMenuAction::IconSizeMedium),
        CMD_SIZE_LARGE => Some(DockMenuAction::IconSizeLarge),
        CMD_TOGGLE_ZOOM => Some(DockMenuAction::ToggleHoverZoom),
        CMD_TOGGLE_RUNNING => Some(DockMenuAction::ToggleRunningIndicator),
        CMD_TOGGLE_FOCUS_RUN => Some(DockMenuAction::ToggleFocusRunning),
        CMD_CHOOSE_ICON => hovered.map(DockMenuAction::ChooseItemIcon),
        CMD_RESET_ICON => hovered.map(DockMenuAction::ResetItemIcon),
        CMD_REMOVE_ITEM => hovered.map(DockMenuAction::RemoveItem),
        CMD_ADD_SEPARATOR => Some(DockMenuAction::AddSeparator),
        CMD_TOGGLE_SETTINGS_BTN => Some(DockMenuAction::ToggleSettingsButton),
        CMD_OPEN_SETTINGS => Some(DockMenuAction::OpenSettings),
        CMD_OPEN_DOCK_CFG => Some(DockMenuAction::OpenDockConfig),
        CMD_RELOAD_DOCK_CFG => Some(DockMenuAction::ReloadDockConfig),
        CMD_ADD_DOCK => Some(DockMenuAction::AddNewDock),
        CMD_REMOVE_DOCK => Some(DockMenuAction::RemoveDock),
        _ => None,
    }
}

fn closest_opacity(pct: u8) -> u8 {
    match pct {
        93..=100 => 100,
        78..=92 => 85,
        60..=77 => 70,
        43..=59 => 50,
        _ => 35,
    }
}
