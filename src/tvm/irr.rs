//! `npv`, `irr` (every real root, numpy-financial's selection) and `mirr`.

use crate::error::{all_finite, div, finite, min_len, output, FinanceError, Result};
use crate::solve::kernel::{brent, geometric_grid, scan_roots};

/// Net present value of `values[t]` received at the end of period `t`
/// (`t = 0` is undiscounted, as in numpy-financial, unlike Excel's NPV).
///
/// `npv = Σ_t values[t] / (1+r)^t`
///
/// ```
/// use finance_rs::tvm::npv;
/// let v = npv(0.08, &[-40_000.0, 5_000.0, 8_000.0, 12_000.0, 30_000.0])?;
/// assert!((v - 3_065.222_668_179_1).abs() < 1e-6);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for no values, [`FinanceError::NonFinite`] for
/// non-finite input, [`FinanceError::Overflow`] for a non-finite result.
pub fn npv(rate: f64, values: &[f64]) -> Result<f64> {
    finite("rate", rate)?;
    min_len("values", values, 1)?;
    all_finite("values", values)?;
    let mut factor = 1.0;
    let mut sum = 0.0;
    for v in values {
        sum += v / factor;
        factor *= 1.0 + rate;
    }
    output("npv", sum)
}

/// Every real internal rate of return of the periodic series, ascending.
///
/// The roots are the real `g > 0` solving `Σ_t values[t]·g^(N−t) = 0`
/// (numpy-financial's polynomial), reported as `r = g − 1`. A series ending
/// in zeros also has the degenerate root `r = −1`, as in numpy-financial.
/// Descartes' rule of signs bounds the count: one sign change means exactly
/// one root, found by Brent's method on the Cauchy root bound; several mean a
/// dense log-spaced scan. Tangent (double) roots, which never change sign,
/// are not reported (numpy-financial's `np.roots` also yields those as
/// complex pairs and drops them).
///
/// ```
/// use finance_rs::tvm::irr_roots;
/// // -100, +230, -132 has IRRs of 10% and 20%.
/// let roots = irr_roots(&[-100.0, 230.0, -132.0])?;
/// assert_eq!(roots.len(), 2);
/// assert!((roots[0] - 0.1).abs() < 1e-12 && (roots[1] - 0.2).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NoSignChange`] when the values never change sign,
/// [`FinanceError::TooShort`] / [`FinanceError::NonFinite`] for bad input.
pub fn irr_roots(values: &[f64]) -> Result<Vec<f64>> {
    min_len("values", values, 2)?;
    all_finite("values", values)?;
    let first = values.iter().position(|v| *v != 0.0);
    let last = values.iter().rposition(|v| *v != 0.0);
    let (Some(first), Some(last)) = (first, last) else {
        return Err(FinanceError::NoSignChange);
    };
    // Leading zeros only shift the polynomial; trailing zeros are roots g = 0.
    let coeffs = &values[first..=last];
    let changes = sign_changes(coeffs);
    if changes == 0 {
        return Err(FinanceError::NoSignChange);
    }
    let lead = coeffs[0].abs();
    let tail = coeffs[coeffs.len() - 1].abs();
    let max_abs = |xs: &[f64]| xs.iter().fold(0.0_f64, |m, v| m.max(v.abs()));
    let hi = 1.0 + max_abs(&coeffs[1..]) / lead;
    let lo = tail / (tail + max_abs(&coeffs[..coeffs.len() - 1]));
    let p = |g: f64| poly_sign_value(coeffs, g);
    let mut roots: Vec<f64> = if changes == 1 {
        vec![brent("irr", p, lo * 0.5, hi * 2.0, 0.0)?]
    } else {
        let n = (64 * coeffs.len()).clamp(2_048, 20_000);
        let mut grid = geometric_grid(lo * 0.5, hi * 2.0, n);
        grid.push(1.0);
        grid.sort_by(f64::total_cmp);
        scan_roots("irr", p, &grid, 0.0)
    };
    for g in &mut roots {
        *g -= 1.0;
    }
    // A root so close to g = 0 that r rounds to −1, or beyond f64's range
    // (coefficients ~1e-308 against ~1e308), is not a usable rate.
    roots.retain(|r| *r > -1.0 && r.is_finite());
    if last + 1 < values.len() {
        roots.insert(0, -1.0);
    }
    if roots.is_empty() {
        return Err(FinanceError::NoSolution { what: "irr" });
    }
    Ok(roots)
}

/// Internal rate of return: the rate where [`npv`] is zero.
///
/// When several rates qualify ([`irr_roots`]) this applies numpy-financial's
/// selection: the smallest non-negative root if any is non-negative,
/// otherwise the negative root closest to zero.
///
/// ```
/// use finance_rs::tvm::irr;
/// let r = irr(&[-100.0, 39.0, 59.0, 55.0, 20.0])?;
/// assert!((r - 0.280_948_421_159_962).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`irr_roots`].
pub fn irr(values: &[f64]) -> Result<f64> {
    let roots = irr_roots(values)?;
    let pick = roots
        .iter()
        .copied()
        .filter(|r| *r >= 0.0)
        .min_by(f64::total_cmp)
        .or_else(|| roots.iter().copied().max_by(f64::total_cmp));
    pick.ok_or(FinanceError::NoSolution { what: "irr" })
}

/// Select the shared periodic IRR only when it lies inside a legacy solver's
/// declared search window.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn irr_with_bounds(values: &[f64], lower: f64, upper: f64) -> Result<f64> {
    if !lower.is_finite() || !upper.is_finite() || lower <= -1.0 || lower >= upper {
        return Err(FinanceError::InvalidInput {
            what: "irr bounds",
            reason: "must satisfy -1 < lower < upper",
        });
    }
    let rate = irr(values)?;
    if (lower..=upper).contains(&rate) {
        Ok(rate)
    } else {
        Err(FinanceError::NoSolution { what: "irr" })
    }
}

/// Periodic IRR with a caller-specified legacy search window and stopping
/// rule. The root calculation lives here so compatibility facades do not
/// carry their own NPV or bisection implementations.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn irr_bisection(
    values: &[f64],
    lower: f64,
    upper: f64,
    npv_tolerance: f64,
    max_iterations: usize,
) -> Result<f64> {
    if values.len() < 2 {
        return Err(FinanceError::TooShort {
            what: "values",
            min: 2,
            got: values.len(),
        });
    }
    if !lower.is_finite()
        || !upper.is_finite()
        || lower <= -1.0
        || lower >= upper
        || !npv_tolerance.is_finite()
        || npv_tolerance <= 0.0
    {
        return Err(FinanceError::InvalidInput {
            what: "irr solver settings",
            reason: "invalid bounds or tolerance",
        });
    }
    let npv = |rate: f64| {
        values
            .iter()
            .enumerate()
            .map(|(period, amount)| amount / (1.0 + rate).powf(period as f64))
            .sum::<f64>()
    };
    let (mut lo, mut hi) = (lower, upper);
    let mut flo = npv(lo);
    let fhi = npv(hi);
    if !flo.is_finite() || !fhi.is_finite() || flo.signum() == fhi.signum() {
        return Err(FinanceError::NoSolution { what: "irr" });
    }
    for _ in 0..max_iterations {
        let mid = (lo + hi) / 2.0;
        let fmid = checked_irr_npv(npv(mid))?;
        if fmid.abs() < npv_tolerance {
            return Ok(mid);
        }
        if flo * fmid < 0.0 {
            hi = mid;
        } else {
            lo = mid;
            flo = fmid;
        }
    }
    Ok((lo + hi) / 2.0)
}

fn checked_irr_npv(value: f64) -> Result<f64> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(FinanceError::Overflow { what: "irr npv" })
    }
}

/// Modified internal rate of return.
///
/// `mirr = (|npv(reinvest, positives)| / |npv(finance, negatives)|)^(1/(n−1)) · (1+reinvest) − 1`
///
/// ```
/// use finance_rs::tvm::mirr;
/// let m = mirr(&[-120_000.0, 39_000.0, 30_000.0, 21_000.0, 37_000.0, 46_000.0], 0.10, 0.12)?;
/// assert!((m - 0.126_094_130_365_905_2).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NoSignChange`] unless there is at least one positive and
/// one negative value; otherwise as [`npv`].
pub fn mirr(values: &[f64], finance_rate: f64, reinvest_rate: f64) -> Result<f64> {
    min_len("values", values, 2)?;
    all_finite("values", values)?;
    if !(values.iter().any(|v| *v > 0.0) && values.iter().any(|v| *v < 0.0)) {
        return Err(FinanceError::NoSignChange);
    }
    let pos: Vec<f64> = values.iter().map(|v| v.max(0.0)).collect();
    let neg: Vec<f64> = values.iter().map(|v| v.min(0.0)).collect();
    let numer = npv(reinvest_rate, &pos)?.abs();
    let denom = npv(finance_rate, &neg)?.abs();
    #[allow(clippy::cast_precision_loss)]
    let periods = (values.len() - 1) as f64;
    output(
        "mirr",
        div("npv of outflows", numer, denom)?.powf(1.0 / periods) * (1.0 + reinvest_rate) - 1.0,
    )
}

fn sign_changes(xs: &[f64]) -> usize {
    let mut last = 0.0_f64;
    let mut n = 0;
    for &x in xs.iter().filter(|x| **x != 0.0) {
        if last != 0.0 && last.signum() != x.signum() {
            n += 1;
        }
        last = x;
    }
    n
}

/// A value with the sign of `P(g) = Σ c_t·g^(N−t)` for `g > 0`, bounded in
/// magnitude: Horner in `g` for `g ≤ 1`, and `P(g)/g^N` (Horner in `1/g`) above.
fn poly_sign_value(coeffs: &[f64], g: f64) -> f64 {
    if g <= 1.0 {
        coeffs.iter().fold(0.0, |acc, c| acc * g + c)
    } else {
        let inv = 1.0 / g;
        coeffs.iter().rev().fold(0.0, |acc, c| acc * inv + c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn npv_of_irr_is_zero() -> Result<()> {
        let flows = [-1_000.0, 300.0, 400.0, 500.0, 200.0];
        let r = irr(&flows)?;
        assert!(npv(r, &flows)?.abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn same_sign_has_no_irr() {
        assert_eq!(irr(&[1.0, 2.0, 3.0]), Err(FinanceError::NoSignChange));
        assert_eq!(irr(&[0.0, 0.0]), Err(FinanceError::NoSignChange));
        assert!(matches!(irr(&[1.0]), Err(FinanceError::TooShort { .. })));
    }

    #[test]
    fn trailing_zero_adds_minus_one() -> Result<()> {
        let roots = irr_roots(&[-100.0, 110.0, 0.0])?;
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0], -1.0);
        assert!((irr(&[-100.0, 110.0, 0.0])? - 0.1).abs() < 1e-12);
        Ok(())
    }

    #[test]
    fn negative_irr() -> Result<()> {
        assert!((irr(&[-100.0, 50.0])? + 0.5).abs() < 1e-12);
        Ok(())
    }

    #[test]
    fn mirr_needs_both_signs() {
        assert_eq!(mirr(&[1.0, 2.0], 0.1, 0.1), Err(FinanceError::NoSignChange));
    }

    #[test]
    fn root_solvers_cover_validation_bounds_and_fallbacks() -> Result<()> {
        assert_eq!(checked_irr_npv(1.0)?, 1.0);
        assert!(matches!(
            checked_irr_npv(f64::INFINITY),
            Err(FinanceError::Overflow { what: "irr npv" })
        ));
        assert!(npv(0.1, &[]).is_err());
        assert!(npv(f64::NAN, &[1.0]).is_err());
        assert!(npv(0.0, &[f64::MAX, f64::MAX]).is_err());
        assert!(irr_roots(&[0.0]).is_err());
        assert!(irr_roots(&[0.0, 0.0]).is_err());
        assert!(irr_roots(&[1.0, 2.0]).is_err());
        assert!(irr_roots(&[f64::NAN, 1.0]).is_err());
        let trailing = irr_roots(&[-100.0, 110.0, 0.0])?;
        assert_eq!(trailing[0], -1.0);
        assert!((trailing[1] - 0.1).abs() < 1e-12);
        assert_eq!(irr_roots(&[-100.0, 100.0, 100.0])?.len(), 1);
        assert!(irr_with_bounds(&[-100.0, 110.0], 0.1, 0.2).is_ok());
        assert!(irr_with_bounds(&[-100.0, 110.0], 0.2, 0.3).is_err());
        assert!(irr_with_bounds(&[-100.0, 110.0], -1.0, 0.2).is_err());
        assert!(irr_with_bounds(&[-100.0, 110.0], 0.3, 0.2).is_err());
        assert!(irr_bisection(&[1.0], -0.5, 0.5, 1e-8, 20).is_err());
        assert!(irr_bisection(&[-100.0, 110.0], -1.0, 0.5, 1e-8, 20).is_err());
        assert!(irr_bisection(&[-100.0, 110.0], -0.5, 0.5, 0.0, 20).is_err());
        assert!(irr_bisection(&[-100.0, -10.0], -0.5, 0.5, 1e-8, 20).is_err());
        let bounded = irr_bisection(&[-100.0, 110.0], -0.5, 0.5, 1e-10, 100)?;
        assert!((bounded - 0.1).abs() < 1e-9);
        assert!(irr_bisection(&[-100.0, 110.0], -0.5, 0.5, 1e-10, 1).is_ok());
        assert!(mirr(&[], 0.1, 0.1).is_err());
        assert!(mirr(&[-1.0, 1.0], -1.0, 0.1).is_err());
        assert!(mirr(&[-1.0, 1.0], 0.0, -1.0).is_err());
        assert_eq!(sign_changes(&[0.0, 1.0, 0.0, -1.0]), 1);
        assert!(poly_sign_value(&[-1.0, 2.0], 0.5).is_sign_positive());
        assert!(poly_sign_value(&[-1.0, 2.0], 2.0).is_sign_positive());
        Ok(())
    }
}
