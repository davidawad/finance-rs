//! Fund portfolio helpers shared by workbook and analysis callers.

use crate::error::{finite, output, FinanceError, Result};
use std::collections::BTreeMap;

/// A fund holding or an underlying exposure: (identifier, value).
pub type Exposure = (String, f64);

/// Aggregate holding values by underlying exposure, preserving total value.
/// Each mapping row is (holding identifier, exposure identifier, weight).
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::look_through), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn look_through(
    holdings: &[Exposure],
    underlying_exposures: &[(String, String, f64)],
) -> Result<BTreeMap<String, f64>> {
    let mut totals = BTreeMap::new();
    for (id, value) in holdings {
        finite("holding_value", *value)?;
        if id.trim().is_empty() {
            return Err(invalid("holding_id", "must not be empty"));
        }
        let rows = underlying_exposures
            .iter()
            .filter(|r| r.0 == *id)
            .collect::<Vec<_>>();
        if rows.is_empty() {
            *totals.entry(id.clone()).or_insert(0.0) += value;
            continue;
        }
        let weight: f64 = rows.iter().map(|r| r.2).sum();
        if (weight - 1.0).abs() > 1e-8 {
            return Err(invalid("exposure_weights", "must sum to one per holding"));
        }
        for (_, name, share) in rows {
            finite("exposure_share", *share)?;
            if !(0.0..=1.0).contains(share) || name.trim().is_empty() {
                return Err(invalid("exposure", "requires named shares in [0,1]"));
            }
            *totals.entry(name.clone()).or_insert(0.0) += value * share;
        }
    }
    for value in totals.values_mut() {
        *value = output("exposure", *value)?;
    }
    Ok(totals)
}

/// Annual commitment target and actual values supplied by an LP portfolio.
#[derive(Debug, Clone, Copy, PartialEq)]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::FundPacingYear>() > 0);
/// ```
pub struct FundPacingYear {
    /// Year for the fund pacing year result.
    pub year: i32,
    /// Target commitment for the fund pacing year result.
    pub target_commitment: f64,
    /// Actual commitment for the fund pacing year result.
    pub actual_commitment: f64,
}
/// Annual and cumulative pace result.
#[derive(Debug, Clone, Copy, PartialEq)]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::FundPacingRow>() > 0);
/// ```
pub struct FundPacingRow {
    /// Year for the fund pacing row result.
    pub year: i32,
    /// Target for the fund pacing row result.
    pub target: f64,
    /// Actual for the fund pacing row result.
    pub actual: f64,
    /// Variance for the fund pacing row result.
    pub variance: f64,
    /// Cumulative target for the fund pacing row result.
    pub cumulative_target: f64,
    /// Cumulative actual for the fund pacing row result.
    pub cumulative_actual: f64,
}

/// Validate, sort and summarize an explicit commitment pacing schedule.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::pacing_schedule), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn pacing_schedule(years: &[FundPacingYear]) -> Result<Vec<FundPacingRow>> {
    if years.is_empty() {
        return Err(FinanceError::TooShort {
            what: "years",
            min: 1,
            got: 0,
        });
    }
    let mut rows = years.to_vec();
    rows.sort_by_key(|r| r.year);
    if rows.windows(2).any(|w| w[0].year == w[1].year) {
        return Err(invalid("years", "must be unique"));
    }
    let (mut ct, mut ca) = (0.0, 0.0);
    rows.into_iter()
        .map(|r| {
            for (what, x) in [
                ("target_commitment", r.target_commitment),
                ("actual_commitment", r.actual_commitment),
            ] {
                finite(what, x)?;
                if x < 0.0 {
                    return Err(invalid(what, "must be non-negative"));
                }
            }
            ct = output("cumulative_target", ct + r.target_commitment)?;
            ca = output("cumulative_actual", ca + r.actual_commitment)?;
            Ok(FundPacingRow {
                year: r.year,
                target: r.target_commitment,
                actual: r.actual_commitment,
                variance: r.actual_commitment - r.target_commitment,
                cumulative_target: ct,
                cumulative_actual: ca,
            })
        })
        .collect()
}

fn invalid(what: &'static str, reason: &'static str) -> FinanceError {
    FinanceError::InvalidInput { what, reason }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn look_through_aggregates_direct_and_underlying_positions() -> Result<()> {
        let positions = [("fund".into(), 100.0), ("cash".into(), 20.0)];
        let allocations = [
            ("fund".into(), "equity".into(), 0.6),
            ("fund".into(), "credit".into(), 0.4),
        ];
        let result = look_through(&positions, &allocations)?;
        assert_eq!(result.get("equity"), Some(&60.0));
        assert_eq!(result.get("credit"), Some(&40.0));
        assert_eq!(result.get("cash"), Some(&20.0));
        Ok(())
    }
    #[test]
    fn pacing_schedule_orders_rows_and_accumulates_targets() -> Result<()> {
        let years = [
            FundPacingYear {
                year: 2027,
                target_commitment: 20.0,
                actual_commitment: 15.0,
            },
            FundPacingYear {
                year: 2026,
                target_commitment: 10.0,
                actual_commitment: 12.0,
            },
        ];
        let rows = pacing_schedule(&years)?;
        assert_eq!(rows[0].year, 2026);
        assert_eq!(rows[1].cumulative_actual, 27.0);
        Ok(())
    }
    #[test]
    fn pacing_rejects_duplicate_years() {
        let row = FundPacingYear {
            year: 2026,
            target_commitment: 1.0,
            actual_commitment: 1.0,
        };
        assert!(pacing_schedule(&[row, row]).is_err());
    }

    #[test]
    fn look_through_and_pacing_reject_invalid_rows() {
        let holding = vec![("fund".to_owned(), 10.0)];
        assert!(look_through(&[(String::new(), 1.0)], &[]).is_err());
        assert!(look_through(&holding, &[("fund".into(), "equity".into(), 0.5)]).is_err());
        assert!(look_through(&holding, &[("fund".into(), String::new(), 1.0)]).is_err());
        assert!(look_through(&holding, &[("fund".into(), "equity".into(), 1.1)]).is_err());
        assert!(look_through(&[("fund".into(), f64::NAN)], &[]).is_err());
        assert!(pacing_schedule(&[]).is_err());
        for (target, actual) in [(-1.0, 0.0), (0.0, -1.0), (f64::INFINITY, 0.0)] {
            assert!(pacing_schedule(&[FundPacingYear {
                year: 2026,
                target_commitment: target,
                actual_commitment: actual,
            }])
            .is_err());
        }
    }
}
