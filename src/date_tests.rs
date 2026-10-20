use super::*;

#[test]
fn ordinal_round_trips() -> Result<()> {
    assert_eq!(Date::new(1970, 1, 1)?.ordinal(), 0);
    assert_eq!(Date::new(2000, 3, 1)?.ordinal(), 11_017);
    for days in [-800_000_i64, -1, 0, 59, 60, 11_016, 20_000, 3_000_000] {
        assert_eq!(Date::from_ordinal(days)?.ordinal(), days);
    }
    Ok(())
}

#[test]
fn add_months_clamps() -> Result<()> {
    let jan31 = Date::new(2024, 1, 31)?;
    assert_eq!(jan31.add_months(1)?, Date::new(2024, 2, 29)?);
    assert_eq!(jan31.add_months(-2)?, Date::new(2023, 11, 30)?);
    assert_eq!(jan31.add_months(13)?, Date::new(2025, 2, 28)?);
    Ok(())
}

#[test]
fn rejects_bad_dates() {
    assert!(Date::new(2023, 2, 29).is_err());
    assert!(Date::new(2024, 13, 1).is_err());
    assert!(Date::new(2024, 4, 31).is_err());
    assert!(Date::new(2024, 1, 0).is_err());
}

#[test]
fn imm_weekday_and_range_boundaries() -> Result<()> {
    let imm = Date::new(2024, 3, 20)?;
    assert!(imm.is_imm());
    assert!(!Date::new(2024, 3, 21)?.is_imm());
    assert!(!Date::new(2024, 4, 17)?.is_imm());
    assert_eq!(Date::imm_next(imm)?, Date::new(2024, 6, 19)?);
    assert_eq!(
        Date::imm_next(Date::new(2024, 12, 18)?)?,
        Date::new(2025, 3, 19)?
    );
    assert!(Date::new(1_000_001, 1, 1).is_err());
    assert!(Date::from_ordinal(i64::MAX).is_err());
    assert!(Date::from_ordinal(i64::MIN).is_err());
    assert!(Date::new(1_000_000, 12, 1)?.add_months(1).is_err());
    assert!(Date::new(1_000_000, 12, 1)?.add_years(1).is_err());
    assert!(Date::new(2024, 1, 1)?.add_years(i32::MAX).is_err());
    assert!(Date::new(2024, 1, 1)?.add_days(i32::MAX).is_err());
    assert_eq!(
        Date::new(2024, 6, 5)?.start_of_month()?,
        Date::new(2024, 6, 1)?
    );
    assert_eq!(
        Date::new(2024, 6, 5)?.end_of_month()?,
        Date::new(2024, 6, 30)?
    );
    assert!(Date::nth_weekday(2024, 13, Weekday::Monday, 1).is_err());
    assert!(Date::nth_weekday(2024, 5, Weekday::Monday, 0).is_err());
    assert_eq!(
        Date::last_weekday(2024, 5, Weekday::Friday)?,
        Date::new(2024, 5, 31)?
    );
    assert!(Date::imm_next(Date::new(1_000_000, 12, 31)?).is_err());
    assert!(Date::parse_iso("20x4-01-01").is_err());
    assert!(Date::parse_iso("2024-13-01").is_err());
    Ok(())
}
