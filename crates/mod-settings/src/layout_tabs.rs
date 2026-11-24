use asrbox_core::IsolationMode;

use crate::layout::{ButtonTone, CardRow, LayoutBuilder};
pub(crate) use crate::layout_cal::build_calendar_tab;
use crate::types::{SettingsAction, SettingsSnapshot};

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
    let auto_sub = Some("Запускать asrbox при входе в систему (HKCU\\...\\Run)");
    let iso_y = b.add_card(
        Some("СИСТЕМА И ЗАПУСК"),
        false,
        vec![
            row(0, 56, "Изоляция модулей", iso_sub, None, true),
            row(56, 54, "Автозагрузка с Windows", auto_sub, None, false),
        ],
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
    let auto_tx = b.content_x + b.content_w as i32 - 60;
    b.push_toggle(
        auto_tx,
        iso_y + 71,
        snap.autostart_enabled,
        SettingsAction::SetAutostart(!snap.autostart_enabled),
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
