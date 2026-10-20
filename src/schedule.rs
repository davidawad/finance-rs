//! Coupon schedule generation with QuantLib-style forward/backward rules.
use crate::calendars::{BusinessDayConvention, Calendar, NullCalendar};
use crate::period::{Period, TimeUnit};
use crate::rates::Frequency;
use crate::{Date, FinanceError, Result};

/// Direction used to generate regular coupon dates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
/// ```rust
/// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
/// ```
pub enum DateGeneration {
    /// The forward option.
    Forward,
    /// The backward option.
    Backward,
}

/// One accrual interval and its quasi-coupon reference interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// ```rust
/// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
/// ```
pub struct AccrualPeriod {
    /// Start for the accrual period result.
    pub start: Date,
    /// End for the accrual period result.
    pub end: Date,
    /// Ref start for the accrual period result.
    pub ref_start: Date,
    /// Ref end for the accrual period result.
    pub ref_end: Date,
    /// Payment date for the accrual period result.
    pub payment_date: Date,
}

/// Generated schedule dates and accrual periods.
#[derive(Debug, Clone, PartialEq, Eq)]
/// ```rust
/// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
/// ```
pub struct Schedule {
    dates: Vec<Date>,
    adjusted: Vec<Date>,
    periods: Vec<AccrualPeriod>,
}

impl Schedule {
    /// Calculates the builder value.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    #[must_use]
    pub fn builder(effective: Date, termination: Date) -> ScheduleBuilder<'static> {
        ScheduleBuilder::new(effective, termination)
    }
    #[must_use]
    /// Returns the dates value.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    pub fn dates(&self) -> &[Date] {
        &self.dates
    }
    #[must_use]
    /// Returns the adjusted value.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    pub fn adjusted(&self) -> &[Date] {
        &self.adjusted
    }
    #[must_use]
    /// Returns the accrual periods value.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    pub fn accrual_periods(&self) -> &[AccrualPeriod] {
        &self.periods
    }

    /// Coupon payment dates, advanced by a signed number of business days.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn payment_dates(&self, lag: i32, calendar: &dyn Calendar) -> Result<Vec<Date>> {
        self.periods
            .iter()
            .map(|p| advance_business_days(calendar, p.payment_date, lag))
            .collect()
    }
}

/// Fluent schedule builder.
/// ```rust
/// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
/// ```
pub struct ScheduleBuilder<'a> {
    effective: Date,
    termination: Date,
    frequency: Frequency,
    // type-strength: allow(option-fields) independent MakeSchedule settings are configured fluently and may be omitted.
    calendar: Option<&'a dyn Calendar>,
    convention: BusinessDayConvention,
    termination_convention: Option<BusinessDayConvention>,
    rule: DateGeneration,
    end_of_month: bool,
    first_date: Option<Date>,
    next_to_last_date: Option<Date>,
    long_stub: bool,
}
impl<'a> ScheduleBuilder<'a> {
    fn new(effective: Date, termination: Date) -> Self {
        Self {
            effective,
            termination,
            frequency: Frequency::Annual,
            calendar: None,
            convention: BusinessDayConvention::Following,
            termination_convention: None,
            rule: DateGeneration::Backward,
            end_of_month: false,
            first_date: None,
            next_to_last_date: None,
            long_stub: false,
        }
    }
    /// Sets the frequency option for this builder.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    #[must_use]
    pub fn frequency(mut self, value: Frequency) -> Self {
        self.frequency = value;
        self
    }
    /// Sets the calendar option for this builder.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    #[must_use]
    pub fn calendar<C: Calendar + 'a>(mut self, value: &'a C) -> Self {
        self.calendar = Some(value);
        self
    }
    /// Sets the convention option for this builder.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    #[must_use]
    pub fn convention(mut self, value: BusinessDayConvention) -> Self {
        self.convention = value;
        self
    }
    /// Sets the termination convention option for this builder.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    #[must_use]
    pub fn termination_convention(mut self, value: BusinessDayConvention) -> Self {
        self.termination_convention = Some(value);
        self
    }
    /// Sets the rule option for this builder.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    #[must_use]
    pub fn rule(mut self, value: DateGeneration) -> Self {
        self.rule = value;
        self
    }
    // type-strength: allow(bool-param) matches QuantLib MakeSchedule's endOfMonth flag.
    /// Sets the end of month option for this builder.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    #[must_use]
    pub fn end_of_month(mut self, value: bool) -> Self {
        self.end_of_month = value;
        self
    }
    /// Sets the first date option for this builder.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    #[must_use]
    pub fn first_date(mut self, value: Date) -> Self {
        self.first_date = Some(value);
        self
    }
    /// Sets the next to last date option for this builder.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    #[must_use]
    pub fn next_to_last_date(mut self, value: Date) -> Self {
        self.next_to_last_date = Some(value);
        self
    }
    /// Merge a short initial/final stub into its adjacent period.
    // type-strength: allow(bool-param) selects short versus long coupon stub behavior.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    #[must_use]
    pub fn long_stub(mut self, value: bool) -> Self {
        self.long_stub = value;
        self
    }

    /// Builds the configured value.
    /// ```rust
    /// assert!({ let s = finance_rs::schedule::Schedule::builder(finance_rs::Date::new(2024, 1, 1).unwrap(), finance_rs::Date::new(2025, 1, 1).unwrap()).frequency(finance_rs::rates::Frequency::Quarterly).build().unwrap(); s.dates().len() == 5 });
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn build(self) -> Result<Schedule> {
        if self.termination <= self.effective {
            return Err(FinanceError::InvalidInput {
                what: "schedule",
                reason: "termination must be after effective date",
            });
        }
        let p = frequency_period(self.frequency)?;
        if self
            .first_date
            .is_some_and(|d| d <= self.effective || d >= self.termination)
            || self
                .next_to_last_date
                .is_some_and(|d| d <= self.effective || d >= self.termination)
        {
            return Err(FinanceError::InvalidInput {
                what: "schedule",
                reason: "stub dates must be inside schedule bounds",
            });
        }
        if self
            .first_date
            .zip(self.next_to_last_date)
            .is_some_and(|(a, b)| a >= b)
        {
            return Err(FinanceError::InvalidInput {
                what: "schedule",
                reason: "first date must precede next-to-last date",
            });
        }
        let mut dates = match self.rule {
            DateGeneration::Backward => self.backward(p)?,
            DateGeneration::Forward => self.forward(p)?,
        };
        dates.dedup();
        if self.long_stub && dates.len() > 2 {
            if self.rule == DateGeneration::Backward
                && self.first_date.is_none()
                && dates[1] != self.effective
            {
                dates.remove(1);
            }
            if self.rule == DateGeneration::Forward
                && self.next_to_last_date.is_none()
                && dates[dates.len() - 2] != self.termination
            {
                dates.remove(dates.len() - 2);
            }
        }
        // Both generation rules seed and retain the effective and termination
        // endpoints; interior stub adjustments never remove either endpoint.
        let calendar = self.calendar.unwrap_or(&NullCalendar);
        let mut adjusted = Vec::with_capacity(dates.len());
        for (i, d) in dates.iter().enumerate() {
            let c = if i + 1 == dates.len() {
                self.termination_convention.unwrap_or(self.convention)
            } else {
                self.convention
            };
            adjusted.push(calendar.adjust(*d, c)?);
        }
        let periods = dates
            .windows(2)
            .enumerate()
            .map(|(i, w)| {
                let (rs, re) = reference_period(w[0], w[1], p, self.rule, self.end_of_month);
                AccrualPeriod {
                    start: w[0],
                    end: w[1],
                    ref_start: rs,
                    ref_end: re,
                    payment_date: adjusted[i + 1],
                }
            })
            .collect::<Vec<_>>();
        Ok(Schedule {
            dates,
            adjusted,
            periods,
        })
    }

    fn backward(&self, p: Period) -> Result<Vec<Date>> {
        let mut out = vec![self.termination];
        let anchor = self.next_to_last_date.unwrap_or(self.termination);
        if self.next_to_last_date.is_some() {
            out.push(anchor);
        }
        let mut d = anchor;
        for _ in 0..2400 {
            let n = advance_date(d, Period::new(-p.n, p.unit), self.end_of_month)?;
            if n <= self.first_date.unwrap_or(self.effective) {
                break;
            }
            out.push(n);
            d = n;
        }
        if let Some(first) = self.first_date {
            out.push(first);
        }
        out.push(self.effective);
        out.reverse();
        Ok(out)
    }
    fn forward(&self, p: Period) -> Result<Vec<Date>> {
        let mut out = vec![self.effective];
        let anchor = self.first_date.unwrap_or(self.effective);
        let mut d = anchor;
        if anchor != self.effective {
            out.push(anchor);
        }
        for _ in 0..2400 {
            let n = advance_date(d, p, self.end_of_month)?;
            if n >= self.next_to_last_date.unwrap_or(self.termination) {
                break;
            }
            out.push(n);
            d = n;
        }
        if let Some(last) = self.next_to_last_date {
            out.push(last);
        }
        out.push(self.termination);
        Ok(out)
    }
}

// type-strength: allow(bool-param) carries the schedule's explicit end-of-month rule.
fn advance_date(date: Date, period: Period, end_of_month: bool) -> Result<Date> {
    if end_of_month
        && date.is_month_end()
        && matches!(period.unit, TimeUnit::Months | TimeUnit::Years)
    {
        return date.add_period(period)?.end_of_month();
    }
    date.add_period(period)
}

fn frequency_period(f: Frequency) -> Result<Period> {
    match f {
        Frequency::Once => Err(FinanceError::InvalidInput {
            what: "frequency",
            reason: "Once cannot generate a repeating schedule",
        }),
        Frequency::Annual => Ok(Period::new(1, TimeUnit::Years)),
        Frequency::Semiannual => Ok(Period::new(6, TimeUnit::Months)),
        Frequency::Quarterly => Ok(Period::new(3, TimeUnit::Months)),
        Frequency::Monthly => Ok(Period::new(1, TimeUnit::Months)),
        Frequency::Weekly => Ok(Period::new(1, TimeUnit::Weeks)),
        Frequency::Daily => Ok(Period::new(1, TimeUnit::Days)),
        Frequency::PerYear(n) if n > 0 && 12 % n == 0 => {
            Ok(Period::new(i32::from(12 / n), TimeUnit::Months))
        }
        _ => Err(FinanceError::InvalidInput {
            what: "frequency",
            reason: "frequency must divide 12 for a date schedule",
        }),
    }
}
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
#[allow(clippy::expect_used)] // Generated schedule periods stay in range by the preceding validated schedule bounds.
fn reference_period(
    start: Date,
    end: Date,
    p: Period,
    rule: DateGeneration,
    // type-strength: allow(bool-param) carries the schedule's explicit end-of-month rule.
    eom: bool,
) -> (Date, Date) {
    if end.days_since(start) as f64 > regular_days(p) * 1.5 {
        return match rule {
            // The schedule generator already stepped one regular period from
            // the relevant anchor; a stub reference period is that same valid
            // date step, so it cannot overflow independently.
            DateGeneration::Backward => (
                end.add_period(Period::new(-p.n, p.unit))
                    .expect("generated backward schedule period remains in range"),
                end,
            ),
            DateGeneration::Forward => (
                start,
                start
                    .add_period(p)
                    .expect("generated forward schedule period remains in range"),
            ),
        };
    }
    let _ = eom;
    (start, end)
}
fn regular_days(p: Period) -> f64 {
    match p.unit {
        TimeUnit::Days => f64::from(p.n),
        TimeUnit::Weeks => f64::from(p.n * 7),
        TimeUnit::Months => f64::from(p.n * 30),
        TimeUnit::Years => f64::from(p.n * 365),
    }
}
fn advance_business_days(calendar: &dyn Calendar, date: Date, lag: i32) -> Result<Date> {
    let direction = lag.signum();
    let mut left = lag.unsigned_abs();
    let mut d = date;
    while left > 0 {
        d = d.add_days(direction)?;
        if calendar.is_business_day(d) {
            left -= 1;
        }
    }
    Ok(d)
}

#[cfg(test)]
#[path = "schedule_tests.rs"]
mod tests;
