//! The XIRR root finder over year fractions.

use super::validate;
use crate::error::{finite, FinanceError, Result};
use crate::solve::kernel::{brent, newton, scan_roots};

/// Search window for `u = ln(1+r)`: `1+r ∈ [e^-21, e^21]` (about 1e-9 to 1e9).
const U_MAX: f64 = 21.0;
const SCAN_POINTS: usize = 8_400;

/// Flows merged by time and sorted, zeros dropped: `(t, amount)`.
#[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
fn merged(times: &[f64], amounts: &[f64]) -> Vec<(f64, f64)> {
    let mut flows: Vec<(f64, f64)> = times.iter().copied().zip(amounts.iter().copied()).collect();
    flows.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(flows.len());
    for (t, a) in flows {
        match out.last_mut() {
            Some(last) if last.0 == t => last.1 += a,
            _ => out.push((t, a)),
        }
    }
    out.retain(|f| f.1 != 0.0);
    out
}

/// `xnpv` at `1+r = e^u`, divided by its largest term's magnitude: same sign,
/// never overflows.
fn scaled(flows: &[(f64, f64)], u: f64) -> f64 {
    let logs = flows.iter().map(|(t, a)| a.abs().ln() - t * u);
    let m = logs.clone().fold(f64::NEG_INFINITY, f64::max);
    flows
        .iter()
        .zip(logs)
        .map(|((_, a), l)| a.signum() * (l - m).exp())
        .sum()
}

fn sign_changes(flows: &[(f64, f64)]) -> usize {
    flows
        .windows(2)
        .filter(|w| w[0].1.signum() != w[1].1.signum())
        .count()
}

/// Every rate solving `Σ a_i (1+r)^-t_i = 0`, ascending.
///
/// ```rust
/// use finance_rs::cashflow::xirr_roots_times;
/// let roots = xirr_roots_times(&[0.0, 1.0, 2.0], &[-100.0, 230.0, -132.0])?;
/// assert!((roots[0] - 0.1).abs() < 1e-12 && (roots[1] - 0.2).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::NoSignChange`] without both a positive and a negative
/// flow, [`FinanceError::NoSolution`] when none is bracketed in the window,
/// and length/finiteness errors.
pub fn xirr_roots_times(times: &[f64], amounts: &[f64]) -> Result<Vec<f64>> {
    validate(times, amounts)?;
    let flows = merged(times, amounts);
    let changes = sign_changes(&flows);
    if changes == 0 {
        return Err(FinanceError::NoSignChange);
    }
    let f = |u: f64| scaled(&flows, u);
    let us = if changes == 1 {
        vec![unique_root(&flows, 0.0)?]
    } else {
        #[allow(clippy::cast_precision_loss)]
        let step = 2.0 * U_MAX / SCAN_POINTS as f64;
        #[allow(clippy::cast_precision_loss)]
        let mut grid: Vec<f64> = (0..=SCAN_POINTS)
            .map(|i| -U_MAX + step * i as f64)
            .collect();
        grid.push(0.0);
        grid.sort_by(f64::total_cmp);
        scan_roots("xirr", f, &grid, 0.0)
    };
    if us.is_empty() {
        return Err(FinanceError::NoSolution { what: "xirr" });
    }
    Ok(us.into_iter().map(f64::exp_m1).collect())
}

/// XIRR over year fractions. With one sign change the unique root; with
/// several, the bracketed root Newton's method converges to from `guess`
/// (pyxirr's choice), else the root closest to `guess`; Newton alone when
/// nothing is bracketed (a tangent root).
///
/// ```rust
/// use finance_rs::cashflow::xirr_times;
/// let r = xirr_times(&[0.0, 0.5, 1.0], &[-1_000.0, 100.0, 1_000.0], 0.1)?;
/// assert!((r - 0.105_124_921_972_503_75).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`xirr_roots_times`]; also [`FinanceError::InvalidInput`] for `guess <= -1`.
pub fn xirr_times(times: &[f64], amounts: &[f64], guess: f64) -> Result<f64> {
    finite("guess", guess)?;
    if guess <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "guess",
            reason: "must be greater than -1",
        });
    }
    validate(times, amounts)?;
    let flows = merged(times, amounts);
    let changes = sign_changes(&flows);
    if changes == 0 {
        return Err(FinanceError::NoSignChange);
    }
    if changes == 1 {
        return unique_root(&flows, guess.ln_1p()).map(f64::exp_m1);
    }
    let newton_root = newton_fallback(&flows, guess).ok();
    // `validate` already succeeded above and `changes > 1`, so root search
    // can fail only because it found no root in its finite search window.
    match xirr_roots_times(times, amounts).ok() {
        // Prefer the root Newton reaches from the guess (pyxirr's first
        // attempt), snapped to its bracketed twin; else the closest to `guess`.
        Some(roots) => {
            let target = newton_root.unwrap_or(guess);
            roots
                .into_iter()
                .min_by(|a, b| (a - target).abs().total_cmp(&(b - target).abs()))
                .ok_or(FinanceError::NoSolution { what: "xirr" })
        }
        None => newton_root.ok_or(FinanceError::NoSolution { what: "xirr" }),
    }
}

/// The single root when the flows change sign once: bracket outward from
/// `u0` (within the `±U_MAX` window, so `r` stays representable above −1)
/// until the signs at the ends match their limits, then Brent.
#[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
fn unique_root(flows: &[(f64, f64)], u0: f64) -> Result<f64> {
    let f = |u: f64| scaled(flows, u);
    // As 1+r → 0 the latest flow dominates; as 1+r → ∞ the earliest does.
    let low_sign = flows.last().map_or(0.0, |x| x.1.signum());
    let mut step = 0.25;
    let (mut lo, mut hi) = (u0 - step, u0 + step);
    while f(lo).signum() != low_sign && lo > -U_MAX {
        step *= 2.0;
        lo = (u0 - step).max(-U_MAX);
    }
    step = 0.25;
    while f(hi).signum() == low_sign && hi < U_MAX {
        step *= 2.0;
        hi = (u0 + step).min(U_MAX);
    }
    brent("xirr", f, lo, hi, 0.0)
}

fn newton_fallback(flows: &[(f64, f64)], guess: f64) -> Result<f64> {
    let f_df = |r: f64| {
        flows.iter().fold((0.0, 0.0), |(v, d), (t, a)| {
            let disc = (1.0 + r).powf(-t);
            (v + a * disc, d - t * a * disc / (1.0 + r))
        })
    };
    let scale: f64 = flows.iter().map(|f| f.1.abs()).sum();
    newton(f_df, guess, 1e-12, 100)
        .filter(|r| *r > -1.0 && f_df(*r).0.abs() <= 1e-9 * scale)
        .ok_or(FinanceError::NoSolution { what: "xirr" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guess_breaks_ties() -> Result<()> {
        let (t, a) = ([0.0, 1.0, 2.0], [-100.0, 230.0, -132.0]);
        assert!((xirr_times(&t, &a, 0.0)? - 0.1).abs() < 1e-12);
        assert!((xirr_times(&t, &a, 0.3)? - 0.2).abs() < 1e-12);
        Ok(())
    }

    #[test]
    fn extreme_rates() -> Result<()> {
        // -99% and +10,000% within the window.
        assert!((xirr_times(&[0.0, 1.0], &[-100.0, 1.0], 0.1)? + 0.99).abs() < 1e-12);
        assert!((xirr_times(&[0.0, 1.0], &[-1.0, 101.0], 0.1)? - 100.0).abs() < 1e-9);
        Ok(())
    }

    #[test]
    fn same_sign_cashflows_have_no_root() {
        assert!(matches!(
            xirr_times(&[0.0, 1.0], &[1.0, 2.0], 0.1),
            Err(FinanceError::NoSignChange)
        ));
    }

    #[test]
    fn same_day_flows_merge() -> Result<()> {
        let r = xirr_times(&[0.0, 0.0, 1.0, 1.0], &[-60.0, -40.0, 100.0, 10.0], 0.1)?;
        assert!((r - 0.1).abs() < 1e-12);
        Ok(())
    }

    #[test]
    fn tangent_root_uses_newton() -> Result<()> {
        // (1+r)^-2 · (x-1.1)^2-shaped: -100 + 220/x... with a double root at r = 0.1.
        let r = xirr_times(&[0.0, 1.0, 2.0], &[1.0, -2.2, 1.21], 0.05)?;
        assert!((r - 0.1).abs() < 1e-5, "{r}");
        Ok(())
    }
}
