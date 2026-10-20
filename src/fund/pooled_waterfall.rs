//! Class-aware whole-fund waterfall with preferred return and promote tiers.

use crate::error::{output, FinanceError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Enumerates pooled mode choices.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::PooledMode>() > 0);
/// ```
pub enum PooledMode {
    /// The european option.
    European,
    /// The american option.
    American,
}

#[derive(Debug, Clone, PartialEq)]
/// Data structure for pooled class.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::PooledClass>() > 0);
/// ```
pub struct PooledClass {
    /// Name for the pooled class result.
    pub name: String,
    /// Contributed capital for the pooled class result.
    pub contributed_capital: f64,
    /// Ownership share for the pooled class result.
    pub ownership_share: f64,
    /// Preferred return rate for the pooled class result.
    pub preferred_return_rate: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Data structure for pooled tier.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::PooledTier>() > 0);
/// ```
pub struct PooledTier {
    /// Hurdle multiple for the pooled tier result.
    pub hurdle_multiple: f64,
    /// Promote rate for the pooled tier result.
    pub promote_rate: f64,
}

#[derive(Debug, Clone, PartialEq)]
/// Data structure for pooled input.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::PooledInput>() > 0);
/// ```
pub struct PooledInput {
    /// Mode for the pooled input result.
    pub mode: PooledMode,
    /// Classes for the pooled input result.
    pub classes: Vec<PooledClass>,
    /// Realizations for the pooled input result.
    pub realizations: Vec<f64>,
    /// Years for the pooled input result.
    pub years: f64,
    /// Catch up rate for the pooled input result.
    pub catch_up_rate: f64,
    /// Tiers for the pooled input result.
    pub tiers: Vec<PooledTier>,
}

#[derive(Debug, Clone, PartialEq)]
/// Data structure for pooled distribution.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::PooledDistribution>() > 0);
/// ```
pub struct PooledDistribution {
    /// Name for the pooled distribution result.
    pub name: String,
    /// Capital return for the pooled distribution result.
    pub capital_return: f64,
    /// Preferred return for the pooled distribution result.
    pub preferred_return: f64,
    /// Lp profit for the pooled distribution result.
    pub lp_profit: f64,
    /// Total distribution for the pooled distribution result.
    pub total_distribution: f64,
}

#[derive(Debug, Clone, PartialEq)]
/// Data structure for pooled output.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::PooledOutput>() > 0);
/// ```
pub struct PooledOutput {
    /// Mode for the pooled output result.
    pub mode: PooledMode,
    /// Total proceeds for the pooled output result.
    pub total_proceeds: f64,
    /// Lp distributions for the pooled output result.
    pub lp_distributions: Vec<PooledDistribution>,
    /// Gp catch up for the pooled output result.
    pub gp_catch_up: f64,
    /// Gp promote for the pooled output result.
    pub gp_promote: f64,
    /// Clawback reserve for the pooled output result.
    pub clawback_reserve: f64,
    /// Reconciliation difference for the pooled output result.
    pub reconciliation_difference: f64,
}

fn cents(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn catch_up(pref: f64, rate: f64) -> f64 {
    if rate == 0.0 {
        0.0
    } else {
        pref * rate / (1.0 - rate)
    }
}

/// GP distribution required to reach a target share after an LP preference.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::catch_up_amount), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn catch_up_amount(pref: f64, rate: f64) -> Result<f64> {
    if !pref.is_finite() || pref < 0.0 || !rate.is_finite() || !(0.0..1.0).contains(&rate) {
        return Err(FinanceError::InvalidInput {
            what: "catch_up",
            reason: "preference must be non-negative and rate in [0, 1)",
        });
    }
    output("catch_up", catch_up(pref, rate))
}

fn promote(profit: f64, capital: f64, tiers: &[PooledTier]) -> f64 {
    let mut sorted = tiers.to_vec();
    sorted.sort_by(|a, b| a.hurdle_multiple.total_cmp(&b.hurdle_multiple));
    let mut gp = 0.0;
    for (index, tier) in sorted.iter().enumerate() {
        let lower = (tier.hurdle_multiple - 1.0) * capital;
        let upper = sorted
            .get(index + 1)
            .map_or(f64::INFINITY, |next| (next.hurdle_multiple - 1.0) * capital);
        gp += (profit.min(upper) - lower).max(0.0) * tier.promote_rate;
    }
    gp
}

/// Apply cumulative gross-proceeds promote tiers to fund profit.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::pooled_promote_tiers), 0);
/// ```
#[must_use]
pub fn pooled_promote_tiers(profit: f64, capital: f64, tiers: &[PooledTier]) -> f64 {
    promote(profit, capital, tiers)
}

/// Calculate a pooled LP waterfall, including class allocations and tiered promote.
///
/// This is the canonical implementation used by fund metrics, workbook oracles,
/// and fund-core. Values are rounded to cents at the same distribution boundaries
/// as the historical workbook formula.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::waterfall), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::too_many_lines)] // This cohesive regression scenario is intentionally kept together to show the full contract.
pub fn waterfall(input: &PooledInput) -> Result<PooledOutput> {
    if input.classes.is_empty() {
        return Err(FinanceError::InvalidInput {
            what: "classes",
            reason: "must not be empty",
        });
    }
    if !input.years.is_finite()
        || input.years < 0.0
        || !input.catch_up_rate.is_finite()
        || !(0.0..1.0).contains(&input.catch_up_rate)
        || input
            .realizations
            .iter()
            .any(|x| !x.is_finite() || *x < 0.0)
        || input.classes.iter().any(|c| {
            c.name.is_empty()
                || !c.contributed_capital.is_finite()
                || c.contributed_capital < 0.0
                || !c.ownership_share.is_finite()
                || !(0.0..=1.0).contains(&c.ownership_share)
                || !c.preferred_return_rate.is_finite()
                || !(0.0..=1.0).contains(&c.preferred_return_rate)
        })
        || input.tiers.iter().any(|t| {
            !t.hurdle_multiple.is_finite()
                || t.hurdle_multiple < 1.0
                || !t.promote_rate.is_finite()
                || !(0.0..=1.0).contains(&t.promote_rate)
        })
    {
        return Err(FinanceError::InvalidInput {
            what: "waterfall",
            reason: "inputs are outside supported bounds",
        });
    }
    let capital = input
        .classes
        .iter()
        .map(|c| c.contributed_capital)
        .sum::<f64>();
    let shares = input.classes.iter().map(|c| c.ownership_share).sum::<f64>();
    if !capital.is_finite() || capital <= 0.0 || (shares - 1.0).abs() > 1e-8 {
        return Err(FinanceError::InvalidInput {
            what: "classes",
            reason: "capital must be positive and ownership shares must sum to one",
        });
    }
    let mut pref = 0.0;
    for class in &input.classes {
        let capital = class.contributed_capital;
        let rate = class.preferred_return_rate;
        let years = input.years;
        let compounding = crate::fund::Compounding::Annual;
        let preferred = crate::fund::preferred_return(capital, rate, years, compounding);
        pref += preferred?;
    }
    let proceeds = input.realizations.iter().sum::<f64>();
    let pref_paid = cents((proceeds - capital).max(0.0).min(pref));
    let gp_catch_up = cents(
        (proceeds - capital - pref_paid)
            .max(0.0)
            .min(catch_up(pref_paid, input.catch_up_rate)),
    );
    let residual = (proceeds - capital - pref_paid - gp_catch_up).max(0.0);
    let gp_promote = cents(promote(residual, capital, &input.tiers).min(residual));
    let lp_residual = cents((residual - gp_promote).max(0.0));
    let mut capital_remaining = capital;
    let mut lp_distributions = Vec::with_capacity(input.classes.len());
    for class in &input.classes {
        let capital_return = capital_remaining.min(proceeds * class.contributed_capital / capital);
        capital_remaining -= capital_return;
        let class_pref = cents(pref_paid * class.ownership_share);
        let lp_profit =
            cents(pref_paid * class.ownership_share + lp_residual * class.ownership_share);
        lp_distributions.push(PooledDistribution {
            name: class.name.clone(),
            capital_return,
            preferred_return: class_pref,
            lp_profit,
            total_distribution: cents(capital_return + lp_profit),
        });
    }
    let distributed = lp_distributions
        .iter()
        .map(|d| d.total_distribution)
        .sum::<f64>()
        + gp_catch_up
        + gp_promote;
    let clawback_reserve = if input.mode == PooledMode::American {
        let interim = input
            .realizations
            .iter()
            .map(|proceeds| {
                let deal_profit = (*proceeds - capital).max(0.0);
                let deal_pref = pref.min(deal_profit);
                let cu = catch_up(deal_pref, input.catch_up_rate)
                    .min((deal_profit - deal_pref).max(0.0));
                let rest = (deal_profit - deal_pref - cu).max(0.0);
                cu + promote(rest, capital, &input.tiers).min(rest)
            })
            .sum::<f64>();
        (interim - gp_catch_up - gp_promote).max(0.0)
    } else {
        0.0
    };
    Ok(PooledOutput {
        mode: input.mode,
        total_proceeds: output("total_proceeds", proceeds)?,
        lp_distributions,
        gp_catch_up,
        gp_promote,
        clawback_reserve,
        reconciliation_difference: cents(proceeds - distributed),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pooled_european_allocation_matches_legacy_golden() -> Result<()> {
        let input = PooledInput {
            mode: PooledMode::European,
            classes: vec![PooledClass {
                name: "Class A".into(),
                contributed_capital: 100.0,
                ownership_share: 1.0,
                preferred_return_rate: 0.08,
            }],
            realizations: vec![100.0, 80.0],
            years: 2.0,
            catch_up_rate: 0.2,
            tiers: vec![
                PooledTier {
                    hurdle_multiple: 1.0,
                    promote_rate: 0.2,
                },
                PooledTier {
                    hurdle_multiple: 2.0,
                    promote_rate: 0.3,
                },
            ],
        };
        let result = waterfall(&input);
        let output = result?;
        assert_eq!(output.total_proceeds, 180.0);
        assert_eq!(output.lp_distributions[0].preferred_return, 16.64);
        assert_eq!(output.gp_catch_up, 4.16);
        assert_eq!(output.gp_promote, 11.84);
        assert_eq!(output.reconciliation_difference, 0.0);
        Ok(())
    }

    #[test]
    fn catch_up_promote_and_american_clawback_edges() -> Result<()> {
        assert_eq!(catch_up_amount(100.0, 0.0)?, 0.0);
        assert_eq!(catch_up_amount(100.0, 0.2)?, 25.0);
        assert!(catch_up_amount(-1.0, 0.2).is_err());
        assert!(catch_up_amount(100.0, 1.0).is_err());
        let tiers = [
            PooledTier {
                hurdle_multiple: 2.0,
                promote_rate: 0.3,
            },
            PooledTier {
                hurdle_multiple: 1.5,
                promote_rate: 0.2,
            },
        ];
        assert_eq!(pooled_promote_tiers(25.0, 100.0, &tiers), 0.0);
        assert!((pooled_promote_tiers(150.0, 100.0, &tiers) - 25.0).abs() < 1e-12);

        let input = PooledInput {
            mode: PooledMode::American,
            classes: vec![PooledClass {
                name: "LP".into(),
                contributed_capital: 100.0,
                ownership_share: 1.0,
                preferred_return_rate: 0.0,
            }],
            realizations: vec![200.0, 50.0],
            years: 0.0,
            catch_up_rate: 0.2,
            tiers: tiers.to_vec(),
        };
        let american = waterfall(&input)?;
        assert_eq!(american.clawback_reserve, 0.0);
        assert_eq!(american.total_proceeds, 250.0);
        let mut european_input = input.clone();
        european_input.mode = PooledMode::European;
        let european = waterfall(&european_input)?;
        assert_eq!(european.clawback_reserve, 0.0);
        assert!(waterfall(&PooledInput {
            classes: vec![],
            ..input.clone()
        })
        .is_err());
        assert!(waterfall(&PooledInput {
            years: -1.0,
            ..input.clone()
        })
        .is_err());
        assert!(waterfall(&PooledInput {
            catch_up_rate: 1.0,
            ..input.clone()
        })
        .is_err());
        assert!(waterfall(&PooledInput {
            realizations: vec![-1.0],
            ..input.clone()
        })
        .is_err());
        assert!(waterfall(&PooledInput {
            classes: vec![PooledClass {
                ownership_share: 0.5,
                ..input.classes[0].clone()
            }],
            ..input.clone()
        })
        .is_err());
        assert!(waterfall(&PooledInput {
            tiers: vec![PooledTier {
                hurdle_multiple: 0.5,
                promote_rate: 0.2
            }],
            ..input.clone()
        })
        .is_err());
        Ok(())
    }
}
