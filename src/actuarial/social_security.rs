/// Social Security retirement-benefit formulas.
use crate::{FinanceError, Result};
/// Bend-point PIA formula, using the eligibility-year SSA factors.
/// Calculate a primary insurance amount using SSA bend points for an eligibility year.
///
/// ```
/// let monthly = finance_rs::actuarial::retirement::social_security::pia(5_000.0, 2026)?;
/// assert!(monthly > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn pia(aime: f64, year: u16) -> Result<f64> {
    let (first, second) = bend_points(year).ok_or(FinanceError::InvalidInput {
        what: "year",
        reason: "SSA bend points available for 1979 through 2026",
    })?;
    super::social_security_pia(aime, year, (first, second))
}
/// PIA formula with explicit bend points for a jurisdiction or future year.
/// Calculate a primary insurance amount using explicit monthly bend points.
///
/// ```
/// let monthly = finance_rs::actuarial::retirement::social_security::pia_with_bend_points(5_000.0, 1_226.0, 7_391.0)?;
/// assert!(monthly > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn pia_with_bend_points(aime: f64, first: f64, second: f64) -> Result<f64> {
    super::social_security_pia(aime, 2026, (first, second))
}
/// Early or delayed retirement benefit factor.
/// Re-export the early and delayed claiming factor at this module path.
///
/// ```
/// let factor = finance_rs::actuarial::retirement::social_security::claiming_adjustment(0)?;
/// assert_eq!(factor, 1.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub use super::claiming_adjustment;
/// Eligibility-year bend points, dollars per month (SSA).
/// Return SSA primary-insurance-amount bend points for an eligibility year.
///
/// ```
/// let points = finance_rs::actuarial::retirement::social_security::bend_points(2026);
/// assert!(points.is_some());
/// ```
#[must_use]
pub fn bend_points(year: u16) -> Option<(f64, f64)> {
    const BP: [(u16, f64, f64); 48] = [
        (1979, 180.0, 1085.0),
        (1980, 194.0, 1171.0),
        (1981, 211.0, 1274.0),
        (1982, 230.0, 1388.0),
        (1983, 254.0, 1528.0),
        (1984, 267.0, 1612.0),
        (1985, 280.0, 1691.0),
        (1986, 297.0, 1790.0),
        (1987, 310.0, 1866.0),
        (1988, 319.0, 1922.0),
        (1989, 339.0, 2044.0),
        (1990, 356.0, 2145.0),
        (1991, 370.0, 2230.0),
        (1992, 387.0, 2333.0),
        (1993, 401.0, 2420.0),
        (1994, 422.0, 2545.0),
        (1995, 426.0, 2567.0),
        (1996, 437.0, 2635.0),
        (1997, 455.0, 2741.0),
        (1998, 477.0, 2875.0),
        (1999, 505.0, 3043.0),
        (2000, 531.0, 3202.0),
        (2001, 561.0, 3381.0),
        (2002, 592.0, 3567.0),
        (2003, 606.0, 3653.0),
        (2004, 612.0, 3689.0),
        (2005, 627.0, 3779.0),
        (2006, 656.0, 3955.0),
        (2007, 680.0, 4100.0),
        (2008, 711.0, 4288.0),
        (2009, 744.0, 4483.0),
        (2010, 761.0, 4586.0),
        (2011, 749.0, 4517.0),
        (2012, 767.0, 4624.0),
        (2013, 791.0, 4768.0),
        (2014, 816.0, 4917.0),
        (2015, 826.0, 4980.0),
        (2016, 856.0, 5157.0),
        (2017, 885.0, 5336.0),
        (2018, 895.0, 5397.0),
        (2019, 926.0, 5583.0),
        (2020, 960.0, 5785.0),
        (2021, 996.0, 6002.0),
        (2022, 1024.0, 6172.0),
        (2023, 1115.0, 6721.0),
        (2024, 1174.0, 7078.0),
        (2025, 1226.0, 7391.0),
        (2026, 1286.0, 7749.0),
    ];
    BP.iter()
        .find(|(y, _, _)| *y == year)
        .map(|(_, a, b)| (*a, *b))
}
