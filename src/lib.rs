//! Civil calendar dates, wall-clock times, and fixed UTC offsets, with
//! the arithmetic needed to convert between a local reading and an
//! actual instant.
//!
//! This crate does not ship a timezone database and does not know
//! about daylight saving rules for any particular place. What it gets
//! right is the boring, easy-to-botch part underneath that: calendar
//! day arithmetic across leap years and month boundaries, and offset
//! math that doesn't drift by a day when the offset pushes a date
//! across midnight.

mod civil;
mod date;

pub use civil::{CivilDateTime, OffsetError, Time, TimeError, UtcOffset};
pub use date::{days_in_month, is_leap_year, Date, DateError};
