//! Future values of dated flows, sign-change diagnostics and the duration
//! of a dated stream.

use super::{xnpv_with, year_fractions};
use crate::daycount::DayCount;
use crate::error::{finite, output, same_len, FinanceError, Result};
use crate::Date;

/// Future value at `end_date` of one flow on `cash_flow_date` (pyxirr
/// `xfv`, ACT/365F): the flow is discounted to `start_date` at
/// `cash_flow_rate`, then compounded to `end_date` at `end_rate`,
/// `amount·(1+end_rate)^τ(start, end)/(1+cash_flow_rate)^τ(start, cash_flow_date)`.
///
/// ```rust
/// use finance_rs::{cashflow::xfv, Date};
/// let d = |y, m| Date::new(y, m, 1);
/// let v = xfv(d(2020, 1)?, d(2020, 6)?, d(2021, 1)?, 0.05, 0.08, 100.0)?;
/// assert!((v - 105.850_104_362_555_48).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`xfv_with`].
#[allow(clippy::similar_names)] // pyxirr's argument names
pub fn xfv<D: Copy + Into<Date>>(
    start_date: D,
    cash_flow_date: D,
    end_date: D,
    cash_flow_rate: f64,
    end_rate: f64,
    amount: f64,
) -> Result<f64> {
    xfv_with(
        start_date,
        cash_flow_date,
        end_date,
        cash_flow_rate,
        end_rate,
        amount,
        DayCount::Act365F,
    )
}

/// [`xfv`] under an explicit day count.
///
/// ```
/// use finance_rs::{cashflow::xfv_with, daycount::DayCount, Date};
/// let start = Date::new(2026, 1, 1)?;
/// let end = Date::new(2027, 1, 1)?;
/// assert!(xfv_with(start, start, end, 0.05, 0.08, 100.0, DayCount::Act365F)? > 100.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for a rate at or below −1,
/// [`FinanceError::NonFinite`] / [`FinanceError::Overflow`].
#[allow(clippy::similar_names)]
pub fn xfv_with<D: Copy + Into<Date>>(
    start_date: D,
    cash_flow_date: D,
    end_date: D,
    cash_flow_rate: f64,
    end_rate: f64,
    amount: f64,
    day_count: DayCount,
) -> Result<f64> {
    finite("amount", amount)?;
    for (what, r) in [("cash_flow_rate", cash_flow_rate), ("end_rate", end_rate)] {
        finite(what, r)?;
        if r <= -1.0 {
            return Err(FinanceError::InvalidInput {
                what,
                reason: "must be greater than -1",
            });
        }
    }
    let start = start_date.into();
    let t_end = day_count.year_fraction(start, end_date.into());
    let t_cf = day_count.year_fraction(start, cash_flow_date.into());
    let log_growth = t_end * end_rate.ln_1p() - t_cf * cash_flow_rate.ln_1p();
    output("xfv", amount * log_growth.exp())
}

/// Net future value of dated flows at the latest date:
/// `xnpv(rate)·(1+rate)^τ_max` (pyxirr `xnfv`, ACT/365F).
///
/// ```rust
/// use finance_rs::{cashflow::xnfv, Date};
/// let dates = [Date::new(2020, 1, 1)?, Date::new(2021, 1, 1)?];
/// let v = xnfv(0.05, &dates, &[-100.0, 50.0])?;
/// assert!((v + 55.014_036_464_796_92).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`super::xnpv`].
pub fn xnfv<D: Copy + Into<Date>>(rate: f64, dates: &[D], amounts: &[f64]) -> Result<f64> {
    xnfv_with(rate, dates, amounts, DayCount::Act365F)
}

/// [`xnfv`] under an explicit day count.
///
/// ```
/// use finance_rs::{cashflow::xnfv_with, daycount::DayCount, Date};
/// let dates = [Date::new(2026, 1, 1)?, Date::new(2027, 1, 1)?];
/// let value = xnfv_with(0.05, &dates, &[-100.0, 110.0], DayCount::Act365F)?;
/// assert!(value.is_finite());
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`super::xnpv_with`].
pub fn xnfv_with<D: Copy + Into<Date>>(
    rate: f64,
    dates: &[D],
    amounts: &[f64],
    day_count: DayCount,
) -> Result<f64> {
    let pv = xnpv_with(rate, dates, amounts, day_count)?;
    let horizon = year_fractions(dates, day_count)?
        .into_iter()
        .fold(0.0, f64::max);
    output("xnfv", pv * (horizon * rate.ln_1p()).exp())
}

fn signed(x: f64) -> Option<bool> {
    (x != 0.0 && !x.is_nan()).then_some(x > 0.0)
}

/// Where the flows change direction: the index of each non-zero amount
/// whose next non-zero amount has the opposite sign (pyxirr
/// `zero_crossing_points`). Zeros and `NaN`s carry no sign. Descartes' rule
/// bounds the number of IRR roots by the length of this list.
///
/// ```rust
/// use finance_rs::cashflow::sign_changes;
/// assert_eq!(sign_changes(&[-1.0, 0.0, 2.0, -3.0, 0.0, 4.0]), vec![0, 2, 3]);
/// ```
#[must_use]
pub fn sign_changes(amounts: &[f64]) -> Vec<usize> {
    let mut out = Vec::new();
    let mut last: Option<(usize, bool)> = None;
    for (i, a) in amounts.iter().enumerate() {
        if let Some(s) = signed(*a) {
            if let Some((j, prev)) = last {
                if prev != s {
                    out.push(j);
                }
            }
            last = Some((i, s));
        }
    }
    out
}

/// Whether the flows change direction exactly once, so their IRR is unique
/// (pyxirr `is_conventional_cash_flow`).
///
/// ```rust
/// use finance_rs::cashflow::is_conventional;
/// assert!(is_conventional(&[-100.0, 0.0, 60.0, 60.0]));
/// assert!(!is_conventional(&[-100.0, 230.0, -132.0]));
/// ```
#[must_use]
pub fn is_conventional(amounts: &[f64]) -> bool {
    sign_changes(amounts).len() == 1
}

/// Macaulay duration of a dated stream, in years:
/// `Σ τ_i·PV_i / Σ PV_i`, with `PV_i = a_i/(1+r)^τ_i` and `τ_i` the year
/// fraction from the earliest date.
///
/// ```rust
/// use finance_rs::{cashflow::macaulay_duration_dated, daycount::DayCount, Date};
/// // A 2-year 5% annual bond priced at a 5% yield: D = 1.952…
/// let dates = [Date::new(2025, 1, 1)?, Date::new(2026, 1, 1)?, Date::new(2027, 1, 1)?];
/// let d = macaulay_duration_dated(0.05, &dates, &[0.0, 5.0, 105.0], DayCount::Thirty360Us)?;
/// assert!((d - (5.0 / 1.05 + 2.0 * 105.0 / 1.1025) / 100.0).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::Undefined`] when the flows' present value is zero;
/// otherwise as [`super::xnpv_with`].
pub fn macaulay_duration_dated<D: Copy + Into<Date>>(
    rate: f64,
    dates: &[D],
    amounts: &[f64],
    day_count: DayCount,
) -> Result<f64> {
    same_len("dates", dates.len(), "amounts", amounts.len())?;
    let pv = xnpv_with(rate, dates, amounts, day_count)?;
    let ln1p = rate.ln_1p();
    let times = year_fractions(dates, day_count)?;
    let weighted: f64 = times
        .iter()
        .zip(amounts)
        .map(|(t, a)| t * a * (-t * ln1p).exp())
        .sum();
    crate::error::div("present value", weighted, pv)
}
