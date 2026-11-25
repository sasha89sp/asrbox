use std::fs;
use std::path::Path;

use asrbox_platform_api::WidgetLayer;

use crate::storage::{MAX_OPACITY_PERCENT, MIN_OPACITY_PERCENT};

pub const MIN_WEATHER_WIDTH: u32 = 280;
pub const MAX_WEATHER_WIDTH: u32 = 560;
pub const MIN_WEATHER_HEIGHT: u32 = 310;
pub const MAX_WEATHER_HEIGHT: u32 = 640;
pub const DEFAULT_WEATHER_WIDTH: u32 = 344;
pub const DEFAULT_WEATHER_HEIGHT: u32 = 384;
pub const YANDEX_WEATHER_CRED_TARGET: &str = "asrbox:weather:yandex_key";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WeatherProvider {
    #[default]
    OpenMeteo,
    Yandex,
}

impl WeatherProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenMeteo => "open_meteo",
            Self::Yandex => "yandex",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::OpenMeteo => "Open-Meteo",
            Self::Yandex => "Яндекс.Погода",
        }
    }

    pub fn from_str_lossy(val: &str) -> Self {
        if val.eq_ignore_ascii_case("yandex") {
            Self::Yandex
        } else {
            Self::OpenMeteo
        }
    }
}

pub fn normalize_forecast_days(days: u8) -> u8 {
    match days {
        0..=1 => 1,
        2..=4 => 3,
        _ => 7,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeatherWidgetConfig {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub layer: WidgetLayer,
    pub opacity_percent: u8,
    pub provider: WeatherProvider,
    pub forecast_days: u8,
    pub city: String,
    pub lat: f64,
    pub lon: f64,
    pub refresh_interval_min: u32,
}

impl Default for WeatherWidgetConfig {
    fn default() -> Self {
        Self {
            visible: false,
            x: 84,
            y: 84,
            width: DEFAULT_WEATHER_WIDTH,
            height: DEFAULT_WEATHER_HEIGHT,
            layer: WidgetLayer::Wallpaper,
            opacity_percent: 88,
            provider: WeatherProvider::OpenMeteo,
            forecast_days: 3,
            city: "Москва".to_owned(),
            lat: 55.7558,
            lon: 37.6173,
            refresh_interval_min: 30,
        }
    }
}

pub fn load_weather_config(path: &Path) -> Result<WeatherWidgetConfig, String> {
    if !path.exists() {
        return Ok(WeatherWidgetConfig::default());
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("read weather.cfg: {e}"))?;
    Ok(parse_weather_config(&raw))
}

pub fn save_weather_config(path: &Path, cfg: &WeatherWidgetConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create_dir_all weather.cfg: {e}"))?;
    }
    let tmp = path.with_extension("cfg.tmp");
    fs::write(&tmp, serialize_weather_config(cfg))
        .map_err(|e| format!("write weather.cfg: {e}"))?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).map_err(|e| format!("rename weather.cfg: {e}"))?;
    Ok(())
}

pub fn parse_weather_config(raw: &str) -> WeatherWidgetConfig {
    let mut cfg = WeatherWidgetConfig::default();
    for line in raw.trim_start_matches('\u{FEFF}').lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        let val = v.trim();
        match k.trim() {
            "visible" => cfg.visible = val == "1" || val.eq_ignore_ascii_case("true"),
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
                    cfg.width = n.clamp(MIN_WEATHER_WIDTH, MAX_WEATHER_WIDTH);
                }
            }
            "height" => {
                if let Ok(n) = val.parse::<u32>() {
                    cfg.height = n.clamp(MIN_WEATHER_HEIGHT, MAX_WEATHER_HEIGHT);
                }
            }
            "layer" => cfg.layer = parse_layer(val),
            "opacity" => {
                if let Ok(n) = val.parse::<u8>() {
                    cfg.opacity_percent = n.clamp(MIN_OPACITY_PERCENT, MAX_OPACITY_PERCENT);
                }
            }
            "provider" => cfg.provider = WeatherProvider::from_str_lossy(val),
            "forecast_days" => {
                if let Ok(n) = val.parse::<u8>() {
                    cfg.forecast_days = normalize_forecast_days(n);
                }
            }
            "city" => {
                if !val.is_empty() {
                    cfg.city = val.to_owned();
                }
            }
            "lat" => {
                if let Ok(n) = val.parse::<f64>() {
                    if n.is_finite() && (-90.0..=90.0).contains(&n) {
                        cfg.lat = n;
                    }
                }
            }
            "lon" => {
                if let Ok(n) = val.parse::<f64>() {
                    if n.is_finite() && (-180.0..=180.0).contains(&n) {
                        cfg.lon = n;
                    }
                }
            }
            "refresh_interval_min" => {
                if let Ok(n) = val.parse::<u32>() {
                    cfg.refresh_interval_min = n.clamp(5, 360);
                }
            }
            _ => {}
        }
    }
    cfg
}

pub fn serialize_weather_config(cfg: &WeatherWidgetConfig) -> String {
    let layer = match cfg.layer {
        WidgetLayer::Wallpaper => "wallpaper",
        WidgetLayer::Normal => "normal",
        WidgetLayer::Topmost => "topmost",
    };
    format!(
        "# Настройки виджета Погоды (Open-Meteo / Яндекс.Погода, asrbox)\n\
         settings_version=2\n\
         visible={}\n\
         x={}\n\
         y={}\n\
         width={}\n\
         height={}\n\
         layer={layer}\n\
         opacity={}\n\
         provider={}\n\
         forecast_days={}\n\
         city={}\n\
         lat={:.4}\n\
         lon={:.4}\n\
         refresh_interval_min={}\n",
        if cfg.visible { 1 } else { 0 },
        cfg.x,
        cfg.y,
        cfg.width,
        cfg.height,
        cfg.opacity_percent,
        cfg.provider.as_str(),
        normalize_forecast_days(cfg.forecast_days),
        cfg.city.trim(),
        cfg.lat,
        cfg.lon,
        cfg.refresh_interval_min,
    )
}

fn parse_layer(val: &str) -> WidgetLayer {
    match val {
        "normal" => WidgetLayer::Normal,
        "topmost" => WidgetLayer::Topmost,
        _ => WidgetLayer::Wallpaper,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_config_roundtrip() {
        let cfg = WeatherWidgetConfig {
            visible: true,
            x: 120,
            y: 220,
            width: 360,
            height: 460,
            layer: WidgetLayer::Normal,
            opacity_percent: 82,
            provider: WeatherProvider::Yandex,
            forecast_days: 7,
            city: "Санкт-Петербург".to_owned(),
            lat: 59.9311,
            lon: 30.3609,
            refresh_interval_min: 15,
        };
        let parsed = parse_weather_config(&serialize_weather_config(&cfg));
        assert_eq!(parsed.visible, cfg.visible);
        assert_eq!(parsed.provider, WeatherProvider::Yandex);
        assert_eq!(parsed.forecast_days, 7);
        assert_eq!(parsed.city, cfg.city);
        assert!((parsed.lat - cfg.lat).abs() < 0.001);
        assert_eq!(parsed.refresh_interval_min, 15);
    }
}
