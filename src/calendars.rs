//! Business-day calendars and date adjustment, with `QuantLib` conventions.
use crate::date::Weekday;
use crate::period::{Period, TimeUnit};
use crate::{Date, FinanceError, Result};

/// Business-day adjustment convention.
///
/// ```rust
/// use finance_rs::calendars::BusinessDayConvention;
/// assert_eq!(BusinessDayConvention::Following, BusinessDayConvention::Following);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub enum BusinessDayConvention {
    /// The following option.
    Following,
    /// The modified following option.
    ModifiedFollowing,
    /// The preceding option.
    Preceding,
    /// The modified preceding option.
    ModifiedPreceding,
    /// The unadjusted option.
    Unadjusted,
    /// The nearest option.
    Nearest,
}

/// The behavior of a date calendar.
///
/// ```rust
/// use finance_rs::calendars::{BusinessDayConvention, Calendar, WeekendsOnly};
/// use finance_rs::Date;
/// let calendar = WeekendsOnly;
/// let saturday = Date::new(2024, 6, 1)?;
/// assert!(!calendar.is_business_day(saturday));
/// assert_eq!(
///     calendar.adjust(saturday, BusinessDayConvention::Following)?,
///     Date::new(2024, 6, 3)?
/// );
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub trait Calendar {
    /// Returns the name value.
    fn name(&self) -> &str;
    /// Calculates the is weekend value.
    fn is_weekend(&self, date: Date) -> bool;
    /// Calculates the is holiday value.
    fn is_holiday(&self, date: Date) -> bool;
    /// Calculates the is business day value.
    fn is_business_day(&self, date: Date) -> bool {
        !self.is_weekend(date) && !self.is_holiday(date)
    }

    /// Calculates the adjust value.
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    fn adjust(&self, date: Date, convention: BusinessDayConvention) -> Result<Date> {
        if self.is_business_day(date) {
            return Ok(date);
        }
        let step = |d: Date, n| d.add_days(n);
        let seek = |start: Date, direction: i32| -> Result<Date> {
            let mut d = start;
            for _ in 0..=14 {
                if self.is_business_day(d) {
                    return Ok(d);
                }
                d = step(d, direction)?;
            }
            Err(FinanceError::InvalidInput {
                what: "calendar",
                reason: "no business day found within 14 days",
            })
        };
        match convention {
            BusinessDayConvention::Following => seek(date, 1),
            BusinessDayConvention::Preceding => seek(date, -1),
            BusinessDayConvention::ModifiedFollowing => {
                let d = seek(date, 1)?;
                if d.month() == date.month() {
                    Ok(d)
                } else {
                    seek(date, -1)
                }
            }
            BusinessDayConvention::ModifiedPreceding => {
                let d = seek(date, -1)?;
                if d.month() == date.month() {
                    Ok(d)
                } else {
                    seek(date, 1)
                }
            }
            BusinessDayConvention::Unadjusted => Ok(date),
            BusinessDayConvention::Nearest => {
                for distance in 1..=14 {
                    let before = step(date, -distance)?;
                    if self.is_business_day(before) {
                        return Ok(before);
                    }
                    let after = step(date, distance)?;
                    if self.is_business_day(after) {
                        return Ok(after);
                    }
                }
                Err(FinanceError::InvalidInput {
                    what: "calendar",
                    reason: "no business day found within 14 days",
                })
            }
        }
    }

    /// Calculates the advance value.
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    fn advance(
        &self,
        date: Date,
        period: Period,
        convention: BusinessDayConvention,
        // type-strength: allow(bool-param) matches QuantLib Calendar::advance endOfMonth.
        end_of_month: bool,
    ) -> Result<Date> {
        if period.n == 0 {
            return self.adjust(date, convention);
        }
        let mut target = date.add_period(period)?;
        if end_of_month
            && matches!(period.unit, TimeUnit::Months | TimeUnit::Years)
            && date.is_month_end()
        {
            target = target.end_of_month()?;
        }
        self.adjust(target, convention)
    }

    /// Signed business-day count. Excluded endpoints are removed after ordering.
    fn business_days_between(
        &self,
        a: Date,
        b: Date,
        // type-strength: allow(bool-param) selects inclusion of the starting endpoint.
        include_first: bool,
        // type-strength: allow(bool-param) selects inclusion of the ending endpoint.
        include_last: bool,
    ) -> i64 {
        if a == b {
            return i64::from((include_first || include_last) && self.is_business_day(a));
        }
        let (lo, hi, sign) = if a < b { (a, b, 1) } else { (b, a, -1) };
        let mut count = 0_i64;
        let mut d = lo;
        while d < hi {
            if self.is_business_day(d) {
                count += 1;
            }
            // Since `d < hi` and `hi` is a valid later Date, the next day is
            // representable and cannot overflow this loop's endpoint range.
            d = d.add_days(1).unwrap_or(hi);
        }
        if include_last && self.is_business_day(hi) {
            count += 1;
        }
        if !include_first && self.is_business_day(lo) {
            count -= 1;
        }
        sign * count
    }
}

/// Saturday/Sunday weekends and no holidays.
///
/// ```rust
/// use finance_rs::calendars::{Calendar, WeekendsOnly};
/// use finance_rs::Date;
/// assert!(!WeekendsOnly.is_business_day(Date::new(2024, 6, 1)?));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct WeekendsOnly;
impl Calendar for WeekendsOnly {
    fn name(&self) -> &'static str {
        "WeekendsOnly"
    }
    fn is_weekend(&self, d: Date) -> bool {
        matches!(d.weekday(), Weekday::Saturday | Weekday::Sunday)
    }
    fn is_holiday(&self, _: Date) -> bool {
        false
    }
}

/// No weekends and no holidays.
///
/// ```rust
/// use finance_rs::calendars::{Calendar, NullCalendar};
/// use finance_rs::Date;
/// assert!(NullCalendar.is_business_day(Date::new(2024, 6, 1)?));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct NullCalendar;
impl Calendar for NullCalendar {
    fn name(&self) -> &'static str {
        "NullCalendar"
    }
    fn is_weekend(&self, _: Date) -> bool {
        false
    }
    fn is_holiday(&self, _: Date) -> bool {
        false
    }
}

/// Supported market identifiers for `Bus252` day counts.
///
/// ```rust
/// use finance_rs::calendars::CalendarId;
/// assert_ne!(CalendarId::WeekendsOnly, CalendarId::NullCalendar);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub enum CalendarId {
    /// The weekends only option.
    WeekendsOnly,
    /// The null calendar option.
    NullCalendar,
    #[cfg(feature = "calendars")]
    /// The united states settlement option.
    UnitedStatesSettlement,
    #[cfg(feature = "calendars")]
    /// The nyse option.
    Nyse,
    #[cfg(feature = "calendars")]
    /// The government bond option.
    GovernmentBond,
    #[cfg(feature = "calendars")]
    /// The federal reserve option.
    FederalReserve,
    #[cfg(feature = "calendars")]
    /// The target option.
    Target,
    #[cfg(feature = "calendars")]
    /// The united kingdom exchange option.
    UnitedKingdomExchange,
    #[cfg(feature = "calendars")]
    /// The japan option.
    Japan,
    #[cfg(feature = "calendars")]
    /// The canada option.
    Canada,
}

/// A value calendar for conventions such as `DayCount::Bus252`.
///
/// ```rust
/// use finance_rs::calendars::{Calendar, CalendarId, CalendarRef};
/// use finance_rs::Date;
/// let calendar = CalendarRef(CalendarId::WeekendsOnly);
/// assert!(!calendar.is_business_day(Date::new(2024, 6, 1)?));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CalendarRef(pub CalendarId);
impl Calendar for CalendarRef {
    fn name(&self) -> &'static str {
        "CalendarRef"
    }
    fn is_weekend(&self, d: Date) -> bool {
        self.0.is_weekend(d)
    }
    fn is_holiday(&self, d: Date) -> bool {
        self.0.is_holiday(d)
    }
}
impl CalendarId {
    fn calendar(self) -> CalendarRef {
        CalendarRef(self)
    }
    fn is_weekend(self, d: Date) -> bool {
        if self == Self::NullCalendar {
            return false;
        }
        #[cfg(feature = "calendars")]
        {
            if self != Self::WeekendsOnly {
                return matches!(d.weekday(), Weekday::Saturday | Weekday::Sunday);
            }
        }
        matches!(d.weekday(), Weekday::Saturday | Weekday::Sunday)
    }
    fn is_holiday(self, d: Date) -> bool {
        #[cfg(feature = "calendars")]
        {
            calendars_market::market_holiday(self, d)
        }
        #[cfg(not(feature = "calendars"))]
        {
            let _ = d;
            false
        }
    }
}

/// Count calendar business days using the selected built-in market rules.
///
/// ```rust
/// use finance_rs::calendars::{business_days, CalendarId};
/// use finance_rs::Date;
/// let friday = Date::new(2024, 5, 31)?;
/// let monday = Date::new(2024, 6, 3)?;
/// assert_eq!(business_days(CalendarId::WeekendsOnly, friday, monday), 1);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[must_use]
pub fn business_days(calendar: CalendarId, start: Date, end: Date) -> i64 {
    calendar
        .calendar()
        .business_days_between(start, end, false, true)
}

/// Combine calendars by taking either their holiday union or business-day intersection.
///
/// ```rust
/// use finance_rs::calendars::{Calendar, JointCalendar, WeekendsOnly};
/// use finance_rs::Date;
/// let weekend = WeekendsOnly;
/// let calendars: [&dyn Calendar; 1] = [&weekend];
/// let joint = JointCalendar::union(calendars)?;
/// assert!(!joint.is_business_day(Date::new(2024, 6, 1)?));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub struct JointCalendar<'a> {
    calendars: Vec<&'a dyn Calendar>,
    rule: JointRule,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JointRule {
    Union,
    Intersection,
}
impl<'a> JointCalendar<'a> {
    /// Combines the supplied calendars by union and returns the result.
    ///
    /// ```rust
    /// use finance_rs::calendars::{Calendar, JointCalendar, WeekendsOnly};
    /// use finance_rs::Date;
    /// let weekend = WeekendsOnly;
    /// let calendars: [&dyn Calendar; 1] = [&weekend];
    /// let joint = JointCalendar::union(calendars)?;
    /// assert!(!joint.is_business_day(Date::new(2024, 6, 1)?));
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn union(calendars: impl IntoIterator<Item = &'a dyn Calendar>) -> Result<Self> {
        Self::new(calendars, JointRule::Union)
    }
    /// Combines the supplied calendars by intersection and returns the result.
    ///
    /// ```rust
    /// use finance_rs::calendars::{Calendar, JointCalendar, WeekendsOnly};
    /// use finance_rs::Date;
    /// let weekend = WeekendsOnly;
    /// let calendars: [&dyn Calendar; 1] = [&weekend];
    /// let joint = JointCalendar::intersection(calendars)?;
    /// assert!(!joint.is_business_day(Date::new(2024, 6, 1)?));
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn intersection(calendars: impl IntoIterator<Item = &'a dyn Calendar>) -> Result<Self> {
        Self::new(calendars, JointRule::Intersection)
    }
    fn new(calendars: impl IntoIterator<Item = &'a dyn Calendar>, rule: JointRule) -> Result<Self> {
        let calendars: Vec<_> = calendars.into_iter().collect();
        if calendars.is_empty() {
            return Err(FinanceError::InvalidInput {
                what: "calendars",
                reason: "at least one calendar is required",
            });
        }
        Ok(Self { calendars, rule })
    }
}
impl Calendar for JointCalendar<'_> {
    fn name(&self) -> &'static str {
        "JointCalendar"
    }
    fn is_weekend(&self, d: Date) -> bool {
        match self.rule {
            JointRule::Union => self.calendars.iter().any(|c| c.is_weekend(d)),
            JointRule::Intersection => self.calendars.iter().all(|c| c.is_weekend(d)),
        }
    }
    fn is_holiday(&self, d: Date) -> bool {
        match self.rule {
            JointRule::Union => self.calendars.iter().any(|c| c.is_holiday(d)),
            JointRule::Intersection => self.calendars.iter().all(|c| c.is_holiday(d)),
        }
    }
    fn is_business_day(&self, d: Date) -> bool {
        match self.rule {
            JointRule::Union => self.calendars.iter().all(|c| c.is_business_day(d)),
            JointRule::Intersection => self.calendars.iter().any(|c| c.is_business_day(d)),
        }
    }
}

#[cfg(feature = "calendars")]
#[path = "calendars_market.rs"]
mod calendars_market;
#[cfg(feature = "calendars")]
/// Market calendar implementations for Canada, Japan, TARGET, the United Kingdom,
/// and the United States.
///
/// ```
/// use finance_rs::calendars::{Calendar, UnitedStates};
/// let us = UnitedStates::Settlement;
/// assert!(us.is_business_day(finance_rs::Date::new(2026, 1, 5)?));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub use calendars_market::{Canada, Japan, Target, UnitedKingdom, UnitedStates};

#[cfg(test)]
#[path = "calendars_tests.rs"]
mod tests;
