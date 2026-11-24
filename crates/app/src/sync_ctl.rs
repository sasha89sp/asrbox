#![cfg(windows)]

use std::path::PathBuf;
use std::time::Instant;

use asrbox_mod_settings::{FileSyncSettingsView, SyncItemView};
use asrbox_mod_webdav::{
    normalize_remote_dir, parent_remote_dir, save_sync_config, SyncInterval, SyncItem,
};
use asrbox_platform_api::{LocalDateTime, ToolsPlatform, WebDavResult};

use crate::backup_ctl::{BackupController, WebDavBrowserState};

impl BackupController {
    pub(crate) fn sync_view(&self) -> FileSyncSettingsView {
        let interval_minutes = match self.sync_cfg.interval {
            SyncInterval::Manual => 0,
            SyncInterval::Min15 => 15,
            SyncInterval::Min30 => 30,
            SyncInterval::Hour1 => 60,
            SyncInterval::Hour6 => 360,
            SyncInterval::Hour24 => 1440,
        };
        let items = self
            .sync_cfg
            .items
            .iter()
            .map(|it| {
                let leaf = PathBuf::from(&it.path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("sync")
                    .to_owned();
                SyncItemView {
                    id: it.path.clone(),
                    local_path: it.path.clone(),
                    is_dir: it.is_dir,
                    remote_subdir: format!("/sync/{leaf}"),
                }
            })
            .collect();
        let status_text = if self.sync_cfg.last_sync_time.is_empty() {
            self.sync_cfg.last_sync_status.clone()
        } else if self.sync_cfg.last_sync_status.is_empty() {
            self.sync_cfg.last_sync_time.clone()
        } else {
            format!(
                "{} · {}",
                self.sync_cfg.last_sync_status, self.sync_cfg.last_sync_time
            )
        };
        FileSyncSettingsView {
            items,
            target_server_ids: self.sync_cfg.target_server_ids.clone(),
            interval_minutes,
            last_sync_time: self.sync_cfg.last_sync_time.clone(),
            status_text,
            busy: self.pending_sync_op.is_some() || !self.sync_queue.is_empty(),
        }
    }

    pub(crate) fn open_browser(
        &mut self,
        host: &dyn ToolsPlatform,
        server_id: &str,
    ) -> Result<(String, String), String> {
        let Some(srv) = self.webdav_store.find(server_id) else {
            return Err(format!("WebDAV сервер {server_id} не найден"));
        };
        let server_name = srv.display_title();
        let start_dir = srv.normalized_remote_dir();
        self.browser = Some(WebDavBrowserState {
            server_id: server_id.to_owned(),
            server_name: server_name.clone(),
            current_dir: start_dir.clone(),
            entries: Vec::new(),
            loading: true,
            status_text: String::new(),
        });
        host.set_settings_input_text(4, "");
        self.browse_dir(host, server_id, &start_dir)?;
        Ok((server_id.to_owned(), server_name))
    }

    pub(crate) fn browse_dir(
        &mut self,
        host: &dyn ToolsPlatform,
        server_id: &str,
        remote_dir: &str,
    ) -> Result<String, String> {
        let norm = normalize_remote_dir(remote_dir);
        let (base_url, user, pass) = self.resolve_server_creds(host, server_id)?;
        let op_id = self.alloc_op_id();
        if let Some(br) = self.browser.as_mut() {
            if br.server_id == server_id {
                br.current_dir = norm.clone();
                br.loading = true;
                br.status_text.clear();
            }
        }
        host.webdav_list_dir(op_id, server_id, &base_url, &norm, &user, &pass)?;
        Ok(format!("WebDAV [{server_id}] список: {norm}"))
    }

    pub(crate) fn browse_up(
        &mut self,
        host: &dyn ToolsPlatform,
        server_id: &str,
    ) -> Result<String, String> {
        let cur = self
            .browser
            .as_ref()
            .filter(|b| b.server_id == server_id)
            .map_or_else(|| "/".to_owned(), |b| b.current_dir.clone());
        let parent = parent_remote_dir(&cur);
        self.browse_dir(host, server_id, &parent)
    }

    pub(crate) fn create_folder_from_input(
        &mut self,
        host: &dyn ToolsPlatform,
        server_id: &str,
    ) -> Result<String, String> {
        let raw = host.settings_input_text(4);
        let name = raw.trim().trim_matches('/').trim_matches('\\');
        if name.is_empty() || name.contains("..") {
            return Err("введите имя новой папки".to_owned());
        }
        let cur = self
            .browser
            .as_ref()
            .filter(|b| b.server_id == server_id)
            .map_or_else(|| "/".to_owned(), |b| b.current_dir.clone());
        let target = if cur == "/" {
            format!("/{name}")
        } else {
            format!("{}/{name}", cur.trim_end_matches('/'))
        };
        let (base_url, user, pass) = self.resolve_server_creds(host, server_id)?;
        let op_id = self.alloc_op_id();
        host.webdav_mkcol(op_id, server_id, &base_url, &target, &user, &pass)?;
        host.set_settings_input_text(4, "");
        Ok(format!("WebDAV [{server_id}] MKCOL {target}"))
    }

    pub(crate) fn select_current_folder(&mut self, server_id: &str) -> Result<String, String> {
        let dir = self
            .browser
            .as_ref()
            .filter(|b| b.server_id == server_id)
            .map(|b| normalize_remote_dir(&b.current_dir))
            .ok_or_else(|| "обозреватель WebDAV не открыт".to_owned())?;
        let Some(srv) = self.webdav_store.find_mut(server_id) else {
            return Err(format!("сервер {server_id} не найден"));
        };
        srv.remote_dir = dir.clone();
        self.save_store_quiet();
        self.set_server_status(server_id, format!("Выбрана папка: {dir}"));
        if let Some(br) = self.browser.as_mut() {
            br.status_text = format!("Папка хранения назначена: {dir}");
        }
        Ok(format!("WebDAV [{server_id}] папка хранения -> {dir}"))
    }

    pub(crate) fn handle_list_dir_done(&mut self, res: WebDavResult) -> String {
        if let Some(br) = self.browser.as_mut() {
            if br.server_id == res.server_id {
                br.loading = false;
                if res.ok {
                    br.current_dir = normalize_remote_dir(&res.remote_path);
                    br.entries = res.entries;
                    br.status_text = res.message.clone();
                } else {
                    br.entries.clear();
                    br.status_text = res.message.clone();
                }
            }
        }
        format!("WebDAV список [{}]: {}", res.server_id, res.message)
    }

    pub(crate) fn handle_mkcol_done(
        &mut self,
        host: &dyn ToolsPlatform,
        res: WebDavResult,
    ) -> String {
        if let Some(br) = self.browser.as_mut() {
            if br.server_id == res.server_id {
                br.status_text = res.message.clone();
            }
        }
        if res.ok {
            let cur = self
                .browser
                .as_ref()
                .filter(|b| b.server_id == res.server_id)
                .map(|b| b.current_dir.clone());
            if let Some(dir) = cur {
                let _ = self.browse_dir(host, &res.server_id, &dir);
            }
        }
        format!("WebDAV MKCOL [{}]: {}", res.server_id, res.message)
    }

    pub(crate) fn pick_sync_files(
        &mut self,
        host: &mut dyn ToolsPlatform,
    ) -> Result<String, String> {
        let picked = host.pick_sync_files()?;
        let mut added = 0usize;
        for p in picked {
            let s = p.display().to_string();
            if !self.sync_cfg.items.iter().any(|i| i.path == s) {
                self.sync_cfg.items.push(SyncItem {
                    path: s,
                    is_dir: false,
                });
                added += 1;
            }
        }
        if added > 0 {
            save_sync_config(&self.sync_cfg_path, &self.sync_cfg)?;
        }
        Ok(format!("Синхронизация: добавлено файлов: {added}"))
    }

    pub(crate) fn pick_sync_folder(
        &mut self,
        host: &mut dyn ToolsPlatform,
    ) -> Result<String, String> {
        let Some(folder) = host.pick_sync_folder()? else {
            return Ok("Выбор папки отменён".to_owned());
        };
        let s = folder.display().to_string();
        if !self.sync_cfg.items.iter().any(|i| i.path == s) {
            self.sync_cfg.items.push(SyncItem {
                path: s.clone(),
                is_dir: true,
            });
            save_sync_config(&self.sync_cfg_path, &self.sync_cfg)?;
        }
        Ok(format!("Синхронизация: добавлена папка {s}"))
    }

    pub(crate) fn remove_sync_item(&mut self, path_id: &str) {
        self.sync_cfg.items.retain(|i| i.path != path_id);
        let _ = save_sync_config(&self.sync_cfg_path, &self.sync_cfg);
    }

    pub(crate) fn toggle_sync_server(&mut self, server_id: &str) {
        if let Some(pos) = self
            .sync_cfg
            .target_server_ids
            .iter()
            .position(|id| id == server_id)
        {
            self.sync_cfg.target_server_ids.remove(pos);
        } else {
            self.sync_cfg.target_server_ids.push(server_id.to_owned());
        }
        let _ = save_sync_config(&self.sync_cfg_path, &self.sync_cfg);
    }

    pub(crate) fn set_sync_interval_min(&mut self, minutes: u32) {
        self.sync_cfg.interval = match minutes {
            15 => SyncInterval::Min15,
            30 => SyncInterval::Min30,
            60 => SyncInterval::Hour1,
            360 => SyncInterval::Hour6,
            1440 => SyncInterval::Hour24,
            _ => SyncInterval::Manual,
        };
        self.last_auto_sync = Instant::now();
        let _ = save_sync_config(&self.sync_cfg_path, &self.sync_cfg);
    }

    pub(crate) fn sync_files_now(
        &mut self,
        host: &dyn ToolsPlatform,
        now: LocalDateTime,
    ) -> Result<String, String> {
        if self.sync_cfg.items.is_empty() {
            return Err("добавьте файлы или папки для синхронизации".to_owned());
        }
        let targets: Vec<String> = self
            .sync_cfg
            .target_server_ids
            .iter()
            .filter(|id| self.webdav_store.find(id).is_some())
            .cloned()
            .collect();
        let targets = if targets.is_empty() {
            self.webdav_store
                .servers
                .iter()
                .map(|s| s.id.clone())
                .collect()
        } else {
            targets
        };
        if targets.is_empty() {
            return Err("добавьте WebDAV сервер для синхронизации".to_owned());
        }
        for srv_id in &targets {
            for item in &self.sync_cfg.items {
                self.sync_queue
                    .push_back((srv_id.clone(), PathBuf::from(&item.path)));
            }
        }
        self.last_auto_sync = Instant::now();
        self.sync_cfg.last_sync_time = format!("{:02}:{:02}", now.hour, now.minute);
        self.sync_cfg.last_sync_status = "Синхронизация...".to_owned();
        self.drain_sync_queue(host);
        let _ = save_sync_config(&self.sync_cfg_path, &self.sync_cfg);
        Ok(format!(
            "Синхронизация запущена (объектов: {}, серверов: {})",
            self.sync_cfg.items.len(),
            targets.len()
        ))
    }

    pub(crate) fn maybe_auto_sync(
        &mut self,
        host: &dyn ToolsPlatform,
        now: LocalDateTime,
    ) -> Option<String> {
        let interval_sec = self.sync_cfg.interval.interval_sec()?;
        if self.sync_cfg.items.is_empty()
            || self.webdav_store.servers.is_empty()
            || self.pending_sync_op.is_some()
            || !self.sync_queue.is_empty()
        {
            return None;
        }
        if self.last_auto_sync.elapsed().as_secs() < interval_sec {
            return None;
        }
        self.sync_files_now(host, now).ok()
    }

    pub(crate) fn handle_sync_upload_done(
        &mut self,
        host: &dyn ToolsPlatform,
        res: WebDavResult,
        now: LocalDateTime,
    ) -> String {
        self.pending_sync_op = None;
        self.sync_cfg.last_sync_time = format!("{:02}:{:02}", now.hour, now.minute);
        self.sync_cfg.last_sync_status = format!("[{}] {}", res.server_id, res.message);
        let _ = save_sync_config(&self.sync_cfg_path, &self.sync_cfg);
        self.drain_sync_queue(host);
        format!("WebDAV синхронизация [{}]: {}", res.server_id, res.message)
    }

    fn drain_sync_queue(&mut self, host: &dyn ToolsPlatform) {
        if self.pending_sync_op.is_some() {
            return;
        }
        while let Some((server_id, path)) = self.sync_queue.pop_front() {
            let Ok((base_url, user, pass)) = self.resolve_server_creds(host, &server_id) else {
                self.sync_cfg.last_sync_status = format!("[{server_id}] нет пароля");
                continue;
            };
            let remote_root = self.webdav_store.find(&server_id).map_or_else(
                || "/asrbox-backups".to_owned(),
                |s| s.normalized_remote_dir(),
            );
            let op_id = self.alloc_op_id();
            if host
                .webdav_sync_path(
                    op_id,
                    &server_id,
                    &base_url,
                    &remote_root,
                    &path,
                    &user,
                    &pass,
                )
                .is_ok()
            {
                self.pending_sync_op = Some(op_id);
                break;
            }
        }
    }
}
