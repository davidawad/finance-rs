//! Conversion between common odds and probabilities.

use crate::error::{finite, FinanceError, Result};

fn positive(name: &'static str, x: f64) -> Result<()> {
    finite(name, x)?;
    if x <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: name,
            reason: "must be positive",
        });
    }
    Ok(())
}

fn probability(p: f64) -> Result<()> {
    finite("probability", p)?;
    if !(0.0..=1.0).contains(&p) {
        return Err(FinanceError::InvalidInput {
            what: "probability",
            reason: "must be in [0, 1]",
        });
    }
    Ok(())
}

/// Convert decimal odds (including returned stake) to probability.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn decimal_to_prob(decimal_odds: f64) -> Result<f64> {
    positive("decimal_odds", decimal_odds)?;
    if decimal_odds < 1.0 {
        return Err(FinanceError::InvalidInput {
            what: "decimal_odds",
            reason: "must be at least one",
        });
    }
    Ok(1.0 / decimal_odds)
}

/// Convert a probability to decimal odds. Zero probability has infinite odds
/// and is rejected because successful results must be finite.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn prob_to_decimal(p: f64) -> Result<f64> {
    probability(p)?;
    if p == 0.0 {
        return Err(FinanceError::Undefined {
            what: "probability",
        });
    }
    Ok(1.0 / p)
}

/// Convert American odds to probability.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn american_to_prob(american: f64) -> Result<f64> {
    finite("american_odds", american)?;
    if american == 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "american_odds",
            reason: "must be nonzero",
        });
    }
    Ok(if american > 0.0 {
        100.0 / (american + 100.0)
    } else {
        -american / (-american + 100.0)
    })
}

/// Convert probability to American odds.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
pub fn prob_to_american(p: f64) -> Result<f64> {
    probability(p)?;
    if p == 0.0 || p == 1.0 {
        return Err(FinanceError::Undefined {
            what: "endpoint probability",
        });
    }
    Ok(if p <= 0.5 {
        100.0 * (1.0 - p) / p
    } else {
        -100.0 * p / (1.0 - p)
    })
}

/// Convert fractional odds `numerator / denominator` (net winnings) to probability.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn fractional_to_prob(numerator: f64, denominator: f64) -> Result<f64> {
    positive("numerator", numerator)?;
    positive("denominator", denominator)?;
    Ok(denominator / (numerator + denominator))
}

/// Represent a probability as an equivalent fractional-odds pair
/// `(numerator, denominator)`, without reducing the pair to integers.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
pub fn prob_to_fractional(p: f64) -> Result<(f64, f64)> {
    probability(p)?;
    if p == 0.0 || p == 1.0 {
        return Err(FinanceError::Undefined {
            what: "endpoint probability",
        });
    }
    Ok((1.0 - p, p))
}
