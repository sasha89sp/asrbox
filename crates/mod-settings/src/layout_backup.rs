use asrbox_platform_api::PixelRect;

use crate::layout::{ButtonTone, LayoutBuilder};
use crate::layout_tabs::row;
use crate::types::{BackupSettingsView, SettingsAction, SettingsSnapshot, WebDavSettingsView};

pub(crate) fn build_backup_tab(b: &mut LayoutBuilder, snap: &SettingsSnapshot) {
    b.set_header(
        "Бэкап и Облако WebDAV",
        "Локальные ZIP-архивы настроек или всей программы с загрузкой на Яндекс.Диск и Mail.ru",
    );
    add_backup_card(b, &snap.backup);
    add_webdav_card(b, &snap.webdav);
}

fn add_backup_card(b: &mut LayoutBuilder, bk: &BackupSettingsView) {
    let last_info = if bk.last_backup_file.is_empty() {
        "Архивы ещё не создавались (папка data/backups)".to_owned()
    } else if bk.last_backup_time.is_empty() {
        bk.last_backup_file.clone()
    } else {
        format!("{} · {}", bk.last_backup_file, bk.last_backup_time)
    };
    let cloud_info = if bk.busy {
        "Выполняется загрузка в облако...".to_owned()
    } else if bk.cloud_status.is_empty() {
        "Готово к загрузке (вручную или автоматически после бэкапа)".to_owned()
    } else {
        bk.cloud_status.clone()
    };
    let rows = vec![
        row(
            0,
            48,
            "Автозагрузка в облако",
            Some("Сразу отправлять новый ZIP на WebDAV после бэкапа"),
            None,
            true,
        ),
        row(48, 46, "Последний бэкап", Some(&last_info), None, true),
        row(94, 46, "Статус облака", Some(&cloud_info), None, false),
    ];
    let cy = b.add_card(Some("РЕЗЕРВНОЕ КОПИРОВАНИЕ"), false, rows);
    let tx = b.content_x + b.content_w as i32 - 60;
    b.push_toggle(
        tx,
        cy + 12,
        bk.auto_upload_cloud,
        SettingsAction::SetAutoUploadBackup(!bk.auto_upload_cloud),
    );
    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 134, 30),
        "Бэкап настроек",
        ButtonTone::Primary,
        SettingsAction::BackupSettingsNow,
    );
    b.push_btn(
        (x + 142, y, 158, 30),
        "Программа + настр.",
        ButtonTone::Secondary,
        SettingsAction::BackupFullNow,
    );
    b.push_btn(
        (x + 308, y, 136, 30),
        "Загрузить в облако",
        ButtonTone::Amber,
        SettingsAction::UploadLatestBackupNow,
    );
    b.push_btn(
        (x + 452, y, 118, 30),
        "Папка backups",
        ButtonTone::Secondary,
        SettingsAction::OpenBackupsFolder,
    );
    b.cursor_y = y + 40;
}

fn add_webdav_card(b: &mut LayoutBuilder, wd: &WebDavSettingsView) {
    let is_custom = wd.provider_index == 2;
    let url_sub = if is_custom {
        None
    } else {
        Some(wd.effective_url.as_str())
    };
    let pass_sub = if wd.has_saved_password {
        "Сохранён в Windows Credential Manager"
    } else {
        "Пароль приложения (только в Credential Manager)"
    };
    let status = if wd.status_text.is_empty() {
        format!("Папка в облаке: /{}", wd.remote_dir.trim_matches('/'))
    } else {
        wd.status_text.clone()
    };
    let rows = vec![
        row(0, 46, "Провайдер WebDAV", None, None, true),
        row(46, 46, "Сервер HTTPS", url_sub, None, true),
        row(92, 46, "Логин / Email", None, None, true),
        row(138, 50, "Пароль приложения", Some(pass_sub), None, true),
        row(188, 46, "Состояние WebDAV", Some(&status), None, false),
    ];
    let cy = b.add_card(Some("ОБЛАКО WEBDAV (ЯНДЕКС.ДИСК / MAIL.RU)"), false, rows);
    b.add_segmented_row(
        cy,
        46,
        vec![
            (
                "Яндекс.Диск",
                wd.provider_index == 0,
                SettingsAction::SetWebDavProvider(0),
            ),
            (
                "Mail.ru",
                wd.provider_index == 1,
                SettingsAction::SetWebDavProvider(1),
            ),
            (
                "Свой",
                wd.provider_index == 2,
                SettingsAction::SetWebDavProvider(2),
            ),
        ],
    );
    let inp_x = b.content_x + 178;
    let inp_w = b.content_w.saturating_sub(194).max(160);
    if is_custom {
        b.out.input_rects.extra = Some(PixelRect {
            x: inp_x,
            y: b.sy(cy + 55),
            width: inp_w,
            height: 28,
        });
    }
    b.out.input_rects.primary = Some(PixelRect {
        x: inp_x,
        y: b.sy(cy + 101),
        width: inp_w,
        height: 28,
    });
    b.out.input_rects.secret = Some(PixelRect {
        x: inp_x,
        y: b.sy(cy + 149),
        width: inp_w,
        height: 28,
    });

    let (x, y) = (b.content_x, b.cursor_y);
    b.push_btn(
        (x, y, 146, 30),
        "Сохранить WebDAV",
        ButtonTone::Primary,
        SettingsAction::SaveWebDavSettings,
    );
    b.push_btn(
        (x + 154, y, 152, 30),
        "Проверить связь",
        ButtonTone::Secondary,
        SettingsAction::TestWebDavConnection,
    );
    b.push_btn(
        (x + 314, y, 142, 30),
        "Удалить пароль",
        ButtonTone::Danger,
        SettingsAction::DeleteWebDavPassword,
    );
    b.cursor_y += 40;
}
