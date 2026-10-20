//! Interest-only and balloon schedules.

use super::{amortize, check_loan, level, next_period, Row, Schedule};
use crate::error::{FinanceError, Result};

/// Interest-only periods followed by a fully amortizing term.
///
/// For the first `io_periods` rows `payment = interest = P·r` and the balance
/// stays at `P`; the next `amort_nper` rows amortize `P` with the level
/// payment `−pmt(r, amort_nper, P, 0, End)`, the last row clamped to the
/// remaining balance so the schedule ends at exactly `0`.
///
/// ```rust
/// use finance_rs::amortization::interest_only;
/// let s = interest_only(100_000.0, 0.005, 12, 240)?;
/// assert_eq!(s.rows.len(), 252);
/// assert!((s.rows[0].payment - 500.0).abs() < 1e-9);
/// assert_eq!(s.rows[11].balance, 100_000.0);
/// assert!((s.rows[12].payment - 716.43).abs() < 0.01);
/// assert_eq!(s.final_balance(), 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for `principal <= 0`, `rate <= -1` or
/// `amort_nper = 0`; [`FinanceError::NonFinite`] for non-finite input.
pub fn interest_only(
    principal: f64,
    rate: f64,
    io_periods: u32,
    amort_nper: u32,
) -> Result<Schedule> {
    check_loan(principal, rate, amort_nper)?;
    let payment = level(rate, amort_nper, principal)?;
    let total = io_periods
        .checked_add(amort_nper)
        .filter(|t| *t <= super::MAX_PERIODS)
        .ok_or(FinanceError::InvalidInput {
            what: "io_periods",
            reason: "io_periods + amort_nper exceeds MAX_PERIODS",
        })?;
    let mut s = Schedule {
        principal,
        rows: Vec::with_capacity(total as usize),
    };
    let interest = principal * rate;
    for _ in 0..io_periods {
        let period = next_period(&s.rows);
        s.rows.push(Row {
            period,
            payment: interest,
            interest,
            balance: principal,
            ..Row::default()
        });
    }
    let mut balance = principal;
    amortize(&mut s.rows, &mut balance, rate, payment, amort_nper, &[]);
    Ok(s)
}

/// Balloon loan: payments sized to amortize over `amort_nper` periods, with
/// the remaining balance due at `maturity` (`maturity <= amort_nper`).
///
/// Rows `1..maturity−1` pay the level `−pmt(r, amort_nper, P, 0, End)`; row
/// `maturity` pays its interest plus the whole remaining balance (the
/// balloon), which equals `fv(r, maturity−1, pmt, P)` before that payment.
///
/// ```rust
/// use finance_rs::amortization::balloon;
/// // 30-year amortization, due in 7 years.
/// let s = balloon(250_000.0, 0.065 / 12.0, 360, 84)?;
/// assert_eq!(s.rows.len(), 84);
/// let last = s.rows[83];
/// assert!(last.principal > 200_000.0);
/// assert_eq!(last.balance, 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for `maturity = 0` or
/// `maturity > amort_nper`, otherwise as [`super::level_payment`].
pub fn balloon(principal: f64, rate: f64, amort_nper: u32, maturity: u32) -> Result<Schedule> {
    check_loan(principal, rate, amort_nper)?;
    if maturity == 0 || maturity > amort_nper {
        return Err(FinanceError::InvalidInput {
            what: "maturity",
            reason: "must be between 1 and amort_nper",
        });
    }
    let payment = level(rate, amort_nper, principal)?;
    let mut s = Schedule {
        principal,
        rows: Vec::with_capacity(maturity as usize),
    };
    let mut balance = principal;
    amortize(&mut s.rows, &mut balance, rate, payment, maturity, &[]);
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amortization::level_payment;
    use crate::tvm::{fv, When};

    #[test]
    fn zero_io_periods_is_level() -> Result<()> {
        let a = interest_only(50_000.0, 0.004, 0, 120)?;
        let b = level_payment(50_000.0, 0.004, 120)?;
        assert_eq!(a, b);
        Ok(())
    }

    #[test]
    fn io_rows_carry_no_principal() -> Result<()> {
        let s = interest_only(80_000.0, 0.006, 24, 96)?;
        assert!(s.rows[..24].iter().all(|r| r.principal == 0.0));
        let sum = s.summary();
        assert!((sum.total_principal - 80_000.0).abs() < 1e-6);
        assert!((sum.total_interest - (sum.total_payment - 80_000.0)).abs() < 1e-6);
        assert_eq!(s.rows[119].period, 120);
        Ok(())
    }

    #[test]
    fn balloon_matches_fv() -> Result<()> {
        let (p, r) = (250_000.0, 0.065 / 12.0);
        let s = balloon(p, r, 360, 84)?;
        let level = s.rows[0].payment;
        let remaining = -fv(r, 83.0, -level, p, When::End)?;
        assert!((s.rows[83].principal - remaining).abs() < 1e-6);
        assert!((s.rows[82].balance - remaining).abs() < 1e-6);
        // Maturity == amortization term is a plain level schedule.
        assert_eq!(balloon(p, r, 360, 360)?, level_payment(p, r, 360)?);
        Ok(())
    }

    #[test]
    fn rejects_bad_maturity() {
        assert!(balloon(1_000.0, 0.01, 12, 0).is_err());
        assert!(balloon(1_000.0, 0.01, 12, 13).is_err());
        assert!(interest_only(1_000.0, 0.01, u32::MAX, 12).is_err());
    }
}
