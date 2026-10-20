//! Proper scoring rules for binary probability forecasts.

use crate::error::{all_finite, FinanceError, Result};

fn validate(forecasts: &[f64], outcomes: &[f64]) -> Result<()> {
    if forecasts.is_empty() {
        return Err(FinanceError::TooShort {
            what: "forecasts",
            min: 1,
            got: 0,
        });
    }
    if forecasts.len() != outcomes.len() {
        return Err(FinanceError::LengthMismatch {
            left: "forecasts",
            right: "outcomes",
            left_len: forecasts.len(),
            right_len: outcomes.len(),
        });
    }
    all_finite("forecasts", forecasts)?;
    all_finite("outcomes", outcomes)?;
    if forecasts
        .iter()
        .chain(outcomes)
        .any(|v| !(0.0..=1.0).contains(v))
    {
        return Err(FinanceError::InvalidInput {
            what: "forecasts/outcomes",
            reason: "values must be in [0, 1]",
        });
    }
    Ok(())
}

/// Mean squared forecast error, `mean((p - outcome)^2)`.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn brier(forecasts: &[f64], outcomes: &[f64]) -> Result<f64> {
    validate(forecasts, outcomes)?;
    Ok(forecasts
        .iter()
        .zip(outcomes)
        .map(|(p, o)| (p - o).powi(2))
        .sum::<f64>()
        / forecasts.len() as f64)
}

/// Mean binary cross entropy. A certain forecast contradicted by an outcome
/// has infinite loss and returns `FinanceError::Overflow`.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
#[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
pub fn log_loss(forecasts: &[f64], outcomes: &[f64]) -> Result<f64> {
    validate(forecasts, outcomes)?;
    let mut sum = 0.0;
    for (&p, &o) in forecasts.iter().zip(outcomes) {
        if (p == 0.0 && o > 0.0) || (p == 1.0 && o < 1.0) {
            return Err(FinanceError::Overflow { what: "log loss" });
        }
        if o > 0.0 {
            sum -= o * p.ln();
        }
        if o < 1.0 {
            sum -= (1.0 - o) * (1.0 - p).ln();
        }
    }
    Ok(sum / forecasts.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_cover_validation_and_certain_forecasts() {
        assert!(matches!(
            brier(&[], &[]),
            Err(FinanceError::TooShort { .. })
        ));
        assert!(matches!(
            brier(&[0.5], &[]),
            Err(FinanceError::LengthMismatch { .. })
        ));
        assert!(brier(&[f64::NAN], &[0.0]).is_err());
        assert!(brier(&[1.1], &[0.0]).is_err());
        assert_eq!(brier(&[0.0, 1.0], &[0.0, 1.0]).unwrap(), 0.0);
        assert!(log_loss(&[0.0], &[1.0]).is_err());
        assert!(log_loss(&[1.0], &[0.0]).is_err());
        assert!(log_loss(&[f64::NAN], &[0.0]).is_err());
        assert!(log_loss(&[0.5], &[0.5]).unwrap().is_finite());
    }
}
