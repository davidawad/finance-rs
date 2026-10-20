use super::*;

#[test]
fn statutory_and_textbook_examples() {
    assert_eq!(rmd_start_age(1955).unwrap(), 73.0);
    assert_eq!(rmd_start_age(1960).unwrap(), 75.0);
    assert!((rmd(27400.0, 73, 26.5).unwrap() - 1_033.962_264).abs() < 1e-6);
    assert_eq!(claiming_adjustment(12).unwrap(), 1.08);
    assert!((claiming_adjustment(-36).unwrap() - 0.8).abs() < 1e-12);
    assert!(vpw(100_000.0, 65, 0.04, 35).unwrap() > 0.0);
    assert_eq!(
        success_rate(&[vec![0.1, 0.1], vec![-0.5, 0.0]], &[0.2, 0.2]).unwrap(),
        1.0
    );
    let sepp = sepp_72t(
        100_000.0,
        60,
        0.05,
        0.04,
        SeppMethod::Amortization,
        20.0,
        15.0,
    )
    .unwrap();
    let expected_sepp = 100_000.0 * 0.05 / (1.0 - 1.05_f64.powf(-20.0));
    assert!((sepp - expected_sepp).abs() < 1e-9);
    assert!(sepp_72t(
        100_000.0,
        60,
        0.09,
        0.04,
        SeppMethod::Amortization,
        20.0,
        15.0
    )
    .is_err());
    assert!(sepp_72t(
        100_000.0,
        60,
        0.09,
        0.08,
        SeppMethod::Amortization,
        20.0,
        15.0
    )
    .is_ok());
    assert_eq!(
        roth_conversion_ladder(100.0, 10.0, 0.0, 2).unwrap(),
        vec![100.0, 110.0, 120.0]
    );
    assert_eq!(
        terminal_path_quantiles(&[vec![0.0, 10.0], vec![0.0, 30.0], vec![0.0, 20.0]]).unwrap(),
        (10.0, 20.0, 30.0)
    );
    assert_eq!(
        spending_smile_schedule(100.0, 0.0, 2, &[(2, 5.0)]).unwrap(),
        vec![100.0, 105.0]
    );
    assert_eq!(social_security::pia(1_000.0, 2026).unwrap(), 900.0);
}

#[test]
fn remaining_retirement_boundaries_and_methods() {
    assert_eq!(rmd_start_age(1900).unwrap(), 70.5);
    assert_eq!(rmd_start_age(1948).unwrap(), 70.5);
    assert_eq!(rmd_start_age(1949).unwrap(), 72.0);
    assert_eq!(rmd_start_age(1950).unwrap(), 72.0);
    assert_eq!(rmd_start_age(1951).unwrap(), 73.0);
    assert_eq!(rmd_start_age(1959).unwrap(), 73.0);
    assert!(rmd_start_age(1899).is_err());
    assert!(rmd(-1.0, 65, 20.0).is_err());
    assert!(rmd(1.0, 0, 20.0).is_err());
    assert!(rmd(1.0, 65, 0.0).is_err());
    assert_eq!(inherited_ira_10yr(100.0, 0.0, 9).unwrap().len(), 1);
    assert!(inherited_ira_10yr(-1.0, 0.0, 0).is_err());
    assert!(inherited_ira_10yr(1.0, -1.0, 0).is_err());
    assert!(inherited_ira_10yr(1.0, 0.0, 10).is_err());
    assert_eq!(constant_real(0.04, 100.0, 0.0, 0).unwrap().len(), 0);
    assert_eq!(
        constant_real(0.04, 100.0, 0.02, 3).unwrap(),
        vec![4.0, 4.08, 4.1616]
    );
    assert!(constant_real(-0.01, 100.0, 0.0, 1).is_err());
    assert!(constant_real(0.04, 100.0, -1.0, 1).is_err());
    assert!(guyton_klinger(100.0, 50.0, 0.2, 0.1, 0.1, 0.2, 0.1).is_err());
    assert_eq!(
        guyton_klinger(100.0, 10.0, 0.2, 0.1, 0.1, 0.1, 0.2).unwrap(),
        11.0
    );
    assert_eq!(
        guyton_klinger(100.0, 30.0, 0.2, 0.1, 0.1, 0.1, 0.2).unwrap(),
        27.0
    );
    assert_eq!(
        guyton_klinger(100.0, 20.0, 0.2, 0.1, 0.1, 0.1, 0.2).unwrap(),
        20.0
    );
    assert_eq!(vpw(100.0, 100, 0.0, 2).unwrap(), 100.0);
    assert_eq!(vpw(100.0, 101, 0.0, 2).unwrap(), 100.0);
    assert!(vpw(100.0, 0, 0.0, 2).is_err());
    assert!(vpw(100.0, 65, -1.0, 2).is_err());
    assert!(success_rate(&[], &[]).is_err());
    assert!(success_rate(&[vec![]], &[0.1]).is_err());
    assert!(success_rate(&[vec![0.0]], &[f64::NAN]).is_err());
    assert!(success_rate(&[vec![0.0]], &[-0.1]).is_err());
    assert!(success_rate(&[vec![0.0]], &[0.5, 0.7]).is_err());
    assert!(success_rate(&[vec![-1.0]], &[0.1]).is_err());
    assert_eq!(
        success_rate(&[vec![0.1, 0.1], vec![-0.9, 0.0]], &[0.2, 0.2]).unwrap(),
        0.5
    );
    assert!(spending_smile(100.0, 0.0).is_ok());
    assert!(spending_smile(100.0, 121.0).is_err());
    assert!(social_security_pia(-1.0, 2026, (1.0, 2.0)).is_err());
    assert!(social_security_pia(100.0, 2026, (2.0, 1.0)).is_err());
    assert!(social_security_pia(100.0, 1978, (1.0, 2.0)).is_err());
    assert!(claiming_adjustment(-61).is_err());
    assert!(claiming_adjustment(61).is_err());
    assert!(sepp_72t(
        100.0,
        60,
        0.0,
        0.0,
        SeppMethod::RequiredMinimumDistribution,
        20.0,
        10.0
    )
    .is_ok());
    assert!(sepp_72t(100.0, 60, 0.0, 0.0, SeppMethod::Annuitization, 20.0, 10.0).is_ok());
    assert!(sepp_72t(100.0, 17, 0.0, 0.0, SeppMethod::Amortization, 20.0, 10.0).is_err());
    assert!(roth_conversion_ladder(f64::MAX, 0.0, 1.0, 2).is_err());
    assert!(spending_smile_schedule(100.0, 0.0, 2, &[(0, 1.0)]).is_err());
    assert!(spending_smile_schedule(100.0, 0.0, 2, &[(3, 1.0)]).is_err());
    assert!(spending_smile_schedule(-1.0, 0.0, 2, &[]).is_err());
    assert!(spending_smile_schedule(100.0, -1.0, 2, &[]).is_err());
    assert!(spending_smile_schedule(100.0, 0.0, 2, &[(1, f64::NAN)]).is_err());
    assert!(spending_smile_schedule(f64::MAX, 0.0, 1, &[(1, f64::MAX)]).is_err());
    assert!(terminal_path_quantiles(&[]).is_err());
    assert!(terminal_path_quantiles(&[vec![]]).is_err());
    assert!(terminal_path_quantiles(&[vec![f64::NAN]]).is_err());
    assert!(social_security::pia(1_000.0, 1978).is_err());
    assert!(social_security::pia_with_bend_points(1_000.0, 0.0, 1.0).is_err());
    assert_eq!(social_security::bend_points(1978), None);
}

#[test]
fn withdrawal_path_summaries_reject_empty_nonfinite_and_failed_scenarios() {
    assert!(success_rate(&[vec![0.0]], &[f64::NAN]).is_err());
    assert!(success_rate(&[vec![0.0]], &[-0.1]).is_err());
    assert_eq!(success_rate(&[vec![0.0]], &[1.1]).unwrap(), 0.0);
    assert!(terminal_path_quantiles(&[vec![]]).is_err());
    assert!(terminal_path_quantiles(&[vec![0.0, f64::NAN]]).is_err());
    assert!(terminal_path_quantiles(&[vec![f64::INFINITY]]).is_err());
    assert_eq!(
        blanchett_spending_schedule(50_000.0, 65.0, 0)
            .unwrap()
            .len(),
        0
    );
}
