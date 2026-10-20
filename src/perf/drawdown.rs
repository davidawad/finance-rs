//! Drawdowns: the underwater series, the maximum drawdown and the individual
//! drawdown periods with their durations.

use super::check_returns;
use crate::error::{FinanceError, Result};

/// One peak-to-recovery episode of a drawdown series.
///
/// Indices are positions in the return series. `end` is the last period
/// still under water (quantstats `drawdown_details` "end"), so
/// `length = end − start + 1`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
pub struct DrawdownPeriod {
    /// First period below the previous peak.
    pub start: usize,
    /// Period of the deepest point (first one, if tied).
    pub valley: usize,
    /// Last period below the previous peak.
    pub end: usize,
    /// Duration in periods, `end − start + 1`.
    pub length: usize,
    /// Deepest drawdown in the episode (negative, e.g. `-0.2` for −20%).
    pub max_drawdown: f64,
    /// `false` when the series ends before regaining the peak.
    pub recovered: bool,
}

/// Drawdown from the running peak after each period (quantstats
/// `to_drawdown_series`).
///
/// ```text
/// equity_t = Π_{s<=t} (1 + r_s)        (equity before the first period is 1)
/// dd_t     = equity_t / max(1, max_{s<=t} equity_s) − 1
/// ```
///
/// The starting equity of 1 counts as a peak, so a loss in the very first
/// period is a drawdown (quantstats' phantom baseline).
///
/// ```
/// use finance_rs::perf::drawdown_series;
/// let dd = drawdown_series(&[-0.1, 0.05, 0.1])?;
/// assert!((dd[0] + 0.1).abs() < 1e-15 && (dd[1] + 0.055).abs() < 1e-15);
/// assert_eq!(dd[2], 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for an empty series, [`FinanceError::NonFinite`]
/// for non-finite input, [`FinanceError::Overflow`] when the equity curve
/// overflows.
pub fn drawdown_series(returns: &[f64]) -> Result<Vec<f64>> {
    check_returns("returns", returns, 1)?;
    let mut growth = 1.0_f64;
    let mut peak = 1.0_f64;
    let mut out = Vec::with_capacity(returns.len());
    for r in returns {
        growth *= 1.0 + r;
        // quantstats rebuilds prices as base + base * compsum.
        let equity = 1.0 + (growth - 1.0);
        peak = peak.max(equity);
        let dd = equity / peak - 1.0;
        if !dd.is_finite() {
            return Err(FinanceError::Overflow {
                what: "drawdown_series",
            });
        }
        // quantstats maps -0.0 to 0.0.
        out.push(if dd == 0.0 { 0.0 } else { dd });
    }
    Ok(out)
}

/// Maximum drawdown (quantstats `max_drawdown`): the most negative value of
/// [`drawdown_series`], or `0` if the series never falls below a peak.
///
/// Always in `[−1, 0]` for returns `>= −1`.
///
/// ```
/// use finance_rs::perf::max_drawdown;
/// // 100 -> 120 -> 90 -> 130: worst fall is 120 -> 90, i.e. −25%.
/// let mdd = max_drawdown(&[0.2, -0.25, 130.0 / 90.0 - 1.0])?;
/// assert!((mdd + 0.25).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`drawdown_series`].
pub fn max_drawdown(returns: &[f64]) -> Result<f64> {
    Ok(drawdown_series(returns)?.into_iter().fold(0.0, f64::min))
}

/// Ulcer index: root mean square decimal drawdown from the running peak.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn ulcer_index(returns: &[f64]) -> Result<f64> {
    crate::error::min_len("returns", returns, 2)?;
    let drawdowns = drawdown_series(returns)?;
    let sum = drawdowns
        .iter()
        .map(|drawdown| drawdown.powi(2))
        .sum::<f64>();
    Ok((sum / (returns.len() - 1) as f64).sqrt())
}

/// Every drawdown episode, oldest first (quantstats `drawdown_details`,
/// with positions instead of dates and durations in periods instead of
/// calendar days).
///
/// An episode starts at the first period below the running peak and ends
/// at the last period before the equity curve regains it.
///
/// ```
/// use finance_rs::perf::drawdown_periods;
/// let p = drawdown_periods(&[0.1, -0.1, -0.1, 0.3, -0.05])?;
/// assert_eq!(p.len(), 2);
/// assert_eq!((p[0].start, p[0].valley, p[0].end, p[0].length), (1, 2, 2, 2));
/// assert!(p[0].recovered && !p[1].recovered);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`drawdown_series`].
pub fn drawdown_periods(returns: &[f64]) -> Result<Vec<DrawdownPeriod>> {
    let dd = drawdown_series(returns)?;
    let mut out = Vec::new();
    let mut current: Option<DrawdownPeriod> = None;
    for (t, &d) in dd.iter().enumerate() {
        match (&mut current, d < 0.0) {
            (None, true) => {
                current = Some(DrawdownPeriod {
                    start: t,
                    valley: t,
                    end: t,
                    length: 1,
                    max_drawdown: d,
                    recovered: false,
                });
            }
            (Some(p), true) => {
                p.end = t;
                p.length += 1;
                if d < p.max_drawdown {
                    p.max_drawdown = d;
                    p.valley = t;
                }
            }
            (Some(p), false) => {
                p.recovered = true;
                out.push(*p);
                current = None;
            }
            (None, false) => {}
        }
    }
    out.extend(current);
    Ok(out)
}

/// Length in periods of the longest drawdown episode (0 if there is none).
///
/// This is the maximum of quantstats' `drawdown_details(...)["days"]` for a
/// series with one period per calendar day.
///
/// ```
/// use finance_rs::perf::max_drawdown_duration;
/// assert_eq!(max_drawdown_duration(&[0.1, -0.1, -0.1, 0.3, -0.05])?, 2);
/// assert_eq!(max_drawdown_duration(&[0.01, 0.02])?, 0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`drawdown_series`].
pub fn max_drawdown_duration(returns: &[f64]) -> Result<usize> {
    Ok(drawdown_periods(returns)?
        .iter()
        .map(|p| p.length)
        .max()
        .unwrap_or(0))
}

/// Top `n` drawdown episodes, deepest first (pyfolio `get_top_drawdowns`).
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn top_drawdowns(returns: &[f64], n: usize) -> Result<Vec<DrawdownPeriod>> {
    let mut periods = drawdown_periods(returns)?;
    periods.sort_by(|a, b| a.max_drawdown.total_cmp(&b.max_drawdown));
    periods.truncate(n);
    Ok(periods)
}

/// A drawdown episode with its original ordinal dates.
#[derive(Debug, Clone, PartialEq)]
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
pub struct DatedDrawdownPeriod {
    /// Start for the dated drawdown period result.
    pub start: crate::Date,
    /// Valley for the dated drawdown period result.
    pub valley: crate::Date,
    /// End for the dated drawdown period result.
    pub end: crate::Date,
    /// Max drawdown for the dated drawdown period result.
    pub max_drawdown: f64,
    /// Length for the dated drawdown period result.
    pub length: usize,
}

/// Drawdown episodes labeled with caller-supplied dates.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn drawdown_periods_dated<D: Copy + Into<crate::Date>>(
    dates: &[D],
    returns: &[f64],
) -> Result<Vec<DatedDrawdownPeriod>> {
    crate::error::same_len("dates", dates.len(), "returns", returns.len())?;
    drawdown_periods(returns).map(|periods| {
        periods
            .into_iter()
            .map(|p| DatedDrawdownPeriod {
                start: dates[p.start].into(),
                valley: dates[p.valley].into(),
                end: dates[p.end].into(),
                max_drawdown: p.max_drawdown,
                length: p.length,
            })
            .collect()
    })
}

/// Ulcer performance index: compounded total return less `rf`, divided by ulcer index.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn ulcer_performance_index(returns: &[f64], rf: f64) -> Result<f64> {
    crate::error::finite("rf", rf)?;
    let ui = ulcer_index(returns)?;
    if ui == 0.0 {
        return Err(crate::FinanceError::Undefined {
            what: "ulcer index",
        });
    }
    let growth = crate::perf::compounded_return(returns)?;
    crate::error::output("ulcer_performance_index", (growth - rf) / ui)
}

/// Serenity index (quantstats): total return adjusted by drawdown CVaR and volatility.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn serenity_index(returns: &[f64], rf: f64) -> Result<f64> {
    crate::error::all_finite("returns", returns)?;
    crate::error::finite("rf", rf)?;
    let dd = drawdown_series(returns)?;
    let ui = ulcer_index(returns)?;
    let sigma = crate::stats::variance(returns, 1)?.sqrt();
    let drawdown_cvar = crate::perf::cvar_historical(&dd, 0.95)?;
    let pitfall = -drawdown_cvar / sigma;
    let denominator = ui * pitfall;
    if denominator == 0.0 {
        return Err(crate::FinanceError::Undefined {
            what: "serenity index denominator",
        });
    }
    crate::error::output(
        "serenity_index",
        (returns.iter().sum::<f64>() - rf) / denominator,
    )
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    #[test]
    fn first_period_loss_is_a_drawdown() {
        let p = drawdown_periods(&[-0.1]).unwrap();
        assert_eq!(p.len(), 1);
        assert!(!p[0].recovered);
        assert!((max_drawdown(&[-0.1]).unwrap() + 0.1).abs() < 1e-15);
    }

    #[test]
    fn total_loss_is_minus_one() {
        assert_eq!(max_drawdown(&[0.5, -1.0, 0.2]).unwrap(), -1.0);
    }

    #[test]
    fn rejects_empty_and_nan() {
        assert!(matches!(
            drawdown_series(&[]),
            Err(FinanceError::TooShort { .. })
        ));
        assert!(matches!(
            max_drawdown(&[f64::INFINITY]),
            Err(FinanceError::NonFinite { .. })
        ));
    }

    #[test]
    fn ulcer_index_uses_decimal_drawdown_from_baseline() {
        assert!((ulcer_index(&[-0.1, 1.0 / 0.9 - 1.0]).unwrap() - 0.1).abs() < 1e-14);
    }

    #[test]
    fn dated_and_ranked_drawdowns() -> Result<()> {
        let dates = [
            crate::Date::new(2024, 1, 1)?,
            crate::Date::new(2024, 1, 2)?,
            crate::Date::new(2024, 1, 3)?,
            crate::Date::new(2024, 1, 4)?,
            crate::Date::new(2024, 1, 5)?,
        ];
        let returns = [0.0, -0.1, 0.2, -0.2, 0.0];
        let periods = drawdown_periods_dated(&dates, &returns)?;
        assert_eq!(periods.len(), 2);
        assert_eq!(periods[0].start, dates[1]);
        assert_eq!(periods[1].valley, dates[3]);
        let ranked = top_drawdowns(&returns, 1)?;
        assert_eq!(ranked.len(), 1);
        assert_eq!(ranked[0].valley, 3);
        let deeper = drawdown_periods(&[0.1, -0.1, -0.2])?;
        assert_eq!(deeper[0].valley, 2);
        assert!(deeper[0].max_drawdown < -0.2);
        Ok(())
    }
}
