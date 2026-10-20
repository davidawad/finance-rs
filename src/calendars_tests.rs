use super::*;

fn date(year: i32, month: u32, day: u32) -> Result<Date> {
    Date::new(year, month, day)
}

#[test]
fn adjustment_conventions_and_business_day_counts() -> Result<()> {
    let calendar = WeekendsOnly;
    let friday = date(2024, 5, 31)?;
    let saturday = date(2024, 6, 1)?;
    let monday = date(2024, 6, 3)?;
    assert_eq!(calendar.name(), "WeekendsOnly");
    assert!(calendar.is_weekend(saturday));
    assert!(!calendar.is_business_day(saturday));
    assert_eq!(
        calendar.adjust(saturday, BusinessDayConvention::Unadjusted)?,
        saturday
    );
    assert_eq!(
        calendar.adjust(saturday, BusinessDayConvention::Following)?,
        monday
    );
    assert_eq!(
        calendar.adjust(saturday, BusinessDayConvention::Preceding)?,
        friday
    );
    assert_eq!(
        calendar.adjust(saturday, BusinessDayConvention::ModifiedFollowing)?,
        monday
    );
    assert_eq!(
        calendar.adjust(date(2024, 8, 31)?, BusinessDayConvention::ModifiedFollowing)?,
        date(2024, 8, 30)?
    );
    assert_eq!(
        calendar.adjust(monday, BusinessDayConvention::ModifiedPreceding)?,
        monday
    );
    assert_eq!(
        calendar.adjust(saturday, BusinessDayConvention::Nearest)?,
        friday
    );
    assert_eq!(
        calendar.business_days_between(friday, monday, true, true),
        2
    );
    assert_eq!(
        calendar.business_days_between(monday, friday, true, true),
        -2
    );
    assert_eq!(
        calendar.business_days_between(monday, monday, false, false),
        0
    );
    assert_eq!(NullCalendar.name(), "NullCalendar");
    assert!(NullCalendar.is_business_day(saturday));
    assert_eq!(business_days(CalendarId::NullCalendar, friday, monday), 3);
    Ok(())
}

#[test]
fn joint_calendars_apply_union_and_intersection_rules() -> Result<()> {
    let weekends = WeekendsOnly;
    let null = NullCalendar;
    let saturday = date(2024, 6, 1)?;
    assert!(JointCalendar::union([] as [&dyn Calendar; 0]).is_err());
    let union = JointCalendar::union([&weekends as &dyn Calendar, &null])?;
    let intersection = JointCalendar::intersection([&weekends as &dyn Calendar, &null])?;
    assert!(union.is_weekend(saturday));
    assert!(!intersection.is_weekend(saturday));
    assert!(!union.is_holiday(saturday));
    assert!(!intersection.is_holiday(saturday));
    assert!(!union.is_business_day(saturday));
    assert!(intersection.is_business_day(saturday));
    assert_eq!(union.name(), "JointCalendar");
    let period = Period::new(1, TimeUnit::Months);
    let month_end = date(2024, 1, 31)?;
    assert_eq!(
        weekends.advance(month_end, period, BusinessDayConvention::Unadjusted, true)?,
        date(2024, 2, 29)?
    );
    assert_eq!(
        weekends.advance(
            month_end,
            Period::new(0, TimeUnit::Days),
            BusinessDayConvention::Unadjusted,
            false
        )?,
        month_end
    );
    Ok(())
}

#[test]
fn adjustment_fallbacks_and_endpoint_options() -> Result<()> {
    let calendar = WeekendsOnly;
    let sunday = date(2024, 9, 1)?;
    assert_eq!(
        calendar.adjust(sunday, BusinessDayConvention::ModifiedPreceding)?,
        date(2024, 9, 2)?
    );
    assert_eq!(
        calendar.adjust(date(2024, 5, 18)?, BusinessDayConvention::Nearest)?,
        date(2024, 5, 17)?
    );
    assert_eq!(
        calendar.business_days_between(date(2024, 5, 17)?, date(2024, 5, 20)?, true, false),
        1
    );
    assert_eq!(
        calendar.business_days_between(date(2024, 5, 17)?, date(2024, 5, 17)?, true, false),
        1
    );
    assert_eq!(
        calendar.business_days_between(date(2024, 5, 18)?, date(2024, 5, 18)?, true, true),
        0
    );
    assert_eq!(
        calendar
            .advance(
                date(2024, 2, 29)?,
                Period::new(1, TimeUnit::Years),
                BusinessDayConvention::Unadjusted,
                true
            )
            .expect("leap-year calendar advance succeeds"),
        date(2025, 2, 28)?
    );
    let union = JointCalendar::union([&calendar as &dyn Calendar, &NullCalendar])?;
    assert!(!union.is_holiday(sunday));
    assert!(union.is_business_day(date(2024, 5, 20)?));
    Ok(())
}

#[test]
fn calendar_reference_and_nearest_after_cover_all_adjustment_paths() -> Result<()> {
    struct OneBusinessDay(Date);
    impl Calendar for OneBusinessDay {
        fn name(&self) -> &'static str {
            "one-day"
        }
        fn is_weekend(&self, _: Date) -> bool {
            false
        }
        fn is_holiday(&self, date: Date) -> bool {
            date != self.0
        }
    }
    struct NeverBusiness;
    impl Calendar for NeverBusiness {
        fn name(&self) -> &'static str {
            "closed"
        }
        fn is_weekend(&self, _: Date) -> bool {
            true
        }
        fn is_holiday(&self, _: Date) -> bool {
            false
        }
    }
    let target = date(2024, 5, 20)?;
    assert_eq!(OneBusinessDay(target).name(), "one-day");
    assert_eq!(NeverBusiness.name(), "closed");
    assert!(!NeverBusiness.is_holiday(target));
    assert_eq!(
        OneBusinessDay(target).adjust(date(2024, 5, 18)?, BusinessDayConvention::Nearest)?,
        target
    );
    assert!(NeverBusiness
        .adjust(date(2024, 5, 18)?, BusinessDayConvention::Following)
        .is_err());
    assert!(NeverBusiness
        .adjust(date(2024, 5, 18)?, BusinessDayConvention::Nearest)
        .is_err());
    let reference = CalendarRef(CalendarId::WeekendsOnly);
    assert_eq!(reference.name(), "CalendarRef");
    assert!(reference.is_weekend(date(2024, 6, 1)?));
    let weekends = WeekendsOnly;
    let null = NullCalendar;
    let intersection = JointCalendar::intersection([&weekends as &dyn Calendar, &null])?;
    assert!(!intersection.is_holiday(target));
    Ok(())
}

#[test]
fn modified_preceding_and_month_end_advance_paths() -> Result<()> {
    let weekends = WeekendsOnly;
    assert_eq!(
        weekends.adjust(date(2024, 5, 19)?, BusinessDayConvention::ModifiedPreceding)?,
        date(2024, 5, 17)?
    );
    let leap_inclusive = weekends
        .advance(
            date(2024, 1, 31)?,
            Period::new(1, TimeUnit::Months),
            BusinessDayConvention::Unadjusted,
            true,
        )
        .expect("month-end advance with end-of-month preservation succeeds");
    assert_eq!(leap_inclusive, date(2024, 2, 29)?);
    let leap_exclusive = weekends
        .advance(
            date(2024, 1, 31)?,
            Period::new(1, TimeUnit::Months),
            BusinessDayConvention::Unadjusted,
            false,
        )
        .expect("month-end advance without end-of-month preservation succeeds");
    assert_eq!(leap_exclusive, date(2024, 2, 29)?);
    Ok(())
}

#[test]
fn counts_and_advances_cover_endpoint_and_period_edges() -> Result<()> {
    let calendar = WeekendsOnly;
    let friday = date(2024, 5, 17)?;
    let monday = date(2024, 5, 20)?;
    assert_eq!(
        calendar.business_days_between(friday, monday, true, true),
        2
    );
    assert_eq!(
        calendar.business_days_between(friday, monday, false, false),
        0
    );
    assert_eq!(
        calendar.business_days_between(monday, friday, false, true),
        -1
    );
    assert_eq!(
        calendar.business_days_between(friday, friday, false, true),
        1
    );

    assert_eq!(
        calendar
            .advance(
                date(2024, 1, 31)?,
                Period::new(1, TimeUnit::Months),
                BusinessDayConvention::Following,
                true,
            )
            .expect("month-end calendar advance succeeds"),
        date(2024, 2, 29)?
    );
    assert_eq!(
        calendar.advance(
            friday,
            Period::new(3, TimeUnit::Days),
            BusinessDayConvention::Following,
            true,
        )?,
        date(2024, 5, 20)?
    );
    Ok(())
}
