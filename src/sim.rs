//! Reproducible Monte Carlo sampling and valuation summaries.
//!
//! The sampler knows distributions and correlations, not valuation logic:
//! callers provide a closure that maps one ordered input vector to metrics.

mod distribution;
mod outcomes;
mod sampling;
use outcomes::{histogram, return_summary, sensitivities, summary, validate_path};
#[cfg(test)]
use sampling::correlate;
pub use sampling::{
    sample_inputs, sample_inputs_with_rng, Distribution, Input, NormalRng, SplitMix64BoxMuller,
};

use crate::error::{FinanceError, Result};

/// An evaluated path. `value` is the primary metric (typically EV or equity value).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathValue {
    /// Primary valuation metric.
    pub value: f64,
    /// Optional pair of leveraged return metrics for LBO-style evaluations.
    pub returns: Option<PathReturns>,
}

/// Periodic IRR and MOIC produced by one leveraged path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PathReturns {
    /// Periodic internal rate of return.
    pub irr: f64,
    /// Multiple on invested capital.
    pub moic: f64,
}

/// Aggregated return metrics across leveraged paths.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReturnsSummary {
    /// Periodic internal rate of return.
    pub irr: Summary,
    /// Multiple on invested capital.
    pub moic: Summary,
}

/// Summary statistics for one metric.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Summary {
    /// Arithmetic mean of simulated outcomes.
    pub mean: f64,
    /// Sample standard deviation (n−1 denominator).
    pub stdev: f64,
    /// Fifth percentile.
    pub p5: f64,
    /// Tenth percentile.
    pub p10: f64,
    /// Median.
    pub p50: f64,
    /// Ninetieth percentile.
    pub p90: f64,
    /// Ninety-fifth percentile.
    pub p95: f64,
    /// `None` when no comparison price was supplied.
    pub probability_above_price: Option<f64>,
}

/// Histogram bucket, with a half-open range except the final bucket.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HistogramBin {
    /// Inclusive lower edge of the bucket.
    pub low: f64,
    /// Upper edge of the bucket; the last bucket includes its upper edge.
    pub high: f64,
    /// Number of outcomes in the bucket.
    pub count: usize,
}

/// One-input-at-a-time sensitivity measured by Pearson correlation.
#[derive(Clone, Debug, PartialEq)]
pub struct Sensitivity {
    /// Name of the simulated input.
    pub input: String,
    /// Signed correlation with the primary output.
    pub correlation: f64,
    /// Absolute correlation, used for ordering.
    pub importance: f64,
}

/// Simulated input and valuation outcomes, in path order.
#[derive(Clone, Debug, PartialEq)]
pub struct Simulation {
    /// Seed used to produce the input draws.
    pub seed: u64,
    /// Sampled input vectors in path order.
    pub draws: Vec<Vec<f64>>,
    /// Evaluated values and optional leveraged returns in path order.
    pub outcomes: Vec<PathValue>,
    /// Primary valuation metric for this path.
    pub value: Summary,
    /// Optional IRR and MOIC for an LBO-style path.
    pub returns: Option<ReturnsSummary>,
    /// Primary metric counts grouped into value buckets.
    pub histogram: Vec<HistogramBin>,
    /// Inputs ranked by absolute correlation with the primary output.
    pub tornado: Vec<Sensitivity>,
}

/// Run `paths` evaluations using an explicit seed.
///
/// `correlation` is an optional row-major correlation matrix over the inputs.
/// The evaluator is independent of the sampler and can call `corp::dcf`,
/// `corp::enterprise_value`, or LBO functions as needed.
///
/// # Errors
/// Invalid distributions, a non-positive-semidefinite correlation matrix,
/// zero paths/bins, or an invalid/non-finite evaluator result.
pub fn simulate<F>(
    inputs: &[Input],
    correlation: Option<&[Vec<f64>]>,
    paths: usize,
    seed: u64,
    price: Option<f64>,
    bins: usize,
    evaluate: F,
) -> Result<Simulation>
where
    F: Fn(&[f64]) -> Result<PathValue>,
{
    if bins == 0 {
        return Err(FinanceError::InvalidInput {
            what: "bins",
            reason: "must be positive",
        });
    }
    if price.is_some_and(|p| !p.is_finite()) {
        return Err(FinanceError::NonFinite { what: "price" });
    }
    let draws = sample_inputs(inputs, correlation, paths, seed)?;
    let mut outcomes = Vec::with_capacity(paths);
    for values in &draws {
        let outcome = evaluate(values)?;
        validate_path(outcome)?;
        outcomes.push(outcome);
    }
    let values: Vec<f64> = outcomes.iter().map(|x| x.value).collect();
    let value = summary(&values, price)?;
    let returns = return_summary(&outcomes)?;
    let histogram = histogram(&values, bins);
    let tornado = sensitivities(inputs, &draws, &values);
    Ok(Simulation {
        seed,
        draws,
        outcomes,
        value,
        returns,
        histogram,
        tornado,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FixedRng(usize);

    impl NormalRng for FixedRng {
        fn normal(&mut self) -> f64 {
            let value = [-1.0, 0.0, 1.0][self.0 % 3];
            self.0 += 1;
            value
        }
    }

    fn input(name: &str, distribution: Distribution) -> Input {
        Input {
            name: name.into(),
            distribution,
        }
    }

    #[test]
    fn seeded_and_caller_supplied_sampling_are_reproducible() {
        let mut rng = SplitMix64BoxMuller::new(7);
        let a = rng.uniform_open01();
        assert!((0.0..1.0).contains(&a));
        assert!(rng.normal().is_finite());
        let inputs = [input(
            "x",
            Distribution::Normal {
                mean: 10.0,
                sd: 2.0,
            },
        )];
        let first = sample_inputs(&inputs, None, 5, 42).unwrap();
        assert_eq!(first, sample_inputs(&inputs, None, 5, 42).unwrap());
        let mut fixed = FixedRng::default();
        assert_eq!(
            sample_inputs_with_rng(&inputs, None, 3, &mut fixed).unwrap(),
            vec![vec![8.0], vec![10.0], vec![12.0]]
        );
        assert_eq!(FixedRng::default().normal(), -1.0);
    }

    #[test]
    fn sampling_validates_names_distributions_and_correlations() {
        assert!(sample_inputs(&[], None, 1, 0).is_err());
        assert!(sample_inputs(
            &[input(" ", Distribution::Normal { mean: 0.0, sd: 1.0 })],
            None,
            1,
            0
        )
        .is_err());
        assert!(sample_inputs(
            &[
                input("x", Distribution::Normal { mean: 0.0, sd: 1.0 }),
                input("x", Distribution::Normal { mean: 0.0, sd: 1.0 })
            ],
            None,
            1,
            0
        )
        .is_err());
        for invalid in [
            Distribution::Normal {
                mean: 0.0,
                sd: -1.0,
            },
            Distribution::LogNormal {
                mu: 0.0,
                sigma: -1.0,
            },
            Distribution::Triangular {
                low: 2.0,
                mode: 1.0,
                high: 3.0,
            },
            Distribution::Pert {
                low: 0.0,
                mode: 3.0,
                high: 2.0,
            },
            Distribution::Uniform {
                low: 2.0,
                high: 1.0,
            },
            Distribution::Discrete {
                scenarios: vec![],
                weights: vec![],
            },
            Distribution::Discrete {
                scenarios: vec![1.0],
                weights: vec![-1.0],
            },
        ] {
            assert!(sample_inputs(&[input("x", invalid)], None, 1, 0).is_err());
        }
        assert!(sample_inputs(
            &[input("x", Distribution::Normal { mean: 0.0, sd: 1.0 })],
            None,
            0,
            0
        )
        .is_err());
        let two = [
            input("x", Distribution::Normal { mean: 0.0, sd: 1.0 }),
            input("y", Distribution::Normal { mean: 0.0, sd: 1.0 }),
        ];
        assert!(sample_inputs(&two, Some(&[vec![1.0]]), 1, 0).is_err());
        assert!(sample_inputs(&two, Some(&[vec![1.0, 0.2], vec![0.1, 1.0]]), 1, 0).is_err());
        assert!(sample_inputs(&two, Some(&[vec![1.0, 2.0], vec![2.0, 1.0]]), 1, 0).is_err());
        assert!(sample_inputs(&two, Some(&[vec![1.0, 0.9], vec![0.9, 1.0]]), 1, 0).is_ok());
        assert!(sample_inputs(&two, Some(&[vec![1.0, 0.0], vec![0.0, -1.0]]), 1, 0).is_err());
        assert!(sample_inputs(&two, Some(&[vec![1.0, 1.0], vec![1.0, 1.0]]), 1, 0).is_ok());
        let three = [
            input("x", Distribution::Normal { mean: 0.0, sd: 1.0 }),
            input("y", Distribution::Normal { mean: 0.0, sd: 1.0 }),
            input("z", Distribution::Normal { mean: 0.0, sd: 1.0 }),
        ];
        let singular_but_invalid = [
            vec![1.0, 0.0, 0.5],
            vec![0.0, 0.0, 0.5],
            vec![0.5, 0.5, 1.0],
        ];
        assert!(sample_inputs(&three, Some(&singular_but_invalid), 1, 0).is_err());
        let singular_psd = [
            vec![1.0, 0.0, 0.0],
            vec![0.0, 0.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        assert!(sample_inputs(&three, Some(&singular_psd), 1, 0).is_ok());
        assert_eq!(correlate(&[vec![1.0, 2.0]], &[3.0, 4.0]), vec![11.0]);
    }

    #[test]
    fn simulate_returns_statistics_and_rejects_invalid_evaluations() {
        let inputs = [input(
            "x",
            Distribution::Discrete {
                scenarios: vec![1.0, 2.0, 3.0],
                weights: vec![1.0, 1.0, 1.0],
            },
        )];
        let evaluate = |row: &[f64]| {
            Ok(PathValue {
                value: row[0],
                returns: Some(PathReturns {
                    irr: row[0] / 10.0,
                    moic: row[0],
                }),
            })
        };
        let result = simulate(&inputs, None, 9, 19, Some(1.0), 3, evaluate).unwrap();
        assert_eq!(result.draws.len(), 9);
        assert!(result.returns.is_some());
        assert_eq!(result.histogram.iter().map(|b| b.count).sum::<usize>(), 9);
        assert_eq!(result.tornado.len(), 1);
        assert!(simulate(&inputs, None, 1, 0, None, 0, evaluate).is_err());
        assert!(simulate(&inputs, None, 1, 0, Some(f64::NAN), 1, evaluate).is_err());
        assert!(simulate(&inputs, None, 0, 0, None, 1, evaluate).is_err());
        assert!(simulate(&inputs, None, 1, 0, None, 1, |_| Err(
            FinanceError::TooShort {
                what: "eval",
                min: 1,
                got: 0
            }
        ))
        .is_err());
        assert!(simulate(&inputs, None, 1, 0, None, 1, |_| Ok(PathValue {
            value: f64::NAN,
            returns: None
        }))
        .is_err());
        let alternating = |row: &[f64]| {
            Ok(PathValue {
                value: row[0],
                returns: if row[0] > 2.0 {
                    None
                } else {
                    Some(PathReturns {
                        irr: 0.1,
                        moic: 2.0,
                    })
                },
            })
        };
        assert!(simulate(&inputs, None, 20, 4, None, 2, alternating).is_err());
    }
}
