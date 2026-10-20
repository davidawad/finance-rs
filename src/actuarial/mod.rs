//! Insurance, annuity, life-contingency, and retirement calculations.
//!
//! Mortality and regulatory tables are explicit inputs or are exposed from
//! [`tables`], so projections remain deterministic and auditable.

/// Annuity factors. For example, value ten level payments with
/// [`annuity::immediate`].
///
/// ```
/// assert!(finance_rs::actuarial::annuity::immediate(0.05, 10.0)? > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub mod annuity;
/// Indexed life insurance and policy-loan calculations.
///
/// ```
/// assert_eq!(finance_rs::actuarial::insurance::iul_credit(0.10, 0.08, 0.0, 1.0)?, 0.08);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub mod insurance;
/// Life-contingency models based on mortality probabilities.
///
/// ```
/// let table = finance_rs::actuarial::life::LifeTable::new(65, vec![0.1])?;
/// assert_eq!(table.px(65)?, 0.9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub mod life;
pub mod remainder;
/// Retirement income, withdrawal, and benefit methods.
///
/// ```
/// assert!(finance_rs::actuarial::retirement::rmd(100_000.0, 73, 26.5)? > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub mod retirement;
#[cfg(feature = "actuarial")]
/// Published IRS and SOA actuarial factors.
///
/// ```
/// assert_eq!(finance_rs::actuarial::tables::irs_uniform_lifetime()[0], 27.4);
/// ```
pub mod tables;
