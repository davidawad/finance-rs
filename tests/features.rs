//! Optional-feature integration: chrono dates and serde round trips.
#![allow(clippy::unwrap_used)]

#[cfg(feature = "chrono")]
#[test]
fn xirr_accepts_naive_dates() {
    use chrono::NaiveDate;
    use finance_rs::{cashflow, Date};
    let d = |y, m, day| NaiveDate::from_ymd_opt(y, m, day).unwrap();
    let dates = [d(2020, 1, 1), d(2020, 3, 1), d(2021, 1, 1)];
    let amounts = [-100.0, 50.0, 60.0];
    let ours: Vec<Date> = dates.iter().map(|x| Date::from(*x)).collect();
    let a = cashflow::xirr(&dates, &amounts).unwrap();
    let b = cashflow::xirr(&ours, &amounts).unwrap();
    assert_eq!(a, b);
    assert!((a - 0.169_725_639_989_622_37).abs() < 1e-9);
    assert_eq!(NaiveDate::try_from(ours[1]).unwrap(), dates[1]);
}

#[cfg(feature = "serde")]
#[test]
fn types_round_trip_through_serde() {
    use finance_rs::cashflow::XirrOptions;
    use finance_rs::daycount::DayCount;
    use finance_rs::{Date, FinanceError};
    let opts = XirrOptions {
        guess: 0.2,
        day_count: DayCount::ActActIsda,
    };
    let text = serde_json::to_string(&opts).unwrap();
    assert_eq!(serde_json::from_str::<XirrOptions>(&text).unwrap(), opts);
    let date = Date::new(2024, 2, 29).unwrap();
    let text = serde_json::to_string(&date).unwrap();
    assert_eq!(serde_json::from_str::<Date>(&text).unwrap(), date);
    assert!(
        serde_json::from_str::<Date>(r#"{"year":2024,"month":2,"day":29,"unexpected":true}"#)
            .is_err()
    );
    let err = FinanceError::NoConvergence {
        what: "rate",
        iterations: 3,
    };
    assert!(serde_json::to_string(&err)
        .unwrap()
        .contains("NoConvergence"));
}

#[cfg(feature = "serde")]
#[test]
fn core_types_round_trip_through_serde() {
    use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    use finance_rs::daycount::DayCount;
    use finance_rs::period::{Period, TimeUnit};
    use finance_rs::rates::{Compounding, Frequency, Rate};
    use finance_rs::Date;
    let rate = Rate::new(
        0.05,
        Compounding::Compounded(Frequency::PerYear(3)),
        DayCount::Act360,
    );
    let text = serde_json::to_string(&rate).unwrap();
    assert_eq!(serde_json::from_str::<Rate>(&text).unwrap(), rate);
    let p = Period::new(-3, TimeUnit::Months);
    assert_eq!(
        serde_json::from_str::<Period>(&serde_json::to_string(&p).unwrap()).unwrap(),
        p
    );
    let d = |y| Date::new(y, 1, 1).unwrap();
    let s = CashFlowStream::new([
        CashFlow::new(d(2021), 5.0, FlowKind::Interest),
        CashFlow::new(d(2020), -5.0, FlowKind::Contribution),
    ])
    .unwrap();
    let text = serde_json::to_string(&s).unwrap();
    assert_eq!(serde_json::from_str::<CashFlowStream>(&text).unwrap(), s);
    // Deserializing re-establishes the date order.
    let reversed = serde_json::to_string(&s.flows().iter().rev().collect::<Vec<_>>()).unwrap();
    assert_eq!(
        serde_json::from_str::<CashFlowStream>(&reversed).unwrap(),
        s
    );
}
