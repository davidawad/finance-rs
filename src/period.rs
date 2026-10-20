//! [`Period`]: a tenor such as `3M` or `1Y` (QuantLib `Period`).
//!
//! `Date + Period` ([`Date::add_period`]) never consults a calendar: days
//! and weeks move by calendar days, months and years by
//! [`Date::add_months`] (clamping to the month end).

// "QuantLib", "VaR" are product names and abbreviations, not code.
#![allow(clippy::doc_markdown)]

use std::fmt;
use std::str::FromStr;

use crate::error::{FinanceError, Result};
use crate::Date;

/// The unit of a [`Period`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
/// ```rust
/// assert!(finance_rs::period::Period::new(3, finance_rs::period::TimeUnit::Months).to_string() == "3M");
/// ```
pub enum TimeUnit {
    /// Calendar days (`D`).
    Days,
    /// Seven calendar days (`W`).
    Weeks,
    /// Calendar months (`M`).
    Months,
    /// Twelve calendar months (`Y`).
    Years,
}

/// A signed number of [`TimeUnit`]s.
///
/// ```
/// use finance_rs::{period::{Period, TimeUnit}, Date};
/// let p: Period = "3M".parse()?;
/// assert_eq!(p, Period::new(3, TimeUnit::Months));
/// assert_eq!(Date::new(2024, 11, 30)?.add_period(p)?, Date::new(2025, 2, 28)?);
/// assert_eq!(p.to_string(), "3M");
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Period {
    /// How many units (negative moves backwards).
    pub n: i32,
    /// The unit.
    pub unit: TimeUnit,
}

impl Period {
    /// `n` units.
    #[must_use]
    /// ```rust
    /// assert!(finance_rs::period::Period::new(3, finance_rs::period::TimeUnit::Months).to_string() == "3M");
    /// ```
    pub const fn new(n: i32, unit: TimeUnit) -> Self {
        Self { n, unit }
    }
}

impl fmt::Display for Period {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let u = match self.unit {
            TimeUnit::Days => 'D',
            TimeUnit::Weeks => 'W',
            TimeUnit::Months => 'M',
            TimeUnit::Years => 'Y',
        };
        write!(f, "{}{u}", self.n)
    }
}

impl FromStr for Period {
    type Err = FinanceError;

    /// Parse `[+-]<digits><D|W|M|Y>`, case-insensitive (`"1y"`, `"-2W"`).
    fn from_str(s: &str) -> Result<Self> {
        let bad = FinanceError::InvalidInput {
            what: "period",
            reason: "expected [+-]<digits><D|W|M|Y>",
        };
        let s = s.trim();
        let Some(last) = s.chars().last() else {
            return Err(bad);
        };
        let unit = match last.to_ascii_uppercase() {
            'D' => TimeUnit::Days,
            'W' => TimeUnit::Weeks,
            'M' => TimeUnit::Months,
            'Y' => TimeUnit::Years,
            _ => return Err(bad),
        };
        let digits = &s[..s.len() - last.len_utf8()];
        let unsigned = digits.strip_prefix(['+', '-']).unwrap_or(digits);
        if unsigned.is_empty() || !unsigned.bytes().all(|b| b.is_ascii_digit()) {
            return Err(bad);
        }
        let n = digits.parse::<i32>().map_err(|_| bad)?;
        Ok(Self { n, unit })
    }
}

impl Date {
    /// This date moved by `p`: days and weeks by calendar days, months and
    /// years by [`Date::add_months`].
    ///
    /// # Errors
    /// [`FinanceError::InvalidInput`] when the result leaves the supported
    /// date range.
    /// ```rust
    /// assert!(finance_rs::period::Period::new(3, finance_rs::period::TimeUnit::Months).to_string() == "3M");
    /// ```
    pub fn add_period(self, p: Period) -> Result<Self> {
        let n = i64::from(p.n);
        match p.unit {
            TimeUnit::Days => Self::from_ordinal(self.ordinal() + n),
            TimeUnit::Weeks => Self::from_ordinal(self.ordinal() + 7 * n),
            TimeUnit::Months => self.add_months(p.n),
            TimeUnit::Years => {
                let months = p.n.checked_mul(12).ok_or(FinanceError::InvalidInput {
                    what: "period",
                    reason: "too many years",
                })?;
                self.add_months(months)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_rejects() -> Result<()> {
        assert_eq!("-2w".parse::<Period>()?, Period::new(-2, TimeUnit::Weeks));
        assert_eq!("+10D".parse::<Period>()?, Period::new(10, TimeUnit::Days));
        for bad in ["", "M", "3", "3X", "1.5Y", "--1M", "99999999999D", "é"] {
            assert!(bad.parse::<Period>().is_err(), "{bad}");
        }
        Ok(())
    }

    #[test]
    fn adds() -> Result<()> {
        let d = Date::new(2024, 2, 29)?;
        assert_eq!(
            d.add_period(Period::new(1, TimeUnit::Years))?,
            Date::new(2025, 2, 28)?
        );
        assert_eq!(
            d.add_period(Period::new(-1, TimeUnit::Weeks))?,
            Date::new(2024, 2, 22)?
        );
        assert!(d
            .add_period(Period::new(i32::MAX, TimeUnit::Years))
            .is_err());
        Ok(())
    }
}
