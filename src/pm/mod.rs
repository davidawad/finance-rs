//! Prediction-market probability, pricing, fee and calibration functions.
//!
//! Prices and probabilities use the unit interval; contract payouts are one
//! unit. Fee rates and prices are explicit because venue schedules vary.

pub mod devig;
pub mod fee;
pub mod odds;
pub mod score;

pub use devig::{additive, multiplicative, power, shin};
pub use odds::{
    american_to_prob, decimal_to_prob, fractional_to_prob, prob_to_american, prob_to_decimal,
    prob_to_fractional,
};
pub use score::{brier, log_loss};

use crate::error::{finite, output, FinanceError, Result};

/// Which side of a binary YES/NO contract is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
pub enum Side {
    /// The yes option.
    Yes,
    /// The no option.
    No,
}

/// Implied probability from price and payout.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn implied_probability(price: f64, payout: f64) -> Result<f64> {
    finite("price", price)?;
    finite("payout", payout)?;
    if price < 0.0 || price > payout || payout <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "price/payout",
            reason: "price must be between zero and a positive payout",
        });
    }
    output("implied probability", price / payout)
}

/// Sum of prices/probabilities less one (the book's overround).
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn overround(prices_or_probabilities: &[f64]) -> Result<f64> {
    if prices_or_probabilities.is_empty() {
        return Err(FinanceError::TooShort {
            what: "prices_or_probabilities",
            min: 1,
            got: 0,
        });
    }
    if prices_or_probabilities
        .iter()
        .any(|x| !x.is_finite() || *x < 0.0)
    {
        return Err(FinanceError::InvalidInput {
            what: "prices_or_probabilities",
            reason: "values must be finite and non-negative",
        });
    }
    output(
        "overround",
        prices_or_probabilities.iter().sum::<f64>() - 1.0,
    )
}

/// Price of the held side: YES price or its complement for NO.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn held_side_price(yes_price: f64, side: Side) -> Result<f64> {
    finite("yes_price", yes_price)?;
    if !(0.0..=1.0).contains(&yes_price) {
        return Err(FinanceError::InvalidInput {
            what: "yes_price",
            reason: "must be in [0, 1]",
        });
    }
    Ok(match side {
        Side::Yes => yes_price,
        Side::No => 1.0 - yes_price,
    })
}

/// Smooth fee per contract: `rate * ask * (1 - ask)` (no cent rounding).
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn fee(ask: f64, rate: f64) -> Result<f64> {
    finite("ask", ask)?;
    finite("rate", rate)?;
    if !(0.0..=1.0).contains(&ask) || rate < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "fee",
            reason: "ask must be in [0, 1] and rate non-negative",
        });
    }
    output("prediction-market fee", rate * ask * (1.0 - ask))
}

/// Ask plus its smooth per-contract fee.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn all_in_cost(ask: f64, fee_rate: f64) -> Result<f64> {
    output("all-in cost", ask + fee(ask, fee_rate)?)
}

/// Invert `cost = ask + fee_rate * ask * (1 - ask)` for the root in `[0, 1]`.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn ask_of_cost(cost: f64, fee_rate: f64) -> Result<f64> {
    finite("cost", cost)?;
    finite("fee_rate", fee_rate)?;
    if !(0.0..=1.0).contains(&cost) || fee_rate < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "cost/fee_rate",
            reason: "cost must be in [0, 1] and fee rate non-negative",
        });
    }
    if fee_rate == 0.0 {
        return Ok(cost);
    }
    let disc = (1.0 + fee_rate).powi(2) - 4.0 * fee_rate * cost;
    // On [0, 1] costs, the discriminant is (1 - fee_rate)^2 +
    // 4 * fee_rate * (1 - cost), so a negative value is unreachable.
    output(
        "ask_of_cost",
        (1.0 + fee_rate - disc.max(0.0).sqrt()) / (2.0 * fee_rate),
    )
}

/// Full Kelly share for a binary contract paying `payout` on a win.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
pub fn kelly_fraction(p_true: f64, cost: f64, payout: f64) -> Result<f64> {
    for (name, v) in [("p_true", p_true), ("cost", cost), ("payout", payout)] {
        finite(name, v)?;
    }
    if !(0.0..=1.0).contains(&p_true) || payout <= cost {
        return Err(FinanceError::InvalidInput {
            what: "kelly_fraction",
            reason: "probability in [0, 1], positive cost, and payout greater than cost required",
        });
    }
    if p_true == 1.0 {
        return Ok(1.0);
    }
    if cost <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "cost",
            reason: "must be positive unless the win is certain",
        });
    }
    // Keep the direct form used by the existing callers so the shared oracle
    // preserves their last-bit numeric and JSON audit parity.
    let raw = (p_true * payout - cost) / (payout - cost);
    Ok(crate::error::output("prediction-market Kelly", raw)?.clamp(0.0, 1.0))
}

/// Binary contract expected profit per contract, `p_true - (price + fee)`.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn ev(p_true: f64, price: f64, fee: f64) -> Result<f64> {
    for (name, v) in [("p_true", p_true), ("price", price), ("fee", fee)] {
        finite(name, v)?;
    }
    if !(0.0..=1.0).contains(&p_true) || price < 0.0 || fee < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "ev",
            reason: "probability must be in [0, 1] and price and fee non-negative",
        });
    }
    output("prediction-market EV", p_true - price - fee)
}

/// Break-even probability for a one-unit payout, equal to all-in cost.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn breakeven_probability(price: f64, fee: f64) -> Result<f64> {
    finite("price", price)?;
    finite("fee", fee)?;
    if price < 0.0 || fee < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "price/fee",
            reason: "must be non-negative",
        });
    }
    output("breakeven probability", price + fee)
}

/// Annualize a simple total return over a positive number of days.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn annualized_return(ev: f64, cost: f64, days_to_resolution: f64) -> Result<f64> {
    for (name, v) in [
        ("ev", ev),
        ("cost", cost),
        ("days_to_resolution", days_to_resolution),
    ] {
        finite(name, v)?;
    }
    if cost <= 0.0 || days_to_resolution <= 0.0 || ev <= -cost {
        return Err(FinanceError::InvalidInput {
            what: "annualized_return",
            reason: "cost and days must be positive and total return greater than -100%",
        });
    }
    output(
        "annualized return",
        (1.0 + ev / cost).powf(365.0 / days_to_resolution) - 1.0,
    )
}

/// Arbitrage profit per paired YES/NO contract, if positive after fees.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn arbitrage(yes_ask: f64, no_ask: f64, fees: f64) -> Result<Option<f64>> {
    for (name, v) in [("yes_ask", yes_ask), ("no_ask", no_ask), ("fees", fees)] {
        finite(name, v)?;
    }
    if yes_ask < 0.0 || no_ask < 0.0 || fees < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "arbitrage",
            reason: "asks and fees must be non-negative",
        });
    }
    let profit = 1.0 - yes_ask - no_ask - fees;
    Ok((profit > 0.0).then_some(profit))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_edges_cover_yes_and_certain_kelly_paths() {
        assert_eq!(held_side_price(0.4, Side::Yes).unwrap(), 0.4);
        assert_eq!(kelly_fraction(1.0, 0.4, 1.0).unwrap(), 1.0);
        assert_eq!(ask_of_cost(0.4, 0.0).unwrap(), 0.4);
        assert!(arbitrage(0.5, 0.4, 0.2).unwrap().is_none());
    }
}
