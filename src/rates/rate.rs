//! [`Rate`]: an interest rate together with its quote basis.

#![allow(clippy::doc_markdown)] // "QuantLib" is a product name.

use super::Compounding;
use crate::daycount::DayCount;
use crate::error::{div, finite, Result};
use crate::Date;

/// An interest rate with its compounding and day count (QuantLib
/// `InterestRate`). The day count only matters when time is measured
/// between dates ([`Rate::compound_factor_between`]).
///
/// ```
/// use finance_rs::{daycount::DayCount, rates::{Compounding, Rate}, Date};
/// let r = Rate::new(0.05, Compounding::Simple, DayCount::Act360);
/// let f = r.compound_factor_between(Date::new(2024, 1, 1)?, Date::new(2024, 7, 1)?)?;
/// assert!((f - (1.0 + 0.05 * 182.0 / 360.0)).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Rate {
    /// The quoted rate as a decimal (`0.05` = 5%).
    pub value: f64,
    /// How the rate accrues.
    pub compounding: Compounding,
    /// How time between dates is measured.
    pub day_count: DayCount,
}

impl Rate {
    /// A rate quoted on `compounding` and `day_count`.
    #[must_use]
    /// ```rust
    /// assert!(finance_rs::rates::effective(0.12, 12.0).unwrap() > 0.12);
    /// ```
    pub const fn new(value: f64, compounding: Compounding, day_count: DayCount) -> Self {
        Self {
            value,
            compounding,
            day_count,
        }
    }

    /// Growth of 1 over `t ≥ 0` years (QuantLib `compoundFactor(t)`).
    ///
    /// # Errors
    /// As [`Compounding::compound_factor`].
    /// ```rust
    /// assert!(finance_rs::rates::effective(0.12, 12.0).unwrap() > 0.12);
    /// ```
    pub fn compound_factor(&self, t: f64) -> Result<f64> {
        self.compounding.compound_factor(self.value, t)
    }

    /// Present value of 1 due in `t ≥ 0` years: `1/compound_factor(t)`.
    ///
    /// ```
    /// use finance_rs::{daycount::DayCount, rates::{Compounding, Rate}};
    /// let r = Rate::new(0.03, Compounding::Continuous, DayCount::Act365F);
    /// assert!((r.discount_factor(2.0)? - (-0.06f64).exp()).abs() < 1e-15);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Compounding::compound_factor`].
    pub fn discount_factor(&self, t: f64) -> Result<f64> {
        div("compound factor", 1.0, self.compound_factor(t)?)
    }

    /// Compound factor from `start` to `end`, with `t` from the day count
    /// (QuantLib `compoundFactor(d1, d2)`).
    ///
    /// # Errors
    /// [`crate::FinanceError::InvalidInput`] when `end` is before `start`;
    /// otherwise as [`Rate::compound_factor`].
    /// ```rust
    /// assert!(finance_rs::rates::effective(0.12, 12.0).unwrap() > 0.12);
    /// ```
    pub fn compound_factor_between<D: Copy + Into<Date>>(&self, start: D, end: D) -> Result<f64> {
        self.compound_factor(self.day_count.year_fraction(start.into(), end.into()))
    }

    /// Discount factor from `end` back to `start`.
    ///
    /// # Errors
    /// As [`Rate::compound_factor_between`].
    /// ```rust
    /// assert!(finance_rs::rates::effective(0.12, 12.0).unwrap() > 0.12);
    /// ```
    pub fn discount_factor_between<D: Copy + Into<Date>>(&self, start: D, end: D) -> Result<f64> {
        div(
            "compound factor",
            1.0,
            self.compound_factor_between(start, end)?,
        )
    }

    /// The rate on another basis with the same compound factor over `t`
    /// years (QuantLib `equivalentRate(comp, freq, t)`); the day count is kept.
    ///
    /// ```
    /// use finance_rs::{daycount::DayCount, rates::{Compounding, Frequency, Rate}};
    /// let m = Rate::new(0.12, Compounding::Compounded(Frequency::Monthly), DayCount::Act365F);
    /// let a = m.equivalent(Compounding::Compounded(Frequency::Annual), 1.0)?;
    /// assert!((a.value - (1.01f64.powi(12) - 1.0)).abs() < 1e-15);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Compounding::implied_rate`] and [`Rate::compound_factor`].
    pub fn equivalent(&self, to: Compounding, t: f64) -> Result<Self> {
        let value = to.implied_rate(self.compound_factor(t)?, t)?;
        Ok(Self::new(value, to, self.day_count))
    }

    /// The rate on `compounding` that grows 1 to `compound` over `t` years
    /// (QuantLib `InterestRate::impliedRate`).
    ///
    /// # Errors
    /// As [`Compounding::implied_rate`].
    /// ```rust
    /// assert!(finance_rs::rates::effective(0.12, 12.0).unwrap() > 0.12);
    /// ```
    pub fn implied(
        compound: f64,
        compounding: Compounding,
        day_count: DayCount,
        t: f64,
    ) -> Result<Self> {
        finite("compound", compound)?;
        let value = compounding.implied_rate(compound, t)?;
        Ok(Self::new(value, compounding, day_count))
    }
}
