//! Fund return conventions and LP secondary pricing.
use crate::{
    cashflow::{xirr, xnpv},
    error::{all_finite, output, FinanceError, Result},
    Date,
};

#[derive(Debug, Clone, Copy, PartialEq)]
/// Gross and LP net money-weighted returns for one fund period set.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::FundReturns>() > 0);
/// ```
pub struct FundReturns {
    /// IRR before management fees and carried interest.
    pub gross_irr: f64,
    /// LP IRR after management fees and carried interest.
    pub net_irr: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
/// Time-weighted and money-weighted return comparison.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::TwrVsMwr>() > 0);
/// ```
pub struct TwrVsMwr {
    /// Geometrically linked time-weighted return.
    pub twr: f64,
    /// Dated money-weighted return.
    pub mwr: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
/// Secondary market price and discount to reported NAV.
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fund::SecondaryPrice>() > 0);
/// ```
pub struct SecondaryPrice {
    /// Implied purchase price per unit of NAV.
    pub price: f64,
    /// Fractional discount from NAV, from 0 through 1.
    pub discount_to_nav: f64,
}

/// Convert stated NAV and a market discount assumption into secondary price.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::secondary_price_from_nav), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn secondary_price_from_nav(nav: f64, discount: f64) -> Result<f64> {
    if !nav.is_finite() || !discount.is_finite() {
        return Err(FinanceError::NonFinite {
            what: "nav/discount",
        });
    }
    if nav < 0.0 || !(0.0..=1.0).contains(&discount) {
        return Err(invalid("NAV must be nonnegative and discount in [0,1]"));
    }
    output("secondary_price", nav * (1.0 - discount))
}

/// Calculate pre-fee/pre-carry gross IRR and LP net IRR from signed cash-flow
/// magnitudes. Fees and carry are period costs deducted only for net IRR.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::gross_net_irr::<finance_rs::Date>), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn gross_net_irr<D: Copy + Into<Date>>(
    dates: &[D],
    calls: &[f64],
    distributions: &[f64],
    fees: &[f64],
    carry: &[f64],
    nav: f64,
    nav_date: D,
) -> Result<FundReturns> {
    let n = dates.len();
    if n < 2 {
        return Err(FinanceError::TooShort {
            what: "dates",
            min: 2,
            got: n,
        });
    }
    for (name, xs) in [
        ("calls", calls),
        ("distributions", distributions),
        ("fees", fees),
        ("carry", carry),
    ] {
        if xs.len() != n {
            return Err(FinanceError::LengthMismatch {
                left: "dates",
                left_len: n,
                right: name,
                right_len: xs.len(),
            });
        }
        all_finite(name, xs)?;
        if xs.iter().any(|x| *x < 0.0) {
            return Err(invalid("cash flow magnitudes must be nonnegative"));
        }
    }
    if !nav.is_finite() || nav < 0.0 {
        return Err(invalid("NAV must be finite and nonnegative"));
    }
    let dates: Vec<Date> = dates.iter().map(|d| (*d).into()).collect();
    let end = nav_date.into();
    let ix = dates
        .iter()
        .position(|d| *d == end)
        .ok_or_else(|| invalid("NAV date must be present in dates"))?;
    let mut gross = distributions
        .iter()
        .zip(calls)
        .map(|(d, c)| d - c)
        .collect::<Vec<_>>();
    gross[ix] += nav;
    let mut net = gross.clone();
    for i in 0..n {
        net[i] -= fees[i] + carry[i];
    } // fees/carry flows at their stated dates
    Ok(FundReturns {
        gross_irr: xirr(&dates, &gross)?,
        net_irr: xirr(&dates, &net)?,
    })
}

/// Link periodic NAV returns and compare them with the dated money-weighted
/// return. Contributions/distributions are signed from the fund's view.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::twr_vs_mwr::<finance_rs::Date>), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn twr_vs_mwr<D: Copy + Into<Date>>(
    dates: &[D],
    nav: &[f64],
    contributions: &[f64],
    distributions: &[f64],
) -> Result<TwrVsMwr> {
    let n = dates.len();
    if n < 2 {
        return Err(FinanceError::TooShort {
            what: "dates",
            min: 2,
            got: n,
        });
    }
    for (name, xs) in [
        ("nav", nav),
        ("contributions", contributions),
        ("distributions", distributions),
    ] {
        if xs.len() != n {
            return Err(FinanceError::LengthMismatch {
                left: "dates",
                left_len: n,
                right: name,
                right_len: xs.len(),
            });
        }
        all_finite(name, xs)?;
    }
    if nav.iter().any(|x| *x < 0.0) || contributions.iter().chain(distributions).any(|x| *x < 0.0) {
        return Err(invalid("NAV and flow magnitudes must be nonnegative"));
    }
    let mut twr = 1.0;
    for i in 1..n {
        let denominator = nav[i - 1] + contributions[i];
        if denominator <= 0.0 {
            return Err(FinanceError::Undefined {
                what: "period beginning NAV plus contribution",
            });
        }
        let r = (nav[i] + distributions[i] - contributions[i] - nav[i - 1]) / denominator;
        twr *= 1.0 + r;
        if !twr.is_finite() {
            return Err(FinanceError::Overflow { what: "twr" });
        }
    }
    let mut flows = contributions
        .iter()
        .zip(distributions)
        .map(|(c, d)| d - c)
        .collect::<Vec<_>>();
    *flows.last_mut().ok_or(FinanceError::TooShort {
        what: "flows",
        min: 1,
        got: 0,
    })? += nav[n - 1];
    let ds: Vec<Date> = dates.iter().map(|d| (*d).into()).collect();
    Ok(TwrVsMwr {
        twr: output("twr", twr - 1.0)?,
        mwr: xirr(&ds, &flows)?,
    })
}

/// Buyer price is the present value of projected signed fund cash flows at
/// the target IRR. Discount is measured from stated NAV: `1 - price / NAV`.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fund::secondary_price::<finance_rs::Date>), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn secondary_price<D: Copy + Into<Date>>(
    projected_flows: &[f64],
    target_irr: f64,
    dates: &[D],
    nav: f64,
) -> Result<SecondaryPrice> {
    if projected_flows.len() != dates.len() {
        return Err(FinanceError::LengthMismatch {
            left: "projected_flows",
            left_len: projected_flows.len(),
            right: "dates",
            right_len: dates.len(),
        });
    }
    if projected_flows.len() < 2 {
        return Err(FinanceError::TooShort {
            what: "projected_flows",
            min: 2,
            got: projected_flows.len(),
        });
    }
    all_finite("projected_flows", projected_flows)?;
    if !target_irr.is_finite() || !nav.is_finite() {
        return Err(FinanceError::NonFinite {
            what: "target_irr/nav",
        });
    }
    if target_irr <= -1.0 || nav <= 0.0 {
        return Err(invalid(
            "target IRR must exceed -100% and NAV must be positive",
        ));
    }
    let ds: Vec<Date> = dates.iter().map(|d| (*d).into()).collect();
    if ds.windows(2).any(|w| w[0] >= w[1]) {
        return Err(invalid("dates must be strictly increasing"));
    }
    let price = xnpv(target_irr, &ds, projected_flows)?;
    Ok(SecondaryPrice {
        price,
        discount_to_nav: output("secondary_discount", 1.0 - price / nav)?,
    })
}
fn invalid(reason: &'static str) -> FinanceError {
    FinanceError::InvalidInput {
        what: "fund_returns",
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secondary_npv_textbook() {
        let d = [
            Date::new(2023, 1, 1).unwrap(),
            Date::new(2024, 1, 1).unwrap(),
        ];
        let p = secondary_price(&[0., 110.], 0.1, &d, 100.).unwrap();
        assert!((p.price - 100.).abs() < 1e-8);
        assert!(p.discount_to_nav.abs() < 1e-10);
    }
    #[test]
    fn gross_net_fee_drag_is_ordered() {
        let d = [
            Date::new(2023, 1, 1).unwrap(),
            Date::new(2024, 1, 1).unwrap(),
        ];
        let r =
            gross_net_irr(&d, &[100., 0.], &[0., 130.], &[0., 5.], &[0., 5.], 0., d[1]).unwrap();
        assert!(r.net_irr < r.gross_irr);
    }
    #[test]
    fn twr_links_returns() {
        let d = [
            Date::new(2023, 1, 1).unwrap(),
            Date::new(2024, 1, 1).unwrap(),
        ];
        let r = twr_vs_mwr(&d, &[100., 110.], &[100., 0.], &[0., 0.]).unwrap();
        assert!((r.twr - 0.1).abs() < 1e-10);
        assert!((r.mwr - 0.1).abs() < 1e-8);
    }
    #[test]
    fn secondary_rejects_nonincreasing_dates() {
        let d = [Date::new(2024, 1, 1).unwrap(); 2];
        assert!(secondary_price(&[1., 2.], 0.1, &d, 1.).is_err());
    }

    #[test]
    fn secondary_price_requires_two_projected_flows() {
        let date = [Date::new(2024, 1, 1).unwrap()];
        assert!(matches!(
            secondary_price(&[], 0.1, &[] as &[Date], 1.0),
            Err(FinanceError::TooShort { got: 0, .. })
        ));
        assert!(matches!(
            secondary_price(&[1.0], 0.1, &date, 1.0),
            Err(FinanceError::TooShort { got: 1, .. })
        ));
    }

    #[test]
    fn nav_discount_secondary_price_textbook() {
        assert_eq!(secondary_price_from_nav(100.0, 0.2).unwrap(), 80.0);
    }

    #[test]
    fn return_helpers_reject_misaligned_and_invalid_flows() {
        let d = [
            Date::new(2023, 1, 1).unwrap(),
            Date::new(2024, 1, 1).unwrap(),
        ];
        assert!(secondary_price_from_nav(f64::NAN, 0.2).is_err());
        assert!(secondary_price_from_nav(-1.0, 0.2).is_err());
        assert!(secondary_price_from_nav(1.0, 1.1).is_err());
        assert!(gross_net_irr(&d[..1], &[1.], &[1.], &[0.], &[0.], 0., d[0]).is_err());
        assert!(gross_net_irr(&d, &[1.], &[1., 2.], &[0., 0.], &[0., 0.], 0., d[1]).is_err());
        assert!(gross_net_irr(&d, &[1., 0.], &[0., 2.], &[0.], &[0., 0.], 0., d[1]).is_err());
        assert!(gross_net_irr(&d, &[1., 0.], &[0., 2.], &[0., 0.], &[0.], 0., d[1]).is_err());
        assert!(gross_net_irr(
            &d,
            &[f64::NAN, 0.],
            &[0., 2.],
            &[0., 0.],
            &[0., 0.],
            0.,
            d[1]
        )
        .is_err());
        assert!(gross_net_irr(&d, &[-1., 0.], &[0., 2.], &[0., 0.], &[0., 0.], 0., d[1]).is_err());
        assert!(gross_net_irr(&d, &[1., 0.], &[0., 2.], &[-1., 0.], &[0., 0.], 0., d[1]).is_err());
        assert!(gross_net_irr(&d, &[1., 0.], &[0., 2.], &[0., 0.], &[0., 0.], -1., d[1]).is_err());
        assert!(gross_net_irr(
            &d,
            &[1., 0.],
            &[0., 2.],
            &[0., 0.],
            &[0., 0.],
            0.,
            Date::new(2025, 1, 1).unwrap()
        )
        .is_err());
        assert!(twr_vs_mwr(&d[..1], &[1.], &[0.], &[0.]).is_err());
        assert!(twr_vs_mwr(&d, &[1.], &[0., 0.], &[0., 0.]).is_err());
        assert!(twr_vs_mwr(&d, &[1., 2.], &[0.], &[0., 0.]).is_err());
        assert!(twr_vs_mwr(&d, &[1., 2.], &[0., 0.], &[0.]).is_err());
        assert!(twr_vs_mwr(&d, &[1., f64::NAN], &[0., 0.], &[0., 0.]).is_err());
        assert!(twr_vs_mwr(&d, &[-1., 0.], &[0., 0.], &[0., 0.]).is_err());
        assert!(twr_vs_mwr(&d, &[1., 2.], &[-1., 0.], &[0., 0.]).is_err());
        assert!(twr_vs_mwr(&d, &[0., 0.], &[0., 0.], &[0., 0.]).is_err());
        assert!(secondary_price(&[1.], 0.1, &d, 10.).is_err());
        assert!(secondary_price(&[1.], 0.1, &d[..0], 10.).is_err());
        assert!(secondary_price(&[1., 2.], f64::NAN, &d, 10.).is_err());
        assert!(secondary_price(&[1., 2.], -1.0, &d, 10.).is_err());
        assert!(secondary_price(&[1., 2.], 0.1, &d, 0.).is_err());
        assert!(secondary_price(&[1., f64::INFINITY], 0.1, &d, 10.).is_err());
    }
}
