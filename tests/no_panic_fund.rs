//! No-panic coverage for the fund accounting and return helpers.
#![allow(clippy::unwrap_used)]

use finance_rs::{fund, Date};
use proptest::prelude::*;

proptest! {
    #[test]
    fn fund_helpers_handle_arbitrary_finite_and_nonfinite_inputs(
        a in any::<f64>(), b in any::<f64>(), c in any::<f64>(), d in any::<f64>(),
        flows in prop::collection::vec(any::<f64>(), 0..10),
    ) {
        let dates = [Date::new(2020, 1, 1).unwrap(), Date::new(2021, 1, 1).unwrap()];
        let schedule = fund::FeeSchedule {
            basis_by_period: vec![fund::FeeBasis::Nav; 2],
            rate_by_period: vec![a, b],
            step_downs: vec![],
            offset_pct: c,
        };
        let _ = fund::management_fee(&[a, b], &[c, d], &[b, c], &schedule);
        let _ = fund::offsets(a, b);
        let _ = fund::capital_account_roll_forward(a, b, c, fund::CapitalAllocations {
            income: d, realized: a, unrealized: b, fees: c, carry: d,
        });
        let _ = fund::capital_call_allocate(a, &[("lp".into(), b, c)]);
        let _ = fund::equalization(a, b, c, d, 365);
        let _ = fund::recallable(a, fund::RecallTerms { cap: b, recalled_to_date: c });
        let _ = fund::takahashi_alexander(a, b, c, d, 4, 0.05);
        let _ = fund::j_curve(&flows);
        let _ = fund::gp_commitment(a, b);
        let _ = fund::unfunded_coverage(a, b);
        let _ = fund::accrued_carry(a, &flows, &flows, fund::WaterfallTerms { catch_up: c, carry: d });
        let _ = fund::multi_hurdle(a, b, &[fund::Hurdle { irr_or_multiple: c, lp_split: d }], &dates);
        let _ = fund::clawback(a, b, c, d);
        let _ = fund::gross_net_irr(&dates, &[a, b], &[c, d], &[a, b], &[c, d], a, dates[1]);
        let _ = fund::twr_vs_mwr(&dates, &[a, b], &[c, d], &[a, b]);
        let _ = fund::secondary_price(&[a, b], c, &dates, d);
        let _ = fund::secondary_price_from_nav(a, b);
    }
}

#[cfg(feature = "decimal")]
#[test]
fn decimal_capital_account_errors_do_not_panic() {
    use rust_decimal::Decimal;
    let zero = Decimal::ZERO;
    let _ = fund::capital_account_roll_forward_decimal(
        zero,
        zero,
        zero,
        fund::DecimalCapitalAllocations {
            income: zero,
            realized: zero,
            unrealized: zero,
            fees: zero,
            carry: zero,
        },
    );
}
