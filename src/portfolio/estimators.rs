use super::matrix::{check_matrix, solve};
use crate::error::{all_finite, output, FinanceError, Result};
use crate::stats;

fn returns_shape(r: &[Vec<f64>], min: usize) -> Result<usize> {
    if r.len() < min {
        return Err(FinanceError::TooShort {
            what: "returns",
            min,
            got: r.len(),
        });
    }
    let n = r.first().map_or(0, Vec::len);
    if n == 0 || r.iter().any(|x| x.len() != n) {
        return Err(FinanceError::InvalidInput {
            what: "returns",
            reason: "must be a nonempty rectangular matrix",
        });
    }
    for row in r {
        all_finite("returns", row)?;
    }
    Ok(n)
}

/// Geometrically compounded historical returns, annualized by observations per year.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn mean_historical(returns: &[Vec<f64>], periods_per_year: f64) -> Result<Vec<f64>> {
    let n = returns_shape(returns, 1)?;
    if !periods_per_year.is_finite() || periods_per_year <= 0. {
        return Err(FinanceError::InvalidInput {
            what: "periods_per_year",
            reason: "must be positive and finite",
        });
    }
    (0..n)
        .map(|j| {
            let log_growth: f64 = returns
                .iter()
                .map(|r| r[j])
                .map(|x| {
                    if x <= -1. {
                        Err(FinanceError::InvalidInput {
                            what: "returns",
                            reason: "must exceed -100% for geometric annualization",
                        })
                    } else {
                        Ok(x.ln_1p())
                    }
                })
                .collect::<Result<Vec<_>>>()?
                .iter()
                .sum();
            output(
                "annualized historical mean",
                (log_growth * periods_per_year / returns.len() as f64).exp_m1(),
            )
        })
        .collect()
}

/// Exponentially weighted means using `span` observations and annualized scale.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn ema_historical(
    returns: &[Vec<f64>],
    periods_per_year: f64,
    span: usize,
) -> Result<Vec<f64>> {
    let n = returns_shape(returns, 1)?;
    if span == 0 || !periods_per_year.is_finite() || periods_per_year <= 0. {
        return Err(FinanceError::InvalidInput {
            what: "EMA parameters",
            reason: "span and periods_per_year must be positive",
        });
    }
    let alpha = 2.0 / (span as f64 + 1.0);
    (0..n)
        .map(|j| {
            let mut x = returns[0][j];
            for row in &returns[1..] {
                x = alpha * row[j] + (1. - alpha) * x;
            }
            if x <= -1. {
                return Err(FinanceError::InvalidInput {
                    what: "EMA return",
                    reason: "must exceed -100% for geometric annualization",
                });
            }
            output("annualized EMA", (x.ln_1p() * periods_per_year).exp_m1())
        })
        .collect()
}

/// CAPM expected returns: annual risk-free rate plus beta times market premium.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn capm(
    asset_returns: &[Vec<f64>],
    market_returns: &[f64],
    risk_free: f64,
    periods_per_year: f64,
) -> Result<Vec<f64>> {
    if asset_returns.len() != market_returns.len() {
        return Err(FinanceError::LengthMismatch {
            left: "asset_returns",
            right: "market_returns",
            left_len: asset_returns.len(),
            right_len: market_returns.len(),
        });
    }
    let n = returns_shape(asset_returns, 2)?;
    all_finite("market_returns", market_returns)?;
    all_finite("risk_free", &[risk_free, periods_per_year])?;
    if periods_per_year <= 0. {
        return Err(FinanceError::InvalidInput {
            what: "periods_per_year",
            reason: "must be positive",
        });
    }
    let mr = stats::mean(market_returns)? * periods_per_year;
    let rf_period = risk_free / periods_per_year;
    let mut out = Vec::with_capacity(n);
    for j in 0..n {
        let x: Vec<_> = asset_returns.iter().map(|r| r[j]).collect();
        let beta = stats::covariance(&x, market_returns, 1)? / stats::variance(market_returns, 1)?;
        let expected_return = output("CAPM expected return", risk_free + beta * (mr - risk_free))?;
        out.push(expected_return);
    }
    let _ = rf_period;
    Ok(out)
}

/// Sample covariance matrix, annualized by periods per year.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn sample(returns: &[Vec<f64>], periods_per_year: f64) -> Result<Vec<Vec<f64>>> {
    if !periods_per_year.is_finite() || periods_per_year <= 0. {
        return Err(FinanceError::InvalidInput {
            what: "periods_per_year",
            reason: "must be positive",
        });
    }
    let n = returns_shape(returns, 2)?;
    let cols: Vec<Vec<f64>> = (0..n)
        .map(|j| returns.iter().map(|r| r[j]).collect())
        .collect();
    let mut out = vec![vec![0.; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let v = stats::covariance(&cols[i], &cols[j], 1)? * periods_per_year;
            out[i][j] = v;
            out[j][i] = v;
        }
    }
    Ok(out)
}

/// Exponentially weighted covariance with `span` observations, annualized.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn exponential(
    returns: &[Vec<f64>],
    periods_per_year: f64,
    span: usize,
) -> Result<Vec<Vec<f64>>> {
    let n = returns_shape(returns, 2)?;
    if span == 0 || !periods_per_year.is_finite() || periods_per_year <= 0. {
        return Err(FinanceError::InvalidInput {
            what: "exponential covariance parameters",
            reason: "span and periods_per_year must be positive",
        });
    }
    let alpha = 2. / (span as f64 + 1.);
    let mut means = vec![0.; n];
    let mut cov = vec![vec![0.; n]; n];
    for row in returns {
        let prev = means.clone();
        for j in 0..n {
            means[j] = alpha * row[j] + (1. - alpha) * means[j];
        }
        for i in 0..n {
            for j in 0..n {
                cov[i][j] =
                    (1. - alpha) * (cov[i][j] + alpha * (row[i] - prev[i]) * (row[j] - prev[j]));
            }
        }
    }
    for row in &mut cov {
        for x in row {
            *x *= periods_per_year;
        }
    }
    Ok(cov)
}

/// Downside covariance around a per-period benchmark, annualized.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn semicovariance(
    returns: &[Vec<f64>],
    benchmark: f64,
    periods_per_year: f64,
) -> Result<Vec<Vec<f64>>> {
    let n = returns_shape(returns, 1)?;
    all_finite("semicovariance parameters", &[benchmark, periods_per_year])?;
    if periods_per_year <= 0. {
        return Err(FinanceError::InvalidInput {
            what: "periods_per_year",
            reason: "must be positive",
        });
    }
    let mut out = vec![vec![0.; n]; n];
    for row in returns {
        let d: Vec<_> = row.iter().map(|x| (x - benchmark).min(0.)).collect();
        for i in 0..n {
            for j in 0..n {
                out[i][j] += d[i] * d[j] / returns.len() as f64 * periods_per_year;
            }
        }
    }
    Ok(out)
}

/// Linear shrinkage of sample covariance toward its diagonal.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
#[allow(clippy::needless_range_loop)] // Indexed iteration keeps row and column coordinates aligned in this matrix algorithm.
pub fn ledoit_wolf(returns: &[Vec<f64>], periods_per_year: f64) -> Result<Vec<Vec<f64>>> {
    let mut s = sample(returns, periods_per_year)?;
    let n = s.len();
    let p = returns_shape(returns, 2)?;
    let mut off = 0.;
    let mut diag = 0.;
    for i in 0..n {
        diag += s[i][i];
        for j in 0..n {
            if i != j {
                off += s[i][j] * s[i][j];
            }
        }
    }
    let denom = off + diag * diag / (n * n) as f64;
    let shrink = if denom == 0. {
        1.
    } else {
        (n as f64 / (p as f64 + n as f64)).clamp(0., 1.)
    };
    for i in 0..n {
        for j in 0..n {
            if i != j {
                s[i][j] *= 1. - shrink;
            }
        }
    }
    Ok(s)
}

/// Oracle-approximating shrinkage toward an identity-scaled covariance target.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
#[allow(clippy::needless_range_loop)] // Indexed iteration keeps row and column coordinates aligned in this matrix algorithm.
pub fn oracle_approximating(returns: &[Vec<f64>], periods_per_year: f64) -> Result<Vec<Vec<f64>>> {
    let mut s = sample(returns, periods_per_year)?;
    let n = s.len();
    let mu = (0..n).map(|i| s[i][i]).sum::<f64>() / n as f64;
    let p = returns.len() as f64;
    let tr = s.iter().enumerate().map(|(i, r)| r[i]).sum::<f64>();
    let tr2 = s
        .iter()
        .map(|r| r.iter().map(|x| x * x).sum::<f64>())
        .sum::<f64>();
    let num = (1. - 2. / n as f64) * tr2 + tr * tr;
    let den = (p + 1. - 2. / n as f64) * (tr2 - tr * tr / n as f64);
    let shrink = if den <= 0. {
        1.
    } else {
        (num / den).clamp(0., 1.)
    };
    for i in 0..n {
        for j in 0..n {
            s[i][j] = (1. - shrink) * s[i][j] + if i == j { shrink * mu } else { 0. };
        }
    }
    Ok(s)
}

#[path = "estimators/spectral.rs"]
mod spectral;
pub use spectral::{black_litterman, fix_nonpositive_semidefinite, market_implied_returns};
