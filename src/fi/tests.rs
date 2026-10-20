use super::*;

#[test]
fn period_index_overflow_is_typed() {
    assert!(matches!(
        period_number(u32::MAX as usize),
        Err(FinanceError::Overflow {
            what: "period index"
        })
    ));
}

#[test]
fn textbook_prices() -> Result<()> {
    // Fabozzi: 20-year 9% semiannual at 12% -> 774.31; at 7% -> 1213.55.
    let b = Bond::new(1_000.0, 0.09, 40, 2)?;
    assert!((b.price(0.12)? - 774.31).abs() < 0.01);
    assert!((b.price(0.07)? - 1_213.55).abs() < 0.01);
    // Price at yield = coupon is par, for every frequency.
    for f in [1, 2, 4, 12] {
        let p = Bond::new(100.0, 0.065, 3 * f, f)?.price(0.065)?;
        assert!((p - 100.0).abs() < 1e-10, "{f}: {p}");
    }
    // Zero yield: undiscounted sum.
    assert!((Bond::new(100.0, 0.04, 6, 2)?.price(0.0)? - 112.0).abs() < 1e-10);
    Ok(())
}

#[test]
fn closed_form_matches_sum() -> Result<()> {
    let b = Bond::new(1_000.0, 0.07, 30, 4)?;
    let i: f64 = 0.055 / 4.0;
    let c = 1_000.0 * 0.07 / 4.0;
    let v = (1.0 + i).powi(-30);
    let want = c * (1.0 - v) / i + 1_000.0 * v;
    assert!((b.price(0.055)? - want).abs() < 1e-9);
    Ok(())
}

#[test]
fn validation() {
    assert!(matches!(
        Bond::new(100.0, 0.05, 10, 0),
        Err(FinanceError::InvalidInput {
            what: "frequency",
            ..
        })
    ));
    assert!(Bond::new(0.0, 0.05, 10, 2).is_err());
    assert!(Bond::new(100.0, -0.01, 10, 2).is_err());
    assert!(Bond::new(100.0, 0.05, 0, 2).is_err());
    assert!(Bond::new(100.0, f64::NAN, 10, 2).is_err());
    assert!(Bond::from_years(100.0, 0.05, 7.3, 2).is_err());
    assert!(Bond::from_years(100.0, 0.05, 0.0, 2).is_err());
    let b = Bond {
        face: 100.0,
        coupon_rate: 0.05,
        periods: 10,
        frequency: 2,
    };
    assert!(b.price(-2.0).is_err());
    assert!(b.price(f64::INFINITY).is_err());
    assert!(b.accrued_interest(1.0).is_err());
    assert!(b.current_yield(0.0).is_err());
    assert!(b.yield_from_price(-5.0).is_err());
    let bad = Bond { frequency: 0, ..b };
    assert!(bad.price(0.05).is_err() && bad.risk(0.05).is_err());
}

#[test]
fn accrued_and_dirty() -> Result<()> {
    let b = Bond::new(100.0, 0.08, 10, 2)?;
    let q0 = b.price_with_accrued(0.07, 0.0)?;
    assert!((q0.dirty - b.price(0.07)?).abs() < 1e-12 && q0.accrued == 0.0);
    let q = b.price_with_accrued(0.07, 0.4)?;
    assert!((q.accrued - 1.6).abs() < 1e-12);
    assert!((q.dirty - q.clean - q.accrued).abs() < 1e-12);
    // Dirty equals discounting each flow over k - a periods.
    let i: f64 = 0.035;
    let direct: f64 = (1..=10)
        .map(|k| {
            let cf = if k == 10 { 104.0 } else { 4.0 };
            cf / (1.0 + i).powf(f64::from(k) - 0.4)
        })
        .sum();
    assert!((q.dirty - direct).abs() < 1e-10);
    Ok(())
}

#[test]
fn yields_round_trip_including_extremes() -> Result<()> {
    let b = Bond::new(100.0, 0.05, 20, 2)?;
    for y in [-0.5, -0.01, 0.0, 0.03, 0.2, 1.5, 10.0] {
        let p = b.price(y)?;
        let got = b.yield_from_price(p)?;
        assert!((got - y).abs() < 1e-9 * (1.0 + y.abs()), "{y}: {got}");
    }
    let z = Bond::new(100.0, 0.0, 1, 1)?;
    assert!((z.yield_from_price(50.0)? - 1.0).abs() < 1e-12);
    assert!(
        (b.yield_from_clean_price(b.price_with_accrued(0.09, 0.7)?.clean, 0.7)? - 0.09).abs()
            < 1e-10
    );
    // A price so small no searchable yield reaches it.
    assert!(matches!(
        b.yield_from_price(1e-300),
        Err(FinanceError::NoSolution { .. })
    ));
    Ok(())
}

#[test]
fn duration_textbook_and_zero() -> Result<()> {
    // 3-year 10% annual coupon at 12%: Macaulay 2.7287.
    let b = Bond::new(100.0, 0.10, 3, 1)?;
    let p = b.price(0.12)?;
    let mac = (10.0 / 1.12 + 2.0 * 10.0 / 1.12f64.powi(2) + 3.0 * 110.0 / 1.12f64.powi(3)) / p;
    assert!((b.macaulay_duration(0.12)? - mac).abs() < 1e-12);
    assert!((mac - 2.72868).abs() < 1e-5, "{mac}");
    let z = Bond::new(100.0, 0.0, 24, 12)?;
    assert!((z.macaulay_duration(0.04)? - 2.0).abs() < 1e-12);
    Ok(())
}

#[test]
fn finite_difference_identities() -> Result<()> {
    for (b, y) in [
        (Bond::new(100.0, 0.06, 20, 2)?, 0.045),
        (Bond::new(1_000.0, 0.0, 30, 1)?, 0.08),
        (Bond::new(100.0, 0.12, 120, 12)?, 0.02),
        (Bond::new(100.0, 0.03, 8, 4)?, 0.10),
    ] {
        let h = 1e-4;
        let (pm, p0, pp) = (b.price(y - h)?, b.price(y)?, b.price(y + h)?);
        let r = b.risk(y)?;
        let fd_mod = -(pp - pm) / (2.0 * h) / p0;
        let fd_conv = (pp - 2.0 * p0 + pm) / (h * h) / p0;
        assert!((r.modified - fd_mod).abs() < 1e-5 * r.modified, "{b:?}");
        assert!((r.convexity - fd_conv).abs() < 1e-4 * r.convexity, "{b:?}");
        assert!((r.modified - r.macaulay / (1.0 + y / f64::from(b.frequency))).abs() < 1e-12);
        assert!((r.price - p0).abs() < 1e-12);
        assert_eq!(b.modified_duration(y)?, r.modified);
        assert_eq!(b.convexity(y)?, r.convexity);
        assert_eq!(b.dv01(y)?, r.dv01);
    }
    Ok(())
}

#[test]
fn current_yield_and_coupon() -> Result<()> {
    let b = Bond::from_years(1_000.0, 0.075, 10.0, 4)?;
    assert_eq!(b.periods, 40);
    assert!((b.coupon()? - 18.75).abs() < 1e-12);
    assert!((b.current_yield(950.0)? - 75.0 / 950.0).abs() < 1e-15);
    Ok(())
}

#[test]
fn generic_cashflow_pricing_and_risk_match_bond() -> Result<()> {
    let bond = Bond::new(100.0, 0.10, 3, 1)?;
    let flows = [10.0, 10.0, 110.0];
    let yield_rate = 0.12;
    let price = cashflow_price(&flows, yield_rate)?;
    let risk = cashflow_risk(&flows, yield_rate, 1, price)?;
    assert!(
        (price - bond.price(yield_rate)?).abs() < 1e-12,
        "{price} vs {}",
        bond.price(yield_rate)?
    );
    assert!((risk.macaulay - bond.macaulay_duration(yield_rate)?).abs() < 1e-12);
    assert!((risk.convexity - bond.convexity(yield_rate)?).abs() < 1e-12);
    assert!(
        (cashflow_price_derivative(&flows, yield_rate)? + risk.modified * price * 1.0).abs()
            < 1e-10
    );
    Ok(())
}

#[test]
fn generic_cashflow_helpers_cover_fallback_and_errors() -> Result<()> {
    let flows = [10.0, 110.0];
    let price = cashflow_price(&flows, 0.05)?;
    let fallback = cashflow_risk(&flows, 0.05, 2, 0.0)?;
    assert_eq!(fallback.price, price);
    assert!(cashflow_risk(&flows, 0.05, 0, price).is_err());
    assert!(cashflow_risk(&[], 0.05, 2, price).is_err());
    assert!(cashflow_price(&[], 0.05).is_err());
    assert!(cashflow_price(&flows, -1.0).is_err());
    assert!(cashflow_price(&flows, f64::NAN).is_err());
    assert!(cashflow_price(&[f64::NAN], 0.05).is_err());
    assert!(cashflow_price_derivative(&flows, -1.0).is_err());
    assert!(cashflow_price_derivative(&[f64::INFINITY], 0.05).is_err());
    Ok(())
}

#[test]
fn cashflow_risk_rejects_invalid_frequency_and_zero_price() {
    assert!(cashflow_risk(&[100.0], 0.0, 0, 100.0).is_err());
    assert!(cashflow_risk(&[100.0], 0.0, 1, 0.0).is_ok());
    assert!(cashflow_risk(&[0.0], 0.0, 1, 0.0).is_err());
}
