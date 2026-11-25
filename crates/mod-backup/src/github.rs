use std::fs;
use std::path::Path;

pub const GITHUB_TOKEN_CRED_TARGET: &str = "asrbox:github:token";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubBackupConfig {
    pub repo: String,
    pub branch: String,
    pub include_in_backup: bool,
    pub last_status: String,
}

impl Default for GitHubBackupConfig {
    fn default() -> Self {
        Self {
            repo: String::new(),
            branch: "main".to_owned(),
            include_in_backup: false,
            last_status: String::new(),
        }
    }
}

pub fn validate_github_repo(repo: &str) -> Result<String, String> {
    let s = repo.trim();
    let Some((owner, name)) = s.split_once('/') else {
        return Err("Формат репозитория: owner/repo".to_owned());
    };
    if owner.is_empty() || name.is_empty() || name.contains('/') {
        return Err("Укажите корректный owner/repo".to_owned());
    }
    let valid_part = |p: &str| {
        !p.starts_with('.')
            && !p.contains("..")
            && p.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
    };
    if !valid_part(owner) || !valid_part(name) {
        return Err("Недопустимые символы в owner/repo".to_owned());
    }
    Ok(format!("{owner}/{name}"))
}

pub fn validate_github_branch(branch: &str) -> Result<String, String> {
    let s = branch.trim();
    if s.is_empty() || s.len() > 120 || s.starts_with('/') || s.ends_with('/') || s.contains("..") {
        return Err("Укажите корректное имя ветки (например, main)".to_owned());
    }
    if !s
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'/'))
    {
        return Err("Недопустимые символы в имени ветки".to_owned());
    }
    Ok(s.to_owned())
}

pub fn load_github_config(path: &Path) -> GitHubBackupConfig {
    let Ok(raw) = fs::read_to_string(path) else {
        return GitHubBackupConfig::default();
    };
    parse_github_config(&raw)
}

pub fn save_github_config(path: &Path, cfg: &GitHubBackupConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all github_backup.cfg: {e}"))?;
    }
    let tmp = path.with_extension("cfg.tmp");
    fs::write(&tmp, serialize_github_config(cfg))
        .map_err(|e| format!("write github_backup.cfg: {e}"))?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).map_err(|e| format!("rename github_backup.cfg: {e}"))?;
    Ok(())
}

pub fn parse_github_config(raw: &str) -> GitHubBackupConfig {
    let mut cfg = GitHubBackupConfig::default();
    for line in raw.trim_start_matches('\u{FEFF}').lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        match k.trim() {
            "repo" => cfg.repo = v.trim().to_owned(),
            "branch" => {
                let b = v.trim();
                if !b.is_empty() {
                    cfg.branch = b.to_owned();
                }
            }
            "include_in_backup" => cfg.include_in_backup = v.trim() == "1" || v.trim() == "true",
            "last_status" => cfg.last_status = v.trim().to_owned(),
            _ => {}
        }
    }
    cfg
}

pub fn serialize_github_config(cfg: &GitHubBackupConfig) -> String {
    format!(
        "# Резервное копирование исходного кода с GitHub (asrbox)\n\
         # Токен хранится только в Windows Credential Manager.\n\
         version=1\n\
         repo={}\n\
         branch={}\n\
         include_in_backup={}\n\
         last_status={}\n",
        cfg.repo.trim(),
        if cfg.branch.trim().is_empty() {
            "main"
        } else {
            cfg.branch.trim()
        },
        if cfg.include_in_backup { 1 } else { 0 },
        cfg.last_status.trim(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_config_and_validation_work() {
        let gh = GitHubBackupConfig {
            repo: "owner/asrbox".to_owned(),
            branch: "main".to_owned(),
            include_in_backup: true,
            last_status: "OK".to_owned(),
        };
        assert_eq!(parse_github_config(&serialize_github_config(&gh)), gh);
        assert!(validate_github_repo("sasha89sp/asrbox").is_ok());
        assert!(validate_github_repo("../evil/repo").is_err());
        assert!(validate_github_repo("no_slash").is_err());
        assert!(validate_github_branch("feature/v0.0.8").is_ok());
        assert!(validate_github_branch("bad branch\r\n").is_err());
    }
}
