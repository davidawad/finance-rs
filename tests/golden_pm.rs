//! Closed-form parity for the prediction-market and Kelly modules.
#![allow(clippy::unwrap_used, clippy::panic)]

mod common;

use common::{f, fs, Golden};
use finance_rs::{kelly, pm, FinanceError};
use serde_json::Value;

fn close(got: f64, want: f64, atol: f64, rtol: f64) {
    assert!(
        (got - want).abs() <= atol + rtol * want.abs(),
        "got {got}, want {want}"
    );
}

#[test]
fn closed_form_fixture_parity() {
    let g = Golden::load("pm");
    for c in g.doc["cases"].as_array().unwrap() {
        let func = c["fn"].as_str().unwrap();
        let a = &c["args"];
        let result: Result<Value, FinanceError> = match func {
            "binary_raw" => {
                kelly::binary_raw(f(c, "probability"), f(c, "net_odds")).map(Into::into)
            }
            "binary" => kelly::binary(f(c, "probability"), f(c, "net_odds")).map(Into::into),
            "fractional" => {
                kelly::fractional(f(c, "full_fraction"), f(c, "fraction")).map(Into::into)
            }
            "capped" => kelly::capped(f(c, "fraction"), f(c, "cap")).map(Into::into),
            "continuous" => {
                kelly::continuous(f(c, "mu"), f(c, "sigma"), f(c, "risk_free")).map(Into::into)
            }
            "multiplicative" => pm::multiplicative(&fs(c, "probabilities")).map(Into::into),
            "additive" => pm::additive(&fs(c, "probabilities")).map(Into::into),
            "power" => pm::power(&fs(c, "probabilities")).map(Into::into),
            "shin" => pm::shin(&fs(c, "probabilities")).map(Into::into),
            "decimal_to_prob" => pm::decimal_to_prob(f(c, "decimal_odds")).map(Into::into),
            "american_to_prob" => pm::american_to_prob(f(c, "american_odds")).map(Into::into),
            "fractional_to_prob" => {
                pm::fractional_to_prob(f(c, "numerator"), f(c, "denominator")).map(Into::into)
            }
            "fee" => pm::fee(f(c, "ask"), f(c, "rate")).map(Into::into),
            "kalshi" => {
                pm::fee::kalshi(f(c, "contracts"), f(c, "price"), f(c, "rate")).map(Into::into)
            }
            "polymarket" => {
                pm::fee::polymarket(f(c, "contracts"), f(c, "price"), f(c, "rate")).map(Into::into)
            }
            "held_side_price" => pm::held_side_price(
                f(c, "yes_price"),
                if a["side"] == "yes" {
                    pm::Side::Yes
                } else {
                    pm::Side::No
                },
            )
            .map(Into::into),
            "implied_probability" => {
                pm::implied_probability(f(c, "price"), f(c, "payout")).map(Into::into)
            }
            "overround" => pm::overround(&fs(c, "values")).map(Into::into),
            "all_in_cost" => pm::all_in_cost(f(c, "ask"), f(c, "fee_rate")).map(Into::into),
            "ask_of_cost" => pm::ask_of_cost(f(c, "cost"), f(c, "fee_rate")).map(Into::into),
            "ev" => pm::ev(f(c, "p_true"), f(c, "price"), f(c, "fee")).map(Into::into),
            "breakeven_probability" => {
                pm::breakeven_probability(f(c, "price"), f(c, "fee")).map(Into::into)
            }
            "annualized_return" => {
                pm::annualized_return(f(c, "ev"), f(c, "cost"), f(c, "days")).map(Into::into)
            }
            "arbitrage" => pm::arbitrage(f(c, "yes_ask"), f(c, "no_ask"), f(c, "fees"))
                .map(|v| v.unwrap().into()),
            "kelly_fraction" => {
                pm::kelly_fraction(f(c, "p_true"), f(c, "cost"), f(c, "payout")).map(Into::into)
            }
            "brier" => pm::brier(&fs(c, "forecasts"), &fs(c, "outcomes")).map(Into::into),
            "log_loss" => pm::log_loss(&fs(c, "forecasts"), &fs(c, "outcomes")).map(Into::into),
            other => panic!("unknown fixture function {other}"),
        };
        let value = result.unwrap_or_else(|e| panic!("{func} failed: {e}"));
        let tol = g.tol(func);
        if let Some(want) = c["expected"].as_f64() {
            close(value.as_f64().unwrap(), want, tol.0, tol.1);
        } else {
            for (actual, want) in value
                .as_array()
                .unwrap()
                .iter()
                .zip(c["expected"].as_array().unwrap())
            {
                close(
                    actual.as_f64().unwrap(),
                    want.as_f64().unwrap(),
                    tol.0,
                    tol.1,
                );
            }
        }
    }
}
