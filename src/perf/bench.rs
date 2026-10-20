//! Benchmark-relative metrics: beta, alpha, information ratio.

use super::{check_periods, check_returns, mean, std_dev};
use crate::error::{div, output, same_len, Result};

/// Validate an aligned strategy/benchmark pair with at least 2 observations.
fn check_pair(returns: &[f64], benchmark: &[f64]) -> Result<()> {
    same_len("returns", returns.len(), "benchmark", benchmark.len())?;
    check_returns("returns", returns, 2)?;
    check_returns("benchmark", benchmark, 2)
}

/// `(cov(r, b), var(b))`, both with `ddof = 1` (numpy `cov`).
#[allow(clippy::cast_precision_loss)]
fn cov_var(returns: &[f64], benchmark: &[f64]) -> (f64, f64) {
    let (mr, mb) = (mean(returns), mean(benchmark));
    let (mut cov, mut var) = (0.0, 0.0);
    for (r, b) in returns.iter().zip(benchmark) {
        cov += (r - mr) * (b - mb);
        var += (b - mb) * (b - mb);
    }
    let dof = (returns.len() - 1) as f64;
    (cov / dof, var / dof)
}

/// Beta against a benchmark (quantstats `greeks()["beta"]`).
///
/// `β = cov(r, b) / var(b)` (sample moments, `ddof = 1`)
///
/// ```
/// use finance_rs::perf::beta;
/// let b = [0.01, -0.02, 0.015, 0.003];
/// let r: Vec<f64> = b.iter().map(|x| 2.0 * x + 0.001).collect();
/// assert!((beta(&r, &b)? - 2.0).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::LengthMismatch`](crate::FinanceError::LengthMismatch) for unaligned series,
/// [`FinanceError::TooShort`](crate::FinanceError::TooShort) for fewer than 2 observations,
/// [`FinanceError::NonFinite`](crate::FinanceError::NonFinite) for non-finite input,
/// [`FinanceError::Undefined`](crate::FinanceError::Undefined) for a zero-variance benchmark (quantstats
/// fills that `NaN` with 0).
pub fn beta(returns: &[f64], benchmark: &[f64]) -> Result<f64> {
    check_pair(returns, benchmark)?;
    let (cov, var) = cov_var(returns, benchmark);
    div("benchmark variance", cov, var)
}

/// Annualized alpha against a benchmark (quantstats `greeks()["alpha"]`).
///
/// `α = (mean(r) − β · mean(b)) · periods_per_year`
///
/// This is quantstats' simple (arithmetic) annualization with no risk-free
/// rate; empyrical's `alpha` instead compounds, `(1 + α_period)^N − 1`, on
/// excess returns.
///
/// ```
/// use finance_rs::perf::alpha;
/// let b = [0.01, -0.02, 0.015, 0.003];
/// let r: Vec<f64> = b.iter().map(|x| 2.0 * x + 0.001).collect();
/// assert!((alpha(&r, &b, 252.0)? - 0.252).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`beta`], plus [`FinanceError::InvalidInput`](crate::FinanceError::InvalidInput) for `periods_per_year <= 0`.
pub fn alpha(returns: &[f64], benchmark: &[f64], periods_per_year: f64) -> Result<f64> {
    check_periods(periods_per_year)?;
    let b = beta(returns, benchmark)?;
    output(
        "alpha",
        (mean(returns) - b * mean(benchmark)) * periods_per_year,
    )
}

/// Information ratio (quantstats `information_ratio`).
///
/// `IR = mean(r − b) / std(r − b, ddof=1)`
///
/// Like quantstats this is the **per-period** ratio and is not annualized;
/// multiply by `√periods_per_year` for an annual figure.
///
/// ```
/// use finance_rs::perf::information_ratio;
/// let ir = information_ratio(&[0.02, 0.0, 0.03], &[0.01, 0.0, 0.01])?;
/// assert!((ir - 1.0).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`beta`], with [`FinanceError::Undefined`](crate::FinanceError::Undefined) for a zero tracking error
/// (quantstats returns 0).
pub fn information_ratio(returns: &[f64], benchmark: &[f64]) -> Result<f64> {
    check_pair(returns, benchmark)?;
    let active: Vec<f64> = returns.iter().zip(benchmark).map(|(r, b)| r - b).collect();
    div("tracking error", mean(&active), std_dev(&active))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FinanceError;

    #[test]
    fn rejects_bad_pairs() {
        assert!(matches!(
            beta(&[0.1, 0.2], &[0.1]),
            Err(FinanceError::LengthMismatch { .. })
        ));
        assert!(matches!(
            beta(&[0.1, 0.2], &[0.1, 0.1]),
            Err(FinanceError::Undefined { .. })
        ));
        assert!(matches!(
            information_ratio(&[0.1, 0.2], &[0.1, 0.2]),
            Err(FinanceError::Undefined { .. })
        ));
        assert!(matches!(
            alpha(&[0.1, 0.2], &[0.1, 0.3], -1.0),
            Err(FinanceError::InvalidInput { .. })
        ));
    }
}
