use super::*;

#[test]
fn zero_rate_is_linear() -> Result<()> {
    assert_eq!(fv(0.0, 10.0, -100.0, -1_000.0, When::End)?, 2_000.0);
    assert_eq!(pmt(0.0, 10.0, 1_000.0, 0.0, When::Begin)?, -100.0);
    assert_eq!(pv(0.0, 10.0, -100.0, 0.0, When::End)?, 1_000.0);
    assert_eq!(nper(0.0, -100.0, 1_000.0, 0.0, When::End)?, 10.0);
    assert_eq!(positive_level_payment_discount(0.0, 10.0, 1_000.0)?, 100.0);
    assert_eq!(positive_level_payment(0.0, 10.0, 1_000.0)?, 100.0);
    assert_eq!(pduration(0.0, 100.0, 100.0)?, 0.0);
    assert!(pduration(0.0, 100.0, 101.0).is_err());
    Ok(())
}

#[test]
fn discount_factor_matches_compounding_convention() -> Result<()> {
    assert!((discount_factor(0.1, 2.0)? - 1.21).abs() < 1e-12);
    assert!(discount_factor(-1.0, 1.0).is_err());
    assert!(discount_factor(0.1, -1.0).is_err());
    Ok(())
}

#[test]
fn factor_helpers_reject_nonfinite_arguments() {
    assert!(discount_factor(0.0, 0.0).is_ok());
    for result in [
        discount_factor(f64::NAN, 1.0),
        discount_factor(0.1, f64::INFINITY),
        annuity_pv_factor(f64::NEG_INFINITY, 1.0),
        annuity_pv_factor(0.1, f64::NAN),
        present_value_factor(f64::NAN, 1.0),
        present_value_factor(0.1, f64::INFINITY),
    ] {
        assert!(matches!(result, Err(FinanceError::NonFinite { .. })));
    }
}

#[test]
fn factor_helpers_cover_zero_equal_and_fractional_periods() -> Result<()> {
    assert_eq!(annuity_pv_factor(0.0, 8.0)?, 8.0);
    assert!((annuity_pv_factor(0.05, 2.0)? - (1.0 / 1.05 + 1.0 / 1.05_f64.powi(2))).abs() < 1e-12);
    assert!((present_value_factor(0.05, 2.0)? - 1.0 / 1.05_f64.powi(2)).abs() < 1e-12);
    assert!((factor_power(1.21, 0.5)? - 1.1).abs() < 1e-12);
    assert_eq!(geometric_discount_sum(1.0, 4.0)?, 4.0);
    assert!((geometric_discount_sum(0.5, 3.0)? - 0.875).abs() < 1e-12);
    assert!((positive_level_payment(0.05, 2.0, 100.0)? - 53.780_487_804_878_05).abs() < 1e-12);
    for result in [
        annuity_pv_factor(-1.0, 2.0),
        annuity_pv_factor(0.05, -1.0),
        present_value_factor(-1.0, 2.0),
        present_value_factor(0.05, -1.0),
        factor_power(0.0, 2.0),
        geometric_discount_sum(0.0, 2.0),
        geometric_discount_sum(0.5, -1.0),
        positive_level_payment(0.05, 0.0, 100.0),
    ] {
        assert!(result.is_err());
    }
    Ok(())
}

#[test]
fn annual_underwriting_payment_keeps_legacy_operation_order() -> Result<()> {
    let p = positive_level_payment_discount(0.06, 4.0, 120_000.0)?;
    assert_eq!(p, 34_630.979_084_792_79);
    Ok(())
}

#[test]
fn bad_input_is_typed() {
    assert!(matches!(
        pmt(f64::NAN, 1.0, 1.0, 0.0, When::End),
        Err(FinanceError::NonFinite { .. })
    ));
    assert!(matches!(
        pmt(0.0, 0.0, 1.0, 0.0, When::End),
        Err(FinanceError::Undefined { .. })
    ));
    assert!(matches!(
        ipmt(0.01, 0.0, 12.0, 1.0, 0.0, When::End),
        Err(FinanceError::InvalidInput { .. })
    ));
    assert!(matches!(
        nper(0.1, -1.0, 1_000.0, 0.0, When::End),
        Err(FinanceError::NoSolution { .. })
    ));
    assert!(fv(1e6, 1e6, 1.0, 1.0, When::End).is_err());
}

#[test]
fn beginning_payment_interest_uses_the_deferred_balance() -> Result<()> {
    let first = ipmt(0.01, 1.0, 12.0, 1_000.0, 0.0, When::Begin)?;
    let second = ipmt(0.01, 2.0, 12.0, 1_000.0, 0.0, When::Begin)?;
    assert_eq!(first, 0.0);
    assert!(second < 0.0);
    Ok(())
}

#[test]
fn begin_period_one_interest_is_zero() -> Result<()> {
    assert_eq!(ipmt(0.01, 1.0, 12.0, 1_000.0, 0.0, When::Begin)?, 0.0);
    Ok(())
}
