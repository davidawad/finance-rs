//! Standard normal distribution: density, CDF and inverse CDF, `std` only.
//!
//! The CDF goes through `erfc`, evaluated by its Taylor series (as
//! `1 − erf`) below `|z| = 2` and by Lentz's continued fraction above it.
//! The inverse CDF is Acklam's rational approximation (relative error
//! about 1.15e-9) polished by Halley steps against that CDF, which brings it
//! to within a few ulp of scipy's `norm.ppf`.

use crate::error::{finite, FinanceError, Result};

const FRAC_1_SQRT_2PI: f64 = 0.398_942_280_401_432_7;
const SQRT_2PI: f64 = 2.506_628_274_631_000_5;
const FRAC_2_SQRT_PI: f64 = std::f64::consts::FRAC_2_SQRT_PI;

/// `erf(z)` for `0 <= z < 2` by `2/√π · e^(−z²) · Σ 2ⁿ z^(2n+1) / (2n+1)!!`
/// (every term positive, so no cancellation).
fn erf_series(z: f64) -> f64 {
    let two_z2 = 2.0 * z * z;
    let mut term = z;
    let mut sum = z;
    let mut k = 1.0;
    while term > sum * 1e-17 {
        k += 2.0;
        term *= two_z2 / k;
        sum += term;
    }
    FRAC_2_SQRT_PI * (-z * z).exp() * sum
}

/// `erfc(z)` for `z >= 2` by the continued fraction
/// `e^(−z²)/√π / (z + (1/2)/(z + 1/(z + (3/2)/(z + …))))` (modified Lentz).
#[allow(clippy::many_single_char_names)] // Conventional symbols follow the published formula or recurrence notation.
fn erfc_cf(z: f64) -> f64 {
    let mut f = z;
    let mut c = z;
    let mut d = 0.0;
    for n in 1..500_u32 {
        let a = f64::from(n) / 2.0;
        d = z + a * d;
        // For z >= 2 and a > 0, both Lentz denominators stay positive.
        c = z + a / c;
        d = 1.0 / d;
        let delta = c * d;
        f *= delta;
        if (delta - 1.0).abs() < 1e-16 {
            break;
        }
    }
    (-z * z).exp() / std::f64::consts::PI.sqrt() / f
}

/// Complementary error function.
fn erfc(z: f64) -> f64 {
    if z < 0.0 {
        2.0 - erfc(-z)
    } else if z < 2.0 {
        1.0 - erf_series(z)
    } else {
        erfc_cf(z)
    }
}

/// Standard normal density `φ(x) = e^(−x²/2) / √(2π)`.
///
/// ```rust
/// use finance_rs::perf::normal_pdf;
/// assert!((normal_pdf(0.0) - 0.398_942_280_401_432_7).abs() < 1e-16);
/// ```
#[must_use]
pub fn normal_pdf(x: f64) -> f64 {
    FRAC_1_SQRT_2PI * (-0.5 * x * x).exp()
}

/// Standard normal CDF `Φ(x) = erfc(−x/√2) / 2`.
///
/// ```rust
/// use finance_rs::perf::normal_cdf;
/// assert!((normal_cdf(1.0) - 0.841_344_746_068_542_9).abs() < 1e-15);
/// ```
#[must_use]
pub fn normal_cdf(x: f64) -> f64 {
    0.5 * erfc(-x * std::f64::consts::FRAC_1_SQRT_2)
}

/// Acklam's initial estimate for `p` in `(0, 0.5]`.
fn acklam_lower(p: f64) -> f64 {
    const A: [f64; 6] = [
        -3.969_683_028_665_376e1,
        2.209_460_984_245_205e2,
        -2.759_285_104_469_687e2,
        1.383_577_518_672_69e2,
        -3.066_479_806_614_716e1,
        2.506_628_277_459_239,
    ];
    const B: [f64; 5] = [
        -5.447_609_879_822_406e1,
        1.615_858_368_580_409e2,
        -1.556_989_798_598_866e2,
        6.680_131_188_771_972e1,
        -1.328_068_155_288_572e1,
    ];
    const C: [f64; 6] = [
        -7.784_894_002_430_293e-3,
        -3.223_964_580_411_365e-1,
        -2.400_758_277_161_838,
        -2.549_732_539_343_734,
        4.374_664_141_464_968,
        2.938_163_982_698_783,
    ];
    const D: [f64; 4] = [
        7.784_695_709_041_462e-3,
        3.224_671_290_700_398e-1,
        2.445_134_137_142_996,
        3.754_408_661_907_416,
    ];
    if p < 0.02425 {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    }
}

/// Inverse standard normal CDF (the probit, scipy's `norm.ppf`).
///
/// Acklam's rational approximation followed by Halley refinement
/// `x ← x − u/(1 + x·u/2)`, `u = (Φ(x) − p)·√(2π)·e^(x²/2)`. Upper-tail
/// arguments use `Φ⁻¹(p) = −Φ⁻¹(1 − p)` (exact in floating point for
/// `p >= 0.5`).
///
/// ```rust
/// use finance_rs::perf::normal_ppf;
/// let z = normal_ppf(0.05)?;
/// assert!((z + 1.644_853_626_951_472_9).abs() < 1e-14);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] unless `0 < p < 1`, [`FinanceError::NonFinite`]
/// for `NaN`.
pub fn normal_ppf(p: f64) -> Result<f64> {
    finite("p", p)?;
    if p <= 0.0 || p >= 1.0 {
        return Err(FinanceError::InvalidInput {
            what: "p",
            reason: "probability must be in (0, 1)",
        });
    }
    if p > 0.5 {
        return Ok(-ppf_lower(1.0 - p));
    }
    Ok(ppf_lower(p))
}

/// `Φ⁻¹(p)` for `p` in `(0, 0.5]`.
#[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
fn ppf_lower(p: f64) -> f64 {
    if p == 0.5 {
        return 0.0;
    }
    let mut x = acklam_lower(p);
    for _ in 0..2 {
        let e = normal_cdf(x) - p;
        let u = e * SQRT_2PI * (0.5 * x * x).exp();
        let next = x - u / (1.0 + 0.5 * x * u);
        if !next.is_finite() {
            break;
        }
        x = next;
    }
    x
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn close(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * b.abs().max(1e-300)
    }

    #[test]
    fn ppf_matches_scipy() {
        let cases = [
            (0.05, -1.644_853_626_951_472_9),
            (0.01, -2.326_347_874_040_840_8),
            (0.025, -1.959_963_984_540_054_5),
            (0.3, -0.524_400_512_708_040_9),
            (0.9, 1.281_551_565_544_600_4),
            (0.975, 1.959_963_984_540_054),
            (1e-10, -6.361_340_902_404_056),
            (1e-300, -37.047_096_299_361_2),
            (0.02425, -1.972_961_051_311_885),
            (0.97575, 1.972_961_051_311_885),
        ];
        for (p, want) in cases {
            let got = normal_ppf(p).unwrap();
            assert!(close(got, want, 4e-15), "ppf({p}) = {got}, want {want}");
        }
        assert_eq!(normal_ppf(0.5).unwrap(), 0.0);
    }

    #[test]
    fn cdf_matches_scipy() {
        let cases = [
            (-8.0, 6.220_960_574_271_74e-16),
            (-3.0, 0.001_349_898_031_630_093_3),
            (-1.5, 0.066_807_201_268_858_07),
            (0.0, 0.5),
            (1.0, 0.841_344_746_068_542_9),
            (2.5, 0.993_790_334_674_223_8),
            (6.0, 0.999_999_999_013_412_3),
        ];
        for (x, want) in cases {
            let got = normal_cdf(x);
            assert!(close(got, want, 1e-13), "cdf({x}) = {got}, want {want}");
        }
        assert!(close(normal_pdf(-3.0), 0.004_431_848_411_938_007_5, 1e-15));
    }

    #[test]
    fn ppf_rejects_out_of_domain() {
        for p in [0.0, 1.0, -0.1, 1.5] {
            assert!(matches!(
                normal_ppf(p),
                Err(FinanceError::InvalidInput { .. })
            ));
        }
        assert!(matches!(
            normal_ppf(f64::NAN),
            Err(FinanceError::NonFinite { .. })
        ));
    }

    #[test]
    fn ppf_inverts_cdf() {
        let mut p = 1e-12;
        while p < 1.0 {
            let x = normal_ppf(p).unwrap();
            assert!(close(normal_cdf(x), p, 1e-12), "p = {p}");
            p *= 1.7;
        }
        assert!(normal_ppf(f64::from_bits(1)).unwrap().is_finite());
    }
}
