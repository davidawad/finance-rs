use super::*;
#[test]
fn valuation_and_dilution_textbook() {
    assert_eq!(post_money(8.0, 2.0).unwrap(), 10.0);
    assert_eq!(price_per_share(8.0, 4.0).unwrap(), 2.0);
    assert_eq!(dilution(0.5, 8.0, 10.0).unwrap(), 0.4);
    assert!((option_pool_shuffle(8.0, 2.0, 0.1).unwrap() - 7.777_777_777_777_778).abs() < 1e-12);
}
#[test]
fn safe_and_waterfall_examples() {
    let s = safe::post_money(1.0, 10.0, 0.2, 2.0, 5.0).unwrap();
    assert!((s.ownership - 0.1).abs() < 1e-12);
    assert_eq!(s.price, 1.6);
    let pre = safe::pre_money(1.0, 8.0, 0.0, 2.0, 4.0).unwrap();
    assert_eq!(pre.price, 2.0);
    assert!((pre.ownership - 0.111_111_111_111_111_1).abs() < 1e-12);
    let mfn = safe::mfn(1.0, 2.0, 5.0, &[(10.0, 0.0), (8.0, 0.1)]).unwrap();
    assert_eq!(mfn.price, 1.6);
    assert!((mfn.ownership - 0.111_111_111_111_111_1).abs() < 1e-12);
    let note = note::conversion(note::Terms {
        principal: 1_000.0,
        annual_rate: 0.1,
        days: 365,
        day_count: crate::daycount::DayCount::Act365F,
        cap: 20.0,
        company_capitalization: 10.0,
        discount: 0.2,
        round_pps: 2.0,
    })
    .unwrap();
    assert!((note.accrued_principal - 1_100.0).abs() < 1e-10);
    assert_eq!(note.price, 1.6);
    assert!((note.shares - 687.5).abs() < 1e-10);
    assert_eq!(
        cap_table::fully_diluted(&[
            cap_table::Holding {
                shares: 10.0,
                kind: cap_table::SecurityKind::Common
            },
            cap_table::Holding {
                shares: 2.0,
                kind: cap_table::SecurityKind::Options
            },
        ])
        .unwrap(),
        12.0
    );
    let p = waterfall::liquidation(
        &[waterfall::PreferredClass {
            shares: 1.0,
            liquidation_multiple: 1.0,
            seniority: 0,
            participating: false,
            participation_cap: None,
        }],
        1.0,
        10.0,
    )
    .unwrap();
    assert_eq!(p.common + p.preferred, 10.0);

    let capped = waterfall::PreferredClass {
        shares: 1.0,
        liquidation_multiple: 1.0,
        seniority: 0,
        participating: true,
        participation_cap: Some(2.0),
    };
    let capped_payout = waterfall::liquidation(&[capped], 1.0, 10.0).unwrap();
    assert!((capped_payout.preferred - 2.0).abs() < 1e-12);
    assert!((capped_payout.common - 8.0).abs() < 1e-12);
    assert_eq!(
        waterfall::breakpoints(&[capped], 1.0).unwrap(),
        vec![2.0, 4.0]
    );

    let senior = waterfall::PreferredClass {
        shares: 1.0,
        liquidation_multiple: 2.0,
        seniority: 0,
        participating: false,
        participation_cap: None,
    };
    let junior = waterfall::PreferredClass {
        shares: 1.0,
        liquidation_multiple: 1.0,
        seniority: 1,
        participating: false,
        participation_cap: None,
    };
    let seniority_payout = waterfall::liquidation(&[senior, junior], 1.0, 3.5).unwrap();
    assert!((seniority_payout.preferred - 3.0).abs() < 1e-12);
    assert!((seniority_payout.common - 0.5).abs() < 1e-12);
}

#[test]
fn nvca_round_and_venture_method_closed_forms() {
    let r = round(
        &[0.9, 0.1],
        Round {
            investment: 2.0,
            pre_money: 8.0,
            pool_target_post: 0.1,
            pool_in_pre: true,
        },
    )
    .unwrap();
    assert_eq!(r.price_per_share, 8.0);
    assert_eq!(r.new_shares, 0.25);
    assert!((r.founder_ownership - 0.72).abs() < 1e-12);
    assert!((r.investor_ownership - 0.2).abs() < 1e-12);
    assert!((r.pool_ownership - 0.08).abs() < 1e-12);

    let shuffled = round(
        &[1.0],
        Round {
            investment: 1.0,
            pre_money: 9.0,
            pool_target_post: 0.1,
            pool_in_pre: false,
        },
    )
    .unwrap();
    assert!((shuffled.founder_ownership - 0.8).abs() < 1e-12);
    assert!((shuffled.investor_ownership - 0.1).abs() < 1e-12);
    assert!((shuffled.pool_ownership - 0.1).abs() < 1e-12);

    let (pre, post) = vc_method(100.0, 4.0, 5.0, 2.0, 0.8).unwrap();
    assert_eq!(post, 20.0);
    assert_eq!(pre, 18.0);
    assert_eq!(required_ownership(2.0, 100.0, 4.0, 0.8).unwrap(), 0.1);
    let returns = scenario_returns(&[(0.5, 0.0), (0.5, 4.0)]).unwrap();
    assert_eq!(returns.expected_moic, 2.0);
    assert_eq!(returns.power_law_spread, 2.0);
}

#[test]
fn yc_nvca_reference_fixture_values() {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/vc_yc_nvca.json")).unwrap();
    let yc = &fixture["cases"][0];
    let safe = safe::post_money(500_000.0, 6_700_000.0, 0.0, 1.0, 10_000_000.0).unwrap();
    assert!((safe.ownership - yc["expected_ownership"].as_f64().unwrap()).abs() < 1e-12);
    assert!((safe.price - yc["expected_conversion_price"].as_f64().unwrap()).abs() < 1e-12);

    let nvca = &fixture["cases"][1];
    let round = round(
        &[0.9, 0.1],
        Round {
            investment: 2_000_000.0,
            pre_money: 8_000_000.0,
            pool_target_post: 0.1,
            pool_in_pre: true,
        },
    )
    .unwrap();
    assert!(
        (round.founder_ownership - nvca["expected_founder_ownership"].as_f64().unwrap()).abs()
            < 1e-12
    );
    assert!(
        (round.investor_ownership - nvca["expected_investor_ownership"].as_f64().unwrap()).abs()
            < 1e-12
    );
    assert!(
        (round.pool_ownership - nvca["expected_pool_ownership"].as_f64().unwrap()).abs() < 1e-12
    );
}
#[test]
fn returns_examples() {
    assert_eq!(return_the_fund(100.0, 0.1).unwrap(), 1000.0);
    assert!((annualized_from_moic(4.0, 2.0).unwrap() - 1.0).abs() < 1e-12);
    assert!((secondary_discount(80.0, 100.0).unwrap() + 0.2).abs() < 1e-12);
    assert_eq!(pro_rata_reserve(0.2, 100.0).unwrap(), 20.0);
    assert_eq!(net_deployed(100.0, 10.0, 5.0).unwrap(), 95.0);
}
#[test]
fn anti_dilution_examples() {
    assert_eq!(anti_dilution::full_ratchet(2.0, 1.5).unwrap(), 1.5);
    assert_eq!(
        anti_dilution::broad_based(2.0, 10.0, 2.0, 5.0).unwrap(),
        1.6
    );
}
#[test]
fn invalid_inputs_are_errors() {
    assert!(post_money(f64::NAN, 1.0).is_err());
    assert!(safe::mfn(1.0, 1.0, 1.0, &[]).is_err());
    assert!(round(
        &[],
        Round {
            investment: 1.0,
            pre_money: 1.0,
            pool_target_post: 0.1,
            pool_in_pre: false
        }
    )
    .is_err());
}

proptest::proptest! {
    #[test]
    fn venture_public_functions_are_panic_free(
        x in 0.001f64..1e6,
        y in 0.001f64..1e6,
        frac in 0.001f64..0.99,
        days in 0i64..10_000,
    ) {
        let _ = round(&[0.9, 0.1], Round { investment: x, pre_money: y, pool_target_post: frac, pool_in_pre: true });
        let _ = safe::post_money(x, y, frac, x, y);
        let _ = safe::pre_money(x, y, frac, x, y);
        let _ = safe::mfn(x, y, y, &[(y, frac)]);
        let _ = note::conversion(note::Terms {
            principal: x,
            annual_rate: frac,
            days,
            day_count: crate::daycount::DayCount::Act365F,
            cap: y,
            company_capitalization: y,
            discount: frac,
            round_pps: x,
        });
        let _ = cap_table::fully_diluted(&[cap_table::Holding { shares: x, kind: cap_table::SecurityKind::Safe }]);
        let classes = [waterfall::PreferredClass { shares: x, liquidation_multiple: frac, seniority: 0, participating: true, participation_cap: Some(2.0) }];
        let _ = waterfall::liquidation(&classes, y, x);
        let _ = waterfall::breakpoints(&classes, y);
        let _ = vc_method(x, 2.0, frac, y, frac);
        let _ = required_ownership(x, y, 2.0, frac);
        let _ = scenario_returns(&[(frac, x), (1.0-frac, y)]);
    }
}

proptest::proptest! {
    #[test]
    fn priced_round_targets_the_post_money_pool(
        investment in 0.0f64..5.0,
        pre_money in 10.0f64..100.0,
        target_pool in 0.0f64..0.4,
    ) {
        let result = round(&[1.0], Round {
            investment,
            pre_money,
            pool_target_post: target_pool,
            pool_in_pre: false,
        }).unwrap();
        proptest::prop_assert!((result.founder_ownership + result.investor_ownership + result.pool_ownership - 1.0).abs() < 1e-10);
        proptest::prop_assert!((result.pool_ownership - target_pool).abs() < 1e-10);
    }
}

#[test]
fn remaining_financing_paths_cover_dilution_and_return_scenarios() -> Result<()> {
    assert_eq!(post_money(8.0, 2.0)?, 10.0);
    assert_eq!(price_per_share(8.0, 4.0)?, 2.0);
    assert_eq!(dilution(0.5, 8.0, 8.0)?, 0.5);
    assert_eq!(option_pool_shuffle(8.0, 2.0, 0.0)?, 8.0);
    assert!(option_pool_shuffle(8.0, 2.0, 1.0).is_err());

    let existing_pool = round(
        &[0.8, 0.2],
        Round {
            investment: 2.0,
            pre_money: 8.0,
            pool_target_post: 0.2,
            pool_in_pre: true,
        },
    )?;
    assert_eq!(existing_pool.price_per_share, 8.0);
    assert!((existing_pool.pool_ownership - 0.2 / 1.25).abs() < 1e-12);
    let new_pool = round(
        &[0.9, 0.1],
        Round {
            investment: 2.0,
            pre_money: 8.0,
            pool_target_post: 0.1,
            pool_in_pre: false,
        },
    )?;
    assert_eq!(new_pool.pool_ownership, 0.1);
    assert!(round(
        &[0.8, 0.3],
        Round {
            investment: 1.0,
            pre_money: 10.0,
            pool_target_post: 0.1,
            pool_in_pre: false,
        }
    )
    .is_err());
    assert!(round(
        &[1.0],
        Round {
            investment: 1.0,
            pre_money: 10.0,
            pool_target_post: 1.0,
            pool_in_pre: false,
        }
    )
    .is_err());

    let (pre, post) = vc_method(100.0, 4.0, 5.0, 10.0, 0.5)?;
    assert_eq!(post, 12.5);
    assert_eq!(pre, 2.5);
    let (pre_irr, post_irr) = vc_method(100.0, 0.2, 5.0, 10.0, 0.5)?;
    assert!((post_irr - 50.0 / 1.2_f64.powi(5)).abs() < 1e-12);
    assert!(pre_irr < post_irr);
    assert_eq!(required_ownership(10.0, 100.0, 2.0, 0.5)?, 0.4);
    assert!(required_ownership(100.0, 10.0, 2.0, 1.0).is_err());
    assert!(required_ownership(1.0, 100.0, 2.0, 0.0).is_err());
    assert_eq!(annualized_from_moic(2.0, 0.0)?, 1.0);
    assert_eq!(net_deployed(10.0, 20.0, 0.0)?, 0.0);
    assert_eq!(
        scenario_returns(&[(0.25, 1.0), (0.75, 3.0)])?.expected_moic,
        2.5
    );
    assert!(scenario_returns(&[]).is_err());
    assert!(scenario_returns(&[(0.5, 1.0)]).is_err());
    assert!(price_per_share(1.0, 0.0).is_err());
    assert!(dilution(0.5, 10.0, 9.0).is_err());
    assert!(secondary_discount(1.0, 0.0).is_err());
    Ok(())
}
proptest::proptest! { #[test] fn annualized_round_trip(years in 0.1f64..50.0, rate in 0.0f64..2.0) { let moic=(1.0+rate).powf(years); let got=annualized_from_moic(moic,years).unwrap(); proptest::prop_assert!((got-rate).abs()<1e-9); } }

proptest::proptest! {
    #[test]
    fn public_arithmetic_does_not_panic(a in -1e6_f64..1e6, b in 0.0_f64..1e6, c in 0.0_f64..1.0) {
        let _ = post_money(b, a.abs());
        let _ = price_per_share(b, b + 1.0);
        let _ = dilution(c, b + 1.0, b + 2.0);
        let _ = option_pool_shuffle(b + 1.0, b, c);
        let _ = safe::post_money(b + 1.0, b + 2.0, c, b + 1.0, b + 1.0);
        let _ = safe::pre_money(b + 1.0, b + 2.0, c, b + 1.0, b + 1.0);
        let _ = safe::mfn(b + 1.0, b + 1.0, b + 1.0, &[(b + 1.0, c)]);
        let _ = return_the_fund(b + 1.0, c + 0.01);
        let _ = scenario_returns(&[(0.5, b), (0.5, a.abs())]);
        let _ = annualized_from_moic(b + 0.01, b + 0.01);
        let _ = secondary_discount(b, b + 1.0);
        let _ = pro_rata_reserve(c, b);
        let _ = net_deployed(b, b / 2.0, b / 3.0);
    }
}
