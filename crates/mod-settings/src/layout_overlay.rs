use asrbox_core::ModuleId;

use crate::layout::{ButtonTone, LayoutBuilder};
use crate::layout_tabs::{add_module_process_card, row};
use crate::types::{OverlaySettingsView, SettingsAction, SettingsSnapshot, SliderKind};

pub(crate) fn build_overlay_tab(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let o = &snap.overlay;
    b.set_header(
        "Игровой оверлей",
        "Отдельный процесс · в игре клики проходят насквозь, без перехвата фокуса и фризов",
    );
    if !b.is_searching() {
        add_module_process_card(b, snap, ModuleId::Overlay);
    }
    add_overlay_mode_card(b, o);
    add_overlay_pos_card(b, o);
    add_overlay_size_card(b, o);

    if !b.is_searching() {
        let (x, y) = (b.content_x, b.cursor_y);
        b.push_btn(
            (x, y, 174, 30),
            "Открыть overlay.cfg",
            ButtonTone::Secondary,
            SettingsAction::OpenOverlayConfig,
        );
        b.cursor_y = y + 40;
    }
}

fn add_overlay_mode_card(b: &mut LayoutBuilder, o: &OverlaySettingsView) {
    let summary = if o.setup_mode {
        "Режим настройки активен"
    } else if o.auto_games_only {
        "Только в играх"
    } else {
        "Всегда активен"
    };
    if !b.accordion(
        "overlay_mode",
        "Режим и настройка оверлея",
        summary,
        "оверлей игра автозапуск fullscreen topmost настройка overlay game setup",
    ) {
        return;
    }
    let setup_sub = if o.setup_mode {
        "Оверлей открыт на экране: перетаскивайте его и тяните углы/края для изменения размера"
    } else {
        "Показать оверлей прямо сейчас, чтобы расставить его и подобрать размер до запуска игры"
    };
    let rows = vec![
        row(
            0,
            52,
            "Автозапуск только в играх",
            Some("Вне игры оверлей скрыт и не опрашивает датчики (0% нагрузки)"),
            None,
            true,
        ),
        row(
            52,
            54,
            "Поверх полноэкранных игр (FSO / Topmost)",
            Some("Без инжекта DLL, безопасно для античитов"),
            None,
            true,
        ),
        row(
            106,
            56,
            "Интерактивная настройка на экране",
            Some(setup_sub),
            None,
            false,
        ),
    ];
    let cy = b.add_card(None, o.setup_mode, rows);
    let tx = b.content_x + b.content_w as i32 - 58;
    b.push_toggle(
        tx,
        cy + 15,
        o.auto_games_only,
        SettingsAction::SetOverlayAutoGamesOnly(!o.auto_games_only),
    );
    b.push_toggle(
        tx,
        cy + 68,
        o.force_topmost_fullscreen,
        SettingsAction::SetOverlayForceTopmost(!o.force_topmost_fullscreen),
    );
    let (label, tone, btn_w) = if o.setup_mode {
        ("Готово (заблокировать клики)", ButtonTone::Amber, 226u32)
    } else {
        ("Настроить на экране", ButtonTone::Primary, 184u32)
    };
    let bx = b.content_x + b.content_w as i32 - btn_w as i32 - 14;
    b.push_btn(
        (bx, cy + 118, btn_w, 30),
        label,
        tone,
        SettingsAction::SetOverlaySetupMode(!o.setup_mode),
    );
}

fn add_overlay_pos_card(b: &mut LayoutBuilder, o: &OverlaySettingsView) {
    let anchor_name = match o.anchor_index {
        0 => "Верх-Л",
        1 => "Верх-П",
        2 => "Низ-Л",
        3 => "Низ-П",
        _ => "Своя позиция",
    };
    if !b.accordion(
        "overlay_pos",
        "Расположение на экране",
        anchor_name,
        "оверлей угол позиция расположение экран overlay position anchor",
    ) {
        return;
    }
    let pos_hint = if o.anchor_index == 4 {
        format!(
            "Произвольная точка: X={}, Y={} (перетащите в режиме настройки)",
            o.x, o.y
        )
    } else {
        "Углы привязаны к активному монитору; перетаскивание переводит в «Своя»".to_owned()
    };
    let cy = b.add_card(
        None,
        false,
        vec![row(0, 56, "Угол привязки", Some(&pos_hint), None, false)],
    );
    b.add_segmented_row(
        cy,
        56,
        vec![
            (
                "↖ Верх-Л",
                o.anchor_index == 0,
                SettingsAction::SetOverlayAnchor(0),
            ),
            (
                "↗ Верх-П",
                o.anchor_index == 1,
                SettingsAction::SetOverlayAnchor(1),
            ),
            (
                "↙ Низ-Л",
                o.anchor_index == 2,
                SettingsAction::SetOverlayAnchor(2),
            ),
            (
                "↘ Низ-П",
                o.anchor_index == 3,
                SettingsAction::SetOverlayAnchor(3),
            ),
            (
                "✥ Своя",
                o.anchor_index == 4,
                SettingsAction::SetOverlayAnchor(4),
            ),
        ],
    );
}

fn add_overlay_size_card(b: &mut LayoutBuilder, o: &OverlaySettingsView) {
    let summary = format!("{}×{} · {}%", o.width, o.height, o.opacity_percent);
    if !b.accordion(
        "overlay_size",
        "Прозрачность, размер и опрос",
        &summary,
        "оверлей прозрачность ширина высота размер частота fps overlay size opacity",
    ) {
        return;
    }
    let rows = vec![
        row(
            0,
            50,
            "Прозрачность фона",
            None,
            Some(format!("{}%", o.opacity_percent)),
            true,
        ),
        row(
            50,
            52,
            "Ширина оверлея",
            Some("Можно также тянуть края и углы в режиме настройки"),
            Some(format!("{} px", o.width)),
            true,
        ),
        row(
            102,
            52,
            "Высота оверлея",
            Some("Шрифты, бары и отступы масштабируются пропорционально"),
            Some(format!("{} px", o.height)),
            true,
        ),
        row(154, 50, "Пресеты размера", None, None, true),
        row(
            204,
            52,
            "Частота обновления датчиков",
            Some("Оптимально 1 с — плавный HUD без нагрузки на игру"),
            None,
            false,
        ),
    ];
    let cy = b.add_card(None, false, rows);
    let sx = b.content_x + b.content_w as i32 - 220;
    b.push_slider(
        sx,
        cy + 13,
        SliderKind::OverlayOpacity,
        u32::from(o.opacity_percent),
    );
    b.push_slider(sx, cy + 64, SliderKind::OverlayWidth, o.width);
    b.push_slider(sx, cy + 116, SliderKind::OverlayHeight, o.height);
    b.add_segmented_row(
        cy + 154,
        50,
        vec![
            (
                "Компакт",
                o.width <= 272 && o.height <= 162,
                SettingsAction::SetOverlaySizePreset(260, 154),
            ),
            (
                "Стандарт",
                (273..=350).contains(&o.width),
                SettingsAction::SetOverlaySizePreset(320, 188),
            ),
            (
                "Крупный",
                o.width > 350,
                SettingsAction::SetOverlaySizePreset(400, 236),
            ),
        ],
    );
    b.add_segmented_row(
        cy + 204,
        52,
        vec![
            (
                "0.5 с",
                o.refresh_interval_ms <= 650,
                SettingsAction::SetOverlayRefreshMs(500),
            ),
            (
                "1 с",
                (651..=1500).contains(&o.refresh_interval_ms),
                SettingsAction::SetOverlayRefreshMs(1000),
            ),
            (
                "2 с",
                (1501..=2500).contains(&o.refresh_interval_ms),
                SettingsAction::SetOverlayRefreshMs(2000),
            ),
            (
                "3 с",
                o.refresh_interval_ms > 2500,
                SettingsAction::SetOverlayRefreshMs(3000),
            ),
        ],
    );
}
