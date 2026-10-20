//! Bounded model checks for identities and conservation laws.

use crate::{
    amortization::level_payment,
    fund::{european_waterfall, WaterfallTerms},
    tvm::{pmt, pv, When},
};

#[kani::proof]
fn pmt_pv_round_trip_is_bounded() {
    let (rate, nper, principal) = (0.0, 1.0, 10_000.0);
    let payment = pmt(rate, nper, principal, 0.0, When::End).unwrap();
    let recovered = pv(rate, nper, payment, 0.0, When::End).unwrap();
    assert!((recovered - principal).abs() <= principal * 1e-10 + 1e-8);
}

#[kani::proof]
fn amortization_balance_and_principal_conserve() {
    let principal = 10_000.0;
    let schedule = level_payment(principal, 0.0, 1).unwrap();
    assert_eq!(schedule.final_balance(), 0.0);
    assert!(schedule.rows.iter().all(|row| row.balance >= 0.0));
    assert!((schedule.summary().total_principal - principal).abs() <= principal * 1e-10 + 1e-8);
}

#[kani::proof]
fn european_waterfall_conserves_distributions() {
    let distribution: u8 = kani::any();
    let capital: u8 = kani::any();
    let preferred: u8 = kani::any();
    kani::assume(distribution <= 3);
    kani::assume(capital <= 3);
    kani::assume(preferred <= 3);

    let distribution = f64::from(distribution);
    let capital = f64::from(capital);
    let preferred = f64::from(preferred);
    let waterfall = european_waterfall(
        distribution,
        capital,
        preferred,
        WaterfallTerms {
            catch_up: 0.25,
            carry: 0.25,
        },
    )
    .unwrap();
    assert!((waterfall.lp_total + waterfall.gp_total - distribution).abs() <= 1e-8);
}
