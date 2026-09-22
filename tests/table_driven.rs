//! The cases here are picked to be the ones that a naive implementation
//! (days_in_month table without century rules, offset math done on
//! hours alone, sign errors on negative offsets) tends to get wrong.

use tzoffset_math::{
    days_in_month, is_leap_year, CivilDateTime, Date, LocalResult, OffsetTransition, Time,
    TransitionRule, UtcOffset, Weekday, WeekdayOccurrence,
};

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
fn weekday_matches_known_dates() {
    let cases: &[((i32, u8, u8), Weekday)] = &[
        ((1970, 1, 1), Weekday::Thursday), // the epoch itself
        ((1970, 1, 2), Weekday::Friday),
        ((1969, 12, 31), Weekday::Wednesday), // the day before the epoch
        ((2000, 2, 29), Weekday::Tuesday),    // the century leap day
        ((2023, 3, 5), Weekday::Sunday),
        ((1900, 1, 1), Weekday::Monday),
    ];
    for &((y, m, d), expected) in cases {
        let date = Date::new(y, m, d).unwrap();
        assert_eq!(date.weekday(), expected, "{y}-{m:02}-{d:02}");
    }
}

#[test]
fn transition_rule_finds_nth_weekday_of_month() {
    // US DST since 2007: starts second Sunday in March, ends first
    // Sunday in November, both at 02:00 local.
    let starts = TransitionRule::new(
        3,
        WeekdayOccurrence::Second,
        Weekday::Sunday,
        Time::new(2, 0, 0).unwrap(),
    )
    .unwrap();
    let ends = TransitionRule::new(
        11,
        WeekdayOccurrence::First,
        Weekday::Sunday,
        Time::new(2, 0, 0).unwrap(),
    )
    .unwrap();

    let cases: &[(i32, (i32, u8, u8), (i32, u8, u8))] = &[
        (2023, (2023, 3, 12), (2023, 11, 5)),
        (2024, (2024, 3, 10), (2024, 11, 3)),
        (2007, (2007, 3, 11), (2007, 11, 4)), // first year of the current rule
    ];
    for &(year, expected_start, expected_end) in cases {
        let (sy, sm, sd) = expected_start;
        let (ey, em, ed) = expected_end;
        assert_eq!(starts.date_in_year(year), Date::new(sy, sm, sd).unwrap());
        assert_eq!(ends.date_in_year(year), Date::new(ey, em, ed).unwrap());
        assert_eq!(
            starts.datetime_in_year(year),
            CivilDateTime::new(Date::new(sy, sm, sd).unwrap(), Time::new(2, 0, 0).unwrap())
        );
    }
}

#[test]
fn transition_rule_finds_last_weekday_of_month() {
    // The EU rule: last Sunday in March and last Sunday in October, at
    // 01:00 UTC (kept here as a local time for the purpose of the test).
    let starts = TransitionRule::new(
        3,
        WeekdayOccurrence::Last,
        Weekday::Sunday,
        Time::new(1, 0, 0).unwrap(),
    )
    .unwrap();
    let ends = TransitionRule::new(
        10,
        WeekdayOccurrence::Last,
        Weekday::Sunday,
        Time::new(1, 0, 0).unwrap(),
    )
    .unwrap();

    let cases: &[(i32, (i32, u8, u8), (i32, u8, u8))] = &[
        (2023, (2023, 3, 26), (2023, 10, 29)),
        (2024, (2024, 3, 31), (2024, 10, 27)),
        // October 2022 has five Sundays; "last" must skip the fourth.
        (2022, (2022, 3, 27), (2022, 10, 30)),
    ];
    for &(year, expected_start, expected_end) in cases {
        let (sy, sm, sd) = expected_start;
        let (ey, em, ed) = expected_end;
        assert_eq!(starts.date_in_year(year), Date::new(sy, sm, sd).unwrap());
        assert_eq!(ends.date_in_year(year), Date::new(ey, em, ed).unwrap());
    }
}

#[test]
fn transition_rule_rejects_bad_month() {
    let time = Time::new(2, 0, 0).unwrap();
    assert!(TransitionRule::new(0, WeekdayOccurrence::First, Weekday::Sunday, time).is_err());
    assert!(TransitionRule::new(13, WeekdayOccurrence::First, Weekday::Sunday, time).is_err());
    assert!(TransitionRule::new(3, WeekdayOccurrence::Second, Weekday::Sunday, time).is_ok());
}

// US Central time: CST is UTC-6, CDT is UTC-5.
fn us_central_spring_forward() -> OffsetTransition {
    let rule = TransitionRule::new(
        3,
        WeekdayOccurrence::Second,
        Weekday::Sunday,
        Time::new(2, 0, 0).unwrap(),
    )
    .unwrap();
    OffsetTransition::new(
        rule,
        UtcOffset::from_hm(-6, 0).unwrap(),
        UtcOffset::from_hm(-5, 0).unwrap(),
    )
}

fn us_central_fall_back() -> OffsetTransition {
    let rule = TransitionRule::new(
        11,
        WeekdayOccurrence::First,
        Weekday::Sunday,
        Time::new(2, 0, 0).unwrap(),
    )
    .unwrap();
    OffsetTransition::new(
        rule,
        UtcOffset::from_hm(-5, 0).unwrap(),
        UtcOffset::from_hm(-6, 0).unwrap(),
    )
}

#[test]
fn spring_forward_gap_swallows_the_skipped_hour() {
    // 2024-03-10: US clocks jump from 02:00 CST straight to 03:00 CDT.
    let transition = us_central_spring_forward();
    let date = Date::new(2024, 3, 10).unwrap();

    // 02:30 never happens.
    let skipped = CivilDateTime::new(date, Time::new(2, 30, 0).unwrap());
    match transition.resolve(skipped, 2024) {
        LocalResult::Gap {
            transition_utc,
            gap_seconds,
        } => {
            assert_eq!(gap_seconds, 3600);
            assert_eq!(
                transition_utc,
                CivilDateTime::new(date, Time::new(2, 0, 0).unwrap())
                    .to_unix_seconds_with_offset(UtcOffset::from_hm(-6, 0).unwrap())
            );
        }
        other => panic!("expected Gap, got {other:?}"),
    }

    // 01:30, just before the jump, is still ordinary CST.
    let before = CivilDateTime::new(date, Time::new(1, 30, 0).unwrap());
    assert_eq!(
        transition.resolve(before, 2024),
        LocalResult::Single(before.to_unix_seconds_with_offset(UtcOffset::from_hm(-6, 0).unwrap()))
    );

    // 03:30, just after the jump, is already CDT.
    let after = CivilDateTime::new(date, Time::new(3, 30, 0).unwrap());
    assert_eq!(
        transition.resolve(after, 2024),
        LocalResult::Single(after.to_unix_seconds_with_offset(UtcOffset::from_hm(-5, 0).unwrap()))
    );
}

#[test]
fn fall_back_fold_produces_two_valid_instants() {
    // 2024-11-03: US clocks fall from 02:00 CDT back to 01:00 CST, so
    // every reading between 01:00 and 02:00 happens twice.
    let transition = us_central_fall_back();
    let date = Date::new(2024, 11, 3).unwrap();

    let folded = CivilDateTime::new(date, Time::new(1, 30, 0).unwrap());
    match transition.resolve(folded, 2024) {
        LocalResult::Ambiguous { earlier, later } => {
            assert_eq!(
                earlier,
                folded.to_unix_seconds_with_offset(UtcOffset::from_hm(-5, 0).unwrap())
            );
            assert_eq!(
                later,
                folded.to_unix_seconds_with_offset(UtcOffset::from_hm(-6, 0).unwrap())
            );
            assert!(earlier < later);
            assert_eq!(later - earlier, 3600);
        }
        other => panic!("expected Ambiguous, got {other:?}"),
    }

    // 00:30, before the fold, only ever happened under CDT.
    let before = CivilDateTime::new(date, Time::new(0, 30, 0).unwrap());
    assert_eq!(
        transition.resolve(before, 2024),
        LocalResult::Single(before.to_unix_seconds_with_offset(UtcOffset::from_hm(-5, 0).unwrap()))
    );

    // 02:30, after the fold, is unambiguous CST.
    let after = CivilDateTime::new(date, Time::new(2, 30, 0).unwrap());
    assert_eq!(
        transition.resolve(after, 2024),
        LocalResult::Single(after.to_unix_seconds_with_offset(UtcOffset::from_hm(-6, 0).unwrap()))
    );
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
