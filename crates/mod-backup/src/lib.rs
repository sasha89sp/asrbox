pub mod zip;

use std::fs;
use std::path::{Path, PathBuf};

use asrbox_platform_api::LocalDateTime;

use crate::zip::{write_store_zip, ZipEntry};

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
    let mut out = String::new();
    out.push_str("# Настройки модуля бэкапа (asrbox)\n");
    out.push_str("settings_version=2\n");
    out.push_str(&format!("written_by={}\n", env!("CARGO_PKG_VERSION")));
    out.push_str(&format!(
        "auto_upload_cloud={}\n",
        if cfg.auto_upload_cloud { 1 } else { 0 }
    ));
    out.push_str(&format!(
        "target_server_ids={}\n",
        cfg.target_server_ids.join(",")
    ));
    out.push_str(&format!("last_backup_file={}\n", cfg.last_backup_file));
    out.push_str(&format!("last_backup_time={}\n", cfg.last_backup_time));
    out.push_str(&format!("last_cloud_status={}\n", cfg.last_cloud_status));
    out
}

pub fn create_backup(
    mode: BackupMode,
    exe_path: &Path,
    config_dir: &Path,
    backups_dir: &Path,
    now: LocalDateTime,
) -> Result<BackupArtifact, String> {
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
    if config_dir.exists() {
        let rd = fs::read_dir(config_dir).map_err(|e| format!("read config dir: {e}"))?;
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
                entries.push(ZipEntry {
                    name: format!("data/config/{fname}"),
                    data,
                });
            }
        }
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
    let bytes = write_store_zip(&out_path, &entries, now)?;
    Ok(BackupArtifact {
        path: out_path,
        file_name,
        bytes,
        entry_count: entries.len(),
    })
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
            day: 23,
            hour: 12,
            minute: 30,
            second: 45,
            utc_offset_min: 180,
        }
    }

    #[test]
    fn backup_config_roundtrip_and_zip_creation() {
        let cfg = BackupConfig {
            auto_upload_cloud: true,
            target_server_ids: vec!["srv-1".to_owned(), "srv-2".to_owned()],
            last_backup_file: "asrbox-full-20261123-123045.zip".to_owned(),
            last_backup_time: "23.11 12:30".to_owned(),
            last_cloud_status: "Загружено".to_owned(),
        };
        assert_eq!(parse_backup_config(&serialize_backup_config(&cfg)), cfg);

        let root = std::env::temp_dir().join("asrbox_backup_test");
        let _ = fs::remove_dir_all(&root);
        let cfg_dir = root.join("config");
        let bkp_dir = root.join("backups");
        let exe_path = root.join("asrbox.exe");
        let _ = fs::create_dir_all(&cfg_dir);
        let _ = fs::write(cfg_dir.join("docks.cfg"), "settings_version=1\n");
        let _ = fs::write(&exe_path, b"MZ-TEST-BINARY");

        let s = create_backup(
            BackupMode::SettingsOnly,
            &exe_path,
            &cfg_dir,
            &bkp_dir,
            sample_now(),
        );
        assert!(s.is_ok());
        let f = create_backup(
            BackupMode::FullAppAndSettings,
            &exe_path,
            &cfg_dir,
            &bkp_dir,
            sample_now(),
        );
        assert!(f.is_ok());
        if let Ok(art) = f {
            assert_eq!(art.entry_count, 2);
            let raw = fs::read(&art.path).unwrap_or_default();
            assert!(raw.starts_with(b"PK\x03\x04"));
        }
        let _ = fs::remove_dir_all(&root);
    }
}
