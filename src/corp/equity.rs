//! Extended equity valuation identities used by the equities catalog.
use super::{dcf, Convention};

/// Dated DCF cash-flow timing convention.
///
/// ```
/// let timing = finance_rs::corp::DcfTiming::MidPeriod;
/// assert_eq!(timing, finance_rs::corp::DcfTiming::MidPeriod);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DcfTiming {
    /// Discount on the actual cash-flow dates.
    ActualDate,
    /// Apply a half-day-count-year mid-period adjustment.
    MidPeriod,
}
use crate::{
    error::{all_finite, finite, output, FinanceError},
    solve::{brent, SolverOptions},
    Date, Result,
};

/// Free cash flow to the firm: `EBIT(1-tax) + D&A - capex - change in NWC`.
///
/// ```
/// let cash_flow = finance_rs::corp::fcff(100.0, 0.25, 10.0, 5.0, 2.0)?;
/// assert_eq!(cash_flow, 78.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn fcff(ebit: f64, tax: f64, da: f64, capex: f64, delta_nwc: f64) -> Result<f64> {
    all_finite("fcff inputs", &[ebit, tax, da, capex, delta_nwc])?;
    if !(0.0..=1.0).contains(&tax) {
        return Err(FinanceError::InvalidInput {
            what: "tax",
            reason: "must be between zero and one",
        });
    }
    output("fcff", ebit * (1.0 - tax) + da - capex - delta_nwc)
}

/// Free cash flow to equity: `FCFF - interest(1-tax) + net borrowing`.
///
/// ```
/// let cash_flow = finance_rs::corp::fcfe(78.0, 8.0, 0.25, 3.0)?;
/// assert_eq!(cash_flow, 75.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn fcfe(fcff: f64, interest: f64, tax: f64, net_borrowing: f64) -> Result<f64> {
    all_finite("fcfe inputs", &[fcff, interest, tax, net_borrowing])?;
    if !(0.0..=1.0).contains(&tax) {
        return Err(FinanceError::InvalidInput {
            what: "tax",
            reason: "must be between zero and one",
        });
    }
    output("fcfe", fcff - interest * (1.0 - tax) + net_borrowing)
}

/// Equity value produced by bridging from enterprise value.
///
/// ```
/// let value = finance_rs::corp::EquityValue { equity_value: 900.0, net_debt: 100.0 };
/// assert_eq!(value.equity_value + value.net_debt, 1_000.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EquityValue {
    /// Equity value for the equity value result.
    pub equity_value: f64,
    /// Net debt for the equity value result.
    pub net_debt: f64,
}
/// `EV - debt + cash - minorities - preferred + non-operating assets`.
///
/// ```
/// let bridge = finance_rs::corp::equity_bridge(1_000.0, 200.0, 50.0, 0.0, 0.0, 10.0)?;
/// assert_eq!(bridge.equity_value, 860.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn equity_bridge(
    ev: f64,
    debt: f64,
    cash: f64,
    minorities: f64,
    preferred: f64,
    non_operating: f64,
) -> Result<EquityValue> {
    all_finite(
        "equity bridge",
        &[ev, debt, cash, minorities, preferred, non_operating],
    )?;
    let net_debt = debt - cash;
    Ok(EquityValue {
        equity_value: output(
            "equity_bridge",
            ev - net_debt - minorities - preferred + non_operating,
        )?,
        net_debt: output("net_debt", net_debt)?,
    })
}

/// Treasury-stock-method diluted share count: options in the money issue net shares.
///
/// ```
/// let shares = finance_rs::corp::diluted_shares_tsm(100.0, &[(10.0, 5.0)], 10.0)?;
/// assert_eq!(shares, 105.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn diluted_shares_tsm(basic: f64, options: &[(f64, f64)], price: f64) -> Result<f64> {
    finite("basic shares", basic)?;
    finite("price", price)?;
    if basic < 0.0 || price <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "TSM inputs",
            reason: "basic shares nonnegative and price positive required",
        });
    }
    let mut diluted = basic;
    for &(count, strike) in options {
        all_finite("options", &[count, strike])?;
        if count < 0.0 || strike < 0.0 {
            return Err(FinanceError::InvalidInput {
                what: "options",
                reason: "counts and strikes must be nonnegative",
            });
        }
        if price > strike {
            diluted += count * (price - strike) / price;
        }
    }
    output("diluted_shares_tsm", diluted)
}

/// Solve for per-share value when TSM dilution depends on price.
///
/// ```
/// let price = finance_rs::corp::value_per_share(1_000.0, 100.0, &[])?;
/// assert!((price - 10.0).abs() < 1e-8);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn value_per_share(
    equity_value: f64,
    basic_shares: f64,
    options: &[(f64, f64)],
) -> Result<f64> {
    finite("equity_value", equity_value)?;
    finite("basic_shares", basic_shares)?;
    if equity_value <= 0.0 || basic_shares <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "value per share",
            reason: "equity value and basic shares must be positive",
        });
    }
    let f = |p: f64| {
        diluted_shares_tsm(basic_shares, options, p).map_or(f64::NAN, |s| p * s - equity_value)
    };
    brent(
        f,
        1e-12,
        equity_value / basic_shares,
        &SolverOptions::default(),
    )
    .or_else(|_| {
        brent(
            f,
            1e-12,
            equity_value / basic_shares * 2.0 + 1.0,
            &SolverOptions::default(),
        )
    })
    .map(|s| s.root)
}

/// Hamada unlevered beta `βL / (1 + (1-tax)D/E)`.
///
/// ```
/// let beta = finance_rs::corp::unlever_beta(1.2, 0.5, 0.25)?;
/// assert!((beta - 1.2 / 1.375).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn unlever_beta(beta_l: f64, debt_to_equity: f64, tax: f64) -> Result<f64> {
    all_finite("unlever beta", &[beta_l, debt_to_equity, tax])?;
    if debt_to_equity < 0.0 || !(0.0..=1.0).contains(&tax) {
        return Err(FinanceError::InvalidInput {
            what: "unlever beta inputs",
            reason: "D/E nonnegative and tax in [0,1] required",
        });
    }
    output(
        "unlever_beta",
        beta_l / (1.0 + (1.0 - tax) * debt_to_equity),
    )
}
/// Hamada relevered beta `βU * (1 + (1-tax)D/E)`.
///
/// ```
/// let beta = finance_rs::corp::relever_beta(0.8, 0.5, 0.25)?;
/// assert!((beta - 0.8 * 1.375).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn relever_beta(beta_u: f64, debt_to_equity: f64, tax: f64) -> Result<f64> {
    all_finite("relever beta", &[beta_u, debt_to_equity, tax])?;
    if debt_to_equity < 0.0 || !(0.0..=1.0).contains(&tax) {
        return Err(FinanceError::InvalidInput {
            what: "relever beta inputs",
            reason: "D/E nonnegative and tax in [0,1] required",
        });
    }
    output(
        "relever_beta",
        beta_u * (1.0 + (1.0 - tax) * debt_to_equity),
    )
}

/// Adjusted present value: unlevered value plus present value of tax shields.
///
/// ```
/// let value = finance_rs::corp::apv(100.0, &[10.0], 0.10)?;
/// assert!((value - 100.0 - 10.0 / 1.1).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn apv(unlevered_value: f64, tax_shields: &[f64], rd: f64) -> Result<f64> {
    all_finite("APV inputs", tax_shields)?;
    finite("unlevered_value", unlevered_value)?;
    finite("rd", rd)?;
    if rd <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "rd",
            reason: "must exceed -1",
        });
    }
    let pv = tax_shields
        .iter()
        .enumerate()
        .map(|(i, x)| x / (1.0 + rd).powf(i as f64 + 1.0))
        .sum::<f64>();
    output("apv", unlevered_value + pv)
}

/// Economic value added: `NOPAT - WACC * invested capital`.
///
/// ```
/// assert_eq!(finance_rs::corp::eva(100.0, 0.1, 800.0)?, 20.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn eva(nopat: f64, wacc: f64, invested_capital: f64) -> Result<f64> {
    all_finite("EVA inputs", &[nopat, wacc, invested_capital])?;
    output("eva", nopat - wacc * invested_capital)
}

/// Implied growth for which the supplied forecast's enterprise value equals market EV.
///
/// ```
/// let growth = finance_rs::corp::reverse_dcf(80.0, |g| vec![100.0 * (1.0 + g)], 0.1, -0.5, 0.5)?;
/// assert!(growth.is_finite());
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn reverse_dcf(
    market_ev: f64,
    cfs_fn: impl Fn(f64) -> Vec<f64>,
    rate: f64,
    lower_growth: f64,
    upper_growth: f64,
) -> Result<f64> {
    all_finite(
        "reverse DCF inputs",
        &[market_ev, rate, lower_growth, upper_growth],
    )?;
    if market_ev <= 0.0 || rate <= -1.0 || lower_growth >= upper_growth {
        return Err(FinanceError::InvalidInput {
            what: "reverse DCF inputs",
            reason: "positive EV, rate above -1, and ordered growth bounds required",
        });
    }
    let f =
        |g: f64| dcf(rate, &cfs_fn(g), Convention::EndOfPeriod).map_or(f64::NAN, |v| v - market_ev);
    brent(f, lower_growth, upper_growth, &SolverOptions::default()).map(|s| s.root)
}

/// Implied discount rate for enterprise value and explicit cash flows plus terminal value.
///
/// ```
/// let rate = finance_rs::corp::implied_discount_rate(100.0, &[80.0], 30.0)?;
/// assert!(rate.is_finite());
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn implied_discount_rate(ev: f64, cfs: &[f64], terminal: f64) -> Result<f64> {
    all_finite("implied discount inputs", cfs)?;
    all_finite("implied discount", &[ev, terminal])?;
    if cfs.is_empty() {
        return Err(FinanceError::TooShort {
            what: "cash_flows",
            min: 1,
            got: 0,
        });
    }
    let f = |r: f64| {
        dcf(r, cfs, Convention::EndOfPeriod)
            .and_then(|pv| {
                let total = pv + terminal / (1.0 + r).powf(cfs.len() as f64);
                output("implied_discount_rate", total - ev)
            })
            .unwrap_or(f64::NAN)
    };
    brent(f, -0.999, 10.0, &SolverOptions::default()).map(|s| s.root)
}

/// Dated DCF using ACT/365F from the caller-supplied valuation date.
///
/// ```
/// use finance_rs::{corp::{dcf_dated, DcfTiming}, Date};
/// let base = Date::new(2026, 1, 1)?;
/// let later = Date::new(2027, 1, 1)?;
/// let value = dcf_dated(0.05, &[later], &[110.0], base, DcfTiming::ActualDate)?;
/// assert!(value > 100.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn dcf_dated<D: Copy + Into<Date>>(
    rate: f64,
    dates: &[D],
    cash_flows: &[f64],
    valuation_date: D,
    timing: DcfTiming,
) -> Result<f64> {
    finite("rate", rate)?;
    all_finite("cash_flows", cash_flows)?;
    if rate <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "rate",
            reason: "must exceed -1",
        });
    }
    if dates.len() != cash_flows.len() {
        return Err(FinanceError::LengthMismatch {
            left: "dates",
            right: "cash_flows",
            left_len: dates.len(),
            right_len: cash_flows.len(),
        });
    }
    if dates.is_empty() {
        return Err(FinanceError::TooShort {
            what: "dates",
            min: 1,
            got: 0,
        });
    }
    let base = valuation_date.into();
    let mut pv = 0.0;
    for (date, cf) in dates.iter().zip(cash_flows) {
        let days = ((*date).into().ordinal() - base.ordinal()) as f64;
        if days < 0.0 {
            return Err(FinanceError::InvalidInput {
                what: "dates",
                reason: "cash flows must be on or after valuation date",
            });
        }
        let t = days / 365.0;
        let adjusted = if timing == DcfTiming::MidPeriod {
            (t - 0.5 / 365.0).max(0.0)
        } else {
            t
        };
        pv += cf / (1.0 + rate).powf(adjusted);
    }
    output("dcf_dated", pv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn worked_corporate_equity_values() -> Result<()> {
        assert!((fcff(100.0, 0.25, 10.0, 20.0, 5.0)? - 60.0).abs() < 1e-12);
        assert!((fcfe(60.0, 10.0, 0.25, 5.0)? - 57.5).abs() < 1e-12);
        assert_eq!(
            equity_bridge(100.0, 30.0, 5.0, 2.0, 3.0, 4.0)?,
            EquityValue {
                equity_value: 74.0,
                net_debt: 25.0
            }
        );
        assert!((diluted_shares_tsm(100.0, &[(10.0, 50.0)], 100.0)? - 105.0).abs() < 1e-12);
        assert!((value_per_share(1_000.0, 100.0, &[(10.0, 50.0)])? - 10.0).abs() < 1e-8);
        assert!((unlever_beta(1.5, 0.5, 0.25)? - 1.090_909_090_909_090_8).abs() < 1e-12);
        assert!((relever_beta(unlever_beta(1.5, 0.5, 0.25)?, 0.5, 0.25)? - 1.5).abs() < 1e-12);
        assert!((apv(100.0, &[10.0, 10.0], 0.1)? - 117.355_371_900_826_44).abs() < 1e-10);
        assert!((eva(20.0, 0.1, 100.0)? - 10.0).abs() < 1e-12);
        let valuation_date = Date::new(2020, 1, 1)?;
        let year_later = Date::new(2021, 1, 1)?;
        let expected = 110.0 / 1.1_f64.powf(366.0 / 365.0);
        assert!(
            (dcf_dated(
                0.1,
                &[year_later],
                &[110.0],
                valuation_date,
                DcfTiming::ActualDate
            )? - expected)
                .abs()
                < 1e-10
        );
        Ok(())
    }

    #[test]
    fn dated_dcf_rejects_unaligned_empty_and_prevaluation_dates() -> Result<()> {
        let valuation_date = Date::new(2024, 1, 1)?;
        assert!(dcf_dated(0.1, &[], &[1.0], valuation_date, DcfTiming::ActualDate).is_err());
        assert!(dcf_dated(0.1, &[], &[], valuation_date, DcfTiming::ActualDate).is_err());
        assert!(dcf_dated(
            0.1,
            &[Date::new(2023, 12, 31)?],
            &[1.0],
            valuation_date,
            DcfTiming::ActualDate,
        )
        .is_err());
        let mid_period_dates = [valuation_date];
        let mid_period_flows = [1.0];
        let mid_period_result = dcf_dated(
            0.1,
            &mid_period_dates,
            &mid_period_flows,
            valuation_date,
            DcfTiming::MidPeriod,
        );
        let mid_period_value = mid_period_result?;
        assert!(mid_period_value.is_finite());
        Ok(())
    }

    #[test]
    fn reverse_dcf_recovers_one_period_growth() -> Result<()> {
        let growth = reverse_dcf(100.0, |g| vec![110.0 * (1.0 + g)], 0.1, -0.5, 0.5)?;
        assert!((growth - 0.0).abs() < 1e-12);
        Ok(())
    }

    proptest! {
        #[test]
        fn beta_leverage_round_trip(beta in -3.0_f64..3.0, debt_to_equity in 0.0_f64..5.0, tax in 0.0_f64..1.0) {
            let unlevered = unlever_beta(beta, debt_to_equity, tax)?;
            let relevered = relever_beta(unlevered, debt_to_equity, tax)?;
            prop_assert!((relevered - beta).abs() < 1e-10);
        }
    }
}
