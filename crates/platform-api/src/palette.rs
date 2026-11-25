use crate::theme::{AccentPreset, DesignTheme, FontRenderer, ThemeMode};

/// Complete color palette supporting `AsrGlass` (Glass Stack) and `Win11Classic`,
/// Light & Dark modes, custom Accent presets, and smooth per-channel interpolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluentPalette {
    pub design_theme: DesignTheme,
    pub is_dark: bool,
    pub font_renderer: FontRenderer,
    pub window_bg: [u8; 4],
    pub sidebar_bg: [u8; 4],
    pub card_bg: [u8; 4],
    pub card_border: [u8; 4],
    pub divider: [u8; 4],
    pub nav_active_bg: [u8; 4],
    pub nav_hover_bg: [u8; 4],
    pub control_bg: [u8; 4],
    pub control_hover: [u8; 4],
    pub control_border: [u8; 4],
    pub input_bg: [u8; 4],
    pub menu_bg: [u8; 4],
    pub menu_border: [u8; 4],
    pub menu_hover: [u8; 4],
    pub menu_divider: [u8; 4],
    pub text_primary: [u8; 4],
    pub text_secondary: [u8; 4],
    pub text_muted: [u8; 4],
    pub accent: [u8; 4],
    pub accent_hover: [u8; 4],
    pub accent_on: [u8; 4],
    pub danger_bg: [u8; 4],
    pub danger_hover: [u8; 4],
    pub warn_bg: [u8; 4],
    pub warn_border: [u8; 4],
    pub warn_text: [u8; 4],
    pub success: [u8; 4],
}

impl Default for FluentPalette {
    fn default() -> Self {
        Self::from_appearance(DesignTheme::AsrGlass, ThemeMode::Dark, AccentPreset::Blue)
    }
}

impl FluentPalette {
    pub fn dark(accent_preset: AccentPreset) -> Self {
        Self::from_appearance(DesignTheme::AsrGlass, ThemeMode::Dark, accent_preset)
    }

    pub fn with_font_renderer(mut self, renderer: FontRenderer) -> Self {
        self.font_renderer = renderer;
        self
    }

    pub fn from_theme(mode: ThemeMode, accent_preset: AccentPreset) -> Self {
        Self::from_appearance(DesignTheme::AsrGlass, mode, accent_preset)
    }

    pub fn from_appearance(
        design_theme: DesignTheme,
        mode: ThemeMode,
        accent_preset: AccentPreset,
    ) -> Self {
        let dark = mode.is_dark();
        let (accent, accent_hover, accent_on) = accent_preset.colors_for_style(design_theme, dark);
        match (design_theme, dark) {
            (DesignTheme::AsrGlass, true) => Self::asr_glass_dark(accent, accent_hover, accent_on),
            (DesignTheme::AsrGlass, false) => {
                Self::asr_glass_light(accent, accent_hover, accent_on)
            }
            (DesignTheme::Win11Classic, true) => {
                Self::win11_classic_dark(accent, accent_hover, accent_on)
            }
            (DesignTheme::Win11Classic, false) => {
                Self::win11_classic_light(accent, accent_hover, accent_on)
            }
        }
    }

    fn asr_glass_dark(accent: [u8; 4], accent_hover: [u8; 4], accent_on: [u8; 4]) -> Self {
        Self {
            design_theme: DesignTheme::AsrGlass,
            is_dark: true,
            font_renderer: FontRenderer::GdiClearType,
            window_bg: [24, 27, 32, 255],      // #181B20
            sidebar_bg: [18, 20, 24, 255],     // #121418
            card_bg: [34, 38, 46, 255],        // #22262E
            card_border: [52, 58, 68, 255],    // ~rgba(255,255,255,0.08)
            divider: [44, 49, 58, 255],        // ~rgba(255,255,255,0.06)
            nav_active_bg: [32, 42, 56, 255],  // subtle tinted pill
            nav_hover_bg: [28, 32, 40, 255],   // hover row
            control_bg: [42, 47, 57, 255],     // #2A2F39
            control_hover: [48, 54, 66, 255],  // #303642
            control_border: [62, 70, 84, 255], // subtle edge
            input_bg: [22, 25, 30, 255],       // #16191E
            menu_bg: [24, 27, 32, 240],        // rgba(24,27,32,0.94)
            menu_border: [54, 60, 72, 255],
            menu_hover: [48, 54, 66, 255], // #303642
            menu_divider: [44, 49, 58, 255],
            text_primary: [241, 245, 249, 255],   // #F1F5F9
            text_secondary: [148, 163, 184, 255], // #94A3B8
            text_muted: [100, 116, 139, 255],     // #64748B
            accent,
            accent_hover,
            accent_on,
            danger_bg: [190, 52, 56, 255],
            danger_hover: [216, 66, 70, 255],
            warn_bg: [52, 42, 26, 255],
            warn_border: [212, 148, 42, 255],
            warn_text: [252, 214, 134, 255],
            success: [16, 185, 129, 255],
        }
    }

    fn asr_glass_light(accent: [u8; 4], accent_hover: [u8; 4], accent_on: [u8; 4]) -> Self {
        Self {
            design_theme: DesignTheme::AsrGlass,
            is_dark: false,
            font_renderer: FontRenderer::GdiClearType,
            window_bg: [243, 245, 248, 255],   // #F3F5F8
            sidebar_bg: [232, 236, 242, 255],  // #E8ECF2
            card_bg: [255, 255, 255, 255],     // #FFFFFF
            card_border: [224, 230, 238, 255], // ~rgba(15,23,42,0.08)
            divider: [234, 239, 245, 255],     // ~rgba(15,23,42,0.06)
            nav_active_bg: [218, 229, 244, 255],
            nav_hover_bg: [222, 228, 238, 255],
            control_bg: [241, 245, 249, 255],
            control_hover: [226, 232, 240, 255], // #E2E8F0
            control_border: [203, 213, 225, 255],
            input_bg: [255, 255, 255, 255], // #FFFFFF
            menu_bg: [250, 251, 253, 245],  // rgba(250,251,253,0.96)
            menu_border: [218, 225, 235, 255],
            menu_hover: [226, 232, 240, 255],
            menu_divider: [232, 237, 243, 255],
            text_primary: [15, 23, 42, 255],    // #0F172A
            text_secondary: [71, 85, 105, 255], // #475569
            text_muted: [148, 163, 184, 255],   // #94A3B8
            accent,
            accent_hover,
            accent_on,
            danger_bg: [208, 48, 38, 255],
            danger_hover: [184, 38, 30, 255],
            warn_bg: [255, 248, 231, 255],
            warn_border: [216, 148, 34, 255],
            warn_text: [138, 86, 12, 255],
            success: [5, 150, 105, 255],
        }
    }

    fn win11_classic_dark(accent: [u8; 4], accent_hover: [u8; 4], accent_on: [u8; 4]) -> Self {
        Self {
            design_theme: DesignTheme::Win11Classic,
            is_dark: true,
            font_renderer: FontRenderer::GdiClearType,
            window_bg: [32, 32, 32, 255],
            sidebar_bg: [32, 32, 32, 255],
            card_bg: [43, 43, 43, 255],
            card_border: [50, 50, 50, 255],
            divider: [45, 45, 45, 255],
            nav_active_bg: [45, 45, 45, 255],
            nav_hover_bg: [40, 40, 40, 255],
            control_bg: [54, 54, 54, 255],
            control_hover: [62, 62, 62, 255],
            control_border: [66, 66, 66, 255],
            input_bg: [37, 37, 37, 255],
            menu_bg: [40, 40, 40, 255],
            menu_border: [56, 56, 56, 255],
            menu_hover: [54, 54, 54, 255],
            menu_divider: [52, 52, 52, 255],
            text_primary: [255, 255, 255, 255],
            text_secondary: [206, 206, 206, 255],
            text_muted: [166, 166, 166, 255],
            accent,
            accent_hover,
            accent_on,
            danger_bg: [180, 52, 52, 255],
            danger_hover: [206, 64, 64, 255],
            warn_bg: [54, 45, 28, 255],
            warn_border: [188, 134, 48, 255],
            warn_text: [248, 212, 138, 255],
            success: [84, 206, 138, 255],
        }
    }

    fn win11_classic_light(accent: [u8; 4], accent_hover: [u8; 4], accent_on: [u8; 4]) -> Self {
        Self {
            design_theme: DesignTheme::Win11Classic,
            is_dark: false,
            font_renderer: FontRenderer::GdiClearType,
            window_bg: [243, 243, 243, 255],
            sidebar_bg: [243, 243, 243, 255],
            card_bg: [251, 251, 251, 255],
            card_border: [229, 229, 229, 255],
            divider: [232, 232, 232, 255],
            nav_active_bg: [234, 234, 234, 255],
            nav_hover_bg: [238, 238, 238, 255],
            control_bg: [249, 249, 249, 255],
            control_hover: [240, 240, 240, 255],
            control_border: [218, 218, 218, 255],
            input_bg: [255, 255, 255, 255],
            menu_bg: [249, 249, 249, 255],
            menu_border: [229, 229, 229, 255],
            menu_hover: [240, 240, 240, 255],
            menu_divider: [229, 229, 229, 255],
            text_primary: [26, 26, 26, 255],
            text_secondary: [96, 96, 96, 255],
            text_muted: [120, 120, 120, 255],
            accent,
            accent_hover,
            accent_on,
            danger_bg: [196, 43, 28, 255],
            danger_hover: [176, 36, 22, 255],
            warn_bg: [255, 248, 231, 255],
            warn_border: [216, 148, 34, 255],
            warn_text: [138, 86, 12, 255],
            success: [16, 124, 65, 255],
        }
    }

    pub fn lerp(a: Self, b: Self, t: f32) -> Self {
        let k = t.clamp(0.0, 1.0);
        Self {
            design_theme: if k < 0.5 {
                a.design_theme
            } else {
                b.design_theme
            },
            is_dark: if k < 0.5 { a.is_dark } else { b.is_dark },
            font_renderer: b.font_renderer,
            window_bg: lerp_rgba(a.window_bg, b.window_bg, k),
            sidebar_bg: lerp_rgba(a.sidebar_bg, b.sidebar_bg, k),
            card_bg: lerp_rgba(a.card_bg, b.card_bg, k),
            card_border: lerp_rgba(a.card_border, b.card_border, k),
            divider: lerp_rgba(a.divider, b.divider, k),
            nav_active_bg: lerp_rgba(a.nav_active_bg, b.nav_active_bg, k),
            nav_hover_bg: lerp_rgba(a.nav_hover_bg, b.nav_hover_bg, k),
            control_bg: lerp_rgba(a.control_bg, b.control_bg, k),
            control_hover: lerp_rgba(a.control_hover, b.control_hover, k),
            control_border: lerp_rgba(a.control_border, b.control_border, k),
            input_bg: lerp_rgba(a.input_bg, b.input_bg, k),
            menu_bg: lerp_rgba(a.menu_bg, b.menu_bg, k),
            menu_border: lerp_rgba(a.menu_border, b.menu_border, k),
            menu_hover: lerp_rgba(a.menu_hover, b.menu_hover, k),
            menu_divider: lerp_rgba(a.menu_divider, b.menu_divider, k),
            text_primary: lerp_rgba(a.text_primary, b.text_primary, k),
            text_secondary: lerp_rgba(a.text_secondary, b.text_secondary, k),
            text_muted: lerp_rgba(a.text_muted, b.text_muted, k),
            accent: lerp_rgba(a.accent, b.accent, k),
            accent_hover: lerp_rgba(a.accent_hover, b.accent_hover, k),
            accent_on: lerp_rgba(a.accent_on, b.accent_on, k),
            danger_bg: lerp_rgba(a.danger_bg, b.danger_bg, k),
            danger_hover: lerp_rgba(a.danger_hover, b.danger_hover, k),
            warn_bg: lerp_rgba(a.warn_bg, b.warn_bg, k),
            warn_border: lerp_rgba(a.warn_border, b.warn_border, k),
            warn_text: lerp_rgba(a.warn_text, b.warn_text, k),
            success: lerp_rgba(a.success, b.success, k),
        }
    }
}

pub fn lerp_rgba(a: [u8; 4], b: [u8; 4], t: f32) -> [u8; 4] {
    [
        lerp_u8(a[0], b[0], t),
        lerp_u8(a[1], b[1], t),
        lerp_u8(a[2], b[2], t),
        lerp_u8(a[3], b[3], t),
    ]
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (f32::from(a) + (f32::from(b) - f32::from(a)) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{
        parse_appearance_config, serialize_appearance_config, AppLanguage, AppearanceConfig,
    };

    #[test]
    fn appearance_config_and_palette_lerp_work() {
        let cfg = AppearanceConfig {
            design_theme: DesignTheme::Win11Classic,
            theme: ThemeMode::Light,
            accent: AccentPreset::Cyan,
            language: AppLanguage::En,
            font_renderer: FontRenderer::DirectWrite,
        };
        assert_eq!(
            parse_appearance_config(&serialize_appearance_config(&cfg)),
            cfg
        );
        assert_eq!(cfg.palette().design_theme, DesignTheme::Win11Classic);
        let glass_dark = FluentPalette::from_appearance(
            DesignTheme::AsrGlass,
            ThemeMode::Dark,
            AccentPreset::Blue,
        );
        assert_eq!(glass_dark.window_bg, [24, 27, 32, 255]);
        assert_eq!(glass_dark.accent, [43, 144, 255, 255]);
        let glass_light = FluentPalette::from_appearance(
            DesignTheme::AsrGlass,
            ThemeMode::Light,
            AccentPreset::Blue,
        );
        assert_eq!(glass_light.window_bg, [243, 245, 248, 255]);
        let mid = FluentPalette::lerp(glass_dark, glass_light, 0.5);
        assert!(mid.window_bg[0] > glass_dark.window_bg[0]);
    }
}
