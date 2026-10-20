//! Convertible-note accrual and conversion under explicit day-count and round terms.
//!
//! ```
//! use finance_rs::{daycount::DayCount, vc::note::{conversion, Terms}};
//! let converted = conversion(Terms { principal: 1_000.0, annual_rate: 0.05, days: 365,
//!     day_count: DayCount::Act365F, cap: 2.0, company_capitalization: 0.8,
//!     discount: 0.2, round_pps: 1.0 })?;
//! assert!(converted.shares > 0.0);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

use super::{finite, fraction, output, positive, FinanceError, Result};
use crate::daycount::DayCount;
use crate::Date;
/// Convertible note conversion, returning accrued principal, shares and price.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Conversion {
    /// Accrued principal for the conversion result.
    pub accrued_principal: f64,
    /// Shares for the conversion result.
    pub shares: f64,
    /// Price for the conversion result.
    pub price: f64,
}
/// Convertible-note terms needed to price conversion into a later financing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Terms {
    /// Original principal.
    pub principal: f64,
    /// Annual simple-interest rate as a decimal.
    pub annual_rate: f64,
    /// Elapsed calendar days.
    pub days: i64,
    /// Day-count convention applied to the accrued period.
    pub day_count: DayCount,
    /// Valuation cap.
    pub cap: f64,
    /// Company shares used to convert the valuation cap into a share price.
    pub company_capitalization: f64,
    /// Round-price discount as a fraction.
    pub discount: f64,
    /// Price per share in the financing round.
    pub round_pps: f64,
}
/// Accrue simple interest using the supplied day count and convert at min(cap price, discount price).
///
/// # Errors
/// Returns an error for invalid terms, an unsupported date range, or overflow.
pub fn conversion(terms: Terms) -> Result<Conversion> {
    let Terms {
        principal,
        annual_rate,
        days,
        day_count,
        cap,
        company_capitalization,
        discount,
        round_pps,
    } = terms;
    positive("principal", principal)?;
    finite("annual_rate", annual_rate)?;
    if annual_rate < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "annual_rate",
            reason: "must be non-negative",
        });
    }
    if days < 0 {
        return Err(FinanceError::InvalidInput {
            what: "days",
            reason: "must be non-negative",
        });
    }
    positive("cap", cap)?;
    positive("company_capitalization", company_capitalization)?;
    fraction("discount", discount)?;
    positive("round_pps", round_pps)?;
    let start = Date::new(2000, 1, 1)?;
    let end_ordinal = start
        .ordinal()
        .checked_add(days)
        .ok_or(FinanceError::Overflow { what: "note dates" })?;
    let end = Date::from_ordinal(end_ordinal)?;
    let accrued = principal * (1.0 + annual_rate * day_count.year_fraction(start, end));
    let price = (cap / company_capitalization).min(round_pps * (1.0 - discount));
    let shares = accrued / positive("conversion price", price)?;
    Ok(Conversion {
        accrued_principal: output("accrued_principal", accrued)?,
        shares: output("shares", shares)?,
        price: output("conversion price", price)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversion_rejects_invalid_terms_and_caps_price() {
        let valid = Terms {
            principal: 1_000.0,
            annual_rate: 0.05,
            days: 365,
            day_count: DayCount::Act365F,
            cap: 2.0,
            company_capitalization: 0.2,
            discount: 0.2,
            round_pps: 1.0,
        };
        assert!(conversion(Terms {
            principal: 0.0,
            ..valid
        })
        .is_err());
        assert!(conversion(Terms {
            annual_rate: f64::NAN,
            ..valid
        })
        .is_err());
        assert!(conversion(Terms {
            annual_rate: -0.01,
            ..valid
        })
        .is_err());
        assert!(conversion(Terms { days: -1, ..valid }).is_err());
        assert!(conversion(Terms { cap: 0.0, ..valid }).is_err());
        assert!(conversion(Terms {
            company_capitalization: 0.0,
            ..valid
        })
        .is_err());
        assert!(conversion(Terms {
            discount: 1.1,
            ..valid
        })
        .is_err());
        assert!(conversion(Terms {
            round_pps: 0.0,
            ..valid
        })
        .is_err());
        let result = conversion(valid).unwrap();
        assert_eq!(result.price, 0.8);
        assert_eq!(result.accrued_principal, 1_050.0);
    }
}
