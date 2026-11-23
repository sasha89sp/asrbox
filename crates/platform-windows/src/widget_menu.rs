use asrbox_platform_api::{PixelPoint, WidgetLayer, WidgetMenuAction, WidgetMenuState};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CreatePopupMenu, DestroyMenu, PostMessageW, SetForegroundWindow, TrackPopupMenuEx,
    TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON, WM_NULL,
};

use crate::tray::{add_item, add_sep, add_submenu};

const CMD_OPEN_SETTINGS: u32 = 400;
const CMD_REFRESH: u32 = 401;
const CMD_OPEN_CFG: u32 = 402;
const CMD_LAYER_WALLPAPER: u32 = 403;
const CMD_LAYER_NORMAL: u32 = 404;
const CMD_LAYER_TOPMOST: u32 = 405;
const CMD_OPACITY_100: u32 = 410;
const CMD_OPACITY_85: u32 = 411;
const CMD_OPACITY_70: u32 = 412;
const CMD_OPACITY_50: u32 = 413;
const CMD_OPACITY_35: u32 = 414;
const CMD_HIDE: u32 = 420;
const CMD_RATE_1S: u32 = 430;
const CMD_RATE_2S: u32 = 431;
const CMD_RATE_3S: u32 = 432;
const CMD_RATE_5S: u32 = 433;
const CMD_RATE_10S: u32 = 434;

pub(crate) fn show_widget_context_menu(
    hwnd: HWND,
    screen_pt: PixelPoint,
    state: &WidgetMenuState,
) -> Result<Option<WidgetMenuAction>, String> {
    let is_sysmon = state.refresh_ms.is_some();
    let active_opacity = closest_opacity_preset(state.opacity_percent);
    // SAFETY: Popup menu and optional submenus are created, tracked, and destroyed on UI thread.
    unsafe {
        let menu = CreatePopupMenu().map_err(|err| format!("CreatePopupMenu failed: {err}"))?;
        let settings_lbl = if is_sysmon {
            "Настройки монитора..."
        } else {
            "Настройки календаря..."
        };
        let refresh_lbl = if is_sysmon {
            "Обновить данные сейчас"
        } else {
            "Обновить календарь сейчас"
        };
        let cfg_lbl = if is_sysmon {
            "Открыть sysmon.cfg (Блокнот)"
        } else {
            "Открыть widgets.cfg (Блокнот)"
        };
        add_item(menu, CMD_OPEN_SETTINGS, settings_lbl, false, true);
        add_item(menu, CMD_REFRESH, refresh_lbl, false, true);
        add_item(menu, CMD_OPEN_CFG, cfg_lbl, false, true);

        if let Some(ms) = state.refresh_ms {
            add_sep(menu);
            if let Ok(rate_sub) = CreatePopupMenu() {
                let rates = [
                    (CMD_RATE_1S, 1000u32, "1 сек (живой)"),
                    (CMD_RATE_2S, 2000, "2 сек (баланс)"),
                    (CMD_RATE_3S, 3000, "3 сек (эконом)"),
                    (CMD_RATE_5S, 5000, "5 сек (тихий)"),
                    (CMD_RATE_10S, 10000, "10 сек (минимум CPU)"),
                ];
                let active_ms = closest_rate_preset(ms);
                for (cmd, val, label) in rates {
                    add_item(rate_sub, cmd, label, active_ms == val, true);
                }
                add_submenu(menu, rate_sub, "Частота обновления");
            }
        }

        add_sep(menu);
        add_item(
            menu,
            CMD_LAYER_WALLPAPER,
            "Слой: На обоях",
            state.layer == WidgetLayer::Wallpaper,
            true,
        );
        add_item(
            menu,
            CMD_LAYER_NORMAL,
            "Слой: Обычное окно",
            state.layer == WidgetLayer::Normal,
            true,
        );
        add_item(
            menu,
            CMD_LAYER_TOPMOST,
            "Слой: Поверх всех",
            state.layer == WidgetLayer::Topmost,
            true,
        );
        add_sep(menu);
        if let Ok(op_sub) = CreatePopupMenu() {
            let presets = [
                (CMD_OPACITY_100, 100u8, "100% (плотный)"),
                (CMD_OPACITY_85, 85, "85%"),
                (CMD_OPACITY_70, 70, "70%"),
                (CMD_OPACITY_50, 50, "50%"),
                (CMD_OPACITY_35, 35, "35%"),
            ];
            for (cmd, pct, label) in presets {
                add_item(op_sub, cmd, label, active_opacity == pct, true);
            }
            add_submenu(menu, op_sub, "Прозрачность фона");
        }
        add_sep(menu);
        add_item(menu, CMD_HIDE, "Скрыть виджет", false, true);

        let _ = SetForegroundWindow(hwnd);
        let flags = (TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON).0;
        let raw = TrackPopupMenuEx(menu, flags, screen_pt.x, screen_pt.y, hwnd, None);
        let _ = PostMessageW(Some(hwnd), WM_NULL, WPARAM(0), LPARAM(0));
        let _ = DestroyMenu(menu);
        Ok(decode_widget_cmd(raw.0 as u32))
    }
}

fn decode_widget_cmd(cmd: u32) -> Option<WidgetMenuAction> {
    match cmd {
        CMD_OPEN_SETTINGS => Some(WidgetMenuAction::OpenSettings),
        CMD_REFRESH => Some(WidgetMenuAction::Refresh),
        CMD_OPEN_CFG => Some(WidgetMenuAction::OpenConfig),
        CMD_LAYER_WALLPAPER => Some(WidgetMenuAction::SetLayerWallpaper),
        CMD_LAYER_NORMAL => Some(WidgetMenuAction::SetLayerNormal),
        CMD_LAYER_TOPMOST => Some(WidgetMenuAction::SetLayerTopmost),
        CMD_OPACITY_100 => Some(WidgetMenuAction::SetOpacity(100)),
        CMD_OPACITY_85 => Some(WidgetMenuAction::SetOpacity(85)),
        CMD_OPACITY_70 => Some(WidgetMenuAction::SetOpacity(70)),
        CMD_OPACITY_50 => Some(WidgetMenuAction::SetOpacity(50)),
        CMD_OPACITY_35 => Some(WidgetMenuAction::SetOpacity(35)),
        CMD_RATE_1S => Some(WidgetMenuAction::SetRefreshMs(1000)),
        CMD_RATE_2S => Some(WidgetMenuAction::SetRefreshMs(2000)),
        CMD_RATE_3S => Some(WidgetMenuAction::SetRefreshMs(3000)),
        CMD_RATE_5S => Some(WidgetMenuAction::SetRefreshMs(5000)),
        CMD_RATE_10S => Some(WidgetMenuAction::SetRefreshMs(10_000)),
        CMD_HIDE => Some(WidgetMenuAction::HideWidget),
        _ => None,
    }
}

fn closest_opacity_preset(pct: u8) -> u8 {
    match pct {
        93..=100 => 100,
        78..=92 => 85,
        60..=77 => 70,
        43..=59 => 50,
        _ => 35,
    }
}

fn closest_rate_preset(ms: u32) -> u32 {
    match ms {
        0..=1500 => 1000,
        1501..=2500 => 2000,
        2501..=4000 => 3000,
        4001..=7500 => 5000,
        _ => 10_000,
    }
}
