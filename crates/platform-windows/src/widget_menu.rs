use asrbox_platform_api::{
    FluentMenuItem, FluentMenuSpec, FluentPalette, FluentQuickBtn, PixelPoint, WidgetLayer,
    WidgetMenuAction, WidgetMenuState,
};
use windows::Win32::Foundation::{HINSTANCE, HWND};

use crate::fluent_menu::show_fluent_menu;

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
const CMD_SYNC_5M: u32 = 440;
const CMD_SYNC_15M: u32 = 441;
const CMD_SYNC_30M: u32 = 442;
const CMD_SYNC_1H: u32 = 443;
const CMD_SYNC_3H: u32 = 444;

pub(crate) fn show_widget_context_menu(
    hinstance: HINSTANCE,
    hwnd: HWND,
    screen_pt: PixelPoint,
    state: &WidgetMenuState,
    palette: FluentPalette,
) -> Result<Option<WidgetMenuAction>, String> {
    let spec = build_widget_menu_spec(state);
    let chosen = show_fluent_menu(hinstance, hwnd, screen_pt, &spec, palette)?;
    Ok(chosen.and_then(decode_widget_cmd))
}

fn build_widget_menu_spec(state: &WidgetMenuState) -> FluentMenuSpec {
    let is_sysmon = state.refresh_ms.is_some();
    let active_opacity = closest_opacity_preset(state.opacity_percent);
    let quick_actions = vec![
        FluentQuickBtn {
            id: CMD_OPEN_SETTINGS,
            glyph: "⚙".to_owned(),
            enabled: true,
        },
        FluentQuickBtn {
            id: CMD_REFRESH,
            glyph: "↻".to_owned(),
            enabled: true,
        },
        FluentQuickBtn {
            id: CMD_OPEN_CFG,
            glyph: "✎".to_owned(),
            enabled: true,
        },
        FluentQuickBtn {
            id: CMD_HIDE,
            glyph: "✕".to_owned(),
            enabled: true,
        },
    ];

    let mut items = vec![
        act(
            CMD_OPEN_SETTINGS,
            if is_sysmon {
                "Настройки монитора"
            } else {
                "Настройки календаря"
            },
            Some("⚙"),
            false,
        ),
        act(
            CMD_REFRESH,
            if is_sysmon {
                "Обновить данные"
            } else {
                "Обновить календарь"
            },
            Some("↻"),
            false,
        ),
        act(
            CMD_OPEN_CFG,
            if is_sysmon {
                "Открыть sysmon.cfg"
            } else {
                "Открыть widgets.cfg"
            },
            Some("✎"),
            false,
        ),
    ];

    if let Some(ms) = state.refresh_ms {
        let active_ms = closest_rate_preset(ms);
        items.push(FluentMenuItem::Separator);
        items.push(FluentMenuItem::Submenu {
            label: "Частота обновления".to_owned(),
            icon: Some("⏱".to_owned()),
            enabled: true,
            items: vec![
                act(CMD_RATE_1S, "1 сек (живой)", None, active_ms == 1000),
                act(CMD_RATE_2S, "2 сек (баланс)", None, active_ms == 2000),
                act(CMD_RATE_3S, "3 сек (эконом)", None, active_ms == 3000),
                act(CMD_RATE_5S, "5 сек (тихий)", None, active_ms == 5000),
                act(
                    CMD_RATE_10S,
                    "10 сек (минимум CPU)",
                    None,
                    active_ms == 10_000,
                ),
            ],
        });
    }
    if let Some(sec) = state.sync_interval_sec {
        let active_s = closest_sync_preset(sec);
        items.push(FluentMenuItem::Separator);
        items.push(FluentMenuItem::Submenu {
            label: "Синхронизация".to_owned(),
            icon: Some("⏱".to_owned()),
            enabled: true,
            items: vec![
                act(CMD_SYNC_5M, "Каждые 5 мин", None, active_s == 300),
                act(CMD_SYNC_15M, "Каждые 15 мин", None, active_s == 900),
                act(CMD_SYNC_30M, "Каждые 30 мин", None, active_s == 1800),
                act(CMD_SYNC_1H, "Каждый час", None, active_s == 3600),
                act(CMD_SYNC_3H, "Каждые 3 часа", None, active_s == 10_800),
            ],
        });
    }

    items.push(FluentMenuItem::Separator);
    items.push(FluentMenuItem::Submenu {
        label: "Слой отображения".to_owned(),
        icon: Some("◫".to_owned()),
        enabled: true,
        items: vec![
            act(
                CMD_LAYER_WALLPAPER,
                "На рабочем столе",
                None,
                state.layer == WidgetLayer::Wallpaper,
            ),
            act(
                CMD_LAYER_NORMAL,
                "Обычное окно",
                None,
                state.layer == WidgetLayer::Normal,
            ),
            act(
                CMD_LAYER_TOPMOST,
                "Поверх всех окон",
                None,
                state.layer == WidgetLayer::Topmost,
            ),
        ],
    });
    items.push(FluentMenuItem::Submenu {
        label: "Прозрачность фона".to_owned(),
        icon: Some("◐".to_owned()),
        enabled: true,
        items: vec![
            act(
                CMD_OPACITY_100,
                "100% (плотный)",
                None,
                active_opacity == 100,
            ),
            act(CMD_OPACITY_85, "85%", None, active_opacity == 85),
            act(CMD_OPACITY_70, "70%", None, active_opacity == 70),
            act(CMD_OPACITY_50, "50%", None, active_opacity == 50),
            act(CMD_OPACITY_35, "35%", None, active_opacity == 35),
        ],
    });
    items.push(FluentMenuItem::Separator);
    items.push(act(CMD_HIDE, "Скрыть виджет", Some("✕"), false));

    FluentMenuSpec {
        quick_actions,
        items,
        min_width: 236,
    }
}

fn act(id: u32, label: &str, icon: Option<&str>, checked: bool) -> FluentMenuItem {
    FluentMenuItem::Action {
        id,
        label: label.to_owned(),
        icon: icon.map(ToOwned::to_owned),
        shortcut: None,
        checked,
        enabled: true,
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
        CMD_SYNC_5M => Some(WidgetMenuAction::SetSyncIntervalSec(300)),
        CMD_SYNC_15M => Some(WidgetMenuAction::SetSyncIntervalSec(900)),
        CMD_SYNC_30M => Some(WidgetMenuAction::SetSyncIntervalSec(1800)),
        CMD_SYNC_1H => Some(WidgetMenuAction::SetSyncIntervalSec(3600)),
        CMD_SYNC_3H => Some(WidgetMenuAction::SetSyncIntervalSec(10_800)),
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

fn closest_sync_preset(sec: u32) -> u32 {
    match sec {
        0..=600 => 300,
        601..=1350 => 900,
        1351..=2700 => 1800,
        2701..=7200 => 3600,
        _ => 10_800,
    }
}
