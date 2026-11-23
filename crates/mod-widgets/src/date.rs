#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IcsDateTime {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub all_day: bool,
}

impl IcsDateTime {
    pub fn date(year: i32, month: u8, day: u8) -> Self {
        Self {
            year,
            month,
            day,
            hour: 0,
            minute: 0,
            all_day: true,
        }
    }

    pub fn date_time(year: i32, month: u8, day: u8, hour: u8, minute: u8) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            all_day: false,
        }
    }

    pub fn days_since_epoch(self) -> i64 {
        days_from_civil(self.year, self.month, self.day)
    }

    pub fn to_epoch_min(self) -> i64 {
        let day_min = self.days_since_epoch() * 1440;
        if self.all_day {
            day_min
        } else {
            day_min + i64::from(self.hour) * 60 + i64::from(self.minute)
        }
    }

    pub fn from_epoch_min(epoch_min: i64, all_day: bool) -> Self {
        let days = epoch_min.div_euclid(1440);
        let rem = epoch_min.rem_euclid(1440);
        let (year, month, day) = civil_from_days(days);
        if all_day {
            Self::date(year, month, day)
        } else {
            Self::date_time(year, month, day, (rem / 60) as u8, (rem % 60) as u8)
        }
    }

    pub fn add_days(self, delta: i64) -> Self {
        let (y, m, d) = civil_from_days(self.days_since_epoch() + delta);
        Self {
            year: y,
            month: m,
            day: d,
            hour: self.hour,
            minute: self.minute,
            all_day: self.all_day,
        }
    }

    pub fn with_date(self, year: i32, month: u8, day: u8) -> Self {
        Self {
            year,
            month,
            day,
            hour: self.hour,
            minute: self.minute,
            all_day: self.all_day,
        }
    }
}

pub fn days_from_civil(year: i32, month: u8, day: u8) -> i64 {
    let m = i64::from(month);
    let y = i64::from(year) - if m <= 2 { 1 } else { 0 };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn civil_from_days(z: i64) -> (i32, u8, u8) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year as i32, m as u8, d as u8)
}

pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 30,
    }
}

/// Returns weekday 0 = Monday .. 6 = Sunday.
pub fn weekday_mon0(days_since_epoch: i64) -> u8 {
    (days_since_epoch + 3).rem_euclid(7) as u8
}
