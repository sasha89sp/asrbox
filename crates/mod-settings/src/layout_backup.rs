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
    let lang = b.lang;
    let summary = if bk.encrypt_zip {
        format!("AES-256 · {} WebDAV", bk.servers.len())
    } else {
        format!("ZIP · {} WebDAV", bk.servers.len())
    };
    if !b.accordion(
        "backup_cloud",
        tr(lang, "Бэкап, шифрование AES-256 и WebDAV", "Backup, AES-256 Encryption & WebDAV"),
        &summary,
        "бэкап шифрование пароль aes 256 zip облако webdav яндекс mail.ru сервер папка синхронизация backup encrypt password cloud sync",
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
    let pwd_sub = if bk.has_zip_password {
        tr(
            lang,
            "Пароль сохранён в Windows Credential Manager (WinZip AE-2 AES-256)",
            "Password saved in Credential Manager (WinZip AE-2 AES-256)",
        )
    } else {
        tr(
            lang,
            "Пароль не задан — введите пароль ниже и нажмите «Сохранить»",
            "No password set — enter below and click Save",
        )
    };
    let has_servers = !bk.servers.is_empty();
    let chips_h = if has_servers {
        56 + bk.servers.len().div_ceil(2) as u32 * 34
    } else {
        0
    };
    let mut rows = vec![
        row(
            0,
            50,
            tr(
                lang,
                "Шифровать ZIP паролем (AES-256)",
                "Encrypt ZIP with password (AES-256)",
            ),
            Some(tr(
                lang,
                "Защита архива стандартом WinZip AES-256 (совместимо с 7-Zip / WinRAR)",
                "Protect backup ZIP with WinZip AES-256 (7-Zip / WinRAR compatible)",
            )),
            None,
            true,
        ),
        row(
            50,
            54,
            tr(lang, "Пароль архива ZIP", "ZIP Archive Password"),
            Some(pwd_sub),
            None,
            true,
        ),
        row(
            104,
            50,
            tr(lang, "Автозагрузка в облако", "Auto-upload to cloud"),
            Some(tr(
                lang,
                "Отправлять новый ZIP на выбранные WebDAV серверы после бэкапа",
                "Upload new ZIP archives to selected WebDAV servers after backup",
            )),
            None,
            true,
        ),
    ];
    let mut off = 154i32;
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
        50,
        tr(lang, "Последний бэкап", "Last backup"),
        Some(&last_info),
        None,
        true,
    ));
    rows.push(row(
        off + 50,
        50,
        tr(lang, "Статус облака", "Cloud status"),
        Some(&cloud_info),
        None,
        false,
    ));

    let cy = b.add_card(None, false, rows);
    let tx = b.content_x + b.content_w as i32 - 58;
    let rx = b.content_x + b.content_w as i32 - 14;
    b.push_toggle(
        tx,
        cy + 14,
        bk.encrypt_zip,
        SettingsAction::SetBackupEncryptZip(!bk.encrypt_zip),
    );

    // AES-256 password input (slot 5) + Save / Clear buttons
    let clr_w = if bk.has_zip_password { 76u32 } else { 0 };
    let save_w = 96u32;
    let inp_w = 156u32;
    let total_right = inp_w + 6 + save_w + if clr_w > 0 { 6 + clr_w } else { 0 };
    let px = rx - total_right as i32;
    b.out.input_rects.zip_secret = Some(PixelRect {
        x: px,
        y: b.sy(cy + 63),
        width: inp_w,
        height: 28,
    });
    b.push_btn(
        (px + inp_w as i32 + 6, cy + 63, save_w, 28),
        tr(lang, "Сохранить", "Save"),
        ButtonTone::Primary,
        SettingsAction::SaveBackupZipPassword,
    );
    if clr_w > 0 {
        b.push_btn(
            (
                px + inp_w as i32 + 6 + save_w as i32 + 6,
                cy + 63,
                clr_w,
                28,
            ),
            tr(lang, "Удалить", "Clear"),
            ButtonTone::Danger,
            SettingsAction::ClearBackupZipPassword,
        );
    }

    b.push_toggle(
        tx,
        cy + 118,
        bk.auto_upload_cloud,
        SettingsAction::SetAutoUploadBackup(!bk.auto_upload_cloud),
    );
    if has_servers {
        push_server_chips(
            b,
            cy + 154 + 52,
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
        (b.content_x, y, 254, 30),
        tr(
            lang,
            "+ Добавить новый WebDAV сервер",
            "+ Add New WebDAV Server",
        ),
        ButtonTone::Primary,
        SettingsAction::OpenAddWebDavMenu,
    );
    b.cursor_y = y + 40;

    if let Some(draft) = &bk.draft {
        add_webdav_draft_card(b, draft);
    }
    if bk.servers.is_empty() && bk.draft.is_none() {
        b.add_card(
            None,
            false,
            vec![row(
                0,
                50,
                tr(
                    lang,
                    "Серверы WebDAV ещё не добавлены",
                    "No WebDAV servers added yet",
                ),
                Some(tr(
                    lang,
                    "Добавьте Яндекс.Диск, Mail.ru или свой HTTPS WebDAV сервер",
                    "Add Yandex Disk, Mail.ru or a custom HTTPS WebDAV server",
                )),
                None,
                false,
            )],
        );
        return;
    }
    for srv in &bk.servers {
        add_saved_server_card(b, srv, editing_server_dir == Some(srv.id.as_str()));
    }
}

fn add_saved_server_card(b: &mut LayoutBuilder, srv: &WebDavServerView, editing_dir: bool) {
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
    let dir_sub = (!editing_dir).then_some(dir_display.as_str());
    let rows = vec![
        row(0, 50, &srv.name, Some(&sub1), None, true),
        row(
            50,
            50,
            tr(lang, "Путь хранения на сервере", "Remote storage path"),
            dir_sub,
            None,
            true,
        ),
        row(
            100,
            50,
            tr(lang, "Обзор папок WebDAV", "WebDAV Folder Browser"),
            Some(tr(
                lang,
                "Просмотр каталогов, создание папок и выбор папки",
                "Browse directories, create folders and pick path",
            )),
            None,
            false,
        ),
    ];
    let cy = b.add_card(None, false, rows);
    let rx = b.content_x + b.content_w as i32 - 14;
    b.push_btn(
        (rx - 80, cy + 11, 80, 28),
        tr(lang, "Удалить", "Delete"),
        ButtonTone::Danger,
        SettingsAction::DeleteWebDavServer(srv.id.clone()),
    );
    b.push_btn(
        (rx - 154, cy + 11, 68, 28),
        tr(lang, "Тест", "Test"),
        ButtonTone::Secondary,
        SettingsAction::TestWebDavServer(srv.id.clone()),
    );
    if editing_dir {
        let inp_x = b.content_x + 178;
        let inp_w = b.content_w.saturating_sub(364).max(120);
        b.out.input_rects.aux = Some(PixelRect {
            x: inp_x,
            y: b.sy(cy + 60),
            width: inp_w,
            height: 28,
        });
        b.push_btn(
            (inp_x + inp_w as i32 + 6, cy + 60, 92, 28),
            tr(lang, "Сохранить", "Save"),
            ButtonTone::Primary,
            SettingsAction::SaveWebDavServerRemoteDir(srv.id.clone()),
        );
        b.push_control(
            PixelRect {
                x: inp_x + inp_w as i32 + 104,
                y: b.sy(cy + 60),
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
                y: b.sy(cy + 60),
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
        (rx - 148, cy + 111, 148, 28),
        tr(lang, "Открыть папку →", "Open Folder →"),
        ButtonTone::Secondary,
        SettingsAction::OpenWebDavBrowser(srv.id.clone()),
    );
}
