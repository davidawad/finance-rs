//! Validated marginal and correlated sampling primitives.

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::StandardNormal;

use super::distribution::from_normal;
use crate::error::{all_finite, FinanceError, Result};

/// Source of standard-normal variates for a simulation. Implement this trait
/// to replay a caller's established random stream through the common sampler.
pub trait NormalRng {
    /// Return one standard-normal variate.
    fn normal(&mut self) -> f64;
}

/// `SplitMix64` with the Box-Muller transform used by dc-site's joint draws.
/// Its bit stream and transform order intentionally match `joint_kit`'s
/// historical implementation so adopting the shared sampler preserves its
/// published draws exactly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplitMix64BoxMuller {
    state: u64,
}

impl SplitMix64BoxMuller {
    /// Create a stream at `seed`.
    #[must_use]
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform variate in the open interval `(0, 1)`.
    #[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
    pub fn uniform_open01(&mut self) -> f64 {
        ((self.next() >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }
}

impl NormalRng for SplitMix64BoxMuller {
    fn normal(&mut self) -> f64 {
        let (a, b) = (self.uniform_open01(), self.uniform_open01());
        (-2.0 * a.ln()).sqrt() * (std::f64::consts::TAU * b).cos()
    }
}

/// A marginal distribution for one model input.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum Distribution {
    /// Normal distribution with arithmetic mean and standard deviation.
    Normal {
        /// Arithmetic mean of simulated outcomes.
        mean: f64,
        /// Sd.
        sd: f64,
    },
    /// Lognormal distribution parameterized by the mean and standard deviation of ln(X).
    LogNormal {
        /// Mu.
        mu: f64,
        /// Sigma.
        sigma: f64,
    },
    /// Bounded triangular distribution.
    Triangular {
        /// Inclusive lower edge of the bucket.
        low: f64,
        /// Mode.
        mode: f64,
        /// Upper edge of the bucket; the last bucket includes its upper edge.
        high: f64,
    },
    /// Modified PERT distribution with lambda 4.
    Pert {
        /// Inclusive lower edge of the bucket.
        low: f64,
        /// Mode.
        mode: f64,
        /// Upper edge of the bucket; the last bucket includes its upper edge.
        high: f64,
    },
    /// Continuous uniform distribution.
    Uniform {
        /// Inclusive lower edge of the bucket.
        low: f64,
        /// Upper edge of the bucket; the last bucket includes its upper edge.
        high: f64,
    },
    /// Discrete outcomes paired with nonnegative probability weights.
    Discrete {
        /// Scenarios.
        scenarios: Vec<f64>,
        /// Weights.
        weights: Vec<f64>,
    },
}

/// Named valuation input and its distribution.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Input {
    /// Stable input name, matching the evaluator's vector position.
    pub name: String,
    /// Marginal distribution.
    pub distribution: Distribution,
}

/// Draw input rows without evaluating them. The same validated sampler backs
/// workbook blocks that publish the draws for downstream formulas.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn sample_inputs(
    inputs: &[Input],
    correlation: Option<&[Vec<f64>]>,
    paths: usize,
    seed: u64,
) -> Result<Vec<Vec<f64>>> {
    let mut rng = ChaCha8NormalRng(ChaCha8Rng::seed_from_u64(seed));
    sample_inputs_with_rng(inputs, correlation, paths, &mut rng)
}

/// Draw input rows with a caller-provided standard-normal stream.
///
/// This keeps distributions and Cholesky correlation centralized while
/// allowing established consumers to preserve a legacy seeded sequence.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn sample_inputs_with_rng<R: NormalRng>(
    inputs: &[Input],
    correlation: Option<&[Vec<f64>]>,
    paths: usize,
    rng: &mut R,
) -> Result<Vec<Vec<f64>>> {
    validate_inputs(inputs)?;
    if paths == 0 {
        return Err(FinanceError::InvalidInput {
            what: "paths",
            reason: "must be positive",
        });
    }
    let factor = correlation.map(|m| cholesky(m, inputs.len())).transpose()?;
    let mut draws = Vec::with_capacity(paths);
    for _ in 0..paths {
        let latent: Vec<f64> = (0..inputs.len()).map(|_| rng.normal()).collect();
        let correlated = factor
            .as_ref()
            .map_or(latent.clone(), |l| correlate(l, &latent));
        let values: Vec<f64> = inputs
            .iter()
            .zip(correlated)
            .map(|(input, z)| from_normal(&input.distribution, z))
            .collect();
        all_finite("sampled inputs", &values)?;
        draws.push(values);
    }
    Ok(draws)
}

pub(super) struct ChaCha8NormalRng(ChaCha8Rng);

impl NormalRng for ChaCha8NormalRng {
    fn normal(&mut self) -> f64 {
        self.0.sample(StandardNormal)
    }
}

fn validate_inputs(inputs: &[Input]) -> Result<()> {
    if inputs.is_empty() {
        return Err(FinanceError::TooShort {
            what: "inputs",
            min: 1,
            got: 0,
        });
    }
    for (i, input) in inputs.iter().enumerate() {
        if input.name.trim().is_empty() || inputs[..i].iter().any(|x| x.name == input.name) {
            return Err(FinanceError::InvalidInput {
                what: "input names",
                reason: "must be nonempty and unique",
            });
        }
        match &input.distribution {
            Distribution::Normal { mean, sd } => {
                all_finite("normal parameters", &[*mean, *sd])?;
                if *sd < 0.0 {
                    return Err(domain("normal sd must be nonnegative"));
                }
            }
            Distribution::LogNormal { mu, sigma } => {
                all_finite("lognormal parameters", &[*mu, *sigma])?;
                if *sigma < 0.0 {
                    return Err(domain("lognormal sigma must be nonnegative"));
                }
            }
            Distribution::Triangular { low, mode, high }
            | Distribution::Pert { low, mode, high } => {
                all_finite("bounded parameters", &[*low, *mode, *high])?;
                if low > mode || mode > high {
                    return Err(domain("require low <= mode <= high"));
                }
            }
            Distribution::Uniform { low, high } => {
                all_finite("uniform parameters", &[*low, *high])?;
                if low > high {
                    return Err(domain("require low <= high"));
                }
            }
            Distribution::Discrete { scenarios, weights } => {
                if scenarios.is_empty() || scenarios.len() != weights.len() {
                    return Err(domain(
                        "discrete scenarios and weights must have equal nonzero length",
                    ));
                }
                all_finite("discrete scenarios", scenarios)?;
                all_finite("discrete weights", weights)?;
                if weights.iter().any(|w| *w < 0.0) || weights.iter().sum::<f64>() <= 0.0 {
                    return Err(domain(
                        "discrete weights must be nonnegative with a positive sum",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn domain(reason: &'static str) -> FinanceError {
    FinanceError::InvalidInput {
        what: "simulation input",
        reason,
    }
}

#[allow(clippy::needless_range_loop)] // Indexed iteration keeps row and column coordinates aligned in this matrix algorithm.
pub(super) fn cholesky(matrix: &[Vec<f64>], n: usize) -> Result<Vec<Vec<f64>>> {
    if matrix.len() != n || matrix.iter().any(|r| r.len() != n) {
        return Err(domain("correlation matrix must be square and match inputs"));
    }
    let mut lower = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            let x = matrix[i][j];
            if !x.is_finite() || !(-1.0..=1.0).contains(&x) {
                return Err(domain("correlations must be finite and in [-1, 1]"));
            }
            if (x - matrix[j][i]).abs() > 1e-10 {
                return Err(domain("correlation matrix must be symmetric"));
            }
        }
    }
    for i in 0..n {
        for j in 0..=i {
            let residual = matrix[i][j] - (0..j).map(|k| lower[i][k] * lower[j][k]).sum::<f64>();
            if i == j {
                if residual < -1e-10 {
                    return Err(domain("correlation matrix must be positive semidefinite"));
                }
                lower[i][j] = residual.max(0.0).sqrt();
            } else if lower[j][j] > 1e-12 {
                lower[i][j] = residual / lower[j][j];
            } else if residual.abs() > 1e-10 {
                return Err(domain("correlation matrix must be positive semidefinite"));
            } else {
                // A zero diagonal in a PSD matrix has a zero residual here.
                lower[i][j] = 0.0;
            }
        }
    }
    Ok(lower)
}

pub(super) fn correlate(l: &[Vec<f64>], z: &[f64]) -> Vec<f64> {
    l.iter()
        .map(|row| row.iter().zip(z).map(|(a, b)| a * b).sum())
        .collect()
}
