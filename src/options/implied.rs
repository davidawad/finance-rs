//! Implied-volatility inversion for Black, BSM and Bachelier prices.
use super::{bachelier, black76, bsm, nonnegative, OptionKind};
use crate::{
    solve::{brent, SolverOptions},
    FinanceError, Result,
};

fn implied(
    target: f64,
    low_price: f64,
    pricer: impl Fn(f64) -> Result<f64>,
    upper: f64,
) -> Result<f64> {
    crate::error::finite("price", target)?;
    if target < low_price - 1e-12 || target >= upper {
        return Err(FinanceError::InvalidInput {
            what: "price",
            reason: "outside the no-arbitrage price bounds",
        });
    }
    let f = |sigma: f64| pricer(sigma).map_or(f64::NAN, |p| p - target);
    let hi = 10.0;
    let fhi = f(hi);
    if !fhi.is_finite() || fhi < 0.0 {
        return Err(FinanceError::NoSolution {
            what: "implied volatility",
        });
    }
    let opts = SolverOptions {
        xtol: 1e-12,
        ftol: 1e-12,
        max_iter: 250,
    };
    brent(f, 1e-12, hi, &opts).map(|x| x.root)
}

/// BSM implied volatility from an observed European option price.
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn bsm_implied_vol(
    kind: OptionKind,
    price: f64,
    spot: f64,
    strike: f64,
    t: f64,
    rate: f64,
    dividend: f64,
) -> Result<f64> {
    crate::error::finite("price", price)?;
    nonnegative("time", t)?;
    if spot <= 0.0 || strike <= 0.0 || t == 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "spot/strike/time",
            reason: "spot and strike must be positive and time non-zero",
        });
    }
    let intrinsic = (-rate * t).exp() * kind.payoff(spot * ((rate - dividend) * t).exp(), strike);
    let upper = if matches!(kind, OptionKind::Call) {
        spot * (-dividend * t).exp()
    } else {
        strike * (-rate * t).exp()
    };
    implied(
        price,
        intrinsic,
        |v| bsm(kind, spot, strike, t, rate, dividend, v),
        upper,
    )
}

/// Black-76 implied volatility from an observed option price.
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn black76_implied_vol(
    kind: OptionKind,
    price: f64,
    forward: f64,
    strike: f64,
    t: f64,
    rate: f64,
) -> Result<f64> {
    crate::error::finite("price", price)?;
    nonnegative("time", t)?;
    if forward <= 0.0 || strike <= 0.0 || t == 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "forward/strike/time",
            reason: "forward and strike must be positive and time non-zero",
        });
    }
    let disc = (-rate * t).exp();
    let intrinsic = disc * kind.payoff(forward, strike);
    let upper = disc
        * if matches!(kind, OptionKind::Call) {
            forward
        } else {
            strike
        };
    implied(
        price,
        intrinsic,
        |v| black76(kind, forward, strike, t, rate, v),
        upper,
    )
}

/// Bachelier normal implied volatility from an observed option price.
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn bachelier_implied_vol(
    kind: OptionKind,
    price: f64,
    forward: f64,
    strike: f64,
    t: f64,
    rate: f64,
) -> Result<f64> {
    crate::error::finite("price", price)?;
    nonnegative("time", t)?;
    if t == 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "time",
            reason: "must be positive",
        });
    }
    let disc = (-rate * t).exp();
    let intrinsic = disc * kind.payoff(forward, strike);
    implied(
        price,
        intrinsic,
        |v| bachelier(kind, forward, strike, t, rate, v),
        f64::MAX,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn implied_vol_round_trips_prices() -> Result<()> {
        for kind in [OptionKind::Call, OptionKind::Put] {
            let p = bsm(kind, 100., 105., 0.75, 0.04, 0.01, 0.32)?;
            assert!((bsm_implied_vol(kind, p, 100., 105., 0.75, 0.04, 0.01)? - 0.32).abs() < 1e-9);
            let q = black76(kind, 99., 100., 1.5, 0.03, 0.21)?;
            assert!((black76_implied_vol(kind, q, 99., 100., 1.5, 0.03)? - 0.21).abs() < 1e-9);
        }
        let p = bachelier(OptionKind::Call, 100., 101., 1., 0.02, 8.)?;
        assert!(
            (bachelier_implied_vol(OptionKind::Call, p, 100., 101., 1., 0.02)? - 8.).abs() < 1e-8
        );
        Ok(())
    }
    #[test]
    fn invalid_price_bounds_fail() {
        assert!(bsm_implied_vol(OptionKind::Call, -1., 100., 100., 1., 0., 0.).is_err());
    }
}
