use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WebDavProvider {
    #[default]
    Yandex,
    MailRu,
    Custom,
}

impl WebDavProvider {
    pub const ALL: [Self; 3] = [Self::Yandex, Self::MailRu, Self::Custom];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Yandex => "yandex",
            Self::MailRu => "mailru",
            Self::Custom => "custom",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "mailru" | "mail.ru" | "mail" => Self::MailRu,
            "custom" => Self::Custom,
            _ => Self::Yandex,
        }
    }

    pub fn label_ru(self) -> &'static str {
        match self {
            Self::Yandex => "Яндекс.Диск",
            Self::MailRu => "Mail.ru",
            Self::Custom => "Свой",
        }
    }

    pub fn preset_url(self) -> &'static str {
        match self {
            Self::Yandex => "https://webdav.yandex.ru",
            Self::MailRu => "https://webdav.cloud.mail.ru",
            Self::Custom => "",
        }
    }

    pub fn cred_target(self) -> &'static str {
        match self {
            Self::Yandex => "asrbox:webdav:yandex",
            Self::MailRu => "asrbox:webdav:mailru",
            Self::Custom => "asrbox:webdav:custom",
        }
    }

    pub fn index(self) -> u8 {
        match self {
            Self::Yandex => 0,
            Self::MailRu => 1,
            Self::Custom => 2,
        }
    }

    pub fn from_index(idx: u8) -> Self {
        match idx {
            1 => Self::MailRu,
            2 => Self::Custom,
            _ => Self::Yandex,
        }
    }
}

/// Non-secret WebDAV configuration persisted in `data/config/webdav.cfg`.
/// Passwords / App Passwords are NEVER stored here — only in Windows Credential Manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebDavConfig {
    pub provider: WebDavProvider,
    pub custom_url: String,
    pub username: String,
    pub remote_dir: String,
}

impl Default for WebDavConfig {
    fn default() -> Self {
        Self {
            provider: WebDavProvider::Yandex,
            custom_url: String::new(),
            username: String::new(),
            remote_dir: "/asrbox-backups".to_owned(),
        }
    }
}

impl WebDavConfig {
    pub fn effective_url(&self) -> String {
        let raw = match self.provider {
            WebDavProvider::Custom => self.custom_url.trim(),
            other => other.preset_url(),
        };
        raw.trim_end_matches('/').to_owned()
    }

    pub fn normalized_remote_dir(&self) -> String {
        let trimmed = self.remote_dir.trim().trim_matches('/');
        if trimmed.is_empty() {
            "/asrbox-backups".to_owned()
        } else {
            format!("/{trimmed}")
        }
    }

    pub fn cred_target(&self) -> &'static str {
        self.provider.cred_target()
    }
}

pub fn validate_https_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if !trimmed.starts_with("https://") || trimmed.len() <= "https://".len() {
        return Err("WebDAV URL должен начинаться с https://".to_owned());
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err("WebDAV URL не должен содержать пробелы".to_owned());
    }
    Ok(())
}

pub fn load_webdav_config(path: &Path) -> WebDavConfig {
    let Ok(raw) = fs::read_to_string(path) else {
        return WebDavConfig::default();
    };
    parse_webdav_config(&raw)
}

pub fn save_webdav_config(path: &Path, cfg: &WebDavConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all webdav.cfg: {e}"))?;
    }
    let tmp = path.with_extension("cfg.tmp");
    fs::write(&tmp, serialize_webdav_config(cfg)).map_err(|e| format!("write webdav.cfg: {e}"))?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).map_err(|e| format!("rename webdav.cfg: {e}"))?;
    Ok(())
}

pub fn parse_webdav_config(raw: &str) -> WebDavConfig {
    let mut cfg = WebDavConfig::default();
    for line in raw.trim_start_matches('\u{FEFF}').lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        match k.trim() {
            "provider" => cfg.provider = WebDavProvider::parse(v),
            "custom_url" => cfg.custom_url = v.trim().to_owned(),
            "username" => cfg.username = v.trim().to_owned(),
            "remote_dir" if !v.trim().is_empty() => cfg.remote_dir = v.trim().to_owned(),
            _ => {}
        }
    }
    cfg
}

pub fn serialize_webdav_config(cfg: &WebDavConfig) -> String {
    let mut out = String::new();
    out.push_str(
        "# Настройки WebDAV (asrbox) — пароли хранятся только в Windows Credential Manager\n",
    );
    out.push_str("settings_version=1\n");
    out.push_str(&format!("written_by={}\n", env!("CARGO_PKG_VERSION")));
    out.push_str(&format!("provider={}\n", cfg.provider.as_str()));
    out.push_str(&format!("custom_url={}\n", cfg.custom_url.trim()));
    out.push_str(&format!("username={}\n", cfg.username.trim()));
    out.push_str(&format!("remote_dir={}\n", cfg.normalized_remote_dir()));
    out
}

pub fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = if chunk.len() > 1 {
            u32::from(chunk[1])
        } else {
            0
        };
        let b2 = if chunk.len() > 2 {
            u32::from(chunk[2])
        } else {
            0
        };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[((n >> 18) & 0x3F) as usize] as char);
        out.push(TABLE[((n >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[((n >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 0x3F) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webdav_config_roundtrip_and_presets() {
        let cfg = WebDavConfig {
            provider: WebDavProvider::MailRu,
            custom_url: String::new(),
            username: "user@mail.ru".to_owned(),
            remote_dir: "/asrbox-backups".to_owned(),
        };
        assert_eq!(cfg.effective_url(), "https://webdav.cloud.mail.ru");
        assert_eq!(parse_webdav_config(&serialize_webdav_config(&cfg)), cfg);
        assert!(validate_https_url("https://webdav.yandex.ru").is_ok());
        assert!(validate_https_url("http://insecure.example").is_err());
        assert_eq!(base64_encode(b"user:pass"), "dXNlcjpwYXNz");
    }
}
