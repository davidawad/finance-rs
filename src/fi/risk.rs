//! Interest-rate sensitivities on a coupon date: Macaulay and modified
//! duration, convexity and DV01, all in years with respect to the nominal
//! annual yield.

use super::Bond;
use crate::error::{div, output, Result};

/// Risk measures for a period-end cash-flow series, useful when a consumer's
/// bond schedule uses a legacy input shape rather than [`Bond`].
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::cashflow_risk), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn cashflow_risk(
    cashflows: &[f64],
    periodic_rate: f64,
    frequency: u32,
    price: f64,
) -> Result<BondRisk> {
    if frequency == 0 {
        return Err(crate::FinanceError::InvalidInput {
            what: "frequency",
            reason: "must be positive",
        });
    }
    let computed_price = super::cashflow_price(cashflows, periodic_rate)?;
    let price = if price > 0.0 { price } else { computed_price };
    let mut factor = 1.0;
    let mut first = 0.0;
    let mut second = 0.0;
    for (index, amount) in cashflows.iter().enumerate() {
        let period = super::period_number(index)?;
        factor *= 1.0 + periodic_rate;
        let k = f64::from(period);
        let pv = amount / factor;
        first += k * pv;
        second += k * (k + 1.0) * pv;
    }
    let f = f64::from(frequency);
    let macaulay = div("price", first, price * f)?;
    let modified = output("modified_duration", macaulay / (1.0 + periodic_rate))?;
    let convexity_denominator = price * f * f * (1.0 + periodic_rate).powi(2);
    let convexity = div("price", second, convexity_denominator)?;
    let dv01 = output("dv01", modified * price * 1e-4)?;
    Ok(BondRisk {
        price,
        macaulay,
        modified,
        convexity,
        dv01,
    })
}

/// Price and every sensitivity at one yield.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fi::BondRisk>() > 0);
/// ```
pub struct BondRisk {
    /// Price on a coupon date, as [`Bond::price`].
    pub price: f64,
    /// Macaulay duration in years.
    pub macaulay: f64,
    /// Modified duration in years: `−(1/P)·dP/dy`.
    pub modified: f64,
    /// Convexity in years²: `(1/P)·d²P/dy²`.
    pub convexity: f64,
    /// Price fall for a one-basis-point rise in yield: `modified · P · 1e-4`.
    pub dv01: f64,
}

impl Bond {
    /// Price, durations, convexity and DV01 in one pass over the cash flows.
    ///
    /// With `i = y/f`, `t_k = k/f` and `PV_k = CF_k/(1+i)^k`:
    /// `Mac = Σ t_k·PV_k / P`, `Mod = Mac/(1+i)`,
    /// `Conv = Σ k(k+1)·PV_k / (P·f²·(1+i)²)`, `DV01 = Mod·P·0.0001`.
    ///
    /// ```rust
    /// use finance_rs::fi::Bond;
    /// let r = Bond::new(100.0, 0.08, 5, 1)?.risk(0.08)?;
    /// assert!((r.price - 100.0).abs() < 1e-10);
    /// assert!((r.macaulay - 4.312_127).abs() < 1e-6);
    /// assert!((r.dv01 - r.modified * r.price * 1e-4).abs() < 1e-15);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Bond::price`]; [`FinanceError::Undefined`] if the price
    /// underflows to zero.
    ///
    /// [`FinanceError::Undefined`]: crate::FinanceError::Undefined
    pub fn risk(&self, ytm: f64) -> Result<BondRisk> {
        let i = self.period_rate(ytm)?;
        let (p, s1, s2) = self.moments(i);
        let price = output("price", p)?;
        let f = f64::from(self.frequency);
        let macaulay = div("price", s1, p * f)?;
        let modified = output("modified_duration", macaulay / (1.0 + i))?;
        let convexity = div("price", s2, p * f * f * (1.0 + i) * (1.0 + i))?;
        let dv01 = output("dv01", modified * price * 1e-4)?;
        Ok(BondRisk {
            price,
            macaulay,
            modified,
            convexity,
            dv01,
        })
    }

    /// Macaulay duration: the present-value-weighted average time to each
    /// cash flow, in years.
    ///
    /// `Mac = (1/P) · Σ_k (k/f) · CF_k / (1+y/f)^k`
    ///
    /// ```rust
    /// use finance_rs::fi::Bond;
    /// // A zero-coupon bond's duration is its maturity.
    /// let d = Bond::new(100.0, 0.0, 20, 2)?.macaulay_duration(0.05)?;
    /// assert!((d - 10.0).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Bond::risk`].
    pub fn macaulay_duration(&self, ytm: f64) -> Result<f64> {
        Ok(self.risk(ytm)?.macaulay)
    }

    /// Modified duration: the percentage price change per unit yield change.
    ///
    /// `Mod = Mac / (1 + y/f) = −(1/P) · dP/dy`
    ///
    /// ```rust
    /// use finance_rs::fi::Bond;
    /// let d = Bond::new(100.0, 0.08, 5, 1)?.modified_duration(0.08)?;
    /// assert!((d - 4.312_127 / 1.08).abs() < 1e-6);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Bond::risk`].
    pub fn modified_duration(&self, ytm: f64) -> Result<f64> {
        Ok(self.risk(ytm)?.modified)
    }

    /// Convexity in years².
    ///
    /// `Conv = (1/P) · d²P/dy² = Σ_k k(k+1) · CF_k / (1+y/f)^(k+2) / (P · f²)`
    ///
    /// ```rust
    /// use finance_rs::fi::Bond;
    /// // Zero-coupon: n(n+1) / (f² (1+i)²).
    /// let c = Bond::new(100.0, 0.0, 10, 1)?.convexity(0.05)?;
    /// assert!((c - 110.0 / 1.05f64.powi(2)).abs() < 1e-10);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Bond::risk`].
    pub fn convexity(&self, ytm: f64) -> Result<f64> {
        Ok(self.risk(ytm)?.convexity)
    }

    /// Dollar value of a basis point (PV01): the price fall for a 1bp rise.
    ///
    /// `DV01 = Mod · P · 0.0001`
    ///
    /// ```rust
    /// use finance_rs::fi::Bond;
    /// let b = Bond::new(100.0, 0.05, 20, 2)?;
    /// let bump = b.price(0.04995)? - b.price(0.05005)?; // central 1bp
    /// assert!((b.dv01(0.05)? - bump).abs() < 1e-7);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Bond::risk`].
    pub fn dv01(&self, ytm: f64) -> Result<f64> {
        Ok(self.risk(ytm)?.dv01)
    }
}
