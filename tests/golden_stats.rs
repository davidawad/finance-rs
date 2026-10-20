//! numpy / statsmodels / scipy parity for `finance_rs::stats` and
//! `finance_rs::solve` (fixtures: tests/golden/stats.json, tests/golden/solve.json).
#![allow(clippy::unwrap_used, clippy::panic, clippy::cast_possible_truncation)]

mod common;

use common::{f, fs, Golden, Tally};
use finance_rs::solve::{self, SolverOptions};
use finance_rs::stats::{self, QuantileMethod};
use serde_json::{json, Value};

fn ddof(c: &Value) -> usize {
    c["args"]["ddof"].as_u64().unwrap() as usize
}

fn run(
    fixture: &str,
    func: &str,
    reference: &str,
    eval: impl Fn(&Value) -> finance_rs::Result<f64>,
) {
    let g = Golden::load(fixture);
    let mut tally = Tally::default();
    for c in g.cases(func) {
        tally.check(c, eval(c), g.tol(func));
    }
    tally.finish(func, reference);
}

/// Compare a list-valued result element by element.
fn check_list(tally: &mut Tally, c: &Value, got: finance_rs::Result<Vec<f64>>, tol: (f64, f64)) {
    let want: Vec<f64> = c["expected"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    match got {
        Ok(g) if g.len() == want.len() => {
            for (x, w) in g.iter().zip(&want) {
                tally.check(&json!({"expected": w, "case": c}), Ok::<f64, ()>(*x), tol);
            }
        }
        other => tally
            .failures
            .push(format!("{c}: want {want:?}, got {other:?}")),
    }
}

#[test]
fn mean() {
    run("stats", "mean", "numpy 2.5.4", |c| {
        stats::mean(&fs(c, "xs"))
    });
}

#[test]
fn variance() {
    run("stats", "variance", "numpy 2.5.4", |c| {
        stats::variance(&fs(c, "xs"), ddof(c))
    });
}

#[test]
fn stdev() {
    run("stats", "stdev", "numpy 2.5.4", |c| {
        stats::stdev(&fs(c, "xs"), ddof(c))
    });
}

#[test]
fn quantile() {
    run("stats", "quantile", "numpy 2.5.4", |c| {
        let method = match c["args"]["method"].as_str().unwrap() {
            "linear" => QuantileMethod::Linear,
            "lower" => QuantileMethod::Lower,
            "higher" => QuantileMethod::Higher,
            "nearest" => QuantileMethod::Nearest,
            "midpoint" => QuantileMethod::Midpoint,
            "excel_exclusive" => QuantileMethod::ExcelExclusive,
            m => panic!("unknown method {m}"),
        };
        stats::quantile(&fs(c, "xs"), f(c, "q"), method)
    });
}

#[test]
fn covariance() {
    run("stats", "covariance", "numpy 2.5.4", |c| {
        stats::covariance(&fs(c, "x"), &fs(c, "y"), ddof(c))
    });
}

#[test]
fn correlation() {
    run("stats", "correlation", "numpy 2.5.4", |c| {
        stats::correlation(&fs(c, "x"), &fs(c, "y"))
    });
}

#[test]
fn ols() {
    let g = Golden::load("stats");
    let tol = g.tol("ols");
    let mut tally = Tally::default();
    for c in g.cases("ols") {
        let y = fs(c, "y");
        let xs: Vec<Vec<f64>> = c["args"]["xs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|col| {
                col.as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_f64().unwrap())
                    .collect()
            })
            .collect();
        let cols: Vec<&[f64]> = xs.iter().map(Vec::as_slice).collect();
        let opts = stats::OlsOptions {
            intercept: c["args"]["intercept"].as_bool().unwrap(),
        };
        let fit = stats::ols_with(&y, &cols, opts).unwrap();
        let e = &c["expected"];
        for (key, got) in [
            ("coef", &fit.coef),
            ("se", &fit.se),
            ("t", &fit.t),
            ("resid", &fit.resid),
        ] {
            check_list(
                &mut tally,
                &json!({"expected": e[key], "case": key}),
                Ok(got.clone()),
                tol,
            );
        }
        tally.check(&json!({"expected": e["r2"]}), Ok::<f64, ()>(fit.r2), tol);
        tally.check(
            &json!({"expected": e["adj_r2"]}),
            Ok::<f64, ()>(fit.adj_r2),
            tol,
        );
    }
    tally.finish("ols", "statsmodels 0.15.0");
}

fn polyval(c: &[f64], x: f64) -> f64 {
    c.iter().fold(0.0, |acc, k| acc * x + k)
}

fn polyder(c: &[f64]) -> Vec<f64> {
    let n = c.len() - 1;
    c[..n]
        .iter()
        .enumerate()
        .map(|(i, k)| k * f64::from(u32::try_from(n - i).unwrap_or(u32::MAX)))
        .collect()
}

fn family(c: &Value) -> Box<dyn Fn(f64) -> f64> {
    match c["args"]["family"].as_str().unwrap() {
        "poly" => {
            let k = fs(c, "c");
            Box::new(move |x| polyval(&k, x))
        }
        "cos_lin" => {
            let k = f(c, "k");
            Box::new(move |x| x.cos() - k * x)
        }
        "cos_map" => {
            let (a, b) = (f(c, "a"), f(c, "b"));
            Box::new(move |x| a * x.cos() + b)
        }
        "sqrt_map" => {
            let k = f(c, "c");
            Box::new(move |x| (x + k).sqrt())
        }
        other => panic!("unknown family {other}"),
    }
}

#[test]
fn brent() {
    run("solve", "brent", "scipy 1.18.1 brentq", |c| {
        solve::brent(family(c), f(c, "a"), f(c, "b"), &SolverOptions::default()).map(|s| s.root)
    });
}

#[test]
fn newton_bracketed() {
    run("solve", "newton_bracketed", "scipy 1.18.1 newton", |c| {
        let d = polyder(&fs(c, "c"));
        solve::newton_bracketed(
            family(c),
            |x| polyval(&d, x),
            f(c, "guess"),
            (f(c, "a"), f(c, "b")),
            &SolverOptions::default(),
        )
        .map(|s| s.root)
    });
}

#[test]
fn fixed_point() {
    run("solve", "fixed_point", "scipy 1.18.1 fixed_point", |c| {
        solve::fixed_point(family(c), f(c, "x0"), &SolverOptions::default()).map(|s| s.root)
    });
}

#[test]
fn scan_roots() {
    let g = Golden::load("solve");
    let tol = g.tol("scan_roots");
    let mut tally = Tally::default();
    for c in g.cases("scan_roots") {
        let (lo, hi) = (f(c, "lo"), f(c, "hi"));
        let n = c["args"]["n"].as_u64().unwrap() as u32;
        let grid: Vec<f64> = (0..n)
            .map(|i| lo + (hi - lo) * f64::from(i) / f64::from(n - 1))
            .collect();
        check_list(
            &mut tally,
            c,
            solve::scan_roots(family(c), &grid, &SolverOptions::default()),
            tol,
        );
    }
    tally.finish("scan_roots", "numpy 2.5.4 roots");
}
