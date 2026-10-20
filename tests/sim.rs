use finance_rs::corp::{dcf, lbo, Convention, Lbo};
use finance_rs::sim::{simulate, Distribution, Input, PathReturns, PathValue};
use finance_rs::FinanceError;
use proptest::prelude::*;

fn input(name: &str, distribution: Distribution) -> Input {
    Input {
        name: name.into(),
        distribution,
    }
}

#[test]
fn seeded_simulation_is_reproducible_and_quantiles_order() {
    let inputs = [input(
        "fcf",
        Distribution::Normal {
            mean: 100.0,
            sd: 12.0,
        },
    )];
    let run = || {
        simulate(&inputs, None, 2_000, 42, Some(110.0), 20, |x| {
            Ok(PathValue {
                value: x[0] * 2.0,
                returns: None,
            })
        })
        .unwrap()
    };
    let a = run();
    let b = run();
    assert_eq!(a, b);
    assert!(
        a.value.p5 <= a.value.p10
            && a.value.p10 <= a.value.p50
            && a.value.p50 <= a.value.p90
            && a.value.p90 <= a.value.p95
    );
    assert_eq!(a.histogram.iter().map(|b| b.count).sum::<usize>(), 2_000);
    assert!(a.value.probability_above_price.unwrap() > 0.0);
}

#[test]
fn normal_dcf_matches_closed_form_mean_and_variance() {
    // Fixed-rate DCF is linear in FCFs, so its normal mean and variance are known.
    let inputs = [
        input(
            "fcf1",
            Distribution::Normal {
                mean: 100.0,
                sd: 2.0,
            },
        ),
        input(
            "fcf2",
            Distribution::Normal {
                mean: 110.0,
                sd: 3.0,
            },
        ),
    ];
    let rate = 0.1_f64;
    let result = simulate(&inputs, None, 100_000, 7, None, 10, |x| {
        Ok(PathValue {
            value: dcf(rate, x, Convention::EndOfPeriod)?,
            returns: None,
        })
    })
    .unwrap();
    let mean = 100.0 / (1.0 + rate) + 110.0 / (1.0 + rate).powi(2);
    let variance = 2.0_f64.powi(2) / (1.0 + rate).powi(2) + 3.0_f64.powi(2) / (1.0 + rate).powi(4);
    assert!(
        (result.value.mean - mean).abs() < 0.12,
        "{}",
        result.value.mean
    );
    assert!(
        (result.value.stdev.powi(2) - variance).abs() < 0.08,
        "{}",
        result.value.stdev.powi(2)
    );
}

#[test]
fn cholesky_correlates_inputs() {
    let inputs = [
        input("a", Distribution::Normal { mean: 0.0, sd: 1.0 }),
        input("b", Distribution::Normal { mean: 0.0, sd: 1.0 }),
    ];
    let corr = vec![vec![1.0, 0.75], vec![0.75, 1.0]];
    let result = simulate(&inputs, Some(&corr), 20_000, 99, None, 5, |x| {
        Ok(PathValue {
            value: x[0],
            returns: Some(PathReturns {
                irr: x[1],
                moic: x[1],
            }),
        })
    })
    .unwrap();
    assert!(result.tornado.iter().all(|s| s.importance <= 1.0));
    let xs: Vec<_> = result.draws.iter().map(|r| r[0]).collect();
    let ys: Vec<_> = result.draws.iter().map(|r| r[1]).collect();
    let sample_count = f64::from(u32::try_from(xs.len()).unwrap_or(u32::MAX));
    let mx = xs.iter().sum::<f64>() / sample_count;
    let my = ys.iter().sum::<f64>() / sample_count;
    let cov = xs
        .iter()
        .zip(&ys)
        .map(|(x, y)| (x - mx) * (y - my))
        .sum::<f64>()
        / sample_count;
    assert!((cov - 0.75).abs() < 0.04, "{cov}");
}

#[test]
fn simulated_lbo_returns_match_direct_corp_flows() {
    let inputs = [input(
        "exit_multiple",
        Distribution::Discrete {
            scenarios: vec![7.0, 9.0, 12.0],
            weights: vec![1.0, 2.0, 1.0],
        },
    )];
    let run = simulate(&inputs, None, 128, 2026, None, 8, |x| {
        let result = lbo(&Lbo {
            entry_ev: 1_000.0,
            debt_to_ev: 0.6,
            debt_rate: 0.08,
            fcf: vec![45.0, 55.0, 65.0, 75.0, 85.0],
            exit_metric: 140.0,
            exit_multiple: x[0],
        })?;
        Ok(PathValue {
            value: result.exit_equity,
            returns: Some(PathReturns {
                irr: result
                    .irr
                    .ok_or(FinanceError::Undefined { what: "LBO IRR" })?,
                moic: result.moic,
            }),
        })
    })
    .unwrap();

    for (draw, path) in run.draws.iter().zip(&run.outcomes) {
        let direct = lbo(&Lbo {
            entry_ev: 1_000.0,
            debt_to_ev: 0.6,
            debt_rate: 0.08,
            fcf: vec![45.0, 55.0, 65.0, 75.0, 85.0],
            exit_metric: 140.0,
            exit_multiple: draw[0],
        })
        .unwrap();
        let returns = path.returns.unwrap();
        assert_eq!(path.value, direct.exit_equity);
        assert_eq!(returns.irr, direct.irr.unwrap());
        assert_eq!(returns.moic, direct.moic);
    }
}

#[test]
fn all_distribution_families_are_supported() {
    let inputs = [
        input(
            "normal",
            Distribution::Normal {
                mean: 10.0,
                sd: 1.0,
            },
        ),
        input(
            "lognormal",
            Distribution::LogNormal {
                mu: 2.0,
                sigma: 0.2,
            },
        ),
        input(
            "triangular",
            Distribution::Triangular {
                low: 0.0,
                mode: 0.5,
                high: 1.0,
            },
        ),
        input(
            "pert",
            Distribution::Pert {
                low: 0.0,
                mode: 0.5,
                high: 1.0,
            },
        ),
        input(
            "uniform",
            Distribution::Uniform {
                low: 0.0,
                high: 1.0,
            },
        ),
        input(
            "discrete",
            Distribution::Discrete {
                scenarios: vec![1.0, 2.0],
                weights: vec![1.0, 3.0],
            },
        ),
    ];
    let sim = simulate(&inputs, None, 100, 1, None, 4, |x| {
        Ok(PathValue {
            value: x.iter().sum(),
            returns: None,
        })
    })
    .unwrap();
    assert_eq!(sim.draws.len(), 100);
    assert!(sim.draws.iter().flatten().all(|x| x.is_finite()));
}

#[test]
fn malformed_correlation_is_an_error_not_a_panic() {
    let inputs = [input(
        "x",
        Distribution::Uniform {
            low: 0.0,
            high: 1.0,
        },
    )];
    let bad = vec![vec![1.0, 0.0]];
    assert!(matches!(
        simulate(&inputs, Some(&bad), 2, 1, None, 2, |_| Ok(PathValue {
            value: 1.0,
            returns: None,
        })),
        Err(FinanceError::InvalidInput { .. })
    ));
}

proptest! {
    #[test]
    fn quantiles_are_ordered_for_finite_normal_inputs(mean in -1.0e6f64..1.0e6, sd in 0.0f64..1.0e5, seed in any::<u64>()) {
        let inputs = [input("x", Distribution::Normal { mean, sd })];
        let result = simulate(&inputs, None, 64, seed, None, 8, |x| Ok(PathValue { value: x[0], returns: None })).unwrap();
        prop_assert!(result.value.p5 <= result.value.p10);
        prop_assert!(result.value.p10 <= result.value.p50);
        prop_assert!(result.value.p50 <= result.value.p90);
        prop_assert!(result.value.p90 <= result.value.p95);
    }
}
