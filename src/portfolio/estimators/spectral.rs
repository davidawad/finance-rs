use super::{check_matrix, solve};
use crate::error::{all_finite, FinanceError, Result};
use crate::portfolio::matrix;

/// Symmetric eigenvalue clipping to the nearest positive-semidefinite matrix.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::needless_range_loop)] // Indexed iteration keeps row and column coordinates aligned in this matrix algorithm.
#[allow(clippy::many_single_char_names)] // Conventional symbols follow the published formula or recurrence notation.
pub fn fix_nonpositive_semidefinite(cov: &[Vec<f64>]) -> Result<Vec<Vec<f64>>> {
    let n = cov.len();
    check_matrix("covariance", cov, n)?;
    if n == 0 {
        return Err(FinanceError::TooShort {
            what: "covariance",
            min: 1,
            got: 0,
        });
    }
    let mut a = cov.to_vec();
    let mut v = vec![vec![0.; n]; n];
    for i in 0..n {
        v[i][i] = 1.;
    }
    for _ in 0..(50 * n * n) {
        let mut p = 0;
        let mut q = 0;
        let mut max = 0.;
        for i in 0..n {
            for j in i + 1..n {
                if a[i][j].abs() > max {
                    max = a[i][j].abs();
                    p = i;
                    q = j;
                }
            }
        }
        if max < 1e-14 {
            break;
        }
        let theta = 0.5 * (2. * a[p][q]).atan2(a[q][q] - a[p][p]);
        let (c, s) = (theta.cos(), theta.sin());
        for k in 0..n {
            let (x, y) = (a[k][p], a[k][q]);
            a[k][p] = c * x - s * y;
            a[k][q] = s * x + c * y;
        }
        for k in 0..n {
            let (x, y) = (a[p][k], a[q][k]);
            a[p][k] = c * x - s * y;
            a[q][k] = s * x + c * y;
        }
        for k in 0..n {
            let (x, y) = (v[k][p], v[k][q]);
            v[k][p] = c * x - s * y;
            v[k][q] = s * x + c * y;
        }
    }
    let eig: Vec<_> = (0..n).map(|i| a[i][i].max(0.)).collect();
    let mut out = vec![vec![0.; n]; n];
    for i in 0..n {
        for j in 0..n {
            out[i][j] = (0..n).map(|k| v[i][k] * eig[k] * v[j][k]).sum();
        }
    }
    Ok(out)
}

/// Market-implied equilibrium returns `delta * covariance * market_weights`.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn market_implied_returns(
    cov: &[Vec<f64>],
    market_weights: &[f64],
    delta: f64,
) -> Result<Vec<f64>> {
    let n = market_weights.len();
    check_matrix("covariance", cov, n)?;
    all_finite("market_weights", market_weights)?;
    all_finite("delta", &[delta])?;
    if delta < 0. {
        return Err(FinanceError::InvalidInput {
            what: "delta",
            reason: "must be nonnegative",
        });
    }
    let mut x = matrix::mat_vec(cov, market_weights);
    for v in &mut x {
        *v *= delta;
    }
    Ok(x)
}

/// Black-Litterman posterior mean and covariance from pick matrix P, views Q,
/// uncertainty Omega, and scalar tau. Inputs are row-major matrices.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn black_litterman(
    cov: &[Vec<f64>],
    market_weights: &[f64],
    delta: f64,
    views: &[Vec<f64>],
    view_returns: &[f64],
    omega: &[Vec<f64>],
    tau: f64,
) -> Result<(Vec<f64>, Vec<Vec<f64>>)> {
    let n = market_weights.len();
    check_matrix("covariance", cov, n)?;
    let k = views.len();
    check_matrix("omega", omega, k)?;
    all_finite("market_weights", market_weights)?;
    all_finite("view_returns", view_returns)?;
    all_finite("delta and tau", &[delta, tau])?;
    if view_returns.len() != k || views.iter().any(|r| r.len() != n) {
        return Err(FinanceError::InvalidInput {
            what: "views",
            reason: "P, Q, and Omega dimensions must align",
        });
    }
    if delta < 0. || tau <= 0. {
        return Err(FinanceError::InvalidInput {
            what: "Black-Litterman parameters",
            reason: "delta must be nonnegative and tau positive",
        });
    }
    let prior = market_implied_returns(cov, market_weights, delta)?;
    if k == 0 {
        return Ok((prior, cov.to_vec()));
    }
    let tc: Vec<Vec<f64>> = cov
        .iter()
        .map(|r| r.iter().map(|x| x * tau).collect())
        .collect();
    let mut middle = omega.to_vec();
    for i in 0..k {
        for j in 0..k {
            let mut value = 0.;
            for a in 0..n {
                for b in 0..n {
                    value += views[i][a] * tc[a][b] * views[j][b];
                }
            }
            middle[i][j] += value;
        }
    }
    let residual: Vec<f64> = (0..k)
        .map(|i| view_returns[i] - matrix::dot(&views[i], &prior))
        .collect();
    let z = solve(&middle, &residual)?;
    let mut posterior = prior;
    for i in 0..n {
        for a in 0..k {
            for b in 0..n {
                posterior[i] += tc[i][b] * views[a][b] * z[a];
            }
        }
    }
    let mut inv = vec![vec![0.; k]; k];
    for j in 0..k {
        let mut e = vec![0.; k];
        e[j] = 1.;
        let col = solve(&middle, &e)?;
        for i in 0..k {
            inv[i][j] = col[i];
        }
    }
    let mut post_cov = cov.to_vec();
    for i in 0..n {
        for j in 0..n {
            let mut sub = 0.;
            for a in 0..k {
                for b in 0..k {
                    let mut left = 0.;
                    let mut right = 0.;
                    for x in 0..n {
                        left += tc[i][x] * views[a][x];
                        right += views[b][x] * tc[x][j];
                    }
                    sub += left * inv[a][b] * right;
                }
            }
            post_cov[i][j] += tau * cov[i][j] - sub;
        }
    }
    Ok((posterior, post_cov))
}
