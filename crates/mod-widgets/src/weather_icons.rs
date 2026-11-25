use asrbox_platform_api::{BgraFrame, FluentPalette};

use crate::date::{days_from_civil, weekday_mon0};
use crate::draw::{fill_rect, fill_rounded_rect};
use crate::weather_api::WeatherIconKind;

pub(crate) fn draw_weather_icon(
    frame: &mut BgraFrame,
    x: i32,
    y: i32,
    sz: i32,
    kind: WeatherIconKind,
    pal: &FluentPalette,
) {
    let sun = [248, 196, 62, 245];
    let cloud = if pal.is_dark {
        [206, 224, 242, 235]
    } else {
        [96, 122, 152, 235]
    };
    match kind {
        WeatherIconKind::Sun => {
            let r = (sz / 3).max(4);
            fill_rounded_rect(
                frame,
                x + sz / 2 - r,
                y + sz / 2 - r,
                r * 2,
                r * 2,
                r as f32,
                sun,
            );
        }
        WeatherIconKind::PartlyCloudy => {
            let sr = (sz / 4).max(3);
            fill_rounded_rect(frame, x + sz / 2, y + 2, sr * 2, sr * 2, sr as f32, sun);
            draw_cloud_shape(frame, x + 1, y + sz / 3, sz - 4, sz / 2, cloud);
        }
        WeatherIconKind::Cloudy | WeatherIconKind::Fog => {
            draw_cloud_shape(frame, x + 2, y + sz / 4, sz - 4, sz / 2, cloud);
        }
        WeatherIconKind::Rain | WeatherIconKind::Storm => {
            draw_cloud_shape(frame, x + 2, y + 2, sz - 4, sz / 2, cloud);
            let drop_col = if kind == WeatherIconKind::Storm {
                sun
            } else {
                pal.accent
            };
            for i in 0..3 {
                fill_rect(
                    frame,
                    x + 5 + i * (sz / 4),
                    y + sz * 2 / 3,
                    2,
                    (sz / 5).max(3),
                    drop_col,
                );
            }
        }
        WeatherIconKind::Snow => {
            draw_cloud_shape(frame, x + 2, y + 2, sz - 4, sz / 2, cloud);
            for i in 0..3 {
                fill_rect(
                    frame,
                    x + 5 + i * (sz / 4),
                    y + sz * 2 / 3,
                    3,
                    3,
                    [226, 240, 255, 240],
                );
            }
        }
    }
}

fn draw_cloud_shape(frame: &mut BgraFrame, x: i32, y: i32, w: i32, h: i32, col: [u8; 4]) {
    let base_h = (h * 2 / 3).max(6);
    fill_rounded_rect(
        frame,
        x,
        y + h - base_h,
        w,
        base_h,
        (base_h as f32) * 0.48,
        col,
    );
    let bump = (w * 5 / 9).max(6);
    fill_rounded_rect(
        frame,
        x + w / 5,
        y,
        bump,
        bump.min(h),
        (bump as f32) * 0.48,
        col,
    );
}

pub(crate) fn format_signed_temp(t: i32) -> String {
    if t > 0 {
        format!("+{t}°")
    } else {
        format!("{t}°")
    }
}

pub(crate) fn weekday_short_ru(mon0: u8) -> &'static str {
    ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"][usize::from(mon0.min(6))]
}

pub(crate) fn weekday_from_iso(iso: &str) -> String {
    let mut parts = iso.split('-');
    let y = parts.next().and_then(|s| s.parse::<i32>().ok());
    let m = parts.next().and_then(|s| s.parse::<u8>().ok());
    let d = parts.next().and_then(|s| s.parse::<u8>().ok());
    if let (Some(y), Some(m), Some(d)) = (y, m, d) {
        weekday_short_ru(weekday_mon0(days_from_civil(y, m, d))).to_owned()
    } else {
        iso.to_owned()
    }
}
