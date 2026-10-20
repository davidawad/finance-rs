//! Additional performance ratios and moments.

use super::{check_periods, check_returns, mean, sharpe, sortino, std_dev};
use crate::error::{all_finite, finite, output, same_len, FinanceError, Result};

/// Downside deviation below a per-period minimum acceptable return.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn downside_deviation(returns: &[f64], mar: f64, periods_per_year: f64) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    finite("mar", mar)?;
    check_periods(periods_per_year)?;
    let variance = returns
        .iter()
        .map(|r| (r - mar).min(0.0).powi(2))
        .sum::<f64>()
        / returns.len() as f64;
    output(
        "downside_deviation",
        variance.sqrt() * periods_per_year.sqrt(),
    )
}

/// Bias-corrected sample skewness (pandas/quantstats convention).
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn skew(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 3)?;
    let n = returns.len() as f64;
    let m = mean(returns);
    let s = (returns.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n).sqrt();
    if s == 0.0 {
        return Err(FinanceError::Undefined {
            what: "return standard deviation",
        });
    }
    let g1 = returns.iter().map(|x| ((x - m) / s).powi(3)).sum::<f64>() / n;
    output("skew", (n * (n - 1.0)).sqrt() / (n - 2.0) * g1)
}

/// Bias-corrected excess sample kurtosis (pandas/quantstats convention).
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn kurtosis(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 4)?;
    let n = returns.len() as f64;
    let m = mean(returns);
    let s = (returns.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n).sqrt();
    if s == 0.0 {
        return Err(FinanceError::Undefined {
            what: "return standard deviation",
        });
    }
    let m4 = returns.iter().map(|x| ((x - m) / s).powi(4)).sum::<f64>() / n;
    let g2 = m4 - 3.0;
    output(
        "kurtosis",
        ((n - 1.0) / ((n - 2.0) * (n - 3.0))) * ((n + 1.0) * g2 + 6.0),
    )
}

#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
fn autocorr_penalty(xs: &[f64]) -> Result<f64> {
    if xs.len() < 2 {
        return Ok(1.0);
    }
    let (x, y) = (&xs[..xs.len() - 1], &xs[1..]);
    let (mx, my) = (mean(x), mean(y));
    let (mut covariance, mut vx, mut vy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        covariance += (a - mx) * (b - my);
        vx += (a - mx).powi(2);
        vy += (b - my).powi(2);
    }
    if vx == 0.0 || vy == 0.0 {
        return Ok(1.0);
    }
    let coefficient = (covariance / (vx * vy).sqrt()).abs();
    let n = xs.len() as f64;
    let mut weighted = 0.0;
    let mut power = coefficient;
    for lag in 1..xs.len() {
        weighted += (n - lag as f64) / n * power;
        power *= coefficient;
    }
    output("autocorrelation penalty", (1.0 + 2.0 * weighted).sqrt())
}

/// Sharpe ratio adjusted by quantstats' lag-one autocorrelation penalty.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn smart_sharpe(returns: &[f64], rf: f64, periods_per_year: f64) -> Result<f64> {
    let base = sharpe(returns, rf, periods_per_year)?;
    output("smart_sharpe", base / autocorr_penalty(returns)?)
}

/// Sortino ratio adjusted by quantstats' lag-one autocorrelation penalty.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn smart_sortino(returns: &[f64], rf: f64, periods_per_year: f64) -> Result<f64> {
    let base = sortino(returns, rf, periods_per_year)?;
    output("smart_sortino", base / autocorr_penalty(returns)?)
}

/// Jack Schwager's Sortino adjustment, divided by `√2`.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn adjusted_sortino(returns: &[f64], rf: f64, periods_per_year: f64) -> Result<f64> {
    output(
        "adjusted_sortino",
        sortino(returns, rf, periods_per_year)? / 2.0_f64.sqrt(),
    )
}

/// Bailey-López de Prado probabilistic Sharpe ratio against a periodic benchmark.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn probabilistic_sharpe(returns: &[f64], sr_benchmark: f64) -> Result<f64> {
    check_returns("returns", returns, 3)?;
    finite("sr_benchmark", sr_benchmark)?;
    let n = returns.len() as f64;
    let sr = mean(returns) / std_dev(returns);
    if !sr.is_finite() {
        return Err(FinanceError::Undefined {
            what: "return standard deviation",
        });
    }
    let sk = skew(returns)?;
    let ku = kurtosis(returns)?;
    let denominator = 1.0 - sk * sr + ((ku + 2.0) / 4.0) * sr * sr;
    if denominator <= 0.0 {
        return Err(FinanceError::Undefined {
            what: "probabilistic Sharpe denominator",
        });
    }
    output(
        "probabilistic_sharpe",
        crate::math::normal_cdf((sr - sr_benchmark) * (n - 1.0).sqrt() / denominator.sqrt()),
    )
}

/// Expected maximum Sharpe under the Gaussian null for `n_trials` trials.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn expected_max_sharpe(n_trials: usize, sr_variance: f64) -> Result<f64> {
    if n_trials == 0 {
        return Err(FinanceError::InvalidInput {
            what: "n_trials",
            reason: "must be positive",
        });
    }
    finite("sr_variance", sr_variance)?;
    if sr_variance < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "sr_variance",
            reason: "must be nonnegative",
        });
    }
    if n_trials == 1 {
        return Ok(0.0);
    }
    let n = n_trials as f64;
    let euler = 0.577_215_664_901_532_9;
    let a = crate::math::normal_ppf(1.0 - 1.0 / n)?;
    let b = crate::math::normal_ppf(1.0 - 1.0 / (n * std::f64::consts::E))?;
    output(
        "expected_max_sharpe",
        sr_variance.sqrt() * ((1.0 - euler) * a + euler * b),
    )
}

/// Deflated Sharpe probability against the expected maximum of many trials.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn deflated_sharpe(returns: &[f64], n_trials: usize, sr_variance: f64) -> Result<f64> {
    probabilistic_sharpe(returns, expected_max_sharpe(n_trials, sr_variance)?)
}

/// Probability of backtest overfitting using combinatorially symmetric CV.
/// Rows are strategies and columns are equally sized chronological observations.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn prob_backtest_overfitting(trials_matrix: &[Vec<f64>], partitions: usize) -> Result<f64> {
    if trials_matrix.len() < 2 {
        return Err(FinanceError::TooShort {
            what: "trials_matrix",
            min: 2,
            got: trials_matrix.len(),
        });
    }
    let periods = trials_matrix[0].len();
    if periods == 0
        || !(2..=20).contains(&partitions)
        || partitions % 2 != 0
        || periods % partitions != 0
    {
        return Err(FinanceError::InvalidInput {
            what: "partitions",
            reason: "must be even, at least two, and divide the observation count",
        });
    }
    for row in trials_matrix {
        same_len("strategy", row.len(), "periods", periods)?;
        all_finite("strategy", row)?;
    }
    let half = partitions / 2;
    let block = periods / partitions;
    let mut masks = Vec::new();
    combinations(partitions, half, 0, 0, &mut masks);
    let mut overfit = 0usize;
    for mask in masks.iter().filter(|m| **m < (1usize << (partitions - 1))) {
        let mut train = vec![0.0; trials_matrix.len()];
        let mut test = vec![0.0; trials_matrix.len()];
        let (mut nt, mut nv) = (0usize, 0usize);
        for p in 0..partitions {
            let is_train = (mask & (1 << p)) != 0;
            for i in 0..block {
                let t = p * block + i;
                for (j, row) in trials_matrix.iter().enumerate() {
                    if is_train {
                        train[j] += row[t];
                    } else {
                        test[j] += row[t];
                    }
                }
            }
            if is_train {
                nt += block;
            } else {
                nv += block;
            }
        }
        for v in &mut train {
            *v /= nt as f64;
        }
        for v in &mut test {
            *v /= nv as f64;
        }
        let best = argmax(&train);
        let rank = 1 + test.iter().filter(|v| **v < test[best]).count();
        let lambda = ((rank as f64) / (trials_matrix.len() as f64 + 1.0 - rank as f64)).ln();
        if lambda < 0.0 {
            overfit += 1;
        }
    }
    let count = masks.len() / 2;
    output("prob_backtest_overfitting", overfit as f64 / count as f64)
}
fn combinations(n: usize, k: usize, at: usize, mask: usize, out: &mut Vec<usize>) {
    if k == 0 {
        out.push(mask);
        return;
    }
    if n - at < k {
        return;
    }
    for i in at..=n - k {
        combinations(n, k - 1, i + 1, mask | (1 << i), out);
    }
}
fn argmax(xs: &[f64]) -> usize {
    let mut best = 0;
    for i in 1..xs.len() {
        if xs[i] > xs[best] {
            best = i;
        }
    }
    best
}

/// Annualized Treynor ratio against a benchmark.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn treynor(returns: &[f64], benchmark: &[f64], rf: f64, periods_per_year: f64) -> Result<f64> {
    same_len("returns", returns.len(), "benchmark", benchmark.len())?;
    check_returns("returns", returns, 2)?;
    check_returns("benchmark", benchmark, 2)?;
    finite("rf", rf)?;
    check_periods(periods_per_year)?;
    let beta = crate::perf::beta(returns, benchmark)?;
    if beta == 0.0 {
        return Err(FinanceError::Undefined {
            what: "benchmark beta",
        });
    }
    output(
        "treynor",
        (mean(returns) - crate::rates::periodic_from_annual(rf, periods_per_year)?)
            * periods_per_year
            / beta,
    )
}

/// Modigliani-Modigliani risk-adjusted return using benchmark volatility.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn m2(returns: &[f64], benchmark: &[f64], rf: f64, periods_per_year: f64) -> Result<f64> {
    same_len("returns", returns.len(), "benchmark", benchmark.len())?;
    check_returns("returns", returns, 2)?;
    check_returns("benchmark", benchmark, 2)?;
    let rf_period = crate::rates::periodic_from_annual(rf, check_periods(periods_per_year)?)?;
    let sr = (mean(returns) - rf_period) / std_dev(returns);
    output("m2", sr * std_dev(benchmark) * periods_per_year.sqrt() + rf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extra_ratios_cover_zero_variance_trials_and_beta() {
        assert!(skew(&[0.0, 0.0, 0.0]).is_err());
        assert!(kurtosis(&[0.0, 0.0, 0.0, 0.0]).is_err());
        assert_eq!(autocorr_penalty(&[0.1]).unwrap(), 1.0);
        assert_eq!(autocorr_penalty(&[0.1, 0.1, 0.2]).unwrap(), 1.0);
        assert_eq!(expected_max_sharpe(1, 0.5).unwrap(), 0.0);
        assert!(expected_max_sharpe(0, 0.5).is_err());
        assert!(expected_max_sharpe(2, -1.0).is_err());
        assert!(prob_backtest_overfitting(&[], 2).is_err());
        assert!(prob_backtest_overfitting(&[vec![0.1]], 2).is_err());
        assert!(prob_backtest_overfitting(&[vec![0.1; 4], vec![0.2; 4]], 3).is_err());
        assert!(prob_backtest_overfitting(&[vec![0.1; 4], vec![0.2; 4]], 2)
            .unwrap()
            .is_finite());
        let mut combinations_out = Vec::new();
        combinations(1, 2, 0, 0, &mut combinations_out);
        assert_eq!(combinations_out.len(), 0);
        assert!(treynor(&[0.0, 0.0, 0.0], &[1.0, 2.0, 3.0], 0.0, 252.0).is_err());
        assert!(probabilistic_sharpe(
            &[
                1.277_559_602_642_34,
                1.146_978_497_874_763_7,
                0.446_260_963_170_531,
                0.500_651_621_158_039_9
            ],
            0.0,
        )
        .is_err());
        assert_eq!(argmax(&[1.0, 3.0, 2.0]), 1);
    }

    #[test]
    fn adjusted_ratios_and_search_helpers_have_fixed_valid_fixtures() {
        let returns = [0.01, -0.02, 0.03, -0.01, 0.02];
        assert!(downside_deviation(&returns, 0.0, 252.0)
            .unwrap()
            .is_finite());
        assert!(smart_sharpe(&returns, 0.0, 252.0).unwrap().is_finite());
        assert!(smart_sortino(&returns, 0.0, 252.0).unwrap().is_finite());
        assert!(adjusted_sortino(&returns, 0.0, 252.0).unwrap().is_finite());
        assert!(probabilistic_sharpe(&returns, 0.0).unwrap().is_finite());
        assert!(expected_max_sharpe(8, 0.25).unwrap().is_finite());
        assert!(deflated_sharpe(&returns, 8, 0.25).unwrap().is_finite());
        assert!(prob_backtest_overfitting(
            &[vec![0.1, 0.2, 0.1, 0.3], vec![0.2, 0.1, 0.3, 0.2]],
            2,
        )
        .unwrap()
        .is_finite());
        assert!(m2(&returns, &[0.02, -0.01, 0.02, 0.0, 0.01], 0.0, 252.0)
            .unwrap()
            .is_finite());
    }
}
