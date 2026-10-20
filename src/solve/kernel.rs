//! The crate-internal solver kernels behind IRR, XIRR, RATE, bond yield and
//! the LBO: Brent's method on a bracket, plain Newton, and a grid scan that
//! reports every sign change. The public [`super`] API wraps the same kernels.

use crate::error::{FinanceError, Result};

/// Iteration budget for bracketed solvers.
pub(crate) const MAX_ITER: usize = 200;

/// Brent's method on `[a, b]` where `f(a)` and `f(b)` have opposite signs
/// (or one is zero). Converges to within `xtol` (absolute plus 4 ulp relative).
pub(crate) fn brent<F: Fn(f64) -> f64>(
    what: &'static str,
    f: F,
    a: f64,
    b: f64,
    xtol: f64,
) -> Result<f64> {
    brent_counted(what, f, a, b, xtol, 0.0, MAX_ITER).map(|(root, _)| root)
}

#[allow(clippy::float_cmp, clippy::many_single_char_names)] // Brent 1973 notation
/// Brent's method returning `(root, iterations)`. Stops when the bracket
/// half-width is within `xtol` (plus 4 ulp relative) or `|f| <= ftol`.
pub(crate) fn brent_counted<F: Fn(f64) -> f64>(
    what: &'static str,
    f: F,
    mut a: f64,
    mut b: f64,
    xtol: f64,
    ftol: f64,
    max_iter: usize,
) -> Result<(f64, usize)> {
    let mut fa = f(a);
    let mut fb = f(b);
    if fa == 0.0 {
        return Ok((a, 0));
    }
    if fb == 0.0 {
        return Ok((b, 0));
    }
    if fa.is_nan() || fb.is_nan() || fa.signum() == fb.signum() {
        return Err(FinanceError::NoSolution { what });
    }
    let (mut c, mut fc) = (a, fa);
    let mut d = b - a;
    let mut e = d;
    for iter in 0..max_iter {
        if fb.signum() == fc.signum() {
            c = a;
            fc = fa;
            d = b - a;
            e = d;
        }
        if fc.abs() < fb.abs() {
            a = b;
            b = c;
            c = a;
            fa = fb;
            fb = fc;
            fc = fa;
        }
        let tol = 2.0 * f64::EPSILON * b.abs() + 0.5 * xtol;
        let m = 0.5 * (c - b);
        if m.abs() <= tol || fb.abs() <= ftol {
            return Ok((b, iter));
        }
        if e.abs() >= tol && fa.abs() > fb.abs() {
            let s = fb / fa;
            let (mut p, mut q);
            if a == c {
                p = 2.0 * m * s;
                q = 1.0 - s;
            } else {
                let qa = fa / fc;
                let r = fb / fc;
                p = s * (2.0 * m * qa * (qa - r) - (b - a) * (r - 1.0));
                q = (qa - 1.0) * (r - 1.0) * (s - 1.0);
            }
            if p > 0.0 {
                q = -q;
            } else {
                p = -p;
            }
            if 2.0 * p < (3.0 * m * q - (tol * q).abs()).min((e * q).abs()) {
                e = d;
                d = p / q;
            } else {
                d = m;
                e = m;
            }
        } else {
            d = m;
            e = m;
        }
        a = b;
        fa = fb;
        b += if d.abs() > tol { d } else { tol.copysign(m) };
        fb = f(b);
        if fb.is_nan() {
            return Err(FinanceError::NoSolution { what });
        }
    }
    Err(FinanceError::NoConvergence {
        what,
        iterations: max_iter,
    })
}

/// Newton's method from `x0` on `f_df(x) = (f(x), f'(x))`; `None` when it
/// diverges, stalls on a flat derivative or exhausts `max_iter`.
pub(crate) fn newton<F: Fn(f64) -> (f64, f64)>(
    f_df: F,
    x0: f64,
    tol: f64,
    max_iter: usize,
) -> Option<f64> {
    let mut x = x0;
    for _ in 0..max_iter {
        let (fx, dfx) = f_df(x);
        if !fx.is_finite() || !dfx.is_finite() || dfx == 0.0 {
            return None;
        }
        let next = x - fx / dfx;
        if !next.is_finite() {
            return None;
        }
        if (next - x).abs() <= tol * (1.0 + next.abs()) {
            return Some(next);
        }
        x = next;
    }
    None
}

/// Every root of `f` bracketed by consecutive points of the ascending `grid`,
/// each polished with Brent. Exact zeros on grid points are reported once.
/// Tangent (even-multiplicity) roots that never change sign are not found.
pub(crate) fn scan_roots<F: Fn(f64) -> f64>(
    what: &'static str,
    f: F,
    grid: &[f64],
    xtol: f64,
) -> Vec<f64> {
    let mut roots: Vec<f64> = Vec::new();
    let mut prev: Option<(f64, f64)> = None;
    for &x in grid {
        let fx = f(x);
        if fx.is_nan() {
            prev = None;
            continue;
        }
        if fx == 0.0 {
            roots.push(x);
        } else if let Some((px, pf)) = prev {
            if pf != 0.0 && pf.signum() != fx.signum() {
                if let Ok(r) = brent(what, &f, px, x, xtol) {
                    roots.push(r);
                }
            }
        }
        prev = Some((x, fx));
    }
    roots.dedup_by(|a, b| (*a - *b).abs() <= xtol.max(1e-14 * a.abs()));
    roots
}

/// `n` points from `lo` to `hi` (both > 0), evenly spaced in log space.
pub(crate) fn geometric_grid(lo: f64, hi: f64, n: usize) -> Vec<f64> {
    let (l, h) = (lo.ln(), hi.ln());
    let steps = n.max(2) - 1;
    #[allow(clippy::cast_precision_loss)]
    let step = (h - l) / steps as f64;
    #[allow(clippy::cast_precision_loss)]
    (0..=steps).map(|i| (l + step * i as f64).exp()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brent_finds_sqrt2() -> Result<()> {
        let r = brent("t", |x| x * x - 2.0, 0.0, 2.0, 1e-15)?;
        assert!((r - 2f64.sqrt()).abs() < 1e-14);
        Ok(())
    }

    #[test]
    fn brent_rejects_no_bracket() {
        assert!(brent("t", |x| x * x + 1.0, -1.0, 1.0, 1e-12).is_err());
    }

    #[test]
    fn brent_returns_exact_upper_endpoint_without_iterations() -> Result<()> {
        let (root, iterations) = brent_counted("t", |x| x - 2.0, 0.0, 2.0, 1e-12, 0.0, 10)?;
        assert_eq!(root, 2.0);
        assert_eq!(iterations, 0);
        Ok(())
    }

    #[test]
    fn scan_finds_all_cubic_roots() {
        let grid: Vec<f64> = (0..=400).map(|i| -4.0 + 0.02 * f64::from(i)).collect();
        let roots = scan_roots("t", |x| (x - 1.0) * (x + 2.0) * (x - 3.0), &grid, 1e-14);
        assert_eq!(roots.len(), 3, "{roots:?}");
        for (r, want) in roots.iter().zip([-2.0, 1.0, 3.0]) {
            assert!((r - want).abs() < 1e-12);
        }
    }

    #[test]
    fn newton_converges() {
        let r = newton(|x| (x * x - 2.0, 2.0 * x), 1.0, 1e-15, 50);
        assert!(r.is_some_and(|r| (r - 2f64.sqrt()).abs() < 1e-14));
    }

    #[test]
    fn newton_stops_when_an_iterate_overflows() {
        assert_eq!(
            newton(|_| (f64::MAX, f64::MIN_POSITIVE), 0.0, 1e-12, 10),
            None
        );
    }
}
