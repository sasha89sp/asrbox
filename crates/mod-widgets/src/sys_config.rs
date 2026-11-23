use std::fs;
use std::path::Path;

use asrbox_platform_api::WidgetLayer;

use crate::storage::{DEFAULT_OPACITY_PERCENT, MAX_OPACITY_PERCENT, MIN_OPACITY_PERCENT};

pub const MIN_SYSMON_WIDTH: u32 = 440;
pub const MAX_SYSMON_WIDTH: u32 = 900;
pub const MIN_SYSMON_HEIGHT: u32 = 380;
pub const MAX_SYSMON_HEIGHT: u32 = 780;
pub const MIN_REFRESH_MS: u32 = 500;
pub const MAX_REFRESH_MS: u32 = 60_000;
pub const DEFAULT_REFRESH_MS: u32 = 2000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SysMonitorConfig {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub layer: WidgetLayer,
    pub opacity_percent: u8,
    pub refresh_interval_ms: u32,
}

impl Default for SysMonitorConfig {
    fn default() -> Self {
        Self {
            visible: true,
            x: 396,
            y: 40,
            width: 560,
            height: 492,
            layer: WidgetLayer::Wallpaper,
            opacity_percent: DEFAULT_OPACITY_PERCENT,
            refresh_interval_ms: DEFAULT_REFRESH_MS,
        }
    }
}

pub fn load_sysmon_config(path: &Path) -> Result<SysMonitorConfig, String> {
    if !path.exists() {
        let cfg = SysMonitorConfig::default();
        let _ = save_sysmon_config(path, &cfg);
        return Ok(cfg);
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("failed to read sysmon.cfg: {e}"))?;
    Ok(parse_sysmon_config(&raw))
}

pub fn save_sysmon_config(path: &Path, cfg: &SysMonitorConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create config dir: {e}"))?;
    }
    let content = serialize_sysmon_config(cfg);
    let tmp = path.with_extension("cfg.tmp");
    fs::write(&tmp, content).map_err(|e| format!("failed to write tmp sysmon.cfg: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("failed to commit sysmon.cfg: {e}"))?;
    Ok(())
}

pub fn parse_sysmon_config(raw: &str) -> SysMonitorConfig {
    let mut cfg = SysMonitorConfig::default();
    for line in raw.trim_start_matches('\u{FEFF}').lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        let val = v.trim();
        match k.trim() {
            "visible" => cfg.visible = val == "1" || val.eq_ignore_ascii_case("true"),
            "x" => parse_into(val, &mut cfg.x),
            "y" => parse_into(val, &mut cfg.y),
            "width" => parse_into(val, &mut cfg.width),
            "height" => parse_into(val, &mut cfg.height),
            "layer" => cfg.layer = parse_layer(val),
            "opacity" => parse_into(val.trim_end_matches('%').trim(), &mut cfg.opacity_percent),
            "refresh_interval_ms" => parse_into(val, &mut cfg.refresh_interval_ms),
            _ => {}
        }
    }
    cfg.width = cfg.width.clamp(MIN_SYSMON_WIDTH, MAX_SYSMON_WIDTH);
    cfg.height = cfg.height.clamp(MIN_SYSMON_HEIGHT, MAX_SYSMON_HEIGHT);
    cfg.opacity_percent = cfg
        .opacity_percent
        .clamp(MIN_OPACITY_PERCENT, MAX_OPACITY_PERCENT);
    cfg.refresh_interval_ms = cfg
        .refresh_interval_ms
        .clamp(MIN_REFRESH_MS, MAX_REFRESH_MS);
    cfg
}

pub fn serialize_sysmon_config(cfg: &SysMonitorConfig) -> String {
    let mut out = String::from(concat!(
        "# Настройки виджета Системный монитор (asrbox)\n",
        "# Размер окна можно менять напрямую за края/углы виджета.\n",
        "# refresh_interval_ms задаёт частоту опроса в миллисекундах (1000–10000).\n\n",
    ));
    out.push_str(&format!("visible={}\n", if cfg.visible { 1 } else { 0 }));
    out.push_str(&format!("x={}\n", cfg.x));
    out.push_str(&format!("y={}\n", cfg.y));
    out.push_str(&format!("width={}\n", cfg.width));
    out.push_str(&format!("height={}\n", cfg.height));
    out.push_str(&format!("layer={}\n", layer_to_str(cfg.layer)));
    out.push_str(&format!("opacity={}\n", cfg.opacity_percent));
    out.push_str(&format!(
        "refresh_interval_ms={}\n",
        cfg.refresh_interval_ms
    ));
    out
}

fn parse_into<T: std::str::FromStr>(val: &str, dst: &mut T) {
    if let Ok(v) = val.parse() {
        *dst = v;
    }
}

fn parse_layer(s: &str) -> WidgetLayer {
    match s.trim() {
        "normal" => WidgetLayer::Normal,
        "topmost" => WidgetLayer::Topmost,
        _ => WidgetLayer::Wallpaper,
    }
}

fn layer_to_str(layer: WidgetLayer) -> &'static str {
    match layer {
        WidgetLayer::Wallpaper => "wallpaper",
        WidgetLayer::Normal => "normal",
        WidgetLayer::Topmost => "topmost",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysmon_config_roundtrip() {
        let cfg = SysMonitorConfig {
            visible: true,
            x: 620,
            y: 48,
            width: 580,
            height: 500,
            layer: WidgetLayer::Normal,
            opacity_percent: 82,
            refresh_interval_ms: 3000,
        };
        let raw = serialize_sysmon_config(&cfg);
        assert_eq!(parse_sysmon_config(&raw), cfg);
    }
}
