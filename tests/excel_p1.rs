//! Excel financial-function P1 identities and documented examples.
#![allow(clippy::unwrap_used)]

use finance_rs::{
    fi::{excel::FixedRateBond, mm},
    rates,
    tax::depreciation::{self as dep, SwitchMode},
    tvm, Date,
};
use proptest::prelude::*;
use serde_json::Value;

#[test]
fn documented_scalar_examples() {
    assert!((tvm::pduration(0.1, 100.0, 200.0).unwrap() - 7.272_540_897_341_714).abs() < 1e-12);
    assert!((tvm::rri(4.0, 10.0, 20.0).unwrap() - 0.189_207_115_002_721_03).abs() < 1e-12);
    assert!((tvm::ispmt(0.1, 1.0, 3.0, 8_000.0).unwrap() + 533.333_333_333_333_3).abs() < 1e-9);
    assert!((rates::dollarde(1.02, 16.0).unwrap() - 1.125).abs() < 1e-12);
    assert!((rates::dollarfr(1.125, 16.0).unwrap() - 1.02).abs() < 1e-12);
    assert!((dep::sln(30_000.0, 7_500.0, 10.0).unwrap() - 2_250.0).abs() < 1e-12);
    assert!((dep::syd(30_000.0, 7_500.0, 10.0, 1.0).unwrap() - 4_090.909_090_909_091).abs() < 1e-9);
}

#[test]
fn money_market_basis_codes_are_supported() {
    let settlement = Date::new(2024, 1, 15).unwrap();
    let maturity = Date::new(2024, 7, 15).unwrap();
    for basis in 0..=4 {
        let price =
            mm::price_discount_instrument(settlement, maturity, 0.04, 100.0, basis).unwrap();
        let discount = mm::discount_rate(settlement, maturity, price, 100.0, basis).unwrap();
        assert!((discount - 0.04).abs() < 1e-14);
        let received = mm::amount_received(settlement, maturity, price, 0.04, basis).unwrap();
        assert!((received - 100.0).abs() < 1e-10);
    }
}

#[test]
fn dated_duration_supports_every_excel_basis() {
    let settlement = Date::new(2024, 1, 15).unwrap();
    let maturity = Date::new(2029, 1, 15).unwrap();
    for basis in 0..=4 {
        let d = FixedRateBond::duration(settlement, maturity, 0.05, 0.04, 2, basis).unwrap();
        let md =
            FixedRateBond::modified_duration(settlement, maturity, 0.05, 0.04, 2, basis).unwrap();
        assert!(d > md && md > 0.0 && d < 5.0);
    }
}

#[test]
fn odd_coupon_prices_and_yields_round_trip() {
    let settlement = Date::new(2008, 11, 11).unwrap();
    let maturity = Date::new(2021, 3, 1).unwrap();
    let issue = Date::new(2008, 10, 15).unwrap();
    let first_coupon = Date::new(2009, 3, 1).unwrap();
    let last_settlement = Date::new(1999, 2, 7).unwrap();
    let last_maturity = Date::new(1999, 6, 15).unwrap();
    let last_interest = Date::new(1998, 10, 15).unwrap();
    let documented_first_yield = FixedRateBond::odd_first_yield(
        settlement,
        maturity,
        issue,
        first_coupon,
        0.0575,
        84.5,
        100.0,
        2,
        0,
    )
    .unwrap();
    assert!((documented_first_yield - 0.0772).abs() < 0.00005);
    for basis in 0..=4 {
        let first_price = FixedRateBond::odd_first_price(
            settlement,
            maturity,
            issue,
            first_coupon,
            0.0575,
            0.0625,
            100.0,
            2,
            basis,
        )
        .unwrap();
        let first_yield = FixedRateBond::odd_first_yield(
            settlement,
            maturity,
            issue,
            first_coupon,
            0.0575,
            first_price,
            100.0,
            2,
            basis,
        )
        .unwrap();
        assert!((first_yield - 0.0625).abs() < 1e-10);

        let last_price = FixedRateBond::odd_last_price(
            last_settlement,
            last_maturity,
            last_interest,
            0.0375,
            0.0405,
            100.0,
            2,
            basis,
        )
        .unwrap();
        let last_yield = FixedRateBond::odd_last_yield(
            last_settlement,
            last_maturity,
            last_interest,
            0.0375,
            last_price,
            100.0,
            2,
            basis,
        )
        .unwrap();
        assert!((last_yield - 0.0405).abs() < 1e-10);
    }
}

#[test]
#[allow(clippy::too_many_lines)] // This single scenario verifies the full LibreOffice recalculation golden.
fn libreoffice_calc_full_recalc_golden() {
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("golden/excel_p1_calc.json")).unwrap();
    let settlement = Date::new(2024, 1, 15).unwrap();
    let maturity = Date::new(2024, 7, 15).unwrap();
    let bond_maturity = Date::new(2029, 1, 15).unwrap();
    let issue = Date::new(2023, 7, 15).unwrap();
    let odd_issue = Date::new(2008, 10, 15).unwrap();
    let odd_settlement = Date::new(2008, 11, 11).unwrap();
    let odd_maturity = Date::new(2021, 3, 1).unwrap();
    let first_coupon = Date::new(2009, 3, 1).unwrap();
    let last_settlement = Date::new(1999, 2, 7).unwrap();
    let last_maturity = Date::new(1999, 6, 15).unwrap();
    let last_interest = Date::new(1998, 10, 15).unwrap();
    for row in &cases {
        let function = row["function"].as_str().unwrap();
        let expected = row["value"].as_f64().unwrap();
        let basis = u8::try_from(row["basis"].as_u64().unwrap_or(0)).unwrap_or_default();
        let actual = match function {
            "ISPMT" => tvm::ispmt(0.1, 1.0, 3.0, 8_000.0).unwrap(),
            "DOLLARDE" => rates::dollarde(1.02, 16.0).unwrap(),
            "DOLLARFR" => rates::dollarfr(1.125, 16.0).unwrap(),
            "PDURATION" => tvm::pduration(0.1, 100.0, 200.0).unwrap(),
            "RRI" => tvm::rri(4.0, 10.0, 20.0).unwrap(),
            "DISC" => mm::discount_rate(settlement, maturity, 95.0, 100.0, basis).unwrap(),
            "INTRATE" => mm::interest_rate(settlement, maturity, 95.0, 100.0, basis).unwrap(),
            "RECEIVED" => mm::amount_received(settlement, maturity, 95.0, 0.05, basis).unwrap(),
            "PRICEDISC" => {
                mm::price_discount_instrument(settlement, maturity, 0.05, 100.0, basis).unwrap()
            }
            "PRICEMAT" => {
                mm::price_maturity_instrument(settlement, maturity, issue, 0.05, 0.04, basis)
                    .unwrap()
            }
            "YIELDDISC" => {
                mm::yield_discount_instrument(settlement, maturity, 95.0, 100.0, basis).unwrap()
            }
            "YIELDMAT" => {
                mm::yield_maturity_instrument(settlement, maturity, issue, 0.05, 96.0, basis)
                    .unwrap()
            }
            "ACCRINTM" => mm::accrued_at_maturity(issue, settlement, 0.05, 100.0, basis).unwrap(),
            "DURATION" => {
                FixedRateBond::duration(settlement, bond_maturity, 0.05, 0.04, 2, basis).unwrap()
            }
            "MDURATION" => {
                FixedRateBond::modified_duration(settlement, bond_maturity, 0.05, 0.04, 2, basis)
                    .unwrap()
            }
            "ODDFPRICE" => FixedRateBond::odd_first_price(
                odd_settlement,
                odd_maturity,
                odd_issue,
                first_coupon,
                0.0575,
                0.0625,
                100.0,
                2,
                basis,
            )
            .unwrap(),
            "ODDFYIELD" => FixedRateBond::odd_first_yield(
                odd_settlement,
                odd_maturity,
                odd_issue,
                first_coupon,
                0.0575,
                84.5,
                100.0,
                2,
                basis,
            )
            .unwrap(),
            "ODDLPRICE" => FixedRateBond::odd_last_price(
                last_settlement,
                last_maturity,
                last_interest,
                0.0375,
                0.0405,
                100.0,
                2,
                basis,
            )
            .unwrap(),
            "ODDLYIELD" => FixedRateBond::odd_last_yield(
                Date::new(1999, 4, 20).unwrap(),
                last_maturity,
                last_interest,
                0.0375,
                99.875,
                100.0,
                2,
                basis,
            )
            .unwrap(),
            "DB" => dep::db(30_000.0, 7_500.0, 10.0, 1.0, 12.0).unwrap(),
            "DDB" => dep::ddb(2_400.0, 300.0, 10.0, 1.0, 2.0).unwrap(),
            "SLN" => dep::sln(30_000.0, 7_500.0, 10.0).unwrap(),
            "SYD" => dep::syd(30_000.0, 7_500.0, 10.0, 1.0).unwrap(),
            "VDB" => dep::vdb(2_400.0, 300.0, 10.0, 0.0, 1.0, 2.0, SwitchMode::Allow).unwrap(),
            other => panic!("unhandled LibreOffice case {other}"),
        };
        assert!(
            (actual - expected).abs() <= 1e-10 * expected.abs().max(1.0),
            "{function} basis {basis}: {actual} != {expected}"
        );
    }
}

proptest! {
    #[test]
    fn scalar_identities(n in 1.0..100.0_f64, pv in 0.1..1e6_f64, rate in 0.001..0.5_f64) {
        let fv = pv * (1.0 + rate).powf(n);
        prop_assert!((tvm::rri(n, pv, fv).unwrap() - rate).abs() < 1e-10 * (1.0 + rate));
        prop_assert!((tvm::pduration(rate, pv, fv).unwrap() - n).abs() < 1e-9 * (1.0 + n));
    }

    #[test]
    fn depreciation_never_crosses_salvage(periods in 1u32..60, cost in 100.0..1e7_f64, fraction in 0.0..0.9_f64) {
        let life = f64::from(periods);
        let salvage = cost * fraction;
        let mut balance = cost;
        for period in 1..=periods {
            let d = dep::ddb(cost, salvage, life, f64::from(period), 2.0).unwrap();
            prop_assert!(d >= 0.0 && d <= balance - salvage + 1e-8 * cost);
            balance -= d;
        }
        prop_assert!(balance >= salvage - 1e-8 * cost);
    }

    #[test]
    fn new_excel_functions_never_panic(a in any::<f64>(), b in any::<f64>(), c in any::<f64>(), d in any::<f64>(), e in any::<f64>()) {
        let _ = tvm::pduration(a, b, c);
        let _ = tvm::rri(a, b, c);
        let _ = tvm::ispmt(a, b, c, d);
        let _ = rates::dollarde(a, b);
        let _ = rates::dollarfr(a, b);
        let _ = dep::db(a, b, c, d, e);
        let _ = dep::ddb(a, b, c, d, e);
        let _ = dep::sln(a, b, c);
        let _ = dep::syd(a, b, c, d);
        let _ = dep::vdb(a, b, c, d, e, 2.0, SwitchMode::Allow);
        let settlement = Date::new(2024, 1, 15).unwrap();
        let maturity = Date::new(2024, 7, 15).unwrap();
        let issue = Date::new(2023, 7, 15).unwrap();
        for basis in [0, 1, 2, 3, 4, 5, 255] {
            let _ = mm::discount_rate(settlement, maturity, a, b, basis);
            let _ = mm::interest_rate(settlement, maturity, a, b, basis);
            let _ = mm::amount_received(settlement, maturity, a, b, basis);
            let _ = mm::price_discount_instrument(settlement, maturity, a, b, basis);
            let _ = mm::price_maturity_instrument(settlement, maturity, issue, a, b, basis);
            let _ = mm::yield_discount_instrument(settlement, maturity, a, b, basis);
            let _ = mm::yield_maturity_instrument(settlement, maturity, issue, a, b, basis);
            let _ = mm::accrued_at_maturity(issue, settlement, a, b, basis);
            let _ = FixedRateBond::duration(settlement, maturity, a, b, 2, basis);
            let _ = FixedRateBond::modified_duration(settlement, maturity, a, b, 2, basis);
        }
    }
}
