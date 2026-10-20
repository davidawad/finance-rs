//! Return aggregation: compounded and cumulative returns, CAGR, volatility,
//! win rate.

pub use super::returns_extra::{
    aggregate, avg_loss, avg_return, avg_win, best, consecutive_losses, consecutive_wins, excess,
    exposure, geometric_mean, log, modified_dietz, mtd, mwr, qtd, rebase, rolling, simple,
    to_price_index, twr, worst, ytd, Period,
};

use super::{check_periods, check_returns, std_dev};
use crate::error::{output, FinanceError, Result};

/// Total compounded return (quantstats `comp`).
///
/// `Π_t (1 + r_t) − 1`
///
/// ```
/// use finance_rs::perf::compounded_return;
/// let total = compounded_return(&[0.10, -0.10])?;
/// assert!((total + 0.01).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for an empty series, [`FinanceError::NonFinite`]
/// for non-finite input, [`FinanceError::Overflow`] for a non-finite result.
pub fn compounded_return(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    let growth: f64 = returns.iter().map(|r| 1.0 + r).product();
    output("compounded_return", growth - 1.0)
}

/// Running compounded return after each period (quantstats `compsum`).
///
/// `c_t = Π_{s<=t} (1 + r_s) − 1`
///
/// ```
/// use finance_rs::perf::cumulative_returns;
/// let c = cumulative_returns(&[0.10, 0.10])?;
/// assert!((c[0] - 0.10).abs() < 1e-15 && (c[1] - 0.21).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`compounded_return`].
pub fn cumulative_returns(returns: &[f64]) -> Result<Vec<f64>> {
    check_returns("returns", returns, 1)?;
    let mut growth = 1.0;
    returns
        .iter()
        .map(|r| {
            growth *= 1.0 + r;
            output("cumulative_returns", growth - 1.0)
        })
        .collect()
}

/// Compound annual growth rate (quantstats `cagr`, `compounded=True`).
///
/// ```text
/// years = n / periods_per_year
/// cagr  = (Π_t (1 + r_t))^(1 / years) − 1
/// ```
///
/// quantstats 0.0.86 also counts years as `returns.count() / periods`, so
/// passing the same `periods_per_year` reproduces it exactly. For a calendar
/// span, pass `periods_per_year = n / years`. quantstats accepts `rf` here
/// but never subtracts it, so this function takes none.
///
/// ```
/// use finance_rs::perf::cagr;
/// // Two years of monthly returns that double the money: CAGR = √2 − 1.
/// let r = [2f64.powf(1.0 / 24.0) - 1.0; 24];
/// assert!((cagr(&r, 12.0)? - (2f64.sqrt() - 1.0)).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for an empty series, [`FinanceError::NonFinite`]
/// for non-finite input, [`FinanceError::InvalidInput`] for
/// `periods_per_year <= 0` or a negative terminal wealth (returns below −100%),
/// [`FinanceError::Overflow`] for a non-finite result.
#[allow(clippy::cast_precision_loss)]
pub fn cagr(returns: &[f64], periods_per_year: f64) -> Result<f64> {
    check_periods(periods_per_year)?;
    let wealth = compounded_return(returns)? + 1.0;
    if wealth < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "returns",
            reason: "terminal wealth is negative, so no real growth rate exists",
        });
    }
    let years = returns.len() as f64 / periods_per_year;
    output("cagr", wealth.powf(1.0 / years) - 1.0)
}

/// Annualized volatility (quantstats `volatility`).
///
/// `σ_annual = std(r, ddof=1) · √periods_per_year`
///
/// ```
/// use finance_rs::perf::volatility;
/// let v = volatility(&[0.01, -0.01, 0.01, -0.01], 252.0)?;
/// assert!((v - (4.0f64 / 3.0).sqrt() * 0.01 * 252f64.sqrt()).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for fewer than 2 returns,
/// [`FinanceError::NonFinite`] for non-finite input,
/// [`FinanceError::InvalidInput`] for `periods_per_year <= 0`.
pub fn volatility(returns: &[f64], periods_per_year: f64) -> Result<f64> {
    check_periods(periods_per_year)?;
    check_returns("returns", returns, 2)?;
    output("volatility", std_dev(returns) * periods_per_year.sqrt())
}

/// Fraction of non-zero periods with a positive return (quantstats `win_rate`).
///
/// `#{r_t > 0} / #{r_t ≠ 0}`, and `0` when every return is zero (as
/// quantstats, which warns and returns `0.0`). Zero-return periods count as
/// "no trade", not as losses.
///
/// ```
/// use finance_rs::perf::win_rate;
/// assert_eq!(win_rate(&[0.01, 0.0, -0.02, 0.03])?, 2.0 / 3.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for an empty series, [`FinanceError::NonFinite`]
/// for non-finite input.
#[allow(clippy::cast_precision_loss)]
pub fn win_rate(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    let wins = returns.iter().filter(|r| **r > 0.0).count();
    let trades = returns.iter().filter(|r| **r != 0.0).count();
    if trades == 0 {
        return Ok(0.0);
    }
    Ok(wins as f64 / trades as f64)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn rejects_bad_input() {
        assert!(matches!(
            compounded_return(&[]),
            Err(FinanceError::TooShort { .. })
        ));
        assert!(matches!(
            volatility(&[0.1], 252.0),
            Err(FinanceError::TooShort { .. })
        ));
        assert!(matches!(
            volatility(&[0.1, f64::NAN], 252.0),
            Err(FinanceError::NonFinite { .. })
        ));
        assert!(matches!(
            cagr(&[0.1], 0.0),
            Err(FinanceError::InvalidInput { .. })
        ));
        assert!(matches!(
            cagr(&[-1.5, 0.1], 252.0),
            Err(FinanceError::InvalidInput { .. })
        ));
    }

    #[test]
    fn total_loss_gives_minus_one_cagr() {
        assert_eq!(cagr(&[0.1, -1.0], 12.0).unwrap(), -1.0);
    }

    #[test]
    fn win_rate_all_zero_is_zero() {
        assert_eq!(win_rate(&[0.0, 0.0]).unwrap(), 0.0);
    }
}
