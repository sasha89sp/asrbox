use crate::weather_api::{
    extract_balanced, extract_json_number, extract_json_string, find_object_after_key, DailySlot,
    HourlySlot, WeatherData,
};

pub fn build_yandex_forecast_url(lat: f64, lon: f64) -> String {
    format!(
        "https://api.weather.yandex.ru/v2/forecast?lat={lat:.4}&lon={lon:.4}&lang=ru_RU&limit=7&hours=true"
    )
}

pub fn yandex_condition_to_wmo(cond: &str) -> u16 {
    match cond.trim() {
        "clear" => 0,
        "partly-cloudy" => 2,
        "cloudy" | "overcast" => 3,
        "light-rain" | "drizzle" => 53,
        "rain" | "moderate-rain" | "heavy-rain" | "continuous-heavy-rain" | "showers" => 63,
        "wet-snow" | "hail" => 66,
        "light-snow" | "snow" | "snow-showers" => 73,
        "thunderstorm" | "thunderstorm-with-rain" | "thunderstorm-with-hail" => 95,
        _ => 2,
    }
}

pub fn parse_yandex_forecast(
    body: &[u8],
    current_hour: u8,
    updated_label: String,
) -> Result<WeatherData, String> {
    let text = std::str::from_utf8(body).map_err(|_| "invalid UTF-8 in Yandex JSON".to_owned())?;
    let fact = find_object_after_key(text, "\"fact\"")
        .ok_or_else(|| "missing fact block in Yandex response".to_owned())?;

    let temp_c = extract_json_number(fact, "temp").unwrap_or(0.0).round() as i32;
    let feels_like_c = extract_json_number(fact, "feels_like")
        .unwrap_or(temp_c as f64)
        .round() as i32;
    let humidity_pct = extract_json_number(fact, "humidity")
        .unwrap_or(0.0)
        .round()
        .clamp(0.0, 100.0) as u8;
    let wind_ms = extract_json_number(fact, "wind_speed").unwrap_or(0.0) as f32;
    let pressure_mmhg = extract_json_number(fact, "pressure_mm")
        .unwrap_or(755.0)
        .round()
        .clamp(500.0, 900.0) as u16;
    let wmo_code = extract_json_string(fact, "condition")
        .map(|s| yandex_condition_to_wmo(&s))
        .unwrap_or(2);

    let (hourly, daily) = parse_yandex_forecasts_array(text, current_hour);

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

fn parse_yandex_forecasts_array(text: &str, current_hour: u8) -> (Vec<HourlySlot>, Vec<DailySlot>) {
    let Some(pos) = text.find("\"forecasts\"") else {
        return (Vec::new(), Vec::new());
    };
    let rest = text[pos + "\"forecasts\"".len()..].trim_start();
    let Some(after_colon) = rest.strip_prefix(':') else {
        return (Vec::new(), Vec::new());
    };
    let Some(arr) = extract_balanced(after_colon.trim_start(), b'[', b']') else {
        return (Vec::new(), Vec::new());
    };

    let day_objs = split_top_level_objects(arr);
    let mut daily = Vec::new();
    let mut all_hours: Vec<(u8, HourlySlot)> = Vec::new();

    for (idx, obj) in day_objs.iter().take(7).enumerate() {
        let date_iso = extract_json_string(obj, "date").unwrap_or_default();
        let parts = find_object_after_key(obj, "\"parts\"");
        let day_part = parts.and_then(|p| {
            find_object_after_key(p, "\"day_short\"")
                .or_else(|| find_object_after_key(p, "\"day\""))
        });
        let night_part = parts.and_then(|p| {
            find_object_after_key(p, "\"night_short\"")
                .or_else(|| find_object_after_key(p, "\"night\""))
        });

        let max_c = day_part
            .and_then(|p| {
                extract_json_number(p, "temp").or_else(|| extract_json_number(p, "temp_max"))
            })
            .unwrap_or(0.0)
            .round() as i32;
        let min_c = day_part
            .and_then(|p| extract_json_number(p, "temp_min"))
            .or_else(|| night_part.and_then(|p| extract_json_number(p, "temp_min")))
            .or_else(|| night_part.and_then(|p| extract_json_number(p, "temp")))
            .unwrap_or(max_c as f64)
            .round() as i32;
        let cond = day_part
            .and_then(|p| extract_json_string(p, "condition"))
            .unwrap_or_default();
        daily.push(DailySlot {
            date_iso,
            min_c: min_c.min(max_c),
            max_c: max_c.max(min_c),
            wmo_code: yandex_condition_to_wmo(&cond),
        });

        if idx < 2 {
            collect_yandex_hours(obj, idx == 0, current_hour, &mut all_hours);
        }
    }

    let hourly = all_hours
        .into_iter()
        .step_by(3)
        .take(5)
        .map(|(_, slot)| slot)
        .collect();
    (hourly, daily)
}

fn collect_yandex_hours(
    day_obj: &str,
    is_today: bool,
    current_hour: u8,
    out: &mut Vec<(u8, HourlySlot)>,
) {
    let Some(pos) = day_obj.find("\"hours\"") else {
        return;
    };
    let rest = day_obj[pos + "\"hours\"".len()..].trim_start();
    let Some(after_colon) = rest.strip_prefix(':') else {
        return;
    };
    let Some(arr) = extract_balanced(after_colon.trim_start(), b'[', b']') else {
        return;
    };
    for h_obj in split_top_level_objects(arr) {
        let Some(hour_str) = extract_json_string(h_obj, "hour") else {
            continue;
        };
        let Ok(hour_num) = hour_str.parse::<u8>() else {
            continue;
        };
        if is_today && hour_num < current_hour {
            continue;
        }
        let temp_c = extract_json_number(h_obj, "temp").unwrap_or(0.0).round() as i32;
        let cond = extract_json_string(h_obj, "condition").unwrap_or_default();
        out.push((
            hour_num,
            HourlySlot {
                label: format!("{hour_num:02}:00"),
                temp_c,
                wmo_code: yandex_condition_to_wmo(&cond),
            },
        ));
    }
}

fn split_top_level_objects(arr: &str) -> Vec<&str> {
    let bytes = arr.as_bytes();
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    let mut obj_start: Option<usize> = None;

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
        match b {
            b'"' => in_str = true,
            b'{' => {
                if depth == 0 {
                    obj_start = Some(i);
                }
                depth += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some(st) = obj_start.take() {
                        out.push(&arr[st..=i]);
                    }
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_yandex_weather_v2_forecast() {
        let sample = br#"{
            "fact":{"temp":3,"feels_like":-1,"humidity":80,"wind_speed":4.2,"pressure_mm":749,"condition":"partly-cloudy"},
            "forecasts":[
                {"date":"2026-11-25","parts":{"day_short":{"temp":4,"temp_min":0,"condition":"rain"}},"hours":[
                    {"hour":"12","temp":4,"condition":"cloudy"},
                    {"hour":"13","temp":4,"condition":"cloudy"},
                    {"hour":"14","temp":3,"condition":"rain"},
                    {"hour":"15","temp":2,"condition":"rain"}
                ]},
                {"date":"2026-11-26","parts":{"day_short":{"temp":2,"temp_min":-2,"condition":"snow"}},"hours":[]}
            ]
        }"#;
        let parsed = parse_yandex_forecast(sample, 12, "12:05".to_owned());
        assert!(parsed.is_ok());
        if let Ok(d) = parsed {
            assert_eq!(d.temp_c, 3);
            assert_eq!(d.feels_like_c, -1);
            assert_eq!(d.humidity_pct, 80);
            assert_eq!(d.pressure_mmhg, 749);
            assert_eq!(d.daily.len(), 2);
            assert_eq!(d.daily[0].min_c, 0);
            assert_eq!(d.daily[0].max_c, 4);
            assert_eq!(d.hourly.len(), 2);
            assert_eq!(d.hourly[0].label, "12:00");
        }
    }
}
