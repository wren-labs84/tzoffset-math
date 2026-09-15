//! A rule describing when a fixed-offset region changes its clock,
//! expressed the way real DST laws are usually written: "the Nth
//! weekday of some month, at some local time" (e.g. "second Sunday in
//! March at 02:00"), or "the last weekday of some month" for the older
//! European-style rules.
//!
//! This module only resolves a rule to the calendar date and wall-clock
//! reading it names for a given year. Turning that local reading into
//! a UTC instant — including the gap it opens on the spring-forward
//! side and the fold it creates on the fall-back side — is separate,
//! later work.

use crate::civil::{CivilDateTime, Time};
use crate::date::{days_in_month, Date};

/// Day of the week. `Monday` is first to match ISO 8601 ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    fn index(self) -> i64 {
        self as i64
    }
}

impl Date {
    /// The day of the week this date falls on. 1970-01-01 (epoch day 0)
    /// was a Thursday.
    pub fn weekday(self) -> Weekday {
        const WEEKDAYS: [Weekday; 7] = [
            Weekday::Thursday,
            Weekday::Friday,
            Weekday::Saturday,
            Weekday::Sunday,
            Weekday::Monday,
            Weekday::Tuesday,
            Weekday::Wednesday,
        ];
        WEEKDAYS[self.to_days_since_epoch().rem_euclid(7) as usize]
    }
}

/// Which occurrence of a weekday within a month a rule refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeekdayOccurrence {
    First,
    Second,
    Third,
    Fourth,
    /// The last occurrence, whether the month has four or five of them.
    /// This is what older DST rules tend to specify, e.g. the EU's
    /// "last Sunday in March / last Sunday in October".
    Last,
}

/// A rule for when a fixed offset changes, in the style real DST laws
/// are written: e.g. "second Sunday in March at 02:00 local".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionRule {
    pub month: u8,
    pub occurrence: WeekdayOccurrence,
    pub weekday: Weekday,
    pub local_time: Time,
}

impl TransitionRule {
    pub fn new(
        month: u8,
        occurrence: WeekdayOccurrence,
        weekday: Weekday,
        local_time: Time,
    ) -> Result<Self, TransitionRuleError> {
        if !(1..=12).contains(&month) {
            return Err(TransitionRuleError::MonthOutOfRange(month));
        }
        Ok(TransitionRule {
            month,
            occurrence,
            weekday,
            local_time,
        })
    }

    /// The calendar date this rule falls on in a given year.
    pub fn date_in_year(self, year: i32) -> Date {
        match self.occurrence {
            WeekdayOccurrence::Last => {
                let last_day = days_in_month(year, self.month);
                let last = Date::new(year, self.month, last_day).unwrap();
                let back = (last.weekday().index() - self.weekday.index()).rem_euclid(7);
                last.add_days(-back)
            }
            _ => {
                let n = match self.occurrence {
                    WeekdayOccurrence::First => 0,
                    WeekdayOccurrence::Second => 1,
                    WeekdayOccurrence::Third => 2,
                    WeekdayOccurrence::Fourth => 3,
                    WeekdayOccurrence::Last => unreachable!(),
                };
                let first = Date::new(year, self.month, 1).unwrap();
                let forward = (self.weekday.index() - first.weekday().index()).rem_euclid(7);
                first.add_days(forward + 7 * n)
            }
        }
    }

    /// The local wall-clock reading (date and time together) this rule
    /// names in a given year.
    pub fn datetime_in_year(self, year: i32) -> CivilDateTime {
        CivilDateTime::new(self.date_in_year(year), self.local_time)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionRuleError {
    MonthOutOfRange(u8),
}
