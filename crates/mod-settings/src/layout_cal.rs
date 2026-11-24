use asrbox_platform_api::{PixelRect, WidgetLayer};

use crate::layout::{ButtonTone, CardRow, LayoutBuilder};
use crate::layout_tabs::row;
use crate::types::{CalendarSettingsView, SettingsAction, SettingsSnapshot, SliderKind};

pub(crate) fn build_calendar_body(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let cal = &snap.calendar;
    if cal.commented_calendar_hint {
        let ay = b.add_card(
            None,
            true,
            vec![row(
                0,
                64,
                "Найдена закомментированная ссылка (# calendar=...)",
                Some("Знак # в начале строки отключает календарь в widgets.cfg"),
                None,
                false,
            )],
        );
        b.push_btn(
            (b.content_x + b.content_w as i32 - 196, ay + 16, 180, 32),
            "Включить (убрать #)",
            ButtonTone::Amber,
            SettingsAction::UncommentCalendarSources,
        );
    }
    let add_y = b.add_card(
        Some("ИСТОЧНИКИ КАЛЕНДАРЯ (.ICS)"),
        false,
        vec![row(0, 54, "", None, None, false)],
    );
    let url_w = b.content_w.saturating_sub(156);
    b.out.input_rects.primary = Some(PixelRect {
        x: b.content_x + 12,
        y: b.sy(add_y + 12),
        width: url_w,
        height: 28,
    });
    b.push_btn(
        (b.content_x + 20 + url_w as i32, add_y + 11, 122, 30),
        "Добавить .ics",
        ButtonTone::Primary,
        SettingsAction::AddCalendarUrlFromInput,
    );
    if !cal.sources.is_empty() {
        let rows: Vec<CardRow> = cal
            .sources
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let short = truncate_url(&s.url, 46);
                row(
                    (i as i32) * 52,
                    52,
                    &s.id,
                    Some(&short),
                    None,
                    i + 1 < cal.sources.len(),
                )
            })
            .collect();
        let sy = b.add_card(None, false, rows);
        let bx = b.content_x + b.content_w as i32 - 94;
        for i in 0..cal.sources.len() {
            b.push_btn(
                (bx, sy + (i as i32) * 52 + 12, 78, 28),
                "Удалить",
                ButtonTone::Danger,
                SettingsAction::RemoveCalendarSource(i),
            );
        }
    }
    add_calendar_display_card(b, cal);
}

fn add_calendar_display_card(b: &mut LayoutBuilder, c: &CalendarSettingsView) {
    let rows = vec![
        row(0, 50, "Показывать виджет", None, None, true),
        row(
            50,
            52,
            "Прозрачность фона",
            None,
            Some(format!("{}%", c.opacity_percent)),
            true,
        ),
        row(102, 52, "Слой окна", None, None, true),
        row(154, 52, "Период событий", None, None, true),
        row(206, 52, "Синхронизация", None, None, true),
        row(
            258,
            52,
            "Ширина окна",
            Some("Можно тянуть края виджета на рабочем столе"),
            Some(format!("{} px", c.width)),
            true,
        ),
        row(
            310,
            52,
            "Видимых строк (высота)",
            None,
            Some(format!("{}", c.visible_rows)),
            false,
        ),
    ];
    let cy = b.add_card(Some("ОТОБРАЖЕНИЕ И РАЗМЕР"), false, rows);
    let tx = b.content_x + b.content_w as i32 - 60;
    let sx = b.content_x + b.content_w as i32 - 220;
    b.push_toggle(
        tx,
        cy + 13,
        c.visible,
        SettingsAction::SetCalendarVisible(!c.visible),
    );
    b.push_slider(
        sx,
        cy + 64,
        SliderKind::CalendarOpacity,
        u32::from(c.opacity_percent),
    );
    b.add_segmented_row(
        cy + 102,
        52,
        vec![
            (
                "Обои",
                c.layer == WidgetLayer::Wallpaper,
                SettingsAction::SetCalendarLayer(WidgetLayer::Wallpaper),
            ),
            (
                "Обычный",
                c.layer == WidgetLayer::Normal,
                SettingsAction::SetCalendarLayer(WidgetLayer::Normal),
            ),
            (
                "Поверх",
                c.layer == WidgetLayer::Topmost,
                SettingsAction::SetCalendarLayer(WidgetLayer::Topmost),
            ),
        ],
    );
    b.add_segmented_row(
        cy + 154,
        52,
        vec![
            (
                "Сегодня",
                c.period_index == 0,
                SettingsAction::SetCalendarPeriod(0),
            ),
            (
                "7 дн.",
                c.period_index == 1,
                SettingsAction::SetCalendarPeriod(1),
            ),
            (
                "30 дн.",
                c.period_index == 2,
                SettingsAction::SetCalendarPeriod(2),
            ),
            (
                "Все",
                c.period_index == 3,
                SettingsAction::SetCalendarPeriod(3),
            ),
        ],
    );
    let s = c.refresh_interval_sec;
    b.add_segmented_row(
        cy + 206,
        52,
        vec![
            ("5м", s == 300, SettingsAction::SetCalendarSyncInterval(300)),
            (
                "15м",
                s == 900,
                SettingsAction::SetCalendarSyncInterval(900),
            ),
            (
                "30м",
                s == 1800,
                SettingsAction::SetCalendarSyncInterval(1800),
            ),
            (
                "1ч",
                s == 3600,
                SettingsAction::SetCalendarSyncInterval(3600),
            ),
            (
                "3ч",
                s == 10800,
                SettingsAction::SetCalendarSyncInterval(10800),
            ),
        ],
    );
    b.push_slider(sx, cy + 272, SliderKind::CalendarWidth, c.width);
    b.push_slider(
        sx,
        cy + 324,
        SliderKind::CalendarRows,
        u32::from(c.visible_rows),
    );
    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 198, 32),
        "Выровнять с Монитором",
        ButtonTone::Primary,
        SettingsAction::MatchCalendarToSysMonitor,
    );
    b.push_btn(
        (x + 208, y, 146, 32),
        "↻ Обновить",
        ButtonTone::Secondary,
        SettingsAction::RefreshCalendarNow,
    );
    b.push_btn(
        (x + 364, y, 174, 32),
        "Открыть widgets.cfg",
        ButtonTone::Secondary,
        SettingsAction::OpenWidgetsConfig,
    );
    b.cursor_y += 44;
}

fn truncate_url(url: &str, max_chars: usize) -> String {
    let chars: Vec<char> = url.chars().collect();
    if chars.len() <= max_chars {
        url.to_owned()
    } else {
        let head: String = chars.iter().take(max_chars.saturating_sub(3)).collect();
        format!("{head}...")
    }
}
