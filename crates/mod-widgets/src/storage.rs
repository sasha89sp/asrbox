use std::fs;
use std::path::{Path, PathBuf};

use asrbox_platform_api::WidgetLayer;

pub const DEFAULT_OPACITY_PERCENT: u8 = 88;
pub const MIN_OPACITY_PERCENT: u8 = 20;
pub const MAX_OPACITY_PERCENT: u8 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PeriodFilter {
    Today,
    Days7,
    #[default]
    Days30,
    All,
}

impl PeriodFilter {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Today => "today",
            Self::Days7 => "7d",
            Self::Days30 => "30d",
            Self::All => "all",
        }
    }

    pub fn from_str_value(s: &str) -> Self {
        match s.trim() {
            "today" => Self::Today,
            "7d" => Self::Days7,
            "all" => Self::All,
            _ => Self::Days30,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DateFormatMode {
    #[default]
    Normal,
    Numeric,
    Countdown,
}

impl DateFormatMode {
    pub fn next(self) -> Self {
        match self {
            Self::Normal => Self::Numeric,
            Self::Numeric => Self::Countdown,
            Self::Countdown => Self::Normal,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Вид: дата",
            Self::Numeric => "Вид: 23.11",
            Self::Countdown => "Вид: отсчёт",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "date",
            Self::Numeric => "numeric",
            Self::Countdown => "countdown",
        }
    }

    pub fn from_str_value(s: &str) -> Self {
        match s.trim() {
            "numeric" => Self::Numeric,
            "countdown" => Self::Countdown,
            _ => Self::Normal,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarSource {
    pub id: String,
    pub url: String,
    pub color_rgb: Option<[u8; 3]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarWidgetConfig {
    pub visible: bool,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub visible_rows: u8,
    pub layer: WidgetLayer,
    pub opacity_percent: u8,
    pub title: String,
    pub refresh_interval_sec: u32,
    pub lookahead_days: u16,
    pub period: PeriodFilter,
    pub date_format: DateFormatMode,
    pub sources: Vec<CalendarSource>,
}

impl Default for CalendarWidgetConfig {
    fn default() -> Self {
        Self {
            visible: true,
            x: 40,
            y: 40,
            width: 336,
            visible_rows: 8,
            layer: WidgetLayer::Wallpaper,
            opacity_percent: DEFAULT_OPACITY_PERCENT,
            title: "Google Календарь".to_owned(),
            refresh_interval_sec: 3600,
            lookahead_days: 365,
            period: PeriodFilter::Days30,
            date_format: DateFormatMode::Normal,
            sources: Vec::new(),
        }
    }
}

pub fn load_widget_config(path: &Path) -> Result<CalendarWidgetConfig, String> {
    if !path.exists() {
        let cfg = CalendarWidgetConfig::default();
        let _ = save_widget_config(path, &cfg);
        return Ok(cfg);
    }
    let raw = fs::read_to_string(path).map_err(|e| format!("failed to read widgets.cfg: {e}"))?;
    Ok(parse_widget_config(&raw))
}

pub fn save_widget_config(path: &Path, cfg: &CalendarWidgetConfig) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("failed to create config dir: {e}"))?;
    }
    let content = serialize_widget_config(cfg);
    let tmp = path.with_extension("cfg.tmp");
    fs::write(&tmp, content).map_err(|e| format!("failed to write tmp widgets.cfg: {e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("failed to commit widgets.cfg: {e}"))?;
    Ok(())
}

pub fn parse_widget_config(raw: &str) -> CalendarWidgetConfig {
    let mut cfg = CalendarWidgetConfig::default();
    let cleaned = raw.trim_start_matches('\u{FEFF}');
    for line in cleaned.lines() {
        let trimmed = line.trim().trim_start_matches('\u{FEFF}');
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        let Some((k, v)) = trimmed.split_once('=') else {
            continue;
        };
        apply_kv(&mut cfg, k.trim(), v.trim());
    }
    cfg.visible_rows = cfg.visible_rows.clamp(3, 12);
    cfg.width = cfg.width.clamp(280, 560);
    cfg.opacity_percent = cfg
        .opacity_percent
        .clamp(MIN_OPACITY_PERCENT, MAX_OPACITY_PERCENT);
    cfg.refresh_interval_sec = cfg.refresh_interval_sec.clamp(60, 86_400);
    cfg.lookahead_days = cfg.lookahead_days.clamp(7, 730);
    cfg
}

fn apply_kv(cfg: &mut CalendarWidgetConfig, key: &str, val: &str) {
    match key {
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
                cfg.width = n;
            }
        }
        "visible_rows" => {
            if let Ok(n) = val.parse::<u8>() {
                cfg.visible_rows = n;
            }
        }
        "layer" => cfg.layer = parse_layer(val),
        "opacity" => {
            if let Ok(n) = val.trim_end_matches('%').trim().parse::<u8>() {
                cfg.opacity_percent = n;
            }
        }
        "title" if !val.is_empty() => cfg.title = strip_quotes(val).to_owned(),
        "refresh_interval_sec" => {
            if let Ok(n) = val.parse::<u32>() {
                cfg.refresh_interval_sec = n;
            }
        }
        "lookahead_days" => {
            if let Ok(n) = val.parse::<u16>() {
                cfg.lookahead_days = n;
            }
        }
        "period" => cfg.period = PeriodFilter::from_str_value(val),
        "date_format" => cfg.date_format = DateFormatMode::from_str_value(val),
        "calendar" | "url" => {
            let idx = cfg.sources.len().saturating_add(1);
            if let Some(src) = parse_calendar_source(val, idx) {
                cfg.sources.push(src);
            }
        }
        _ => {}
    }
}

pub fn serialize_widget_config(cfg: &CalendarWidgetConfig) -> String {
    let mut out = String::from(concat!(
        "# Настройки виджета Google Календарь (asrbox)\n",
        "# Сохраните файл и нажмите кнопку обновления (↻) на виджете.\n",
        "# Прозрачность фона: opacity=20..100 (по умолчанию 88)\n",
        "# Подключение календаря (подходит любой формат):\n",
        "#   calendar=https://calendar.google.com/.../basic.ics\n",
        "#   calendar=main|auto|https://calendar.google.com/.../basic.ics\n",
        "#   calendar=work|#A855F7|https://calendar.google.com/.../basic.ics\n\n",
    ));
    out.push_str(&format!("visible={}\n", if cfg.visible { 1 } else { 0 }));
    out.push_str(&format!("x={}\n", cfg.x));
    out.push_str(&format!("y={}\n", cfg.y));
    out.push_str(&format!("width={}\n", cfg.width));
    out.push_str(&format!("visible_rows={}\n", cfg.visible_rows));
    out.push_str(&format!("layer={}\n", layer_to_str(cfg.layer)));
    out.push_str(&format!("opacity={}\n", cfg.opacity_percent));
    out.push_str(&format!("title={}\n", cfg.title));
    out.push_str(&format!(
        "refresh_interval_sec={}\n",
        cfg.refresh_interval_sec
    ));
    out.push_str(&format!("lookahead_days={}\n", cfg.lookahead_days));
    out.push_str(&format!("period={}\n", cfg.period.as_str()));
    out.push_str(&format!("date_format={}\n", cfg.date_format.as_str()));
    for src in &cfg.sources {
        let color = src
            .color_rgb
            .map_or_else(|| "auto".to_owned(), format_hex_rgb);
        out.push_str(&format!("calendar={}|{}|{}\n", src.id, color, src.url));
    }
    out
}

pub fn cache_file_path(cache_dir: &Path, source_id: &str) -> PathBuf {
    let safe: String = source_id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let suffix = if safe.is_empty() { "default" } else { &safe };
    cache_dir.join(format!("calendar_cache_{suffix}.ics"))
}

pub fn load_cached_ics(cache_dir: &Path, source_id: &str) -> Option<Vec<u8>> {
    fs::read(cache_file_path(cache_dir, source_id)).ok()
}

pub fn save_cached_ics(cache_dir: &Path, source_id: &str, bytes: &[u8]) -> Result<(), String> {
    fs::create_dir_all(cache_dir).map_err(|e| format!("failed to create cache dir: {e}"))?;
    let target = cache_file_path(cache_dir, source_id);
    let tmp = target.with_extension("ics.tmp");
    fs::write(&tmp, bytes).map_err(|e| format!("failed to write cache tmp: {e}"))?;
    fs::rename(&tmp, &target).map_err(|e| format!("failed to commit cache: {e}"))?;
    Ok(())
}

fn parse_calendar_source(val: &str, fallback_idx: usize) -> Option<CalendarSource> {
    let mut parts = val.splitn(3, '|');
    let first = strip_quotes(parts.next()?.trim());
    let second = parts.next().map(|s| strip_quotes(s.trim()));
    let third = parts.next().map(|s| strip_quotes(s.trim()));
    match (second, third) {
        (Some(color), Some(url)) if is_http_url(url) => Some(CalendarSource {
            id: normalize_id(first, fallback_idx),
            url: url.to_owned(),
            color_rgb: parse_hex_rgb(color),
        }),
        (Some(url), None) if is_http_url(url) => Some(CalendarSource {
            id: normalize_id(first, fallback_idx),
            url: url.to_owned(),
            color_rgb: None,
        }),
        (None, None) if is_http_url(first) => Some(CalendarSource {
            id: format!("cal_{fallback_idx}"),
            url: first.to_owned(),
            color_rgb: None,
        }),
        _ => None,
    }
}

fn normalize_id(raw: &str, fallback_idx: usize) -> String {
    if raw.is_empty() {
        format!("cal_{fallback_idx}")
    } else {
        raw.to_owned()
    }
}

fn is_http_url(s: &str) -> bool {
    s.starts_with("https://") || s.starts_with("http://")
}

fn strip_quotes(s: &str) -> &str {
    let trimmed = s.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        let q = bytes[0];
        if (q == b'"' || q == b'\'') && bytes[bytes.len() - 1] == q {
            return &trimmed[1..trimmed.len() - 1];
        }
    }
    trimmed
}

fn parse_hex_rgb(s: &str) -> Option<[u8; 3]> {
    let hex = s.strip_prefix('#').unwrap_or(s);
    if hex.len() != 6 || hex.eq_ignore_ascii_case("auto") {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

fn format_hex_rgb(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
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
    fn widget_config_roundtrip() {
        let cfg = CalendarWidgetConfig {
            x: 120,
            y: 85,
            visible_rows: 6,
            layer: WidgetLayer::Topmost,
            opacity_percent: 70,
            period: PeriodFilter::Days7,
            date_format: DateFormatMode::Countdown,
            sources: vec![CalendarSource {
                id: "work".to_owned(),
                url: "https://example.com/cal.ics".to_owned(),
                color_rgb: Some([0xA8, 0x55, 0xF7]),
            }],
            ..Default::default()
        };
        let text = serialize_widget_config(&cfg);
        let parsed = parse_widget_config(&text);
        assert_eq!(parsed, cfg);
    }

    #[test]
    fn parses_utf8_bom_quotes_and_short_calendar_formats() {
        let raw = "\u{FEFF}opacity=50%\n\
            acrylic=1\n\
            calendar=\"https://example.com/a.ics\"\n\
            calendar=personal|https://example.com/b.ics\n\
            url=https://example.com/c.ics\n";
        let cfg = parse_widget_config(raw);
        assert_eq!(cfg.opacity_percent, 50);
        assert_eq!(cfg.sources.len(), 3);
        assert_eq!(cfg.sources[0].url, "https://example.com/a.ics");
        assert_eq!(cfg.sources[1].id, "personal");
        assert_eq!(cfg.sources[1].url, "https://example.com/b.ics");
        assert_eq!(cfg.sources[2].url, "https://example.com/c.ics");
    }
}
