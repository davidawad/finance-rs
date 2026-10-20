//! Golden parity for `finance_rs::perf` against quantstats 0.0.86 and
//! empyrical-reloaded 0.5.12 (fixture: tests/golden/perf.json).
#![allow(clippy::unwrap_used, clippy::panic, clippy::cast_precision_loss)]

mod common;

use common::{f, fs, Golden, Tally};
use finance_rs::perf;
use finance_rs::FinanceError;
use serde_json::{json, Value};

const QS: &str = "quantstats 0.0.86";
const EP: &str = "empyrical-reloaded 0.5.12";

fn golden() -> Golden {
    Golden::load("perf")
}

/// Check a scalar function over every case of `func`.
fn scalar(func: &str, reference: &str, run: impl Fn(&Value) -> Result<f64, FinanceError>) {
    let g = golden();
    let tol = g.tol(func);
    let mut t = Tally::default();
    for case in g.cases(func) {
        t.check(case, run(case), tol);
    }
    t.finish(func, reference);
}

/// Check a vector-valued function element by element.
fn vector(func: &str, run: impl Fn(&Value) -> Result<Vec<f64>, FinanceError>) {
    let g = golden();
    let tol = g.tol(func);
    let mut t = Tally::default();
    for case in g.cases(func) {
        let want = case["expected"].as_array().unwrap();
        let got = run(case).unwrap();
        assert_eq!(
            got.len(),
            want.len(),
            "{func}: length in {}",
            case["args"]["series"]
        );
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            let elem = json!({"fn": func, "series": case["args"]["series"], "i": i, "expected": w});
            t.check(&elem, Ok::<f64, FinanceError>(*g), tol);
        }
    }
    t.finish(func, QS);
}

#[test]
fn compounded_return() {
    scalar("compounded_return", QS, |c| {
        perf::compounded_return(&fs(c, "returns"))
    });
}

#[test]
fn cumulative_returns() {
    vector("cumulative_returns", |c| {
        perf::cumulative_returns(&fs(c, "returns"))
    });
}

#[test]
fn volatility() {
    scalar("volatility", QS, |c| {
        perf::volatility(&fs(c, "returns"), f(c, "periods_per_year"))
    });
}

#[test]
fn cagr() {
    scalar("cagr", QS, |c| {
        perf::cagr(&fs(c, "returns"), f(c, "periods_per_year"))
    });
}

#[test]
fn calmar() {
    scalar("calmar", QS, |c| {
        perf::calmar(&fs(c, "returns"), f(c, "periods_per_year"))
    });
}

#[test]
fn sharpe() {
    scalar("sharpe", QS, |c| {
        perf::sharpe(&fs(c, "returns"), f(c, "rf"), f(c, "periods_per_year"))
    });
}

#[test]
fn sortino() {
    scalar("sortino", QS, |c| {
        perf::sortino(&fs(c, "returns"), f(c, "rf"), f(c, "periods_per_year"))
    });
}

#[test]
fn omega() {
    scalar("omega", QS, |c| {
        perf::omega(
            &fs(c, "returns"),
            f(c, "rf"),
            f(c, "required_return"),
            f(c, "periods_per_year"),
        )
    });
}

#[test]
fn win_rate() {
    scalar("win_rate", QS, |c| perf::win_rate(&fs(c, "returns")));
}

#[test]
fn max_drawdown() {
    scalar("max_drawdown", QS, |c| {
        perf::max_drawdown(&fs(c, "returns"))
    });
}

#[test]
fn drawdown_series() {
    vector("drawdown_series", |c| {
        perf::drawdown_series(&fs(c, "returns"))
    });
}

#[test]
fn max_drawdown_duration() {
    scalar("max_drawdown_duration", QS, |c| {
        perf::max_drawdown_duration(&fs(c, "returns")).map(|d| d as f64)
    });
}

/// Rows are `[start, valley, end, days, max_drawdown]`; every field is
/// compared (positions exactly, depth within tolerance).
#[test]
fn drawdown_details() {
    let func = "drawdown_details";
    let g = golden();
    let tol = g.tol(func);
    let mut t = Tally::default();
    for case in g.cases(func) {
        let want = case["expected"].as_array().unwrap();
        let got = perf::drawdown_periods(&fs(case, "returns")).unwrap();
        let series = &case["args"]["series"];
        assert_eq!(got.len(), want.len(), "{func}: episode count in {series}");
        for (p, w) in got.iter().zip(want) {
            let w: Vec<f64> = w
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_f64().unwrap())
                .collect();
            let fields = [
                p.start as f64,
                p.valley as f64,
                p.end as f64,
                p.length as f64,
                p.max_drawdown,
            ];
            for (k, (g, w)) in fields.iter().zip(&w).enumerate() {
                let elem = json!({"fn": func, "series": series, "field": k, "expected": w});
                let atol = if k < 4 { (0.0, 0.0) } else { tol };
                t.check(&elem, Ok::<f64, FinanceError>(*g), atol);
            }
            assert!(!p.recovered || p.end + 1 < fs(case, "returns").len());
        }
    }
    t.finish(func, QS);
}

#[test]
fn var_parametric() {
    scalar("var_parametric", QS, |c| {
        perf::var_parametric(&fs(c, "returns"), f(c, "confidence"))
    });
}

#[test]
fn var_historical() {
    scalar("var_historical", EP, |c| {
        perf::var_historical(&fs(c, "returns"), f(c, "confidence"))
    });
}

#[test]
fn cvar_parametric() {
    scalar("cvar_parametric", QS, |c| {
        perf::cvar_parametric(&fs(c, "returns"), f(c, "confidence"))
    });
}

#[test]
fn cvar_historical() {
    scalar("cvar_historical", QS, |c| {
        perf::cvar_historical(&fs(c, "returns"), f(c, "confidence"))
    });
}

#[test]
fn beta() {
    scalar("beta", QS, |c| {
        perf::beta(&fs(c, "returns"), &fs(c, "benchmark"))
    });
}

#[test]
fn alpha() {
    scalar("alpha", QS, |c| {
        perf::alpha(
            &fs(c, "returns"),
            &fs(c, "benchmark"),
            f(c, "periods_per_year"),
        )
    });
}

#[test]
fn information_ratio() {
    scalar("information_ratio", QS, |c| {
        perf::information_ratio(&fs(c, "returns"), &fs(c, "benchmark"))
    });
}
