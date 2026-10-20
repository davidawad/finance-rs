//! Risk-adjusted return ratios: Sharpe, Sortino, Calmar, Omega.

use super::{cagr, check_periods, check_returns, excess_returns, max_drawdown, mean, std_dev};
use crate::error::{div, finite, output, FinanceError, Result};

/// Annualized Sharpe ratio (quantstats `sharpe`, `smart=False`).
///
/// ```text
/// x_t    = r_t − ((1 + rf)^(1/periods_per_year) − 1)
/// sharpe = mean(x) / std(x, ddof=1) · √periods_per_year
/// ```
///
/// `rf` is an annual rate.
///
/// ```
/// use finance_rs::perf::sharpe;
/// let r = [0.01, -0.02, 0.03, -0.01, 0.02];
/// let s = sharpe(&r, 0.0, 252.0)?;
/// assert!((s - 4.593_220_484_431_883).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for fewer than 2 returns,
/// [`FinanceError::NonFinite`] for non-finite input,
/// [`FinanceError::InvalidInput`] for `periods_per_year <= 0` or `rf <= −1`,
/// [`FinanceError::Undefined`] for zero volatility.
pub fn sharpe(returns: &[f64], rf: f64, periods_per_year: f64) -> Result<f64> {
    check_periods(periods_per_year)?;
    check_returns("returns", returns, 2)?;
    let x = excess_returns(returns, rf, periods_per_year)?;
    let ratio = div("volatility", mean(&x), std_dev(&x))?;
    output("sharpe", ratio * periods_per_year.sqrt())
}

/// Sharpe ratio for a per-period risk-free return and population standard
/// deviation. Kept for compatibility with older metric contracts that
/// supplied periodic `rf` and treated the input as a complete population.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
#[must_use]
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn sharpe_periodic_population(
    returns: &[f64],
    risk_free_per_period: f64,
    annualization: f64,
) -> Option<f64> {
    if returns.is_empty()
        || !risk_free_per_period.is_finite()
        || !annualization.is_finite()
        || annualization <= 0.0
        || returns.iter().any(|value| !value.is_finite())
    {
        return None;
    }
    let excess = returns
        .iter()
        .map(|value| value - risk_free_per_period)
        .collect::<Vec<_>>();
    let mean = excess.iter().sum::<f64>() / excess.len() as f64;
    let variance = excess
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / excess.len() as f64;
    (variance > 0.0).then_some(mean / variance.sqrt() * annualization.sqrt())
}

/// Compatibility Sortino ratio for a per-period minimum acceptable return.
///
/// This preserves the invest-metrics convention: subtract `mar` from the
/// arithmetic mean, measure downside from `mar`, divide by every observation,
/// and annualize by `sqrt(annualization)`.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn sortino_periodic_mar(returns: &[f64], mar: f64, annualization: f64) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    finite("mar", mar)?;
    check_periods(annualization)?;
    let excess_mean = mean(returns) - mar;
    let downside = (returns
        .iter()
        .map(|value| (value.min(mar) - mar).powi(2))
        .sum::<f64>()
        / returns.len() as f64)
        .sqrt();
    let ratio = div("downside deviation", excess_mean, downside)?;
    output("sortino_periodic_mar", ratio * annualization.sqrt())
}

/// Annualized Sharpe ratio from a mean and standard deviation supplied by
/// the caller. Returns zero for non-positive volatility, matching legacy
/// portfolio reporting contracts.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
#[must_use]
pub fn sharpe_from_moments(mean: f64, stdev: f64, periods_per_year: f64) -> f64 {
    if stdev <= 0.0 || periods_per_year <= 0.0 {
        0.0
    } else {
        mean / stdev * periods_per_year.sqrt()
    }
}

/// Annualized Sortino ratio (quantstats `sortino`, `smart=False`).
///
/// ```text
/// x_t      = r_t − ((1 + rf)^(1/periods_per_year) − 1)
/// downside = √(Σ_{x_t<0} x_t² / n)          (n counts every period)
/// sortino  = mean(x) / downside · √periods_per_year
/// ```
///
/// This is the Red Rock Capital target downside deviation quantstats uses
/// (target 0, divide by all `n`), not empyrical's.
///
/// ```
/// use finance_rs::perf::sortino;
/// let r = [0.01, -0.02, 0.03, -0.01, 0.02];
/// let s = sortino(&r, 0.0, 252.0)?;
/// assert!((s - 9.524_704_719_832_526).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for an empty series,
/// [`FinanceError::NonFinite`] for non-finite input,
/// [`FinanceError::InvalidInput`] for `periods_per_year <= 0` or `rf <= −1`,
/// [`FinanceError::Undefined`] when no excess return is negative.
#[allow(clippy::cast_precision_loss)]
pub fn sortino(returns: &[f64], rf: f64, periods_per_year: f64) -> Result<f64> {
    check_periods(periods_per_year)?;
    check_returns("returns", returns, 1)?;
    let x = excess_returns(returns, rf, periods_per_year)?;
    let ss: f64 = x.iter().filter(|v| **v < 0.0).map(|v| v * v).sum();
    let downside = (ss / x.len() as f64).sqrt();
    let ratio = div("downside deviation", mean(&x), downside)?;
    output("sortino", ratio * periods_per_year.sqrt())
}

/// Calmar ratio (quantstats `calmar`): CAGR over the absolute max drawdown.
///
/// `calmar = cagr(r, periods_per_year) / |max_drawdown(r)|`
///
/// ```
/// use finance_rs::perf::{cagr, calmar};
/// let r = [0.01, -0.02, 0.03, -0.01, 0.02];
/// let c = calmar(&r, 252.0)?;
/// assert!((c - cagr(&r, 252.0)? / 0.02).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`cagr`], plus [`FinanceError::Undefined`] when the series never draws
/// down.
pub fn calmar(returns: &[f64], periods_per_year: f64) -> Result<f64> {
    let growth = cagr(returns, periods_per_year)?;
    div("max drawdown", growth, max_drawdown(returns)?.abs())
}

/// Omega ratio (quantstats `omega`): gains over losses around a threshold.
///
/// ```text
/// x_t = r_t − ((1 + rf)^(1/periods_per_year) − 1)
/// τ   = (1 + required_return)^(1/periods_per_year) − 1   (τ = required_return
///                                                         when periods_per_year = 1)
/// Ω   = Σ max(x_t − τ, 0) / Σ max(τ − x_t, 0)
/// ```
///
/// Both `rf` and `required_return` are annual.
///
/// ```
/// use finance_rs::perf::omega;
/// let r = [0.01, -0.02, 0.03, -0.01, 0.02];
/// assert!((omega(&r, 0.0, 0.0, 252.0)? - 2.0).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for fewer than 2 returns (as quantstats),
/// [`FinanceError::NonFinite`] for non-finite input,
/// [`FinanceError::InvalidInput`] for `periods_per_year <= 0`, `rf <= −1` or
/// `required_return <= −1`, [`FinanceError::Undefined`] when no period falls
/// below the threshold.
pub fn omega(returns: &[f64], rf: f64, required_return: f64, periods_per_year: f64) -> Result<f64> {
    check_periods(periods_per_year)?;
    check_returns("returns", returns, 2)?;
    finite("required_return", required_return)?;
    if required_return <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "required_return",
            reason: "must be greater than -100%",
        });
    }
    let x = excess_returns(returns, rf, periods_per_year)?;
    #[allow(clippy::float_cmp)]
    let threshold = if periods_per_year == 1.0 {
        required_return
    } else {
        (1.0 + required_return).powf(1.0 / periods_per_year) - 1.0
    };
    let (mut gains, mut losses) = (0.0, 0.0);
    for v in &x {
        let d = v - threshold;
        if d > 0.0 {
            gains += d;
        } else if d < 0.0 {
            losses -= d;
        }
    }
    div("losses below threshold", gains, losses)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn zero_denominators_are_undefined() {
        let flat = [0.01; 5];
        assert!(matches!(
            sortino(&flat, 0.0, 252.0),
            Err(FinanceError::Undefined { .. })
        ));
        assert!(matches!(
            calmar(&flat, 252.0),
            Err(FinanceError::Undefined { .. })
        ));
        assert!(matches!(
            omega(&flat, 0.0, 0.0, 252.0),
            Err(FinanceError::Undefined { .. })
        ));
        assert!(matches!(
            sharpe(&[0.0; 4], 0.0, 252.0),
            Err(FinanceError::Undefined { .. })
        ));
    }

    #[test]
    fn compatibility_ratios_cover_population_and_periodic_contracts() -> Result<()> {
        let returns = [0.01, -0.02, 0.03, -0.01];
        let legacy = sharpe_periodic_population(&returns, 0.0, 12.0).unwrap();
        assert!(legacy.is_finite());
        assert_eq!(sharpe_periodic_population(&[0.01], 0.01, 12.0), None);
        assert_eq!(sharpe_periodic_population(&returns, f64::NAN, 12.0), None);
        assert_eq!(sharpe_periodic_population(&returns, 0.0, 0.0), None);
        assert_eq!(
            sharpe_periodic_population(&[0.01, f64::INFINITY], 0.0, 12.0),
            None
        );
        assert_eq!(sharpe_periodic_population(&[0.01; 3], 0.0, 12.0), None);
        assert_eq!(sharpe_from_moments(1.0, 0.0, 12.0), 0.0);
        assert_eq!(sharpe_from_moments(1.0, 1.0, 0.0), 0.0);
        assert_eq!(sharpe_from_moments(1.0, 2.0, 4.0), 1.0);

        let mar = sortino_periodic_mar(&returns, 0.0, 12.0)?;
        assert!(mar.is_finite());
        assert!(sortino_periodic_mar(&[], 0.0, 12.0).is_err());
        assert!(sortino_periodic_mar(&returns, f64::INFINITY, 12.0).is_err());
        assert!(sortino_periodic_mar(&returns, 0.0, 0.0).is_err());
        Ok(())
    }

    #[test]
    fn rejects_bad_rates() {
        let r = [0.01, -0.02];
        assert!(matches!(
            sharpe(&r, -1.0, 252.0),
            Err(FinanceError::InvalidInput { .. })
        ));
        assert!(omega(&r, 0.0, -1.0, 252.0).is_err());
        assert!(omega(&[0.1, -0.1, 0.2], 0.0, 0.0, 1.0).is_ok());
        assert!(matches!(
            omega(&r, 0.0, -1.0, 252.0),
            Err(FinanceError::InvalidInput { .. })
        ));
        assert!(matches!(
            omega(&[0.1], 0.0, 0.0, 252.0),
            Err(FinanceError::TooShort { .. })
        ));
    }

    #[test]
    fn rf_lowers_sharpe() {
        let r = [0.01, -0.02, 0.03, -0.01, 0.02];
        assert!(sharpe(&r, 0.05, 252.0).unwrap() < sharpe(&r, 0.0, 252.0).unwrap());
    }

    #[test]
    #[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
    fn periodic_mar_sortino_matches_legacy_definition() {
        let returns = [0.04, -0.02, 0.03, -0.05];
        let mar = 0.01;
        let mean = returns.iter().sum::<f64>() / returns.len() as f64;
        let downside = (returns
            .iter()
            .map(|r| (r.min(mar) - mar).powi(2))
            .sum::<f64>()
            / returns.len() as f64)
            .sqrt();
        let expected = (mean - mar) / downside * 12.0_f64.sqrt();
        assert!((sortino_periodic_mar(&returns, mar, 12.0).unwrap() - expected).abs() < 1e-14);
    }
}
