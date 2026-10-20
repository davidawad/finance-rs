use super::*;

fn date(year: i32, month: u32, day: u32) -> Result<Date> {
    Date::new(year, month, day)
}

#[allow(clippy::unnecessary_wraps)] // rolling requires a Result callback; summing a valid test window cannot fail.
fn sum_window(window: &[f64]) -> Result<f64> {
    Ok(window.iter().sum())
}

#[test]
fn return_transforms_and_summaries_cover_valid_and_invalid_inputs() -> Result<()> {
    assert!((simple(&[100.0, 110.0])?[0] - 0.1).abs() < 1e-12);
    assert!((log(&[100.0, 110.0])?[0] - 0.1_f64.ln_1p()).abs() < 1e-12);
    let index = to_price_index(&[0.1, -0.1], 100.0)?;
    assert!((index[1] - 110.0).abs() < 1e-12);
    assert!((index[2] - 99.0).abs() < 1e-12);
    assert_eq!(rebase(&[2.0, 3.0], 10.0)?, vec![10.0, 15.0]);
    assert_eq!(excess(&[0.1], 0.0, 12.0)?, vec![0.1]);
    assert!((twr(&[0.1, -0.1])? - -0.01).abs() < 1e-12);
    assert!(modified_dietz(0.0, 0.0, &[]).is_err());
    assert!((modified_dietz(100.0, 120.0, &[(0.5, 10.0)])? - 10.0 / 105.0).abs() < 1e-12);
    let returns = [-0.1, 0.0, 0.2, 0.3, -0.2, -0.1];
    assert_eq!(best(&returns)?, 0.3);
    assert_eq!(worst(&returns)?, -0.2);
    assert!((avg_return(&returns)? - (0.1 / 6.0)).abs() < 1e-12);
    assert!((avg_win(&returns)? - 0.25).abs() < 1e-12);
    assert!((avg_loss(&returns)? - (-0.4 / 3.0)).abs() < 1e-12);
    assert_eq!(consecutive_wins(&returns)?, 2);
    assert_eq!(consecutive_losses(&returns)?, 2);
    assert!((exposure(&returns)? - 5.0 / 6.0).abs() < 1e-12);
    assert!(geometric_mean(&returns)?.is_finite());
    assert_eq!(rolling(&[1.0, 2.0, 3.0], 2, sum_window)?, vec![3.0, 5.0]);
    assert!(simple(&[0.0, 1.0]).is_err());
    assert!(log(&[0.0, 1.0]).is_err());
    assert!(to_price_index(&[-1.1], 100.0).is_err());
    assert!(rebase(&[0.0], 100.0).is_err());
    assert!(excess(&[0.1], -1.0, 12.0).is_err());
    assert!(twr(&[-1.1]).is_err());
    assert!(modified_dietz(100.0, 100.0, &[(2.0, 10.0)]).is_err());
    assert!(avg_win(&[-0.1]).is_err());
    assert!(avg_loss(&[0.1]).is_err());
    assert!(geometric_mean(&[-1.1]).is_err());
    assert!(rolling(&[1.0], 0, sum_window).is_err());
    Ok(())
}

#[test]
fn calendar_returns_aggregate_ptd_and_money_weighted_paths() -> Result<()> {
    let dates = [date(2024, 1, 31)?, date(2024, 2, 7)?, date(2024, 2, 8)?];
    let month = aggregate(&dates, &[0.1, 0.2, 0.1], Period::Month)?;
    assert_eq!(month.len(), 2);
    assert!((month[1].1 - 0.32).abs() < 1e-12);
    assert_eq!(
        aggregate(&dates, &[0.1, 0.2, 0.1], Period::Quarter)?.len(),
        1
    );
    assert_eq!(aggregate(&dates, &[0.1, 0.2, 0.1], Period::Year)?.len(), 1);
    assert_eq!(aggregate(&dates, &[0.1, 0.2, 0.1], Period::Week)?.len(), 2);
    let week_start = date(2024, 2, 4)?;
    let week_end = date(2024, 2, 11)?;
    assert_eq!(
        aggregate(&[week_start, week_end], &[0.1, 0.1], Period::Week)?.len(),
        2
    );
    assert!((mtd(&dates, &[100.0, 110.0, 121.0], dates[2])? - 0.21).abs() < 1e-12);
    assert!((qtd(&dates, &[100.0, 110.0, 121.0], dates[2])? - 0.21).abs() < 1e-12);
    assert!((ytd(&dates, &[100.0, 110.0, 121.0], dates[2])? - 0.21).abs() < 1e-12);
    assert!(mtd(&dates, &[100.0, 110.0, 121.0], date(2024, 2, 3)?).is_err());
    assert!(aggregate(&dates, &[0.1], Period::Month).is_err());
    assert!(aggregate(&[dates[0], dates[0]], &[0.1, 0.1], Period::Month).is_err());
    assert!(mtd(&[dates[0], dates[0]], &[100.0, 110.0], dates[0]).is_err());
    assert!(qtd(&dates[..2], &[100.0, 110.0], dates[0]).is_err());
    assert!(ytd(&dates, &[0.0, 110.0, 121.0], dates[2]).is_err());
    assert!(mwr(&[date(2024, 1, 1)?, date(2025, 1, 1)?], &[-100.0, 110.0])?.is_finite());
    Ok(())
}
