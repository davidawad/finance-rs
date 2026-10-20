//! Portfolio construction inputs, estimators, risk budgets and rebalancing.
//!
//! The constrained efficient frontier is intentionally composed with an
//! external QP solver; this module owns the solver-independent inputs.

mod estimators;
mod matrix;
mod operations;
mod weight_impl;

pub use estimators::{
    black_litterman, capm, ema_historical, exponential, fix_nonpositive_semidefinite, ledoit_wolf,
    market_implied_returns, mean_historical, oracle_approximating, sample, semicovariance,
};
pub use matrix::{correlation_matrix, covariance_matrix};
pub use operations::{
    concentration_hhi, discrete_allocation, diversification_ratio, drift, gross_leverage,
    kelly_portfolio, needs_rebalance, rebalance_trades, return_, risk_budget_allocations,
    risk_contributions, transaction_cost, turnover, variance, vol_target_leverage, volatility,
};
pub use weight_impl::{
    equal_risk_contribution, hrp, inverse_volatility, min_variance_unconstrained,
    tangency_unconstrained,
};

/// Expected-return estimators.
pub mod expected_returns {
    pub use super::estimators::{capm, ema_historical, mean_historical};
}
/// Covariance estimators and matrix repair.
pub mod cov {
    pub use super::estimators::{
        exponential, fix_nonpositive_semidefinite, ledoit_wolf, oracle_approximating, sample,
        semicovariance,
    };
}
/// Portfolio weight construction methods.
pub mod weights {
    pub use super::weight_impl::{
        equal_risk_contribution, hrp, inverse_volatility, min_variance_unconstrained,
        tangency_unconstrained,
    };
}
/// Rebalancing calculations.
pub mod rebalance {
    pub use super::operations::{drift, needs_rebalance, rebalance_trades};
}

/// Rebalancing band interpretation.
#[derive(Debug, Clone, Copy, PartialEq)]
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
pub enum Band {
    /// Absolute weight difference (e.g. `0.05` for five percentage points).
    Absolute(f64),
    /// Relative deviation from target (e.g. `0.2` for twenty percent).
    Relative(f64),
}
