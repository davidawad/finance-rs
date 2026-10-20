//! `rate`: numpy-financial's Newton iteration, with a bracketed fallback.

use super::When;
use crate::error::{finite, FinanceError, Result};
use crate::solve::kernel::{brent, geometric_grid};

/// Solver settings for [`rate_with`] (numpy-financial's `guess`, `tol`, `maxiter`).
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct RateOptions {
    /// Newton starting point (numpy-financial default 0.1).
    pub guess: f64,
    /// Stop when successive iterates differ by less than this (default 1e-6).
    pub tol: f64,
    /// Newton iteration budget (default 100).
    pub max_iter: usize,
    /// When Newton fails, search `(-1, 1e6)` for a sign change and solve it
    /// with Brent's method instead of failing (default `true`; numpy-financial
    /// would return `NaN`).
    pub bracket_fallback: bool,
}

impl Default for RateOptions {
    fn default() -> Self {
        Self {
            guess: 0.1,
            tol: 1e-6,
            max_iter: 100,
            bracket_fallback: true,
        }
    }
}

/// Interest rate per period, with default [`RateOptions`].
///
/// Solves `g(r) = fv + pv·(1+r)^n + pmt·(1+r·w)/r·((1+r)^n − 1) = 0` by
/// Newton's method `r ← r − g(r)/g'(r)` from `guess`, exactly as
/// numpy-financial does.
///
/// ```
/// use finance_rs::tvm::{rate, When};
/// // A 10,000 loan repaid by 48 payments of 250: the monthly rate.
/// let r = rate(48.0, -250.0, 10_000.0, 0.0, When::End)?;
/// assert!((r - 0.007_701_472_488_232_971).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`rate_with`].
pub fn rate(nper: f64, pmt: f64, pv: f64, fv: f64, when: When) -> Result<f64> {
    rate_with(nper, pmt, pv, fv, when, RateOptions::default())
}

/// Interest rate per period with explicit solver settings.
///
/// ```
/// use finance_rs::tvm::{rate_with, RateOptions, When};
/// let opts = RateOptions { guess: 0.05, ..RateOptions::default() };
/// let r = rate_with(10.0, 0.0, -100.0, 200.0, When::End, opts)?;
/// assert!((r - (2f64.powf(0.1) - 1.0)).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NonFinite`] for non-finite input, [`FinanceError::NoConvergence`]
/// when Newton fails and the fallback is disabled or finds no sign change.
pub fn rate_with(
    nper: f64,
    pmt: f64,
    pv: f64,
    fv: f64,
    when: When,
    opts: RateOptions,
) -> Result<f64> {
    finite("nper", nper)?;
    finite("pmt", pmt)?;
    finite("pv", pv)?;
    finite("fv", fv)?;
    finite("guess", opts.guess)?;
    let w = when.factor();
    let mut rn = opts.guess;
    for _ in 0..opts.max_iter {
        let next = rn - g_div_gp(rn, nper, pmt, pv, fv, w);
        if !next.is_finite() {
            break;
        }
        let close = (next - rn).abs() < opts.tol;
        rn = next;
        if close {
            return Ok(rn);
        }
    }
    let failed = FinanceError::NoConvergence {
        what: "rate",
        iterations: opts.max_iter,
    };
    if !opts.bracket_fallback {
        return Err(failed);
    }
    let g = |r: f64| fallback_equation(r, nper, pmt, pv, fv, w);
    // Scan 1+r over (1e-9, 1e6) for the sign change nearest r = 0.
    let grid = geometric_grid(1e-9, 1e6, 2_000);
    let mut best: Option<f64> = None;
    for pair in grid.windows(2) {
        let (a, b) = (pair[0] - 1.0, pair[1] - 1.0);
        let (ga, gb) = (g(a), g(b));
        if ga.is_finite() && gb.is_finite() && (ga == 0.0 || ga.signum() != gb.signum()) {
            best = brent("rate", g, a, b, 1e-15).ok().map_or(best, |r| {
                Some(best.map_or(r, |x| if r.abs() < x.abs() { r } else { x }))
            });
        }
    }
    best.ok_or(failed)
}

fn fallback_equation(r: f64, nper: f64, pmt: f64, pv: f64, fv: f64, w: f64) -> f64 {
    if r == 0.0 {
        fv + pv + pmt * nper
    } else {
        let t1 = (1.0 + r).powf(nper);
        fv + pv * t1 + pmt * (1.0 + r * w) * (t1 - 1.0) / r
    }
}

/// numpy-financial's `_g_div_gp`: `g(r)/g'(r)`.
#[allow(clippy::many_single_char_names)] // Conventional symbols follow the published formula or recurrence notation.
fn g_div_gp(r: f64, n: f64, p: f64, x: f64, y: f64, w: f64) -> f64 {
    let t1 = (r + 1.0).powf(n);
    let t2 = (r + 1.0).powf(n - 1.0);
    let g = y + t1 * x + p * (t1 - 1.0) * (r * w + 1.0) / r;
    let gp = n * t2 * x - p * (t1 - 1.0) * (r * w + 1.0) / (r * r)
        + n * p * t2 * (r * w + 1.0) / r
        + p * (t1 - 1.0) * w / r;
    g / gp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_rescues_bad_guess() -> Result<()> {
        // Newton from a guess of 0 divides by zero; the bracket still solves it.
        let opts = RateOptions {
            guess: 0.0,
            ..RateOptions::default()
        };
        let r = rate_with(12.0, -100.0, 1_000.0, 0.0, When::End, opts)?;
        let fv = crate::tvm::fv(r, 12.0, -100.0, 1_000.0, When::End)?;
        assert!(fv.abs() < 1e-8, "{r} {fv}");
        let strict = RateOptions {
            bracket_fallback: false,
            ..opts
        };
        assert!(rate_with(12.0, -100.0, 1_000.0, 0.0, When::End, strict).is_err());
        Ok(())
    }

    #[test]
    fn no_solution_is_an_error() {
        // Receiving money every period and at the end: no rate balances it.
        assert!(rate(10.0, 100.0, 100.0, 100.0, When::End).is_err());
    }

    #[test]
    fn fallback_evaluates_zero_rate_limit() {
        assert_eq!(fallback_equation(0.0, 2.0, -10.0, 20.0, 0.0, 1.0), 0.0);
    }
}
