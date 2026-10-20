use super::{CdsInstrument, CreditCurve, HazardNode};
use crate::{Date, FinanceError, Result, Weekday};

/// Present value of CDS legs, on a per-notional basis.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CdsValuation {
    /// Protection leg present value per unit notional.
    pub protection_leg: f64,
    /// Risky premium annuity, in year-equivalents per unit notional.
    pub risky_annuity: f64,
    /// Running premium leg present value per unit notional.
    pub premium_leg: f64,
    /// Clean buyer protection value before settlement accrued rebate/upfront.
    pub clean_value: f64,
    /// Dirty buyer value, including discounted settlement accrued rebate and
    /// the contract's upfront payment, per unit notional.
    pub dirty_value: f64,
    /// Par spread in annual decimal.
    pub par_spread: f64,
}

/// Spread and discount sensitivities for a CDS position, in currency units.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CdsRisk {
    /// Protection buyer PV after upfront.
    pub present_value: f64,
    /// Approximate PV change for a 1 bp wider credit spread.
    pub cs01: f64,
    /// PV change for a 1 bp higher discount rate.
    pub dv01: f64,
    /// PV change for a one percentage-point higher recovery assumption.
    pub recovery_sensitivity: f64,
}

fn validate_discount(discount_rate: f64) -> Result<()> {
    if !discount_rate.is_finite() {
        return Err(FinanceError::NonFinite {
            what: "discount_rate",
        });
    }
    if discount_rate <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "discount_rate",
            reason: "must exceed -1",
        });
    }
    Ok(())
}

#[allow(clippy::cast_precision_loss)] // Financial discounting uses a bounded day count.
fn discount(rate: f64, days: i64) -> f64 {
    (-rate * days as f64 / 365.0).exp()
}

/// ISDA CDS cash settlement is three business days after trade date. The
/// standard model's non-JPY calendar treats weekdays as business days.
fn cash_settlement_days(date: Date) -> Result<i64> {
    let mut cursor = date;
    let mut business_days = 0;
    while business_days < 3 {
        cursor = cursor.add_days(1)?;
        if !matches!(cursor.weekday(), Weekday::Saturday | Weekday::Sunday) {
            business_days += 1;
        }
    }
    debug_assert_eq!(business_days, 3);
    Ok(cursor.days_since(date))
}

fn settlement_discount_factor(date: Date, discount_rate: f64) -> Result<f64> {
    Ok(discount(discount_rate, cash_settlement_days(date)?))
}

fn accrued_rebate_factor(date: Date, discount_rate: f64) -> Result<f64> {
    Ok(settlement_discount_factor(date, discount_rate)? / 360.0)
}

pub(crate) fn par_spread_for_nodes(
    valuation_date: Date,
    maturity: Date,
    discount_rate: f64,
    recovery: f64,
    nodes: &[HazardNode],
) -> Result<f64> {
    let curve = CreditCurve {
        valuation_date,
        nodes: nodes.to_vec(),
    };
    let (protection, annuity) =
        super::pricing_legs::legs(valuation_date, maturity, discount_rate, recovery, &curve)?;
    Ok(protection / (annuity - accrued_rebate_factor(valuation_date, discount_rate)?))
}

impl CdsInstrument {
    /// Value legs and net buyer PV under a calibrated credit curve.
    ///
    /// # Errors
    /// Returns an error for an invalid instrument, discount rate, curve date,
    /// settlement date, or CDS leg calculation.
    pub fn value(&self, curve: &CreditCurve, discount_rate: f64) -> Result<CdsValuation> {
        self.validate()?;
        validate_discount(discount_rate)?;
        if curve.valuation_date != self.effective {
            return Err(FinanceError::InvalidInput {
                what: "curve",
                reason: "valuation date must equal CDS effective date",
            });
        }
        let (protection, annuity) = super::pricing_legs::legs(
            self.effective,
            self.maturity,
            discount_rate,
            self.recovery,
            curve,
        )?;
        let premium = self.coupon * annuity;
        let clean_value = protection - premium;
        let rebate_factor = accrued_rebate_factor(self.effective, discount_rate)?;
        let accrued_rebate = self.coupon * rebate_factor;
        let settlement_df = settlement_discount_factor(self.effective, discount_rate)?;
        Ok(CdsValuation {
            protection_leg: protection,
            risky_annuity: annuity,
            premium_leg: premium,
            clean_value,
            dirty_value: clean_value + accrued_rebate
                - self.upfront / self.notional * settlement_df,
            par_spread: protection / (annuity - rebate_factor),
        })
    }

    /// Market-value change for 1 bp spread, 1 bp discount-rate, and 1% recovery bumps.
    ///
    /// # Errors
    /// Returns an error when the base or bumped valuation cannot be computed.
    pub fn risk(&self, curve: &CreditCurve, discount_rate: f64) -> Result<CdsRisk> {
        let base = self.value(curve, discount_rate)?.dirty_value * self.notional;
        let bump_h = 1e-4 / (1.0 - self.recovery);
        let bumped = CreditCurve {
            valuation_date: curve.valuation_date,
            nodes: curve
                .nodes
                .iter()
                .map(|n| HazardNode {
                    maturity: n.maturity,
                    hazard_rate: (n.hazard_rate + bump_h).min(10.0),
                })
                .collect(),
        };
        let spread_pv = self.value(&bumped, discount_rate)?.dirty_value * self.notional;
        let dv01_pv = self.value(curve, discount_rate + 1e-4)?.dirty_value * self.notional;
        let mut recovery_up = self.clone();
        recovery_up.recovery = (self.recovery + 0.01).min(0.999_999);
        let recovery_pv = recovery_up.value(curve, discount_rate)?.dirty_value * self.notional;
        Ok(CdsRisk {
            present_value: base,
            cs01: spread_pv - base,
            dv01: dv01_pv - base,
            recovery_sensitivity: recovery_pv - base,
        })
    }
}

/// Convert a running coupon to clean upfront using the CDS's risky annuity.
///
/// # Errors
/// Returns an error for invalid market spreads or when valuation fails.
pub fn upfront_from_running(
    instrument: &CdsInstrument,
    curve: &CreditCurve,
    discount_rate: f64,
    market_spread: f64,
) -> Result<f64> {
    if !market_spread.is_finite() {
        return Err(FinanceError::NonFinite {
            what: "market_spread",
        });
    }
    let settlement_df = settlement_discount_factor(instrument.effective, discount_rate)?;
    let value = instrument.value(curve, discount_rate)?;
    let clean_value = (market_spread - instrument.coupon)
        * (value.risky_annuity - accrued_rebate_factor(instrument.effective, discount_rate)?);
    Ok(clean_value * instrument.notional / settlement_df)
}

/// Convert a clean upfront amount to an equivalent running spread.
///
/// # Errors
/// Returns an error for invalid upfront or when valuation fails.
pub fn running_from_upfront(
    instrument: &CdsInstrument,
    curve: &CreditCurve,
    discount_rate: f64,
    upfront: f64,
) -> Result<f64> {
    if !upfront.is_finite() {
        return Err(FinanceError::NonFinite { what: "upfront" });
    }
    let settlement_df = settlement_discount_factor(instrument.effective, discount_rate)?;
    let annuity = instrument.value(curve, discount_rate)?.risky_annuity;
    let adjusted_annuity = annuity - accrued_rebate_factor(instrument.effective, discount_rate)?;
    Ok(instrument.coupon + upfront * settlement_df / (instrument.notional * adjusted_annuity))
}

/// Carry and roll-down over `days`, as changes in protection-buyer PV.
///
/// # Errors
/// Returns an error when the position cannot be rolled while preserving a
/// positive remaining tenor or a required valuation fails.
pub fn carry_and_roll(
    instrument: &CdsInstrument,
    curve: &CreditCurve,
    discount_rate: f64,
    days: u32,
) -> Result<(f64, f64)> {
    let base = instrument.value(curve, discount_rate)?.dirty_value * instrument.notional;
    let new_effective = instrument
        .effective
        .add_days(i32::try_from(days).map_err(|_| FinanceError::InvalidInput {
            what: "days",
            reason: "too large",
        })?)?;
    if new_effective >= instrument.maturity {
        return Err(FinanceError::InvalidInput {
            what: "days",
            reason: "must leave positive remaining tenor",
        });
    }
    let new_curve = CreditCurve {
        valuation_date: new_effective,
        nodes: curve
            .nodes
            .iter()
            .filter(|n| n.maturity > new_effective)
            .copied()
            .collect(),
    };
    let mut rolled = instrument.clone();
    rolled.effective = new_effective;
    let future = rolled.value(&new_curve, discount_rate)?.dirty_value * rolled.notional;
    let carry = -instrument.coupon * f64::from(days) / 360.0 * instrument.notional;
    let roll = future - base - carry;
    Ok((carry, roll))
}

/// CDS-bond basis: CDS spread minus bond z-spread / asset-swap spread.
///
/// # Errors
/// Returns an error when either spread is not finite.
pub fn basis(cds_spread: f64, bond_spread: f64) -> Result<f64> {
    if !cds_spread.is_finite() || !bond_spread.is_finite() {
        return Err(FinanceError::NonFinite { what: "spread" });
    }
    Ok(cds_spread - bond_spread)
}

/// Implied par spread from a curve for one maturity.
///
/// # Errors
/// Returns an error for an invalid maturity, discount rate, recovery, or CDS
/// leg calculation.
pub fn implied_spread(
    curve: &CreditCurve,
    maturity: Date,
    discount_rate: f64,
    recovery: f64,
) -> Result<f64> {
    if maturity <= curve.valuation_date {
        return Err(FinanceError::InvalidInput {
            what: "maturity",
            reason: "must follow curve date",
        });
    }
    validate_discount(discount_rate)?;
    if !(0.0..1.0).contains(&recovery) {
        return Err(FinanceError::InvalidInput {
            what: "recovery",
            reason: "must be in [0, 1)",
        });
    }
    let (protection, annuity) = super::pricing_legs::legs(
        curve.valuation_date,
        maturity,
        discount_rate,
        recovery,
        curve,
    )?;
    Ok(protection / (annuity - accrued_rebate_factor(curve.valuation_date, discount_rate)?))
}
