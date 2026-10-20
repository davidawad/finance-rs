//! Private-equity fund metrics (ILPA definitions) and distribution waterfalls.
//!
//! * Multiples: [`tvpi`], [`dpi`], [`rvpi`] and [`multiples`], all relative
//!   to paid-in capital.
//! * Public-market equivalents: [`ks_pme`] (Kaplan-Schoar) and
//!   [`direct_alpha`] (Gredil-Griffiths-Stucke).
//! * Waterfalls: [`european_waterfall`] (whole fund) and
//!   [`american_waterfall`] (deal by deal), with the preferred-return helpers
//!   [`preferred_return`] and [`preferred_return_schedule`].
//!
//! Cash-flow series are from the LP's point of view and are given as
//! non-negative magnitudes: `contributions[t]` paid in and
//! `distributions[t]` received in period `t`, aligned with the public index
//! level `index[t]` observed on the same date.

mod administration;
mod fees;
mod operations;
mod pe;
mod pooled_waterfall;
mod returns;
mod waterfall;
mod waterfall_extra;
pub use administration::{
    capital_account_roll_forward, capital_call_allocate, equalization, gp_commitment, j_curve,
    recallable, takahashi_alexander, unfunded_coverage, CapitalAllocations, CapitalCallAllocation,
    Equalization, JCurve, RecallTerms, Recallable, UnfundedCoverage,
};
#[cfg(feature = "decimal")]
pub use administration::{capital_account_roll_forward_decimal, DecimalCapitalAllocations};
pub use fees::{management_fee, offsets, FeeBasis, FeeSchedule, ManagementFee};
pub use operations::{look_through, pacing_schedule, Exposure, FundPacingRow, FundPacingYear};
pub use pe::{
    gross_irr, ks_pme_flows, ln_pme, ln_pme_nav, m_pme, moic, net_irr, pic, pme_plus,
    pme_plus_flows, pme_plus_lambda,
};
pub use pooled_waterfall::{
    catch_up_amount, pooled_promote_tiers, waterfall, PooledClass, PooledDistribution, PooledInput,
    PooledMode, PooledOutput, PooledTier,
};
pub use returns::{
    gross_net_irr, secondary_price, secondary_price_from_nav, twr_vs_mwr, FundReturns,
    SecondaryPrice, TwrVsMwr,
};
pub use waterfall_extra::{accrued_carry, clawback, multi_hurdle, Clawback, Hurdle, MultiHurdle};

pub use waterfall::{
    american_waterfall, european_waterfall, preferred_return, preferred_return_schedule,
    AmericanWaterfall, Compounding, Deal, Losses, PrefBalance, Tier, Waterfall, WaterfallTerms,
};

use crate::error::{all_finite, div, finite, min_len, output, same_len, FinanceError, Result};
use crate::tvm::irr;

/// The three ILPA multiples of a fund, all on paid-in capital.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::Multiples>() > 0);
/// ```
pub struct Multiples {
    /// Total value to paid-in: `(distributions + NAV) / paid_in`.
    pub tvpi: f64,
    /// Distributions to paid-in: `distributions / paid_in`.
    pub dpi: f64,
    /// Residual value to paid-in: `NAV / paid_in`.
    pub rvpi: f64,
}

fn check_amounts(paid_in: f64, distributions: f64, nav: f64) -> Result<()> {
    finite("paid_in", paid_in)?;
    finite("distributions", distributions)?;
    finite("nav", nav)?;
    if paid_in < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "paid_in",
            reason: "must be non-negative",
        });
    }
    if distributions < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "distributions",
            reason: "must be non-negative",
        });
    }
    Ok(())
}

/// Total value to paid-in capital.
///
/// `TVPI = (distributions + NAV) / paid_in`
///
/// ```rust
/// use finance_rs::fund::tvpi;
/// assert!((tvpi(100.0, 60.0, 90.0)? - 1.5).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NonFinite`] for non-finite input,
/// [`FinanceError::InvalidInput`] for negative paid-in or distributions,
/// [`FinanceError::Undefined`] when `paid_in` is zero.
pub fn tvpi(paid_in: f64, distributions: f64, nav: f64) -> Result<f64> {
    check_amounts(paid_in, distributions, nav)?;
    div("paid_in", distributions + nav, paid_in)
}

/// Distributions to paid-in capital (the realized multiple).
///
/// `DPI = distributions / paid_in`
///
/// ```rust
/// use finance_rs::fund::dpi;
/// assert!((dpi(100.0, 60.0)? - 0.6).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`tvpi`].
pub fn dpi(paid_in: f64, distributions: f64) -> Result<f64> {
    check_amounts(paid_in, distributions, 0.0)?;
    div("paid_in", distributions, paid_in)
}

/// Residual value to paid-in capital (the unrealized multiple).
///
/// `RVPI = NAV / paid_in`
///
/// ```rust
/// use finance_rs::fund::rvpi;
/// assert!((rvpi(100.0, 90.0)? - 0.9).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`tvpi`].
pub fn rvpi(paid_in: f64, nav: f64) -> Result<f64> {
    check_amounts(paid_in, 0.0, nav)?;
    div("paid_in", nav, paid_in)
}

/// TVPI, DPI and RVPI at once; `tvpi = dpi + rvpi` by construction.
///
/// `DPI = D / PI`, `RVPI = NAV / PI`, `TVPI = DPI + RVPI`
///
/// ```rust
/// use finance_rs::fund::multiples;
/// let m = multiples(100.0, 60.0, 90.0)?;
/// assert!((m.tvpi - (m.dpi + m.rvpi)).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`tvpi`].
pub fn multiples(paid_in: f64, distributions: f64, nav: f64) -> Result<Multiples> {
    let d = dpi(paid_in, distributions)?;
    let r = rvpi(paid_in, nav)?;
    Ok(Multiples {
        tvpi: output("tvpi", d + r)?,
        dpi: d,
        rvpi: r,
    })
}

/// Validate aligned PME inputs and return the index growth factors
/// `I_T / I_t` for each period.
fn pme_factors(
    contributions: &[f64],
    distributions: &[f64],
    nav: f64,
    index: &[f64],
) -> Result<Vec<f64>> {
    min_len("contributions", contributions, 1)?;
    same_len(
        "contributions",
        contributions.len(),
        "distributions",
        distributions.len(),
    )?;
    same_len("contributions", contributions.len(), "index", index.len())?;
    all_finite("contributions", contributions)?;
    all_finite("distributions", distributions)?;
    all_finite("index", index)?;
    finite("nav", nav)?;
    if contributions.iter().any(|c| *c < 0.0) {
        return Err(FinanceError::InvalidInput {
            what: "contributions",
            reason: "must be non-negative magnitudes",
        });
    }
    if distributions.iter().any(|d| *d < 0.0) {
        return Err(FinanceError::InvalidInput {
            what: "distributions",
            reason: "must be non-negative magnitudes",
        });
    }
    if index.iter().any(|i| *i <= 0.0) {
        return Err(FinanceError::InvalidInput {
            what: "index",
            reason: "levels must be positive",
        });
    }
    let last = index[index.len() - 1];
    index.iter().map(|i| output("index", last / i)).collect()
}

/// Kaplan-Schoar public market equivalent.
///
/// Each cash flow is carried to the final period `T` at the public index's
/// return, and the final NAV is taken as-is:
///
/// `KS-PME = (Σ_t dist_t · I_T/I_t + NAV_T) / (Σ_t call_t · I_T/I_t)`
///
/// Above 1 the fund beat the index. `contributions`, `distributions` and
/// `index` are aligned per period (non-negative amounts, positive levels).
///
/// ```rust
/// use finance_rs::fund::ks_pme;
/// // 100 called when the index is 100; 150 returned when it is 120.
/// let pme = ks_pme(&[100.0, 0.0], &[0.0, 150.0], 0.0, &[100.0, 120.0])?;
/// assert!((pme - 1.25).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for empty series,
/// [`FinanceError::LengthMismatch`] for unaligned series,
/// [`FinanceError::NonFinite`] / [`FinanceError::InvalidInput`] for bad
/// values, [`FinanceError::Undefined`] when there are no contributions.
pub fn ks_pme(
    contributions: &[f64],
    distributions: &[f64],
    nav: f64,
    index: &[f64],
) -> Result<f64> {
    let g = pme_factors(contributions, distributions, nav, index)?;
    let fv = |xs: &[f64]| xs.iter().zip(&g).map(|(x, g)| x * g).sum::<f64>();
    div("contributions", fv(distributions) + nav, fv(contributions))
}

/// Direct alpha (Gredil, Griffiths and Stucke, 2014), annualized.
///
/// Compound every net LP cash flow to the final period at the index's
/// return, `f_t = (dist_t − call_t) · I_T/I_t` with the NAV added at `T`,
/// then take the periodic IRR `a` of `f` and annualize it:
///
/// `direct_alpha = (1 + a)^periods_per_year − 1`
///
/// Zero means the fund matched the index exactly.
///
/// ```rust
/// use finance_rs::fund::direct_alpha;
/// // Index flat, fund doubles in one year: alpha is the fund's own 100%.
/// let a = direct_alpha(&[100.0, 0.0], &[0.0, 200.0], 0.0, &[50.0, 50.0], 1.0)?;
/// assert!((a - 1.0).abs() < 1e-10);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`ks_pme`] (series need at least 2 periods), plus
/// [`FinanceError::InvalidInput`] for a non-positive `periods_per_year` and
/// [`FinanceError::NoSignChange`] / [`FinanceError::NoSolution`] when the
/// compounded flows have no IRR.
pub fn direct_alpha(
    contributions: &[f64],
    distributions: &[f64],
    nav: f64,
    index: &[f64],
    periods_per_year: f64,
) -> Result<f64> {
    finite("periods_per_year", periods_per_year)?;
    if periods_per_year <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "periods_per_year",
            reason: "must be positive",
        });
    }
    min_len("contributions", contributions, 2)?;
    let g = pme_factors(contributions, distributions, nav, index)?;
    let mut flows: Vec<f64> = distributions
        .iter()
        .zip(contributions)
        .zip(&g)
        .map(|((d, c), g)| (d - c) * g)
        .collect();
    // `pme_factors` requires at least one contribution, so this flow exists.
    let last = flows.len() - 1;
    flows[last] += nav;
    all_finite("flows", &flows).map_err(|_| FinanceError::Overflow { what: "flows" })?;
    let a = irr(&flows)?;
    output("direct_alpha", (1.0 + a).powf(periods_per_year) - 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiples_identity_and_errors() -> Result<()> {
        let m = multiples(250.0, 120.0, 310.0)?;
        assert!((m.tvpi - 430.0 / 250.0).abs() < 1e-12);
        assert!((m.tvpi - m.dpi - m.rvpi).abs() < 1e-12);
        assert_eq!(
            tvpi(0.0, 1.0, 1.0),
            Err(FinanceError::Undefined { what: "paid_in" })
        );
        assert_eq!(pic(25.0, 100.0)?, 0.25);
        assert!(matches!(
            dpi(-1.0, 1.0),
            Err(FinanceError::InvalidInput { .. })
        ));
        assert!(matches!(
            rvpi(1.0, f64::NAN),
            Err(FinanceError::NonFinite { .. })
        ));
        Ok(())
    }

    #[test]
    fn ks_pme_matches_hand_calc() -> Result<()> {
        // Calls of 100 at I=100 and 50 at I=110; dist 80 at I=120; NAV 120 at I=132.
        let c = [100.0, 50.0, 0.0, 0.0];
        let d = [0.0, 0.0, 80.0, 0.0];
        let i = [100.0, 110.0, 120.0, 132.0];
        let num = 80.0 * 132.0 / 120.0 + 120.0;
        let den = 100.0 * 1.32 + 50.0 * 1.2;
        assert!((ks_pme(&c, &d, 120.0, &i)? - num / den).abs() < 1e-12);
        Ok(())
    }

    #[test]
    fn ks_pme_is_one_when_fund_tracks_index() -> Result<()> {
        // Fund value follows the index exactly: 100 in at 100, out at 150 -> 150.
        let contributions = [100.0, 0.0, 0.0];
        let distributions = [0.0, 60.0, 0.0];
        let index = [100.0, 120.0, 150.0];
        let pme_result = ks_pme(&contributions, &distributions, 108.0, &index);
        let pme = pme_result?;
        // 60 at I=120 carried to 150 is 75; plus NAV 75 tracking -> 150 / 150.
        assert!((pme - (75.0 + 108.0) / 150.0).abs() < 1e-12);
        let tracking_result = ks_pme(&contributions, &distributions, 75.0, &index);
        let tracks = tracking_result?;
        assert!((tracks - 1.0).abs() < 1e-12);
        Ok(())
    }

    #[test]
    fn pme_validates_distribution_and_series_inputs() {
        assert!(pme_factors(&[-1.0], &[0.0], 0.0, &[1.0]).is_err());
        assert!(pme_factors(&[1.0], &[-1.0], 0.0, &[1.0]).is_err());
        assert!(pme_factors(&[1.0], &[0.0], 0.0, &[0.0]).is_err());
        assert!(pme_factors(&[], &[], 0.0, &[]).is_err());
    }

    #[test]
    fn pme_input_errors() {
        assert!(matches!(
            ks_pme(&[1.0], &[1.0, 2.0], 0.0, &[1.0]),
            Err(FinanceError::LengthMismatch { .. })
        ));
        assert!(matches!(
            ks_pme(&[1.0], &[1.0], 0.0, &[0.0]),
            Err(FinanceError::InvalidInput { what: "index", .. })
        ));
        assert!(matches!(
            ks_pme(&[0.0], &[1.0], 0.0, &[1.0]),
            Err(FinanceError::Undefined { .. })
        ));
        assert!(matches!(
            ks_pme(&[], &[], 0.0, &[]),
            Err(FinanceError::TooShort { .. })
        ));
        assert!(matches!(
            direct_alpha(&[1.0, 0.0], &[0.0, 2.0], 0.0, &[1.0, 1.0], 0.0),
            Err(FinanceError::InvalidInput { .. })
        ));
        assert!(matches!(
            direct_alpha(&[0.0, 0.0], &[10.0, 0.0], 0.0, &[1.0, 1e308], 1.0),
            Err(FinanceError::Overflow { what: "flows" })
        ));
    }

    #[test]
    fn direct_alpha_zero_when_tracking_index() -> Result<()> {
        let contributions = [100.0, 0.0, 0.0];
        let distributions = [0.0, 60.0, 0.0];
        let index = [100.0, 120.0, 150.0];
        let alpha_result = direct_alpha(&contributions, &distributions, 75.0, &index, 4.0);
        let a = alpha_result?;
        assert!(a.abs() < 1e-10, "{a}");
        Ok(())
    }

    #[test]
    fn direct_alpha_annualizes() -> Result<()> {
        // Fund +21% over two quarters on a flat index: 10% per quarter.
        let a = direct_alpha(&[100.0, 0.0, 0.0], &[0.0, 0.0, 121.0], 0.0, &[1.0; 3], 4.0)?;
        assert!((a - (1.1f64.powi(4) - 1.0)).abs() < 1e-10);
        Ok(())
    }
}
