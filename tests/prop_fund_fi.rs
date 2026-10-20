//! Property tests for the fund and fixed-income modules.
#![allow(clippy::unwrap_used)]

use finance_rs::fi::Bond;
use finance_rs::fund::{
    american_waterfall, european_waterfall, ks_pme, look_through, multiples, Deal, Waterfall,
    WaterfallTerms,
};
use proptest::prelude::*;

fn tiers_ok(w: &Waterfall) -> Result<(), TestCaseError> {
    let tiers = [w.return_of_capital, w.preferred, w.catch_up, w.carry];
    let tol = 1e-9 * (1.0 + w.distributable);
    for t in tiers {
        prop_assert!(t.lp >= -tol && t.gp >= -tol, "{w:?}");
    }
    let total: f64 = tiers.iter().map(|t| t.lp + t.gp).sum();
    prop_assert!((total - w.distributable).abs() < tol, "{w:?}");
    prop_assert!(
        (w.lp_total + w.gp_total - w.distributable).abs() < tol,
        "{w:?}"
    );
    Ok(())
}

fn terms() -> impl Strategy<Value = WaterfallTerms> {
    (0.0..=1.0f64, 0.0..=0.5f64).prop_map(|(catch_up, carry)| WaterfallTerms { catch_up, carry })
}

proptest! {
    #[test]
    fn european_tiers_sum_and_carry_is_capped(
        d in 0.0..1e7f64, cap in 0.0..1e7f64, pref_rate in 0.0..0.5f64, t in terms()
    ) {
        let pref = cap * pref_rate;
        let w = european_waterfall(d, cap, pref, t).unwrap();
        tiers_ok(&w)?;
        let profit = (d - cap).max(0.0);
        prop_assert!(w.gp_total <= t.carry * profit + 1e-9 * (1.0 + d), "{w:?}");
    }

    #[test]
    fn full_catch_up_pays_exact_carry(
        cap in 1.0..1e6f64, pref_rate in 0.0..0.3f64, extra in 0.0..1e6f64, carry in 0.0..0.5f64
    ) {
        let pref = cap * pref_rate;
        let d = cap + pref / (1.0 - carry) + extra;
        let w = european_waterfall(d, cap, pref, WaterfallTerms { catch_up: 1.0, carry }).unwrap();
        prop_assert!((w.gp_total - carry * (d - cap)).abs() < 1e-8 * (1.0 + d), "{w:?}");
    }

    #[test]
    fn american_tiers_sum(
        deals in prop::collection::vec((0.0..1e6f64, 0.0..0.3f64, 0.0..3e6f64), 1..8),
        t in terms(), carry_forward: bool
    ) {
        let deals: Vec<Deal> = deals
            .into_iter()
            .map(|(capital, r, proceeds)| Deal { capital, preferred: capital * r, proceeds })
            .collect();
        let a = american_waterfall(&deals, t, if carry_forward { finance_rs::fund::Losses::CarriedForward } else { finance_rs::fund::Losses::Isolated }).unwrap();
        for w in &a.deals {
            tiers_ok(w)?;
        }
        tiers_ok(&a.total)?;
        prop_assert!(a.clawback >= 0.0);
        let profit: f64 = deals.iter().map(|d| d.proceeds - d.capital).sum();
        prop_assert!(a.total.gp_total - a.clawback <= t.carry * profit.max(0.0) + 1e-6);
    }

    #[test]
    fn tvpi_is_dpi_plus_rvpi(pi in 1e-3..1e9f64, d in 0.0..1e9f64, nav in -1e8..1e9f64) {
        let m = multiples(pi, d, nav).unwrap();
        prop_assert!((m.tvpi - m.dpi - m.rvpi).abs() <= 1e-12 * (1.0 + m.tvpi.abs()));
    }

    #[test]
    fn ks_pme_on_flat_index_is_tvpi(
        flows in prop::collection::vec((0.0..1e6f64, 0.0..1e6f64), 1..20),
        nav in 0.0..1e6f64, level in 1.0..1e4f64
    ) {
        let c: Vec<f64> = flows.iter().map(|f| f.0).collect();
        let d: Vec<f64> = flows.iter().map(|f| f.1).collect();
        let paid: f64 = c.iter().sum();
        prop_assume!(paid > 1.0);
        let pme = ks_pme(&c, &d, nav, &vec![level; c.len()]).unwrap();
        let m = multiples(paid, d.iter().sum(), nav).unwrap();
        prop_assert!((pme - m.tvpi).abs() < 1e-9 * (1.0 + m.tvpi));
        let (future_calls, future_distributions) = finance_rs::fund::ks_pme_flows(&c, &d, &vec![level; c.len()]).unwrap();
        prop_assert_eq!(future_calls, c);
        prop_assert_eq!(future_distributions, d);
    }

    #[test]
    fn fund_lookthrough_conserves_value(
        shares in prop::collection::vec(0.0..1e6f64, 1..20),
        value in 0.0..1e9f64,
    ) {
        let share_total = shares.iter().sum::<f64>();
        prop_assume!(share_total > 0.0);
        let exposures = shares.iter().enumerate().map(|(i, share)| ("fund".to_string(), format!("asset-{i}"), share / share_total)).collect::<Vec<_>>();
        let result = look_through(&[("fund".to_string(), value)], &exposures).unwrap();
        prop_assert!((result.values().sum::<f64>() - value).abs() < 1e-8 * (1.0 + value));
    }

    #[test]
    fn fund_capital_call_allocation_conserves_cents(
        unfunded in prop::collection::vec(0.01..1e6f64, 1..12), fraction in 0.0..=1.0f64
    ) {
        let commitments = unfunded.iter().enumerate().map(|(i,u)|(format!("lp-{i}"),*u,0.0)).collect::<Vec<_>>();
        let amount = fraction * unfunded.iter().sum::<f64>();
        let allocated=finance_rs::fund::capital_call_allocate(amount,&commitments).unwrap();
        prop_assert!((allocated.iter().map(|r|r.amount).sum::<f64>()-(amount*100.0).round()/100.0).abs()<1e-7);
    }

    #[test]
    fn management_fee_net_never_exceeds_gross(
        base in prop::collection::vec(0.0..1e8f64, 1..12), rate in 0.0..0.1f64, offset in 0.0..=1.0f64
    ) {
        let basis=vec![finance_rs::fund::FeeBasis::Committed;base.len()];let rates=vec![rate;base.len()];
        let schedule=finance_rs::fund::FeeSchedule{basis_by_period:basis,rate_by_period:rates,step_downs:vec![],offset_pct:offset};
        let result=finance_rs::fund::management_fee(&base,&base,&base,&schedule).unwrap();
        prop_assert!(result.net.iter().zip(&result.gross).all(|(n,g)|n<=g));
        prop_assert!((result.net.iter().sum::<f64>()+result.offsets.iter().sum::<f64>()-result.gross.iter().sum::<f64>()).abs()<1e-7);
    }

    #[test]
    fn multi_hurdle_tiers_conserve_cash(
        cash in 0.0..1e8f64, capital in 0.0..1e7f64,
        lp_split in prop::collection::vec(0.0..=1.0f64, 1..6)
    ) {
        let dates=[finance_rs::Date::new(2020,1,1).unwrap(),finance_rs::Date::new(2025,1,1).unwrap()];
        let tiers=lp_split.iter().enumerate().map(|(i,share)|finance_rs::fund::Hurdle{irr_or_multiple:0.05+0.02*f64::from(u32::try_from(i).unwrap_or(u32::MAX)),lp_split:*share}).collect::<Vec<_>>();
        let split=finance_rs::fund::multi_hurdle(cash,capital,&tiers,&dates).unwrap();
        prop_assert!((split.lp+split.gp-cash).abs()<1e-7*(1.0+cash));
        prop_assert!(split.lp>=0.0&&split.gp>=0.0);
    }

    #[test]
    fn takahashi_alexander_never_overcalls_commitment(
        commitment in 0.0..1e7f64, rate in 0.0..=1.0f64, bow in 0.0..5.0f64, growth in -0.5..0.5f64, life in 1usize..20
    ) {
        let rows=finance_rs::fund::takahashi_alexander(commitment,rate,bow,growth,life,0.0).unwrap();
        let called=rows.iter().map(|row|row.1).sum::<f64>();
        prop_assert!(called<=commitment+1e-7*(1.0+commitment));
        prop_assert!(rows.iter().all(|row|row.2>=0.0&&row.3>=0.0));
    }

    #[test]

    #[test]
    fn price_yield_round_trip(
        coupon in 0.0..0.2f64, n in 1u32..120, f in prop::sample::select(vec![1u32, 2, 4, 12]),
        y in -0.05..0.4f64
    ) {
        let b = Bond::new(100.0, coupon, n, f).unwrap();
        let p = b.price(y).unwrap();
        let y2 = b.yield_from_price(p).unwrap();
        let p2 = b.price(y2).unwrap();
        prop_assert!((p2 - p).abs() < 1e-9 * p, "{p} {p2}");
        prop_assert!((y2 - y).abs() < 1e-7, "{y} {y2}");
    }

    #[test]
    fn duration_positive_and_within_maturity(
        coupon in 0.0..0.2f64, n in 1u32..400, f in prop::sample::select(vec![1u32, 2, 4, 12]),
        y in -0.02..0.5f64
    ) {
        let b = Bond::new(1_000.0, coupon, n, f).unwrap();
        let r = b.risk(y).unwrap();
        let maturity = f64::from(n) / f64::from(f);
        prop_assert!(r.macaulay > 0.0);
        prop_assert!(r.macaulay <= maturity * (1.0 + 1e-12));
        prop_assert!(r.convexity > 0.0 && r.dv01 > 0.0);
    }

    #[test]
    fn price_decreasing_in_yield(
        coupon in 0.0..0.2f64, n in 1u32..200, f in prop::sample::select(vec![1u32, 2, 4, 12]),
        y in -0.05..0.5f64, dy in 1e-4..0.2f64
    ) {
        let b = Bond::new(100.0, coupon, n, f).unwrap();
        prop_assert!(b.price(y + dy).unwrap() < b.price(y).unwrap());
    }
}
