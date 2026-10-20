//! Date, calendar, and schedule textbook examples and property coverage.
use finance_rs::calendars::{BusinessDayConvention as Bdc, Calendar, NullCalendar, WeekendsOnly};
use finance_rs::daycount::DayCount;
use finance_rs::period::Period;
use finance_rs::rates::Frequency;
use finance_rs::schedule::{DateGeneration, Schedule};
use finance_rs::{Date, Weekday};
use proptest::prelude::*;

fn d(y: i32, m: u32, day: u32) -> Date {
    match Date::new(y, m, day) {
        Ok(date) => date,
        Err(error) => {
            eprintln!("invalid fixture date {y}-{m}-{day}: {error}");
            std::process::abort();
        }
    }
}

#[test]
fn date_helpers_match_calendar_examples() {
    let leap = d(2024, 2, 29);
    assert_eq!(leap.add_days(1).unwrap(), d(2024, 3, 1));
    assert_eq!(leap.add_years(1).unwrap(), d(2025, 2, 28));
    assert_eq!(leap.weekday(), Weekday::Thursday);
    assert_eq!(
        Date::nth_weekday(2024, 11, Weekday::Thursday, 4).unwrap(),
        d(2024, 11, 28)
    );
    assert_eq!(
        Date::last_weekday(2024, 5, Weekday::Monday).unwrap(),
        d(2024, 5, 27)
    );
    assert_eq!(Date::easter_monday(2024).unwrap(), d(2024, 4, 1));
    assert_eq!(
        Date::parse_iso("2024-02-29").unwrap().to_string(),
        "2024-02-29"
    );
    assert_eq!(Date::imm_next(d(2024, 3, 20)).unwrap(), d(2024, 6, 19));
    assert!(Date::parse_iso("2024-02-30").is_err());
}

#[test]
fn calendars_adjust_count_and_advance() {
    let cal = WeekendsOnly;
    let saturday = d(2024, 6, 29);
    assert_eq!(cal.adjust(saturday, Bdc::Following).unwrap(), d(2024, 7, 1));
    assert_eq!(
        cal.adjust(saturday, Bdc::ModifiedFollowing).unwrap(),
        d(2024, 6, 28)
    );
    assert_eq!(
        cal.business_days_between(d(2024, 6, 28), d(2024, 7, 1), false, true),
        1
    );
    assert_eq!(
        cal.advance(
            d(2024, 1, 31),
            Period::new(1, finance_rs::period::TimeUnit::Months),
            Bdc::Unadjusted,
            true
        )
        .unwrap(),
        d(2024, 2, 29)
    );
    assert!(NullCalendar.is_business_day(saturday));
}

#[test]
fn extended_day_counts_match_definitions() {
    let start = d(2024, 1, 15);
    let end = d(2024, 1, 30);
    let dc = DayCount::ActActIcma;
    assert!(
        (dc.year_fraction_ref(start, end, d(2024, 1, 1), d(2024, 7, 1), 2) - 15.0 / (182.0 * 2.0))
            .abs()
            < 1e-15
    );
    assert_eq!(
        DayCount::Act366.year_fraction(d(2024, 1, 1), d(2025, 1, 1)),
        366.0 / 366.0
    );
    assert_eq!(DayCount::Thirty365.days(d(2024, 1, 31), d(2024, 2, 28)), 28);
    assert_eq!(
        DayCount::ActActAfb.year_fraction(d(2023, 1, 1), d(2024, 1, 1)),
        1.0
    );
    #[cfg(feature = "calendars")]
    assert_eq!(
        DayCount::Bus252(finance_rs::calendars::CalendarId::WeekendsOnly)
            .year_fraction(d(2024, 6, 28), d(2024, 7, 1)),
        1.0 / 252.0
    );
}

#[test]
fn schedule_matches_textbook_quarterly_backwards_and_forward() {
    let start = d(2024, 1, 15);
    let end = d(2025, 1, 15);
    let cal = WeekendsOnly;
    let backward = Schedule::builder(start, end)
        .frequency(Frequency::Quarterly)
        .calendar(&cal)
        .convention(Bdc::ModifiedFollowing)
        .rule(DateGeneration::Backward)
        .build()
        .unwrap();
    assert_eq!(
        backward.dates(),
        &[
            d(2024, 1, 15),
            d(2024, 4, 15),
            d(2024, 7, 15),
            d(2024, 10, 15),
            d(2025, 1, 15)
        ]
    );
    assert_eq!(backward.accrual_periods().len(), 4);
    assert_eq!(backward.payment_dates(2, &cal).unwrap().len(), 4);
    let forward = Schedule::builder(start, end)
        .frequency(Frequency::Quarterly)
        .calendar(&cal)
        .rule(DateGeneration::Forward)
        .build()
        .unwrap();
    assert_eq!(forward.dates(), backward.dates());
}

proptest! {
    #[test]
    fn date_arithmetic_never_panics(day in -300_000_000_i64..300_000_000_i64, months in i32::MIN..=i32::MAX, raw in "[ -~]{0,30}") {
        let date=Date::from_ordinal(day);
        if let Ok(d)=date { let _=d.add_months(months); let _=d.add_days(months); let _=d.add_years(months); let _=d.end_of_month(); let _=d.start_of_month(); let _=d.is_imm(); let _=Date::imm_next(d); let _=Date::easter_monday(d.year()); let _=Date::nth_weekday(d.year(),d.month(),d.weekday(),(months%5)+1); }
        let _=Date::parse_iso(&raw);
    }
    #[test]
    fn schedule_bad_inputs_never_panic(year in -1_000_000_i32..=1_000_000_i32, shift in -4000_i32..=4000_i32) {
        if let Ok(start)=Date::new(year,1,1) {
            if let Ok(end)=start.add_days(shift) {
                let cal=WeekendsOnly;
                let _=Schedule::builder(start,end).frequency(Frequency::Monthly).calendar(&cal).build();
            }
        }
    }
}
