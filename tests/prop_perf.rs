//! Property tests for `finance_rs::perf` invariants.
#![allow(clippy::unwrap_used)]

use finance_rs::perf;
use proptest::prelude::*;

/// Realistic simple returns: never below −100%.
fn returns(min: usize) -> impl Strategy<Value = Vec<f64>> {
    prop::collection::vec(-0.99f64..0.5, min..200)
}

proptest! {
    #[test]
    fn max_drawdown_in_unit_interval(r in returns(1)) {
        let mdd = perf::max_drawdown(&r).unwrap();
        prop_assert!((-1.0..=0.0).contains(&mdd));
        for d in perf::drawdown_series(&r).unwrap() {
            prop_assert!(d >= mdd && d <= 0.0);
        }
    }

    #[test]
    fn drawdown_periods_are_consistent(r in returns(1)) {
        let periods = perf::drawdown_periods(&r).unwrap();
        let mdd = perf::max_drawdown(&r).unwrap();
        let mut last_end = None;
        for p in &periods {
            prop_assert_eq!(p.length, p.end - p.start + 1);
            prop_assert!(p.start <= p.valley && p.valley <= p.end && p.end < r.len());
            prop_assert!(p.max_drawdown >= mdd && p.max_drawdown < 0.0);
            if let Some(e) = last_end {
                prop_assert!(p.start > e + 1, "episodes are separated by a peak");
            }
            last_end = Some(p.end);
        }
        let worst = periods.iter().map(|p| p.max_drawdown).fold(0.0, f64::min);
        prop_assert!((worst - mdd).abs() < f64::EPSILON);
        let longest = periods.iter().map(|p| p.length).max().unwrap_or(0);
        prop_assert_eq!(perf::max_drawdown_duration(&r).unwrap(), longest);
    }

    #[test]
    fn volatility_non_negative(r in returns(2), ppy in 1.0f64..400.0) {
        prop_assert!(perf::volatility(&r, ppy).unwrap() >= 0.0);
    }

    #[test]
    fn sharpe_is_scale_invariant(r in returns(2), k in 0.01f64..100.0) {
        let scaled: Vec<f64> = r.iter().map(|x| x * k).collect();
        if let (Ok(a), Ok(b)) = (perf::sharpe(&r, 0.0, 252.0), perf::sharpe(&scaled, 0.0, 252.0)) {
            prop_assert!((a - b).abs() <= 1e-9 * (1.0 + a.abs()), "{a} vs {b}");
        }
    }

    #[test]
    fn win_rate_in_unit_interval(r in returns(1)) {
        let w = perf::win_rate(&r).unwrap();
        prop_assert!((0.0..=1.0).contains(&w));
    }

    #[test]
    fn cvar_not_above_var(r in returns(2), conf in 0.5f64..0.999) {
        let var_h = perf::var_historical(&r, conf).unwrap();
        prop_assert!(perf::cvar_historical(&r, conf).unwrap() <= var_h + 1e-15);
        if let Ok(var_p) = perf::var_parametric(&r, conf) {
            prop_assert!(perf::cvar_parametric(&r, conf).unwrap() <= var_p + 1e-15);
        }
    }

    #[test]
    fn beta_of_self_is_one(r in returns(2)) {
        if let Ok(b) = perf::beta(&r, &r) {
            prop_assert!((b - 1.0).abs() < 1e-12);
            prop_assert!(perf::alpha(&r, &r, 252.0).unwrap().abs() < 1e-12);
        }
    }

    #[test]
    fn cumulative_ends_at_compounded(r in returns(1)) {
        let c = perf::cumulative_returns(&r).unwrap();
        prop_assert!((*c.last().unwrap() - perf::compounded_return(&r).unwrap()).abs() < f64::EPSILON);
    }

    #[test]
    fn normal_ppf_inverts_cdf(p in 1e-12f64..(1.0 - 1e-12)) {
        let x = perf::normal_ppf(p).unwrap();
        let back = perf::normal_cdf(x);
        prop_assert!((back - p).abs() <= 2e-13 * p.min(1.0 - p).max(1e-3), "{p} -> {x} -> {back}");
    }
}
