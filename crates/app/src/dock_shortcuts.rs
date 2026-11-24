#![cfg_attr(not(windows), allow(dead_code))]

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use asrbox_mod_dock::DockConfig;

pub fn shortcuts_dir_for_config(docks_cfg: &Path) -> PathBuf {
    if let Some(cfg_dir) = docks_cfg.parent() {
        if cfg_dir.file_name().is_some_and(|n| n == "config") {
            if let Some(data_dir) = cfg_dir.parent() {
                return data_dir.join("shortcuts");
            }
        }
        return cfg_dir.join("shortcuts");
    }
    PathBuf::from("data").join("shortcuts")
}

pub fn is_shortcut_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("lnk") || ext.eq_ignore_ascii_case("url"))
}

pub fn is_inside_dir(path: &Path, dir: &Path) -> bool {
    let norm = |p: &Path| {
        p.to_string_lossy()
            .replace('/', "\\")
            .trim_end_matches('\\')
            .to_ascii_lowercase()
    };
    let dir_s = norm(dir);
    if dir_s.is_empty() {
        return false;
    }
    let path_s = norm(path);
    path_s.starts_with(&format!("{dir_s}\\"))
}

/// Copies dropped `.lnk` / `.url` files into portable `data/shortcuts/` so deleting
/// the original desktop shortcut never breaks the dock item.
pub fn localize_shortcut_path(source: &Path, shortcuts_dir: &Path) -> PathBuf {
    if !is_shortcut_file(source) || !source.is_file() || is_inside_dir(source, shortcuts_dir) {
        return source.to_path_buf();
    }
    if fs::create_dir_all(shortcuts_dir).is_err() {
        return source.to_path_buf();
    }
    let Ok(src_bytes) = fs::read(source) else {
        return source.to_path_buf();
    };
    let ext = source
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("lnk")
        .to_ascii_lowercase();
    let stem = sanitize_stem(
        source
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("shortcut"),
    );

    for idx in 1u32..=999 {
        let name = if idx == 1 {
            format!("{stem}.{ext}")
        } else {
            format!("{stem}-{idx}.{ext}")
        };
        let candidate = shortcuts_dir.join(&name);
        if candidate.exists() {
            if fs::read(&candidate).is_ok_and(|b| b == src_bytes) {
                return candidate;
            }
            continue;
        }
        if fs::write(&candidate, &src_bytes).is_ok() {
            return candidate;
        }
        break;
    }
    source.to_path_buf()
}

pub fn localize_existing_docks(configs: &mut [DockConfig], shortcuts_dir: &Path) -> bool {
    let mut changed = false;
    for dock in configs {
        for item in &mut dock.items {
            if item.is_separator() {
                continue;
            }
            if is_shortcut_file(&item.target)
                && !is_inside_dir(&item.target, shortcuts_dir)
                && item.target.is_file()
            {
                let localized = localize_shortcut_path(&item.target, shortcuts_dir);
                if localized != item.target {
                    item.target = localized;
                    changed = true;
                }
            }
        }
    }
    changed
}

pub fn cleanup_unused_shortcuts(configs: &[DockConfig], shortcuts_dir: &Path) {
    let Ok(entries) = fs::read_dir(shortcuts_dir) else {
        return;
    };
    let mut referenced = HashSet::new();
    for dock in configs {
        for item in &dock.items {
            if !item.is_separator() && is_inside_dir(&item.target, shortcuts_dir) {
                if let Some(name) = item.target.file_name().and_then(|n| n.to_str()) {
                    referenced.insert(name.to_ascii_lowercase());
                }
            }
        }
    }
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() || !is_shortcut_file(&path) {
            continue;
        }
        let keep = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| referenced.contains(&n.to_ascii_lowercase()));
        if !keep {
            let _ = fs::remove_file(path);
        }
    }
}

fn sanitize_stem(raw: &str) -> String {
    let cleaned: String = raw
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.');
    if trimmed.is_empty() {
        "shortcut".to_owned()
    } else {
        trimmed.chars().take(64).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use asrbox_mod_dock::DockItem;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_temp_dir(tag: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("asrbox_sc_{tag}_{nanos}"));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn lnk_shortcut_is_copied_and_survives_source_deletion() {
        let root = make_temp_dir("copy");
        let desktop = root.join("desktop");
        let shortcuts = root.join("data").join("shortcuts");
        let _ = fs::create_dir_all(&desktop);
        let src = desktop.join("Telegram.lnk");
        let _ = fs::write(&src, b"fake-lnk-payload-v1");

        let localized = localize_shortcut_path(&src, &shortcuts);
        assert!(is_inside_dir(&localized, &shortcuts));
        assert!(localized.is_file());

        // Delete original desktop shortcut: portable copy must survive!
        let _ = fs::remove_file(&src);
        assert!(!src.exists());
        assert_eq!(fs::read(&localized).ok().as_deref(), Some(&b"fake-lnk-payload-v1"[..]));

        // Non-shortcut (.exe) is never copied.
        let exe = desktop.join("app.exe");
        let _ = fs::write(&exe, b"MZ");
        assert_eq!(localize_shortcut_path(&exe, &shortcuts), exe);

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn cleanup_deletes_unreferenced_shortcuts_only() {
        let root = make_temp_dir("clean");
        let shortcuts = root.join("shortcuts");
        let _ = fs::create_dir_all(&shortcuts);
        let keep = shortcuts.join("Keep.lnk");
        let orphan = shortcuts.join("Orphan.lnk");
        let _ = fs::write(&keep, b"a");
        let _ = fs::write(&orphan, b"b");

        let mut dock = DockConfig::new(1, "Док 1");
        if let Ok(item) = DockItem::from_dropped_path(1, keep.clone()) {
            dock.items.push(item);
        }
        cleanup_unused_shortcuts(&[dock], &shortcuts);

        assert!(keep.exists());
        assert!(!orphan.exists());
        let _ = fs::remove_dir_all(&root);
    }
}
