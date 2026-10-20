//! pyxirr parity for the chunk-1 TVM and dated-flow extras
//! (fixtures: `tests/golden/tvm_extras.json`, `tests/golden/cashflow_extras.json`).
#![allow(clippy::unwrap_used, clippy::panic, clippy::cast_possible_truncation)]

mod common;

use common::{f, fs, Golden, Tally};
use finance_rs::cashflow;
use finance_rs::daycount::DayCount;
use finance_rs::tvm::{self, When};
use finance_rs::Date;
use serde_json::Value;

fn date(s: &str) -> Date {
    let p: Vec<i64> = s.split('-').map(|x| x.parse().unwrap()).collect();
    Date::new(
        i32::try_from(p[0]).unwrap(),
        u32::try_from(p[1]).unwrap(),
        u32::try_from(p[2]).unwrap(),
    )
    .unwrap()
}

fn arg_date(c: &Value, key: &str) -> Date {
    date(c["args"][key].as_str().unwrap())
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
    DayCount::from_name(c["args"]["day_count"].as_str().unwrap()).unwrap()
}

fn when(c: &Value) -> When {
    if c["args"]["when"] == "begin" {
        When::Begin
    } else {
        When::End
    }
}

fn u(c: &Value, key: &str) -> u32 {
    c["args"][key].as_u64().unwrap() as u32
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
fn cumipmt() {
    run("tvm_extras", "cumipmt", |c| {
        tvm::cumipmt(
            f(c, "rate"),
            f(c, "nper"),
            f(c, "pv"),
            u(c, "start"),
            u(c, "end"),
            when(c),
        )
    });
}

#[test]
fn cumprinc() {
    run("tvm_extras", "cumprinc", |c| {
        tvm::cumprinc(
            f(c, "rate"),
            f(c, "nper"),
            f(c, "pv"),
            u(c, "start"),
            u(c, "end"),
            when(c),
        )
    });
}

#[test]
fn nfv() {
    run("tvm_extras", "nfv", |c| {
        tvm::nfv(f(c, "rate"), f(c, "nper"), &fs(c, "values"))
    });
}

fn xfv(c: &Value, dc: DayCount) -> finance_rs::Result<f64> {
    cashflow::xfv_with(
        arg_date(c, "start"),
        arg_date(c, "cash_flow_date"),
        arg_date(c, "end"),
        f(c, "cash_flow_rate"),
        f(c, "end_rate"),
        f(c, "amount"),
        dc,
    )
}

#[test]
fn xfv_default() {
    run("cashflow_extras", "xfv", |c| {
        let r = xfv(c, DayCount::Act365F);
        let plain = cashflow::xfv(
            arg_date(c, "start"),
            arg_date(c, "cash_flow_date"),
            arg_date(c, "end"),
            f(c, "cash_flow_rate"),
            f(c, "end_rate"),
            f(c, "amount"),
        );
        assert_eq!(r, plain);
        r
    });
}

#[test]
fn xfv_day_count() {
    run("cashflow_extras", "xfv_day_count", |c| xfv(c, day_count(c)));
}

#[test]
fn xnfv() {
    run("cashflow_extras", "xnfv", |c| {
        cashflow::xnfv(f(c, "rate"), &dates(c), &fs(c, "amounts"))
    });
}

#[test]
fn xnfv_day_count() {
    run("cashflow_extras", "xnfv_day_count", |c| {
        cashflow::xnfv_with(f(c, "rate"), &dates(c), &fs(c, "amounts"), day_count(c))
    });
}

#[test]
fn sign_changes_and_is_conventional() {
    let g = Golden::load("cashflow_extras");
    let mut n = 0;
    for c in g.cases("sign_changes") {
        let want: Vec<usize> = c["expected"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        assert_eq!(cashflow::sign_changes(&fs(c, "amounts")), want, "{c}");
        n += 1;
    }
    for c in g.cases("is_conventional") {
        assert_eq!(
            cashflow::is_conventional(&fs(c, "amounts")),
            c["expected"].as_bool().unwrap(),
            "{c}"
        );
        n += 1;
    }
    println!("PARITY sign_changes + is_conventional | pyxirr 0.10.8 | {n} cases | exact");
    assert!(n > 0);
}
