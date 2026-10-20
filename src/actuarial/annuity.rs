//! Closed-form annuity factors (Bowers et al., *Actuarial Mathematics*, 2e).
use crate::{FinanceError, Result};

fn check(rate: f64, n: f64) -> Result<()> {
    if !rate.is_finite() || rate <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "rate",
            reason: "must be finite and greater than -1",
        });
    }
    if !n.is_finite() || n < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "n",
            reason: "must be finite and nonnegative",
        });
    }
    Ok(())
}
fn out(value: f64, what: &'static str) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(FinanceError::Overflow { what })
    }
}

/// Present value of n unit payments at each period end.
///
/// ```
/// let value = finance_rs::actuarial::annuity::immediate(0.05, 10.0)?;
/// assert!((value - 7.721_734_929).abs() < 1e-8);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn immediate(rate: f64, n: f64) -> Result<f64> {
    check(rate, n)?;
    if rate == 0.0 {
        return Ok(n);
    }
    out(-(-n * rate.ln_1p()).exp_m1() / rate, "annuity-immediate")
}
/// Present value of n unit payments at each period beginning.
///
/// ```
/// let value = finance_rs::actuarial::annuity::due(0.05, 10.0)?;
/// assert!((value - 8.107_821_675).abs() < 1e-8);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn due(rate: f64, n: f64) -> Result<f64> {
    out(immediate(rate, n)? * (1.0 + rate), "annuity-due")
}
/// Present value of a continuous unit payment stream over n years.
///
/// ```
/// let value = finance_rs::actuarial::annuity::continuous(0.05, 10.0)?;
/// assert!((value - 7.869_386_806).abs() < 1e-8);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn continuous(force: f64, n: f64) -> Result<f64> {
    if !force.is_finite() {
        return Err(FinanceError::NonFinite { what: "force" });
    }
    if !n.is_finite() || n < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "n",
            reason: "must be finite and nonnegative",
        });
    }
    if force == 0.0 {
        return Ok(n);
    }
    out(-(-force * n).exp_m1() / force, "continuous-annuity")
}
/// Present value of a deferred n-payment annuity-immediate.
///
/// ```
/// let value = finance_rs::actuarial::annuity::deferred(0.05, 3.0, 10.0)?;
/// assert!(value > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn deferred(rate: f64, defer: f64, n: f64) -> Result<f64> {
    check(rate, n)?;
    if !defer.is_finite() || defer < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "defer",
            reason: "must be finite and nonnegative",
        });
    }
    out(
        (1.0 + rate).powf(-defer) * immediate(rate, n)?,
        "deferred-annuity",
    )
}
/// Present value of payments 1, 2, …, n at period ends.
///
/// ```
/// let value = finance_rs::actuarial::annuity::increasing(0.05, 4)?;
/// assert!((value - 8.648_762_604).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn increasing(rate: f64, n: u32) -> Result<f64> {
    check(rate, f64::from(n))?;
    if rate == 0.0 {
        return Ok(f64::from(n) * (f64::from(n) + 1.0) / 2.0);
    }
    let v_n = (1.0 + rate).powf(-f64::from(n));
    out(
        ((1.0 + rate) * immediate(rate, f64::from(n))? - f64::from(n) * v_n) / rate,
        "increasing-annuity",
    )
}
/// Perpetuity-immediate (`1/i`) or perpetuity-due (`(1+i)/i`).
///
/// ```
/// use finance_rs::tvm::When;
/// let value = finance_rs::actuarial::annuity::perpetuity(0.05, When::End)?;
/// assert_eq!(value, 20.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn perpetuity(rate: f64, when: crate::tvm::When) -> Result<f64> {
    if !rate.is_finite() || rate <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "rate",
            reason: "perpetuity requires a positive rate",
        });
    }
    Ok(match when {
        crate::tvm::When::Begin => (1.0 + rate) / rate,
        crate::tvm::When::End => 1.0 / rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    #[test]
    fn textbook_examples() {
        assert!((immediate(0.05, 10.0).unwrap() - 7.721_734_929).abs() < 1e-8);
        assert!((due(0.05, 10.0).unwrap() - 8.107_821_675).abs() < 1e-8);
        assert!((continuous(0.05, 10.0).unwrap() - 7.869_386_806).abs() < 1e-8);
        assert!(
            (deferred(0.05, 3.0, 10.0).unwrap() - immediate(0.05, 10.0).unwrap() / 1.05f64.powi(3))
                .abs()
                < 1e-12
        );
        assert_eq!(increasing(0.0, 4).unwrap(), 10.0);
        assert!((increasing(0.05, 4).unwrap() - 8.648_762_604).abs() < 1e-9);
        assert_eq!(perpetuity(0.05, crate::tvm::When::End).unwrap(), 20.0);
    }

    #[test]
    fn invalid_and_overflowing_values_are_rejected() {
        assert!(immediate(-1.0, 1.0).is_err());
        assert!(immediate(f64::NAN, 1.0).is_err());
        assert!(immediate(0.05, f64::INFINITY).is_err());
        assert!(continuous(f64::INFINITY, 1.0).is_err());
        assert!(continuous(0.05, -1.0).is_err());
        assert!(deferred(0.05, -1.0, 1.0).is_err());
        assert!(deferred(0.05, f64::NAN, 1.0).is_err());
        assert!(perpetuity(0.0, crate::tvm::When::End).is_err());
        assert!(perpetuity(f64::NAN, crate::tvm::When::Begin).is_err());
        assert!(immediate(-0.5, f64::MAX).is_err());
    }
    proptest! { #[test] fn immediate_matches_sum(r in 0.0001f64..0.3, n in 0u32..100) {
        let explicit: f64 = (1..=n).map(|k| 1.0/(1.0+r).powi(i32::try_from(k).unwrap_or(i32::MAX))).sum();
        prop_assert!((immediate(r, f64::from(n)).unwrap()-explicit).abs() < 1e-9);
    }}
}
