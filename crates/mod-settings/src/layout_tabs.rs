use asrbox_core::IsolationMode;
use asrbox_platform_api::PixelRect;

use crate::layout::{ButtonTone, CardRow, HitKind, LayoutBuilder};
pub(crate) use crate::layout_cal::build_calendar_tab;
use crate::types::{DockSettingsView, SettingsAction, SettingsSnapshot, SliderKind};

pub(crate) fn row(
    y: i32,
    h: u32,
    title: &str,
    sub: Option<&str>,
    val: Option<String>,
    div: bool,
) -> CardRow {
    CardRow {
        y,
        height: h,
        title: title.to_owned(),
        subtitle: sub.map(str::to_owned),
        value_label: val,
        divider: div,
    }
}

pub(crate) fn build_modules_tab(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    b.set_header(
        "Модули и изоляция процессов",
        "Отключённые модули не расходуют CPU/RAM; ошибка в одном модуле не роняет остальные",
    );
    let iso_sub = Some("Отдельный процесс для каждого модуля или общий");
    let iso_y = b.add_card(
        Some("РЕЖИМ ЗАПУСКА"),
        false,
        vec![row(0, 56, "Изоляция модулей", iso_sub, None, false)],
    );
    let is_proc = snap.isolation == IsolationMode::Process;
    b.add_segmented_row(
        iso_y,
        56,
        vec![
            (
                "Процессы",
                is_proc,
                SettingsAction::SetIsolation(IsolationMode::Process),
            ),
            (
                "Один процесс",
                !is_proc,
                SettingsAction::SetIsolation(IsolationMode::InProc),
            ),
        ],
    );
    let rows: Vec<CardRow> = snap
        .modules
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let st = match (m.enabled, m.running, m.pid) {
                (false, _, _) => "Отключён (0 МБ RAM)".to_owned(),
                (true, true, Some(p)) => format!("Работает · PID {p} · рестартов: {}", m.restarts),
                (true, true, None) => "Работает (inproc)".to_owned(),
                (true, false, _) => format!("Остановлен ({})", m.note),
            };
            row(
                (i as i32) * 62,
                62,
                m.id.display_name(),
                Some(&st),
                None,
                i + 1 < snap.modules.len(),
            )
        })
        .collect();
    let card_y = b.add_card(Some("АКТИВНЫЕ МОДУЛИ"), false, rows);
    let tx = b.content_x + b.content_w as i32 - 60;
    for (i, m) in snap.modules.iter().enumerate() {
        let ry = card_y + (i as i32) * 62;
        let act = SettingsAction::SetModuleEnabled {
            id: m.id,
            enabled: !m.enabled,
        };
        b.push_toggle(tx, ry + 19, m.enabled, act);
        if m.enabled {
            b.push_btn(
                (tx - 108, ry + 17, 96, 28),
                "Перезапуск",
                ButtonTone::Secondary,
                SettingsAction::RestartModule(m.id),
            );
        }
    }
}

pub(crate) fn build_dock_tab(b: &mut LayoutBuilder, snap: &SettingsSnapshot, selected_idx: usize) {
    b.set_header(
        "Док-панели",
        "Перетаскивайте значки и разделители между панелями, а за пределы панели — для удаления",
    );
    let sel_y = b.cursor_y;
    let mut px = b.content_x;
    for idx in 0..snap.docks.len() {
        let r = PixelRect {
            x: px,
            y: b.sy(sel_y),
            width: 92,
            height: 30,
        };
        b.push_control(r, HitKind::SelectDock(idx));
        px += 100;
    }
    b.push_btn(
        (px, sel_y, 112, 30),
        "+ Новый док",
        ButtonTone::Primary,
        SettingsAction::AddDock,
    );
    b.cursor_y = sel_y + 44;
    let Some(dock) = snap.docks.get(selected_idx).or_else(|| snap.docks.first()) else {
        return;
    };
    add_dock_appearance_card(b, dock);
    add_dock_behavior_card(b, dock, snap.docks.len() > 1);
}

fn add_dock_appearance_card(b: &mut LayoutBuilder, d: &DockSettingsView) {
    let rows = vec![
        row(0, 52, "Край экрана", None, None, true),
        row(
            52,
            56,
            "Размер иконок",
            Some("Произвольный размер 24–128 px"),
            Some(format!("{} px", d.icon_size_px)),
            true,
        ),
        row(
            108,
            56,
            "Прозрачность панели",
            Some("Видимость стеклянного фона"),
            Some(format!("{}%", d.opacity_percent)),
            false,
        ),
    ];
    let cy = b.add_card(Some("ВНЕШНИЙ ВИД"), false, rows);
    let mk = |edge_index| SettingsAction::SetDockEdge {
        dock_id: d.id.clone(),
        edge_index,
    };
    b.add_segmented_row(
        cy,
        52,
        vec![
            ("Низ", d.edge_index == 0, mk(0)),
            ("Верх", d.edge_index == 1, mk(1)),
            ("Лево", d.edge_index == 2, mk(2)),
            ("Право", d.edge_index == 3, mk(3)),
        ],
    );
    let sx = b.content_x + b.content_w as i32 - 220;
    b.push_slider(sx, cy + 68, SliderKind::DockIconSize, d.icon_size_px);
    b.push_slider(
        sx,
        cy + 124,
        SliderKind::DockOpacity,
        u32::from(d.opacity_percent),
    );
}

fn add_dock_behavior_card(b: &mut LayoutBuilder, d: &DockSettingsView, can_delete: bool) {
    let rows = vec![
        row(
            0,
            54,
            "Закрепить панель",
            Some("Отключите, чтобы перетащить панель за ручку"),
            None,
            true,
        ),
        row(
            54,
            54,
            "Индикатор запущенных программ",
            Some("Точка под открытыми приложениями"),
            None,
            true,
        ),
        row(
            108,
            54,
            "Активировать запущенное окно",
            Some("Клик поднимает открытое окно"),
            None,
            false,
        ),
    ];
    let cy = b.add_card(Some("ПОВЕДЕНИЕ"), false, rows);
    let tx = b.content_x + b.content_w as i32 - 60;
    let id = d.id.clone();
    b.push_toggle(
        tx,
        cy + 15,
        d.locked,
        SettingsAction::SetDockLocked {
            dock_id: id.clone(),
            locked: !d.locked,
        },
    );
    b.push_toggle(
        tx,
        cy + 69,
        d.show_running_indicator,
        SettingsAction::SetDockShowRunning {
            dock_id: id.clone(),
            enabled: !d.show_running_indicator,
        },
    );
    b.push_toggle(
        tx,
        cy + 123,
        d.focus_running_window,
        SettingsAction::SetDockFocusRunning {
            dock_id: id.clone(),
            enabled: !d.focus_running_window,
        },
    );
    let y = b.cursor_y;
    let x = b.content_x;
    b.push_btn(
        (x, y, 156, 32),
        "+ Разделитель",
        ButtonTone::Secondary,
        SettingsAction::AddDockSeparator(id.clone()),
    );
    b.push_btn(
        (x + 166, y, 156, 32),
        "Открыть docks.cfg",
        ButtonTone::Secondary,
        SettingsAction::OpenDocksConfig,
    );
    if can_delete {
        b.push_btn(
            (x + 332, y, 118, 32),
            "Удалить док",
            ButtonTone::Danger,
            SettingsAction::RemoveDock(id),
        );
    }
    b.cursor_y += 44;
}

pub(crate) fn build_debug_tab(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    b.set_header(
        "Диагностика и логи",
        "Отслеживание событий, нагрузки и ошибок всех процессов",
    );
    let rows = vec![
        row(
            0,
            54,
            "Подробные логи (TRACE)",
            Some("Детальные события движения, рендера и IPC"),
            None,
            true,
        ),
        row(
            54,
            54,
            "Запись логов в файл",
            Some("Сохранение в data/logs/asrbox.log"),
            None,
            true,
        ),
        row(
            108,
            54,
            "Сводка",
            Some(&snap.debug.summary_line),
            None,
            false,
        ),
    ];
    let cy = b.add_card(Some("МОНИТОРИНГ"), false, rows);
    let tx = b.content_x + b.content_w as i32 - 60;
    b.push_toggle(
        tx,
        cy + 15,
        snap.debug.verbose,
        SettingsAction::SetDebugVerbose(!snap.debug.verbose),
    );
    b.push_toggle(
        tx,
        cy + 69,
        snap.debug.file_log,
        SettingsAction::SetDebugFileLog(!snap.debug.file_log),
    );
    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 250, 34),
        "Открыть окно диагностики и логов",
        ButtonTone::Primary,
        SettingsAction::OpenDebugWindow,
    );
    b.cursor_y += 44;
}

pub(crate) fn build_about_tab(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let a = &snap.about;
    b.set_header(
        &format!("asrbox v{}", a.version),
        "Портативный модульный центр рабочего стола Windows 10/11",
    );
    let rows = vec![
        row(
            0,
            52,
            "Режим хранения",
            Some("Полностью портативный — настройки в data рядом с asrbox.exe"),
            None,
            true,
        ),
        row(52, 52, "Исполняемый файл", Some(&a.exe_path), None, true),
        row(104, 52, "Папка данных", Some(&a.data_dir), None, true),
        row(156, 52, "Папка конфигов", Some(&a.config_dir), None, false),
    ];
    b.add_card(Some("СВЕДЕНИЯ И ПУТИ"), false, rows);
    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 184, 34),
        "Открыть папку data",
        ButtonTone::Primary,
        SettingsAction::OpenDataFolder,
    );
    b.push_btn(
        (x + 196, y, 230, 34),
        "История версий и подробности",
        ButtonTone::Secondary,
        SettingsAction::OpenAboutWindow,
    );
    b.cursor_y += 44;
}
