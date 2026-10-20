//! LBO returns: MOIC, periodic equity IRR, and a simple debt-paydown model.

use crate::error::{all_finite, div, finite, min_len, output, FinanceError, Result};
use crate::tvm::irr;

/// Multiple on invested capital.
///
/// `MOIC = (Σ interim distributions + exit equity) / entry equity`
///
/// ```rust
/// use finance_rs::corp::moic;
/// assert!((moic(100.0, &[10.0, 10.0], 230.0)? - 2.5).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::Undefined`] for zero entry equity,
/// [`FinanceError::InvalidInput`] for negative entry equity,
/// [`FinanceError::NonFinite`] for non-finite input.
pub fn moic(entry_equity: f64, distributions: &[f64], exit_equity: f64) -> Result<f64> {
    finite("entry_equity", entry_equity)?;
    finite("exit_equity", exit_equity)?;
    all_finite("distributions", distributions)?;
    if entry_equity < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "entry_equity",
            reason: "must be positive",
        });
    }
    let total: f64 = distributions.iter().sum::<f64>() + exit_equity;
    div("entry_equity", total, entry_equity)
}

/// Periodic equity IRR of an investment held `distributions.len()` periods.
///
/// The flows are `[−entry_equity, d_1, …, d_{n−1}, d_n + exit_equity]` and the
/// IRR is [`crate::tvm::irr`] of them: the `r` with
/// `−E + Σ_t d_t/(1+r)^t + X/(1+r)^n = 0`. Pass zeros for periods without
/// distributions.
///
/// ```rust
/// use finance_rs::corp::equity_irr;
/// // 100 in, 2.5x out after 5 years with nothing in between: 2.5^(1/5) − 1.
/// let r = equity_irr(100.0, &[0.0; 5], 250.0)?;
/// assert!((r - (2.5_f64.powf(0.2) - 1.0)).abs() < 1e-10);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for an empty `distributions` (no holding period),
/// [`FinanceError::NoSignChange`] when nothing comes back, plus the errors of
/// [`crate::tvm::irr`].
pub fn equity_irr(entry_equity: f64, distributions: &[f64], exit_equity: f64) -> Result<f64> {
    finite("entry_equity", entry_equity)?;
    finite("exit_equity", exit_equity)?;
    min_len("distributions", distributions, 1)?;
    all_finite("distributions", distributions)?;
    let mut flows = Vec::with_capacity(distributions.len() + 1);
    flows.push(-entry_equity);
    flows.extend_from_slice(distributions);
    // Entry equity is always inserted above, so the final cash-flow slot exists.
    let last = flows.len() - 1;
    flows[last] += exit_equity;
    irr(&flows)
}

/// Inputs to the simple LBO model in [`lbo`].
///
/// ```
/// use finance_rs::corp::{lbo, Lbo};
/// let deal = Lbo { entry_ev: 100.0, debt_to_ev: 0.5, debt_rate: 0.0, fcf: vec![10.0], exit_metric: 12.0, exit_multiple: 10.0 };
/// assert!(lbo(&deal)?.exit_equity > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Lbo {
    /// Purchase enterprise value.
    pub entry_ev: f64,
    /// Acquisition debt as a fraction of `entry_ev`, in `[0, 1)`.
    pub debt_to_ev: f64,
    /// Periodic interest rate on the debt (use `0` when `fcf` is already after interest).
    pub debt_rate: f64,
    /// Free cash flow per period before debt interest; one entry per holding period.
    pub fcf: Vec<f64>,
    /// Exit-period metric (e.g. EBITDA) the exit multiple applies to.
    pub exit_metric: f64,
    /// Exit multiple (e.g. EV/EBITDA).
    pub exit_multiple: f64,
}

/// Outputs of [`lbo`].
///
/// ```
/// use finance_rs::corp::{lbo, Lbo, LboResult};
/// let deal = Lbo { entry_ev: 100.0, debt_to_ev: 0.5, debt_rate: 0.0, fcf: vec![10.0], exit_metric: 12.0, exit_multiple: 10.0 };
/// let result: LboResult = lbo(&deal)?;
/// assert!(result.moic > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct LboResult {
    /// `entry_ev · debt_to_ev`.
    pub entry_debt: f64,
    /// `entry_ev − entry_debt`.
    pub entry_equity: f64,
    /// Debt outstanding at the end of each period.
    pub debt: Vec<f64>,
    /// Cash accumulated after the debt is fully repaid.
    pub cash: f64,
    /// `exit_metric · exit_multiple`.
    pub exit_ev: f64,
    /// `exit_ev − final debt + cash`.
    pub exit_equity: f64,
    /// `exit_equity / entry_equity`.
    pub moic: f64,
    /// Periodic equity IRR; `None` when the exit equity is not positive.
    pub irr: Option<f64>,
}

/// Simple LBO: buy at `entry_ev` with `debt_to_ev` leverage, sweep free cash
/// flow to repay debt, sell at `exit_metric · exit_multiple`.
///
/// Each period: `available = fcf_t − debt_rate·D_{t−1}`; debt falls by
/// `available` (but not below zero, the excess builds `cash`; a shortfall
/// adds to the debt). Then
/// `exit_equity = exit_ev − D_n + cash`, `MOIC = exit_equity / entry_equity`,
/// `IRR = irr([−entry_equity, 0, …, exit_equity])` (no interim distributions).
///
/// ```rust
/// use finance_rs::corp::{lbo, Lbo};
/// let deal = Lbo { entry_ev: 1_000.0, debt_to_ev: 0.6, debt_rate: 0.0,
///                  fcf: vec![50.0; 5], exit_metric: 125.0, exit_multiple: 10.0 };
/// let r = lbo(&deal)?;
/// assert!((r.entry_equity - 400.0).abs() < 1e-9);
/// assert!((r.debt[4] - 350.0).abs() < 1e-9);
/// assert!((r.exit_equity - 900.0).abs() < 1e-9);
/// assert!((r.moic - 2.25).abs() < 1e-12);
/// assert!((r.irr.unwrap_or(0.0) - (2.25_f64.powf(0.2) - 1.0)).abs() < 1e-10);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for a non-positive `entry_ev`, a
/// `debt_to_ev` outside `[0, 1)`, or `debt_rate <= -1`;
/// [`FinanceError::TooShort`] for an empty `fcf`;
/// [`FinanceError::NonFinite`] / [`FinanceError::Overflow`] for non-finite
/// input or results.
pub fn lbo(deal: &Lbo) -> Result<LboResult> {
    finite("entry_ev", deal.entry_ev)?;
    finite("debt_to_ev", deal.debt_to_ev)?;
    finite("debt_rate", deal.debt_rate)?;
    finite("exit_metric", deal.exit_metric)?;
    finite("exit_multiple", deal.exit_multiple)?;
    min_len("fcf", &deal.fcf, 1)?;
    all_finite("fcf", &deal.fcf)?;
    let bad = |what, reason| Err(FinanceError::InvalidInput { what, reason });
    if deal.entry_ev <= 0.0 {
        return bad("entry_ev", "must be positive");
    }
    if !(0.0..1.0).contains(&deal.debt_to_ev) {
        return bad("debt_to_ev", "must be in [0, 1)");
    }
    if deal.debt_rate <= -1.0 {
        return bad("debt_rate", "must be greater than -1");
    }
    let entry_debt = deal.entry_ev * deal.debt_to_ev;
    let entry_equity = deal.entry_ev - entry_debt;
    let mut balance = entry_debt;
    let mut cash = 0.0;
    let mut debt = Vec::with_capacity(deal.fcf.len());
    for f in &deal.fcf {
        let available = f - deal.debt_rate * balance;
        let paydown = available.min(balance);
        balance = output("debt", balance - paydown)?;
        cash = output("cash", cash + (available - paydown))?;
        debt.push(balance);
    }
    let exit_ev = output("exit_ev", deal.exit_metric * deal.exit_multiple)?;
    let exit_equity = output("exit_equity", exit_ev - balance + cash)?;
    let multiple = div("entry_equity", exit_equity, entry_equity)?;
    let irr = if exit_equity > 0.0 {
        let distributions = vec![0.0; deal.fcf.len()];
        let solved_irr = equity_irr(entry_equity, &distributions, exit_equity)?;
        Some(solved_irr)
    } else {
        None
    };
    Ok(LboResult {
        entry_debt,
        entry_equity,
        debt,
        cash,
        exit_ev,
        exit_equity,
        moic: multiple,
        irr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deal() -> Lbo {
        Lbo {
            entry_ev: 1_000.0,
            debt_to_ev: 0.6,
            debt_rate: 0.08,
            fcf: vec![100.0, 110.0, 120.0, 130.0, 140.0],
            exit_metric: 140.0,
            exit_multiple: 9.0,
        }
    }

    #[test]
    fn interest_and_paydown() -> Result<()> {
        let r = lbo(&deal())?;
        // Year 1: interest 48, paydown 52 -> 548.
        assert!((r.debt[0] - 548.0).abs() < 1e-9);
        let mut d: f64 = 600.0;
        for f in [100.0, 110.0, 120.0, 130.0, 140.0] {
            d -= f - 0.08 * d;
        }
        assert!((r.debt[4] - d).abs() < 1e-9);
        assert!((r.exit_equity - (1_260.0 - d)).abs() < 1e-9);
        let irr = r.irr.unwrap_or(f64::NAN);
        assert!((irr - r.moic.powf(0.2) + 1.0).abs() < 1e-10);
        Ok(())
    }

    #[test]
    fn excess_cash_accumulates() -> Result<()> {
        let excess_cash = Lbo {
            debt_to_ev: 0.1,
            debt_rate: 0.0,
            ..deal()
        };
        let r = lbo(&excess_cash)?;
        assert_eq!(r.debt[4], 0.0);
        assert!((r.cash - (600.0 - 100.0)).abs() < 1e-9);
        assert!((r.exit_equity - (1_260.0 + 500.0)).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn wiped_out_equity_has_no_irr() -> Result<()> {
        let wiped_out = Lbo {
            debt_to_ev: 0.9,
            fcf: vec![-50.0; 3],
            exit_metric: 50.0,
            ..deal()
        };
        let r = lbo(&wiped_out)?;
        assert!(r.exit_equity < 0.0 && r.moic < 0.0 && r.irr.is_none());
        Ok(())
    }

    #[test]
    fn moic_and_irr_with_distributions() -> Result<()> {
        assert!((equity_irr(100.0, &[0.0, 0.0], 121.0)? - 0.1).abs() < 1e-10);
        assert!((moic(100.0, &[], 150.0)? - 1.5).abs() < 1e-15);
        let r = equity_irr(100.0, &[10.0, 10.0, 10.0], 100.0)?;
        assert!((r - 0.10).abs() < 1e-10);
        assert_eq!(
            moic(0.0, &[], 1.0),
            Err(FinanceError::Undefined {
                what: "entry_equity"
            })
        );
        assert!(moic(-1.0, &[], 1.0).is_err());
        assert!(equity_irr(100.0, &[], 100.0).is_err());
        assert_eq!(
            equity_irr(100.0, &[0.0], 0.0),
            Err(FinanceError::NoSignChange)
        );
        Ok(())
    }

    #[test]
    fn rejects_bad_deals() {
        assert!(lbo(&Lbo {
            entry_ev: 0.0,
            ..deal()
        })
        .is_err());
        assert!(lbo(&Lbo {
            debt_to_ev: 1.0,
            ..deal()
        })
        .is_err());
        assert!(lbo(&Lbo {
            debt_rate: -1.0,
            ..deal()
        })
        .is_err());
        assert!(lbo(&Lbo {
            fcf: vec![],
            ..deal()
        })
        .is_err());
        assert!(lbo(&Lbo {
            exit_multiple: f64::NAN,
            ..deal()
        })
        .is_err());
    }
}
