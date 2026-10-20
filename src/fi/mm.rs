//! Excel money-market securities using settlement and maturity dates.

use crate::{
    daycount::DayCount,
    error::{finite, output, FinanceError, Result},
    Date,
};

#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
fn year_fraction(start: Date, end: Date, basis: u8) -> Result<f64> {
    if basis == 1 {
        if end.year() == start.year() + 1
            && end.month() == start.month()
            && end.day() == start.day()
        {
            return Ok(1.0);
        }
        let year_days = if crate::is_leap_year(start.year()) {
            366.0
        } else {
            365.0
        };
        return Ok(end.days_since(start) as f64 / year_days);
    }
    let dc = match basis {
        0 => DayCount::Thirty360Us,
        2 => DayCount::Act360,
        3 => DayCount::Act365F,
        4 => DayCount::Thirty360Eu,
        _ => {
            return Err(FinanceError::InvalidInput {
                what: "basis",
                reason: "must be an Excel basis code from 0 through 4",
            })
        }
    };
    Ok(dc.year_fraction(start, end))
}

fn inputs(settlement: Date, maturity: Date, basis: u8) -> Result<f64> {
    if maturity <= settlement {
        return Err(FinanceError::InvalidInput {
            what: "maturity",
            reason: "must be after settlement",
        });
    }
    year_fraction(settlement, maturity, basis)
}

/// Discount rate for a discounted security (Excel `DISC`).
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::mm::discount_rate), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn discount_rate(
    settlement: Date,
    maturity: Date,
    price: f64,
    redemption: f64,
    basis: u8,
) -> Result<f64> {
    finite("price", price)?;
    finite("redemption", redemption)?;
    let yf = inputs(settlement, maturity, basis)?;
    if price <= 0.0 || redemption <= 0.0 || price > redemption {
        return Err(FinanceError::InvalidInput {
            what: "price/redemption",
            reason: "must be positive and price must not exceed redemption",
        });
    }
    output("discount_rate", ((redemption - price) / redemption) / yf)
}

/// Annual interest rate for a fully invested maturity security (Excel `INTRATE`).
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::mm::interest_rate), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn interest_rate(
    settlement: Date,
    maturity: Date,
    investment: f64,
    redemption: f64,
    basis: u8,
) -> Result<f64> {
    finite("investment", investment)?;
    finite("redemption", redemption)?;
    let yf = inputs(settlement, maturity, basis)?;
    if investment <= 0.0 || redemption <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "investment/redemption",
            reason: "must be positive",
        });
    }
    output(
        "interest_rate",
        ((redemption - investment) / investment) / yf,
    )
}

/// Amount received at maturity for a discounted security (Excel `RECEIVED`).
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::mm::amount_received), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn amount_received(
    settlement: Date,
    maturity: Date,
    investment: f64,
    discount: f64,
    basis: u8,
) -> Result<f64> {
    finite("investment", investment)?;
    finite("discount", discount)?;
    let yf = inputs(settlement, maturity, basis)?;
    if investment <= 0.0 || discount < 0.0 || discount * yf >= 1.0 {
        return Err(FinanceError::InvalidInput {
            what: "investment/discount",
            reason: "investment must be positive and discount must produce positive proceeds",
        });
    }
    output("amount_received", investment / (1.0 - discount * yf))
}

/// Price per 100 face value of a discounted security (Excel `PRICEDISC`).
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::mm::price_discount_instrument), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn price_discount_instrument(
    settlement: Date,
    maturity: Date,
    discount: f64,
    redemption: f64,
    basis: u8,
) -> Result<f64> {
    finite("discount", discount)?;
    finite("redemption", redemption)?;
    let yf = inputs(settlement, maturity, basis)?;
    if discount < 0.0 || redemption <= 0.0 || discount * yf >= 1.0 {
        return Err(FinanceError::InvalidInput {
            what: "discount/redemption",
            reason: "discount must produce a positive price and redemption must be positive",
        });
    }
    output(
        "price_discount_instrument",
        redemption * (1.0 - discount * yf),
    )
}

/// Price per 100 face value of a security paying interest at maturity (Excel `PRICEMAT`).
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::mm::price_maturity_instrument), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn price_maturity_instrument(
    settlement: Date,
    maturity: Date,
    issue: Date,
    rate: f64,
    yld: f64,
    basis: u8,
) -> Result<f64> {
    finite("rate", rate)?;
    finite("yield", yld)?;
    let _ = inputs(settlement, maturity, basis)?;
    let issue_to_maturity = year_fraction(issue, maturity, basis)?;
    let settlement_to_maturity = year_fraction(settlement, maturity, basis)?;
    let accrued_fraction = year_fraction(issue, settlement, basis)?;
    if issue > settlement || rate < 0.0 || yld <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "issue/rate/yield",
            reason:
                "issue must not follow settlement, rate must be non-negative, yield must exceed -1",
        });
    }
    let interest = 100.0 * rate * issue_to_maturity;
    let denominator = 1.0 + yld * settlement_to_maturity;
    if denominator <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "yield",
            reason: "produces a non-positive discount factor",
        });
    }
    output(
        "price_maturity_instrument",
        (100.0 + interest) / denominator - 100.0 * rate * accrued_fraction,
    )
}

/// Yield on a discounted security (Excel `YIELDDISC`).
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::mm::yield_discount_instrument), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn yield_discount_instrument(
    settlement: Date,
    maturity: Date,
    price: f64,
    redemption: f64,
    basis: u8,
) -> Result<f64> {
    finite("price", price)?;
    finite("redemption", redemption)?;
    let yf = inputs(settlement, maturity, basis)?;
    if price <= 0.0 || redemption <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "price/redemption",
            reason: "must be positive",
        });
    }
    output(
        "yield_discount_instrument",
        ((redemption - price) / price) / yf,
    )
}

/// Yield on a security paying interest at maturity (Excel `YIELDMAT`).
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::mm::yield_maturity_instrument), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn yield_maturity_instrument(
    settlement: Date,
    maturity: Date,
    issue: Date,
    rate: f64,
    price: f64,
    basis: u8,
) -> Result<f64> {
    finite("rate", rate)?;
    finite("price", price)?;
    let sm = year_fraction(settlement, maturity, basis)?;
    let im = year_fraction(issue, maturity, basis)?;
    if issue > settlement || rate < 0.0 || price <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "issue/rate/price",
            reason: "issue must not follow settlement; rate and price must be positive",
        });
    }
    let interest = 100.0 * rate * im;
    let accrued_fraction = year_fraction(issue, settlement, basis)?;
    let dirty_price = price + 100.0 * rate * accrued_fraction;
    output(
        "yield_maturity_instrument",
        ((100.0 + interest) / dirty_price - 1.0) / sm,
    )
}

/// Accrued interest for a security paying interest only at maturity (Excel `ACCRINTM`).
///
/// # Errors
/// Returns an error when settlement precedes issue, the rate is negative, par is non-positive,
/// or the basis is outside Excel's 0–4 range.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::mm::accrued_at_maturity), 0);
/// ```
pub fn accrued_at_maturity(
    issue: Date,
    settlement: Date,
    rate: f64,
    par: f64,
    basis: u8,
) -> Result<f64> {
    finite("rate", rate)?;
    finite("par", par)?;
    if settlement < issue || rate < 0.0 || par <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "dates/rate/par",
            reason: "require issue <= settlement, non-negative rate, and positive par",
        });
    }
    output(
        "accrued_at_maturity",
        par * rate * year_fraction(issue, settlement, basis)?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> Date {
        Date::new(y, m, day).unwrap()
    }

    #[test]
    fn money_market_formulas_and_excel_bases() {
        let settlement = d(2024, 1, 1);
        let maturity = d(2025, 1, 1);
        assert_eq!(year_fraction(settlement, maturity, 1).unwrap(), 1.0);
        assert!(year_fraction(d(2024, 1, 1), d(2024, 7, 1), 1).unwrap() > 0.0);
        for basis in [0, 2, 3, 4] {
            assert!(year_fraction(settlement, maturity, basis).unwrap() > 0.0);
        }
        assert!(year_fraction(settlement, maturity, 5).is_err());
        assert!(discount_rate(settlement, maturity, 95.0, 100.0, 3).unwrap() > 0.0);
        assert!(interest_rate(settlement, maturity, 95.0, 100.0, 3).unwrap() > 0.0);
        assert!(amount_received(settlement, maturity, 95.0, 0.05, 3).unwrap() > 95.0);
        assert!(price_discount_instrument(settlement, maturity, 0.05, 100.0, 3).unwrap() < 100.0);
        assert!(
            price_maturity_instrument(settlement, maturity, d(2023, 1, 1), 0.05, 0.04, 3).unwrap()
                > 0.0
        );
        assert!(yield_discount_instrument(settlement, maturity, 95.0, 100.0, 3).unwrap() > 0.0);
        assert!(
            yield_maturity_instrument(settlement, maturity, d(2023, 1, 1), 0.05, 100.0, 3)
                .unwrap()
                .is_finite()
        );
        assert!(accrued_at_maturity(d(2023, 1, 1), settlement, 0.05, 100.0, 3).unwrap() > 0.0);
    }

    #[test]
    fn money_market_rejects_invalid_dates_values_and_bases() {
        let s = d(2024, 1, 1);
        let m = d(2025, 1, 1);
        assert!(inputs(m, s, 3).is_err());
        assert!(discount_rate(s, m, 101.0, 100.0, 3).is_err());
        assert!(interest_rate(s, m, 0.0, 100.0, 3).is_err());
        assert!(amount_received(s, m, 100.0, 1.0, 3).is_err());
        assert!(price_discount_instrument(s, m, 1.0, 100.0, 3).is_err());
        assert!(price_maturity_instrument(s, m, d(2025, 1, 2), 0.05, 0.0, 3).is_err());
        assert!(price_maturity_instrument(s, m, d(2023, 1, 1), 0.05, -2.0, 3).is_err());
        assert!(price_maturity_instrument(s, d(2028, 1, 1), d(2023, 1, 1), 0.05, -0.5, 3).is_err());
        assert!(yield_discount_instrument(s, m, 0.0, 100.0, 3).is_err());
        assert!(yield_maturity_instrument(s, m, d(2025, 1, 2), 0.05, 100.0, 3).is_err());
        assert!(accrued_at_maturity(s, d(2023, 1, 1), 0.05, 100.0, 3).is_err());
        assert!(accrued_at_maturity(s, m, -0.01, 100.0, 3).is_err());
        assert!(accrued_at_maturity(s, m, 0.05, 100.0, 5).is_err());
    }
}
