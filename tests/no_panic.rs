//! "Typed errors, no panics on bad input": every public function, fed empty,
//! NaN, infinite, zero, negative and huge inputs, returns (Ok or Err) and
//! never panics or aborts.
#![allow(clippy::unwrap_used)]

use finance_rs::amortization::{self as am, Pmi, Refinance};
use finance_rs::cashflow::{xirr_roots_times, xirr_times, xnpv_times};
use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
use finance_rs::corp::{self, Convention, Lbo};
use finance_rs::daycount::DayCount;
use finance_rs::equity::{self, GrahamDefensiveInput};
use finance_rs::fi::Bond;
use finance_rs::fund::{self, Compounding, Deal, WaterfallTerms};
use finance_rs::period::{Period, TimeUnit};
use finance_rs::rates::{self, Compounding as Basis, Frequency, Rate};
use finance_rs::solve::{self, SolverOptions};
use finance_rs::stats::{self, OlsOptions, QuantileMethod};
use finance_rs::tvm::{self, When};
use finance_rs::{attribution, cashflow, factors, math, options, perf, re, Date};
use proptest::prelude::*;

/// An `Ok` number is always finite (errors are typed, never a silent NaN).
#[allow(clippy::needless_pass_by_value)]
fn fin(what: &str, r: finance_rs::Result<f64>) {
    if let Ok(x) = r {
        assert!(x.is_finite(), "{what} -> Ok({x})");
    }
}

fn nasty() -> impl Strategy<Value = f64> {
    prop_oneof![
        Just(f64::NAN),
        Just(f64::INFINITY),
        Just(f64::NEG_INFINITY),
        Just(0.0),
        Just(-0.0),
        Just(-1.0),
        Just(1.0),
        Just(1e308),
        Just(-1e308),
        Just(1e-308),
        -2.0..2.0_f64,
        -1e6..1e6_f64,
    ]
}

fn nasty_vec() -> impl Strategy<Value = Vec<f64>> {
    prop::collection::vec(nasty(), 0..8)
}

fn periods() -> impl Strategy<Value = u32> {
    prop_oneof![Just(0), Just(1), Just(u32::MAX), 1u32..400]
}

proptest! {
    #[test]
    fn tvm_and_cashflow(a in nasty(), b in nasty(), c in nasty(), d in nasty(), v in nasty_vec(), t in nasty_vec()) {
        for w in [When::End, When::Begin] {
            fin("tvm::fv", tvm::fv(a, b, c, d, w));
            fin("tvm::pv", tvm::pv(a, b, c, d, w));
            fin("tvm::pmt", tvm::pmt(a, b, c, d, w));
            fin("tvm::nper", tvm::nper(a, b, c, d, w));
            fin("tvm::ipmt", tvm::ipmt(a, b, c, d, 0.0, w));
            fin("tvm::ppmt", tvm::ppmt(a, b, c, d, 0.0, w));
            fin("tvm::rate", tvm::rate(a, b, c, d, w));
        }
        fin("tvm::npv", tvm::npv(a, &v));
        fin("tvm::irr", tvm::irr(&v));
        let _ = tvm::irr_roots(&v);
        fin("tvm::mirr", tvm::mirr(&v, a, b));
        fin("xnpv_times", xnpv_times(a, &t, &v));
        fin("xirr_times", xirr_times(&t, &v, a));
        let _ = xirr_roots_times(&t, &v);
    }

    #[test]
    fn perf_metrics(r in nasty_vec(), bm in nasty_vec(), x in nasty(), y in nasty()) {
        fin("perf::compounded_return", perf::compounded_return(&r));
        let _ = perf::cumulative_returns(&r);
        fin("perf::cagr", perf::cagr(&r, x));
        fin("perf::volatility", perf::volatility(&r, x));
        fin("perf::win_rate", perf::win_rate(&r));
        fin("perf::sharpe", perf::sharpe(&r, x, y));
        fin("perf::sortino", perf::sortino(&r, x, y));
        fin("perf::calmar", perf::calmar(&r, x));
        fin("perf::omega", perf::omega(&r, x, y, 252.0));
        let _ = perf::drawdown_series(&r);
        fin("perf::max_drawdown", perf::max_drawdown(&r));
        let _ = perf::drawdown_periods(&r);
        let _ = perf::max_drawdown_duration(&r);
        let _ = perf::top_drawdowns(&r, 3);
        let dates: [Date; 0] = [];
        let _ = perf::drawdown_periods_dated(&dates, &r);
        fin("perf::ulcer_performance_index", perf::ulcer_performance_index(&r, x));
        fin("perf::serenity_index", perf::serenity_index(&r, x));
        fin("perf::var_historical", perf::var_historical(&r, x));
        fin("perf::cvar_historical", perf::cvar_historical(&r, x));
        fin("perf::var_parametric", perf::var_parametric(&r, x));
        fin("perf::cvar_parametric", perf::cvar_parametric(&r, x));
        fin("perf::var_cornish_fisher", perf::var_cornish_fisher(&r, x));
        fin("perf::cvar_cornish_fisher", perf::cvar_cornish_fisher(&r, x));
        fin("perf::var_ewma", perf::var_ewma(&r, y, x));
        fin("perf::risk_scale_var", perf::risk_scale_var(x, y));
        let _ = perf::risk_component_var(&r, &[], x);
        let _ = perf::risk_marginal_var(&r, &[], x);
        fin("perf::beta_fragility", perf::beta_fragility(&r, &bm));
        fin("perf::beta", perf::beta(&r, &bm));
        fin("perf::alpha", perf::alpha(&r, &bm, x));
        fin("perf::information_ratio", perf::information_ratio(&r, &bm));
        fin("perf::tracking_error", perf::tracking_error(&r, &bm, x));
        fin("perf::r_squared", perf::r_squared(&r, &bm));
        fin("perf::up_capture", perf::up_capture(&r, &bm));
        fin("perf::down_capture", perf::down_capture(&r, &bm));
        fin("perf::up_down_capture", perf::up_down_capture(&r, &bm));
        fin("perf::batting_average", perf::batting_average(&r, &bm));
        let _ = perf::up_alpha_beta(&r, &bm);
        let _ = perf::down_alpha_beta(&r, &bm);
        fin("perf::excess_sharpe", perf::excess_sharpe(&r, &bm, x));
        let _ = attribution::brinson_hood_beebower(&r, &bm, &r, &bm);
        let _ = attribution::brinson_fachler(&r, &bm, &r, &bm);
        let linked = [attribution::LinkedPeriod {
            portfolio_return: x,
            benchmark_return: y,
            effects: [x, y, x - y],
        }];
        let _ = attribution::link::carino(&linked);
        let _ = attribution::link::menchero(&linked);
        let _ = attribution::link::grap(&linked);
        let fit = factors::exposures(&r, &[&bm]);
        if let Ok(fit) = fit {
            let _ = factors::attribution(&fit, &[&bm]);
        }
        let _ = factors::fama_french_alpha(&r, &[&bm, &r, &bm]);
        let _ = factors::rolling_beta(&r, &bm, 2);
        fin("perf::normal_ppf", perf::normal_ppf(x));
        let _ = (perf::normal_pdf(x), perf::normal_cdf(x));
        let _ = perf::returns::simple(&r);
        let _ = perf::returns::log(&r);
        let _ = perf::returns::to_price_index(&r, x);
        let _ = perf::returns::rebase(&r, x);
        let _ = perf::returns::excess(&r, x, y);
        let _ = perf::returns::twr(&r);
        let _ = perf::returns::modified_dietz(x, y, &[(x, y)]);
        let _ = perf::returns::best(&r);
        let _ = perf::returns::worst(&r);
        let _ = perf::returns::avg_return(&r);
        let _ = perf::returns::avg_win(&r);
        let _ = perf::returns::avg_loss(&r);
        let _ = perf::returns::consecutive_wins(&r);
        let _ = perf::returns::consecutive_losses(&r);
        let _ = perf::returns::exposure(&r);
        let _ = perf::returns::geometric_mean(&r);
        let _ = perf::returns::rolling(&r, 1, perf::compounded_return);
        fin("downside_deviation", perf::downside_deviation(&r, x, y));
        fin("skew", perf::skew(&r));
        fin("kurtosis", perf::kurtosis(&r));
        fin("smart_sharpe", perf::smart_sharpe(&r, x, y));
        fin("smart_sortino", perf::smart_sortino(&r, x, y));
        fin("adjusted_sortino", perf::adjusted_sortino(&r, x, y));
        fin("probabilistic_sharpe", perf::probabilistic_sharpe(&r, x));
        fin("expected_max_sharpe", perf::expected_max_sharpe(2, x));
        fin("deflated_sharpe", perf::deflated_sharpe(&r, 2, x));
        fin("prob_backtest_overfitting", perf::prob_backtest_overfitting(&[r.clone(), bm.clone()], 2));
        fin("treynor", perf::treynor(&r, &bm, x, y));
        fin("m2", perf::m2(&r, &bm, x, y));
        fin("gain_to_pain", perf::gain_to_pain(&r));
        fin("payoff_ratio", perf::payoff_ratio(&r));
        fin("profit_factor", perf::profit_factor(&r));
        fin("profit_ratio", perf::profit_ratio(&r));
        fin("tail_ratio", perf::tail_ratio(&r));
        fin("common_sense_ratio", perf::common_sense_ratio(&r));
        fin("cpc_index", perf::cpc_index(&r));
        fin("outlier_win_ratio", perf::outlier_win_ratio(&r));
        fin("outlier_loss_ratio", perf::outlier_loss_ratio(&r));
        fin("recovery_factor", perf::recovery_factor(&r));
        fin("risk_of_ruin", perf::risk_of_ruin(&r));
        fin("kelly_criterion", perf::kelly_criterion(&r));
        fin("risk_return_ratio", perf::risk_return_ratio(&r));
        fin("stability", perf::stability(&r));
        fin("weighted_sharpe", perf::weighted_sharpe(&r, &bm));
        fin("weighted_outcome_sharpe", perf::weighted_outcome_sharpe(&[(x, y)]));
        let d = [Date::new(2024, 1, 2).unwrap(), Date::new(2024, 2, 2).unwrap()];
        let _ = perf::returns::aggregate(&d, &r, perf::returns::Period::Month);
        let _ = perf::returns::mtd(&d, &r, d[1]);
        let _ = perf::returns::qtd(&d, &r, d[1]);
        let _ = perf::returns::ytd(&d, &r, d[1]);
        let _ = perf::returns::mwr(&d, &r);
    }

    #[test]
    fn option_functions_never_panic(s in nasty(), k in nasty(), t in nasty(), r in nasty(), q in nasty(), vol in nasty(), rho in nasty(), xs in nasty_vec()) {
        for kind in [options::OptionKind::Call, options::OptionKind::Put] {
            let _ = options::bsm(kind, s, k, t, r, q, vol);
            let _ = options::black76(kind, s, k, t, r, vol);
            let _ = options::bachelier(kind, s, k, t, r, vol);
            let _ = options::garman_kohlhagen(kind, s, k, t, r, q, vol);
            let _ = options::bsm_greeks(kind, s, k, t, r, q, vol);
            let _ = options::black76_greeks(kind, s, k, t, r, vol);
            let _ = options::bsm_implied_vol(kind, s, s, k, t, r, q);
            let _ = options::black76_implied_vol(kind, s, s, k, t, r);
            let _ = options::bachelier_implied_vol(kind, s, s, k, t, r);
        }
        let _ = options::margrabe(options::OptionKind::Call, s, k, t, q, r, vol, vol, rho);
        let _ = options::close_to_close(&xs, s);
        let _ = options::parkinson(&xs, &xs, s);
        let _ = options::garman_klass(&xs, &xs, &xs, &xs, s);
        let _ = options::rogers_satchell(&xs, &xs, &xs, &xs, s);
        let _ = options::yang_zhang(&xs, &xs, &xs, &xs, s);
        let _ = options::ewma(&xs, r, s);
    }

    #[test]
    fn amortization(p in nasty(), r in nasty(), n in periods(), m in periods(), extra in nasty_vec(), x in nasty()) {
        let _ = am::level_payment(p, r, n);
        let _ = am::level_payment_with_extra(p, r, n, &extra);
        let _ = am::interest_only(p, r, m, n);
        let _ = am::balloon(p, r, n, m);
        let refi = Refinance { at_period: m, new_rate: x, new_nper: n, closing_costs: x, roll_in: true };
        let _ = am::refinance(p, r, n, &refi);
        let _ = am::compare_refinance(p, r, n, &refi);
        if let Ok(mut s) = am::level_payment(1_000.0, 0.01, 12) {
            let pmi = Pmi { home_value: p, annual_rate: r, drop_ltv: x, periods_per_year: m };
            let _ = am::apply_pmi(&mut s, &pmi);
            let _ = (s.summary(), s.total_interest(), s.total_paid(), s.final_balance());
        }
    }

    #[test]
    fn re_corp(a in nasty(), b in nasty(), c in nasty(), d in nasty(), e in nasty(), v in nasty_vec()) {
        let _ = (re::noi(a, b, c, d), re::cap_rate(a, b), re::value_from_cap_rate(a, b));
        let _ = (re::cash_on_cash(a, b), re::dscr(a, b), re::grm(a, b));
        let _ = (re::breakeven_occupancy(a, b, c), re::debt_yield(a, b), re::ltv(a, b));
        for conv in [Convention::EndOfPeriod, Convention::MidPeriod] {
            let _ = corp::dcf(a, &v, conv);
            let _ = corp::enterprise_value(a, &v, b, conv);
        }
        let _ = (corp::terminal_value_gordon(a, b, c), corp::terminal_value_exit_multiple(a, b));
        let _ = (corp::wacc(a, b, c, d, e), corp::capm(a, b, c));
        let _ = (corp::moic(a, &v, b), corp::equity_irr(a, &v, b));
        let _ = corp::lbo(&Lbo { entry_ev: a, debt_to_ev: b, debt_rate: c, fcf: v.clone(), exit_metric: d, exit_multiple: e });
    }

    #[test]
    fn graham_equity_screen_never_panics(
        a in nasty(), b in nasty(), c in nasty(), d in nasty(), e in nasty(),
        f in nasty(), g in nasty(), h in nasty(), eps in prop::collection::vec(nasty(), 10),
    ) {
        let mut earnings = [0.0; 10];
        earnings.copy_from_slice(&eps);
        let criteria = equity::graham_defensive_criteria(&GrahamDefensiveInput {
            current_assets: a,
            total_liabilities: b,
            long_term_debt: c,
            current_ratio: d,
            eps: earnings,
            uninterrupted_dividends: [true; 20],
            eps_growth: e,
            pe: f,
            pb: g,
            aaa_yield: h,
        });
        if let Ok(criteria) = criteria {
            let _ = criteria.as_array();
        }
    }

    #[test]
    fn fund_fi(a in nasty(), b in nasty(), c in nasty(), d in nasty(), v in nasty_vec(), w in nasty_vec(), ix in nasty_vec(), n in periods(), f in prop_oneof![Just(0u32), Just(1), Just(2), Just(3), Just(12)]) {
        let _ = (fund::tvpi(a, b, c), fund::dpi(a, b), fund::rvpi(a, b), fund::multiples(a, b, c));
        let _ = fund::ks_pme(&v, &w, a, &ix);
        let _ = fund::direct_alpha(&v, &w, a, &ix, b);
        let _ = fund::preferred_return(a, b, c, Compounding::Annual);
        let _ = fund::preferred_return(a, b, c, Compounding::Simple);
        let _ = fund::preferred_return_schedule(&v, &w, a);
        let _ = fund::moic(&v, &w, a);
        let _ = fund::pic(a, b);
        let _ = fund::ks_pme_flows(&v, &w, &ix);
        let _ = fund::ln_pme_nav(&v, &w, &ix);
        let _ = fund::ln_pme(&v, &w, &ix);
        let _ = fund::pme_plus_lambda(&v, &w, a, &ix);
        let _ = fund::pme_plus_flows(&v, &w, a, &ix);
        let _ = fund::pme_plus(&v, &w, a, &ix);
        let _ = fund::m_pme(&v, &w, &ix, &v);
        let schedule = v.iter().zip(&w).enumerate().map(|(i, (target, actual))| fund::FundPacingYear {
            year: i32::try_from(i).unwrap_or(i32::MAX), target_commitment: *target, actual_commitment: *actual,
        }).collect::<Vec<_>>();
        let _ = fund::pacing_schedule(&schedule);
        let holdings = v.iter().enumerate().map(|(i, value)| (format!("h{i}"), *value)).collect::<Vec<_>>();
        let _ = fund::look_through(&holdings, &[]);
        let dates = [Date::new(2020, 1, 1).unwrap(), Date::new(2021, 1, 1).unwrap()];
        let calls = [a, b]; let dists = [c, d];
        let _ = fund::net_irr(&dates, &calls, &dists, a, dates[1]);
        let _ = fund::gross_irr(&dates, &calls, &dists, a, dates[1]);
        let terms = WaterfallTerms { catch_up: c, carry: d };
        let _ = fund::european_waterfall(a, b, c, terms);
        let deals: Vec<Deal> = v.iter().zip(&w).map(|(x, y)| Deal { capital: *x, preferred: *y, proceeds: a }).collect();
        let _ = fund::american_waterfall(&deals, terms, fund::Losses::CarriedForward);
        let _ = fund::american_waterfall(&deals, WaterfallTerms::default(), fund::Losses::Isolated);
        let _ = Bond::from_years(a, b, c, f);
        if let Ok(bond) = Bond::new(a, b, n, f) {
            let _ = (bond.coupon(), bond.price(c), bond.accrued_interest(d), bond.price_with_accrued(c, d));
            let _ = (bond.current_yield(c), bond.risk(c), bond.macaulay_duration(c), bond.modified_duration(c));
            let _ = (bond.convexity(c), bond.dv01(c), bond.yield_from_price(c), bond.yield_from_clean_price(c, d));
        }
    }

    #[test]
    fn stats_and_solvers(x in nasty_vec(), y in nasty_vec(), q in nasty(), a in nasty(), b in nasty(), dd in 0usize..3, n in 0usize..1_000) {
        fin("stats::mean", stats::mean(&x));
        fin("stats::variance", stats::variance(&x, dd));
        fin("stats::stdev", stats::stdev(&x, usize::MAX));
        fin("stats::covariance", stats::covariance(&x, &y, dd));
        fin("stats::correlation", stats::correlation(&x, &y));
        for m in [QuantileMethod::Linear, QuantileMethod::Lower, QuantileMethod::Higher,
                  QuantileMethod::Nearest, QuantileMethod::Midpoint, QuantileMethod::ExcelExclusive] {
            fin("stats::quantile", stats::quantile(&x, q, m));
        }
        for intercept in [true, false] {
            if let Ok(fit) = stats::ols_with(&x, &[&y, &x], OlsOptions { intercept }) {
                assert!(fit.coef.iter().chain(&fit.se).chain(&fit.t).all(|v| v.is_finite()));
                assert!(fit.r2.is_finite() && fit.adj_r2.is_finite());
            }
        }
        let _ = stats::ols(&x, &[]);
        let o = SolverOptions { xtol: q.abs(), ftol: b.abs(), max_iter: n };
        let f = |t: f64| t * t * t - a * t - b;
        for opts in [o, SolverOptions::default()] {
            let _ = solve::brent(f, a, b, &opts).map(|s| assert!(s.root.is_finite()));
            let _ = solve::newton_bracketed(f, |t| 3.0 * t * t - a, q, (a, b), &opts);
            let _ = solve::scan_roots(f, &x, &opts);
            let _ = solve::fixed_point(|t| a * t.cos() + b, q, &opts);
        }
        let _ = solve::expand_bracket(f, q, (a, b));
        fin("math::normal_ppf", math::normal_ppf(q));
    }

    #[test]
    fn actuarial_functions_do_not_panic(a in nasty(), b in nasty(), n in 0u32..120, age in 1u32..100) {
        use finance_rs::actuarial::{annuity, insurance, life, retirement};
        let _ = annuity::immediate(a, b);
        let _ = annuity::due(a, b);
        let _ = annuity::continuous(a, b);
        let _ = annuity::deferred(a, b, 10.0);
        let _ = annuity::increasing(a, n);
        let _ = annuity::perpetuity(a, When::Begin);
        let _ = insurance::iul_credit(a, b, -0.1, 1.0);
        let _ = insurance::policy_loan_spread(a, b, 0.05);
        if let Ok(table) = life::LifeTable::new(1, vec![0.01; 121]) {
            let _ = (table.lx(), table.px(age), table.tpx(age, n), table.tqx(age, n));
            let _ = (life::expectancy(&table, age), life::annuity_due(&table, age, a));
            let _ = (life::temporary_annuity(&table, age, n, a), life::whole_life_insurance(&table, age, a));
            let _ = (life::term_insurance(&table, age, n, a), life::endowment(&table, age, n, a));
            let _ = (life::net_level_premium(&table, age, a), life::reserve_prospective(&table, age, n, a));
            let _ = life::spia_payout(a, &table, age, b, 0.03);
        }
        let _ = retirement::rmd(a, u8::try_from(age).unwrap_or(u8::MAX), b);
        let _ = retirement::rmd_start_age(u16::try_from(age).unwrap_or_default().saturating_add(1900));
        let _ = retirement::inherited_ira_10yr(a, b, (n % 10) as u8);
        let _ = retirement::constant_real(a, b, 0.02, n);
        let _ = retirement::guyton_klinger(a, b, 100.0, 0.1, 0.1, 0.2, 0.2);
        let _ = retirement::vpw(a, u8::try_from(age).unwrap_or(u8::MAX), b, u8::try_from(n % 60 + 1).unwrap_or(u8::MAX));
        let _ = retirement::success_rate(&[vec![a, b]], &[0.1, 0.1]);
        let _ = retirement::spending_smile(a, b);
        let _ = retirement::social_security::pia(a, 2026);
        let _ = retirement::claiming_adjustment(i16::try_from(n).unwrap_or(i16::MAX).saturating_sub(60));
        let _ = retirement::sepp_72t(a, u8::try_from(age).unwrap_or(u8::MAX), b, 0.04, retirement::SeppMethod::Amortization, 20.0, 15.0);
        let _ = retirement::roth_conversion_ladder(a, b, 0.03, n);
        let _ = retirement::social_security::bend_points(u16::try_from(age).unwrap_or_default().saturating_add(1979));
        #[cfg(feature = "actuarial")]
        {
            let _ = finance_rs::actuarial::tables::irs_single_life();
            let _ = finance_rs::actuarial::tables::irs_uniform_lifetime();
            let _ = finance_rs::actuarial::tables::irs_joint_last_survivor(0, 0, Vec::new());
            let _ = finance_rs::actuarial::tables::soa_2012_iam(
                finance_rs::actuarial::tables::IamSex::Male,
            );
        }
    }

    #[test]
    fn rates_and_periods(r in nasty(), t in nasty(), m in nasty(), k in 0u16..400, start in -100_000i64..100_000, len in -10_000i64..10_000, p in -2_000i32..2_000, s in ".{0,6}") {
        for basis in [Basis::Simple, Basis::Continuous, Basis::Compounded(Frequency::PerYear(k)),
                      Basis::Compounded(Frequency::Once), Basis::SimpleThenCompounded(Frequency::Daily),
                      Basis::SimpleThenCompounded(Frequency::PerYear(k))] {
            let rate = Rate::new(r, basis, DayCount::Act360);
            fin("Rate::compound_factor", rate.compound_factor(t));
            fin("Rate::discount_factor", rate.discount_factor(t));
            fin("Compounding::implied_rate", basis.implied_rate(m, t));
            let _ = rate.equivalent(Basis::Compounded(Frequency::Monthly), t).map(|e| assert!(e.value.is_finite()));
            let _ = Rate::implied(m, basis, DayCount::Act365F, t);
            if let (Ok(a), Ok(b)) = (Date::from_ordinal(start), Date::from_ordinal(start + len)) {
                fin("Rate::compound_factor_between", rate.compound_factor_between(a, b));
                fin("Rate::discount_factor_between", rate.discount_factor_between(a, b));
            }
        }
        fin("rates::effective", rates::effective(r, m));
        fin("rates::nominal", rates::nominal(r, m));
        fin("rates::periodic_from_annual", rates::periodic_from_annual(r, m));
        fin("rates::annualize", rates::annualize(r, m));
        fin("rates::continuous_from_discrete", rates::continuous_from_discrete(r, m));
        fin("rates::discrete_from_continuous", rates::discrete_from_continuous(r, m));
        fin("rates::real_rate", rates::real_rate(r, t));
        let _ = s.parse::<Period>();
        if let Ok(d) = Date::from_ordinal(start) {
            for unit in [TimeUnit::Days, TimeUnit::Weeks, TimeUnit::Months, TimeUnit::Years] {
                let _ = d.add_period(Period::new(p.saturating_mul(1_000), unit));
            }
        }
    }

    #[test]
    fn tvm_and_cashflow_extras(a in nasty(), b in nasty(), c in nasty(), v in nasty_vec(), s in 0u32..400, e in 0u32..400, offs in prop::collection::vec(-5_000i64..5_000, 0..8)) {
        for w in [When::End, When::Begin] {
            fin("tvm::cumipmt", tvm::cumipmt(a, b, c, s, e, w));
            fin("tvm::cumprinc", tvm::cumprinc(a, b, c, s, e, w));
            fin("tvm::perpetuity_pv", tvm::perpetuity_pv(a, b, c, w));
            fin("tvm::growing_annuity_pv", tvm::growing_annuity_pv(a, b, c, f64::from(s), w));
            fin("tvm::growing_annuity_fv", tvm::growing_annuity_fv(c, a, b, c, w));
        }
        fin("tvm::nfv", tvm::nfv(a, b, &v));
        fin("tvm::fv_schedule", tvm::fv_schedule(a, &v));
        fin("tvm::equivalent_annual_annuity", tvm::equivalent_annual_annuity(a, &v));
        fin("tvm::profitability_index", tvm::profitability_index(a, &v));
        if let Ok(Some(t)) = tvm::payback_period(&v) { assert!(t.is_finite()); }
        if let Ok(Some(t)) = tvm::discounted_payback(a, &v) { assert!(t.is_finite()); }
        let _ = (cashflow::sign_changes(&v), cashflow::is_conventional(&v));
        let base = Date::new(2020, 1, 1).unwrap().ordinal();
        let dates: Vec<Date> = offs.iter().map(|o| Date::from_ordinal(base + o).unwrap()).collect();
        let amounts: Vec<f64> = v.iter().copied().take(dates.len()).collect();
        fin("cashflow::xnfv", cashflow::xnfv(a, &dates, &amounts));
        fin("cashflow::macaulay_duration_dated", cashflow::macaulay_duration_dated(a, &dates, &amounts, DayCount::ActActIsda));
        if dates.len() >= 3 {
            fin("cashflow::xfv", cashflow::xfv(dates[0], dates[1], dates[2], a, b, c));
        }
        let flows = dates.iter().zip(&v).map(|(d, x)| CashFlow::new(*d, *x, FlowKind::Nav));
        if let Ok(st) = CashFlowStream::new(flows) {
            let _ = (st.merge(&st).net_by_date(), st.split_signs(), st.filter(&[FlowKind::Nav]));
            let _ = st.between(Date::new(2019, 1, 1).unwrap(), Date::new(2021, 1, 1).unwrap());
            let _ = (st.scale(a), st.shift(Period::new(i32::MAX, TimeUnit::Years)), st.total());
            fin("CashFlowStream::xnpv", st.xnpv(a, DayCount::Act365F));
            fin("CashFlowStream::xirr", st.xirr());
            fin("CashFlowStream::macaulay_duration", st.macaulay_duration(a, DayCount::Act365F));
            let rate = Rate::new(a, Basis::Continuous, DayCount::Act365F);
            fin("CashFlowStream::pv", st.pv(&rate, Date::new(2019, 1, 1).unwrap()));
        }
    }

    #[test]
    fn equity_functions_no_panic(a in nasty(), b in nasty(), c in nasty(), d in nasty(), xs in nasty_vec()) {
        use finance_rs::equity as eq;
        let _ = eq::ddm_gordon(a, b, c);
        let _ = eq::ddm_two_stage(a, b, 2, c, d);
        let _ = eq::ddm_h_model(a, b, c, d, 0.1);
        let _ = eq::residual_income(a, &xs, b, c);
        let _ = eq::implied_cost_of_equity(a, b, c);
        let _ = eq::sustainable_growth(a, b);
        let _ = eq::total_shareholder_return(a, b, &xs);
        let _ = eq::dividend_discount_irr(a, &xs, c);
        let _ = eq::earnings_power_value(a, b, c);
        let _ = eq::multiples::pe(a, b);
        let _ = eq::multiples::ev_ebitda(a, b);
        let _ = eq::multiples::ev_sales(a, b);
        let _ = eq::multiples::ev_fcf(a, b);
        let _ = eq::multiples::pb(a, b);
        let _ = eq::multiples::p_fcf(a, b);
        let _ = eq::multiples::peg(a, b);
        let _ = eq::yields(a, b, c, d, 1.0);
        let _ = eq::returns(a, b, c, d, 1.0);
        let _ = eq::dupont(a, b, c, d);
        let _ = eq::ncav_per_share(a, b, c, d);
        let _ = eq::graham_number(a, b);
        let _ = eq::comps_implied_value(a, &xs, eq::PeerStat::Quartiles);
        let _ = eq::short_carry(a, b, c);
        let _ = eq::fcf_margin(a, b);
        let _ = corp::fcff(a, b, c, d, 1.0);
        let _ = corp::fcfe(a, b, c, d);
        let _ = corp::equity_bridge(a, b, c, d, 1.0, 2.0);
        let _ = corp::diluted_shares_tsm(a, &[(b, c)], d);
        let _ = corp::value_per_share(a, b, &[(c, d)]);
        let _ = corp::unlever_beta(a, b, c);
        let _ = corp::relever_beta(a, b, c);
        let _ = corp::apv(a, &xs, b);
        let _ = corp::eva(a, b, c);
        let _ = corp::reverse_dcf(a, |_| xs.clone(), b, -0.5, 0.5);
        let _ = corp::implied_discount_rate(a, &xs, b);
        if let Ok(date) = Date::new(2020, 1, 1) {
            let _ = corp::dcf_dated(a, &[date], &[b], date, corp::DcfTiming::ActualDate);
        }
    }
}
