//! numpy-financial parity for `finance_rs::tvm` (fixtures: tests/golden/tvm.json).
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::{f, fs, Golden, Tally};
use finance_rs::tvm::{self, When};
use serde_json::Value;

fn when(c: &Value) -> When {
    match c["args"]["when"].as_str() {
        Some("begin") => When::Begin,
        _ => When::End,
    }
}

fn run(func: &str, eval: impl Fn(&Value) -> finance_rs::Result<f64>) {
    let g = Golden::load("tvm");
    let mut tally = Tally::default();
    for c in g.cases(func) {
        tally.check(c, eval(c), g.tol(func));
    }
    tally.finish(func, "numpy-financial 1.1.0");
}

#[test]
fn pmt() {
    run("pmt", |c| {
        tvm::pmt(f(c, "rate"), f(c, "nper"), f(c, "pv"), f(c, "fv"), when(c))
    });
}

#[test]
fn fv() {
    run("fv", |c| {
        tvm::fv(f(c, "rate"), f(c, "nper"), f(c, "pmt"), f(c, "pv"), when(c))
    });
}

#[test]
fn pv() {
    run("pv", |c| {
        tvm::pv(f(c, "rate"), f(c, "nper"), f(c, "pmt"), f(c, "fv"), when(c))
    });
}

#[test]
fn ipmt() {
    run("ipmt", |c| {
        tvm::ipmt(
            f(c, "rate"),
            f(c, "per"),
            f(c, "nper"),
            f(c, "pv"),
            f(c, "fv"),
            when(c),
        )
    });
}

#[test]
fn ppmt() {
    run("ppmt", |c| {
        tvm::ppmt(
            f(c, "rate"),
            f(c, "per"),
            f(c, "nper"),
            f(c, "pv"),
            f(c, "fv"),
            when(c),
        )
    });
}

#[test]
fn nper() {
    run("nper", |c| {
        tvm::nper(f(c, "rate"), f(c, "pmt"), f(c, "pv"), f(c, "fv"), when(c))
    });
}

/// Strict numpy-financial parity: Newton only, NaN where it fails.
#[test]
fn rate() {
    let strict = tvm::RateOptions {
        bracket_fallback: false,
        ..tvm::RateOptions::default()
    };
    run("rate", |c| {
        tvm::rate_with(
            f(c, "nper"),
            f(c, "pmt"),
            f(c, "pv"),
            f(c, "fv"),
            when(c),
            strict,
        )
    });
}

/// Where numpy-financial's Newton gives up, the default bracketed fallback
/// still returns a rate that solves the TVM identity.
#[test]
fn rate_fallback_solves_numpy_failures() {
    let g = Golden::load("tvm");
    let mut rescued = 0;
    for c in g
        .cases("rate")
        .into_iter()
        .filter(|c| c["expected"].is_null())
    {
        let (n, p, v, fv) = (f(c, "nper"), f(c, "pmt"), f(c, "pv"), f(c, "fv"));
        if let Ok(r) = tvm::rate(n, p, v, fv, when(c)) {
            // Relative to the identity's largest term, pv·(1+r)^n.
            let resid = tvm::fv(r, n, p, v, when(c)).unwrap() - fv;
            let scale = v.abs() * (1.0 + r).powf(n) + fv.abs();
            assert!(
                resid.abs() <= 1e-9 * scale,
                "{c}: r = {r}, residual {resid}"
            );
            rescued += 1;
        }
    }
    println!("rate fallback rescued {rescued} numpy-financial NaN cases");
}

#[test]
fn npv() {
    run("npv", |c| tvm::npv(f(c, "rate"), &fs(c, "values")));
}

#[test]
fn irr() {
    run("irr", |c| tvm::irr(&fs(c, "values")));
}

#[test]
fn mirr() {
    run("mirr", |c| {
        tvm::mirr(
            &fs(c, "values"),
            f(c, "finance_rate"),
            f(c, "reinvest_rate"),
        )
    });
}
