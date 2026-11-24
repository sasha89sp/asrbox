#![cfg(windows)]

use asrbox_mod_backup::save_backup_config;
use asrbox_mod_webdav::{
    normalize_remote_dir, save_sync_config, save_webdav_store, validate_https_url, WebDavProvider,
    WebDavServer,
};
use asrbox_platform_api::{
    AppLanguage, FluentMenuItem, FluentMenuSpec, FluentPalette, ToolsPlatform,
};

use crate::backup_ctl::{BackupController, WebDavDraftState};

impl BackupController {
    pub(crate) fn open_add_webdav_menu(
        &mut self,
        host: &mut dyn ToolsPlatform,
        palette: FluentPalette,
        lang: AppLanguage,
    ) -> Result<bool, String> {
        let spec = FluentMenuSpec {
            quick_actions: Vec::new(),
            min_width: 248,
            items: vec![
                FluentMenuItem::Action {
                    id: 1,
                    icon: Some("☁".to_owned()),
                    label: WebDavProvider::Yandex.label_ru().to_owned(),
                    shortcut: Some("webdav.yandex.ru".to_owned()),
                    checked: false,
                    enabled: true,
                },
                FluentMenuItem::Action {
                    id: 2,
                    icon: Some("☁".to_owned()),
                    label: WebDavProvider::MailRu.label_ru().to_owned(),
                    shortcut: Some("webdav.cloud.mail.ru".to_owned()),
                    checked: false,
                    enabled: true,
                },
                FluentMenuItem::Separator,
                FluentMenuItem::Action {
                    id: 3,
                    icon: Some("⚙".to_owned()),
                    label: match lang {
                        AppLanguage::Ru => "Свой WebDAV (HTTPS)",
                        AppLanguage::En => "Custom WebDAV (HTTPS)",
                    }
                    .to_owned(),
                    shortcut: Some("https://...".to_owned()),
                    checked: false,
                    enabled: true,
                },
            ],
        };
        let pt = host.cursor_screen_pos();
        let Some(choice) = host.show_fluent_popup_menu(pt, &spec, palette)? else {
            return Ok(false);
        };
        let provider = match choice {
            2 => WebDavProvider::MailRu,
            3 => WebDavProvider::Custom,
            _ => WebDavProvider::Yandex,
        };
        self.draft = Some(WebDavDraftState {
            provider,
            status_text: String::new(),
        });
        host.set_settings_input_text(0, "");
        host.set_settings_input_text(1, "");
        host.set_settings_input_text(2, "");
        Ok(true)
    }

    pub(crate) fn test_draft(&mut self, host: &dyn ToolsPlatform) -> Result<String, String> {
        let Some(draft) = self.draft.as_mut() else {
            return Err("нет активной формы добавления сервера".to_owned());
        };
        let url = if draft.provider == WebDavProvider::Custom {
            host.settings_input_text(2).trim().to_owned()
        } else {
            draft.provider.preset_url().to_owned()
        };
        validate_https_url(&url)?;
        let user = host.settings_input_text(0).trim().to_owned();
        let pass = host.settings_input_text(1).trim().to_owned();
        if user.is_empty() || pass.is_empty() {
            return Err("укажите логин и пароль приложения".to_owned());
        }
        let op_id = self.alloc_op_id();
        host.webdav_test(op_id, "__draft__", &url, &user, &pass)?;
        self.pending_test_op = Some(op_id);
        if let Some(d) = self.draft.as_mut() {
            d.status_text = "Проверка подключения...".to_owned();
        }
        Ok(format!("WebDAV тест (черновик) -> {url}"))
    }

    pub(crate) fn save_draft(&mut self, host: &dyn ToolsPlatform) -> Result<String, String> {
        let Some(draft) = self.draft.as_ref() else {
            return Err("нет активной формы добавления сервера".to_owned());
        };
        let provider = draft.provider;
        let custom_url = if provider == WebDavProvider::Custom {
            host.settings_input_text(2).trim().to_owned()
        } else {
            String::new()
        };
        let user = host.settings_input_text(0).trim().to_owned();
        let pass = host.settings_input_text(1).trim().to_owned();
        let id = self.webdav_store.next_server_id();
        let name = if user.is_empty() {
            provider.label_ru().to_owned()
        } else {
            format!("{} ({user})", provider.label_ru())
        };
        let srv = WebDavServer {
            id: id.clone(),
            name,
            provider,
            custom_url,
            username: user.clone(),
            remote_dir: "/asrbox-backups".to_owned(),
            legacy_cred_target: None,
        };
        validate_https_url(&srv.effective_url())?;
        if user.is_empty() {
            return Err("укажите логин/email WebDAV".to_owned());
        }
        if !pass.is_empty() {
            host.cred_save(&srv.cred_target(), &user, &pass)?;
        }
        self.webdav_store.servers.push(srv);
        save_webdav_store(&self.webdav_cfg_path, &self.webdav_store)?;
        if self.backup_cfg.target_server_ids.is_empty() {
            self.backup_cfg.target_server_ids.push(id.clone());
            let _ = save_backup_config(&self.backup_cfg_path, &self.backup_cfg);
        }
        self.draft = None;
        host.set_settings_input_text(0, "");
        host.set_settings_input_text(1, "");
        host.set_settings_input_text(2, "");
        self.refresh_all_password_flags(host);
        Ok(format!("Добавлен WebDAV сервер [{id}]"))
    }

    pub(crate) fn cancel_draft(&mut self, host: &dyn ToolsPlatform) {
        self.draft = None;
        host.set_settings_input_text(0, "");
        host.set_settings_input_text(1, "");
        host.set_settings_input_text(2, "");
    }

    pub(crate) fn test_server(
        &mut self,
        host: &dyn ToolsPlatform,
        server_id: &str,
    ) -> Result<String, String> {
        let (base_url, user, pass) = self.resolve_server_creds(host, server_id)?;
        let op_id = self.alloc_op_id();
        host.webdav_test(op_id, server_id, &base_url, &user, &pass)?;
        self.pending_test_op = Some(op_id);
        self.set_server_status(server_id, "Проверка...".to_owned());
        Ok(format!("WebDAV тест [{server_id}] -> {base_url}"))
    }

    pub(crate) fn delete_server(
        &mut self,
        host: &dyn ToolsPlatform,
        server_id: &str,
    ) -> Result<String, String> {
        if let Some(srv) = self.webdav_store.find(server_id) {
            let _ = host.cred_delete(&srv.cred_target());
        }
        self.webdav_store.servers.retain(|s| s.id != server_id);
        save_webdav_store(&self.webdav_cfg_path, &self.webdav_store)?;
        self.backup_cfg
            .target_server_ids
            .retain(|id| id != server_id);
        let _ = save_backup_config(&self.backup_cfg_path, &self.backup_cfg);
        self.sync_cfg.target_server_ids.retain(|id| id != server_id);
        let _ = save_sync_config(&self.sync_cfg_path, &self.sync_cfg);
        self.remove_server_meta(server_id);
        if self
            .browser
            .as_ref()
            .is_some_and(|b| b.server_id == server_id)
        {
            self.browser = None;
        }
        Ok(format!("Удалён WebDAV сервер [{server_id}]"))
    }

    pub(crate) fn save_server_remote_dir(
        &mut self,
        host: &dyn ToolsPlatform,
        server_id: &str,
    ) -> Result<String, String> {
        let raw = host.settings_input_text(4);
        let norm = normalize_remote_dir(&raw);
        let Some(srv) = self.webdav_store.find_mut(server_id) else {
            return Err(format!("сервер {server_id} не найден"));
        };
        srv.remote_dir = norm.clone();
        save_webdav_store(&self.webdav_cfg_path, &self.webdav_store)?;
        self.set_server_status(server_id, format!("Путь: {norm}"));
        Ok(format!("WebDAV [{server_id}] путь хранения -> {norm}"))
    }
}
