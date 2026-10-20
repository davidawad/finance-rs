use super::{
    covariance_matrix,
    matrix::{check_matrix, solve},
};
use crate::error::{all_finite, FinanceError, Result};

/// Inverse-volatility weights, normalized to sum to one.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn inverse_volatility(returns: &[Vec<f64>]) -> Result<Vec<f64>> {
    let cov = covariance_matrix(returns, 1)?;
    let mut x: Vec<_> = (0..cov.len())
        .map(|i| {
            if cov[i][i] > 0. {
                1. / cov[i][i].sqrt()
            } else {
                0.
            }
        })
        .collect();
    normalize(&mut x)?;
    Ok(x)
}
/// Long-only minimum-variance weights from cyclic coordinate descent.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn equal_risk_contribution(cov: &[Vec<f64>]) -> Result<Vec<f64>> {
    let n = cov.len();
    check_matrix("covariance", cov, n)?;
    if n == 0 {
        return Err(FinanceError::TooShort {
            what: "covariance",
            min: 1,
            got: 0,
        });
    }
    if (0..n).any(|i| cov[i][i] <= 0.) {
        return Err(FinanceError::InvalidInput {
            what: "covariance",
            reason: "diagonal variances must be positive",
        });
    }
    let mut w = vec![1. / n as f64; n];
    for _ in 0..5000 {
        let rc: Vec<_> = (0..n)
            .map(|i| w[i] * (0..n).map(|j| cov[i][j] * w[j]).sum::<f64>())
            .collect();
        let target = rc.iter().sum::<f64>() / n as f64;
        let mut change: f64 = 0.;
        for i in 0..n {
            let other = (0..n)
                .filter(|j| *j != i)
                .map(|j| cov[i][j] * w[j])
                .sum::<f64>();
            let a = cov[i][i];
            let next = if other == 0. {
                (target / a).sqrt()
            } else {
                (-other + (other * other + 4. * a * target).sqrt()) / (2. * a)
            };
            change = change.max((next - w[i]).abs());
            w[i] = next.max(0.);
        }
        normalize(&mut w)?;
        if change < 1e-12 {
            break;
        }
    }
    Ok(w)
}
/// Unconstrained global minimum-variance weights `Σ⁻¹1 / (1ᵀΣ⁻¹1)`.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn min_variance_unconstrained(cov: &[Vec<f64>]) -> Result<Vec<f64>> {
    let n = cov.len();
    check_matrix("covariance", cov, n)?;
    let x = solve(cov, &vec![1.; n])?;
    let s = x.iter().sum::<f64>();
    if s.abs() < 1e-14 {
        return Err(FinanceError::Undefined {
            what: "minimum-variance normalization",
        });
    }
    Ok(x.iter().map(|v| v / s).collect())
}
/// Unconstrained tangency weights `Σ⁻¹(μ-rf·1)` normalized to unit sum.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn tangency_unconstrained(mu: &[f64], cov: &[Vec<f64>], risk_free: f64) -> Result<Vec<f64>> {
    all_finite("expected returns", mu)?;
    all_finite("risk_free", &[risk_free])?;
    let n = mu.len();
    check_matrix("covariance", cov, n)?;
    let x = solve(cov, &mu.iter().map(|m| m - risk_free).collect::<Vec<_>>())?;
    let s = x.iter().sum::<f64>();
    if s.abs() < 1e-14 {
        return Err(FinanceError::Undefined {
            what: "tangency normalization",
        });
    }
    Ok(x.iter().map(|v| v / s).collect())
}

/// Hierarchical risk parity weights using single-linkage correlation ordering
/// and recursive inverse-variance bisection (López de Prado, 2016).
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn hrp(returns: &[Vec<f64>]) -> Result<Vec<f64>> {
    let cov = covariance_matrix(returns, 1)?;
    let n = cov.len();
    if n == 1 {
        return Ok(vec![1.]);
    }
    let corr = super::correlation_matrix(returns)?;
    // Deterministic single-linkage leaf order, seeded by asset zero.
    let mut clusters: Vec<(usize, Vec<usize>)> = (0..n).map(|i| (i, vec![i])).collect();
    let mut next_cluster_id = n;
    while clusters.len() > 1 {
        let mut best = (f64::INFINITY, 0, 1);
        for i in 0..clusters.len() {
            for j in i + 1..clusters.len() {
                let mut d = f64::INFINITY;
                for a in &clusters[i].1 {
                    for b in &clusters[j].1 {
                        d = d.min(((1. - corr[*a][*b]) / 2.).sqrt());
                    }
                }
                if d < best.0 {
                    best = (d, i, j);
                }
            }
        }
        let right = clusters.remove(best.2);
        let left = clusters.remove(best.1);
        // Cluster IDs stay ascending: initial IDs follow input order, each
        // merge removes two entries in ascending vector order, then appends a
        // new maximum ID. Thus `left` always precedes `right`.
        let merged = [left.1, right.1].concat();
        clusters.push((next_cluster_id, merged));
        next_cluster_id += 1;
    }
    let order = &clusters[0].1;
    let mut w = vec![0.; n];
    for i in order {
        w[*i] = 1.;
    }
    let mut todo = vec![order.clone()];
    while let Some(c) = todo.pop() {
        if c.len() <= 1 {
            continue;
        }
        let mid = c.len() / 2;
        let (left, right) = (&c[..mid], &c[mid..]);
        let vl = cluster_variance(left, &cov);
        let vr = cluster_variance(right, &cov);
        // Correlation construction rejects zero-variance assets, so each
        // cluster has strictly positive inverse-variance scale here.
        let alpha = 1. - vl / (vl + vr);
        for i in left {
            w[*i] *= alpha;
        }
        for i in right {
            w[*i] *= 1. - alpha;
        }
        todo.push(left.to_vec());
        todo.push(right.to_vec());
    }
    normalize(&mut w)?;
    Ok(w)
}

fn cluster_variance(ids: &[usize], cov: &[Vec<f64>]) -> f64 {
    let mut inv: Vec<_> = ids
        .iter()
        // `hrp` validates through `correlation_matrix`, which rejects every
        // non-positive variance before cluster splitting begins.
        .map(|i| 1. / cov[*i][*i])
        .collect();
    let sum = inv.iter().sum::<f64>();
    // HRP calls correlation_matrix first, which guarantees positive diagonals.
    for x in &mut inv {
        *x /= sum;
    }
    let mut v = 0.;
    for (i, a) in ids.iter().enumerate() {
        for (j, b) in ids.iter().enumerate() {
            v += inv[i] * cov[*a][*b] * inv[j];
        }
    }
    v.max(0.)
}
fn normalize(w: &mut [f64]) -> Result<()> {
    let s = w.iter().sum::<f64>();
    if !s.is_finite() || s <= 0. {
        return Err(FinanceError::Undefined {
            what: "weight normalization",
        });
    }
    for x in w {
        *x /= s;
    }
    Ok(())
}
