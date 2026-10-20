//! Generalized Pareto peaks-over-threshold tail risk checks.
#![allow(clippy::unwrap_used)]

use finance_rs::perf::gpd_tail_risk;

#[test]
fn exponential_gpd_mle_has_scale_one_and_closed_form_tail_mean() {
    // This four point sample has mean 1 and second moment 2, so the
    // zero-shape (exponential) GPD is the likelihood maximum.
    let a = (3.8 + 1.52_f64.sqrt()) / 2.0;
    let b = (3.8 - 1.52_f64.sqrt()) / 2.0;
    let returns = [-0.1, -0.1, -a, -b];
    let risk = gpd_tail_risk(&returns, 0.0).unwrap();
    assert_eq!(risk.var, 0.0);
    assert!((risk.scale - 1.0).abs() < 1e-5);
    assert!(risk.shape.abs() < 1e-4);
    assert!((risk.expected_shortfall + 1.0).abs() < 1e-4);
}

#[test]
fn rejects_insufficient_or_degenerate_tail() {
    assert!(gpd_tail_risk(&[-0.1, -0.2], 0.0).is_err());
    assert!(gpd_tail_risk(&[-0.1, -0.1, -0.1], 0.0).is_err());
    assert!(gpd_tail_risk(&[-0.1, f64::NAN, -0.2, -0.3], 0.0).is_err());
}
