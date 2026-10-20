//! Excel depreciation schedules (DB, DDB, SLN, SYD and VDB).

use crate::error::{finite, output, FinanceError, Result};

/// Whether variable declining balance may switch to straight-line depreciation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum SwitchMode {
    /// Switch when straight-line depreciation becomes larger.
    Allow,
    /// Keep declining-balance depreciation for the full term.
    DecliningBalanceOnly,
}

fn common(cost: f64, salvage: f64, life: f64) -> Result<()> {
    finite("cost", cost)?;
    finite("salvage", salvage)?;
    finite("life", life)?;
    if cost <= 0.0 || salvage < 0.0 || salvage >= cost || life <= 0.0 || life > 100_000.0 {
        return Err(FinanceError::InvalidInput {
            what: "cost/salvage/life",
            reason: "cost and life must be positive (life <= 100000) and 0 <= salvage < cost",
        });
    }
    Ok(())
}

fn period(period: f64, life: f64) -> Result<()> {
    finite("period", period)?;
    if period < 1.0 || period > life || period.fract() != 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "period",
            reason: "must be an integer from 1 through life",
        });
    }
    Ok(())
}

/// Straight-line depreciation per period (Excel `SLN`).
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn sln(cost: f64, salvage: f64, life: f64) -> Result<f64> {
    common(cost, salvage, life)?;
    output("sln", (cost - salvage) / life)
}

/// Sum-of-years-digits depreciation for one period (Excel `SYD`).
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn syd(cost: f64, salvage: f64, life: f64, period_number: f64) -> Result<f64> {
    common(cost, salvage, life)?;
    period(period_number, life)?;
    output(
        "syd",
        (cost - salvage) * (life - period_number + 1.0) * 2.0 / (life * (life + 1.0)),
    )
}

/// Double-declining balance depreciation (Excel `DDB`); `factor` defaults to 2 in Excel.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
pub fn ddb(cost: f64, salvage: f64, life: f64, period_number: f64, factor: f64) -> Result<f64> {
    common(cost, salvage, life)?;
    period(period_number, life)?;
    finite("factor", factor)?;
    if factor <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "factor",
            reason: "must be positive",
        });
    }
    let rate = factor / life;
    let mut book = cost;
    for _ in 1..period_number as usize {
        book -= (book * rate).min(book - salvage);
    }
    output("ddb", (book * rate).min((book - salvage).max(0.0)))
}

/// Fixed-declining-balance depreciation (Excel `DB`), with `month` in 1..=12.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
#[allow(clippy::float_cmp)] // Exact zero and endpoint comparisons define this algorithm’s boundary behavior.
pub fn db(cost: f64, salvage: f64, life: f64, period_number: f64, month: f64) -> Result<f64> {
    common(cost, salvage, life)?;
    finite("month", month)?;
    finite("period", period_number)?;
    if month < 1.0 || month > 12.0 || month.fract() != 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "month",
            reason: "must be an integer from 1 through 12",
        });
    }
    if period_number < 1.0 || period_number > life.ceil() || period_number.fract() != 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "period",
            reason: "must be a positive integer no greater than ceil(life)",
        });
    }
    let rate = (1.0 - (salvage / cost).powf(1.0 / life) + 0.0005) * 1000.0;
    let rate = rate.floor() / 1000.0;
    let p = period_number as usize;
    let first = cost * rate * month / 12.0;
    let mut book = cost - first;
    if p == 1 {
        return output("db", first);
    }
    for _ in 2..p {
        book -= book * rate;
    }
    let depreciation = if p as f64 == life.ceil() {
        (book - salvage).max(0.0)
    } else {
        book * rate
    };
    output("db", depreciation)
}

/// Variable declining balance depreciation between fractional periods (Excel `VDB`).
/// `switch_mode` selects whether to switch to straight line when it is larger.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
pub fn vdb(
    cost: f64,
    salvage: f64,
    life: f64,
    start_period: f64,
    end_period: f64,
    factor: f64,
    switch_mode: SwitchMode,
) -> Result<f64> {
    common(cost, salvage, life)?;
    finite("start_period", start_period)?;
    finite("end_period", end_period)?;
    finite("factor", factor)?;
    if start_period < 0.0 || end_period <= start_period || end_period > life || factor <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "periods/factor",
            reason: "require 0 <= start < end <= life and positive factor",
        });
    }
    let rate = factor / life;
    let mut book = cost;
    let mut result = 0.0;
    for n in 0..life.ceil() as usize {
        let begin = n as f64;
        let end = (begin + 1.0).min(life);
        let overlap = (end.min(end_period) - begin.max(start_period)).max(0.0);
        if overlap > 0.0 {
            let db_amount = (book * rate).min((book - salvage).max(0.0));
            let sl_amount = ((book - salvage).max(0.0)) / (life - begin);
            let amount = if switch_mode == SwitchMode::Allow && sl_amount > db_amount {
                sl_amount
            } else {
                db_amount
            };
            result += amount * overlap / (end - begin);
        }
        let db_amount = (book * rate).min((book - salvage).max(0.0));
        // n is below ceil(life), so begin is strictly less than life for
        // every iteration, including a fractional final period.
        let sl_amount = (book - salvage).max(0.0) / (life - begin);
        book -= if switch_mode == SwitchMode::Allow && sl_amount > db_amount {
            sl_amount
        } else {
            db_amount
        };
        book = book.max(salvage);
    }
    output("vdb", result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedules_cover_switches_fractional_periods_and_caps() {
        assert_eq!(sln(1_000.0, 100.0, 3.0).unwrap(), 300.0);
        assert_eq!(syd(1_000.0, 100.0, 3.0, 1.0).unwrap(), 450.0);
        assert!(ddb(1_000.0, 900.0, 3.0, 3.0, 2.0).unwrap() <= 100.0);
        let first = db(1_000.0, 100.0, 5.0, 1.0, 6.0).unwrap();
        let last = db(1_000.0, 100.0, 5.0, 5.0, 12.0).unwrap();
        assert!(first > 0.0 && last >= 0.0);
        let declining = vdb(
            1_000.0,
            100.0,
            5.0,
            1.5,
            2.5,
            2.0,
            SwitchMode::DecliningBalanceOnly,
        )
        .unwrap();
        let switched = vdb(1_000.0, 100.0, 5.0, 1.5, 2.5, 2.0, SwitchMode::Allow).unwrap();
        assert!(declining > 0.0 && switched >= declining);
        assert!(vdb(1_000.0, 0.0, 5.0, 0.0, 5.0, 2.0, SwitchMode::Allow).unwrap() > 0.0);
        assert!(vdb(100.0, 90.0, 2.5, 2.0, 2.5, 20.0, SwitchMode::Allow).unwrap() <= 10.0);
    }

    #[test]
    fn invalid_inputs_are_rejected_for_every_schedule() {
        assert!(sln(f64::NAN, 0.0, 1.0).is_err());
        assert!(sln(1.0, 1.0, 1.0).is_err());
        assert!(syd(10.0, 0.0, 2.0, 1.5).is_err());
        assert!(ddb(10.0, 0.0, 2.0, 1.0, 0.0).is_err());
        assert!(db(10.0, 0.0, 2.0, 1.0, 0.0).is_err());
        assert!(db(10.0, 0.0, 2.0, 3.0, 1.0).is_err());
        assert!(vdb(10.0, 0.0, 2.0, -1.0, 1.0, 2.0, SwitchMode::Allow).is_err());
        assert!(vdb(10.0, 0.0, 2.0, 0.0, 1.0, 0.0, SwitchMode::Allow).is_err());
        assert!(sln(10.0, f64::NAN, 2.0).is_err());
        assert!(sln(10.0, 0.0, f64::NAN).is_err());
        assert!(sln(-1.0, 0.0, 2.0).is_err());
        assert!(sln(10.0, 0.0, 100_001.0).is_err());
        assert!(syd(10.0, 0.0, 2.0, f64::NAN).is_err());
        assert!(ddb(10.0, 0.0, 2.0, 1.0, f64::NAN).is_err());
        assert!(db(10.0, 0.0, 2.0, 1.0, f64::NAN).is_err());
        assert!(db(10.0, 0.0, 2.0, f64::NAN, 1.0).is_err());
        assert!(db(10.0, 0.0, 2.0, 1.5, 1.0).is_err());
        assert!(db(1_000.0, 100.0, 5.0, 2.0, 12.0).unwrap() > 0.0);
        assert!(vdb(10.0, 0.0, 2.0, f64::NAN, 1.0, 2.0, SwitchMode::Allow).is_err());
        assert!(vdb(10.0, 0.0, 2.0, 0.0, f64::NAN, 2.0, SwitchMode::Allow).is_err());
        assert!(vdb(10.0, 0.0, 2.0, 0.0, 1.0, f64::NAN, SwitchMode::Allow).is_err());
    }
}
