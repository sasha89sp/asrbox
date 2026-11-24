use asrbox_platform_api::WidgetLayer;

use crate::layout::{ButtonTone, LayoutBuilder};
use crate::layout_tabs::row;
use crate::types::{SettingsAction, SettingsSnapshot, SliderKind};

pub(crate) fn build_sysmon_tab(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let s = &snap.sysmon;
    b.set_header(
        "Системный монитор",
        "Windows 11 Fluent Design · регулируйте частоту опроса, чтобы снизить нагрузку на CPU",
    );

    let main_rows = vec![
        row(0, 50, "Показывать виджет", None, None, true),
        row(
            50,
            56,
            "Частота обновления",
            Some("Реже опрос — ниже нагрузка на процессор"),
            None,
            true,
        ),
        row(
            106,
            52,
            "Прозрачность фона",
            None,
            Some(format!("{}%", s.opacity_percent)),
            true,
        ),
        row(158, 52, "Слой окна", None, None, false),
    ];
    let cy = b.add_card(Some("ОТОБРАЖЕНИЕ И НАГРУЗКА"), false, main_rows);
    let tx = b.content_x + b.content_w as i32 - 60;
    let sx = b.content_x + b.content_w as i32 - 220;

    b.push_toggle(
        tx,
        cy + 13,
        s.visible,
        SettingsAction::SetSysMonitorVisible(!s.visible),
    );
    b.add_segmented_row(
        cy + 50,
        56,
        vec![
            (
                "1 с",
                s.refresh_interval_ms <= 1500,
                SettingsAction::SetSysMonitorRefreshMs(1000),
            ),
            (
                "2 с",
                (1501..=2500).contains(&s.refresh_interval_ms),
                SettingsAction::SetSysMonitorRefreshMs(2000),
            ),
            (
                "3 с",
                (2501..=4000).contains(&s.refresh_interval_ms),
                SettingsAction::SetSysMonitorRefreshMs(3000),
            ),
            (
                "5 с",
                (4001..=7500).contains(&s.refresh_interval_ms),
                SettingsAction::SetSysMonitorRefreshMs(5000),
            ),
            (
                "10 с",
                s.refresh_interval_ms > 7500,
                SettingsAction::SetSysMonitorRefreshMs(10_000),
            ),
        ],
    );
    b.push_slider(
        sx,
        cy + 120,
        SliderKind::SysMonitorOpacity,
        u32::from(s.opacity_percent),
    );
    b.add_segmented_row(
        cy + 158,
        52,
        vec![
            (
                "Обои",
                s.layer == WidgetLayer::Wallpaper,
                SettingsAction::SetSysMonitorLayer(WidgetLayer::Wallpaper),
            ),
            (
                "Обычный",
                s.layer == WidgetLayer::Normal,
                SettingsAction::SetSysMonitorLayer(WidgetLayer::Normal),
            ),
            (
                "Поверх",
                s.layer == WidgetLayer::Topmost,
                SettingsAction::SetSysMonitorLayer(WidgetLayer::Topmost),
            ),
        ],
    );

    let size_rows = vec![
        row(
            0,
            54,
            "Ширина окна",
            Some("Можно также тянуть края и углы самого виджета"),
            Some(format!("{} px", s.width)),
            true,
        ),
        row(
            54,
            54,
            "Высота окна",
            Some("Автоматически масштабирует блоки и список процессов"),
            Some(format!("{} px", s.height)),
            false,
        ),
    ];
    let sy = b.add_card(Some("РАЗМЕР И ВЫРАВНИВАНИЕ"), false, size_rows);
    b.push_slider(sx, sy + 15, SliderKind::SysMonitorWidth, s.width);
    b.push_slider(sx, sy + 69, SliderKind::SysMonitorHeight, s.height);

    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 206, 32),
        "Выровнять с Календарём",
        ButtonTone::Primary,
        SettingsAction::MatchSysMonitorToCalendar,
    );
    b.push_btn(
        (x + 216, y, 146, 32),
        "↻ Обновить",
        ButtonTone::Secondary,
        SettingsAction::RefreshSysMonitorNow,
    );
    b.push_btn(
        (x + 372, y, 166, 32),
        "Открыть sysmon.cfg",
        ButtonTone::Secondary,
        SettingsAction::OpenSysMonitorConfig,
    );
    b.cursor_y += 44;
}
