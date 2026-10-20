//! Remove bookmaker overround using common closed-form and solved models.

use crate::error::{all_finite, FinanceError, Result};
use crate::kelly::solve_unit_interval;

fn validate(ps: &[f64]) -> Result<()> {
    if ps.len() < 2 {
        return Err(FinanceError::TooShort {
            what: "probabilities",
            min: 2,
            got: ps.len(),
        });
    }
    all_finite("probabilities", ps)?;
    if ps.iter().any(|p| *p <= 0.0 || *p > 1.0) {
        return Err(FinanceError::InvalidInput {
            what: "probabilities",
            reason: "each probability must be in (0, 1]",
        });
    }
    Ok(())
}

/// Normalize probabilities by their sum.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn multiplicative(probabilities: &[f64]) -> Result<Vec<f64>> {
    validate(probabilities)?;
    let sum: f64 = probabilities.iter().sum();
    Ok(probabilities.iter().map(|p| p / sum).collect())
}

/// Remove an equal share of the overround from every outcome.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn additive(probabilities: &[f64]) -> Result<Vec<f64>> {
    validate(probabilities)?;
    let adjustment = (probabilities.iter().sum::<f64>() - 1.0) / probabilities.len() as f64;
    let result: Vec<f64> = probabilities.iter().map(|p| p - adjustment).collect();
    if result.iter().any(|p| *p < 0.0) {
        return Err(FinanceError::InvalidInput {
            what: "probabilities",
            reason: "additive devig produces a negative probability",
        });
    }
    Ok(result)
}

/// Power method: solve for `k` such that `sum(p_i^k) == 1`.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn power(probabilities: &[f64]) -> Result<Vec<f64>> {
    validate(probabilities)?;
    let sum: f64 = probabilities.iter().sum();
    if (sum - 1.0).abs() <= 1e-14 {
        return Ok(probabilities.to_vec());
    }
    if sum < 1.0 || probabilities.iter().any(|p| *p >= 1.0) {
        return Err(FinanceError::InvalidInput {
            what: "probabilities",
            reason: "power devig requires an overround and each probability below one",
        });
    }
    let residual = |x| probabilities.iter().map(|p| p.powf(x)).sum::<f64>() - 1.0;
    let options = crate::solve::SolverOptions {
        xtol: 1e-12,
        ftol: 1e-14,
        max_iter: 256,
    };
    let solved = crate::solve::brent(residual, 1.0, 1_000_000.0, &options)?;
    let k = solved.root;
    Ok(probabilities.iter().map(|p| p.powf(k)).collect())
}

/// Shin's insider-trading model, solving its common `z` parameter in `[0, 1]`.
/// ```rust
/// assert!((finance_rs::pm::decimal_to_prob(2.0).unwrap() - 0.5).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn shin(probabilities: &[f64]) -> Result<Vec<f64>> {
    validate(probabilities)?;
    let sum: f64 = probabilities.iter().sum();
    if (sum - 1.0).abs() <= 1e-14 {
        return Ok(probabilities.to_vec());
    }
    if sum < 1.0 {
        return Err(FinanceError::InvalidInput {
            what: "probabilities",
            reason: "Shin devig requires an overround",
        });
    }
    let implied = |z: f64| -> Vec<f64> {
        probabilities
            .iter()
            .map(|p| {
                let x = p * p / sum;
                let root = (z * z + 4.0 * (1.0 - z) * x).sqrt();
                // Validation requires p > 0, so x > 0 and root + z stays positive.
                2.0 * x / (root + z)
            })
            .collect()
    };
    let z = solve_unit_interval(|z| implied(z).iter().sum::<f64>() - 1.0)?;
    Ok(implied(z))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn devig_models_cover_identity_and_invalid_probability_paths() {
        assert_eq!(multiplicative(&[0.4, 0.6]).unwrap(), vec![0.4, 0.6]);
        assert_eq!(power(&[0.4, 0.6]).unwrap(), vec![0.4, 0.6]);
        assert_eq!(shin(&[0.4, 0.6]).unwrap(), vec![0.4, 0.6]);
        assert!((power(&[0.6, 0.5]).unwrap().iter().sum::<f64>() - 1.0).abs() < 1e-10);
        assert!((shin(&[0.6, 0.5]).unwrap().iter().sum::<f64>() - 1.0).abs() < 1e-10);
        assert!(additive(&[0.8, 0.8, 0.01]).is_err());
        assert!(power(&[0.4, 0.5]).is_err());
        assert!(power(&[1.0, 0.1]).is_err());
        assert!(shin(&[0.4, 0.5]).is_err());
        assert!(multiplicative(&[0.0, 1.0]).is_err());
    }
}
