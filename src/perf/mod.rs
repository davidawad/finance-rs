//! Performance and risk metrics with quantstats 0.0.86 semantics
//! (empyrical-reloaded 0.5.12 for historical VaR).
//!
//! Inputs are simple per-period returns (`0.01` = +1%), oldest first, and an
//! optional benchmark return series aligned period by period. Nothing is
//! implicit:
//!
//! - **Annualization** is the explicit `periods_per_year` argument (252 for
//!   trading days, 12 for months, 52 for weeks).
//! - **Risk-free rate** `rf` is an *annual* rate, as in quantstats, and is
//!   de-annualized per period exactly as quantstats'
//!   `_utils.to_excess_returns(returns, rf, nperiods)` does:
//!
//! ```text
//! rf_period = (1 + rf)^(1 / periods_per_year) − 1
//! excess[t] = returns[t] − rf_period
//! ```
//!
//! - **Standard deviations** are sample standard deviations (`ddof = 1`,
//!   pandas' default).
//! - **Signs**: drawdowns, VaR and CVaR are returns, so losses are negative
//!   (`max_drawdown = −0.25` is a 25% peak-to-trough fall).
//!
//! Deviations from quantstats, all at degenerate inputs where quantstats
//! returns `NaN`, `0` or warns: a zero denominator (zero volatility, zero
//! downside deviation, zero drawdown, zero tracking error, zero benchmark
//! variance) is [`FinanceError::Undefined`] rather than `NaN` or a filled
//! `0`; confidence levels must lie in `(0, 1)` (quantstats also accepts
//! percentages); a negative `rf` is subtracted (quantstats only applies
//! `rf > 0`); and gaps (`NaN`) are rejected rather than skipped.
//!
//! ```
//! use finance_rs::perf::{max_drawdown, sharpe, volatility};
//! let r = [0.01, -0.02, 0.03, -0.01, 0.02];
//! let vol = volatility(&r, 252.0)?;
//! let s = sharpe(&r, 0.0, 252.0)?;
//! assert!((s - 0.006 / (vol / 252f64.sqrt()) * 252f64.sqrt()).abs() < 1e-9);
//! assert!((max_drawdown(&r)? + 0.02).abs() < 1e-12);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

// "VaR" / "CVaR" are standard abbreviations, not code identifiers.
#![allow(clippy::doc_markdown)]

mod bench;
mod bench_extra;
mod drawdown;
mod gpd;
mod ratios;
mod ratios_extra;
mod ratios_profile;
pub mod returns;
mod returns_extra;
pub mod risk;
mod risk_extra;

pub use crate::math::{normal_cdf, normal_pdf, normal_ppf};
pub use bench::{alpha, beta, information_ratio};
pub use bench_extra::{
    batting_average, down_alpha_beta, down_capture, excess_sharpe, r_squared, tracking_error,
    up_alpha_beta, up_capture, up_down_capture,
};
pub use drawdown::{
    drawdown_periods, drawdown_periods_dated, drawdown_series, max_drawdown, max_drawdown_duration,
    serenity_index, top_drawdowns, ulcer_index, ulcer_performance_index, DatedDrawdownPeriod,
    DrawdownPeriod,
};
pub use gpd::{gpd_tail_risk, GpdTailRisk};
pub use ratios::{
    calmar, omega, sharpe, sharpe_from_moments, sharpe_periodic_population, sortino,
    sortino_periodic_mar,
};
pub use ratios_extra::{
    adjusted_sortino, deflated_sharpe, downside_deviation, expected_max_sharpe, kurtosis, m2,
    prob_backtest_overfitting, probabilistic_sharpe, skew, smart_sharpe, smart_sortino, treynor,
};
pub use ratios_profile::{
    common_sense_ratio, cpc_index, gain_to_pain, kelly_criterion, outlier_loss_ratio,
    outlier_win_ratio, payoff_ratio, profit_factor, profit_ratio, recovery_factor, risk_of_ruin,
    risk_return_ratio, stability, tail_ratio, weighted_outcome_sharpe, weighted_sharpe,
};
pub use returns::{
    aggregate, avg_loss, avg_return, avg_win, best, cagr, compounded_return, consecutive_losses,
    consecutive_wins, cumulative_returns, excess, exposure, geometric_mean, log, modified_dietz,
    mtd, mwr, qtd, rebase, rolling, simple, to_price_index, twr, volatility, win_rate, worst, ytd,
    Period as ReturnPeriod,
};
pub use risk::{
    cvar_historical, cvar_parametric, historical_var_cvar_floor, parametric_var_cvar_population,
    var_historical, var_parametric,
};
pub use risk_extra::{
    beta_fragility, cvar_cornish_fisher, risk_component_var, risk_marginal_var, risk_scale_var,
    var_cornish_fisher, var_ewma,
};

use crate::error::{all_finite, finite, min_len, FinanceError, Result};

/// Validate a return series: at least `min` values, all finite.
fn check_returns(what: &'static str, returns: &[f64], min: usize) -> Result<()> {
    min_len(what, returns, min)?;
    all_finite(what, returns)
}

/// Validate `periods_per_year`: finite and positive.
fn check_periods(periods_per_year: f64) -> Result<f64> {
    finite("periods_per_year", periods_per_year)?;
    if periods_per_year <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "periods_per_year",
            reason: "must be positive",
        });
    }
    Ok(periods_per_year)
}

/// Arithmetic mean (caller guarantees a non-empty slice).
fn mean(xs: &[f64]) -> f64 {
    crate::stats::mean_of(xs)
}

/// Sample standard deviation, `ddof = 1`, two-pass (caller guarantees `len >= 2`).
fn std_dev(xs: &[f64]) -> f64 {
    crate::stats::variance_of(xs, 1).sqrt()
}

/// Excess returns over the annual `rf`, de-annualized per period.
///
/// `rf = 0` leaves the returns untouched (bit-for-bit, as quantstats skips
/// the subtraction).
fn excess_returns(returns: &[f64], rf: f64, periods_per_year: f64) -> Result<Vec<f64>> {
    finite("rf", rf)?;
    if rf <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "rf",
            reason: "annual risk-free rate must be greater than -100%",
        });
    }
    if rf == 0.0 {
        return Ok(returns.to_vec());
    }
    let per_period = crate::rates::periodic_from_annual(rf, periods_per_year)?;
    Ok(returns.iter().map(|r| r - per_period).collect())
}

/// Validate a confidence level in `(0, 1)` and return the tail probability
/// `1 − confidence`.
fn tail_probability(confidence: f64) -> Result<f64> {
    finite("confidence", confidence)?;
    if confidence <= 0.0 || confidence >= 1.0 {
        return Err(FinanceError::InvalidInput {
            what: "confidence",
            reason: "must be in (0, 1)",
        });
    }
    Ok(1.0 - confidence)
}
