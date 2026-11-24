use asrbox_platform_api::PixelRect;

use crate::i18n::tr;
use crate::layout::{ButtonTone, CardRow, LayoutBuilder};
use crate::layout_tabs::row;
use crate::types::{BrowserTab, SettingsAction, SettingsSnapshot};

pub(crate) fn build_browser_tab(b: &mut LayoutBuilder, snap: &SettingsSnapshot, tab: &BrowserTab) {
    match tab {
        BrowserTab::Main => {}
        BrowserTab::Diagnostics => build_diagnostics_browser_tab(b, snap),
        BrowserTab::VersionHistory => build_version_history_browser_tab(b, snap),
        BrowserTab::WebDavBrowser { server_id, .. } => {
            build_webdav_browser_tab(b, snap, server_id);
        }
    }
}

fn build_diagnostics_browser_tab(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let lang = b.lang;
    let d = &snap.diagnostics;
    b.set_header(
        tr(
            lang,
            "Диагностика в реальном времени",
            "Real-Time Diagnostics",
        ),
        tr(
            lang,
            "Живой монитор ресурсов, модулей и логов — можно включать и выключать на лету",
            "Live resource, module and log monitor — toggle on/off anytime",
        ),
    );
    let monitor_rows = vec![
        row(
            0,
            52,
            tr(lang, "Монитор в реальном времени", "Real-time monitor"),
            Some(tr(
                lang,
                "Автообновление метрик CPU/RAM/GDI и журнала каждую секунду",
                "Auto-refresh CPU/RAM/GDI metrics and live log every second",
            )),
            None,
            true,
        ),
        row(
            52,
            50,
            tr(lang, "Подробные логи (TRACE)", "Verbose logs (TRACE)"),
            Some(tr(
                lang,
                "Детальные события ввода, рендера и IPC",
                "Detailed input, rendering and IPC trace events",
            )),
            None,
            true,
        ),
        row(
            102,
            50,
            tr(lang, "Запись логов в файл", "Write logs to file"),
            Some("data/logs/asrbox.log"),
            None,
            false,
        ),
    ];
    let cy = b.add_card(
        Some(tr(lang, "УПРАВЛЕНИЕ ДИАГНОСТИКОЙ", "DIAGNOSTICS CONTROLS")),
        false,
        monitor_rows,
    );
    let tx = b.content_x + b.content_w as i32 - 58;
    b.push_toggle(
        tx,
        cy + 15,
        d.monitor_enabled,
        SettingsAction::SetDiagnosticsMonitorEnabled(!d.monitor_enabled),
    );
    b.push_toggle(
        tx,
        cy + 66,
        d.verbose,
        SettingsAction::SetDebugVerbose(!d.verbose),
    );
    b.push_toggle(
        tx,
        cy + 116,
        d.file_log,
        SettingsAction::SetDebugFileLog(!d.file_log),
    );

    let (x, y) = (b.content_x, b.cursor_y);
    let filt_lbl = format!("{}: {}", tr(lang, "Фильтр", "Filter"), d.filter_label);
    b.push_btn(
        (x, y, 138, 30),
        &filt_lbl,
        ButtonTone::Secondary,
        SettingsAction::CycleDebugFilter,
    );
    b.push_btn(
        (x + 146, y, 102, 30),
        tr(lang, "Очистить", "Clear"),
        ButtonTone::Secondary,
        SettingsAction::ClearDebugLogs,
    );
    b.push_btn(
        (x + 256, y, 146, 30),
        tr(lang, "Копировать отчёт", "Copy Report"),
        ButtonTone::Primary,
        SettingsAction::CopyDebugReport,
    );
    b.push_btn(
        (x + 410, y, 134, 30),
        tr(lang, "Сохранить .txt", "Save .txt"),
        ButtonTone::Secondary,
        SettingsAction::SaveDebugReport,
    );
    b.cursor_y = y + 42;

    if !d.metrics_lines.is_empty() {
        let rows = lines_to_rows(&d.metrics_lines, 30);
        b.add_card(
            Some(tr(lang, "МЕТРИКИ ПРОЦЕССА", "PROCESS METRICS")),
            false,
            rows,
        );
    }
    if !d.module_lines.is_empty() {
        let rows = lines_to_rows(&d.module_lines, 30);
        b.add_card(
            Some(tr(
                lang,
                "СОСТОЯНИЕ МОДУЛЕЙ И ВОРКЕРОВ",
                "MODULES & WORKERS",
            )),
            false,
            rows,
        );
    }
    let log_rows = if d.log_lines.is_empty() {
        vec![row(
            0,
            36,
            tr(lang, "Событий пока нет", "No events yet"),
            None,
            None,
            false,
        )]
    } else {
        lines_to_rows(&d.log_lines, 24)
    };
    b.add_card(
        Some(tr(lang, "ЖУРНАЛ СОБЫТИЙ", "EVENT LOG")),
        false,
        log_rows,
    );
}

fn build_version_history_browser_tab(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let lang = b.lang;
    let a = &snap.about;
    b.set_header(
        &format!(
            "{} — asrbox v{}",
            tr(lang, "История версий", "Version History"),
            a.version
        ),
        tr(
            lang,
            "Полный журнал изменений за все выпуски программы (CHANGELOG.md)",
            "Full multi-version release changelog (CHANGELOG.md)",
        ),
    );
    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 194, 30),
        tr(lang, "Копировать информацию", "Copy Version Info"),
        ButtonTone::Primary,
        SettingsAction::CopyVersionInfo,
    );
    b.push_btn(
        (x + 204, y, 166, 30),
        tr(lang, "Открыть папку data", "Open Data Folder"),
        ButtonTone::Secondary,
        SettingsAction::OpenDataFolder,
    );
    b.cursor_y = y + 42;

    let rows: Vec<CardRow> = a
        .changelog_lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let is_ver = line.starts_with("## ");
            let clean = line.trim_start_matches("## ").trim_start_matches("### ");
            if is_ver {
                row((i as i32) * 26, 26, clean, None, None, false)
            } else {
                CardRow {
                    y: (i as i32) * 26,
                    height: 26,
                    title: String::new(),
                    subtitle: Some(clean.to_owned()),
                    value_label: None,
                    divider: false,
                }
            }
        })
        .collect();
    let card_rows = if rows.is_empty() {
        vec![row(0, 40, "CHANGELOG.md", Some(&a.version), None, false)]
    } else {
        rows
    };
    b.add_card(
        Some(tr(lang, "ИСТОРИЯ ИЗМЕНЕНИЙ", "RELEASE HISTORY")),
        false,
        card_rows,
    );
}

fn build_webdav_browser_tab(b: &mut LayoutBuilder, snap: &SettingsSnapshot, server_id: &str) {
    let lang = b.lang;
    let Some(br) = snap
        .backup
        .browser
        .as_ref()
        .filter(|v| v.server_id == server_id)
    else {
        b.set_header(
            tr(lang, "Обзор WebDAV", "WebDAV Browser"),
            tr(
                lang,
                "Выберите сервер в разделе Общие настройки",
                "Select a server in General",
            ),
        );
        return;
    };
    b.set_header(
        &format!("☁  {}", br.server_name),
        &format!(
            "{}: {}",
            tr(lang, "Текущий путь", "Current path"),
            br.current_dir
        ),
    );
    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 92, 30),
        tr(lang, "↑ Наверх", "↑ Up"),
        ButtonTone::Secondary,
        SettingsAction::WebDavBrowseUp(br.server_id.clone()),
    );
    b.push_btn(
        (x + 100, y, 114, 30),
        tr(lang, "↻ Обновить", "↻ Refresh"),
        ButtonTone::Secondary,
        SettingsAction::WebDavBrowseDir {
            server_id: br.server_id.clone(),
            path: br.current_dir.clone(),
        },
    );
    b.push_btn(
        (x + 222, y, 186, 30),
        tr(lang, "✓ Выбрать эту папку", "✓ Select This Folder"),
        ButtonTone::Primary,
        SettingsAction::WebDavSelectCurrentFolder(br.server_id.clone()),
    );
    b.cursor_y = y + 42;

    let mk_y = b.add_card(
        Some(tr(lang, "СОЗДАТЬ НОВУЮ ПАПКУ", "CREATE NEW FOLDER")),
        false,
        vec![row(0, 52, "", None, None, false)],
    );
    let inp_w = b.content_w.saturating_sub(176).max(160);
    b.out.input_rects.aux = Some(PixelRect {
        x: b.content_x + 12,
        y: b.sy(mk_y + 12),
        width: inp_w,
        height: 28,
    });
    b.push_btn(
        (b.content_x + 20 + inp_w as i32, mk_y + 11, 140, 30),
        tr(lang, "+ Создать папку", "+ Create Folder"),
        ButtonTone::Primary,
        SettingsAction::WebDavCreateFolderFromInput(br.server_id.clone()),
    );

    if br.loading {
        b.add_card(
            Some(tr(lang, "КАТАЛОГИ", "FOLDERS")),
            false,
            vec![row(
                0,
                46,
                tr(
                    lang,
                    "Загрузка списка папок WebDAV...",
                    "Loading WebDAV folders...",
                ),
                Some(&br.current_dir),
                None,
                false,
            )],
        );
        return;
    }
    let dirs: Vec<_> = br.entries.iter().filter(|e| e.is_dir).collect();
    if dirs.is_empty() {
        let msg = if br.status_text.is_empty() {
            tr(lang, "Подпапок не найдено", "No subfolders found")
        } else {
            br.status_text.as_str()
        };
        b.add_card(
            Some(tr(lang, "КАТАЛОГИ", "FOLDERS")),
            false,
            vec![row(0, 46, msg, Some(&br.current_dir), None, false)],
        );
        return;
    }
    let rows: Vec<CardRow> = dirs
        .iter()
        .enumerate()
        .map(|(i, e)| {
            row(
                (i as i32) * 44,
                44,
                &format!("📁  {}", e.name),
                Some(&e.full_path),
                None,
                i + 1 < dirs.len(),
            )
        })
        .collect();
    let list_y = b.add_card(Some(tr(lang, "КАТАЛОГИ", "FOLDERS")), false, rows);
    let rx = b.content_x + b.content_w as i32 - 14;
    for (i, e) in dirs.iter().enumerate() {
        let ry = list_y + (i as i32) * 44 + 8;
        b.push_btn(
            (rx - 102, ry, 102, 28),
            tr(lang, "Открыть →", "Open →"),
            ButtonTone::Secondary,
            SettingsAction::WebDavBrowseDir {
                server_id: br.server_id.clone(),
                path: e.full_path.clone(),
            },
        );
    }
}

fn lines_to_rows(lines: &[String], row_h: u32) -> Vec<CardRow> {
    lines
        .iter()
        .enumerate()
        .map(|(i, line)| CardRow {
            y: (i as i32) * row_h as i32,
            height: row_h,
            title: String::new(),
            subtitle: Some(line.clone()),
            value_label: None,
            divider: i + 1 < lines.len(),
        })
        .collect()
}
