use asrbox_core::IsolationMode;
use asrbox_platform_api::{AccentPreset, AppLanguage, FontRenderer, PixelRect, ThemeMode};

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
    let l = b.lang;
    if !b.matches_search(
        "тема оформление светлая тёмная акцент цвет шрифт directwrite cleartype theme dark light accent font",
    ) {
        return;
    }
    let rows = vec![
        row(
            0,
            56,
            tr(l, "Тема оформления", "Color theme"),
            Some(tr(
                l,
                "Windows 11 Fluent Design — плавное переключение для всех окон",
                "Windows 11 Fluent Design — smooth transition across all windows",
            )),
            None,
            true,
        ),
        row(
            56,
            56,
            tr(l, "Акцентный цвет", "Accent color"),
            Some(tr(
                l,
                "Цвет активных элементов, переключателей и индикаторов",
                "Highlight color for active controls, toggles and indicators",
            )),
            None,
            true,
        ),
        row(
            112,
            56,
            tr(l, "Рендеринг шрифтов", "Font rendering"),
            Some(tr(
                l,
                "Segoe UI Variable (Win 11): лёгкий GDI ClearType или DirectWrite",
                "Segoe UI Variable (Win 11): lightweight GDI ClearType or DirectWrite",
            )),
            None,
            false,
        ),
    ];
    let cy = b.add_card(Some(tr(l, "1. ОФОРМЛЕНИЕ", "1. APPEARANCE")), false, rows);
    add_theme_card_controls(b, snap, cy);
}

fn add_theme_card_controls(b: &mut LayoutBuilder, snap: &SettingsSnapshot, cy: i32) {
    let l = b.lang;
    let app = &snap.appearance;
    b.add_segmented_row(
        cy,
        56,
        vec![
            (
                tr(l, "Светлая", "Light"),
                app.theme == ThemeMode::Light,
                SettingsAction::SetThemeMode(ThemeMode::Light),
            ),
            (
                tr(l, "Тёмная", "Dark"),
                app.theme == ThemeMode::Dark,
                SettingsAction::SetThemeMode(ThemeMode::Dark),
            ),
        ],
    );
    let count = AccentPreset::ALL.len() as i32;
    let start_x = b.content_x + b.content_w as i32 - 14 - count * 32 + 4;
    let sy = b.sy(cy + 56 + 14);
    for (idx, preset) in AccentPreset::ALL.iter().copied().enumerate() {
        let rect = PixelRect {
            x: start_x + (idx as i32) * 32,
            y: sy,
            width: 26,
            height: 26,
        };
        b.push_control(
            rect,
            HitKind::AccentSwatch {
                preset,
                active: app.accent == preset,
            },
        );
    }
    b.add_segmented_row(
        cy + 112,
        56,
        vec![
            (
                "GDI ClearType",
                app.font_renderer == FontRenderer::GdiClearType,
                SettingsAction::SetFontRenderer(FontRenderer::GdiClearType),
            ),
            (
                "DirectWrite",
                app.font_renderer == FontRenderer::DirectWrite,
                SettingsAction::SetFontRenderer(FontRenderer::DirectWrite),
            ),
        ],
    );
}

fn add_language_card(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let l = b.lang;
    if !b.matches_search(
        "язык интерфейс русский english автозагрузка windows изоляция процессы перезапуск модули language autostart isolation restart",
    ) {
        return;
    }
    let rows = vec![
        row(
            0,
            54,
            tr(l, "Язык интерфейса", "Interface language"),
            Some(tr(
                l,
                "Применяется мгновенно без перезапуска программы",
                "Applies immediately without restarting",
            )),
            None,
            true,
        ),
        row(
            54,
            54,
            tr(l, "Автозагрузка с Windows", "Launch with Windows"),
            Some(tr(
                l,
                "Запускать asrbox при входе в систему (HKCU\\...\\Run)",
                "Start asrbox automatically on Windows sign-in",
            )),
            None,
            true,
        ),
        row(
            108,
            56,
            tr(l, "Изоляция модулей", "Module isolation"),
            Some(tr(
                l,
                "Отдельный процесс для каждого модуля или общий процесс",
                "Separate worker process per module or shared process",
            )),
            None,
            true,
        ),
        row(
            164,
            56,
            tr(l, "Перезапуск всех модулей", "Restart all modules"),
            Some(tr(
                l,
                "Перезагружает все включённые модули без выхода из asrbox",
                "Restarts every enabled module without exiting asrbox",
            )),
            None,
            false,
        ),
    ];
    let cy = b.add_card(
        Some(tr(l, "2. ЯЗЫК И СИСТЕМА", "2. LANGUAGE & SYSTEM")),
        false,
        rows,
    );
    add_language_card_controls(b, snap, cy);
}

fn add_language_card_controls(b: &mut LayoutBuilder, snap: &SettingsSnapshot, cy: i32) {
    let l = b.lang;
    let cur_lang = snap.appearance.language;
    let is_proc = snap.isolation == IsolationMode::Process;
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
    b.add_segmented_row(
        cy + 108,
        56,
        vec![
            (
                tr(l, "Процессы", "Processes"),
                is_proc,
                SettingsAction::SetIsolation(IsolationMode::Process),
            ),
            (
                tr(l, "Один процесс", "In-Process"),
                !is_proc,
                SettingsAction::SetIsolation(IsolationMode::InProc),
            ),
        ],
    );
    let bx = b.content_x + b.content_w as i32 - 204 - 14;
    b.push_btn(
        (bx, cy + 164 + 13, 204, 30),
        tr(l, "Перезапустить все модули", "Restart All Modules"),
        ButtonTone::Secondary,
        SettingsAction::RestartAllModules,
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
            52,
            tr(lang, "Исполняемый файл", "Executable"),
            Some(&a.exe_path),
            None,
            true,
        ),
        row(
            52,
            52,
            tr(lang, "Папка данных (data)", "Data directory"),
            Some(&a.data_dir),
            None,
            true,
        ),
        row(
            104,
            52,
            tr(lang, "Папка конфигов", "Config directory"),
            Some(&a.config_dir),
            None,
            true,
        ),
        row(
            156,
            52,
            tr(lang, "Папка бэкапов", "Backups directory"),
            Some(&a.backups_dir),
            None,
            true,
        ),
        row(
            208,
            52,
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
