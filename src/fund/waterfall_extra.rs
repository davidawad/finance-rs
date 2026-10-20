//! Carry accrual and hurdle-tier waterfall helpers.
use super::{european_waterfall, WaterfallTerms};
use crate::{
    error::{all_finite, output, FinanceError, Result},
    Date,
};

#[derive(Debug, Clone, Copy, PartialEq)]
/// One carry hurdle and the LP share of the proceeds above it.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::Hurdle>() > 0);
/// ```
pub struct Hurdle {
    /// Hurdle as an annual IRR or return multiple, according to the waterfall convention.
    pub irr_or_multiple: f64,
    /// LP share of the marginal proceeds above this hurdle.
    pub lp_split: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
/// LP and GP allocations from a multi-tier carry calculation.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::MultiHurdle>() > 0);
/// ```
pub struct MultiHurdle {
    /// Cash distributed to limited partners.
    pub lp: f64,
    /// Cash distributed to the general partner.
    pub gp: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
/// Clawback amount after tax benefits and escrow already applied.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::Clawback>() > 0);
/// ```
pub struct Clawback {
    /// Gross carry subject to repayment.
    pub gross: f64,
    /// Tax benefit reducing the GP repayment obligation.
    pub tax_benefit: f64,
    /// Escrow released to satisfy the repayment obligation.
    pub escrow_applied: f64,
    /// Remaining amount due from the GP.
    pub due: f64,
}
/// Hypothetical accrued carry if current NAV were liquidated today, after
/// returning contributed capital net of prior distributions.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::accrued_carry), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn accrued_carry(
    nav: f64,
    contributions: &[f64],
    distributions: &[f64],
    terms: WaterfallTerms,
) -> Result<f64> {
    aligned_nonnegative(contributions, distributions)?;
    let capital = (contributions.iter().sum::<f64>() - distributions.iter().sum::<f64>()).max(0.0);
    Ok(european_waterfall(nav, capital, 0.0, terms)?.gp_total)
}

/// Split distributable cash at sequential annual-return hurdles. Each LP split
/// is the LP's marginal share above that hurdle; final proceeds are conserved.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::multi_hurdle), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn multi_hurdle(
    cash: f64,
    capital: f64,
    tiers: &[Hurdle],
    dates: &[Date],
) -> Result<MultiHurdle> {
    if !cash.is_finite() || !capital.is_finite() {
        return Err(FinanceError::NonFinite {
            what: "cash/capital",
        });
    }
    if cash < 0.0 || capital < 0.0 {
        return Err(invalid("cash and capital must be nonnegative"));
    }
    if tiers.is_empty() || dates.is_empty() {
        return Err(FinanceError::TooShort {
            what: "tiers/dates",
            min: 1,
            got: 0,
        });
    }
    if dates.windows(2).any(|w| w[0] >= w[1]) {
        return Err(invalid("dates must be strictly increasing"));
    }
    if tiers.iter().any(|t| {
        !t.irr_or_multiple.is_finite()
            || t.irr_or_multiple < 0.0
            || !t.lp_split.is_finite()
            || !(0.0..=1.0).contains(&t.lp_split)
    }) {
        return Err(invalid(
            "hurdles must be nonnegative and LP splits in [0,1]",
        ));
    }
    let years = (dates[dates.len() - 1].days_since(dates[0]) as f64) / 365.0;
    let mut sorted = tiers.to_vec();
    sorted.sort_by(|a, b| a.irr_or_multiple.total_cmp(&b.irr_or_multiple));
    let first_target = hurdle_target(sorted[0], capital, years);
    let mut lp = cash.min(first_target);
    for (i, t) in sorted.iter().enumerate() {
        let target = hurdle_target(*t, capital, years);
        let upper = sorted
            .get(i + 1)
            .map_or(f64::INFINITY, |next| hurdle_target(*next, capital, years));
        let tranche = (cash.min(upper) - target).max(0.0);
        lp += tranche * t.lp_split;
    }
    // Cash below the first hurdle belongs to LP; tier allocations above it
    // use the stated LP marginal share, with residual to GP.
    let gp = (cash - lp).max(0.0);
    Ok(MultiHurdle {
        lp: output("multi_hurdle_lp", lp)?,
        gp: output("multi_hurdle_gp", gp)?,
    })
}

fn hurdle_target(hurdle: Hurdle, capital: f64, years: f64) -> f64 {
    if hurdle.irr_or_multiple > 1.0 {
        capital * hurdle.irr_or_multiple
    } else {
        capital * (1.0 + hurdle.irr_or_multiple).powf(years)
    }
}

/// GP clawback net of the tax benefit and escrow already available.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::clawback), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn clawback(
    gp_distributions: f64,
    gp_entitled: f64,
    tax_rate: f64,
    escrow: f64,
) -> Result<Clawback> {
    let xs = [gp_distributions, gp_entitled, tax_rate, escrow];
    all_finite("clawback", &xs)?;
    if gp_distributions < 0.0
        || gp_entitled < 0.0
        || escrow < 0.0
        || !(0.0..=1.0).contains(&tax_rate)
    {
        return Err(invalid(
            "distributions, entitlement and escrow must be nonnegative; tax rate in [0,1]",
        ));
    }
    let gross = (gp_distributions - gp_entitled).max(0.0);
    let tax_benefit = gross * tax_rate;
    let due = (gross - tax_benefit - escrow).max(0.0);
    Ok(Clawback {
        gross,
        tax_benefit,
        escrow_applied: escrow.min(gross - tax_benefit),
        due,
    })
}

fn aligned_nonnegative(a: &[f64], b: &[f64]) -> Result<()> {
    if a.is_empty() {
        return Err(FinanceError::TooShort {
            what: "contributions",
            min: 1,
            got: 0,
        });
    }
    if a.len() != b.len() {
        return Err(FinanceError::LengthMismatch {
            left: "contributions",
            left_len: a.len(),
            right: "distributions",
            right_len: b.len(),
        });
    }
    all_finite("contributions", a)?;
    all_finite("distributions", b)?;
    if a.iter().chain(b).any(|x| *x < 0.0) {
        return Err(invalid("cash flows must be nonnegative magnitudes"));
    }
    Ok(())
}
fn invalid(reason: &'static str) -> FinanceError {
    FinanceError::InvalidInput {
        what: "waterfall",
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn net_tax_clawback_and_unreturned_capital() {
        let c = clawback(30., 20., 0.4, 2.).unwrap();
        assert_eq!(c.due, 4.);
        assert_eq!(
            accrued_carry(120., &[100.], &[40.], WaterfallTerms::default()).unwrap(),
            12.
        );
    }
    #[test]
    fn hurdle_split_conserves_cash() {
        let d = [
            Date::new(2020, 1, 1).unwrap(),
            Date::new(2021, 1, 1).unwrap(),
        ];
        let r = multi_hurdle(
            150.,
            100.,
            &[
                Hurdle {
                    irr_or_multiple: 0.08,
                    lp_split: 0.8,
                },
                Hurdle {
                    irr_or_multiple: 0.12,
                    lp_split: 0.7,
                },
            ],
            &d,
        )
        .unwrap();
        assert!((r.lp + r.gp - 150.).abs() < 1e-10);
    }
    #[test]
    fn bad_cash_is_rejected() {
        assert!(clawback(-1., 0., 0.2, 0.).is_err());
    }

    #[test]
    fn hurdle_and_clawback_validation_and_allocation_edges() {
        let d = [
            Date::new(2020, 1, 1).unwrap(),
            Date::new(2021, 1, 1).unwrap(),
        ];
        let zero_split = multi_hurdle(
            300.0,
            100.0,
            &[Hurdle {
                irr_or_multiple: 2.0,
                lp_split: 0.0,
            }],
            &d,
        )
        .unwrap();
        assert_eq!(zero_split.gp, 100.0);
        assert!(multi_hurdle(
            f64::NAN,
            1.0,
            &[Hurdle {
                irr_or_multiple: 0.1,
                lp_split: 0.8
            }],
            &d
        )
        .is_err());
        assert!(multi_hurdle(
            -1.0,
            1.0,
            &[Hurdle {
                irr_or_multiple: 0.1,
                lp_split: 0.8
            }],
            &d
        )
        .is_err());
        assert!(multi_hurdle(1.0, 1.0, &[], &d).is_err());
        assert!(multi_hurdle(
            1.0,
            1.0,
            &[Hurdle {
                irr_or_multiple: 0.1,
                lp_split: 0.8
            }],
            &[]
        )
        .is_err());
        assert!(multi_hurdle(
            1.0,
            1.0,
            &[Hurdle {
                irr_or_multiple: 0.1,
                lp_split: 0.8
            }],
            &[d[0], d[0]]
        )
        .is_err());
        assert!(multi_hurdle(
            1.0,
            1.0,
            &[Hurdle {
                irr_or_multiple: -0.1,
                lp_split: 0.8
            }],
            &d
        )
        .is_err());
        assert!(multi_hurdle(
            1.0,
            1.0,
            &[Hurdle {
                irr_or_multiple: 0.1,
                lp_split: 1.1
            }],
            &d
        )
        .is_err());
        assert!(clawback(1.0, 1.0, 1.1, 0.0).is_err());
        assert_eq!(clawback(100.0, 0.0, 0.0, 100.0).unwrap().due, 0.0);
        assert!(accrued_carry(1.0, &[], &[], WaterfallTerms::default()).is_err());
        assert!(accrued_carry(1.0, &[1.0], &[], WaterfallTerms::default()).is_err());
        assert!(accrued_carry(1.0, &[f64::NAN], &[0.0], WaterfallTerms::default()).is_err());
        assert!(accrued_carry(1.0, &[-1.0], &[0.0], WaterfallTerms::default()).is_err());
    }
}
