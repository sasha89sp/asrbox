pub mod crypto;
pub mod github;
pub mod zip;

use std::fs;
use std::path::{Path, PathBuf};

use asrbox_platform_api::LocalDateTime;

pub use crate::github::{
    load_github_config, parse_github_config, save_github_config, serialize_github_config,
    validate_github_branch, validate_github_repo, GitHubBackupConfig, GITHUB_TOKEN_CRED_TARGET,
};
use crate::zip::{write_zip_with_encryption, ZipEntry};

pub const BACKUP_ZIP_CRED_TARGET: &str = "asrbox:backup:zip_password";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupMode {
    SettingsOnly,
    FullAppAndSettings,
}

impl BackupMode {
    pub fn prefix(self) -> &'static str {
        match self {
            Self::SettingsOnly => "asrbox-settings",
            Self::FullAppAndSettings => "asrbox-full",
        }
    }

    pub fn label_ru(self) -> &'static str {
        match self {
            Self::SettingsOnly => "Настройки",
            Self::FullAppAndSettings => "Программа + настройки",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BackupConfig {
    pub auto_upload_cloud: bool,
    pub encrypt_zip: bool,
    pub target_server_ids: Vec<String>,
    pub last_backup_file: String,
    pub last_backup_time: String,
    pub last_cloud_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackupArtifact {
    pub path: PathBuf,
    pub file_name: String,
    pub bytes: u64,
    pub entry_count: usize,
    pub encrypted: bool,
}

pub fn load_backup_config(path: &Path) -> BackupConfig {
    let Ok(raw) = fs::read_to_string(path) else {
        return BackupConfig::default();
    };
    parse_backup_config(&raw)
}

pub fn save_backup_config(path: &Path, cfg: &BackupConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all backup.cfg: {e}"))?;
    }
    let tmp = path.with_extension("cfg.tmp");
    fs::write(&tmp, serialize_backup_config(cfg)).map_err(|e| format!("write backup.cfg: {e}"))?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).map_err(|e| format!("rename backup.cfg: {e}"))?;
    Ok(())
}

pub fn parse_backup_config(raw: &str) -> BackupConfig {
    let mut cfg = BackupConfig::default();
    for line in raw.trim_start_matches('\u{FEFF}').lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        match k.trim() {
            "auto_upload_cloud" => cfg.auto_upload_cloud = v.trim() == "1" || v.trim() == "true",
            "encrypt_zip" => cfg.encrypt_zip = v.trim() == "1" || v.trim() == "true",
            "target_server_ids" => {
                cfg.target_server_ids = v
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned)
                    .collect();
            }
            "last_backup_file" => cfg.last_backup_file = v.trim().to_owned(),
            "last_backup_time" => cfg.last_backup_time = v.trim().to_owned(),
            "last_cloud_status" => cfg.last_cloud_status = v.trim().to_owned(),
            _ => {}
        }
    }
    cfg
}

pub fn serialize_backup_config(cfg: &BackupConfig) -> String {
    format!(
        "# Настройки модуля бэкапа (asrbox)\n\
         settings_version=3\n\
         written_by={}\n\
         auto_upload_cloud={}\n\
         encrypt_zip={}\n\
         target_server_ids={}\n\
         last_backup_file={}\n\
         last_backup_time={}\n\
         last_cloud_status={}\n",
        env!("CARGO_PKG_VERSION"),
        if cfg.auto_upload_cloud { 1 } else { 0 },
        if cfg.encrypt_zip { 1 } else { 0 },
        cfg.target_server_ids.join(","),
        cfg.last_backup_file,
        cfg.last_backup_time,
        cfg.last_cloud_status,
    )
}

pub fn create_backup(
    mode: BackupMode,
    exe_path: &Path,
    config_dir: &Path,
    backups_dir: &Path,
    now: LocalDateTime,
) -> Result<BackupArtifact, String> {
    create_backup_with_options(
        mode,
        exe_path,
        config_dir,
        None,
        backups_dir,
        now,
        &[],
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn create_backup_with_options(
    mode: BackupMode,
    exe_path: &Path,
    config_dir: &Path,
    notes_dir: Option<&Path>,
    backups_dir: &Path,
    now: LocalDateTime,
    extra_entries: &[ZipEntry],
    aes: Option<(&str, &[[u8; 16]])>,
) -> Result<BackupArtifact, String> {
    let mut entries = collect_backup_entries(mode, exe_path, config_dir, notes_dir)?;
    for extra in extra_entries {
        entries.push(ZipEntry {
            name: extra.name.clone(),
            data: extra.data.clone(),
        });
    }
    if entries.is_empty() {
        return Err("нет файлов для резервного копирования".to_owned());
    }
    let file_name = format!(
        "{}-{:04}{:02}{:02}-{:02}{:02}{:02}.zip",
        mode.prefix(),
        now.year,
        now.month,
        now.day,
        now.hour,
        now.minute,
        now.second
    );
    let out_path = backups_dir.join(&file_name);
    let bytes = write_zip_with_encryption(&out_path, &entries, now, aes)?;
    Ok(BackupArtifact {
        path: out_path,
        file_name,
        bytes,
        entry_count: entries.len(),
        encrypted: aes.is_some(),
    })
}

pub fn count_backup_entries(
    mode: BackupMode,
    config_dir: &Path,
    notes_dir: Option<&Path>,
    extra_count: usize,
) -> usize {
    let mut n = usize::from(mode == BackupMode::FullAppAndSettings);
    n += count_dir_files(config_dir);
    if let Some(nd) = notes_dir {
        n += count_dir_files(nd);
    }
    n + extra_count
}

fn count_dir_files(dir: &Path) -> usize {
    let Ok(rd) = fs::read_dir(dir) else {
        return 0;
    };
    rd.flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|f| !f.ends_with(".tmp"))
        })
        .count()
}

fn collect_backup_entries(
    mode: BackupMode,
    exe_path: &Path,
    config_dir: &Path,
    notes_dir: Option<&Path>,
) -> Result<Vec<ZipEntry>, String> {
    let mut entries = Vec::new();
    if mode == BackupMode::FullAppAndSettings {
        let exe_bytes = fs::read(exe_path)
            .map_err(|e| format!("не удалось прочитать {}: {e}", exe_path.display()))?;
        let exe_name = exe_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("asrbox.exe")
            .to_owned();
        entries.push(ZipEntry {
            name: exe_name,
            data: exe_bytes,
        });
    }
    collect_dir_into(config_dir, "data/config", &mut entries)?;
    if let Some(nd) = notes_dir {
        collect_dir_into(nd, "data/notes", &mut entries)?;
    }
    Ok(entries)
}

fn collect_dir_into(dir: &Path, prefix: &str, out: &mut Vec<ZipEntry>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }
    let rd = fs::read_dir(dir).map_err(|e| format!("read dir {}: {e}", dir.display()))?;
    let mut files: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    for p in files {
        let Some(fname) = p.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if fname.ends_with(".tmp") {
            continue;
        }
        if let Ok(data) = fs::read(&p) {
            out.push(ZipEntry {
                name: format!("{prefix}/{fname}"),
                data,
            });
        }
    }
    Ok(())
}

pub fn find_latest_backup(backups_dir: &Path) -> Option<PathBuf> {
    let rd = fs::read_dir(backups_dir).ok()?;
    let mut zips: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
        })
        .collect();
    zips.sort();
    zips.pop()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_now() -> LocalDateTime {
        LocalDateTime {
            year: 2026,
            month: 11,
            day: 25,
            hour: 12,
            minute: 30,
            second: 45,
            utc_offset_min: 180,
        }
    }

    #[test]
    fn backup_config_and_encrypted_zip_work() {
        let cfg = BackupConfig {
            auto_upload_cloud: true,
            encrypt_zip: true,
            target_server_ids: vec!["srv-1".to_owned()],
            last_backup_file: "asrbox-full.zip".to_owned(),
            last_backup_time: "25.11 12:30".to_owned(),
            last_cloud_status: "OK".to_owned(),
        };
        assert_eq!(parse_backup_config(&serialize_backup_config(&cfg)), cfg);

        let root = std::env::temp_dir().join("asrbox_backup_v8_test");
        let _ = fs::remove_dir_all(&root);
        let cfg_dir = root.join("config");
        let bkp_dir = root.join("backups");
        let exe_path = root.join("asrbox.exe");
        let _ = fs::create_dir_all(&cfg_dir);
        let _ = fs::write(cfg_dir.join("docks.cfg"), "settings_version=2\n");
        let _ = fs::write(&exe_path, b"MZ-TEST-BINARY");

        let plain = create_backup(
            BackupMode::FullAppAndSettings,
            &exe_path,
            &cfg_dir,
            &bkp_dir,
            sample_now(),
        );
        assert!(plain.is_ok());
        let salts = [[1u8; 16], [2u8; 16]];
        let enc = create_backup_with_options(
            BackupMode::FullAppAndSettings,
            &exe_path,
            &cfg_dir,
            None,
            &bkp_dir,
            sample_now(),
            &[],
            Some(("secret123", &salts)),
        );
        assert!(enc
            .as_ref()
            .is_ok_and(|a| a.encrypted && a.entry_count == 2));
        let _ = fs::remove_dir_all(&root);
    }
}
