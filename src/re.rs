//! Real-estate underwriting ratios: NOI, cap rate, cash-on-cash, DSCR, GRM,
//! breakeven occupancy, debt yield and LTV.
//!
//! All inputs are annual amounts. Every ratio requires a strictly positive
//! denominator (as in `finance-core`): a zero denominator is
//! [`FinanceError::Undefined`], a negative one [`FinanceError::InvalidInput`].
//! Numerators may be negative (a property can lose money).
//!
//! ```
//! use finance_rs::re::{cap_rate, dscr, noi};
//! let income = noi(120_000.0, 0.05, 2_000.0, 40_000.0)?;
//! assert!((income - 76_000.0).abs() < 1e-9);
//! assert!((cap_rate(income, 1_000_000.0)? - 0.076).abs() < 1e-12);
//! assert!((dscr(income, 60_000.0)? - 1.266_666_666_666_666_7).abs() < 1e-12);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

use crate::error::{div, finite, output, FinanceError, Result};

/// `num / den` with a finite numerator and a strictly positive denominator.
fn ratio(num_what: &'static str, num: f64, den_what: &'static str, den: f64) -> Result<f64> {
    finite(num_what, num)?;
    finite(den_what, den)?;
    if den < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: den_what,
            reason: "must be positive",
        });
    }
    div(den_what, num, den)
}

/// Net operating income.
///
/// `NOI = gross_potential_income·(1 − vacancy_rate) + other_income − operating_expenses`
///
/// ```
/// use finance_rs::re::noi;
/// assert!((noi(100_000.0, 0.10, 5_000.0, 35_000.0)? - 60_000.0).abs() < 1e-9);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for a `vacancy_rate` outside `[0, 1]`,
/// [`FinanceError::NonFinite`] for non-finite input.
pub fn noi(
    gross_potential_income: f64,
    vacancy_rate: f64,
    other_income: f64,
    operating_expenses: f64,
) -> Result<f64> {
    finite("gross_potential_income", gross_potential_income)?;
    finite("vacancy_rate", vacancy_rate)?;
    finite("other_income", other_income)?;
    finite("operating_expenses", operating_expenses)?;
    if !(0.0..=1.0).contains(&vacancy_rate) {
        return Err(FinanceError::InvalidInput {
            what: "vacancy_rate",
            reason: "must be between 0 and 1",
        });
    }
    output(
        "noi",
        gross_potential_income * (1.0 - vacancy_rate) + other_income - operating_expenses,
    )
}

/// Capitalization rate: `cap = NOI / property_value`.
///
/// ```
/// use finance_rs::re::cap_rate;
/// assert!((cap_rate(12_000.0, 200_000.0)? - 0.06).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::Undefined`] / [`FinanceError::InvalidInput`] for a zero /
/// negative value, [`FinanceError::NonFinite`] for non-finite input.
pub fn cap_rate(noi: f64, property_value: f64) -> Result<f64> {
    ratio("noi", noi, "property_value", property_value)
}

/// Value implied by a cap rate: `value = NOI / cap_rate` (direct capitalization).
///
/// ```
/// use finance_rs::re::value_from_cap_rate;
/// assert!((value_from_cap_rate(76_000.0, 0.08)? - 950_000.0).abs() < 1e-6);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`cap_rate`], for a non-positive `cap_rate`.
pub fn value_from_cap_rate(noi: f64, cap_rate: f64) -> Result<f64> {
    ratio("noi", noi, "cap_rate", cap_rate)
}

/// Cash-on-cash return: `CoC = annual pre-tax cash flow / cash invested`,
/// where pre-tax cash flow is NOI minus annual debt service.
///
/// ```
/// use finance_rs::re::cash_on_cash;
/// assert!((cash_on_cash(16_000.0, 200_000.0)? - 0.08).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`cap_rate`], for a non-positive `cash_invested`.
pub fn cash_on_cash(pre_tax_cash_flow: f64, cash_invested: f64) -> Result<f64> {
    ratio(
        "pre_tax_cash_flow",
        pre_tax_cash_flow,
        "cash_invested",
        cash_invested,
    )
}

/// Debt service coverage ratio: `DSCR = NOI / annual debt service`.
///
/// ```
/// use finance_rs::re::dscr;
/// assert!((dscr(12_000.0, 10_000.0)? - 1.2).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`cap_rate`], for a non-positive `annual_debt_service`.
pub fn dscr(noi: f64, annual_debt_service: f64) -> Result<f64> {
    ratio("noi", noi, "annual_debt_service", annual_debt_service)
}

/// Gross rent multiplier: `GRM = price / gross annual rent`.
///
/// ```
/// use finance_rs::re::grm;
/// assert!((grm(500_000.0, 60_000.0)? - 8.333_333_333_333_334).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`cap_rate`], for a non-positive `gross_annual_rent`.
pub fn grm(price: f64, gross_annual_rent: f64) -> Result<f64> {
    ratio("price", price, "gross_annual_rent", gross_annual_rent)
}

/// Breakeven occupancy:
/// `(operating_expenses + debt_service) / gross_potential_income`.
///
/// The occupancy at which effective income just covers expenses and debt
/// service; above `1` means the property cannot break even even fully leased.
///
/// ```
/// use finance_rs::re::breakeven_occupancy;
/// let b = breakeven_occupancy(40_000.0, 45_000.0, 120_000.0)?;
/// assert!((b - 0.708_333_333_333_333_4).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`cap_rate`], for a non-positive `gross_potential_income`.
pub fn breakeven_occupancy(
    operating_expenses: f64,
    debt_service: f64,
    gross_potential_income: f64,
) -> Result<f64> {
    finite("operating_expenses", operating_expenses)?;
    finite("debt_service", debt_service)?;
    ratio(
        "operating_expenses + debt_service",
        operating_expenses + debt_service,
        "gross_potential_income",
        gross_potential_income,
    )
}

/// Occupancy needed for net operating income to cover debt service.
/// `expense_ratio` is the variable operating-expense share of occupied rent;
/// vacancy is applied before that ratio. Fixed monthly costs are dollar amounts.
/// ```rust
/// assert!((finance_rs::re::cap_rate(12_000.0, 200_000.0).unwrap() - 0.06).abs() < 1e-15);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn breakeven_occupancy_net(
    debt_service: f64,
    fixed_monthly_expenses: f64,
    gross_monthly_rent: f64,
    vacancy_rate: f64,
    expense_ratio: f64,
) -> Result<f64> {
    for (name, value) in [
        ("debt_service", debt_service),
        ("fixed_monthly_expenses", fixed_monthly_expenses),
        ("gross_monthly_rent", gross_monthly_rent),
        ("vacancy_rate", vacancy_rate),
        ("expense_ratio", expense_ratio),
    ] {
        finite(name, value)?;
    }
    if !(0.0..=1.0).contains(&vacancy_rate) || !(0.0..=1.0).contains(&expense_ratio) {
        return Err(FinanceError::InvalidInput {
            what: "vacancy_rate/expense_ratio",
            reason: "must be in [0, 1]",
        });
    }
    if gross_monthly_rent <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "gross_monthly_rent",
            reason: "must be positive",
        });
    }
    let net_factor = (1.0 - vacancy_rate) * (1.0 - expense_ratio);
    if net_factor <= 0.0 {
        return Err(FinanceError::NoSolution {
            what: "breakeven occupancy",
        });
    }
    let occupancy = (debt_service + fixed_monthly_expenses) / (gross_monthly_rent * net_factor);
    output("breakeven occupancy", occupancy.min(1.0))
}

/// Debt yield: `NOI / loan amount`.
///
/// ```
/// use finance_rs::re::debt_yield;
/// assert!((debt_yield(76_000.0, 760_000.0)? - 0.1).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`cap_rate`], for a non-positive `loan_amount`.
pub fn debt_yield(noi: f64, loan_amount: f64) -> Result<f64> {
    ratio("noi", noi, "loan_amount", loan_amount)
}

/// Loan-to-value: `LTV = loan amount / property value`.
///
/// ```
/// use finance_rs::re::ltv;
/// assert!((ltv(750_000.0, 1_000_000.0)? - 0.75).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// As [`cap_rate`], for a non-positive `property_value`.
pub fn ltv(loan_amount: f64, property_value: f64) -> Result<f64> {
    ratio("loan_amount", loan_amount, "property_value", property_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finance_core_vectors() -> Result<()> {
        // Same vectors as finance-core's Decimal tests.
        assert!((cap_rate(12_000.0, 200_000.0)? - 0.06).abs() < 1e-15);
        assert!((dscr(12_000.0, 10_000.0)? - 1.2).abs() < 1e-15);
        Ok(())
    }

    #[test]
    fn worked_property() -> Result<()> {
        // 1.0M property, 120k GPI, 5% vacancy, 2k other, 40k opex,
        // 750k loan with 60k/yr debt service, 250k equity.
        let n = noi(120_000.0, 0.05, 2_000.0, 40_000.0)?;
        assert!((n - 76_000.0).abs() < 1e-9);
        assert!((cash_on_cash(n - 60_000.0, 250_000.0)? - 0.064).abs() < 1e-12);
        assert!((debt_yield(n, 750_000.0)? - 0.101_333_333_333_333_33).abs() < 1e-12);
        assert!(
            (breakeven_occupancy(40_000.0, 60_000.0, 120_000.0)? - 0.833_333_333_333_333_4).abs()
                < 1e-12
        );
        assert!((grm(1_000_000.0, 120_000.0)? - 8.333_333_333_333_334).abs() < 1e-12);
        assert!((value_from_cap_rate(n, cap_rate(n, 1_000_000.0)?)? - 1_000_000.0).abs() < 1e-6);
        // Negative cash flow is allowed.
        assert!(cash_on_cash(-5_000.0, 100_000.0)? < 0.0);
        Ok(())
    }

    #[test]
    fn denominators_must_be_positive() {
        assert_eq!(
            cap_rate(1.0, 0.0),
            Err(FinanceError::Undefined {
                what: "property_value"
            })
        );
        assert!(matches!(
            dscr(1.0, -1.0),
            Err(FinanceError::InvalidInput { .. })
        ));
        assert!(matches!(
            grm(f64::NAN, 1.0),
            Err(FinanceError::NonFinite { .. })
        ));
        assert!(matches!(
            ltv(1.0, f64::INFINITY),
            Err(FinanceError::NonFinite { .. })
        ));
        assert!(noi(1.0, 1.5, 0.0, 0.0).is_err());
        assert!(breakeven_occupancy(f64::MAX, f64::MAX, 1.0).is_err());
        assert!(
            (breakeven_occupancy_net(1_000.0, 200.0, 3_000.0, 0.05, 0.2).unwrap()
                - 0.526_315_789_473_684_2)
                .abs()
                < 1e-12
        );
        assert!(breakeven_occupancy_net(1.0, 0.0, 100.0, 1.0, 0.0).is_err());
        assert!(breakeven_occupancy_net(1.0, 0.0, 100.0, -0.1, 0.0).is_err());
        assert!(breakeven_occupancy_net(1.0, 0.0, 0.0, 0.0, 0.0).is_err());
    }
}
