//! European (whole-fund) and American (deal-by-deal) distribution waterfalls.
//!
//! Every waterfall runs the same four tiers on the cash available:
//!
//! 1. **Return of capital** — 100% to the LP until contributed capital is
//!    returned.
//! 2. **Preferred return** — 100% to the LP until the hurdle amount is paid
//!    (computed up front by [`preferred_return`] or
//!    [`preferred_return_schedule`] and passed in as an amount).
//! 3. **GP catch-up** — the GP takes `catch_up` of each dollar until it holds
//!    `carry` of all profit distributed so far (preferred + catch-up).
//! 4. **Carried interest** — the rest splits `1 − carry` / `carry`.
//!
//! LP amounts include any GP commitment invested alongside the LPs; the GP
//! columns are the promote only. Tier amounts always sum to the distributable
//! cash.

use crate::error::{finite, output, FinanceError, Result};

/// How the preferred return accrues in [`preferred_return`]: the fund-level
/// name for the two bases waterfalls use. It converts into the general
/// [`crate::rates::Compounding`] (`Annual` is `Compounded(Annual)`), which
/// [`preferred_return`] also accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::Compounding>() > 0);
/// ```
pub enum Compounding {
    /// Compounded annually: `capital · ((1 + rate)^years − 1)` (fractional
    /// years compound fractionally).
    #[default]
    Annual,
    /// Simple interest: `capital · rate · years`.
    Simple,
}

impl From<Compounding> for crate::rates::Compounding {
    fn from(c: Compounding) -> Self {
        match c {
            Compounding::Annual => Self::Compounded(crate::rates::Frequency::Annual),
            Compounding::Simple => Self::Simple,
        }
    }
}

/// Preferred return owed on `capital` outstanding for `years` at the annual
/// hurdle `rate`: `capital · (compound factor − 1)` on any
/// [`crate::rates::Compounding`].
///
/// `Annual: pref = capital · ((1 + rate)^years − 1)`;
/// `Simple: pref = capital · rate · years`
///
/// ```rust
/// use finance_rs::fund::{preferred_return, Compounding};
/// let p = preferred_return(100.0, 0.08, 2.0, Compounding::Annual)?;
/// assert!((p - 16.64).abs() < 1e-9);
/// let s = preferred_return(100.0, 0.08, 2.0, Compounding::Simple)?;
/// assert!((s - 16.0).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NonFinite`] for non-finite input,
/// [`FinanceError::InvalidInput`] for a negative capital, rate or years,
/// [`FinanceError::Overflow`] for a non-finite result.
pub fn preferred_return(
    capital: f64,
    rate: f64,
    years: f64,
    compounding: impl Into<crate::rates::Compounding>,
) -> Result<f64> {
    non_negative("capital", capital)?;
    non_negative("rate", rate)?;
    non_negative("years", years)?;
    let pref = match compounding.into() {
        crate::rates::Compounding::Simple => capital * rate * years,
        c => capital * c.growth(rate, years)?,
    };
    output("preferred_return", pref)
}

/// What the LP is still owed: unreturned capital and unpaid preferred return.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::PrefBalance>() > 0);
/// ```
pub struct PrefBalance {
    /// Contributed capital not yet returned, `max(Σ c − Σ r, 0)`.
    pub capital: f64,
    /// Accrued preferred return not yet paid.
    pub preferred: f64,
}

/// Hurdle balance from a periodic schedule of contributions and prior
/// distributions to the LP.
///
/// Flows land at the end of each period; the hurdle account compounds once
/// per period at `rate_per_period` (use the annual hurdle for annual periods,
/// or `(1 + h)^(1/f) − 1` for `f` periods a year) on contributed capital net
/// of what has been returned, and is valued at the end of the last period:
///
/// `H = Σ_t (c_t − r_t) · (1 + rate)^(n−1−t)`,
/// `capital = max(Σ c − Σ r, 0)`, `preferred = max(H − capital, 0)`
///
/// ```rust
/// use finance_rs::fund::preferred_return_schedule;
/// // 100 in at t=0, 30 returned at t=1, valued at t=2 with an 8% hurdle.
/// let b = preferred_return_schedule(&[100.0, 0.0, 0.0], &[0.0, 30.0, 0.0], 0.08)?;
/// assert!((b.capital - 70.0).abs() < 1e-9);
/// assert!((b.preferred - (100.0 * 1.08f64.powi(2) - 30.0 * 1.08 - 70.0)).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for empty series,
/// [`FinanceError::LengthMismatch`] for unaligned series,
/// [`FinanceError::NonFinite`] / [`FinanceError::InvalidInput`] for
/// non-finite or negative values, [`FinanceError::Overflow`] on overflow.
pub fn preferred_return_schedule(
    contributions: &[f64],
    returned: &[f64],
    rate_per_period: f64,
) -> Result<PrefBalance> {
    use crate::error::{min_len, same_len};
    min_len("contributions", contributions, 1)?;
    same_len(
        "contributions",
        contributions.len(),
        "returned",
        returned.len(),
    )?;
    non_negative("rate_per_period", rate_per_period)?;
    let mut hurdle = 0.0;
    let mut net = 0.0;
    for (c, r) in contributions.iter().zip(returned) {
        non_negative("contributions", *c)?;
        non_negative("returned", *r)?;
        hurdle = hurdle * (1.0 + rate_per_period) + c - r;
        net += c - r;
    }
    let capital = output("capital", net.max(0.0))?;
    let preferred = output("preferred", (hurdle - capital).max(0.0))?;
    Ok(PrefBalance { capital, preferred })
}

/// Catch-up and carry percentages shared by both waterfalls.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::WaterfallTerms>() > 0);
/// ```
pub struct WaterfallTerms {
    /// GP share of each dollar in the catch-up tier, in `[0, 1]`: `1.0` is a
    /// full (100%) catch-up, `0.0` skips the tier. A share at or below
    /// `carry` never completes and takes all remaining cash at that split.
    pub catch_up: f64,
    /// Carried interest, in `[0, 1]` (e.g. `0.20` for an 80/20 split).
    pub carry: f64,
}

impl Default for WaterfallTerms {
    /// 100% catch-up and 20% carry.
    fn default() -> Self {
        Self {
            catch_up: 1.0,
            carry: 0.2,
        }
    }
}

/// LP and GP amounts paid in one tier.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::Tier>() > 0);
/// ```
pub struct Tier {
    /// Paid to the limited partners.
    pub lp: f64,
    /// Paid to the general partner.
    pub gp: f64,
}

impl Tier {
    fn add(self, other: Self) -> Self {
        Self {
            lp: self.lp + other.lp,
            gp: self.gp + other.gp,
        }
    }
}

/// Result of a waterfall: per-tier splits, totals and what the LP is still owed.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::Waterfall>() > 0);
/// ```
pub struct Waterfall {
    /// Cash run through the waterfall; equals `lp_total + gp_total`.
    pub distributable: f64,
    /// Tier 1: return of contributed capital (LP only).
    pub return_of_capital: Tier,
    /// Tier 2: preferred return (LP only).
    pub preferred: Tier,
    /// Tier 3: GP catch-up.
    pub catch_up: Tier,
    /// Tier 4: carried-interest split.
    pub carry: Tier,
    /// Sum of the LP column.
    pub lp_total: f64,
    /// Sum of the GP column.
    pub gp_total: f64,
    /// Capital still unreturned after this distribution.
    pub unreturned_capital: f64,
    /// Preferred return still unpaid after this distribution.
    pub unpaid_preferred: f64,
}

fn non_negative(what: &'static str, x: f64) -> Result<f64> {
    finite(what, x)?;
    if x < 0.0 {
        return Err(FinanceError::InvalidInput {
            what,
            reason: "must be non-negative",
        });
    }
    Ok(x)
}

fn check_terms(terms: WaterfallTerms) -> Result<()> {
    for (what, x) in [("catch_up", terms.catch_up), ("carry", terms.carry)] {
        if non_negative(what, x)? > 1.0 {
            return Err(FinanceError::InvalidInput {
                what,
                reason: "must be a fraction in [0, 1]",
            });
        }
    }
    Ok(())
}

/// Whole-fund (European) waterfall: the four tiers on the fund's aggregate
/// `distributable` cash, `capital` contributed and `preferred` hurdle amount.
///
/// `roc = min(D, capital)`, `pref = min(D − roc, preferred)`;
/// catch-up runs on `x = min(rest, carry · pref / (catch_up − carry))`
/// (all of `rest` when `catch_up ≤ carry`) with `gp = catch_up · x`;
/// the remainder `y` splits `gp = carry · y`, `lp = (1 − carry) · y`.
/// Once the catch-up completes the GP holds exactly `carry · (D − capital)`.
///
/// ```rust
/// use finance_rs::fund::{european_waterfall, WaterfallTerms};
/// // 100 in, 8% pref owed (8), 200 out; 100% catch-up, 20% carry.
/// let w = european_waterfall(200.0, 100.0, 8.0, WaterfallTerms::default())?;
/// assert!((w.catch_up.gp - 2.0).abs() < 1e-9);       // 20% of (8 + 2)
/// assert!((w.gp_total - 20.0).abs() < 1e-9);         // 20% of the 100 profit
/// assert!((w.lp_total + w.gp_total - 200.0).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NonFinite`] for non-finite input,
/// [`FinanceError::InvalidInput`] for negative amounts or percentages
/// outside `[0, 1]`.
pub fn european_waterfall(
    distributable: f64,
    capital: f64,
    preferred: f64,
    terms: WaterfallTerms,
) -> Result<Waterfall> {
    non_negative("distributable", distributable)?;
    non_negative("capital", capital)?;
    non_negative("preferred", preferred)?;
    check_terms(terms)?;
    let WaterfallTerms { catch_up, carry } = terms;

    let roc = distributable.min(capital);
    let mut rest = distributable - roc;
    let pref = rest.min(preferred);
    rest -= pref;

    let target = if catch_up == 0.0 {
        0.0
    } else if catch_up > carry {
        carry * pref / (catch_up - carry)
    } else {
        f64::INFINITY
    };
    let cu = rest.min(target);
    let cu_gp = catch_up * cu;
    rest -= cu;
    let carry_gp = carry * rest;

    let return_of_capital = Tier { lp: roc, gp: 0.0 };
    let preferred_tier = Tier { lp: pref, gp: 0.0 };
    let catch_up_tier = Tier {
        lp: cu - cu_gp,
        gp: cu_gp,
    };
    let carry_tier = Tier {
        lp: rest - carry_gp,
        gp: carry_gp,
    };
    Ok(Waterfall {
        distributable,
        return_of_capital,
        preferred: preferred_tier,
        catch_up: catch_up_tier,
        carry: carry_tier,
        lp_total: roc + pref + catch_up_tier.lp + carry_tier.lp,
        gp_total: cu_gp + carry_gp,
        unreturned_capital: capital - roc,
        unpaid_preferred: preferred - pref,
    })
}

/// One realized investment for [`american_waterfall`].
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::Deal>() > 0);
/// ```
pub struct Deal {
    /// Capital contributed to the deal.
    pub capital: f64,
    /// Preferred return owed on that capital (see [`preferred_return`]).
    pub preferred: f64,
    /// Proceeds realized from the deal.
    pub proceeds: f64,
}

/// What a deal-by-deal waterfall does with a losing deal's shortfall.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::Losses>() > 0);
/// ```
pub enum Losses {
    /// Each deal stands alone: a loser's unreturned capital is not recouped.
    #[default]
    Isolated,
    /// Unreturned capital and unpaid preferred return roll into the next
    /// deal, so later winners make the LP whole before the GP earns carry.
    CarriedForward,
}

/// Result of a deal-by-deal waterfall.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::AmericanWaterfall>() > 0);
/// ```
pub struct AmericanWaterfall {
    /// One waterfall per deal, in order.
    pub deals: Vec<Waterfall>,
    /// Tier-by-tier sums over all deals; `unreturned_capital` and
    /// `unpaid_preferred` are measured against the deals' own totals.
    pub total: Waterfall,
    /// GP carry above what a whole-fund (European) waterfall on the same
    /// aggregates would pay: `max(gp_american − gp_european, 0)`. This is the
    /// amount a whole-fund clawback would return to the LPs; it is reported,
    /// not deducted.
    pub clawback: f64,
}

/// Deal-by-deal (American) waterfall: [`european_waterfall`]'s tiers applied
/// to each deal's own capital, preferred return and proceeds, in order.
///
/// With [`Losses::CarriedForward`], capital and preferred return a deal leaves
/// unpaid are added to the next deal's `capital` / `preferred` (carried
/// without further compounding), so later winners first make the LP whole on
/// earlier losers before the GP earns carry. With [`Losses::Isolated`],
/// each deal stands alone. Either way [`AmericanWaterfall::clawback`] reports the excess carry
/// relative to a whole-fund waterfall.
///
/// `total.tier = Σ_deals deal.tier`;
/// `clawback = max(total.gp_total − european(Σ proceeds, Σ capital, Σ pref).gp_total, 0)`
///
/// ```rust
/// use finance_rs::fund::{american_waterfall, Deal, Losses, WaterfallTerms};
/// let deals = [
///     Deal { capital: 100.0, preferred: 0.0, proceeds: 200.0 }, // +100
///     Deal { capital: 100.0, preferred: 0.0, proceeds: 0.0 },   // -100
/// ];
/// let w = american_waterfall(&deals, WaterfallTerms::default(), Losses::Isolated)?;
/// assert!((w.total.gp_total - 20.0).abs() < 1e-9); // carry on the winner
/// assert!((w.clawback - 20.0).abs() < 1e-9);       // the fund made no profit
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for no deals; otherwise as
/// [`european_waterfall`].
pub fn american_waterfall(
    deals: &[Deal],
    terms: WaterfallTerms,
    losses: Losses,
) -> Result<AmericanWaterfall> {
    if deals.is_empty() {
        return Err(FinanceError::TooShort {
            what: "deals",
            min: 1,
            got: 0,
        });
    }
    let mut out = Vec::with_capacity(deals.len());
    let mut total = Waterfall::default();
    let (mut cap_sum, mut pref_sum) = (0.0, 0.0);
    let (mut cap_owed, mut pref_owed) = (0.0, 0.0);
    for d in deals {
        non_negative("capital", d.capital)?;
        non_negative("preferred", d.preferred)?;
        let w = european_waterfall(
            d.proceeds,
            d.capital + cap_owed,
            d.preferred + pref_owed,
            terms,
        )?;
        if losses == Losses::CarriedForward {
            cap_owed = w.unreturned_capital;
            pref_owed = w.unpaid_preferred;
        }
        cap_sum += d.capital;
        pref_sum += d.preferred;
        total.distributable += w.distributable;
        total.return_of_capital = total.return_of_capital.add(w.return_of_capital);
        total.preferred = total.preferred.add(w.preferred);
        total.catch_up = total.catch_up.add(w.catch_up);
        total.carry = total.carry.add(w.carry);
        total.lp_total += w.lp_total;
        total.gp_total += w.gp_total;
        out.push(w);
    }
    total.distributable = output("distributable", total.distributable)?;
    total.unreturned_capital = (cap_sum - total.return_of_capital.lp).max(0.0);
    total.unpaid_preferred = (pref_sum - total.preferred.lp).max(0.0);
    let whole = european_waterfall(total.distributable, cap_sum, pref_sum, terms)?;
    let clawback = (total.gp_total - whole.gp_total).max(0.0);
    Ok(AmericanWaterfall {
        deals: out,
        total,
        clawback,
    })
}

#[cfg(test)]
#[path = "waterfall_tests.rs"]
mod tests;
