//! Additional return transforms and return-series summaries.

use super::{check_returns, mean};
use crate::{
    cashflow::xirr,
    date::Date,
    error::{all_finite, finite, min_len, output, same_len, FinanceError, Result},
    rates::periodic_from_annual,
};

/// Return simple price changes, oldest first.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn simple(prices: &[f64]) -> Result<Vec<f64>> {
    min_len("prices", prices, 2)?;
    all_finite("prices", prices)?;
    let mut out = Vec::with_capacity(prices.len() - 1);
    for pair in prices.windows(2) {
        if pair[0] == 0.0 {
            return Err(FinanceError::Undefined {
                what: "prior price",
            });
        }
        out.push(output("simple return", pair[1] / pair[0] - 1.0)?);
    }
    Ok(out)
}
/// Return log price changes, oldest first.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn log(prices: &[f64]) -> Result<Vec<f64>> {
    min_len("prices", prices, 2)?;
    all_finite("prices", prices)?;
    let mut out = Vec::with_capacity(prices.len() - 1);
    for pair in prices.windows(2) {
        if pair[0] <= 0.0 || pair[1] <= 0.0 {
            return Err(FinanceError::InvalidInput {
                what: "prices",
                reason: "log returns require strictly positive prices",
            });
        }
        out.push(output("log return", (pair[1] / pair[0]).ln())?);
    }
    Ok(out)
}
/// Compound returns into a price index whose first value is `base`.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn to_price_index(returns: &[f64], base: f64) -> Result<Vec<f64>> {
    check_returns("returns", returns, 1)?;
    finite("base", base)?;
    if base <= 0.0 || returns.iter().any(|r| *r < -1.0) {
        return Err(FinanceError::InvalidInput {
            what: "returns/base",
            reason: "base must be positive and returns cannot be below -100%",
        });
    }
    let mut index = base;
    let mut out = Vec::with_capacity(returns.len() + 1);
    out.push(base);
    for r in returns {
        index = output("price index", index * (1.0 + r))?;
        out.push(index);
    }
    Ok(out)
}
/// Scale prices so the first price equals `base`.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn rebase(prices: &[f64], base: f64) -> Result<Vec<f64>> {
    min_len("prices", prices, 1)?;
    all_finite("prices", prices)?;
    finite("base", base)?;
    if prices[0] == 0.0 || base <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "prices/base",
            reason: "first price must be nonzero and base must be positive",
        });
    }
    let scale = base / prices[0];
    prices
        .iter()
        .map(|p| output("rebased price", p * scale))
        .collect()
}
/// Subtract the effective periodic risk-free rate from each return.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn excess(returns: &[f64], rf: f64, periods_per_year: f64) -> Result<Vec<f64>> {
    check_returns("returns", returns, 1)?;
    finite("rf", rf)?;
    if rf <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "rf",
            reason: "must be greater than -100%",
        });
    }
    let periodic = periodic_from_annual(rf, periods_per_year)?;
    returns
        .iter()
        .map(|r| output("excess return", r - periodic))
        .collect()
}
/// Calendar interval used by [`aggregate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
pub enum Period {
    /// The week option.
    Week,
    /// The month option.
    Month,
    /// The quarter option.
    Quarter,
    /// The year option.
    Year,
}

fn period_key(date: Date, period: Period) -> i64 {
    match period {
        Period::Week => date
            .ordinal()
            .saturating_add(GREGORIAN_EPOCH_OFFSET_DAYS)
            .div_euclid(7),
        Period::Month => i64::from(date.year()) * 12 + i64::from(date.month()) - 1,
        Period::Quarter => i64::from(date.year()) * 4 + i64::from((date.month() - 1) / 3),
        Period::Year => i64::from(date.year()),
    }
}

// `Date::ordinal` starts at 1970-01-01; the civil Gregorian ordinal used by
// weekly buckets starts at 0000-03-01, 719,468 days earlier.
const GREGORIAN_EPOCH_OFFSET_DAYS: i64 = 719_468;

/// Compound observations in each calendar period, returning the last date of each group.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn aggregate<D: Copy + Into<Date>>(
    dates: &[D],
    returns: &[f64],
    period: Period,
) -> Result<Vec<(Date, f64)>> {
    same_len("dates", dates.len(), "returns", returns.len())?;
    min_len("returns", returns, 1)?;
    all_finite("returns", returns)?;
    let dates: Vec<Date> = dates.iter().copied().map(Into::into).collect();
    if dates.windows(2).any(|w| w[0] >= w[1]) {
        return Err(FinanceError::InvalidInput {
            what: "dates",
            reason: "must be strictly increasing",
        });
    }
    let mut out = Vec::new();
    let mut key = period_key(dates[0], period);
    let mut value = 1.0;
    let mut end = dates[0];
    for (date, r) in dates.iter().copied().zip(returns.iter().copied()) {
        let next = period_key(date, period);
        if next != key {
            out.push((end, value - 1.0));
            key = next;
            value = 1.0;
        }
        value *= 1.0 + r;
        end = date;
    }
    out.push((end, value - 1.0));
    Ok(out)
}
/// Period-to-date price return from the last observation before the period start.
fn period_to_date<D: Copy + Into<Date>>(
    dates: &[D],
    prices: &[f64],
    as_of: D,
    months: Option<u32>,
) -> Result<f64> {
    same_len("dates", dates.len(), "prices", prices.len())?;
    min_len("prices", prices, 2)?;
    all_finite("prices", prices)?;
    let dates: Vec<Date> = dates.iter().copied().map(Into::into).collect();
    if dates.windows(2).any(|w| w[0] >= w[1]) {
        return Err(FinanceError::InvalidInput {
            what: "dates",
            reason: "must be strictly increasing",
        });
    }
    let as_of = as_of.into();
    let end = dates
        .iter()
        .position(|d| *d == as_of)
        .ok_or(FinanceError::InvalidInput {
            what: "as_of",
            reason: "must match an observation date",
        })?;
    let date = dates[end];
    let first_in_period = match months {
        None => Date::new(date.year(), 1, 1)?,
        Some(m) => Date::new(date.year(), ((date.month() - 1) / m) * m + 1, 1)?,
    };
    let start = (0..=end)
        .rev()
        .find(|i| dates[*i] < first_in_period)
        .unwrap_or(0);
    if start == end || prices[start] == 0.0 {
        return Err(FinanceError::Undefined {
            what: "period start price",
        });
    }
    output("period-to-date return", prices[end] / prices[start] - 1.0)
}
/// Month-to-date price return.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn mtd<D: Copy + Into<Date>>(dates: &[D], prices: &[f64], as_of: D) -> Result<f64> {
    period_to_date(dates, prices, as_of, Some(1))
}
/// Quarter-to-date price return.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn qtd<D: Copy + Into<Date>>(dates: &[D], prices: &[f64], as_of: D) -> Result<f64> {
    period_to_date(dates, prices, as_of, Some(3))
}
/// Year-to-date price return.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn ytd<D: Copy + Into<Date>>(dates: &[D], prices: &[f64], as_of: D) -> Result<f64> {
    period_to_date(dates, prices, as_of, None)
}
/// Chain-link subperiod returns into a time-weighted return.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn twr(sub_period_returns: &[f64]) -> Result<f64> {
    check_returns("sub_period_returns", sub_period_returns, 1)?;
    if sub_period_returns.iter().any(|r| *r < -1.0) {
        return Err(FinanceError::InvalidInput {
            what: "sub_period_returns",
            reason: "returns cannot be below -100%",
        });
    }
    output(
        "twr",
        sub_period_returns
            .iter()
            .fold(1.0, |wealth, r| wealth * (1.0 + r))
            - 1.0,
    )
}
/// Modified Dietz return with externally supplied flow timing weights.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn modified_dietz(begin: f64, end: f64, flows: &[(f64, f64)]) -> Result<f64> {
    finite("begin", begin)?;
    finite("end", end)?;
    let mut flow_sum = 0.0;
    let mut weighted = 0.0;
    for (weight, amount) in flows {
        finite("flow weight", *weight)?;
        finite("flow amount", *amount)?;
        if !(0.0..=1.0).contains(weight) {
            return Err(FinanceError::InvalidInput {
                what: "flow weight",
                reason: "must be in [0, 1]",
            });
        }
        flow_sum += amount;
        weighted += weight * amount;
    }
    let denominator = begin + weighted;
    if denominator == 0.0 {
        return Err(FinanceError::Undefined {
            what: "modified Dietz denominator",
        });
    }
    output("modified_dietz", (end - begin - flow_sum) / denominator)
}
/// Money-weighted return, using the canonical XIRR solver.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn mwr<D: Copy + Into<Date>>(dates: &[D], flows: &[f64]) -> Result<f64> {
    xirr(dates, flows)
}

/// Largest period return.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn best(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    Ok(returns.iter().copied().fold(f64::NEG_INFINITY, f64::max))
}
/// Smallest period return.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn worst(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    Ok(returns.iter().copied().fold(f64::INFINITY, f64::min))
}
/// Arithmetic mean return.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn avg_return(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    output("avg_return", mean(returns))
}
/// Arithmetic mean of positive returns; undefined when there are no wins.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn avg_win(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    let xs: Vec<f64> = returns.iter().copied().filter(|r| *r > 0.0).collect();
    if xs.is_empty() {
        return Err(FinanceError::Undefined {
            what: "winning returns",
        });
    }
    output("avg_win", mean(&xs))
}
/// Arithmetic mean of negative returns; undefined when there are no losses.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn avg_loss(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    let xs: Vec<f64> = returns.iter().copied().filter(|r| *r < 0.0).collect();
    if xs.is_empty() {
        return Err(FinanceError::Undefined {
            what: "losing returns",
        });
    }
    output("avg_loss", mean(&xs))
}

/// Longest consecutive run of strictly positive returns.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn consecutive_wins(returns: &[f64]) -> Result<usize> {
    check_returns("returns", returns, 1)?;
    Ok(run(returns, |r| r > 0.0))
}
/// Longest consecutive run of strictly negative returns.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn consecutive_losses(returns: &[f64]) -> Result<usize> {
    check_returns("returns", returns, 1)?;
    Ok(run(returns, |r| r < 0.0))
}
fn run<F: Fn(f64) -> bool>(xs: &[f64], test: F) -> usize {
    let mut best = 0;
    let mut current = 0;
    for x in xs {
        if test(*x) {
            current += 1;
            best = best.max(current);
        } else {
            current = 0;
        }
    }
    best
}

/// Share of observations with a non-zero return.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn exposure(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    Ok(returns.iter().filter(|r| **r != 0.0).count() as f64 / returns.len() as f64)
}
/// Geometric mean return per observation.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn geometric_mean(returns: &[f64]) -> Result<f64> {
    check_returns("returns", returns, 1)?;
    if returns.iter().any(|r| *r < -1.0) {
        return Err(FinanceError::InvalidInput {
            what: "returns",
            reason: "returns cannot be below -100%",
        });
    }
    let value = returns.iter().map(|r| (1.0 + r).ln()).sum::<f64>() / returns.len() as f64;
    output("geometric_mean", value.exp() - 1.0)
}

/// Apply a statistic to every complete rolling window.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn rolling<T, F>(returns: &[f64], window: usize, mut f: F) -> Result<Vec<T>>
where
    F: FnMut(&[f64]) -> Result<T>,
{
    check_returns("returns", returns, 1)?;
    if window == 0 || window > returns.len() {
        return Err(FinanceError::InvalidInput {
            what: "window",
            reason: "must be between 1 and the number of returns",
        });
    }
    returns.windows(window).map(&mut f).collect()
}

#[cfg(test)]
#[path = "returns_extra_tests.rs"]
mod tests;
