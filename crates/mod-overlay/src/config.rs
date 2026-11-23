use std::fs;
use std::path::Path;

use asrbox_platform_api::PixelRect;

pub const OVERLAY_WIDGET_ID: u64 = 3;
pub const MIN_OVERLAY_WIDTH: u32 = 220;
pub const MAX_OVERLAY_WIDTH: u32 = 640;
pub const MIN_OVERLAY_HEIGHT: u32 = 132;
pub const MAX_OVERLAY_HEIGHT: u32 = 380;
pub const MIN_OVERLAY_REFRESH_MS: u32 = 500;
pub const MAX_OVERLAY_REFRESH_MS: u32 = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverlayAnchor {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Custom,
}

impl OverlayAnchor {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TopLeft => "top_left",
            Self::TopRight => "top_right",
            Self::BottomLeft => "bottom_left",
            Self::BottomRight => "bottom_right",
            Self::Custom => "custom",
        }
    }

    pub fn from_str_value(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "top_right" => Self::TopRight,
            "bottom_left" => Self::BottomLeft,
            "bottom_right" => Self::BottomRight,
            "custom" => Self::Custom,
            _ => Self::TopLeft,
        }
    }

    pub fn to_index(self) -> u8 {
        match self {
            Self::TopLeft => 0,
            Self::TopRight => 1,
            Self::BottomLeft => 2,
            Self::BottomRight => 3,
            Self::Custom => 4,
        }
    }

    pub fn from_index(idx: u8) -> Self {
        match idx {
            1 => Self::TopRight,
            2 => Self::BottomLeft,
            3 => Self::BottomRight,
            4 => Self::Custom,
            _ => Self::TopLeft,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayConfig {
    pub auto_games_only: bool,
    pub anchor: OverlayAnchor,
    pub margin_px: i32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub opacity_percent: u8,
    pub refresh_interval_ms: u32,
    pub extra_games: Vec<String>,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            auto_games_only: true,
            anchor: OverlayAnchor::TopLeft,
            margin_px: 16,
            x: 24,
            y: 24,
            width: 320,
            height: 188,
            opacity_percent: 78,
            refresh_interval_ms: 1000,
            extra_games: Vec::new(),
        }
    }
}

impl OverlayConfig {
    pub fn compute_rect(&self, monitor: PixelRect) -> PixelRect {
        let w = self.width.clamp(MIN_OVERLAY_WIDTH, MAX_OVERLAY_WIDTH);
        let h = self.height.clamp(MIN_OVERLAY_HEIGHT, MAX_OVERLAY_HEIGHT);
        let m = self.margin_px.clamp(0, 128);
        let mon_w = i32::try_from(monitor.width).unwrap_or(1920);
        let mon_h = i32::try_from(monitor.height).unwrap_or(1080);
        let (x, y) = match self.anchor {
            OverlayAnchor::TopLeft => (monitor.x + m, monitor.y + m),
            OverlayAnchor::TopRight => (monitor.x + mon_w - w as i32 - m, monitor.y + m),
            OverlayAnchor::BottomLeft => (monitor.x + m, monitor.y + mon_h - h as i32 - m),
            OverlayAnchor::BottomRight => (
                monitor.x + mon_w - w as i32 - m,
                monitor.y + mon_h - h as i32 - m,
            ),
            OverlayAnchor::Custom => (self.x, self.y),
        };
        PixelRect {
            x,
            y,
            width: w,
            height: h,
        }
    }
}

pub fn load_overlay_config(path: &Path) -> Result<OverlayConfig, String> {
    if !path.exists() {
        let cfg = OverlayConfig::default();
        let _ = save_overlay_config(path, &cfg);
        return Ok(cfg);
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("failed to read overlay.cfg: {e}"))?;
    Ok(parse_overlay_config(&raw))
}

pub fn save_overlay_config(path: &Path, cfg: &OverlayConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create config dir: {e}"))?;
    }
    let tmp = path.with_extension("cfg.tmp");
    let text = serialize_overlay_config(cfg);
    fs::write(&tmp, text).map_err(|e| format!("failed to write tmp overlay.cfg: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("failed to commit overlay.cfg: {e}"))?;
    Ok(())
}

pub fn parse_overlay_config(raw: &str) -> OverlayConfig {
    let mut cfg = OverlayConfig::default();
    for line in raw.trim_start_matches('\u{FEFF}').lines() {
        let trimmed = line.trim().trim_start_matches('\u{FEFF}');
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        let val = v.trim();
        match k.trim() {
            "auto_games_only" => {
                cfg.auto_games_only = val == "1" || val.eq_ignore_ascii_case("true")
            }
            "anchor" => cfg.anchor = OverlayAnchor::from_str_value(val),
            "margin_px" => {
                if let Ok(n) = val.parse::<i32>() {
                    cfg.margin_px = n.clamp(0, 128);
                }
            }
            "x" => {
                if let Ok(n) = val.parse::<i32>() {
                    cfg.x = n;
                }
            }
            "y" => {
                if let Ok(n) = val.parse::<i32>() {
                    cfg.y = n;
                }
            }
            "width" => {
                if let Ok(n) = val.parse::<u32>() {
                    cfg.width = n.clamp(MIN_OVERLAY_WIDTH, MAX_OVERLAY_WIDTH);
                }
            }
            "height" => {
                if let Ok(n) = val.parse::<u32>() {
                    cfg.height = n.clamp(MIN_OVERLAY_HEIGHT, MAX_OVERLAY_HEIGHT);
                }
            }
            "opacity_percent" => {
                if let Ok(n) = val.parse::<u8>() {
                    cfg.opacity_percent = n.clamp(20, 100);
                }
            }
            "refresh_interval_ms" => {
                if let Ok(n) = val.parse::<u32>() {
                    cfg.refresh_interval_ms =
                        n.clamp(MIN_OVERLAY_REFRESH_MS, MAX_OVERLAY_REFRESH_MS);
                }
            }
            "game_process" => {
                let clean = val.trim().to_ascii_lowercase();
                if !clean.is_empty() && !cfg.extra_games.contains(&clean) {
                    cfg.extra_games.push(clean);
                }
            }
            _ => {}
        }
    }
    cfg
}

pub fn serialize_overlay_config(cfg: &OverlayConfig) -> String {
    let mut out = format!(
        concat!(
            "# Настройки игрового оверлея asrbox\n",
            "# anchor: top_left | top_right | bottom_left | bottom_right | custom\n",
            "# Доп. процессы игр в оконном режиме (по одному в строке): game_process=cs2.exe\n",
            "settings_version=1\n",
            "auto_games_only={}\n",
            "anchor={}\n",
            "margin_px={}\n",
            "x={}\n",
            "y={}\n",
            "width={}\n",
            "height={}\n",
            "opacity_percent={}\n",
            "refresh_interval_ms={}\n",
        ),
        if cfg.auto_games_only { 1 } else { 0 },
        cfg.anchor.as_str(),
        cfg.margin_px.clamp(0, 128),
        cfg.x,
        cfg.y,
        cfg.width.clamp(MIN_OVERLAY_WIDTH, MAX_OVERLAY_WIDTH),
        cfg.height.clamp(MIN_OVERLAY_HEIGHT, MAX_OVERLAY_HEIGHT),
        cfg.opacity_percent.clamp(20, 100),
        cfg.refresh_interval_ms
            .clamp(MIN_OVERLAY_REFRESH_MS, MAX_OVERLAY_REFRESH_MS),
    );
    for proc_name in &cfg.extra_games {
        out.push_str(&format!("game_process={proc_name}\n"));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_config_roundtrip_and_corner_anchors() {
        let cfg = OverlayConfig {
            auto_games_only: false,
            anchor: OverlayAnchor::BottomRight,
            margin_px: 20,
            x: 150,
            y: 220,
            width: 360,
            height: 210,
            opacity_percent: 65,
            refresh_interval_ms: 1500,
            extra_games: vec!["cs2.exe".to_owned(), "dota2.exe".to_owned()],
        };
        let raw = serialize_overlay_config(&cfg);
        assert_eq!(parse_overlay_config(&raw), cfg);

        let mon = PixelRect {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        };
        let br = cfg.compute_rect(mon);
        assert_eq!(br.x, 1920 - 360 - 20);
        assert_eq!(br.y, 1080 - 210 - 20);

        let mut custom = cfg.clone();
        custom.anchor = OverlayAnchor::Custom;
        let cr = custom.compute_rect(mon);
        assert_eq!((cr.x, cr.y), (150, 220));
    }
}
