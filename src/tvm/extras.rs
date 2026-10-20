//! Cumulative interest and principal, net future value, variable-rate
//! compounding and growing perpetuities and annuities.

use super::{ipmt, npv, ppmt, When};
use crate::error::{all_finite, div, finite, output, FinanceError, Result};

fn above_minus_one(what: &'static str, r: f64) -> Result<f64> {
    finite(what, r)?;
    if r <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what,
            reason: "must be greater than -1",
        });
    }
    Ok(r)
}

fn period_range(nper: f64, start: u32, end: u32) -> Result<()> {
    finite("nper", nper)?;
    if start < 1 {
        return Err(FinanceError::InvalidInput {
            what: "start",
            reason: "periods are numbered from 1",
        });
    }
    if end < start {
        return Err(FinanceError::InvalidInput {
            what: "end",
            reason: "must not precede start",
        });
    }
    if f64::from(end) > nper {
        return Err(FinanceError::InvalidInput {
            what: "end",
            reason: "must not exceed nper",
        });
    }
    Ok(())
}

/// Interest paid from period `start` through `end` (1-based, inclusive) on
/// a level-payment loan of `pv` over `nper` periods:
/// `Σ_{p=start..end} ipmt(rate, p, nper, pv, 0, when)` (pyxirr `cumipmt`,
/// Excel `CUMIPMT`). Negative for a loan (`pv > 0`).
///
/// ```
/// use finance_rs::tvm::{cumipmt, When};
/// let i = cumipmt(0.09 / 12.0, 360.0, 125_000.0, 13, 24, When::End)?;
/// assert!((i + 11_135.232_130_750_845).abs() < 1e-8);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] unless `1 ≤ start ≤ end ≤ nper`; otherwise
/// as [`ipmt`].
pub fn cumipmt(rate: f64, nper: f64, pv: f64, start: u32, end: u32, when: When) -> Result<f64> {
    period_range(nper, start, end)?;
    let mut sum = 0.0;
    for p in start..=end {
        sum += ipmt(rate, f64::from(p), nper, pv, 0.0, when)?;
    }
    output("cumipmt", sum)
}

/// Principal repaid from period `start` through `end` (1-based, inclusive):
/// `Σ_{p=start..end} ppmt(rate, p, nper, pv, 0, when)` (pyxirr `cumprinc`,
/// Excel `CUMPRINC`).
///
/// ```
/// use finance_rs::tvm::{cumprinc, When};
/// let p = cumprinc(0.09 / 12.0, 360.0, 125_000.0, 13, 24, When::End)?;
/// assert!((p + 934.107_123_420_877).abs() < 1e-8);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`cumipmt`].
pub fn cumprinc(rate: f64, nper: f64, pv: f64, start: u32, end: u32, when: When) -> Result<f64> {
    period_range(nper, start, end)?;
    let mut sum = 0.0;
    for p in start..=end {
        sum += ppmt(rate, f64::from(p), nper, pv, 0.0, when)?;
    }
    output("cumprinc", sum)
}

/// Net future value after `nper` periods of flows paid at the *end* of
/// periods 1, 2, … (Excel's `NPV` timing, as pyxirr `nfv` uses):
/// `Σ_i values[i]·(1+r)^(nper − i − 1)`.
///
/// ```
/// use finance_rs::tvm::nfv;
/// let v = nfv(0.05, 10.0, &[-100.0, 10.0, 20.0, 30.0])?;
/// assert!((v + 72.013_389_488_085_98).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for `rate ≤ −1`; otherwise as [`npv`].
pub fn nfv(rate: f64, nper: f64, values: &[f64]) -> Result<f64> {
    above_minus_one("rate", rate)?;
    finite("nper", nper)?;
    let pv = npv(rate, values)?;
    output("nfv", pv * (rate.ln_1p() * (nper - 1.0)).exp())
}

/// Future value of `principal` under a schedule of per-period rates:
/// `principal·Π(1 + rates[i])` (Excel `FVSCHEDULE`); an empty schedule
/// returns `principal`.
///
/// ```
/// use finance_rs::tvm::fv_schedule;
/// // Excel's documentation example.
/// assert!((fv_schedule(1.0, &[0.09, 0.11, 0.1])? - 1.33089).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NonFinite`] for non-finite input,
/// [`FinanceError::Overflow`] for a non-finite result.
pub fn fv_schedule(principal: f64, rates: &[f64]) -> Result<f64> {
    finite("principal", principal)?;
    all_finite("rates", rates)?;
    output(
        "fv_schedule",
        rates.iter().fold(principal, |acc, r| acc * (1.0 + r)),
    )
}

fn rate_above_growth(rate: f64, growth: f64) -> Result<()> {
    above_minus_one("rate", rate)?;
    above_minus_one("growth", growth)?;
    if rate <= growth {
        return Err(FinanceError::InvalidInput {
            what: "rate",
            reason: "must exceed growth for a finite perpetuity",
        });
    }
    Ok(())
}

/// Present value of a perpetuity whose first flow is `cash_flow` and which
/// grows by `growth` each period: `C/(r − g)` for flows at period ends,
/// `C·(1 + r)/(r − g)` for flows at period starts (Gordon).
///
/// ```
/// use finance_rs::tvm::{perpetuity_pv, When};
/// assert!((perpetuity_pv(100.0, 0.08, 0.03, When::End)? - 2_000.0).abs() < 1e-9);
/// assert!((perpetuity_pv(100.0, 0.08, 0.03, When::Begin)? - 2_160.0).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] when `rate ≤ growth` (no finite value) or
/// either is at or below −100%; finiteness errors.
pub fn perpetuity_pv(cash_flow: f64, rate: f64, growth: f64, when: When) -> Result<f64> {
    finite("cash_flow", cash_flow)?;
    rate_above_growth(rate, growth)?;
    let pv = cash_flow / (rate - growth);
    output("perpetuity_pv", pv * (1.0 + rate * when.factor()))
}

/// Present value of `nper` flows starting at `cash_flow` and growing by
/// `growth` each period:
/// `C/(r − g)·(1 − ((1+g)/(1+r))ⁿ)`, or `n·C/(1+r)` when `r = g`; times
/// `(1 + r)` for flows at period starts.
///
/// ```
/// use finance_rs::tvm::{growing_annuity_pv, pv, When};
/// // With no growth it is the level annuity.
/// let g = growing_annuity_pv(100.0, 0.05, 0.0, 10.0, When::End)?;
/// assert!((g - pv(0.05, 10.0, -100.0, 0.0, When::End)?).abs() < 1e-9);
/// assert!((growing_annuity_pv(100.0, 0.05, 0.05, 10.0, When::End)? - 1_000.0 / 1.05).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for `nper < 0` or a rate or growth at or
/// below −100%; finiteness and overflow errors.
#[allow(clippy::float_cmp)] // r = g is the closed form's removable singularity
pub fn growing_annuity_pv(
    cash_flow: f64,
    rate: f64,
    growth: f64,
    nper: f64,
    when: When,
) -> Result<f64> {
    finite("cash_flow", cash_flow)?;
    above_minus_one("rate", rate)?;
    above_minus_one("growth", growth)?;
    finite("nper", nper)?;
    if nper < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "nper",
            reason: "must be non-negative",
        });
    }
    let pv = if rate == growth {
        nper * cash_flow / (1.0 + rate)
    } else {
        // 1 − ((1+g)/(1+r))ⁿ without forming the ratio.
        let shrink = -(nper * (growth.ln_1p() - rate.ln_1p())).exp_m1();
        cash_flow / (rate - growth) * shrink
    };
    output("growing_annuity_pv", pv * (1.0 + rate * when.factor()))
}

/// Future value at period `nper` of the growing annuity:
/// `growing_annuity_pv·(1 + r)ⁿ`.
///
/// ```
/// use finance_rs::tvm::{growing_annuity_fv, When};
/// let v = growing_annuity_fv(100.0, 0.1, 0.05, 2.0, When::End)?;
/// assert!((v - (100.0 * 1.1 + 105.0)).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`growing_annuity_pv`].
pub fn growing_annuity_fv(
    cash_flow: f64,
    rate: f64,
    growth: f64,
    nper: f64,
    when: When,
) -> Result<f64> {
    let pv = growing_annuity_pv(cash_flow, rate, growth, nper, when)?;
    output("growing_annuity_fv", pv * (rate.ln_1p() * nper).exp())
}

/// Equivalent annual annuity of a project with flows `values[0..=n]` at
/// `t = 0..n`: the level end-of-period payment over `n` periods with the
/// same NPV, `npv·r/(1 − (1+r)^−n)` (`npv/n` at `r = 0`).
///
/// ```
/// use finance_rs::tvm::equivalent_annual_annuity;
/// let e = equivalent_annual_annuity(0.1, &[-1_000.0, 600.0, 600.0])?;
/// assert!((e - 23.809_523_809_523_8).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for fewer than two values,
/// [`FinanceError::InvalidInput`] for `rate ≤ −1`; as [`npv`] otherwise.
#[allow(clippy::cast_precision_loss)]
pub fn equivalent_annual_annuity(rate: f64, values: &[f64]) -> Result<f64> {
    above_minus_one("rate", rate)?;
    crate::error::min_len("values", values, 2)?;
    let n = (values.len() - 1) as f64;
    let v = npv(rate, values)?;
    if rate == 0.0 {
        return output("equivalent_annual_annuity", v / n);
    }
    div("annuity factor", v * rate, -(-n * rate.ln_1p()).exp_m1())
}
