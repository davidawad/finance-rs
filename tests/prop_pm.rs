//! Identity and panic-safety properties for prediction markets and Kelly.
#![allow(clippy::unwrap_used)]

use finance_rs::{kelly, pm};
use proptest::prelude::*;

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * (1.0 + a.abs().max(b.abs()))
}

proptest! {
    #[test]
    fn odds_conversions_round_trip(p in 0.01..0.99_f64) {
        prop_assert!(close(pm::decimal_to_prob(pm::prob_to_decimal(p).unwrap()).unwrap(), p));
        prop_assert!(close(pm::american_to_prob(pm::prob_to_american(p).unwrap()).unwrap(), p));
        let (num, den) = pm::prob_to_fractional(p).unwrap();
        prop_assert!(close(pm::fractional_to_prob(num, den).unwrap(), p));
    }

    #[test]
    fn all_devig_methods_sum_to_one(p1 in 0.35..0.8_f64, p2 in 0.25..0.7_f64) {
        let ps = [p1, p2];
        for qs in [pm::multiplicative(&ps), pm::additive(&ps), pm::power(&ps), pm::shin(&ps)]
            .into_iter()
            .flatten()
        {
            prop_assert!(close(qs.iter().sum(), 1.0));
            prop_assert!(qs.iter().all(|q| *q >= 0.0 && *q <= 1.0));
        }
    }

    #[test]
    fn fee_and_cost_inverse(ask in 0.01..0.99_f64, rate in 0.0..0.1_f64) {
        let cost = pm::all_in_cost(ask, rate).unwrap();
        prop_assert!(close(pm::ask_of_cost(cost, rate).unwrap(), ask));
        prop_assert!(pm::fee(ask, rate).unwrap() >= 0.0);
        prop_assert!(pm::fee::kalshi(10.0, ask, rate).unwrap() >= 0.0);
        prop_assert!(pm::fee::polymarket(10.0, ask, rate).unwrap() >= 0.0);
    }

    #[test]
    fn kelly_bounds_and_fraction(p in 0.0..1.0_f64, odds in 0.01..100.0_f64, fraction in 0.0..1.0_f64) {
        let full = kelly::binary(p, odds).unwrap();
        let partial = kelly::fractional(full, fraction).unwrap();
        prop_assert!((0.0..=1.0).contains(&full));
        prop_assert!(partial <= full + 1e-14);
        prop_assert!(kelly::capped(full, 0.25).unwrap() <= 0.25);
    }

    #[test]
    fn public_functions_never_panic(a in any::<f64>(), b in any::<f64>(), c in any::<f64>(), xs in prop::collection::vec(any::<f64>(), 0..8)) {
        let _ = (pm::implied_probability(a,b), pm::overround(&xs), pm::held_side_price(a,pm::Side::No));
        let _ = (pm::fee(a,b), pm::all_in_cost(a,b), pm::ask_of_cost(a,b), pm::kelly_fraction(a,b,c));
        let _ = (pm::ev(a,b,c), pm::breakeven_probability(a,b), pm::annualized_return(a,b,c));
        let _ = pm::arbitrage(a,b,c);
        let _ = (pm::decimal_to_prob(a), pm::prob_to_decimal(a), pm::american_to_prob(a), pm::prob_to_american(a));
        let _ = (pm::fractional_to_prob(a,b), pm::prob_to_fractional(a));
        let _ = (pm::multiplicative(&xs), pm::additive(&xs), pm::power(&xs), pm::shin(&xs));
        let _ = (pm::brier(&xs,&xs), pm::log_loss(&xs,&xs));
        let _ = (pm::fee::kalshi(a,b,c), pm::fee::polymarket(a,b,c));
        let _ = (kelly::binary_raw(a,b), kelly::binary(a,b), kelly::fractional(a,b), kelly::capped(a,b), kelly::continuous(a,b,c));
        let _ = kelly::multi_outcome(&xs,&xs);
    }
}

#[test]
fn simultaneous_kelly_textbook_cases() {
    assert_eq!(
        kelly::multi_outcome(&[0.5, 0.5], &[2.0, 2.0]).unwrap(),
        vec![0.0, 0.0]
    );
    let fractions = kelly::multi_outcome(&[0.6, 0.2], &[2.0, 3.0]).unwrap();
    assert!(fractions.iter().all(|f| *f >= 0.0));
    assert!(fractions.iter().sum::<f64>() < 1.0);
    assert!(kelly::multi_outcome(&[0.6, 0.6], &[2.0, 2.0]).is_err());
}
