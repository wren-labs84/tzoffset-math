//! The cases here are picked to be the ones that a naive implementation
//! (days_in_month table without century rules, offset math done on
//! hours alone, sign errors on negative offsets) tends to get wrong.

use tzoffset_math::{days_in_month, is_leap_year, CivilDateTime, Date, Time, UtcOffset};

#[test]
fn leap_year_edge_cases() {
    let cases: &[(i32, bool)] = &[
        (1996, true),  // ordinary leap year
        (1900, false), // divisible by 100, not by 400: not leap
        (2000, true),  // divisible by 400: leap
        (2004, true),
        (2100, false), // next century-non-leap after 2000
        (1, false),
        (0, true), // year 0 (1 BCE) is divisible by 400
        (-4, true),
    ];
    for &(year, expected) in cases {
        assert_eq!(is_leap_year(year), expected, "year {year}");
    }
}

#[test]
fn days_in_month_edge_cases() {
    let cases: &[(i32, u8, u8)] = &[
        (2024, 2, 29), // leap February
        (2023, 2, 28), // non-leap February
        (1900, 2, 28), // century non-leap February
        (2000, 2, 29), // century leap February
        (2023, 4, 30),
        (2023, 12, 31),
    ];
    for &(year, month, expected) in cases {
        assert_eq!(days_in_month(year, month), expected, "{year}-{month:02}");
    }
}

#[test]
fn date_rejects_invalid_days() {
    assert!(Date::new(2023, 2, 29).is_err()); // not a leap year
    assert!(Date::new(2023, 4, 31).is_err()); // April has 30 days
    assert!(Date::new(2023, 1, 0).is_err());
    assert!(Date::new(2023, 13, 1).is_err());
    assert!(Date::new(2024, 2, 29).is_ok());
}

#[test]
fn date_epoch_round_trip_across_awkward_boundaries() {
    // (year, month, day, expected days since 1970-01-01)
    let cases: &[(i32, u8, u8, i64)] = &[
        (1970, 1, 1, 0),
        (1970, 1, 2, 1),
        (1969, 12, 31, -1),   // day before the epoch
        (2000, 2, 29, 11_016), // the famous century leap day
        (2000, 3, 1, 11_017),
        (1900, 3, 1, -25_508), // right after the century non-leap Feb
        (1, 1, 1, -719_162),   // proleptic calendar, year 1
        (2024, 12, 31, 20_088),
        (2025, 1, 1, 20_089), // year rollover
    ];
    for &(year, month, day, expected_days) in cases {
        let date = Date::new(year, month, day).unwrap();
        assert_eq!(
            date.to_days_since_epoch(),
            expected_days,
            "{year}-{month:02}-{day:02} -> days"
        );
        assert_eq!(
            Date::from_days_since_epoch(expected_days),
            date,
            "days {expected_days} -> date"
        );
    }
}

#[test]
fn add_days_crosses_month_and_year_and_leap_boundaries() {
    let cases: &[((i32, u8, u8), i64, (i32, u8, u8))] = &[
        ((2023, 2, 28), 1, (2023, 3, 1)),  // non-leap Feb rollover
        ((2024, 2, 28), 1, (2024, 2, 29)), // leap Feb has a 29th
        ((2024, 2, 29), 1, (2024, 3, 1)),  // leap Feb rollover
        ((2023, 12, 31), 1, (2024, 1, 1)), // year rollover
        ((2024, 1, 1), -1, (2023, 12, 31)), // stepping backward over a year
        ((2000, 1, 1), -1, (1999, 12, 31)), // backward over a century leap year
    ];
    for &((y, m, d), delta, (ey, em, ed)) in cases {
        let start = Date::new(y, m, d).unwrap();
        let got = start.add_days(delta);
        let expected = Date::new(ey, em, ed).unwrap();
        assert_eq!(got, expected, "{y}-{m:02}-{d:02} + {delta}d");
    }
}

#[test]
fn offset_conversion_can_shift_the_calendar_date() {
    // (local date, local time, offset h:m, expected UTC unix seconds)
    let cases: &[((i32, u8, u8), (u8, u8, u8), (i32, i32), i64)] = &[
        // 23:30 in UTC+14 (Kiritimati) is still the previous UTC day.
        ((2023, 6, 1), (23, 30, 0), (14, 0), {
            let local = CivilDateTime::new(
                Date::new(2023, 6, 1).unwrap(),
                Time::new(23, 30, 0).unwrap(),
            );
            local.to_unix_seconds() - 14 * 3600
        }),
        // 00:15 in UTC-12 (Baker Island) is already the next UTC day.
        ((2023, 6, 1), (0, 15, 0), (-12, 0), {
            let local = CivilDateTime::new(
                Date::new(2023, 6, 1).unwrap(),
                Time::new(0, 15, 0).unwrap(),
            );
            local.to_unix_seconds() + 12 * 3600
        }),
        // Non-integer-hour offsets: India (+05:30) and Nepal (+05:45).
        ((2023, 6, 1), (5, 30, 0), (5, 30), {
            let local = CivilDateTime::new(
                Date::new(2023, 6, 1).unwrap(),
                Time::new(5, 30, 0).unwrap(),
            );
            local.to_unix_seconds() - (5 * 3600 + 30 * 60)
        }),
    ];
    for &((y, m, d), (h, mi, s), (oh, om), expected_utc) in cases {
        let date = Date::new(y, m, d).unwrap();
        let time = Time::new(h, mi, s).unwrap();
        let offset = UtcOffset::from_hm(oh, om).unwrap();
        let local = CivilDateTime::new(date, time);
        let got = local.to_unix_seconds_with_offset(offset);
        assert_eq!(got, expected_utc, "{y}-{m:02}-{d:02} {h:02}:{mi:02} at {oh}:{om}");

        // And the round trip: going back from that UTC instant at the
        // same offset must reproduce the original wall-clock reading.
        let back = CivilDateTime::from_unix_seconds_with_offset(got, offset);
        assert_eq!(back, local, "round trip at offset {oh}:{om}");
    }
}

#[test]
fn offset_rejects_mixed_sign_and_out_of_range() {
    assert!(UtcOffset::from_hm(5, -30).is_err()); // signs disagree
    assert!(UtcOffset::from_hm(-5, 30).is_err());
    assert!(UtcOffset::from_hm(19, 0).is_err()); // beyond +/-18:00
    assert!(UtcOffset::from_hm(5, 30).is_ok());
    assert!(UtcOffset::from_hm(-5, -30).is_ok());
    assert!(UtcOffset::from_hm(0, 0).is_ok());
}

#[test]
fn unix_seconds_round_trip_for_negative_instants() {
    // A handful of instants before 1970, including one that lands
    // exactly on a leap day, to catch off-by-one errors in div_euclid
    // usage for negative seconds.
    let cases: &[i64] = &[-1, -86_400, -86_399, -1_000_000_000, 0, 1_700_000_000];
    for &secs in cases {
        let dt = CivilDateTime::from_unix_seconds(secs);
        assert_eq!(dt.to_unix_seconds(), secs, "round trip for {secs}");
    }
}
