use super::*;
use crate::Result;

fn d(y: i32, m: u32, day: u32) -> Result<Date> {
    Date::new(y, m, day)
}

#[test]
fn pyxirr_feb_end_examples() -> Result<()> {
    let (a, b) = (d(2020, 2, 29)?, d(2021, 2, 28)?);
    assert_eq!(DayCount::Thirty360Us.year_fraction(a, b), 1.0);
    assert_eq!(DayCount::Thirty360EuIsda.year_fraction(a, b), 1.0);
    assert_eq!(DayCount::Thirty360Eu.days(a, b), 359);
    assert_eq!(DayCount::Thirty360Isda.days(a, b), 359);
    Ok(())
}

#[test]
fn reversed_dates_negate() -> Result<()> {
    let (a, b) = (d(2019, 5, 17)?, d(2023, 11, 2)?);
    for dc in DayCount::ALL {
        let f = dc.year_fraction(a, b);
        assert!(f > 0.0, "{}", dc.name());
        if matches!(dc, DayCount::ActActIsda) || dc.name().starts_with("ACT") {
            assert!((dc.year_fraction(b, a) + f).abs() < 1e-15, "{}", dc.name());
        }
    }
    Ok(())
}

#[test]
fn no_leap_skips_feb29() -> Result<()> {
    assert_eq!(DayCount::Nl365.days(d(2020, 1, 1)?, d(2021, 1, 1)?), 365);
    assert_eq!(DayCount::Nl365.days(d(2020, 2, 29)?, d(2020, 3, 1)?), 0);
    assert_eq!(DayCount::Nl365.days(d(2020, 2, 28)?, d(2020, 2, 29)?), 0);
    Ok(())
}

#[cfg(feature = "calendars")]
#[test]
fn business_day_convention_is_named() {
    assert_eq!(
        DayCount::Bus252(crate::calendars::CalendarId::WeekendsOnly).name(),
        "BUS/252"
    );
}

#[test]
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
fn conventions_cover_reference_and_month_end_rules() -> Result<()> {
    let start = d(2020, 1, 31)?;
    let end = d(2020, 2, 29)?;
    assert_eq!(DayCount::Thirty360Us.days(start, end), 29);
    assert_eq!(DayCount::Thirty360Isda.days(start, end), 29);
    assert_eq!(DayCount::Thirty360Eu.days(start, end), 29);
    assert_eq!(DayCount::Thirty360EuIsda.days(start, end), 30);
    assert_eq!(
        DayCount::Thirty360EuPlus.days(d(2020, 1, 1)?, d(2020, 1, 31)?),
        30
    );
    assert_eq!(DayCount::Thirty365.days(start, end), 29);
    assert_eq!(DayCount::from_name("  act/360 "), Some(DayCount::Act360));
    assert_eq!(DayCount::from_name("not a basis"), None);
    let ref_start = d(2020, 1, 31)?;
    let ref_end = d(2020, 7, 31)?;
    let icma = DayCount::ActActIcma.year_fraction_ref(
        d(2020, 2, 29)?,
        d(2020, 5, 31)?,
        ref_start,
        ref_end,
        2,
    );
    let expected_icma = d(2020, 5, 31)?.days_since(d(2020, 2, 29)?) as f64
        / (ref_end.days_since(ref_start) as f64 * 2.0);
    assert!((icma - expected_icma).abs() < 1e-12);
    assert!(
        DayCount::ActActIcma.year_fraction_ref(
            d(2020, 1, 31)?,
            d(2020, 10, 31)?,
            d(2020, 7, 31)?,
            d(2021, 1, 31)?,
            2,
        ) > 0.0
    );
    assert!(DayCount::ActActIcma
        .year_fraction_ref(
            d(-999_999, 1, 1)?,
            d(999_999, 1, 1)?,
            d(-999_999, 1, 1)?,
            d(-999_998, 1, 1)?,
            12,
        )
        .is_nan());
    let previous_reference = DayCount::ActActIcma.year_fraction_ref(
        d(2019, 1, 31)?,
        d(2019, 7, 31)?,
        d(2020, 1, 31)?,
        d(2020, 7, 31)?,
        2,
    );
    assert!((previous_reference - 0.5).abs() < 1e-12);
    assert!(
        (DayCount::ActActIcma
            .year_fraction_ref(ref_start, d(2021, 1, 31)?, ref_start, ref_end, 2,)
            - 1.0)
            .abs()
            < 1e-12
    );
    assert!(DayCount::ActActIcma
        .year_fraction_ref(start, end, ref_start, ref_end, 0)
        .is_nan());
    assert!(DayCount::ActActIcma
        .year_fraction_ref(start, end, ref_end, ref_start, 2)
        .is_nan());
    assert!(DayCount::ActActIcma
        .year_fraction_ref(start, end, ref_start, ref_end, 13)
        .is_nan());
    assert!(DayCount::Act360
        .year_fraction_ref(start, end, ref_start, ref_end, 2)
        .is_nan());
    assert!(DayCount::ActActIcma.year_fraction_ref(end, start, ref_start, ref_end, 2) < 0.0);
    assert!(DayCount::ActActIcma
        .year_fraction_ref(
            d(-1_000_000, 1, 1)?,
            d(-999_999, 1, 1)?,
            d(-1_000_000, 1, 31)?,
            d(-999_999, 1, 31)?,
            2,
        )
        .is_nan());
    assert!(DayCount::ActActIcma
        .year_fraction_ref(
            d(1_000_000, 7, 31)?,
            d(1_000_000, 8, 1)?,
            d(1_000_000, 1, 31)?,
            d(1_000_000, 7, 31)?,
            2,
        )
        .is_nan());
    assert!(
        (DayCount::ActActAfb.year_fraction(d(2019, 2, 28)?, d(2020, 2, 29)?) - 1.0).abs() < 1e-12
    );
    // The partial AFB year contains leap day and therefore uses 366 days.
    assert!(
        (DayCount::ActActAfb.year_fraction(d(2020, 2, 28)?, d(2021, 2, 27)?) - 365.0 / 366.0).abs()
            < 1e-12
    );
    Ok(())
}
