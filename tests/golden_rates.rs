//! `QuantLib` 1.44 / ffn 1.4.1 parity for `finance_rs::rates`
//! (fixture: tests/golden/rates.json).
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::{f, Golden, Tally};
use finance_rs::daycount::DayCount;
use finance_rs::rates::{self, Compounding, Frequency, Rate};
use finance_rs::Date;
use serde_json::Value;

fn frequency(v: &Value) -> Frequency {
    match v.as_str() {
        None | Some("annual") => Frequency::Annual,
        Some("semiannual") => Frequency::Semiannual,
        Some("quarterly") => Frequency::Quarterly,
        Some("monthly") => Frequency::Monthly,
        Some("weekly") => Frequency::Weekly,
        Some("daily") => Frequency::Daily,
        Some(other) => panic!("unknown frequency {other}"),
    }
}

fn basis(c: &Value, b: &str, fq: &str) -> Compounding {
    let freq = frequency(&c["args"][fq]);
    match c["args"][b].as_str().unwrap() {
        "simple" => Compounding::Simple,
        "continuous" => Compounding::Continuous,
        "compounded" => Compounding::Compounded(freq),
        other => panic!("unknown basis {other}"),
    }
}

fn rate(c: &Value) -> Rate {
    let dc = c["args"]["day_count"]
        .as_str()
        .map_or(DayCount::Act365F, |n| DayCount::from_name(n).unwrap());
    Rate::new(f(c, "rate"), basis(c, "basis", "frequency"), dc)
}

fn date(c: &Value, key: &str) -> Date {
    let s = c["args"][key].as_str().unwrap();
    let p: Vec<i64> = s.split('-').map(|x| x.parse().unwrap()).collect();
    Date::new(
        i32::try_from(p[0]).unwrap(),
        u32::try_from(p[1]).unwrap(),
        u32::try_from(p[2]).unwrap(),
    )
    .unwrap()
}

fn run(func: &str, reference: &str, eval: impl Fn(&Value) -> finance_rs::Result<f64>) {
    let g = Golden::load("rates");
    let mut tally = Tally::default();
    for c in g.cases(func) {
        tally.check(c, eval(c), g.tol(func));
    }
    tally.finish(func, reference);
}

const QL: &str = "QuantLib 1.44 InterestRate";

#[test]
fn compound_factor() {
    run("compound_factor", QL, |c| {
        rate(c).compound_factor(f(c, "t"))
    });
}

#[test]
fn discount_factor() {
    run("discount_factor", QL, |c| {
        rate(c).discount_factor(f(c, "t"))
    });
}

#[test]
fn equivalent() {
    run("equivalent", QL, |c| {
        rate(c)
            .equivalent(basis(c, "to_basis", "to_frequency"), f(c, "t"))
            .map(|r| r.value)
    });
}

#[test]
fn compound_factor_between() {
    run("compound_factor_between", QL, |c| {
        rate(c).compound_factor_between(date(c, "start"), date(c, "end"))
    });
}

#[test]
fn effective() {
    run("effective", QL, |c| {
        rates::effective(f(c, "rate"), f(c, "periods_per_year"))
    });
}

#[test]
fn nominal() {
    run("nominal", QL, |c| {
        rates::nominal(f(c, "rate"), f(c, "periods_per_year"))
    });
}

#[test]
fn continuous_from_discrete() {
    run("continuous_from_discrete", QL, |c| {
        rates::continuous_from_discrete(f(c, "rate"), f(c, "periods_per_year"))
    });
}

#[test]
fn discrete_from_continuous() {
    run("discrete_from_continuous", QL, |c| {
        rates::discrete_from_continuous(f(c, "rate"), f(c, "periods_per_year"))
    });
}

#[test]
fn annualize() {
    run("annualize", "ffn 1.4.1", |c| {
        rates::annualize(f(c, "rate"), f(c, "periods_per_year"))
    });
}

#[test]
fn deannualize() {
    run("deannualize", "ffn 1.4.1", |c| {
        let r = rates::deannualize(f(c, "rate"), f(c, "periods_per_year"));
        assert_eq!(
            r,
            rates::periodic_from_annual(f(c, "rate"), f(c, "periods_per_year"))
        );
        r
    });
}
