//! Bankroll fractions from Kelly's log-utility criterion.
//!
//! Fractions are proportions of bankroll (not percentages). Binary odds `b`
//! are net profit per unit staked; decimal odds include returned stake.

use crate::error::{all_finite, finite, output, FinanceError, Result};
use crate::solve::{brent, SolverOptions};

/// Uncapped binary Kelly fraction, `(b p - (1-p))/b`.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::kelly::binary_raw), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn binary_raw(probability: f64, net_odds: f64) -> Result<f64> {
    finite("probability", probability)?;
    finite("net_odds", net_odds)?;
    if !(0.0..=1.0).contains(&probability) || net_odds <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "binary Kelly",
            reason: "probability must be in [0, 1] and net odds positive",
        });
    }
    output(
        "binary Kelly",
        (net_odds * probability - (1.0 - probability)) / net_odds,
    )
}

/// Long-only binary Kelly fraction, clipped to `[0, 1]`.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::kelly::binary), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn binary(probability: f64, net_odds: f64) -> Result<f64> {
    Ok(binary_raw(probability, net_odds)?.clamp(0.0, 1.0))
}

/// Apply a fractional Kelly multiplier in `[0, 1]`.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::kelly::fractional), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn fractional(full_fraction: f64, fraction: f64) -> Result<f64> {
    finite("full_fraction", full_fraction)?;
    finite("fraction", fraction)?;
    if !(0.0..=1.0).contains(&fraction) {
        return Err(FinanceError::InvalidInput {
            what: "fraction",
            reason: "must be in [0, 1]",
        });
    }
    output("fractional Kelly", full_fraction.max(0.0) * fraction)
}

/// Apply a non-negative cap to a long-only fraction.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::kelly::capped), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn capped(fraction: f64, cap: f64) -> Result<f64> {
    finite("fraction", fraction)?;
    finite("cap", cap)?;
    if cap < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "cap",
            reason: "must be non-negative",
        });
    }
    output("capped Kelly", fraction.max(0.0).min(cap))
}

/// Continuous-return Kelly allocation `(mu - risk_free) / sigma²`.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::kelly::continuous), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn continuous(mu: f64, sigma: f64, risk_free: f64) -> Result<f64> {
    for (name, value) in [("mu", mu), ("sigma", sigma), ("risk_free", risk_free)] {
        finite(name, value)?;
    }
    if sigma <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "sigma",
            reason: "must be positive",
        });
    }
    output("continuous Kelly", (mu - risk_free) / sigma.powi(2))
}

/// Simultaneous Kelly fractions for mutually exclusive outcomes.
///
/// `probabilities` may sum to less than one; the remainder is the no-win
/// outcome. `decimal_odds[i]` is the total payout per unit staked on outcome
/// `i`. The concave log-wealth objective is maximized by coordinate-wise
/// bisection of its monotone derivatives.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::kelly::multi_outcome), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn multi_outcome(probabilities: &[f64], decimal_odds: &[f64]) -> Result<Vec<f64>> {
    if probabilities.is_empty() {
        return Err(FinanceError::TooShort {
            what: "probabilities",
            min: 1,
            got: 0,
        });
    }
    if probabilities.len() != decimal_odds.len() {
        return Err(FinanceError::LengthMismatch {
            left: "probabilities",
            right: "decimal_odds",
            left_len: probabilities.len(),
            right_len: decimal_odds.len(),
        });
    }
    all_finite("probabilities", probabilities)?;
    all_finite("decimal_odds", decimal_odds)?;
    if probabilities.iter().any(|p| *p < 0.0 || *p > 1.0) || decimal_odds.iter().any(|o| *o <= 1.0)
    {
        return Err(FinanceError::InvalidInput {
            what: "multi-outcome Kelly",
            reason: "probabilities must be in [0, 1] and decimal odds greater than one",
        });
    }
    let total: f64 = probabilities.iter().sum();
    if total > 1.0 + 1e-12 {
        return Err(FinanceError::InvalidInput {
            what: "probabilities",
            reason: "mutually exclusive probabilities must sum to at most one",
        });
    }
    let n = probabilities.len();
    let mut fractions = vec![0.0; n];
    let p_none = (1.0 - total).max(0.0);
    for _ in 0..500 {
        let before = fractions.clone();
        for i in 0..n {
            let other: f64 = fractions.iter().sum::<f64>() - fractions[i];
            let upper = (1.0 - other).max(0.0);
            let derivative = |stake: f64| {
                let mut value = 0.0;
                for j in 0..n {
                    if probabilities[j] == 0.0 {
                        continue;
                    }
                    let wealth = 1.0 - other - stake
                        + if i == j {
                            decimal_odds[j] * stake
                        } else {
                            decimal_odds[j] * fractions[j]
                        };
                    if wealth <= 0.0 {
                        return f64::NEG_INFINITY;
                    }
                    let slope = if i == j { decimal_odds[j] - 1.0 } else { -1.0 };
                    value += probabilities[j] * slope / wealth;
                }
                let no_win_wealth = 1.0 - other - stake;
                if p_none > 0.0 {
                    if no_win_wealth <= 0.0 {
                        return f64::NEG_INFINITY;
                    }
                    value -= p_none / no_win_wealth;
                }
                value
            };
            let at_zero = derivative(0.0);
            fractions[i] = if at_zero <= 0.0 {
                0.0
            } else if derivative(upper) >= 0.0 {
                upper
            } else {
                let (mut lo, mut hi) = (0.0, upper);
                for _ in 0..80 {
                    let mid = (lo + hi) * 0.5;
                    if derivative(mid) > 0.0 {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                (lo + hi) * 0.5
            };
        }
        if fractions
            .iter()
            .zip(&before)
            .all(|(a, b)| (a - b).abs() <= 1e-12)
        {
            break;
        }
    }
    fractions
        .into_iter()
        .map(|f| output("multi-outcome Kelly", f))
        .collect()
}

/// Solve a single bracketed scalar equation with the crate-wide solver.
pub(crate) fn solve_unit_interval(f: impl Fn(f64) -> f64) -> Result<f64> {
    Ok(brent(f, 0.0, 1.0, &SolverOptions::default())?.root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kelly_forms_cover_errors_zero_probability_and_boundaries() {
        assert!(multi_outcome(&[], &[]).is_err());
        assert!(multi_outcome(&[0.5], &[]).is_err());
        assert!(multi_outcome(&[1.1], &[2.0]).is_err());
        assert!(multi_outcome(&[0.6, 0.6], &[2.0, 2.0]).is_err());
        let fractions = multi_outcome(&[0.0, 0.5], &[3.0, 3.0]).unwrap();
        assert_eq!(fractions[0], 0.0);
        let full = multi_outcome(&[1.0], &[3.0]).unwrap();
        assert_eq!(full, vec![1.0]);
        assert!(multi_outcome(&[0.5], &[3.0]).unwrap()[0] > 0.0);
        assert!(binary_raw(0.4, 2.0).unwrap() > 0.0);
        assert!(fractional(0.5, 0.5).unwrap() > 0.0);
        assert!(capped(0.5, 0.2).unwrap() <= 0.2);
        assert!(continuous(0.1, 0.0, 0.0).is_err());
        assert!(solve_unit_interval(|x| x - 0.25).unwrap() > 0.2);
    }
}
