#![cfg(windows)]

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::time::Instant;

use asrbox_mod_about::AppPaths;
use asrbox_mod_backup::{
    create_backup, find_latest_backup, load_backup_config, save_backup_config, BackupConfig,
    BackupMode,
};
use asrbox_mod_settings::{
    BackupSettingsView, WebDavBrowserView, WebDavDraftView, WebDavServerView,
};
use asrbox_mod_webdav::{
    load_sync_config, load_webdav_store, save_webdav_store, validate_https_url, FileSyncConfig,
    WebDavProvider, WebDavStore,
};
use asrbox_platform_api::{
    AppLanguage, LocalDateTime, ToolsPlatform, WebDavDirEntry, WebDavOpKind, WebDavResult,
};

pub(crate) struct WebDavDraftState {
    pub provider: WebDavProvider,
    pub status_text: String,
}

pub(crate) struct WebDavBrowserState {
    pub server_id: String,
    pub server_name: String,
    pub current_dir: String,
    pub entries: Vec<WebDavDirEntry>,
    pub loading: bool,
    pub status_text: String,
}

pub(crate) struct BackupController {
    exe_path: PathBuf,
    config_dir: PathBuf,
    backups_dir: PathBuf,
    pub(crate) backup_cfg_path: PathBuf,
    pub(crate) webdav_cfg_path: PathBuf,
    pub(crate) sync_cfg_path: PathBuf,
    pub(crate) backup_cfg: BackupConfig,
    pub(crate) webdav_store: WebDavStore,
    pub(crate) sync_cfg: FileSyncConfig,
    saved_passwords: HashMap<String, bool>,
    server_statuses: HashMap<String, String>,
    pub(crate) draft: Option<WebDavDraftState>,
    pub(crate) browser: Option<WebDavBrowserState>,
    next_op_id: u64,
    pub(crate) pending_test_op: Option<u64>,
    pending_upload_op: Option<u64>,
    pub(crate) pending_sync_op: Option<u64>,
    upload_queue: VecDeque<(String, PathBuf, String)>,
    pub(crate) sync_queue: VecDeque<(String, PathBuf)>,
    pub(crate) last_auto_sync: Instant,
}

impl BackupController {
    pub(crate) fn load(paths: &AppPaths, host: &dyn ToolsPlatform) -> Self {
        let backup_cfg = load_backup_config(&paths.backup_cfg);
        let webdav_store = load_webdav_store(&paths.webdav_cfg);
        let sync_cfg = load_sync_config(&paths.sync_cfg);
        let mut ctrl = Self {
            exe_path: paths.exe_path.clone(),
            config_dir: paths.data_dir.join("config"),
            backups_dir: paths.backups_dir.clone(),
            backup_cfg_path: paths.backup_cfg.clone(),
            webdav_cfg_path: paths.webdav_cfg.clone(),
            sync_cfg_path: paths.sync_cfg.clone(),
            backup_cfg,
            webdav_store,
            sync_cfg,
            saved_passwords: HashMap::new(),
            server_statuses: HashMap::new(),
            draft: None,
            browser: None,
            next_op_id: 1,
            pending_test_op: None,
            pending_upload_op: None,
            pending_sync_op: None,
            upload_queue: VecDeque::new(),
            sync_queue: VecDeque::new(),
            last_auto_sync: Instant::now(),
        };
        ctrl.refresh_all_password_flags(host);
        ctrl
    }

    pub(crate) fn backup_view(&self, lang: AppLanguage) -> BackupSettingsView {
        let servers = self
            .webdav_store
            .servers
            .iter()
            .map(|srv| {
                let prov = match lang {
                    AppLanguage::Ru => srv.provider.label_ru(),
                    AppLanguage::En => srv.provider.label_en(),
                };
                WebDavServerView {
                    id: srv.id.clone(),
                    name: srv.display_title(),
                    provider_label: prov.to_owned(),
                    effective_url: srv.effective_url(),
                    username: srv.username.clone(),
                    remote_dir: srv.normalized_remote_dir(),
                    has_saved_password: self.saved_passwords.get(&srv.id).copied().unwrap_or(false),
                    status_text: self
                        .server_statuses
                        .get(&srv.id)
                        .cloned()
                        .unwrap_or_default(),
                }
            })
            .collect();
        let draft = self.draft.as_ref().map(|d| WebDavDraftView {
            provider_index: match d.provider {
                WebDavProvider::Yandex => 0,
                WebDavProvider::MailRu => 1,
                WebDavProvider::Custom => 2,
            },
            provider_label: match lang {
                AppLanguage::Ru => d.provider.label_ru(),
                AppLanguage::En => d.provider.label_en(),
            }
            .to_owned(),
            effective_url: d.provider.preset_url().to_owned(),
            is_custom: d.provider == WebDavProvider::Custom,
            status_text: d.status_text.clone(),
        });
        let browser = self.browser.as_ref().map(|br| WebDavBrowserView {
            server_id: br.server_id.clone(),
            server_name: br.server_name.clone(),
            current_dir: br.current_dir.clone(),
            entries: br.entries.clone(),
            loading: br.loading,
            status_text: br.status_text.clone(),
        });
        BackupSettingsView {
            auto_upload_cloud: self.backup_cfg.auto_upload_cloud,
            target_server_ids: self.backup_cfg.target_server_ids.clone(),
            last_backup_file: self.backup_cfg.last_backup_file.clone(),
            last_backup_time: self.backup_cfg.last_backup_time.clone(),
            cloud_status: self.backup_cfg.last_cloud_status.clone(),
            busy: self.pending_upload_op.is_some() || !self.upload_queue.is_empty(),
            servers,
            draft,
            browser,
            sync: self.sync_view(),
        }
    }

    pub(crate) fn set_auto_upload(&mut self, enabled: bool) {
        self.backup_cfg.auto_upload_cloud = enabled;
        let _ = save_backup_config(&self.backup_cfg_path, &self.backup_cfg);
    }

    pub(crate) fn toggle_backup_target_server(&mut self, server_id: &str) {
        if let Some(pos) = self
            .backup_cfg
            .target_server_ids
            .iter()
            .position(|id| id == server_id)
        {
            self.backup_cfg.target_server_ids.remove(pos);
        } else {
            self.backup_cfg.target_server_ids.push(server_id.to_owned());
        }
        let _ = save_backup_config(&self.backup_cfg_path, &self.backup_cfg);
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
            match self.enqueue_upload_to_targets(host, &art.path, &art.file_name) {
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
        let msg = self.enqueue_upload_to_targets(host, &path, &name)?;
        let _ = save_backup_config(&self.backup_cfg_path, &self.backup_cfg);
        Ok(msg)
    }

    pub(crate) fn on_webdav_done(
        &mut self,
        host: &dyn ToolsPlatform,
        res: WebDavResult,
        now: LocalDateTime,
    ) -> String {
        match res.kind {
            WebDavOpKind::Test => {
                if self.pending_test_op == Some(res.op_id) {
                    self.pending_test_op = None;
                }
                if res.server_id == "__draft__" {
                    if let Some(d) = self.draft.as_mut() {
                        d.status_text = res.message.clone();
                    }
                } else {
                    self.server_statuses
                        .insert(res.server_id.clone(), res.message.clone());
                }
                format!("WebDAV тест [{}]: {}", res.server_id, res.message)
            }
            WebDavOpKind::Upload => {
                self.pending_upload_op = None;
                self.server_statuses
                    .insert(res.server_id.clone(), res.message.clone());
                self.backup_cfg.last_cloud_status = format!("[{}] {}", res.server_id, res.message);
                let _ = save_backup_config(&self.backup_cfg_path, &self.backup_cfg);
                self.drain_upload_queue(host);
                format!("WebDAV загрузка [{}]: {}", res.server_id, res.message)
            }
            WebDavOpKind::ListDir => self.handle_list_dir_done(res),
            WebDavOpKind::MkCol => self.handle_mkcol_done(host, res),
            WebDavOpKind::SyncUpload => self.handle_sync_upload_done(host, res, now),
        }
    }

    fn enqueue_upload_to_targets(
        &mut self,
        host: &dyn ToolsPlatform,
        local_path: &Path,
        remote_name: &str,
    ) -> Result<String, String> {
        let targets = self.resolve_backup_targets();
        if targets.is_empty() {
            return Err("добавьте и выберите хотя бы один WebDAV сервер".to_owned());
        }
        let count = targets.len();
        for id in targets {
            self.upload_queue
                .push_back((id, local_path.to_path_buf(), remote_name.to_owned()));
        }
        self.drain_upload_queue(host);
        Ok(format!("Загрузка {remote_name} -> серверов: {count}"))
    }

    fn resolve_backup_targets(&self) -> Vec<String> {
        let valid: Vec<String> = self
            .backup_cfg
            .target_server_ids
            .iter()
            .filter(|id| self.webdav_store.find(id).is_some())
            .cloned()
            .collect();
        if !valid.is_empty() {
            return valid;
        }
        self.webdav_store
            .servers
            .iter()
            .map(|s| s.id.clone())
            .collect()
    }

    fn drain_upload_queue(&mut self, host: &dyn ToolsPlatform) {
        if self.pending_upload_op.is_some() {
            return;
        }
        while let Some((server_id, path, name)) = self.upload_queue.pop_front() {
            let Ok((base_url, user, pass)) = self.resolve_server_creds(host, &server_id) else {
                self.server_statuses
                    .insert(server_id, "Нет пароля в Credential Manager".to_owned());
                continue;
            };
            let remote_dir = self.webdav_store.find(&server_id).map_or_else(
                || "/asrbox-backups".to_owned(),
                |s| s.normalized_remote_dir(),
            );
            let op_id = self.alloc_op_id();
            if host
                .webdav_upload(
                    op_id,
                    &server_id,
                    &base_url,
                    &remote_dir,
                    &name,
                    &path,
                    &user,
                    &pass,
                )
                .is_ok()
            {
                self.pending_upload_op = Some(op_id);
                self.backup_cfg.last_cloud_status = format!("Загрузка {name} -> {server_id}...");
                break;
            }
        }
    }

    pub(crate) fn resolve_server_creds(
        &self,
        host: &dyn ToolsPlatform,
        server_id: &str,
    ) -> Result<(String, String, String), String> {
        let Some(srv) = self.webdav_store.find(server_id) else {
            return Err(format!("WebDAV сервер {server_id} не найден"));
        };
        let base_url = srv.effective_url();
        validate_https_url(&base_url)?;
        let saved = host.cred_load(&srv.cred_target())?;
        let user = if !srv.username.trim().is_empty() {
            srv.username.trim().to_owned()
        } else if let Some((u, _)) = &saved {
            u.clone()
        } else {
            return Err("не задан логин WebDAV".to_owned());
        };
        let Some((_, pass)) = saved else {
            return Err("не сохранён пароль приложения WebDAV".to_owned());
        };
        Ok((base_url, user, pass))
    }

    pub(crate) fn save_store_quiet(&self) {
        let _ = save_webdav_store(&self.webdav_cfg_path, &self.webdav_store);
    }

    pub(crate) fn set_server_status(&mut self, server_id: &str, msg: String) {
        self.server_statuses.insert(server_id.to_owned(), msg);
    }

    pub(crate) fn remove_server_meta(&mut self, server_id: &str) {
        self.saved_passwords.remove(server_id);
        self.server_statuses.remove(server_id);
    }

    pub(crate) fn alloc_op_id(&mut self) -> u64 {
        let id = self.next_op_id;
        self.next_op_id = self.next_op_id.saturating_add(1);
        id
    }

    pub(crate) fn refresh_all_password_flags(&mut self, host: &dyn ToolsPlatform) {
        self.saved_passwords.clear();
        for srv in &self.webdav_store.servers {
            let has = host.cred_load(&srv.cred_target()).ok().flatten().is_some();
            self.saved_passwords.insert(srv.id.clone(), has);
        }
    }
}
