//! Fund parity against the pinned pyxirr 0.10.8 `pe` module.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::{f, fs, Golden, Tally};
use finance_rs::fund;
use serde_json::Value;

fn run(func: &str, eval: impl Fn(&Value) -> finance_rs::Result<f64>) {
    let golden = Golden::load("funds");
    let mut tally = Tally::default();
    for case in golden.cases(func) {
        tally.check(case, eval(case), golden.tol(func));
    }
    tally.finish(func, "pyxirr 0.10.8 pe");
}

#[test]
fn moic() {
    run("moic", |c| {
        fund::moic(
            &fs(c, "contributions"),
            &fs(c, "distributions"),
            f(c, "nav_scalar"),
        )
    });
}
#[test]
fn ks_pme() {
    run("ks_pme", |c| {
        fund::ks_pme(
            &fs(c, "contributions"),
            &fs(c, "distributions"),
            f(c, "nav_scalar"),
            &fs(c, "index"),
        )
    });
}
#[test]
fn ln_pme_nav() {
    run("ln_pme_nav", |c| {
        fund::ln_pme_nav(
            &fs(c, "contributions"),
            &fs(c, "distributions"),
            &fs(c, "index"),
        )
    });
}
#[test]
fn ln_pme() {
    run("ln_pme", |c| {
        fund::ln_pme(
            &fs(c, "contributions"),
            &fs(c, "distributions"),
            &fs(c, "index"),
        )
    });
}
#[test]
fn pme_plus_lambda() {
    run("pme_plus_lambda", |c| {
        fund::pme_plus_lambda(
            &fs(c, "contributions"),
            &fs(c, "distributions"),
            f(c, "nav_scalar"),
            &fs(c, "index"),
        )
    });
}
#[test]
fn pme_plus() {
    run("pme_plus", |c| {
        fund::pme_plus(
            &fs(c, "contributions"),
            &fs(c, "distributions"),
            f(c, "nav_scalar"),
            &fs(c, "index"),
        )
    });
}
#[test]
fn m_pme() {
    run("m_pme", |c| {
        fund::m_pme(
            &fs(c, "contributions"),
            &fs(c, "distributions"),
            &fs(c, "index"),
            &fs(c, "nav"),
        )
    });
}
