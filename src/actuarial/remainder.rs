//! Remainder interest factors from an explicit annual discount rate and
//! survival curve. The caller supplies the survival inputs so this routine
//! does not infer ages, mortality, or a §7520 rate.

use crate::error::{output, FinanceError, Result};

/// Present value of a unit remainder paid when the life interest terminates.
///
/// `survival[t]` is the probability the life interest remains in force at the
/// end of year `t + 1`; the curve must start at one or less and be
/// non-increasing. Any probability surviving the final supplied year is
/// treated as a termination at that horizon. This is a planning factor, not
/// an IRS table factor or tax opinion.
///
/// # Errors
/// Returns an error when the rate or survival curve is invalid, the horizon
/// exceeds the supported integer exponent, or the output is not finite.
pub fn remainder_interest_factor(rate: f64, survival: &[f64]) -> Result<f64> {
    if !rate.is_finite() || rate <= -1.0 || survival.is_empty() {
        return Err(FinanceError::InvalidInput {
            what: "remainder_interest_factor",
            reason: "rate must be finite and greater than -100%, and survival curve nonempty",
        });
    }
    if survival
        .iter()
        .any(|p| !p.is_finite() || !(0.0..=1.0).contains(p))
        || survival.windows(2).any(|w| w[1] > w[0])
    {
        return Err(FinanceError::InvalidInput {
            what: "remainder_interest_factor",
            reason: "survival probabilities must be within [0, 1] and non-increasing",
        });
    }

    let mut prior = 1.0;
    let mut factor = 0.0;
    for (year, remaining) in survival.iter().copied().enumerate() {
        let death_probability = prior - remaining;
        factor += death_probability / (1.0 + rate).powi(survival_exponent(year + 1)?);
        prior = remaining;
    }
    // Close the remaining tail at the stated projection horizon.
    factor += prior / (1.0 + rate).powi(survival_exponent(survival.len())?);
    output("remainder_interest_factor", factor)
}

fn survival_exponent(year: usize) -> Result<i32> {
    i32::try_from(year).map_err(|_| FinanceError::InvalidInput {
        what: "survival horizon",
        reason: "exceeds the supported projection length",
    })
}

#[cfg(test)]
mod tests {
    use super::{remainder_interest_factor, survival_exponent};

    #[test]
    fn synthetic_survival_curve_matches_discounted_death_probability_oracle() {
        let rate = 0.05;
        let survival = [0.8, 0.5, 0.0];
        let oracle = 0.2 / 1.05 + 0.3 / 1.05_f64.powi(2) + 0.5 / 1.05_f64.powi(3);
        assert!((remainder_interest_factor(rate, &survival).unwrap() - oracle).abs() < 1e-12);
    }

    #[test]
    fn rejects_invalid_survival_curve() {
        assert!(remainder_interest_factor(0.05, &[0.8, 0.9]).is_err());
        assert!(remainder_interest_factor(0.05, &[1.1]).is_err());
        assert!(remainder_interest_factor(0.05, &[f64::NAN]).is_err());
    }

    #[test]
    fn rejects_invalid_rates_and_empty_horizon() {
        assert!(remainder_interest_factor(f64::NAN, &[1.0]).is_err());
        assert!(remainder_interest_factor(-1.0, &[1.0]).is_err());
        assert!(remainder_interest_factor(0.05, &[]).is_err());
    }

    #[test]
    fn rejects_projection_exponents_that_do_not_fit() {
        assert_eq!(survival_exponent(i32::MAX as usize).unwrap(), i32::MAX);
        assert!(survival_exponent(i32::MAX as usize + 1).is_err());
    }
}
