//! Beneficiary withdrawal and dynamic spending rules for retirement plans.
use crate::{FinanceError, Result};

/// IRS beneficiary path selected from the beneficiary class and decedent's RMD status.
///
/// ```
/// use finance_rs::actuarial::retirement::InheritedIraRule;
/// let rule = InheritedIraRule::FiveYear { years_elapsed: 0 };
/// assert_eq!(std::mem::discriminant(&rule), std::mem::discriminant(&InheritedIraRule::FiveYear { years_elapsed: 2 }));
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InheritedIraRule {
    /// Surviving spouse or eligible designated beneficiary using a single-life divisor.
    LifeExpectancy {
        /// Initial divisor.
        initial_divisor: f64,
        /// Years elapsed.
        years_elapsed: u8,
    },
    /// Designated beneficiary subject to the 10-year rule; when `annual_divisor` is
    /// present, annual RMDs are also due before the final year.
    TenYear {
        /// Annual divisor.
        annual_divisor: Option<f64>,
        /// Years elapsed.
        years_elapsed: u8,
    },
    /// Non-designated beneficiary subject to the 5-year rule.
    FiveYear {
        /// Number of full distribution years already elapsed.
        years_elapsed: u8,
    },
}

/// Project beneficiary-specific inherited IRA distributions.
///
/// Returns end-of-year distributions, applying return before distribution. A
/// life-expectancy divisor declines by one each year; 5- and 10-year rules
/// distribute any remaining balance at their deadline.
///
/// ```
/// use finance_rs::actuarial::retirement::{inherited_ira_schedule, InheritedIraRule};
/// let schedule = inherited_ira_schedule(100.0, 0.0, InheritedIraRule::FiveYear { years_elapsed: 0 })?;
/// assert_eq!(schedule, vec![0.0, 0.0, 0.0, 0.0, 100.0]);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
pub fn inherited_ira_schedule(
    balance: f64,
    annual_return: f64,
    rule: InheritedIraRule,
) -> Result<Vec<f64>> {
    if !balance.is_finite() || balance < 0.0 || !annual_return.is_finite() || annual_return <= -1.0
    {
        return Err(FinanceError::InvalidInput {
            what: "inherited IRA inputs",
            reason: "balance must be nonnegative and return must be finite and greater than -1",
        });
    }
    let (deadline, elapsed) = match rule {
        InheritedIraRule::LifeExpectancy {
            initial_divisor,
            years_elapsed,
        } => {
            if !initial_divisor.is_finite()
                || !(0.0..=120.0).contains(&initial_divisor)
                || initial_divisor.ceil() <= f64::from(years_elapsed)
            {
                return Err(FinanceError::InvalidInput {
                    what: "life_expectancy_divisor",
                    reason: "must exceed elapsed years",
                });
            }
            (initial_divisor.ceil() as u8, years_elapsed)
        }
        InheritedIraRule::TenYear {
            annual_divisor,
            years_elapsed,
        } => {
            if annual_divisor.is_some_and(|d| !d.is_finite() || d <= 0.0) || years_elapsed >= 10 {
                return Err(FinanceError::InvalidInput {
                    what: "ten_year_rule",
                    reason: "divisor must be positive and elapsed years less than ten",
                });
            }
            (10, years_elapsed)
        }
        InheritedIraRule::FiveYear { years_elapsed } => {
            if years_elapsed >= 5 {
                return Err(FinanceError::InvalidInput {
                    what: "five_year_rule",
                    reason: "elapsed years must be less than five",
                });
            }
            (5, years_elapsed)
        }
    };
    let mut value = balance;
    let mut out = Vec::with_capacity(usize::from(deadline - elapsed));
    for year in elapsed..deadline {
        value *= 1.0 + annual_return;
        if !value.is_finite() {
            return Err(FinanceError::Overflow {
                what: "inherited IRA schedule",
            });
        }
        let divisor = match rule {
            InheritedIraRule::LifeExpectancy {
                initial_divisor, ..
            } => Some(initial_divisor - f64::from(year)),
            InheritedIraRule::TenYear { annual_divisor, .. } if year + 1 < deadline => {
                annual_divisor
                    .map(|initial| initial - f64::from(year))
                    .filter(|d| *d > 0.0)
            }
            _ => None,
        };
        let draw = if year + 1 == deadline {
            value
        } else if let Some(divisor) = divisor {
            (value / divisor).min(value)
        } else {
            0.0
        };
        out.push(draw);
        value -= draw;
    }
    Ok(out)
}

/// Tunable parameters from the Guyton-Klinger decision rules.
///
/// ```
/// use finance_rs::actuarial::retirement::GuytonKlingerPolicy;
/// let policy = GuytonKlingerPolicy {
///     modified_withdrawal_rule: true,
///     preservation_threshold: 0.2,
///     preservation_cut: 0.1,
///     prosperity_threshold: 0.2,
///     prosperity_raise: 0.1,
/// };
/// assert!(policy.modified_withdrawal_rule);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GuytonKlingerPolicy {
    /// Freeze the inflation increase after a negative-return year when the
    /// resulting withdrawal rate exceeds the initial rate.
    pub modified_withdrawal_rule: bool,
    /// Capital-preservation trigger above initial rate (e.g. 0.20 for 20%).
    pub preservation_threshold: f64,
    /// Fraction cut when capital preservation triggers (e.g. 0.10 for 10%).
    pub preservation_cut: f64,
    /// Prosperity trigger below initial rate (e.g. 0.20 for 20%).
    pub prosperity_threshold: f64,
    /// Fraction raise when prosperity triggers (e.g. 0.10 for 10%).
    pub prosperity_raise: f64,
}

/// Standard Guyton-Klinger real spending update for one annual review.
///
/// ```
/// use finance_rs::actuarial::retirement::{guyton_klinger_withdrawal, GuytonKlingerPolicy};
/// let policy = GuytonKlingerPolicy {
///     modified_withdrawal_rule: false,
///     preservation_threshold: 0.2,
///     preservation_cut: 0.1,
///     prosperity_threshold: 0.2,
///     prosperity_raise: 0.1,
/// };
/// let next = guyton_klinger_withdrawal(100_000.0, 4_000.0, 0.04, 0.02, 0.05, 20, policy)?;
/// assert!(next > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn guyton_klinger_withdrawal(
    balance: f64,
    current_spend: f64,
    initial_rate: f64,
    inflation: f64,
    prior_portfolio_return: f64,
    years_remaining: u8,
    policy: GuytonKlingerPolicy,
) -> Result<f64> {
    let values = [
        balance,
        current_spend,
        initial_rate,
        inflation,
        prior_portfolio_return,
        policy.preservation_threshold,
        policy.preservation_cut,
        policy.prosperity_threshold,
        policy.prosperity_raise,
    ];
    if values.iter().any(|v| !v.is_finite())
        || balance < 0.0
        || current_spend < 0.0
        || initial_rate <= 0.0
        || inflation <= -1.0
        || prior_portfolio_return <= -1.0
        || policy.preservation_threshold < 0.0
        || !(0.0..=1.0).contains(&policy.preservation_cut)
        || policy.prosperity_threshold < 0.0
        || policy.prosperity_raise < 0.0
    {
        return Err(FinanceError::InvalidInput {
            what: "Guyton-Klinger inputs",
            reason:
                "rates, returns, spending, and policy parameters are outside their valid ranges",
        });
    }
    let inflated = if policy.modified_withdrawal_rule
        && prior_portfolio_return < 0.0
        && current_spend * (1.0 + inflation) / balance.max(f64::MIN_POSITIVE) > initial_rate
    {
        current_spend
    } else {
        current_spend * (1.0 + inflation)
    };
    if !inflated.is_finite() {
        return Err(FinanceError::Overflow {
            what: "Guyton-Klinger withdrawal",
        });
    }
    let rate = inflated / balance.max(f64::MIN_POSITIVE);
    if years_remaining > 15 && rate > initial_rate * (1.0 + policy.preservation_threshold) {
        Ok(inflated * (1.0 - policy.preservation_cut))
    } else if rate < initial_rate * (1.0 - policy.prosperity_threshold) {
        let raised = inflated * (1.0 + policy.prosperity_raise);
        if raised.is_finite() {
            Ok(raised)
        } else {
            Err(FinanceError::Overflow {
                what: "Guyton-Klinger withdrawal",
            })
        }
    } else {
        Ok(inflated)
    }
}

/// Blanchett (2014) modeled annual real spending change as a function of age
/// and target annual spending. The paper caps the change to [-1%, +1%].
///
/// ```
/// let change = finance_rs::actuarial::retirement::blanchett_spending_change(65.0, 50_000.0)?;
/// assert!((-0.01..=0.01).contains(&change));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn blanchett_spending_change(age: f64, target_spending: f64) -> Result<f64> {
    if !age.is_finite()
        || !(0.0..=120.0).contains(&age)
        || !target_spending.is_finite()
        || target_spending <= 0.0
    {
        return Err(FinanceError::InvalidInput {
            what: "age/target_spending",
            reason: "age must be 0 through 120 and target spending must be positive",
        });
    }
    let change = 0.00008 * age * age - 0.0125 * age - 0.0066 * target_spending.ln() + 0.546;
    Ok(change.clamp(-0.01, 0.01))
}

/// Blanchett (2014) age- and target-specific real spending path.
///
/// ```
/// let path = finance_rs::actuarial::retirement::blanchett_spending_schedule(50_000.0, 65.0, 3)?;
/// assert_eq!(path.len(), 3);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn blanchett_spending_schedule(
    initial_spending: f64,
    starting_age: f64,
    years: u32,
) -> Result<Vec<f64>> {
    if !initial_spending.is_finite()
        || initial_spending <= 0.0
        || !starting_age.is_finite()
        || starting_age < 0.0
        || starting_age + f64::from(years) > 120.0
    {
        return Err(FinanceError::InvalidInput {
            what: "spending schedule",
            reason: "initial spending must be positive and the age range must end by 120",
        });
    }
    let mut spending = initial_spending;
    let mut path = Vec::with_capacity(years as usize);
    for year in 0..years {
        path.push(spending);
        spending *=
            1.0 + blanchett_spending_change(starting_age + f64::from(year), initial_spending)?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    const POLICY: GuytonKlingerPolicy = GuytonKlingerPolicy {
        modified_withdrawal_rule: true,
        preservation_threshold: 0.2,
        preservation_cut: 0.1,
        prosperity_threshold: 0.2,
        prosperity_raise: 0.1,
    };

    #[test]
    fn inherited_beneficiary_classes_follow_distinct_deadlines() {
        let ten = inherited_ira_schedule(
            100.0,
            0.0,
            InheritedIraRule::TenYear {
                annual_divisor: None,
                years_elapsed: 0,
            },
        )
        .unwrap();
        assert_eq!(ten.len(), 10);
        assert_eq!(ten[9], 100.0);
        let five =
            inherited_ira_schedule(100.0, 0.0, InheritedIraRule::FiveYear { years_elapsed: 0 })
                .unwrap();
        assert_eq!(five.len(), 5);
        assert_eq!(five[4], 100.0);
        let ten_with_divisor = inherited_ira_schedule(
            100.0,
            0.0,
            InheritedIraRule::TenYear {
                annual_divisor: Some(4.0),
                years_elapsed: 0,
            },
        )
        .unwrap();
        assert_eq!(ten_with_divisor[0], 25.0);
        assert_eq!(ten_with_divisor[4], 0.0);
        let life = inherited_ira_schedule(
            120.0,
            0.0,
            InheritedIraRule::LifeExpectancy {
                initial_divisor: 12.0,
                years_elapsed: 0,
            },
        )
        .unwrap();
        assert_eq!(life.len(), 12);
        assert!((life[0] - 10.0).abs() < 1e-12);
        assert!(inherited_ira_schedule(
            100.0,
            0.0,
            InheritedIraRule::FiveYear { years_elapsed: 5 }
        )
        .is_err());
        assert!(inherited_ira_schedule(
            100.0,
            0.0,
            InheritedIraRule::TenYear {
                annual_divisor: Some(f64::NAN),
                years_elapsed: 0,
            }
        )
        .is_err());
        assert!(inherited_ira_schedule(
            f64::MAX,
            1.0,
            InheritedIraRule::FiveYear { years_elapsed: 0 }
        )
        .is_err());
    }

    #[test]
    fn guyton_klinger_rules_apply_freeze_cut_raise_and_final_fifteen_exception() {
        assert_eq!(
            guyton_klinger_withdrawal(800.0, 40.0, 0.05, 0.03, -0.1, 20, POLICY).unwrap(),
            40.0
        );
        assert_eq!(
            guyton_klinger_withdrawal(500.0, 40.0, 0.05, 0.0, 0.1, 20, POLICY).unwrap(),
            36.0
        );
        assert_eq!(
            guyton_klinger_withdrawal(2_000.0, 40.0, 0.05, 0.0, 0.1, 20, POLICY).unwrap(),
            44.0
        );
        assert_eq!(
            guyton_klinger_withdrawal(500.0, 40.0, 0.0 + 0.05, 0.0, 0.1, 15, POLICY).unwrap(),
            40.0
        );
    }

    #[test]
    fn blanchett_equation_and_schedule_match_published_capped_change() {
        let target = 50_000.0_f64;
        let age = 65.0_f64;
        let expected =
            (0.00008 * age * age - 0.0125 * age - 0.0066 * target.ln() + 0.546).clamp(-0.01, 0.01);
        assert!((blanchett_spending_change(age, target).unwrap() - expected).abs() < 1e-12);
        let path = blanchett_spending_schedule(target, age, 2).unwrap();
        assert_eq!(path[0], target);
        assert!((path[1] - target * (1.0 + expected)).abs() < 1e-8);
    }

    #[test]
    fn inherited_schedule_rejects_invalid_inputs_without_panicking() {
        assert!(inherited_ira_schedule(
            f64::NAN,
            0.0,
            InheritedIraRule::FiveYear { years_elapsed: 0 }
        )
        .is_err());
        assert!(inherited_ira_schedule(
            100.0,
            -1.0,
            InheritedIraRule::FiveYear { years_elapsed: 0 }
        )
        .is_err());
        assert!(
            guyton_klinger_withdrawal(0.0, 1.0, 0.05, 0.02, 0.0, 30, POLICY)
                .unwrap()
                .is_finite()
        );
        assert!(blanchett_spending_schedule(0.0, 65.0, 2).is_err());
        assert!(guyton_klinger_withdrawal(1.0, f64::MAX, 0.05, 1.0, 0.0, 30, POLICY).is_err());
        let overflow_policy = GuytonKlingerPolicy {
            modified_withdrawal_rule: false,
            preservation_threshold: 0.0,
            preservation_cut: 0.0,
            prosperity_threshold: 0.0,
            prosperity_raise: 1.0,
        };
        assert!(
            guyton_klinger_withdrawal(f64::MAX, 1e308, 1e308, 0.0, 0.0, 30, overflow_policy)
                .is_err()
        );
        assert!(blanchett_spending_schedule(1.0, 119.0, 2).is_err());
    }

    proptest! {
        #[test]
        fn inherited_distributions_are_finite_and_never_exceed_balance(
            balance in 0.0_f64..1e12,
            return_rate in -0.99_f64..1.0,
            divisor in 1.0_f64..120.0,
        ) {
            let schedule = inherited_ira_schedule(balance, return_rate,
                InheritedIraRule::LifeExpectancy { initial_divisor: divisor, years_elapsed: 0 }).unwrap();
            prop_assert!(schedule.iter().all(|draw| draw.is_finite() && *draw >= 0.0));
        }

        #[test]
        fn guyton_klinger_preserves_finite_spending(balance in 0.0_f64..1e9, spend in 0.0_f64..1e7) {
            let result = guyton_klinger_withdrawal(balance, spend, 0.04, 0.02, -0.1, 30, POLICY);
            prop_assert!(result.is_ok());
            prop_assert!(result.unwrap().is_finite());
        }

        #[test]
        fn blanchett_schedule_is_finite_and_starts_at_target(
            target in 1.0_f64..1e9,
            age in 0.0_f64..=90.0,
            years in 1_u32..=30,
        ) {
            let schedule = blanchett_spending_schedule(target, age, years).unwrap();
            prop_assert!((schedule[0] - target).abs() < f64::EPSILON);
            prop_assert!(schedule.iter().all(|amount| amount.is_finite() && *amount > 0.0));
        }
    }
}
