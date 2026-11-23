use std::collections::{HashMap, HashSet};
use std::io::BufRead;

use crate::date::IcsDateTime;
use crate::ics_time::{
    default_end, is_busy_title, parse_ics_datetime, parse_tz_offset, unescape_ics_text,
};
use crate::rrule::{Rrule, expand_rrule, parse_rrule};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarEvent {
    pub uid: String,
    pub calendar_id: String,
    pub summary: String,
    pub description: String,
    pub start: IcsDateTime,
    pub end: IcsDateTime,
    pub all_day: bool,
    pub custom_color: Option<[u8; 3]>,
}

#[derive(Debug, Clone, Copy)]
pub struct ParseOptions {
    pub window_start_min: i64,
    pub window_end_min: i64,
    pub local_utc_offset_min: i32,
}

#[derive(Debug, Clone, Default)]
pub struct ParsedCalendar {
    pub events: Vec<CalendarEvent>,
    pub busy_only_warning: bool,
    pub total_vevents_seen: usize,
}

#[derive(Default)]
struct RawEvent {
    uid: String,
    summary: String,
    description: String,
    start: Option<IcsDateTime>,
    end: Option<IcsDateTime>,
    recurrence_id: Option<IcsDateTime>,
    rrule: Option<Rrule>,
    exdates: Vec<i64>,
    cancelled: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Root,
    VTimezone,
    VEvent,
}

struct StreamState {
    section: Section,
    cur_tz_id: String,
    tz_offsets: HashMap<String, i32>,
    current: RawEvent,
    masters: Vec<RawEvent>,
    overrides: HashMap<(String, i64), Option<RawEvent>>,
    seen: usize,
    opts: ParseOptions,
}

pub fn parse_ics_bytes(
    bytes: &[u8],
    calendar_id: &str,
    custom_color: Option<[u8; 3]>,
    opts: ParseOptions,
) -> ParsedCalendar {
    let mut state = StreamState {
        section: Section::Root,
        cur_tz_id: String::new(),
        tz_offsets: HashMap::new(),
        current: RawEvent::default(),
        masters: Vec::new(),
        overrides: HashMap::new(),
        seen: 0,
        opts,
    };

    let mut pending = String::new();
    for line in bytes.lines().map_while(Result::ok) {
        let trimmed = line.trim_end_matches('\r');
        if let Some(cont) = trimmed
            .strip_prefix(' ')
            .or_else(|| trimmed.strip_prefix('\t'))
        {
            pending.push_str(cont);
            continue;
        }
        if !pending.is_empty() {
            state.process_line(&pending);
            pending.clear();
        }
        pending.push_str(trimmed);
    }
    if !pending.is_empty() {
        state.process_line(&pending);
    }

    finalize_events(state, calendar_id, custom_color)
}

impl StreamState {
    fn process_line(&mut self, line: &str) {
        match line {
            "BEGIN:VTIMEZONE" => {
                self.section = Section::VTimezone;
                self.cur_tz_id.clear();
                return;
            }
            "END:VTIMEZONE" => {
                self.section = Section::Root;
                return;
            }
            "BEGIN:VEVENT" => {
                self.section = Section::VEvent;
                self.current = RawEvent::default();
                return;
            }
            "END:VEVENT" => {
                self.section = Section::Root;
                self.seen += 1;
                let finished = std::mem::take(&mut self.current);
                self.store_raw_event(finished);
                return;
            }
            _ => {}
        }

        let Some((key_and_params, value)) = line.split_once(':') else {
            return;
        };
        match self.section {
            Section::VTimezone => self.handle_tz_prop(key_and_params, value),
            Section::VEvent => self.handle_event_prop(key_and_params, value),
            Section::Root => {}
        }
    }

    fn handle_tz_prop(&mut self, key: &str, value: &str) {
        let name = key.split(';').next().unwrap_or("");
        if name == "TZID" {
            self.cur_tz_id = value.trim().to_owned();
        } else if name == "TZOFFSETTO" && !self.cur_tz_id.is_empty() {
            if let Some(off) = parse_tz_offset(value.trim()) {
                self.tz_offsets.entry(self.cur_tz_id.clone()).or_insert(off);
            }
        }
    }

    fn handle_event_prop(&mut self, key_and_params: &str, value: &str) {
        let mut parts = key_and_params.split(';');
        let name = parts.next().unwrap_or("");
        let params: Vec<&str> = parts.collect();
        let tz = &self.tz_offsets;
        let off = self.opts.local_utc_offset_min;
        match name {
            "UID" => self.current.uid = value.trim().to_owned(),
            "SUMMARY" => self.current.summary = unescape_ics_text(value),
            "DESCRIPTION" => self.current.description = unescape_ics_text(value),
            "STATUS" if value.trim().eq_ignore_ascii_case("CANCELLED") => {
                self.current.cancelled = true;
            }
            "DTSTART" => self.current.start = parse_ics_datetime(value, &params, tz, off),
            "DTEND" => self.current.end = parse_ics_datetime(value, &params, tz, off),
            "RECURRENCE-ID" => {
                self.current.recurrence_id = parse_ics_datetime(value, &params, tz, off);
            }
            "RRULE" => self.current.rrule = parse_rrule(value, off),
            "EXDATE" => {
                for tok in value.split(',') {
                    if let Some(dt) = parse_ics_datetime(tok, &params, tz, off) {
                        self.current.exdates.push(dt.to_epoch_min());
                    }
                }
            }
            _ => {}
        }
    }

    fn store_raw_event(&mut self, ev: RawEvent) {
        if let Some(rec_id) = ev.recurrence_id {
            let key = (ev.uid.clone(), rec_id.to_epoch_min());
            let entry = if ev.cancelled { None } else { Some(ev) };
            self.overrides.insert(key, entry);
            return;
        }
        if ev.cancelled {
            return;
        }
        let Some(start) = ev.start else {
            return;
        };
        let end = ev.end.unwrap_or_else(|| default_end(start));
        if let Some(ref rrule) = ev.rrule {
            if let Some(until) = rrule.until_min {
                let dur = (end.to_epoch_min() - start.to_epoch_min()).max(0);
                if until + dur < self.opts.window_start_min {
                    return;
                }
            }
            self.masters.push(ev);
        } else if in_window(start, end, self.opts) {
            self.masters.push(ev);
        }
    }
}

fn finalize_events(
    mut state: StreamState,
    calendar_id: &str,
    custom_color: Option<[u8; 3]>,
) -> ParsedCalendar {
    let mut events = Vec::new();
    for raw in state.masters {
        expand_master_event(
            &raw,
            &mut state.overrides,
            calendar_id,
            custom_color,
            state.opts,
            &mut events,
        );
    }
    for (_, over) in state.overrides {
        if let Some(ov_ev) = over {
            push_override(&ov_ev, calendar_id, custom_color, state.opts, &mut events);
        }
    }
    let busy_count = events.iter().filter(|e| is_busy_title(&e.summary)).count();
    let busy_only_warning = !events.is_empty() && busy_count == events.len();
    ParsedCalendar {
        events,
        busy_only_warning,
        total_vevents_seen: state.seen,
    }
}

fn expand_master_event(
    raw: &RawEvent,
    overrides: &mut HashMap<(String, i64), Option<RawEvent>>,
    calendar_id: &str,
    custom_color: Option<[u8; 3]>,
    opts: ParseOptions,
    events: &mut Vec<CalendarEvent>,
) {
    let Some(start) = raw.start else {
        return;
    };
    let end = raw.end.unwrap_or_else(|| default_end(start));
    let min_dur = if start.all_day { 1440 } else { 0 };
    let duration_min = (end.to_epoch_min() - start.to_epoch_min()).max(min_dur);
    let ex_set: HashSet<i64> = raw.exdates.iter().copied().collect();
    let starts = if let Some(ref rrule) = raw.rrule {
        let from = opts.window_start_min - duration_min;
        expand_rrule(start, rrule, from, opts.window_end_min, 512)
    } else {
        vec![start]
    };

    for occ_start in starts {
        let occ_min = occ_start.to_epoch_min();
        if ex_set.contains(&occ_min) {
            continue;
        }
        let key = (raw.uid.clone(), occ_min);
        if let Some(over) = overrides.remove(&key) {
            if let Some(ov_ev) = over {
                push_override(&ov_ev, calendar_id, custom_color, opts, events);
            }
            continue;
        }
        let occ_end = IcsDateTime::from_epoch_min(occ_min + duration_min, start.all_day);
        if in_window(occ_start, occ_end, opts) {
            events.push(CalendarEvent {
                uid: raw.uid.clone(),
                calendar_id: calendar_id.to_owned(),
                summary: raw.summary.clone(),
                description: raw.description.clone(),
                start: occ_start,
                end: occ_end,
                all_day: start.all_day,
                custom_color,
            });
        }
    }
}

fn push_override(
    ov: &RawEvent,
    calendar_id: &str,
    custom_color: Option<[u8; 3]>,
    opts: ParseOptions,
    out: &mut Vec<CalendarEvent>,
) {
    let Some(start) = ov.start else {
        return;
    };
    let end = ov.end.unwrap_or_else(|| default_end(start));
    if in_window(start, end, opts) {
        out.push(CalendarEvent {
            uid: ov.uid.clone(),
            calendar_id: calendar_id.to_owned(),
            summary: ov.summary.clone(),
            description: ov.description.clone(),
            start,
            end,
            all_day: start.all_day,
            custom_color,
        });
    }
}

fn in_window(start: IcsDateTime, end: IcsDateTime, opts: ParseOptions) -> bool {
    end.to_epoch_min() > opts.window_start_min && start.to_epoch_min() <= opts.window_end_min
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unfolds_unescapes_tz_exdate_and_override() {
        let ics = concat!(
            "BEGIN:VCALENDAR\r\n",
            "BEGIN:VTIMEZONE\r\nTZID:Custom/MSK\r\nBEGIN:STANDARD\r\n",
            "TZOFFSETTO:+0300\r\nEND:STANDARD\r\nEND:VTIMEZONE\r\n",
            "BEGIN:VEVENT\r\nUID:e1\r\n",
            "SUMMARY:ОЗОН\\, карта\r\n",
            "  моя\r\n",
            "DESCRIPTION:Строка 1\\nСтрока 2\\; ок\r\n",
            "DTSTART;TZID=Custom/MSK:20261123T100000\r\n",
            "DTEND;TZID=Custom/MSK:20261123T110000\r\n",
            "RRULE:FREQ=DAILY;COUNT=3\r\n",
            "EXDATE;TZID=Custom/MSK:20261124T100000\r\n",
            "END:VEVENT\r\n",
            "BEGIN:VEVENT\r\nUID:e1\r\nRECURRENCE-ID;TZID=Custom/MSK:20261125T100000\r\n",
            "SUMMARY:Перенесено\r\n",
            "DTSTART;TZID=Custom/MSK:20261125T140000\r\n",
            "DTEND;TZID=Custom/MSK:20261125T150000\r\n",
            "END:VEVENT\r\nEND:VCALENDAR\r\n",
        );
        let opts = ParseOptions {
            window_start_min: IcsDateTime::date(2026, 11, 23).to_epoch_min(),
            window_end_min: IcsDateTime::date(2026, 11, 30).to_epoch_min(),
            local_utc_offset_min: 180,
        };
        let parsed = parse_ics_bytes(ics.as_bytes(), "main", None, opts);
        assert_eq!(parsed.events.len(), 2);
        assert_eq!(parsed.events[0].summary, "ОЗОН, карта моя");
        assert_eq!(parsed.events[0].description, "Строка 1\nСтрока 2; ок");
        assert_eq!(parsed.events[1].summary, "Перенесено");
        assert_eq!(parsed.events[1].start.hour, 14);
    }

    #[test]
    fn streams_seven_thousand_events_fast_skipping_archive() {
        let mut buf = String::from("BEGIN:VCALENDAR\r\n");
        for i in 0..7_050 {
            let year = if i < 7_000 { 2020 } else { 2026 };
            buf.push_str(&format!(
                "BEGIN:VEVENT\r\nUID:u{i}\r\nSUMMARY:Ev {i}\r\n\
                 DTSTART;VALUE=DATE:{year}1125\r\nDTEND;VALUE=DATE:{year}1126\r\nEND:VEVENT\r\n"
            ));
        }
        buf.push_str("END:VCALENDAR\r\n");
        let opts = ParseOptions {
            window_start_min: IcsDateTime::date(2026, 11, 23).to_epoch_min(),
            window_end_min: IcsDateTime::date(2026, 12, 31).to_epoch_min(),
            local_utc_offset_min: 180,
        };
        let parsed = parse_ics_bytes(buf.as_bytes(), "big", None, opts);
        assert_eq!(parsed.total_vevents_seen, 7_050);
        assert_eq!(parsed.events.len(), 50);
    }
}
