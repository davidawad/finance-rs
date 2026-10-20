//! Property tests for amortization schedules and the TVM pmt/pv round trip.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::float_cmp
)]

use finance_rs::amortization::{
    balloon, interest_only, level_payment, level_payment_with_extra, refinance, Refinance, Schedule,
};
use finance_rs::tvm::{pmt, pv, When};
use proptest::prelude::*;

/// The invariants every schedule must satisfy for a financed `amount`.
fn check_schedule(s: &Schedule, amount: f64) -> Result<(), TestCaseError> {
    let sum = s.summary();
    let tol = 1e-6 * amount;
    prop_assert!(
        (sum.total_principal - amount).abs() <= tol,
        "principal {} vs {amount}",
        sum.total_principal
    );
    prop_assert!(
        (sum.total_payment - sum.total_interest - sum.total_principal).abs() <= tol,
        "payment {} interest {} principal {}",
        sum.total_payment,
        sum.total_interest,
        sum.total_principal
    );
    let mut prev = s.principal;
    for r in &s.rows {
        prop_assert!(r.balance <= prev, "balance rose at {r:?}");
        prop_assert!(r.balance >= 0.0, "negative balance at {r:?}");
        prev = r.balance;
    }
    prop_assert_eq!(s.final_balance(), 0.0);
    Ok(())
}

proptest! {
    #[test]
    fn level_payment_invariants(
        amount in 1.0_f64..5_000_000.0,
        annual in 0.0_f64..0.25,
        nper in 1_u32..480,
    ) {
        let rate = annual / 12.0;
        let s = level_payment(amount, rate, nper).unwrap();
        prop_assert_eq!(s.rows.len(), nper as usize);
        check_schedule(&s, amount)?;
        let level = -pmt(rate, f64::from(nper), amount, 0.0, When::End).unwrap();
        for r in &s.rows[..s.rows.len() - 1] {
            prop_assert_eq!(r.payment, level);
        }
        let last = s.rows[s.rows.len() - 1].payment;
        prop_assert!((last - level).abs() <= 1e-6 * level.max(1.0), "last {last} vs {level}");
    }

    #[test]
    fn extra_principal_invariants(
        amount in 1_000.0_f64..1_000_000.0,
        annual in 0.0_f64..0.2,
        nper in 12_u32..360,
        extra in proptest::collection::vec(0.0_f64..5_000.0, 0..120),
    ) {
        let s = level_payment_with_extra(amount, annual / 12.0, nper, &extra).unwrap();
        prop_assert!(s.rows.len() <= nper as usize);
        check_schedule(&s, amount)?;
        let base = level_payment(amount, annual / 12.0, nper).unwrap();
        prop_assert!(s.total_interest() <= base.total_interest() * (1.0 + 1e-12) + 1e-9);
    }

    #[test]
    fn interest_only_and_balloon_invariants(
        amount in 1_000.0_f64..1_000_000.0,
        annual in 0.0_f64..0.2,
        io in 0_u32..60,
        amort in 1_u32..360,
        maturity_frac in 0.01_f64..1.0,
    ) {
        let rate = annual / 12.0;
        let s = interest_only(amount, rate, io, amort).unwrap();
        prop_assert_eq!(s.rows.len(), (io + amort) as usize);
        check_schedule(&s, amount)?;
        // Truncation to a whole period count is the intent.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let maturity = ((f64::from(amort) * maturity_frac).ceil() as u32).clamp(1, amort);
        let b = balloon(amount, rate, amort, maturity).unwrap();
        prop_assert_eq!(b.rows.len(), maturity as usize);
        check_schedule(&b, amount)?;
    }

    #[test]
    fn refinance_invariants(
        amount in 10_000.0_f64..1_000_000.0,
        annual in 0.0_f64..0.15,
        new_annual in 0.0_f64..0.15,
        at in 1_u32..359,
        new_nper in 1_u32..360,
        closing in 0.0_f64..10_000.0,
        roll_in in any::<bool>(),
    ) {
        let refi = Refinance { at_period: at, new_rate: new_annual / 12.0, new_nper, closing_costs: closing, roll_in };
        let s = refinance(amount, annual / 12.0, 360, &refi).unwrap();
        let financed = amount + if roll_in { closing } else { 0.0 };
        prop_assert_eq!(s.rows.len(), (at + new_nper) as usize);
        let sum = s.summary();
        prop_assert!((sum.total_principal - financed).abs() <= 1e-6 * financed);
        prop_assert!((sum.total_payment - sum.total_interest - sum.total_principal).abs() <= 1e-6 * financed);
        prop_assert_eq!(s.final_balance(), 0.0);
    }

    #[test]
    fn pmt_pv_round_trip(
        amount in 1.0_f64..1e7,
        rate in -0.05_f64..0.05,
        nper in 1_u32..600,
        begin in any::<bool>(),
    ) {
        let when = if begin { When::Begin } else { When::End };
        let n = f64::from(nper);
        let p = pmt(rate, n, amount, 0.0, when).unwrap();
        let back = pv(rate, n, p, 0.0, when).unwrap();
        prop_assert!((back - amount).abs() <= 1e-9 * amount, "pv {back} vs {amount}");
    }
}
