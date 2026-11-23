use asrbox_platform_api::LocalDateTime;

use crate::ics::CalendarEvent;
use crate::rrule::{IcsDateTime, days_from_civil, weekday_mon0};
use crate::storage::{DateFormatMode, PeriodFilter};

const COLOR_TODAY: [u8; 3] = [52, 211, 153];
const COLOR_TOMORROW: [u8; 3] = [245, 158, 11];
const COLOR_LATER: [u8; 3] = [59, 130, 246];

pub fn local_now_as_ics(now: LocalDateTime) -> IcsDateTime {
    IcsDateTime::date_time(now.year, now.month, now.day, now.hour, now.minute)
}

pub fn event_matches_period(
    ev: &CalendarEvent,
    now: LocalDateTime,
    period: PeriodFilter,
    lookahead_days: u16,
) -> bool {
    let today_day = days_from_civil(now.year, now.month, now.day);
    let now_min = local_now_as_ics(now).to_epoch_min();
    if ev.end.to_epoch_min() <= now_min {
        return false;
    }
    let max_days = match period {
        PeriodFilter::Today => 1,
        PeriodFilter::Days7 => 7,
        PeriodFilter::Days30 => 30,
        PeriodFilter::All => i64::from(lookahead_days),
    };
    let end_limit_min = (today_day + max_days) * 1440;
    ev.start.to_epoch_min() < end_limit_min
}

pub fn pick_bar_color(ev: &CalendarEvent, now: LocalDateTime) -> [u8; 3] {
    if let Some(custom) = ev.custom_color {
        return custom;
    }
    let today_day = days_from_civil(now.year, now.month, now.day);
    let ev_day = ev.start.days_since_epoch();
    if ev_day <= today_day {
        COLOR_TODAY
    } else if ev_day == today_day + 1 {
        COLOR_TOMORROW
    } else {
        COLOR_LATER
    }
}

pub fn format_event_subtitle(
    ev: &CalendarEvent,
    now: LocalDateTime,
    mode: DateFormatMode,
) -> String {
    match mode {
        DateFormatMode::Normal => format_normal(ev, now),
        DateFormatMode::Numeric => format_numeric(ev),
        DateFormatMode::Countdown => format_countdown(ev, now),
    }
}

pub fn format_event_tooltip(ev: &CalendarEvent, now: LocalDateTime) -> String {
    let mut tip = String::new();
    tip.push_str(&ev.summary);
    tip.push('\n');
    tip.push_str(&format_normal(ev, now));
    if !ev.all_day {
        tip.push_str(&format!(" — {:02}:{:02}", ev.end.hour, ev.end.minute));
    }
    let desc = ev.description.trim();
    if !desc.is_empty() {
        tip.push('\n');
        let snippet: String = desc.chars().take(180).collect();
        tip.push_str(&snippet);
        if desc.chars().count() > 180 {
            tip.push('…');
        }
    }
    tip
}

pub fn event_day_url(ev: &CalendarEvent) -> String {
    format!(
        "https://calendar.google.com/calendar/r/day/{}/{}/{}",
        ev.start.year, ev.start.month, ev.start.day
    )
}

fn format_normal(ev: &CalendarEvent, now: LocalDateTime) -> String {
    let today_day = days_from_civil(now.year, now.month, now.day);
    let ev_day = ev.start.days_since_epoch();
    let date_part = if ev_day <= today_day {
        "Сегодня".to_owned()
    } else if ev_day == today_day + 1 {
        "Завтра".to_owned()
    } else {
        let wd = short_weekday_ru(weekday_mon0(ev_day));
        let mon = short_month_ru(ev.start.month);
        format!("{wd}, {} {mon}", ev.start.day)
    };
    if ev.all_day {
        format!("{date_part} · весь день")
    } else {
        format!("{date_part} · {:02}:{:02}", ev.start.hour, ev.start.minute)
    }
}

fn format_numeric(ev: &CalendarEvent) -> String {
    let date_part = format!(
        "{:02}.{:02}.{}",
        ev.start.day, ev.start.month, ev.start.year
    );
    if ev.all_day {
        format!("{date_part} · весь день")
    } else {
        format!("{date_part} · {:02}:{:02}", ev.start.hour, ev.start.minute)
    }
}

fn format_countdown(ev: &CalendarEvent, now: LocalDateTime) -> String {
    let now_min = local_now_as_ics(now).to_epoch_min();
    let start_min = ev.start.to_epoch_min();
    let end_min = ev.end.to_epoch_min();
    if now_min >= start_min && now_min < end_min {
        if ev.all_day {
            return "идёт сейчас (весь день)".to_owned();
        }
        return format!("идёт сейчас (до {:02}:{:02})", ev.end.hour, ev.end.minute);
    }
    let diff = (start_min - now_min).max(1);
    if diff < 60 {
        format!("через {diff} мин")
    } else if diff < 1440 {
        let h = diff / 60;
        let m = diff % 60;
        if m == 0 {
            format!("через {h} ч")
        } else {
            format!("через {h} ч {m} мин")
        }
    } else {
        let days = (diff + 720) / 1440;
        format!("через {days} дн.")
    }
}

fn short_weekday_ru(wd: u8) -> &'static str {
    match wd {
        0 => "Пн",
        1 => "Вт",
        2 => "Ср",
        3 => "Чт",
        4 => "Пт",
        5 => "Сб",
        _ => "Вс",
    }
}

fn short_month_ru(month: u8) -> &'static str {
    match month {
        1 => "янв",
        2 => "фев",
        3 => "мар",
        4 => "апр",
        5 => "май",
        6 => "июн",
        7 => "июл",
        8 => "авг",
        9 => "сен",
        10 => "окт",
        11 => "ноя",
        _ => "дек",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_all_three_modes() {
        let now = LocalDateTime {
            year: 2026,
            month: 11,
            day: 23,
            hour: 9,
            minute: 15,
            second: 0,
            utc_offset_min: 180,
        };
        let ev = CalendarEvent {
            uid: "1".to_owned(),
            calendar_id: "main".to_owned(),
            summary: "Встреча".to_owned(),
            description: "Детали".to_owned(),
            start: IcsDateTime::date_time(2026, 11, 23, 10, 0),
            end: IcsDateTime::date_time(2026, 11, 23, 11, 30),
            all_day: false,
            custom_color: None,
        };
        assert_eq!(
            format_event_subtitle(&ev, now, DateFormatMode::Normal),
            "Сегодня · 10:00"
        );
        assert_eq!(
            format_event_subtitle(&ev, now, DateFormatMode::Numeric),
            "23.11.2026 · 10:00"
        );
        assert_eq!(
            format_event_subtitle(&ev, now, DateFormatMode::Countdown),
            "через 45 мин"
        );
    }
}
