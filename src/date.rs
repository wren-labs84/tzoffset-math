//! Proleptic Gregorian calendar dates, independent of any clock or offset.

/// A calendar date. `year` may be zero or negative (year 0 is 1 BCE in the
/// proleptic Gregorian calendar, following the astronomical convention).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl Date {
    /// Builds a date, rejecting anything that isn't a real day on the
    /// calendar (Feb 30, day 0, month 13, etc).
    pub fn new(year: i32, month: u8, day: u8) -> Result<Self, DateError> {
        if !(1..=12).contains(&month) {
            return Err(DateError::MonthOutOfRange(month));
        }
        let max_day = days_in_month(year, month);
        if day == 0 || day > max_day {
            return Err(DateError::DayOutOfRange { year, month, day });
        }
        Ok(Date { year, month, day })
    }

    /// Days since 1970-01-01, using Howard Hinnant's `days_from_civil`
    /// algorithm. Works for any proleptic Gregorian date, positive or
    /// negative, without overflowing for realistic years.
    pub fn to_days_since_epoch(self) -> i64 {
        let y = self.year as i64;
        let m = self.month as i64;
        let d = self.day as i64;
        let y = if m <= 2 { y - 1 } else { y };
        let era = if y >= 0 { y } else { y - 399 } / 400;
        let yoe = y - era * 400; // [0, 399]
        let mp = (m + 9) % 12; // [0, 11], Mar=0 .. Feb=11
        let doy = (153 * mp + 2) / 5 + d - 1; // [0, 365]
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
        era * 146_097 + doe - 719_468
    }

    /// Inverse of [`Date::to_days_since_epoch`].
    pub fn from_days_since_epoch(days: i64) -> Self {
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097; // [0, 146096]
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
        let mp = (5 * doy + 2) / 153; // [0, 11]
        let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u8;
        let y = if m <= 2 { y + 1 } else { y };
        Date {
            year: y as i32,
            month: m,
            day: d,
        }
    }

    /// Returns the date `n` days after this one (or before, if negative).
    pub fn add_days(self, n: i64) -> Self {
        Self::from_days_since_epoch(self.to_days_since_epoch() + n)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateError {
    MonthOutOfRange(u8),
    DayOutOfRange { year: i32, month: u8, day: u8 },
}

pub fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

pub fn days_in_month(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}
