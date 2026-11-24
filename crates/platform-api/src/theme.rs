use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeMode {
    #[default]
    Dark,
    Light,
}

impl ThemeMode {
    pub fn as_str(self) -> &'static str {
        if self == Self::Light {
            "light"
        } else {
            "dark"
        }
    }
    pub fn parse(raw: &str) -> Self {
        if raw.trim().eq_ignore_ascii_case("light") {
            Self::Light
        } else {
            Self::Dark
        }
    }
    pub fn is_dark(self) -> bool {
        self == Self::Dark
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccentPreset {
    #[default]
    Blue,
    Violet,
    Emerald,
    Amber,
    Coral,
    Slate,
}

impl AccentPreset {
    pub const ALL: [Self; 6] = [
        Self::Blue,
        Self::Violet,
        Self::Emerald,
        Self::Amber,
        Self::Coral,
        Self::Slate,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blue => "blue",
            Self::Violet => "violet",
            Self::Emerald => "emerald",
            Self::Amber => "amber",
            Self::Coral => "coral",
            Self::Slate => "slate",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "violet" | "purple" => Self::Violet,
            "emerald" | "green" => Self::Emerald,
            "amber" | "gold" => Self::Amber,
            "coral" | "orange" => Self::Coral,
            "slate" | "gray" => Self::Slate,
            _ => Self::Blue,
        }
    }

    pub fn index(self) -> u8 {
        Self::ALL.iter().position(|&p| p == self).unwrap_or(0) as u8
    }

    pub fn from_index(idx: u8) -> Self {
        Self::ALL
            .get(usize::from(idx))
            .copied()
            .unwrap_or(Self::Blue)
    }

    pub fn label(self, lang: AppLanguage) -> &'static str {
        match (self, lang) {
            (Self::Blue, AppLanguage::Ru) => "Синий",
            (Self::Blue, AppLanguage::En) => "Blue",
            (Self::Violet, AppLanguage::Ru) => "Фиолет.",
            (Self::Violet, AppLanguage::En) => "Violet",
            (Self::Emerald, AppLanguage::Ru) => "Изумруд",
            (Self::Emerald, AppLanguage::En) => "Emerald",
            (Self::Amber, AppLanguage::Ru) => "Янтарь",
            (Self::Amber, AppLanguage::En) => "Amber",
            (Self::Coral, AppLanguage::Ru) => "Коралл",
            (Self::Coral, AppLanguage::En) => "Coral",
            (Self::Slate, AppLanguage::Ru) => "Графит",
            (Self::Slate, AppLanguage::En) => "Slate",
        }
    }

    pub fn colors(self, dark: bool) -> ([u8; 4], [u8; 4], [u8; 4]) {
        // Returns (accent, accent_hover, accent_text_on_fill)
        match (self, dark) {
            (Self::Blue, false) => ([0, 103, 192, 255], [0, 90, 170, 255], [255, 255, 255, 255]),
            (Self::Blue, true) => ([76, 194, 255, 255], [108, 208, 255, 255], [16, 16, 16, 255]),
            (Self::Violet, false) => (
                [116, 77, 169, 255],
                [100, 62, 152, 255],
                [255, 255, 255, 255],
            ),
            (Self::Violet, true) => (
                [178, 136, 255, 255],
                [196, 162, 255, 255],
                [16, 16, 16, 255],
            ),
            (Self::Emerald, false) => {
                ([15, 123, 84, 255], [12, 106, 72, 255], [255, 255, 255, 255])
            }
            (Self::Emerald, true) => ([76, 214, 156, 255], [106, 228, 176, 255], [16, 16, 16, 255]),
            (Self::Amber, false) => (
                [182, 114, 14, 255],
                [160, 98, 10, 255],
                [255, 255, 255, 255],
            ),
            (Self::Amber, true) => ([246, 186, 74, 255], [252, 202, 106, 255], [16, 16, 16, 255]),
            (Self::Coral, false) => ([209, 78, 52, 255], [188, 66, 42, 255], [255, 255, 255, 255]),
            (Self::Coral, true) => (
                [255, 132, 102, 255],
                [255, 156, 130, 255],
                [16, 16, 16, 255],
            ),
            (Self::Slate, false) => ([82, 92, 106, 255], [66, 76, 90, 255], [255, 255, 255, 255]),
            (Self::Slate, true) => (
                [168, 180, 198, 255],
                [188, 198, 214, 255],
                [16, 16, 16, 255],
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppLanguage {
    #[default]
    Ru,
    En,
}

impl AppLanguage {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ru => "ru",
            Self::En => "en",
        }
    }

    pub fn parse(raw: &str) -> Self {
        if raw.trim().eq_ignore_ascii_case("en") {
            Self::En
        } else {
            Self::Ru
        }
    }

    pub fn index(self) -> u8 {
        match self {
            Self::Ru => 0,
            Self::En => 1,
        }
    }

    pub fn from_index(idx: u8) -> Self {
        if idx == 1 {
            Self::En
        } else {
            Self::Ru
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AppearanceConfig {
    pub theme: ThemeMode,
    pub accent: AccentPreset,
    pub language: AppLanguage,
}

impl AppearanceConfig {
    pub fn palette(self) -> FluentPalette {
        FluentPalette::from_theme(self.theme, self.accent)
    }
}

pub fn load_appearance_config(path: &Path) -> AppearanceConfig {
    let Ok(raw) = fs::read_to_string(path) else {
        return AppearanceConfig::default();
    };
    parse_appearance_config(&raw)
}

pub fn save_appearance_config(path: &Path, cfg: &AppearanceConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all appearance.cfg: {e}"))?;
    }
    let tmp = path.with_extension("cfg.tmp");
    fs::write(&tmp, serialize_appearance_config(cfg))
        .map_err(|e| format!("write appearance.cfg: {e}"))?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).map_err(|e| format!("rename appearance.cfg: {e}"))?;
    Ok(())
}

pub fn parse_appearance_config(raw: &str) -> AppearanceConfig {
    let mut cfg = AppearanceConfig::default();
    for line in raw.trim_start_matches('\u{FEFF}').lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        match k.trim() {
            "theme" => cfg.theme = ThemeMode::parse(v),
            "accent" => cfg.accent = AccentPreset::parse(v),
            "language" => cfg.language = AppLanguage::parse(v),
            _ => {}
        }
    }
    cfg
}

pub fn serialize_appearance_config(cfg: &AppearanceConfig) -> String {
    format!(
        "# Оформление Windows 11 Fluent Design (asrbox)\n\
         settings_version=1\n\
         theme={}\n\
         accent={}\n\
         language={}\n",
        cfg.theme.as_str(),
        cfg.accent.as_str(),
        cfg.language.as_str(),
    )
}

pub use crate::palette::{lerp_rgba, FluentPalette};
