use asrbox_platform_api::PixelRect;

use crate::i18n::tr;
use crate::layout::{ButtonTone, LayoutBuilder};
use crate::layout_tabs::row;
use crate::types::{FileSyncSettingsView, SettingsAction, WebDavDraftView, WebDavServerView};

pub(crate) fn add_webdav_draft_card(b: &mut LayoutBuilder, draft: &WebDavDraftView) {
    let lang = b.lang;
    let url_sub = (!draft.is_custom).then_some(draft.effective_url.as_str());
    let status = if draft.status_text.is_empty() {
        tr(
            lang,
            "Введите логин и пароль приложения, проверьте связь и сохраните сервер",
            "Enter login and app password, test connection and save server",
        )
    } else {
        draft.status_text.as_str()
    };
    let rows = vec![
        row(
            0,
            46,
            tr(lang, "Сервер HTTPS", "HTTPS Server"),
            url_sub,
            None,
            true,
        ),
        row(
            46,
            46,
            tr(lang, "Логин / Email", "Login / Email"),
            None,
            None,
            true,
        ),
        row(
            92,
            50,
            tr(lang, "Пароль приложения", "App Password"),
            Some(tr(
                lang,
                "Сохраняется только в Windows Credential Manager",
                "Stored securely in Windows Credential Manager only",
            )),
            None,
            true,
        ),
        row(
            142,
            44,
            tr(lang, "Состояние", "Status"),
            Some(status),
            None,
            false,
        ),
    ];
    let header = format!(
        "{}: {}",
        tr(lang, "НОВЫЙ WEBDAV СЕРВЕР", "NEW WEBDAV SERVER"),
        draft.provider_label
    );
    let cy = b.add_card(Some(&header), true, rows);
    let inp_x = b.content_x + 178;
    let inp_w = b.content_w.saturating_sub(194).max(160);
    if draft.is_custom {
        b.out.input_rects.extra = Some(PixelRect {
            x: inp_x,
            y: b.sy(cy + 9),
            width: inp_w,
            height: 28,
        });
    }
    b.out.input_rects.primary = Some(PixelRect {
        x: inp_x,
        y: b.sy(cy + 55),
        width: inp_w,
        height: 28,
    });
    b.out.input_rects.secret = Some(PixelRect {
        x: inp_x,
        y: b.sy(cy + 103),
        width: inp_w,
        height: 28,
    });
    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 146, 30),
        tr(lang, "Проверить связь", "Test Connection"),
        ButtonTone::Secondary,
        SettingsAction::TestWebDavDraft,
    );
    b.push_btn(
        (x + 154, y, 156, 30),
        tr(lang, "Сохранить сервер", "Save Server"),
        ButtonTone::Primary,
        SettingsAction::SaveWebDavDraft,
    );
    b.push_btn(
        (x + 318, y, 96, 30),
        tr(lang, "Отмена", "Cancel"),
        ButtonTone::Danger,
        SettingsAction::CancelWebDavDraft,
    );
    b.cursor_y = y + 42;
}

pub(crate) fn add_file_sync_card(
    b: &mut LayoutBuilder,
    sync: &FileSyncSettingsView,
    servers: &[WebDavServerView],
) {
    let lang = b.lang;
    let has_servers = !servers.is_empty();
    let chip_rows = if has_servers {
        servers.len().div_ceil(3) as u32
    } else {
        0
    };
    let chips_h = if has_servers { 34 + chip_rows * 34 } else { 0 };
    let items_count = sync.items.len();
    let mut rows = vec![row(
        0,
        50,
        tr(
            lang,
            "Выбранные файлы и папки Windows",
            "Windows files & folders to sync",
        ),
        Some(tr(
            lang,
            "Добавьте любые файлы или папки для синхронизации с WebDAV",
            "Pick any local files or folders to sync with WebDAV",
        )),
        Some(format!("{items_count}")),
        true,
    )];
    let mut off = 50i32;
    for item in &sync.items {
        let kind = if item.is_dir {
            tr(lang, "Папка", "Folder")
        } else {
            tr(lang, "Файл", "File")
        };
        let sub = format!("{kind} → {}", item.remote_subdir);
        rows.push(row(off, 46, &item.local_path, Some(&sub), None, true));
        off += 46;
    }
    let servers_row_off = off;
    if has_servers {
        rows.push(row(
            off,
            chips_h,
            tr(lang, "Серверы синхронизации", "Sync target servers"),
            Some(tr(
                lang,
                "Выберите один или несколько серверов",
                "Select one or more servers",
            )),
            None,
            true,
        ));
        off += chips_h as i32;
    }
    let freq_off = off;
    rows.push(row(
        off,
        52,
        tr(lang, "Частота синхронизации", "Sync frequency"),
        None,
        None,
        true,
    ));
    off += 52;
    let st = if sync.busy {
        tr(lang, "Выполняется синхронизация...", "Syncing...")
    } else if sync.status_text.is_empty() {
        tr(lang, "Ожидает запуска", "Idle")
    } else {
        sync.status_text.as_str()
    };
    rows.push(row(
        off,
        44,
        tr(lang, "Статус", "Status"),
        Some(st),
        None,
        false,
    ));
    let cy = b.add_card(
        Some(tr(
            lang,
            "СИНХРОНИЗАЦИЯ ФАЙЛОВ И ПАПОК",
            "FILE & FOLDER SYNC",
        )),
        false,
        rows,
    );
    let rx = b.content_x + b.content_w as i32 - 14;
    for (idx, item) in sync.items.iter().enumerate() {
        let ry = cy + 50 + (idx as i32) * 46 + 9;
        b.push_btn(
            (rx - 32, ry, 32, 28),
            "×",
            ButtonTone::Danger,
            SettingsAction::RemoveSyncItem(item.id.clone()),
        );
    }
    if has_servers {
        push_server_chips(
            b,
            cy + servers_row_off + 32,
            servers,
            &sync.target_server_ids,
            SettingsAction::ToggleSyncServer,
        );
    }
    let iv = sync.interval_minutes;
    b.add_segmented_row(
        cy + freq_off,
        52,
        vec![
            (
                tr(lang, "Вручн.", "Manual"),
                iv == 0,
                SettingsAction::SetSyncIntervalMin(0),
            ),
            ("15м", iv == 15, SettingsAction::SetSyncIntervalMin(15)),
            ("30м", iv == 30, SettingsAction::SetSyncIntervalMin(30)),
            ("1ч", iv == 60, SettingsAction::SetSyncIntervalMin(60)),
            ("6ч", iv == 360, SettingsAction::SetSyncIntervalMin(360)),
            ("24ч", iv == 1440, SettingsAction::SetSyncIntervalMin(1440)),
        ],
    );
    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 126, 30),
        tr(lang, "+ Файлы", "+ Add Files"),
        ButtonTone::Secondary,
        SettingsAction::PickSyncFiles,
    );
    b.push_btn(
        (x + 134, y, 126, 30),
        tr(lang, "+ Папка", "+ Add Folder"),
        ButtonTone::Secondary,
        SettingsAction::PickSyncFolder,
    );
    b.push_btn(
        (x + 268, y, 198, 30),
        tr(lang, "Синхронизировать сейчас", "Sync Now"),
        ButtonTone::Primary,
        SettingsAction::SyncFilesNow,
    );
    b.cursor_y = y + 42;
}

pub(crate) fn push_server_chips(
    b: &mut LayoutBuilder,
    start_y_log: i32,
    servers: &[WebDavServerView],
    selected: &[String],
    make_action: fn(String) -> SettingsAction,
) {
    let cols = 3u32;
    let gap = 8u32;
    let inner_w = b.content_w.saturating_sub(28);
    let chip_w = (inner_w.saturating_sub(gap * (cols - 1))) / cols;
    for (idx, srv) in servers.iter().enumerate() {
        let col = (idx as u32) % cols;
        let r = (idx as u32) / cols;
        let cx = b.content_x + 14 + (col * (chip_w + gap)) as i32;
        let cy = start_y_log + (r * 34) as i32;
        let active = selected.iter().any(|id| id == &srv.id);
        let lbl = if active {
            format!("✓ {}", srv.name)
        } else {
            srv.name.clone()
        };
        b.push_chip(
            (cx, cy, chip_w, 28),
            &lbl,
            active,
            make_action(srv.id.clone()),
        );
    }
}
