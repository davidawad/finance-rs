//! Algebraic properties of the return and ratio additions.
#![allow(clippy::unwrap_used)]

use finance_rs::perf;
use proptest::prelude::*;

fn returns() -> impl Strategy<Value = Vec<f64>> {
    prop::collection::vec(-0.5_f64..0.5, 2..60)
}

proptest! {
    #[test]
    fn price_index_round_trips_to_returns(rs in returns(), base in 1.0_f64..1000.0) {
        let prices = perf::returns::to_price_index(&rs, base).unwrap();
        let actual = perf::returns::simple(&prices).unwrap();
        prop_assert_eq!(actual.len(), rs.len());
        for (a, b) in actual.iter().zip(&rs) {
            prop_assert!((a - b).abs() < 1e-12);
        }
    }

    #[test]
    fn rebasing_preserves_returns(ps in prop::collection::vec(1.0_f64..1e6, 2..60), base in 1.0_f64..1000.0) {
        let before = perf::returns::simple(&ps).unwrap();
        let after = perf::returns::simple(&perf::returns::rebase(&ps, base).unwrap()).unwrap();
        for (a, b) in before.iter().zip(&after) {
            // Rebased prices can be many orders of magnitude smaller than
            // the source values; use relative tolerance for the resulting
            // large returns so floating-point rounding does not make this
            // scale-invariance property flaky.
            prop_assert!((a - b).abs() <= 1e-12 * a.abs().max(b.abs()).max(1.0));
        }
    }

    #[test]
    fn twr_matches_compounded_return(rs in returns()) {
        prop_assert!((perf::returns::twr(&rs).unwrap() - perf::compounded_return(&rs).unwrap()).abs() < 1e-12);
    }

    #[test]
    fn weighted_sharpe_is_invariant_to_weight_scale(rs in returns(), ws in prop::collection::vec(0.01_f64..10.0, 2..60), scale in 0.01_f64..100.0) {
        let n = rs.len().min(ws.len());
        let (rs, ws) = (&rs[..n], &ws[..n]);
        let scaled: Vec<f64> = ws.iter().map(|w| w * scale).collect();
        let a = perf::weighted_sharpe(rs, ws).unwrap();
        let b = perf::weighted_sharpe(rs, &scaled).unwrap();
        prop_assert!((a - b).abs() < 1e-10);
    }

    #[test]
    fn exposure_is_bounded(rs in returns()) {
        prop_assert!((0.0..=1.0).contains(&perf::returns::exposure(&rs).unwrap()));
    }

    #[test]
    fn gpd_tail_risk_never_panics(rs in prop::collection::vec(-0.5_f64..0.5, 0..80), threshold in -0.5_f64..0.5) {
        let result = std::panic::catch_unwind(|| perf::gpd_tail_risk(&rs, threshold));
        prop_assert!(result.is_ok());
        if let Ok(risk) = result.unwrap() {
            prop_assert!(risk.var.is_finite());
            prop_assert!(risk.scale.is_finite());
            prop_assert!(risk.shape.is_finite());
            prop_assert!(risk.expected_shortfall.is_finite());
            prop_assert!(risk.expected_shortfall <= risk.var);
        }
    }
}
