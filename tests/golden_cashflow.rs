//! pyxirr parity for `finance_rs::cashflow` and `finance_rs::daycount`
//! (fixtures: tests/golden/cashflow.json, tests/golden/daycount.json).
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::{f, fs, Golden, Tally};
use finance_rs::cashflow::{self, XirrOptions};
use finance_rs::daycount::DayCount;
use finance_rs::Date;
use serde_json::Value;

fn date(s: &str) -> Date {
    let p: Vec<&str> = s.split('-').collect();
    Date::new(
        p[0].parse().unwrap(),
        p[1].parse().unwrap(),
        p[2].parse().unwrap(),
    )
    .unwrap()
}

fn dates(c: &Value) -> Vec<Date> {
    c["args"]["dates"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| date(d.as_str().unwrap()))
        .collect()
}

fn day_count(c: &Value) -> DayCount {
    let name = c["args"]["day_count"].as_str().unwrap();
    DayCount::from_name(name).unwrap_or_else(|| panic!("unknown day count {name}"))
}

fn run(fixture: &str, func: &str, eval: impl Fn(&Value) -> finance_rs::Result<f64>) {
    let g = Golden::load(fixture);
    let mut tally = Tally::default();
    for c in g.cases(func) {
        tally.check(c, eval(c), g.tol(func));
    }
    tally.finish(func, "pyxirr 0.10.8");
}

#[test]
fn xnpv() {
    run("cashflow", "xnpv", |c| {
        cashflow::xnpv(f(c, "rate"), &dates(c), &fs(c, "amounts"))
    });
}

#[test]
fn xnpv_day_count() {
    run("cashflow", "xnpv_day_count", |c| {
        cashflow::xnpv_with(f(c, "rate"), &dates(c), &fs(c, "amounts"), day_count(c))
    });
}

#[test]
fn xirr() {
    run("cashflow", "xirr", |c| {
        cashflow::xirr(&dates(c), &fs(c, "amounts"))
    });
}

#[test]
fn xirr_day_count() {
    run("cashflow", "xirr_day_count", |c| {
        let opts = XirrOptions {
            day_count: day_count(c),
            ..XirrOptions::default()
        };
        cashflow::xirr_with(&dates(c), &fs(c, "amounts"), opts)
    });
}

/// Whatever root pyxirr picks is one of ours.
#[test]
fn xirr_root_is_reported() {
    let g = Golden::load("cashflow");
    for c in g.cases("xirr") {
        let Some(want) = c["expected"].as_f64() else {
            continue;
        };
        let roots = cashflow::xirr_roots(&dates(c), &fs(c, "amounts"), DayCount::Act365F).unwrap();
        assert!(
            roots.iter().any(|r| (r - want).abs() < 1e-8),
            "{c}: {roots:?}"
        );
    }
}

#[test]
fn year_fraction() {
    run("daycount", "year_fraction", |c| {
        let s = date(c["args"]["start"].as_str().unwrap());
        let e = date(c["args"]["end"].as_str().unwrap());
        Ok(day_count(c).year_fraction(s, e))
    });
}
