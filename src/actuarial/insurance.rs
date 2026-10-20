//! Indexed universal life crediting and policy-loan spread formulas.
use crate::{FinanceError, Result};
/// Annual credited amount: `clamp(participation × index_return, floor, cap)`.
///
/// ```
/// let credit = finance_rs::actuarial::insurance::iul_credit(0.12, 0.10, 0.0, 0.8)?;
/// assert_eq!(credit, 0.096);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn iul_credit(index_return: f64, cap: f64, floor: f64, participation: f64) -> Result<f64> {
    if [index_return, cap, floor, participation]
        .iter()
        .any(|v| !v.is_finite())
    {
        return Err(FinanceError::NonFinite { what: "IUL inputs" });
    }
    if cap < floor || floor < -1.0 || participation < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "IUL inputs",
            reason: "require cap >= floor >= -1 and nonnegative participation",
        });
    }
    Ok((participation * index_return).clamp(floor, cap))
}
/// Net policy-loan spread cost. A negative result means the credit exceeds the loan rate.
///
/// ```
/// let cost = finance_rs::actuarial::insurance::policy_loan_spread(100.0, 0.03, 0.05)?;
/// assert!((cost - 2.0).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn policy_loan_spread(balance: f64, credited_rate: f64, loan_rate: f64) -> Result<f64> {
    if [balance, credited_rate, loan_rate]
        .iter()
        .any(|v| !v.is_finite())
    {
        return Err(FinanceError::NonFinite {
            what: "policy loan inputs",
        });
    }
    if balance < 0.0 || credited_rate < 0.0 || loan_rate < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "policy loan inputs",
            reason: "must be nonnegative",
        });
    }
    let v = balance * (loan_rate - credited_rate);
    if v.is_finite() {
        Ok(v)
    } else {
        Err(FinanceError::Overflow {
            what: "policy loan spread",
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn credit_floor_cap_and_spread() {
        assert_eq!(iul_credit(-0.4, 0.1, 0.0, 1.0).unwrap(), 0.0);
        assert_eq!(iul_credit(0.2, 0.1, 0.0, 0.8).unwrap(), 0.1);
        assert!((policy_loan_spread(100.0, 0.03, 0.05).unwrap() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_nonfinite_invalid_and_overflowing_policy_inputs() {
        assert!(matches!(
            iul_credit(f64::NAN, 0.1, 0.0, 1.0),
            Err(FinanceError::NonFinite { .. })
        ));
        assert!(iul_credit(0.1, -0.1, 0.0, 1.0).is_err());
        assert!(iul_credit(0.1, 0.1, -1.1, 1.0).is_err());
        assert!(iul_credit(0.1, 0.1, 0.0, -1.0).is_err());
        assert!(matches!(
            policy_loan_spread(f64::INFINITY, 0.03, 0.05),
            Err(FinanceError::NonFinite { .. })
        ));
        assert!(policy_loan_spread(-1.0, 0.03, 0.05).is_err());
        assert!(policy_loan_spread(1.0, -0.03, 0.05).is_err());
        assert!(policy_loan_spread(1.0, 0.03, -0.05).is_err());
        assert!(matches!(
            policy_loan_spread(f64::MAX, 0.0, 2.0),
            Err(FinanceError::Overflow { .. })
        ));
    }
}
