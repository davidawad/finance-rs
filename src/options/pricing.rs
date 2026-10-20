//! European option prices, including equity, FX, futures and normal models.
use super::{check_positive, nonnegative, validate};
use crate::{math, FinanceError, Result};

/// Call or put payoff direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
pub enum OptionKind {
    /// The call option.
    Call,
    /// The put option.
    Put,
}

impl OptionKind {
    pub(crate) fn sign(self) -> f64 {
        if matches!(self, Self::Call) {
            1.0
        } else {
            -1.0
        }
    }
    pub(crate) fn payoff(self, spot: f64, strike: f64) -> f64 {
        (self.sign() * (spot - strike)).max(0.0)
    }
}

pub(super) fn d1_d2(forward: f64, strike: f64, t: f64, sigma: f64) -> Result<(f64, f64)> {
    let inputs = [
        ("forward", forward),
        ("strike", strike),
        ("time", t),
        ("volatility", sigma),
    ];
    validate(&inputs)?;
    nonnegative("time", t)?;
    nonnegative("volatility", sigma)?;
    check_positive("forward", forward)?;
    check_positive("strike", strike)?;
    if t == 0.0 || sigma == 0.0 {
        return Err(FinanceError::Undefined {
            what: "d1 and d2 at zero time or volatility",
        });
    }
    let st = sigma * t.sqrt();
    let d1 = ((forward / strike).ln() + 0.5 * st * st) / st;
    Ok((d1, d1 - st))
}

/// Black-Scholes-Merton price with continuous dividend yield.
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn bsm(
    kind: OptionKind,
    spot: f64,
    strike: f64,
    t: f64,
    rate: f64,
    dividend: f64,
    sigma: f64,
) -> Result<f64> {
    validate(&[
        ("spot", spot),
        ("strike", strike),
        ("time", t),
        ("volatility", sigma),
        ("rate", rate),
        ("dividend", dividend),
    ])?;
    nonnegative("time", t)?;
    nonnegative("volatility", sigma)?;
    check_positive("spot", spot)?;
    check_positive("strike", strike)?;
    if t == 0.0 {
        return Ok(kind.payoff(spot, strike));
    }
    if sigma == 0.0 {
        let fwd = spot * ((rate - dividend) * t).exp();
        return crate::error::output("BSM price", (-rate * t).exp() * kind.payoff(fwd, strike));
    }
    let forward = spot * ((rate - dividend) * t).exp();
    let (d1, d2) = d1_d2(forward, strike, t, sigma)?;
    let sign = kind.sign();
    let value = if sign > 0.0 {
        spot * (-dividend * t).exp() * math::normal_cdf(d1)
            - strike * (-rate * t).exp() * math::normal_cdf(d2)
    } else {
        strike * (-rate * t).exp() * math::normal_cdf(-d2)
            - spot * (-dividend * t).exp() * math::normal_cdf(-d1)
    };
    crate::error::output("BSM price", value)
}

/// Black-76 price for an option on a forward/futures price.
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn black76(
    kind: OptionKind,
    forward: f64,
    strike: f64,
    t: f64,
    rate: f64,
    sigma: f64,
) -> Result<f64> {
    validate(&[
        ("forward", forward),
        ("strike", strike),
        ("time", t),
        ("volatility", sigma),
        ("rate", rate),
    ])?;
    nonnegative("time", t)?;
    nonnegative("volatility", sigma)?;
    check_positive("forward", forward)?;
    check_positive("strike", strike)?;
    if t == 0.0 {
        return Ok((-rate * t).exp() * kind.payoff(forward, strike));
    }
    if sigma == 0.0 {
        return crate::error::output(
            "Black-76 price",
            (-rate * t).exp() * kind.payoff(forward, strike),
        );
    }
    let (d1, d2) = d1_d2(forward, strike, t, sigma)?;
    let sign = kind.sign();
    let value = if sign > 0.0 {
        (-rate * t).exp() * (forward * math::normal_cdf(d1) - strike * math::normal_cdf(d2))
    } else {
        (-rate * t).exp() * (strike * math::normal_cdf(-d2) - forward * math::normal_cdf(-d1))
    };
    crate::error::output("Black-76 price", value)
}

/// Bachelier (normal volatility) price. `sigma_n` is price units per √year.
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn bachelier(
    kind: OptionKind,
    forward: f64,
    strike: f64,
    t: f64,
    rate: f64,
    sigma_n: f64,
) -> Result<f64> {
    validate(&[
        ("forward", forward),
        ("strike", strike),
        ("time", t),
        ("normal volatility", sigma_n),
        ("rate", rate),
    ])?;
    nonnegative("time", t)?;
    nonnegative("normal volatility", sigma_n)?;
    if t == 0.0 || sigma_n == 0.0 {
        return crate::error::output(
            "Bachelier price",
            (-rate * t).exp() * kind.payoff(forward, strike),
        );
    }
    let st = sigma_n * t.sqrt();
    let d = (forward - strike) / st;
    let value = (-rate * t).exp()
        * (kind.sign() * (forward - strike) * math::normal_cdf(kind.sign() * d)
            + st * math::normal_pdf(d));
    crate::error::output("Bachelier price", value)
}

/// Garman-Kohlhagen FX option price (domestic and foreign rates).
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn garman_kohlhagen(
    kind: OptionKind,
    spot: f64,
    strike: f64,
    t: f64,
    domestic_rate: f64,
    foreign_rate: f64,
    sigma: f64,
) -> Result<f64> {
    bsm(kind, spot, strike, t, domestic_rate, foreign_rate, sigma)
}

/// Margrabe exchange-option price, with correlated lognormal assets.
// Its signature follows the nine independent inputs in the exchange-option formula.
#[allow(clippy::too_many_arguments)]
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn margrabe(
    kind: OptionKind,
    s1: f64,
    s2: f64,
    t: f64,
    q1: f64,
    q2: f64,
    sigma1: f64,
    sigma2: f64,
    rho: f64,
) -> Result<f64> {
    validate(&[
        ("s1", s1),
        ("s2", s2),
        ("time", t),
        ("q1", q1),
        ("q2", q2),
        ("sigma1", sigma1),
        ("sigma2", sigma2),
        ("correlation", rho),
    ])?;
    nonnegative("time", t)?;
    nonnegative("sigma1", sigma1)?;
    nonnegative("sigma2", sigma2)?;
    check_positive("s1", s1)?;
    check_positive("s2", s2)?;
    if !(-1.0..=1.0).contains(&rho) {
        return Err(FinanceError::InvalidInput {
            what: "correlation",
            reason: "must be in [-1, 1]",
        });
    }
    let variance = sigma1 * sigma1 + sigma2 * sigma2 - 2.0 * rho * sigma1 * sigma2;
    let vol = variance.max(0.0).sqrt();
    if t == 0.0 || vol == 0.0 {
        let a = s1 * (-q1 * t).exp();
        let b = s2 * (-q2 * t).exp();
        return crate::error::output("Margrabe price", kind.payoff(a, b));
    }
    let forward = s1 * (-q1 * t).exp() / (s2 * (-q2 * t).exp());
    let value_call = s1
        * (-q1 * t).exp()
        * math::normal_cdf((forward.ln() + 0.5 * vol * vol * t) / (vol * t.sqrt()))
        - s2 * (-q2 * t).exp()
            * math::normal_cdf((forward.ln() - 0.5 * vol * vol * t) / (vol * t.sqrt()));
    crate::error::output(
        "Margrabe price",
        if matches!(kind, OptionKind::Call) {
            value_call
        } else {
            value_call + s2 * (-q2 * t).exp() - s1 * (-q1 * t).exp()
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn textbook_prices_and_put_call_parity() -> Result<()> {
        let c = bsm(OptionKind::Call, 100.0, 100.0, 1.0, 0.05, 0.0, 0.2)?;
        let p = bsm(OptionKind::Put, 100.0, 100.0, 1.0, 0.05, 0.0, 0.2)?;
        assert!((c - 10.450_583_572_185_565).abs() < 1e-10);
        assert!((c - p - (100.0 - 100.0 * (-0.05f64).exp())).abs() < 1e-12);
        assert!(
            (black76(OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.2)? - 7.965_567_455_405_804).abs()
                < 1e-10
        );
        assert!(
            (bachelier(OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 10.0)? - 3.989_422_804_014_327)
                .abs()
                < 1e-12
        );
        Ok(())
    }
    #[test]
    fn degenerate_and_invalid_inputs_are_typed() {
        assert_eq!(
            bsm(OptionKind::Call, 90.0, 100.0, 0.0, 0.04, 0.0, 0.2).unwrap(),
            0.0
        );
        assert!(bsm(OptionKind::Call, -1.0, 100.0, 1.0, 0.0, 0.0, 0.2).is_err());
        assert!(bsm(OptionKind::Call, 100.0, 100.0, f64::NAN, 0.0, 0.0, 0.2).is_err());
        assert!(d1_d2(100.0, 100.0, 0.0, 0.2).is_err());
        assert!(d1_d2(100.0, 100.0, 1.0, 0.0).is_err());
    }

    #[test]
    fn deterministic_puts_fx_and_exchange_option_limits() -> Result<()> {
        assert_eq!(
            bsm(OptionKind::Call, 110.0, 100.0, 0.0, 0.03, 0.01, 0.2)?,
            10.0
        );
        assert!((bsm(OptionKind::Put, 90.0, 100.0, 1.0, 0.0, 0.0, 0.0)? - 10.0).abs() < 1e-12);
        assert!((black76(OptionKind::Put, 90.0, 100.0, 0.0, 0.0, 0.2)? - 10.0).abs() < 1e-12);
        assert!((black76(OptionKind::Call, 110.0, 100.0, 1.0, 0.0, 0.0)? - 10.0).abs() < 1e-12);
        assert_eq!(
            bachelier(OptionKind::Put, 90.0, 100.0, 0.0, 0.0, 5.0)?,
            10.0
        );
        assert!((bachelier(OptionKind::Call, 110.0, 100.0, 1.0, 0.0, 0.0)? - 10.0).abs() < 1e-12);
        assert!(garman_kohlhagen(OptionKind::Call, 1.1, 1.0, 1.0, 0.03, 0.01, 0.2)?.is_finite());
        assert_eq!(
            margrabe(OptionKind::Call, 110.0, 100.0, 0.0, 0.0, 0.0, 0.2, 0.2, 0.0)?,
            10.0
        );
        assert!(
            (margrabe(OptionKind::Put, 90.0, 100.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0)? - 10.0).abs()
                < 1e-12
        );
        assert!(margrabe(OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2, 0.2, 1.1).is_err());
        assert!(black76(OptionKind::Call, 0.0, 100.0, 1.0, 0.0, 0.2).is_err());
        assert!(bachelier(OptionKind::Call, 100.0, 100.0, 1.0, 0.0, -1.0).is_err());
        let exchange_call = exchange_price(OptionKind::Call)?;
        let exchange_put = exchange_price(OptionKind::Put)?;
        assert!(exchange_call.is_finite() && exchange_put.is_finite());
        assert!(margrabe(OptionKind::Call, 0.0, 100.0, 1.0, 0.0, 0.0, 0.2, 0.2, 0.0).is_err());
        assert!(bsm(OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, f64::INFINITY).is_err());
        Ok(())
    }

    fn exchange_price(kind: OptionKind) -> Result<f64> {
        margrabe(kind, 105.0, 100.0, 1.0, 0.01, 0.02, 0.25, 0.2, 0.3)
    }
}
