use asrbox_platform_api::{
    BgraFrame, FluentPalette, LocalDateTime, PixelPoint, TextAlign, WidgetFrame, WidgetTextCmd,
};

use crate::date::{days_from_civil, weekday_mon0};
use crate::draw::{
    draw_hline, draw_resize_grip, fill_macos_glass_card, fill_rect, fill_rounded_rect, push_txt,
};
use crate::weather_api::{wmo_description_ru, wmo_icon_kind, WeatherData, WeatherIconKind};

pub const WEATHER_HEADER_H: i32 = 52;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WeatherHitTarget {
    #[default]
    None,
    HeaderDrag,
    SettingsBtn,
    RefreshBtn,
}

pub struct WeatherRenderInput<'a> {
    pub width: u32,
    pub height: u32,
    pub opacity_percent: u8,
    pub city: &'a str,
    pub now: LocalDateTime,
    pub data: &'a WeatherData,
    pub loading: bool,
    pub error: Option<&'a str>,
    pub hovered: WeatherHitTarget,
    pub palette: FluentPalette,
}

pub fn hit_test_weather(pt: PixelPoint, width: u32, height: u32) -> WeatherHitTarget {
    let (w, h) = (width as i32, height as i32);
    if pt.x < 0 || pt.y < 0 || pt.x >= w || pt.y >= h {
        return WeatherHitTarget::None;
    }
    if pt.y < WEATHER_HEADER_H {
        if pt.x >= w - 34 && pt.x < w - 10 {
            return WeatherHitTarget::RefreshBtn;
        }
        if pt.x >= w - 62 && pt.x < w - 38 {
            return WeatherHitTarget::SettingsBtn;
        }
        return WeatherHitTarget::HeaderDrag;
    }
    WeatherHitTarget::None
}

pub fn render_weather_widget(inp: &WeatherRenderInput<'_>) -> WidgetFrame {
    let (w, h) = (inp.width.max(280), inp.height.max(310));
    let (wi, hi) = (w as i32, h as i32);
    let mut bgra = BgraFrame::transparent(w, h);
    let mut texts = Vec::with_capacity(48);
    let pal = &inp.palette;
    let pct = u16::from(inp.opacity_percent.clamp(20, 100));
    let div = [
        pal.divider[0],
        pal.divider[1],
        pal.divider[2],
        ((pct * 210) / 100).clamp(55, 225) as u8,
    ];

    fill_macos_glass_card(&mut bgra, inp.opacity_percent, pal);
    draw_header(&mut bgra, &mut texts, inp, wi);
    draw_hline(&mut bgra, WEATHER_HEADER_H, div);

    let y_hero = WEATHER_HEADER_H + 8;
    draw_hero(&mut texts, inp, wi, y_hero);

    let y_pills = y_hero + 56;
    draw_metric_pills(&mut bgra, &mut texts, inp, wi, y_pills);
    draw_hline(&mut bgra, y_pills + 34, div);

    let y_hourly = y_pills + 42;
    draw_hourly_strip(&mut bgra, &mut texts, inp, wi, y_hourly);
    draw_hline(&mut bgra, y_hourly + 68, div);

    let y_daily = y_hourly + 74;
    let footer_y = hi - 26;
    draw_daily_list(&mut bgra, &mut texts, inp, wi, y_daily, footer_y - y_daily);
    draw_footer(&mut texts, inp, wi, footer_y);
    draw_resize_grip(&mut bgra, pal);
    WidgetFrame { bgra, texts }
}

fn draw_header(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    inp: &WeatherRenderInput<'_>,
    w: i32,
) {
    let pal = &inp.palette;
    draw_weather_icon(frame, 14, 12, 28, wmo_icon_kind(inp.data.wmo_code), pal);
    push_txt(
        texts,
        (48, 8, w - 118, 22),
        inp.city,
        (15, true),
        pal.text_primary,
        TextAlign::Left,
    );
    let dt = inp.now;
    let wd = weekday_short_ru(weekday_mon0(days_from_civil(dt.year, dt.month, dt.day)));
    let date_line = format!("{wd}, {:02}.{:02}.{:04}", dt.day, dt.month, dt.year);
    push_txt(
        texts,
        (48, 29, w - 118, 16),
        &date_line,
        (11, false),
        pal.text_secondary,
        TextAlign::Left,
    );

    let hov_bg = [pal.menu_hover[0], pal.menu_hover[1], pal.menu_hover[2], 200];
    let refresh_glyph = if inp.loading { "…" } else { "↻" };
    for (target, bx, glyph) in [
        (WeatherHitTarget::SettingsBtn, w - 62, "⚙"),
        (WeatherHitTarget::RefreshBtn, w - 34, refresh_glyph),
    ] {
        let hov = inp.hovered == target;
        if hov {
            fill_rounded_rect(frame, bx, 14, 24, 24, 6.0, hov_bg);
        }
        let col = if hov {
            pal.text_primary
        } else {
            pal.text_secondary
        };
        push_txt(
            texts,
            (bx, 13, 24, 24),
            glyph,
            (14, false),
            col,
            TextAlign::Center,
        );
    }
}

fn draw_hero(texts: &mut Vec<WidgetTextCmd>, inp: &WeatherRenderInput<'_>, w: i32, y: i32) {
    let pal = &inp.palette;
    let temp_str = if inp.data.has_live_data {
        format_signed_temp(inp.data.temp_c)
    } else {
        "—°".to_owned()
    };
    push_txt(
        texts,
        (16, y, 92, 48),
        &temp_str,
        (36, true),
        pal.text_primary,
        TextAlign::Left,
    );
    let desc = if inp.data.has_live_data {
        wmo_description_ru(inp.data.wmo_code)
    } else if inp.loading {
        "Загрузка погоды…"
    } else {
        "Нажмите ↻ для обновления"
    };
    push_txt(
        texts,
        (108, y + 6, w - 122, 22),
        desc,
        (14, true),
        pal.text_primary,
        TextAlign::Left,
    );
    let feels = if inp.data.has_live_data {
        format!(
            "Ощущается как {}",
            format_signed_temp(inp.data.feels_like_c)
        )
    } else {
        "Open-Meteo (без API-ключа)".to_owned()
    };
    push_txt(
        texts,
        (108, y + 29, w - 122, 18),
        &feels,
        (12, false),
        pal.text_secondary,
        TextAlign::Left,
    );
}

fn draw_metric_pills(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    inp: &WeatherRenderInput<'_>,
    w: i32,
    y: i32,
) {
    let pal = &inp.palette;
    let pill_bg = [
        pal.card_bg[0],
        pal.card_bg[1],
        pal.card_bg[2],
        if pal.is_dark { 175 } else { 215 },
    ];
    let gap = 6;
    let pw = ((w - 28 - gap * 2) / 3).max(60);
    let items = [
        format!("Ветер: {:.1} м/с", inp.data.wind_ms),
        format!("Влажность: {}%", inp.data.humidity_pct),
        format!("Давление: {} мм", inp.data.pressure_mmhg),
    ];
    for (i, label) in items.iter().enumerate() {
        let px = 14 + (i as i32) * (pw + gap);
        fill_rounded_rect(frame, px, y, pw, 26, 7.0, pill_bg);
        push_txt(
            texts,
            (px + 3, y + 4, pw - 6, 18),
            label,
            (11, false),
            pal.text_primary,
            TextAlign::Center,
        );
    }
}

fn draw_hourly_strip(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    inp: &WeatherRenderInput<'_>,
    w: i32,
    y: i32,
) {
    let pal = &inp.palette;
    let count = inp.data.hourly.len().min(5);
    if count == 0 {
        push_txt(
            texts,
            (14, y + 20, w - 28, 20),
            "Почасовой прогноз появится после обновления",
            (11, false),
            pal.text_muted,
            TextAlign::Center,
        );
        return;
    }
    let col_w = ((w - 28) / count as i32).max(40);
    for (i, slot) in inp.data.hourly.iter().take(count).enumerate() {
        let cx = 14 + (i as i32) * col_w;
        push_txt(
            texts,
            (cx, y, col_w, 16),
            &slot.label,
            (11, false),
            pal.text_secondary,
            TextAlign::Center,
        );
        draw_weather_icon(
            frame,
            cx + (col_w - 20) / 2,
            y + 20,
            20,
            wmo_icon_kind(slot.wmo_code),
            pal,
        );
        push_txt(
            texts,
            (cx, y + 44, col_w, 18),
            &format_signed_temp(slot.temp_c),
            (12, true),
            pal.text_primary,
            TextAlign::Center,
        );
    }
}

fn draw_daily_list(
    frame: &mut BgraFrame,
    texts: &mut Vec<WidgetTextCmd>,
    inp: &WeatherRenderInput<'_>,
    w: i32,
    y: i32,
    avail_h: i32,
) {
    let pal = &inp.palette;
    let count = inp.data.daily.len().min(4);
    if count == 0 || avail_h < 28 {
        return;
    }
    let min_all = inp.data.daily.iter().map(|d| d.min_c).min().unwrap_or(0);
    let max_all = inp
        .data
        .daily
        .iter()
        .map(|d| d.max_c)
        .max()
        .unwrap_or(min_all + 1)
        .max(min_all + 1);
    let span = (max_all - min_all) as f32;
    let step = (avail_h / count as i32).clamp(22, 34);
    let track_bg = if pal.is_dark {
        [255, 255, 255, 32]
    } else {
        [15, 23, 42, 26]
    };

    for (i, day) in inp.data.daily.iter().take(count).enumerate() {
        let ry = y + (i as i32) * step;
        let name = if i == 0 {
            "Сегодня".to_owned()
        } else {
            weekday_from_iso(&day.date_iso)
        };
        push_txt(
            texts,
            (16, ry + 3, 66, 18),
            &name,
            (12, i == 0),
            pal.text_primary,
            TextAlign::Left,
        );
        draw_weather_icon(frame, 88, ry + 2, 18, wmo_icon_kind(day.wmo_code), pal);

        let bar_x = 120;
        let bar_w = (w - 208).clamp(44, 150);
        let bar_y = ry + 9;
        fill_rounded_rect(frame, bar_x, bar_y, bar_w, 5, 2.5, track_bg);
        let f0 = ((day.min_c - min_all) as f32 / span).clamp(0.0, 1.0);
        let f1 = ((day.max_c - min_all) as f32 / span).clamp(f0, 1.0);
        let bx = bar_x + (f0 * bar_w as f32).round() as i32;
        let bw = (((f1 - f0) * bar_w as f32).round() as i32)
            .max(10)
            .min(bar_x + bar_w - bx);
        fill_rounded_rect(frame, bx, bar_y, bw, 5, 2.5, pal.accent);

        let range = format!(
            "{} … {}",
            format_signed_temp(day.min_c),
            format_signed_temp(day.max_c)
        );
        push_txt(
            texts,
            (w - 84, ry + 3, 70, 18),
            &range,
            (12, false),
            pal.text_secondary,
            TextAlign::Right,
        );
    }
}

fn draw_footer(texts: &mut Vec<WidgetTextCmd>, inp: &WeatherRenderInput<'_>, w: i32, y: i32) {
    let pal = &inp.palette;
    let (msg, col) = if let Some(err) = inp.error {
        (err.to_owned(), pal.warn_text)
    } else if inp.data.updated_label.is_empty() {
        ("Open-Meteo".to_owned(), pal.text_muted)
    } else {
        (
            format!("Обн. {} • Open-Meteo", inp.data.updated_label),
            pal.text_muted,
        )
    };
    push_txt(
        texts,
        (16, y + 3, w - 36, 18),
        &msg,
        (11, false),
        col,
        TextAlign::Right,
    );
}

fn draw_weather_icon(
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

fn format_signed_temp(t: i32) -> String {
    if t > 0 {
        format!("+{t}°")
    } else {
        format!("{t}°")
    }
}

fn weekday_short_ru(mon0: u8) -> &'static str {
    ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"][usize::from(mon0.min(6))]
}

fn weekday_from_iso(iso: &str) -> String {
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
