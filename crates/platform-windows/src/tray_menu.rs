use asrbox_platform_api::{
    FluentMenuItem, FluentMenuSpec, FluentQuickBtn, TrayAction, TrayMenuEntry, TrayModuleFlags,
};

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
const TRAY_TOGGLE_SYSMON: u32 = 1011;
const TRAY_REFRESH_SYSMON: u32 = 1012;
const TRAY_OPEN_SYSMON_CFG: u32 = 1013;
const TRAY_TOGGLE_AUTOSTART: u32 = 1014;
const TRAY_BACKUP_SETTINGS: u32 = 1015;
const TRAY_BACKUP_FULL: u32 = 1016;
const TRAY_OPEN_BACKUP_TAB: u32 = 1017;
const TRAY_TOGGLE_OVERLAY_MOD: u32 = 1018;
const TRAY_OVERLAY_SETUP: u32 = 1019;
const TRAY_OPEN_OVERLAY_TAB: u32 = 1020;
const TRAY_OPEN_OVERLAY_CFG: u32 = 1021;
const TRAY_TOGGLE_WEATHER: u32 = 1022;
const TRAY_REFRESH_WEATHER: u32 = 1023;
const TRAY_TOGGLE_NOTES: u32 = 1024;
const TRAY_TOGGLE_BASE: u32 = 2000;
const TRAY_REMOVE_BASE: u32 = 3000;

pub(crate) fn build_tray_menu_spec(
    docks: &[TrayMenuEntry],
    flags: TrayModuleFlags,
) -> FluentMenuSpec {
    let vis = flags.menu_vis;
    let mut quick_actions = vec![FluentQuickBtn {
        id: TRAY_OPEN_SETTINGS,
        glyph: "⚙".to_owned(),
        enabled: true,
    }];
    if vis.show_widgets {
        quick_actions.push(FluentQuickBtn {
            id: TRAY_TOGGLE_SYSMON,
            glyph: "◫".to_owned(),
            enabled: flags.widgets_module_enabled,
        });
    }
    if vis.show_overlay {
        quick_actions.push(FluentQuickBtn {
            id: TRAY_OVERLAY_SETUP,
            glyph: "◎".to_owned(),
            enabled: flags.overlay_module_enabled,
        });
    }
    quick_actions.push(FluentQuickBtn {
        id: TRAY_EXIT,
        glyph: "✕".to_owned(),
        enabled: true,
    });

    let mut items = vec![act(
        TRAY_OPEN_SETTINGS,
        "Настройки asrbox",
        Some("⚙"),
        Some("Ctrl+,"),
        false,
        true,
    )];

    let any_mod = vis.show_docks || vis.show_widgets || vis.show_overlay || vis.show_backup;
    if any_mod {
        items.push(FluentMenuItem::Separator);
    }
    if vis.show_docks {
        items.push(FluentMenuItem::Submenu {
            label: "Док-панели".to_owned(),
            icon: Some("▤".to_owned()),
            enabled: true,
            items: build_dock_submenu(docks, flags),
        });
    }
    if vis.show_widgets {
        items.push(FluentMenuItem::Submenu {
            label: "Виджеты".to_owned(),
            icon: Some("◫".to_owned()),
            enabled: true,
            items: build_widgets_submenu(flags),
        });
    }
    if vis.show_overlay {
        items.push(FluentMenuItem::Submenu {
            label: "Игровой оверлей".to_owned(),
            icon: Some("◈".to_owned()),
            enabled: true,
            items: build_overlay_submenu(flags),
        });
    }
    if vis.show_backup {
        items.push(FluentMenuItem::Submenu {
            label: "Бэкап и облако".to_owned(),
            icon: Some("☁".to_owned()),
            enabled: true,
            items: build_backup_submenu(),
        });
    }

    if vis.show_autostart || vis.show_debug || vis.show_about {
        items.push(FluentMenuItem::Separator);
    }
    if vis.show_autostart {
        items.push(act(
            TRAY_TOGGLE_AUTOSTART,
            "Автозагрузка с Windows",
            Some("↻"),
            None,
            flags.autostart_enabled,
            true,
        ));
    }
    if vis.show_debug {
        items.push(act(
            TRAY_OPEN_DEBUG,
            "Диагностика и логи",
            Some("◉"),
            None,
            false,
            true,
        ));
    }
    if vis.show_about {
        items.push(act(
            TRAY_OPEN_ABOUT,
            "История версий",
            Some("ⓘ"),
            None,
            false,
            true,
        ));
    }
    items.push(FluentMenuItem::Separator);
    items.push(act(
        TRAY_EXIT,
        "Выход из asrbox",
        Some("✕"),
        Some("Alt+Q"),
        false,
        true,
    ));

    FluentMenuSpec {
        quick_actions,
        items,
        min_width: 244,
    }
}

fn build_dock_submenu(docks: &[TrayMenuEntry], flags: TrayModuleFlags) -> Vec<FluentMenuItem> {
    let mut sub = vec![
        act(
            TRAY_TOGGLE_DOCK_MOD,
            "Модуль включён",
            None,
            None,
            flags.dock_module_enabled,
            true,
        ),
        FluentMenuItem::Separator,
        act(
            TRAY_ADD_DOCK,
            "Добавить док-панель",
            Some("+"),
            None,
            false,
            flags.dock_module_enabled,
        ),
    ];
    for (idx, dock) in docks.iter().take(20).enumerate() {
        let id = TRAY_TOGGLE_BASE + u32::try_from(idx).unwrap_or(0);
        sub.push(act(
            id,
            &format!("Показать: {}", dock.title),
            None,
            None,
            dock.enabled,
            flags.dock_module_enabled,
        ));
    }
    if docks.len() > 1 {
        sub.push(FluentMenuItem::Separator);
        for (idx, dock) in docks.iter().take(20).enumerate() {
            let id = TRAY_REMOVE_BASE + u32::try_from(idx).unwrap_or(0);
            sub.push(act(
                id,
                &format!("Удалить: {}", dock.title),
                Some("✕"),
                None,
                false,
                flags.dock_module_enabled,
            ));
        }
    }
    sub.push(FluentMenuItem::Separator);
    sub.push(act(
        TRAY_OPEN_DOCK_CFG,
        "Открыть docks.cfg",
        Some("✎"),
        None,
        false,
        true,
    ));
    sub
}

fn build_widgets_submenu(flags: TrayModuleFlags) -> Vec<FluentMenuItem> {
    let w_on = flags.widgets_module_enabled;
    vec![
        act(
            TRAY_TOGGLE_WIDGETS_MOD,
            "Модуль включён",
            None,
            None,
            w_on,
            true,
        ),
        FluentMenuItem::Separator,
        act(
            TRAY_TOGGLE_CALENDAR,
            "Календарь: показывать",
            None,
            None,
            flags.calendar_visible,
            w_on,
        ),
        act(
            TRAY_REFRESH_CALENDAR,
            "Календарь: обновить",
            Some("↻"),
            None,
            false,
            w_on,
        ),
        act(
            TRAY_OPEN_WIDGET_CFG,
            "Открыть widgets.cfg",
            Some("✎"),
            None,
            false,
            true,
        ),
        FluentMenuItem::Separator,
        act(
            TRAY_TOGGLE_SYSMON,
            "Системный монитор",
            None,
            None,
            flags.sys_monitor_visible,
            w_on,
        ),
        act(
            TRAY_REFRESH_SYSMON,
            "Монитор: обновить",
            Some("↻"),
            None,
            false,
            w_on,
        ),
        act(
            TRAY_OPEN_SYSMON_CFG,
            "Открыть sysmon.cfg",
            Some("✎"),
            None,
            false,
            true,
        ),
        FluentMenuItem::Separator,
        act(
            TRAY_TOGGLE_WEATHER,
            "Погода: показывать",
            None,
            None,
            flags.weather_visible,
            w_on,
        ),
        act(
            TRAY_REFRESH_WEATHER,
            "Погода: обновить",
            Some("↻"),
            None,
            false,
            w_on,
        ),
        FluentMenuItem::Separator,
        act(
            TRAY_TOGGLE_NOTES,
            "Заметки и задачи",
            None,
            None,
            flags.notes_visible,
            w_on,
        ),
    ]
}

fn build_overlay_submenu(flags: TrayModuleFlags) -> Vec<FluentMenuItem> {
    vec![
        act(
            TRAY_TOGGLE_OVERLAY_MOD,
            "Модуль включён",
            None,
            None,
            flags.overlay_module_enabled,
            true,
        ),
        FluentMenuItem::Separator,
        act(
            TRAY_OVERLAY_SETUP,
            "Режим настройки",
            None,
            None,
            flags.overlay_setup_mode,
            flags.overlay_module_enabled,
        ),
        act(
            TRAY_OPEN_OVERLAY_TAB,
            "Настройки оверлея...",
            Some("⚙"),
            None,
            false,
            true,
        ),
        act(
            TRAY_OPEN_OVERLAY_CFG,
            "Открыть overlay.cfg",
            Some("✎"),
            None,
            false,
            true,
        ),
    ]
}

fn build_backup_submenu() -> Vec<FluentMenuItem> {
    vec![
        act(
            TRAY_BACKUP_SETTINGS,
            "Бэкап настроек (.zip)",
            Some("↓"),
            None,
            false,
            true,
        ),
        act(
            TRAY_BACKUP_FULL,
            "Программа + настройки (.zip)",
            Some("↓"),
            None,
            false,
            true,
        ),
        FluentMenuItem::Separator,
        act(
            TRAY_OPEN_BACKUP_TAB,
            "Настройки бэкапа и WebDAV...",
            Some("☁"),
            None,
            false,
            true,
        ),
    ]
}

fn act(
    id: u32,
    label: &str,
    icon: Option<&str>,
    shortcut: Option<&str>,
    checked: bool,
    enabled: bool,
) -> FluentMenuItem {
    FluentMenuItem::Action {
        id,
        label: label.to_owned(),
        icon: icon.map(ToOwned::to_owned),
        shortcut: shortcut.map(ToOwned::to_owned),
        checked,
        enabled,
    }
}

pub(crate) fn decode_tray_command(cmd: u32, docks: &[TrayMenuEntry]) -> Option<TrayAction> {
    match cmd {
        TRAY_OPEN_SETTINGS => Some(TrayAction::OpenSettings),
        TRAY_TOGGLE_DOCK_MOD => Some(TrayAction::ToggleDockModule),
        TRAY_ADD_DOCK => Some(TrayAction::AddDock),
        TRAY_OPEN_DOCK_CFG => Some(TrayAction::OpenDockConfig),
        TRAY_TOGGLE_WIDGETS_MOD => Some(TrayAction::ToggleWidgetsModule),
        TRAY_TOGGLE_CALENDAR => Some(TrayAction::ToggleCalendarWidget),
        TRAY_REFRESH_CALENDAR => Some(TrayAction::RefreshCalendarWidget),
        TRAY_OPEN_WIDGET_CFG => Some(TrayAction::OpenWidgetConfig),
        TRAY_TOGGLE_SYSMON => Some(TrayAction::ToggleSysMonitorWidget),
        TRAY_REFRESH_SYSMON => Some(TrayAction::RefreshSysMonitorWidget),
        TRAY_OPEN_SYSMON_CFG => Some(TrayAction::OpenSysMonitorConfig),
        TRAY_TOGGLE_WEATHER => Some(TrayAction::ToggleWeatherWidget),
        TRAY_REFRESH_WEATHER => Some(TrayAction::RefreshWeatherWidget),
        TRAY_TOGGLE_NOTES => Some(TrayAction::ToggleNotesWidget),
        TRAY_BACKUP_SETTINGS => Some(TrayAction::BackupSettingsNow),
        TRAY_BACKUP_FULL => Some(TrayAction::BackupFullNow),
        TRAY_OPEN_BACKUP_TAB => Some(TrayAction::OpenBackupTab),
        TRAY_TOGGLE_OVERLAY_MOD => Some(TrayAction::ToggleOverlayModule),
        TRAY_OVERLAY_SETUP => Some(TrayAction::ToggleOverlaySetupMode),
        TRAY_OPEN_OVERLAY_TAB => Some(TrayAction::OpenOverlayTab),
        TRAY_OPEN_OVERLAY_CFG => Some(TrayAction::OpenOverlayConfig),
        TRAY_TOGGLE_AUTOSTART => Some(TrayAction::ToggleAutostart),
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
