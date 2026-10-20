//! Retirement rules and withdrawal methods. Tax and return inputs are explicit.
use crate::{tvm, FinanceError, Result};
/// Beneficiary IRA and adaptive withdrawal policy types and functions.
///
/// ```
/// use finance_rs::actuarial::retirement::{inherited_ira_schedule, InheritedIraRule};
/// let draws = inherited_ira_schedule(100.0, 0.0, InheritedIraRule::FiveYear { years_elapsed: 0 })?;
/// assert_eq!(draws.len(), 5);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[path = "retirement_extra.rs"]
mod retirement_extra;
#[path = "retirement_rules.rs"]
mod retirement_rules;
/// Additional Social Security and retirement projection functions.
///
/// ```
/// use finance_rs::actuarial::retirement::{claiming_adjustment, roth_conversion_ladder};
/// assert_eq!(claiming_adjustment(0)?, 1.0);
/// assert_eq!(roth_conversion_ladder(100.0, 10.0, 0.0, 1)?, vec![100.0, 110.0]);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub use retirement_extra::{
    claiming_adjustment, roth_conversion_ladder, sepp_72t, social_security_pia,
    spending_smile_schedule, SeppMethod,
};
/// Beneficiary IRA distribution schedules and Guyton-Klinger policy types.
///
/// ```
/// use finance_rs::actuarial::retirement::{inherited_ira_schedule, InheritedIraRule};
/// let draws = inherited_ira_schedule(100.0, 0.0, InheritedIraRule::FiveYear { years_elapsed: 0 })?;
/// assert_eq!(draws.last(), Some(&100.0));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub use retirement_rules::{
    blanchett_spending_change, blanchett_spending_schedule, guyton_klinger_withdrawal,
    inherited_ira_schedule, GuytonKlingerPolicy, InheritedIraRule,
};

/// Applicable RMD age under SECURE 2.0 §107.
///
/// ```
/// assert_eq!(finance_rs::actuarial::retirement::rmd_start_age(1960)?, 75.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn rmd_start_age(birth_year: u16) -> Result<f64> {
    match birth_year {
        1900..=1948 => Ok(70.5),
        1949..=1950 => Ok(72.0),
        1951..=1959 => Ok(73.0),
        1960..=u16::MAX => Ok(75.0),
        _ => Err(FinanceError::InvalidInput {
            what: "birth_year",
            reason: "must be 1900 or later",
        }),
    }
}
/// Required minimum distribution for the supplied IRS distribution-period divisor.
///
/// ```
/// assert_eq!(finance_rs::actuarial::retirement::rmd(100_000.0, 73, 25.5)?, 100_000.0 / 25.5);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn rmd(balance: f64, age: u8, distribution_period: f64) -> Result<f64> {
    if !balance.is_finite() || balance < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "balance",
            reason: "must be finite and nonnegative",
        });
    }
    if !(distribution_period.is_finite() && distribution_period > 0.0) {
        return Err(FinanceError::InvalidInput {
            what: "distribution_period",
            reason: "must be finite and positive",
        });
    }
    if age < 1 {
        return Err(FinanceError::InvalidInput {
            what: "age",
            reason: "must be positive",
        });
    }
    Ok(balance / distribution_period)
}
/// Beneficiary 10-year rule: equal end-of-year withdrawals that exhaust the balance.
///
/// ```
/// let schedule = finance_rs::actuarial::retirement::inherited_ira_10yr(100.0, 0.0, 9)?;
/// assert_eq!(schedule, vec![100.0]);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn inherited_ira_10yr(balance: f64, annual_return: f64, years_elapsed: u8) -> Result<Vec<f64>> {
    if !balance.is_finite() || balance < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "balance",
            reason: "must be finite and nonnegative",
        });
    }
    if !annual_return.is_finite() || annual_return <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "annual_return",
            reason: "must be finite and greater than -1",
        });
    }
    if years_elapsed >= 10 {
        return Err(FinanceError::InvalidInput {
            what: "years_elapsed",
            reason: "must be less than 10",
        });
    }
    let remaining = 10 - years_elapsed;
    let mut b = balance;
    let mut out = Vec::with_capacity(remaining as usize);
    for _ in 0..remaining {
        b *= 1.0 + annual_return;
        let draw = b / f64::from(remaining);
        out.push(draw);
        b -= draw;
    }
    Ok(out)
}
/// Real annual withdrawal for the constant-real (4% rule) method, over explicit years.
///
/// ```
/// let spend = finance_rs::actuarial::retirement::constant_real(0.04, 100_000.0, 0.02, 3)?;
/// assert_eq!(spend, vec![4_000.0, 4_080.0, 4_161.6]);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn constant_real(
    initial_rate: f64,
    balance: f64,
    inflation: f64,
    years: u32,
) -> Result<Vec<f64>> {
    if !initial_rate.is_finite()
        || initial_rate < 0.0
        || !balance.is_finite()
        || balance < 0.0
        || !inflation.is_finite()
        || inflation <= -1.0
    {
        return Err(FinanceError::InvalidInput {
            what: "withdrawal inputs",
            reason: "rate and balance must be nonnegative; inflation must exceed -1",
        });
    }
    let first = initial_rate * balance;
    let mut out = Vec::with_capacity(years as usize);
    let mut spend = first;
    for _ in 0..years {
        out.push(spend);
        spend *= 1.0 + inflation;
    }
    Ok(out)
}
/// Guyton-Klinger guardrail adjustment to the current real spending amount.
///
/// ```
/// let spend = finance_rs::actuarial::retirement::guyton_klinger(
///     1_000.0, 40.0, 0.04, 0.1, 0.1, 0.2, 0.2,
/// )?;
/// assert!(spend.is_finite());
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn guyton_klinger(
    balance: f64,
    current_spend: f64,
    initial_spend: f64,
    cut: f64,
    raise: f64,
    lower_guardrail: f64,
    upper_guardrail: f64,
) -> Result<f64> {
    if [
        balance,
        current_spend,
        initial_spend,
        cut,
        raise,
        lower_guardrail,
        upper_guardrail,
    ]
    .iter()
    .any(|v| !v.is_finite())
        || balance < 0.0
        || current_spend < 0.0
        || initial_spend < 0.0
        || cut < 0.0
        || cut > 1.0
        || raise < 0.0
        || lower_guardrail < 0.0
        || upper_guardrail < lower_guardrail
        || initial_spend == 0.0
    {
        return Err(FinanceError::InvalidInput {
            what: "guardrail inputs",
            reason: "invalid balance, spending, rates, or guardrail range",
        });
    }
    let rate = current_spend / balance.max(f64::MIN_POSITIVE);
    let initial_rate = initial_spend;
    Ok(if rate > initial_rate * (1.0 + upper_guardrail) {
        current_spend * (1.0 - cut)
    } else if rate < initial_rate * (1.0 - lower_guardrail) {
        current_spend * (1.0 + raise)
    } else {
        current_spend
    })
}
/// Variable-percentage withdrawal from an amortizing finite horizon.
///
/// ```
/// let annual = finance_rs::actuarial::retirement::vpw(1_000_000.0, 65, 0.03, 30)?;
/// assert!(annual > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn vpw(balance: f64, age: u8, rate: f64, horizon: u8) -> Result<f64> {
    if horizon == 0 || age == 0 {
        return Err(FinanceError::InvalidInput {
            what: "horizon",
            reason: "age and horizon must be positive",
        });
    }
    if !balance.is_finite() || balance < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "balance",
            reason: "must be finite and nonnegative",
        });
    }
    if rate <= -1.0 || !rate.is_finite() {
        return Err(FinanceError::InvalidInput {
            what: "rate",
            reason: "must be finite and greater than -1",
        });
    }
    let years = horizon.min(100 - age.saturating_sub(1));
    if years == 0 {
        return Ok(balance);
    }
    Ok(-tvm::pmt(
        rate,
        f64::from(years),
        balance,
        0.0,
        tvm::When::End,
    )?)
}
/// Fraction of return paths whose balance remains nonnegative after each withdrawal.
///
/// ```rust
/// use finance_rs::actuarial::retirement::success_rate;
/// let probability = success_rate(&[vec![0.10, 0.10], vec![-0.90, 0.0]], &[0.20, 0.20])?;
/// assert_eq!(probability, 0.5);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn success_rate(paths: &[Vec<f64>], withdrawals: &[f64]) -> Result<f64> {
    if paths.is_empty() {
        return Err(FinanceError::TooShort {
            what: "paths",
            min: 1,
            got: 0,
        });
    }
    if withdrawals.iter().any(|x| !x.is_finite() || *x < 0.0) {
        return Err(FinanceError::InvalidInput {
            what: "withdrawals",
            reason: "must be finite and nonnegative",
        });
    }
    let mut successes = 0usize;
    for path in paths {
        if path.len() < withdrawals.len() {
            return Err(FinanceError::LengthMismatch {
                left: "path",
                right: "withdrawals",
                left_len: path.len(),
                right_len: withdrawals.len(),
            });
        }
        let mut balance = 1.0;
        let mut ok = true;
        for (ret, draw) in path.iter().zip(withdrawals) {
            if !ret.is_finite() || *ret <= -1.0 {
                return Err(FinanceError::InvalidInput {
                    what: "returns",
                    reason: "must be finite and greater than -1",
                });
            }
            balance = balance * (1.0 + ret) - draw;
            if balance < 0.0 {
                ok = false;
                break;
            }
        }
        if ok {
            successes += 1;
        }
    }
    Ok(successes as f64 / paths.len() as f64)
}
/// Blanchett-style U-shaped real spending multiplier, normalized to 1 at age 65.
///
/// ```
/// assert_eq!(finance_rs::actuarial::retirement::spending_smile(1_000.0, 65.0)?, 920.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn spending_smile(base: f64, age: f64) -> Result<f64> {
    if !base.is_finite() || base < 0.0 || !age.is_finite() || !(0.0..=120.0).contains(&age) {
        return Err(FinanceError::InvalidInput {
            what: "base/age",
            reason: "base must be nonnegative and age must be between 0 and 120",
        });
    }
    let x = (age - 65.0) / 10.0;
    let factor = (0.92 - 0.08 * x + 0.006 * x * x).max(0.0);
    Ok(base * factor)
}
/// Terminal value deciles for caller-provided retirement scenarios (legacy dashboard summary).
///
/// ```rust
/// use finance_rs::actuarial::retirement::terminal_path_quantiles;
/// let (lower, median, upper) = terminal_path_quantiles(&[
///     vec![0.0, 10.0], vec![0.0, 20.0], vec![0.0, 30.0],
/// ])?;
/// assert_eq!((lower, median, upper), (10.0, 20.0, 30.0));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
pub fn terminal_path_quantiles(paths: &[Vec<f64>]) -> Result<(f64, f64, f64)> {
    if paths.is_empty()
        || paths
            .iter()
            .any(|p| p.is_empty() || p.iter().any(|x| !x.is_finite()))
    {
        return Err(FinanceError::InvalidInput {
            what: "paths",
            reason: "must contain finite terminal observations",
        });
    }
    let mut ends: Vec<f64> = paths.iter().map(|p| p[p.len() - 1]).collect();
    ends.sort_by(f64::total_cmp);
    let q = |p: f64| ends[((ends.len() - 1) as f64 * p).round() as usize];
    Ok((q(0.1), q(0.5), q(0.9)))
}

#[cfg(test)]
#[path = "retirement_tests.rs"]
mod tests;

/// Nested API matching the catalog's module paths.
///
/// ```
/// let amount = finance_rs::actuarial::retirement::withdrawal::vpw(1_000.0, 65, 0.02, 20)?;
/// assert!(amount > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub mod withdrawal {
    /// Re-export withdrawal policies at the catalog path.
    ///
    /// ```
    /// use finance_rs::actuarial::retirement::withdrawal::{constant_real, vpw};
    /// assert_eq!(constant_real(0.04, 1_000.0, 0.0, 1)?, vec![40.0]);
    /// assert!(vpw(1_000.0, 65, 0.02, 20)? > 0.0);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    pub use super::{
        constant_real, guyton_klinger, guyton_klinger_withdrawal, success_rate, vpw,
        GuytonKlingerPolicy,
    };
}
#[path = "social_security.rs"]
/// Social security.
///
/// ```
/// let amount = finance_rs::actuarial::retirement::social_security::pia(5_000.0, 2026)?;
/// assert!(amount > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub mod social_security;
