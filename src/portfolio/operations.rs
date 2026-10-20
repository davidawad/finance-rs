use super::{
    matrix::{check_matrix, dot, mat_vec},
    Band,
};
use crate::error::{all_finite, output, FinanceError, Result};

fn aligned(a: &[f64], b: &[f64], what: &'static str) -> Result<()> {
    if a.len() != b.len() {
        return Err(FinanceError::LengthMismatch {
            left: "left",
            right: "right",
            left_len: a.len(),
            right_len: b.len(),
        });
    }
    all_finite(what, a)?;
    all_finite(what, b)
}
/// Expected portfolio return `wᵀμ`.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn return_(weights: &[f64], expected_returns: &[f64]) -> Result<f64> {
    aligned(weights, expected_returns, "portfolio inputs")?;
    output("portfolio return", dot(weights, expected_returns))
}
/// Portfolio variance `wᵀΣw`.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn variance(weights: &[f64], cov: &[Vec<f64>]) -> Result<f64> {
    check_matrix("covariance", cov, weights.len())?;
    all_finite("weights", weights)?;
    output("portfolio variance", dot(weights, &mat_vec(cov, weights))).and_then(|v| {
        if v < -1e-10 {
            Err(FinanceError::InvalidInput {
                what: "covariance",
                reason: "produced negative portfolio variance",
            })
        } else {
            Ok(v.max(0.))
        }
    })
}
/// Portfolio volatility `sqrt(wᵀΣw)`.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn volatility(weights: &[f64], cov: &[Vec<f64>]) -> Result<f64> {
    variance(weights, cov).map(f64::sqrt)
}
/// Absolute risk contributions `w_i(Σw)_i / σ_p`.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn risk_contributions(weights: &[f64], cov: &[Vec<f64>]) -> Result<Vec<f64>> {
    let vol = volatility(weights, cov)?;
    if vol == 0. {
        return Err(FinanceError::Undefined {
            what: "portfolio volatility",
        });
    }
    let m = mat_vec(cov, weights);
    Ok(weights.iter().zip(m).map(|(w, x)| w * x / vol).collect())
}
/// Diversification ratio `Σ |w_i|σ_i / σ_p`.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn diversification_ratio(weights: &[f64], cov: &[Vec<f64>]) -> Result<f64> {
    check_matrix("covariance", cov, weights.len())?;
    let vol = volatility(weights, cov)?;
    if vol == 0. {
        return Err(FinanceError::Undefined {
            what: "portfolio volatility",
        });
    }
    output(
        "diversification ratio",
        weights
            .iter()
            .enumerate()
            .map(|(i, w)| w.abs() * cov[i][i].max(0.).sqrt())
            .sum::<f64>()
            / vol,
    )
}
/// Herfindahl-Hirschman concentration for nonnegative portfolio shares.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn concentration_hhi(weights: &[f64]) -> Result<f64> {
    if weights.is_empty() {
        return Err(FinanceError::TooShort {
            what: "weights",
            min: 1,
            got: 0,
        });
    }
    all_finite("weights", weights)?;
    if weights.iter().any(|w| *w < 0.) {
        return Err(FinanceError::InvalidInput {
            what: "weights",
            reason: "must be nonnegative",
        });
    }
    let total = weights.iter().sum::<f64>();
    if total <= 0. {
        return Err(FinanceError::Undefined {
            what: "total weight",
        });
    }
    output(
        "concentration HHI",
        weights.iter().map(|w| (w / total).powi(2)).sum(),
    )
}

/// Allocate a numeric budget in proportion to positive weights, applying a
/// minimum allocation floor to each included line.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn risk_budget_allocations(
    total_budget: f64,
    weights: &[f64],
    min_per_line: f64,
) -> Result<Vec<f64>> {
    all_finite("risk budget", weights)?;
    all_finite("risk budget parameters", &[total_budget, min_per_line])?;
    if total_budget < 0.0 || min_per_line < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "risk budget",
            reason: "budget and line floor must be nonnegative",
        });
    }
    let sum = weights.iter().filter(|w| **w > 0.0).sum::<f64>();
    if sum <= 0.0 || total_budget == 0.0 {
        return Ok(Vec::new());
    }
    weights
        .iter()
        .filter(|w| **w > 0.0)
        .map(|w| {
            output(
                "risk budget allocation",
                (w / sum * total_budget).max(min_per_line),
            )
        })
        .collect()
}
/// Scale gross portfolio exposure to a target volatility, subject to a cap.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn vol_target_leverage(target_sigma: f64, realized_sigma: f64, cap: f64) -> Result<f64> {
    all_finite("volatility target", &[target_sigma, realized_sigma, cap])?;
    if target_sigma < 0. || realized_sigma <= 0. || cap < 0. {
        return Err(FinanceError::InvalidInput {
            what: "volatility target",
            reason: "target/cap nonnegative and realized volatility positive",
        });
    }
    output(
        "volatility target leverage",
        (target_sigma / realized_sigma).min(cap),
    )
}
/// Kelly portfolio `fraction * Σ⁻¹μ`; `mu` is expected excess return.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn kelly_portfolio(mu: &[f64], cov: &[Vec<f64>], fraction: f64) -> Result<Vec<f64>> {
    use super::matrix::solve;
    check_matrix("covariance", cov, mu.len())?;
    all_finite("expected excess returns", mu)?;
    all_finite("fraction", &[fraction])?;
    if fraction < 0. {
        return Err(FinanceError::InvalidInput {
            what: "fraction",
            reason: "must be nonnegative",
        });
    }
    Ok(solve(cov, mu)?.iter().map(|x| x * fraction).collect())
}
/// Turnover: half the sum of absolute weight changes.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn turnover(before: &[f64], after: &[f64]) -> Result<f64> {
    aligned(before, after, "weights")?;
    output(
        "turnover",
        before
            .iter()
            .zip(after)
            .map(|(a, b)| (b - a).abs())
            .sum::<f64>()
            * 0.5,
    )
}
/// Gross leverage `Σ |position| / equity`.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn gross_leverage(positions: &[f64], equity: f64) -> Result<f64> {
    all_finite("positions", positions)?;
    all_finite("equity", &[equity])?;
    if equity <= 0. {
        return Err(FinanceError::InvalidInput {
            what: "equity",
            reason: "must be positive",
        });
    }
    output(
        "gross leverage",
        positions.iter().map(|x| x.abs()).sum::<f64>() / equity,
    )
}
/// Trade amounts in dollars to move current holdings to target weights.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn rebalance_trades(
    holdings: &[f64],
    prices: &[f64],
    target_weights: &[f64],
    total_value: f64,
    min_trade: f64,
) -> Result<Vec<f64>> {
    all_finite("holdings", holdings)?;
    all_finite("prices", prices)?;
    all_finite("target_weights", target_weights)?;
    all_finite("rebalance parameters", &[total_value, min_trade])?;
    if holdings.len() != prices.len() || prices.len() != target_weights.len() {
        return Err(FinanceError::InvalidInput {
            what: "rebalance inputs",
            reason: "holdings, prices, and targets must align",
        });
    }
    if prices.iter().any(|x| *x <= 0.) || total_value < 0. || min_trade < 0. {
        return Err(FinanceError::InvalidInput {
            what: "rebalance inputs",
            reason: "prices positive and value/min_trade nonnegative",
        });
    }
    Ok(holdings
        .iter()
        .zip(prices)
        .zip(target_weights)
        .map(|((h, p), w)| {
            let d = w * total_value - h * p;
            if d.abs() < min_trade {
                0.
            } else {
                d
            }
        })
        .collect())
}
/// Post-return weights after buy-and-hold drift.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn drift(weights: &[f64], returns: &[f64]) -> Result<Vec<f64>> {
    aligned(weights, returns, "weights and returns")?;
    if returns.iter().any(|r| *r <= -1.) {
        return Err(FinanceError::InvalidInput {
            what: "returns",
            reason: "must exceed -100%",
        });
    }
    let mut x: Vec<_> = weights
        .iter()
        .zip(returns)
        .map(|(w, r)| w * (1. + r))
        .collect();
    let s = x.iter().sum::<f64>();
    if s == 0. {
        return Err(FinanceError::Undefined {
            what: "post-return portfolio value",
        });
    }
    for w in &mut x {
        *w /= s;
    }
    Ok(x)
}
/// Decide whether any absolute or relative rebalance band is breached.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn needs_rebalance(weights: &[f64], targets: &[f64], band: Band) -> Result<bool> {
    aligned(weights, targets, "weights")?;
    let (v, relative) = match band {
        Band::Absolute(v) => (v, false),
        Band::Relative(v) => (v, true),
    };
    if !v.is_finite() || v < 0. {
        return Err(FinanceError::InvalidInput {
            what: "band",
            reason: "must be nonnegative and finite",
        });
    }
    Ok(weights.iter().zip(targets).any(|(w, t)| {
        if relative {
            if *t == 0. {
                *w != 0.
            } else {
                (w - t).abs() / t.abs() > v
            }
        } else {
            (w - t).abs() > v
        }
    }))
}
/// Greedy whole-share allocation. Returns `(shares, uninvested_cash)`.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
pub fn discrete_allocation(weights: &[f64], prices: &[f64], total: f64) -> Result<(Vec<u64>, f64)> {
    aligned(weights, prices, "allocation inputs")?;
    all_finite("total", &[total])?;
    if weights.iter().any(|w| *w < 0.) || prices.iter().any(|p| *p <= 0.) || total < 0. {
        return Err(FinanceError::InvalidInput {
            what: "allocation inputs",
            reason: "weights nonnegative, prices positive, total nonnegative",
        });
    }
    let mut shares: Vec<u64> = weights
        .iter()
        .zip(prices)
        .map(|(w, p)| (w * total / p).floor() as u64)
        .collect();
    let mut cash = total
        - shares
            .iter()
            .zip(prices)
            .map(|(s, p)| *s as f64 * p)
            .sum::<f64>();
    loop {
        let mut best = None;
        for i in 0..shares.len() {
            if prices[i] <= cash
                && best.is_none_or(|j| weights[i] / prices[i] > weights[j] / prices[j])
            {
                best = Some(i);
            }
        }
        if let Some(i) = best {
            let max_add = (cash / prices[i]).floor() as u64;
            let added = max_add.min(u64::MAX - shares[i]);
            if added == 0 {
                break;
            }
            shares[i] += added;
            cash -= added as f64 * prices[i];
        } else {
            break;
        }
    }
    Ok((shares, cash.max(0.)))
}
/// Linear transaction cost: variable basis points plus fixed fee per trade.
/// ```rust
/// assert!((finance_rs::portfolio::return_(&[0.6, 0.4], &[0.05, 0.10]).unwrap() - 0.07).abs() < 1e-12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn transaction_cost(trades: &[f64], basis_points: f64, fixed_per_trade: f64) -> Result<f64> {
    all_finite("trades", trades)?;
    all_finite("cost parameters", &[basis_points, fixed_per_trade])?;
    if basis_points < 0. || fixed_per_trade < 0. {
        return Err(FinanceError::InvalidInput {
            what: "cost parameters",
            reason: "must be nonnegative",
        });
    }
    output(
        "transaction cost",
        trades
            .iter()
            .map(|x| x.abs() * basis_points / 10_000. + if *x == 0. { 0. } else { fixed_per_trade })
            .sum(),
    )
}
