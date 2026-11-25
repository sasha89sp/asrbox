use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub fn default_expanded_sections() -> HashSet<String> {
    [
        "appearance",
        "system",
        "dock_appearance",
        "cal_main",
        "sys_main",
        "weather_main",
        "notes_main",
        "overlay_mode",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

pub fn load_expanded_sections(path: &Path) -> HashSet<String> {
    let Ok(raw) = fs::read_to_string(path) else {
        return default_expanded_sections();
    };
    for line in raw.trim_start_matches('\u{FEFF}').lines() {
        let trimmed = line.trim();
        if let Some((k, v)) = trimmed.split_once('=') {
            if k.trim() == "expanded_sections" {
                return v
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect();
            }
        }
    }
    default_expanded_sections()
}

pub fn save_expanded_sections(path: &Path, expanded: &HashSet<String>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all settings_ui.cfg: {e}"))?;
    }
    let mut keys: Vec<&str> = expanded.iter().map(String::as_str).collect();
    keys.sort_unstable();
    let content = format!(
        "# Состояние аккордеонов окна настроек asrbox\nversion=1\nexpanded_sections={}\n",
        keys.join(",")
    );
    let tmp = path.with_extension("cfg.tmp");
    fs::write(&tmp, content).map_err(|e| format!("write settings_ui.cfg: {e}"))?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).map_err(|e| format!("rename settings_ui.cfg: {e}"))?;
    Ok(())
}
