use asrbox_platform_api::{PixelRect, WidgetLayer};

use crate::i18n::tr;
use crate::layout::{ButtonTone, LayoutBuilder};
use crate::layout_tabs::row;
use crate::types::{ComboKind, SettingsAction, SettingsSnapshot, SliderKind};

pub(crate) fn build_weather_body(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let lang = b.lang;
    let w = &snap.weather;
    let prov = if w.provider_is_yandex {
        "Яндекс.Погода"
    } else {
        "Open-Meteo"
    };
    let summary = format!(
        "{} · {} · {}д · {}",
        w.city,
        prov,
        w.forecast_days,
        if w.visible {
            tr(lang, "Вкл", "On")
        } else {
            tr(lang, "Выкл", "Off")
        }
    );
    if !b.accordion(
        "weather_main",
        tr(lang, "Виджет «Погода»", "Weather Widget"),
        &summary,
        "погода город open-meteo яндекс yandex прогноз ветер влажность давление 1 3 7 дней weather city forecast",
    ) {
        return;
    }
    let key_sub = if w.has_yandex_key {
        tr(
            lang,
            "Ключ сохранён в Диспетчере учётных данных Windows (X-Yandex-Weather-Key)",
            "Saved in Windows Credential Manager (X-Yandex-Weather-Key)",
        )
    } else {
        tr(
            lang,
            "Введите X-Yandex-Weather-Key (хранится в Диспетчере учётных данных)",
            "Enter X-Yandex-Weather-Key (stored in Credential Manager)",
        )
    };
    let mut rows = Vec::new();
    let mut off = 0i32;
    let mut push_r =
        |h: u32, title: &str, sub: Option<&str>, val: Option<String>, sep: bool| -> i32 {
            let y = off;
            rows.push(row(y, h, title, sub, val, sep));
            off += h as i32;
            y
        };
    let y_vis = push_r(
        50,
        tr(lang, "Показывать виджет Погоды", "Show Weather widget"),
        Some(tr(
            lang,
            "Выбор источника: Open-Meteo (без ключа) или Яндекс.Погода",
            "Source: Open-Meteo (keyless) or Yandex Weather",
        )),
        None,
        true,
    );
    let y_prov = push_r(
        52,
        tr(lang, "Источник погоды", "Weather provider"),
        Some(tr(
            lang,
            "Координаты города определяются автоматически",
            "City coordinates are resolved automatically",
        )),
        None,
        true,
    );
    let y_key = if w.provider_is_yandex {
        Some(push_r(
            56,
            tr(lang, "API-ключ Яндекс.Погоды", "Yandex Weather API key"),
            Some(key_sub),
            None,
            true,
        ))
    } else {
        None
    };
    let y_city = push_r(
        54,
        tr(lang, "Город", "City"),
        Some(&format!("{}: {}", tr(lang, "Текущий", "Current"), w.city)),
        None,
        true,
    );
    let y_days = push_r(
        52,
        tr(lang, "Горизонт прогноза", "Forecast horizon"),
        Some(tr(
            lang,
            "Также можно переключать прямо в виджете (1д | 3д | 7д)",
            "Also switchable directly on the widget (1d | 3d | 7d)",
        )),
        None,
        true,
    );
    let y_layer = push_r(50, tr(lang, "Слой окна", "Window layer"), None, None, true);
    let y_opac = push_r(
        52,
        tr(lang, "Прозрачность фона", "Background opacity"),
        None,
        Some(format!("{}%", w.opacity_percent)),
        true,
    );
    let y_ref = push_r(
        52,
        tr(lang, "Интервал обновления", "Refresh interval"),
        None,
        None,
        true,
    );
    let y_w = push_r(
        52,
        tr(lang, "Ширина карточки", "Card width"),
        None,
        Some(format!("{} px", w.width)),
        true,
    );
    let y_h = push_r(
        52,
        tr(lang, "Высота карточки", "Card height"),
        None,
        Some(format!("{} px", w.height)),
        false,
    );

    let cy = b.add_card(None, false, rows);
    let tx = b.content_x + b.content_w as i32 - 58;
    let rx = b.content_x + b.content_w as i32 - 14;
    let sx = b.content_x + b.content_w as i32 - 220;

    b.push_toggle(
        tx,
        cy + y_vis + 14,
        w.visible,
        SettingsAction::SetWeatherVisible(!w.visible),
    );
    b.add_segmented_row(
        cy + y_prov,
        52,
        vec![
            (
                "Open-Meteo",
                !w.provider_is_yandex,
                SettingsAction::SetWeatherProvider(false),
            ),
            (
                "Яндекс.Погода",
                w.provider_is_yandex,
                SettingsAction::SetWeatherProvider(true),
            ),
        ],
    );
    if let Some(yk) = y_key {
        let clear_w = if w.has_yandex_key { 78u32 } else { 0 };
        let save_w = 104u32;
        let gap = if clear_w > 0 { 6 } else { 0 };
        let inp_w = 166u32;
        let clear_x = rx - clear_w as i32;
        let save_x = clear_x - gap - save_w as i32;
        let inp_x = save_x - 8 - inp_w as i32;
        b.out.input_rects.secret = Some(PixelRect {
            x: inp_x,
            y: b.sy(cy + yk + 14),
            width: inp_w,
            height: 28,
        });
        b.push_btn(
            (save_x, cy + yk + 14, save_w, 28),
            tr(lang, "Сохранить", "Save"),
            ButtonTone::Primary,
            SettingsAction::SaveYandexWeatherKey,
        );
        if clear_w > 0 {
            b.push_btn(
                (clear_x, cy + yk + 14, clear_w, 28),
                tr(lang, "Удалить", "Clear"),
                ButtonTone::Danger,
                SettingsAction::ClearYandexWeatherKey,
            );
        }
    }

    let apply_w = 114u32;
    let inp_w = 180u32;
    b.out.input_rects.primary = Some(PixelRect {
        x: rx - apply_w as i32 - 8 - inp_w as i32,
        y: b.sy(cy + y_city + 13),
        width: inp_w,
        height: 28,
    });
    b.push_btn(
        (rx - apply_w as i32, cy + y_city + 13, apply_w, 28),
        tr(lang, "Применить", "Apply"),
        ButtonTone::Primary,
        SettingsAction::ApplyWeatherCityFromInput,
    );
    b.add_segmented_row(
        cy + y_days,
        52,
        vec![
            (
                tr(lang, "1 день", "1 day"),
                w.forecast_days == 1,
                SettingsAction::SetWeatherForecastDays(1),
            ),
            (
                tr(lang, "3 дня", "3 days"),
                w.forecast_days == 3,
                SettingsAction::SetWeatherForecastDays(3),
            ),
            (
                tr(lang, "7 дней", "7 days"),
                w.forecast_days == 7,
                SettingsAction::SetWeatherForecastDays(7),
            ),
        ],
    );
    b.push_combo(
        rx - 184,
        cy + y_layer + 11,
        184,
        layer_label(w.layer),
        None,
        ComboKind::WeatherLayer,
    );
    b.push_slider(
        sx,
        cy + y_opac + 14,
        SliderKind::WeatherOpacity,
        u32::from(w.opacity_percent),
    );
    b.add_segmented_row(
        cy + y_ref,
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
    b.push_slider(sx, cy + y_w + 14, SliderKind::WeatherWidth, w.width);
    b.push_slider(sx, cy + y_h + 14, SliderKind::WeatherHeight, w.height);

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
