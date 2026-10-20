//! Day-count conventions: the year fraction between two dates, with pyxirr
//! 0.10 semantics (which follow the ISDA 2006 definitions without the
//! maturity-date exceptions).
//!
//! ```
//! use finance_rs::{daycount::DayCount, Date};
//! let (a, b) = (Date::new(2020, 1, 31)?, Date::new(2020, 3, 31)?);
//! assert_eq!(DayCount::Thirty360Us.year_fraction(a, b), 60.0 / 360.0);
//! assert_eq!(DayCount::Act365F.year_fraction(a, b), 60.0 / 365.0);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

#[cfg(feature = "calendars")]
use crate::calendars::CalendarId;
use crate::date::{is_leap_year, Date};

/// A day-count convention.
///
/// ```rust
/// use finance_rs::daycount::DayCount;
/// assert_eq!(DayCount::Act365F.name(), "ACT/365F");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub enum DayCount {
    /// ACT/365 Fixed: actual days / 365 (the XIRR default).
    #[default]
    Act365F,
    /// ACT/360: actual days / 360 (money markets).
    Act360,
    /// ACT/364: actual days / 364.
    Act364,
    /// ACT/365.25: actual days / 365.25.
    Act36525,
    /// ACT/ACT ISDA: days in leap years / 366 + days in other years / 365.
    ActActIsda,
    /// NL/360: actual days excluding each Feb 29 in `[start, end]`, / 360.
    /// As in pyxirr, a reversed period (end before start) is not leap-adjusted.
    Nl360,
    /// NL/365: actual days excluding each Feb 29 in `[start, end]`, / 365.
    /// As in pyxirr, a reversed period is not leap-adjusted.
    Nl365,
    /// 30/360 US (NASD, pyxirr `30U/360`): end-of-February and day-31 rules.
    Thirty360Us,
    /// 30/360 ISDA (bond basis): D1 31→30; D2 31→30 only when D1 ≥ 30.
    Thirty360Isda,
    /// 30E/360 (Eurobond basis): any 31st becomes the 30th.
    Thirty360Eu,
    /// 30E/360 ISDA: any month-end (including end of February) becomes the 30th.
    Thirty360EuIsda,
    /// 30E+/360: D1 31→30; D2 31 rolls to the 1st of the next month.
    Thirty360EuPlus,
    /// ACT/366: actual days / 366.
    Act366,
    /// 30/365: 30/360 numerator / 365.
    Thirty365,
    /// ACT/ACT ICMA; use `year_fraction_ref` with reference dates and frequency.
    ActActIcma,
    /// ACT/ACT AFB (Euro).
    ActActAfb,
    /// Brazilian business days / 252.
    #[cfg(feature = "calendars")]
    Bus252(CalendarId),
}

impl DayCount {
    /// Every convention, for iteration.
    ///
    /// ```rust
    /// use finance_rs::daycount::DayCount;
    /// assert_eq!(DayCount::ALL.len(), 16);
    /// assert!(DayCount::ALL.contains(&DayCount::Act360));
    /// ```
    pub const ALL: [Self; 16] = [
        Self::Act365F,
        Self::Act360,
        Self::Act364,
        Self::Act36525,
        Self::ActActIsda,
        Self::Nl360,
        Self::Nl365,
        Self::Thirty360Us,
        Self::Thirty360Isda,
        Self::Thirty360Eu,
        Self::Thirty360EuIsda,
        Self::Thirty360EuPlus,
        Self::Act366,
        Self::Thirty365,
        Self::ActActIcma,
        Self::ActActAfb,
    ];

    /// All conventions including the feature-gated calendar basis.
    ///
    /// ```rust
    /// # #[cfg(feature = "calendars")]
    /// # {
    /// use finance_rs::daycount::DayCount;
    /// assert!(DayCount::ALL_WITH_CALENDARS.contains(&DayCount::Act360));
    /// # }
    /// ```
    #[cfg(feature = "calendars")]
    pub const ALL_WITH_CALENDARS: [Self; 17] = [
        Self::Act365F,
        Self::Act360,
        Self::Act364,
        Self::Act36525,
        Self::ActActIsda,
        Self::Nl360,
        Self::Nl365,
        Self::Thirty360Us,
        Self::Thirty360Isda,
        Self::Thirty360Eu,
        Self::Thirty360EuIsda,
        Self::Thirty360EuPlus,
        Self::Act366,
        Self::Thirty365,
        Self::ActActIcma,
        Self::ActActAfb,
        Self::Bus252(CalendarId::WeekendsOnly),
    ];

    /// The pyxirr / market name (`"ACT/365F"`, `"30U/360"`, ...).
    ///
    /// ```rust
    /// use finance_rs::daycount::DayCount;
    /// assert_eq!(DayCount::ActActIsda.name(), "ACT/ACT ISDA");
    /// ```
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Act365F => "ACT/365F",
            Self::Act360 => "ACT/360",
            Self::Act364 => "ACT/364",
            Self::Act36525 => "ACT/365.25",
            Self::ActActIsda => "ACT/ACT ISDA",
            Self::Nl360 => "NL/360",
            Self::Nl365 => "NL/365",
            Self::Thirty360Us => "30U/360",
            Self::Thirty360Isda => "30/360 ISDA",
            Self::Thirty360Eu => "30E/360",
            Self::Thirty360EuIsda => "30E/360 ISDA",
            Self::Thirty360EuPlus => "30E+/360",
            Self::Act366 => "ACT/366",
            Self::Thirty365 => "30/365",
            Self::ActActIcma => "ACT/ACT ICMA",
            Self::ActActAfb => "ACT/ACT AFB",
            #[cfg(feature = "calendars")]
            Self::Bus252(_) => "BUS/252",
        }
    }

    /// Day count numerator between `start` and `end` under this convention
    /// (signed; negative when `end` precedes `start`).
    ///
    /// ```rust
    /// use finance_rs::{daycount::DayCount, Date};
    /// let (a, b) = (Date::new(2020, 2, 28)?, Date::new(2020, 3, 1)?);
    /// assert_eq!(DayCount::Act360.days(a, b), 2);
    /// assert_eq!(DayCount::Nl365.days(a, b), 1);
    /// assert_eq!(DayCount::Thirty360Eu.days(a, b), 3);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn days(self, start: Date, end: Date) -> i64 {
        match self {
            Self::Act365F
            | Self::Act360
            | Self::Act364
            | Self::Act36525
            | Self::ActActIsda
            | Self::Act366
            | Self::ActActIcma
            | Self::ActActAfb => end.days_since(start),
            Self::Thirty365 => self.thirty_360_days(start, end),
            #[cfg(feature = "calendars")]
            Self::Bus252(calendar) => crate::calendars::business_days(calendar, start, end),
            // pyxirr leap-adjusts forward periods only.
            Self::Nl360 | Self::Nl365 if end < start => end.days_since(start),
            Self::Nl360 | Self::Nl365 => {
                end.days_since(start) - (leap_days_through(end) - leap_days_before(start))
            }
            Self::Thirty360Us
            | Self::Thirty360Isda
            | Self::Thirty360Eu
            | Self::Thirty360EuIsda
            | Self::Thirty360EuPlus => self.thirty_360_days(start, end),
        }
    }

    /// Year fraction from `start` to `end` (negative when `end` precedes `start`).
    ///
    /// ```text
    /// ACT/xxx       (end − start) / xxx
    /// ACT/ACT ISDA  Σ over calendar years: days in that year / (365 or 366)
    /// 30/360 family (360·ΔY + 30·ΔM + ΔD) / 360, after each rule's day adjustments
    /// ```
    ///
    /// ```rust
    /// use finance_rs::{daycount::DayCount, Date};
    /// let (a, b) = (Date::new(2019, 12, 1)?, Date::new(2020, 2, 1)?);
    /// let f = DayCount::ActActIsda.year_fraction(a, b);
    /// assert!((f - (31.0 / 365.0 + 31.0 / 366.0)).abs() < 1e-15);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn year_fraction(self, start: Date, end: Date) -> f64 {
        let days = self.days(start, end) as f64;
        match self {
            Self::Act365F | Self::Nl365 => days / 365.0,
            Self::Act366 => days / 366.0,
            Self::Thirty365 => days / 365.0,
            Self::Act364 => days / 364.0,
            Self::Act36525 => days / 365.25,
            Self::ActActIsda => act_act_isda(start, end),
            Self::Act360
            | Self::Nl360
            | Self::Thirty360Us
            | Self::Thirty360Isda
            | Self::Thirty360Eu
            | Self::Thirty360EuIsda
            | Self::Thirty360EuPlus => days / 360.0,
            Self::ActActIcma | Self::ActActAfb => self.year_fraction_ref_default(start, end),
            #[cfg(feature = "calendars")]
            Self::Bus252(_) => days / 252.0,
        }
    }

    /// ACT/ACT ICMA with an explicit quasi-coupon reference period and payments per year.
    ///
    /// ```rust
    /// use finance_rs::{daycount::DayCount, Date};
    /// let (a, b) = (Date::new(2024, 1, 1)?, Date::new(2024, 4, 1)?);
    /// let fraction = DayCount::ActActIcma.year_fraction_ref(a, b, a, b, 4);
    /// assert!((fraction - 0.25).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[allow(clippy::cast_precision_loss)]
    #[must_use]
    pub fn year_fraction_ref(
        self,
        start: Date,
        end: Date,
        ref_start: Date,
        ref_end: Date,
        freq: u16,
    ) -> f64 {
        if self != Self::ActActIcma || freq == 0 || ref_end <= ref_start {
            return f64::NAN;
        }
        if end < start {
            return -self.year_fraction_ref(end, start, ref_start, ref_end, freq);
        }
        let months = 12 / i32::from(freq);
        if months == 0 {
            return f64::NAN;
        }
        let mut rs = ref_start;
        let mut re = ref_end;
        for _ in 0..100_000 {
            if rs <= start {
                break;
            }
            re = rs;
            rs = match rs.add_months(-months) {
                Ok(d) => d,
                Err(_) => return f64::NAN,
            };
        }
        let mut cursor = start;
        let mut total = 0.0;
        for _ in 0..100_000 {
            if cursor >= end {
                return total;
            }
            if cursor >= re {
                rs = re;
                re = match re.add_months(months) {
                    Ok(d) => d,
                    Err(_) => return f64::NAN,
                };
                continue;
            }
            let stop = if end < re { end } else { re };
            total += cursor.days_since(stop).unsigned_abs() as f64
                / (re.days_since(rs) as f64 * f64::from(freq));
            cursor = stop;
        }
        f64::NAN
    }

    #[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
    fn year_fraction_ref_default(self, start: Date, end: Date) -> f64 {
        match self {
            Self::ActActAfb => act_act_afb(start, end),
            _ => end.days_since(start) as f64 / 365.0,
        }
    }

    /// Parse a [`DayCount::name`] (case-insensitive).
    ///
    /// ```rust
    /// use finance_rs::daycount::DayCount;
    /// assert_eq!(DayCount::from_name("30u/360"), Some(DayCount::Thirty360Us));
    /// assert_eq!(DayCount::from_name("ACT/366"), Some(DayCount::Act366));
    /// ```
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|dc| dc.name().eq_ignore_ascii_case(name.trim()))
    }

    fn thirty_360_days(self, start: Date, end: Date) -> i64 {
        let (y1, m1, mut d1) = (start.year(), start.month(), start.day());
        let (mut y2, mut m2, mut d2) = (end.year(), end.month(), end.day());
        let feb_end = |d: Date| d.month() == 2 && d.is_month_end();
        match self {
            Self::Thirty360Us => {
                if feb_end(start) && feb_end(end) {
                    d2 = 30;
                }
                if feb_end(start) {
                    d1 = 30;
                }
                if d2 == 31 && d1 >= 30 {
                    d2 = 30;
                }
                if d1 == 31 {
                    d1 = 30;
                }
            }
            Self::Thirty360Isda => {
                if d1 == 31 {
                    d1 = 30;
                }
                if d2 == 31 && d1 == 30 {
                    d2 = 30;
                }
            }
            Self::Thirty360Eu | Self::Thirty365 => {
                d1 = d1.min(30);
                d2 = d2.min(30);
            }
            Self::Thirty360EuIsda => {
                if start.is_month_end() {
                    d1 = 30;
                }
                if end.is_month_end() {
                    d2 = 30;
                }
            }
            _ => {
                // 30E+/360
                d1 = d1.min(30);
                if d2 == 31 {
                    d2 = 1;
                    m2 += 1;
                    if m2 > 12 {
                        m2 = 1;
                        y2 += 1;
                    }
                }
            }
        }
        360 * (i64::from(y2) - i64::from(y1))
            + 30 * (i64::from(m2) - i64::from(m1))
            + (i64::from(d2) - i64::from(d1))
    }
}

/// ACT/ACT ISDA year fraction, signed.
#[allow(clippy::cast_precision_loss)]
fn act_act_isda(start: Date, end: Date) -> f64 {
    if end < start {
        return -act_act_isda(end, start);
    }
    let basis = |y: i32| if is_leap_year(y) { 366.0 } else { 365.0 };
    let (y1, y2) = (start.year(), end.year());
    if y1 == y2 {
        return end.days_since(start) as f64 / basis(y1);
    }
    let jan1 = |y: i32| Date::new(y, 1, 1).map_or(0, Date::ordinal);
    let head = (jan1(y1 + 1) - start.ordinal()) as f64 / basis(y1);
    let tail = (end.ordinal() - jan1(y2)) as f64 / basis(y2);
    head + f64::from(y2 - y1 - 1) + tail
}

/// Feb 29s in years before `d`'s, counted from a fixed epoch (only
/// differences are meaningful).
fn leaps_before_year(d: Date) -> i64 {
    let y = i64::from(d.year()) - 1;
    y.div_euclid(4) - y.div_euclid(100) + y.div_euclid(400)
}

/// Feb 29s strictly before `d` (same epoch as [`leap_days_through`]).
fn leap_days_before(d: Date) -> i64 {
    leaps_before_year(d) + i64::from(is_leap_year(d.year()) && d.month() > 2)
}

/// Feb 29s on or before `d`, so `through(end) − before(start)` counts the
/// Feb 29s in `[start, end]`, both ends included (pyxirr).
fn leap_days_through(d: Date) -> i64 {
    leaps_before_year(d)
        + i64::from(is_leap_year(d.year()) && (d.month() > 2 || (d.month() == 2 && d.day() == 29)))
}

/// AFB/Euro: count complete years backwards from the later date, then divide
/// the remaining days by 366 only when the remainder contains February 29.
#[allow(clippy::cast_precision_loss)]
fn act_act_afb(start: Date, end: Date) -> f64 {
    if end < start {
        return -act_act_afb(end, start);
    }
    let mut anchor = end;
    let mut years = 0_i64;
    while let Ok(prior) = anchor.add_years(-1) {
        if prior < start {
            break;
        }
        years += 1;
        anchor = prior;
    }
    let days = anchor.days_since(start);
    let leap = (start.year()..=anchor.year())
        .any(|y| is_leap_year(y) && Date::new(y, 2, 29).is_ok_and(|d| d >= start && d < anchor));
    years as f64 + days as f64 / if leap { 366.0 } else { 365.0 }
}

#[cfg(test)]
#[path = "daycount_tests.rs"]
mod tests;
