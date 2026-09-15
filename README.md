# tzoffset-math

Calendar and UTC-offset arithmetic for Rust, standard library only.

## The problem

A surprising amount of "timezone" code is really just calendar
arithmetic done carelessly: is 1900 a leap year (no), does adding a day
to Feb 28 in a leap year land on Feb 29 or roll into March, does a
UTC+14 wall-clock reading of 23:30 belong to today or tomorrow in UTC.
Get any of these wrong and every downstream calculation is off by a
day at exactly the boundary cases nobody tests.

This crate is the boring, careful layer underneath: proleptic
Gregorian dates, wall-clock times, and fixed UTC offsets, with the
conversions between them worked out so the awkward cases (century
leap years, negative offsets, non-integer offsets like +05:30 and
+05:45) are actually correct instead of merely untested.

It does not ship a timezone database and does not know when any real
place observes daylight saving. That is a real limitation, not an
oversight — see Roadmap below.

## Usage

```rust
use tzoffset_math::{CivilDateTime, Date, Time, UtcOffset};

// A meeting scheduled for 23:30 local time in a UTC+14 timezone.
let local = CivilDateTime::new(
    Date::new(2023, 6, 1).unwrap(),
    Time::new(23, 30, 0).unwrap(),
);
let offset = UtcOffset::from_hm(14, 0).unwrap();

// That instant is still May 31st in UTC.
let utc_seconds = local.to_unix_seconds_with_offset(offset);
let utc = CivilDateTime::from_unix_seconds(utc_seconds);
assert_eq!(utc.date, Date::new(2023, 5, 31).unwrap());
assert_eq!(utc.time, Time::new(9, 30, 0).unwrap());

// Calendar arithmetic handles century leap years correctly.
let day_before = Date::new(1900, 3, 1).unwrap().add_days(-1);
assert_eq!(day_before, Date::new(1900, 2, 28).unwrap()); // 1900 was not a leap year
```

## What's here

- `Date` — a proleptic Gregorian calendar date, with day arithmetic and
  a Unix-epoch day count in both directions.
- `Time` — a wall-clock time of day (no leap seconds).
- `UtcOffset` — a fixed offset from UTC, down to the minute, so
  half-hour and 45-minute offsets are exact.
- `CivilDateTime` — a date and time together, with conversion to and
  from Unix seconds either as UTC directly or through a `UtcOffset`.
- `Weekday` and `Date::weekday` — the day of the week a date falls on.
- `TransitionRule` — a DST-style rule ("second Sunday in March at
  02:00 local", "last Sunday in October") resolved to a concrete date
  for a given year.

## Roadmap

`TransitionRule` describes *when* a rule fires but doesn't yet turn
that local reading into a UTC instant. That's the next piece: handling
the "gap" a spring-forward transition opens (a local time that never
happens) and the "fold" a fall-back transition creates (a local time
that happens twice).

## License

MIT, see `LICENSE`.
