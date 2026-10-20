//! Annual interest-only schedules used by the acquisition models.

use crate::error::{finite, FinanceError, Result};
use crate::tvm::positive_level_payment_discount;

/// Annual debt service and year-end balances, with an initial interest-only
/// period followed by level amortization. Keeps the legacy annual operation
/// order used by underwriting fixtures, including its final floating residue.
///
/// ```rust
/// use finance_rs::amortization::interest_only_annual;
/// let (payments, balances) = interest_only_annual(120_000.0, 0.06, 4, 1)?;
/// assert_eq!(payments.len(), 4);
/// assert_eq!(balances[0], 120_000.0);
/// assert!(balances[3].abs() < 1e-8);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn interest_only_annual(
    principal: f64,
    annual_rate: f64,
    term_years: u32,
    io_years: u32,
) -> Result<(Vec<f64>, Vec<f64>)> {
    finite("principal", principal)?;
    finite("annual_rate", annual_rate)?;
    if principal <= 0.0 || annual_rate < 0.0 || term_years == 0 || io_years > term_years {
        return Err(FinanceError::InvalidInput {
            what: "annual interest-only schedule",
            reason: "requires positive principal and term, non-negative rate, and io_years <= term_years",
        });
    }
    if term_years > super::MAX_PERIODS {
        return Err(FinanceError::InvalidInput {
            what: "term_years",
            reason: "exceeds MAX_PERIODS",
        });
    }

    let amort_years = term_years - io_years;
    let payment = if amort_years == 0 {
        0.0
    } else if annual_rate == 0.0 {
        principal / f64::from(amort_years)
    } else {
        positive_level_payment_discount(annual_rate, f64::from(amort_years), principal)?
    };
    let mut debt_service = Vec::with_capacity(term_years as usize);
    let mut balances = Vec::with_capacity(term_years as usize);
    let mut balance = principal;
    for _ in 0..io_years {
        debt_service.push(balance * annual_rate);
        balances.push(balance);
    }
    for _ in 0..amort_years {
        let interest = balance * annual_rate;
        let principal_paid = (payment - interest).max(0.0);
        balance = (balance - principal_paid).max(0.0);
        debt_service.push(payment);
        balances.push(balance);
    }
    Ok((debt_service, balances))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn annual_schedule_preserves_underwriting_operation_order() {
        let (payments, balances) = interest_only_annual(120_000.0, 0.06, 4, 0).unwrap();
        assert_eq!(payments[0], 34_630.979_084_792_79);
        assert!((balances[3] - 1.200_533_006_340_265e-10).abs() < 1e-25);
    }

    #[test]
    fn annual_schedule_handles_a_fully_interest_only_term() {
        let (payments, balances) = interest_only_annual(100_000.0, 0.0, 2, 2).unwrap();
        assert_eq!(payments, [0.0, 0.0]);
        assert_eq!(balances, [100_000.0, 100_000.0]);
    }

    #[test]
    fn annual_schedule_uses_equal_principal_at_zero_rate() {
        let (payments, balances) = interest_only_annual(100.0, 0.0, 2, 0).unwrap();
        assert_eq!(payments, [50.0, 50.0]);
        assert_eq!(balances, [50.0, 0.0]);
    }

    #[test]
    fn annual_schedule_rejects_invalid_inputs() {
        assert!(interest_only_annual(f64::NAN, 0.05, 2, 0).is_err());
        assert!(interest_only_annual(100.0, f64::INFINITY, 2, 0).is_err());
        assert!(interest_only_annual(0.0, 0.05, 2, 0).is_err());
        assert!(interest_only_annual(100.0, -0.05, 2, 0).is_err());
        assert!(interest_only_annual(100.0, 0.05, 0, 0).is_err());
        assert!(interest_only_annual(100.0, 0.05, 2, 3).is_err());
        assert!(interest_only_annual(100.0, 0.05, super::super::MAX_PERIODS + 1, 0).is_err());
    }
}
