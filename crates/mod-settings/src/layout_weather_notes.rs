use asrbox_platform_api::{PixelRect, WidgetLayer};

use crate::i18n::tr;
use crate::layout::{ButtonTone, LayoutBuilder};
use crate::layout_tabs::row;
use crate::types::{ComboKind, SettingsAction, SettingsSnapshot, SliderKind};

pub(crate) fn build_weather_body(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let lang = b.lang;
    let w = &snap.weather;
    let summary = format!(
        "{} · {}",
        w.city,
        if w.visible {
            tr(lang, "Вкл", "On")
        } else {
            tr(lang, "Выкл", "Off")
        }
    );
    if !b.accordion(
        "weather_main",
        tr(
            lang,
            "Виджет «Погода» (Open-Meteo)",
            "Weather Widget (Open-Meteo)",
        ),
        &summary,
        "погода город open-meteo прогноз ветер влажность давление weather city forecast",
    ) {
        return;
    }
    let rows = vec![
        row(
            0,
            50,
            tr(lang, "Показывать виджет Погоды", "Show Weather widget"),
            Some(tr(
                lang,
                "Без API-ключей, обновляется через Open-Meteo",
                "Keyless forecast via Open-Meteo",
            )),
            None,
            true,
        ),
        row(
            50,
            54,
            tr(lang, "Город", "City"),
            Some(&format!("{}: {}", tr(lang, "Текущий", "Current"), w.city)),
            None,
            true,
        ),
        row(
            104,
            50,
            tr(lang, "Слой окна", "Window layer"),
            None,
            None,
            true,
        ),
        row(
            154,
            52,
            tr(lang, "Прозрачность фона", "Background opacity"),
            None,
            Some(format!("{}%", w.opacity_percent)),
            true,
        ),
        row(
            206,
            52,
            tr(lang, "Интервал обновления", "Refresh interval"),
            None,
            None,
            true,
        ),
        row(
            258,
            52,
            tr(lang, "Ширина карточки", "Card width"),
            None,
            Some(format!("{} px", w.width)),
            true,
        ),
        row(
            310,
            52,
            tr(lang, "Высота карточки", "Card height"),
            None,
            Some(format!("{} px", w.height)),
            false,
        ),
    ];
    let cy = b.add_card(None, false, rows);
    let tx = b.content_x + b.content_w as i32 - 58;
    b.push_toggle(
        tx,
        cy + 14,
        w.visible,
        SettingsAction::SetWeatherVisible(!w.visible),
    );

    let rx = b.content_x + b.content_w as i32 - 14;
    let apply_w = 114u32;
    let inp_w = 180u32;
    b.out.input_rects.aux = Some(PixelRect {
        x: rx - apply_w as i32 - 8 - inp_w as i32,
        y: b.sy(cy + 50 + 13),
        width: inp_w,
        height: 28,
    });
    b.push_btn(
        (rx - apply_w as i32, cy + 50 + 13, apply_w, 28),
        tr(lang, "Применить", "Apply"),
        ButtonTone::Primary,
        SettingsAction::ApplyWeatherCityFromInput,
    );

    b.push_combo(
        rx - 184,
        cy + 104 + 11,
        184,
        layer_label(w.layer),
        None,
        ComboKind::WeatherLayer,
    );

    let sx = b.content_x + b.content_w as i32 - 220;
    b.push_slider(
        sx,
        cy + 154 + 14,
        SliderKind::WeatherOpacity,
        u32::from(w.opacity_percent),
    );
    b.add_segmented_row(
        cy + 206,
        52,
        vec![
            (
                "15 мин",
                w.refresh_interval_min <= 15,
                SettingsAction::SetWeatherRefreshMin(15),
            ),
            (
                "30 мин",
                (16..=45).contains(&w.refresh_interval_min),
                SettingsAction::SetWeatherRefreshMin(30),
            ),
            (
                "60 мин",
                w.refresh_interval_min > 45,
                SettingsAction::SetWeatherRefreshMin(60),
            ),
        ],
    );
    b.push_slider(sx, cy + 258 + 14, SliderKind::WeatherWidth, w.width);
    b.push_slider(sx, cy + 310 + 14, SliderKind::WeatherHeight, w.height);

    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 166, 30),
        tr(lang, "Обновить сейчас", "Refresh Now"),
        ButtonTone::Primary,
        SettingsAction::RefreshWeatherNow,
    );
    b.push_btn(
        (x + 176, y, 164, 30),
        "Открыть weather.cfg",
        ButtonTone::Secondary,
        SettingsAction::OpenWeatherConfig,
    );
    b.cursor_y = y + 40;
}

pub(crate) fn build_notes_body(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let lang = b.lang;
    let n = &snap.notes;
    let summary = if n.visible {
        tr(lang, "Открыт", "Open")
    } else {
        tr(lang, "Скрыт", "Hidden")
    };
    if !b.accordion(
        "notes_main",
        tr(lang, "Виджет «Заметки и задачи»", "Notes & Tasks Widget"),
        summary,
        "заметки задачи папки чекбоксы редактор notes tasks folders checklist editor",
    ) {
        return;
    }
    let rows = vec![
        row(
            0,
            52,
            tr(lang, "Показывать окно Заметок", "Show Notes window"),
            Some(tr(
                lang,
                "3 колонки с переключением [◧][◫][▭] и Ctrl+1/2/3, оффлайн-база data/notes",
                "3 collapsible panes (Ctrl+1/2/3), offline DB in data/notes",
            )),
            None,
            true,
        ),
        row(
            52,
            50,
            tr(lang, "Слой окна", "Window layer"),
            None,
            None,
            true,
        ),
        row(
            102,
            52,
            tr(lang, "Прозрачность окна", "Window opacity"),
            None,
            Some(format!("{}%", n.opacity_percent)),
            true,
        ),
        row(
            154,
            48,
            tr(
                lang,
                "Колонка 1: Папки (Ctrl+1)",
                "Pane 1: Folders (Ctrl+1)",
            ),
            None,
            None,
            true,
        ),
        row(
            202,
            48,
            tr(
                lang,
                "Колонка 2: Список заметок (Ctrl+2)",
                "Pane 2: Notes List (Ctrl+2)",
            ),
            None,
            None,
            true,
        ),
        row(
            250,
            48,
            tr(
                lang,
                "Колонка 3: Редактор и задачи (Ctrl+3)",
                "Pane 3: Editor & Tasks (Ctrl+3)",
            ),
            None,
            None,
            false,
        ),
    ];
    let cy = b.add_card(None, false, rows);
    let tx = b.content_x + b.content_w as i32 - 58;
    let rx = b.content_x + b.content_w as i32 - 14;
    b.push_toggle(
        tx,
        cy + 15,
        n.visible,
        SettingsAction::SetNotesVisible(!n.visible),
    );
    b.push_combo(
        rx - 184,
        cy + 52 + 11,
        184,
        layer_label(n.layer),
        None,
        ComboKind::NotesLayer,
    );
    let sx = b.content_x + b.content_w as i32 - 220;
    b.push_slider(
        sx,
        cy + 102 + 14,
        SliderKind::NotesOpacity,
        u32::from(n.opacity_percent),
    );
    b.push_toggle(
        tx,
        cy + 154 + 13,
        n.show_folders_pane,
        SettingsAction::ToggleNotesPane(0),
    );
    b.push_toggle(
        tx,
        cy + 202 + 13,
        n.show_list_pane,
        SettingsAction::ToggleNotesPane(1),
    );
    b.push_toggle(
        tx,
        cy + 250 + 13,
        n.show_editor_pane,
        SettingsAction::ToggleNotesPane(2),
    );

    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 198, 30),
        tr(lang, "Открыть папку заметок", "Open Notes Folder"),
        ButtonTone::Secondary,
        SettingsAction::OpenNotesFolder,
    );
    b.cursor_y = y + 40;
}

pub(crate) fn layer_label(layer: WidgetLayer) -> &'static str {
    match layer {
        WidgetLayer::Wallpaper => "На рабочем столе",
        WidgetLayer::Normal => "Обычный",
        WidgetLayer::Topmost => "Поверх всех окон",
    }
}
