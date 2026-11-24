use asrbox_platform_api::{AccentPreset, AppLanguage, PixelRect, ThemeMode};

use crate::i18n::tr;
use crate::layout::{ButtonTone, HitKind, LayoutBuilder};
use crate::layout_backup::build_backup_and_webdav_section;
use crate::layout_tabs::row;
use crate::types::{SettingsAction, SettingsSnapshot};

pub(crate) fn build_general_tab(
    b: &mut LayoutBuilder,
    snap: &SettingsSnapshot,
    editing_server_dir: Option<&str>,
) {
    let lang = b.lang;
    b.set_header(
        tr(lang, "Общие настройки", "General Settings"),
        tr(
            lang,
            "Оформление, язык, бэкап и облако WebDAV, диагностика, история версий и пути программы",
            "Appearance, language, backup & WebDAV cloud, diagnostics, version history and paths",
        ),
    );
    // Strict top-to-bottom order requested by user:
    // 1. Theme & Accent
    add_theme_card(b, snap);
    // 2. Language (+ Autostart)
    add_language_card(b, snap);
    // 3. Backup, Multi-Server WebDAV & File/Folder Sync
    build_backup_and_webdav_section(b, &snap.backup, editing_server_dir);
    // 4. Diagnostics (opens browser-style inner tab)
    add_diagnostics_launcher_card(b, snap);
    // 5. Version History (renamed from About, opens browser-style inner tab)
    add_version_history_launcher_card(b, snap);
    // 6. Program Paths + Open Folder / Copy Paths buttons
    add_paths_card(b, snap);
}

fn add_theme_card(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let lang = b.lang;
    if !b.matches_search("тема оформление светлая тёмная акцент цвет theme dark light accent color")
    {
        return;
    }
    let app = &snap.appearance;
    let rows = vec![
        row(
            0,
            56,
            tr(lang, "Тема оформления", "Color theme"),
            Some(tr(
                lang,
                "Windows 11 Fluent Design — плавное переключение для всех окон",
                "Windows 11 Fluent Design — smooth transition across all windows",
            )),
            None,
            true,
        ),
        row(
            56,
            56,
            tr(lang, "Акцентный цвет", "Accent color"),
            Some(tr(
                lang,
                "Цвет активных элементов, переключателей и индикаторов",
                "Highlight color for active controls, toggles and indicators",
            )),
            None,
            false,
        ),
    ];
    let cy = b.add_card(
        Some(tr(lang, "1. ОФОРМЛЕНИЕ", "1. APPEARANCE")),
        false,
        rows,
    );
    b.add_segmented_row(
        cy,
        56,
        vec![
            (
                tr(lang, "Светлая", "Light"),
                app.theme == ThemeMode::Light,
                SettingsAction::SetThemeMode(ThemeMode::Light),
            ),
            (
                tr(lang, "Тёмная", "Dark"),
                app.theme == ThemeMode::Dark,
                SettingsAction::SetThemeMode(ThemeMode::Dark),
            ),
        ],
    );
    let count = AccentPreset::ALL.len() as i32;
    let step = 32i32;
    let start_x = b.content_x + b.content_w as i32 - 14 - count * step + 4;
    let sy = b.sy(cy + 56 + 14);
    for (idx, preset) in AccentPreset::ALL.iter().copied().enumerate() {
        b.push_control(
            PixelRect {
                x: start_x + (idx as i32) * step,
                y: sy,
                width: 26,
                height: 26,
            },
            HitKind::AccentSwatch {
                preset,
                active: app.accent == preset,
            },
        );
    }
}

fn add_language_card(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let lang = b.lang;
    if !b.matches_search("язык интерфейс русский english автозагрузка windows language autostart")
    {
        return;
    }
    let cur_lang = snap.appearance.language;
    let rows = vec![
        row(
            0,
            54,
            tr(lang, "Язык интерфейса", "Interface language"),
            Some(tr(
                lang,
                "Применяется мгновенно без перезапуска программы",
                "Applies immediately without restarting",
            )),
            None,
            true,
        ),
        row(
            54,
            54,
            tr(lang, "Автозагрузка с Windows", "Launch with Windows"),
            Some(tr(
                lang,
                "Запускать asrbox при входе в систему (HKCU\\...\\Run)",
                "Start asrbox automatically on Windows sign-in",
            )),
            None,
            false,
        ),
    ];
    let cy = b.add_card(
        Some(tr(lang, "2. ЯЗЫК И СИСТЕМА", "2. LANGUAGE & STARTUP")),
        false,
        rows,
    );
    b.add_segmented_row(
        cy,
        54,
        vec![
            (
                "Русский",
                cur_lang == AppLanguage::Ru,
                SettingsAction::SetLanguage(AppLanguage::Ru),
            ),
            (
                "English",
                cur_lang == AppLanguage::En,
                SettingsAction::SetLanguage(AppLanguage::En),
            ),
        ],
    );
    let tx = b.content_x + b.content_w as i32 - 58;
    b.push_toggle(
        tx,
        cy + 54 + 16,
        snap.autostart_enabled,
        SettingsAction::SetAutostart(!snap.autostart_enabled),
    );
}

fn add_diagnostics_launcher_card(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let lang = b.lang;
    if !b.matches_search("диагностика монитор логи trace cpu ram diagnostics logs monitor")
    {
        return;
    }
    let summary = if snap.diagnostics.summary_line.is_empty() {
        tr(
            lang,
            "Монитор ресурсов, модули и живые логи в отдельной вкладке",
            "Live resource monitor, module stats and logs in a tab",
        )
    } else {
        snap.diagnostics.summary_line.as_str()
    };
    let rows = vec![row(
        0,
        58,
        tr(
            lang,
            "Монитор и журнал диагностики",
            "Live Diagnostics & Logs",
        ),
        Some(summary),
        None,
        false,
    )];
    let cy = b.add_card(
        Some(tr(lang, "4. ДИАГНОСТИКА", "4. DIAGNOSTICS")),
        false,
        rows,
    );
    let bw = 188u32;
    let bx = b.content_x + b.content_w as i32 - bw as i32 - 14;
    b.push_btn(
        (bx, cy + 14, bw, 30),
        tr(lang, "Открыть диагностику →", "Open Diagnostics →"),
        ButtonTone::Primary,
        SettingsAction::OpenDiagnosticsTab,
    );
}

fn add_version_history_launcher_card(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let lang = b.lang;
    if !b.matches_search("история версий о программе changelog версия version history about")
    {
        return;
    }
    let ver_title = format!("asrbox v{}", snap.about.version);
    let rows = vec![row(
        0,
        58,
        &ver_title,
        Some(tr(
            lang,
            "Полная история изменений за все версии откроется в новой вкладке этого окна",
            "Full multi-version changelog opens in a new tab inside this window",
        )),
        None,
        false,
    )];
    let cy = b.add_card(
        Some(tr(lang, "5. ИСТОРИЯ ВЕРСИЙ", "5. VERSION HISTORY")),
        false,
        rows,
    );
    let bw = 204u32;
    let bx = b.content_x + b.content_w as i32 - bw as i32 - 14;
    b.push_btn(
        (bx, cy + 14, bw, 30),
        tr(lang, "Открыть историю версий →", "Open Version History →"),
        ButtonTone::Secondary,
        SettingsAction::OpenVersionHistoryTab,
    );
}

fn add_paths_card(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let lang = b.lang;
    if !b.matches_search("пути программа папка data config backups logs paths folder")
    {
        return;
    }
    let a = &snap.about;
    let rows = vec![
        row(
            0,
            48,
            tr(lang, "Исполняемый файл", "Executable"),
            Some(&a.exe_path),
            None,
            true,
        ),
        row(
            48,
            48,
            tr(lang, "Папка данных (data)", "Data directory"),
            Some(&a.data_dir),
            None,
            true,
        ),
        row(
            96,
            48,
            tr(lang, "Папка конфигов", "Config directory"),
            Some(&a.config_dir),
            None,
            true,
        ),
        row(
            144,
            48,
            tr(lang, "Папка бэкапов", "Backups directory"),
            Some(&a.backups_dir),
            None,
            true,
        ),
        row(
            192,
            48,
            tr(lang, "Папка логов", "Logs directory"),
            Some(&a.logs_dir),
            None,
            false,
        ),
    ];
    b.add_card(
        Some(tr(lang, "6. ПУТИ ПРОГРАММЫ", "6. PROGRAM PATHS")),
        false,
        rows,
    );
    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 166, 30),
        tr(lang, "Открыть папку", "Open Folder"),
        ButtonTone::Primary,
        SettingsAction::OpenDataFolder,
    );
    b.push_btn(
        (x + 176, y, 174, 30),
        tr(lang, "Копировать пути", "Copy Paths"),
        ButtonTone::Secondary,
        SettingsAction::CopyPathsToClipboard,
    );
    b.cursor_y = y + 42;
}
