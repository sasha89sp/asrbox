use std::fs;
use std::path::Path;

use asrbox_platform_api::AppLanguage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SyncInterval {
    #[default]
    Manual,
    Min15,
    Min30,
    Hour1,
    Hour6,
    Hour24,
}

impl SyncInterval {
    pub const ALL: [Self; 6] = [
        Self::Manual,
        Self::Min15,
        Self::Min30,
        Self::Hour1,
        Self::Hour6,
        Self::Hour24,
    ];

    pub fn index(self) -> u8 {
        match self {
            Self::Manual => 0,
            Self::Min15 => 1,
            Self::Min30 => 2,
            Self::Hour1 => 3,
            Self::Hour6 => 4,
            Self::Hour24 => 5,
        }
    }

    pub fn from_index(idx: u8) -> Self {
        match idx {
            1 => Self::Min15,
            2 => Self::Min30,
            3 => Self::Hour1,
            4 => Self::Hour6,
            5 => Self::Hour24,
            _ => Self::Manual,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Min15 => "15m",
            Self::Min30 => "30m",
            Self::Hour1 => "1h",
            Self::Hour6 => "6h",
            Self::Hour24 => "24h",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "15m" => Self::Min15,
            "30m" => Self::Min30,
            "1h" => Self::Hour1,
            "6h" => Self::Hour6,
            "24h" => Self::Hour24,
            _ => Self::Manual,
        }
    }

    pub fn interval_sec(self) -> Option<u64> {
        match self {
            Self::Manual => None,
            Self::Min15 => Some(15 * 60),
            Self::Min30 => Some(30 * 60),
            Self::Hour1 => Some(60 * 60),
            Self::Hour6 => Some(6 * 60 * 60),
            Self::Hour24 => Some(24 * 60 * 60),
        }
    }

    pub fn label(self, lang: AppLanguage) -> &'static str {
        match (self, lang) {
            (Self::Manual, AppLanguage::Ru) => "Вручную",
            (Self::Manual, AppLanguage::En) => "Manual",
            (Self::Min15, AppLanguage::Ru) => "15 мин",
            (Self::Min15, AppLanguage::En) => "15 min",
            (Self::Min30, AppLanguage::Ru) => "30 мин",
            (Self::Min30, AppLanguage::En) => "30 min",
            (Self::Hour1, AppLanguage::Ru) => "1 час",
            (Self::Hour1, AppLanguage::En) => "1 hour",
            (Self::Hour6, AppLanguage::Ru) => "6 часов",
            (Self::Hour6, AppLanguage::En) => "6 hours",
            (Self::Hour24, AppLanguage::Ru) => "24 часа",
            (Self::Hour24, AppLanguage::En) => "24 hours",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncItem {
    pub path: String,
    pub is_dir: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileSyncConfig {
    pub items: Vec<SyncItem>,
    pub target_server_ids: Vec<String>,
    pub interval: SyncInterval,
    pub last_sync_time: String,
    pub last_sync_status: String,
}

pub fn load_sync_config(path: &Path) -> FileSyncConfig {
    let Ok(raw) = fs::read_to_string(path) else {
        return FileSyncConfig::default();
    };
    parse_sync_config(&raw)
}

pub fn save_sync_config(path: &Path, cfg: &FileSyncConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all sync.cfg: {e}"))?;
    }
    let tmp = path.with_extension("cfg.tmp");
    fs::write(&tmp, serialize_sync_config(cfg)).map_err(|e| format!("write sync.cfg: {e}"))?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).map_err(|e| format!("rename sync.cfg: {e}"))?;
    Ok(())
}

pub fn parse_sync_config(raw: &str) -> FileSyncConfig {
    let mut cfg = FileSyncConfig::default();
    for line in raw.trim_start_matches('\u{FEFF}').lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        match k.trim() {
            "interval" => cfg.interval = SyncInterval::parse(v),
            "target_server_ids" => {
                cfg.target_server_ids = v
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned)
                    .collect();
            }
            "last_sync_time" => cfg.last_sync_time = v.trim().to_owned(),
            "last_sync_status" => cfg.last_sync_status = v.trim().to_owned(),
            "file" if !v.trim().is_empty() => cfg.items.push(SyncItem {
                path: v.trim().to_owned(),
                is_dir: false,
            }),
            "dir" if !v.trim().is_empty() => cfg.items.push(SyncItem {
                path: v.trim().to_owned(),
                is_dir: true,
            }),
            _ => {}
        }
    }
    cfg
}

pub fn serialize_sync_config(cfg: &FileSyncConfig) -> String {
    let mut out = String::new();
    out.push_str("# Синхронизация файлов и папок Windows с WebDAV (asrbox)\n");
    out.push_str("settings_version=1\n");
    out.push_str(&format!("written_by={}\n", env!("CARGO_PKG_VERSION")));
    out.push_str(&format!("interval={}\n", cfg.interval.as_str()));
    out.push_str(&format!(
        "target_server_ids={}\n",
        cfg.target_server_ids.join(",")
    ));
    out.push_str(&format!("last_sync_time={}\n", cfg.last_sync_time));
    out.push_str(&format!("last_sync_status={}\n", cfg.last_sync_status));
    for item in &cfg.items {
        let tag = if item.is_dir { "dir" } else { "file" };
        out.push_str(&format!("{tag}={}\n", item.path.trim()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_config_roundtrip() {
        let cfg = FileSyncConfig {
            items: vec![
                SyncItem {
                    path: "C:\\Docs\\notes.txt".to_owned(),
                    is_dir: false,
                },
                SyncItem {
                    path: "D:\\Projects".to_owned(),
                    is_dir: true,
                },
            ],
            target_server_ids: vec!["srv-1".to_owned(), "srv-2".to_owned()],
            interval: SyncInterval::Min30,
            last_sync_time: "24.11 12:00".to_owned(),
            last_sync_status: "Синхронизировано".to_owned(),
        };
        assert_eq!(parse_sync_config(&serialize_sync_config(&cfg)), cfg);
        assert_eq!(SyncInterval::Min30.interval_sec(), Some(1800));
    }
}
