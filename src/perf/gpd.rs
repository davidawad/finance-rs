//! Peaks-over-threshold generalized Pareto tail estimates.

use super::check_returns;
use crate::error::{output, FinanceError, Result};
use crate::solve::{brent, SolverOptions};

/// Generalized Pareto fit to return losses below a supplied return threshold.
///
/// `var` is the supplied cutoff. `expected_shortfall` is the fitted mean
/// return in the threshold tail, using the GPD mean excess `scale/(1-shape)`.
/// Returns and risk estimates use the usual signed-return convention (losses
/// are negative).
#[derive(Debug, Clone, Copy, PartialEq)]
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
pub struct GpdTailRisk {
    /// Return cutoff used to select tail observations.
    pub threshold: f64,
    /// The cutoff VaR, in return units.
    pub var: f64,
    /// Fitted GPD scale for loss excesses below the cutoff.
    pub scale: f64,
    /// Fitted GPD shape (ξ).
    pub shape: f64,
    /// Fitted conditional mean return below the cutoff.
    pub expected_shortfall: f64,
}

/// Fit a generalized Pareto distribution by maximum likelihood to losses
/// exceeding `threshold`, and estimate threshold-tail VaR and expected
/// shortfall. The input `threshold` is a return (for example `-0.02`); every
/// return strictly below it contributes the positive excess
/// `threshold - return`.
///
/// The threshold is also the reported VaR. Expected shortfall extrapolates
/// from that cutoff using the fitted GPD mean excess. Shape values at or above
/// one have infinite mean excess and return [`FinanceError::Undefined`]. At
/// least three excess observations are required for the two-parameter MLE.
///
/// # Errors
/// Returns a typed error for non-finite inputs, fewer than three exceedances,
/// a degenerate tail, an unidentifiable fit, or infinite expected shortfall.
/// ```rust
/// assert!((finance_rs::perf::compounded_return(&[0.10, -0.05]).unwrap() - 0.045).abs() < 1e-12);
/// ```
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
#[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
pub fn gpd_tail_risk(returns: &[f64], threshold: f64) -> Result<GpdTailRisk> {
    crate::error::finite("threshold", threshold)?;
    check_returns("returns", returns, 1)?;
    let excesses: Vec<f64> = returns
        .iter()
        .filter(|r| **r < threshold)
        .map(|r| threshold - r)
        .collect();
    crate::error::min_len("threshold exceedances", &excesses, 3)?;
    let mean = excesses.iter().sum::<f64>() / excesses.len() as f64;
    if !mean.is_finite() || mean <= 0.0 {
        return Err(FinanceError::Undefined {
            what: "GPD tail scale",
        });
    }
    if excesses.iter().all(|x| *x == excesses[0]) {
        return Err(FinanceError::Undefined {
            what: "GPD tail fit",
        });
    }

    // Profile out scale by its likelihood score, then maximize the resulting
    // one-dimensional likelihood in shape. The constrained lower endpoint
    // keeps the usual regular MLE away from the non-regular ξ <= -1/2 region.
    let (shape, scale) = fit_gpd(&excesses, mean)?;
    if shape >= 1.0 {
        return Err(FinanceError::Undefined {
            what: "GPD expected shortfall",
        });
    }
    let expected_shortfall = threshold - scale / (1.0 - shape);
    Ok(GpdTailRisk {
        threshold,
        var: threshold,
        scale: output("gpd_tail_risk scale", scale)?,
        shape: output("gpd_tail_risk shape", shape)?,
        expected_shortfall: output("gpd_tail_risk expected shortfall", expected_shortfall)?,
    })
}

fn fit_gpd(excesses: &[f64], mean: f64) -> Result<(f64, f64)> {
    const LOW_SHAPE: f64 = -0.49;
    const HIGH_SHAPE: f64 = 3.0;
    const PHI: f64 = 0.618_033_988_749_894_9;
    let mut lo = LOW_SHAPE;
    let mut hi = HIGH_SHAPE;
    let mut x1 = hi - PHI * (hi - lo);
    let mut x2 = lo + PHI * (hi - lo);
    let mut f1 = profile_log_likelihood(excesses, mean, x1);
    let mut f2 = profile_log_likelihood(excesses, mean, x2);
    for _ in 0..180 {
        if f1 < f2 {
            lo = x1;
            x1 = x2;
            f1 = f2;
            x2 = lo + PHI * (hi - lo);
            f2 = profile_log_likelihood(excesses, mean, x2);
        } else {
            hi = x2;
            x2 = x1;
            f2 = f1;
            x1 = hi - PHI * (hi - lo);
            f1 = profile_log_likelihood(excesses, mean, x1);
        }
    }
    let shape = (lo + hi) * 0.5;
    let scale = profile_scale(excesses, mean, shape)?;
    let likelihood = log_likelihood(excesses, shape, scale);
    ensure_finite_likelihood(likelihood)?;
    Ok((shape, scale))
}

fn ensure_finite_likelihood(likelihood: f64) -> Result<()> {
    if !likelihood.is_finite() {
        return Err(FinanceError::Undefined { what: "GPD MLE" });
    }
    Ok(())
}

fn profile_log_likelihood(excesses: &[f64], mean: f64, shape: f64) -> f64 {
    match profile_scale(excesses, mean, shape) {
        Ok(scale) => log_likelihood(excesses, shape, scale),
        Err(_) => f64::NEG_INFINITY,
    }
}

#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
fn profile_scale(excesses: &[f64], mean: f64, shape: f64) -> Result<f64> {
    if shape.abs() < 1e-8 {
        return Ok(mean);
    }
    let max = excesses.iter().copied().fold(0.0_f64, f64::max);
    let mut low = if shape < 0.0 {
        -shape * max * (1.0 + 1e-10)
    } else {
        mean * 1e-12
    };
    if low == 0.0 {
        low = f64::MIN_POSITIVE;
    }
    let mut high = mean.max(max).max(1.0);
    let score = |scale: f64| {
        let sum = excesses
            .iter()
            .map(|x| x / (1.0 + shape * x / scale))
            .sum::<f64>();
        scale - (1.0 + shape) * sum / excesses.len() as f64
    };
    let mut f_high = score(high);
    for _ in 0..128 {
        if f_high.is_finite() && f_high > 0.0 {
            break;
        }
        high *= 2.0;
        f_high = score(high);
    }
    if !f_high.is_finite() || f_high <= 0.0 {
        return Err(FinanceError::NoSolution {
            what: "GPD scale MLE",
        });
    }
    let f_low = score(low);
    if !f_low.is_finite() || f_low >= 0.0 {
        return Err(FinanceError::NoSolution {
            what: "GPD scale MLE",
        });
    }
    Ok(brent(score, low, high, &SolverOptions::default())?.root)
}

#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
fn log_likelihood(excesses: &[f64], shape: f64, scale: f64) -> f64 {
    if !scale.is_finite() || scale <= 0.0 {
        return f64::NEG_INFINITY;
    }
    let mut sum_log = 0.0;
    for x in excesses {
        let support = 1.0 + shape * x / scale;
        if support <= 0.0 || !support.is_finite() {
            return f64::NEG_INFINITY;
        }
        sum_log += if shape.abs() < 1e-8 {
            x / scale
        } else {
            support.ln()
        };
    }
    if shape.abs() < 1e-8 {
        -(excesses.len() as f64) * scale.ln() - sum_log
    } else {
        -(excesses.len() as f64) * scale.ln() - (1.0 + 1.0 / shape) * sum_log
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
    fn gpd_inputs_and_profile_helpers_cover_degenerate_paths() {
        assert!(gpd_tail_risk(&[-0.1, -0.2, -0.3], f64::NAN).is_err());
        assert!(gpd_tail_risk(&[], -0.1).is_err());
        assert!(gpd_tail_risk(&[-0.01, -0.02], 0.0).is_err());
        assert!(gpd_tail_risk(&[-0.1, -0.1, -0.1, -0.1], 0.0).is_err());
        assert!(gpd_tail_risk(&[-f64::MAX, -f64::MAX, -f64::MAX], f64::MAX).is_err());
        let heavy_tail = [-1.0, -2.0, -4.0, -8.0, -16.0, -32.0, -64.0, -128.0, -256.0];
        let _ = gpd_tail_risk(&heavy_tail, 0.0);
        let x = [1.0, 2.0, 3.0, 4.0];
        assert_eq!(profile_scale(&x, 2.5, 0.0).unwrap(), 2.5);
        assert!(profile_scale(&x, 2.5, 0.2).unwrap().is_finite());
        assert!(profile_scale(&x, 2.5, -0.2).unwrap().is_finite());
        assert!(profile_log_likelihood(&x, 2.5, 0.1).is_finite());
        assert_eq!(
            profile_log_likelihood(&[0.0, 0.0], 0.0, -0.2),
            f64::NEG_INFINITY
        );
        assert!(matches!(
            profile_scale(&[1.0, 2.0, f64::NAN], 1.0, 0.5),
            Err(FinanceError::NoSolution {
                what: "GPD scale MLE"
            })
        ));
        assert!(matches!(
            ensure_finite_likelihood(f64::NAN),
            Err(FinanceError::Undefined { what: "GPD MLE" })
        ));
        assert!(profile_scale(&x, 0.0, 0.5).is_ok());
        assert!(log_likelihood(&x, 0.0, 2.5).is_finite());
        assert!(log_likelihood(&x, 0.2, 2.5).is_finite());
        assert_eq!(log_likelihood(&x, 0.0, 0.0), f64::NEG_INFINITY);
        assert_eq!(log_likelihood(&x, -1.0, 1.0), f64::NEG_INFINITY);
        assert_eq!(
            log_likelihood(&x, 0.1, f64::from_bits(1)),
            f64::NEG_INFINITY
        );
        assert!(fit_gpd(&x, 2.5).unwrap().1 > 0.0);
    }
}
