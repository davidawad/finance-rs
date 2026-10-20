//! # finance-rs
//!
//! One dependency-free library for the financial math that spreadsheets,
//! metrics and strategies keep re-deriving:
//!
//! | module | scope | reference semantics |
//! |---|---|---|
//! | [`math`] | standard normal `pdf`, `cdf`, `ppf` | scipy |
//! | [`kelly`] | binary, fractional, capped, continuous and simultaneous Kelly sizing | closed forms |
//! | [`pm`] | odds, devigging, prediction-market pricing/fees and forecast scores | closed forms / venue schedules |
//! | [`stats`] | mean, variance, stdev, quantile (6 methods), covariance, correlation, OLS | numpy / statsmodels |
//! | [`solve`] | Brent, bracketed Newton, bracket expansion, root scan, fixed point | scipy |
//! | [`rates`] | `Rate`, `Compounding`, `Frequency`; effective/nominal, (de)annualize, continuous, Fisher | `QuantLib` `InterestRate`, Excel, ffn |
//! | [`period`] | `Period` tenors (`"3M"`) and `Date::add_period` | `QuantLib` `Period` |
//! | [`calendars`] | business-day adjustment, market calendars, joint calendars | `QuantLib` |
//! | [`schedule`] | forward/backward dated schedules, stubs, payment dates | `QuantLib` `MakeSchedule` |
//! | [`tvm`] | `pv`, `fv`, `pmt`, `ipmt`, `ppmt`, `nper`, `rate`, `npv`, `irr`, `mirr`, `cumipmt`, `cumprinc`, `nfv`, `fv_schedule`, growing annuities and perpetuities, payback, PI, EAA | numpy-financial / pyxirr |
//! | [`cashflow`] | `xnpv`, `xirr` (bracketing + Newton, every root reported), `xfv`, `xnfv`, sign changes, dated duration, `CashFlowStream` | pyxirr |
//! | [`daycount`] | ACT/365F, ACT/360, 30/360 family, ACT/ACT variants, Bus/252 | pyxirr / `QuantLib` |
//! | [`amortization`] | level-payment, interest-only, balloon, PMI drop-off, refinance | closed forms |
//! | [`perf`] | CAGR, volatility, Sharpe, Sortino, Calmar, drawdowns, VaR/CVaR, beta/alpha, IR, omega, win rate | quantstats / empyrical |
//! | [`portfolio`] | Expected-return/covariance estimators, Black–Litterman, HRP, risk parity, weights, risk, allocations and rebalancing | `PyPortfolioOpt` / ffn / textbook |
//! | [`attribution`] | Brinson attribution and multi-period linking | textbook definitions |
//! | [`factors`] | Factor exposures, factor attribution, Fama-French alpha and rolling beta | statsmodels / French data |
//! | [`re`] | cap rate, cash-on-cash, DSCR, GRM, breakeven occupancy, debt yield | closed forms |
//! | [`corp`] | DCF and terminal value, WACC, LBO MOIC/IRR | closed forms |
//! | [`equity`] | DDM, residual income, multiples, yields, operating returns, Graham and comps | closed forms |
//! | [`fund`] | TVPI, DPI, RVPI, Kaplan-Schoar PME, European/American waterfalls | ILPA definitions |
//! | [`fund`] | fund multiples, KS/Long-Nickels/PME+/mPME, fund IRR, waterfalls, look-through and pacing | pyxirr `pe`, ILPA |
//! | [`fi`] | bond price and yield, Macaulay/modified duration, convexity | closed forms |
//! | [`options`] | BSM, Black-76, Bachelier, FX, Margrabe, Greeks, implied vol and realized vol | `py_vollib` / `QuantLib` |
//!
//! Everything is `f64` and `std`-only by default. Optional features:
//! `chrono` (accept `chrono::NaiveDate` for XIRR and day counts), `decimal`
//! (`rust_decimal` versions of the money-exact TVM and amortization paths),
//! `serde` (derive `Serialize`/`Deserialize` on every public data type).
//!
//! Every fallible function returns [`Result`]: bad input is a typed
//! [`FinanceError`], never a panic and never a silent `NaN`.
#![deny(missing_docs)]
//!
//! ```
//! use finance_rs::tvm::{pmt, When};
//! // Monthly payment on a 30-year, 6% mortgage of 300,000.
//! let p = pmt(0.06 / 12.0, 360.0, 300_000.0, 0.0, When::End)?;
//! assert!((p + 1798.65).abs() < 0.01);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

pub mod actuarial;
pub mod amortization;
pub mod attribution;
pub mod calendars;
pub mod cashflow;
pub mod cds;
pub mod corp;
mod date;
pub mod daycount;
#[cfg(feature = "decimal")]
pub mod decimal;
pub mod equity;
mod error;
pub mod factors;
#[cfg(test)]
#[path = "factors_tests.rs"]
mod factors_tests;
pub mod fi;
pub mod fund;
pub mod kelly;
pub mod math;
pub mod mm;
pub mod options;
pub mod perf;
pub mod period;
pub mod pm;
pub mod portfolio;
pub mod rates;
pub mod re;
pub mod schedule;
pub mod sim;
pub mod solve;
pub mod stats;
pub mod tax;
#[cfg(feature = "chrono")]
pub mod timestamp;
pub mod tvm;
pub mod vc;

#[cfg(kani)]
mod proofs;

pub use date::{days_in_month, is_leap_year, Date, Weekday};
pub use error::{FinanceError, Result};

/// The README's examples run as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
