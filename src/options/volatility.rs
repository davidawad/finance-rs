//! Realized-volatility estimators from closing prices and OHLC bars.
use super::{nonnegative, validate};
use crate::{FinanceError, Result};

fn bars(what: &'static str, arrays: &[(&'static str, &[f64])], min: usize) -> Result<()> {
    let n = arrays[0].1.len();
    if n < min {
        return Err(FinanceError::TooShort { what, min, got: n });
    }
    for (name, a) in arrays {
        crate::error::same_len("series", n, name, a.len())?;
        crate::error::all_finite(name, a)?;
    }
    Ok(())
}
fn annualize_variance(v: f64, ppy: f64) -> Result<f64> {
    nonnegative("periods_per_year", ppy)?;
    if ppy == 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "periods_per_year",
            reason: "must be positive",
        });
    }
    crate::error::output("annualized volatility", (v * ppy).max(0.0).sqrt())
}

/// Annualized sample standard deviation of log price returns.
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn close_to_close(prices: &[f64], periods_per_year: f64) -> Result<f64> {
    bars("prices", &[("prices", prices)], 2)?;
    nonnegative("periods_per_year", periods_per_year)?;
    if periods_per_year == 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "periods_per_year",
            reason: "must be positive",
        });
    }
    let mut logs = Vec::with_capacity(prices.len() - 1);
    for w in prices.windows(2) {
        if w[0] <= 0.0 || w[1] <= 0.0 {
            return Err(FinanceError::InvalidInput {
                what: "prices",
                reason: "must be positive",
            });
        }
        logs.push((w[1] / w[0]).ln());
    }
    let mean = logs.iter().sum::<f64>() / (logs.len() as f64);
    let variance = logs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / ((logs.len() - 1) as f64);
    annualize_variance(variance, periods_per_year)
}

/// Parkinson range estimator, annualized over observed bars.
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn parkinson(high: &[f64], low: &[f64], periods_per_year: f64) -> Result<f64> {
    bars("high/low", &[("low", low), ("high", high)], 1)?;
    let mut sum = 0.0;
    for (&h, &l) in high.iter().zip(low) {
        if l <= 0.0 || h < l {
            return Err(FinanceError::InvalidInput {
                what: "high/low",
                reason: "require 0 < low <= high",
            });
        }
        sum += (h / l).ln().powi(2);
    }
    annualize_variance(
        sum / (4.0 * (high.len() as f64) * 2.0_f64.ln()),
        periods_per_year,
    )
}

/// Garman-Klass OHLC estimator, annualized. Bars need not include gaps.
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn garman_klass(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    periods_per_year: f64,
) -> Result<f64> {
    bars(
        "OHLC",
        &[
            ("open", open),
            ("high", high),
            ("low", low),
            ("close", close),
        ],
        1,
    )?;
    let mut sum = 0.0;
    for i in 0..open.len() {
        let (o, h, l, c) = (open[i], high[i], low[i], close[i]);
        if [o, h, l, c].iter().any(|x| *x <= 0.0) || h < o.max(c) || l > o.min(c) || h < l {
            return Err(FinanceError::InvalidInput {
                what: "OHLC",
                reason: "require positive prices and low <= open/close <= high",
            });
        }
        sum += 0.5 * (h / l).ln().powi(2) - (2.0 * 2.0_f64.ln() - 1.0) * (c / o).ln().powi(2);
    }
    annualize_variance(sum / (open.len() as f64), periods_per_year)
}

/// Rogers-Satchell OHLC estimator, robust to nonzero drift.
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn rogers_satchell(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    periods_per_year: f64,
) -> Result<f64> {
    bars(
        "OHLC",
        &[
            ("open", open),
            ("high", high),
            ("low", low),
            ("close", close),
        ],
        1,
    )?;
    let mut sum = 0.0;
    for i in 0..open.len() {
        let (o, h, l, c) = (open[i], high[i], low[i], close[i]);
        if [o, h, l, c].iter().any(|x| *x <= 0.0) || h < o.max(c) || l > o.min(c) || h < l {
            return Err(FinanceError::InvalidInput {
                what: "OHLC",
                reason: "require positive prices and low <= open/close <= high",
            });
        }
        sum += (h / c).ln() * (h / o).ln() + (l / c).ln() * (l / o).ln();
    }
    annualize_variance(sum / (open.len() as f64), periods_per_year)
}

/// Yang-Zhang OHLC estimator including overnight gaps.
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
#[allow(clippy::many_single_char_names)] // Conventional symbols follow the published formula or recurrence notation.
pub fn yang_zhang(
    open: &[f64],
    high: &[f64],
    low: &[f64],
    close: &[f64],
    periods_per_year: f64,
) -> Result<f64> {
    bars(
        "OHLC",
        &[
            ("open", open),
            ("high", high),
            ("low", low),
            ("close", close),
        ],
        2,
    )?;
    let n = open.len() as f64;
    let mut overnight = Vec::with_capacity(open.len() - 1);
    let mut close_to_open = Vec::with_capacity(open.len() - 1);
    let mut rs = 0.0;
    for i in 0..open.len() {
        let (o, h, l, c) = (open[i], high[i], low[i], close[i]);
        if [o, h, l, c].iter().any(|x| *x <= 0.0) || h < o.max(c) || l > o.min(c) || h < l {
            return Err(FinanceError::InvalidInput {
                what: "OHLC",
                reason: "require positive prices and low <= open/close <= high",
            });
        }
        rs += (h / c).ln() * (h / o).ln() + (l / c).ln() * (l / o).ln();
        if i > 0 {
            overnight.push((o / close[i - 1]).ln());
            close_to_open.push((c / o).ln());
        }
    }
    let sample_var = |v: &[f64]| {
        let m = v.iter().sum::<f64>() / (v.len() as f64);
        v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / ((v.len() - 1) as f64)
    };
    let k = 0.34 / (1.34 + (n + 1.0) / (n - 1.0));
    let variance = sample_var(&overnight) + k * sample_var(&close_to_open) + (1.0 - k) * rs / n;
    annualize_variance(variance, periods_per_year)
}

/// `RiskMetrics` EWMA volatility from returns, annualized. `lambda` is the decay factor.
/// ```rust
/// assert!(finance_rs::options::bsm(finance_rs::options::OptionKind::Call, 100.0, 100.0, 1.0, 0.0, 0.0, 0.2).unwrap() > 0.0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn ewma(returns: &[f64], lambda: f64, periods_per_year: f64) -> Result<f64> {
    bars("returns", &[("returns", returns)], 2)?;
    validate(&[("lambda", lambda)])?;
    if !(0.0..1.0).contains(&lambda) {
        return Err(FinanceError::InvalidInput {
            what: "lambda",
            reason: "must be in [0, 1)",
        });
    }
    let mut variance = returns[0] * returns[0];
    for r in &returns[1..] {
        variance = lambda * variance + (1.0 - lambda) * r * r;
    }
    annualize_variance(variance, periods_per_year)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn estimators_are_scale_invariant_and_finite() -> Result<()> {
        let (o, h, l, c) = (
            vec![100., 101., 100.5],
            vec![103., 104., 103.],
            vec![98., 99., 98.],
            vec![101., 102., 100.],
        );
        for f in [
            garman_klass(&o, &h, &l, &c, 252.)?,
            rogers_satchell(&o, &h, &l, &c, 252.)?,
            yang_zhang(&o, &h, &l, &c, 252.)?,
        ] {
            assert!(f.is_finite() && f >= 0.0);
        }
        let scaled_high: Vec<_> = h.iter().map(|x| x * 2.).collect();
        let scaled_low: Vec<_> = l.iter().map(|x| x * 2.).collect();
        let original_vol = parkinson(&h, &l, 252.)?;
        let scaled_vol = parkinson(&scaled_high, &scaled_low, 252.)?;
        assert!((original_vol - scaled_vol).abs() < 1e-14);
        assert!(close_to_close(&[100., 101., 102.], 252.)? > 0.0);
        assert!(ewma(&[0.01, -0.02, 0.015], 0.94, 252.)? > 0.0);
        Ok(())
    }
    #[test]
    fn malformed_bars_fail_without_panicking() {
        assert!(parkinson(&[1.0], &[0.0], 252.0).is_err());
        assert!(garman_klass(&[], &[], &[], &[], 252.0).is_err());
        assert!(ewma(&[1.0, 2.0], 1.0, 252.0).is_err());
    }
}
