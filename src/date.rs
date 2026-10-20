//! A minimal proleptic-Gregorian calendar date, so day counts and XIRR work
//! without any dependency. With the `chrono` feature, [`Date`] converts to
//! and from `chrono::NaiveDate`.

use crate::error::{FinanceError, Result};

/// A calendar date (proleptic Gregorian), ordered chronologically.
///
/// ```rust
/// use finance_rs::Date; let d = Date::new(2024, 2, 29)?; assert_eq!(Date::new(2024, 3, 1)?.days_since(d), 1); assert!(Date::new(2023, 2, 29).is_err());
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Date {
    year: i32,
    month: u32,
    day: u32,
}

pub(crate) const MARKET_CALENDAR_FALLBACK: Date = Date {
    year: 2000,
    month: 1,
    day: 1,
};

/// Weekday numbered from Monday, matching `QuantLib` and ISO-8601.
///
/// ```rust
/// use finance_rs::Weekday; assert_eq!(Weekday::Monday.number_from_monday(), 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub enum Weekday {
    /// The monday option.
    Monday,
    /// The tuesday option.
    Tuesday,
    /// The wednesday option.
    Wednesday,
    /// The thursday option.
    Thursday,
    /// The friday option.
    Friday,
    /// The saturday option.
    Saturday,
    /// The sunday option.
    Sunday,
}

impl Weekday {
    /// Zero-based ISO weekday number.
    ///
    /// ```rust
    /// use finance_rs::Weekday; assert_eq!(Weekday::Wednesday.number_from_monday(), 2);
    /// ```
    #[must_use]
    pub const fn number_from_monday(self) -> u32 {
        match self {
            Self::Monday => 0,
            Self::Tuesday => 1,
            Self::Wednesday => 2,
            Self::Thursday => 3,
            Self::Friday => 4,
            Self::Saturday => 5,
            Self::Sunday => 6,
        }
    }
}

/// Whether `year` is a Gregorian leap year: divisible by 4, except
/// centuries not divisible by 400.
///
/// ```rust
/// use finance_rs::is_leap_year; assert!(is_leap_year(2000) && is_leap_year(2024)); assert!(!is_leap_year(1900) && !is_leap_year(2023));
/// ```
#[must_use]
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Number of days in `month` (1-12) of `year`; 0 for an invalid month.
///
/// ```rust
/// use finance_rs::days_in_month; assert_eq!(days_in_month(2024, 2), 29); assert_eq!(days_in_month(2023, 2), 28); assert_eq!(days_in_month(2023, 13), 0);
/// ```
#[must_use]
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

impl Date {
    /// Shift by calendar days, with supported-range validation.
    ///
    /// ```rust
    /// use finance_rs::Date; assert_eq!(Date::new(2024, 1, 31)?.add_days(1)?, Date::new(2024, 2, 1)?);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn add_days(self, days: i32) -> Result<Self> {
        // A supported ordinal is bounded by ±366 million days, while `days`
        // is an i32, so this addition cannot overflow i64.
        let ordinal = self.ordinal() + i64::from(days);
        Self::from_ordinal(ordinal)
    }

    /// Shift by calendar years, clamping leap day to February's end.
    ///
    /// ```rust
    /// use finance_rs::Date; assert_eq!(Date::new(2024, 2, 29)?.add_years(1)?, Date::new(2025, 2, 28)?);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn add_years(self, years: i32) -> Result<Self> {
        let year = self
            .year
            .checked_add(years)
            .ok_or(FinanceError::InvalidInput {
                what: "date",
                reason: "year out of range",
            })?;
        Self::new(
            year,
            self.month,
            self.day.min(days_in_month(year, self.month)),
        )
    }

    /// Weekday with Monday as the first day of the week.
    ///
    /// ```rust
    /// use finance_rs::{Date, Weekday}; assert_eq!(Date::new(2024, 6, 1)?.weekday(), Weekday::Saturday);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn weekday(self) -> Weekday {
        match (self.ordinal() + 3).rem_euclid(7) {
            0 => Weekday::Monday,
            1 => Weekday::Tuesday,
            2 => Weekday::Wednesday,
            3 => Weekday::Thursday,
            4 => Weekday::Friday,
            5 => Weekday::Saturday,
            _ => Weekday::Sunday,
        }
    }

    /// Last calendar day of this date's month.
    ///
    /// ```rust
    /// use finance_rs::Date; assert_eq!(Date::new(2024, 2, 3)?.end_of_month()?, Date::new(2024, 2, 29)?);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn end_of_month(self) -> Result<Self> {
        Self::new(self.year, self.month, days_in_month(self.year, self.month))
    }

    /// First calendar day of this date's month.
    ///
    /// ```rust
    /// use finance_rs::Date; assert_eq!(Date::new(2024, 2, 29)?.start_of_month()?, Date::new(2024, 2, 1)?);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn start_of_month(self) -> Result<Self> {
        Self::new(self.year, self.month, 1)
    }

    /// The nth weekday of a month (positive from the start, negative from the end).
    ///
    /// ```rust
    /// use finance_rs::{Date, Weekday}; assert_eq!(Date::nth_weekday(2024, 6, Weekday::Wednesday, 3)?, Date::new(2024, 6, 19)?);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    #[allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
    pub fn nth_weekday(year: i32, month: u32, weekday: Weekday, n: i32) -> Result<Self> {
        if n == 0 || !(-5..=5).contains(&n) || !(1..=12).contains(&month) {
            return Err(FinanceError::InvalidInput {
                what: "nth weekday",
                reason: "month must be 1..=12 and n must be -5..=-1 or 1..=5",
            });
        }
        if n > 0 {
            let first = Self::new(year, month, 1)?;
            let offset = (weekday.number_from_monday() as i32
                - first.weekday().number_from_monday() as i32)
                .rem_euclid(7);
            Self::new(year, month, 1 + (offset + 7 * (n - 1)) as u32)
        } else {
            let last = Self::new(year, month, days_in_month(year, month))?;
            let offset = (last.weekday().number_from_monday() as i32
                - weekday.number_from_monday() as i32)
                .rem_euclid(7);
            Self::new(
                year,
                month,
                days_in_month(year, month) - (offset + 7 * (-n - 1)) as u32,
            )
        }
    }

    /// Last requested weekday in a month.
    ///
    /// ```rust
    /// use finance_rs::{Date, Weekday}; assert_eq!(Date::last_weekday(2024, 6, Weekday::Sunday)?, Date::new(2024, 6, 30)?);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn last_weekday(year: i32, month: u32, weekday: Weekday) -> Result<Self> {
        Self::nth_weekday(year, month, weekday, -1)
    }

    /// Easter Monday in the Gregorian calendar (Meeus/Jones/Butcher algorithm).
    ///
    /// ```rust
    /// use finance_rs::Date; assert_eq!(Date::easter_monday(2024)?, Date::new(2024, 4, 1)?);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    #[allow(clippy::cast_sign_loss)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
    #[allow(clippy::many_single_char_names)] // Conventional symbols follow the published formula or recurrence notation.
    pub fn easter_monday(year: i32) -> Result<Self> {
        let a = year.rem_euclid(19);
        let b = year.div_euclid(100);
        let c = year.rem_euclid(100);
        let d = b / 4;
        let e = b % 4;
        let f = (b + 8) / 25;
        let g = (b - f + 1) / 3;
        let h = (19 * a + b - d - g + 15).rem_euclid(30);
        let i = c / 4;
        let k = c % 4;
        let l = (32 + 2 * e + 2 * i - h - k).rem_euclid(7);
        let m = (a + 11 * h + 22 * l) / 451;
        let month = (h + l - 7 * m + 114) / 31;
        let day = (h + l - 7 * m + 114) % 31 + 1;
        Self::new(year, month as u32, day as u32)?.add_days(1)
    }

    /// Parse exactly `YYYY-MM-DD`.
    ///
    /// ```rust
    /// use finance_rs::Date; assert_eq!(Date::parse_iso("2024-02-29")?, Date::new(2024, 2, 29)?);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    #[allow(clippy::cast_possible_wrap)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
    pub fn parse_iso(s: &str) -> Result<Self> {
        let bytes = s.as_bytes();
        if bytes.len() != 10
            || bytes[4] != b'-'
            || bytes[7] != b'-'
            || !bytes
                .iter()
                .enumerate()
                .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
        {
            return Err(FinanceError::InvalidInput {
                what: "date",
                reason: "expected YYYY-MM-DD",
            });
        }
        let number = |range: std::ops::Range<usize>| {
            bytes[range]
                .iter()
                .fold(0_u32, |value, digit| value * 10 + u32::from(digit - b'0'))
        };
        let year = number(0..4) as i32;
        let month = number(5..7);
        let day = number(8..10);
        Self::new(year, month, day)
    }

    /// Whether this is an IMM quarterly expiry (third Wednesday of Mar/Jun/Sep/Dec).
    ///
    /// ```rust
    /// use finance_rs::Date; assert!(Date::new(2024, 6, 19)?.is_imm());
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn is_imm(self) -> bool {
        matches!(self.month, 3 | 6 | 9 | 12)
            && Self::nth_weekday(self.year, self.month, Weekday::Wednesday, 3)
                .is_ok_and(|d| d == self)
    }

    /// First quarterly IMM date strictly after `after`.
    ///
    /// ```rust
    /// use finance_rs::Date; assert_eq!(Date::imm_next(Date::new(2024, 6, 19)?)?, Date::new(2024, 9, 18)?);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn imm_next(after: Self) -> Result<Self> {
        for year in after.year..=(after.year + 2).min(1_000_000) {
            for month in [3, 6, 9, 12] {
                let d = Self::nth_weekday(year, month, Weekday::Wednesday, 3)?;
                if d > after {
                    return Ok(d);
                }
            }
        }
        Err(FinanceError::InvalidInput {
            what: "date",
            reason: "IMM date out of range",
        })
    }
    /// Build a date, rejecting impossible ones (month 13, Feb 30, ...).
    /// Years are limited to ±1,000,000.
    ///
    /// ```rust
    /// use finance_rs::Date; assert!(Date::new(2024, 2, 29).is_ok()); assert!(Date::new(2024, 4, 31).is_err());
    /// ```
    ///
    /// # Errors
    /// [`FinanceError::InvalidInput`] for an impossible date.
    pub fn new(year: i32, month: u32, day: u32) -> Result<Self> {
        if !(-1_000_000..=1_000_000).contains(&year) {
            return Err(FinanceError::InvalidInput {
                what: "year",
                reason: "must be within ±1,000,000",
            });
        }
        if day == 0 || day > days_in_month(year, month) {
            return Err(FinanceError::InvalidInput {
                what: "date",
                reason: "no such calendar day",
            });
        }
        Ok(Self { year, month, day })
    }

    /// Calendar year.
    ///
    /// ```rust
    /// use finance_rs::Date; assert_eq!(Date::new(2024, 7, 4)?.year(), 2024);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn year(self) -> i32 {
        self.year
    }

    /// Month, 1-12.
    ///
    /// ```rust
    /// use finance_rs::Date; assert_eq!(Date::new(2024, 7, 4)?.month(), 7);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn month(self) -> u32 {
        self.month
    }

    /// Day of month, 1-31.
    ///
    /// ```rust
    /// use finance_rs::Date; assert_eq!(Date::new(2024, 7, 4)?.day(), 4);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn day(self) -> u32 {
        self.day
    }

    /// Whether this is the last day of its month.
    ///
    /// ```rust
    /// use finance_rs::Date; assert!(Date::new(2024, 2, 29)?.is_month_end()); assert!(!Date::new(2023, 2, 27)?.is_month_end());
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn is_month_end(self) -> bool {
        self.day == days_in_month(self.year, self.month)
    }

    /// Days since 1970-01-01 (negative before). Howard Hinnant's
    /// `days_from_civil`.
    ///
    /// ```rust
    /// use finance_rs::Date; assert_eq!(Date::new(1970, 1, 2)?.ordinal(), 1); assert_eq!(Date::new(1969, 12, 31)?.ordinal(), -1);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn ordinal(self) -> i64 {
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let mp = if m > 2 { m - 3 } else { m + 9 };
        let doy = (153 * mp + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// The date `days` days after 1970-01-01 (inverse of [`Date::ordinal`]).
    ///
    /// ```rust
    /// use finance_rs::Date; assert_eq!(Date::from_ordinal(19_782)?, Date::new(2024, 2, 29)?);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// [`FinanceError::InvalidInput`] when the year leaves ±1,000,000.
    pub fn from_ordinal(days: i64) -> Result<Self> {
        let z = days
            .checked_add(719_468)
            .ok_or(FinanceError::InvalidInput {
                what: "date",
                reason: "ordinal out of range",
            })?;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = yoe + era * 400 + i64::from(m <= 2);
        let year = i32::try_from(y).map_err(|_| FinanceError::InvalidInput {
            what: "date",
            reason: "year out of range",
        })?;
        // m in 1..=12 and d in 1..=31 by construction.
        Self::new(
            year,
            u32::try_from(m).unwrap_or(0),
            u32::try_from(d).unwrap_or(0),
        )
    }

    /// Signed number of days from `earlier` to `self`.
    ///
    /// ```rust
    /// use finance_rs::Date; let (a, b) = (Date::new(2024, 1, 1)?, Date::new(2025, 1, 1)?); assert_eq!(b.days_since(a), 366); assert_eq!(a.days_since(b), -366);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn days_since(self, earlier: Self) -> i64 {
        self.ordinal() - earlier.ordinal()
    }

    /// Shift by whole months, clamping the day to the target month's end
    /// (Jan 31 + 1 month = Feb 28/29).
    ///
    /// ```rust
    /// use finance_rs::Date; assert_eq!(Date::new(2024, 1, 31)?.add_months(1)?, Date::new(2024, 2, 29)?);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// [`FinanceError::InvalidInput`] when the year leaves ±1,000,000.
    #[allow(clippy::cast_possible_truncation)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
    pub fn add_months(self, months: i32) -> Result<Self> {
        let total = i64::from(self.year) * 12 + i64::from(self.month) - 1 + i64::from(months);
        // Date years and the i32 month offset keep this quotient within i32.
        let year = total.div_euclid(12) as i32;
        let month = u32::try_from(total.rem_euclid(12) + 1).unwrap_or(1);
        Self::new(year, month, self.day.min(days_in_month(year, month)))
    }
}

#[path = "date_impl.rs"]
mod date_impl;

#[cfg(test)]
#[path = "date_tests.rs"]
mod tests;
