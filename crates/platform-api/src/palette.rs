use crate::theme::{AccentPreset, ThemeMode};

/// Complete Windows 11 Fluent Design color palette supporting Light, Deep Dark,
/// custom Accent presets, and smooth per-channel interpolation (`lerp`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluentPalette {
    pub is_dark: bool,
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
        Self::from_theme(ThemeMode::Dark, AccentPreset::Blue)
    }
}

impl FluentPalette {
    pub fn dark(accent_preset: AccentPreset) -> Self {
        Self::from_theme(ThemeMode::Dark, accent_preset)
    }

    pub fn from_theme(mode: ThemeMode, accent_preset: AccentPreset) -> Self {
        let dark = mode.is_dark();
        let (accent, accent_hover, accent_on) = accent_preset.colors(dark);
        if dark {
            Self {
                is_dark: true,
                window_bg: [32, 32, 32, 255],      // #202020
                sidebar_bg: [25, 25, 25, 255],     // #191919
                card_bg: [43, 43, 43, 255],        // #2B2B2B
                card_border: [53, 53, 53, 255],    // #353535
                divider: [53, 53, 53, 255],        // #353535
                nav_active_bg: [45, 45, 45, 255],  // #2D2D2D
                nav_hover_bg: [39, 39, 39, 255],   // #272727
                control_bg: [51, 51, 51, 255],     // #333333
                control_hover: [62, 62, 62, 255],  // #3E3E3E
                control_border: [68, 68, 68, 255], // #444444
                input_bg: [31, 31, 31, 255],       // #1F1F1F
                menu_bg: [36, 36, 36, 255],        // #242424
                menu_border: [56, 56, 56, 255],    // #383838
                menu_hover: [51, 51, 51, 255],     // #333333
                menu_divider: [53, 53, 53, 255],   // #353535
                text_primary: [255, 255, 255, 255],
                text_secondary: [160, 160, 160, 255], // #A0A0A0
                text_muted: [142, 142, 142, 255],     // #8E8E8E
                accent,
                accent_hover,
                accent_on,
                danger_bg: [184, 52, 52, 255],
                danger_hover: [212, 66, 66, 255],
                warn_bg: [58, 46, 26, 255],
                warn_border: [198, 138, 44, 255],
                warn_text: [252, 212, 132, 255],
                success: [84, 206, 138, 255],
            }
        } else {
            Self {
                is_dark: false,
                window_bg: [243, 243, 243, 255],      // #F3F3F3
                sidebar_bg: [243, 243, 243, 255],     // #F3F3F3
                card_bg: [255, 255, 255, 255],        // #FFFFFF
                card_border: [229, 229, 229, 255],    // #E5E5E5
                divider: [229, 229, 229, 255],        // #E5E5E5
                nav_active_bg: [234, 234, 234, 255],  // #EAEAEA
                nav_hover_bg: [238, 238, 238, 255],   // #EEEEEE
                control_bg: [249, 249, 249, 255],     // #F9F9F9
                control_hover: [239, 239, 239, 255],  // #EFEFEF
                control_border: [214, 214, 214, 255], // #D6D6D6
                input_bg: [255, 255, 255, 255],       // #FFFFFF
                menu_bg: [249, 249, 249, 255],        // #F9F9F9
                menu_border: [229, 229, 229, 255],    // #E5E5E5
                menu_hover: [240, 240, 240, 255],     // #F0F0F0
                menu_divider: [229, 229, 229, 255],   // #E5E5E5
                text_primary: [26, 26, 26, 255],      // #1A1A1A
                text_secondary: [93, 93, 93, 255],    // #5D5D5D
                text_muted: [122, 122, 122, 255],     // #7A7A7A
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
    }

    pub fn lerp(a: Self, b: Self, t: f32) -> Self {
        let k = t.clamp(0.0, 1.0);
        Self {
            is_dark: if k < 0.5 { a.is_dark } else { b.is_dark },
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
            theme: ThemeMode::Light,
            accent: AccentPreset::Emerald,
            language: AppLanguage::En,
        };
        assert_eq!(
            parse_appearance_config(&serialize_appearance_config(&cfg)),
            cfg
        );
        let dark = FluentPalette::from_theme(ThemeMode::Dark, AccentPreset::Blue);
        let light = FluentPalette::from_theme(ThemeMode::Light, AccentPreset::Blue);
        assert_eq!(dark.accent, [76, 194, 255, 255]);
        assert_eq!(light.accent, [0, 103, 192, 255]);
        let mid = FluentPalette::lerp(dark, light, 0.5);
        assert!(mid.window_bg[0] > dark.window_bg[0] && mid.window_bg[0] < light.window_bg[0]);
    }
}
