//! Private-equity performance measures from the pyxirr `pe` API.

use super::pme_factors;
use crate::error::{all_finite, finite, min_len, output, FinanceError, Result};
use crate::{cashflow::xirr, Date};

/// Multiple on invested capital: (distributions + residual NAV) / contributions.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::moic), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn moic(contributions: &[f64], distributions: &[f64], nav: f64) -> Result<f64> {
    let (paid, distributed) = totals(contributions, distributions)?;
    finite("nav", nav)?;
    if nav < 0.0 {
        return Err(invalid("nav", "must be non-negative"));
    }
    ratio("contributions", distributed + nav, paid)
}

/// Paid-in capital over commitments.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::pic), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn pic(paid_in: f64, committed: f64) -> Result<f64> {
    finite("paid_in", paid_in)?;
    finite("committed", committed)?;
    if paid_in < 0.0 || committed < 0.0 {
        return Err(invalid("capital", "must be non-negative"));
    }
    ratio("committed", paid_in, committed)
}

/// Index-compounded contribution and distribution series, in that order.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::ks_pme_flows), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn ks_pme_flows(
    contributions: &[f64],
    distributions: &[f64],
    index: &[f64],
) -> Result<(Vec<f64>, Vec<f64>)> {
    let factors = pme_factors(contributions, distributions, 0.0, index)?;
    Ok((
        contributions
            .iter()
            .zip(&factors)
            .map(|(x, f)| x * f)
            .collect(),
        distributions
            .iter()
            .zip(&factors)
            .map(|(x, f)| x * f)
            .collect(),
    ))
}

/// Long-Nickels NAV: index-compounded calls less distributions.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::ln_pme_nav), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn ln_pme_nav(contributions: &[f64], distributions: &[f64], index: &[f64]) -> Result<f64> {
    let (calls, dists) = ks_pme_flows(contributions, distributions, index)?;
    output(
        "ln_pme_nav",
        calls.iter().sum::<f64>() - dists.iter().sum::<f64>(),
    )
}

/// Long-Nickels PME IRR using the index-adjusted NAV as the terminal value.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::ln_pme), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn ln_pme(contributions: &[f64], distributions: &[f64], index: &[f64]) -> Result<f64> {
    let nav = ln_pme_nav(contributions, distributions, index)?;
    periodic_irr(contributions, distributions, nav)
}

/// PME+ distribution scaling factor. It matches the index NAV to fund NAV.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::pme_plus_lambda), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn pme_plus_lambda(
    contributions: &[f64],
    distributions: &[f64],
    nav: f64,
    index: &[f64],
) -> Result<f64> {
    finite("nav", nav)?;
    let (calls, dists) = ks_pme_flows(contributions, distributions, index)?;
    ratio(
        "scaled_distributions",
        calls.iter().sum::<f64>() - nav,
        dists.iter().sum::<f64>(),
    )
}

/// PME+ flows as index purchases and scaled distributions, in that order.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::pme_plus_flows), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn pme_plus_flows(
    contributions: &[f64],
    distributions: &[f64],
    nav: f64,
    index: &[f64],
) -> Result<(Vec<f64>, Vec<f64>)> {
    let lambda = pme_plus_lambda(contributions, distributions, nav, index)?;
    Ok((
        contributions.to_vec(),
        distributions.iter().map(|d| d * lambda).collect(),
    ))
}

/// PME+ annualized periodic IRR over the supplied equally-spaced periods.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::pme_plus), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn pme_plus(
    contributions: &[f64],
    distributions: &[f64],
    nav: f64,
    index: &[f64],
) -> Result<f64> {
    let (calls, dists) = pme_plus_flows(contributions, distributions, nav, index)?;
    let mut flows = calls
        .iter()
        .zip(dists)
        .map(|(c, d)| d - c)
        .collect::<Vec<_>>();
    // PME validation requires at least one contribution and the arrays align.
    let last = flows.len() - 1;
    flows[last] = nav - calls[calls.len() - 1];
    periodic_root(&flows)
}

/// Net fund IRR, with NAV added at its valuation date.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::net_irr::<finance_rs::Date>), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn net_irr<D: Copy + Into<Date>>(
    dates: &[D],
    calls: &[f64],
    dists: &[f64],
    nav: f64,
    nav_date: D,
) -> Result<f64> {
    finite("nav", nav)?;
    if nav < 0.0 {
        return Err(invalid("nav", "must be non-negative"));
    }
    if dates.is_empty() {
        return Err(FinanceError::TooShort {
            what: "dates",
            min: 1,
            got: 0,
        });
    }
    if calls.len() != dates.len() || dists.len() != dates.len() {
        return Err(FinanceError::LengthMismatch {
            left: "dates",
            left_len: dates.len(),
            right: "cash_flows",
            right_len: calls.len().min(dists.len()),
        });
    }
    all_finite("calls", calls)?;
    all_finite("distributions", dists)?;
    let mut flows = calls
        .iter()
        .zip(dists)
        .map(|(c, d)| d - c)
        .collect::<Vec<_>>();
    let valuation = nav_date.into();
    if let Some(i) = dates.iter().position(|d| (*d).into() == valuation) {
        flows[i] += nav;
    } else {
        return Err(invalid("nav_date", "must be present in the dates"));
    }
    xirr(dates, &flows)
}

/// Gross IRR applies the same dated-flow convention to gross calls/distributions.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::gross_irr::<finance_rs::Date>), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn gross_irr<D: Copy + Into<Date>>(
    dates: &[D],
    calls: &[f64],
    dists: &[f64],
    nav: f64,
    nav_date: D,
) -> Result<f64> {
    net_irr(dates, calls, dists, nav, nav_date)
}

/// Cambridge Associates modified PME, with one NAV and index level per period.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::m_pme), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn m_pme(
    contributions: &[f64],
    distributions: &[f64],
    index: &[f64],
    nav: &[f64],
) -> Result<f64> {
    let n = contributions.len();
    if distributions.len() != n || index.len() != n || nav.len() != n {
        return Err(FinanceError::LengthMismatch {
            left: "contributions",
            left_len: n,
            right: "aligned_series",
            right_len: distributions.len().min(index.len()).min(nav.len()),
        });
    }
    min_len("contributions", contributions, 2)?;
    all_finite("contributions", contributions)?;
    all_finite("distributions", distributions)?;
    all_finite("index", index)?;
    all_finite("nav", nav)?;
    if contributions
        .iter()
        .chain(distributions)
        .chain(nav)
        .any(|x| *x < 0.0)
        || index.iter().any(|x| *x <= 0.0)
    {
        return Err(invalid(
            "m_pme",
            "cash flows and NAV must be non-negative and index levels positive",
        ));
    }
    let mut theoretical_nav = 0.0;
    let mut flows = Vec::with_capacity(n);
    for i in 0..n {
        let grown =
            theoretical_nav * if i == 0 { 1.0 } else { index[i] / index[i - 1] } + contributions[i];
        let denom = distributions[i] + nav[i];
        let weight = if denom == 0.0 {
            0.0
        } else {
            distributions[i] / denom
        };
        let distribution = grown * weight;
        theoretical_nav = output("m_pme_nav", grown - distribution)?;
        flows.push(distribution - contributions[i]);
    }
    let last = flows.len() - 1;
    flows[last] = theoretical_nav - contributions[contributions.len() - 1];
    periodic_root(&flows)
}

fn periodic_irr(calls: &[f64], dists: &[f64], nav: f64) -> Result<f64> {
    if calls.len() != dists.len() {
        return Err(FinanceError::LengthMismatch {
            left: "calls",
            left_len: calls.len(),
            right: "distributions",
            right_len: dists.len(),
        });
    }
    min_len("calls", calls, 2)?;
    let mut flows = calls
        .iter()
        .zip(dists)
        .map(|(c, d)| d - c)
        .collect::<Vec<_>>();
    // `min_len` above guarantees at least two aligned cash-flow periods.
    let last = flows.len() - 1;
    flows[last] = nav - calls[calls.len() - 1];
    periodic_root(&flows)
}

#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
fn periodic_root(flows: &[f64]) -> Result<f64> {
    let times = (0..flows.len()).map(|i| i as f64).collect::<Vec<_>>();
    crate::cashflow::xirr_times(&times, flows, 0.1)
}

fn totals(calls: &[f64], dists: &[f64]) -> Result<(f64, f64)> {
    if calls.len() != dists.len() {
        return Err(FinanceError::LengthMismatch {
            left: "contributions",
            left_len: calls.len(),
            right: "distributions",
            right_len: dists.len(),
        });
    }
    min_len("contributions", calls, 1)?;
    all_finite("contributions", calls)?;
    all_finite("distributions", dists)?;
    if calls.iter().chain(dists).any(|x| *x < 0.0) {
        return Err(invalid("cash_flows", "must be non-negative magnitudes"));
    }
    Ok((calls.iter().sum(), dists.iter().sum()))
}
fn ratio(what: &'static str, numerator: f64, denominator: f64) -> Result<f64> {
    finite(what, numerator)?;
    finite("denominator", denominator)?;
    if denominator <= 0.0 {
        return Err(FinanceError::Undefined { what });
    }
    output(what, numerator / denominator)
}
fn invalid(what: &'static str, reason: &'static str) -> FinanceError {
    FinanceError::InvalidInput { what, reason }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> Result<Date> {
        Date::new(year, month, day)
    }

    #[test]
    fn private_equity_pme_and_dated_irr_paths() -> Result<()> {
        assert!(periodic_irr(&[1.0], &[], 0.0).is_err());
        assert!((periodic_irr(&[100.0, 0.0], &[0.0, 110.0], 110.0)? - 0.1).abs() < 1e-10);
        let calls = [100.0, 0.0];
        let dists = [0.0, 100.0];
        let index = [100.0, 110.0];
        assert_eq!(moic(&calls, &[0.0, 150.0], 50.0)?, 2.0);
        assert_eq!(pic(50.0, 100.0)?, 0.5);
        let (adjusted_calls, adjusted_dists) = ks_pme_flows(&calls, &dists, &index)?;
        assert!((adjusted_calls[0] - 110.0).abs() < 1e-12);
        assert_eq!(adjusted_calls[1], 0.0);
        assert_eq!(adjusted_dists, vec![0.0, 100.0]);
        assert!((ln_pme_nav(&calls, &dists, &index)? - 10.0).abs() < 1e-12);
        assert!(ln_pme(&calls, &dists, &index)?.is_finite());
        assert!((pme_plus_lambda(&calls, &dists, 10.0, &index)? - 1.0).abs() < 1e-12);
        let (scaled_calls, scaled_dists) = pme_plus_flows(&calls, &dists, 10.0, &index)?;
        assert_eq!(scaled_calls, calls);
        assert!((scaled_dists[0] - dists[0]).abs() < 1e-12);
        assert!((scaled_dists[1] - dists[1]).abs() < 1e-12);
        assert!(pme_plus(&calls, &dists, 10.0, &index)?.is_finite());
        let dates = [date(2024, 1, 1)?, date(2024, 12, 31)?];
        assert!((net_irr(&dates, &calls, &[0.0, 110.0], 0.0, dates[1])? - 0.1).abs() < 1e-10);
        assert!((gross_irr(&dates, &calls, &[0.0, 110.0], 0.0, dates[1])? - 0.1).abs() < 1e-10);
        assert!(m_pme(&calls, &dists, &index, &[0.0, 10.0])?.is_finite());
        assert!(m_pme(&calls, &[-1.0, 0.0], &index, &[0.0, 10.0]).is_err());
        assert!(moic(&[1.0], &[], 0.0).is_err());
        assert!(moic(&[0.0], &[1.0], -1.0).is_err());
        assert!(pic(1.0, 0.0).is_err());
        assert!(ks_pme_flows(&calls, &[0.0], &index).is_err());
        assert!(ks_pme_flows(&calls, &dists, &[0.0, 110.0]).is_err());
        assert!(pme_plus_lambda(&calls, &[0.0, 0.0], 0.0, &index).is_err());
        assert!(net_irr(&dates, &calls, &dists, -1.0, dates[1]).is_err());
        assert!(net_irr(&dates, &[100.0], &dists, 0.0, dates[1]).is_err());
        assert!(net_irr(&[], &[], &[], 0.0, dates[1]).is_err());
        assert!(net_irr(&dates, &calls, &dists, 0.0, date(2025, 2, 1)?).is_err());
        assert!(m_pme(&calls, &dists, &index, &[0.0]).is_err());
        assert!(m_pme(&calls, &dists, &[100.0, 0.0], &[0.0, 0.0]).is_err());
        Ok(())
    }
}
