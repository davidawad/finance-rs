//! Loan amortization schedules: level payment (with optional extra
//! principal), interest-only, balloon, PMI drop-off and refinance.
//!
//! Rates are **periodic** (`annual / 12` for a monthly loan) and payments fall
//! at period end, matching [`crate::tvm::pmt`], [`crate::tvm::ipmt`] and
//! [`crate::tvm::ppmt`] with [`crate::tvm::When::End`]. Unlike the TVM
//! functions, schedule amounts are reported as positive magnitudes.
//!
//! Each period `k` with opening balance `B` charges `interest = B·r`; the
//! principal is `payment − interest` (plus any extra principal), and the
//! closing balance is `B − principal`. The last row always retires the loan:
//! its principal is clamped to the opening balance, so the final balance is
//! exactly `0`.
//!
//! ```
//! use finance_rs::amortization::level_payment;
//! let s = level_payment(300_000.0, 0.06 / 12.0, 360)?;
//! assert_eq!(s.rows.len(), 360);
//! assert_eq!(s.final_balance(), 0.0);
//! assert!((s.rows[0].payment - 1_798.65).abs() < 0.01);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

mod annual;
mod pmi;
mod refinance;
mod variants;

/// Annual interest-only schedule constructor.
///
/// ```
/// use finance_rs::amortization::interest_only_annual;
/// let (payments, balances) = interest_only_annual(100_000.0, 0.06, 2, 1)?;
/// assert_eq!(payments.len(), 2);
/// assert_eq!(balances.len(), 2);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub use annual::interest_only_annual;
/// PMI schedule adjustment and monthly premium type.
///
/// ```
/// use finance_rs::amortization::Pmi;
/// let pmi = Pmi::monthly(300_000.0, 0.006);
/// assert_eq!(pmi.periods_per_year, 12);
/// assert!(pmi.annual_rate > 0.0);
/// ```
pub use pmi::{apply_pmi, Pmi};
/// Refinance comparison inputs, outputs, and calculators.
///
/// ```
/// use finance_rs::amortization::{compare_refinance, refinance, Refinance};
/// let terms = Refinance { at_period: 36, new_rate: 0.05 / 12.0, new_nper: 324, closing_costs: 0.0, roll_in: false };
/// let comparison = compare_refinance(200_000.0, 0.06 / 12.0, 360, &terms)?;
/// let schedule = refinance(200_000.0, 0.06 / 12.0, 360, &terms)?;
/// assert_eq!(comparison.breakeven_periods, Some(0));
/// assert_eq!(schedule.final_balance(), 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub use refinance::{compare_refinance, refinance, Refinance, RefinanceComparison};
/// Balloon and interest-only schedule variants.
///
/// ```
/// use finance_rs::amortization::{balloon, interest_only};
/// let interest_only_schedule = interest_only(100_000.0, 0.005, 12, 240)?;
/// let balloon_schedule = balloon(100_000.0, 0.005, 240, 12)?;
/// assert_eq!(interest_only_schedule.rows.len(), 252);
/// assert_eq!(balloon_schedule.rows.len(), 12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub use variants::{balloon, interest_only};

use crate::error::{all_finite, finite, FinanceError, Result};
use crate::tvm::{pmt, When};

/// One period of an amortization schedule. All amounts are non-negative
/// magnitudes (for non-negative rates).
///
/// ```rust
/// use finance_rs::amortization::Row;
/// let row = Row { period: 1, payment: 100.0, interest: 10.0, principal: 90.0, balance: 10.0, ..Row::default() };
/// assert_eq!(row.payment, row.interest + row.principal);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Row {
    /// Period number, starting at 1.
    pub period: u32,
    /// Loan payment for the period: `interest + principal` (excludes PMI).
    pub payment: f64,
    /// Interest charged: opening balance times the periodic rate.
    pub interest: f64,
    /// Principal repaid, including [`Row::extra`].
    pub principal: f64,
    /// The part of [`Row::principal`] that was an extra (prepaid) amount.
    pub extra: f64,
    /// Private mortgage insurance premium charged this period (see [`apply_pmi`]).
    pub pmi: f64,
    /// Closing balance after the payment.
    pub balance: f64,
}

/// A full schedule: the amount originally borrowed plus one [`Row`] per period.
///
/// ```rust
/// use finance_rs::amortization::{level_payment, Schedule};
/// let schedule: Schedule = level_payment(1_200.0, 0.0, 12)?;
/// assert_eq!(schedule.rows.len(), 12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Schedule {
    /// Amount originally borrowed (opening balance of period 1).
    pub principal: f64,
    /// Periods in order; the last row's balance is exactly `0`.
    pub rows: Vec<Row>,
}

/// Totals over a [`Schedule`].
///
/// ```rust
/// use finance_rs::amortization::{level_payment, Summary};
/// let totals: Summary = level_payment(1_200.0, 0.0, 12)?.summary();
/// assert_eq!(totals.total_principal, 1_200.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Summary {
    /// Number of rows (periods until payoff).
    pub periods: usize,
    /// `Σ payment` (loan payments only).
    pub total_payment: f64,
    /// `Σ interest`.
    pub total_interest: f64,
    /// `Σ principal` (equals the amount financed).
    pub total_principal: f64,
    /// `Σ extra`.
    pub total_extra: f64,
    /// `Σ pmi`.
    pub total_pmi: f64,
    /// Everything the borrower paid: `Σ payment + Σ pmi`.
    pub total_paid: f64,
}

impl Schedule {
    /// Totals over every row: `Σ payment`, `Σ interest`, `Σ principal`,
    /// `Σ extra`, `Σ pmi`, and `total_paid = Σ payment + Σ pmi`.
    ///
    /// ```rust
    /// use finance_rs::amortization::level_payment;
    /// let s = level_payment(10_000.0, 0.01, 12)?.summary();
    /// assert!((s.total_principal - 10_000.0).abs() < 1e-9);
    /// assert!((s.total_payment - s.total_interest - s.total_principal).abs() < 1e-9);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn summary(&self) -> Summary {
        let mut s = Summary {
            periods: self.rows.len(),
            ..Summary::default()
        };
        for r in &self.rows {
            s.total_payment += r.payment;
            s.total_interest += r.interest;
            s.total_principal += r.principal;
            s.total_extra += r.extra;
            s.total_pmi += r.pmi;
        }
        s.total_paid = s.total_payment + s.total_pmi;
        s
    }

    /// Total interest over the schedule, `Σ interest`.
    ///
    /// ```rust
    /// use finance_rs::amortization::level_payment;
    /// let s = level_payment(1_200.0, 0.0, 12)?;
    /// assert_eq!(s.total_interest(), 0.0);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn total_interest(&self) -> f64 {
        self.rows.iter().map(|r| r.interest).sum()
    }

    /// Total paid over the schedule, `Σ (payment + pmi)`.
    ///
    /// ```rust
    /// use finance_rs::amortization::level_payment;
    /// let s = level_payment(1_200.0, 0.0, 12)?;
    /// assert!((s.total_paid() - 1_200.0).abs() < 1e-9);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn total_paid(&self) -> f64 {
        self.rows.iter().map(|r| r.payment + r.pmi).sum()
    }

    /// Closing balance of the last row (`0` for every schedule this module
    /// builds), or the principal for an empty schedule.
    ///
    /// ```rust
    /// use finance_rs::amortization::level_payment;
    /// assert_eq!(level_payment(5_000.0, 0.004, 60)?.final_balance(), 0.0);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn final_balance(&self) -> f64 {
        self.rows.last().map_or(self.principal, |r| r.balance)
    }
}

/// Level-payment (fully amortizing) schedule.
///
/// The payment is `−pmt(r, n, P, 0, End)` =
/// `P·r / (1 − (1+r)^−n)` (or `P/n` at `r = 0`); period `k` charges
/// `interest_k = B_{k−1}·r` and repays `principal_k = payment − interest_k`,
/// matching numpy-financial `ipmt`/`ppmt` up to sign. The last row's principal
/// is the remaining balance, so the schedule ends at exactly `0`.
///
/// ```rust
/// use finance_rs::amortization::level_payment;
/// use finance_rs::tvm::{ipmt, When};
/// let (p, r, n) = (200_000.0, 0.075 / 12.0, 180);
/// let s = level_payment(p, r, n)?;
/// assert!((s.rows[0].payment - 1_854.024_720_005_831_6).abs() < 1e-9);
/// let want = -ipmt(r, 10.0, f64::from(n), p, 0.0, When::End)?;
/// assert!((s.rows[9].interest - want).abs() < 1e-8);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for `principal <= 0`, `nper` outside
/// `1..=`[`MAX_PERIODS`] or `rate <= -1`; [`FinanceError::NonFinite`] for non-finite input.
pub fn level_payment(principal: f64, rate: f64, nper: u32) -> Result<Schedule> {
    level_payment_with_extra(principal, rate, nper, &[])
}

/// Level-payment schedule with extra principal prepayments.
///
/// `extra[k−1]` is added to the principal of period `k` (periods past the
/// slice get no extra). The scheduled payment stays `−pmt(r, n, P, 0, End)`;
/// row `k` pays `payment_k = level + extra_k`, and the loan stops as soon as
/// the balance is retired (the last row is clamped to the remaining balance),
/// so the schedule can be shorter than `nper`.
///
/// ```rust
/// use finance_rs::amortization::{level_payment, level_payment_with_extra};
/// let base = level_payment(300_000.0, 0.06 / 12.0, 360)?;
/// let fast = level_payment_with_extra(300_000.0, 0.06 / 12.0, 360, &[200.0; 360])?;
/// assert!(fast.rows.len() < base.rows.len());
/// assert!(fast.total_interest() < base.total_interest());
/// assert_eq!(fast.final_balance(), 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`level_payment`], plus [`FinanceError::InvalidInput`] for a negative
/// extra amount and [`FinanceError::NonFinite`] for a non-finite one.
pub fn level_payment_with_extra(
    principal: f64,
    rate: f64,
    nper: u32,
    extra: &[f64],
) -> Result<Schedule> {
    check_loan(principal, rate, nper)?;
    all_finite("extra", extra)?;
    if extra.iter().any(|x| *x < 0.0) {
        return Err(FinanceError::InvalidInput {
            what: "extra",
            reason: "extra principal must be non-negative",
        });
    }
    let payment = level(rate, nper, principal)?;
    let mut s = Schedule {
        principal,
        rows: Vec::with_capacity(nper as usize),
    };
    let mut balance = principal;
    amortize(&mut s.rows, &mut balance, rate, payment, nper, extra);
    Ok(s)
}

/// Longest schedule any builder will allocate (8,333 years of monthly rows).
///
/// ```rust
/// use finance_rs::amortization::MAX_PERIODS;
/// assert_eq!(MAX_PERIODS, 100_000);
/// ```
pub const MAX_PERIODS: u32 = 100_000;

/// Validate the loan terms shared by every schedule builder.
pub(super) fn check_loan(principal: f64, rate: f64, nper: u32) -> Result<()> {
    finite("principal", principal)?;
    finite("rate", rate)?;
    if principal <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "principal",
            reason: "must be positive",
        });
    }
    if rate <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "rate",
            reason: "must be greater than -1",
        });
    }
    if nper == 0 || nper > MAX_PERIODS {
        return Err(FinanceError::InvalidInput {
            what: "nper",
            reason: "must be within 1..=MAX_PERIODS",
        });
    }
    Ok(())
}

/// The positive level payment `−pmt(r, n, P, 0, End)`.
pub(super) fn level(rate: f64, nper: u32, principal: f64) -> Result<f64> {
    Ok(-pmt(rate, f64::from(nper), principal, 0.0, When::End)?)
}

/// Append up to `count` amortizing rows paying `payment` (+ `extra` indexed by
/// absolute period); the `count`-th row (or the first row that would overpay)
/// retires the remaining balance exactly.
pub(super) fn amortize(
    rows: &mut Vec<Row>,
    balance: &mut f64,
    rate: f64,
    payment: f64,
    count: u32,
    extra: &[f64],
) {
    for step in 1..=count {
        if *balance <= 0.0 {
            break;
        }
        let period = next_period(rows);
        let interest = *balance * rate;
        let scheduled = payment - interest;
        let want_extra = extra
            .get((period as usize).wrapping_sub(1))
            .copied()
            .unwrap_or(0.0);
        let mut principal = scheduled + want_extra;
        // Retire the loan on the last step, or when this payment would
        // overpay (also absorbs floating-point dust left by the level formula).
        let last = step == count || principal >= *balance * (1.0 - 1e-12);
        let (row_payment, row_extra) = if last {
            principal = *balance;
            (
                interest + principal,
                (principal - scheduled).clamp(0.0, want_extra),
            )
        } else {
            (payment + want_extra, want_extra)
        };
        *balance = if last { 0.0 } else { *balance - principal };
        rows.push(Row {
            period,
            payment: row_payment,
            interest,
            principal,
            extra: row_extra,
            pmi: 0.0,
            balance: *balance,
        });
    }
}

/// Period number of the next row.
pub(super) fn next_period(rows: &[Row]) -> u32 {
    rows.last().map_or(1, |r| r.period + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tvm::{ipmt, ppmt};

    #[test]
    fn matches_numpy_financial_rows() -> Result<()> {
        // numpy_financial.ipmt/ppmt(0.0824/12, per, 12, 2500)
        let (r, n, p) = (0.0824 / 12.0, 12, 2_500.0);
        let s = level_payment(p, r, n)?;
        assert!((s.rows[0].interest - 17.166_666_666_666_67).abs() < 1e-9);
        assert!((s.rows[0].principal - 200.581_923_686_782_77).abs() < 1e-9);
        assert!((s.rows[11].principal - 216.263_580_434_466_1).abs() < 1e-9);
        for row in &s.rows {
            let per = f64::from(row.period);
            let want_i = -ipmt(r, per, f64::from(n), p, 0.0, When::End)?;
            let want_p = -ppmt(r, per, f64::from(n), p, 0.0, When::End)?;
            assert!((row.interest - want_i).abs() < 1e-9, "{row:?}");
            assert!((row.principal - want_p).abs() < 1e-9, "{row:?}");
        }
        assert_eq!(s.final_balance(), 0.0);
        Ok(())
    }

    #[test]
    fn zero_rate_is_straight_line() -> Result<()> {
        let s = level_payment(1_200.0, 0.0, 12)?;
        assert!(s.rows.iter().all(|r| (r.principal - 100.0).abs() < 1e-9));
        assert_eq!(s.total_interest(), 0.0);
        Ok(())
    }

    #[test]
    fn single_period() -> Result<()> {
        let s = level_payment(1_000.0, 0.05, 1)?;
        assert_eq!(s.rows.len(), 1);
        assert!((s.rows[0].payment - 1_050.0).abs() < 1e-9);
        assert_eq!(s.final_balance(), 0.0);
        Ok(())
    }

    #[test]
    fn extra_principal_pays_off_early() -> Result<()> {
        let s = level_payment_with_extra(10_000.0, 0.01, 24, &[5_000.0])?;
        assert!(s.rows.len() < 24);
        assert!((s.rows[0].extra - 5_000.0).abs() < 1e-12);
        let sum = s.summary();
        assert!((sum.total_principal - 10_000.0).abs() < 1e-6);
        assert!((sum.total_payment - sum.total_interest - sum.total_principal).abs() < 1e-6);
        // A huge extra retires the loan in one period without overpaying.
        let one = level_payment_with_extra(10_000.0, 0.01, 24, &[1e9])?;
        assert_eq!(one.rows.len(), 1);
        assert!((one.rows[0].payment - 10_100.0).abs() < 1e-9);
        assert!(one.rows[0].extra < 10_000.0);
        Ok(())
    }

    #[test]
    fn rejects_bad_input() {
        assert!(level_payment(0.0, 0.01, 12).is_err());
        assert!(level_payment(100.0, -1.0, 12).is_err());
        assert!(level_payment(100.0, 0.01, 0).is_err());
        assert!(level_payment(f64::NAN, 0.01, 12).is_err());
        assert!(level_payment_with_extra(100.0, 0.01, 12, &[-1.0]).is_err());
        assert!(level_payment_with_extra(100.0, 0.01, 12, &[f64::INFINITY]).is_err());
    }
}
