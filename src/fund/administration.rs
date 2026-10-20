//! Fund administration: capital accounts, calls, pacing and liquidity.
use crate::error::{all_finite, output, FinanceError, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
/// Fund-period allocations applied to an LP capital account.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::CapitalAllocations>() > 0);
/// ```
pub struct CapitalAllocations {
    /// Allocated investment income.
    pub income: f64,
    /// Realized investment gain or loss.
    pub realized: f64,
    /// Unrealized investment gain or loss.
    pub unrealized: f64,
    /// Management and fund fees charged to the account.
    pub fees: f64,
    /// Carried interest charged to the account.
    pub carry: f64,
}
#[cfg(feature = "decimal")]
#[derive(Debug, Clone, Copy, PartialEq)]
/// Decimal-exact fund-period allocations for reconciled capital accounts.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::DecimalCapitalAllocations>() > 0);
/// ```
pub struct DecimalCapitalAllocations {
    /// Allocated investment income.
    pub income: rust_decimal::Decimal,
    /// Realized investment gain or loss.
    pub realized: rust_decimal::Decimal,
    /// Unrealized investment gain or loss.
    pub unrealized: rust_decimal::Decimal,
    /// Management and fund fees charged to the account.
    pub fees: rust_decimal::Decimal,
    /// Carried interest charged to the account.
    pub carry: rust_decimal::Decimal,
}

/// Decimal-exact capital-account roll-forward for statement reconciliation.
#[cfg(feature = "decimal")]
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::capital_account_roll_forward_decimal), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn capital_account_roll_forward_decimal(
    begin: rust_decimal::Decimal,
    contributions: rust_decimal::Decimal,
    distributions: rust_decimal::Decimal,
    a: DecimalCapitalAllocations,
) -> Result<rust_decimal::Decimal> {
    use rust_decimal::Decimal;
    let zero = Decimal::ZERO;
    if [begin, contributions, distributions, a.fees, a.carry]
        .iter()
        .any(|value| *value < zero)
    {
        return Err(invalid(
            "opening balance, flows, fees and carry must be nonnegative",
        ));
    }
    begin
        .checked_add(contributions)
        .and_then(|value| value.checked_sub(distributions))
        .and_then(|value| value.checked_add(a.income))
        .and_then(|value| value.checked_add(a.realized))
        .and_then(|value| value.checked_add(a.unrealized))
        .and_then(|value| value.checked_sub(a.fees))
        .and_then(|value| value.checked_sub(a.carry))
        .ok_or(FinanceError::Overflow {
            what: "decimal_capital_account",
        })
}
#[derive(Debug, Clone, PartialEq)]
/// Investor allocation in a capital call.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::CapitalCallAllocation>() > 0);
/// ```
pub struct CapitalCallAllocation {
    /// Investor name used to identify the allocation.
    pub investor: String,
    /// Allocated amount in currency units.
    pub amount: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
/// Equalization catch-up and interest due on a late closing.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::Equalization>() > 0);
/// ```
pub struct Equalization {
    /// Preferred return catch-up amount.
    pub catch_up: f64,
    /// Accrued equalization interest.
    pub interest: f64,
    /// Combined catch-up and interest.
    pub total: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
/// Limits for calculating recallable distributions.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::RecallTerms>() > 0);
/// ```
pub struct RecallTerms {
    /// Maximum recallable amount.
    pub cap: f64,
    /// Amount already recalled under the cap.
    pub recalled_to_date: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
/// Recallable amount and remaining capacity under the terms.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::Recallable>() > 0);
/// ```
pub struct Recallable {
    /// Allocated amount in currency units.
    pub amount: f64,
    /// Remaining capacity.
    pub remaining_capacity: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
/// Trough and recovery timing for cumulative fund cash flows.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::JCurve>() > 0);
/// ```
pub struct JCurve {
    /// Lowest cumulative net cash flow.
    pub trough: f64,
    /// Index of the trough period.
    pub trough_period: usize,
    /// First period with nonnegative cumulative cash flow.
    pub breakeven_period: Option<usize>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
/// Unfunded commitments available relative to expected obligations.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::UnfundedCoverage>() > 0);
/// ```
pub struct UnfundedCoverage {
    /// Coverage of obligations by unfunded commitments.
    pub ratio: f64,
    /// Unfunded commitment remaining after obligations.
    pub surplus: f64,
}

/// Roll forward an LP capital account: opening + calls − distributions plus
/// income and realized/unrealized allocations, less fees and carry.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::capital_account_roll_forward), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn capital_account_roll_forward(
    begin: f64,
    contributions: f64,
    distributions: f64,
    a: CapitalAllocations,
) -> Result<f64> {
    let xs = [
        begin,
        contributions,
        distributions,
        a.income,
        a.realized,
        a.unrealized,
        a.fees,
        a.carry,
    ];
    all_finite("capital_account", &xs)?;
    if begin < 0. || contributions < 0. || distributions < 0. || a.fees < 0. || a.carry < 0. {
        return Err(invalid(
            "begin, calls, distributions, fees and carry must be nonnegative",
        ));
    }
    output(
        "ending_capital_account",
        begin + contributions - distributions + a.income + a.realized + a.unrealized
            - a.fees
            - a.carry,
    )
}

/// Allocate a capital call by unfunded commitment, rounded to cents using
/// largest remainders; input is `(investor, commitment, called-to-date)`.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::capital_call_allocate), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
pub fn capital_call_allocate(
    amount: f64,
    commitments: &[(String, f64, f64)],
) -> Result<Vec<CapitalCallAllocation>> {
    if !amount.is_finite() || amount < 0. || commitments.is_empty() {
        return Err(invalid(
            "amount must be nonnegative and commitments nonempty",
        ));
    }
    let mut total = 0.0;
    let mut weights = Vec::with_capacity(commitments.len());
    for (name, commitment, called) in commitments {
        if name.trim().is_empty()
            || !commitment.is_finite()
            || !called.is_finite()
            || *commitment < 0.
            || *called < 0.
            || called > commitment
        {
            return Err(invalid(
                "investors need names and called capital within commitment",
            ));
        }
        let u = commitment - called;
        total += u;
        weights.push(u);
    }
    if total <= 0. && amount > 0. {
        return Err(FinanceError::Undefined {
            what: "unfunded commitments",
        });
    }
    if amount > total {
        return Err(invalid(
            "capital call cannot exceed total unfunded commitment",
        ));
    }
    let cents = (amount * 100.0).round() as i64;
    let shares = weights
        .iter()
        .map(|u| {
            if total == 0.0 {
                0.0
            } else {
                cents as f64 * u / total
            }
        })
        .collect::<Vec<_>>();
    let mut rounded = shares.iter().map(|x| x.floor() as i64).collect::<Vec<_>>();
    let mut order = (0..shares.len()).collect::<Vec<_>>();
    order.sort_by(|a, b| {
        (shares[*b] - shares[*b].floor())
            .total_cmp(&(shares[*a] - shares[*a].floor()))
            .then_with(|| commitments[*a].0.cmp(&commitments[*b].0))
    });
    let left = cents - rounded.iter().sum::<i64>();
    for i in order.into_iter().take(left.max(0) as usize) {
        rounded[i] += 1;
    }
    Ok(commitments
        .iter()
        .zip(rounded)
        .map(|(row, c)| CapitalCallAllocation {
            investor: row.0.clone(),
            amount: c as f64 / 100.0,
        })
        .collect())
}

/// Late investor pays its pro-rata share of prior calls plus simple interest.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::equalization), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn equalization(
    late_lp: f64,
    commitment: f64,
    prior_calls: f64,
    annual_rate: f64,
    days: u32,
) -> Result<Equalization> {
    let xs = [late_lp, commitment, prior_calls, annual_rate];
    all_finite("equalization", &xs)?;
    if late_lp < 0.
        || commitment < 0.
        || prior_calls < 0.
        || late_lp > commitment
        || annual_rate < 0.
    {
        return Err(invalid(
            "LP share and prior calls must be nonnegative, share <= commitment, rate nonnegative",
        ));
    }
    let catch_up = if commitment == 0.0 {
        0.0
    } else {
        prior_calls * late_lp / commitment
    };
    let interest = output(
        "equalization_interest",
        catch_up * annual_rate * f64::from(days) / 365.0,
    )?;
    Ok(Equalization {
        catch_up,
        interest,
        total: output("equalization_total", catch_up + interest)?,
    })
}

/// Recallable distributions are capped by the uncalled recall authorization.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::recallable), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn recallable(distributions: f64, terms: RecallTerms) -> Result<Recallable> {
    let xs = [distributions, terms.cap, terms.recalled_to_date];
    all_finite("recallable", &xs)?;
    if xs.iter().any(|x| *x < 0.0) {
        return Err(invalid("distribution and recall terms must be nonnegative"));
    }
    let remaining = (terms.cap - terms.recalled_to_date).max(0.0);
    let amount = distributions.min(remaining);
    Ok(Recallable {
        amount,
        remaining_capacity: remaining - amount,
    })
}

/// Takahashi-Alexander annual contribution, distribution and NAV paths.
/// Calls are a rate of uncalled commitment shaped by the bow exponent;
/// contributions enter at midyear, NAV grows at `growth + yield`, and yield
/// is distributed. Any residual NAV remains at the fund's stated life.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::takahashi_alexander), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn takahashi_alexander(
    commitment: f64,
    rate_of_contribution: f64,
    bow: f64,
    growth: f64,
    life: usize,
    yield_rate: f64,
) -> Result<Vec<(usize, f64, f64, f64)>> {
    let xs = [commitment, rate_of_contribution, bow, growth, yield_rate];
    all_finite("pacing", &xs)?;
    if commitment < 0.
        || !(0.0..=1.0).contains(&rate_of_contribution)
        || bow < 0.0
        || growth < -1.0
        || yield_rate < 0.0
        || life == 0
        || life > 1_000
    {
        return Err(invalid(
            "commitment nonnegative, contribution rate in [0,1], bow nonnegative, growth >= -1, yield nonnegative, life in 1..=1000",
        ));
    }
    let mut nav = 0.;
    let mut uncalled = commitment;
    let mut rows = Vec::with_capacity(life);
    for year in 0..life {
        let bow_factor = if commitment == 0.0 {
            0.0
        } else {
            (uncalled / commitment).powf(bow)
        };
        let contribution = (rate_of_contribution * uncalled * bow_factor).min(uncalled);
        let pre_growth = nav + contribution * 0.5;
        let pre_distribution = pre_growth * (1.0 + growth + yield_rate) + contribution * 0.5;
        let distribution = (yield_rate * pre_distribution).min(pre_distribution);
        nav = output("pacing_nav", pre_distribution - distribution)?;
        uncalled = output("uncalled_commitment", uncalled - contribution)?;
        rows.push((year + 1, contribution, distribution, nav));
    }
    Ok(rows)
}

/// J-curve trough and first nonnegative cumulative flow period.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::j_curve), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn j_curve(flows: &[f64]) -> Result<JCurve> {
    if flows.is_empty() {
        return Err(FinanceError::TooShort {
            what: "flows",
            min: 1,
            got: 0,
        });
    }
    all_finite("flows", flows)?;
    let (mut total, mut trough, mut trough_period) = (0.0, 0.0, 0);
    let mut breakeven = None;
    for (i, f) in flows.iter().enumerate() {
        total = output("cumulative_fund_flow", total + f)?;
        if total < trough {
            trough = total;
            trough_period = i;
        }
        if total >= 0.0 && breakeven.is_none() {
            breakeven = Some(i);
        }
    }
    Ok(JCurve {
        trough,
        trough_period,
        breakeven_period: breakeven,
    })
}

/// GP commitment amount as a stated share of fund commitment.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::gp_commitment), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn gp_commitment(commitment: f64, gp_pct: f64) -> Result<f64> {
    let xs = [commitment, gp_pct];
    all_finite("gp_commitment", &xs)?;
    if commitment < 0. || !(0.0..=1.0).contains(&gp_pct) {
        return Err(invalid("commitment nonnegative and GP percentage in [0,1]"));
    }
    output("gp_commitment", commitment * gp_pct)
}

/// Liquid asset coverage of unfunded commitments; zero unfunded means infinite
/// coverage, represented as `None` would be awkward for ratios, so it is 0/0.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::unfunded_coverage), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn unfunded_coverage(liquid_assets: f64, unfunded: f64) -> Result<UnfundedCoverage> {
    let xs = [liquid_assets, unfunded];
    all_finite("unfunded_coverage", &xs)?;
    if xs.iter().any(|x| *x < 0.0) {
        return Err(invalid(
            "assets and unfunded commitments must be nonnegative",
        ));
    }
    if unfunded == 0.0 {
        return Ok(UnfundedCoverage {
            ratio: 0.0,
            surplus: liquid_assets,
        });
    }
    Ok(UnfundedCoverage {
        ratio: output("coverage_ratio", liquid_assets / unfunded)?,
        surplus: output("coverage_surplus", liquid_assets - unfunded)?,
    })
}

fn invalid(reason: &'static str) -> FinanceError {
    FinanceError::InvalidInput {
        what: "fund_administration",
        reason,
    }
}

#[cfg(test)]
#[path = "administration_tests.rs"]
mod tests;
