//! Analytic first-order Greeks. Vega and rho are per unit annual volatility/rate;
//! theta is per year (unlike `py_vollib`'s per 1% / per day presentation).
use super::{
    nonnegative,
    pricing::{d1_d2, OptionKind},
    validate,
};
use crate::{math, FinanceError, Result};

/// First order sensitivities for a European vanilla option.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
pub struct Greeks {
    /// Price change for a one unit increase in spot (or forward for Black-76).
    pub delta: f64,
    /// Second derivative with respect to spot/forward.
    pub gamma: f64,
    /// Price change for a one unit increase in annual volatility.
    pub vega: f64,
    /// Price change for a one unit increase in the annual risk-free rate.
    pub rho: f64,
    /// Calendar-time decay per year.
    pub theta: f64,
}

fn finite_greeks(g: Greeks) -> Result<Greeks> {
    Ok(Greeks {
        delta: crate::error::output("option delta", g.delta)?,
        gamma: crate::error::output("option gamma", g.gamma)?,
        vega: crate::error::output("option vega", g.vega)?,
        rho: crate::error::output("option rho", g.rho)?,
        theta: crate::error::output("option theta", g.theta)?,
    })
}

/// BSM Greeks for an equity option, with continuous dividend yield.
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn bsm_greeks(
    kind: OptionKind,
    spot: f64,
    strike: f64,
    t: f64,
    rate: f64,
    dividend: f64,
    sigma: f64,
) -> Result<Greeks> {
    validate(&[
        ("spot", spot),
        ("strike", strike),
        ("time", t),
        ("rate", rate),
        ("dividend", dividend),
        ("volatility", sigma),
    ])?;
    nonnegative("time", t)?;
    nonnegative("volatility", sigma)?;
    if spot <= 0.0 || strike <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "spot/strike",
            reason: "must be positive",
        });
    }
    if t == 0.0 || sigma == 0.0 {
        let delta = if kind.sign() * (spot - strike) > 0.0 {
            kind.sign()
        } else {
            0.0
        };
        return finite_greeks(Greeks {
            delta,
            gamma: 0.0,
            vega: 0.0,
            rho: 0.0,
            theta: 0.0,
        });
    }
    let fwd = spot * ((rate - dividend) * t).exp();
    let (d1, d2) = d1_d2(fwd, strike, t, sigma)?;
    let sign = kind.sign();
    let root_t = t.sqrt();
    let dq = (-dividend * t).exp();
    let dr = (-rate * t).exp();
    let pdf = math::normal_pdf(d1);
    let delta = dq
        * (if sign > 0.0 {
            math::normal_cdf(d1)
        } else {
            math::normal_cdf(d1) - 1.0
        });
    let gamma = dq * pdf / (spot * sigma * root_t);
    let vega = spot * dq * pdf * root_t;
    let rho = sign * strike * t * dr * math::normal_cdf(sign * d2);
    let theta = -spot * dq * pdf * sigma / (2.0 * root_t)
        - sign * rate * strike * dr * math::normal_cdf(sign * d2)
        + sign * dividend * spot * dq * math::normal_cdf(sign * d1);
    finite_greeks(Greeks {
        delta,
        gamma,
        vega,
        rho,
        theta,
    })
}

/// Black-76 Greeks with respect to the forward price.
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn black76_greeks(
    kind: OptionKind,
    forward: f64,
    strike: f64,
    t: f64,
    rate: f64,
    sigma: f64,
) -> Result<Greeks> {
    validate(&[
        ("forward", forward),
        ("strike", strike),
        ("time", t),
        ("rate", rate),
        ("volatility", sigma),
    ])?;
    nonnegative("time", t)?;
    nonnegative("volatility", sigma)?;
    if forward <= 0.0 || strike <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "forward/strike",
            reason: "must be positive",
        });
    }
    if t == 0.0 || sigma == 0.0 {
        return finite_greeks(Greeks {
            delta: if kind.sign() * (forward - strike) > 0.0 {
                kind.sign()
            } else {
                0.0
            },
            gamma: 0.0,
            vega: 0.0,
            rho: 0.0,
            theta: 0.0,
        });
    }
    let (d1, d2) = d1_d2(forward, strike, t, sigma)?;
    let sign = kind.sign();
    let rt = t.sqrt();
    let disc = (-rate * t).exp();
    let pdf = math::normal_pdf(d1);
    let delta = disc * sign * math::normal_cdf(sign * d1);
    let gamma = disc * pdf / (forward * sigma * rt);
    let vega = disc * forward * pdf * rt;
    let rho = -t
        * disc
        * sign
        * (forward * sign * math::normal_cdf(sign * d1) - strike * math::normal_cdf(sign * d2));
    let theta = rate
        * disc
        * (forward * sign * math::normal_cdf(sign * d1) - strike * math::normal_cdf(sign * d2))
        - disc * forward * pdf * sigma / (2.0 * rt);
    finite_greeks(Greeks {
        delta,
        gamma,
        vega,
        rho,
        theta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn greeks_match_textbook_and_finite_differences() -> Result<()> {
        let g = bsm_greeks(OptionKind::Call, 100.0, 100.0, 1.0, 0.05, 0.0, 0.2)?;
        assert!((g.delta - 0.636_830_651_175_619_1).abs() < 1e-12);
        assert!((g.gamma - 0.018_762_017_345_846_895).abs() < 1e-12);
        assert!((g.vega - 37.524_034_691_693_79).abs() < 1e-10);
        let h = 1e-3;
        let up = crate::options::bsm(OptionKind::Call, 100.0 + h, 100.0, 1.0, 0.05, 0.0, 0.2)?;
        let dn = crate::options::bsm(OptionKind::Call, 100.0 - h, 100.0, 1.0, 0.05, 0.0, 0.2)?;
        assert!(((up - dn) / (2.0 * h) - g.delta).abs() < 1e-7);
        Ok(())
    }

    #[test]
    fn greeks_cover_puts_expiry_zero_volatility_and_validation() -> Result<()> {
        let put = bsm_greeks(OptionKind::Put, 90.0, 100.0, 1.0, 0.03, 0.01, 0.25)?;
        assert!(put.delta < 0.0);
        assert!(put.gamma > 0.0);
        assert!(put.vega > 0.0);
        assert!(put.rho < 0.0);

        let expired = bsm_greeks(OptionKind::Call, 110.0, 100.0, 0.0, 0.03, 0.01, 0.25)?;
        assert_eq!(expired.delta, 1.0);
        assert_eq!(expired.gamma, 0.0);
        let deterministic = bsm_greeks(OptionKind::Put, 90.0, 100.0, 1.0, 0.03, 0.01, 0.0)?;
        assert_eq!(deterministic.delta, -1.0);

        assert!(bsm_greeks(OptionKind::Call, 0.0, 100.0, 1.0, 0.03, 0.0, 0.2).is_err());
        assert!(bsm_greeks(OptionKind::Call, 100.0, 100.0, -1.0, 0.03, 0.0, 0.2).is_err());
        assert!(bsm_greeks(OptionKind::Call, 100.0, 100.0, 1.0, 0.03, 0.0, f64::NAN).is_err());

        let future = black76_greeks(OptionKind::Put, 90.0, 100.0, 1.0, 0.03, 0.25)?;
        assert!(future.delta < 0.0);
        assert!(future.vega > 0.0);
        let expired = black76_greeks(OptionKind::Call, 110.0, 100.0, 0.0, 0.03, 0.25)?;
        assert_eq!(expired.delta, 1.0);
        assert!(black76_greeks(OptionKind::Call, 0.0, 100.0, 1.0, 0.03, 0.2).is_err());
        assert!(black76_greeks(OptionKind::Call, 100.0, 100.0, 1.0, 0.03, -0.2).is_err());
        Ok(())
    }

    #[test]
    fn finite_greeks_rejects_non_finite_sensitivities() {
        let result = finite_greeks(Greeks {
            delta: f64::NAN,
            gamma: 0.0,
            vega: 0.0,
            rho: 0.0,
            theta: 0.0,
        });
        assert!(result.is_err());
    }
}
