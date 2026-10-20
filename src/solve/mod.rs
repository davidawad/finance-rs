//! One-dimensional solvers, public so every inversion in the crate and its
//! callers (IRR, XIRR, RATE, bond yield, implied volatility, curve
//! bootstrapping, reverse DCF, breakevens) shares one implementation.
//!
//! | function | method | reference |
//! |---|---|---|
//! | [`brent`] | Brent (1973) on a sign-changing bracket | scipy `brentq` |
//! | [`newton_bracketed`] | Newton steps kept inside a shrinking bracket, bisection when a step leaves it | scipy `newton` + bracket (Numerical Recipes `rtsafe`) |
//! | [`expand_bracket`] | Geometric outward search from a guess until the sign changes | Numerical Recipes `zbrac` |
//! | [`scan_roots`] | Every sign change on a grid, each refined by Brent | cf |
//! | [`fixed_point`] | `x = g(x)` by Steffensen's method (Aitken Δ²) | scipy `fixed_point(method="del2")` |
//!
//! Errors, not `NaN`: no bracket is [`FinanceError::NoSolution`], a spent
//! iteration budget is [`FinanceError::NoConvergence`]. An `Ok` root and its
//! residual are always finite.
//!
//! ```
//! use finance_rs::solve::{brent, SolverOptions};
//! let s = brent(|x| x * x - 2.0, 0.0, 2.0, &SolverOptions::default())?;
//! assert!((s.root - 2f64.sqrt()).abs() < 1e-12);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

pub(crate) mod kernel;

use crate::error::{finite, output, FinanceError, Result};

/// Stopping rules shared by every solver.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct SolverOptions {
    /// Absolute tolerance on the root (default `1e-12`); each solver adds a
    /// few ulp of relative slack so large roots still converge.
    pub xtol: f64,
    /// Stop as soon as `|f(x)| <= ftol` (default `0`: only an exact zero).
    pub ftol: f64,
    /// Iteration budget (default 200).
    pub max_iter: usize,
}

impl Default for SolverOptions {
    fn default() -> Self {
        Self {
            xtol: 1e-12,
            ftol: 0.0,
            max_iter: 200,
        }
    }
}

/// A converged root.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Solved {
    /// The root `x` (for [`fixed_point`], the `x` with `g(x) = x`).
    pub root: f64,
    /// Iterations spent.
    pub iterations: usize,
    /// `f(root)`, or `g(root) − root` for [`fixed_point`].
    pub residual: f64,
}

fn check_options(o: &SolverOptions) -> Result<()> {
    for (what, v) in [("xtol", o.xtol), ("ftol", o.ftol)] {
        finite(what, v)?;
        if v < 0.0 {
            return Err(FinanceError::InvalidInput {
                what,
                reason: "must be non-negative",
            });
        }
    }
    if o.max_iter == 0 {
        return Err(FinanceError::InvalidInput {
            what: "max_iter",
            reason: "must be positive",
        });
    }
    Ok(())
}

fn solved(what: &'static str, root: f64, iterations: usize, residual: f64) -> Result<Solved> {
    Ok(Solved {
        root: output(what, root)?,
        iterations,
        residual: output(what, residual)?,
    })
}

/// Brent's method on `[a, b]`, where `f(a)` and `f(b)` have opposite signs
/// (or one is zero).
///
/// ```
/// use finance_rs::solve::{brent, SolverOptions};
/// let s = brent(f64::cos, 1.0, 2.0, &SolverOptions::default())?;
/// assert!((s.root - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NoSolution`] when the ends do not bracket a sign change
/// (or `f` is `NaN` along the way), [`FinanceError::NoConvergence`] when the
/// budget runs out, [`FinanceError::NonFinite`] / [`FinanceError::InvalidInput`]
/// for bad ends or options.
pub fn brent(f: impl Fn(f64) -> f64, a: f64, b: f64, o: &SolverOptions) -> Result<Solved> {
    check_options(o)?;
    finite("a", a)?;
    finite("b", b)?;
    let (root, iterations) = kernel::brent_counted("brent", &f, a, b, o.xtol, o.ftol, o.max_iter)?;
    solved("brent", root, iterations, f(root))
}

/// Newton's method from `guess`, safeguarded by `bracket`: a step that
/// leaves the current bracket, or does not at least halve the error the way
/// bisection would, is replaced by a bisection step, and the bracket shrinks
/// around the sign change every iteration (Numerical Recipes `rtsafe`).
///
/// ```
/// use finance_rs::solve::{newton_bracketed, SolverOptions};
/// let s = newton_bracketed(|x| x.powi(3) - 2.0, |x| 3.0 * x * x, 1.0, (0.0, 2.0),
///                          &SolverOptions::default())?;
/// assert!((s.root - 2f64.cbrt()).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] when `guess` lies outside `bracket`;
/// otherwise as [`brent`].
pub fn newton_bracketed(
    f: impl Fn(f64) -> f64,
    df: impl Fn(f64) -> f64,
    guess: f64,
    bracket: (f64, f64),
    o: &SolverOptions,
) -> Result<Solved> {
    const WHAT: &str = "newton_bracketed";
    check_options(o)?;
    finite("guess", guess)?;
    let (lo, hi) = ordered(bracket)?;
    if guess < lo || guess > hi {
        return Err(FinanceError::InvalidInput {
            what: "guess",
            reason: "must lie inside the bracket",
        });
    }
    let (flo, fhi) = (f(lo), f(hi));
    if flo == 0.0 {
        return solved(WHAT, lo, 0, 0.0);
    }
    if fhi == 0.0 {
        return solved(WHAT, hi, 0, 0.0);
    }
    if flo.is_nan() || fhi.is_nan() || flo.signum() == fhi.signum() {
        return Err(FinanceError::NoSolution { what: WHAT });
    }
    // Orient so f(neg) < 0 < f(pos).
    let (mut neg, mut pos) = if flo < 0.0 { (lo, hi) } else { (hi, lo) };
    let mut x = guess;
    let mut step_old = hi - lo;
    let mut step = step_old;
    let (mut fx, mut dfx) = (f(x), df(x));
    for iter in 1..=o.max_iter {
        let newton = x - fx / dfx;
        let inside = newton.is_finite() && (newton - neg) * (newton - pos) < 0.0;
        let fast = (2.0 * fx).abs() <= (step_old * dfx).abs();
        step_old = step;
        let next = if inside && fast && fx.is_finite() {
            newton
        } else {
            neg + 0.5 * (pos - neg)
        };
        step = next - x;
        x = next;
        fx = f(x);
        dfx = df(x);
        let tol = o.xtol + 4.0 * f64::EPSILON * x.abs();
        if fx == 0.0 || fx.abs() <= o.ftol || step.abs() <= tol || (pos - neg).abs() <= tol {
            return solved(WHAT, x, iter, fx);
        }
        if fx.is_nan() {
            return Err(FinanceError::NoSolution { what: WHAT });
        }
        if fx < 0.0 {
            neg = x;
        } else {
            pos = x;
        }
    }
    Err(FinanceError::NoConvergence {
        what: WHAT,
        iterations: o.max_iter,
    })
}

#[allow(clippy::float_cmp)] // an empty interval is exactly a == b
fn ordered((a, b): (f64, f64)) -> Result<(f64, f64)> {
    finite("bracket", a)?;
    finite("bracket", b)?;
    if a == b {
        return Err(FinanceError::InvalidInput {
            what: "bracket",
            reason: "ends must differ",
        });
    }
    Ok((a.min(b), a.max(b)))
}

/// Search outward from `guess`, within `domain`, for an interval on which
/// `f` changes sign; the steps double each round, alternating sides, and the
/// returned bracket is the last step that crossed (so it stays narrow).
/// A zero at `guess` returns `(guess, guess)`.
///
/// ```
/// use finance_rs::solve::expand_bracket;
/// let (a, b) = expand_bracket(|x| x - 37.0, 1.0, (0.0, 1e3))?;
/// assert!(a <= 37.0 && 37.0 <= b);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NoSolution`] when the whole domain is searched without a
/// sign change; [`FinanceError::InvalidInput`] when `guess` is outside
/// `domain` or the domain is empty.
pub fn expand_bracket(
    f: impl Fn(f64) -> f64,
    guess: f64,
    domain: (f64, f64),
) -> Result<(f64, f64)> {
    finite("guess", guess)?;
    let (dlo, dhi) = ordered(domain)?;
    if guess < dlo || guess > dhi {
        return Err(FinanceError::InvalidInput {
            what: "guess",
            reason: "must lie inside the domain",
        });
    }
    let f0 = f(guess);
    if f0 == 0.0 {
        return Ok((guess, guess));
    }
    let mut step = (0.1 * guess.abs().max(1.0)).min(0.25 * (dhi - dlo));
    let (mut lo, mut flo) = (guess, f0);
    let (mut hi, mut fhi) = (guess, f0);
    let crosses =
        |a: f64, b: f64| !a.is_nan() && !b.is_nan() && (b == 0.0 || a.signum() != b.signum());
    for _ in 0..2_100 {
        if lo <= dlo && hi >= dhi {
            break;
        }
        if hi < dhi {
            let x = (guess + step).min(dhi);
            let fx = f(x);
            if crosses(fhi, fx) {
                return Ok((hi, x));
            }
            (hi, fhi) = (x, if fx.is_nan() { fhi } else { fx });
        }
        if lo > dlo {
            let x = (guess - step).max(dlo);
            let fx = f(x);
            if crosses(flo, fx) {
                return Ok((x, lo));
            }
            (lo, flo) = (x, if fx.is_nan() { flo } else { fx });
        }
        step *= 2.0;
    }
    Err(FinanceError::NoSolution {
        what: "expand_bracket",
    })
}

/// Every root `f` crosses between consecutive points of the ascending
/// `grid`, each refined by Brent to `o.xtol`; exact zeros on grid points are
/// reported once. Tangent roots that never change sign are not found. An
/// empty result means no sign change on the grid.
///
/// ```
/// use finance_rs::solve::{scan_roots, SolverOptions};
/// let grid: Vec<f64> = (0..=80).map(|i| -4.0 + 0.1 * f64::from(i)).collect();
/// let r = scan_roots(|x| (x - 1.0) * (x + 2.0) * (x - 3.0), &grid, &SolverOptions::default())?;
/// assert_eq!(r.len(), 3);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::TooShort`] for fewer than two grid points,
/// [`FinanceError::InvalidInput`] unless the grid is finite and strictly
/// ascending.
pub fn scan_roots(f: impl Fn(f64) -> f64, grid: &[f64], o: &SolverOptions) -> Result<Vec<f64>> {
    check_options(o)?;
    crate::error::min_len("grid", grid, 2)?;
    crate::error::all_finite("grid", grid)?;
    if grid.windows(2).any(|w| w[0] >= w[1]) {
        return Err(FinanceError::InvalidInput {
            what: "grid",
            reason: "must be strictly ascending",
        });
    }
    Ok(kernel::scan_roots("scan_roots", f, grid, o.xtol))
}

/// A fixed point `x = g(x)` from `x0` by Steffensen's method: two plain
/// iterations `p₁ = g(p₀)`, `p₂ = g(p₁)`, then Aitken's
/// `p = p₀ − (p₁ − p₀)²/(p₂ − 2p₁ + p₀)` (plain `p₂` when the denominator is
/// zero), until `|p − p₀| <= xtol·max(1, |p₀|)` (scipy's `del2`). Covers the
/// LBO average-balance interest circularity and dilution loops.
///
/// ```
/// use finance_rs::solve::{fixed_point, SolverOptions};
/// let s = fixed_point(f64::cos, 1.0, &SolverOptions::default())?;
/// assert!((s.root - 0.739_085_133_215_160_6).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NoSolution`] when `g` leaves the finite numbers,
/// [`FinanceError::NoConvergence`] when the budget runs out.
pub fn fixed_point(g: impl Fn(f64) -> f64, x0: f64, o: &SolverOptions) -> Result<Solved> {
    const WHAT: &str = "fixed_point";
    check_options(o)?;
    let mut p0 = finite("x0", x0)?;
    for iter in 1..=o.max_iter {
        let p1 = g(p0);
        let p2 = g(p1);
        let d = p2 - 2.0 * p1 + p0;
        let p = if d == 0.0 {
            p2
        } else {
            p0 - (p1 - p0) * (p1 - p0) / d
        };
        if !p.is_finite() || !p1.is_finite() || !p2.is_finite() {
            return Err(FinanceError::NoSolution { what: WHAT });
        }
        if (p - p0).abs() <= o.xtol * p0.abs().max(1.0) {
            return solved(WHAT, p, iter, g(p) - p);
        }
        p0 = p;
    }
    Err(FinanceError::NoConvergence {
        what: WHAT,
        iterations: o.max_iter,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn newton_falls_back_to_bisection() {
        // A zero derivative at the guess forces bisection.
        let f = |x| x * x * x - x - 2.0;
        let df = |x| 3.0 * x * x - 1.0;
        let options = SolverOptions::default();
        let s = newton_bracketed(f, df, 1.0 / 3f64.sqrt(), (0.0, 3.0), &options)
            .expect("bisection fallback converges");
        assert!((s.root - 1.521_379_706_804_567_6).abs() < 1e-12);
    }

    #[test]
    fn expand_finds_far_and_negative_roots() -> Result<()> {
        let (a, b) = expand_bracket(|x| x + 500.0, 0.0, (-1e6, 1e6))?;
        assert!(a <= -500.0 && -500.0 <= b);
        assert!(expand_bracket(|x| x * x + 1.0, 0.0, (-10.0, 10.0)).is_err());
        Ok(())
    }

    #[test]
    #[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
    fn options_are_validated() {
        let bad = SolverOptions {
            max_iter: 0,
            ..SolverOptions::default()
        };
        assert!(brent(|x| x, -1.0, 1.0, &bad).is_err());
        let bad = SolverOptions {
            xtol: f64::NAN,
            ..SolverOptions::default()
        };
        assert!(fixed_point(f64::cos, 1.0, &bad).is_err());
        let bad = SolverOptions {
            xtol: -1.0,
            ..SolverOptions::default()
        };
        assert!(brent(|x| x, -1.0, 1.0, &bad).is_err());
        assert!(scan_roots(|x| x, &[1.0, 0.0], &SolverOptions::default()).is_err());
        let non_finite_iterate = newton_bracketed(
            |x| {
                if x == 0.5 {
                    f64::NAN
                } else {
                    x - 0.5
                }
            },
            |_| 1.0,
            0.0,
            (-1.0, 1.0),
            &SolverOptions::default(),
        );
        assert!(
            matches!(
                non_finite_iterate,
                Err(FinanceError::NoSolution {
                    what: "newton_bracketed"
                })
            ),
            "got {non_finite_iterate:?}"
        );
    }
}
