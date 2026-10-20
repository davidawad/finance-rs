use super::*;

#[test]
fn matches_f64_tvm() -> Result<()> {
    let r = Decimal::new(5, 3);
    let p = pmt(r, 360, Decimal::from(300_000), Decimal::ZERO, When::Begin)?;
    let f = crate::tvm::pmt(0.005, 360.0, 300_000.0, 0.0, When::Begin)?;
    let p64: f64 = p.round_dp(8).to_string().parse().unwrap_or(f64::NAN);
    assert!((p64 - f).abs() < 1e-6, "{p} vs {f}");
    Ok(())
}

#[test]
fn schedule_rejects_bad_input() {
    assert!(level_schedule(Decimal::ZERO, Decimal::ONE, 1, 2).is_err());
    assert!(level_schedule(Decimal::ONE, Decimal::new(-1, 2), 1, 2).is_err());
    assert!(level_schedule(Decimal::ONE, Decimal::ONE, 0, 2).is_err());
    assert!(npv(Decimal::ONE, &[]).is_err());
    assert!(matches!(
        npv(Decimal::NEGATIVE_ONE, &[Decimal::ONE, Decimal::ONE]),
        Err(FinanceError::Undefined { .. })
    ));
}

#[test]
fn decimal_xirr_and_real_estate_helpers_cover_edge_cases() -> Result<()> {
    let flows = [(Decimal::from(-100), 0), (Decimal::from(110), 365)];
    assert!(
        (irr(&[Decimal::from(-100), Decimal::from(110)])? - Decimal::new(1, 1)).abs()
            < Decimal::new(1, 12)
    );
    assert!((xirr(&flows)? - Decimal::new(1, 1)).abs() < Decimal::new(1, 12));
    assert_eq!(xnpv(Decimal::new(1, 1), &flows)?, Decimal::ZERO);
    assert_eq!(
        re_cap_rate(Decimal::from(12), Decimal::from(100)),
        Some(Decimal::new(12, 2))
    );
    assert_eq!(
        re_dscr(Decimal::from(12), Decimal::from(4)),
        Some(Decimal::from(3))
    );
    assert_eq!(
        re_cash_on_cash(Decimal::from(12), Decimal::from(4)),
        Some(Decimal::from(3))
    );
    assert_eq!(
        re_yield_on_cost(Decimal::from(12), Decimal::from(100)),
        Some(Decimal::new(12, 2))
    );
    assert_eq!(
        re_gross_yield(Decimal::from(12), Decimal::from(100)),
        Some(Decimal::new(12, 2))
    );
    assert_eq!(
        re_total_roi(Decimal::from(60), Decimal::from(60), Decimal::from(100)),
        Some(Decimal::new(2, 1))
    );
    assert_eq!(re_annualized_roi(Decimal::ONE, 365), Some(Decimal::ONE));
    assert_eq!(re_annualized_roi(Decimal::ONE, 0), None);
    assert_eq!(re_annualized_roi(Decimal::NEGATIVE_ONE, 365), None);
    assert_eq!(re_cap_rate(Decimal::ONE, Decimal::ZERO), None);
    assert_eq!(re_dscr(Decimal::ONE, Decimal::ZERO), None);
    assert_eq!(re_cash_on_cash(Decimal::ONE, Decimal::ZERO), None);
    assert_eq!(re_yield_on_cost(Decimal::ONE, Decimal::ZERO), None);
    assert_eq!(re_gross_yield(Decimal::ONE, Decimal::ZERO), None);
    assert_eq!(
        re_total_roi(Decimal::ONE, Decimal::ONE, Decimal::ZERO),
        None
    );
    Ok(())
}

#[test]
fn decimal_amortization_clears_balance_for_early_and_last_payment() -> Result<()> {
    let early = amortization_schedule(Decimal::ZERO, 4, Decimal::from(100), Decimal::from(50))?;
    assert_eq!(early.len(), 4);
    assert_eq!(early.last().map(|row| row.balance), Some(Decimal::ZERO));
    let final_rows =
        amortization_schedule(Decimal::new(1, 2), 2, Decimal::from(100), Decimal::from(1))?;
    assert_eq!(
        final_rows.last().map(|row| row.principal),
        Some(Decimal::from(100))
    );
    assert_eq!(
        final_rows.last().map(|row| row.balance),
        Some(Decimal::ZERO)
    );
    assert_eq!(
        amortization_schedule(Decimal::ZERO, 0, Decimal::from(100), Decimal::from(10))?.len(),
        0
    );
    Ok(())
}
