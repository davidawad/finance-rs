//! Corporate finance: discounted cash flow, terminal value, enterprise value,
//! WACC, CAPM, and LBO returns (MOIC / IRR).
//!
//! Rates are per period (annual for the usual yearly DCF). Cash flow `t`
//! (1-based) is received at the end of period `t`, or at `t − ½` under the
//! mid-year convention.
//!
//! ```
//! use finance_rs::corp::{enterprise_value, terminal_value_gordon, Convention};
//! let fcf = [100.0, 110.0, 121.0];
//! let tv = terminal_value_gordon(121.0, 0.10, 0.02)?;
//! let v = enterprise_value(0.10, &fcf, tv, Convention::EndOfPeriod)?;
//! assert!((v.enterprise_value - (v.pv_cash_flows + v.pv_terminal_value)).abs() < 1e-9);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

mod equity;
mod lbo;

/// Extended equity valuation functions and result types.
///
/// ```
/// assert_eq!(finance_rs::corp::fcff(100.0, 0.2, 5.0, 3.0, 2.0)?, 80.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub use equity::{
    apv, dcf_dated, diluted_shares_tsm, equity_bridge, eva, fcfe, fcff, implied_discount_rate,
    relever_beta, reverse_dcf, unlever_beta, value_per_share, DcfTiming, EquityValue,
};
/// Leveraged buyout inputs, outputs, and return measures.
///
/// ```
/// assert!((finance_rs::corp::moic(100.0, &[], 150.0)? - 1.5).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub use lbo::{equity_irr, lbo, moic, Lbo, LboResult};

use crate::error::{all_finite, finite, min_len, output, FinanceError, Result};

/// When within each period a DCF cash flow is assumed to arrive.
///
/// ```
/// let convention = finance_rs::corp::Convention::MidPeriod;
/// assert_eq!(convention, finance_rs::corp::Convention::MidPeriod);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum Convention {
    /// Cash flow `t` arrives at the end of period `t`: discount by `(1+r)^t`.
    #[default]
    EndOfPeriod,
    /// Cash flow `t` arrives mid-period: discount by `(1+r)^(t − ½)`.
    MidPeriod,
}

/// Present value of a DCF: the parts of an enterprise value.
///
/// ```
/// use finance_rs::corp::{enterprise_value, Convention, Valuation};
/// let value: Valuation = enterprise_value(0.1, &[100.0], 0.0, Convention::EndOfPeriod)?;
/// assert_eq!(value.enterprise_value, value.pv_cash_flows + value.pv_terminal_value);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Valuation {
    /// `Σ PV(FCF_t)` of the explicit forecast.
    pub pv_cash_flows: f64,
    /// `TV / (1+r)^n`.
    pub pv_terminal_value: f64,
    /// `pv_cash_flows + pv_terminal_value`.
    pub enterprise_value: f64,
}

/// Validate a discount rate: finite and greater than −100%.
fn check_rate(rate: f64) -> Result<()> {
    finite("rate", rate)?;
    if rate <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "rate",
            reason: "must be greater than -1",
        });
    }
    Ok(())
}

/// Present value of explicit cash flows.
///
/// `PV = Σ_{t=1..n} CF_t / (1+r)^(t − s)`, with `s = 0` for
/// [`Convention::EndOfPeriod`] and `s = ½` for [`Convention::MidPeriod`].
/// The first cash flow is one period out (unlike [`crate::tvm::npv`], whose
/// first value is undiscounted).
///
/// ```rust
/// use finance_rs::corp::{dcf, Convention};
/// let end = dcf(0.10, &[100.0, 100.0], Convention::EndOfPeriod)?;
/// assert!((end - 173.553_719_008_264_46).abs() < 1e-9);
/// let mid = dcf(0.10, &[100.0, 100.0], Convention::MidPeriod)?;
/// assert!((mid - end * 1.1_f64.sqrt()).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for no cash flows, [`FinanceError::InvalidInput`]
/// for `rate <= -1`, [`FinanceError::NonFinite`] for non-finite input,
/// [`FinanceError::Overflow`] for a non-finite result.
pub fn dcf(rate: f64, cash_flows: &[f64], convention: Convention) -> Result<f64> {
    output(
        "dcf",
        dcf_cash_flows(rate, cash_flows, convention)?.iter().sum(),
    )
}

/// Present value of each forecast cash flow, in input order.
///
/// ```
/// use finance_rs::corp::{dcf_cash_flows, Convention};
/// assert_eq!(dcf_cash_flows(0.0, &[100.0, 200.0], Convention::EndOfPeriod)?, vec![100.0, 200.0]);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn dcf_cash_flows(rate: f64, cash_flows: &[f64], convention: Convention) -> Result<Vec<f64>> {
    check_rate(rate)?;
    min_len("cash_flows", cash_flows, 1)?;
    all_finite("cash_flows", cash_flows)?;
    let shift = match convention {
        Convention::EndOfPeriod => 0.0,
        Convention::MidPeriod => 0.5,
    };
    let mut t = 1.0 - shift;
    let mut values = Vec::with_capacity(cash_flows.len());
    for cf in cash_flows {
        values.push(output("dcf", cf / (1.0 + rate).powf(t))?);
        t += 1.0;
    }
    Ok(values)
}

/// Gordon-growth terminal value at the end of the final forecast period.
///
/// `TV = FCF_n · (1+g) / (r − g)`
///
/// ```rust
/// use finance_rs::corp::terminal_value_gordon;
/// let tv = terminal_value_gordon(100.0, 0.10, 0.03)?;
/// assert!((tv - 1_471.428_571_428_571_4).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] when `rate <= growth` (no finite value) or
/// `growth <= -1`, [`FinanceError::NonFinite`] for non-finite input.
pub fn terminal_value_gordon(final_cash_flow: f64, rate: f64, growth: f64) -> Result<f64> {
    finite("final_cash_flow", final_cash_flow)?;
    check_rate(rate)?;
    finite("growth", growth)?;
    if growth <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "growth",
            reason: "must be greater than -1",
        });
    }
    if rate <= growth {
        return Err(FinanceError::InvalidInput {
            what: "rate",
            reason: "must exceed the growth rate",
        });
    }
    // The Gordon growth perpetuity on the first post-horizon flow.
    let next = output("terminal value", final_cash_flow * (1.0 + growth))?;
    crate::tvm::perpetuity_pv(next, rate, growth, crate::tvm::When::End)
}

/// Exit-multiple terminal value: `TV = metric_n · multiple` (e.g. EBITDA × EV/EBITDA).
///
/// ```rust
/// use finance_rs::corp::terminal_value_exit_multiple;
/// assert!((terminal_value_exit_multiple(50.0, 8.0)? - 400.0).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NonFinite`] for non-finite input, [`FinanceError::Overflow`]
/// for a non-finite result.
pub fn terminal_value_exit_multiple(final_metric: f64, multiple: f64) -> Result<f64> {
    finite("final_metric", final_metric)?;
    finite("multiple", multiple)?;
    output("terminal value", final_metric * multiple)
}

/// Enterprise value from an explicit forecast and a terminal value.
///
/// `EV = dcf(r, FCF, convention) + TV / (1+r)^n`, `n = FCF.len()`. The
/// terminal value is always discounted from the end of period `n` (it is a
/// value at that date), under either convention.
///
/// ```rust
/// use finance_rs::corp::{enterprise_value, Convention};
/// let v = enterprise_value(0.10, &[100.0, 100.0], 1_000.0, Convention::EndOfPeriod)?;
/// assert!((v.pv_terminal_value - 826.446_280_991_735_5).abs() < 1e-9);
/// assert!((v.enterprise_value - 1_000.0).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`dcf`], plus [`FinanceError::NonFinite`] for a non-finite terminal value.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn enterprise_value(
    rate: f64,
    cash_flows: &[f64],
    terminal_value: f64,
    convention: Convention,
) -> Result<Valuation> {
    finite("terminal_value", terminal_value)?;
    let pv_cash_flows = dcf(rate, cash_flows, convention)?;
    let n = cash_flows.len();
    let factor = (1.0 + rate).powf(n as f64);
    let pv_terminal_value = output("pv terminal value", terminal_value / factor)?;
    Ok(Valuation {
        pv_cash_flows,
        pv_terminal_value,
        enterprise_value: output("enterprise value", pv_cash_flows + pv_terminal_value)?,
    })
}

/// Weighted average cost of capital.
///
/// `WACC = E/V·Re + D/V·Rd·(1 − t)`, `V = E + D`.
///
/// ```rust
/// use finance_rs::corp::wacc;
/// let w = wacc(600.0, 400.0, 0.10, 0.05, 0.25)?;
/// assert!((w - 0.075).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for negative `equity_value`/`debt_value` or
/// a `tax_rate` outside `[0, 1]`; [`FinanceError::Undefined`] when
/// `E + D = 0`; [`FinanceError::NonFinite`] for non-finite input.
pub fn wacc(
    equity_value: f64,
    debt_value: f64,
    cost_of_equity: f64,
    cost_of_debt: f64,
    tax_rate: f64,
) -> Result<f64> {
    finite("equity_value", equity_value)?;
    finite("debt_value", debt_value)?;
    finite("cost_of_equity", cost_of_equity)?;
    finite("cost_of_debt", cost_of_debt)?;
    finite("tax_rate", tax_rate)?;
    if equity_value < 0.0 || debt_value < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "equity_value/debt_value",
            reason: "must be non-negative",
        });
    }
    if !(0.0..=1.0).contains(&tax_rate) {
        return Err(FinanceError::InvalidInput {
            what: "tax_rate",
            reason: "must be between 0 and 1",
        });
    }
    let v = equity_value + debt_value;
    if v == 0.0 {
        return Err(FinanceError::Undefined {
            what: "equity_value + debt_value",
        });
    }
    output(
        "wacc",
        equity_value / v * cost_of_equity + debt_value / v * cost_of_debt * (1.0 - tax_rate),
    )
}

/// CAPM cost of equity: `Re = rf + β·(Rm − rf)`.
///
/// ```rust
/// use finance_rs::corp::capm;
/// assert!((capm(0.04, 1.2, 0.09)? - 0.10).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NonFinite`] for non-finite input.
pub fn capm(risk_free: f64, beta: f64, market_return: f64) -> Result<f64> {
    finite("risk_free", risk_free)?;
    finite("beta", beta)?;
    finite("market_return", market_return)?;
    output("capm", risk_free + beta * (market_return - risk_free))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tvm::npv;

    #[test]
    fn dcf_matches_npv_shifted() -> Result<()> {
        let cf = [-50.0, 30.0, 40.0, 55.0];
        let mut shifted = vec![0.0];
        shifted.extend_from_slice(&cf);
        let a = dcf(0.08, &cf, Convention::EndOfPeriod)?;
        assert_eq!(
            dcf_cash_flows(0.08, &cf, Convention::EndOfPeriod)?
                .iter()
                .sum::<f64>(),
            a
        );
        assert!((a - npv(0.08, &shifted)?).abs() < 1e-9);
        let mid = dcf(0.08, &cf, Convention::MidPeriod)?;
        assert!((mid - a * 1.08_f64.sqrt()).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn textbook_valuation() -> Result<()> {
        // FCF 100 growing 5% for 5 years, WACC 9%, terminal growth 2.5%.
        let fcf: Vec<f64> = (0..5).map(|i| 100.0 * 1.05_f64.powi(i)).collect();
        let tv = terminal_value_gordon(fcf[4], 0.09, 0.025)?;
        assert!((tv - 1_916.759_855_769_231).abs() < 1e-9);
        let v = enterprise_value(0.09, &fcf, tv, Convention::EndOfPeriod)?;
        assert!((v.pv_terminal_value - tv / 1.09_f64.powi(5)).abs() < 1e-9);
        assert!((v.enterprise_value - 1_672.023_777_204_504).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn gordon_requires_rate_above_growth() {
        assert!(terminal_value_gordon(100.0, 0.05, 0.05).is_err());
        assert!(terminal_value_gordon(100.0, 0.03, 0.05).is_err());
        assert!(terminal_value_gordon(100.0, 0.05, -1.0).is_err());
    }

    #[test]
    fn wacc_edges() -> Result<()> {
        assert!((wacc(1.0, 0.0, 0.11, 0.06, 0.3)? - 0.11).abs() < 1e-15);
        assert!((wacc(0.0, 1.0, 0.11, 0.06, 0.3)? - 0.042).abs() < 1e-15);
        assert_eq!(
            wacc(0.0, 0.0, 0.1, 0.05, 0.2),
            Err(FinanceError::Undefined {
                what: "equity_value + debt_value"
            })
        );
        assert!(wacc(-1.0, 1.0, 0.1, 0.05, 0.2).is_err());
        assert!(wacc(1.0, 1.0, 0.1, 0.05, 1.2).is_err());
        assert!(capm(f64::NAN, 1.0, 0.1).is_err());
        Ok(())
    }

    #[test]
    fn rejects_bad_dcf_input() {
        assert!(dcf(0.1, &[], Convention::EndOfPeriod).is_err());
        assert!(dcf(-1.0, &[1.0], Convention::EndOfPeriod).is_err());
        assert!(dcf(0.1, &[f64::NAN], Convention::MidPeriod).is_err());
        assert!(enterprise_value(0.1, &[1.0], f64::INFINITY, Convention::EndOfPeriod).is_err());
        assert!(terminal_value_exit_multiple(f64::MAX, 10.0).is_err());
    }
}
