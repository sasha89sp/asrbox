use crate::date::IcsDateTime;
use crate::rrule::{ByDay, Freq, Rrule};

pub fn parse_rrule(raw: &str, local_utc_offset_min: i32) -> Option<Rrule> {
    let mut freq = None;
    let mut interval = 1u32;
    let mut count = None;
    let mut until_min = None;
    let mut by_day = Vec::new();
    let mut by_month_day = Vec::new();
    let mut by_month = Vec::new();
    let mut wkst = 0u8;

    for part in raw.split(';') {
        let Some((k, v)) = part.split_once('=') else {
            continue;
        };
        let v = v.trim();
        match k.trim() {
            "FREQ" => freq = parse_freq(v),
            "INTERVAL" => interval = v.parse::<u32>().unwrap_or(1).max(1),
            "COUNT" => count = v.parse::<u32>().ok().filter(|&c| c > 0),
            "UNTIL" => until_min = parse_until_min(v, local_utc_offset_min),
            "BYDAY" => by_day = v.split(',').filter_map(parse_by_day).collect(),
            "BYMONTHDAY" => {
                by_month_day = v
                    .split(',')
                    .filter_map(|s| s.trim().parse::<i8>().ok())
                    .filter(|&d| d != 0 && (-31..=31).contains(&d))
                    .collect();
            }
            "BYMONTH" => {
                by_month = v
                    .split(',')
                    .filter_map(|s| s.trim().parse::<u8>().ok())
                    .filter(|&m| (1..=12).contains(&m))
                    .collect();
            }
            "WKST" => wkst = parse_weekday(v).unwrap_or(0),
            _ => {}
        }
    }

    Some(Rrule {
        freq: freq?,
        interval,
        count,
        until_min,
        by_day,
        by_month_day,
        by_month,
        wkst,
    })
}

fn parse_freq(s: &str) -> Option<Freq> {
    match s {
        "DAILY" => Some(Freq::Daily),
        "WEEKLY" => Some(Freq::Weekly),
        "MONTHLY" => Some(Freq::Monthly),
        "YEARLY" => Some(Freq::Yearly),
        _ => None,
    }
}

fn parse_weekday(s: &str) -> Option<u8> {
    match s {
        "MO" => Some(0),
        "TU" => Some(1),
        "WE" => Some(2),
        "TH" => Some(3),
        "FR" => Some(4),
        "SA" => Some(5),
        "SU" => Some(6),
        _ => None,
    }
}

fn parse_by_day(raw: &str) -> Option<ByDay> {
    let s = raw.trim();
    if s.len() < 2 {
        return None;
    }
    let at = s.len() - 2;
    let weekday = parse_weekday(&s[at..])?;
    let ordinal = if at == 0 {
        None
    } else {
        let n = s[..at].parse::<i8>().ok()?;
        (n != 0).then_some(n)
    };
    Some(ByDay { ordinal, weekday })
}

fn parse_until_min(s: &str, local_utc_offset_min: i32) -> Option<i64> {
    if s.len() < 8 {
        return None;
    }
    let (y, m, d) = (
        s[0..4].parse::<i32>().ok()?,
        s[4..6].parse::<u8>().ok()?,
        s[6..8].parse::<u8>().ok()?,
    );
    if s.len() >= 15 && s.as_bytes()[8] == b'T' {
        let hh = s[9..11].parse::<u8>().ok()?;
        let mm = s[11..13].parse::<u8>().ok()?;
        let base = IcsDateTime::date_time(y, m, d, hh, mm).to_epoch_min();
        let shift = if s.ends_with('Z') {
            i64::from(local_utc_offset_min)
        } else {
            0
        };
        Some(base + shift)
    } else {
        Some(IcsDateTime::date(y, m, d).to_epoch_min() + 1439)
    }
}
