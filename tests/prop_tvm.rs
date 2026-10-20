//! Property tests for `tvm` and `cashflow` invariants.
#![allow(clippy::unwrap_used)]

use finance_rs::cashflow::{xirr_times, xnpv_times};
use finance_rs::tvm::{fv, irr, irr_roots, npv, pmt, pv, When};
use proptest::prelude::*;

fn when() -> impl Strategy<Value = When> {
    prop_oneof![Just(When::End), Just(When::Begin)]
}

/// A conventional investment: one outflow then inflows (exactly one IRR).
fn conventional() -> impl Strategy<Value = Vec<f64>> {
    (1.0..1e6_f64, prop::collection::vec(0.0..5e5_f64, 1..40)).prop_map(|(out, ins)| {
        let mut v = vec![-out];
        v.extend(ins);
        v
    })
}

proptest! {
    #[test]
    fn npv_at_irr_is_zero(values in conventional()) {
        if let Ok(r) = irr(&values) {
            let scale: f64 = values.iter().map(|v| v.abs()).sum();
            prop_assert!(npv(r, &values).unwrap().abs() <= 1e-9 * scale, "r = {r}");
        }
    }

    #[test]
    fn every_reported_root_zeroes_npv(values in prop::collection::vec(-1e4..1e4_f64, 2..12)) {
        if let Ok(roots) = irr_roots(&values) {
            for r in roots.into_iter().filter(|r| *r > -0.95 && *r < 50.0) {
                let v = npv(r, &values).unwrap();
                // Relative to the largest discounted flow at r.
                let big = values.iter().enumerate().map(|(t, x)| {
                    x.abs() / (1.0 + r).powi(i32::try_from(t).unwrap())
                }).fold(0.0, f64::max);
                prop_assert!(v.abs() <= 1e-8 * big.max(1.0), "r = {r}, npv = {v}");
            }
        }
    }

    #[test]
    fn pmt_pv_round_trip(rate in -0.05..0.5_f64, n in 1u32..480, principal in 1.0..1e7_f64, w in when()) {
        let nper = f64::from(n);
        let p = pmt(rate, nper, principal, 0.0, w).unwrap();
        let back = pv(rate, nper, p, 0.0, w).unwrap();
        prop_assert!((back - principal).abs() <= 1e-8 * principal);
        // And the loan is fully repaid.
        prop_assert!(fv(rate, nper, p, principal, w).unwrap().abs() <= 1e-6 * principal * (1.0 + rate).powf(nper).max(1.0));
    }

    #[test]
    fn xnpv_at_xirr_is_zero(values in conventional(), gaps in prop::collection::vec(1u32..400, 40)) {
        let mut t = 0.0;
        let times: Vec<f64> = values.iter().zip(&gaps).map(|(_, g)| {
            let now = t;
            t += f64::from(*g) / 365.0;
            now
        }).collect();
        if let Ok(r) = xirr_times(&times, &values, 0.1) {
            prop_assert!(r > -1.0);
            let scale: f64 = values.iter().map(|v| v.abs()).sum();
            prop_assert!(xnpv_times(r, &times, &values).unwrap().abs() <= 1e-8 * scale, "r = {r}");
        }
    }

    #[test]
    fn periodic_xirr_equals_irr(values in conventional()) {
        #[allow(clippy::cast_precision_loss)]
        let times: Vec<f64> = (0..values.len()).map(|t| t as f64).collect();
        if let (Ok(a), Ok(b)) = (irr(&values), xirr_times(&times, &values, 0.1)) {
            prop_assert!((a - b).abs() <= 1e-9 * (1.0 + a.abs()), "{a} vs {b}");
        }
    }
}
