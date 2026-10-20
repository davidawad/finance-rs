//! Closed-form portfolio checks and bad-input panic safety.
#![allow(clippy::unwrap_used)]

use finance_rs::portfolio as p;
use proptest::prelude::*;
use serde_json::Value;

mod common;

fn matrix(value: &Value) -> Vec<Vec<f64>> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|row| {
            row.as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect()
        })
        .collect()
}

fn assert_vector(actual: &[f64], expected: &Value, tol: f64) {
    let expected: Vec<f64> = expected
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect();
    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected) {
        assert!((a - e).abs() <= tol + tol * e.abs(), "{a} != {e}");
    }
}

fn assert_matrix(actual: &[Vec<f64>], expected: &Value, tol: f64) {
    let expected = matrix(expected);
    assert_eq!(actual.len(), expected.len());
    for (a, e) in actual.iter().zip(expected) {
        assert_vector(a, &serde_json::json!(e), tol);
    }
}

#[test]
fn pyportfolioopt_values_only_golden_parity() {
    let golden = common::Golden::load("portfolio");
    for c in golden.cases("mean_historical") {
        let r = matrix(&c["args"]["returns"]);
        let got = p::mean_historical(&r, c["args"]["ppy"].as_f64().unwrap()).unwrap();
        assert_vector(&got, &c["expected"], 1e-10);
    }
    for c in golden.cases("sample_cov") {
        let got = p::sample(
            &matrix(&c["args"]["returns"]),
            c["args"]["ppy"].as_f64().unwrap(),
        )
        .unwrap();
        assert_matrix(&got, &c["expected"], 1e-10);
    }
    for c in golden.cases("market_implied_returns") {
        let got = p::market_implied_returns(
            &matrix(&c["args"]["cov"]),
            &c["args"]["weights"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect::<Vec<_>>(),
            c["args"]["delta"].as_f64().unwrap(),
        )
        .unwrap();
        assert_vector(&got, &c["expected"], 1e-10);
    }
    for c in golden.cases("black_litterman_mean") {
        let (got, _) = p::black_litterman(
            &matrix(&c["args"]["cov"]),
            &c["args"]["weights"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect::<Vec<_>>(),
            c["args"]["delta"].as_f64().unwrap(),
            &matrix(&c["args"]["P"]),
            &c["args"]["Q"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect::<Vec<_>>(),
            &matrix(&c["args"]["omega"]),
            c["args"]["tau"].as_f64().unwrap(),
        )
        .unwrap();
        assert_vector(&got, &c["expected"], 1e-10);
    }
    for c in golden.cases("black_litterman_cov") {
        let (_, got) = p::black_litterman(
            &matrix(&c["args"]["cov"]),
            &c["args"]["weights"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect::<Vec<_>>(),
            c["args"]["delta"].as_f64().unwrap(),
            &matrix(&c["args"]["P"]),
            &c["args"]["Q"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect::<Vec<_>>(),
            &matrix(&c["args"]["omega"]),
            c["args"]["tau"].as_f64().unwrap(),
        )
        .unwrap();
        assert_matrix(&got, &c["expected"], 1e-10);
    }
    for c in golden.cases("hrp") {
        let got = p::hrp(&matrix(&c["args"]["returns"])).unwrap();
        assert_vector(&got, &c["expected"], 1e-8);
    }
    for c in golden.cases("ffn_inverse_vol") {
        let got = p::inverse_volatility(&matrix(&c["args"]["returns"])).unwrap();
        assert_vector(&got, &c["expected"], 1e-10);
    }
    let pf = common::Golden::load("portfolio_pyfolio");
    for c in pf.cases("turnover") {
        let before = c["args"]["weights_before"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect::<Vec<_>>();
        let after = c["args"]["weights_after"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_f64().unwrap())
            .collect::<Vec<_>>();
        let half_turnover = p::turnover(&before, &after).unwrap();
        let expected = c["expected"].as_f64().unwrap();
        assert!((2.0 * half_turnover - expected).abs() < 1e-12);
    }
}

#[test]
fn portfolio_validation_edges_cover_estimators_and_weights() {
    assert!(p::mean_historical(&[vec![0.1]], 0.0).is_err());
    assert!(p::capm(&[vec![0.1], vec![0.2]], &[0.1, 0.2], 0.01, 0.0).is_err());
    assert!(p::fix_nonpositive_semidefinite(&[]).is_err());
    assert!(p::fix_nonpositive_semidefinite(&[vec![1.0, 0.0], vec![0.5, 1.0]]).is_err());
    assert!(p::equal_risk_contribution(&[]).is_err());
    assert!(p::tangency_unconstrained(&[0.05], &[vec![1.0]], 0.05).is_err());
    assert!(p::black_litterman(
        &[vec![1.0]],
        &[1.0],
        1.0,
        &[vec![]],
        &[0.1],
        &[vec![1.0]],
        0.1,
    )
    .is_err());
}

#[test]
fn estimator_and_covariance_examples() {
    let r = vec![vec![0.01, 0.02], vec![0.03, 0.04], vec![0.02, 0.01]];
    let mu = p::expected_returns::mean_historical(&r, 1.0).unwrap();
    let geometric = (1.01_f64 * 1.03 * 1.02).powf(1.0 / 3.0) - 1.0;
    assert!((mu[0] - geometric).abs() < 1e-12);
    let cov = p::cov::sample(&r, 252.0).unwrap();
    assert_eq!(cov.len(), 2);
    assert!((cov[0][1] - cov[1][0]).abs() < 1e-15);
    let corr = p::correlation_matrix(&r).unwrap();
    assert!((corr[0][0] - 1.0).abs() < 1e-14);
    assert!(p::expected_returns::ema_historical(&r, 1.0, 2).unwrap()[0].is_finite());
    assert!(
        p::expected_returns::capm(&r, &[0.01, 0.03, 0.02], 0.02, 252.0).unwrap()[0].is_finite()
    );
    assert!(p::cov::exponential(&r, 252.0, 2).unwrap()[0][0] > 0.0);
    assert!(p::cov::semicovariance(&r, 0.02, 252.0).unwrap()[0][0] >= 0.0);
    assert!(p::cov::ledoit_wolf(&r, 252.0).unwrap()[0][0] > 0.0);
    assert!(p::cov::oracle_approximating(&r, 252.0).unwrap()[0][0] > 0.0);
}

#[test]
fn repair_black_litterman_and_portfolio_identities() {
    let cov = vec![vec![0.04, 0.0], vec![0.0, 0.09]];
    let repaired = p::cov::fix_nonpositive_semidefinite(&[vec![1.0, 2.0], vec![2.0, 1.0]]).unwrap();
    assert!(repaired[0][0] >= 0.0 && repaired[1][1] >= 0.0);
    let prior = p::market_implied_returns(&cov, &[0.6, 0.4], 2.0).unwrap();
    assert!((prior[0] - 0.048).abs() < 1e-12);
    let (posterior, post_cov) = p::black_litterman(
        &cov,
        &[0.6, 0.4],
        2.0,
        &[vec![1.0, 0.0]],
        &[0.08],
        &[vec![0.01]],
        0.05,
    )
    .unwrap();
    assert!(posterior[0] > prior[0]);
    assert!(post_cov[0][0] > 0.0 && post_cov[0][0] < cov[0][0] + 0.01);
    assert!((p::return_(&[0.6, 0.4], &[0.1, 0.2]).unwrap() - 0.14).abs() < 1e-12);
    assert!((p::variance(&[0.6, 0.4], &cov).unwrap() - 0.0288).abs() < 1e-12);
    assert!((p::volatility(&[0.6, 0.4], &cov).unwrap() - 0.0288_f64.sqrt()).abs() < 1e-12);
    let rc = p::risk_contributions(&[0.6, 0.4], &cov).unwrap();
    assert!((rc.iter().sum::<f64>() - 0.0288_f64.sqrt()).abs() < 1e-12);
    assert!(p::diversification_ratio(&[0.6, 0.4], &cov).unwrap() > 1.0);
    assert!((p::concentration_hhi(&[0.5, 0.5]).unwrap() - 0.5).abs() < 1e-12);
    assert_eq!(
        p::kelly_portfolio(&[0.1, 0.09], &cov, 0.5).unwrap().len(),
        2
    );
}

#[test]
fn weights_and_rebalancing_examples() {
    let r = vec![
        vec![0.01, 0.02],
        vec![-0.01, -0.02],
        vec![0.02, 0.01],
        vec![-0.02, -0.01],
    ];
    let cov = vec![vec![0.04, 0.0], vec![0.0, 0.09]];
    let inv = p::weights::inverse_volatility(&r).unwrap();
    assert!((inv.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    let erc = p::weights::equal_risk_contribution(&cov).unwrap();
    assert!((erc[0] * 0.2 - erc[1] * 0.3).abs() < 1e-8);
    let min = p::weights::min_variance_unconstrained(&cov).unwrap();
    assert!((min[0] - 0.692_307_692_307_692_3).abs() < 1e-12);
    let tan = p::weights::tangency_unconstrained(&[0.1, 0.12], &cov, 0.02).unwrap();
    assert!((tan.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    assert!((p::weights::hrp(&r).unwrap().iter().sum::<f64>() - 1.0).abs() < 1e-12);
    assert_eq!(
        p::discrete_allocation(&[0.5, 0.5], &[10.0, 20.0], 100.0).unwrap(),
        (vec![6, 2], 0.0)
    );
    assert_eq!(
        p::rebalance::drift(&[0.5, 0.5], &[0.1, 0.0]).unwrap(),
        vec![0.523_809_523_809_523_8, 0.476_190_476_190_476_16]
    );
    assert!(
        p::rebalance::needs_rebalance(&[0.6, 0.4], &[0.5, 0.5], p::Band::Absolute(0.05)).unwrap()
    );
    assert_eq!(
        p::rebalance::rebalance_trades(&[1.0, 1.0], &[10.0, 10.0], &[0.5, 0.5], 20.0, 0.0).unwrap(),
        vec![0.0, 0.0]
    );
    assert!((p::turnover(&[0.4, 0.6], &[0.6, 0.4]).unwrap() - 0.2).abs() < 1e-12);
    assert_eq!(p::gross_leverage(&[2.0, -1.0], 3.0).unwrap(), 1.0);
    assert_eq!(p::vol_target_leverage(0.2, 0.1, 1.5).unwrap(), 1.5);
    assert!((p::transaction_cost(&[100.0, -50.0], 10.0, 1.0).unwrap() - 2.15).abs() < 1e-12);
}

#[test]
fn portfolio_estimators_and_operations_cover_rejections_and_degenerate_inputs() {
    assert!(p::mean_historical(&[], 252.0).is_err());
    assert!(p::mean_historical(&[vec![0.1], vec![0.1, 0.2]], 252.0).is_err());
    assert!(p::ema_historical(&[vec![-1.0], vec![-1.0]], 252.0, 1).is_err());
    assert!(p::ema_historical(&[vec![0.1], vec![0.2]], 252.0, 0).is_err());
    assert!(p::exponential(&[vec![0.1], vec![0.2]], 252.0, 0).is_err());
    assert!(p::sample(&[], 252.0).is_err());
    assert!(p::covariance_matrix(&[], 1).is_err());
    assert!(p::covariance_matrix(&[vec![0.1], vec![0.2, 0.3]], 1).is_err());
    assert!(p::correlation_matrix(&[vec![0.1], vec![0.1]]).is_err());

    let cov = vec![vec![0.04, 0.0], vec![0.0, 0.09]];
    let weights = [0.6, 0.4];
    let (prior, prior_cov) = p::black_litterman(&cov, &weights, 2.0, &[], &[], &[], 0.05).unwrap();
    assert_eq!(prior.len(), 2);
    assert_eq!(prior_cov, cov);
    assert!(p::black_litterman(&cov, &weights, 2.0, &[vec![1.0]], &[], &[], 0.05).is_err());
    assert!(p::black_litterman(
        &cov,
        &weights,
        2.0,
        &[vec![1.0, 0.0]],
        &[0.1],
        &[vec![0.1]],
        0.0
    )
    .is_err());

    assert!(p::return_(&[1.0], &[]).is_err());
    assert!(p::variance(&[1.0], &cov).is_err());
    assert!(p::concentration_hhi(&[]).is_err());
    assert_eq!(
        p::risk_budget_allocations(0.0, &[], 1.0).unwrap(),
        [] as [f64; 0]
    );
    assert!(p::risk_budget_allocations(-1.0, &[1.0], 0.0).is_err());
    assert!(p::needs_rebalance(&[1.0], &[], p::Band::Absolute(0.1)).is_err());
    assert!(p::needs_rebalance(&[0.1], &[0.0], p::Band::Relative(0.1)).unwrap());
    assert!(p::needs_rebalance(&[0.1], &[0.1], p::Band::Relative(-0.1)).is_err());
    assert!(p::rebalance_trades(&[1.0], &[1.0], &[], 10.0, 0.0).is_err());
    assert!(p::discrete_allocation(&[1.0], &[0.0], 10.0).is_err());

    assert_eq!(p::weights::hrp(&[vec![0.0], vec![0.0]]).unwrap(), vec![1.0]);
    assert!(p::weights::tangency_unconstrained(&[0.0], &[vec![0.0]], 0.0).is_err());
    assert!(p::weights::hrp(&[vec![0.0, 0.0], vec![0.0, 0.0]]).is_err());
}

proptest! {
    #[test]
    fn public_portfolio_functions_do_not_panic(x in any::<f64>(), y in any::<f64>()) {
        let r = vec![vec![x, y], vec![y, x], vec![x * 0.5, y * 0.5]];
        let _ = p::mean_historical(&r, x);
        let _ = p::ema_historical(&r, x, 0);
        let _ = p::capm(&r, &[x, y], x, y);
        let _ = p::sample(&r, x);
        let _ = p::exponential(&r, x, 1);
        let _ = p::semicovariance(&r, x, y);
        let _ = p::ledoit_wolf(&r, x);
        let _ = p::oracle_approximating(&r, x);
        let _ = p::fix_nonpositive_semidefinite(&[vec![x, y], vec![y, x]]);
        let _ = p::market_implied_returns(&[vec![x, y], vec![y, x]], &[x, y], x);
        let _ = p::black_litterman(&[vec![x, y], vec![y, x]], &[x, y], x, &[vec![x,y]], &[y], &[vec![x]], y);
        let _ = p::correlation_matrix(&r);
        let _ = p::covariance_matrix(&r, 1);
        let _ = p::return_(&[x, y], &[y, x]);
        let _ = p::variance(&[x, y], &[vec![x, y], vec![y, x]]);
        let _ = p::volatility(&[x, y], &[vec![x, y], vec![y, x]]);
        let _ = p::risk_contributions(&[x, y], &[vec![x, y], vec![y, x]]);
        let _ = p::diversification_ratio(&[x, y], &[vec![x, y], vec![y, x]]);
        let _ = p::concentration_hhi(&[x, y]);
        let _ = p::risk_budget_allocations(x, &[x, y], y);
        let _ = p::kelly_portfolio(&[x, y], &[vec![x, y], vec![y, x]], x);
        let _ = p::inverse_volatility(&r);
        let _ = p::equal_risk_contribution(&[vec![x, y], vec![y, x]]);
        let _ = p::min_variance_unconstrained(&[vec![x, y], vec![y, x]]);
        let _ = p::tangency_unconstrained(&[x, y], &[vec![x, y], vec![y, x]], x);
        let _ = p::hrp(&r);
        let _ = p::discrete_allocation(&[x, y], &[y, x], x);
        let _ = p::rebalance_trades(&[x, y], &[y, x], &[x, y], x, y);
        let _ = p::drift(&[x, y], &[y, x]);
        let _ = p::needs_rebalance(&[x, y], &[y, x], p::Band::Relative(x));
        let _ = p::turnover(&[x, y], &[y, x]);
        let _ = p::gross_leverage(&[x, y], y);
        let _ = p::vol_target_leverage(x, y, x);
        let _ = p::transaction_cost(&[x, y], x, y);
    }
}
