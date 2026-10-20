use finance_rs::cds::{
    carry_and_roll, implied_spread, running_from_upfront, upfront_from_running, CdsInstrument,
    CdsKind, CreditCurve, HazardNode,
};
use finance_rs::Date;

#[allow(clippy::expect_used)] // Test fixtures use fixed dates validated by the cases below.
fn date(y: i32, m: u32, d: u32) -> Date {
    Date::new(y, m, d).expect("valid test date")
}

#[allow(clippy::expect_used)] // The fixture uses fixed, validated CDS terms.
fn sample() -> (CdsInstrument, CreditCurve) {
    let effective = date(2024, 3, 20);
    let maturity = date(2029, 3, 20);
    let curve = CreditCurve {
        valuation_date: effective,
        nodes: vec![HazardNode {
            maturity,
            hazard_rate: 0.02,
        }],
    };
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
fn clean_dirty_values_separate_upfront_and_accrued_rebate() {
    let (instrument, curve) = sample();
    let value = instrument.value(&curve, 0.03).unwrap();
    let settlement_df = (-0.03_f64 * 5.0 / 365.0).exp();
    assert!((value.clean_value - (value.protection_leg - value.premium_leg)).abs() < 1e-14);
    assert!(
        (value.dirty_value - value.clean_value - instrument.coupon / 360.0 * settlement_df).abs()
            < 1e-14
    );

    let mut upfront_trade = instrument.clone();
    upfront_trade.upfront = 25_000.0;
    let upfront_value = upfront_trade.value(&curve, 0.03).unwrap();
    assert!(
        (upfront_value.dirty_value
            - upfront_value.clean_value
            - upfront_trade.coupon / 360.0 * settlement_df
            + upfront_trade.upfront / upfront_trade.notional * settlement_df)
            .abs()
            < 1e-14
    );
}

#[test]
#[allow(clippy::cast_precision_loss)] // This fixed fixture spans five years, exactly representable here.
fn zero_length_curve_intervals_and_invalid_valuation_terms_are_handled() {
    let (instrument, curve) = sample();
    let zero_length = CreditCurve {
        valuation_date: curve.valuation_date,
        nodes: vec![
            HazardNode {
                maturity: curve.valuation_date,
                hazard_rate: 0.01,
            },
            curve.nodes[0],
        ],
    };
    let expected =
        (-0.02 * instrument.maturity.days_since(instrument.effective) as f64 / 365.0).exp();
    assert!((zero_length.survival_probability(instrument.maturity) - expected).abs() < 1e-14);

    let mut invalid_terms = instrument.clone();
    invalid_terms.notional = f64::NAN;
    assert!(invalid_terms.value(&curve, 0.03).is_err());
}

#[test]
fn risk_caps_bumped_hazard_at_ten() {
    let (instrument, curve) = sample();
    let high_hazard = CreditCurve {
        valuation_date: curve.valuation_date,
        nodes: curve
            .nodes
            .iter()
            .map(|node| HazardNode {
                maturity: node.maturity,
                hazard_rate: 20.0,
            })
            .collect(),
    };
    assert!(instrument
        .risk(&high_hazard, 0.03)
        .unwrap()
        .cs01
        .is_finite());
}

#[test]
fn upfront_conversions_propagate_cash_settlement_date_overflow() {
    let effective = date(1_000_000, 12, 30);
    let maturity = date(1_000_000, 12, 31);
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
        nodes: vec![HazardNode {
            maturity,
            hazard_rate: 0.02,
        }],
    };
    assert!(upfront_from_running(&instrument, &curve, 0.03, 0.015).is_err());
    assert!(running_from_upfront(&instrument, &curve, 0.03, 25_000.0).is_err());
}

#[test]
fn carry_roll_propagates_date_overflow() {
    let effective = date(999_999, 12, 31);
    let maturity = date(1_000_000, 3, 20);
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
        nodes: vec![HazardNode {
            maturity,
            hazard_rate: 0.02,
        }],
    };
    assert!(carry_and_roll(&instrument, &curve, 0.03, 400).is_err());

    let mut negative_curve = curve.clone();
    negative_curve.nodes[0].hazard_rate = -100.0;
    assert!(implied_spread(&negative_curve, maturity, 0.03, 0.4).is_err());
}

#[cfg(feature = "serde")]
#[test]
fn malformed_deserialized_curve_date_is_rejected_by_spread_solver() {
    let invalid_date: Date = serde_json::from_str(r#"{"year":2024,"month":2,"day":30}"#).unwrap();
    let malformed_curve = CreditCurve {
        valuation_date: invalid_date,
        nodes: vec![HazardNode {
            maturity: date(2024, 3, 20),
            hazard_rate: 0.02,
        }],
    };
    assert!(implied_spread(&malformed_curve, date(2024, 3, 1), 0.03, 0.4).is_err());
}
