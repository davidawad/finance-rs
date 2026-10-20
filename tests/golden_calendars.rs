//! Calendar and `MakeSchedule` parity against `QuantLib` 1.44.
#![cfg(feature = "calendars")]
#![allow(clippy::unwrap_used, clippy::panic)]
mod common;
use common::Golden;
use finance_rs::calendars::Target;
use finance_rs::calendars::{Calendar, CalendarId, CalendarRef};
use finance_rs::rates::Frequency;
use finance_rs::schedule::{DateGeneration, Schedule};
use finance_rs::Date;

fn date(s: &str) -> Date {
    Date::parse_iso(s).unwrap()
}

fn calendar(name: &str) -> CalendarId {
    match name {
        "UnitedStatesSettlement" => CalendarId::UnitedStatesSettlement,
        "Nyse" => CalendarId::Nyse,
        "GovernmentBond" => CalendarId::GovernmentBond,
        "FederalReserve" => CalendarId::FederalReserve,
        "Target" => CalendarId::Target,
        "UnitedKingdomExchange" => CalendarId::UnitedKingdomExchange,
        "Japan" => CalendarId::Japan,
        "Canada" => CalendarId::Canada,
        _ => panic!("unknown calendar {name}"),
    }
}

#[test]
fn market_holiday_samples_match_quantlib() {
    let golden = Golden::load("calendars");
    for c in golden.cases("holiday") {
        let name = c["args"]["calendar"].as_str().unwrap();
        let d = date(c["args"]["date"].as_str().unwrap());
        let calendar = CalendarRef(calendar(name));
        assert_eq!(
            calendar.is_holiday(d),
            c["expected"].as_bool().unwrap(),
            "{name}, {d}"
        );
    }
}

#[test]
fn nyse_historical_closures_match_quantlib_source() {
    let nyse = CalendarRef(CalendarId::Nyse);
    for value in [
        "2025-01-09",
        "2018-12-05",
        "2012-10-29",
        "2012-10-30",
        "2007-01-02",
        "2004-06-11",
        "2001-09-11",
        "2001-09-12",
        "2001-09-13",
        "2001-09-14",
        "1994-04-27",
        "1985-09-27",
        "1980-11-04",
        "1977-07-14",
        "1976-11-02",
        "1973-01-25",
        "1972-12-28",
        "1972-11-07",
        "1969-07-21",
        "1969-03-31",
        "1969-02-10",
        "1968-07-05",
        "1968-04-09",
        "1968-11-05",
        "1968-06-12",
        "1968-06-19",
        "1968-06-26",
        "1968-07-03",
        "1968-07-10",
        "1968-07-17",
        "1968-11-06",
        "1968-11-20",
        "1968-11-27",
        "1968-12-04",
        "1968-12-11",
        "1968-12-18",
        "1964-11-03",
        "1963-11-25",
        "1961-05-29",
        "1958-12-26",
        "1956-11-06",
        "1956-12-24",
        "1954-11-02",
        "1954-12-24",
        "1952-11-04",
    ] {
        assert!(nyse.is_holiday(date(value)), "NYSE {value}");
    }
    assert!(
        !nyse.is_holiday(date("1960-11-08")),
        "QuantLib leaves this date open"
    );
}

#[test]
fn target_and_uk_exchange_historical_closures_match_quantlib_source() {
    for value in ["1998-12-31", "1999-12-31", "2001-12-31"] {
        assert!(Target.is_holiday(date(value)), "TARGET {value}");
    }
    assert!(finance_rs::calendars::UnitedKingdom::Exchange.is_holiday(date("1999-12-31")));
}

#[test]
fn make_schedule_dates_match_quantlib() {
    let golden = Golden::load("calendars");
    for c in golden.cases("schedule") {
        let frequency = match c["args"]["frequency"].as_str().unwrap() {
            "Monthly" => Frequency::Monthly,
            "Quarterly" => Frequency::Quarterly,
            _ => unreachable!(),
        };
        let rule = match c["args"]["rule"].as_str().unwrap() {
            "Backward" => DateGeneration::Backward,
            "Forward" => DateGeneration::Forward,
            _ => unreachable!(),
        };
        let calendar = Target;
        let schedule = Schedule::builder(
            date(c["args"]["effective"].as_str().unwrap()),
            date(c["args"]["termination"].as_str().unwrap()),
        )
        .frequency(frequency)
        .calendar(&calendar)
        .convention(finance_rs::calendars::BusinessDayConvention::ModifiedFollowing)
        .termination_convention(finance_rs::calendars::BusinessDayConvention::ModifiedFollowing)
        .rule(rule)
        .end_of_month(c["args"]["eom"].as_bool().unwrap())
        .build()
        .unwrap();
        let actual: Vec<_> = schedule
            .adjusted()
            .iter()
            .map(ToString::to_string)
            .collect();
        let expected: Vec<_> = c["expected"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_owned())
            .collect();
        assert_eq!(actual, expected, "{c}");
    }
}

#[allow(clippy::cast_possible_truncation)] // Fixture frequency values are the supported small calendar periods.
#[test]
fn actual_actual_variants_match_quantlib() {
    let golden = Golden::load("calendars");
    for c in golden.cases("icma") {
        let value = finance_rs::daycount::DayCount::ActActIcma.year_fraction_ref(
            date(c["args"]["start"].as_str().unwrap()),
            date(c["args"]["end"].as_str().unwrap()),
            date(c["args"]["ref_start"].as_str().unwrap()),
            date(c["args"]["ref_end"].as_str().unwrap()),
            c["args"]["frequency"].as_u64().unwrap() as u16,
        );
        assert!(
            (value - c["expected"].as_f64().unwrap()).abs() <= 1e-14,
            "{c}"
        );
    }
    for c in golden.cases("afb") {
        let value = finance_rs::daycount::DayCount::ActActAfb.year_fraction(
            date(c["args"]["start"].as_str().unwrap()),
            date(c["args"]["end"].as_str().unwrap()),
        );
        assert!(
            (value - c["expected"].as_f64().unwrap()).abs() <= 1e-14,
            "{c}"
        );
    }
}
