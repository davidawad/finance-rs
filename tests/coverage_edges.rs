//! Public error and boundary paths that complement the textbook golden cases.

#[cfg(feature = "actuarial")]
use finance_rs::actuarial::{
    annuity,
    life::{self, LifeTable},
    retirement::{self, InheritedIraRule},
    tables::JointLastSurvivor,
};
use finance_rs::{
    options::{bsm_greeks, OptionKind},
    pm::score::{brier, log_loss},
    portfolio::{correlation_matrix, covariance_matrix},
    FinanceError,
};

#[test]
#[cfg(feature = "actuarial")]
fn actuarial_boundaries_and_failures_are_observable() {
    assert!(annuity::continuous(0.0, 3.0).is_ok());
    assert!(annuity::perpetuity(0.05, finance_rs::tvm::When::Begin).is_ok());
    assert!(annuity::deferred(-0.5, 3.0, f64::MAX).is_err());
    assert!(annuity::increasing(-0.5, u32::MAX).is_err());

    assert!(retirement::rmd_start_age(1899).is_err());
    assert!(retirement::rmd(-1.0, 70, 20.0).is_err());
    assert!(retirement::rmd(100.0, 70, 0.0).is_err());
    assert!(retirement::rmd(100.0, 0, 20.0).is_err());
    assert!(retirement::inherited_ira_10yr(100.0, 0.0, 10).is_err());
    assert!(retirement::constant_real(-0.01, 100.0, 0.02, 30).is_err());
    assert!(retirement::vpw(100.0, 65, -1.0, 20).is_err());
    assert!(retirement::success_rate(&[], &[]).is_err());
    assert!(retirement::success_rate(&[vec![0.0]], &[1.0, 1.0]).is_err());
    assert!(retirement::success_rate(&[vec![-1.0]], &[0.0]).is_err());

    assert!(retirement::inherited_ira_schedule(
        100.0,
        0.0,
        InheritedIraRule::LifeExpectancy {
            initial_divisor: 1.0,
            years_elapsed: 1,
        },
    )
    .is_err());
    assert!(retirement::inherited_ira_schedule(
        100.0,
        0.0,
        InheritedIraRule::TenYear {
            annual_divisor: Some(0.0),
            years_elapsed: 0,
        },
    )
    .is_err());
    assert!(retirement::inherited_ira_schedule(
        100.0,
        0.0,
        InheritedIraRule::FiveYear { years_elapsed: 5 },
    )
    .is_err());

    assert!(LifeTable::new(40, vec![]).is_err());
    assert!(LifeTable::new(40, vec![f64::NAN]).is_err());
    let table = LifeTable::new(40, vec![0.1, 0.2]);
    assert!(table.is_ok());
    if let Ok(table) = table {
        assert!(life::expectancy(&table, 39).is_err());
        assert!(life::whole_life_insurance(&table, 100, 0.0).is_err());
    }

    assert!(JointLastSurvivor::new(0, 1, vec![]).is_err());
    assert!(JointLastSurvivor::new(1, 2, vec![1.0]).is_err());
    assert!(JointLastSurvivor::new(1, 1, vec![f64::INFINITY]).is_err());
}

#[test]
fn options_portfolio_and_scoring_boundaries_are_checked() {
    let at_the_money = bsm_greeks(OptionKind::Call, 100.0, 100.0, 0.0, 0.02, 0.0, 0.2);
    assert!(matches!(at_the_money, Ok(greeks) if greeks.delta == 0.0));
    assert!(bsm_greeks(OptionKind::Call, 100.0, 100.0, -1.0, 0.02, 0.0, 0.2).is_err());
    assert!(bsm_greeks(OptionKind::Put, 100.0, 0.0, 1.0, 0.02, 0.0, 0.2).is_err());

    assert!(covariance_matrix(&[], 1).is_err());
    assert!(correlation_matrix(&[vec![0.1], vec![0.2, 0.3]]).is_err());
    assert!(brier(&[], &[]).is_err());
    assert!(brier(&[0.5], &[1.5]).is_err());
    assert!(matches!(
        log_loss(&[0.0], &[1.0]),
        Err(FinanceError::Overflow { .. })
    ));
}
