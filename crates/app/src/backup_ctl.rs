#![cfg(windows)]

use std::path::{Path, PathBuf};

use asrbox_mod_about::AppPaths;
use asrbox_mod_backup::{
    create_backup, find_latest_backup, load_backup_config, save_backup_config, BackupConfig,
    BackupMode,
};
use asrbox_mod_settings::{BackupSettingsView, WebDavSettingsView};
use asrbox_mod_webdav::{
    load_webdav_config, save_webdav_config, validate_https_url, WebDavConfig, WebDavProvider,
};
use asrbox_platform_api::{LocalDateTime, ToolsPlatform, WebDavResult};

pub(crate) struct BackupController {
    exe_path: PathBuf,
    config_dir: PathBuf,
    backups_dir: PathBuf,
    backup_cfg_path: PathBuf,
    webdav_cfg_path: PathBuf,
    backup_cfg: BackupConfig,
    webdav_cfg: WebDavConfig,
    next_op_id: u64,
    pending_test_op: Option<u64>,
    pending_upload_op: Option<u64>,
    has_saved_password: bool,
    webdav_status: String,
}

impl BackupController {
    pub(crate) fn load(paths: &AppPaths, host: &dyn ToolsPlatform) -> Self {
        let backup_cfg = load_backup_config(&paths.backup_cfg);
        let webdav_cfg = load_webdav_config(&paths.webdav_cfg);
        let has_saved_password = host
            .cred_load(webdav_cfg.cred_target())
            .ok()
            .flatten()
            .is_some();
        Self {
            exe_path: paths.exe_path.clone(),
            config_dir: paths.data_dir.join("config"),
            backups_dir: paths.backups_dir.clone(),
            backup_cfg_path: paths.backup_cfg.clone(),
            webdav_cfg_path: paths.webdav_cfg.clone(),
            backup_cfg,
            webdav_cfg,
            next_op_id: 1,
            pending_test_op: None,
            pending_upload_op: None,
            has_saved_password,
            webdav_status: String::new(),
        }
    }

    pub(crate) fn webdav_username(&self) -> &str {
        &self.webdav_cfg.username
    }

    pub(crate) fn webdav_custom_url(&self) -> &str {
        &self.webdav_cfg.custom_url
    }

    pub(crate) fn backup_view(&self) -> BackupSettingsView {
        BackupSettingsView {
            auto_upload_cloud: self.backup_cfg.auto_upload_cloud,
            last_backup_file: self.backup_cfg.last_backup_file.clone(),
            last_backup_time: self.backup_cfg.last_backup_time.clone(),
            cloud_status: self.backup_cfg.last_cloud_status.clone(),
            busy: self.pending_upload_op.is_some() || self.pending_test_op.is_some(),
        }
    }

    pub(crate) fn webdav_view(&self) -> WebDavSettingsView {
        WebDavSettingsView {
            provider_index: self.webdav_cfg.provider.index(),
            effective_url: self.webdav_cfg.effective_url(),
            username: self.webdav_cfg.username.clone(),
            remote_dir: self.webdav_cfg.normalized_remote_dir(),
            has_saved_password: self.has_saved_password,
            status_text: self.webdav_status.clone(),
        }
    }

    pub(crate) fn set_auto_upload(&mut self, enabled: bool) {
        self.backup_cfg.auto_upload_cloud = enabled;
        let _ = save_backup_config(&self.backup_cfg_path, &self.backup_cfg);
    }

    pub(crate) fn set_provider(&mut self, idx: u8, host: &dyn ToolsPlatform) {
        let next = WebDavProvider::from_index(idx);
        if self.webdav_cfg.provider != next {
            self.webdav_cfg.provider = next;
            let _ = save_webdav_config(&self.webdav_cfg_path, &self.webdav_cfg);
            self.refresh_saved_password_flag(host);
            self.webdav_status = format!("Выбран провайдер: {}", next.label_ru());
        }
    }

    pub(crate) fn run_backup(
        &mut self,
        host: &dyn ToolsPlatform,
        mode: BackupMode,
        now: LocalDateTime,
    ) -> Result<String, String> {
        let art = create_backup(
            mode,
            &self.exe_path,
            &self.config_dir,
            &self.backups_dir,
            now,
        )?;
        self.backup_cfg.last_backup_file = art.file_name.clone();
        self.backup_cfg.last_backup_time = format!(
            "{:02}.{:02}.{:04} {:02}:{:02}",
            now.day, now.month, now.year, now.hour, now.minute
        );
        let kb = art.bytes.div_ceil(1024);
        let mut summary = format!(
            "Бэкап ({}) создан: {} ({} КБ, файлов: {})",
            mode.label_ru(),
            art.file_name,
            kb,
            art.entry_count
        );
        if self.backup_cfg.auto_upload_cloud {
            match self.start_upload_file(host, &art.path, &art.file_name) {
                Ok(msg) => {
                    summary.push_str(" · ");
                    summary.push_str(&msg);
                }
                Err(err) => {
                    self.backup_cfg.last_cloud_status = format!("Автозагрузка пропущена: {err}");
                }
            }
        }
        let _ = save_backup_config(&self.backup_cfg_path, &self.backup_cfg);
        Ok(summary)
    }

    pub(crate) fn upload_latest(&mut self, host: &dyn ToolsPlatform) -> Result<String, String> {
        let candidate = if !self.backup_cfg.last_backup_file.is_empty() {
            let p = self.backups_dir.join(&self.backup_cfg.last_backup_file);
            if p.is_file() {
                Some(p)
            } else {
                find_latest_backup(&self.backups_dir)
            }
        } else {
            find_latest_backup(&self.backups_dir)
        };
        let Some(path) = candidate else {
            return Err("сначала создайте локальный бэкап".to_owned());
        };
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("asrbox-backup.zip")
            .to_owned();
        let msg = self.start_upload_file(host, &path, &name)?;
        let _ = save_backup_config(&self.backup_cfg_path, &self.backup_cfg);
        Ok(msg)
    }

    fn start_upload_file(
        &mut self,
        host: &dyn ToolsPlatform,
        local_path: &Path,
        remote_name: &str,
    ) -> Result<String, String> {
        if self.pending_upload_op.is_some() {
            return Err("загрузка в облако уже выполняется".to_owned());
        }
        let (base_url, user, pass) = self.resolve_credentials(host, None)?;
        let op_id = self.alloc_op_id();
        let remote_dir = self.webdav_cfg.normalized_remote_dir();
        host.webdav_upload(
            op_id,
            &base_url,
            &remote_dir,
            remote_name,
            local_path,
            &user,
            &pass,
        )?;
        self.pending_upload_op = Some(op_id);
        self.backup_cfg.last_cloud_status = format!("Загрузка {remote_name}...");
        Ok(format!(
            "Загрузка {remote_name} -> {}",
            self.webdav_cfg.provider.label_ru()
        ))
    }

    pub(crate) fn save_webdav(&mut self, host: &dyn ToolsPlatform) -> Result<String, String> {
        self.read_inputs_into_config(host);
        let url = self.webdav_cfg.effective_url();
        validate_https_url(&url)?;
        if self.webdav_cfg.username.trim().is_empty() {
            return Err("укажите логин/email WebDAV".to_owned());
        }
        save_webdav_config(&self.webdav_cfg_path, &self.webdav_cfg)?;
        let pass = host.settings_input_text(1);
        let pass_trimmed = pass.trim();
        if !pass_trimmed.is_empty() {
            host.cred_save(
                self.webdav_cfg.cred_target(),
                self.webdav_cfg.username.trim(),
                pass_trimmed,
            )?;
            host.set_settings_input_text(1, "");
        }
        self.refresh_saved_password_flag(host);
        let note = if self.has_saved_password {
            "Настройки и пароль сохранены (Credential Manager)"
        } else {
            "Настройки сохранены (пароль ещё не задан)"
        };
        self.webdav_status = note.to_owned();
        Ok(format!(
            "WebDAV [{}]: {note}",
            self.webdav_cfg.provider.label_ru()
        ))
    }

    pub(crate) fn test_webdav(&mut self, host: &dyn ToolsPlatform) -> Result<String, String> {
        if self.pending_test_op.is_some() {
            return Err("проверка подключения уже выполняется".to_owned());
        }
        self.read_inputs_into_config(host);
        let typed = host.settings_input_text(1);
        let typed_opt = (!typed.trim().is_empty()).then_some(typed.trim().to_owned());
        let (base_url, user, pass) = self.resolve_credentials(host, typed_opt)?;
        let op_id = self.alloc_op_id();
        host.webdav_test(op_id, &base_url, &user, &pass)?;
        self.pending_test_op = Some(op_id);
        self.webdav_status = "Проверка подключения...".to_owned();
        Ok(format!(
            "WebDAV тест -> {}",
            self.webdav_cfg.provider.label_ru()
        ))
    }

    pub(crate) fn delete_password(&mut self, host: &dyn ToolsPlatform) -> Result<String, String> {
        host.cred_delete(self.webdav_cfg.cred_target())?;
        host.set_settings_input_text(1, "");
        self.has_saved_password = false;
        self.webdav_status = "Пароль удалён из Windows Credential Manager".to_owned();
        Ok(self.webdav_status.clone())
    }

    pub(crate) fn on_webdav_done(&mut self, res: WebDavResult) -> String {
        if self.pending_test_op == Some(res.op_id) {
            self.pending_test_op = None;
            self.webdav_status = res.message.clone();
            return format!("WebDAV тест: {}", res.message);
        }
        if self.pending_upload_op == Some(res.op_id) {
            self.pending_upload_op = None;
            self.backup_cfg.last_cloud_status = res.message.clone();
            let _ = save_backup_config(&self.backup_cfg_path, &self.backup_cfg);
            return format!("WebDAV загрузка: {}", res.message);
        }
        format!("WebDAV: {}", res.message)
    }

    fn read_inputs_into_config(&mut self, host: &dyn ToolsPlatform) {
        let user = host.settings_input_text(0);
        if !user.trim().is_empty() {
            self.webdav_cfg.username = user.trim().to_owned();
        }
        if self.webdav_cfg.provider == WebDavProvider::Custom {
            let custom = host.settings_input_text(2);
            if !custom.trim().is_empty() {
                self.webdav_cfg.custom_url = custom.trim().to_owned();
            }
        }
    }

    fn resolve_credentials(
        &self,
        host: &dyn ToolsPlatform,
        override_pass: Option<String>,
    ) -> Result<(String, String, String), String> {
        let base_url = self.webdav_cfg.effective_url();
        validate_https_url(&base_url)?;
        let saved = host.cred_load(self.webdav_cfg.cred_target())?;
        let user = if !self.webdav_cfg.username.trim().is_empty() {
            self.webdav_cfg.username.trim().to_owned()
        } else if let Some((saved_user, _)) = &saved {
            saved_user.clone()
        } else {
            return Err("не задан логин WebDAV".to_owned());
        };
        let pass = if let Some(p) = override_pass {
            p
        } else if let Some((_, saved_pass)) = saved {
            saved_pass
        } else {
            return Err("не сохранён пароль приложения WebDAV".to_owned());
        };
        Ok((base_url, user, pass))
    }

    fn refresh_saved_password_flag(&mut self, host: &dyn ToolsPlatform) {
        self.has_saved_password = host
            .cred_load(self.webdav_cfg.cred_target())
            .ok()
            .flatten()
            .is_some();
    }

    fn alloc_op_id(&mut self) -> u64 {
        let id = self.next_op_id;
        self.next_op_id = self.next_op_id.saturating_add(1);
        id
    }
}
