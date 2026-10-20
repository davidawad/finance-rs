//! Descriptive statistics with numpy 2.5 semantics: moments with an explicit
//! `ddof`, quantiles by named method, covariance and Pearson correlation, and
//! ordinary least squares (statsmodels `OLS` values).
//!
//! Inputs must be finite (gaps are rejected, not skipped). A statistic that
//! numpy returns as `NaN` or `±∞` (too few observations for the `ddof`, a
//! zero variance in a correlation) is a [`FinanceError`] instead.
//!
//! ```
//! use finance_rs::stats::{correlation, mean, quantile, stdev, QuantileMethod};
//! let x = [1.0, 2.0, 3.0, 4.0];
//! assert_eq!(mean(&x)?, 2.5);
//! assert!((stdev(&x, 1)? - 1.290_994_448_735_805_6).abs() < 1e-15);
//! assert_eq!(quantile(&x, 0.5, QuantileMethod::Linear)?, 2.5);
//! assert!((correlation(&x, &[2.0, 4.0, 6.0, 8.0])? - 1.0).abs() < 1e-15);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

mod ols;

pub use ols::{ols, ols_with, Ols, OlsOptions};

use crate::error::{all_finite, finite, min_len, output, same_len, FinanceError, Result};

/// Arithmetic mean, left-to-right sum (caller guarantees a non-empty slice).
#[allow(clippy::cast_precision_loss)]
pub(crate) fn mean_of(xs: &[f64]) -> f64 {
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// Two-pass variance with `ddof` (caller guarantees `len > ddof`).
#[allow(clippy::cast_precision_loss)]
pub(crate) fn variance_of(xs: &[f64], ddof: usize) -> f64 {
    let m = mean_of(xs);
    let ss: f64 = xs.iter().map(|x| (x - m) * (x - m)).sum();
    ss / (xs.len() - ddof) as f64
}

fn check(what: &'static str, xs: &[f64], min: usize) -> Result<()> {
    min_len(what, xs, min)?;
    all_finite(what, xs)
}

/// Arithmetic mean `x̄ = Σx/n` (numpy `mean`).
///
/// # Errors
/// [`FinanceError::TooShort`] for an empty slice, [`FinanceError::NonFinite`]
/// for a non-finite value, [`FinanceError::Overflow`] when the sum overflows.
pub fn mean(xs: &[f64]) -> Result<f64> {
    check("xs", xs, 1)?;
    output("mean", mean_of(xs))
}

/// Variance `Σ(x − x̄)²/(n − ddof)`: `ddof = 1` is the sample variance,
/// `ddof = 0` the population variance (numpy `var(ddof=)`).
///
/// ```
/// use finance_rs::stats::variance;
/// assert_eq!(variance(&[1.0, 2.0, 3.0, 4.0], 0)?, 1.25);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] unless `n > ddof`; otherwise as [`mean`].
pub fn variance(xs: &[f64], ddof: usize) -> Result<f64> {
    check("xs", xs, ddof.saturating_add(1))?;
    output("variance", variance_of(xs, ddof))
}

/// Standard deviation `√variance(xs, ddof)` (numpy `std(ddof=)`).
///
/// # Errors
/// As [`variance`].
pub fn stdev(xs: &[f64], ddof: usize) -> Result<f64> {
    variance(xs, ddof).map(f64::sqrt)
}

/// Covariance `Σ(x − x̄)(y − ȳ)/(n − ddof)` (numpy `cov(x, y, ddof=)[0, 1]`).
///
/// ```
/// use finance_rs::stats::covariance;
/// assert_eq!(covariance(&[1.0, 2.0, 3.0], &[1.0, 3.0, 2.0], 1)?, 0.5);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::LengthMismatch`] for unaligned series; otherwise as
/// [`variance`].
#[allow(clippy::cast_precision_loss)]
pub fn covariance(x: &[f64], y: &[f64], ddof: usize) -> Result<f64> {
    same_len("x", x.len(), "y", y.len())?;
    check("x", x, ddof.saturating_add(1))?;
    check("y", y, ddof.saturating_add(1))?;
    let (mx, my) = (mean_of(x), mean_of(y));
    let s: f64 = x.iter().zip(y).map(|(a, b)| (a - mx) * (b - my)).sum();
    output("covariance", s / (x.len() - ddof) as f64)
}

/// Pearson correlation `ρ = cov(x, y)/(σ_x·σ_y)`, clipped to `[−1, 1]` as
/// numpy `corrcoef` does.
///
/// ```
/// use finance_rs::stats::correlation;
/// let r = correlation(&[1.0, 2.0, 3.0, 4.0], &[1.0, 3.0, 2.0, 4.0])?;
/// assert!((r - 0.8).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::Undefined`] when either series is constant (numpy returns
/// `NaN`), [`FinanceError::TooShort`] for fewer than two pairs, plus length
/// and finiteness errors.
pub fn correlation(x: &[f64], y: &[f64]) -> Result<f64> {
    same_len("x", x.len(), "y", y.len())?;
    check("x", x, 2)?;
    check("y", y, 2)?;
    let (mx, my) = (mean_of(x), mean_of(y));
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        let (dx, dy) = (a - mx, b - my);
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    if sxx == 0.0 || syy == 0.0 {
        return Err(FinanceError::Undefined {
            what: "variance of x or y",
        });
    }
    output("correlation", sxy / (sxx.sqrt() * syy.sqrt())).map(|r| r.clamp(-1.0, 1.0))
}

/// How [`quantile`] picks or interpolates between order statistics
/// `x₀ ≤ … ≤ x_{n−1}` (numpy `quantile(method=)` names).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub enum QuantileMethod {
    /// Interpolate at `h = (n−1)q` (numpy default, Excel `PERCENTILE.INC`).
    #[default]
    Linear,
    /// `x_⌊h⌋`.
    Lower,
    /// `x_⌈h⌉`.
    Higher,
    /// `x` at `h` rounded half to even (numpy `nearest`).
    Nearest,
    /// `(x_⌊h⌋ + x_⌈h⌉)/2`.
    Midpoint,
    /// Interpolate at `h = (n+1)q − 1` (Excel `PERCENTILE.EXC`, numpy
    /// `weibull`); an error unless `1/(n+1) ≤ q ≤ n/(n+1)`, as in Excel.
    ExcelExclusive,
}

/// numpy's `_lerp`: `a + (b−a)·t`, evaluated from `b` when `t ≥ ½`.
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    let d = b - a;
    if t >= 0.5 {
        b - d * (1.0 - t)
    } else {
        a + d * t
    }
}

/// The `q`-quantile of `xs` (`0 ≤ q ≤ 1`) under `method`.
///
/// ```
/// use finance_rs::stats::{quantile, QuantileMethod};
/// let x = [15.0, 20.0, 35.0, 40.0, 50.0];
/// assert_eq!(quantile(&x, 0.4, QuantileMethod::Linear)?, 29.0);
/// assert!((quantile(&x, 0.4, QuantileMethod::ExcelExclusive)? - 26.0).abs() < 1e-12);
/// assert_eq!(quantile(&x, 0.4, QuantileMethod::Lower)?, 20.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for `q` outside `[0, 1]` (or outside
/// Excel's range for [`QuantileMethod::ExcelExclusive`]); otherwise as [`mean`].
#[allow(
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn quantile(xs: &[f64], q: f64, method: QuantileMethod) -> Result<f64> {
    check("xs", xs, 1)?;
    finite("q", q)?;
    if !(0.0..=1.0).contains(&q) {
        return Err(FinanceError::InvalidInput {
            what: "q",
            reason: "must be in [0, 1]",
        });
    }
    let mut v = xs.to_vec();
    v.sort_by(f64::total_cmp);
    let n = v.len();
    let last = (n - 1) as f64;
    let at = |i: f64| v[(i.max(0.0) as usize).min(n - 1)];
    let h = last * q;
    let x = match method {
        QuantileMethod::Lower => at(h.floor()),
        QuantileMethod::Higher => at(h.ceil()),
        QuantileMethod::Nearest => at(h.round_ties_even()),
        QuantileMethod::Midpoint if h.fract() == 0.0 => at(h),
        QuantileMethod::Midpoint => lerp(at(h.floor()), at(h.ceil()), 0.5),
        QuantileMethod::Linear => lerp(at(h.floor()), at(h.floor() + 1.0), h - h.floor()),
        QuantileMethod::ExcelExclusive => {
            let h = (n as f64 + 1.0) * q - 1.0;
            if h < 0.0 || h > last {
                return Err(FinanceError::InvalidInput {
                    what: "q",
                    reason: "PERCENTILE.EXC needs 1/(n+1) <= q <= n/(n+1)",
                });
            }
            lerp(at(h.floor()), at(h.floor() + 1.0), h - h.floor())
        }
    };
    output("quantile", x)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_are_typed() {
        assert!(matches!(mean(&[]), Err(FinanceError::TooShort { .. })));
        assert!(matches!(
            variance(&[1.0], 1),
            Err(FinanceError::TooShort { .. })
        ));
        assert!(matches!(
            correlation(&[1.0, 1.0], &[1.0, 2.0]),
            Err(FinanceError::Undefined { .. })
        ));
        assert!(quantile(&[1.0, 2.0], 1.5, QuantileMethod::Linear).is_err());
        assert!(quantile(&[1.0, 2.0], 0.2, QuantileMethod::ExcelExclusive).is_err());
        assert!(variance(&[1.0, 2.0], usize::MAX).is_err());
    }

    #[test]
    fn nearest_rounds_half_to_even() -> Result<()> {
        // h = 0.5 rounds to 0, h = 1.5 to 2 (numpy `around`).
        assert_eq!(quantile(&[1.0, 2.0], 0.5, QuantileMethod::Nearest)?, 1.0);
        assert_eq!(
            quantile(&[1.0, 2.0, 3.0, 4.0], 0.5, QuantileMethod::Nearest)?,
            3.0
        );
        Ok(())
    }

    #[test]
    fn variance_and_linear_quantile_use_interpolated_values() -> Result<()> {
        assert_eq!(variance(&[1.0, 3.0], 1)?, 2.0);
        assert_eq!(quantile(&[1.0, 3.0], 0.25, QuantileMethod::Linear)?, 1.5);
        Ok(())
    }
}
