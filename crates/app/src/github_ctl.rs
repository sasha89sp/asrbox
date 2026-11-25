#![cfg(windows)]

use std::fs;

use asrbox_mod_backup::zip::ZipEntry;
use asrbox_mod_backup::{
    count_backup_entries, create_backup_with_options, save_backup_config, save_github_config,
    validate_github_branch, validate_github_repo, BackupMode, BACKUP_ZIP_CRED_TARGET,
    GITHUB_TOKEN_CRED_TARGET,
};
use asrbox_mod_settings::GitHubBackupView;
use asrbox_platform_api::{GitHubFetchResult, LocalDateTime, ToolsPlatform};

use crate::backup_ctl::BackupController;

impl BackupController {
    pub(crate) fn github_view(&self) -> GitHubBackupView {
        GitHubBackupView {
            repo: self.github_cfg.repo.clone(),
            branch: self.github_cfg.branch.clone(),
            include_in_backup: self.github_cfg.include_in_backup,
            has_token: self.has_github_token,
            busy: self.pending_github_op.is_some(),
            last_status: self.github_cfg.last_status.clone(),
        }
    }

    pub(crate) fn set_encrypt_zip(&mut self, enabled: bool) {
        self.backup_cfg.encrypt_zip = enabled;
        let _ = save_backup_config(&self.backup_cfg_path, &self.backup_cfg);
    }

    pub(crate) fn save_zip_password_from_input(
        &mut self,
        host: &dyn ToolsPlatform,
    ) -> Result<String, String> {
        let pwd = host.settings_input_text(5);
        let trimmed = pwd.trim();
        if trimmed.len() < 4 {
            return Err("Пароль архива должен содержать минимум 4 символа".to_owned());
        }
        host.cred_save(BACKUP_ZIP_CRED_TARGET, "zip", trimmed)?;
        host.set_settings_input_text(5, "");
        self.has_zip_password = true;
        Ok("Пароль AES-256 сохранён в Windows Credential Manager".to_owned())
    }

    pub(crate) fn clear_zip_password(
        &mut self,
        host: &dyn ToolsPlatform,
    ) -> Result<String, String> {
        let _ = host.cred_delete(BACKUP_ZIP_CRED_TARGET);
        host.set_settings_input_text(5, "");
        self.has_zip_password = false;
        self.backup_cfg.encrypt_zip = false;
        let _ = save_backup_config(&self.backup_cfg_path, &self.backup_cfg);
        Ok("Пароль AES-256 удалён из Credential Manager".to_owned())
    }

    pub(crate) fn set_github_include_in_backup(&mut self, enabled: bool) {
        self.github_cfg.include_in_backup = enabled;
        let _ = save_github_config(&self.github_cfg_path, &self.github_cfg);
    }

    pub(crate) fn save_github_settings_from_inputs(
        &mut self,
        host: &dyn ToolsPlatform,
    ) -> Result<String, String> {
        let repo_raw = host.settings_input_text(6);
        let branch_raw = host.settings_input_text(7);
        let token_raw = host.settings_input_text(8);

        let repo = if repo_raw.trim().is_empty() {
            String::new()
        } else {
            validate_github_repo(&repo_raw)?
        };
        let branch = if branch_raw.trim().is_empty() {
            "main".to_owned()
        } else {
            validate_github_branch(&branch_raw)?
        };
        self.github_cfg.repo = repo;
        self.github_cfg.branch = branch;

        let tok = token_raw.trim();
        if !tok.is_empty() {
            host.cred_save(GITHUB_TOKEN_CRED_TARGET, "github", tok)?;
            host.set_settings_input_text(8, "");
            self.has_github_token = true;
        }
        self.github_cfg.last_status = "Настройки GitHub сохранены".to_owned();
        save_github_config(&self.github_cfg_path, &self.github_cfg)?;
        Ok("Настройки GitHub-бэкапа сохранены".to_owned())
    }

    pub(crate) fn clear_github_token(
        &mut self,
        host: &dyn ToolsPlatform,
    ) -> Result<String, String> {
        let _ = host.cred_delete(GITHUB_TOKEN_CRED_TARGET);
        host.set_settings_input_text(8, "");
        self.has_github_token = false;
        self.github_cfg.last_status = "Токен GitHub удалён".to_owned();
        let _ = save_github_config(&self.github_cfg_path, &self.github_cfg);
        Ok("Токен GitHub удалён из Credential Manager".to_owned())
    }

    pub(crate) fn download_github_source_now(
        &mut self,
        host: &dyn ToolsPlatform,
        now: LocalDateTime,
    ) -> Result<String, String> {
        if self.pending_github_op.is_some() {
            return Err("Загрузка исходников GitHub уже выполняется".to_owned());
        }
        let repo = validate_github_repo(&self.github_cfg.repo)?;
        let branch = validate_github_branch(&self.github_cfg.branch)?;
        let token = host
            .cred_load(GITHUB_TOKEN_CRED_TARGET)
            .ok()
            .flatten()
            .map(|(_, secret)| secret);
        let safe_repo = repo.replace('/', "-");
        let safe_branch = branch.replace('/', "_");
        let fname = format!(
            "github-{safe_repo}-{safe_branch}-{:04}{:02}{:02}-{:02}{:02}{:02}.zip",
            now.year, now.month, now.day, now.hour, now.minute, now.second
        );
        let dest = self.backups_dir.join(fname);
        let op_id = self.alloc_op_id();
        host.github_fetch_zipball(
            op_id,
            &repo,
            &branch,
            token.as_deref().unwrap_or(""),
            &dest,
            false,
        )?;
        self.pending_github_op = Some(op_id);
        self.github_cfg.last_status = format!("Скачивание {repo}@{branch}...");
        let _ = save_github_config(&self.github_cfg_path, &self.github_cfg);
        Ok(format!("Запущено скачивание исходников {repo}@{branch}"))
    }

    pub(crate) fn start_github_download_for_backup(
        &mut self,
        host: &dyn ToolsPlatform,
        mode: BackupMode,
        now: LocalDateTime,
    ) -> Result<String, String> {
        if self.pending_github_op.is_some() {
            return Err("Дождитесь завершения текущей загрузки GitHub".to_owned());
        }
        let repo = validate_github_repo(&self.github_cfg.repo)?;
        let branch = validate_github_branch(&self.github_cfg.branch)?;
        let token = host
            .cred_load(GITHUB_TOKEN_CRED_TARGET)
            .ok()
            .flatten()
            .map(|(_, secret)| secret);
        let tmp_path = self.backups_dir.join(".github-source-staging.zip");
        let op_id = self.alloc_op_id();
        host.github_fetch_zipball(
            op_id,
            &repo,
            &branch,
            token.as_deref().unwrap_or(""),
            &tmp_path,
            true,
        )?;
        self.pending_github_op = Some(op_id);
        self.pending_backup_after_gh = Some((mode, now));
        self.pending_gh_temp_path = Some(tmp_path);
        self.github_cfg.last_status = format!("Скачивание {repo}@{branch} для бэкапа...");
        let _ = save_github_config(&self.github_cfg_path, &self.github_cfg);
        Ok(format!(
            "Скачивание исходников {repo}@{branch} перед созданием бэкапа..."
        ))
    }

    pub(crate) fn on_github_fetch_done(
        &mut self,
        host: &dyn ToolsPlatform,
        res: GitHubFetchResult,
        now: LocalDateTime,
    ) -> (bool, String) {
        if self.pending_github_op == Some(res.op_id) {
            self.pending_github_op = None;
        }
        let pending_bkp = self.pending_backup_after_gh.take();
        let tmp_path = self.pending_gh_temp_path.take();

        if !res.ok {
            if let Some(p) = tmp_path {
                let _ = fs::remove_file(p);
            }
            self.github_cfg.last_status = format!("Ошибка: {}", res.message);
            let _ = save_github_config(&self.github_cfg_path, &self.github_cfg);
            return (false, format!("GitHub бэкап: {}", res.message));
        }

        if res.include_in_pending_backup {
            let Some((mode, bkp_now)) = pending_bkp else {
                return (true, format!("GitHub: {}", res.message));
            };
            let src_path = res.archive_path.clone().or(tmp_path);
            let Some(path) = src_path else {
                return (
                    false,
                    "GitHub: не найден временный архив исходников".to_owned(),
                );
            };
            let bytes = match fs::read(&path) {
                Ok(b) => b,
                Err(e) => {
                    let _ = fs::remove_file(&path);
                    return (false, format!("GitHub: ошибка чтения архива: {e}"));
                }
            };
            let _ = fs::remove_file(&path);
            let extra = [ZipEntry {
                name: "github-source.zip".to_owned(),
                data: bytes,
            }];
            match self.finish_create_backup(host, mode, bkp_now, &extra) {
                Ok(msg) => {
                    self.github_cfg.last_status =
                        format!("Включено в бэкап ({:02}:{:02})", now.hour, now.minute);
                    let _ = save_github_config(&self.github_cfg_path, &self.github_cfg);
                    (true, msg)
                }
                Err(err) => {
                    self.github_cfg.last_status = format!("Ошибка упаковки: {err}");
                    let _ = save_github_config(&self.github_cfg_path, &self.github_cfg);
                    (false, err)
                }
            }
        } else {
            self.github_cfg.last_status =
                format!("{} ({:02}:{:02})", res.message, now.hour, now.minute);
            let _ = save_github_config(&self.github_cfg_path, &self.github_cfg);
            (true, format!("GitHub исходники: {}", res.message))
        }
    }

    pub(crate) fn finish_create_backup(
        &mut self,
        host: &dyn ToolsPlatform,
        mode: BackupMode,
        now: LocalDateTime,
        extra_entries: &[ZipEntry],
    ) -> Result<String, String> {
        let notes_opt = self.notes_dir.exists().then_some(self.notes_dir.as_path());
        let aes_owned = if self.backup_cfg.encrypt_zip {
            let Some((_, pwd)) = host.cred_load(BACKUP_ZIP_CRED_TARGET)? else {
                return Err(
                    "Включено шифрование AES-256, но пароль не задан в настройках".to_owned(),
                );
            };
            let n = count_backup_entries(mode, &self.config_dir, notes_opt, extra_entries.len());
            let raw = host.random_bytes(n.saturating_mul(16))?;
            let mut salts = Vec::with_capacity(n);
            for i in 0..n {
                let start = i * 16;
                if let Some(slice) = raw.get(start..start + 16) {
                    let mut s = [0u8; 16];
                    s.copy_from_slice(slice);
                    salts.push(s);
                }
            }
            Some((pwd, salts))
        } else {
            None
        };
        let aes_ref = aes_owned
            .as_ref()
            .map(|(pwd, salts)| (pwd.as_str(), salts.as_slice()));
        let art = create_backup_with_options(
            mode,
            &self.exe_path,
            &self.config_dir,
            notes_opt,
            &self.backups_dir,
            now,
            extra_entries,
            aes_ref,
        )?;
        self.backup_cfg.last_backup_file = art.file_name.clone();
        self.backup_cfg.last_backup_time = format!(
            "{:02}.{:02}.{:04} {:02}:{:02}",
            now.day, now.month, now.year, now.hour, now.minute
        );
        let kb = art.bytes.div_ceil(1024);
        let enc_tag = if art.encrypted { " · AES-256" } else { "" };
        let mut summary = format!(
            "Бэкап ({}{enc_tag}) создан: {} ({kb} КБ, файлов: {})",
            mode.label_ru(),
            art.file_name,
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
}
