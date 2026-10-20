//! Textbook and identity checks for the portfolio/risk additions.
#![allow(clippy::unwrap_used)]

use finance_rs::{perf, Date};

#[test]
fn price_returns_and_index_round_trip() {
    let prices = [100.0, 110.0, 99.0];
    let simple = perf::returns::simple(&prices).unwrap();
    assert!((simple[0] - 0.1).abs() < 1e-14);
    assert!((simple[1] + 0.1).abs() < 1e-14);
    let logs = perf::returns::log(&prices).unwrap();
    assert!((logs[0] - 0.1f64.ln_1p()).abs() < 1e-14);
    let index = perf::returns::to_price_index(&simple, 100.0).unwrap();
    assert!((index[2] - 99.0).abs() < 1e-12);
    assert_eq!(
        perf::returns::rebase(&prices, 10.0).unwrap(),
        [10.0, 11.0, 9.9]
    );
}

#[test]
fn return_series_statistics() {
    let r = [0.1, -0.05, 0.0, 0.2, -0.1];
    assert_eq!(perf::returns::best(&r).unwrap(), 0.2);
    assert_eq!(perf::returns::worst(&r).unwrap(), -0.1);
    assert!((perf::returns::avg_win(&r).unwrap() - 0.15).abs() < 1e-14);
    assert!((perf::returns::avg_loss(&r).unwrap() + 0.075).abs() < 1e-14);
    assert!((perf::returns::avg_return(&r).unwrap() - 0.03).abs() < 1e-14);
    let excess = perf::returns::excess(&[0.1, -0.1], 0.05, 1.0).unwrap();
    assert!((excess[0] - 0.05).abs() < 1e-14 && (excess[1] + 0.15).abs() < 1e-14);
    assert_eq!(
        perf::returns::consecutive_wins(&[0.1, 0.2, -0.1, 0.3]).unwrap(),
        2
    );
    assert_eq!(
        perf::returns::consecutive_losses(&[0.1, -0.2, -0.1, 0.3]).unwrap(),
        2
    );
    assert_eq!(
        perf::returns::exposure(&[0.1, 0.0, -0.1]).unwrap(),
        2.0 / 3.0
    );
    assert!(
        (perf::returns::geometric_mean(&[0.1, -0.1]).unwrap() + 0.005_012_562_893_380_045).abs()
            < 1e-12
    );
    assert!((perf::returns::twr(&[0.1, -0.1]).unwrap() + 0.01).abs() < 1e-14);
    assert!(
        (perf::returns::modified_dietz(100.0, 125.0, &[(0.5, 20.0)]).unwrap() - 5.0 / 110.0).abs()
            < 1e-14
    );
    let rolling =
        perf::returns::rolling(&[0.1, 0.2, 0.3], 2, |w| Ok(w.iter().sum::<f64>())).unwrap();
    assert!((rolling[0] - 0.3).abs() < 1e-14 && (rolling[1] - 0.5).abs() < 1e-14);
}

#[test]
fn dated_returns_and_mwr() {
    let dates = [
        Date::new(2023, 12, 29).unwrap(),
        Date::new(2024, 1, 31).unwrap(),
        Date::new(2024, 2, 29).unwrap(),
        Date::new(2024, 4, 1).unwrap(),
    ];
    let prices = [100.0, 105.0, 110.0, 120.0];
    assert!(
        (perf::returns::mtd(&dates, &prices, dates[2]).unwrap() - (110.0 / 105.0 - 1.0)).abs()
            < 1e-14
    );
    assert!((perf::returns::ytd(&dates, &prices, dates[2]).unwrap() - 0.1).abs() < 1e-14);
    assert!((perf::returns::qtd(&dates, &prices, dates[2]).unwrap() - 0.1).abs() < 1e-14);
    let aggregated = perf::returns::aggregate(
        &dates[1..],
        &[0.05, 110.0 / 105.0 - 1.0, 120.0 / 110.0 - 1.0],
        perf::returns::Period::Month,
    )
    .unwrap();
    assert_eq!(aggregated.len(), 3);
    let flows = [-100.0, 110.0];
    let xirr_dates = [
        Date::new(2020, 1, 1).unwrap(),
        Date::new(2021, 1, 1).unwrap(),
    ];
    let expected = 1.1_f64.powf(365.0 / 366.0) - 1.0;
    assert!((perf::returns::mwr(&xirr_dates, &flows).unwrap() - expected).abs() < 1e-10);
}

#[test]
fn ratios_have_textbook_values_and_domains() {
    let r = [0.1, -0.1, 0.1, -0.1];
    assert!(
        (perf::downside_deviation(&r, 0.0, 1.0).unwrap() - (0.01f64 / 2.0).sqrt()).abs() < 1e-14
    );
    assert!(perf::skew(&r).unwrap().abs() < 1e-14);
    assert!((perf::kurtosis(&r).unwrap() + 6.0).abs() < 1e-12);
    assert!((perf::expected_max_sharpe(1, 0.5).unwrap()).abs() < 1e-14);
    assert!(perf::probabilistic_sharpe(&[0.1, -0.1, 0.2, -0.05], 0.0)
        .unwrap()
        .is_finite());
    assert!(perf::deflated_sharpe(&[0.1, -0.1, 0.2, -0.05], 5, 0.1)
        .unwrap()
        .is_finite());
    assert!((perf::weighted_sharpe(&[0.1, -0.1], &[0.5, 0.5]).unwrap()).abs() < 1e-14);
    assert!((perf::weighted_outcome_sharpe(&[(0.5, 0.1), (0.5, -0.1)]).unwrap()).abs() < 1e-14);
    assert!((perf::treynor(&r, &r, 0.0, 1.0).unwrap()).abs() < 1e-14);
    assert!((perf::m2(&r, &r, 0.0, 1.0).unwrap()).abs() < 1e-14);
    assert!((perf::gain_to_pain(&r).unwrap()).abs() < 1e-14);
    assert!((perf::payoff_ratio(&r).unwrap() - 1.0).abs() < 1e-14);
    assert!((perf::profit_factor(&r).unwrap() - 1.0).abs() < 1e-14);
    assert!((perf::profit_ratio(&r).unwrap() - 1.0).abs() < 1e-14);
    assert!((perf::tail_ratio(&r).unwrap() - 1.0).abs() < 1e-14);
    assert!((perf::cpc_index(&r).unwrap() - 0.5).abs() < 1e-14);
    assert!((perf::outlier_win_ratio(&r).unwrap() - 1.0).abs() < 1e-14);
    assert!((perf::outlier_loss_ratio(&r).unwrap() - 1.0).abs() < 1e-14);
    assert!(perf::recovery_factor(&r).unwrap() < 0.0);
    assert!(perf::kelly_criterion(&r).unwrap().abs() < 1e-14);
    assert!(perf::risk_return_ratio(&r).unwrap().abs() < 1e-14);
    assert_eq!(
        perf::prob_backtest_overfitting(&[vec![1.0, 2.0, 3.0, 4.0], vec![4.0, 3.0, 2.0, 1.0]], 2)
            .unwrap(),
        1.0
    );
    assert!(perf::risk_of_ruin(&[0.1, 0.1, -0.1]).unwrap() <= 1.0);
    assert!(perf::stability(&[0.01, 0.02, 0.03, 0.04]).unwrap() > 0.9);
}
use finance_rs::{attribution, factors};

#[test]
fn risk_and_benchmark_textbook_examples() {
    let returns = [0.01, -0.02, 0.03, -0.01, 0.02, -0.015];
    assert!(perf::var_cornish_fisher(&returns, 0.95)
        .unwrap()
        .is_finite());
    assert!(perf::cvar_cornish_fisher(&returns, 0.95)
        .unwrap()
        .is_finite());
    assert!(perf::var_ewma(&returns, 0.94, 0.95).unwrap().is_finite());
    assert!((perf::risk_scale_var(-0.02, 4.0).unwrap() + 0.04).abs() < 1e-14);
    let b = [0.02, -0.01, 0.04, -0.02, 0.01, -0.01];
    assert!(perf::tracking_error(&returns, &b, 252.0).unwrap() > 0.0);
    assert!(
        (perf::r_squared(&returns, &b).unwrap()
            - perf::beta(&returns, &b).unwrap().powi(2)
                * finance_rs::stats::variance(&b, 1).unwrap()
                / finance_rs::stats::variance(&returns, 1).unwrap())
        .abs()
            < 1e-12
    );
    assert!(perf::up_capture(&returns, &b).is_ok());
    assert!(perf::down_capture(&returns, &b).is_ok());
    assert!(perf::batting_average(&returns, &b).is_ok());
    assert!(perf::beta_fragility(&returns, &b).unwrap() >= 0.0);
}

#[test]
fn ulcer_metrics_match_quantstats_definitions() {
    let r = [0.1, -0.2, 0.1];
    let ui = ((0.04_f64 + 0.0144) / 2.0).sqrt();
    let total = 1.1_f64 * 0.8 * 1.1 - 1.0;
    assert!((perf::ulcer_index(&r).unwrap() - ui).abs() < 1e-14);
    assert!((perf::ulcer_performance_index(&r, 0.0).unwrap() - total / ui).abs() < 1e-14);
    let sigma = finance_rs::stats::variance(&r, 1).unwrap().sqrt();
    let pitfall = -perf::cvar_historical(&[0.0, -0.2, -0.12], 0.95).unwrap() / sigma;
    let serenity = perf::serenity_index(&r, 0.0).unwrap();
    assert!(serenity.is_finite());
    assert!((serenity - r.iter().sum::<f64>() / (ui * pitfall)).abs() < 1e-14);
}

#[test]
fn drawdown_selection_and_dates_preserve_episode_values() {
    let returns = [0.1, -0.2, 0.15, -0.1, -0.1, 0.3];
    let dates = [
        Date::new(2024, 1, 1).unwrap(),
        Date::new(2024, 1, 2).unwrap(),
        Date::new(2024, 1, 3).unwrap(),
        Date::new(2024, 1, 4).unwrap(),
        Date::new(2024, 1, 5).unwrap(),
        Date::new(2024, 1, 6).unwrap(),
    ];
    let episodes = perf::drawdown_periods_dated(&dates, &returns).unwrap();
    let top = perf::top_drawdowns(&returns, 1).unwrap();
    assert_eq!(top.len(), 1);
    assert_eq!(
        top[0].max_drawdown,
        episodes.iter().map(|p| p.max_drawdown).fold(0.0, f64::min)
    );
    assert!(episodes
        .iter()
        .all(|p| p.start <= p.valley && p.valley <= p.end));
}

#[test]
fn risk_and_linking_conservation_properties() {
    let covariance = [&[0.04, 0.01][..], &[0.01, 0.09][..]];
    for weights in [[0.2, 0.8], [0.5, 0.5], [1.0, 0.0], [0.0, 1.0]] {
        let components = perf::risk::component_var(&weights, &covariance, 0.95).unwrap();
        let marginal = perf::risk::marginal_var(&weights, &covariance, 0.95).unwrap();
        let variance = weights[0] * (covariance[0][0] * weights[0] + covariance[0][1] * weights[1])
            + weights[1] * (covariance[1][0] * weights[0] + covariance[1][1] * weights[1]);
        let portfolio_var = finance_rs::math::normal_ppf(0.95).unwrap() * variance.sqrt();
        assert!((components.iter().sum::<f64>() - portfolio_var).abs() < 1e-12);
        assert!((components[0] - weights[0] * marginal[0]).abs() < 1e-12);
    }
    let periods = [
        attribution::LinkedPeriod {
            portfolio_return: 0.1,
            benchmark_return: 0.05,
            effects: [0.02, 0.02, 0.01],
        },
        attribution::LinkedPeriod {
            portfolio_return: -0.02,
            benchmark_return: -0.01,
            effects: [-0.004, -0.003, -0.003],
        },
    ];
    let target = (1.1f64 * 0.98 - 1.0) - (1.05f64 * 0.99 - 1.0);
    for linked in [
        attribution::link::carino(&periods),
        attribution::link::menchero(&periods),
        attribution::link::grap(&periods),
    ] {
        assert!((linked.unwrap().iter().sum::<f64>() - target).abs() < 1e-12);
    }
}

#[test]
fn brinson_effects_reconcile_active_return() {
    let (wp, wb, rp, rb) = ([0.6, 0.4], [0.5, 0.5], [0.10, 0.02], [0.08, 0.04]);
    let effects = attribution::brinson_hood_beebower(&wp, &wb, &rp, &rb).unwrap();
    let active = wp.iter().zip(rp).map(|(w, r)| w * r).sum::<f64>()
        - wb.iter().zip(rb).map(|(w, r)| w * r).sum::<f64>();
    assert!(
        (effects
            .iter()
            .map(|e| e.allocation + e.selection + e.interaction)
            .sum::<f64>()
            - active)
            .abs()
            < 1e-14
    );
    let bf = attribution::brinson_fachler(&wp, &wb, &rp, &rb).unwrap();
    assert!(bf.iter().all(|e| e.allocation.is_finite()));
}

#[test]
fn factor_regression_and_rolling_beta_reconcile() {
    let f1 = [-0.02, -0.01, 0.0, 0.01, 0.02, 0.03, 0.04];
    let f2 = [0.03, 0.01, -0.01, 0.02, 0.0, -0.02, 0.01];
    let f3 = [0.01, -0.02, 0.03, -0.01, 0.02, 0.01, -0.03];
    let y: Vec<f64> = (0..f1.len())
        .map(|i| {
            0.001 + 1.2 * f1[i] - 0.4 * f2[i]
                + [0.0001, -0.0002, 0.0002, -0.0001, 0.0003, -0.0003, 0.0][i]
        })
        .collect();
    let fit = factors::exposures(&y, &[&f1, &f2]).unwrap();
    assert!((fit.coef[0] - 0.001).abs() < 1e-3);
    assert_eq!(factors::attribution(&fit, &[&f1, &f2]).unwrap().len(), 3);
    assert!(factors::fama_french_alpha(&y, &[&f1, &f2, &f3]).is_ok());
    assert_eq!(factors::rolling_beta(&y, &f1, 4).unwrap().len(), 4);
}
