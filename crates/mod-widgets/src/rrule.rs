pub use crate::date::{civil_from_days, days_from_civil, days_in_month, weekday_mon0, IcsDateTime};
pub use crate::rrule_parse::parse_rrule;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freq {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ByDay {
    pub ordinal: Option<i8>,
    pub weekday: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rrule {
    pub freq: Freq,
    pub interval: u32,
    pub count: Option<u32>,
    pub until_min: Option<i64>,
    pub by_day: Vec<ByDay>,
    pub by_month_day: Vec<i8>,
    pub by_month: Vec<u8>,
    pub wkst: u8,
}

struct ExpandCtx<'a> {
    start: IcsDateTime,
    start_min: i64,
    rrule: &'a Rrule,
    win_start: i64,
    end_min: i64,
    max_results: usize,
    emitted: u32,
    out: Vec<IcsDateTime>,
}

impl<'a> ExpandCtx<'a> {
    fn new(
        start: IcsDateTime,
        rrule: &'a Rrule,
        win_start: i64,
        end_min: i64,
        max_results: usize,
    ) -> Self {
        Self {
            start,
            start_min: start.to_epoch_min(),
            rrule,
            win_start,
            end_min,
            max_results,
            emitted: 0,
            out: Vec::new(),
        }
    }

    fn push_candidate(&mut self, dt: IcsDateTime) -> bool {
        let t = dt.to_epoch_min();
        if t < self.start_min {
            return true;
        }
        if t > self.end_min {
            return false;
        }
        self.emitted += 1;
        if t >= self.win_start {
            self.out.push(dt);
            if self.out.len() >= self.max_results {
                return false;
            }
        }
        !self.rrule.count.is_some_and(|c| self.emitted >= c)
    }
}

pub fn expand_rrule(
    start: IcsDateTime,
    rrule: &Rrule,
    win_start_min: i64,
    win_end_min: i64,
    max_results: usize,
) -> Vec<IcsDateTime> {
    if rrule.until_min.is_some_and(|u| u < win_start_min) {
        return Vec::new();
    }
    let effective_end = rrule.until_min.map_or(win_end_min, |u| u.min(win_end_min));
    if start.to_epoch_min() > effective_end {
        return Vec::new();
    }
    let mut ctx = ExpandCtx::new(start, rrule, win_start_min, effective_end, max_results);
    match rrule.freq {
        Freq::Daily => expand_daily(&mut ctx),
        Freq::Weekly => expand_weekly(&mut ctx),
        Freq::Monthly => expand_monthly(&mut ctx),
        Freq::Yearly => expand_yearly(&mut ctx),
    }
    ctx.out
}

fn expand_daily(ctx: &mut ExpandCtx<'_>) {
    let step = i64::from(ctx.rrule.interval);
    let start_day = ctx.start.days_since_epoch();
    let mut idx = 0i64;
    if ctx.rrule.count.is_none() && ctx.rrule.by_day.is_empty() && ctx.rrule.by_month.is_empty() {
        let win_day = ctx.win_start.div_euclid(1440) - 2;
        if win_day > start_day {
            idx = (win_day - start_day) / step;
        }
    }
    while idx < 40_000 {
        let dt = ctx.start.add_days(idx * step);
        if dt.to_epoch_min() > ctx.end_min {
            break;
        }
        if matches_day_filters(dt, ctx.rrule) && !ctx.push_candidate(dt) {
            break;
        }
        idx += 1;
    }
}

fn expand_weekly(ctx: &mut ExpandCtx<'_>) {
    let start_day = ctx.start.days_since_epoch();
    let wkst = ctx.rrule.wkst;
    let wk_offset = (weekday_mon0(start_day) + 7 - wkst) % 7;
    let first_week = start_day - i64::from(wk_offset);
    let step_days = i64::from(ctx.rrule.interval) * 7;

    let mut days: Vec<u8> = if ctx.rrule.by_day.is_empty() {
        vec![weekday_mon0(start_day)]
    } else {
        ctx.rrule.by_day.iter().map(|b| b.weekday).collect()
    };
    days.sort_by_key(|&wd| (wd + 7 - wkst) % 7);
    days.dedup();

    let mut wk = 0i64;
    if ctx.rrule.count.is_none() {
        let win_day = ctx.win_start.div_euclid(1440) - 7;
        if win_day > first_week {
            wk = (win_day - first_week) / step_days;
        }
    }
    while wk < 10_000 {
        let base = first_week + wk * step_days;
        if base * 1440 > ctx.end_min + 1440 {
            break;
        }
        for &wd in &days {
            let (y, m, d) = civil_from_days(base + i64::from((wd + 7 - wkst) % 7));
            if !ctx.rrule.by_month.is_empty() && !ctx.rrule.by_month.contains(&m) {
                continue;
            }
            if !ctx.push_candidate(ctx.start.with_date(y, m, d)) {
                return;
            }
        }
        wk += 1;
    }
}

fn expand_monthly(ctx: &mut ExpandCtx<'_>) {
    let base_m = i64::from(ctx.start.year) * 12 + i64::from(ctx.start.month - 1);
    let step = i64::from(ctx.rrule.interval);
    let mut m_idx = 0i64;
    if ctx.rrule.count.is_none() {
        let (wy, wm, _) = civil_from_days(ctx.win_start.div_euclid(1440));
        let win_m = i64::from(wy) * 12 + i64::from(wm - 1) - 1;
        if win_m > base_m {
            m_idx = (win_m - base_m) / step;
        }
    }
    while m_idx < 3_600 {
        let total = base_m + m_idx * step;
        let y = total.div_euclid(12) as i32;
        let m = (total.rem_euclid(12) + 1) as u8;
        if days_from_civil(y, m, 1) * 1440 > ctx.end_min + 1440 {
            break;
        }
        if ctx.rrule.by_month.is_empty() || ctx.rrule.by_month.contains(&m) {
            for d in month_candidate_days(y, m, ctx.start.day, ctx.rrule) {
                if !ctx.push_candidate(ctx.start.with_date(y, m, d)) {
                    return;
                }
            }
        }
        m_idx += 1;
    }
}

fn expand_yearly(ctx: &mut ExpandCtx<'_>) {
    let step = i64::from(ctx.rrule.interval);
    let mut y_idx = 0i64;
    if ctx.rrule.count.is_none() {
        let (wy, _, _) = civil_from_days(ctx.win_start.div_euclid(1440));
        let delta = i64::from(wy - ctx.start.year) - 1;
        if delta > 0 {
            y_idx = delta / step;
        }
    }
    let mut months: Vec<u8> = if ctx.rrule.by_month.is_empty() {
        vec![ctx.start.month]
    } else {
        ctx.rrule.by_month.clone()
    };
    months.sort_unstable();
    months.dedup();

    while y_idx < 300 {
        let y = ctx.start.year + (y_idx * step) as i32;
        if days_from_civil(y, 1, 1) * 1440 > ctx.end_min + 1440 {
            break;
        }
        for &m in &months {
            for d in month_candidate_days(y, m, ctx.start.day, ctx.rrule) {
                if !ctx.push_candidate(ctx.start.with_date(y, m, d)) {
                    return;
                }
            }
        }
        y_idx += 1;
    }
}

fn month_candidate_days(year: i32, month: u8, default_day: u8, rrule: &Rrule) -> Vec<u8> {
    let dim = days_in_month(year, month);
    let mut days = Vec::new();
    if !rrule.by_day.is_empty() {
        for rule_day in &rrule.by_day {
            let matching: Vec<u8> = (1..=dim)
                .filter(|&d| weekday_mon0(days_from_civil(year, month, d)) == rule_day.weekday)
                .collect();
            match rule_day.ordinal {
                Some(ord) if ord > 0 => {
                    if let Some(&d) = matching.get((ord - 1) as usize) {
                        days.push(d);
                    }
                }
                Some(ord) if ord < 0 => {
                    if let Some(pos) = matching.len().checked_sub(ord.unsigned_abs() as usize) {
                        days.push(matching[pos]);
                    }
                }
                _ => days.extend(matching),
            }
        }
    } else if !rrule.by_month_day.is_empty() {
        for &md in &rrule.by_month_day {
            let res = if md > 0 {
                i16::from(md)
            } else {
                i16::from(dim) + i16::from(md) + 1
            };
            if res >= 1 && res <= i16::from(dim) {
                days.push(res as u8);
            }
        }
    } else if default_day <= dim {
        days.push(default_day);
    }
    days.sort_unstable();
    days.dedup();
    days
}

fn matches_day_filters(dt: IcsDateTime, rrule: &Rrule) -> bool {
    if !rrule.by_month.is_empty() && !rrule.by_month.contains(&dt.month) {
        return false;
    }
    if !rrule.by_day.is_empty() {
        let wd = weekday_mon0(dt.days_since_epoch());
        if !rrule.by_day.iter().any(|b| b.weekday == wd) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_monthly_ordinals_and_weekly_wkst() {
        let Some(m_rule) = parse_rrule("FREQ=MONTHLY;BYDAY=2MO,-1FR", 0) else {
            panic!("valid monthly rrule");
        };
        let m_res = expand_rrule(
            IcsDateTime::date_time(2026, 1, 1, 10, 0),
            &m_rule,
            IcsDateTime::date(2026, 1, 1).to_epoch_min(),
            IcsDateTime::date(2026, 2, 28).to_epoch_min() + 1439,
            10,
        );
        let m_days: Vec<(u8, u8)> = m_res.iter().map(|d| (d.month, d.day)).collect();
        assert_eq!(m_days, vec![(1, 12), (1, 30), (2, 9), (2, 27)]);

        let Some(w_rule) = parse_rrule("FREQ=WEEKLY;INTERVAL=2;BYDAY=SU,MO;WKST=SU;COUNT=4", 0)
        else {
            panic!("valid weekly rrule");
        };
        let w_res = expand_rrule(
            IcsDateTime::date_time(2026, 11, 22, 9, 0),
            &w_rule,
            IcsDateTime::date(2026, 11, 1).to_epoch_min(),
            IcsDateTime::date(2026, 12, 31).to_epoch_min(),
            10,
        );
        let w_days: Vec<(u8, u8)> = w_res.iter().map(|d| (d.month, d.day)).collect();
        assert_eq!(w_days, vec![(11, 22), (11, 23), (12, 6), (12, 7)]);
    }
}
