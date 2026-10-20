//! Ordinary least squares by Householder QR, sized for factor regressions
//! (a few regressors, up to thousands of observations). No linear-algebra
//! dependency: the design matrix is factored in place, column-major.

use crate::error::{all_finite, output, same_len, FinanceError, Result};

/// Settings for [`ols_with`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct OlsOptions {
    /// Prepend a column of ones, so `coef[0]` is the intercept (default
    /// `true`, statsmodels `add_constant`). Without it, `r2` is uncentered,
    /// as statsmodels reports it.
    pub intercept: bool,
}

impl Default for OlsOptions {
    fn default() -> Self {
        Self { intercept: true }
    }
}

/// An OLS fit `y = Xβ + ε`, with `k` coefficients on `n` observations.
///
/// Identities: `resid[i] = y[i] − Σ_j X[i][j]·coef[j]`;
/// `t[j] = coef[j]/se[j]`; `adj_r2 = 1 − (1 − r2)·(n − c)/(n − k)` where `c`
/// is 1 with an intercept and 0 without.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Ols {
    /// `β`, intercept first when fitted.
    pub coef: Vec<f64>,
    /// Standard errors `√(s²·diag((XᵀX)⁻¹))`, `s² = SSR/(n − k)`.
    pub se: Vec<f64>,
    /// t statistics `coef/se`.
    pub t: Vec<f64>,
    /// `1 − SSR/SST` (SST centered with an intercept, uncentered without).
    pub r2: f64,
    /// R² adjusted for degrees of freedom.
    pub adj_r2: f64,
    /// Residuals `y − Xβ`.
    pub resid: Vec<f64>,
}

/// OLS of `y` on the regressors `xs` (each aligned with `y`) plus an
/// intercept (statsmodels `OLS(y, add_constant(X)).fit()`).
///
/// ```
/// use finance_rs::stats::ols;
/// let x = [1.0, 2.0, 3.0, 4.0, 5.0];
/// let y = [3.1, 4.9, 7.2, 8.8, 11.1];
/// let fit = ols(&y, &[&x])?;
/// assert!((fit.coef[1] - 1.99).abs() < 1e-12 && (fit.coef[0] - 1.05).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`ols_with`].
pub fn ols(y: &[f64], xs: &[&[f64]]) -> Result<Ols> {
    ols_with(y, xs, OlsOptions::default())
}

/// OLS with explicit options.
///
/// # Errors
/// [`FinanceError::TooShort`] unless `n > k` (and `k ≥ 1`),
/// [`FinanceError::LengthMismatch`] for unaligned regressors,
/// [`FinanceError::Undefined`] for a rank-deficient design, a zero total sum
/// of squares, or a perfect fit (zero residual variance, where statsmodels
/// reports infinite t statistics); finiteness errors.
#[allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
pub fn ols_with(y: &[f64], xs: &[&[f64]], opts: OlsOptions) -> Result<Ols> {
    let n = y.len();
    all_finite("y", y)?;
    for x in xs {
        same_len("y", n, "x", x.len())?;
        all_finite("x", x)?;
    }
    let k = xs.len() + usize::from(opts.intercept);
    if k == 0 {
        return Err(FinanceError::TooShort {
            what: "regressors",
            min: 1,
            got: 0,
        });
    }
    if n <= k {
        return Err(FinanceError::TooShort {
            what: "y",
            min: k + 1,
            got: n,
        });
    }
    // Column-major design matrix.
    let mut a: Vec<Vec<f64>> = Vec::with_capacity(k);
    if opts.intercept {
        a.push(vec![1.0; n]);
    }
    a.extend(xs.iter().map(|x| x.to_vec()));
    let design = a.clone();
    let mut qty = y.to_vec();
    householder(&mut a, &mut qty)?;
    // Back-substitute R β = Qᵀy.
    let mut coef = vec![0.0; k];
    for j in (0..k).rev() {
        let s: f64 = (j + 1..k).map(|c| a[c][j] * coef[c]).sum();
        coef[j] = (qty[j] - s) / a[j][j];
    }
    let resid: Vec<f64> = (0..n)
        .map(|i| y[i] - (0..k).map(|j| design[j][i] * coef[j]).sum::<f64>())
        .collect();
    let ssr: f64 = resid.iter().map(|e| e * e).sum();
    let df = (n - k) as f64;
    let s2 = ssr / df;
    if s2 == 0.0 {
        return Err(FinanceError::Undefined {
            what: "residual variance",
        });
    }
    let rinv = upper_inverse(&a, k);
    let se: Vec<f64> = (0..k)
        .map(|j| (s2 * (j..k).map(|c| rinv[c][j] * rinv[c][j]).sum::<f64>()).sqrt())
        .collect();
    let sst: f64 = if opts.intercept {
        let m = super::mean_of(y);
        y.iter().map(|v| (v - m) * (v - m)).sum()
    } else {
        y.iter().map(|v| v * v).sum()
    };
    // Zero total variation implies a zero residual for either intercept mode,
    // which was rejected by the residual-variance check above.
    let r2 = 1.0 - ssr / sst;
    let adj_r2 = 1.0 - (n - usize::from(opts.intercept)) as f64 / df * (1.0 - r2);
    let t = coef.iter().zip(&se).map(|(b, s)| b / s).collect::<Vec<_>>();
    for v in coef.iter().chain(&se).chain(&t).chain(&resid) {
        output("ols", *v)?;
    }
    Ok(Ols {
        coef,
        se,
        t,
        r2: output("ols r2", r2)?,
        adj_r2: output("ols adj_r2", adj_r2)?,
        resid,
    })
}

/// Householder QR in place: `a` (column-major, `k` columns of length `n`)
/// becomes `R` in its upper triangle, and `qty` becomes `Qᵀy`.
#[allow(clippy::many_single_char_names)] // textbook QR notation
fn householder(a: &mut [Vec<f64>], qty: &mut [f64]) -> Result<()> {
    let k = a.len();
    let n = qty.len();
    let scale = a
        .iter()
        .map(|c| c.iter().map(|v| v * v).sum::<f64>().sqrt())
        .fold(0.0, f64::max);
    for j in 0..k {
        let norm = a[j][j..].iter().map(|v| v * v).sum::<f64>().sqrt();
        #[allow(clippy::cast_precision_loss)]
        let tiny = scale * f64::EPSILON * n as f64;
        if norm <= tiny || !norm.is_finite() {
            return Err(FinanceError::Undefined {
                what: "rank of the design matrix",
            });
        }
        let alpha = if a[j][j] > 0.0 { -norm } else { norm };
        // v = x − α e₁, stored in a[j][j..]; R_jj = α.
        let mut v: Vec<f64> = a[j][j..].to_vec();
        v[0] -= alpha;
        let vv: f64 = v.iter().map(|x| x * x).sum();
        for col in a.iter_mut().skip(j) {
            let dot: f64 = v.iter().zip(&col[j..]).map(|(p, q)| p * q).sum();
            let f = 2.0 * dot / vv;
            for (x, vi) in col[j..].iter_mut().zip(&v) {
                *x -= f * vi;
            }
        }
        let dot: f64 = v.iter().zip(&qty[j..]).map(|(p, q)| p * q).sum();
        let f = 2.0 * dot / vv;
        for (i, vi) in v.iter().enumerate() {
            qty[j + i] -= f * vi;
        }
        a[j][j] = alpha;
    }
    Ok(())
}

/// `R⁻¹` of the upper-triangular `R` held in `a` (column-major), returned
/// column-major: `out[c][r]` is row `r`, column `c`.
fn upper_inverse(a: &[Vec<f64>], k: usize) -> Vec<Vec<f64>> {
    let mut inv = vec![vec![0.0; k]; k];
    for c in 0..k {
        inv[c][c] = 1.0 / a[c][c];
        for r in (0..c).rev() {
            let s: f64 = (r + 1..=c).map(|m| a[m][r] * inv[c][m]).sum();
            inv[c][r] = -s / a[r][r];
        }
    }
    inv
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_collinear_and_short() {
        let x = [1.0, 2.0, 3.0, 4.0];
        let x2 = [2.0, 4.0, 6.0, 8.0];
        assert!(matches!(
            ols(&[1.0, 3.0, 2.0, 5.0], &[&x, &x2]),
            Err(FinanceError::Undefined { .. })
        ));
        assert!(matches!(
            ols(&[1.0, 2.0], &[&[1.0, 2.0]]),
            Err(FinanceError::TooShort { .. })
        ));
        assert!(ols_with(&[1.0, 2.0], &[], OlsOptions { intercept: false }).is_err());
        assert!(matches!(
            ols_with(
                &[0.0, 0.0, 0.0],
                &[&[1.0, 2.0, 3.0]],
                OlsOptions { intercept: false }
            ),
            Err(FinanceError::Undefined {
                what: "residual variance"
            })
        ));
    }

    #[test]
    fn residuals_are_orthogonal_to_regressors() -> Result<()> {
        let x = [0.5, -1.0, 2.0, 3.5, 0.0, 1.5];
        let y = [1.0, -0.5, 3.2, 4.1, 0.7, 2.2];
        let fit = ols(&y, &[&x])?;
        let dot: f64 = fit.resid.iter().zip(x).map(|(e, v)| e * v).sum();
        assert!(dot.abs() < 1e-12 && fit.resid.iter().sum::<f64>().abs() < 1e-12);
        Ok(())
    }
}
