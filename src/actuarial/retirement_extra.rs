//! Additional retirement calculators kept separate from the core withdrawal module.
use crate::{FinanceError, Result};

/// PIA calculation using the eligibility year's supplied bend points (2026 amounts).
///
/// ```
/// let pia = finance_rs::actuarial::retirement::social_security_pia(5_000.0, 2026, (1_226.0, 7_391.0))?;
/// assert!(pia > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn social_security_pia(aime: f64, year: u16, bend_points: (f64, f64)) -> Result<f64> {
    if !aime.is_finite() || aime < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "aime",
            reason: "must be finite and nonnegative",
        });
    }
    let (b1, b2) = bend_points;
    if !b1.is_finite() || !b2.is_finite() || b1 <= 0.0 || b2 <= b1 {
        return Err(FinanceError::InvalidInput {
            what: "bend_points",
            reason: "must be positive and increasing",
        });
    }
    if year < 1979 {
        return Err(FinanceError::InvalidInput {
            what: "year",
            reason: "must be 1979 or later",
        });
    }
    let raw =
        0.9 * aime.min(b1) + 0.32 * (aime - b1).max(0.0).min(b2 - b1) + 0.15 * (aime - b2).max(0.0);
    Ok((raw * 10.0).floor() / 10.0)
}
/// Benefit factor for early/delayed claiming relative to full retirement age, in months.
///
/// ```
/// let factor = finance_rs::actuarial::retirement::claiming_adjustment(36)?;
/// assert!((factor - 1.24).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn claiming_adjustment(fra_months_delta: i16) -> Result<f64> {
    let months = i32::from(fra_months_delta);
    if !(-60..=60).contains(&months) {
        return Err(FinanceError::InvalidInput {
            what: "fra_months_delta",
            reason: "must be within five years of full retirement age",
        });
    }
    if months < 0 {
        let early = -months;
        let first = early.min(36);
        Ok(1.0
            - f64::from(first) * (5.0 / 9.0) / 100.0
            - f64::from((early - 36).max(0)) * (5.0 / 12.0) / 100.0)
    } else {
        Ok(1.0 + f64::from(months) * (2.0 / 3.0) / 100.0)
    }
}
/// IRS §72(t) calculation method.
///
/// ```
/// use finance_rs::actuarial::retirement::SeppMethod;
/// let method = SeppMethod::RequiredMinimumDistribution;
/// assert_eq!(method, SeppMethod::RequiredMinimumDistribution);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SeppMethod {
    /// The required minimum distribution option.
    RequiredMinimumDistribution,
    /// The amortization option.
    Amortization,
    /// The annuitization option.
    Annuitization,
}
/// Annual substantially-equal periodic payment; factor is actuarial for the selected method.
///
/// ```
/// use finance_rs::actuarial::retirement::{sepp_72t, SeppMethod};
/// let annual = sepp_72t(100_000.0, 55, 0.04, 0.03, SeppMethod::RequiredMinimumDistribution, 30.0, 20.0)?;
/// assert!((annual - 100_000.0 / 30.0).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn sepp_72t(
    balance: f64,
    age: u8,
    rate: f64,
    federal_mid_term_rate: f64,
    method: SeppMethod,
    life_expectancy: f64,
    annuity_factor: f64,
) -> Result<f64> {
    if !balance.is_finite()
        || balance <= 0.0
        || age < 18
        || !rate.is_finite()
        || rate < 0.0
        || !federal_mid_term_rate.is_finite()
        || federal_mid_term_rate < 0.0
        || !life_expectancy.is_finite()
        || life_expectancy <= 0.0
        || !annuity_factor.is_finite()
        || annuity_factor <= 0.0
    {
        return Err(FinanceError::InvalidInput {
            what: "SEPP inputs",
            reason: "balance, age, rate and actuarial factors must be positive and finite",
        });
    }
    let rate_cap = 0.05_f64.max(1.2 * federal_mid_term_rate);
    if rate > rate_cap {
        return Err(FinanceError::InvalidInput {
            what: "rate",
            reason: "must not exceed the greater of 5% or 120% of the federal mid-term rate",
        });
    }
    Ok(match method {
        SeppMethod::RequiredMinimumDistribution => balance / life_expectancy,
        SeppMethod::Amortization => balance * rate / (1.0 - (1.0 + rate).powf(-life_expectancy)),
        SeppMethod::Annuitization => balance / annuity_factor,
    })
}
/// Roth conversion ladder: beginning balance followed by annual growth and conversions.
///
/// ```
/// let path = finance_rs::actuarial::retirement::roth_conversion_ladder(100.0, 10.0, 0.0, 2)?;
/// assert_eq!(path, vec![100.0, 110.0, 120.0]);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn roth_conversion_ladder(
    balance: f64,
    annual_conversion: f64,
    real_return: f64,
    years: u32,
) -> Result<Vec<f64>> {
    if !balance.is_finite()
        || balance < 0.0
        || !annual_conversion.is_finite()
        || annual_conversion < 0.0
        || !real_return.is_finite()
        || real_return <= -1.0
    {
        return Err(FinanceError::InvalidInput {
            what: "ladder inputs",
            reason: "balances and conversions must be nonnegative; return must exceed -1",
        });
    }
    let mut b = balance;
    let mut path = vec![b];
    for _ in 0..years {
        b = b * (1.0 + real_return) + annual_conversion;
        if !b.is_finite() {
            return Err(FinanceError::Overflow {
                what: "Roth conversion ladder",
            });
        }
        path.push(b);
    }
    Ok(path)
}

/// Annual real spending schedule with indexed one-time cash-flow adjustments.
///
/// ```
/// let path = finance_rs::actuarial::retirement::spending_smile_schedule(100.0, 0.0, 3, &[(2, 5.0)])?;
/// assert_eq!(path, vec![100.0, 105.0, 100.0]);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn spending_smile_schedule(
    base: f64,
    annual_growth: f64,
    years: u32,
    shocks: &[(u32, f64)],
) -> Result<Vec<f64>> {
    if !base.is_finite()
        || base < 0.0
        || !annual_growth.is_finite()
        || annual_growth <= -1.0
        || shocks.iter().any(|(_, v)| !v.is_finite())
    {
        return Err(FinanceError::InvalidInput {
            what: "spending inputs",
            reason: "base must be nonnegative, growth must exceed -1, and shocks must be finite",
        });
    }
    let mut out: Vec<f64> = (0..years)
        .map(|y| base * (1.0 + annual_growth).powf(f64::from(y)))
        .collect();
    for (year, amount) in shocks {
        if *year == 0 || *year > years {
            return Err(FinanceError::InvalidInput {
                what: "shock year",
                reason: "must be within the projection",
            });
        }
        let value = &mut out[(*year - 1) as usize];
        *value += amount;
        if !value.is_finite() {
            return Err(FinanceError::Overflow {
                what: "spending schedule",
            });
        }
    }
    Ok(out)
}
