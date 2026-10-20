//! Property tests for the chunk-1 core: stats, solvers, rates, the TVM and
//! dated-flow extras and `CashFlowStream`. Each checks a defining identity.
#![allow(clippy::unwrap_used)]

use finance_rs::cashflow::{self, CashFlow, CashFlowStream, FlowKind};
use finance_rs::daycount::DayCount;
use finance_rs::period::{Period, TimeUnit};
use finance_rs::rates::{self, Compounding, Frequency, Rate};
use finance_rs::solve::{self, SolverOptions};
use finance_rs::stats::{self, QuantileMethod};
use finance_rs::tvm::{self, When};
use finance_rs::Date;
use proptest::prelude::*;

fn close(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol * (1.0 + a.abs().max(b.abs()))
}

fn returns(min: usize) -> impl Strategy<Value = Vec<f64>> {
    prop::collection::vec(-0.2..0.2_f64, min..40)
}

fn basis() -> impl Strategy<Value = Compounding> {
    prop_oneof![
        Just(Compounding::Simple),
        Just(Compounding::Continuous),
        Just(Compounding::Compounded(Frequency::Annual)),
        Just(Compounding::Compounded(Frequency::Semiannual)),
        Just(Compounding::Compounded(Frequency::Monthly)),
        Just(Compounding::Compounded(Frequency::Daily)),
        Just(Compounding::Compounded(Frequency::PerYear(3))),
    ]
}

fn dated(n: usize) -> impl Strategy<Value = (Vec<Date>, Vec<f64>)> {
    (
        prop::collection::vec(0i64..4_000, n),
        prop::collection::vec(-1_000.0..1_000.0_f64, n),
    )
        .prop_map(|(offs, amts)| {
            let base = Date::new(2015, 1, 1).unwrap().ordinal();
            let dates = offs
                .iter()
                .map(|o| Date::from_ordinal(base + o).unwrap())
                .collect();
            (dates, amts)
        })
}

proptest! {
    #[test]
    fn moments(xs in returns(2)) {
        let n = f64::from(u32::try_from(xs.len()).unwrap_or(u32::MAX));
        let pop = stats::variance(&xs, 0).unwrap();
        let sample = stats::variance(&xs, 1).unwrap();
        prop_assert!(close(pop * n, sample * (n - 1.0), 1e-12));
        prop_assert!(close(stats::stdev(&xs, 1).unwrap().powi(2), sample, 1e-12));
        prop_assert!(close(stats::covariance(&xs, &xs, 1).unwrap(), sample, 1e-12));
    }

    #[test]
    fn correlation_is_bounded_and_affine_invariant(xs in returns(3), ys in returns(3), a in 0.1..5.0_f64, b in -1.0..1.0_f64) {
        let n = xs.len().min(ys.len());
        let (x, y) = (&xs[..n], &ys[..n]);
        if let Ok(r) = stats::correlation(x, y) {
            prop_assert!((-1.0..=1.0).contains(&r));
            let moved: Vec<f64> = x.iter().map(|v| a * v + b).collect();
            prop_assert!(close(stats::correlation(&moved, y).unwrap(), r, 1e-9));
            let flipped: Vec<f64> = x.iter().map(|v| -v).collect();
            prop_assert!(close(stats::correlation(&flipped, y).unwrap(), -r, 1e-12));
        }
    }

    #[test]
    fn quantiles_are_monotone_and_bounded(xs in returns(1), q1 in 0.0..1.0_f64, q2 in 0.0..1.0_f64) {
        let (lo, hi) = (q1.min(q2), q1.max(q2));
        let min = xs.iter().copied().fold(f64::INFINITY, f64::min);
        let max = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        for m in [QuantileMethod::Linear, QuantileMethod::Lower, QuantileMethod::Higher,
                  QuantileMethod::Nearest, QuantileMethod::Midpoint] {
            let (a, b) = (stats::quantile(&xs, lo, m).unwrap(), stats::quantile(&xs, hi, m).unwrap());
            prop_assert!(a <= b + 1e-15 && min <= a && b <= max, "{m:?}");
        }
        prop_assert!((stats::quantile(&xs, 0.0, QuantileMethod::Linear).unwrap() - min).abs() < f64::EPSILON);
        prop_assert!((stats::quantile(&xs, 1.0, QuantileMethod::Linear).unwrap() - max).abs() < f64::EPSILON);
    }

    #[test]
    fn simple_ols_slope_is_cov_over_var(x in returns(4), noise in returns(4), a in -1.0..1.0_f64, b in -3.0..3.0_f64) {
        let n = x.len().min(noise.len());
        let x = &x[..n];
        let y: Vec<f64> = x.iter().zip(&noise).map(|(v, e)| a + b * v + e).collect();
        if let Ok(fit) = stats::ols(&y, &[x]) {
            let slope = stats::covariance(x, &y, 1).unwrap() / stats::variance(x, 1).unwrap();
            prop_assert!(close(fit.coef[1], slope, 1e-8));
            let dot: f64 = fit.resid.iter().zip(x).map(|(e, v)| e * v).sum();
            prop_assert!(dot.abs() < 1e-9 && fit.resid.iter().sum::<f64>().abs() < 1e-9);
            for j in 0..2 {
                prop_assert!(close(fit.t[j] * fit.se[j], fit.coef[j], 1e-12));
            }
        }
    }

    #[test]
    fn solvers_agree(r in -5.0..5.0_f64, w in 0.1..3.0_f64, s in 0.2..2.0_f64) {
        let f = |x: f64| s * (x - r) * ((x - r) * (x - r) + 1.0);
        let df = |x: f64| s * (3.0 * (x - r) * (x - r) + 1.0);
        let o = SolverOptions::default();
        let b = solve::brent(f, r - w, r + 2.0 * w, &o).unwrap();
        let nb = solve::newton_bracketed(f, df, r + w, (r - w, r + 2.0 * w), &o).unwrap();
        prop_assert!((b.root - r).abs() < 1e-11 && (nb.root - r).abs() < 1e-11);
        let (lo, hi) = solve::expand_bracket(f, 0.0, (-100.0, 100.0)).unwrap();
        prop_assert!(lo <= r && r <= hi);
        let grid: Vec<f64> = (0..=200).map(|i| -10.0 + 0.1 * f64::from(i)).collect();
        let roots = solve::scan_roots(f, &grid, &o).unwrap();
        prop_assert!(roots.len() == 1 && (roots[0] - r).abs() < 1e-11);
    }

    #[test]
    fn fixed_point_is_fixed(a in -0.9..0.9_f64, b in -2.0..2.0_f64, x0 in -5.0..5.0_f64) {
        let g = |x: f64| a * x.cos() + b;
        let s = solve::fixed_point(g, x0, &SolverOptions::default()).unwrap();
        prop_assert!((g(s.root) - s.root).abs() < 1e-10 && s.residual.abs() < 1e-10);
    }

    #[test]
    fn rate_conversions_round_trip(r in -0.05..0.5_f64, t in 0.01..30.0_f64, from in basis(), to in basis()) {
        let rate = Rate::new(r, from, DayCount::Act365F);
        // A simple rate below −1/t has no positive compound factor.
        prop_assume!(from != Compounding::Simple || r * t > -0.9);
        let cf = rate.compound_factor(t).unwrap();
        prop_assert!(close(cf * rate.discount_factor(t).unwrap(), 1.0, 1e-14));
        let there = rate.equivalent(to, t).unwrap();
        prop_assert!(close(there.compound_factor(t).unwrap(), cf, 1e-11));
        prop_assert!(close(there.equivalent(from, t).unwrap().value, r, 1e-9));
    }

    #[test]
    fn simple_then_compounded_is_its_definition(r in -0.05..0.5_f64, t in 0.0..10.0_f64) {
        let f = Frequency::Quarterly;
        let got = Compounding::SimpleThenCompounded(f).compound_factor(r, t).unwrap();
        let want = if t <= 0.25 { Compounding::Simple } else { Compounding::Compounded(f) };
        prop_assert!((got - want.compound_factor(r, t).unwrap()).abs() < f64::EPSILON);
    }

    #[test]
    fn scalar_conversions_invert(r in -0.2..1.0_f64, m in 1.0..400.0_f64, pi in -0.5..0.5_f64) {
        prop_assert!(close(rates::nominal(rates::effective(r, m).unwrap(), m).unwrap(), r, 1e-12));
        // (1+r)^m − 1 near −1 loses the digits of 1+r; keep the round trip well conditioned.
        let m = m.min(4.0 / r.ln_1p().abs().max(1e-3));
        let rc = rates::continuous_from_discrete(r, m).unwrap();
        prop_assert!(close(rates::discrete_from_continuous(rc, m).unwrap(), r, 1e-12));
        prop_assert!(close(rates::deannualize(rates::annualize(r, m).unwrap(), m).unwrap(), r, 1e-12));
        let real = rates::real_rate(r, pi).unwrap();
        prop_assert!(close((1.0 + real) * (1.0 + pi), 1.0 + r, 1e-14));
    }

    #[test]
    fn cumulative_parts_sum_to_payments(r in -0.01..0.05_f64, n in 2u32..240, pv in 100.0..1e6_f64, a in 1u32..240, b in 1u32..240, begin in any::<bool>()) {
        let w = if begin { When::Begin } else { When::End };
        let nper = f64::from(n);
        let (s, e) = (a.min(b).min(n), a.max(b).min(n));
        let i = tvm::cumipmt(r, nper, pv, s, e, w).unwrap();
        let p = tvm::cumprinc(r, nper, pv, s, e, w).unwrap();
        let pmt = tvm::pmt(r, nper, pv, 0.0, w).unwrap();
        prop_assert!(close(i + p, f64::from(e - s + 1) * pmt, 1e-9));
        prop_assert!(close(tvm::cumprinc(r, nper, pv, 1, n, w).unwrap(), -pv, 1e-9));
    }

    #[test]
    fn growing_annuities_match_their_sums(c in 1.0..1e4_f64, r in 0.0..0.3_f64, g in -0.1..0.2_f64, n in 0u32..60) {
        let nper = f64::from(n);
        let explicit: f64 = (1..=n).map(|t| c * (1.0 + g).powi(i32::try_from(t).unwrap_or(i32::MAX) - 1) / (1.0 + r).powi(i32::try_from(t).unwrap_or(i32::MAX))).sum();
        let pv = tvm::growing_annuity_pv(c, r, g, nper, When::End).unwrap();
        prop_assert!(close(pv, explicit, 1e-10));
        let due = tvm::growing_annuity_pv(c, r, g, nper, When::Begin).unwrap();
        prop_assert!(close(due, explicit * (1.0 + r), 1e-10));
        let fv = tvm::growing_annuity_fv(c, r, g, nper, When::End).unwrap();
        prop_assert!(close(fv, pv * (1.0 + r).powi(i32::try_from(n).unwrap_or(i32::MAX)), 1e-10));
        if r > g + 0.01 {
            let far = tvm::growing_annuity_pv(c, r, g, 1e5, When::End).unwrap();
            prop_assert!(close(far, tvm::perpetuity_pv(c, r, g, When::End).unwrap(), 1e-9));
        }
    }

    #[test]
    fn schedule_and_npv_identities(rates_v in prop::collection::vec(-0.05..0.2_f64, 0..20), r in -0.05..0.3_f64, v in prop::collection::vec(-500.0..500.0_f64, 2..20)) {
        let prod: f64 = rates_v.iter().map(|x| 1.0 + x).product();
        prop_assert!(close(tvm::fv_schedule(100.0, &rates_v).unwrap(), 100.0 * prod, 1e-12));
        let npv = tvm::npv(r, &v).unwrap();
        let n = f64::from(u32::try_from(v.len() - 1).unwrap_or(u32::MAX));
        prop_assert!(close(tvm::nfv(r, n + 3.0, &v).unwrap(), npv * (1.0 + r).powf(n + 2.0), 1e-10));
        let eaa = tvm::equivalent_annual_annuity(r, &v).unwrap();
        prop_assert!(close(tvm::pv(r, n, -eaa, 0.0, When::End).unwrap(), npv, 1e-9));
        let scale: f64 = v.iter().map(|x| x.abs()).sum();
        if let Ok(pi) = tvm::profitability_index(r, &v) {
            if npv.abs() > 1e-9 * scale {
                prop_assert_eq!(pi > 1.0, npv > 0.0);
            }
        }
    }

    #[test]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Payback returns a nonnegative period bounded by the generated flow sequence.
    fn payback_recovers_the_outlay(outlay in 100.0..1e4_f64, v in prop::collection::vec(0.0..2e3_f64, 1..20)) {
        let mut flows = vec![-outlay];
        flows.extend(&v);
        let p = tvm::payback_period(&flows).unwrap();
        prop_assert_eq!(p, tvm::discounted_payback(0.0, &flows).unwrap());
        if let Some(t) = p {
            let k = t.floor() as usize;
            let before: f64 = flows[..=k].iter().sum();
            let frac = t - t.floor();
            let at = if frac == 0.0 { before } else { before + frac * flows[k + 1] };
            prop_assert!(at.abs() < 1e-9 * outlay);
            prop_assert!(tvm::discounted_payback(0.1, &flows).unwrap().is_none_or(|d| d >= t));
        }
    }

    #[test]
    fn dated_extras((d, a) in dated(6), r in -0.1..0.3_f64) {
        let npv = cashflow::xnpv(r, &d, &a).unwrap();
        let times = cashflow::year_fractions(&d, DayCount::Act365F).unwrap();
        let horizon = times.iter().copied().fold(0.0, f64::max);
        prop_assert!(close(cashflow::xnfv(r, &d, &a).unwrap(), npv * (1.0 + r).powf(horizon), 1e-10));
        let changes = cashflow::sign_changes(&a);
        prop_assert_eq!(cashflow::is_conventional(&a), changes.len() == 1);
        if let Ok(dur) = cashflow::macaulay_duration_dated(r, &d, &a, DayCount::Act365F) {
            // D = −(1+r)·P′(r)/P(r), by a central difference.
            let h = 1e-6;
            let dp = (cashflow::xnpv(r + h, &d, &a).unwrap() - cashflow::xnpv(r - h, &d, &a).unwrap()) / (2.0 * h);
            if npv.abs() > 1.0 {
                prop_assert!(close(dur, -(1.0 + r) * dp / npv, 1e-5));
            }
        }
        let fv = cashflow::xfv(d[0], d[1], d[2], r, r, a[0]).unwrap();
        let t = DayCount::Act365F.year_fraction(d[1], d[2]);
        prop_assert!(close(fv, a[0] * (1.0 + r).powf(t), 1e-10));
    }

    #[test]
    fn streams_compose((d, a) in dated(8), (d2, a2) in dated(4), k in -3.0..3.0_f64) {
        let s = CashFlowStream::from_slices(&d, &a, FlowKind::Distribution).unwrap();
        let o = CashFlowStream::from_slices(&d2, &a2, FlowKind::Fee).unwrap();
        prop_assert!(s.flows().windows(2).all(|w| w[0].date <= w[1].date));
        let m = s.merge(&o);
        prop_assert_eq!(m.len(), 12);
        prop_assert!(close(m.total().unwrap(), s.total().unwrap() + o.total().unwrap(), 1e-12));
        let net = m.net_by_date();
        prop_assert!(close(net.total().unwrap(), m.total().unwrap(), 1e-12));
        prop_assert!(net.flows().windows(2).all(|w| w[0].date < w[1].date));
        prop_assert!(close(s.scale(k).unwrap().total().unwrap(), k * s.total().unwrap(), 1e-12));
        let (inflows, outflows) = m.split_signs();
        prop_assert!(close(inflows.total().unwrap() + outflows.total().unwrap(), m.total().unwrap(), 1e-12));
        prop_assert_eq!(m.filter(&[FlowKind::Fee]).len(), 4);
        let first = s.flows()[0].date;
        let r = Rate::new(0.07, Compounding::Compounded(Frequency::Annual), DayCount::Act365F);
        prop_assert!(close(s.pv(&r, first).unwrap(), s.xnpv(0.07, DayCount::Act365F).unwrap(), 1e-12));
        let week = s.shift(Period::new(7, TimeUnit::Days)).unwrap();
        prop_assert!(close(week.xnpv(0.07, DayCount::Act365F).unwrap(), s.xnpv(0.07, DayCount::Act365F).unwrap(), 1e-12));
        let last = s.flows()[s.len() - 1].date;
        prop_assert_eq!(s.between(first, last).len(), s.len());
    }

    #[test]
    fn periods_round_trip(n in -500i32..500, u in 0usize..4) {
        let unit = [TimeUnit::Days, TimeUnit::Weeks, TimeUnit::Months, TimeUnit::Years][u];
        let p = Period::new(n, unit);
        prop_assert_eq!(p.to_string().parse::<Period>().unwrap(), p);
    }
}

#[test]
fn stream_rejects_nan_and_keeps_same_day_order() {
    let d = Date::new(2024, 1, 1).unwrap();
    assert!(CashFlowStream::new([CashFlow::new(d, f64::NAN, FlowKind::Other)]).is_err());
    let s = CashFlowStream::new([
        CashFlow::new(d, 1.0, FlowKind::Fee),
        CashFlow::new(d, 2.0, FlowKind::Tax),
    ])
    .unwrap();
    assert_eq!(s.amounts(), vec![1.0, 2.0]);
    let net = s.net_by_date();
    assert_eq!((net.len(), net.flows()[0].kind), (1, FlowKind::Other));
}
