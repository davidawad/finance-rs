//! Benchmark-relative performance extensions.
use super::{check_periods, check_returns, mean, std_dev};
use crate::error::{all_finite, same_len, FinanceError, Result};

fn aligned(r: &[f64], b: &[f64]) -> Result<()> {
    same_len("returns", r.len(), "benchmark", b.len())?;
    check_returns("returns", r, 2)?;
    all_finite("benchmark", b)
}

/// Annualized sample tracking error of active returns.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn tracking_error(r: &[f64], b: &[f64], ppy: f64) -> Result<f64> {
    aligned(r, b)?;
    check_periods(ppy)?;
    crate::error::output(
        "tracking_error",
        std_dev(&r.iter().zip(b).map(|(x, y)| x - y).collect::<Vec<_>>()) * ppy.sqrt(),
    )
}

/// Squared Pearson correlation with the benchmark.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn r_squared(r: &[f64], b: &[f64]) -> Result<f64> {
    aligned(r, b)?;
    let corr = crate::stats::correlation(r, b)?;
    Ok(corr * corr)
}

/// Fraction of benchmark-up periods where the portfolio return is positive.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn batting_average(r: &[f64], b: &[f64]) -> Result<f64> {
    aligned(r, b)?;
    let up: Vec<bool> = b
        .iter()
        .zip(r)
        .filter(|(x, _)| **x > 0.0)
        .map(|(_, y)| *y > 0.0)
        .collect();
    if up.is_empty() {
        return Err(FinanceError::Undefined {
            what: "benchmark up periods",
        });
    }
    Ok(up.iter().filter(|x| **x).count() as f64 / up.len() as f64)
}

#[derive(Clone, Copy)]
enum MarketRegime {
    Up,
    Down,
}

#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
fn capture(r: &[f64], b: &[f64], regime: MarketRegime) -> Result<f64> {
    aligned(r, b)?;
    let selected: Vec<(f64, f64)> = r
        .iter()
        .copied()
        .zip(b.iter().copied())
        .filter(|(_, x)| match regime {
            MarketRegime::Up => *x > 0.0,
            MarketRegime::Down => *x < 0.0,
        })
        .collect();
    if selected.is_empty() {
        return Err(FinanceError::Undefined {
            what: "benchmark capture periods",
        });
    }
    let rm = selected.iter().map(|x| x.0).sum::<f64>() / selected.len() as f64;
    let bm = selected.iter().map(|x| x.1).sum::<f64>() / selected.len() as f64;
    // Every selected benchmark observation has the regime's strict sign, so
    // its arithmetic mean cannot be zero.
    crate::error::output("capture", rm / bm)
}
/// Up-market capture ratio (mean portfolio return / mean benchmark return on up periods).
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn up_capture(r: &[f64], b: &[f64]) -> Result<f64> {
    capture(r, b, MarketRegime::Up)
}
/// Down-market capture ratio (mean portfolio return / mean benchmark return on down periods).
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn down_capture(r: &[f64], b: &[f64]) -> Result<f64> {
    capture(r, b, MarketRegime::Down)
}
/// Up capture divided by absolute down capture.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn up_down_capture(r: &[f64], b: &[f64]) -> Result<f64> {
    crate::error::output(
        "up_down_capture",
        up_capture(r, b)? / down_capture(r, b)?.abs(),
    )
}

fn conditional_alpha_beta(r: &[f64], b: &[f64], regime: MarketRegime) -> Result<(f64, f64)> {
    aligned(r, b)?;
    let pairs: Vec<(f64, f64)> = r
        .iter()
        .copied()
        .zip(b.iter().copied())
        .filter(|(_, x)| match regime {
            MarketRegime::Up => *x > 0.0,
            MarketRegime::Down => *x < 0.0,
        })
        .collect();
    if pairs.len() < 2 {
        return Err(FinanceError::TooShort {
            what: "conditional benchmark observations",
            min: 2,
            got: pairs.len(),
        });
    }
    let (y, x): (Vec<f64>, Vec<f64>) = pairs.into_iter().unzip();
    let fit = crate::stats::ols(&y, &[&x])?;
    Ok((fit.coef[0], fit.coef[1]))
}
/// Conditional regression intercept and beta in benchmark-up periods.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn up_alpha_beta(r: &[f64], b: &[f64]) -> Result<(f64, f64)> {
    conditional_alpha_beta(r, b, MarketRegime::Up)
}
/// Conditional regression intercept and beta in benchmark-down periods.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn down_alpha_beta(r: &[f64], b: &[f64]) -> Result<(f64, f64)> {
    conditional_alpha_beta(r, b, MarketRegime::Down)
}

/// Sharpe ratio of active returns with explicit periods per year.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn excess_sharpe(r: &[f64], b: &[f64], ppy: f64) -> Result<f64> {
    aligned(r, b)?;
    check_periods(ppy)?;
    let active: Vec<f64> = r.iter().zip(b).map(|(x, y)| x - y).collect();
    let sd = std_dev(&active);
    if sd == 0.0 {
        return Err(FinanceError::Undefined {
            what: "active return variance",
        });
    }
    crate::error::output("excess_sharpe", mean(&active) / sd * ppy.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_metrics_cover_empty_regimes_and_regressions() {
        let r = [0.02, 0.03, 0.04, -0.02, -0.01, -0.03];
        let b = [0.01, 0.02, 0.03, -0.01, -0.02, -0.03];
        assert!(batting_average(&r, &[0.0; 6]).is_err());
        assert!(up_capture(&r, &[-0.1; 6]).is_err());
        assert!(down_capture(&r, &[0.1; 6]).is_err());
        assert!(up_alpha_beta(&r, &[0.1, -0.1, -0.2, -0.3, -0.4, -0.5]).is_err());
        assert!(down_alpha_beta(&r, &[0.1, 0.2, -0.1, 0.1, 0.2, 0.3]).is_err());
        assert!(up_alpha_beta(&r, &b).unwrap().1.is_finite());
        assert!(down_alpha_beta(&r, &b).unwrap().1.is_finite());
        assert!(up_down_capture(&r, &b).unwrap().is_finite());
        assert!(excess_sharpe(&[0.1, 0.2], &[0.1, 0.2], 252.0).is_err());
        assert!(excess_sharpe(&[0.1, 0.3, 0.2], &[0.05, 0.1, 0.1], 252.0)
            .unwrap()
            .is_finite());
        assert!(r_squared(&r, &b).unwrap().is_finite());
    }

    #[test]
    fn benchmark_metrics_have_fixed_valid_fixtures() {
        let returns = [0.03, -0.01, 0.04, -0.02, 0.02, -0.03];
        let benchmark = [0.02, -0.02, 0.03, -0.01, 0.01, -0.03];
        assert!(tracking_error(&returns, &benchmark, 252.0)
            .unwrap()
            .is_finite());
        assert!(batting_average(&returns, &benchmark).unwrap() > 0.0);
        assert!(up_alpha_beta(&returns, &benchmark).unwrap().0.is_finite());
        assert!(down_alpha_beta(&returns, &benchmark).unwrap().1.is_finite());
        assert!(excess_sharpe(&returns, &benchmark, 252.0)
            .unwrap()
            .is_finite());
    }
}
