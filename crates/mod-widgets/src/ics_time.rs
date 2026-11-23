use std::collections::HashMap;

use crate::date::IcsDateTime;

pub fn default_end(start: IcsDateTime) -> IcsDateTime {
    if start.all_day {
        start.add_days(1)
    } else {
        IcsDateTime::from_epoch_min(start.to_epoch_min() + 60, false)
    }
}

pub fn parse_ics_datetime(
    raw: &str,
    params: &[&str],
    tz_offsets: &HashMap<String, i32>,
    local_utc_offset_min: i32,
) -> Option<IcsDateTime> {
    let s = raw.trim();
    if s.len() < 8 {
        return None;
    }
    let y = s[0..4].parse::<i32>().ok()?;
    let m = s[4..6].parse::<u8>().ok()?;
    let d = s[6..8].parse::<u8>().ok()?;
    let is_date_param = params
        .iter()
        .any(|p| p.eq_ignore_ascii_case("VALUE=DATE"));
    if is_date_param || s.len() == 8 {
        return Some(IcsDateTime::date(y, m, d));
    }
    if s.len() < 15 || s.as_bytes()[8] != b'T' {
        return None;
    }
    let hh = s[9..11].parse::<u8>().ok()?;
    let mm = s[11..13].parse::<u8>().ok()?;
    let raw_min = IcsDateTime::date_time(y, m, d, hh, mm).to_epoch_min();

    let shifted = if s.ends_with('Z') {
        raw_min + i64::from(local_utc_offset_min)
    } else if let Some(tzid) = extract_tzid(params) {
        let src = tz_offsets
            .get(tzid)
            .copied()
            .or_else(|| known_tz_offset(tzid))
            .unwrap_or(local_utc_offset_min);
        raw_min - i64::from(src) + i64::from(local_utc_offset_min)
    } else {
        raw_min
    };
    Some(IcsDateTime::from_epoch_min(shifted, false))
}

pub fn parse_tz_offset(s: &str) -> Option<i32> {
    let bytes = s.as_bytes();
    if bytes.len() < 5 {
        return None;
    }
    let sign = match bytes[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let hh = s[1..3].parse::<i32>().ok()?;
    let mm = s[3..5].parse::<i32>().ok()?;
    Some(sign * (hh * 60 + mm))
}

pub fn unescape_ics_text(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n' | 'N') => out.push('\n'),
                Some(',') => out.push(','),
                Some(';') => out.push(';'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn is_busy_title(title: &str) -> bool {
    let t = title.trim();
    t.eq_ignore_ascii_case("busy")
        || t.eq_ignore_ascii_case("занят")
        || t.eq_ignore_ascii_case("занято")
}

fn extract_tzid<'a>(params: &[&'a str]) -> Option<&'a str> {
    for p in params {
        if let Some(rest) = p
            .strip_prefix("TZID=")
            .or_else(|| p.strip_prefix("tzid="))
        {
            return Some(rest.trim_matches('"'));
        }
    }
    None
}

fn known_tz_offset(tzid: &str) -> Option<i32> {
    match tzid {
        "UTC" | "Etc/UTC" | "GMT" | "Europe/London" => Some(0),
        "Europe/Berlin" | "Europe/Paris" | "Europe/Warsaw" => Some(60),
        "Europe/Kiev" | "Europe/Kyiv" | "Europe/Helsinki" => Some(120),
        "Europe/Moscow" | "Europe/Minsk" | "Europe/Istanbul" => Some(180),
        "Asia/Dubai" | "Europe/Samara" => Some(240),
        "Asia/Yekaterinburg" | "Asia/Tashkent" => Some(300),
        "Asia/Almaty" | "Asia/Omsk" => Some(360),
        "Asia/Novosibirsk" | "Asia/Bangkok" => Some(420),
        "Asia/Tokyo" => Some(540),
        "America/New_York" => Some(-300),
        "America/Los_Angeles" => Some(-480),
        _ => None,
    }
}
