//! Rule-based market calendars; the feature avoids storing holiday tables.
use super::{Calendar, CalendarId};
use crate::date::MARKET_CALENDAR_FALLBACK;
use crate::{Date, Weekday};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UsMarket {
    Settlement,
    Nyse,
    GovernmentBond,
    FederalReserve,
}
/// US market calendar variants.
///
/// ```rust
/// use finance_rs::calendars::{Calendar, UnitedStates}; use finance_rs::Date; assert!(UnitedStates::Settlement.is_holiday(Date::new(2024, 7, 4)?));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, Copy)]
pub struct UnitedStates {
    market: UsMarket,
}
#[allow(non_upper_case_globals)]
impl UnitedStates {
    /// Standard settlement configuration.
    ///
    /// ```rust
    /// use finance_rs::calendars::{Calendar, UnitedStates}; use finance_rs::Date; assert!(UnitedStates::Settlement.is_holiday(Date::new(2024, 7, 4)?));
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    pub const Settlement: Self = Self {
        market: UsMarket::Settlement,
    };
    /// Standard nyse configuration.
    ///
    /// ```rust
    /// use finance_rs::calendars::{Calendar, UnitedStates}; use finance_rs::Date; assert!(UnitedStates::Nyse.is_holiday(Date::new(2024, 7, 4)?));
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    pub const Nyse: Self = Self {
        market: UsMarket::Nyse,
    };
    /// Standard government bond configuration.
    ///
    /// ```rust
    /// use finance_rs::calendars::{Calendar, UnitedStates}; use finance_rs::Date; assert!(UnitedStates::GovernmentBond.is_holiday(Date::new(2024, 7, 4)?));
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    pub const GovernmentBond: Self = Self {
        market: UsMarket::GovernmentBond,
    };
    /// Standard federal reserve configuration.
    ///
    /// ```rust
    /// use finance_rs::calendars::{Calendar, UnitedStates}; use finance_rs::Date; assert!(UnitedStates::FederalReserve.is_holiday(Date::new(2024, 7, 4)?));
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    pub const FederalReserve: Self = Self {
        market: UsMarket::FederalReserve,
    };
}
impl Calendar for UnitedStates {
    fn name(&self) -> &'static str {
        "UnitedStates"
    }
    fn is_weekend(&self, d: Date) -> bool {
        weekend(d)
    }
    fn is_holiday(&self, d: Date) -> bool {
        us_holiday(d, self.market)
    }
}

#[derive(Debug, Clone, Copy, Default)]
/// Data structure for target.
///
/// ```rust
/// use finance_rs::calendars::{Calendar, Target}; use finance_rs::Date; assert!(Target.is_holiday(Date::new(2024, 5, 1)?));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub struct Target;
impl Calendar for Target {
    fn name(&self) -> &'static str {
        "TARGET"
    }
    fn is_weekend(&self, d: Date) -> bool {
        weekend(d)
    }
    fn is_holiday(&self, d: Date) -> bool {
        let easter = easter_sunday(d.year());
        same(d, d.year(), 1, 1)
            || same(d, d.year(), 5, 1)
            || same(d, d.year(), 12, 25)
            || same(d, d.year(), 12, 26)
            || d == easter.add_days(-2).unwrap_or(d)
            || d == easter.add_days(1).unwrap_or(d)
            || (d.month() == 12 && d.day() == 31 && matches!(d.year(), 1998 | 1999 | 2001))
    }
}

#[derive(Debug, Clone, Copy, Default)]
/// Data structure for united kingdom.
///
/// ```rust
/// use finance_rs::calendars::{Calendar, UnitedKingdom}; use finance_rs::Date; assert!(UnitedKingdom::Exchange.is_holiday(Date::new(2024, 12, 25)?));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub struct UnitedKingdom;
#[allow(non_upper_case_globals)]
impl UnitedKingdom {
    /// Standard exchange configuration.
    ///
    /// ```rust
    /// use finance_rs::calendars::{Calendar, UnitedKingdom}; use finance_rs::Date; assert!(UnitedKingdom::Exchange.is_holiday(Date::new(2024, 12, 25)?));
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    pub const Exchange: Self = Self;
}
impl Calendar for UnitedKingdom {
    fn name(&self) -> &'static str {
        "UnitedKingdomExchange"
    }
    fn is_weekend(&self, d: Date) -> bool {
        weekend(d)
    }
    fn is_holiday(&self, d: Date) -> bool {
        let e = easter_sunday(d.year());
        d == observed(d.year(), 1, 1)
            || d == e.add_days(-2).unwrap_or(d)
            || d == e.add_days(1).unwrap_or(d)
            || d == last_monday(d.year(), 5)
            || d == last_monday(d.year(), 8)
            || d == observed(d.year(), 12, 25)
            || d == observed(d.year(), 12, 26)
            || (d.year() == 1999 && d.month() == 12 && d.day() == 31)
    }
}

#[derive(Debug, Clone, Copy, Default)]
/// Data structure for japan.
///
/// ```rust
/// use finance_rs::calendars::{Calendar, Japan}; use finance_rs::Date; assert!(Japan.is_holiday(Date::new(2024, 5, 3)?));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub struct Japan;
impl Calendar for Japan {
    fn name(&self) -> &'static str {
        "Japan"
    }
    fn is_weekend(&self, d: Date) -> bool {
        weekend(d)
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
    fn is_holiday(&self, d: Date) -> bool {
        let y = d.year();
        let fixed = [
            (1, 1),
            (2, 11),
            (2, 23),
            (4, 29),
            (5, 3),
            (5, 4),
            (5, 5),
            (8, 11),
            (11, 3),
            (11, 23),
        ];
        if fixed.iter().any(|&(m, day)| same(d, y, m, day))
            || matches!((d.month(),d.day()), (1,2..=3) if y >= 1973)
            || (y >= 1973 && same(d, y, 12, 31))
        {
            return true;
        }
        let spring = ((20.8431 + 0.242_194 * f64::from(y - 1980) - f64::from((y - 1980) / 4))
            .floor()) as u32;
        let autumn = ((23.2488 + 0.242_194 * f64::from(y - 1980) - f64::from((y - 1980) / 4))
            .floor()) as u32;
        same(d, y, 3, spring)
            || same(d, y, 9, autumn)
            || (y >= 2003 && d == nth_monday(y, 1, 2))
            || (y >= 2000 && d == nth_monday(y, 7, 3))
            || (y >= 2016 && d == nth_monday(y, 8, 3))
            || (y >= 2003 && d == nth_monday(y, 10, 2))
            || d == nth_monday(y, 1, 2)
            || d == nth_monday(y, 7, 3)
            || d == nth_monday(y, 10, 2)
    }
}

#[derive(Debug, Clone, Copy, Default)]
/// Data structure for canada.
///
/// ```rust
/// use finance_rs::calendars::{Calendar, Canada}; use finance_rs::Date; assert!(Canada.is_holiday(Date::new(2024, 7, 1)?));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub struct Canada;
impl Calendar for Canada {
    fn name(&self) -> &'static str {
        "Canada"
    }
    fn is_weekend(&self, d: Date) -> bool {
        weekend(d)
    }
    fn is_holiday(&self, d: Date) -> bool {
        let y = d.year();
        let e = easter_sunday(y);
        d == observed(y, 1, 1)
            || d == e.add_days(-2).unwrap_or(d)
            || d == third_monday(y, 2)
            || d == observed(y, 7, 1)
            || d == first_monday(y, 8)
            || d == first_monday(y, 9)
            || d == second_monday(y, 10)
            || d == observed(y, 11, 11)
            || d == observed(y, 12, 25)
            || d == observed(y, 12, 26)
    }
}

pub(super) fn market_holiday(id: CalendarId, d: Date) -> bool {
    match id {
        CalendarId::WeekendsOnly | CalendarId::NullCalendar => false,
        CalendarId::UnitedStatesSettlement => us_holiday(d, UsMarket::Settlement),
        CalendarId::Nyse => us_holiday(d, UsMarket::Nyse),
        CalendarId::GovernmentBond => us_holiday(d, UsMarket::GovernmentBond),
        CalendarId::FederalReserve => us_holiday(d, UsMarket::FederalReserve),
        CalendarId::Target => Target.is_holiday(d),
        CalendarId::UnitedKingdomExchange => UnitedKingdom.is_holiday(d),
        CalendarId::Japan => Japan.is_holiday(d),
        CalendarId::Canada => Canada.is_holiday(d),
    }
}
fn us_holiday(d: Date, market: UsMarket) -> bool {
    let y = d.year();
    let e = easter_sunday(y);
    if market == UsMarket::Nyse && nyse_historical_closure(d) {
        return true;
    }
    let new_year = d == observed(y, 1, 1) || d == observed(y + 1, 1, 1);
    let gov_new_year = d == no_saturday_observed(y, 1, 1) || d == no_saturday_observed(y + 1, 1, 1);
    let mlk = y >= if market == UsMarket::Nyse { 1998 } else { 1983 } && d == third_monday(y, 1);
    let washington = d == third_monday(y, 2);
    let memorial = d == last_monday(y, 5);
    let juneteenth = y >= 2022 && d == observed(y, 6, 19);
    let independence = d == observed(y, 7, 4);
    let labor = d == first_monday(y, 9);
    let columbus = d == second_monday(y, 10);
    let veterans = d == observed(y, 11, 11);
    let thanksgiving = d == fourth_thursday(y, 11);
    let christmas = d == observed(y, 12, 25);
    let good_friday = d == e.add_days(-2).unwrap_or(d);
    if market == UsMarket::GovernmentBond
        && matches!(
            d.to_string().as_str(),
            "2004-06-11" | "2012-10-30" | "2018-12-05"
        )
    {
        return true;
    }
    match market {
        UsMarket::Settlement | UsMarket::FederalReserve => {
            new_year
                || mlk
                || washington
                || memorial
                || juneteenth
                || independence
                || labor
                || columbus
                || veterans
                || thanksgiving
                || christmas
        }
        UsMarket::Nyse => {
            let ny_new_year = d == nyse_observed_new_year(y) || d == nyse_observed_new_year(y + 1);
            ny_new_year
                || mlk
                || washington
                || memorial
                || juneteenth
                || independence
                || labor
                || thanksgiving
                || christmas
                || good_friday
        }
        UsMarket::GovernmentBond => {
            gov_new_year
                || (y >= 1983 && d == third_monday(y, 1))
                || washington
                || memorial
                || (y >= 2022 && d == no_saturday_observed(y, 6, 19))
                || d == no_saturday_observed(y, 7, 4)
                || labor
                || columbus
                || d == no_saturday_observed(y, 11, 11)
                || thanksgiving
                || d == no_saturday_observed(y, 12, 25)
        }
    }
}

/// NYSE-specific dates from `QuantLib`'s historical-closing rules.
fn nyse_historical_closure(d: Date) -> bool {
    let (y, m, day) = (d.year(), d.month(), d.day());
    let election_day = (y <= 1968 || (y <= 1980 && y % 4 == 0))
        && m == 11
        && day <= 7
        && d.weekday() == Weekday::Tuesday;
    let paperwork_wednesday = y == 1968
        && d.weekday() == Weekday::Wednesday
        && Date::new(y, 1, 1).is_ok_and(|new_year| d.ordinal() - new_year.ordinal() + 1 >= 163);
    election_day
        || paperwork_wednesday
        || matches!(
            (y, m, day),
            (2025, 1, 9)
                | (2018, 12, 5)
                | (2012, 10, 29..=30)
                | (2007, 1, 2)
                | (2004, 6, 11)
                | (2001, 9, 11..=14)
                | (1994, 4, 27)
                | (1985, 9, 27)
                | (1977, 7, 14)
                | (1973, 1, 25)
                | (1972, 12, 28)
                | (1969, 7, 21)
                | (1969, 3, 31)
                | (1969, 2, 10)
                | (1968, 7, 5)
                | (1968, 4, 9)
                | (1963, 11, 25)
                | (1961, 5, 29)
                | (1958, 12, 26)
                | (1954 | 1956 | 1965, 12, 24)
        )
}

fn same(d: Date, y: i32, m: u32, day: u32) -> bool {
    d.year() == y && d.month() == m && d.day() == day
}
fn weekend(d: Date) -> bool {
    matches!(d.weekday(), Weekday::Saturday | Weekday::Sunday)
}
fn observed(y: i32, m: u32, day: u32) -> Date {
    let d = Date::new(y, m, day).unwrap_or(MARKET_CALENDAR_FALLBACK);
    match d.weekday() {
        Weekday::Saturday => d.add_days(-1).unwrap_or(d),
        Weekday::Sunday => d.add_days(1).unwrap_or(d),
        _ => d,
    }
}
fn nyse_observed_new_year(y: i32) -> Date {
    let d = Date::new(y, 1, 1).unwrap_or(MARKET_CALENDAR_FALLBACK);
    if d.weekday() == Weekday::Sunday {
        d.add_days(1).unwrap_or(d)
    } else {
        d
    }
}
fn no_saturday_observed(y: i32, m: u32, day: u32) -> Date {
    let d = Date::new(y, m, day).unwrap_or(MARKET_CALENDAR_FALLBACK);
    if d.weekday() == Weekday::Sunday {
        d.add_days(1).unwrap_or(d)
    } else {
        d
    }
}
fn nth_monday(y: i32, m: u32, n: i32) -> Date {
    Date::nth_weekday(y, m, Weekday::Monday, n).unwrap_or(observed(y, m, 1))
}
fn third_monday(y: i32, m: u32) -> Date {
    nth_monday(y, m, 3)
}
fn second_monday(y: i32, m: u32) -> Date {
    nth_monday(y, m, 2)
}
fn first_monday(y: i32, m: u32) -> Date {
    nth_monday(y, m, 1)
}
fn last_monday(y: i32, m: u32) -> Date {
    Date::last_weekday(y, m, Weekday::Monday).unwrap_or(observed(y, m, 1))
}
fn fourth_thursday(y: i32, m: u32) -> Date {
    Date::nth_weekday(y, m, Weekday::Thursday, 4).unwrap_or(observed(y, m, 1))
}
fn easter_sunday(y: i32) -> Date {
    Date::easter_monday(y)
        .and_then(|d| d.add_days(-1))
        .unwrap_or(observed(y, 4, 1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> crate::Result<Date> {
        Date::new(year, month, day)
    }

    #[test]
    fn market_calendars_cover_named_rules_and_calendar_ids() -> crate::Result<()> {
        let new_year = date(2024, 1, 1)?;
        let sunday = date(2024, 1, 7)?;
        assert_eq!(UnitedStates::Settlement.name(), "UnitedStates");
        assert!(UnitedStates::Settlement.is_holiday(new_year));
        assert!(UnitedStates::Nyse.is_holiday(date(2024, 3, 29)?));
        assert!(UnitedStates::GovernmentBond.is_holiday(date(2024, 6, 19)?));
        assert!(UnitedStates::FederalReserve.is_holiday(date(2024, 1, 15)?));
        assert!(UnitedStates::Nyse.is_weekend(sunday));

        assert_eq!(Target.name(), "TARGET");
        assert!(Target.is_weekend(sunday));
        assert!(Target.is_holiday(date(2024, 5, 1)?));
        assert!(Target.is_holiday(date(2024, 3, 29)?));
        assert!(Target.is_holiday(date(2024, 4, 1)?));
        assert_eq!(UnitedKingdom::Exchange.name(), "UnitedKingdomExchange");
        assert!(UnitedKingdom::Exchange.is_weekend(sunday));
        assert!(UnitedKingdom::Exchange.is_holiday(date(2024, 12, 25)?));
        assert!(UnitedKingdom::Exchange.is_holiday(date(2024, 12, 26)?));
        assert!(UnitedKingdom::Exchange.is_holiday(date(2024, 5, 27)?));

        assert_eq!(Japan.name(), "Japan");
        assert!(Japan.is_weekend(sunday));
        assert!(Japan.is_holiday(date(2024, 1, 2)?));
        assert!(Japan.is_holiday(date(2024, 3, 20)?));
        assert!(Japan.is_holiday(date(2024, 9, 22)?));
        assert!(Japan.is_holiday(date(2024, 7, 15)?));
        assert!(!Japan.is_holiday(date(1960, 1, 2)?));

        assert_eq!(Canada.name(), "Canada");
        assert!(Canada.is_weekend(sunday));
        assert!(Canada.is_holiday(date(2024, 7, 1)?));
        assert!(Canada.is_holiday(date(2024, 2, 19)?));
        assert!(!Canada.is_holiday(date(2024, 7, 2)?));
        assert!(CalendarId::UnitedStatesSettlement.is_holiday(new_year));
        assert!(UnitedStates::Nyse.is_holiday(date(2001, 9, 11)?));
        assert!(UnitedStates::GovernmentBond.is_holiday(date(2004, 6, 11)?));
        for holiday in [
            (2024, 7, 4),
            (2024, 9, 2),
            (2024, 10, 14),
            (2024, 11, 11),
            (2024, 11, 28),
            (2024, 12, 25),
        ] {
            assert!(UnitedStates::GovernmentBond.is_holiday(date(holiday.0, holiday.1, holiday.2)?));
            assert!(UnitedStates::FederalReserve.is_holiday(date(holiday.0, holiday.1, holiday.2)?));
        }
        assert!(CalendarId::Nyse.is_holiday(date(2024, 3, 29)?));
        assert!(CalendarId::Target.is_holiday(date(2024, 5, 1)?));
        assert_eq!(
            super::super::business_days(
                CalendarId::UnitedStatesSettlement,
                new_year,
                date(2024, 1, 3)?
            ),
            2
        );
        Ok(())
    }

    #[test]
    fn observed_dates_and_weekday_helpers_are_consistent() -> crate::Result<()> {
        assert_eq!(observed(2022, 1, 1), date(2021, 12, 31)?);
        assert_eq!(observed(2022, 1, 2), date(2022, 1, 3)?);
        assert_eq!(nyse_observed_new_year(2023), date(2023, 1, 2)?);
        assert_eq!(no_saturday_observed(2022, 7, 4), date(2022, 7, 4)?);
        assert_eq!(no_saturday_observed(2022, 1, 2), date(2022, 1, 3)?);
        assert_eq!(nth_monday(2024, 1, 3), date(2024, 1, 15)?);
        assert_eq!(third_monday(2024, 1), date(2024, 1, 15)?);
        assert_eq!(second_monday(2024, 1), date(2024, 1, 8)?);
        assert_eq!(first_monday(2024, 1), date(2024, 1, 1)?);
        assert_eq!(last_monday(2024, 5), date(2024, 5, 27)?);
        assert_eq!(fourth_thursday(2024, 11), date(2024, 11, 28)?);
        assert_eq!(easter_sunday(2024), date(2024, 3, 31)?);
        assert!(!same(date(2024, 1, 1)?, 2024, 1, 2));
        Ok(())
    }
}
