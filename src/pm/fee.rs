//! Prediction-market transaction fee formulas.

use crate::error::{all_finite, output, FinanceError, Result};

/// Published default Kalshi taker fee coefficient (some markets differ).
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
pub const KALSHI_STANDARD_TAKER_RATE: f64 = 0.07;

fn validate(contracts: f64, price: f64, rate: f64) -> Result<()> {
    all_finite("fee inputs", &[contracts, price, rate])?;
    if contracts < 0.0 || !(0.0..=1.0).contains(&price) || rate < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "fee inputs",
            reason: "contracts and rate must be non-negative and price in [0, 1]",
        });
    }
    Ok(())
}

/// Kalshi general taker fee, rounded up to the next cent.
///
/// `ceil(rate * contracts * price * (1 - price) * 100) / 100`.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn kalshi(contracts: f64, price: f64, rate: f64) -> Result<f64> {
    validate(contracts, price, rate)?;
    let raw = rate * contracts * price * (1.0 - price);
    // Stabilize ordinary decimal inputs before ceiling the cent; otherwise
    // values such as 0.07 * 100 * 0.1 * 0.9 can land a few ulps above 0.63.
    let stable = (raw * 100_000_000.0).round() / 100_000_000.0;
    output("Kalshi fee", (stable * 100.0).ceil() / 100.0)
}

/// Polymarket taker fee under `rate * C * P * (1-P)`, rounded to five
/// decimal places. The caller supplies the market's current taker rate.
///
/// Makers are not charged this fee. Polymarket sets the rate by market
/// category, so callers pass the rate attached to the market they trade.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn polymarket(contracts: f64, price: f64, rate: f64) -> Result<f64> {
    validate(contracts, price, rate)?;
    let rounded = (rate * contracts * price * (1.0 - price) * 100_000.0).round() / 100_000.0;
    output("Polymarket fee", rounded)
}
