#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WeatherIconKind {
    Sun,
    #[default]
    PartlyCloudy,
    Cloudy,
    Fog,
    Rain,
    Snow,
    Storm,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HourlySlot {
    pub label: String,
    pub temp_c: i32,
    pub wmo_code: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DailySlot {
    pub date_iso: String,
    pub min_c: i32,
    pub max_c: i32,
    pub wmo_code: u16,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WeatherData {
    pub temp_c: i32,
    pub feels_like_c: i32,
    pub humidity_pct: u8,
    pub wind_ms: f32,
    pub pressure_mmhg: u16,
    pub wmo_code: u16,
    pub hourly: Vec<HourlySlot>,
    pub daily: Vec<DailySlot>,
    pub updated_label: String,
    pub has_live_data: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeoMatch {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

pub fn build_forecast_url(lat: f64, lon: f64) -> String {
    format!(
        "https://api.open-meteo.com/v1/forecast?latitude={lat:.4}&longitude={lon:.4}\
         &current=temperature_2m,apparent_temperature,relative_humidity_2m,weather_code,wind_speed_10m,surface_pressure\
         &hourly=temperature_2m,weather_code\
         &daily=weather_code,temperature_2m_max,temperature_2m_min\
         &wind_speed_unit=ms&timezone=auto&forecast_days=4"
    )
}

pub fn build_geocode_url(city: &str) -> String {
    let encoded = percent_encode(city.trim());
    format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={encoded}&count=1&language=ru&format=json"
    )
}

pub fn wmo_icon_kind(code: u16) -> WeatherIconKind {
    match code {
        0 | 1 => WeatherIconKind::Sun,
        2 => WeatherIconKind::PartlyCloudy,
        3 => WeatherIconKind::Cloudy,
        45 | 48 => WeatherIconKind::Fog,
        51..=67 | 80..=82 => WeatherIconKind::Rain,
        71..=77 | 85 | 86 => WeatherIconKind::Snow,
        95..=99 => WeatherIconKind::Storm,
        _ => WeatherIconKind::PartlyCloudy,
    }
}

pub fn wmo_description_ru(code: u16) -> &'static str {
    match code {
        0 => "Ясно",
        1 => "Преимущественно ясно",
        2 => "Облачно с прояснениями",
        3 => "Пасмурно",
        45 | 48 => "Туман",
        51..=57 => "Морось",
        61..=65 | 80..=82 => "Дождь",
        66 | 67 => "Ледяной дождь",
        71..=77 | 85 | 86 => "Снег",
        95..=99 => "Гроза",
        _ => "Переменная облачность",
    }
}

pub fn parse_open_meteo_geocode(body: &[u8]) -> Option<GeoMatch> {
    let text = std::str::from_utf8(body).ok()?;
    let idx = text.find("\"results\"")?;
    let sub = &text[idx..];
    let name = extract_json_string(sub, "name")?;
    let lat = extract_json_number(sub, "latitude")?;
    let lon = extract_json_number(sub, "longitude")?;
    if !lat.is_finite() || !lon.is_finite() {
        return None;
    }
    Some(GeoMatch { name, lat, lon })
}

pub fn parse_open_meteo_forecast(
    body: &[u8],
    current_hour: u8,
    updated_label: String,
) -> Result<WeatherData, String> {
    let text = std::str::from_utf8(body).map_err(|_| "invalid UTF-8 in weather JSON".to_owned())?;
    let cur_block = find_object_after_key(text, "\"current\"")
        .ok_or_else(|| "missing current block in Open-Meteo response".to_owned())?;

    let temp_c = extract_json_number(cur_block, "temperature_2m")
        .unwrap_or(0.0)
        .round() as i32;
    let feels_like_c = extract_json_number(cur_block, "apparent_temperature")
        .unwrap_or(temp_c as f64)
        .round() as i32;
    let humidity_pct = extract_json_number(cur_block, "relative_humidity_2m")
        .unwrap_or(0.0)
        .round()
        .clamp(0.0, 100.0) as u8;
    let wind_ms = extract_json_number(cur_block, "wind_speed_10m").unwrap_or(0.0) as f32;
    let pressure_hpa = extract_json_number(cur_block, "surface_pressure").unwrap_or(1013.0);
    let pressure_mmhg = (pressure_hpa * 0.750_062).round().clamp(500.0, 900.0) as u16;
    let wmo_code = extract_json_number(cur_block, "weather_code")
        .unwrap_or(0.0)
        .round()
        .clamp(0.0, 99.0) as u16;

    let mut hourly = Vec::new();
    if let Some(h_block) = find_object_after_key(text, "\"hourly\"") {
        let times = extract_string_array(h_block, "time");
        let temps = extract_number_array(h_block, "temperature_2m");
        let codes = extract_number_array(h_block, "weather_code");
        let start = usize::from(current_hour).min(times.len().saturating_sub(1));
        for step in 0..5 {
            let idx = start + step * 3;
            if let (Some(ts), Some(&t)) = (times.get(idx), temps.get(idx)) {
                let label = ts.split('T').nth(1).unwrap_or(ts).to_owned();
                let code = codes.get(idx).copied().unwrap_or(0.0).round() as u16;
                hourly.push(HourlySlot {
                    label,
                    temp_c: t.round() as i32,
                    wmo_code: code,
                });
            }
        }
    }

    let mut daily = Vec::new();
    if let Some(d_block) = find_object_after_key(text, "\"daily\"") {
        let dates = extract_string_array(d_block, "time");
        let mins = extract_number_array(d_block, "temperature_2m_min");
        let maxs = extract_number_array(d_block, "temperature_2m_max");
        let codes = extract_number_array(d_block, "weather_code");
        for (i, date) in dates.iter().enumerate().take(4) {
            daily.push(DailySlot {
                date_iso: date.clone(),
                min_c: mins.get(i).copied().unwrap_or(0.0).round() as i32,
                max_c: maxs.get(i).copied().unwrap_or(0.0).round() as i32,
                wmo_code: codes.get(i).copied().unwrap_or(0.0).round() as u16,
            });
        }
    }

    Ok(WeatherData {
        temp_c,
        feels_like_c,
        humidity_pct,
        wind_ms,
        pressure_mmhg,
        wmo_code,
        hourly,
        daily,
        updated_label,
        has_live_data: true,
    })
}

fn find_object_after_key<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let mut pos = 0;
    while let Some(rel) = text[pos..].find(key) {
        let after = pos + rel + key.len();
        let rest = text[after..].trim_start();
        if let Some(colon_rest) = rest.strip_prefix(':') {
            let trimmed = colon_rest.trim_start();
            if trimmed.starts_with('{') {
                return extract_balanced(trimmed, b'{', b'}');
            }
        }
        pos = after;
    }
    None
}

fn extract_balanced(s: &str, open: u8, close: u8) -> Option<&str> {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (i, &b) in bytes.iter().enumerate() {
        if in_str {
            if esc {
                esc = false;
            } else if b == b'\\' {
                esc = true;
            } else if b == b'"' {
                in_str = false;
            }
            continue;
        }
        if b == b'"' {
            in_str = true;
        } else if b == open {
            depth += 1;
        } else if b == close {
            depth -= 1;
            if depth == 0 {
                return Some(&s[..=i]);
            }
        }
    }
    None
}

fn extract_json_number(block: &str, field: &str) -> Option<f64> {
    let needle = format!("\"{field}\"");
    let pos = block.find(&needle)?;
    let rest = block[pos + needle.len()..]
        .trim_start()
        .strip_prefix(':')?
        .trim_start();
    let end = rest
        .find(|c: char| !matches!(c, '0'..='9' | '-' | '+' | '.' | 'e' | 'E'))
        .unwrap_or(rest.len());
    rest[..end].parse::<f64>().ok()
}

fn extract_json_string(block: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\"");
    let pos = block.find(&needle)?;
    let rest = block[pos + needle.len()..]
        .trim_start()
        .strip_prefix(':')?
        .trim_start();
    let inner = rest.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '"' {
            return Some(out);
        }
        if c == '\\' {
            if let Some(esc) = chars.next() {
                out.push(esc);
            }
        } else {
            out.push(c);
        }
    }
    None
}

fn extract_string_array(block: &str, field: &str) -> Vec<String> {
    let needle = format!("\"{field}\"");
    let Some(pos) = block.find(&needle) else {
        return Vec::new();
    };
    let rest = block[pos + needle.len()..].trim_start();
    let Some(after_colon) = rest.strip_prefix(':') else {
        return Vec::new();
    };
    let Some(arr) = extract_balanced(after_colon.trim_start(), b'[', b']') else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut in_str = false;
    let mut cur = String::new();
    for ch in arr.chars() {
        if in_str {
            if ch == '"' {
                out.push(std::mem::take(&mut cur));
                in_str = false;
            } else {
                cur.push(ch);
            }
        } else if ch == '"' {
            in_str = true;
        }
    }
    out
}

fn extract_number_array(block: &str, field: &str) -> Vec<f64> {
    let needle = format!("\"{field}\"");
    let Some(pos) = block.find(&needle) else {
        return Vec::new();
    };
    let rest = block[pos + needle.len()..].trim_start();
    let Some(after_colon) = rest.strip_prefix(':') else {
        return Vec::new();
    };
    let Some(arr) = extract_balanced(after_colon.trim_start(), b'[', b']') else {
        return Vec::new();
    };
    arr.trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .filter_map(|p| p.trim().parse::<f64>().ok())
        .collect()
}

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for b in input.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(*b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_open_meteo_forecast_and_geocode() {
        let sample = br#"{
            "current":{"temperature_2m":4.2,"apparent_temperature":1.1,"relative_humidity_2m":76,"weather_code":2,"wind_speed_10m":3.4,"surface_pressure":1002.6},
            "hourly":{"time":["2026-11-25T00:00","2026-11-25T03:00","2026-11-25T06:00","2026-11-25T09:00","2026-11-25T12:00"],"temperature_2m":[4,4,3,2,1],"weather_code":[2,3,3,61,61]},
            "daily":{"time":["2026-11-25","2026-11-26","2026-11-27","2026-11-28"],"weather_code":[2,3,2,61],"temperature_2m_max":[4,3,2,2],"temperature_2m_min":[0,-1,-2,-1]}
        }"#;
        let parsed = parse_open_meteo_forecast(sample, 0, "02:10".to_owned());
        assert!(parsed.is_ok());
        if let Ok(d) = parsed {
            assert_eq!(d.temp_c, 4);
            assert_eq!(d.feels_like_c, 1);
            assert_eq!(d.humidity_pct, 76);
            assert_eq!(d.pressure_mmhg, 752);
            assert_eq!(d.daily.len(), 4);
            assert_eq!(wmo_description_ru(d.wmo_code), "Облачно с прояснениями");
        }
        let geo =
            r#"{"results":[{"name":"Москва","latitude":55.7522,"longitude":37.6156}]}"#.as_bytes();
        let g = parse_open_meteo_geocode(geo);
        assert!(g.is_some());
    }
}
