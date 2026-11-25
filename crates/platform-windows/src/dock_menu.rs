use asrbox_platform_api::{
    DockMenuAction, DockMenuState, FluentMenuItem, FluentMenuSpec, FluentPalette, FluentQuickBtn,
    PixelPoint, WidgetLayer,
};
use windows::Win32::Foundation::{HINSTANCE, HWND};

use crate::fluent_menu::show_fluent_menu;

const CMD_EDGE_LEFT: u32 = 101;
const CMD_EDGE_RIGHT: u32 = 102;
const CMD_EDGE_TOP: u32 = 103;
const CMD_EDGE_BOTTOM: u32 = 104;
const CMD_ALIGN_CENTER: u32 = 110;
const CMD_LAYER_WALLPAPER: u32 = 111;
const CMD_LAYER_NORMAL: u32 = 112;
const CMD_LAYER_TOPMOST: u32 = 119;
const CMD_TOGGLE_LOCK: u32 = 113;
const CMD_TOGGLE_LOCK_ITEMS: u32 = 164;
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
const CMD_TOGGLE_LAUNCH_ANIM: u32 = 133;
const CMD_OPEN_ITEM_FOLDER: u32 = 139;
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
const CMD_LABEL_ICONS: u32 = 160;
const CMD_LABEL_HOVER: u32 = 161;
const CMD_LABEL_ICON_TEXT: u32 = 162;
const CMD_LABEL_ONLY: u32 = 163;

pub(crate) fn show_dock_context_menu(
    hinstance: HINSTANCE,
    hwnd: HWND,
    screen_pt: PixelPoint,
    state: &DockMenuState,
    palette: FluentPalette,
) -> Result<Option<DockMenuAction>, String> {
    let spec = build_dock_menu_spec(state);
    let chosen = show_fluent_menu(hinstance, hwnd, screen_pt, &spec, palette)?;
    Ok(chosen.and_then(|cmd| decode_dock_command(cmd, state.hovered_item_id)))
}

fn build_dock_menu_spec(state: &DockMenuState) -> FluentMenuSpec {
    let vis = state.menu_vis;
    let mut quick_actions = vec![FluentQuickBtn {
        id: CMD_OPEN_SETTINGS,
        glyph: "⚙".to_owned(),
        enabled: true,
    }];
    if vis.show_add_separator {
        quick_actions.push(FluentQuickBtn {
            id: CMD_ADD_SEPARATOR,
            glyph: "|".to_owned(),
            enabled: true,
        });
    }
    if vis.show_lock_controls {
        quick_actions.push(FluentQuickBtn {
            id: CMD_TOGGLE_LOCK,
            glyph: if state.locked { "🔒" } else { "🔓" }.to_owned(),
            enabled: true,
        });
    }
    if vis.show_dock_management {
        quick_actions.push(FluentQuickBtn {
            id: CMD_ADD_DOCK,
            glyph: "+".to_owned(),
            enabled: true,
        });
    }

    let mut items = Vec::new();
    if vis.show_item_actions && state.hovered_item_id.is_some() {
        push_hovered_item_actions(&mut items, state.hovered_is_separator);
        items.push(FluentMenuItem::Separator);
    }
    items.push(act(
        CMD_OPEN_SETTINGS,
        "Настройки asrbox",
        Some("⚙"),
        false,
        true,
    ));
    if vis.show_add_separator {
        items.push(act(
            CMD_ADD_SEPARATOR,
            "Добавить разделитель",
            Some("|"),
            false,
            true,
        ));
    }
    if vis.show_lock_controls {
        items.push(act(
            CMD_TOGGLE_LOCK,
            "Закрепить панель",
            None,
            state.locked,
            true,
        ));
        items.push(act(
            CMD_TOGGLE_LOCK_ITEMS,
            "Закрепить значки",
            None,
            state.lock_items,
            true,
        ));
    }
    if vis.show_edge_and_layer || vis.show_appearance || vis.show_behavior {
        items.push(FluentMenuItem::Separator);
    }
    if vis.show_edge_and_layer {
        push_edge_and_layer_submenus(&mut items, state);
    }
    if vis.show_appearance {
        push_appearance_submenus(&mut items, state);
    }
    if vis.show_behavior {
        items.push(build_behavior_submenu(state));
    }
    if vis.show_dock_management {
        items.push(FluentMenuItem::Separator);
        items.push(act(
            CMD_OPEN_DOCK_CFG,
            "Открыть docks.cfg",
            Some("✎"),
            false,
            true,
        ));
        items.push(act(
            CMD_RELOAD_DOCK_CFG,
            "Перечитать docks.cfg",
            Some("↻"),
            false,
            true,
        ));
        items.push(FluentMenuItem::Separator);
        items.push(act(
            CMD_ADD_DOCK,
            "Добавить док-панель",
            Some("+"),
            false,
            true,
        ));
        items.push(act(
            CMD_REMOVE_DOCK,
            "Удалить эту панель",
            Some("✕"),
            false,
            state.can_remove_dock,
        ));
    }
    FluentMenuSpec {
        quick_actions,
        items,
        min_width: 244,
    }
}

fn push_hovered_item_actions(items: &mut Vec<FluentMenuItem>, is_separator: bool) {
    if is_separator {
        items.push(act(
            CMD_REMOVE_ITEM,
            "Удалить разделитель",
            Some("✕"),
            false,
            true,
        ));
    } else {
        items.push(act(
            CMD_OPEN_ITEM_FOLDER,
            "Открыть расположение файла",
            Some("↗"),
            false,
            true,
        ));
        items.push(act(
            CMD_CHOOSE_ICON,
            "Сменить иконку...",
            Some("✎"),
            false,
            true,
        ));
        items.push(act(
            CMD_RESET_ICON,
            "Авто-иконка Windows",
            Some("↻"),
            false,
            true,
        ));
        items.push(act(
            CMD_REMOVE_ITEM,
            "Удалить значок",
            Some("✕"),
            false,
            true,
        ));
    }
}

fn push_edge_and_layer_submenus(items: &mut Vec<FluentMenuItem>, state: &DockMenuState) {
    items.push(FluentMenuItem::Submenu {
        label: "Положение на экране".to_owned(),
        icon: Some("⊞".to_owned()),
        enabled: true,
        items: vec![
            act(CMD_EDGE_BOTTOM, "Снизу", None, state.edge_code == 3, true),
            act(CMD_EDGE_TOP, "Сверху", None, state.edge_code == 2, true),
            act(CMD_EDGE_LEFT, "Слева", None, state.edge_code == 0, true),
            act(CMD_EDGE_RIGHT, "Справа", None, state.edge_code == 1, true),
            FluentMenuItem::Separator,
            act(CMD_ALIGN_CENTER, "По центру", None, state.is_centered, true),
        ],
    });
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
                true,
            ),
            act(
                CMD_LAYER_NORMAL,
                "Обычное окно",
                None,
                state.layer == WidgetLayer::Normal,
                true,
            ),
            act(
                CMD_LAYER_TOPMOST,
                "Поверх всех окон",
                None,
                state.layer == WidgetLayer::Topmost,
                true,
            ),
        ],
    });
}

fn push_appearance_submenus(items: &mut Vec<FluentMenuItem>, state: &DockMenuState) {
    items.push(FluentMenuItem::Submenu {
        label: "Отображение имён".to_owned(),
        icon: Some("Aa".to_owned()),
        enabled: true,
        items: vec![
            act(
                CMD_LABEL_ICONS,
                "Только иконки",
                None,
                state.label_mode_code == 0,
                true,
            ),
            act(
                CMD_LABEL_HOVER,
                "При наведении",
                None,
                state.label_mode_code == 1,
                true,
            ),
            act(
                CMD_LABEL_ICON_TEXT,
                "Иконка + имя",
                None,
                state.label_mode_code == 2,
                true,
            ),
            act(
                CMD_LABEL_ONLY,
                "Только имена",
                None,
                state.label_mode_code == 3,
                true,
            ),
        ],
    });
    let p = closest_opacity(state.opacity_percent);
    items.push(FluentMenuItem::Submenu {
        label: "Прозрачность панели".to_owned(),
        icon: Some("◐".to_owned()),
        enabled: true,
        items: vec![
            act(CMD_OPACITY_100, "100% (плотный)", None, p == 100, true),
            act(CMD_OPACITY_85, "85%", None, p == 85, true),
            act(CMD_OPACITY_70, "70%", None, p == 70, true),
            act(CMD_OPACITY_50, "50%", None, p == 50, true),
            act(CMD_OPACITY_35, "35%", None, p == 35, true),
        ],
    });
}

fn build_behavior_submenu(state: &DockMenuState) -> FluentMenuItem {
    let md = (43..=56).contains(&state.icon_size_px);
    FluentMenuItem::Submenu {
        label: "Значки и поведение".to_owned(),
        icon: Some("✦".to_owned()),
        enabled: true,
        items: vec![
            act(
                CMD_SIZE_SMALL,
                "Значки: 40 px",
                None,
                state.icon_size_px <= 42,
                true,
            ),
            act(CMD_SIZE_MEDIUM, "Значки: 48 px", None, md, true),
            act(
                CMD_SIZE_LARGE,
                "Значки: 64 px",
                None,
                state.icon_size_px >= 57,
                true,
            ),
            FluentMenuItem::Separator,
            act(
                CMD_TOGGLE_ZOOM,
                "Увеличение при наведении",
                None,
                state.hover_zoom_enabled,
                true,
            ),
            act(
                CMD_TOGGLE_LAUNCH_ANIM,
                "Анимация запуска",
                None,
                state.launch_animation_enabled,
                true,
            ),
            act(
                CMD_TOGGLE_RUNNING,
                "Индикатор запущенных",
                None,
                state.show_running_indicator,
                true,
            ),
            act(
                CMD_TOGGLE_FOCUS_RUN,
                "Не запускать 2-ю копию",
                None,
                state.focus_running_window,
                true,
            ),
            act(
                CMD_TOGGLE_SETTINGS_BTN,
                "Кнопка ⚙ на панели",
                None,
                state.show_settings_button,
                true,
            ),
        ],
    }
}

fn act(id: u32, label: &str, icon: Option<&str>, checked: bool, enabled: bool) -> FluentMenuItem {
    FluentMenuItem::Action {
        id,
        label: label.to_owned(),
        icon: icon.map(ToOwned::to_owned),
        shortcut: None,
        checked,
        enabled,
    }
}

fn decode_dock_command(cmd: u32, hovered: Option<u64>) -> Option<DockMenuAction> {
    match cmd {
        CMD_EDGE_LEFT => Some(DockMenuAction::SetEdgeLeft),
        CMD_EDGE_RIGHT => Some(DockMenuAction::SetEdgeRight),
        CMD_EDGE_TOP => Some(DockMenuAction::SetEdgeTop),
        CMD_EDGE_BOTTOM => Some(DockMenuAction::SetEdgeBottom),
        CMD_ALIGN_CENTER => Some(DockMenuAction::AlignCenter),
        CMD_LAYER_WALLPAPER => Some(DockMenuAction::SetLayerWallpaper),
        CMD_LAYER_NORMAL => Some(DockMenuAction::SetLayerNormal),
        CMD_LAYER_TOPMOST => Some(DockMenuAction::SetLayerTopmost),
        CMD_LABEL_ICONS => Some(DockMenuAction::SetLabelMode(0)),
        CMD_LABEL_HOVER => Some(DockMenuAction::SetLabelMode(1)),
        CMD_LABEL_ICON_TEXT => Some(DockMenuAction::SetLabelMode(2)),
        CMD_LABEL_ONLY => Some(DockMenuAction::SetLabelMode(3)),
        CMD_TOGGLE_LOCK => Some(DockMenuAction::ToggleLock),
        CMD_TOGGLE_LOCK_ITEMS => Some(DockMenuAction::ToggleLockItems),
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
        CMD_TOGGLE_LAUNCH_ANIM => Some(DockMenuAction::ToggleLaunchAnimation),
        CMD_OPEN_ITEM_FOLDER => hovered.map(DockMenuAction::OpenItemFolder),
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
