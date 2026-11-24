use asrbox_platform_api::PixelRect;

use crate::i18n::tr;
use crate::layout::{ButtonTone, HitKind, LayoutBuilder};
use crate::layout_sync::{add_file_sync_card, add_webdav_draft_card, push_server_chips};
use crate::layout_tabs::row;
use crate::types::{BackupSettingsView, SettingsAction, WebDavServerView};

pub(crate) fn build_backup_and_webdav_section(
    b: &mut LayoutBuilder,
    bk: &BackupSettingsView,
    editing_server_dir: Option<&str>,
) {
    if !b.matches_search(
        "бэкап облако webdav яндекс mail.ru сервер папка синхронизация backup cloud sync",
    ) {
        return;
    }
    add_backup_card(b, bk);
    add_webdav_servers_block(b, bk, editing_server_dir);
    add_file_sync_card(b, &bk.sync, &bk.servers);
}

fn add_backup_card(b: &mut LayoutBuilder, bk: &BackupSettingsView) {
    let lang = b.lang;
    let last_info = if bk.last_backup_file.is_empty() {
        tr(
            lang,
            "Архивы ещё не создавались (data/backups)",
            "No backups created yet (data/backups)",
        )
        .to_owned()
    } else if bk.last_backup_time.is_empty() {
        bk.last_backup_file.clone()
    } else {
        format!("{} · {}", bk.last_backup_file, bk.last_backup_time)
    };
    let cloud_info = if bk.busy {
        tr(
            lang,
            "Выполняется загрузка в облако...",
            "Uploading to cloud...",
        )
        .to_owned()
    } else if bk.cloud_status.is_empty() {
        tr(
            lang,
            "Готово к загрузке на выбранные WebDAV серверы",
            "Ready to upload to selected WebDAV servers",
        )
        .to_owned()
    } else {
        bk.cloud_status.clone()
    };
    let has_servers = !bk.servers.is_empty();
    let chip_rows = if has_servers {
        bk.servers.len().div_ceil(2) as u32
    } else {
        0
    };
    let chips_h = if has_servers { 56 + chip_rows * 34 } else { 0 };
    let mut rows = vec![row(
        0,
        52,
        tr(lang, "Автозагрузка в облако", "Auto-upload to cloud"),
        Some(tr(
            lang,
            "Отправлять новый ZIP на выбранные WebDAV серверы после бэкапа",
            "Upload new ZIP archives to selected WebDAV servers after backup",
        )),
        None,
        true,
    )];
    let mut off = 52i32;
    if has_servers {
        rows.push(row(
            off,
            chips_h,
            tr(lang, "Серверы для загрузки бэкапа", "Target WebDAV servers"),
            Some(tr(
                lang,
                "Можно выбрать один или несколько серверов",
                "Select one or multiple servers",
            )),
            None,
            true,
        ));
        off += chips_h as i32;
    }
    rows.push(row(
        off,
        52,
        tr(lang, "Последний бэкап", "Last backup"),
        Some(&last_info),
        None,
        true,
    ));
    rows.push(row(
        off + 52,
        52,
        tr(lang, "Статус облака", "Cloud status"),
        Some(&cloud_info),
        None,
        false,
    ));
    let cy = b.add_card(
        Some(tr(
            lang,
            "3. БЭКАП И ОБЛАКО WEBDAV",
            "3. BACKUP & WEBDAV CLOUD",
        )),
        false,
        rows,
    );
    let tx = b.content_x + b.content_w as i32 - 58;
    b.push_toggle(
        tx,
        cy + 15,
        bk.auto_upload_cloud,
        SettingsAction::SetAutoUploadBackup(!bk.auto_upload_cloud),
    );
    if has_servers {
        push_server_chips(
            b,
            cy + 52 + 52,
            &bk.servers,
            &bk.target_server_ids,
            SettingsAction::ToggleBackupTargetServer,
        );
    }
    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 134, 30),
        tr(lang, "Бэкап настроек", "Backup Settings"),
        ButtonTone::Primary,
        SettingsAction::BackupSettingsNow,
    );
    b.push_btn(
        (x + 142, y, 152, 30),
        tr(lang, "Программа + настр.", "Full Backup"),
        ButtonTone::Secondary,
        SettingsAction::BackupFullNow,
    );
    b.push_btn(
        (x + 302, y, 140, 30),
        tr(lang, "Загрузить в облако", "Upload Now"),
        ButtonTone::Amber,
        SettingsAction::UploadLatestBackupNow,
    );
    b.push_btn(
        (x + 450, y, 118, 30),
        tr(lang, "Папка backups", "Backups Dir"),
        ButtonTone::Secondary,
        SettingsAction::OpenBackupsFolder,
    );
    b.cursor_y = y + 40;
}

fn add_webdav_servers_block(
    b: &mut LayoutBuilder,
    bk: &BackupSettingsView,
    editing_server_dir: Option<&str>,
) {
    let lang = b.lang;
    let y = b.cursor_y;
    b.push_btn(
        (b.content_x, y, 254, 32),
        tr(
            lang,
            "+ Добавить новый WebDAV сервер",
            "+ Add New WebDAV Server",
        ),
        ButtonTone::Primary,
        SettingsAction::OpenAddWebDavMenu,
    );
    b.cursor_y = y + 44;

    if let Some(draft) = &bk.draft {
        add_webdav_draft_card(b, draft);
    }
    if bk.servers.is_empty() && bk.draft.is_none() {
        b.add_card(
            Some(tr(lang, "СОХРАНЁННЫЕ WEBDAV СЕРВЕРЫ", "SAVED WEBDAV SERVERS")),
            false,
            vec![row(
                0,
                52,
                tr(lang, "Серверы ещё не добавлены", "No WebDAV servers added yet"),
                Some(tr(
                    lang,
                    "Нажмите «+ Добавить новый WebDAV сервер», выберите Яндекс.Диск, Mail.ru или свой сервер",
                    "Click «+ Add New WebDAV Server» to add Yandex Disk, Mail.ru or a custom HTTPS server",
                )),
                None,
                false,
            )],
        );
        return;
    }
    for (idx, srv) in bk.servers.iter().enumerate() {
        let title = (idx == 0).then_some(tr(
            lang,
            "СОХРАНЁННЫЕ WEBDAV СЕРВЕРЫ",
            "SAVED WEBDAV SERVERS",
        ));
        add_saved_server_card(b, title, srv, editing_server_dir == Some(srv.id.as_str()));
    }
}

fn add_saved_server_card(
    b: &mut LayoutBuilder,
    card_title: Option<&str>,
    srv: &WebDavServerView,
    editing_dir: bool,
) {
    let lang = b.lang;
    let cred_tag = if srv.has_saved_password {
        tr(lang, "пароль сохранён", "password saved")
    } else {
        tr(lang, "нет пароля", "no password")
    };
    let sub1 = if srv.status_text.is_empty() {
        format!("{} · {} ({cred_tag})", srv.provider_label, srv.username)
    } else {
        format!(
            "{} · {} · {}",
            srv.provider_label, srv.username, srv.status_text
        )
    };
    let dir_display = format!("/{}", srv.remote_dir.trim_matches('/'));
    let dir_sub = if editing_dir {
        None
    } else {
        Some(dir_display.as_str())
    };
    let rows = vec![
        row(0, 52, &srv.name, Some(&sub1), None, true),
        row(
            52,
            52,
            tr(lang, "Путь хранения на сервере", "Remote storage path"),
            dir_sub,
            None,
            true,
        ),
        row(
            104,
            52,
            tr(lang, "Обзор папок WebDAV", "WebDAV Folder Browser"),
            Some(tr(
                lang,
                "Просмотр каталогов, создание папок и выбор папки хранения",
                "Browse directories, create folders and pick storage folder",
            )),
            None,
            false,
        ),
    ];
    let cy = b.add_card(card_title, false, rows);
    let rx = b.content_x + b.content_w as i32 - 14;
    b.push_btn(
        (rx - 80, cy + 12, 80, 28),
        tr(lang, "Удалить", "Delete"),
        ButtonTone::Danger,
        SettingsAction::DeleteWebDavServer(srv.id.clone()),
    );
    b.push_btn(
        (rx - 154, cy + 12, 68, 28),
        tr(lang, "Тест", "Test"),
        ButtonTone::Secondary,
        SettingsAction::TestWebDavServer(srv.id.clone()),
    );
    if editing_dir {
        let inp_x = b.content_x + 178;
        let inp_w = b.content_w.saturating_sub(364).max(120);
        b.out.input_rects.aux = Some(PixelRect {
            x: inp_x,
            y: b.sy(cy + 62),
            width: inp_w,
            height: 28,
        });
        b.push_btn(
            (inp_x + inp_w as i32 + 6, cy + 62, 92, 28),
            tr(lang, "Сохранить", "Save"),
            ButtonTone::Primary,
            SettingsAction::SaveWebDavServerRemoteDir(srv.id.clone()),
        );
        b.push_control(
            PixelRect {
                x: inp_x + inp_w as i32 + 104,
                y: b.sy(cy + 62),
                width: 66,
                height: 28,
            },
            HitKind::CancelEditServerDir {
                label: tr(lang, "Отмена", "Cancel").to_owned(),
            },
        );
    } else {
        b.push_control(
            PixelRect {
                x: rx - 124,
                y: b.sy(cy + 62),
                width: 124,
                height: 28,
            },
            HitKind::StartEditServerDir {
                server_id: srv.id.clone(),
                label: tr(lang, "Изменить путь", "Edit Path").to_owned(),
            },
        );
    }
    b.push_btn(
        (rx - 148, cy + 116, 148, 28),
        tr(lang, "Открыть папку →", "Open Folder →"),
        ButtonTone::Secondary,
        SettingsAction::OpenWebDavBrowser(srv.id.clone()),
    );
}
