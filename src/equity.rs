//! Closed-form equity valuation, operating returns, yields and multiples.
//!
//! Rates, growth rates and payout ratios are decimals. Ratio functions reject
//! non-positive denominators instead of returning misleading infinities.

mod graham;
mod models;
mod ratios;

pub use graham::{graham_defensive_criteria, GrahamCriteria, GrahamDefensiveInput};
pub use models::{
    ddm_gordon, ddm_h_model, ddm_two_stage, dividend_discount_irr, earnings_power_value,
    implied_cost_of_equity, residual_income, sustainable_growth, total_shareholder_return,
    DdmStage,
};
pub use ratios::{
    comps_implied_value, dupont, fcf_margin, graham_number, multiples, ncav_per_share, returns,
    short_carry, yields, MultipleSet, PeerStat, ReturnRatios, YieldSet,
};
