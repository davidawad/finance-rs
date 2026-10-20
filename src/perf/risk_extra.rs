//! Additional risk estimators and portfolio VaR decompositions.
use super::{check_returns, mean, std_dev, tail_probability};
use crate::error::{all_finite, output, same_len, FinanceError, Result};

/// Cornish-Fisher modified VaR, returned as a return threshold (losses negative).
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn var_cornish_fisher(returns: &[f64], confidence: f64) -> Result<f64> {
    check_returns("returns", returns, 3)?;
    let alpha = tail_probability(confidence)?;
    let mu = mean(returns);
    let sd = std_dev(returns);
    if sd == 0.0 {
        return Err(FinanceError::Undefined {
            what: "return variance",
        });
    }
    let skew = crate::perf::skew(returns)?;
    let kurtosis = crate::perf::kurtosis(returns)?;
    let z = crate::math::normal_ppf(alpha)?;
    let z_cf = z + (z * z - 1.0) * skew / 6.0 + (z.powi(3) - 3.0 * z) * kurtosis / 24.0
        - (2.0 * z.powi(3) - 5.0 * z) * skew * skew / 36.0;
    output("var_cornish_fisher", mu + sd * z_cf)
}

/// Cornish-Fisher expected shortfall approximation using the adjusted quantile.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn cvar_cornish_fisher(returns: &[f64], confidence: f64) -> Result<f64> {
    let q = var_cornish_fisher(returns, confidence)?;
    let left: Vec<f64> = returns.iter().copied().filter(|r| *r <= q).collect();
    if left.is_empty() {
        return Ok(q);
    }
    output("cvar_cornish_fisher", mean(&left))
}

/// EWMA volatility Gaussian VaR (RiskMetrics), with the newest observation weighted most.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn var_ewma(returns: &[f64], lambda: f64, confidence: f64) -> Result<f64> {
    check_returns("returns", returns, 2)?;
    crate::error::finite("lambda", lambda)?;
    if !(0.0..1.0).contains(&lambda) {
        return Err(FinanceError::InvalidInput {
            what: "lambda",
            reason: "must be in (0, 1)",
        });
    }
    let mu = mean(returns);
    let variance = returns
        .iter()
        .rev()
        .fold(0.0, |v, r| lambda * v + (1.0 - lambda) * (r - mu).powi(2));
    let z = crate::math::normal_ppf(tail_probability(confidence)?)?;
    output("var_ewma", mu + variance.sqrt() * z)
}

/// Square-root-of-time scaling of VaR for a positive horizon.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn risk_scale_var(var: f64, horizon: f64) -> Result<f64> {
    crate::error::finite("var", var)?;
    crate::error::finite("horizon", horizon)?;
    if horizon <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "horizon",
            reason: "must be positive",
        });
    }
    output("scale_var", var * horizon.sqrt())
}

/// Component VaR contributions for a weight vector and row-major covariance matrix.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn risk_component_var(
    weights: &[f64],
    covariance: &[&[f64]],
    confidence: f64,
) -> Result<Vec<f64>> {
    let n = weights.len();
    if n == 0 || covariance.len() != n || covariance.iter().any(|r| r.len() != n) {
        return Err(FinanceError::InvalidInput {
            what: "covariance",
            reason: "must be a non-empty square matrix matching weights",
        });
    }
    all_finite("weights", weights)?;
    for row in covariance {
        all_finite("covariance", row)?;
    }
    let z = -crate::math::normal_ppf(tail_probability(confidence)?)?;
    let marginal: Vec<f64> = (0..n)
        .map(|i| (0..n).map(|j| covariance[i][j] * weights[j]).sum())
        .collect();
    let variance: f64 = weights.iter().zip(&marginal).map(|(w, m)| w * m).sum();
    if variance <= 0.0 {
        return Err(FinanceError::Undefined {
            what: "portfolio variance",
        });
    }
    let sigma = variance.sqrt();
    marginal
        .iter()
        .zip(weights)
        .map(|(m, w)| output("component_var", z * w * m / sigma))
        .collect()
}

/// Marginal VaR by asset (derivative of portfolio VaR with respect to weight).
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn risk_marginal_var(
    weights: &[f64],
    covariance: &[&[f64]],
    confidence: f64,
) -> Result<Vec<f64>> {
    let n = weights.len();
    if n == 0 || covariance.len() != n || covariance.iter().any(|r| r.len() != n) {
        return Err(FinanceError::InvalidInput {
            what: "covariance",
            reason: "must be a non-empty square matrix matching weights",
        });
    }
    all_finite("weights", weights)?;
    for row in covariance {
        all_finite("covariance", row)?;
    }
    let z = -crate::math::normal_ppf(tail_probability(confidence)?)?;
    let covw: Vec<f64> = (0..n)
        .map(|i| (0..n).map(|j| covariance[i][j] * weights[j]).sum())
        .collect();
    let variance: f64 = weights.iter().zip(&covw).map(|(w, m)| w * m).sum();
    if variance <= 0.0 {
        return Err(FinanceError::Undefined {
            what: "portfolio variance",
        });
    }
    covw.iter()
        .map(|m| output("marginal_var", z * m / variance.sqrt()))
        .collect()
}

/// Empyrical beta fragility heuristic: maximum change in beta after dropping one observation.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn beta_fragility(returns: &[f64], benchmark: &[f64]) -> Result<f64> {
    same_len("returns", returns.len(), "benchmark", benchmark.len())?;
    check_returns("returns", returns, 3)?;
    all_finite("benchmark", benchmark)?;
    let base = crate::perf::beta(returns, benchmark)?;
    let mut max_change: f64 = 0.0;
    for i in 0..returns.len() {
        let r: Vec<f64> = returns
            .iter()
            .enumerate()
            .filter_map(|(j, x)| (i != j).then_some(*x))
            .collect();
        let b: Vec<f64> = benchmark
            .iter()
            .enumerate()
            .filter_map(|(j, x)| (i != j).then_some(*x))
            .collect();
        max_change = max_change.max((crate::perf::beta(&r, &b)? - base).abs());
    }
    output("beta_fragility", max_change)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extra_risk_estimators_reject_degenerate_inputs() {
        assert!(var_cornish_fisher(&[0.1, 0.1, 0.1], 0.95).is_err());
        assert!(var_cornish_fisher(&[0.0, 0.0, 0.0], 0.95).is_err());
        assert!(cvar_cornish_fisher(&[-0.02, 0.01, 0.03, 0.04], 0.999_999).is_ok());
        assert!(cvar_cornish_fisher(&[-0.2, -0.1, 0.05, 0.15, 0.3], 0.95).is_ok());
        assert!(cvar_cornish_fisher(&[-0.2, -0.1, 0.05, 0.15, 0.3], 0.6).is_ok());
        assert!(risk_scale_var(-0.1, 0.0).is_err());
        let zero = [&[0.0][..]];
        assert!(risk_component_var(&[1.0], &zero, 0.95).is_err());
        assert!(risk_marginal_var(&[1.0], &zero, 0.95).is_err());
        assert!(risk_component_var(&[], &[], 0.95).is_err());
        assert!(beta_fragility(&[0.1, 0.2, 0.3], &[0.1, 0.1]).is_err());
    }

    #[test]
    fn extra_risk_estimators_cover_finite_portfolio_cases() {
        let returns = [-0.04, 0.01, 0.02, 0.03, -0.01];
        assert!(var_ewma(&returns, 0.94, 0.95).unwrap().is_finite());
        let covariance = [&[0.04, 0.01][..], &[0.01, 0.09][..]];
        let weights = [0.6, 0.4];
        let component = risk_component_var(&weights, &covariance, 0.95).unwrap();
        let marginal = risk_marginal_var(&weights, &covariance, 0.95).unwrap();
        assert_eq!(component.len(), 2);
        assert_eq!(marginal.len(), 2);
        let portfolio_variance =
            0.6_f64.powi(2) * 0.04 + 2.0 * 0.6 * 0.4 * 0.01 + 0.4_f64.powi(2) * 0.09;
        let expected = -crate::math::normal_ppf(0.05).unwrap() * portfolio_variance.sqrt();
        assert!((component.iter().sum::<f64>() - expected).abs() < 1e-12);
        assert!(marginal.iter().all(|value| value.is_finite()));
        let benchmark = [-0.03, 0.015, 0.01, 0.025, -0.005];
        assert!(beta_fragility(&returns, &benchmark).unwrap().is_finite());
    }
}
