use super::CreditCurve;
use crate::{Date, FinanceError, Result};

fn pay_dates(effective: Date, maturity: Date) -> Vec<Date> {
    let mut dates = Vec::new();
    // At the upper supported year, there is no later IMM date to return. In
    // that case maturity is the only remaining payment date.
    let mut date = super::next_imm_date(effective).unwrap_or(maturity);
    while date < maturity {
        dates.push(date);
        if date.month() == 12 && date.year() >= maturity.year() {
            break;
        }
        date = super::next_imm_date(date).unwrap_or(maturity);
    }
    dates.push(maturity);
    dates
}

fn following_payment_date(date: Date) -> Result<Date> {
    let mut payment_date = date;
    while matches!(
        payment_date.weekday(),
        crate::Weekday::Saturday | crate::Weekday::Sunday
    ) {
        payment_date = payment_date.add_days(1)?;
    }
    Ok(payment_date)
}

fn segment_dates(curve: &CreditCurve, start: Date, end: Date) -> Vec<Date> {
    let mut dates = Vec::with_capacity(curve.nodes.len() + 2);
    dates.push(start);
    dates.extend(
        curve
            .nodes
            .iter()
            .map(|node| node.maturity)
            .filter(|date| *date > start && *date < end),
    );
    dates.push(end);
    dates
}

#[allow(clippy::cast_precision_loss)] // Date's supported range keeps day counts within financial precision.
fn default_accrual_segment(
    curve: &CreditCurve,
    discount_rate: f64,
    valuation_date: Date,
    accrual_start: Date,
    payment_end: Date,
) -> Result<f64> {
    let step_in = valuation_date.add_days(1)?;
    let schedule_start = accrual_start;
    let protected_start = schedule_start.max(step_in);
    // `protected_start` is at least one day after the valuation date, so the
    // prior day is inside Date's supported range for every valid input.
    let start = protected_start.add_days(-1).unwrap_or(valuation_date);
    let end = payment_end.add_days(-1)?;
    let t_start =
        schedule_start.add_days(-1)?.days_since(valuation_date) as f64 / 365.0 - 1.0 / 730.0;
    let mut accrued = 0.0;

    for dates in segment_dates(curve, start, end).windows(2) {
        let (left, right) = (dates[0], dates[1]);
        let days = right.days_since(left);
        if days <= 0 {
            continue;
        }
        let p0 = (-discount_rate * left.days_since(valuation_date) as f64 / 365.0).exp();
        let p1 = (-discount_rate * right.days_since(valuation_date) as f64 / 365.0).exp();
        let q0 = curve.survival_probability(left);
        let q1 = curve.survival_probability(right);
        let f_hat = p0.ln() - p1.ln();
        let h_hat = q0.ln() - q1.ln();
        let combined = f_hat + h_hat;
        let interval = days as f64 / 365.0;
        let (discount_survival_average, first_moment_average) = if combined.abs() < 1e-4 {
            let square = combined * combined;
            (
                1.0 - 0.5 * combined + square / 6.0 - square * combined / 24.0
                    + square * square / 120.0,
                0.5 - combined / 3.0 + square / 8.0 - square * combined / 30.0
                    + square * square / 144.0,
            )
        } else {
            let tail = (-combined).exp();
            (
                -(-combined).exp_m1() / combined,
                (1.0 - tail * (1.0 + combined)) / (combined * combined),
            )
        };
        let t0 = left.days_since(valuation_date) as f64 / 365.0;
        let accrual = h_hat
            * p0
            * q0
            * ((t0 - t_start) * discount_survival_average + interval * first_moment_average)
            * (365.0 / 360.0);
        accrued += accrual;
    }
    Ok(accrued)
}

#[allow(clippy::cast_precision_loss)] // Date's supported range keeps day counts within financial precision.
pub(super) fn legs(
    valuation_date: Date,
    maturity: Date,
    discount_rate: f64,
    recovery: f64,
    curve: &CreditCurve,
) -> Result<(f64, f64)> {
    let mut previous_maturity = curve.valuation_date;
    for node in &curve.nodes {
        if node.maturity < previous_maturity
            || (node.maturity == previous_maturity && previous_maturity > curve.valuation_date)
        {
            return Err(FinanceError::InvalidInput {
                what: "credit curve",
                reason: "hazard maturities must not decrease",
            });
        }
        if !node.hazard_rate.is_finite() {
            return Err(FinanceError::NonFinite {
                what: "hazard_rate",
            });
        }
        if node.hazard_rate < 0.0 {
            return Err(FinanceError::InvalidInput {
                what: "hazard_rate",
                reason: "must be non-negative",
            });
        }
        previous_maturity = node.maturity;
    }

    let dates = pay_dates(valuation_date, maturity);
    let mut protection = 0.0;
    let mut annuity = 0.0;
    let mut accrual_start = valuation_date;

    for accrual_end in dates {
        let accrual_days = accrual_end.days_since(accrual_start);
        if accrual_days <= 0 {
            return Err(FinanceError::InvalidInput {
                what: "payment dates",
                reason: "must be increasing",
            });
        }

        for dates in segment_dates(curve, accrual_start, accrual_end).windows(2) {
            let (start, end) = (dates[0], dates[1]);
            let days = end.days_since(start);
            let hazard = curve.hazard_at(end);
            let combined = discount_rate + hazard;
            let interval = days as f64 / 365.0;
            let default_pv = if combined.abs() < 1e-12 {
                hazard * interval
            } else {
                hazard * (1.0 - (-combined * interval).exp()) / combined
            };
            let survival = curve.survival_probability(start);
            let discount_factor =
                (-discount_rate * start.days_since(valuation_date) as f64 / 365.0).exp();
            protection += (1.0 - recovery) * survival * discount_factor * default_pv;
        }

        let payment_date = following_payment_date(accrual_end)?;
        // Payment dates follow a positive accrual interval, so the preceding
        // day cannot underflow the supported date range.
        let payment_day = payment_date.add_days(-1).unwrap_or(valuation_date);
        let payment_survival = curve.survival_probability(payment_day);
        let payment_discount =
            (-discount_rate * payment_date.days_since(valuation_date) as f64 / 365.0).exp();
        annuity += (accrual_days as f64 / 360.0) * payment_survival * payment_discount;
        annuity += default_accrual_segment(
            curve,
            discount_rate,
            valuation_date,
            accrual_start,
            payment_date,
        )?;
        accrual_start = accrual_end;
    }

    if !protection.is_finite() || !annuity.is_finite() || annuity <= 0.0 {
        return Err(FinanceError::NoSolution { what: "CDS legs" });
    }
    Ok((protection, annuity))
}

#[cfg(test)]
mod tests {
    use super::{default_accrual_segment, following_payment_date, legs, pay_dates};
    use crate::cds::{CreditCurve, HazardNode};
    use crate::Date;

    #[test]
    fn payment_dates_follow_weekends_without_changing_accrual_dates() {
        assert_eq!(
            following_payment_date(Date::new(2025, 9, 20).unwrap()).unwrap(),
            Date::new(2025, 9, 22).unwrap()
        );
        assert_eq!(
            following_payment_date(Date::new(2025, 9, 22).unwrap()).unwrap(),
            Date::new(2025, 9, 22).unwrap()
        );
    }

    #[test]
    fn default_accrual_applies_half_day_bias_and_step_in() {
        let valuation = Date::new(2024, 3, 20).unwrap();
        let curve = CreditCurve {
            valuation_date: valuation,
            nodes: vec![],
        };
        let first_period = default_accrual_segment(
            &curve,
            0.03,
            valuation,
            valuation,
            Date::new(2024, 6, 20).unwrap(),
        )
        .unwrap();
        assert_eq!(first_period, 0.0);

        let empty_segment = default_accrual_segment(
            &curve,
            0.03,
            valuation,
            valuation,
            valuation.add_days(1).unwrap(),
        )
        .unwrap();
        assert_eq!(empty_segment, 0.0);
    }

    #[test]
    fn date_range_edges_return_errors_instead_of_wrapping() {
        let last_day = Date::new(1_000_000, 12, 31).unwrap();
        assert_eq!(pay_dates(last_day, last_day), vec![last_day]);

        let weekend_at_limit = (29..=31)
            .map(|day| Date::new(1_000_000, 12, day).unwrap())
            .find(|date| {
                matches!(
                    date.weekday(),
                    crate::Weekday::Saturday | crate::Weekday::Sunday
                )
            })
            .unwrap();
        assert!(following_payment_date(weekend_at_limit).is_err());

        let payment_weekend_at_limit = (29..=31)
            .map(|day| Date::new(1_000_000, 12, day).unwrap())
            .find(|date| {
                matches!(
                    date.weekday(),
                    crate::Weekday::Saturday | crate::Weekday::Sunday
                )
            })
            .unwrap();
        let upper_curve = CreditCurve {
            valuation_date: Date::new(1_000_000, 12, 1).unwrap(),
            nodes: vec![],
        };
        assert!(legs(
            upper_curve.valuation_date,
            payment_weekend_at_limit,
            0.03,
            0.4,
            &upper_curve,
        )
        .is_err());

        let upper_curve = CreditCurve {
            valuation_date: last_day,
            nodes: vec![],
        };
        assert!(default_accrual_segment(&upper_curve, 0.0, last_day, last_day, last_day,).is_err());

        let lower_day = Date::new(-1_000_000, 1, 1).unwrap();
        let lower_curve = CreditCurve {
            valuation_date: lower_day,
            nodes: vec![],
        };
        assert!(default_accrual_segment(
            &lower_curve,
            0.0,
            lower_day,
            lower_day,
            Date::new(-1_000_000, 1, 2).unwrap(),
        )
        .is_err());
        assert!(legs(
            lower_day,
            Date::new(-1_000_000, 4, 1).unwrap(),
            0.03,
            0.4,
            &lower_curve,
        )
        .is_err());
        assert!(default_accrual_segment(
            &lower_curve,
            0.0,
            lower_day,
            Date::new(-1_000_000, 1, 2).unwrap(),
            lower_day,
        )
        .is_err());
    }

    #[test]
    fn legs_reject_nonfinite_and_decreasing_hazard_nodes() {
        let valuation = Date::new(2024, 3, 20).unwrap();
        let maturity = Date::new(2025, 3, 20).unwrap();
        for nodes in [
            vec![HazardNode {
                maturity,
                hazard_rate: f64::NAN,
            }],
            vec![
                HazardNode {
                    maturity: Date::new(2025, 6, 20).unwrap(),
                    hazard_rate: 0.02,
                },
                HazardNode {
                    maturity,
                    hazard_rate: 0.03,
                },
            ],
            vec![
                HazardNode {
                    maturity,
                    hazard_rate: 0.02,
                },
                HazardNode {
                    maturity,
                    hazard_rate: 0.03,
                },
            ],
        ] {
            let curve = CreditCurve {
                valuation_date: valuation,
                nodes,
            };
            assert!(legs(valuation, maturity, 0.03, 0.4, &curve).is_err());
        }
    }
}
