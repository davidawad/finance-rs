//! Irregular (dated) cash flows: `xnpv` and `xirr` with pyxirr 0.10 semantics.
//!
//! A flow `a_i` on date `d_i` is discounted over `t_i = yf(d_0, d_i)` years,
//! where `d_0` is the earliest date and `yf` the [`DayCount`] (ACT/365F by
//! default, as in pyxirr and Excel):
//!
//! ```text
//! xnpv(r) = Σ_i a_i / (1+r)^t_i
//! ```
//!
//! Dates may be [`Date`] or (with the `chrono` feature) `chrono::NaiveDate`;
//! anything `Copy + Into<Date>` works. The `*_times` variants take year
//! fractions directly.

mod extras;
mod solver;
mod stream;

use crate::daycount::DayCount;
use crate::error::{all_finite, finite, output, same_len, FinanceError, Result};
use crate::Date;

/// Dated future values, duration, and cash-flow sign diagnostics.
///
/// ```
/// use finance_rs::cashflow::sign_changes;
/// assert_eq!(sign_changes(&[-1.0, 2.0]), vec![0]);
/// ```
pub use extras::{
    is_conventional, macaulay_duration_dated, sign_changes, xfv, xfv_with, xnfv, xnfv_with,
};
/// IRR solvers that accept explicit year fractions.
///
/// ```
/// let rate = finance_rs::cashflow::xirr_times(&[0.0, 1.0], &[-100.0, 110.0], 0.1)?;
/// assert!((rate - 0.1).abs() < 1e-8);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub use solver::{xirr_roots_times, xirr_times};
/// Typed dated cash-flow records and their composable stream wrapper.
///
/// ```
/// use finance_rs::cashflow::{CashFlow, FlowKind};
/// use finance_rs::Date;
/// let flow = CashFlow::new(Date::new(2026, 1, 1)?, -100.0, FlowKind::Contribution);
/// assert_eq!(flow.amount, -100.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub use stream::{CashFlow, CashFlowStream, FlowKind};

/// Settings for [`xirr_with`].
///
/// ```
/// let options = finance_rs::cashflow::XirrOptions::default();
/// assert_eq!(options.guess, 0.1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct XirrOptions {
    /// Starting point, and the tie-breaker when several rates solve the
    /// flows: the root Newton's method reaches from `guess`, else the one
    /// closest to it, wins (default 0.1, as pyxirr).
    pub guess: f64,
    /// Year-fraction convention (default ACT/365F).
    pub day_count: DayCount,
}

impl Default for XirrOptions {
    fn default() -> Self {
        Self {
            guess: 0.1,
            day_count: DayCount::Act365F,
        }
    }
}

/// Year fractions from the earliest date, one per flow:
/// `t_i = day_count.year_fraction(min(dates), dates[i])`.
///
/// ```rust
/// use finance_rs::{cashflow::year_fractions, daycount::DayCount, Date};
/// let dates = [Date::new(2021, 1, 1)?, Date::new(2020, 1, 1)?];
/// assert_eq!(year_fractions(&dates, DayCount::Act365F)?, vec![366.0 / 365.0, 0.0]);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::LengthMismatch`] / [`FinanceError::TooShort`] for bad input.
pub fn year_fractions<D: Copy + Into<Date>>(dates: &[D], day_count: DayCount) -> Result<Vec<f64>> {
    let dates: Vec<Date> = dates.iter().map(|d| (*d).into()).collect();
    let Some(&d0) = dates.iter().min() else {
        return Err(FinanceError::TooShort {
            what: "dates",
            min: 1,
            got: 0,
        });
    };
    Ok(dates
        .iter()
        .map(|d| day_count.year_fraction(d0, *d))
        .collect())
}

/// XNPV over year fractions `times`: `Σ amounts[i] / (1+rate)^times[i]`.
///
/// ```rust
/// use finance_rs::cashflow::xnpv_times;
/// let v = xnpv_times(0.1, &[0.0, 1.0, 2.0], &[-100.0, 55.0, 60.5])?;
/// assert!(v.abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] when `rate <= -1`; length and finiteness
/// errors for bad input; [`FinanceError::Overflow`] for a non-finite result.
pub fn xnpv_times(rate: f64, times: &[f64], amounts: &[f64]) -> Result<f64> {
    validate(times, amounts)?;
    finite("rate", rate)?;
    if rate <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "rate",
            reason: "must be greater than -1",
        });
    }
    let ln1p = rate.ln_1p();
    let sum = times
        .iter()
        .zip(amounts)
        .map(|(t, a)| a * (-t * ln1p).exp())
        .sum();
    output("xnpv", sum)
}

/// XNPV of dated flows under ACT/365F (pyxirr `xnpv`, Excel `XNPV`).
///
/// ```rust
/// use finance_rs::{cashflow::xnpv, Date};
/// let dates = [Date::new(2020, 1, 1)?, Date::new(2020, 3, 1)?, Date::new(2021, 1, 1)?];
/// let v = xnpv(0.1, &dates, &[-100.0, 50.0, 60.0])?;
/// assert!((v - 3.753_946_731_669_565).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`xnpv_times`].
pub fn xnpv<D: Copy + Into<Date>>(rate: f64, dates: &[D], amounts: &[f64]) -> Result<f64> {
    xnpv_with(rate, dates, amounts, DayCount::Act365F)
}

/// XNPV of dated flows under an explicit day count.
///
/// ```rust
/// use finance_rs::{cashflow::xnpv_with, daycount::DayCount, Date};
/// let dates = [Date::new(2020, 1, 1)?, Date::new(2021, 1, 1)?];
/// let v = xnpv_with(0.1, &dates, &[-100.0, 110.0], DayCount::Thirty360Us)?;
/// assert!(v.abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`xnpv_times`].
pub fn xnpv_with<D: Copy + Into<Date>>(
    rate: f64,
    dates: &[D],
    amounts: &[f64],
    day_count: DayCount,
) -> Result<f64> {
    same_len("dates", dates.len(), "amounts", amounts.len())?;
    xnpv_times(rate, &year_fractions(dates, day_count)?, amounts)
}

/// XIRR of dated flows under ACT/365F with guess 0.1 (pyxirr `xirr`, Excel `XIRR`).
///
/// Finds every rate `r > −1` with `xnpv(r) = 0` and returns the one Newton's
/// method converges to from the guess (pyxirr's choice), else the closest. Flows with one sign change have exactly one root, bracketed
/// from the guess outward and solved by Brent's method; otherwise a dense
/// log-spaced scan of `1+r ∈ [1e-9, 1e9]` brackets each sign change. If no
/// bracket exists (a tangent root), Newton's method from the guess is the
/// fallback.
///
/// ```rust
/// use finance_rs::{cashflow::xirr, Date};
/// let dates = [Date::new(2020, 1, 1)?, Date::new(2020, 3, 1)?, Date::new(2021, 1, 1)?];
/// let r = xirr(&dates, &[-100.0, 50.0, 60.0])?;
/// assert!((r - 0.169_725_639_989_622_37).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NoSignChange`] unless there is a positive and a negative
/// flow, [`FinanceError::NoSolution`] when no rate solves them, plus length
/// and finiteness errors.
pub fn xirr<D: Copy + Into<Date>>(dates: &[D], amounts: &[f64]) -> Result<f64> {
    xirr_with(dates, amounts, XirrOptions::default())
}

/// XIRR with an explicit guess and day count.
///
/// ```rust
/// use finance_rs::{cashflow::{xirr_with, XirrOptions}, daycount::DayCount, Date};
/// let dates = [Date::new(2020, 1, 1)?, Date::new(2021, 1, 1)?];
/// let opts = XirrOptions { day_count: DayCount::Act360, ..XirrOptions::default() };
/// let r = xirr_with(&dates, &[-100.0, 110.0], opts)?;
/// assert!((r - 0.098_282_633_848_621_13).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`xirr`].
pub fn xirr_with<D: Copy + Into<Date>>(
    dates: &[D],
    amounts: &[f64],
    opts: XirrOptions,
) -> Result<f64> {
    same_len("dates", dates.len(), "amounts", amounts.len())?;
    xirr_times(&year_fractions(dates, opts.day_count)?, amounts, opts.guess)
}

/// Every rate solving the dated flows, ascending (multiple-root reporting).
///
/// ```rust
/// use finance_rs::{cashflow::xirr_roots, daycount::DayCount, Date};
/// let dates = [Date::new(2021, 1, 1)?, Date::new(2022, 1, 1)?, Date::new(2023, 1, 1)?];
/// let roots = xirr_roots(&dates, &[-100.0, 230.0, -132.0], DayCount::Act365F)?;
/// assert_eq!(roots.len(), 2);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`xirr`].
pub fn xirr_roots<D: Copy + Into<Date>>(
    dates: &[D],
    amounts: &[f64],
    day_count: DayCount,
) -> Result<Vec<f64>> {
    same_len("dates", dates.len(), "amounts", amounts.len())?;
    xirr_roots_times(&year_fractions(dates, day_count)?, amounts)
}

pub(crate) fn validate(times: &[f64], amounts: &[f64]) -> Result<()> {
    same_len("times", times.len(), "amounts", amounts.len())?;
    if amounts.is_empty() {
        return Err(FinanceError::TooShort {
            what: "amounts",
            min: 1,
            got: 0,
        });
    }
    all_finite("times", times)?;
    all_finite("amounts", amounts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn earliest_date_anchors() -> Result<()> {
        let dates = [
            Date::new(2020, 3, 1)?,
            Date::new(2020, 1, 1)?,
            Date::new(2021, 1, 1)?,
        ];
        let v = xnpv(0.1, &dates, &[50.0, -100.0, 60.0])?;
        assert!((v - 3.753_946_731_669_565).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn errors_are_typed() -> Result<()> {
        let dates = [Date::new(2020, 1, 1)?, Date::new(2021, 1, 1)?];
        assert_eq!(xirr(&dates, &[1.0, 2.0]), Err(FinanceError::NoSignChange));
        assert!(matches!(
            xirr(&dates, &[1.0]),
            Err(FinanceError::LengthMismatch { .. })
        ));
        assert!(matches!(
            xnpv(-1.0, &dates, &[1.0, 2.0]),
            Err(FinanceError::InvalidInput { .. })
        ));
        assert!(matches!(
            xnpv(0.1, &dates, &[f64::NAN, 2.0]),
            Err(FinanceError::NonFinite { .. })
        ));
        let none: [Date; 0] = [];
        assert!(xirr(&none, &[]).is_err());
        Ok(())
    }
}
