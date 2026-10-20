//! Value at risk and conditional value at risk (expected shortfall),
//! historical and parametric (Gaussian).
//!
//! All four return a per-period *return* at the `1 − confidence` tail, so a
//! loss is negative (quantstats' and empyrical's sign convention).

use super::{check_returns, mean, normal_pdf, normal_ppf, std_dev, tail_probability};
use crate::error::{output, Result};

/// Portfolio risk decomposition and horizon scaling APIs.
pub use super::risk_extra::{
    risk_component_var as component_var, risk_marginal_var as marginal_var,
    risk_scale_var as scale_var,
};

/// numpy's default (`linear`) quantile of `xs` at `q`, exactly as
/// `np.percentile` / `pandas.Series.quantile` compute it.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
#[allow(clippy::many_single_char_names)] // Conventional symbols follow the published formula or recurrence notation.
fn quantile_linear(xs: &[f64], q: f64) -> f64 {
    let mut s = xs.to_vec();
    s.sort_by(f64::total_cmp);
    let h = (s.len() - 1) as f64 * q;
    let lo = h.floor();
    let i = (lo as usize).min(s.len() - 1);
    let j = (i + 1).min(s.len() - 1);
    let t = h - lo;
    let (a, b) = (s[i], s[j]);
    let diff = b - a;
    if t >= 0.5 {
        b - diff * (1.0 - t)
    } else {
        a + diff * t
    }
}

/// Historical VaR (empyrical `value_at_risk(returns, cutoff = 1 − confidence)`).
///
/// The `1 − confidence` quantile of the returns with numpy's linear
/// interpolation between order statistics:
///
/// ```text
/// h = (n − 1)·(1 − confidence),  VaR = x_(⌊h⌋) + (h − ⌊h⌋)·(x_(⌊h⌋+1) − x_(⌊h⌋))
/// ```
///
/// quantstats has no historical VaR (its `value_at_risk` is parametric).
///
/// ```
/// use finance_rs::perf::var_historical;
/// let r = [-0.05, -0.01, 0.0, 0.01, 0.02];
/// // h = 4 × 0.25 = 1 -> the second-smallest return.
/// assert!((var_historical(&r, 0.75)? + 0.01).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`](crate::FinanceError::TooShort) for an empty series, [`FinanceError::NonFinite`](crate::FinanceError::NonFinite)
/// for non-finite input, [`FinanceError::InvalidInput`](crate::FinanceError::InvalidInput) unless
/// `0 < confidence < 1`.
pub fn var_historical(returns: &[f64], confidence: f64) -> Result<f64> {
    let alpha = tail_probability(confidence)?;
    check_returns("returns", returns, 1)?;
    output("var_historical", quantile_linear(returns, alpha))
}

/// Historical CVaR / expected shortfall (quantstats
/// `conditional_value_at_risk(method="historical")`).
///
/// The mean of every return at or below [`var_historical`]:
/// `CVaR = mean{ r_t : r_t <= VaR }`.
///
/// ```
/// use finance_rs::perf::cvar_historical;
/// let r = [-0.05, -0.01, 0.0, 0.01, 0.02];
/// assert!((cvar_historical(&r, 0.75)? + 0.03).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`var_historical`].
pub fn cvar_historical(returns: &[f64], confidence: f64) -> Result<f64> {
    let threshold = var_historical(returns, confidence)?;
    // The minimum is always <= the quantile, so the tail is never empty.
    let tail: Vec<f64> = returns
        .iter()
        .copied()
        .filter(|r| *r <= threshold)
        .collect();
    output("cvar_historical", mean(&tail))
}

/// Historical loss VaR/CVaR using the legacy floor-index quantile contract.
/// Returns positive loss magnitudes and is retained for existing metric APIs.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn historical_var_cvar_floor(returns: &[f64], confidence: f64) -> Result<(f64, f64)> {
    tail_probability(confidence)?;
    check_returns("returns", returns, 1)?;
    let mut sorted = returns.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = ((1.0 - confidence) * sorted.len() as f64).floor() as usize;
    let threshold = sorted[index.min(sorted.len() - 1)];
    let tail = sorted.iter().filter(|value| **value <= threshold).copied();
    let tail_count = tail.clone().count();
    let cvar = -tail.sum::<f64>() / tail_count as f64;
    Ok((-threshold, cvar))
}

/// Parametric (Gaussian, variance-covariance) VaR (quantstats `value_at_risk`,
/// `sigma = 1`).
///
/// `VaR = μ + σ·Φ⁻¹(1 − confidence)`, with `μ` the mean and `σ` the sample
/// standard deviation (`ddof = 1`) of the returns.
///
/// ```
/// use finance_rs::perf::var_parametric;
/// let r = [0.01, -0.02, 0.03, -0.01, 0.02];
/// assert!((var_parametric(&r, 0.95)? + 0.028_108_410_770_087_598).abs() < 1e-14);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`](crate::FinanceError::TooShort) for fewer than 2 returns,
/// [`FinanceError::NonFinite`](crate::FinanceError::NonFinite) for non-finite input,
/// [`FinanceError::InvalidInput`](crate::FinanceError::InvalidInput) unless `0 < confidence < 1`,
/// [`FinanceError::Undefined`](crate::FinanceError::Undefined) for zero volatility (scipy's `norm.ppf`
/// returns `NaN` for a zero scale).
pub fn var_parametric(returns: &[f64], confidence: f64) -> Result<f64> {
    let alpha = tail_probability(confidence)?;
    check_returns("returns", returns, 2)?;
    let sd = std_dev(returns);
    if sd == 0.0 {
        return Err(crate::FinanceError::Undefined { what: "volatility" });
    }
    output("var_parametric", mean(returns) + sd * normal_ppf(alpha)?)
}

/// Parametric (Gaussian) CVaR / expected shortfall (quantstats
/// `conditional_value_at_risk(method="parametric")`, `sigma = 1`).
///
/// `CVaR = μ − σ·φ(Φ⁻¹(α))/α`, `α = 1 − confidence`; the mean `μ` itself
/// when `σ = 0` (as quantstats).
///
/// ```
/// use finance_rs::perf::{cvar_parametric, var_parametric};
/// let r = [0.01, -0.02, 0.03, -0.01, 0.02];
/// let es = cvar_parametric(&r, 0.95)?;
/// assert!((es + 0.036_773_323_161_635_71).abs() < 1e-14);
/// assert!(es < var_parametric(&r, 0.95)?);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`](crate::FinanceError::TooShort) for fewer than 2 returns,
/// [`FinanceError::NonFinite`](crate::FinanceError::NonFinite) for non-finite input,
/// [`FinanceError::InvalidInput`](crate::FinanceError::InvalidInput) unless `0 < confidence < 1`.
pub fn cvar_parametric(returns: &[f64], confidence: f64) -> Result<f64> {
    let alpha = tail_probability(confidence)?;
    check_returns("returns", returns, 2)?;
    let (mu, sd) = (mean(returns), std_dev(returns));
    if sd == 0.0 {
        return Ok(mu);
    }
    output(
        "cvar_parametric",
        mu - sd * normal_pdf(normal_ppf(alpha)?) / alpha,
    )
}

/// Gaussian loss VaR/CVaR with population standard deviation, matching the
/// original invest-metrics metric contract.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn parametric_var_cvar_population(returns: &[f64], confidence: f64) -> Result<(f64, f64)> {
    let alpha = tail_probability(confidence)?;
    check_returns("returns", returns, 1)?;
    let mu = mean(returns);
    let variance = returns
        .iter()
        .map(|value| (value - mu).powi(2))
        .sum::<f64>()
        / returns.len() as f64;
    let sigma = variance.sqrt();
    let z = normal_ppf(alpha)?;
    let density = normal_pdf(z);
    Ok((-(mu + z * sigma), -mu + sigma * density / alpha))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::FinanceError;

    #[test]
    fn quantile_matches_numpy_linear() {
        let xs = [3.0, 1.0, 2.0, 4.0];
        assert_eq!(quantile_linear(&xs, 0.0), 1.0);
        assert_eq!(quantile_linear(&xs, 1.0), 4.0);
        assert!((quantile_linear(&xs, 0.5) - 2.5).abs() < 1e-15);
        assert!((quantile_linear(&xs, 0.1) - 1.3).abs() < 1e-15);
        assert_eq!(quantile_linear(&[7.0], 0.05), 7.0);
    }

    #[test]
    fn rejects_bad_confidence() {
        let r = [0.01, -0.02, 0.03];
        for c in [0.0, 1.0, 95.0, -0.5] {
            assert!(matches!(
                var_historical(&r, c),
                Err(FinanceError::InvalidInput { .. })
            ));
            assert!(matches!(
                cvar_parametric(&r, c),
                Err(FinanceError::InvalidInput { .. })
            ));
        }
        assert!(matches!(
            var_parametric(&[0.0; 3], 0.95),
            Err(FinanceError::Undefined { .. })
        ));
        assert_eq!(cvar_parametric(&[0.0; 3], 0.95).unwrap(), 0.0);
    }

    #[test]
    #[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
    fn legacy_floor_var_and_population_var_match_metric_contract() {
        let returns = [-0.05, -0.01, 0.0, 0.01, 0.02];
        let historical = historical_var_cvar_floor(&returns, 0.7).unwrap();
        assert!((historical.0 - 0.01).abs() < 1e-14);
        assert!((historical.1 - 0.03).abs() < 1e-14);
        let pair = parametric_var_cvar_population(&returns, 0.8).unwrap();
        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let sigma = (returns
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / returns.len() as f64)
            .sqrt();
        let z = normal_ppf(0.2).unwrap();
        assert!((pair.0 + mean + z * sigma).abs() < 1e-14);
        assert!((pair.1 + mean - sigma * normal_pdf(z) / 0.2).abs() < 1e-14);
    }
}
