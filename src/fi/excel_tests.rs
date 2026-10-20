use super::*;

fn d(y: i32, m: u32, day: u32) -> Date {
    Date::new(y, m, day).unwrap()
}

#[test]
fn dated_bond_duration_and_odd_coupon_price_yields() {
    let settlement = d(2024, 3, 1);
    let maturity = d(2026, 3, 1);
    let (previous, next, schedule) = coupon_dates(settlement, maturity, 2).unwrap();
    assert!(previous <= settlement && settlement < next);
    assert_eq!(schedule.last(), Some(&maturity));
    assert!(validate_odd_inputs(settlement, maturity, d(2023, 12, 1), 0.05, 0.04, 2, 1).is_ok());
    for basis in 0..=4 {
        assert!(FixedRateBond::duration(settlement, maturity, 0.05, 0.04, 2, basis).unwrap() > 0.0);
    }
    assert!(
        FixedRateBond::modified_duration(settlement, maturity, 0.05, 0.04, 2, 3).unwrap() > 0.0
    );
    let issue = d(2023, 12, 1);
    let first_coupon = d(2024, 6, 1);
    let first_price = FixedRateBond::odd_first_price(
        settlement,
        maturity,
        issue,
        first_coupon,
        0.05,
        0.04,
        100.0,
        2,
        1,
    )
    .unwrap();
    let first_yield = FixedRateBond::odd_first_yield(
        settlement,
        maturity,
        issue,
        first_coupon,
        0.05,
        first_price,
        100.0,
        2,
        1,
    )
    .unwrap();
    assert!((first_yield - 0.04).abs() < 1e-8);
    let last_interest = d(2023, 12, 1);
    let last_price =
        FixedRateBond::odd_last_price(settlement, maturity, last_interest, 0.05, 0.04, 100.0, 2, 1)
            .unwrap();
    let last_yield = FixedRateBond::odd_last_yield(
        settlement,
        maturity,
        last_interest,
        0.05,
        last_price,
        100.0,
        2,
        1,
    )
    .unwrap();
    assert!((last_yield - 0.04).abs() < 1e-8);
}

#[test]
fn dated_bond_rejects_invalid_dates_and_yield_cases() {
    let s = d(2024, 3, 1);
    let m = d(2025, 3, 1);
    assert!(basis_day_count(5).is_err());
    assert_eq!(day_count_as_f64(DayCount::Act365F, s, m), 365.0);
    assert!(coupon_dates(m, s, 2).is_err());
    assert!(coupon_dates(s, m, 3).is_err());
    assert!(coupon_dates(d(-20_000, 1, 1), d(20_000, 1, 1), 4).is_err());
    assert!(FixedRateBond::duration(s, m, -0.1, 0.05, 2, 0).is_err());
    assert!(FixedRateBond::duration(s, m, 0.05, -2.0, 2, 0).is_err());
    assert!(FixedRateBond::duration(s, m, 0.05, 0.05, 2, 5).is_err());
    let mut unused_price_callback = |_| Ok(1.0);
    assert!(unused_price_callback(0.0).is_ok());
    assert!(solve_odd_yield(0.0, &mut unused_price_callback).is_err());
    assert!(solve_odd_yield(1.0, |_| {
        Err(FinanceError::InvalidInput {
            what: "test callback",
            reason: "propagated",
        })
    })
    .is_err());
    assert!(solve_odd_yield(10.0, |_| Ok(20.0)).is_err());
    let approximate = solve_odd_yield(1.0, |yield_rate| {
        Ok(if yield_rate < 0.5 { 0.0 } else { 2.0 })
    })
    .unwrap();
    assert!((approximate - 0.5).abs() < 1e-12);
    let issue = d(2023, 12, 1);
    let first = d(2024, 6, 1);
    assert!(FixedRateBond::odd_first_price(s, m, s, first, 0.05, 0.04, 100.0, 2, 1).is_err());
    assert!(FixedRateBond::odd_first_price(s, m, issue, s, 0.05, 0.04, 100.0, 2, 1).is_err());
    assert!(FixedRateBond::odd_first_price(s, m, issue, first, 0.05, 0.04, 100.0, 3, 1).is_err());
    assert!(FixedRateBond::odd_first_price(
        d(2024, 1, 30),
        m,
        d(2024, 1, 29),
        d(2024, 1, 31),
        0.05,
        0.04,
        100.0,
        1,
        0
    )
    .is_err());
    assert!(FixedRateBond::odd_first_price(
        d(0, 1, 1),
        d(30_000, 1, 1),
        d(-1, 1, 1),
        d(1, 1, 1),
        0.05,
        0.04,
        100.0,
        4,
        1
    )
    .is_err());
    let last = d(2023, 12, 1);
    assert!(FixedRateBond::odd_last_price(s, m, m, 0.05, 0.04, 100.0, 2, 1).is_err());
    assert!(FixedRateBond::odd_last_price(s, m, s, 0.05, 0.04, 100.0, 2, 1).is_err());
    assert!(FixedRateBond::odd_last_price(s, m, last, 0.05, 0.04, 100.0, 2, 5).is_err());
    assert!(FixedRateBond::odd_first_yield(s, m, issue, first, 0.05, -1.0, 100.0, 2, 1).is_err());
    assert!(FixedRateBond::odd_last_yield(s, m, last, 0.05, 0.0, 100.0, 2, 1).is_err());
}
