//! Excel dated fixed-rate bond calculations, including duration and odd-coupon prices.
//!
//! ```
//! use finance_rs::{fi::excel::FixedRateBond, Date};
//! let settlement = Date::new(2024, 3, 1)?;
//! let maturity = Date::new(2026, 3, 1)?;
//! assert!(FixedRateBond::duration(settlement, maturity, 0.05, 0.04, 2, 1)? > 0.0);
//! let price = FixedRateBond::odd_last_price(
//!     settlement,
//!     maturity,
//!     Date::new(2023, 12, 1)?,
//!     0.05,
//!     0.04,
//!     100.0,
//!     2,
//!     1,
//! )?;
//! assert!(price > 0.0);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

use crate::{
    daycount::DayCount,
    error::{finite, output, FinanceError, Result},
    Date,
};

/// Namespace for Excel's dated fixed-rate bond calculations.
/// ```rust
/// let bond = finance_rs::fi::excel::FixedRateBond; assert!(matches!(bond, finance_rs::fi::excel::FixedRateBond));
/// ```
pub struct FixedRateBond;

fn basis_day_count(basis: u8) -> Result<DayCount> {
    match basis {
        0 => Ok(DayCount::Thirty360Us),
        1 => Ok(DayCount::ActActIsda),
        2 => Ok(DayCount::Act360),
        3 => Ok(DayCount::Act365F),
        4 => Ok(DayCount::Thirty360Eu),
        _ => Err(FinanceError::InvalidInput {
            what: "basis",
            reason: "must be an Excel basis code from 0 through 4",
        }),
    }
}

#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
fn day_count_as_f64(day_count: DayCount, start: Date, end: Date) -> f64 {
    // Date::new bounds years to ±1,000,000, so even the full supported span
    // is below i32::MAX days. The conversion cannot overflow for valid dates.
    day_count.days(start, end) as f64
}

#[allow(clippy::cast_possible_wrap)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
fn coupon_dates(
    settlement: Date,
    maturity: Date,
    frequency: u32,
) -> Result<(Date, Date, Vec<Date>)> {
    if maturity <= settlement || !matches!(frequency, 1 | 2 | 4) {
        return Err(FinanceError::InvalidInput {
            what: "maturity/frequency",
            reason: "maturity must follow settlement and frequency must be 1, 2, or 4",
        });
    }
    // `frequency` is restricted to 1, 2, or 4 above, so the month interval
    // always fits in i32. Keep this conversion infallible instead of adding
    // an error path that can never be reached.
    let months = (12 / frequency) as i32;
    let mut dates = vec![maturity];
    while dates.last().is_some_and(|date| *date > settlement) {
        let next = dates
            .last()
            .copied()
            .unwrap_or(maturity)
            .add_months(-months)?;
        if next <= settlement {
            break;
        }
        dates.push(next);
        if dates.len() > 100_000 {
            return Err(FinanceError::InvalidInput {
                what: "maturity",
                reason: "coupon schedule is too long",
            });
        }
    }
    let next = *dates.last().unwrap_or(&maturity);
    let prev = next.add_months(-months)?;
    dates.reverse();
    // Dates are reversed for calculation below: next coupon through maturity.
    let mut pay_dates = vec![next];
    let mut cursor = next;
    while cursor < maturity {
        cursor = cursor.add_months(months)?;
        pay_dates.push(cursor.min(maturity));
    }
    Ok((prev, next, pay_dates))
}

impl FixedRateBond {
    /// Excel `DURATION(settlement, maturity, coupon, yld, frequency, basis)`.
    ///
    /// # Errors
    /// Returns an error for invalid dates, coupon/yield inputs, frequency, or basis.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::excel::FixedRateBond::duration), 0);
    /// ```
    #[allow(clippy::cast_possible_truncation)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
    pub fn duration(
        settlement: Date,
        maturity: Date,
        coupon: f64,
        yld: f64,
        frequency: u32,
        basis: u8,
    ) -> Result<f64> {
        finite("coupon", coupon)?;
        finite("yield", yld)?;
        if coupon < 0.0 || yld <= -1.0 || yld / f64::from(frequency) <= -1.0 {
            return Err(FinanceError::InvalidInput {
                what: "coupon/yield",
                reason:
                    "coupon must be non-negative and yield must produce positive discount factors",
            });
        }
        let dc = basis_day_count(basis)?;
        let (previous, next, payments) = coupon_dates(settlement, maturity, frequency)?;
        let period_days = day_count_as_f64(dc, previous, next);
        let remaining_days = day_count_as_f64(dc, settlement, next);
        // coupon_dates constructs previous <= settlement < next, and all
        // supported day-count conventions are positive over this interval.
        // The former defensive check was unreachable for valid Date values.
        let fraction = remaining_days / period_days;
        let per_yield = yld / f64::from(frequency);
        let coupon_cash = 100.0 * coupon / f64::from(frequency);
        let mut price = 0.0;
        let mut weighted = 0.0;
        for (index, date) in payments.iter().enumerate() {
            // coupon_dates rejects schedules longer than 100,000 periods.
            let period_index = index as u32;
            let exponent = f64::from(period_index) + fraction;
            let cash = coupon_cash + if *date == maturity { 100.0 } else { 0.0 };
            let pv = cash / (1.0 + per_yield).powf(exponent);
            price += pv;
            // Excel's duration is measured in coupon periods.  Basis affects
            // the settlement fraction within the current period, while every
            // later coupon advances by exactly one period.
            weighted += (f64::from(period_index) + fraction) / f64::from(frequency) * pv;
        }
        output("duration", weighted / price)
    }

    /// Excel `MDURATION(settlement, maturity, coupon, yld, frequency, basis)`.
    ///
    /// # Errors
    /// Returns an error for invalid inputs or a non-finite result.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::excel::FixedRateBond::modified_duration), 0);
    /// ```
    pub fn modified_duration(
        settlement: Date,
        maturity: Date,
        coupon: f64,
        yld: f64,
        frequency: u32,
        basis: u8,
    ) -> Result<f64> {
        let duration = Self::duration(settlement, maturity, coupon, yld, frequency, basis)?;
        output(
            "modified_duration",
            duration / (1.0 + yld / f64::from(frequency)),
        )
    }
}

#[cfg(test)]
#[path = "excel_tests.rs"]
mod tests;

#[allow(clippy::cast_possible_wrap)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
fn validate_odd_inputs(
    settlement: Date,
    maturity: Date,
    odd_date: Date,
    coupon: f64,
    yld: f64,
    frequency: u32,
    basis: u8,
) -> Result<(DayCount, i32)> {
    finite("coupon", coupon)?;
    finite("yield", yld)?;
    if coupon < 0.0 || yld < 0.0 || !matches!(frequency, 1 | 2 | 4) {
        return Err(FinanceError::InvalidInput {
            what: "coupon/yield/frequency",
            reason: "coupon and yield must be non-negative, and frequency must be 1, 2, or 4",
        });
    }
    if odd_date >= maturity || settlement >= maturity {
        return Err(FinanceError::InvalidInput {
            what: "odd_date/maturity",
            reason: "odd date and settlement must precede maturity",
        });
    }
    // The frequency check above admits only 1, 2, or 4.
    let months = 12 / frequency as i32;
    Ok((basis_day_count(basis)?, months))
}

fn solve_odd_yield<F>(price: f64, mut price_at: F) -> Result<f64>
where
    F: FnMut(f64) -> Result<f64>,
{
    finite("price", price)?;
    if price <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "price",
            reason: "must be positive",
        });
    }
    let mut low = 0.0;
    let mut high = 1.0;
    let mut low_value = price_at(low)? - price;
    let mut high_value = price_at(high)? - price;
    for _ in 0..64 {
        if low_value.signum() != high_value.signum() {
            break;
        }
        high = high * 2.0 + 1.0;
        high_value = price_at(high)? - price;
    }
    if low_value.signum() == high_value.signum() {
        return Err(FinanceError::NoConvergence {
            what: "odd bond yield",
            iterations: 64,
        });
    }
    for _ in 0..100 {
        let mid = (low + high) / 2.0;
        let value = price_at(mid)? - price;
        if value.abs() <= 1e-13 * price.max(1.0) {
            return output("yield", mid);
        }
        if value.signum() == low_value.signum() {
            low = mid;
            low_value = value;
        } else {
            high = mid;
        }
    }
    output("yield", (low + high) / 2.0)
}

impl FixedRateBond {
    /// Excel `ODDFPRICE(settlement, maturity, issue, first_coupon, rate, yld, redemption, frequency, basis)`.
    ///
    /// # Errors
    /// Returns an error for invalid dates or non-finite/out-of-domain inputs.
    #[allow(clippy::too_many_arguments)] // Mirrors Excel's documented positional signature.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::excel::FixedRateBond::odd_first_price), 0);
    /// ```
    pub fn odd_first_price(
        settlement: Date,
        maturity: Date,
        issue: Date,
        first_coupon: Date,
        coupon: f64,
        yld: f64,
        redemption: f64,
        frequency: u32,
        basis: u8,
    ) -> Result<f64> {
        let (dc, months) = validate_odd_inputs(
            settlement,
            maturity,
            first_coupon,
            coupon,
            yld,
            frequency,
            basis,
        )?;
        finite("redemption", redemption)?;
        if issue >= settlement || first_coupon <= settlement || first_coupon <= issue {
            return Err(FinanceError::InvalidInput {
                what: "issue/settlement/first_coupon",
                reason: "issue must precede settlement and first coupon must follow settlement",
            });
        }
        let previous = first_coupon.add_months(-months)?;
        let e = day_count_as_f64(dc, previous, first_coupon);
        let dsc = day_count_as_f64(dc, settlement, first_coupon);
        let dfc = day_count_as_f64(dc, issue, first_coupon);
        if e <= 0.0 || dsc <= 0.0 || dfc <= 0.0 {
            return Err(FinanceError::InvalidInput {
                what: "coupon dates",
                reason: "must define positive day-count periods",
            });
        }
        let period_yield = yld / f64::from(frequency);
        let fraction = dsc / e;
        let first_cash = 100.0 * coupon / f64::from(frequency) * dfc / e;
        let mut price = first_cash / (1.0 + period_yield).powf(fraction);
        let mut date = first_coupon;
        let mut index = 1_u32;
        while date < maturity {
            date = date.add_months(months)?.min(maturity);
            index += 1;
            let cash = 100.0 * coupon / f64::from(frequency)
                + if date == maturity { redemption } else { 0.0 };
            price += cash / (1.0 + period_yield).powf(f64::from(index - 1) + fraction);
            if index > 100_000 {
                return Err(FinanceError::InvalidInput {
                    what: "maturity",
                    reason: "coupon schedule is too long",
                });
            }
        }
        let accrued =
            100.0 * coupon / f64::from(frequency) * day_count_as_f64(dc, issue, settlement) / e;
        output("price", price - accrued)
    }

    /// Excel `ODDFYIELD(settlement, maturity, issue, first_coupon, rate, pr, redemption, frequency, basis)`.
    ///
    /// # Errors
    /// Returns an error for invalid dates, price, frequency, basis, or yield domain.
    #[allow(clippy::too_many_arguments)] // Mirrors Excel's documented positional signature.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::excel::FixedRateBond::odd_first_yield), 0);
    /// ```
    pub fn odd_first_yield(
        settlement: Date,
        maturity: Date,
        issue: Date,
        first_coupon: Date,
        coupon: f64,
        price: f64,
        redemption: f64,
        frequency: u32,
        basis: u8,
    ) -> Result<f64> {
        solve_odd_yield(price, |yield_rate| {
            Self::odd_first_price(
                settlement,
                maturity,
                issue,
                first_coupon,
                coupon,
                yield_rate,
                redemption,
                frequency,
                basis,
            )
        })
    }

    /// Excel `ODDLPRICE(settlement, maturity, last_interest, rate, yld, redemption, frequency, basis)`.
    ///
    /// # Errors
    /// Returns an error for invalid dates or non-finite/out-of-domain inputs.
    #[allow(clippy::too_many_arguments)] // Mirrors Excel's documented positional signature.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::excel::FixedRateBond::odd_last_price), 0);
    /// ```
    pub fn odd_last_price(
        settlement: Date,
        maturity: Date,
        last_interest: Date,
        coupon: f64,
        yld: f64,
        redemption: f64,
        frequency: u32,
        basis: u8,
    ) -> Result<f64> {
        let (dc, months) = validate_odd_inputs(
            settlement,
            maturity,
            last_interest,
            coupon,
            yld,
            frequency,
            basis,
        )?;
        finite("redemption", redemption)?;
        if settlement <= last_interest {
            return Err(FinanceError::InvalidInput {
                what: "settlement/last_interest",
                reason: "settlement must follow the last interest date",
            });
        }
        let regular_next = last_interest.add_months(months)?;
        let regular_days = day_count_as_f64(dc, last_interest, regular_next);
        // add_months(months) strictly advances for the validated 1/2/4
        // coupon frequencies, so the regular coupon day count is positive.
        let period_yield = yld / f64::from(frequency);
        let fraction = day_count_as_f64(dc, settlement, maturity) / regular_days;
        let stub_fraction = day_count_as_f64(dc, last_interest, maturity) / regular_days;
        let accrued_fraction = day_count_as_f64(dc, last_interest, settlement) / regular_days;
        let dirty = (redemption + 100.0 * coupon / f64::from(frequency) * stub_fraction)
            / (1.0 + period_yield).powf(fraction);
        let accrued = 100.0 * coupon / f64::from(frequency) * accrued_fraction;
        output("price", dirty - accrued)
    }

    /// Excel `ODDLYIELD(settlement, maturity, last_interest, rate, pr, redemption, frequency, basis)`.
    ///
    /// # Errors
    /// Returns an error for invalid dates, price, frequency, basis, or yield domain.
    #[allow(clippy::too_many_arguments)] // Mirrors Excel's documented positional signature.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::excel::FixedRateBond::odd_last_yield), 0);
    /// ```
    pub fn odd_last_yield(
        settlement: Date,
        maturity: Date,
        last_interest: Date,
        coupon: f64,
        price: f64,
        redemption: f64,
        frequency: u32,
        basis: u8,
    ) -> Result<f64> {
        solve_odd_yield(price, |yield_rate| {
            Self::odd_last_price(
                settlement,
                maturity,
                last_interest,
                coupon,
                yield_rate,
                redemption,
                frequency,
                basis,
            )
        })
    }
}
