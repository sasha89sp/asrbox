use asrbox_core::IsolationMode;
use asrbox_platform_api::{AppLanguage, FontRenderer, PixelRect, ThemeMode};

use crate::i18n::tr;
use crate::layout::{ButtonTone, HitKind, LayoutBuilder};
use crate::layout_backup::build_backup_and_webdav_section;
use crate::layout_dev::{add_developer_github_section, add_tray_menu_section};
use crate::layout_tabs::row;
use crate::types::{ComboKind, SettingsAction, SettingsSnapshot};

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
            "Стиль и тема, система, контекстное меню трея, бэкап AES-256 и инструменты разработчика",
            "Design theme, system, tray menu, AES-256 backup and developer tools",
        ),
    );
    add_appearance_section(b, snap);
    add_system_section(b, snap);
    add_tray_menu_section(b, snap.menus.tray);
    build_backup_and_webdav_section(b, &snap.backup, editing_server_dir);
    add_diagnostics_and_history_section(b, snap);
    add_paths_section(b, snap);
    add_developer_github_section(b, &snap.github);
}

fn add_appearance_section(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let l = b.lang;
    let app = &snap.appearance;
    let theme_lbl = match app.theme {
        ThemeMode::Dark => tr(l, "Тёмная", "Dark"),
        ThemeMode::Light => tr(l, "Светлая", "Light"),
    };
    let summary = format!("{} · {}", app.design_theme.label(l), theme_lbl);
    let kw = "стиль стеклянный блок win11 тема светлая тёмная акцент цвет шрифт язык directwrite cleartype theme accent font language";
    if !b.accordion(
        "appearance",
        tr(l, "Оформление и язык", "Appearance & Language"),
        &summary,
        kw,
    ) {
        return;
    }
    let rows = vec![
        row(
            0,
            52,
            tr(l, "Стиль оформления", "Design theme"),
            Some(tr(
                l,
                "Стеклянный блок ASR или классический Windows 11",
                "ASR Glass Stack or classic Windows 11",
            )),
            None,
            true,
        ),
        row(
            52,
            50,
            tr(l, "Цветовая тема", "Color theme"),
            Some(tr(
                l,
                "Плавное переключение для всех окон и виджетов",
                "Smooth transition across all windows",
            )),
            None,
            true,
        ),
        row(
            102,
            50,
            tr(l, "Акцентный цвет", "Accent color"),
            Some(tr(
                l,
                "Цвет активных элементов и индикаторов",
                "Highlight color for controls and indicators",
            )),
            None,
            true,
        ),
        row(
            152,
            50,
            tr(l, "Рендеринг шрифтов", "Font rendering"),
            Some(tr(
                l,
                "Лёгкий GDI ClearType или DirectWrite",
                "Lightweight GDI ClearType or DirectWrite",
            )),
            None,
            true,
        ),
        row(
            202,
            50,
            tr(l, "Язык интерфейса", "Interface language"),
            Some(tr(
                l,
                "Применяется мгновенно без перезапуска",
                "Applies immediately without restarting",
            )),
            None,
            false,
        ),
    ];
    let cy = b.add_card(None, false, rows);
    let cw = 216u32;
    let cx = b.content_x + b.content_w as i32 - 14 - cw as i32;
    b.push_combo(
        cx,
        cy + 11,
        cw,
        app.design_theme.label(l),
        None,
        ComboKind::DesignTheme,
    );
    b.push_combo(cx, cy + 62, cw, theme_lbl, None, ComboKind::ThemeMode);
    let accent_col = app
        .accent
        .colors_for_style(app.design_theme, app.theme == ThemeMode::Dark)
        .0;
    b.push_combo(
        cx,
        cy + 112,
        cw,
        app.accent.label(l),
        Some(accent_col),
        ComboKind::AccentPreset,
    );
    let font_lbl = match app.font_renderer {
        FontRenderer::GdiClearType => "GDI ClearType",
        FontRenderer::DirectWrite => "DirectWrite",
    };
    b.push_combo(cx, cy + 162, cw, font_lbl, None, ComboKind::FontRenderer);
    let lang_lbl = match app.language {
        AppLanguage::Ru => "Русский",
        AppLanguage::En => "English",
    };
    b.push_combo(cx, cy + 212, cw, lang_lbl, None, ComboKind::Language);
}

fn add_system_section(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let l = b.lang;
    let iso_lbl = match snap.isolation {
        IsolationMode::Process => tr(l, "Отдельные процессы", "Separate processes"),
        IsolationMode::InProc => tr(l, "Один процесс", "In-Process"),
    };
    let auto_lbl = if snap.autostart_enabled {
        tr(l, "Автозапуск вкл", "Autostart on")
    } else {
        tr(l, "Автозапуск выкл", "Autostart off")
    };
    let summary = format!("{auto_lbl} · {iso_lbl}");
    let kw = "автозагрузка windows изоляция процессы перезапуск модули сброс настроек сбросить reset autostart isolation restart";
    if !b.accordion(
        "system",
        tr(l, "Система, модули и сброс", "System, Modules & Reset"),
        &summary,
        kw,
    ) {
        return;
    }
    let rows = vec![
        row(
            0,
            52,
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
            52,
            54,
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
            106,
            54,
            tr(l, "Перезапуск всех модулей", "Restart all modules"),
            Some(tr(
                l,
                "Перезагружает включённые модули без выхода из asrbox",
                "Restarts enabled modules without exiting asrbox",
            )),
            None,
            true,
        ),
        row(
            160,
            56,
            tr(l, "Сброс настроек программы", "Reset all settings"),
            Some(tr(
                l,
                "Вернуть оформление, доки, виджеты, меню и оверлей к заводским",
                "Restore appearance, docks, widgets, menus and overlay defaults",
            )),
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
        snap.autostart_enabled,
        SettingsAction::SetAutostart(!snap.autostart_enabled),
    );
    b.push_combo(rx - 204, cy + 64, 204, iso_lbl, None, ComboKind::Isolation);
    b.push_btn(
        (rx - 204, cy + 118, 204, 30),
        tr(l, "Перезапустить все модули", "Restart All Modules"),
        ButtonTone::Secondary,
        SettingsAction::RestartAllModules,
    );
    if !b.confirming_reset {
        b.push_control(
            PixelRect {
                x: rx - 186,
                y: b.sy(cy + 173),
                width: 186,
                height: 30,
            },
            HitKind::StartResetConfirm {
                label: tr(l, "Сбросить настройки…", "Reset Settings…").to_owned(),
            },
        );
    } else {
        add_reset_warning_card(b);
    }
}

fn add_reset_warning_card(b: &mut LayoutBuilder) {
    let l = b.lang;
    let warn_rows = vec![
        row(0, 52, tr(l, "Подтвердите сброс настроек asrbox", "Confirm resetting asrbox settings"), Some(tr(l, "Будут сброшены: тема и акцент, конфиги док-панелей, виджетов, меню и игрового оверлея.", "Will reset: theme/accent, dock panels, widgets, context menus and overlay configs.")), None, false),
        row(52, 50, "", Some(tr(l, "СОХРАНЯТСЯ без изменений: архивы в data/backups, заметки в data/notes и пароли в Windows Credential Manager.", "PRESERVED safely: archives in data/backups, notes in data/notes, and passwords in Credential Manager.")), None, false),
    ];
    b.add_card(None, true, warn_rows);
    let (x, y) = (b.content_x, b.cursor_y - 4);
    b.push_btn(
        (x, y, 178, 30),
        tr(l, "Подтвердить сброс", "Confirm Reset"),
        ButtonTone::Danger,
        SettingsAction::ConfirmResetAll,
    );
    b.push_control(
        PixelRect {
            x: x + 188,
            y: b.sy(y),
            width: 108,
            height: 30,
        },
        HitKind::CancelResetConfirm {
            label: tr(l, "Отмена", "Cancel").to_owned(),
        },
    );
    b.cursor_y = y + 40;
}

fn add_diagnostics_and_history_section(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let l = b.lang;
    let ver = format!("v{}", snap.about.version);
    let kw = "диагностика монитор логи trace cpu ram история версий changelog diagnostics logs version history";
    if !b.accordion(
        "diag_history",
        tr(
            l,
            "Диагностика и история версий",
            "Diagnostics & Version History",
        ),
        &ver,
        kw,
    ) {
        return;
    }
    let summary = if snap.diagnostics.summary_line.is_empty() {
        tr(
            l,
            "Монитор работает только при открытой вкладке «Диагностика»",
            "Live monitor runs only while Diagnostics tab is open",
        )
    } else {
        snap.diagnostics.summary_line.as_str()
    };
    let ver_title = format!("asrbox v{}", snap.about.version);
    let rows = vec![
        row(
            0,
            54,
            tr(l, "Монитор и журнал диагностики", "Live Diagnostics & Logs"),
            Some(summary),
            None,
            true,
        ),
        row(
            54,
            54,
            &ver_title,
            Some(tr(
                l,
                "История изменений за все релизы в отдельной вкладке",
                "Full changelog in an inner tab",
            )),
            None,
            false,
        ),
    ];
    let cy = b.add_card(None, false, rows);
    let rx = b.content_x + b.content_w as i32 - 14;
    b.push_btn(
        (rx - 196, cy + 12, 196, 30),
        tr(l, "Открыть диагностику →", "Open Diagnostics →"),
        ButtonTone::Primary,
        SettingsAction::OpenDiagnosticsTab,
    );
    b.push_btn(
        (rx - 196, cy + 66, 196, 30),
        tr(l, "История версий →", "Version History →"),
        ButtonTone::Secondary,
        SettingsAction::OpenVersionHistoryTab,
    );
}

fn add_paths_section(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    let l = b.lang;
    if !b.accordion(
        "paths",
        tr(l, "Пути программы", "Program Paths"),
        "portable · data/",
        "пути программа папка data config backups logs paths folder",
    ) {
        return;
    }
    let a = &snap.about;
    let rows = vec![
        row(
            0,
            50,
            tr(l, "Исполняемый файл", "Executable"),
            Some(&a.exe_path),
            None,
            true,
        ),
        row(
            50,
            50,
            tr(l, "Папка данных (data)", "Data directory"),
            Some(&a.data_dir),
            None,
            true,
        ),
        row(
            100,
            50,
            tr(l, "Папка конфигов", "Config directory"),
            Some(&a.config_dir),
            None,
            true,
        ),
        row(
            150,
            50,
            tr(l, "Папка бэкапов", "Backups directory"),
            Some(&a.backups_dir),
            None,
            true,
        ),
        row(
            200,
            50,
            tr(l, "Папка логов", "Logs directory"),
            Some(&a.logs_dir),
            None,
            false,
        ),
    ];
    b.add_card(None, false, rows);
    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 166, 30),
        tr(l, "Открыть папку", "Open Folder"),
        ButtonTone::Primary,
        SettingsAction::OpenDataFolder,
    );
    b.push_btn(
        (x + 176, y, 174, 30),
        tr(l, "Копировать пути", "Copy Paths"),
        ButtonTone::Secondary,
        SettingsAction::CopyPathsToClipboard,
    );
    b.cursor_y = y + 40;
}
