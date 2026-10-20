//! Operating ratios, valuation multiples, and shareholder-return decompositions.
//!
//! ```
//! use finance_rs::equity::returns::roe;
//! assert_eq!(roe(12.0, 100.0)?, 0.12);
//! # Ok::<(), finance_rs::FinanceError>(())

use crate::{
    error::{all_finite, finite, output, FinanceError},
    Result,
};

/// Common equity multiples. Non-positive earnings or cash flow are undefined.
pub mod multiples {
    use super::{all_finite, output, FinanceError, Result};
    fn ratio(name: &'static str, numerator: f64, denominator: f64) -> Result<f64> {
        all_finite(name, &[numerator, denominator])?;
        if denominator <= 0.0 {
            return Err(FinanceError::Undefined { what: name });
        }
        output(name, numerator / denominator)
    }
    /// Price / earnings.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::multiples::pe), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn pe(price: f64, eps: f64) -> Result<f64> {
        ratio("pe", price, eps)
    }
    /// Enterprise value / EBITDA.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::multiples::ev_ebitda), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn ev_ebitda(ev: f64, ebitda: f64) -> Result<f64> {
        ratio("ev_ebitda", ev, ebitda)
    }
    /// Enterprise value / sales.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::multiples::ev_sales), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn ev_sales(ev: f64, sales: f64) -> Result<f64> {
        ratio("ev_sales", ev, sales)
    }
    /// Enterprise value / free cash flow.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::multiples::ev_fcf), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn ev_fcf(ev: f64, fcf: f64) -> Result<f64> {
        ratio("ev_fcf", ev, fcf)
    }
    /// Price / book value per share.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::multiples::pb), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn pb(price: f64, book_value_per_share: f64) -> Result<f64> {
        ratio("pb", price, book_value_per_share)
    }
    /// Price / free cash flow per share.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::multiples::p_fcf), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn p_fcf(price: f64, fcf_per_share: f64) -> Result<f64> {
        ratio("p_fcf", price, fcf_per_share)
    }
    /// PEG ratio, with growth expressed as a decimal.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::multiples::peg), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn peg(pe_ratio: f64, eps_growth: f64) -> Result<f64> {
        all_finite("peg", &[pe_ratio, eps_growth])?;
        if pe_ratio < 0.0 || eps_growth <= 0.0 {
            return Err(FinanceError::Undefined {
                what: "peg requires positive growth and nonnegative P/E",
            });
        }
        output("peg", pe_ratio / (100.0 * eps_growth))
    }
}

/// Equity yield metrics, expressed as decimals.
pub mod yields {
    use super::{all_finite, output, FinanceError, Result};
    fn yield_ratio(name: &'static str, amount: f64, price: f64) -> Result<f64> {
        all_finite(name, &[amount, price])?;
        if price <= 0.0 {
            return Err(FinanceError::Undefined { what: name });
        }
        output(name, amount / price)
    }
    /// Earnings / market capitalization.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::yields::earnings), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn earnings(earnings: f64, market_cap: f64) -> Result<f64> {
        yield_ratio("earnings_yield", earnings, market_cap)
    }
    /// Free cash flow / market capitalization.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::yields::fcf), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn fcf(free_cash_flow: f64, market_cap: f64) -> Result<f64> {
        yield_ratio("fcf_yield", free_cash_flow, market_cap)
    }
    /// Dividends / market capitalization.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::yields::dividend), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn dividend(dividends: f64, market_cap: f64) -> Result<f64> {
        yield_ratio("dividend_yield", dividends, market_cap)
    }
    /// Net buybacks / market capitalization.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::yields::buyback), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn buyback(net_buybacks: f64, market_cap: f64) -> Result<f64> {
        yield_ratio("buyback_yield", net_buybacks, market_cap)
    }
    /// Total shareholder distributions / market capitalization.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::yields::shareholder), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn shareholder(dividends: f64, net_buybacks: f64, market_cap: f64) -> Result<f64> {
        all_finite("shareholder_yield", &[dividends, net_buybacks, market_cap])?;
        yield_ratio("shareholder_yield", dividends + net_buybacks, market_cap)
    }
}

/// Return-on-capital ratios and the `DuPont` decomposition.
pub mod returns {
    use super::{all_finite, output, FinanceError, Result};
    fn ret(name: &'static str, numerator: f64, denominator: f64) -> Result<f64> {
        all_finite(name, &[numerator, denominator])?;
        if denominator <= 0.0 {
            return Err(FinanceError::Undefined { what: name });
        }
        output(name, numerator / denominator)
    }
    /// Net income / average equity.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::returns::roe), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn roe(net_income: f64, average_equity: f64) -> Result<f64> {
        ret("roe", net_income, average_equity)
    }
    /// Net income / average assets.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::returns::roa), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn roa(net_income: f64, average_assets: f64) -> Result<f64> {
        ret("roa", net_income, average_assets)
    }
    /// NOPAT / invested capital.
    /// ```rust
    /// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::returns::roic), 0);
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn roic(nopat: f64, invested_capital: f64) -> Result<f64> {
        ret("roic", nopat, invested_capital)
    }
}

/// `DuPont` factors. Their product equals `net_income / equity` when sales/assets/equity are positive.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::dupont), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn dupont(
    net_income: f64,
    sales: f64,
    average_assets: f64,
    average_equity: f64,
) -> Result<f64> {
    all_finite(
        "dupont",
        &[net_income, sales, average_assets, average_equity],
    )?;
    if sales <= 0.0 || average_assets <= 0.0 || average_equity <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "dupont denominators",
            reason: "sales, assets, and equity must be positive",
        });
    }
    output(
        "dupont",
        (net_income / sales) * (sales / average_assets) * (average_assets / average_equity),
    )
}

/// Multi-ratio return outputs for callers that need all ratios together.
#[derive(Debug, Clone, Copy, PartialEq)]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::equity::ReturnRatios>() > 0);
/// ```
pub struct ReturnRatios {
    /// Roe for the return ratios result.
    pub roe: f64,
    /// Roa for the return ratios result.
    pub roa: f64,
    /// Roic for the return ratios result.
    pub roic: f64,
}

/// Calculate ROE, ROA and ROIC in one call.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::returns), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn returns(
    net_income: f64,
    average_equity: f64,
    average_assets: f64,
    nopat: f64,
    invested_capital: f64,
) -> Result<ReturnRatios> {
    Ok(ReturnRatios {
        roe: returns::roe(net_income, average_equity)?,
        roa: returns::roa(net_income, average_assets)?,
        roic: returns::roic(nopat, invested_capital)?,
    })
}

/// Multi-yield output for callers that need all payout components together.
#[derive(Debug, Clone, Copy, PartialEq)]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::equity::YieldSet>() > 0);
/// ```
pub struct YieldSet {
    /// Earnings for the yield set result.
    pub earnings: f64,
    /// Fcf for the yield set result.
    pub fcf: f64,
    /// Dividend for the yield set result.
    pub dividend: f64,
    /// Buyback for the yield set result.
    pub buyback: f64,
    /// Shareholder for the yield set result.
    pub shareholder: f64,
}

/// Calculate common shareholder yields using market capitalization as denominator.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::yields), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn yields(
    earnings: f64,
    free_cash_flow: f64,
    dividends: f64,
    net_buybacks: f64,
    market_cap: f64,
) -> Result<YieldSet> {
    Ok(YieldSet {
        earnings: yields::earnings(earnings, market_cap)?,
        fcf: yields::fcf(free_cash_flow, market_cap)?,
        dividend: yields::dividend(dividends, market_cap)?,
        buyback: yields::buyback(net_buybacks, market_cap)?,
        shareholder: yields::shareholder(dividends, net_buybacks, market_cap)?,
    })
}

/// Net current asset value per share: `(current assets - liabilities - preferred) / shares`.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::ncav_per_share), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn ncav_per_share(
    current_assets: f64,
    total_liabilities: f64,
    preferred: f64,
    shares: f64,
) -> Result<f64> {
    all_finite(
        "ncav inputs",
        &[current_assets, total_liabilities, preferred, shares],
    )?;
    if shares <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "shares",
            reason: "must be positive",
        });
    }
    output(
        "ncav_per_share",
        (current_assets - total_liabilities - preferred) / shares,
    )
}

/// Graham's defensive-investor number: `sqrt(22.5 * EPS * book value per share)`.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::graham_number), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn graham_number(eps: f64, book_value_per_share: f64) -> Result<f64> {
    all_finite("graham inputs", &[eps, book_value_per_share])?;
    if eps < 0.0 || book_value_per_share < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "graham inputs",
            reason: "EPS and book value per share must be nonnegative",
        });
    }
    output("graham_number", (22.5 * eps * book_value_per_share).sqrt())
}

/// Peer multiple aggregation statistic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::equity::PeerStat>() > 0);
/// ```
pub enum PeerStat {
    /// The median option.
    Median,
    /// The mean option.
    Mean,
    /// The quartiles option.
    Quartiles,
}
/// Implied company value or range from peer multiples and a company metric.
#[derive(Debug, Clone, Copy, PartialEq)]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::equity::MultipleSet>() > 0);
/// ```
pub struct MultipleSet {
    /// Low for the multiple set result.
    pub low: f64,
    /// Central for the multiple set result.
    pub central: f64,
    /// High for the multiple set result.
    pub high: f64,
}

/// Apply a peer set's mean, median, or interquartile range to an operating metric.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::comps_implied_value), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn comps_implied_value(
    metric: f64,
    peer_multiples: &[f64],
    stat: PeerStat,
) -> Result<MultipleSet> {
    finite("metric", metric)?;
    all_finite("peer_multiples", peer_multiples)?;
    if peer_multiples.is_empty() {
        return Err(FinanceError::TooShort {
            what: "peer_multiples",
            min: 1,
            got: 0,
        });
    }
    if peer_multiples.iter().any(|m| *m < 0.0) {
        return Err(FinanceError::InvalidInput {
            what: "peer_multiples",
            reason: "multiples must be nonnegative",
        });
    }
    let mut sorted = peer_multiples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mean = sorted.iter().sum::<f64>() / sorted.len() as f64;
    let median = quantile(&sorted, 0.5);
    let q1 = quantile(&sorted, 0.25);
    let q3 = quantile(&sorted, 0.75);
    let result = match stat {
        PeerStat::Median => MultipleSet {
            low: median * metric,
            central: median * metric,
            high: median * metric,
        },
        PeerStat::Mean => MultipleSet {
            low: mean * metric,
            central: mean * metric,
            high: mean * metric,
        },
        PeerStat::Quartiles => MultipleSet {
            low: q1 * metric,
            central: median * metric,
            high: q3 * metric,
        },
    };
    Ok(MultipleSet {
        low: output("comps value", result.low)?,
        central: output("comps value", result.central)?,
        high: output("comps value", result.high)?,
    })
}

#[cfg(test)]
#[path = "ratios_comps_tests.rs"]
mod comps_tests;
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
fn quantile(sorted: &[f64], q: f64) -> f64 {
    let pos = q * (sorted.len() - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    sorted[lo] + (sorted[hi] - sorted[lo]) * (pos - lo as f64)
}

/// Annualized short carry cost, net of rebate and dividend income.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::short_carry), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn short_carry(borrow_fee: f64, dividend_yield: f64, rebate: f64) -> Result<f64> {
    all_finite("short carry inputs", &[borrow_fee, dividend_yield, rebate])?;
    if borrow_fee < 0.0 || dividend_yield < 0.0 || rebate < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "short carry inputs",
            reason: "fee, dividend yield, and rebate must be nonnegative",
        });
    }
    output("short_carry", borrow_fee - dividend_yield - rebate)
}

/// Free cash flow margin: `FCF / revenue`.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::equity::fcf_margin), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn fcf_margin(free_cash_flow: f64, revenue: f64) -> Result<f64> {
    all_finite("fcf margin", &[free_cash_flow, revenue])?;
    if revenue <= 0.0 {
        return Err(FinanceError::Undefined {
            what: "fcf_margin revenue",
        });
    }
    output("fcf_margin", free_cash_flow / revenue)
}

#[cfg(test)]
#[path = "ratios_tests.rs"]
mod tests;
