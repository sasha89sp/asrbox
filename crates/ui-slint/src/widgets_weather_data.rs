use std::rc::Rc;

use asrbox_mod_widgets::{
    build_forecast_url, build_geocode_url, build_yandex_forecast_url, normalize_forecast_days,
    parse_open_meteo_forecast, parse_open_meteo_geocode, parse_yandex_forecast, wmo_description_ru,
    wmo_icon_kind, DailySlot, WeatherData, WeatherIconKind, WeatherProvider,
    YANDEX_WEATHER_CRED_TARGET,
};
use slint::{ModelRc, SharedString, VecModel};

use crate::generated::ui::{SlintWeatherDayRow, SlintWeatherHourRow};
use crate::widgets_weather::WeatherSlintState;
use crate::win_os::{cred_load, http_get, local_time_now};

pub(crate) struct WeatherFetchPayload {
    pub(crate) city_override: Option<(String, f64, f64)>,
    pub(crate) data: Option<WeatherData>,
    pub(crate) is_hybrid: bool,
    pub(crate) error: Option<String>,
}

pub(crate) fn fetch_weather_blocking(
    city: String,
    mut lat: f64,
    mut lon: f64,
    provider: WeatherProvider,
    geocode_first: bool,
) -> WeatherFetchPayload {
    let mut city_override = None;
    if geocode_first && !city.trim().is_empty() {
        let geo = http_get(&build_geocode_url(&city), &[]);
        if geo.error.is_none() && geo.status_code == 200 {
            if let Some(m) = parse_open_meteo_geocode(&geo.body) {
                lat = m.lat;
                lon = m.lon;
                city_override = Some((m.name, lat, lon));
            }
        }
    }
    let now = local_time_now();
    let updated = format!("{:02}:{:02}", now.hour, now.minute);
    match provider {
        WeatherProvider::OpenMeteo => {
            let res = http_get(&build_forecast_url(lat, lon), &[]);
            if let Some(err) = res.error {
                return err_payload(city_override, format!("Нет сети: {err}"));
            }
            if res.status_code != 200 {
                return err_payload(city_override, format!("HTTP {}", res.status_code));
            }
            match parse_open_meteo_forecast(&res.body, now.hour, updated) {
                Ok(data) => WeatherFetchPayload {
                    city_override,
                    data: Some(data),
                    is_hybrid: false,
                    error: None,
                },
                Err(e) => err_payload(city_override, format!("Ошибка Open-Meteo: {e}")),
            }
        }
        WeatherProvider::Yandex => fetch_yandex_hybrid(city_override, lat, lon, now.hour, updated),
    }
}

fn fetch_yandex_hybrid(
    city_override: Option<(String, f64, f64)>,
    lat: f64,
    lon: f64,
    hour: u8,
    updated: String,
) -> WeatherFetchPayload {
    let key = match cred_load(YANDEX_WEATHER_CRED_TARGET) {
        Ok(Some((_, secret))) if !secret.trim().is_empty() => secret.trim().to_owned(),
        Ok(_) => return err_payload(city_override, "Укажите API-ключ Яндекс.Погоды".to_owned()),
        Err(e) => return err_payload(city_override, format!("Credential Manager: {e}")),
    };
    let res = http_get(
        &build_yandex_forecast_url(lat, lon),
        &[("X-Yandex-Weather-Key".to_owned(), key)],
    );
    if let Some(err) = res.error {
        return err_payload(city_override, format!("Нет сети: {err}"));
    }
    if res.status_code != 200 {
        return err_payload(city_override, format!("Яндекс HTTP {}", res.status_code));
    }
    let mut data = match parse_yandex_forecast(&res.body, hour, updated.clone()) {
        Ok(d) => d,
        Err(e) => return err_payload(city_override, format!("Ошибка Яндекс: {e}")),
    };
    let mut is_hybrid = false;
    if data.daily.len() < 7 {
        let extra = http_get(&build_forecast_url(lat, lon), &[]);
        if extra.error.is_none() && extra.status_code == 200 {
            if let Ok(om) = parse_open_meteo_forecast(&extra.body, hour, updated) {
                if merge_hybrid_days(&mut data.daily, &om.daily) > 0 {
                    is_hybrid = true;
                }
            }
        }
    }
    WeatherFetchPayload {
        city_override,
        data: Some(data),
        is_hybrid,
        error: None,
    }
}

fn merge_hybrid_days(base: &mut Vec<DailySlot>, extra: &[DailySlot]) -> usize {
    let mut added = 0usize;
    let last = base.last().map(|d| d.date_iso.clone()).unwrap_or_default();
    for day in extra {
        if base.len() >= 7 {
            break;
        }
        let newer = if last.is_empty() {
            !base.iter().any(|b| b.date_iso == day.date_iso)
        } else {
            day.date_iso.as_str() > last.as_str()
                && !base.iter().any(|b| b.date_iso == day.date_iso)
        };
        if newer {
            base.push(day.clone());
            added += 1;
        }
    }
    added
}

fn err_payload(city_override: Option<(String, f64, f64)>, msg: String) -> WeatherFetchPayload {
    WeatherFetchPayload {
        city_override,
        data: None,
        is_hybrid: false,
        error: Some(msg),
    }
}

pub(crate) fn sync_weather_ui(s: &WeatherSlintState) {
    let ui = &s.ui;
    let now = local_time_now();
    ui.set_city_name(SharedString::from(s.cfg.city.as_str()));
    ui.set_provider_label(SharedString::from(s.cfg.provider.display_name()));
    ui.set_date_subtitle(SharedString::from(format!(
        "{:02}.{:02}.{} · {}",
        now.day,
        now.month,
        now.year,
        s.cfg.provider.display_name()
    )));
    ui.set_forecast_days(i32::from(normalize_forecast_days(s.cfg.forecast_days)));
    ui.set_loading(s.loading);
    ui.set_error_text(SharedString::from(s.error.as_deref().unwrap_or("")));
    sync_weather_metrics_and_lists(s);
}

fn sync_weather_metrics_and_lists(s: &WeatherSlintState) {
    let ui = &s.ui;
    if s.data.has_live_data {
        ui.set_temp_text(SharedString::from(fmt_temp(s.data.temp_c)));
        ui.set_condition_glyph(SharedString::from(icon_glyph(s.data.wmo_code)));
        ui.set_condition_text(SharedString::from(wmo_description_ru(s.data.wmo_code)));
        ui.set_feels_like_text(SharedString::from(format!(
            "Ощущается как {}",
            fmt_temp(s.data.feels_like_c)
        )));
        ui.set_wind_text(SharedString::from(format!("{:.1} м/с", s.data.wind_ms)));
        ui.set_humidity_text(SharedString::from(format!("{}%", s.data.humidity_pct)));
        ui.set_pressure_text(SharedString::from(format!("{} мм", s.data.pressure_mmhg)));
    }
    let h_rows: Vec<SlintWeatherHourRow> = s
        .data
        .hourly
        .iter()
        .take(5)
        .map(|h| SlintWeatherHourRow {
            time_label: SharedString::from(h.label.as_str()),
            icon_glyph: SharedString::from(icon_glyph(h.wmo_code)),
            temp_text: SharedString::from(fmt_temp(h.temp_c)),
        })
        .collect();
    ui.set_hourly_rows(ModelRc::from(Rc::new(VecModel::from(h_rows))));

    let take_n = usize::from(normalize_forecast_days(s.cfg.forecast_days));
    let slice = &s.data.daily[..s.data.daily.len().min(take_n)];
    let g_min = slice.iter().map(|d| d.min_c).min().unwrap_or(-5);
    let g_max = slice
        .iter()
        .map(|d| d.max_c)
        .max()
        .unwrap_or(10)
        .max(g_min + 1);
    let span = f32::from((g_max - g_min).max(1) as i16);
    let d_rows: Vec<SlintWeatherDayRow> = slice
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let start = ((d.min_c - g_min) as f32 / span).clamp(0.0, 0.85);
            let width = ((d.max_c - d.min_c).max(1) as f32 / span).clamp(0.14, 1.0 - start);
            SlintWeatherDayRow {
                day_label: SharedString::from(day_label_ru(i, &d.date_iso)),
                icon_glyph: SharedString::from(icon_glyph(d.wmo_code)),
                min_text: SharedString::from(fmt_temp(d.min_c)),
                max_text: SharedString::from(fmt_temp(d.max_c)),
                bar_start: start,
                bar_span: width,
                is_hybrid: s.is_hybrid && i >= 2,
            }
        })
        .collect();
    ui.set_daily_rows(ModelRc::from(Rc::new(VecModel::from(d_rows))));

    let footer = if s.is_hybrid {
        format!("Яндекс + Open-Meteo (3–7д) · обн. {}", s.data.updated_label)
    } else if s.data.has_live_data {
        format!(
            "{} · обн. {}",
            s.cfg.provider.display_name(),
            s.data.updated_label
        )
    } else {
        "Ожидание данных…".to_owned()
    };
    ui.set_footer_text(SharedString::from(footer));
}

fn fmt_temp(t: i32) -> String {
    if t > 0 {
        format!("+{t}°")
    } else {
        format!("{t}°")
    }
}

fn icon_glyph(wmo: u16) -> &'static str {
    match wmo_icon_kind(wmo) {
        WeatherIconKind::Sun => "☀",
        WeatherIconKind::PartlyCloudy => "⛅",
        WeatherIconKind::Cloudy => "☁",
        WeatherIconKind::Fog => "🌫",
        WeatherIconKind::Rain => "🌧",
        WeatherIconKind::Snow => "❄",
        WeatherIconKind::Storm => "⛈",
    }
}

fn day_label_ru(idx: usize, iso: &str) -> String {
    if idx == 0 {
        return "Сегодня".to_owned();
    }
    if idx == 1 {
        return "Завтра".to_owned();
    }
    let parts: Vec<&str> = iso.split('-').collect();
    if parts.len() == 3 {
        let d: u8 = parts[2].parse().unwrap_or(1);
        let m: u8 = parts[1].parse().unwrap_or(1);
        let y: i32 = parts[0].parse().unwrap_or(2026);
        let days = asrbox_mod_widgets::rrule::days_from_civil(y, m, d);
        let wd = match asrbox_mod_widgets::rrule::weekday_mon0(days) {
            0 => "Пн",
            1 => "Вт",
            2 => "Ср",
            3 => "Чт",
            4 => "Пт",
            5 => "Сб",
            _ => "Вс",
        };
        return format!("{wd}, {d:02}.{m:02}");
    }
    iso.to_owned()
}
