//! Post-money, pre-money, and MFN SAFE conversions.
//!
//! ```
//! use finance_rs::vc::safe::post_money;
//! let conversion = post_money(100_000.0, 5_000_000.0, 0.8, 2.0, 3_000_000.0)?;
//! assert!(conversion.ownership > 0.0);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

use super::{fraction, output, positive, FinanceError, Result};
/// Post-money SAFE conversion output (ownership fraction and conversion price).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Conversion {
    /// Ownership for the conversion result.
    pub ownership: f64,
    /// Price for the conversion result.
    pub price: f64,
}
/// YC post-money SAFE: ownership is investment / post-money cap; price is the lower cap/discount price.
///
/// # Errors
/// Returns an error for invalid terms or a SAFE investment greater than its cap.
pub fn post_money(
    investment: f64,
    cap: f64,
    discount: f64,
    round_pps: f64,
    company_capitalization: f64,
) -> Result<Conversion> {
    positive("investment", investment)?;
    positive("cap", cap)?;
    fraction("discount", discount)?;
    positive("round_pps", round_pps)?;
    positive("company_capitalization", company_capitalization)?;
    let cap_price = cap / company_capitalization;
    let price = cap_price.min(round_pps * (1.0 - discount));
    let ownership = investment / cap;
    if ownership > 1.0 {
        return Err(FinanceError::InvalidInput {
            what: "investment",
            reason: "cannot exceed the post-money cap",
        });
    }
    Ok(Conversion {
        ownership: output("SAFE ownership", ownership)?,
        price: output("SAFE conversion price", price)?,
    })
}
/// Legacy pre-money SAFE conversion based on pre-round capitalization.
///
/// # Errors
/// Returns an error for invalid terms or a non-finite result.
pub fn pre_money(
    investment: f64,
    cap: f64,
    discount: f64,
    round_pps: f64,
    pre_round_shares: f64,
) -> Result<Conversion> {
    positive("investment", investment)?;
    positive("cap", cap)?;
    fraction("discount", discount)?;
    positive("round_pps", round_pps)?;
    positive("pre_round_shares", pre_round_shares)?;
    let price = (cap / pre_round_shares).min(round_pps * (1.0 - discount));
    let shares = investment / price;
    Ok(Conversion {
        ownership: output("SAFE ownership", shares / (pre_round_shares + shares))?,
        price: output("SAFE conversion price", price)?,
    })
}
/// MFN-only SAFE: select the most favorable cap/discount among later instruments.
///
/// # Errors
/// Returns an error for invalid terms, no later instruments, or a non-finite result.
pub fn mfn(
    investment: f64,
    round_pps: f64,
    company_capitalization: f64,
    later_terms: &[(f64, f64)],
) -> Result<Conversion> {
    positive("investment", investment)?;
    positive("round_pps", round_pps)?;
    positive("company_capitalization", company_capitalization)?;
    if later_terms.is_empty() {
        return Err(FinanceError::TooShort {
            what: "later_terms",
            min: 1,
            got: 0,
        });
    }
    let mut best = f64::INFINITY;
    for (cap, discount) in later_terms {
        positive("cap", *cap)?;
        fraction("discount", *discount)?;
        let cap_price = cap / company_capitalization;
        best = best.min(cap_price.min(round_pps * (1.0 - discount)));
    }
    let price = positive("conversion price", best)?;
    let shares = investment / price;
    Ok(Conversion {
        ownership: output("SAFE ownership", shares / (company_capitalization + shares))?,
        price: output("SAFE conversion price", price)?,
    })
}
