use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DesignTheme {
    #[default]
    AsrGlass,
    Win11Classic,
}

impl DesignTheme {
    pub const ALL: [Self; 2] = [Self::AsrGlass, Self::Win11Classic];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AsrGlass => "asr_glass",
            Self::Win11Classic => "win11_classic",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "win11_classic" | "win11" | "classic" => Self::Win11Classic,
            _ => Self::AsrGlass,
        }
    }

    pub fn index(self) -> u8 {
        match self {
            Self::AsrGlass => 0,
            Self::Win11Classic => 1,
        }
    }

    pub fn from_index(idx: u8) -> Self {
        if idx == 1 {
            Self::Win11Classic
        } else {
            Self::AsrGlass
        }
    }

    pub fn label(self, lang: AppLanguage) -> &'static str {
        match (self, lang) {
            (Self::AsrGlass, AppLanguage::Ru) => "Стеклянный блок ASR",
            (Self::AsrGlass, AppLanguage::En) => "ASR Glass Stack",
            (Self::Win11Classic, AppLanguage::Ru) => "Windows 11 Classic",
            (Self::Win11Classic, AppLanguage::En) => "Windows 11 Classic",
        }
    }

    pub fn is_glass(self) -> bool {
        self == Self::AsrGlass
    }
}

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
    pub fn label(self, lang: AppLanguage) -> &'static str {
        match (self, lang) {
            (Self::Dark, AppLanguage::Ru) => "Тёмная",
            (Self::Dark, AppLanguage::En) => "Dark",
            (Self::Light, AppLanguage::Ru) => "Светлая",
            (Self::Light, AppLanguage::En) => "Light",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AccentPreset {
    #[default]
    Blue,
    Cyan,
    Emerald,
    Amber,
    Violet,
    Coral,
    Slate,
}

impl AccentPreset {
    pub const ALL: [Self; 7] = [
        Self::Blue,
        Self::Cyan,
        Self::Emerald,
        Self::Amber,
        Self::Violet,
        Self::Coral,
        Self::Slate,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Blue => "blue",
            Self::Cyan => "cyan",
            Self::Emerald => "emerald",
            Self::Amber => "amber",
            Self::Violet => "violet",
            Self::Coral => "coral",
            Self::Slate => "slate",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "cyan" | "teal" | "turquoise" => Self::Cyan,
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
            (Self::Cyan, AppLanguage::Ru) => "Бирюзовый",
            (Self::Cyan, AppLanguage::En) => "Cyan",
            (Self::Emerald, AppLanguage::Ru) => "Изумрудный",
            (Self::Emerald, AppLanguage::En) => "Emerald",
            (Self::Amber, AppLanguage::Ru) => "Янтарный",
            (Self::Amber, AppLanguage::En) => "Amber",
            (Self::Violet, AppLanguage::Ru) => "Фиолетовый",
            (Self::Violet, AppLanguage::En) => "Violet",
            (Self::Coral, AppLanguage::Ru) => "Коралл",
            (Self::Coral, AppLanguage::En) => "Coral",
            (Self::Slate, AppLanguage::Ru) => "Графит",
            (Self::Slate, AppLanguage::En) => "Slate",
        }
    }

    pub fn colors(self, dark: bool) -> ([u8; 4], [u8; 4], [u8; 4]) {
        self.colors_for_style(DesignTheme::AsrGlass, dark)
    }

    pub fn colors_for_style(self, design: DesignTheme, dark: bool) -> ([u8; 4], [u8; 4], [u8; 4]) {
        if design == DesignTheme::AsrGlass {
            return match (self, dark) {
                (Self::Blue, true) => (
                    [43, 144, 255, 255],
                    [72, 162, 255, 255],
                    [255, 255, 255, 255],
                ),
                (Self::Blue, false) => {
                    ([0, 103, 192, 255], [0, 88, 168, 255], [255, 255, 255, 255])
                }
                (Self::Cyan, true) => ([6, 182, 212, 255], [34, 211, 238, 255], [15, 23, 42, 255]),
                (Self::Cyan, false) => (
                    [8, 145, 178, 255],
                    [14, 116, 144, 255],
                    [255, 255, 255, 255],
                ),
                (Self::Emerald, true) => {
                    ([16, 185, 129, 255], [52, 211, 153, 255], [15, 23, 42, 255])
                }
                (Self::Emerald, false) => {
                    ([5, 150, 105, 255], [4, 120, 87, 255], [255, 255, 255, 255])
                }
                (Self::Amber, true) => {
                    ([245, 158, 11, 255], [251, 191, 36, 255], [15, 23, 42, 255])
                }
                (Self::Amber, false) => {
                    ([217, 119, 6, 255], [180, 83, 9, 255], [255, 255, 255, 255])
                }
                (Self::Violet, true) => (
                    [139, 92, 246, 255],
                    [167, 139, 250, 255],
                    [255, 255, 255, 255],
                ),
                (Self::Violet, false) => (
                    [124, 58, 237, 255],
                    [109, 40, 217, 255],
                    [255, 255, 255, 255],
                ),
                (Self::Coral, true) => {
                    ([249, 115, 92, 255], [253, 146, 126, 255], [15, 23, 42, 255])
                }
                (Self::Coral, false) => {
                    ([214, 72, 48, 255], [190, 58, 36, 255], [255, 255, 255, 255])
                }
                (Self::Slate, true) => (
                    [148, 163, 184, 255],
                    [176, 190, 208, 255],
                    [15, 23, 42, 255],
                ),
                (Self::Slate, false) => {
                    ([71, 85, 105, 255], [51, 65, 85, 255], [255, 255, 255, 255])
                }
            };
        }
        match (self, dark) {
            (Self::Blue, false) => ([0, 103, 192, 255], [0, 90, 170, 255], [255, 255, 255, 255]),
            (Self::Blue, true) => ([76, 194, 255, 255], [108, 208, 255, 255], [16, 16, 16, 255]),
            (Self::Cyan, false) => ([0, 131, 143, 255], [0, 112, 124, 255], [255, 255, 255, 255]),
            (Self::Cyan, true) => ([56, 208, 226, 255], [96, 222, 238, 255], [16, 16, 16, 255]),
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
        if self == Self::En {
            1
        } else {
            0
        }
    }

    pub fn from_index(idx: u8) -> Self {
        if idx == 1 {
            Self::En
        } else {
            Self::Ru
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ru => "Русский",
            Self::En => "English",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontRenderer {
    #[default]
    GdiClearType,
    DirectWrite,
}

impl FontRenderer {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GdiClearType => "gdi_cleartype",
            Self::DirectWrite => "directwrite",
        }
    }

    pub fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "directwrite" | "dwrite" => Self::DirectWrite,
            _ => Self::GdiClearType,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::GdiClearType => "GDI ClearType",
            Self::DirectWrite => "DirectWrite",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AppearanceConfig {
    pub design_theme: DesignTheme,
    pub theme: ThemeMode,
    pub accent: AccentPreset,
    pub language: AppLanguage,
    pub font_renderer: FontRenderer,
}

impl AppearanceConfig {
    pub fn palette(self) -> FluentPalette {
        FluentPalette::from_appearance(self.design_theme, self.theme, self.accent)
            .with_font_renderer(self.font_renderer)
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
            "design_theme" => cfg.design_theme = DesignTheme::parse(v),
            "theme" => cfg.theme = ThemeMode::parse(v),
            "accent" => cfg.accent = AccentPreset::parse(v),
            "language" => cfg.language = AppLanguage::parse(v),
            "font_renderer" => cfg.font_renderer = FontRenderer::parse(v),
            _ => {}
        }
    }
    cfg
}

pub fn serialize_appearance_config(cfg: &AppearanceConfig) -> String {
    format!(
        "# Оформление asrbox\n\
         settings_version=2\n\
         design_theme={}\n\
         theme={}\n\
         accent={}\n\
         language={}\n\
         font_renderer={}\n",
        cfg.design_theme.as_str(),
        cfg.theme.as_str(),
        cfg.accent.as_str(),
        cfg.language.as_str(),
        cfg.font_renderer.as_str(),
    )
}

pub use crate::palette::{lerp_rgba, FluentPalette};
