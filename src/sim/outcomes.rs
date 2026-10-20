//! Aggregation helpers for simulated outcomes, return metrics, and sensitivities.
use super::{
    FinanceError, HistogramBin, Input, PathReturns, PathValue, Result, ReturnsSummary, Sensitivity,
    Summary,
};

fn domain(reason: &'static str) -> FinanceError {
    FinanceError::InvalidInput {
        what: "simulation input",
        reason,
    }
}

pub(super) fn validate_path(path: PathValue) -> Result<()> {
    if !path.value.is_finite()
        || path
            .returns
            .is_some_and(|r| !r.irr.is_finite() || !r.moic.is_finite())
    {
        return Err(FinanceError::NonFinite {
            what: "simulation outcome",
        });
    }
    Ok(())
}

#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
pub(super) fn summary(values: &[f64], price: Option<f64>) -> Result<Summary> {
    if values.is_empty() {
        return Err(FinanceError::TooShort {
            what: "outcomes",
            min: 1,
            got: 0,
        });
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = if values.len() > 1 {
        values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64
    } else {
        0.0
    };
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let quantile = |p: f64| {
        let pos = p * (sorted.len() - 1) as f64;
        let lo = pos.floor() as usize;
        let hi = pos.ceil() as usize;
        sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
    };
    Ok(Summary {
        mean,
        stdev: variance.sqrt(),
        p5: quantile(0.05),
        p10: quantile(0.10),
        p50: quantile(0.50),
        p90: quantile(0.90),
        p95: quantile(0.95),
        probability_above_price: price
            .map(|p| values.iter().filter(|x| **x > p).count() as f64 / values.len() as f64),
    })
}

pub(super) fn return_summary(paths: &[PathValue]) -> Result<Option<ReturnsSummary>> {
    if paths.iter().all(|p| p.returns.is_none()) {
        return Ok(None);
    }
    if paths.iter().any(|p| p.returns.is_none()) {
        return Err(domain(
            "LBO return metrics must be present on every path or none",
        ));
    }
    let returns: Vec<PathReturns> = paths.iter().filter_map(|p| p.returns).collect();
    Ok(Some(ReturnsSummary {
        irr: summary(&returns.iter().map(|r| r.irr).collect::<Vec<_>>(), None)?,
        moic: summary(&returns.iter().map(|r| r.moic).collect::<Vec<_>>(), None)?,
    }))
}

#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
#[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
pub(super) fn histogram(values: &[f64], bins: usize) -> Vec<HistogramBin> {
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if min == max {
        return (0..bins)
            .map(|i| HistogramBin {
                low: min,
                high: max,
                count: if i == 0 { values.len() } else { 0 },
            })
            .collect();
    }
    let width = (max - min) / bins as f64;
    let mut out: Vec<_> = (0..bins)
        .map(|i| HistogramBin {
            low: min + i as f64 * width,
            high: min + (i + 1) as f64 * width,
            count: 0,
        })
        .collect();
    for value in values {
        let i = (((*value - min) / width).floor() as usize).min(bins - 1);
        out[i].count += 1;
    }
    out
}

#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub(super) fn sensitivities(
    inputs: &[Input],
    draws: &[Vec<f64>],
    outputs: &[f64],
) -> Vec<Sensitivity> {
    let mean_y = outputs.iter().sum::<f64>() / outputs.len() as f64;
    let sy = outputs.iter().map(|y| (y - mean_y).powi(2)).sum::<f64>();
    let mut rows: Vec<_> = inputs
        .iter()
        .enumerate()
        .map(|(j, input)| {
            let xs: Vec<f64> = draws.iter().map(|row| row[j]).collect();
            let mean_x = xs.iter().sum::<f64>() / xs.len() as f64;
            let sx = xs.iter().map(|x| (x - mean_x).powi(2)).sum::<f64>();
            let cov = xs
                .iter()
                .zip(outputs)
                .map(|(x, y)| (x - mean_x) * (y - mean_y))
                .sum::<f64>();
            let correlation = if sx == 0.0 || sy == 0.0 {
                0.0
            } else {
                cov / (sx * sy).sqrt()
            };
            Sensitivity {
                input: input.name.clone(),
                correlation,
                importance: correlation.abs(),
            }
        })
        .collect();
    rows.sort_by(|a, b| b.importance.total_cmp(&a.importance));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::Distribution;

    #[test]
    fn summaries_histograms_returns_and_sensitivities_cover_edges() {
        assert!(summary(&[], None).is_err());
        let one = summary(&[2.0], Some(1.0)).unwrap();
        assert_eq!(one.stdev, 0.0);
        assert_eq!(one.probability_above_price, Some(1.0));
        let many = summary(&[1.0, 2.0, 3.0], None).unwrap();
        assert_eq!(many.mean, 2.0);
        assert_eq!(many.stdev, 1.0);
        assert_eq!(many.p50, 2.0);
        assert_eq!(
            return_summary(&[PathValue {
                value: 1.0,
                returns: None
            }])
            .unwrap(),
            None
        );
        assert!(return_summary(&[
            PathValue {
                value: 1.0,
                returns: None
            },
            PathValue {
                value: 2.0,
                returns: Some(PathReturns {
                    irr: 0.1,
                    moic: 2.0
                })
            }
        ])
        .is_err());
        let returns = return_summary(&[
            PathValue {
                value: 1.0,
                returns: Some(PathReturns {
                    irr: 0.1,
                    moic: 2.0,
                }),
            },
            PathValue {
                value: 2.0,
                returns: Some(PathReturns {
                    irr: 0.2,
                    moic: 3.0,
                }),
            },
        ])
        .unwrap()
        .unwrap();
        assert!((returns.irr.mean - 0.15).abs() < 1e-12);
        assert_eq!(
            histogram(&[5.0, 5.0], 3)
                .iter()
                .map(|bin| bin.count)
                .collect::<Vec<_>>(),
            vec![2, 0, 0]
        );
        assert_eq!(
            histogram(&[1.0, 2.0, 3.0], 2)
                .iter()
                .map(|bin| bin.count)
                .sum::<usize>(),
            3
        );
        assert_eq!(
            sensitivities(
                &[Input {
                    name: "constant".into(),
                    distribution: Distribution::Normal { mean: 0.0, sd: 0.0 }
                }],
                &[vec![1.0], vec![1.0]],
                &[1.0, 2.0]
            )[0]
            .correlation,
            0.0
        );
        assert!(validate_path(PathValue {
            value: f64::INFINITY,
            returns: None
        })
        .is_err());
        assert!(validate_path(PathValue {
            value: 1.0,
            returns: Some(PathReturns {
                irr: f64::NAN,
                moic: 1.0
            })
        })
        .is_err());
        assert!(validate_path(PathValue {
            value: 1.0,
            returns: Some(PathReturns {
                irr: 0.1,
                moic: f64::INFINITY
            })
        })
        .is_err());
    }
}
