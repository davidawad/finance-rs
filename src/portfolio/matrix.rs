use crate::error::{all_finite, FinanceError, Result};
use crate::stats;

#[allow(clippy::needless_range_loop)] // Indexed iteration keeps row and column coordinates aligned in this matrix algorithm.
pub(super) fn check_matrix(name: &'static str, a: &[Vec<f64>], n: usize) -> Result<()> {
    if a.len() != n || a.iter().any(|r| r.len() != n) {
        return Err(FinanceError::InvalidInput {
            what: name,
            reason: "matrix must be square and dimensionally aligned",
        });
    }
    for row in a {
        all_finite(name, row)?;
    }
    for i in 0..n {
        for j in 0..i {
            if (a[i][j] - a[j][i]).abs() > 1e-10 {
                return Err(FinanceError::InvalidInput {
                    what: name,
                    reason: "matrix must be symmetric",
                });
            }
        }
    }
    Ok(())
}

pub(super) fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
pub(super) fn mat_vec(a: &[Vec<f64>], x: &[f64]) -> Vec<f64> {
    a.iter().map(|r| dot(r, x)).collect()
}

#[allow(clippy::needless_range_loop)] // Indexed iteration keeps row and column coordinates aligned in this matrix algorithm.
#[allow(clippy::many_single_char_names)] // Conventional symbols follow the published formula or recurrence notation.
pub(super) fn solve(a: &[Vec<f64>], b: &[f64]) -> Result<Vec<f64>> {
    let n = b.len();
    check_matrix("matrix", a, n)?;
    all_finite("rhs", b)?;
    let mut m = a.to_vec();
    let mut y = b.to_vec();
    for k in 0..n {
        let p = (k..n)
            .max_by(|&i, &j| m[i][k].abs().total_cmp(&m[j][k].abs()))
            .unwrap_or(k);
        if m[p][k].abs() <= 1e-14 {
            return Err(FinanceError::Undefined {
                what: "singular matrix",
            });
        }
        m.swap(k, p);
        y.swap(k, p);
        let d = m[k][k];
        for j in k..n {
            m[k][j] /= d;
        }
        y[k] /= d;
        for i in 0..n {
            if i != k {
                let f = m[i][k];
                for j in k..n {
                    m[i][j] -= f * m[k][j];
                }
                y[i] -= f * y[k];
            }
        }
    }
    if y.iter().all(|x| x.is_finite()) {
        Ok(y)
    } else {
        Err(FinanceError::Overflow {
            what: "linear solve",
        })
    }
}

/// Covariance matrix for row-major observations (rows=time, columns=assets).
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn covariance_matrix(returns: &[Vec<f64>], ddof: usize) -> Result<Vec<Vec<f64>>> {
    if returns.len() <= ddof {
        return Err(FinanceError::TooShort {
            what: "returns",
            min: ddof + 1,
            got: returns.len(),
        });
    }
    let n = returns[0].len();
    if n == 0 || returns.iter().any(|r| r.len() != n) {
        return Err(FinanceError::InvalidInput {
            what: "returns",
            reason: "must be a nonempty rectangular matrix",
        });
    }
    let cols: Vec<Vec<f64>> = (0..n)
        .map(|j| returns.iter().map(|r| r[j]).collect())
        .collect();
    for c in &cols {
        all_finite("returns", c)?;
    }
    let mut out = vec![vec![0.; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let v = stats::covariance(&cols[i], &cols[j], ddof)?;
            out[i][j] = v;
            out[j][i] = v;
        }
    }
    Ok(out)
}

/// Pearson correlation matrix for row-major observations.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn correlation_matrix(returns: &[Vec<f64>]) -> Result<Vec<Vec<f64>>> {
    let cov = covariance_matrix(returns, 1)?;
    let n = cov.len();
    let sd: Vec<f64> = (0..n).map(|i| cov[i][i].sqrt()).collect();
    let mut out = vec![vec![0.; n]; n];
    for i in 0..n {
        if sd[i] == 0. {
            return Err(FinanceError::Undefined {
                what: "constant asset correlation",
            });
        }
        for j in 0..n {
            out[i][j] = (cov[i][j] / (sd[i] * sd[j])).clamp(-1., 1.);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_solve_reports_overflow_after_finite_inputs() {
        assert!(matches!(
            solve(&[vec![1e-10]], &[f64::MAX]),
            Err(FinanceError::Overflow {
                what: "linear solve"
            })
        ));
    }

    #[test]
    fn matrix_validation_and_elimination_cover_symmetry_and_lower_rows() {
        assert!(matches!(
            check_matrix("covariance", &[vec![1.0, 0.2], vec![0.1, 1.0]], 2),
            Err(FinanceError::InvalidInput {
                what: "covariance",
                ..
            })
        ));
        let solution = solve(&[vec![4.0, 1.0], vec![1.0, 3.0]], &[9.0, 7.0]).unwrap();
        assert!((solution[0] - 20.0 / 11.0).abs() < 1e-12);
        assert!((solution[1] - 19.0 / 11.0).abs() < 1e-12);
    }
}
