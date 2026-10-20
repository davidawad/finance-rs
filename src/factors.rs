//! Multi-factor return attribution. Factor data are supplied by the caller.
use crate::error::{all_finite, same_len, FinanceError, Result};
use crate::stats::Ols;

/// Estimate intercept and factor exposures by OLS.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::factors::exposures), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn exposures(returns: &[f64], factor_returns: &[&[f64]]) -> Result<Ols> {
    if factor_returns.is_empty() {
        return Err(FinanceError::TooShort {
            what: "factor_returns",
            min: 1,
            got: 0,
        });
    }
    crate::stats::ols(returns, factor_returns)
}

/// Factor contribution averages: intercept and each beta times mean factor return.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::factors::attribution), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn attribution(exposure: &Ols, factor_returns: &[&[f64]]) -> Result<Vec<f64>> {
    if exposure.coef.len() != factor_returns.len() + 1 {
        return Err(FinanceError::InvalidInput {
            what: "exposures",
            reason: "coefficient count must equal factors plus intercept",
        });
    }
    let n = factor_returns.first().map_or(0, |x| x.len());
    if n == 0 {
        return Err(FinanceError::TooShort {
            what: "factor_returns",
            min: 1,
            got: 0,
        });
    }
    let mut out = Vec::with_capacity(factor_returns.len() + 1);
    out.push(exposure.coef[0]);
    for (j, f) in factor_returns.iter().enumerate() {
        same_len("factor_returns", n, "factor", f.len())?;
        all_finite("factor_returns", f)?;
        out.push(exposure.coef[j + 1] * f.iter().sum::<f64>() / n as f64);
    }
    Ok(out)
}

/// Intercept from a three- or five-factor regression on caller-provided French data.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::factors::fama_french_alpha), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn fama_french_alpha(returns: &[f64], ff_factors: &[&[f64]]) -> Result<f64> {
    if ff_factors.len() != 3 && ff_factors.len() != 5 {
        return Err(FinanceError::InvalidInput {
            what: "ff_factors",
            reason: "expected three or five factor series",
        });
    }
    Ok(exposures(returns, ff_factors)?.coef[0])
}

/// Rolling benchmark beta. Each output aligns with the final sample in its window.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::factors::rolling_beta), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn rolling_beta(returns: &[f64], benchmark: &[f64], window: usize) -> Result<Vec<f64>> {
    same_len("returns", returns.len(), "benchmark", benchmark.len())?;
    all_finite("returns", returns)?;
    all_finite("benchmark", benchmark)?;
    if window < 2 || returns.len() < window {
        return Err(FinanceError::TooShort {
            what: "window",
            min: 2,
            got: window,
        });
    }
    returns
        .windows(window)
        .zip(benchmark.windows(window))
        .map(|(r, b)| crate::perf::beta(r, b))
        .collect()
}
