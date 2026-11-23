use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModuleId {
    Dock,
    Widgets,
}

impl ModuleId {
    pub const ALL: [Self; 2] = [Self::Dock, Self::Widgets];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dock => "dock",
            Self::Widgets => "widgets",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Dock => "Док-панели",
            Self::Widgets => "Google Календарь",
        }
    }

    pub fn from_str_id(raw: &str) -> Option<Self> {
        match raw.trim() {
            "dock" => Some(Self::Dock),
            "widgets" => Some(Self::Widgets),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IsolationMode {
    #[default]
    Process,
    InProc,
}

impl IsolationMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Process => "process",
            Self::InProc => "inproc",
        }
    }

    pub fn from_str_value(raw: &str) -> Self {
        if raw.trim().eq_ignore_ascii_case("inproc") {
            Self::InProc
        } else {
            Self::Process
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModulesConfig {
    pub isolation: IsolationMode,
    pub dock_enabled: bool,
    pub widgets_enabled: bool,
}

impl Default for ModulesConfig {
    fn default() -> Self {
        Self {
            isolation: IsolationMode::Process,
            dock_enabled: true,
            widgets_enabled: true,
        }
    }
}

impl ModulesConfig {
    pub fn is_enabled(&self, id: ModuleId) -> bool {
        match id {
            ModuleId::Dock => self.dock_enabled,
            ModuleId::Widgets => self.widgets_enabled,
        }
    }

    pub fn set_enabled(&mut self, id: ModuleId, enabled: bool) {
        match id {
            ModuleId::Dock => self.dock_enabled = enabled,
            ModuleId::Widgets => self.widgets_enabled = enabled,
        }
    }
}

pub fn load_modules_config(path: &Path) -> Result<ModulesConfig, String> {
    if !path.exists() {
        let cfg = ModulesConfig::default();
        let _ = save_modules_config(path, &cfg);
        return Ok(cfg);
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("failed to read modules.cfg: {e}"))?;
    Ok(parse_modules_config(&raw))
}

pub fn save_modules_config(path: &Path, cfg: &ModulesConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create config dir: {e}"))?;
    }
    let tmp = path.with_extension("cfg.tmp");
    let text = serialize_modules_config(cfg);
    fs::write(&tmp, text).map_err(|e| format!("failed to write tmp modules.cfg: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("failed to commit modules.cfg: {e}"))?;
    Ok(())
}

pub fn parse_modules_config(raw: &str) -> ModulesConfig {
    let mut cfg = ModulesConfig::default();
    for line in raw.trim_start_matches('\u{FEFF}').lines() {
        let trimmed = line.trim().trim_start_matches('\u{FEFF}');
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        let key = k.trim();
        let val = v.trim();
        match key {
            "isolation" => cfg.isolation = IsolationMode::from_str_value(val),
            "module.dock" => cfg.dock_enabled = parse_bool(val),
            "module.widgets" => cfg.widgets_enabled = parse_bool(val),
            _ => {}
        }
    }
    cfg
}

pub fn serialize_modules_config(cfg: &ModulesConfig) -> String {
    format!(
        concat!(
            "# Настройки модулей и изоляции процессов asrbox\n",
            "# isolation=process (каждый модуль в отдельном процессе) или inproc\n",
            "settings_version=1\n",
            "isolation={}\n",
            "module.dock={}\n",
            "module.widgets={}\n",
        ),
        cfg.isolation.as_str(),
        if cfg.dock_enabled { 1 } else { 0 },
        if cfg.widgets_enabled { 1 } else { 0 },
    )
}

fn parse_bool(v: &str) -> bool {
    v == "1" || v.eq_ignore_ascii_case("true")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modules_config_roundtrip() {
        let cfg = ModulesConfig {
            isolation: IsolationMode::InProc,
            dock_enabled: true,
            widgets_enabled: false,
        };
        let raw = serialize_modules_config(&cfg);
        assert_eq!(parse_modules_config(&raw), cfg);
    }
}
