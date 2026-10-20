//! Dividend-discount, residual-income, and implied-cost-of-equity models.
//!
//! ```
//! use finance_rs::equity::ddm_gordon;
//! assert!((ddm_gordon(2.0, 0.10, 0.04)? - 33.333_333_333_333_336).abs() < 1e-12);
//! # Ok::<(), finance_rs::FinanceError>(())

use crate::{
    error::{all_finite, finite, output, FinanceError},
    tvm, Result,
};

/// Gordon dividend discount value: `D1 / (r - g)`.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::ddm_gordon), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn ddm_gordon(d1: f64, required_return: f64, growth: f64) -> Result<f64> {
    finite("d1", d1)?;
    finite("required_return", required_return)?;
    finite("growth", growth)?;
    if required_return <= growth {
        return Err(FinanceError::InvalidInput {
            what: "required_return",
            reason: "must exceed growth",
        });
    }
    output("ddm_gordon", d1 / (required_return - growth))
}

/// One high-growth period for a two-stage dividend discount model.
#[derive(Debug, Clone, Copy, PartialEq)]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::equity::DdmStage>() > 0);
/// ```
pub struct DdmStage {
    /// Dividend for the ddm stage result.
    pub dividend: f64,
    /// Growth for the ddm stage result.
    pub growth: f64,
    /// Periods for the ddm stage result.
    pub periods: u32,
}

/// Two-stage DDM: explicit growing dividends followed by a Gordon terminal value.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::ddm_two_stage), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn ddm_two_stage(
    d0: f64,
    high_growth: f64,
    high_periods: u32,
    stable_growth: f64,
    required_return: f64,
) -> Result<f64> {
    all_finite(
        "ddm inputs",
        &[d0, high_growth, stable_growth, required_return],
    )?;
    if d0 < 0.0
        || high_growth <= -1.0
        || stable_growth <= -1.0
        || required_return <= stable_growth
        || required_return <= -1.0
    {
        return Err(FinanceError::InvalidInput { what: "ddm inputs", reason: "dividend must be nonnegative, growth greater than -1 and return above stable growth" });
    }
    let mut value = 0.0;
    let mut dividend = d0;
    for year in 1..=high_periods {
        dividend *= 1.0 + high_growth;
        value += dividend / (1.0 + required_return).powf(f64::from(year));
    }
    let terminal_dividend = dividend * (1.0 + stable_growth);
    value += ddm_gordon(terminal_dividend, required_return, stable_growth)?
        / (1.0 + required_return).powf(f64::from(high_periods));
    output("ddm_two_stage", value)
}

/// H-model value: perpetual stable growth plus a linearly declining excess-growth premium.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::ddm_h_model), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn ddm_h_model(
    d0: f64,
    short_growth: f64,
    long_growth: f64,
    half_life: f64,
    required_return: f64,
) -> Result<f64> {
    all_finite(
        "h model inputs",
        &[d0, short_growth, long_growth, half_life, required_return],
    )?;
    if d0 < 0.0
        || short_growth <= -1.0
        || long_growth <= -1.0
        || half_life < 0.0
        || required_return <= long_growth
        || required_return <= -1.0
    {
        return Err(FinanceError::InvalidInput {
            what: "h model inputs",
            reason: "invalid dividend, growth, half-life, or return",
        });
    }
    let spread = required_return - long_growth;
    output(
        "ddm_h_model",
        d0 * (1.0 + long_growth) / spread + d0 * half_life * (short_growth - long_growth) / spread,
    )
}

/// Residual-income value using a finite ROE path and a growing terminal residual.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::residual_income), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn residual_income(
    book_value: f64,
    roe_path: &[f64],
    required_return: f64,
    terminal_growth: f64,
) -> Result<f64> {
    finite("book_value", book_value)?;
    finite("required_return", required_return)?;
    finite("terminal_growth", terminal_growth)?;
    all_finite("roe_path", roe_path)?;
    if roe_path.is_empty() {
        return Err(FinanceError::TooShort {
            what: "roe_path",
            min: 1,
            got: 0,
        });
    }
    if book_value < 0.0
        || required_return <= terminal_growth
        || required_return <= -1.0
        || terminal_growth <= -1.0
    {
        return Err(FinanceError::InvalidInput {
            what: "residual income inputs",
            reason: "book value nonnegative and return greater than growth and -1 required",
        });
    }
    let mut bv = book_value;
    let mut pv = book_value;
    let mut t = 1.0;
    for roe in roe_path {
        let ri = (roe - required_return) * bv;
        pv += ri / (1.0 + required_return).powf(t);
        bv *= 1.0 + roe;
        t += 1.0;
    }
    let final_ri = (roe_path[roe_path.len() - 1] - required_return) * bv;
    pv += (final_ri * (1.0 + terminal_growth) / (required_return - terminal_growth))
        / (1.0 + required_return).powf(t - 1.0);
    output("residual_income", pv)
}

/// Implied cost of equity under constant-growth DDM: `D1 / P + g`.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::implied_cost_of_equity), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn implied_cost_of_equity(price: f64, d1: f64, growth: f64) -> Result<f64> {
    all_finite("implied cost inputs", &[price, d1, growth])?;
    if price <= 0.0 || d1 < 0.0 || growth <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "implied cost inputs",
            reason: "price positive, dividend nonnegative and growth above -1 required",
        });
    }
    output("implied_cost_of_equity", d1 / price + growth)
}

/// Sustainable growth from return on equity and payout ratio: `ROE * (1 - payout)`.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::sustainable_growth), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn sustainable_growth(roe: f64, payout: f64) -> Result<f64> {
    all_finite("sustainable growth inputs", &[roe, payout])?;
    if !(0.0..=1.0).contains(&payout) {
        return Err(FinanceError::InvalidInput {
            what: "payout",
            reason: "must be between zero and one",
        });
    }
    output("sustainable_growth", roe * (1.0 - payout))
}

/// Holding-period shareholder return including cash dividends.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::total_shareholder_return), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn total_shareholder_return(p0: f64, p1: f64, dividends: &[f64]) -> Result<f64> {
    all_finite("dividends", dividends)?;
    all_finite("prices", &[p0, p1])?;
    if p0 <= 0.0 || p1 < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "prices",
            reason: "initial price positive and final price nonnegative required",
        });
    }
    output(
        "total_shareholder_return",
        (p1 - p0 + dividends.iter().sum::<f64>()) / p0,
    )
}

/// Dividend IRR for an initial share purchase, dated annual dividends and terminal value.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::dividend_discount_irr), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn dividend_discount_irr(price: f64, dividends: &[f64], terminal: f64) -> Result<f64> {
    all_finite("dividends", dividends)?;
    all_finite("terminal inputs", &[price, terminal])?;
    if price <= 0.0 || terminal < 0.0 || dividends.iter().any(|d| *d < 0.0) {
        return Err(FinanceError::InvalidInput {
            what: "dividend cash flows",
            reason: "price positive and distributions nonnegative required",
        });
    }
    let mut flows = Vec::with_capacity(dividends.len() + 1);
    flows.push(-price);
    flows.extend_from_slice(dividends);
    // The initial price is always inserted, so a final cash-flow slot exists.
    let last = flows.len() - 1;
    flows[last] += terminal;
    tvm::irr(&flows)
}

/// Earnings power value: after-tax normalized earnings capitalized at WACC.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::earnings_power_value), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn earnings_power_value(ebit: f64, tax_rate: f64, wacc: f64) -> Result<f64> {
    all_finite("earnings power inputs", &[ebit, tax_rate, wacc])?;
    if !(0.0..=1.0).contains(&tax_rate) || wacc <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "earnings power inputs",
            reason: "tax must be in [0, 1] and WACC positive",
        });
    }
    output("earnings_power_value", ebit * (1.0 - tax_rate) / wacc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn textbook_equity_models() -> Result<()> {
        assert!((ddm_gordon(2.0, 0.10, 0.04)? - 33.333_333_333_333_336).abs() < 1e-12);
        assert!((ddm_two_stage(1.0, 0.10, 2, 0.04, 0.10)? - 19.333_333_333_333_332).abs() < 1e-12);
        assert!((ddm_h_model(1.0, 0.10, 0.04, 5.0, 0.10)? - 22.333_333_333_333_332).abs() < 1e-12);
        assert!((implied_cost_of_equity(20.0, 1.0, 0.04)? - 0.09).abs() < 1e-12);
        assert!((sustainable_growth(0.15, 0.40)? - 0.09).abs() < 1e-12);
        assert!((total_shareholder_return(100.0, 110.0, &[2.0, 3.0])? - 0.15).abs() < 1e-12);
        assert!((dividend_discount_irr(100.0, &[0.0, 0.0], 121.0)? - 0.1).abs() < 1e-10);
        assert!((earnings_power_value(100.0, 0.25, 0.10)? - 750.0).abs() < 1e-12);
        assert!(ddm_gordon(1.0, 0.04, 0.04).is_err());
        Ok(())
    }

    proptest! {
        #[test]
        fn valuation_identities(d1 in 0.01_f64..1e4, g in -0.05_f64..0.05, spread in 0.01_f64..0.2,
                                roe in -0.5_f64..0.5, payout in 0.0_f64..1.0,
                                p0 in 1.0_f64..1e4, p1 in 0.0_f64..1e4, div in 0.0_f64..1e3) {
            let r = g + spread;
            let value = ddm_gordon(d1, r, g)?;
            prop_assert!((value * spread - d1).abs() < 1e-8 * d1.max(1.0));
            prop_assert!((implied_cost_of_equity(value, d1, g)? - r).abs() < 1e-9);
            prop_assert!((sustainable_growth(roe, payout)? - roe * (1.0 - payout)).abs() < 1e-12);
            prop_assert!((total_shareholder_return(p0, p1, &[div])? - (p1 - p0 + div) / p0).abs() < 1e-10);
            prop_assert!((ddm_h_model(d1, g + 0.03, g, 2.0, r)? - (d1*(1.0+g)/spread + d1*2.0*0.03/spread)).abs() < 1e-8);
        }
    }
}
