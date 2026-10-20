//! Closed-form option pricing, risk, payoff and volatility estimators.
//!
//! Rates and volatilities are annual decimals, time is years, and prices are
//! per underlying unit. Stochastic and exotic engines are composed externally.

mod greeks;
mod implied;
mod pricing;
mod volatility;

pub use greeks::{black76_greeks, bsm_greeks, Greeks};
pub use implied::{bachelier_implied_vol, black76_implied_vol, bsm_implied_vol};
pub use pricing::{bachelier, black76, bsm, garman_kohlhagen, margrabe, OptionKind};
pub use volatility::{close_to_close, ewma, garman_klass, parkinson, rogers_satchell, yang_zhang};

use crate::{FinanceError, Result};

pub(crate) fn validate(values: &[(&'static str, f64)]) -> Result<()> {
    for (what, value) in values {
        crate::error::finite(what, *value)?;
    }
    Ok(())
}

pub(crate) fn nonnegative(what: &'static str, value: f64) -> Result<()> {
    crate::error::finite(what, value)?;
    if value < 0.0 {
        return Err(FinanceError::InvalidInput {
            what,
            reason: "must be non-negative",
        });
    }
    Ok(())
}

pub(crate) fn check_positive(what: &'static str, value: f64) -> Result<()> {
    crate::error::finite(what, value)?;
    if value <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what,
            reason: "must be positive",
        });
    }
    Ok(())
}
