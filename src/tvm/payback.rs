//! Capital-budgeting measures on periodic flows `values[t]` at `t = 0, 1, …`:
//! payback, discounted payback and the profitability index.

use crate::error::{all_finite, finite, min_len, output, FinanceError, Result};

fn check(values: &[f64]) -> Result<()> {
    min_len("values", values, 1)?;
    all_finite("values", values)
}

fn discount_rate(rate: f64) -> Result<f64> {
    finite("rate", rate)?;
    if rate <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "rate",
            reason: "must be greater than -1",
        });
    }
    Ok(rate)
}

/// First time the running sum of `flows` reaches zero from below,
/// interpolated linearly within the period that crosses.
#[allow(clippy::cast_precision_loss)]
fn first_crossing(flows: impl Iterator<Item = f64>) -> Option<f64> {
    let mut cum = 0.0;
    for (t, v) in flows.enumerate() {
        let prev = cum;
        cum += v;
        if cum >= 0.0 {
            return Some(if t == 0 {
                0.0
            } else {
                (t - 1) as f64 + (-prev) / v
            });
        }
    }
    None
}

/// Payback period: the first `t` at which the cumulative flow
/// `Σ_{s ≤ t} values[s]` reaches zero, interpolated within the period
/// (`t − 1 + |C_{t−1}|/values[t]`). `Ok(None)` when the flows never pay
/// back; `Some(0.0)` when `values[0] ≥ 0`.
///
/// ```
/// use finance_rs::tvm::payback_period;
/// assert_eq!(payback_period(&[-1_000.0, 400.0, 400.0, 400.0])?, Some(2.5));
/// assert_eq!(payback_period(&[-1_000.0, 100.0])?, None);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for no values, [`FinanceError::NonFinite`].
pub fn payback_period(values: &[f64]) -> Result<Option<f64>> {
    check(values)?;
    first_crossing(values.iter().copied())
        .map(|t| output("payback_period", t))
        .transpose()
}

/// Discounted payback: [`payback_period`] on `values[t]/(1+rate)^t`.
///
/// ```
/// use finance_rs::tvm::discounted_payback;
/// let t = discounted_payback(0.1, &[-1_000.0, 550.0, 605.0])?;
/// assert!(t.is_some_and(|t| (t - 1.0 - 500.0 / 500.0).abs() < 1e-12));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for `rate ≤ −1`; as [`payback_period`].
pub fn discounted_payback(rate: f64, values: &[f64]) -> Result<Option<f64>> {
    check(values)?;
    let ln1p = discount_rate(rate)?.ln_1p();
    #[allow(clippy::cast_precision_loss)]
    let flows = values
        .iter()
        .enumerate()
        .map(|(t, v)| v * (-(t as f64) * ln1p).exp());
    first_crossing(flows)
        .map(|t| output("discounted_payback", t))
        .transpose()
}

/// Profitability index: `PV(inflows)/|PV(outflows)|`, discounting
/// `values[t]` by `(1+rate)^t`.
///
/// ```
/// use finance_rs::tvm::profitability_index;
/// let pi = profitability_index(0.1, &[-1_000.0, 550.0, 605.0])?;
/// assert!((pi - 1.0).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::Undefined`] when there is no outflow;
/// [`FinanceError::InvalidInput`] for `rate ≤ −1`; as [`payback_period`].
pub fn profitability_index(rate: f64, values: &[f64]) -> Result<f64> {
    check(values)?;
    let ln1p = discount_rate(rate)?.ln_1p();
    let (mut inflow, mut outflow) = (0.0, 0.0);
    for (t, v) in values.iter().enumerate() {
        #[allow(clippy::cast_precision_loss)]
        let pv = v * (-(t as f64) * ln1p).exp();
        if pv > 0.0 {
            inflow += pv;
        } else {
            outflow -= pv;
        }
    }
    crate::error::div("present value of outflows", inflow, outflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payback_edges() -> Result<()> {
        assert_eq!(payback_period(&[0.0, -5.0])?, Some(0.0));
        assert_eq!(payback_period(&[-10.0, 0.0, 10.0])?, Some(1.0 + 1.0));
        assert!(payback_period(&[]).is_err());
        assert!(profitability_index(0.1, &[1.0, 2.0]).is_err());
        assert!(profitability_index(0.1, &[-1_000.0, -100.0, 1_200.0])? > 0.0);
        assert_eq!(discounted_payback(0.0, &[-3.0, 1.0, 1.0, 1.0])?, Some(3.0));
        Ok(())
    }
}
