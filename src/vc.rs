//! Venture financing, SAFE conversion, ownership and exit-waterfall arithmetic.
//!
//! Money and share counts are expressed in consistent units; rates and ownership
//! are decimals. Functions are pure and reject non-finite or out-of-domain inputs.
use crate::error::{finite, output, FinanceError};
use crate::Result;

fn nonnegative(name: &'static str, x: f64) -> Result<f64> {
    finite(name, x)?;
    if x < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: name,
            reason: "must be non-negative",
        });
    }
    Ok(x)
}
fn positive(name: &'static str, x: f64) -> Result<f64> {
    finite(name, x)?;
    if x <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: name,
            reason: "must be positive",
        });
    }
    Ok(x)
}
fn fraction(name: &'static str, x: f64) -> Result<f64> {
    finite(name, x)?;
    if !(0.0..=1.0).contains(&x) {
        return Err(FinanceError::InvalidInput {
            what: name,
            reason: "must be in [0, 1]",
        });
    }
    Ok(x)
}

/// Post-money valuation from pre-money valuation and new investment.
///
/// # Errors
/// Returns an error for negative/non-finite inputs or overflow.
pub fn post_money(pre_money: f64, investment: f64) -> Result<f64> {
    nonnegative("pre_money", pre_money)?;
    nonnegative("investment", investment)?;
    output("post_money", pre_money + investment)
}
/// Price per share immediately before financing.
///
/// # Errors
/// Returns an error unless pre-money valuation is non-negative and shares are positive.
pub fn price_per_share(pre_money: f64, fd_shares_pre: f64) -> Result<f64> {
    nonnegative("pre_money", pre_money)?;
    positive("fd_shares_pre", fd_shares_pre)?;
    output("price_per_share", pre_money / fd_shares_pre)
}
/// Ownership after investment at pre-money valuation, before any pool shuffle.
///
/// # Errors
/// Returns an error for invalid ownership/valuation inputs or an undefined ratio.
pub fn dilution(ownership: f64, pre_money: f64, post_money: f64) -> Result<f64> {
    fraction("ownership", ownership)?;
    positive("pre_money", pre_money)?;
    positive("post_money", post_money)?;
    if post_money < pre_money {
        return Err(FinanceError::InvalidInput {
            what: "post_money",
            reason: "must be at least pre_money",
        });
    }
    output("dilution", ownership * pre_money / post_money)
}
/// Effective pre-money valuation after a pool top-up allocated to existing holders.
///
/// # Errors
/// Returns an error for invalid terms, a 100% pool, or a non-finite result.
pub fn option_pool_shuffle(pre_money: f64, investment: f64, target_pool_post: f64) -> Result<f64> {
    positive("pre_money", pre_money)?;
    nonnegative("investment", investment)?;
    fraction("target_pool_post", target_pool_post)?;
    let denom = 1.0 - target_pool_post;
    if denom == 0.0 {
        return Err(FinanceError::Undefined {
            what: "pre-money after a 100% option pool",
        });
    }
    output(
        "effective_pre_money",
        pre_money - investment * target_pool_post / denom,
    )
}

/// One financing round's inputs. `pool_in_pre` means the target is already inside pre-money shares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Round {
    /// Investment for the round result.
    pub investment: f64,
    /// Pre money for the round result.
    pub pre_money: f64,
    /// Pool target post for the round result.
    pub pool_target_post: f64,
    /// Pool in pre for the round result.
    pub pool_in_pre: bool,
}
/// Founder, investor and option-pool ownership after a priced round.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoundResult {
    /// Price per share for the round result result.
    pub price_per_share: f64,
    /// New shares for the round result result.
    pub new_shares: f64,
    /// Founder ownership for the round result result.
    pub founder_ownership: f64,
    /// Investor ownership for the round result result.
    pub investor_ownership: f64,
    /// Pool ownership for the round result result.
    pub pool_ownership: f64,
}
/// Resolve round issuance. Share counts are normalized to one pre-round fully-diluted share.
///
/// # Errors
/// Returns an error when ownership does not sum to one or terms cannot be priced.
pub fn round(cap_table: &[f64], terms: Round) -> Result<RoundResult> {
    if cap_table.is_empty() {
        return Err(FinanceError::TooShort {
            what: "cap_table",
            min: 1,
            got: 0,
        });
    }
    let mut sum = 0.0;
    for x in cap_table {
        nonnegative("cap_table ownership", *x)?;
        sum += x;
    }
    if (sum - 1.0).abs() > 1e-9 {
        return Err(FinanceError::InvalidInput {
            what: "cap_table",
            reason: "ownership fractions must sum to 1",
        });
    }
    positive("pre_money", terms.pre_money)?;
    nonnegative("investment", terms.investment)?;
    fraction("pool_target_post", terms.pool_target_post)?;
    let existing_pool = if terms.pool_in_pre {
        cap_table.last().copied().unwrap_or(0.0)
    } else {
        0.0
    };
    let new_pool_shares = if terms.pool_in_pre || terms.pool_target_post == 0.0 {
        0.0
    } else {
        let investment_to_pre = terms.investment / terms.pre_money;
        let denom = 1.0 - terms.pool_target_post * (1.0 + investment_to_pre);
        if denom <= 0.0 {
            return Err(FinanceError::Undefined {
                what: "option pool target at this investment size",
            });
        }
        terms.pool_target_post * (1.0 + investment_to_pre) / denom
    };
    let pps = terms.pre_money / (1.0 + new_pool_shares);
    positive("price_per_share", pps)?;
    let new_shares = terms.investment / pps;
    let total = 1.0 + new_pool_shares + new_shares;
    let pool = if terms.pool_in_pre {
        existing_pool / total
    } else {
        new_pool_shares / total
    };
    let founder = (1.0 - existing_pool) / total;
    // The cap table is validated to sum to one above. With `total` including
    // exactly the founder remainder, investor shares, and (pre or post) pool,
    // these three fractions sum algebraically to one; the former tolerance
    // guard was unreachable for finite values validated by this function.
    Ok(RoundResult {
        price_per_share: output("price_per_share", pps)?,
        new_shares: output("new_shares", new_shares)?,
        founder_ownership: output("founder_ownership", founder)?,
        investor_ownership: output("investor_ownership", new_shares / total)?,
        pool_ownership: output("pool_ownership", pool)?,
    })
}

/// Venture-capital method: post-money valuation = exit × retained ownership / target multiple.
///
/// # Errors
/// Returns an error for invalid targets, years, retention, or overflow.
pub fn vc_method(
    exit_value: f64,
    target_multiple_or_irr: f64,
    years: f64,
    investment: f64,
    retention: f64,
) -> Result<(f64, f64)> {
    nonnegative("exit_value", exit_value)?;
    positive("target_multiple_or_irr", target_multiple_or_irr)?;
    nonnegative("years", years)?;
    nonnegative("investment", investment)?;
    fraction("retention", retention)?;
    let multiple = if target_multiple_or_irr <= 1.0 {
        (1.0 + target_multiple_or_irr).powf(years)
    } else {
        target_multiple_or_irr
    };
    let post = exit_value * retention / multiple;
    let pre = post - investment;
    Ok((output("pre_money", pre)?, output("post_money", post)?))
}
/// Ownership required for investment to achieve the target exit multiple.
///
/// # Errors
/// Returns an error for invalid inputs, zero retention, or a requirement above 100%.
pub fn required_ownership(
    investment: f64,
    exit_value: f64,
    target_multiple: f64,
    retention: f64,
) -> Result<f64> {
    positive("investment", investment)?;
    positive("exit_value", exit_value)?;
    positive("target_multiple", target_multiple)?;
    fraction("retention", retention)?;
    if retention == 0.0 {
        return Err(FinanceError::Undefined {
            what: "required ownership at zero retention",
        });
    }
    let ownership = investment * target_multiple / (exit_value * retention);
    if ownership > 1.0 {
        return Err(FinanceError::Undefined {
            what: "required ownership exceeds 100%",
        });
    }
    output("required_ownership", ownership)
}
/// Exit value required for an ownership stake to return the full fund size.
///
/// # Errors
/// Returns an error unless fund size and ownership are positive and finite.
pub fn return_the_fund(fund_size: f64, ownership_at_exit: f64) -> Result<f64> {
    positive("fund_size", fund_size)?;
    positive("ownership_at_exit", ownership_at_exit)?;
    output("return_the_fund", fund_size / ownership_at_exit)
}
/// Probability weighted expected MOIC and arithmetic spread of exit multiples.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScenarioReturns {
    /// Expected moic for the scenario returns result.
    pub expected_moic: f64,
    /// Power law spread for the scenario returns result.
    pub power_law_spread: f64,
}

/// Probability-weighted expected MOIC and standard deviation across exit cases.
///
/// # Errors
/// Returns an error for empty scenarios, invalid probabilities, or probabilities that do not sum to one.
pub fn scenario_returns(scenarios: &[(f64, f64)]) -> Result<ScenarioReturns> {
    if scenarios.is_empty() {
        return Err(FinanceError::TooShort {
            what: "scenarios",
            min: 1,
            got: 0,
        });
    }
    let (mut p, mut mean, mut variance) = (0.0, 0.0, 0.0);
    for (prob, m) in scenarios {
        fraction("probability", *prob)?;
        nonnegative("exit_multiple", *m)?;
        p += prob;
        mean += prob * m;
    }
    if (p - 1.0).abs() > 1e-9 {
        return Err(FinanceError::InvalidInput {
            what: "probabilities",
            reason: "must sum to 1",
        });
    }
    for (prob, m) in scenarios {
        variance += prob * (m - mean).powi(2);
    }
    Ok(ScenarioReturns {
        expected_moic: output("expected_moic", mean)?,
        power_law_spread: output("power_law_spread", variance.sqrt())?,
    })
}
/// Annualized return from a positive MOIC; a zero/negative holding period returns MOIC minus one.
///
/// # Errors
/// Returns an error for non-positive MOIC, negative holding periods, or overflow.
pub fn annualized_from_moic(moic: f64, years: f64) -> Result<f64> {
    positive("moic", moic)?;
    nonnegative("years", years)?;
    if years == 0.0 {
        return output("annualized_return", moic - 1.0);
    }
    output("annualized_return", moic.powf(1.0 / years) - 1.0)
}
/// Secondary price premium/(discount) to NAV; negative values indicate a discount.
///
/// # Errors
/// Returns an error unless NAV is positive and the price is non-negative.
pub fn secondary_discount(price: f64, nav: f64) -> Result<f64> {
    nonnegative("price", price)?;
    positive("nav", nav)?;
    output("secondary_discount", price / nav - 1.0)
}
/// Capital reserve needed to maintain ownership in the next financing.
///
/// # Errors
/// Returns an error for an invalid ownership fraction or negative round size.
pub fn pro_rata_reserve(ownership: f64, next_round_size: f64) -> Result<f64> {
    fraction("ownership", ownership)?;
    nonnegative("next_round_size", next_round_size)?;
    output("pro_rata_reserve", ownership * next_round_size)
}
/// Investable capital after fees and recycling.
///
/// # Errors
/// Returns an error for negative/non-finite inputs or overflow.
pub fn net_deployed(commitments: f64, fees: f64, recycling: f64) -> Result<f64> {
    nonnegative("commitments", commitments)?;
    nonnegative("fees", fees)?;
    nonnegative("recycling", recycling)?;
    output("net_deployed", (commitments - fees + recycling).max(0.0))
}

#[path = "vc/anti_dilution.rs"]
/// Public submodule for anti dilution.
pub mod anti_dilution;
#[path = "vc/cap_table.rs"]
/// Public submodule for cap table.
pub mod cap_table;
#[path = "vc/note.rs"]
/// Public submodule for note.
pub mod note;
#[path = "vc/safe.rs"]
/// Public submodule for safe.
pub mod safe;
#[cfg(test)]
#[path = "vc/tests.rs"]
mod tests;
#[path = "vc/waterfall.rs"]
/// Public submodule for waterfall.
pub mod waterfall;
