//! Time value of money with numpy-financial 1.x semantics.
//!
//! Sign convention (as in numpy-financial and every spreadsheet): money you
//! receive is positive, money you pay is negative, so a loan has `pv > 0`
//! and `pmt < 0`. Every function solves the same identity
//!
//! ```text
//! fv + pv·(1+r)^n + pmt·(1+r·w)/r·((1+r)^n − 1) = 0      (r ≠ 0)
//! fv + pv + pmt·n = 0                                      (r = 0)
//! ```
//!
//! for one unknown, where `w` is 0 for payments at period end and 1 for
//! payments at period start ([`When`]).

mod extras;
mod irr;
mod payback;
mod rate;

pub use extras::{
    cumipmt, cumprinc, equivalent_annual_annuity, fv_schedule, growing_annuity_fv,
    growing_annuity_pv, nfv, perpetuity_pv,
};
pub use irr::{irr, irr_bisection, irr_roots, irr_with_bounds, mirr, npv};
pub use payback::{discounted_payback, payback_period, profitability_index};
pub use rate::{rate, rate_with, RateOptions};

use crate::error::{div, finite, output, FinanceError, Result};

/// When payments fall within each period (numpy-financial `when`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum When {
    /// Payments at the end of each period (ordinary annuity, `when='end'`).
    #[default]
    End,
    /// Payments at the start of each period (annuity due, `when='begin'`).
    Begin,
}

/// Accumulation factor from one period to another: `(1 + rate)^periods`.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn discount_factor(rate: f64, periods: f64) -> Result<f64> {
    finite("rate", rate)?;
    finite("periods", periods)?;
    if rate <= -1.0 || periods < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "discount factor",
            reason: "rate must exceed -1 and periods must be non-negative",
        });
    }
    output("discount_factor", (1.0 + rate).powf(periods))
}

/// Present value factor for equal payments at the end of each period.
///
/// This uses the closed form `((1+r)^n - 1) / (r(1+r)^n)` so financial
/// adapters can preserve established operation order without maintaining a
/// local discount loop.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn annuity_pv_factor(rate: f64, periods: f64) -> Result<f64> {
    finite("rate", rate)?;
    finite("periods", periods)?;
    if rate <= -1.0 || periods < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "annuity",
            reason: "rate must exceed -1 and periods must be non-negative",
        });
    }
    if rate == 0.0 {
        return output("annuity pv factor", periods);
    }
    let factor = 1.0 + rate;
    output(
        "annuity pv factor",
        (1.0 - factor.powf(-periods)) / (factor - 1.0),
    )
}

/// Discount one future amount by `periods` at an annual or periodic rate.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn present_value_factor(rate: f64, periods: f64) -> Result<f64> {
    finite("rate", rate)?;
    finite("periods", periods)?;
    if rate <= -1.0 || periods < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "present value factor",
            reason: "rate must exceed -1 and periods must be non-negative",
        });
    }
    output("present value factor", (1.0 + rate).powf(-periods))
}

/// Raise a positive accumulation or discount factor to a period count.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn factor_power(factor: f64, periods: f64) -> Result<f64> {
    finite("factor", factor)?;
    finite("periods", periods)?;
    if factor <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "factor",
            reason: "must be positive",
        });
    }
    output("factor power", factor.powf(periods))
}

/// Sum end-of-period discounts `d + d² + … + dⁿ`.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn geometric_discount_sum(discount: f64, periods: f64) -> Result<f64> {
    finite("discount", discount)?;
    finite("periods", periods)?;
    if discount <= 0.0 || periods < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "discount sum",
            reason: "discount must be positive and periods must be non-negative",
        });
    }
    let sum = if (1.0 - discount).abs() < 1e-12 {
        periods
    } else {
        discount * (1.0 - discount.powf(periods)) / (1.0 - discount)
    };
    output("discount sum", sum)
}

impl When {
    /// The `w` in the TVM identity: 0 for [`When::End`], 1 for [`When::Begin`].
    ///
    /// ```
    /// use finance_rs::tvm::When;
    /// assert_eq!(When::End.factor(), 0.0);
    /// assert_eq!(When::Begin.factor(), 1.0);
    /// ```
    #[must_use]
    pub fn factor(self) -> f64 {
        match self {
            Self::End => 0.0,
            Self::Begin => 1.0,
        }
    }
}

/// The annuity factor `(1+r·w)·((1+r)^n − 1)/r` (or `n` when `r = 0`).
fn annuity_factor(rate: f64, nper: f64, when: When) -> f64 {
    if rate == 0.0 {
        nper
    } else {
        (1.0 + rate * when.factor()) * ((1.0 + rate).powf(nper) - 1.0) / rate
    }
}

/// Future value.
///
/// `fv = −pv·(1+r)^n − pmt·(1+r·w)/r·((1+r)^n − 1)`, or `−(pv + pmt·n)` at `r = 0`.
///
/// ```
/// use finance_rs::tvm::{fv, When};
/// // Save 100/month for 10 years at 5%/yr, starting with 100.
/// let v = fv(0.05 / 12.0, 120.0, -100.0, -100.0, When::End)?;
/// assert!((v - 15_692.928_894_335_748).abs() < 1e-6);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NonFinite`] for non-finite input, [`FinanceError::Overflow`]
/// when the result is not finite.
pub fn fv(rate: f64, nper: f64, pmt: f64, pv: f64, when: When) -> Result<f64> {
    check4(rate, nper, pmt, pv)?;
    if rate == 0.0 {
        return output("fv", -(pv + pmt * nper));
    }
    let temp = (1.0 + rate).powf(nper);
    output(
        "fv",
        -pv * temp - pmt * (1.0 + rate * when.factor()) / rate * (temp - 1.0),
    )
}

/// Number of periods required for `pv` to grow to `fv` at a constant rate.
/// This is Excel `PDURATION`: `ln(fv/pv) / ln(1+rate)`.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
pub fn pduration(rate: f64, pv: f64, fv: f64) -> Result<f64> {
    finite("rate", rate)?;
    finite("pv", pv)?;
    finite("fv", fv)?;
    if rate <= -1.0 || pv <= 0.0 || fv <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "pduration",
            reason: "rate must exceed -1 and pv/fv must be positive",
        });
    }
    if rate == 0.0 {
        return if pv == fv {
            Ok(0.0)
        } else {
            Err(FinanceError::Undefined {
                what: "zero growth rate",
            })
        };
    }
    output("pduration", (fv / pv).ln() / rate.ln_1p())
}

/// Equivalent constant compound rate from `pv` to `fv` over `nper` periods
/// (Excel `RRI`): `(fv/pv)^(1/nper) - 1`.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn rri(nper: f64, pv: f64, fv: f64) -> Result<f64> {
    finite("nper", nper)?;
    finite("pv", pv)?;
    finite("fv", fv)?;
    if nper <= 0.0 || pv <= 0.0 || fv <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "rri",
            reason: "nper, pv, and fv must be positive",
        });
    }
    output("rri", ((fv / pv).ln() / nper).exp_m1())
}

/// Interest paid in a particular period under simple interest (Excel `ISPMT`).
/// Periods are zero-based and the payment is negative for a positive `pv`.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn ispmt(rate: f64, per: f64, nper: f64, pv: f64) -> Result<f64> {
    finite("rate", rate)?;
    finite("per", per)?;
    finite("nper", nper)?;
    finite("pv", pv)?;
    if per < 0.0 || nper <= 0.0 || per >= nper {
        return Err(FinanceError::InvalidInput {
            what: "period",
            reason: "must satisfy 0 <= per < nper and nper > 0",
        });
    }
    output("ispmt", -pv * rate * (1.0 - per / nper))
}

/// Present value.
///
/// `pv = −(fv + pmt·(1+r·w)·((1+r)^n − 1)/r) / (1+r)^n`, with the factor `n` at `r = 0`.
///
/// ```
/// use finance_rs::tvm::{pv, When};
/// // What loan does 2,000/month for 15 years at 7%/yr buy?
/// let v = pv(0.07 / 12.0, 180.0, -2_000.0, 0.0, When::End)?;
/// assert!((v - 222_511.915_218_604_9).abs() < 1e-6);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::Undefined`] when `(1+r)^n = 0`, otherwise as [`fv`].
pub fn pv(rate: f64, nper: f64, pmt: f64, fv: f64, when: When) -> Result<f64> {
    check4(rate, nper, pmt, fv)?;
    let temp = (1.0 + rate).powf(nper);
    div(
        "(1+rate)^nper",
        -(fv + pmt * annuity_factor(rate, nper, when)),
        temp,
    )
}

/// Payment per period.
///
/// `pmt = −(fv + pv·(1+r)^n) / ((1+r·w)·((1+r)^n − 1)/r)`, with the factor `n` at `r = 0`.
///
/// ```
/// use finance_rs::tvm::{pmt, When};
/// let p = pmt(0.075 / 12.0, 12.0 * 15.0, 200_000.0, 0.0, When::End)?;
/// assert!((p + 1_854.024_720_005_461_9).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::Undefined`] when the annuity factor is zero (`nper = 0`),
/// otherwise as [`fv`].
pub fn pmt(rate: f64, nper: f64, pv: f64, fv: f64, when: When) -> Result<f64> {
    check4(rate, nper, pv, fv)?;
    let temp = (1.0 + rate).powf(nper);
    div(
        "annuity factor",
        -(fv + pv * temp),
        annuity_factor(rate, nper, when),
    )
}

/// Positive level payment for legacy underwriting paths using the direct
/// `pv * rate * growth / (growth - 1)` operation order. The conventional
/// signed spreadsheet API remains [`pmt`].
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
pub fn positive_level_payment(rate: f64, nper: f64, pv: f64) -> Result<f64> {
    check4(rate, nper, pv, 0.0)?;
    if rate == 0.0 {
        return div("nper", pv, nper);
    }
    let growth = (1.0 + rate).powf(nper);
    if growth == 1.0 {
        return div("nper", pv, nper);
    }
    output("pmt", pv * rate * growth / (growth - 1.0))
}

/// Positive level payment using the inverse-growth operation order used by
/// legacy annual underwriting schedules: `P·r / (1 − (1+r)^−n)`.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn positive_level_payment_discount(rate: f64, nper: f64, pv: f64) -> Result<f64> {
    check4(rate, nper, pv, 0.0)?;
    if rate == 0.0 {
        return div("nper", pv, nper);
    }
    div("annuity factor", pv * rate, 1.0 - (1.0 + rate).powf(-nper))
}

/// Number of periods.
///
/// `n = ln((pmt·(1+r·w)/r − fv) / (pmt·(1+r·w)/r + pv)) / ln(1+r)`, or
/// `−(fv + pv)/pmt` at `r = 0`.
///
/// ```
/// use finance_rs::tvm::{nper, When};
/// // How long to pay off 8,000 at 7%/yr with 150/month?
/// let n = nper(0.07 / 12.0, -150.0, 8_000.0, 0.0, When::End)?;
/// assert!((n - 64.073_348_770_661_86).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NoSolution`] when the payment never retires the balance
/// (the log argument is not positive), [`FinanceError::Undefined`] for
/// `pmt = 0` at `r = 0`.
pub fn nper(rate: f64, pmt: f64, pv: f64, fv: f64, when: When) -> Result<f64> {
    check4(rate, pmt, pv, fv)?;
    if rate == 0.0 {
        return div("pmt", -(fv + pv), pmt);
    }
    if rate <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "rate",
            reason: "must be greater than -1",
        });
    }
    let z = pmt * (1.0 + rate * when.factor()) / rate;
    let ratio = (-fv + z) / (pv + z);
    if ratio.is_nan() || ratio <= 0.0 || ratio.is_infinite() {
        return Err(FinanceError::NoSolution { what: "nper" });
    }
    div("ln(1+rate)", ratio.ln(), rate.ln_1p())
}

/// Interest portion of the payment in period `per` (1-based).
///
/// `ipmt = r · fv(r, per−1, pmt, pv, w)`; for [`When::Begin`] it is 0 in
/// period 1 and divided by `1+r` after.
///
/// ```
/// use finance_rs::tvm::{ipmt, When};
/// let i = ipmt(0.0824 / 12.0, 1.0, 12.0, 2_500.0, 0.0, When::End)?;
/// assert!((i + 17.166_666_666_666_67).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] when `per < 1`, otherwise as [`pmt`].
#[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
pub fn ipmt(rate: f64, per: f64, nper: f64, pv: f64, fv: f64, when: When) -> Result<f64> {
    finite("per", per)?;
    if per < 1.0 {
        return Err(FinanceError::InvalidInput {
            what: "per",
            reason: "periods are numbered from 1",
        });
    }
    let total = pmt(rate, nper, pv, fv, when)?;
    let interest = self::fv(rate, per - 1.0, total, pv, when)? * rate;
    match when {
        When::End => output("ipmt", interest),
        When::Begin if per == 1.0 => Ok(0.0),
        When::Begin => div("1+rate", interest, 1.0 + rate),
    }
}

/// Principal portion of the payment in period `per`: `ppmt = pmt − ipmt`.
///
/// ```
/// use finance_rs::tvm::{ipmt, pmt, ppmt, When};
/// let (r, n, p) = (0.0824 / 12.0, 12.0, 2_500.0);
/// let total = pmt(r, n, p, 0.0, When::End)?;
/// let split = ppmt(r, 3.0, n, p, 0.0, When::End)? + ipmt(r, 3.0, n, p, 0.0, When::End)?;
/// assert!((split - total).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`ipmt`].
pub fn ppmt(rate: f64, per: f64, nper: f64, pv: f64, fv: f64, when: When) -> Result<f64> {
    let total = pmt(rate, nper, pv, fv, when)?;
    output("ppmt", total - ipmt(rate, per, nper, pv, fv, when)?)
}

fn check4(a: f64, b: f64, c: f64, d: f64) -> Result<()> {
    finite("rate", a)?;
    finite("nper", b)?;
    finite("pmt", c)?;
    finite("pv/fv", d)?;
    Ok(())
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
