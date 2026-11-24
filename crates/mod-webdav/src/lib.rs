pub mod propfind;
pub mod sync;

use std::fs;
use std::path::Path;

pub use propfind::{parent_remote_dir, parse_propfind_xml};
pub use sync::{
    load_sync_config, parse_sync_config, save_sync_config, serialize_sync_config, FileSyncConfig,
    SyncInterval, SyncItem,
};

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
            Self::MailRu => "Облако Mail.ru",
            Self::Custom => "Свой WebDAV",
        }
    }

    pub fn label_en(self) -> &'static str {
        match self {
            Self::Yandex => "Yandex.Disk",
            Self::MailRu => "Mail.ru Cloud",
            Self::Custom => "Custom WebDAV",
        }
    }

    pub fn preset_url(self) -> &'static str {
        match self {
            Self::Yandex => "https://webdav.yandex.ru",
            Self::MailRu => "https://webdav.cloud.mail.ru",
            Self::Custom => "",
        }
    }

    pub fn legacy_cred_target(self) -> &'static str {
        match self {
            Self::Yandex => "asrbox:webdav:yandex",
            Self::MailRu => "asrbox:webdav:mailru",
            Self::Custom => "asrbox:webdav:custom",
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

/// Non-secret WebDAV server profile persisted in `data/config/webdav.cfg`.
/// Passwords / App Passwords are NEVER stored here — only in Windows Credential Manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebDavServer {
    pub id: String,
    pub name: String,
    pub provider: WebDavProvider,
    pub custom_url: String,
    pub username: String,
    pub remote_dir: String,
    pub legacy_cred_target: Option<String>,
}

impl WebDavServer {
    pub fn effective_url(&self) -> String {
        let raw = match self.provider {
            WebDavProvider::Custom => self.custom_url.trim(),
            other => other.preset_url(),
        };
        raw.trim_end_matches('/').to_owned()
    }

    pub fn normalized_remote_dir(&self) -> String {
        normalize_remote_dir(&self.remote_dir)
    }

    pub fn cred_target(&self) -> String {
        if let Some(ref legacy) = self.legacy_cred_target {
            if !legacy.trim().is_empty() {
                return legacy.clone();
            }
        }
        format!("asrbox:webdav:srv:{}", self.id)
    }

    pub fn display_title(&self) -> String {
        if !self.name.trim().is_empty() {
            return self.name.clone();
        }
        if !self.username.trim().is_empty() {
            format!("{} ({})", self.provider.label_ru(), self.username.trim())
        } else {
            self.provider.label_ru().to_owned()
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WebDavStore {
    pub servers: Vec<WebDavServer>,
}

impl WebDavStore {
    pub fn find(&self, id: &str) -> Option<&WebDavServer> {
        self.servers.iter().find(|s| s.id == id)
    }

    pub fn find_mut(&mut self, id: &str) -> Option<&mut WebDavServer> {
        self.servers.iter_mut().find(|s| s.id == id)
    }

    pub fn next_server_id(&self) -> String {
        let mut idx = self.servers.len() + 1;
        loop {
            let candidate = format!("srv-{idx}");
            if !self.servers.iter().any(|s| s.id == candidate) {
                return candidate;
            }
            idx += 1;
        }
    }
}

pub fn normalize_remote_dir(raw: &str) -> String {
    let clean: Vec<String> = raw
        .trim()
        .replace('\\', "/")
        .split('/')
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .map(ToOwned::to_owned)
        .collect();
    if clean.is_empty() {
        "/".to_owned()
    } else {
        format!("/{}", clean.join("/"))
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

pub fn load_webdav_store(path: &Path) -> WebDavStore {
    let Ok(raw) = fs::read_to_string(path) else {
        return WebDavStore::default();
    };
    parse_webdav_store(&raw)
}

pub fn save_webdav_store(path: &Path, store: &WebDavStore) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all webdav.cfg: {e}"))?;
    }
    let tmp = path.with_extension("cfg.tmp");
    fs::write(&tmp, serialize_webdav_store(store)).map_err(|e| format!("write webdav.cfg: {e}"))?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).map_err(|e| format!("rename webdav.cfg: {e}"))?;
    Ok(())
}

pub fn parse_webdav_store(raw: &str) -> WebDavStore {
    let text = raw.trim_start_matches('\u{FEFF}');
    if text.contains("[server]") {
        return parse_v2_store(text);
    }
    migrate_v1_store(text)
}

fn parse_v2_store(text: &str) -> WebDavStore {
    let mut store = WebDavStore::default();
    let mut cur: Option<WebDavServer> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if trimmed == "[server]" {
            if let Some(srv) = cur.take() {
                if !srv.id.is_empty() {
                    store.servers.push(srv);
                }
            }
            cur = Some(WebDavServer {
                id: String::new(),
                name: String::new(),
                provider: WebDavProvider::Yandex,
                custom_url: String::new(),
                username: String::new(),
                remote_dir: "/asrbox-backups".to_owned(),
                legacy_cred_target: None,
            });
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        let Some(srv) = cur.as_mut() else {
            continue;
        };
        apply_server_field(srv, k.trim(), v.trim());
    }
    if let Some(srv) = cur.take() {
        if !srv.id.is_empty() {
            store.servers.push(srv);
        }
    }
    store
}

fn apply_server_field(srv: &mut WebDavServer, key: &str, val: &str) {
    match key {
        "id" => srv.id = val.to_owned(),
        "name" => srv.name = val.to_owned(),
        "provider" => srv.provider = WebDavProvider::parse(val),
        "custom_url" => srv.custom_url = val.to_owned(),
        "username" => srv.username = val.to_owned(),
        "remote_dir" if !val.is_empty() => srv.remote_dir = normalize_remote_dir(val),
        "legacy_cred_target" if !val.is_empty() => srv.legacy_cred_target = Some(val.to_owned()),
        _ => {}
    }
}

fn migrate_v1_store(text: &str) -> WebDavStore {
    let mut provider = WebDavProvider::Yandex;
    let mut custom_url = String::new();
    let mut username = String::new();
    let mut remote_dir = "/asrbox-backups".to_owned();
    let mut saw_any = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        match k.trim() {
            "provider" => {
                provider = WebDavProvider::parse(v);
                saw_any = true;
            }
            "custom_url" => custom_url = v.trim().to_owned(),
            "username" => {
                username = v.trim().to_owned();
                if !username.is_empty() {
                    saw_any = true;
                }
            }
            "remote_dir" if !v.trim().is_empty() => remote_dir = normalize_remote_dir(v),
            _ => {}
        }
    }
    if !saw_any || (username.is_empty() && custom_url.is_empty()) {
        return WebDavStore::default();
    }
    let name = if username.is_empty() {
        provider.label_ru().to_owned()
    } else {
        format!("{} ({username})", provider.label_ru())
    };
    WebDavStore {
        servers: vec![WebDavServer {
            id: "srv-1".to_owned(),
            name,
            provider,
            custom_url,
            username,
            remote_dir,
            legacy_cred_target: Some(provider.legacy_cred_target().to_owned()),
        }],
    }
}

pub fn serialize_webdav_store(store: &WebDavStore) -> String {
    let mut out = String::new();
    out.push_str(
        "# Настройки WebDAV серверов (asrbox) — пароли хранятся только в Windows Credential Manager\n",
    );
    out.push_str("settings_version=2\n");
    out.push_str(&format!("written_by={}\n", env!("CARGO_PKG_VERSION")));
    for srv in &store.servers {
        out.push_str("\n[server]\n");
        out.push_str(&format!("id={}\n", srv.id.trim()));
        out.push_str(&format!("name={}\n", srv.name.trim()));
        out.push_str(&format!("provider={}\n", srv.provider.as_str()));
        out.push_str(&format!("custom_url={}\n", srv.custom_url.trim()));
        out.push_str(&format!("username={}\n", srv.username.trim()));
        out.push_str(&format!("remote_dir={}\n", srv.normalized_remote_dir()));
        if let Some(ref legacy) = srv.legacy_cred_target {
            if !legacy.trim().is_empty() {
                out.push_str(&format!("legacy_cred_target={}\n", legacy.trim()));
            }
        }
    }
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
    fn webdav_store_v2_roundtrip_and_v1_migration() {
        let v1 = "settings_version=1\nprovider=mailru\nusername=user@mail.ru\nremote_dir=/asrbox-backups\n";
        let migrated = parse_webdav_store(v1);
        assert_eq!(migrated.servers.len(), 1);
        assert_eq!(migrated.servers[0].provider, WebDavProvider::MailRu);
        assert_eq!(migrated.servers[0].cred_target(), "asrbox:webdav:mailru");
        assert_eq!(
            parse_webdav_store(&serialize_webdav_store(&migrated)),
            migrated
        );
        assert!(validate_https_url("https://webdav.yandex.ru").is_ok());
        assert!(validate_https_url("http://insecure.example").is_err());
        assert_eq!(base64_encode(b"user:pass"), "dXNlcjpwYXNz");
    }
}
