//! Wall-clock time and the arithmetic that ties a [`Date`] plus a
//! [`Time`] plus a [`UtcOffset`] together into an actual instant.

use crate::date::Date;

/// A time of day, with no notion of timezone. Leap seconds are not
/// modeled: `second` is always in `0..=59`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Time {
    pub fn new(hour: u8, minute: u8, second: u8) -> Result<Self, TimeError> {
        if hour > 23 || minute > 59 || second > 59 {
            return Err(TimeError::OutOfRange { hour, minute, second });
        }
        Ok(Time { hour, minute, second })
    }

    pub fn seconds_since_midnight(self) -> i64 {
        self.hour as i64 * 3600 + self.minute as i64 * 60 + self.second as i64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeError {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

/// A fixed offset from UTC, such as UTC+05:30. Does not know about
/// daylight saving transitions; it is the offset in force at one instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct UtcOffset {
    total_seconds: i32,
}

impl UtcOffset {
    pub const UTC: UtcOffset = UtcOffset { total_seconds: 0 };

    /// `hours` and `minutes` must have the same sign (or one of them
    /// zero). This is what lets India's +05:30 and Nepal's +05:45 be
    /// expressed exactly, alongside whole-hour offsets like -12:00.
    pub fn from_hm(hours: i32, minutes: i32) -> Result<Self, OffsetError> {
        if hours.signum() * minutes.signum() < 0 {
            return Err(OffsetError::MixedSign { hours, minutes });
        }
        let total_seconds = hours * 3600 + minutes * 60;
        if !(-18 * 3600..=18 * 3600).contains(&total_seconds) {
            return Err(OffsetError::OutOfRange(total_seconds));
        }
        Ok(UtcOffset { total_seconds })
    }

    pub fn total_seconds(self) -> i32 {
        self.total_seconds
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OffsetError {
    MixedSign { hours: i32, minutes: i32 },
    OutOfRange(i32),
}

/// A date and time as they would appear on a wall clock, with no
/// attached timezone. Combine with a [`UtcOffset`] to get a real instant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CivilDateTime {
    pub date: Date,
    pub time: Time,
}

impl CivilDateTime {
    pub fn new(date: Date, time: Time) -> Self {
        CivilDateTime { date, time }
    }

    /// Seconds since the Unix epoch, treating `self` as if it were UTC.
    /// Use [`Self::to_unix_seconds_with_offset`] when the wall clock
    /// reading was taken in a non-UTC offset.
    pub fn to_unix_seconds(self) -> i64 {
        self.date.to_days_since_epoch() * 86_400 + self.time.seconds_since_midnight()
    }

    pub fn from_unix_seconds(secs: i64) -> Self {
        let days = secs.div_euclid(86_400);
        let remainder = secs.rem_euclid(86_400);
        let date = Date::from_days_since_epoch(days);
        let time = Time {
            hour: (remainder / 3600) as u8,
            minute: (remainder / 60 % 60) as u8,
            second: (remainder % 60) as u8,
        };
        CivilDateTime { date, time }
    }

    /// Converts a wall-clock reading taken at `offset` into the UTC
    /// instant it refers to (as Unix seconds).
    pub fn to_unix_seconds_with_offset(self, offset: UtcOffset) -> i64 {
        self.to_unix_seconds() - offset.total_seconds() as i64
    }

    /// The wall-clock reading that `offset` would show at the given UTC
    /// instant.
    pub fn from_unix_seconds_with_offset(secs: i64, offset: UtcOffset) -> Self {
        Self::from_unix_seconds(secs + offset.total_seconds() as i64)
    }
}
