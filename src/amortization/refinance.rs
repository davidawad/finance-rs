//! Refinancing: switch to a new rate and term mid-schedule, and compare.

use super::{amortize, check_loan, level, level_payment, Schedule};
use crate::error::{finite, FinanceError, Result};

/// Terms of a refinance applied to a level-payment loan.
///
/// ```rust
/// use finance_rs::amortization::Refinance;
/// let terms = Refinance { at_period: 36, new_rate: 0.05 / 12.0, new_nper: 324, closing_costs: 2_000.0, roll_in: true };
/// assert!(terms.roll_in && terms.at_period < terms.new_nper);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Refinance {
    /// Payments made on the original loan before refinancing (`>= 1`, `< nper`).
    pub at_period: u32,
    /// Periodic rate of the new loan.
    pub new_rate: f64,
    /// Term of the new loan in periods.
    pub new_nper: u32,
    /// Closing costs of the new loan.
    pub closing_costs: f64,
    /// `true` to add the closing costs to the new loan balance, `false` to pay
    /// them in cash.
    pub roll_in: bool,
}

/// How a refinance compares to keeping the original loan.
///
/// ```rust
/// use finance_rs::amortization::{compare_refinance, Refinance, RefinanceComparison};
/// let terms = Refinance { at_period: 36, new_rate: 0.05 / 12.0, new_nper: 324, closing_costs: 0.0, roll_in: false };
/// let comparison: RefinanceComparison = compare_refinance(200_000.0, 0.06 / 12.0, 360, &terms)?;
/// assert_eq!(comparison.breakeven_periods, Some(0));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct RefinanceComparison {
    /// Level payment of the original loan.
    pub old_payment: f64,
    /// Level payment of the new loan.
    pub new_payment: f64,
    /// `old_payment − new_payment` (negative when the refinance costs more per period).
    pub payment_savings: f64,
    /// Total interest keeping the original loan.
    pub interest_without: f64,
    /// Total interest with the refinance (original rows plus new loan).
    pub interest_with: f64,
    /// `interest_without − interest_with`.
    pub interest_saved: f64,
    /// Total paid keeping the original loan.
    pub total_paid_without: f64,
    /// Total paid with the refinance, including closing costs paid in cash.
    pub total_paid_with: f64,
    /// Periods after the refinance until cumulative payment savings cover the
    /// closing costs, `ceil(closing_costs / payment_savings)`; `None` when the
    /// payment does not fall or breakeven would come after the new term ends.
    pub breakeven_periods: Option<u32>,
}

/// Schedule of a level-payment loan refinanced after `refi.at_period` payments.
///
/// Rows `1..=at_period` follow the original loan. The new loan's principal is
/// `B_at + closing_costs` when rolled in (else `B_at`), amortized at
/// `new_rate` over `new_nper` with payment `−pmt(new_rate, new_nper, B', 0, End)`;
/// its rows continue the period numbering. The schedule's `principal` stays
/// the original amount, so `Σ principal = principal + rolled-in costs`.
///
/// ```rust
/// use finance_rs::amortization::{refinance, Refinance};
/// let refi = Refinance { at_period: 60, new_rate: 0.05 / 12.0, new_nper: 300,
///                        closing_costs: 4_000.0, roll_in: true };
/// let s = refinance(300_000.0, 0.07 / 12.0, 360, &refi)?;
/// assert_eq!(s.rows.len(), 360);
/// assert!(s.rows[60].payment < s.rows[59].payment);
/// assert_eq!(s.final_balance(), 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for `at_period` outside `1..nper`, a
/// negative `closing_costs`, or bad loan terms (as
/// [`super::level_payment`]); [`FinanceError::NonFinite`] for non-finite input.
pub fn refinance(principal: f64, rate: f64, nper: u32, refi: &Refinance) -> Result<Schedule> {
    check_loan(principal, rate, nper)?;
    finite("closing_costs", refi.closing_costs)?;
    if refi.at_period == 0 || refi.at_period >= nper {
        return Err(FinanceError::InvalidInput {
            what: "at_period",
            reason: "must be between 1 and nper - 1",
        });
    }
    if refi.closing_costs < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "closing_costs",
            reason: "must be non-negative",
        });
    }
    let mut s = level_payment(principal, rate, nper)?;
    s.rows.truncate(refi.at_period as usize);
    let mut balance = s.final_balance();
    if refi.roll_in {
        balance += refi.closing_costs;
    }
    check_loan(balance, refi.new_rate, refi.new_nper)?;
    let payment = level(refi.new_rate, refi.new_nper, balance)?;
    amortize(
        &mut s.rows,
        &mut balance,
        refi.new_rate,
        payment,
        refi.new_nper,
        &[],
    );
    Ok(s)
}

/// Compare a refinance against keeping the original level-payment loan.
///
/// `payment_savings = old_payment − new_payment`;
/// `interest_saved = Σ interest(original) − Σ interest(refinanced)`;
/// `breakeven_periods = ceil(closing_costs / payment_savings)` (`0` with no
/// closing costs), the classic months-to-recoup rule.
///
/// ```rust
/// use finance_rs::amortization::{compare_refinance, Refinance};
/// let refi = Refinance { at_period: 36, new_rate: 0.055 / 12.0, new_nper: 324,
///                        closing_costs: 5_000.0, roll_in: false };
/// let c = compare_refinance(400_000.0, 0.075 / 12.0, 360, &refi)?;
/// assert!(c.payment_savings > 0.0 && c.interest_saved > 0.0);
/// let months = c.breakeven_periods.unwrap_or(0);
/// assert_eq!(months, (5_000.0 / c.payment_savings).ceil() as u32);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`refinance`].
pub fn compare_refinance(
    principal: f64,
    rate: f64,
    nper: u32,
    refi: &Refinance,
) -> Result<RefinanceComparison> {
    let with = refinance(principal, rate, nper, refi)?;
    let without = level_payment(principal, rate, nper)?;
    let old_payment = without.rows[0].payment;
    let new_payment = with
        .rows
        .get(refi.at_period as usize)
        .map_or(0.0, |r| r.payment);
    let payment_savings = old_payment - new_payment;
    let cash_costs = if refi.roll_in {
        0.0
    } else {
        refi.closing_costs
    };
    let interest_without = without.total_interest();
    let interest_with = with.total_interest();
    let breakeven_periods = if refi.closing_costs == 0.0 {
        Some(0)
    } else if payment_savings > 0.0 {
        let months = (refi.closing_costs / payment_savings).ceil();
        // `months` is a positive integer below u32::MAX here, so the cast is exact.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        (months <= f64::from(refi.new_nper)).then_some(months as u32)
    } else {
        None
    };
    Ok(RefinanceComparison {
        old_payment,
        new_payment,
        payment_savings,
        interest_without,
        interest_with,
        interest_saved: interest_without - interest_with,
        total_paid_without: without.total_paid(),
        total_paid_with: with.total_paid() + cash_costs,
        breakeven_periods,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // type-strength: allow(bool-param) test helper mirroring the `roll_in` field
    fn refi(roll_in: bool, closing_costs: f64) -> Refinance {
        Refinance {
            at_period: 60,
            new_rate: 0.05 / 12.0,
            new_nper: 360,
            closing_costs,
            roll_in,
        }
    }

    #[test]
    fn rolled_in_costs_join_the_balance() -> Result<()> {
        let base = level_payment(300_000.0, 0.07 / 12.0, 360)?;
        let s = refinance(300_000.0, 0.07 / 12.0, 360, &refi(true, 4_000.0))?;
        assert_eq!(s.rows.len(), 420);
        assert_eq!(s.rows[..60], base.rows[..60]);
        let sum = s.summary();
        assert!((sum.total_principal - 304_000.0).abs() < 1e-6);
        let cash = refinance(300_000.0, 0.07 / 12.0, 360, &refi(false, 4_000.0))?;
        assert!(cash.rows[60].payment < s.rows[60].payment);
        assert!((cash.summary().total_principal - 300_000.0).abs() < 1e-6);
        Ok(())
    }

    #[test]
    fn comparison_and_breakeven() -> Result<()> {
        assert!(
            compare_refinance(300_000.0, 0.07 / 12.0, 360, &refi(true, 4_000.0))?
                .total_paid_with
                .is_finite()
        );
        let c = compare_refinance(300_000.0, 0.07 / 12.0, 360, &refi(false, 4_000.0))?;
        assert!(c.payment_savings > 0.0);
        let want = (4_000.0 / c.payment_savings).ceil();
        assert_eq!(c.breakeven_periods.map(f64::from), Some(want));
        assert!(c.total_paid_with > c.total_paid_without - 1e9);
        assert_eq!(
            compare_refinance(300_000.0, 0.07 / 12.0, 360, &refi(false, 0.0))?.breakeven_periods,
            Some(0)
        );
        // A higher new rate never breaks even.
        let worse = Refinance {
            new_rate: 0.09 / 12.0,
            ..refi(false, 1_000.0)
        };
        let c = compare_refinance(300_000.0, 0.07 / 12.0, 360, &worse)?;
        assert!(c.payment_savings < 0.0 && c.breakeven_periods.is_none());
        Ok(())
    }

    #[test]
    fn rejects_bad_terms() {
        let p = (1_000.0, 0.01, 12);
        assert!(refinance(
            p.0,
            p.1,
            p.2,
            &Refinance {
                at_period: 0,
                ..refi(true, 0.0)
            }
        )
        .is_err());
        assert!(refinance(
            p.0,
            p.1,
            p.2,
            &Refinance {
                at_period: 12,
                ..refi(true, 0.0)
            }
        )
        .is_err());
        assert!(refinance(
            p.0,
            p.1,
            p.2,
            &Refinance {
                at_period: 3,
                ..refi(true, -1.0)
            }
        )
        .is_err());
        assert!(refinance(
            p.0,
            p.1,
            p.2,
            &Refinance {
                at_period: 3,
                new_nper: 0,
                ..refi(true, 0.0)
            }
        )
        .is_err());
    }
}
