use super::*;

fn date(year: i32, month: u32, day: u32) -> Result<Date> {
    Date::new(year, month, day)
}

#[test]
fn stubs_end_of_month_payment_lags_and_frequency_errors() -> Result<()> {
    let calendar = crate::calendars::WeekendsOnly;
    let start = date(2024, 1, 15)?;
    let end = date(2024, 12, 15)?;
    let schedule = Schedule::builder(start, end)
        .frequency(Frequency::Quarterly)
        .calendar(&calendar)
        .convention(BusinessDayConvention::ModifiedFollowing)
        .termination_convention(BusinessDayConvention::Unadjusted)
        .rule(DateGeneration::Forward)
        .first_date(date(2024, 2, 15)?)
        .next_to_last_date(date(2024, 11, 15)?)
        .long_stub(true)
        .build()
        .expect("forward stub schedule builds");
    assert_eq!(schedule.dates().first(), Some(&start));
    assert_eq!(schedule.dates().last(), Some(&end));
    assert_eq!(schedule.adjusted().last(), Some(&end));
    assert_eq!(schedule.accrual_periods().len() + 1, schedule.dates().len());
    assert_eq!(
        schedule.payment_dates(0, &calendar)?,
        schedule
            .adjusted()
            .iter()
            .copied()
            .skip(1)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        schedule.payment_dates(-2, &calendar)?.len(),
        schedule.accrual_periods().len()
    );

    let end_month = Schedule::builder(date(2024, 1, 31)?, date(2024, 7, 31)?)
        .frequency(Frequency::Monthly)
        .end_of_month(true)
        .rule(DateGeneration::Backward)
        .build()
        .expect("end-of-month schedule builds");
    assert_eq!(end_month.dates()[1], date(2024, 2, 29)?);
    assert_eq!(end_month.dates()[6], date(2024, 7, 31)?);
    let per_year = Schedule::builder(start, end)
        .frequency(Frequency::PerYear(4))
        .build()?;
    assert_eq!(per_year.accrual_periods().len(), 4);
    assert_eq!(
        Schedule::builder(start, end)
            .frequency(Frequency::Semiannual)
            .build()?
            .accrual_periods()
            .len(),
        2
    );
    assert_eq!(regular_days(Period::new(2, TimeUnit::Days)), 2.0);
    assert_eq!(regular_days(Period::new(2, TimeUnit::Weeks)), 14.0);
    assert_eq!(regular_days(Period::new(2, TimeUnit::Months)), 60.0);
    assert_eq!(regular_days(Period::new(1, TimeUnit::Years)), 365.0);
    let irregular_start = date(2024, 1, 1)?;
    let irregular_end = date(2024, 4, 1)?;
    assert_eq!(
        reference_period(
            irregular_start,
            irregular_end,
            Period::new(1, TimeUnit::Months),
            DateGeneration::Forward,
            false,
        ),
        (irregular_start, date(2024, 2, 1)?)
    );
    assert_eq!(
        reference_period(
            irregular_start,
            irregular_end,
            Period::new(1, TimeUnit::Months),
            DateGeneration::Backward,
            false,
        ),
        (date(2024, 3, 1)?, irregular_end)
    );
    assert!(Schedule::builder(start, end)
        .frequency(Frequency::Once)
        .build()
        .is_err());
    assert!(Schedule::builder(start, end)
        .frequency(Frequency::PerYear(5))
        .build()
        .is_err());
    assert!(Schedule::builder(start, end)
        .first_date(start)
        .build()
        .is_err());
    assert!(Schedule::builder(start, end)
        .first_date(date(2024, 8, 1)?)
        .next_to_last_date(date(2024, 7, 1)?)
        .build()
        .is_err());
    assert!(Schedule::builder(end, start).build().is_err());
    Ok(())
}

#[test]
fn forward_backward_short_long_and_frequency_cases() -> Result<()> {
    let start = date(2024, 1, 1)?;
    let end = date(2024, 4, 15)?;
    let forward = Schedule::builder(start, end)
        .frequency(Frequency::Monthly)
        .rule(DateGeneration::Forward)
        .build()?;
    assert_eq!(
        forward.dates(),
        &[
            start,
            date(2024, 2, 1)?,
            date(2024, 3, 1)?,
            date(2024, 4, 1)?,
            end,
        ]
    );
    let backward = Schedule::builder(start, end)
        .frequency(Frequency::Monthly)
        .rule(DateGeneration::Backward)
        .build()?;
    assert_eq!(
        backward.dates(),
        &[
            start,
            date(2024, 1, 15)?,
            date(2024, 2, 15)?,
            date(2024, 3, 15)?,
            end
        ]
    );
    let long_forward = Schedule::builder(start, end)
        .frequency(Frequency::Monthly)
        .rule(DateGeneration::Forward)
        .long_stub(true)
        .build()?;
    assert!(!long_forward.dates().contains(&date(2024, 4, 1)?));
    let long_backward = Schedule::builder(start, end)
        .frequency(Frequency::Monthly)
        .rule(DateGeneration::Backward)
        .long_stub(true)
        .build()?;
    assert!(!long_backward.dates().contains(&date(2024, 1, 15)?));
    let backward_stubs = Schedule::builder(start, date(2024, 12, 31)?)
        .frequency(Frequency::Quarterly)
        .rule(DateGeneration::Backward)
        .first_date(date(2024, 2, 1)?)
        .next_to_last_date(date(2024, 11, 1)?)
        .build()
        .expect("backward stub schedule builds");
    assert_eq!(backward_stubs.dates().first(), Some(&start));
    assert_eq!(backward_stubs.dates()[1], date(2024, 2, 1)?);
    let daily = Schedule::builder(start, date(2024, 1, 4)?)
        .frequency(Frequency::Daily)
        .build()
        .expect("daily schedule builds");
    assert_eq!(daily.dates().len(), 4);
    let weekly = Schedule::builder(start, date(2024, 1, 22)?)
        .frequency(Frequency::Weekly)
        .build()
        .expect("weekly schedule builds");
    assert_eq!(weekly.dates().len(), 4);
    assert!(Schedule::builder(start, end)
        .first_date(end)
        .build()
        .is_err());
    assert!(Schedule::builder(start, end)
        .first_date(date(2024, 2, 1)?)
        .next_to_last_date(date(2024, 2, 1)?)
        .build()
        .is_err());
    let lagged = forward.payment_dates(2, &crate::calendars::WeekendsOnly)?;
    assert_eq!(lagged.len(), forward.accrual_periods().len());
    Ok(())
}
