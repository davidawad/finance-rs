//! Parity for the second portfolio/risk group.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::{f, fs, Golden, Tally};
use finance_rs::{perf, FinanceError};
use serde_json::Value;

const REFS: &str = "quantstats 0.0.86 / empyrical-reloaded 0.5.12 / ffn 1.4.1";

fn scalar(name: &str, run: impl Fn(&Value) -> Result<f64, FinanceError>) {
    let g = Golden::load("perf_extra");
    let mut tally = Tally::default();
    for case in g.cases(name) {
        tally.check(case, run(case), g.tol(name));
    }
    tally.finish(name, REFS);
}

#[test]
fn downside_deviation() {
    scalar("downside_deviation", |c| {
        perf::downside_deviation(&fs(c, "returns"), 0.0, f(c, "periods_per_year"))
    });
}
#[test]
fn skew() {
    scalar("skew", |c| perf::skew(&fs(c, "returns")));
}
#[test]
fn kurtosis() {
    scalar("kurtosis", |c| perf::kurtosis(&fs(c, "returns")));
}
#[test]
fn smart_sharpe() {
    scalar("smart_sharpe", |c| {
        perf::smart_sharpe(&fs(c, "returns"), f(c, "rf"), f(c, "periods_per_year"))
    });
}
#[test]
fn smart_sortino() {
    scalar("smart_sortino", |c| {
        perf::smart_sortino(&fs(c, "returns"), f(c, "rf"), f(c, "periods_per_year"))
    });
}
#[test]
fn adjusted_sortino() {
    scalar("adjusted_sortino", |c| {
        perf::adjusted_sortino(&fs(c, "returns"), f(c, "rf"), f(c, "periods_per_year"))
    });
}
#[test]
fn probabilistic_sharpe() {
    scalar("probabilistic_sharpe", |c| {
        perf::probabilistic_sharpe(&fs(c, "returns"), 0.0)
    });
}
#[test]
fn expected_max_sharpe() {
    scalar("expected_max_sharpe", |c| {
        perf::expected_max_sharpe(
            usize::try_from(c["args"]["n_trials"].as_u64().unwrap()).unwrap_or(usize::MAX),
            f(c, "sr_variance"),
        )
    });
}

#[test]
fn weekly_return_aggregation_uses_gregorian_ordinals() {
    let dates = [
        finance_rs::Date::new(2024, 2, 4).unwrap(),
        finance_rs::Date::new(2024, 2, 11).unwrap(),
    ];
    let weekly =
        perf::returns::aggregate(&dates, &[0.1, 0.1], perf::returns::Period::Week).unwrap();
    assert_eq!(weekly.len(), 2);
}
#[test]
fn deflated_sharpe() {
    scalar("deflated_sharpe", |c| {
        perf::deflated_sharpe(
            &fs(c, "returns"),
            usize::try_from(c["args"]["n_trials"].as_u64().unwrap()).unwrap_or(usize::MAX),
            f(c, "sr_variance"),
        )
    });
}
#[test]
fn risk_return_ratio() {
    scalar("risk_return_ratio", |c| {
        perf::risk_return_ratio(&fs(c, "returns"))
    });
}
#[test]
fn gain_to_pain() {
    scalar("gain_to_pain", |c| perf::gain_to_pain(&fs(c, "returns")));
}
#[test]
fn payoff_ratio() {
    scalar("payoff_ratio", |c| perf::payoff_ratio(&fs(c, "returns")));
}
#[test]
fn profit_factor() {
    scalar("profit_factor", |c| perf::profit_factor(&fs(c, "returns")));
}
#[test]
fn profit_ratio() {
    scalar("profit_ratio", |c| perf::profit_ratio(&fs(c, "returns")));
}
#[test]
fn tail_ratio() {
    scalar("tail_ratio", |c| perf::tail_ratio(&fs(c, "returns")));
}
#[test]
fn common_sense_ratio() {
    scalar("common_sense_ratio", |c| {
        perf::common_sense_ratio(&fs(c, "returns"))
    });
}
#[test]
fn cpc_index() {
    scalar("cpc_index", |c| perf::cpc_index(&fs(c, "returns")));
}
#[test]
fn outlier_win_ratio() {
    scalar("outlier_win_ratio", |c| {
        perf::outlier_win_ratio(&fs(c, "returns"))
    });
}
#[test]
fn outlier_loss_ratio() {
    scalar("outlier_loss_ratio", |c| {
        perf::outlier_loss_ratio(&fs(c, "returns"))
    });
}
#[test]
fn stability() {
    scalar("stability", |c| perf::stability(&fs(c, "returns")));
}
