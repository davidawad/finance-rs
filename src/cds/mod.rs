//! Credit default swap conventions, hazard curves, pricing and risk.
//!
//! The implementation uses quarterly ACT/360 premiums, IMM-20 accrual dates,
//! following-adjusted weekday payments, continuous flat discounting,
//! piecewise-constant hazard rates, and ISDA step-in/half-day accrued premium
//! integration. Term discount curves and holiday calendars are not modeled.

use crate::Result;

mod curve;
mod pricing;
mod pricing_legs;
mod types;

pub use curve::{CreditCurve, HazardNode, ParSpreadQuote};
pub use pricing::{
    basis, carry_and_roll, implied_spread, running_from_upfront, upfront_from_running, CdsRisk,
    CdsValuation,
};
pub use types::{next_imm_date, standard_coupon_amount, CdsInstrument, CdsKind, StandardCoupon};

/// A clearly labeled CDS-style estimate derived from a bond spread proxy.
/// It is not an observed CDS quote or an issuer-specific curve.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BondSpreadProxy {
    /// Input spread in annual decimal units.
    pub spread: f64,
    /// Flat hazard-rate approximation.
    pub hazard_rate: f64,
    /// Cumulative probability over the selected horizon.
    pub cumulative_default_probability: f64,
    /// Recovery assumption used in the approximation.
    pub recovery: f64,
    /// Projection horizon in years.
    pub horizon_years: f64,
}

/// Convert a bond spread in annual decimal units to a flat CDS-style proxy.
/// The result is an approximation and must be presented as a proxy.
///
/// # Errors
/// Returns an error when the spread, recovery, or horizon is invalid.
pub fn bond_spread_proxy(
    spread: f64,
    recovery: f64,
    horizon_years: f64,
) -> Result<BondSpreadProxy> {
    if !spread.is_finite() || spread < 0.0 {
        return Err(crate::FinanceError::InvalidInput {
            what: "bond spread",
            reason: "must be finite and non-negative",
        });
    }
    if !recovery.is_finite() || !(0.0..1.0).contains(&recovery) {
        return Err(crate::FinanceError::InvalidInput {
            what: "recovery",
            reason: "must be in [0, 1)",
        });
    }
    if !horizon_years.is_finite() || horizon_years <= 0.0 {
        return Err(crate::FinanceError::InvalidInput {
            what: "horizon",
            reason: "must be finite and positive",
        });
    }
    let hazard_rate = spread / (1.0 - recovery);
    let cumulative_default_probability = 1.0 - (-hazard_rate * horizon_years).exp();
    Ok(BondSpreadProxy {
        spread,
        hazard_rate,
        cumulative_default_probability,
        recovery,
        horizon_years,
    })
}

/// Convert a percent-valued bond spread (for example FRED's 0.92 percent)
/// before applying [`bond_spread_proxy`].
///
/// # Errors
/// Returns an error when the spread, recovery, or horizon is invalid.
pub fn bond_spread_proxy_from_percent(
    spread_percent: f64,
    recovery: f64,
    horizon_years: f64,
) -> Result<BondSpreadProxy> {
    bond_spread_proxy(spread_percent / 100.0, recovery, horizon_years)
}

#[cfg(test)]
mod proxy_tests {
    use super::bond_spread_proxy;

    #[test]
    fn bond_spread_proxy_is_labeled_math_with_validated_inputs() {
        let proxy = bond_spread_proxy(0.01, 0.4, 5.0).unwrap();
        assert!((proxy.hazard_rate - 1.0 / 60.0).abs() < 1e-12);
        assert!(
            (proxy.cumulative_default_probability - (1.0 - (-1.0_f64 / 12.0).exp())).abs() < 1e-12
        );
        assert!(bond_spread_proxy(-0.01, 0.4, 5.0).is_err());
        assert!(bond_spread_proxy(f64::NAN, 0.4, 5.0).is_err());
        assert!(bond_spread_proxy(0.01, 1.0, 5.0).is_err());
        assert!(bond_spread_proxy(0.01, 0.4, 0.0).is_err());
        let fred_proxy = super::bond_spread_proxy_from_percent(0.92, 0.4, 5.0).unwrap();
        assert!((fred_proxy.spread - 0.0092).abs() < 1e-12);
        assert!(super::bond_spread_proxy_from_percent(-0.92, 0.4, 5.0).is_err());
    }
}
