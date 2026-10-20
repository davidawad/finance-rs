//! Private mortgage insurance with automatic termination at an LTV threshold.

use super::Schedule;
use crate::error::{div, finite, FinanceError, Result};

/// PMI terms: an annual premium on the original loan amount that stops once
/// the loan-to-value ratio reaches `drop_ltv`.
///
/// ```rust
/// use finance_rs::amortization::Pmi;
/// let pmi = Pmi { home_value: 200_000.0, annual_rate: 0.006, drop_ltv: 0.8, periods_per_year: 12 };
/// assert_eq!(pmi.home_value * pmi.drop_ltv, 160_000.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Pmi {
    /// Original home value (purchase price or appraisal) the LTV is measured against.
    pub home_value: f64,
    /// Annual premium as a fraction of the original loan amount (e.g. `0.005`).
    pub annual_rate: f64,
    /// LTV at which PMI terminates, e.g. `0.78` (US Homeowners Protection Act
    /// automatic termination) or `0.80` (borrower-requested cancellation).
    pub drop_ltv: f64,
    /// Payment periods per year (12 for monthly).
    pub periods_per_year: u32,
}

impl Pmi {
    /// Monthly-payment PMI with automatic termination at 78% LTV.
    ///
    /// ```rust
    /// use finance_rs::amortization::Pmi;
    /// let p = Pmi::monthly(400_000.0, 0.006);
    /// assert_eq!(p.drop_ltv, 0.78);
    /// assert_eq!(p.periods_per_year, 12);
    /// ```
    #[must_use]
    pub fn monthly(home_value: f64, annual_rate: f64) -> Self {
        Self {
            home_value,
            annual_rate,
            drop_ltv: 0.78,
            periods_per_year: 12,
        }
    }
}

/// Charge PMI on `schedule` in place and return the number of periods charged.
///
/// Period `k` is charged `annual_rate · principal / periods_per_year` while its
/// opening balance satisfies `B_{k−1} / home_value > drop_ltv` (with
/// `B_0 = principal`); once the balance reaches the threshold PMI stops for
/// good. The balance used is the schedule's own (so extra principal brings
/// the drop-off forward). Any previous PMI on the schedule is replaced.
///
/// ```rust
/// use finance_rs::amortization::{apply_pmi, level_payment, Pmi};
/// // 95% LTV: 380,000 on a 400,000 home at 6.5%, PMI 0.6%/yr = 190/month.
/// let mut s = level_payment(380_000.0, 0.065 / 12.0, 360)?;
/// let months = apply_pmi(&mut s, &Pmi::monthly(400_000.0, 0.006))?;
/// assert!((s.rows[0].pmi - 190.0).abs() < 1e-9);
/// assert_eq!(s.rows[months as usize].pmi, 0.0);
/// // PMI ends the month after the balance first reaches 78% LTV (312,000).
/// assert!(s.rows[months as usize - 1].balance <= 312_000.0);
/// assert!(s.rows[months as usize - 2].balance > 312_000.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for a non-positive `home_value` or
/// `periods_per_year`, a negative `annual_rate`, or a `drop_ltv` outside
/// `[0, 1]`; [`FinanceError::NonFinite`] for non-finite terms.
pub fn apply_pmi(schedule: &mut Schedule, pmi: &Pmi) -> Result<u32> {
    finite("home_value", pmi.home_value)?;
    finite("annual_rate", pmi.annual_rate)?;
    finite("drop_ltv", pmi.drop_ltv)?;
    let bad = |what, reason| Err(FinanceError::InvalidInput { what, reason });
    if pmi.home_value <= 0.0 {
        return bad("home_value", "must be positive");
    }
    if pmi.annual_rate < 0.0 {
        return bad("annual_rate", "must be non-negative");
    }
    if !(0.0..=1.0).contains(&pmi.drop_ltv) {
        return bad("drop_ltv", "must be between 0 and 1");
    }
    if pmi.periods_per_year == 0 {
        return bad("periods_per_year", "must be at least 1");
    }
    let premium = div(
        "periods_per_year",
        pmi.annual_rate * schedule.principal,
        f64::from(pmi.periods_per_year),
    )?;
    let threshold = pmi.drop_ltv * pmi.home_value;
    let mut opening = schedule.principal;
    let mut charged = 0;
    let mut active = true;
    for row in &mut schedule.rows {
        active = active && opening > threshold;
        row.pmi = if active { premium } else { 0.0 };
        if active {
            charged += 1;
        }
        opening = row.balance;
    }
    Ok(charged)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amortization::{level_payment, level_payment_with_extra};

    #[test]
    fn no_pmi_below_threshold() -> Result<()> {
        let mut s = level_payment(300_000.0, 0.005, 360)?;
        assert_eq!(apply_pmi(&mut s, &Pmi::monthly(400_000.0, 0.005))?, 0);
        assert_eq!(s.summary().total_pmi, 0.0);
        Ok(())
    }

    #[test]
    fn extra_principal_drops_pmi_sooner_and_totals_include_it() -> Result<()> {
        let pmi = Pmi::monthly(400_000.0, 0.006);
        let mut base = level_payment(380_000.0, 0.065 / 12.0, 360)?;
        let mut fast = level_payment_with_extra(380_000.0, 0.065 / 12.0, 360, &[500.0; 360])?;
        let a = apply_pmi(&mut base, &pmi)?;
        let b = apply_pmi(&mut fast, &pmi)?;
        assert!(b < a);
        let sum = base.summary();
        assert!((sum.total_pmi - 190.0 * f64::from(a)).abs() < 1e-6);
        assert!((sum.total_paid - sum.total_payment - sum.total_pmi).abs() < 1e-6);
        // Re-applying replaces rather than accumulates.
        assert_eq!(apply_pmi(&mut base, &pmi)?, a);
        assert!((base.summary().total_pmi - sum.total_pmi).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn rejects_bad_terms() -> Result<()> {
        let mut s = level_payment(100.0, 0.01, 12)?;
        let ok = Pmi::monthly(120.0, 0.01);
        assert!(apply_pmi(
            &mut s,
            &Pmi {
                home_value: 0.0,
                ..ok
            }
        )
        .is_err());
        assert!(apply_pmi(
            &mut s,
            &Pmi {
                annual_rate: -0.1,
                ..ok
            }
        )
        .is_err());
        assert!(apply_pmi(
            &mut s,
            &Pmi {
                drop_ltv: 1.5,
                ..ok
            }
        )
        .is_err());
        assert!(apply_pmi(
            &mut s,
            &Pmi {
                periods_per_year: 0,
                ..ok
            }
        )
        .is_err());
        assert!(apply_pmi(
            &mut s,
            &Pmi {
                home_value: f64::NAN,
                ..ok
            }
        )
        .is_err());
        Ok(())
    }
}
