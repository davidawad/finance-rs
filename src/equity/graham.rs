//! Graham-style defensive-investor criteria and valuation screens.
//!
//! ```
//! use finance_rs::equity::{graham_defensive_criteria, GrahamDefensiveInput};
//! let input = GrahamDefensiveInput {
//!     current_assets: 100.0, total_liabilities: 20.0, long_term_debt: 0.0,
//!     current_ratio: 2.0, eps: [1.0; 10], uninterrupted_dividends: [true; 20],
//!     eps_growth: 0.05, pe: 10.0, pb: 1.0, aaa_yield: 0.04,
//! };
//! assert!(graham_defensive_criteria(&input)?.positive_earnings);
//! # Ok::<(), finance_rs::FinanceError>(())

use crate::{error::all_finite, FinanceError, Result};

/// Inputs to Graham's defensive-investor tests from *The Intelligent Investor*, chapter 14.
#[derive(Debug, Clone, Copy, PartialEq)]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::equity::GrahamDefensiveInput>() > 0);
/// ```
pub struct GrahamDefensiveInput {
    /// Current assets for the graham defensive input result.
    pub current_assets: f64,
    /// Total liabilities for the graham defensive input result.
    pub total_liabilities: f64,
    /// Long term debt for the graham defensive input result.
    pub long_term_debt: f64,
    /// Current ratio for the graham defensive input result.
    pub current_ratio: f64,
    /// Eps for the graham defensive input result.
    pub eps: [f64; 10],
    /// Uninterrupted dividends for the graham defensive input result.
    pub uninterrupted_dividends: [bool; 20],
    /// Eps growth for the graham defensive input result.
    pub eps_growth: f64,
    /// Pe for the graham defensive input result.
    pub pe: f64,
    /// Pb for the graham defensive input result.
    pub pb: f64,
    /// Aaa yield for the graham defensive input result.
    pub aaa_yield: f64,
}

/// Results for Graham's ten defensive-investor criteria, in the original order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::equity::GrahamCriteria>() > 0);
/// ```
#[allow(clippy::struct_excessive_bools)] // These booleans are independent public financial inputs with distinct meanings.
pub struct GrahamCriteria {
    /// Net current assets for the graham criteria result.
    pub net_current_assets: bool,
    /// Debt coverage for the graham criteria result.
    pub debt_coverage: bool,
    /// Current ratio for the graham criteria result.
    pub current_ratio: bool,
    /// Positive earnings for the graham criteria result.
    pub positive_earnings: bool,
    /// Dividend record for the graham criteria result.
    pub dividend_record: bool,
    /// Ten year eps growth for the graham criteria result.
    pub ten_year_eps_growth: bool,
    /// Earnings stability for the graham criteria result.
    pub earnings_stability: bool,
    /// Pe aaa adjusted for the graham criteria result.
    pub pe_aaa_adjusted: bool,
    /// Price to book for the graham criteria result.
    pub price_to_book: bool,
    /// Pe x pb aaa adjusted for the graham criteria result.
    pub pe_x_pb_aaa_adjusted: bool,
}

impl GrahamCriteria {
    /// Return criteria in their stable catalog/metric order.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::GrahamCriteria::as_array), 0);
    /// ```
    #[must_use]
    pub fn as_array(self) -> [bool; 10] {
        [
            self.net_current_assets,
            self.debt_coverage,
            self.current_ratio,
            self.positive_earnings,
            self.dividend_record,
            self.ten_year_eps_growth,
            self.earnings_stability,
            self.pe_aaa_adjusted,
            self.price_to_book,
            self.pe_x_pb_aaa_adjusted,
        ]
    }
}

/// Evaluate Graham's defensive-investor tests, preserving the legacy criteria and order.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::graham_defensive_criteria), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn graham_defensive_criteria(input: &GrahamDefensiveInput) -> Result<GrahamCriteria> {
    all_finite(
        "graham inputs",
        &[
            input.current_assets,
            input.total_liabilities,
            input.long_term_debt,
            input.current_ratio,
            input.eps_growth,
            input.pe,
            input.pb,
            input.aaa_yield,
        ],
    )?;
    all_finite("eps", &input.eps)?;
    if input.aaa_yield <= 0.0 || input.pe < 0.0 || input.pb < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "graham inputs",
            reason: "AAA yield must be positive; valuation multiples nonnegative",
        });
    }

    let positive_earnings = input.eps.iter().all(|eps| *eps > 0.0);
    let yield_adjusted_pe = input.pe * (4.4 / input.aaa_yield);
    let net_current_assets = input.current_assets - input.total_liabilities;
    Ok(GrahamCriteria {
        net_current_assets: input.current_assets >= 2.0 * input.total_liabilities,
        debt_coverage: input.long_term_debt <= net_current_assets,
        current_ratio: input.current_ratio >= 2.0,
        positive_earnings,
        dividend_record: input.uninterrupted_dividends.iter().all(|paid| *paid),
        ten_year_eps_growth: input.eps[9] >= input.eps[0] * 1.333,
        earnings_stability: input.eps_growth >= 0.0,
        pe_aaa_adjusted: yield_adjusted_pe <= 15.0,
        price_to_book: input.pb <= 1.5,
        pe_x_pb_aaa_adjusted: yield_adjusted_pe * input.pb <= 22.5,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn example() -> GrahamDefensiveInput {
        GrahamDefensiveInput {
            current_assets: 100.0,
            total_liabilities: 30.0,
            long_term_debt: 10.0,
            current_ratio: 2.5,
            eps: [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.4],
            uninterrupted_dividends: [true; 20],
            eps_growth: 0.4,
            pe: 10.0,
            pb: 1.0,
            aaa_yield: 5.0,
        }
    }

    #[test]
    fn textbook_graham_defensive_screen() -> Result<()> {
        assert_eq!(
            graham_defensive_criteria(&example())?.as_array(),
            [true; 10]
        );
        let mut input = example();
        input.uninterrupted_dividends[19] = false;
        assert!(!graham_defensive_criteria(&input)?.dividend_record);
        let mut invalid = example();
        invalid.aaa_yield = 0.0;
        assert!(graham_defensive_criteria(&invalid).is_err());
        Ok(())
    }

    proptest! {
        #[test]
        fn criteria_match_their_definitions(
            assets in 0.0_f64..1e6,
            liabilities in 0.0_f64..1e6,
            debt in 0.0_f64..1e6,
            ratio in 0.0_f64..10.0,
            pe in 0.0_f64..100.0,
            pb in 0.0_f64..5.0,
            aaa in 0.01_f64..20.0,
        ) {
            let mut input = example();
            input.current_assets = assets;
            input.total_liabilities = liabilities;
            input.long_term_debt = debt;
            input.current_ratio = ratio;
            input.pe = pe;
            input.pb = pb;
            input.aaa_yield = aaa;
            let criteria = graham_defensive_criteria(&input)?;
            prop_assert_eq!(criteria.net_current_assets, assets >= 2.0 * liabilities);
            prop_assert_eq!(criteria.debt_coverage, debt <= assets - liabilities);
            prop_assert_eq!(criteria.current_ratio, ratio >= 2.0);
            prop_assert_eq!(criteria.pe_aaa_adjusted, pe * (4.4 / aaa) <= 15.0);
            prop_assert_eq!(criteria.price_to_book, pb <= 1.5);
            prop_assert_eq!(criteria.pe_x_pb_aaa_adjusted, pe * (4.4 / aaa) * pb <= 22.5);
            prop_assert_eq!(criteria.as_array(), [
                criteria.net_current_assets,
                criteria.debt_coverage,
                criteria.current_ratio,
                criteria.positive_earnings,
                criteria.dividend_record,
                criteria.ten_year_eps_growth,
                criteria.earnings_stability,
                criteria.pe_aaa_adjusted,
                criteria.price_to_book,
                criteria.pe_x_pb_aaa_adjusted,
            ]);
        }
    }
}
