use finance_rs::cds::{
    basis, carry_and_roll, implied_spread, next_imm_date, running_from_upfront,
    standard_coupon_amount, upfront_from_running, CdsInstrument, CdsKind, CreditCurve,
    ParSpreadQuote, StandardCoupon,
};
use finance_rs::{Date, FinanceError};
use proptest::prelude::*;

#[allow(clippy::expect_used)] // Generated schedule periods stay in range by the preceding validated schedule bounds.
fn date(y: i32, m: u32, d: u32) -> Date {
    Date::new(y, m, d).expect("valid test date")
}

#[allow(clippy::expect_used)] // Generated schedule periods stay in range by the preceding validated schedule bounds.
fn sample() -> (CdsInstrument, CreditCurve) {
    let effective = date(2024, 3, 20);
    let maturity = date(2029, 3, 20);
    let curve = CreditCurve::bootstrap(
        effective,
        &[ParSpreadQuote {
            maturity,
            spread: 0.01,
        }],
        0.03,
        0.4,
    )
    .expect("bootstrap sample curve");
    let instrument = CdsInstrument::new(
        CdsKind::SingleName {
            reference_entity: "Example Corp".into(),
        },
        effective,
        maturity,
        10_000_000.0,
        0.01,
        0.4,
        0.0,
    )
    .expect("valid sample CDS");
    (instrument, curve)
}

#[test]
fn isda_standard_examples_coupon_and_imm_dates() {
    // Published values from ISDA's Standard CDS Examples (updated Oct 2012)
    // and Standard North American Contract Test Cases (Mar 18, 2009),
    // https://www.cdsmodel.com/archive.html. ISDA's disclaimer says calculated values
    // are not source code or an executable derivative of the model. Only the
    // numeric outputs are reproduced here; source code and document text are
    // not copied. License/disclaimer: https://www.cdsmodel.com/cds-disclaimer.html.
    let coupon = standard_coupon_amount(StandardCoupon::InvestmentGrade, 10_000_000.0, 94).unwrap();
    assert!((coupon - 26_111.111_111).abs() < 0.001);
    assert!(
        (standard_coupon_amount(StandardCoupon::InvestmentGrade, 10_000_000.0, 87).unwrap()
            - 24_166.666_667)
            .abs()
            < 0.001
    );
    assert!(
        (standard_coupon_amount(StandardCoupon::InvestmentGrade, 10_000_000.0, 1).unwrap()
            - 277.777_778)
            .abs()
            < 0.001
    );
    assert!(
        (standard_coupon_amount(StandardCoupon::InvestmentGrade, 1_000_000_000.0, 18).unwrap()
            - 500_000.0)
            .abs()
            < 0.001
    );
    assert!(
        (standard_coupon_amount(StandardCoupon::HighYield, 1_000_000_000.0, 18).unwrap()
            - 2_500_000.0)
            .abs()
            < 0.001
    );
    assert_eq!(next_imm_date(date(2024, 3, 20)).unwrap(), date(2024, 6, 20));
    assert_eq!(
        next_imm_date(date(2024, 12, 21)).unwrap(),
        date(2025, 3, 20)
    );
    assert_eq!(StandardCoupon::HighYield.rate(), 0.05);
}

#[test]
fn flat_and_term_hazard_values_match_quantlib_isda_legs() {
    // Reference calculations use the ISDA Standard CDS conventions described
    // in the public model materials: ACT/360 premiums, ACT/365 hazard and
    // discount time, quarterly IMM-20 dates, and accrued premium on default.
    // QuantLib 1.44 IsdaCdsEngine cases use the same ACT/360 coupons,
    // ACT/365 curves, and following-adjusted payment dates.
    let effective = date(2024, 3, 20);
    let maturity = date(2029, 3, 20);
    let flat_curve = CreditCurve {
        valuation_date: effective,
        nodes: vec![finance_rs::cds::HazardNode {
            maturity,
            hazard_rate: 0.02,
        }],
    };
    let instrument = CdsInstrument::new(
        CdsKind::SingleName {
            reference_entity: "Reference case".into(),
        },
        effective,
        maturity,
        1.0,
        0.01,
        0.4,
        0.0,
    )
    .unwrap();
    let flat = instrument.value(&flat_curve, 0.03).unwrap();
    assert!((flat.protection_leg - 0.053_113_414_718_522_7).abs() < 1e-13);
    assert!((flat.risky_annuity - 4.470_952_912_972_72).abs() < 1e-12);
    assert!((flat.par_spread - 0.011_887_045_503_937_403).abs() < 1e-13);

    let term_curve = CreditCurve {
        valuation_date: effective,
        nodes: vec![
            finance_rs::cds::HazardNode {
                maturity: date(2025, 3, 20),
                hazard_rate: 0.005,
            },
            finance_rs::cds::HazardNode {
                maturity,
                hazard_rate: 0.03,
            },
        ],
    };
    let term = instrument.value(&term_curve, 0.03).unwrap();
    assert!((term.protection_leg - 0.064_795_550_360_384_69).abs() < 1e-13);
    assert!((term.risky_annuity - 4.463_516_058_819_203).abs() < 1e-12);
    assert!((term.par_spread - 0.014_525_742_085_627_71).abs() < 1e-13);
}

#[test]
fn bootstrap_recovers_quotes_and_upfront_converts_round_trip() {
    let (instrument, curve) = sample();
    let value = instrument.value(&curve, 0.03).unwrap();
    assert!((value.par_spread - 0.01).abs() < 1e-10);
    assert!(value.risky_annuity > 0.0);
    let upfront = upfront_from_running(&instrument, &curve, 0.03, 0.015).unwrap();
    let recovered = running_from_upfront(&instrument, &curve, 0.03, upfront).unwrap();
    assert!((recovered - 0.015).abs() < 1e-12);
}

#[test]
fn zero_spread_and_discount_rate_keep_zero_hazard_pricing_finite() {
    let effective = date(2024, 3, 20);
    let maturity = date(2025, 3, 20);
    let curve = CreditCurve::bootstrap(
        effective,
        &[ParSpreadQuote {
            maturity,
            spread: 0.0,
        }],
        0.0,
        0.4,
    )
    .unwrap();
    let instrument = CdsInstrument::new(
        CdsKind::SingleName {
            reference_entity: "Zero Risk Corp".into(),
        },
        effective,
        maturity,
        1_000_000.0,
        0.0,
        0.4,
        0.0,
    )
    .unwrap();
    let value = instrument.value(&curve, 0.0).unwrap();
    assert!(value.protection_leg.abs() < 1e-12);
    assert!(value.risky_annuity > 0.0);
}

#[test]
fn multi_tenor_curve_uses_each_hazard_interval() {
    let effective = date(2024, 3, 20);
    let curve = CreditCurve::bootstrap(
        effective,
        &[
            ParSpreadQuote {
                maturity: date(2025, 3, 20),
                spread: 0.005,
            },
            ParSpreadQuote {
                maturity: date(2029, 3, 20),
                spread: 0.02,
            },
        ],
        0.02,
        0.4,
    )
    .unwrap();
    assert!((curve.nodes[0].hazard_rate - 0.008_404_792_492_845_62).abs() < 1e-12);
    assert!((curve.nodes[1].hazard_rate - 0.040_916_565_623_678_305).abs() < 1e-12);
    assert!(curve.hazard_at(date(2025, 3, 20)) < curve.hazard_at(date(2026, 3, 20)));
    assert!(
        curve.survival_probability(date(2025, 3, 20))
            > curve.survival_probability(date(2029, 3, 20))
    );
}

#[test]
fn empty_curve_has_zero_hazard_and_unit_survival() {
    let curve = CreditCurve {
        valuation_date: date(2024, 3, 20),
        nodes: Vec::new(),
    };
    assert_eq!(curve.hazard_at(date(2026, 3, 20)), 0.0);
    assert_eq!(curve.survival_probability(date(2026, 3, 20)), 1.0);
    assert_eq!(curve.default_probability(date(2026, 3, 20)), 0.0);
}

#[test]
fn pricing_returns_date_range_errors_from_payment_schedule() {
    let effective = date(1_000_000, 12, 1);
    let maturity = date(1_000_000, 12, 30);
    let instrument = CdsInstrument::new(
        CdsKind::SingleName {
            reference_entity: "Boundary Corp".into(),
        },
        effective,
        maturity,
        1_000_000.0,
        0.01,
        0.4,
        0.0,
    )
    .unwrap();
    let curve = CreditCurve {
        valuation_date: effective,
        nodes: vec![finance_rs::cds::HazardNode {
            maturity,
            hazard_rate: 0.02,
        }],
    };
    assert!(instrument.value(&curve, 0.03).is_err());
    assert!(implied_spread(&curve, maturity, 0.03, 0.4).is_err());
}

#[test]
fn index_contract_and_basis_are_typed_and_signed() {
    let instrument = CdsInstrument::new(
        CdsKind::Index {
            index: "CDX.NA.IG".into(),
            series: 42,
            version: 1,
        },
        date(2024, 3, 20),
        date(2029, 3, 20),
        5_000_000.0,
        0.01,
        0.4,
        12_000.0,
    )
    .unwrap();
    instrument.validate().unwrap();
    assert!((basis(0.012, 0.015).unwrap() + 0.003).abs() < 1e-15);
    assert!(matches!(
        CdsInstrument::new(
            CdsKind::SingleName {
                reference_entity: "x".into()
            },
            date(2024, 3, 20),
            date(2029, 3, 20),
            1.0,
            0.01,
            1.0,
            0.0,
        ),
        Err(FinanceError::InvalidInput { .. })
    ));
}

#[test]
fn curve_survival_risk_carry_and_roll_are_available() {
    let (instrument, curve) = sample();
    assert!(curve.hazard_at(date(2025, 3, 20)) > 0.0);
    assert!(curve.survival_probability(date(2026, 3, 20)) < 1.0);
    assert!(curve.survival_probability(date(2026, 3, 20)) > 0.0);
    assert!(curve.default_probability(date(2026, 3, 20)) > 0.0);
    assert!(curve.hazard_at(date(2031, 3, 20)) > 0.0);
    assert!(curve.survival_probability(date(2031, 3, 20)) > 0.0);
    assert!(curve.default_probability(date(2031, 3, 20)) > 0.0);
    assert_eq!(curve.survival_probability(date(2024, 3, 19)), 1.0);
    assert_eq!(curve.default_probability(date(2024, 3, 19)), 0.0);
    assert!((implied_spread(&curve, instrument.maturity, 0.03, 0.4).unwrap() - 0.01).abs() < 1e-10);
    let risk = instrument.risk(&curve, 0.03).unwrap();
    assert!(risk.present_value.is_finite());
    assert!(
        risk.cs01.is_finite() && risk.dv01.is_finite() && risk.recovery_sensitivity.is_finite()
    );
    let capped_recovery = CdsInstrument {
        recovery: 0.995,
        ..instrument.clone()
    };
    assert!(capped_recovery
        .risk(&curve, 0.03)
        .unwrap()
        .recovery_sensitivity
        .is_finite());
    let (carry, roll) = carry_and_roll(&instrument, &curve, 0.03, 30).unwrap();
    assert!(carry < 0.0);
    assert!(roll.is_finite());
    assert_eq!(StandardCoupon::InvestmentGrade.basis_points(), 100);
    assert_eq!(StandardCoupon::HighYield.basis_points(), 500);
}

#[test]
fn invalid_terms_and_curve_inputs_return_errors() {
    let effective = date(2024, 3, 20);
    let maturity = date(2029, 3, 20);
    let single = CdsKind::SingleName {
        reference_entity: "x".into(),
    };
    for (notional, coupon, recovery, upfront) in [
        (0.0, 0.01, 0.4, 0.0),
        (1.0, -0.01, 0.4, 0.0),
        (1.0, 0.01, 1.0, 0.0),
        (1.0, 0.01, 0.4, f64::NAN),
    ] {
        assert!(CdsInstrument::new(
            single.clone(),
            effective,
            maturity,
            notional,
            coupon,
            recovery,
            upfront
        )
        .is_err());
    }
    assert!(CdsInstrument::new(single.clone(), effective, effective, 1.0, 0.01, 0.4, 0.0).is_err());
    assert!(CreditCurve::bootstrap(effective, &[], 0.03, 0.4).is_err());
    assert!(CreditCurve::bootstrap(
        effective,
        &[ParSpreadQuote {
            maturity,
            spread: 0.01
        }],
        f64::NAN,
        0.4
    )
    .is_err());
    assert!(CreditCurve::bootstrap(
        effective,
        &[ParSpreadQuote {
            maturity,
            spread: 0.01
        }],
        0.03,
        1.0
    )
    .is_err());
    assert!(CreditCurve::bootstrap(
        effective,
        &[ParSpreadQuote {
            maturity,
            spread: -0.01
        }],
        0.03,
        0.4
    )
    .is_err());
    assert!(CreditCurve::bootstrap(
        effective,
        &[ParSpreadQuote {
            maturity,
            spread: f64::NAN
        }],
        0.03,
        0.4,
    )
    .is_err());
    assert!(CreditCurve::bootstrap(
        effective,
        &[ParSpreadQuote {
            maturity,
            spread: 0.01
        }],
        0.03,
        f64::NAN,
    )
    .is_err());
    assert!(CreditCurve::bootstrap(
        effective,
        &[ParSpreadQuote {
            maturity: effective,
            spread: 0.01
        }],
        0.03,
        0.4
    )
    .is_err());
    assert!(CreditCurve::bootstrap(
        effective,
        &[ParSpreadQuote {
            maturity,
            spread: 10.0
        }],
        0.03,
        0.4
    )
    .is_err());
    assert!(standard_coupon_amount(StandardCoupon::InvestmentGrade, f64::INFINITY, 94).is_err());
    assert!(standard_coupon_amount(StandardCoupon::InvestmentGrade, 0.0, 94).is_err());
    assert!(standard_coupon_amount(StandardCoupon::HighYield, f64::MAX, u32::MAX).is_err());
    assert!(basis(f64::INFINITY, 0.01).is_err());
}

#[test]
fn reference_identity_requires_names_and_positive_index_series_and_version() {
    let effective = date(2024, 3, 20);
    let maturity = date(2029, 3, 20);
    let terms = |kind| CdsInstrument::new(kind, effective, maturity, 1_000_000.0, 0.01, 0.4, 0.0);

    assert!(terms(CdsKind::SingleName {
        reference_entity: "Example Corp".into(),
    })
    .is_ok());
    assert!(terms(CdsKind::SingleName {
        reference_entity: " ".into(),
    })
    .is_err());

    for (index, series, version) in [
        ("CDX.NA.IG", 42, 1),
        (" ", 42, 1),
        ("CDX.NA.IG", 0, 1),
        ("CDX.NA.IG", 42, 0),
    ] {
        let result = terms(CdsKind::Index {
            index: index.into(),
            series,
            version,
        });
        assert_eq!(
            result.is_ok(),
            index == "CDX.NA.IG" && series == 42 && version == 1
        );
    }
}

#[test]
fn pricing_error_paths_are_typed() {
    let (instrument, curve) = sample();
    assert!(instrument.value(&curve, f64::NAN).is_err());
    assert!(instrument.value(&curve, -1.0).is_err());
    let wrong_date_curve = CreditCurve::bootstrap(
        date(2024, 3, 21),
        &[ParSpreadQuote {
            maturity: instrument.maturity,
            spread: 0.01,
        }],
        0.03,
        0.4,
    )
    .unwrap();
    assert!(instrument.value(&wrong_date_curve, 0.03).is_err());
    assert!(instrument.risk(&curve, -1.0).is_err());
    assert!(instrument.risk(&curve, f64::NAN).is_err());
    assert!(upfront_from_running(&instrument, &curve, 0.03, f64::NAN).is_err());
    assert!(upfront_from_running(&instrument, &curve, f64::NAN, 0.015).is_err());
    assert!(running_from_upfront(&instrument, &curve, 0.03, f64::NAN).is_err());
    assert!(running_from_upfront(&instrument, &curve, f64::NAN, 25_000.0).is_err());
    assert!(implied_spread(&curve, instrument.effective, 0.03, 0.4).is_err());
    assert!(implied_spread(&curve, instrument.maturity, f64::NAN, 0.4).is_err());
    assert!(implied_spread(&curve, instrument.maturity, 0.03, 1.0).is_err());
    assert!(carry_and_roll(&instrument, &curve, 0.03, u32::MAX).is_err());
    assert!(carry_and_roll(&instrument, &curve, 0.03, 1_826).is_err());
    assert!(carry_and_roll(&instrument, &curve, f64::NAN, 30).is_err());
    assert!(next_imm_date(date(1_000_000, 12, 21)).is_err());
}

proptest! {
    #[test]
    fn cds_public_paths_do_not_panic(
        notional in any::<f64>(), coupon in any::<f64>(), recovery in any::<f64>(),
        spread in any::<f64>(), discount in any::<f64>(), upfront in any::<f64>(),
    ) {
        let effective = date(2024, 3, 20);
        let maturity = date(2029, 3, 20);
        let instrument = CdsInstrument {
            kind: CdsKind::SingleName { reference_entity: "x".into() },
            effective, maturity, notional, coupon, recovery, upfront,
        };
        let _ = instrument.validate();
        let _ = standard_coupon_amount(StandardCoupon::InvestmentGrade, notional, 94);
        let _ = basis(spread, discount);
        let _ = CreditCurve::bootstrap(effective, &[ParSpreadQuote { maturity, spread }], discount, recovery);
    }
}
