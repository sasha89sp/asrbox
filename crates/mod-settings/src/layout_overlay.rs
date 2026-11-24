use asrbox_core::ModuleId;

use crate::layout::{ButtonTone, LayoutBuilder};
use crate::layout_tabs::row;
use crate::types::{OverlaySettingsView, SettingsAction, SettingsSnapshot, SliderKind};

pub(crate) fn build_overlay_tab(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let o = &snap.overlay;
    b.set_header(
        "Игровой оверлей",
        "Отдельный процесс · в игре клики проходят насквозь, без перехвата фокуса и фризов",
    );
    add_overlay_mode_card(b, o);
    add_overlay_pos_card(b, o);
    add_overlay_size_card(b, o);

    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 174, 32),
        "Открыть overlay.cfg",
        ButtonTone::Secondary,
        SettingsAction::OpenOverlayConfig,
    );
    b.cursor_y += 44;
}

fn add_overlay_mode_card(b: &mut LayoutBuilder, o: &OverlaySettingsView) {
    let setup_sub = if o.setup_mode {
        "Оверлей открыт на экране: перетаскивайте его и тяните углы/края для изменения размера"
    } else {
        "Показать оверлей прямо сейчас, чтобы расставить его и подобрать размер до запуска игры"
    };
    let rows = vec![
        row(
            0,
            50,
            "Модуль игрового оверлея",
            Some("Запускается отдельным изолированным процессом (asrbox --worker overlay)"),
            None,
            true,
        ),
        row(
            50,
            52,
            "Автозапуск только в играх",
            Some("Вне игры оверлей скрыт и не опрашивает датчики (0% нагрузки)"),
            None,
            true,
        ),
        row(
            102,
            54,
            "Поверх полноэкранных игр (FSO / Topmost)",
            Some("Окно без рамки и FSO Windows 10/11 (без инжекта DLL, безопасно для античитов)"),
            None,
            true,
        ),
        row(
            156,
            58,
            "Интерактивная настройка на экране",
            Some(setup_sub),
            None,
            false,
        ),
    ];
    let cy = b.add_card(Some("РЕЖИМ И НАСТРОЙКА"), o.setup_mode, rows);
    let tx = b.content_x + b.content_w as i32 - 60;
    b.push_toggle(
        tx,
        cy + 13,
        o.module_enabled,
        SettingsAction::SetModuleEnabled {
            id: ModuleId::Overlay,
            enabled: !o.module_enabled,
        },
    );
    b.push_toggle(
        tx,
        cy + 64,
        o.auto_games_only,
        SettingsAction::SetOverlayAutoGamesOnly(!o.auto_games_only),
    );
    b.push_toggle(
        tx,
        cy + 117,
        o.force_topmost_fullscreen,
        SettingsAction::SetOverlayForceTopmost(!o.force_topmost_fullscreen),
    );
    let (label, tone) = if o.setup_mode {
        ("Готово (заблокировать клики)", ButtonTone::Amber)
    } else {
        ("Настроить на экране", ButtonTone::Primary)
    };
    let btn_w = if o.setup_mode { 226 } else { 184 };
    let bx = b.content_x + b.content_w as i32 - btn_w as i32 - 16;
    b.push_btn(
        (bx, cy + 169, btn_w, 32),
        label,
        tone,
        SettingsAction::SetOverlaySetupMode(!o.setup_mode),
    );
}

fn add_overlay_pos_card(b: &mut LayoutBuilder, o: &OverlaySettingsView) {
    let pos_hint = if o.anchor_index == 4 {
        format!(
            "Произвольная точка: X={}, Y={} (перетащите в режиме настройки)",
            o.x, o.y
        )
    } else {
        "Углы привязаны к активному монитору; перетаскивание переводит в «Своя»".to_owned()
    };
    let rows = vec![row(
        0,
        58,
        "Расположение на экране",
        Some(&pos_hint),
        None,
        false,
    )];
    let cy = b.add_card(Some("РАСПОЛОЖЕНИЕ"), false, rows);
    b.add_segmented_row(
        cy,
        58,
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
    let rows = vec![
        row(
            0,
            52,
            "Прозрачность фона",
            None,
            Some(format!("{}%", o.opacity_percent)),
            true,
        ),
        row(
            52,
            54,
            "Ширина оверлея",
            Some("Можно также тянуть края и углы в режиме настройки"),
            Some(format!("{} px", o.width)),
            true,
        ),
        row(
            106,
            54,
            "Высота оверлея",
            Some("Шрифты, бары и отступы масштабируются пропорционально"),
            Some(format!("{} px", o.height)),
            true,
        ),
        row(160, 52, "Пресеты размера", None, None, true),
        row(
            212,
            54,
            "Частота обновления датчиков",
            Some("Оптимально 1 с — плавный HUD без нагрузки на игру"),
            None,
            false,
        ),
    ];
    let cy = b.add_card(Some("ПРОЗРАЧНОСТЬ, РАЗМЕР И ОПРОС"), false, rows);
    let sx = b.content_x + b.content_w as i32 - 220;
    b.push_slider(
        sx,
        cy + 14,
        SliderKind::OverlayOpacity,
        u32::from(o.opacity_percent),
    );
    b.push_slider(sx, cy + 67, SliderKind::OverlayWidth, o.width);
    b.push_slider(sx, cy + 121, SliderKind::OverlayHeight, o.height);
    b.add_segmented_row(
        cy + 160,
        52,
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
        cy + 212,
        54,
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
