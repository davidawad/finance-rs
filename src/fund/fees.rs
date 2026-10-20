//! ILPA-style management fee basis, step-down and transaction fee offsets.
use crate::error::{all_finite, output, FinanceError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// `FeeBasis` type.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::FeeBasis>() > 0);
/// ```
pub enum FeeBasis {
    /// Committed.
    Committed,
    /// Invested.
    Invested,
    /// Nav.
    Nav,
}

#[derive(Debug, Clone, PartialEq)]
/// `FeeSchedule` type.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::FeeSchedule>() > 0);
/// ```
pub struct FeeSchedule {
    /// Basis by period.
    pub basis_by_period: Vec<FeeBasis>,
    /// Rate by period.
    pub rate_by_period: Vec<f64>,
    /// (first period, replacement annual rate) step-downs.
    pub step_downs: Vec<(usize, f64)>,
    /// Offset pct.
    pub offset_pct: f64,
}

#[derive(Debug, Clone, PartialEq)]
/// `ManagementFee` type.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::ManagementFee>() > 0);
/// ```
pub struct ManagementFee {
    /// Gross.
    pub gross: Vec<f64>,
    /// Offsets.
    pub offsets: Vec<f64>,
    /// Net.
    pub net: Vec<f64>,
}

/// Fee per period is the selected fee base times that period's annual rate.
/// Committed capital is usual during investment; invested capital or NAV may
/// be selected after a contractual step-down. Inputs are period-end snapshots.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::management_fee), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn management_fee(
    commitments: &[f64],
    invested: &[f64],
    nav: &[f64],
    schedule: &FeeSchedule,
) -> Result<ManagementFee> {
    let n = commitments.len();
    if n == 0 {
        return Err(FinanceError::TooShort {
            what: "commitments",
            min: 1,
            got: 0,
        });
    }
    for (name, xs) in [
        ("invested", invested),
        ("nav", nav),
        ("rate_by_period", schedule.rate_by_period.as_slice()),
    ] {
        if xs.len() != n {
            return Err(FinanceError::LengthMismatch {
                left: "commitments",
                left_len: n,
                right: name,
                right_len: xs.len(),
            });
        }
    }
    if schedule.basis_by_period.len() != n {
        return Err(FinanceError::LengthMismatch {
            left: "commitments",
            left_len: n,
            right: "basis_by_period",
            right_len: schedule.basis_by_period.len(),
        });
    }
    all_finite("commitments", commitments)?;
    all_finite("invested", invested)?;
    all_finite("nav", nav)?;
    all_finite("fee_rates", &schedule.rate_by_period)?;
    if commitments
        .iter()
        .chain(invested)
        .chain(nav)
        .any(|x| *x < 0.0)
        || schedule.rate_by_period.iter().any(|x| *x < 0.0)
        || !(0.0..=1.0).contains(&schedule.offset_pct)
    {
        return Err(invalid(
            "fee inputs must be nonnegative and offset_pct in [0,1]",
        ));
    }
    let mut steps = schedule.step_downs.clone();
    steps.sort_by_key(|x| x.0);
    if steps.windows(2).any(|w| w[0].0 == w[1].0)
        || steps
            .iter()
            .any(|(i, r)| *i >= n || !r.is_finite() || *r < 0.0)
    {
        return Err(invalid(
            "step-down periods must be unique/in-range and rates nonnegative",
        ));
    }
    let mut gross = Vec::with_capacity(n);
    for i in 0..n {
        let rate = steps
            .iter()
            .rfind(|(period, _)| *period <= i)
            .map_or(schedule.rate_by_period[i], |(_, r)| *r);
        let base = match schedule.basis_by_period[i] {
            FeeBasis::Committed => commitments[i],
            FeeBasis::Invested => invested[i],
            FeeBasis::Nav => nav[i],
        };
        gross.push(output("management_fee", base * rate)?);
    }
    let offsets = gross
        .iter()
        .map(|fee| fee * schedule.offset_pct)
        .collect::<Vec<_>>();
    let net = gross
        .iter()
        .zip(&offsets)
        .map(|(fee, off)| fee - off)
        .collect();
    Ok(ManagementFee {
        gross,
        offsets,
        net,
    })
}

/// Amount of transaction fees credited against management fees.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::offsets), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn offsets(transaction_fees: f64, offset_pct: f64) -> Result<f64> {
    if !transaction_fees.is_finite() || !offset_pct.is_finite() {
        return Err(FinanceError::NonFinite {
            what: "transaction_fees/offset_pct",
        });
    }
    if transaction_fees < 0.0 || !(0.0..=1.0).contains(&offset_pct) {
        return Err(invalid(
            "transaction fees must be nonnegative and offset_pct in [0,1]",
        ));
    }
    output("fee_offset", transaction_fees * offset_pct)
}
fn invalid(reason: &'static str) -> FinanceError {
    FinanceError::InvalidInput {
        what: "fee_schedule",
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fee_basis_stepdown_and_offset_textbook_values() -> Result<()> {
        let got = management_fee(
            &[100., 100.],
            &[80., 80.],
            &[75., 70.],
            &FeeSchedule {
                basis_by_period: vec![FeeBasis::Committed, FeeBasis::Invested],
                rate_by_period: vec![0.02, 0.02],
                step_downs: vec![(1, 0.01)],
                offset_pct: 0.5,
            },
        )?;
        assert_eq!(got.gross, vec![2., 0.8]);
        assert_eq!(got.net, vec![1., 0.4]);
        assert_eq!(offsets(10., 0.8)?, 8.);
        Ok(())
    }
    #[test]
    fn rejects_misaligned_fee_inputs() {
        assert!(management_fee(
            &[1.],
            &[],
            &[1.],
            &FeeSchedule {
                basis_by_period: vec![FeeBasis::Nav],
                rate_by_period: vec![0.02],
                step_downs: vec![],
                offset_pct: 0.
            }
        )
        .is_err());
    }

    #[test]
    fn fee_schedule_validates_each_shape_and_applies_nav_steps() {
        let schedule = FeeSchedule {
            basis_by_period: vec![FeeBasis::Nav, FeeBasis::Nav],
            rate_by_period: vec![0.02, 0.03],
            step_downs: vec![(1, 0.01)],
            offset_pct: 0.25,
        };
        let fees = management_fee(&[100., 100.], &[80., 80.], &[50., 40.], &schedule).unwrap();
        assert_eq!(fees.gross, vec![1.0, 0.4]);
        assert_eq!(fees.offsets, vec![0.25, 0.1]);
        assert!(management_fee(&[], &[], &[], &schedule).is_err());
        let base = FeeSchedule {
            basis_by_period: vec![FeeBasis::Committed],
            rate_by_period: vec![0.01],
            step_downs: vec![],
            offset_pct: 0.0,
        };
        assert!(management_fee(
            &[1.],
            &[1.],
            &[1.],
            &FeeSchedule {
                rate_by_period: vec![],
                ..base.clone()
            }
        )
        .is_err());
        assert!(management_fee(
            &[1.],
            &[1.],
            &[1.],
            &FeeSchedule {
                basis_by_period: vec![],
                ..base.clone()
            }
        )
        .is_err());
        assert!(management_fee(
            &[1.],
            &[1.],
            &[1.],
            &FeeSchedule {
                offset_pct: 1.1,
                ..base.clone()
            }
        )
        .is_err());
        assert!(management_fee(&[-1.], &[1.], &[1.], &base).is_err());
        assert!(management_fee(
            &[1.],
            &[1.],
            &[1.],
            &FeeSchedule {
                step_downs: vec![(0, 0.01), (0, 0.02)],
                ..base.clone()
            }
        )
        .is_err());
        assert!(management_fee(
            &[1.],
            &[1.],
            &[1.],
            &FeeSchedule {
                step_downs: vec![(1, 0.01)],
                ..base
            }
        )
        .is_err());
        assert!(offsets(f64::NAN, 0.5).is_err());
        assert!(offsets(1.0, 1.1).is_err());
    }
}
