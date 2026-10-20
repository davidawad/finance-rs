//! Preferred liquidation allocation and exit-value conversion breakpoints.
//!
//! ```
//! use finance_rs::vc::waterfall::{liquidation, PreferredClass};
//! let out = liquidation(&[PreferredClass { shares: 1.0, liquidation_multiple: 1.0,
//!     seniority: 0, participating: false, participation_cap: None }], 1.0, 3.0)?;
//! assert!((out.common + out.preferred - 3.0).abs() < 1e-12);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

use super::{nonnegative, output, positive, FinanceError, Result};
/// Preferred security terms; seniority is ascending (0 is paid first).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreferredClass {
    /// Shares for the preferred class result.
    pub shares: f64,
    /// Liquidation multiple for the preferred class result.
    pub liquidation_multiple: f64,
    /// Seniority for the preferred class result.
    pub seniority: u32,
    /// Participating for the preferred class result.
    pub participating: bool,
    /// Participation cap for the preferred class result.
    pub participation_cap: Option<f64>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
/// Data structure for payouts.
pub struct Payouts {
    /// Common for the payouts result.
    pub common: f64,
    /// Preferred for the payouts result.
    pub preferred: f64,
}
/// Allocate proceeds, honoring seniority and the larger of preference or as-converted value.
///
/// # Errors
/// Returns an error for invalid share terms, non-finite inputs, no shares,
/// or conversion elections that do not reach a stable fixed point.
pub fn liquidation(
    classes: &[PreferredClass],
    common_shares: f64,
    exit_value: f64,
) -> Result<Payouts> {
    nonnegative("common_shares", common_shares)?;
    nonnegative("exit_value", exit_value)?;
    for c in classes {
        nonnegative("preferred shares", c.shares)?;
        nonnegative("liquidation_multiple", c.liquidation_multiple)?;
        if let Some(cap) = c.participation_cap {
            positive("participation_cap", cap)?;
            if cap < 1.0 {
                return Err(FinanceError::InvalidInput {
                    what: "participation_cap",
                    reason: "must be at least 1x the liquidation preference",
                });
            }
        }
    }
    let total_shares = common_shares + classes.iter().map(|c| c.shares).sum::<f64>();
    if total_shares.is_infinite() {
        return Err(FinanceError::Overflow {
            what: "waterfall shares",
        });
    }
    if total_shares == 0.0 {
        return Err(FinanceError::Undefined {
            what: "waterfall with no shares",
        });
    }
    let converted = solve_conversion(classes, common_shares, exit_value)?;
    let (class_paid, common) = simulate_payouts(classes, common_shares, exit_value, &converted);
    let pref_paid = class_paid.iter().sum();
    Ok(Payouts {
        common: output("common payout", common)?,
        preferred: output("preferred payout", pref_paid)?,
    })
}

fn solve_conversion(
    classes: &[PreferredClass],
    common_shares: f64,
    exit_value: f64,
) -> Result<Vec<bool>> {
    let max_iterations = classes.len().saturating_mul(4).saturating_add(8);
    solve_conversion_state(
        classes,
        common_shares,
        exit_value,
        vec![false; classes.len()],
        Vec::new(),
        max_iterations,
    )
}

fn solve_conversion_state(
    classes: &[PreferredClass],
    common_shares: f64,
    exit_value: f64,
    mut converted: Vec<bool>,
    mut visited: Vec<Vec<bool>>,
    max_iterations: usize,
) -> Result<Vec<bool>> {
    // Compare each non-participating class's preference and as-converted
    // payout while holding other classes' elections fixed. This converges to
    // a joint election for ordinary venture waterfalls; cycles are explicit
    // solver errors rather than guessed payouts.
    for _ in 0..max_iterations {
        if visited.contains(&converted) {
            return Err(FinanceError::NoConvergence {
                what: "venture conversion elections",
                iterations: visited.len(),
            });
        }
        visited.push(converted.clone());
        let (current_payouts, _) = simulate_payouts(classes, common_shares, exit_value, &converted);
        let mut changed = false;
        for index in 0..classes.len() {
            if classes[index].participating {
                continue;
            }
            let mut alternative = converted.clone();
            alternative[index] = !alternative[index];
            let (alternative_payouts, _) =
                simulate_payouts(classes, common_shares, exit_value, &alternative);
            let tolerance = 1e-12 * exit_value.max(1.0);
            if alternative_payouts[index] > current_payouts[index] + tolerance {
                converted[index] = alternative[index];
                changed = true;
            }
        }
        if !changed {
            return Ok(converted);
        }
    }
    Err(FinanceError::NoConvergence {
        what: "venture conversion elections",
        iterations: max_iterations,
    })
}

fn simulate_payouts(
    classes: &[PreferredClass],
    common_shares: f64,
    exit_value: f64,
    converted: &[bool],
) -> (Vec<f64>, f64) {
    let mut order: Vec<usize> = (0..classes.len()).collect();
    order.sort_by_key(|index| classes[*index].seniority);
    let mut remaining = exit_value;
    let mut class_paid = vec![0.0; classes.len()];
    for index in order {
        let c = classes[index];
        if (!c.participating && converted[index]) || remaining <= 0.0 {
            continue;
        }
        let take = (c.shares * c.liquidation_multiple).min(remaining);
        class_paid[index] = take;
        remaining -= take;
    }

    residual_distribution(classes, common_shares, remaining, converted, class_paid)
}

fn residual_distribution(
    classes: &[PreferredClass],
    common_shares: f64,
    mut remaining: f64,
    converted: &[bool],
    mut class_paid: Vec<f64>,
) -> (Vec<f64>, f64) {
    // Residual proceeds are shared by common, participating preferred, and
    // non-participating classes that elected conversion. Capped participants
    // leave the pool once their aggregate payout reaches their cap.
    let mut active = vec![true; classes.len()];
    let mut common_paid = 0.0;
    let common_active = common_shares > 0.0;
    for _ in 0..=classes.len() {
        if remaining <= 0.0 {
            break;
        }
        let mut weight = if common_active { common_shares } else { 0.0 };
        for (index, c) in classes.iter().enumerate() {
            if active[index] && (c.participating || converted[index]) {
                weight += c.shares;
            }
        }
        if weight <= 0.0 {
            break;
        }
        let pool = remaining;
        let mut allocated = 0.0;
        for (index, c) in classes.iter().enumerate() {
            if !active[index] || !(c.participating || converted[index]) {
                continue;
            }
            let share = pool * c.shares / weight;
            let cap_left = if c.participating {
                c.participation_cap.map_or(f64::INFINITY, |cap| {
                    (c.shares * c.liquidation_multiple * cap - class_paid[index]).max(0.0)
                })
            } else {
                f64::INFINITY
            };
            let take = share.min(cap_left);
            class_paid[index] += take;
            allocated += take;
            if take + 1e-12 < share {
                active[index] = false;
            }
        }
        if common_active {
            let take = pool * common_shares / weight;
            common_paid += take;
            allocated += take;
        }
        if allocated <= 0.0 {
            break;
        }
        remaining = (remaining - allocated).max(0.0);
    }
    (class_paid, common_paid)
}
/// Candidate conversion/cap thresholds in exit-value space.
///
/// # Errors
/// Returns an error for invalid share terms or non-finite breakpoints.
pub fn breakpoints(classes: &[PreferredClass], common_shares: f64) -> Result<Vec<f64>> {
    nonnegative("common_shares", common_shares)?;
    let mut out = Vec::new();
    let total = common_shares + classes.iter().map(|c| c.shares).sum::<f64>();
    positive("total shares", total)?;
    for c in classes {
        nonnegative("shares", c.shares)?;
        nonnegative("liquidation_multiple", c.liquidation_multiple)?;
        if c.shares == 0.0 {
            continue;
        }
        out.push(output(
            "waterfall conversion breakpoint",
            c.liquidation_multiple * total,
        )?);
        if let Some(cap) = c.participation_cap {
            positive("participation_cap", cap)?;
            if cap < 1.0 {
                return Err(FinanceError::InvalidInput {
                    what: "participation_cap",
                    reason: "must be at least 1x the liquidation preference",
                });
            }
            out.push(output(
                "waterfall participation cap breakpoint",
                c.liquidation_multiple * cap * total,
            )?);
        }
    }
    out.sort_by(f64::total_cmp);
    out.dedup_by(|a, b| (*a - *b).abs() <= 1e-12);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[allow(clippy::too_many_lines)] // This cohesive regression scenario is intentionally kept together to show the full contract.
    fn participation_seniority_caps_and_breakpoints() -> Result<()> {
        let classes = [
            PreferredClass {
                shares: 1.0,
                liquidation_multiple: 1.0,
                seniority: 1,
                participating: true,
                participation_cap: Some(2.0),
            },
            PreferredClass {
                shares: 2.0,
                liquidation_multiple: 1.0,
                seniority: 0,
                participating: false,
                participation_cap: None,
            },
        ];
        let payouts = liquidation(&classes, 3.0, 10.0)?;
        assert!((payouts.common + payouts.preferred - 10.0).abs() < 1e-12);
        assert!(liquidation(&[], 0.0, 10.0).is_err());
        assert!(liquidation(&[], 0.0, 0.0).is_err());
        assert_eq!(liquidation(&classes, 3.0, 0.0)?.common, 0.0);
        let zero_share = [PreferredClass {
            shares: 0.0,
            ..classes[0]
        }];
        assert!(liquidation(&zero_share, 0.0, 1.0).is_err());
        assert!(liquidation(&[], -1.0, 1.0).is_err());
        assert!(liquidation(&[], 1.0, -1.0).is_err());
        assert!(liquidation(
            &[PreferredClass {
                shares: -1.0,
                ..classes[0]
            }],
            1.0,
            1.0
        )
        .is_err());
        assert!(liquidation(
            &[PreferredClass {
                participation_cap: Some(0.0),
                ..classes[0]
            }],
            1.0,
            1.0
        )
        .is_err());
        assert!(liquidation(
            &[PreferredClass {
                participation_cap: Some(0.5),
                ..classes[0]
            }],
            1.0,
            1.0
        )
        .is_err());
        assert!(liquidation(&[], f64::MAX, f64::MAX).is_err());
        assert!(matches!(
            liquidation(
                &[PreferredClass {
                    shares: f64::MAX,
                    ..classes[0]
                }],
                f64::MAX,
                1.0
            ),
            Err(FinanceError::Overflow {
                what: "waterfall shares"
            })
        ));
        assert_eq!(breakpoints(&classes, 3.0)?.len(), 2);
        assert_eq!(
            breakpoints(
                &[PreferredClass {
                    shares: 0.0,
                    ..classes[0]
                }],
                1.0
            )?
            .len(),
            0
        );
        assert!(breakpoints(&[], 0.0).is_err());
        assert!(breakpoints(&[], -1.0).is_err());
        assert!(breakpoints(
            &[PreferredClass {
                participation_cap: Some(0.5),
                ..classes[0]
            }],
            1.0
        )
        .is_err());
        assert!(breakpoints(
            &[PreferredClass {
                shares: -1.0,
                ..classes[0]
            }],
            1.0
        )
        .is_err());
        let inactive = [PreferredClass {
            participating: false,
            participation_cap: None,
            ..classes[0]
        }];
        assert_eq!(
            residual_distribution(&inactive, 0.0, 10.0, &[false], vec![1.0]),
            (vec![1.0], 0.0)
        );
        assert_eq!(
            residual_distribution(&[], 1.0, 10.0, &[], vec![]),
            (vec![], 10.0)
        );
        let capped = [PreferredClass {
            participating: true,
            participation_cap: Some(1.0),
            ..classes[0]
        }];
        assert_eq!(
            residual_distribution(&capped, 0.0, 5.0, &[false], vec![1.0]),
            (vec![1.0], 0.0)
        );
        let convert = [PreferredClass {
            shares: 1.0,
            liquidation_multiple: 1.0,
            seniority: 0,
            participating: false,
            participation_cap: None,
        }];
        assert!(matches!(
            solve_conversion_state(&convert, 1.0, 10.0, vec![false], vec![vec![false]], 4),
            Err(FinanceError::NoConvergence { iterations: 1, .. })
        ));
        assert!(matches!(
            solve_conversion_state(&convert, 1.0, 10.0, vec![false], Vec::new(), 1),
            Err(FinanceError::NoConvergence { iterations: 1, .. })
        ));
        Ok(())
    }
}
