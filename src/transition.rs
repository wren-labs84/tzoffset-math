//! A rule describing when a fixed-offset region changes its clock,
//! expressed the way real DST laws are usually written: "the Nth
//! weekday of some month, at some local time" (e.g. "second Sunday in
//! March at 02:00"), or "the last weekday of some month" for the older
//! European-style rules.
//!
//! [`TransitionRule`] resolves a rule to the calendar date and wall-clock
//! reading it names for a given year. [`OffsetTransition`] pairs a rule
//! with the offsets on either side of it and turns a local reading into
//! the UTC instant(s) it refers to, including the gap a spring-forward
//! jump opens (a local time that never happens) and the fold a
//! fall-back jump creates (a local time that happens twice).

use crate::civil::{CivilDateTime, Time, UtcOffset};
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

/// A single change of UTC offset at the moment named by a
/// [`TransitionRule`], e.g. "clocks go from PST to PDT at 02:00 PST" or
/// the reverse. The rule's local time is always read using
/// `offset_before` — that's the convention real DST laws use: the wall
/// clock named in the law hasn't changed yet at the instant it names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetTransition {
    pub rule: TransitionRule,
    pub offset_before: UtcOffset,
    pub offset_after: UtcOffset,
}

/// What a local wall-clock reading means once a nearby offset change is
/// taken into account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalResult {
    /// The reading names a moment that never happened: the clocks
    /// jumped past it. `transition_utc` is the instant (Unix seconds)
    /// the jump occurred; `gap_seconds` is how far the clock jumped, so
    /// a caller that wants a best-effort instant instead of an error can
    /// shift the reading by that much.
    Gap { transition_utc: i64, gap_seconds: i32 },
    /// The reading happened exactly once, at this UTC instant.
    Single(i64),
    /// The reading happened twice, once under each offset. `earlier` is
    /// always the smaller (earlier) UTC instant of the two.
    Ambiguous { earlier: i64, later: i64 },
}

impl OffsetTransition {
    pub fn new(rule: TransitionRule, offset_before: UtcOffset, offset_after: UtcOffset) -> Self {
        OffsetTransition {
            rule,
            offset_before,
            offset_after,
        }
    }

    /// The UTC instant, in Unix seconds, at which the offset changes in
    /// `year`.
    pub fn instant_in_year(self, year: i32) -> i64 {
        self.rule
            .datetime_in_year(year)
            .to_unix_seconds_with_offset(self.offset_before)
    }

    /// Resolves a local wall-clock reading against this transition for
    /// `year`, accounting for the gap a forward jump opens or the fold a
    /// backward jump creates.
    pub fn resolve(self, local: CivilDateTime, year: i32) -> LocalResult {
        let shift =
            self.offset_after.total_seconds() as i64 - self.offset_before.total_seconds() as i64;
        let wall = local.to_unix_seconds();
        let named_wall = self.rule.datetime_in_year(year).to_unix_seconds();
        // The named wall-clock reading is where the change happens; the
        // affected interval sits after it for a forward jump (the gap)
        // and before it for a backward jump (the fold).
        let (lo, hi) = if shift >= 0 {
            (named_wall, named_wall + shift)
        } else {
            (named_wall + shift, named_wall)
        };

        if wall < lo {
            LocalResult::Single(local.to_unix_seconds_with_offset(self.offset_before))
        } else if wall < hi {
            if shift > 0 {
                LocalResult::Gap {
                    transition_utc: self.instant_in_year(year),
                    gap_seconds: shift as i32,
                }
            } else {
                LocalResult::Ambiguous {
                    earlier: local.to_unix_seconds_with_offset(self.offset_before),
                    later: local.to_unix_seconds_with_offset(self.offset_after),
                }
            }
        } else {
            LocalResult::Single(local.to_unix_seconds_with_offset(self.offset_after))
        }
    }
}
