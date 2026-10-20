//! Trade, payoff, and portfolio return ratios.

use super::{check_returns, mean, std_dev};
use crate::{
    error::{all_finite, output, same_len, FinanceError, Result},
    stats::{quantile, QuantileMethod},
};

/// Sum of returns divided by absolute losses.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn gain_to_pain(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    let losses = returns
        .iter()
        .filter(|r| **r < 0.0)
        .map(|r| -r)
        .sum::<f64>();
    if losses == 0.0 {
        return Err(FinanceError::Undefined { what: "losses" });
    }
    output("gain_to_pain", returns.iter().sum::<f64>() / losses)
}
/// Average win divided by absolute average loss.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn payoff_ratio(returns: &[f64]) -> Result<f64> {
    output(
        "payoff_ratio",
        super::returns_extra::avg_win(returns)? / super::returns_extra::avg_loss(returns)?.abs(),
    )
}
/// Sum of wins divided by absolute sum of losses.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn profit_factor(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    let wins = returns.iter().filter(|r| **r > 0.0).sum::<f64>();
    let losses = -returns.iter().filter(|r| **r < 0.0).sum::<f64>();
    if losses == 0.0 {
        return Err(FinanceError::Undefined { what: "losses" });
    }
    output("profit_factor", wins / losses)
}
/// Ratio of winning observation frequency to losing observation frequency.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn profit_ratio(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    let wins = returns.iter().filter(|r| **r >= 0.0).count() as f64;
    let losses = returns.iter().filter(|r| **r < 0.0).count() as f64;
    if wins == 0.0 {
        return Ok(0.0);
    }
    if losses == 0.0 {
        return Err(FinanceError::Undefined {
            what: "losing observations",
        });
    }
    let win_sum = returns.iter().filter(|r| **r >= 0.0).sum::<f64>();
    let loss_sum = -returns.iter().filter(|r| **r < 0.0).sum::<f64>();
    output(
        "profit_ratio",
        (win_sum / wins.powi(2)) / (loss_sum / losses.powi(2)),
    )
}
/// Absolute ratio of 95th to 5th percentile returns.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn tail_ratio(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 2)?;
    let hi = quantile(returns, 0.95, QuantileMethod::Linear)?;
    let lo = quantile(returns, 0.05, QuantileMethod::Linear)?;
    if lo == 0.0 {
        return Err(FinanceError::Undefined {
            what: "5th percentile return",
        });
    }
    output("tail_ratio", (hi / lo).abs())
}
/// Profit factor multiplied by the tail ratio.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn common_sense_ratio(returns: &[f64]) -> Result<f64> {
    output(
        "common_sense_ratio",
        profit_factor(returns)? * tail_ratio(returns)?,
    )
}
/// Win rate times payoff ratio times loss rate.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn cpc_index(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    let traded = returns.iter().filter(|r| **r != 0.0).count() as f64;
    if traded == 0.0 {
        return Err(FinanceError::Undefined {
            what: "nonzero returns",
        });
    }
    let wins = returns.iter().filter(|r| **r > 0.0).count() as f64 / traded;
    output(
        "cpc_index",
        profit_factor(returns)? * wins * payoff_ratio(returns)?,
    )
}

/// 99th percentile return divided by the mean nonnegative return.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn outlier_win_ratio(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 2)?;
    let wins: Vec<f64> = returns.iter().copied().filter(|r| *r >= 0.0).collect();
    let avg = mean(&wins);
    if avg == 0.0 {
        return Err(FinanceError::Undefined {
            what: "mean nonnegative return",
        });
    }
    output(
        "outlier_win_ratio",
        quantile(returns, 0.99, QuantileMethod::Linear)? / avg,
    )
}
/// 1st percentile return divided by the mean negative return.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn outlier_loss_ratio(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 2)?;
    let losses: Vec<f64> = returns.iter().copied().filter(|r| *r < 0.0).collect();
    if losses.is_empty() {
        return Err(FinanceError::Undefined {
            what: "losing returns",
        });
    }
    output(
        "outlier_loss_ratio",
        quantile(returns, 0.01, QuantileMethod::Linear)? / mean(&losses),
    )
}
/// Total compounded return divided by the absolute maximum drawdown.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn recovery_factor(returns: &[f64]) -> Result<f64> {
    let dd = crate::perf::max_drawdown(returns)?.abs();
    if dd == 0.0 {
        return Err(FinanceError::Undefined {
            what: "maximum drawdown",
        });
    }
    output(
        "recovery_factor",
        crate::perf::compounded_return(returns)? / dd,
    )
}
/// Estimated probability that a strategy loses its capital, using the win-rate model.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn risk_of_ruin(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    let traded = returns.iter().filter(|r| **r != 0.0).count() as f64;
    if traded == 0.0 {
        return Err(FinanceError::Undefined {
            what: "nonzero returns",
        });
    }
    let wins = returns.iter().filter(|r| **r > 0.0).count() as f64 / traded;
    let losses = 1.0 - wins;
    if wins == 0.0 {
        return Ok(1.0);
    }
    output(
        "risk_of_ruin",
        (losses / (1.0 + wins)).powf(returns.len() as f64),
    )
}
/// Kelly fraction from empirical win rate and average win/loss payoff.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn kelly_criterion(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    let wins = returns.iter().filter(|r| **r > 0.0).count() as f64 / returns.len() as f64;
    let payoff = payoff_ratio(returns)?;
    output(
        "kelly_criterion",
        (wins - (1.0 - wins) / payoff).clamp(0.0, 1.0),
    )
}
/// Mean return divided by sample standard deviation.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn risk_return_ratio(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 2)?;
    output("risk_return_ratio", mean(returns) / std_dev(returns))
}
/// R-squared of cumulative log wealth regressed on a time trend.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn stability(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 2)?;
    if returns.iter().any(|r| *r <= -1.0) {
        return Err(FinanceError::InvalidInput {
            what: "returns",
            reason: "log wealth requires returns greater than -100%",
        });
    }
    let mut y = Vec::with_capacity(returns.len());
    let mut wealth = 1.0;
    for r in returns {
        wealth *= 1.0 + r;
        y.push(wealth.ln());
    }
    let ym = mean(&y);
    let xm = (y.len() - 1) as f64 / 2.0;
    let cov = y
        .iter()
        .enumerate()
        .map(|(i, v)| (i as f64 - xm) * (v - ym))
        .sum::<f64>();
    let xx = (0..y.len()).map(|i| (i as f64 - xm).powi(2)).sum::<f64>();
    let yy = y.iter().map(|v| (v - ym).powi(2)).sum::<f64>();
    if yy == 0.0 {
        return Err(FinanceError::Undefined {
            what: "cumulative log return variance",
        });
    }
    output("stability", cov * cov / (xx * yy))
}

/// Sharpe ratio using nonnegative observation weights.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn weighted_sharpe(returns: &[f64], weights: &[f64]) -> Result<f64> {
    same_len("returns", returns.len(), "weights", weights.len())?;
    check_returns("returns", returns, 2)?;
    all_finite("weights", weights)?;
    if weights.iter().any(|w| *w < 0.0) {
        return Err(FinanceError::InvalidInput {
            what: "weights",
            reason: "must be nonnegative",
        });
    }
    let total = weights.iter().sum::<f64>();
    if total <= 0.0 {
        return Err(FinanceError::Undefined { what: "weight sum" });
    }
    let m = returns.iter().zip(weights).map(|(r, w)| r * w).sum::<f64>() / total;
    let variance = returns
        .iter()
        .zip(weights)
        .map(|(r, w)| w * (r - m).powi(2))
        .sum::<f64>()
        / total;
    if variance <= 0.0 {
        return Err(FinanceError::Undefined {
            what: "weighted return variance",
        });
    }
    output("weighted_sharpe", m / variance.sqrt())
}

/// Probability-weighted Sharpe over outcome/value pairs.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn weighted_outcome_sharpe(outcomes: &[(f64, f64)]) -> Result<f64> {
    let (weights, returns): (Vec<f64>, Vec<f64>) = outcomes.iter().map(|(w, r)| (*w, *r)).unzip();
    weighted_sharpe(&returns, &weights)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weighted_ratios_validate_weights_and_variance() {
        assert!(weighted_sharpe(&[0.1, 0.2], &[1.0]).is_err());
        assert!(weighted_sharpe(&[0.1, 0.2], &[-1.0, 1.0]).is_err());
        assert!(weighted_sharpe(&[0.1, 0.2], &[0.0, 0.0]).is_err());
        assert!(weighted_sharpe(&[0.1, 0.1], &[1.0, 1.0]).is_err());
        assert!(weighted_sharpe(&[0.1, f64::NAN], &[1.0, 1.0]).is_err());
        assert!(weighted_outcome_sharpe(&[(1.0, 0.1), (1.0, 0.2)]).is_ok());
    }
}
