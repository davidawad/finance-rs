//! Exact-decimal TVM for money paths (feature `decimal`, `rust_decimal`).
//!
//! The same identities as [`crate::tvm`], restricted to whole periods so
//! `(1+r)^n` is exact repeated multiplication, plus a level-payment schedule
//! rounded to the cent whose final payment absorbs the rounding residue, so
//! the balance ends at exactly zero.
//!
//! ```
//! use finance_rs::decimal::{level_schedule, pmt};
//! use finance_rs::tvm::When;
//! use rust_decimal::Decimal;
//! let rate = Decimal::new(5, 3); // 0.5% per month
//! let p = pmt(rate, 360, Decimal::from(300_000), Decimal::ZERO, When::End)?;
//! assert_eq!(p.round_dp(2), Decimal::new(-179_865, 2));
//! let rows = level_schedule(Decimal::from(300_000), rate, 360, 2)?;
//! assert_eq!(rows.last().map(|r| r.balance), Some(Decimal::ZERO));
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal, MathematicalOps,
};

use crate::error::{FinanceError, Result};
use crate::tvm::When;

const OVERFLOW: FinanceError = FinanceError::Overflow { what: "decimal" };

fn as_f64(value: Decimal) -> f64 {
    // Decimal's bounded 96-bit coefficient is far inside f64's finite range;
    // `None` cannot occur for a value representable by rust_decimal.
    value.to_f64().unwrap_or(f64::NAN)
}

/// Periodic IRR for exact-decimal cash flows, solved by the shared f64 root
/// finder and converted back to Decimal. This avoids unsafe Decimal powers at
/// the extreme lower bracket while retaining Decimal inputs and output.
///
/// ```rust
/// use finance_rs::decimal::irr; use rust_decimal::Decimal; let rate = irr(&[Decimal::from(-100), Decimal::from(110)])?; assert!((rate - Decimal::new(1, 1)).abs() < Decimal::new(1, 6));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn irr(values: &[Decimal]) -> Result<Decimal> {
    let values = values
        .iter()
        .map(|value| as_f64(*value))
        .collect::<Vec<_>>();
    let rate = crate::tvm::irr(&values)?;
    Decimal::from_f64(rate).ok_or(OVERFLOW)
}

/// Actual/365 XIRR for Decimal cash flows and day offsets from the first
/// flow. Inputs preserve their supplied order, matching `cashflow` APIs.
///
/// ```rust
/// use finance_rs::decimal::xirr; use rust_decimal::Decimal; let rate = xirr(&[(Decimal::from(-100), 0), (Decimal::from(110), 365)])?; assert!((rate - Decimal::new(1, 1)).abs() < Decimal::new(1, 6));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn xirr(values: &[(Decimal, i32)]) -> Result<Decimal> {
    let amounts = values
        .iter()
        .map(|(value, _)| as_f64(*value))
        .collect::<Vec<_>>();
    let times = values
        .iter()
        .map(|(_, day)| f64::from(*day) / 365.0)
        .collect::<Vec<_>>();
    let rate = crate::cashflow::xirr_times(&times, &amounts, 0.1)?;
    Decimal::from_f64(rate).ok_or(OVERFLOW)
}

/// Actual/365 XNPV for Decimal cash flows and day offsets from the first
/// flow. Dates before the first flow are rejected by the shared f64 API.
///
/// ```rust
/// use finance_rs::decimal::xnpv; use rust_decimal::Decimal; let value = xnpv(Decimal::new(1, 1), &[(Decimal::from(-100), 0), (Decimal::from(110), 365)])?; assert!(value.abs() < Decimal::new(1, 6));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn xnpv(rate: Decimal, values: &[(Decimal, i32)]) -> Result<Decimal> {
    let amounts = values
        .iter()
        .map(|(value, _)| as_f64(*value))
        .collect::<Vec<_>>();
    let times = values
        .iter()
        .map(|(_, day)| f64::from(*day) / 365.0)
        .collect::<Vec<_>>();
    let rate = as_f64(rate);
    Decimal::from_f64(crate::cashflow::xnpv_times(rate, &times, &amounts)?).ok_or(OVERFLOW)
}

/// Net operating income divided by property value.
///
/// ```rust
/// use finance_rs::decimal::re_cap_rate; use rust_decimal::Decimal; assert_eq!(re_cap_rate(Decimal::from(12_000), Decimal::from(200_000)), Some(Decimal::new(6, 2)));
/// ```
#[must_use]
pub fn re_cap_rate(noi: Decimal, property_value: Decimal) -> Option<Decimal> {
    (property_value > Decimal::ZERO).then(|| noi / property_value)
}

/// Net operating income divided by annual debt service.
///
/// ```rust
/// use finance_rs::decimal::re_dscr; use rust_decimal::Decimal; assert_eq!(re_dscr(Decimal::from(120_000), Decimal::from(100_000)), Some(Decimal::new(12, 1)));
/// ```
#[must_use]
pub fn re_dscr(noi: Decimal, debt_service: Decimal) -> Option<Decimal> {
    (debt_service > Decimal::ZERO).then(|| noi / debt_service)
}

/// Annual cash flow divided by cash invested.
///
/// ```rust
/// use finance_rs::decimal::re_cash_on_cash; use rust_decimal::Decimal; assert_eq!(re_cash_on_cash(Decimal::from(12_000), Decimal::from(100_000)), Some(Decimal::new(12, 2)));
/// ```
#[must_use]
pub fn re_cash_on_cash(cash_flow: Decimal, cash_invested: Decimal) -> Option<Decimal> {
    (cash_invested > Decimal::ZERO).then(|| cash_flow / cash_invested)
}

/// Stabilized annual NOI divided by total project cost.
///
/// ```rust
/// use finance_rs::decimal::re_yield_on_cost; use rust_decimal::Decimal; assert_eq!(re_yield_on_cost(Decimal::from(15_000), Decimal::from(250_000)), Some(Decimal::new(6, 2)));
/// ```
#[must_use]
pub fn re_yield_on_cost(noi: Decimal, total_project_cost: Decimal) -> Option<Decimal> {
    (total_project_cost > Decimal::ZERO).then(|| noi / total_project_cost)
}

/// Annual gross rent divided by property value.
///
/// ```rust
/// use finance_rs::decimal::re_gross_yield; use rust_decimal::Decimal; assert_eq!(re_gross_yield(Decimal::from(24_000), Decimal::from(400_000)), Some(Decimal::new(6, 2)));
/// ```
#[must_use]
pub fn re_gross_yield(gross_rent: Decimal, property_value: Decimal) -> Option<Decimal> {
    (property_value > Decimal::ZERO).then(|| gross_rent / property_value)
}

/// Return on contributed capital: `(distributions + terminal equity - capital) / capital`.
///
/// ```rust
/// use finance_rs::decimal::re_total_roi; use rust_decimal::Decimal; assert_eq!(re_total_roi(Decimal::from(50), Decimal::from(75), Decimal::from(100)), Some(Decimal::new(25, 2)));
/// ```
#[must_use]
pub fn re_total_roi(
    distributions: Decimal,
    terminal_equity: Decimal,
    contributed_capital: Decimal,
) -> Option<Decimal> {
    (contributed_capital > Decimal::ZERO)
        .then(|| (distributions + terminal_equity - contributed_capital) / contributed_capital)
}

/// Annualized compound return over elapsed days using a 365-day year.
///
/// ```rust
/// use finance_rs::decimal::re_annualized_roi; use rust_decimal::Decimal; assert_eq!(re_annualized_roi(Decimal::new(1, 1), 365), Some(Decimal::new(1, 1)));
/// ```
#[must_use]
pub fn re_annualized_roi(total_return: Decimal, elapsed_days: i64) -> Option<Decimal> {
    if elapsed_days <= 0 || total_return <= -Decimal::ONE {
        return None;
    }
    let years = Decimal::from(elapsed_days) / Decimal::from(365);
    Some((Decimal::ONE + total_return).powd(Decimal::ONE / years) - Decimal::ONE)
}

fn mul(a: Decimal, b: Decimal) -> Result<Decimal> {
    a.checked_mul(b).ok_or(OVERFLOW)
}

fn add(a: Decimal, b: Decimal) -> Result<Decimal> {
    a.checked_add(b).ok_or(OVERFLOW)
}

fn sub(a: Decimal, b: Decimal) -> Result<Decimal> {
    a.checked_sub(b).ok_or(OVERFLOW)
}

fn div(what: &'static str, a: Decimal, b: Decimal) -> Result<Decimal> {
    if b.is_zero() {
        return Err(FinanceError::Undefined { what });
    }
    a.checked_div(b).ok_or(OVERFLOW)
}

fn growth(rate: Decimal, nper: u32) -> Result<Decimal> {
    add(Decimal::ONE, rate)?
        .checked_powi(i64::from(nper))
        .ok_or(OVERFLOW)
}

fn when_factor(when: When) -> Decimal {
    match when {
        When::End => Decimal::ZERO,
        When::Begin => Decimal::ONE,
    }
}

/// `(1+r·w)·((1+r)^n − 1)/r`, or `n` at `r = 0`.
fn annuity_factor(rate: Decimal, nper: u32, when: When) -> Result<Decimal> {
    if rate.is_zero() {
        return Ok(Decimal::from(nper));
    }
    let lead = add(Decimal::ONE, mul(rate, when_factor(when))?)?;
    div(
        "rate",
        mul(lead, sub(growth(rate, nper)?, Decimal::ONE)?)?,
        rate,
    )
}

/// Future value: `fv = −pv·(1+r)^n − pmt·(1+r·w)·((1+r)^n − 1)/r`.
///
/// ```rust
/// use finance_rs::{decimal::fv, tvm::When}; use rust_decimal::Decimal; let v = fv(Decimal::new(1, 1), 2, Decimal::ZERO, Decimal::from(-100), When::End)?; assert_eq!(v, Decimal::from(121));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::Overflow`] when an intermediate exceeds `Decimal`'s range.
pub fn fv(rate: Decimal, nper: u32, pmt: Decimal, pv: Decimal, when: When) -> Result<Decimal> {
    let grown = mul(-pv, growth(rate, nper)?)?;
    sub(grown, mul(pmt, annuity_factor(rate, nper, when)?)?)
}

/// Present value: `pv = −(fv + pmt·factor) / (1+r)^n`.
///
/// ```rust
/// use finance_rs::{decimal::pv, tvm::When}; use rust_decimal::Decimal; let v = pv(Decimal::new(1, 1), 2, Decimal::ZERO, Decimal::from(121), When::End)?; assert_eq!(v, Decimal::from(-100));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::Undefined`] when `(1+r)^n = 0`; [`FinanceError::Overflow`].
pub fn pv(rate: Decimal, nper: u32, pmt: Decimal, fv: Decimal, when: When) -> Result<Decimal> {
    let num = -add(fv, mul(pmt, annuity_factor(rate, nper, when)?)?)?;
    div("(1+rate)^nper", num, growth(rate, nper)?)
}

/// Payment per period: `pmt = −(fv + pv·(1+r)^n) / factor`.
///
/// ```rust
/// use finance_rs::{decimal::pmt, tvm::When}; use rust_decimal::Decimal; let p = pmt(Decimal::ZERO, 4, Decimal::from(1_000), Decimal::ZERO, When::End)?; assert_eq!(p, Decimal::from(-250));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::Undefined`] when `nper = 0`; [`FinanceError::Overflow`].
pub fn pmt(rate: Decimal, nper: u32, pv: Decimal, fv: Decimal, when: When) -> Result<Decimal> {
    let num = -add(fv, mul(pv, growth(rate, nper)?)?)?;
    div("annuity factor", num, annuity_factor(rate, nper, when)?)
}

/// Net present value with `values[0]` undiscounted: `Σ_t values[t]/(1+r)^t`.
///
/// ```rust
/// use finance_rs::decimal::npv; use rust_decimal::Decimal; let v = npv(Decimal::new(1, 1), &[Decimal::from(-100), Decimal::from(110)])?; assert_eq!(v, Decimal::ZERO);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for no values; [`FinanceError::Undefined`] at `rate = −1`.
pub fn npv(rate: Decimal, values: &[Decimal]) -> Result<Decimal> {
    if values.is_empty() {
        return Err(FinanceError::TooShort {
            what: "values",
            min: 1,
            got: 0,
        });
    }
    let step = add(Decimal::ONE, rate)?;
    let mut factor = Decimal::ONE;
    let mut sum = Decimal::ZERO;
    for v in values {
        sum = add(sum, div("1+rate", *v, factor)?)?;
        factor = mul(factor, step)?;
    }
    Ok(sum)
}

/// One period of a rounded level-payment schedule.
///
/// ```rust
/// use finance_rs::decimal::DecimalRow; use rust_decimal::Decimal; let row = DecimalRow { period: 1, payment: Decimal::from(10), interest: Decimal::from(2), principal: Decimal::from(8), balance: Decimal::from(92) }; assert_eq!(row.payment, row.interest + row.principal);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct DecimalRow {
    /// 1-based period number.
    pub period: u32,
    /// Total paid this period (interest + principal).
    pub payment: Decimal,
    /// Interest accrued this period, rounded.
    pub interest: Decimal,
    /// Principal retired this period.
    pub principal: Decimal,
    /// Balance after this period's payment.
    pub balance: Decimal,
}

/// Level-payment amortization in exact decimals, rounded to `dp` places.
///
/// Payment = `round(−pmt(r, n, principal))`; each period's interest is
/// `round(balance·r)`; the last payment is whatever clears the balance, so
/// `Σ principal = principal` exactly.
///
/// ```rust
/// use finance_rs::decimal::level_schedule; use rust_decimal::Decimal; let rows = level_schedule(Decimal::from(1_000), Decimal::new(1, 2), 12, 2)?; let paid: Decimal = rows.iter().map(|r| r.principal).sum(); assert_eq!(paid, Decimal::from(1_000)); assert_eq!(rows[0].payment, Decimal::new(8_885, 2));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for a non-positive principal, a negative
/// rate or zero periods; [`FinanceError::Overflow`].
pub fn level_schedule(
    principal: Decimal,
    rate: Decimal,
    nper: u32,
    dp: u32,
) -> Result<Vec<DecimalRow>> {
    let invalid = |what, reason| FinanceError::InvalidInput { what, reason };
    if principal <= Decimal::ZERO {
        return Err(invalid("principal", "must be positive"));
    }
    if rate.is_sign_negative() && !rate.is_zero() {
        return Err(invalid("rate", "must not be negative"));
    }
    if nper == 0 {
        return Err(invalid("nper", "must be at least 1"));
    }
    let level = (-pmt(rate, nper, principal, Decimal::ZERO, When::End)?).round_dp(dp);
    let mut balance = principal;
    let mut rows = Vec::with_capacity(usize::try_from(nper).unwrap_or(0));
    for period in 1..=nper {
        let interest = mul(balance, rate)?.round_dp(dp);
        let due = add(balance, interest)?;
        let payment = if period == nper || level >= due {
            due
        } else {
            level
        };
        let principal_paid = sub(payment, interest)?;
        balance = sub(balance, principal_paid)?;
        rows.push(DecimalRow {
            period,
            payment,
            interest,
            principal: principal_paid,
            balance,
        });
        if balance.is_zero() {
            break;
        }
    }
    Ok(rows)
}

/// Decimal amortization schedule for a caller-supplied payment. Interest is
/// rounded to cents each period and the final period clears the balance.
///
/// ```rust
/// use finance_rs::decimal::amortization_schedule; use rust_decimal::Decimal; let rows = amortization_schedule(Decimal::ZERO, 2, Decimal::from(100), Decimal::from(50))?; assert_eq!(rows.last().map(|r| r.balance), Some(Decimal::ZERO));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn amortization_schedule(
    rate: Decimal,
    periods: u32,
    principal: Decimal,
    payment: Decimal,
) -> Result<Vec<DecimalRow>> {
    let mut balance = principal;
    let mut rows = Vec::with_capacity(usize::try_from(periods).unwrap_or(0));
    for period in 1..=periods {
        let interest = mul(balance, rate)?
            .round_dp_with_strategy(2, rust_decimal::RoundingStrategy::MidpointAwayFromZero);
        let mut principal_payment = (payment.abs() - interest).min(balance);
        if period == periods {
            principal_payment = balance;
        }
        balance = sub(balance, principal_payment)?;
        rows.push(DecimalRow {
            period,
            payment,
            interest,
            principal: principal_payment,
            balance,
        });
    }
    Ok(rows)
}

#[cfg(test)]
#[path = "decimal_tests.rs"]
mod tests;
