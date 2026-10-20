//! Yield to maturity from a price: Newton from the coupon rate, falling back
//! to Brent on an expanding bracket. Price is strictly decreasing in yield
//! (every cash flow is non-negative and the face is positive), so a positive
//! price has exactly one yield.

use super::{check_fraction, check_price, Bond};
use crate::error::{output, FinanceError, Result};
use crate::solve::kernel::{brent, newton};

const WHAT: &str = "bond yield";

impl Bond {
    /// Yield to maturity (nominal, compounded `frequency` times a year) that
    /// reproduces a price quoted on a coupon date.
    ///
    /// Solves `Σ_{k=1..n} CF_k / (1 + y/f)^k = price` for `y`.
    ///
    /// ```rust
    /// use finance_rs::fi::Bond;
    /// let b = Bond::new(1_000.0, 0.08, 20, 2)?;
    /// let y = b.yield_from_price(875.37792)?;
    /// assert!((y - 0.10).abs() < 1e-8);
    /// // At par the yield is the coupon rate.
    /// assert!((b.yield_from_price(1_000.0)? - 0.08).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Bond::new`], plus [`FinanceError::InvalidInput`] for a
    /// non-positive or non-finite price and [`FinanceError::NoSolution`] when
    /// the yield lies beyond the searchable range (prices so extreme that
    /// `y/f` would be within `1e-12` of −1 or above `1e12`).
    pub fn yield_from_price(&self, price: f64) -> Result<f64> {
        self.solve_yield(price, 0.0)
    }

    /// Yield to maturity from a clean (quoted) price a fraction
    /// `accrued_fraction ∈ [0, 1)` of the way through the current period.
    ///
    /// Solves `P(y) · (1 + y/f)^a = clean + C·a` for `y` (see
    /// [`Bond::price_with_accrued`]).
    ///
    /// ```rust
    /// use finance_rs::fi::Bond;
    /// let b = Bond::new(100.0, 0.05, 9, 2)?;
    /// let q = b.price_with_accrued(0.047, 0.3)?;
    /// assert!((b.yield_from_clean_price(q.clean, 0.3)? - 0.047).abs() < 1e-10);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Bond::yield_from_price`], plus [`FinanceError::InvalidInput`] for
    /// a fraction outside `[0, 1)` or a clean price whose dirty price
    /// `clean + C·a` is not positive.
    pub fn yield_from_clean_price(&self, clean_price: f64, accrued_fraction: f64) -> Result<f64> {
        check_fraction(accrued_fraction)?;
        let dirty = clean_price + self.accrued_interest(accrued_fraction)?;
        self.solve_yield(dirty, accrued_fraction)
    }

    fn solve_yield(&self, dirty: f64, a: f64) -> Result<f64> {
        self.check()?;
        check_price(dirty)?;
        let gap = |i: f64| self.moments(i).0 * (1.0 + i).powf(a) - dirty;
        let f = f64::from(self.frequency);
        let tol = 1e-14;

        let f_df = |i: f64| {
            let (p, s1, _) = self.moments(i);
            let g = (1.0 + i).powf(a);
            (p * g - dirty, g * (a * p - s1) / (1.0 + i))
        };
        if let Some(i) = newton(f_df, self.coupon_rate / f, tol, 50)
            .filter(|i| acceptable_newton_yield(*i, gap(*i), dirty))
        {
            return output(WHAT, i * f);
        }

        // Brent on a bracket: price falls from +inf (i -> -1) to 0 (i -> inf).
        let mut lo = -0.5;
        let mut tries = 0;
        while gap(lo) < 0.0 {
            lo = -1.0 + (lo + 1.0) * 1e-2;
            tries += 1;
            if tries > 6 {
                return Err(FinanceError::NoSolution { what: WHAT });
            }
        }
        let mut hi = 1.0;
        while gap(hi) > 0.0 {
            hi *= 10.0;
            if hi > 1e12 {
                return Err(FinanceError::NoSolution { what: WHAT });
            }
        }
        let i = brent(WHAT, gap, lo, hi, tol)?;
        output(WHAT, i * f)
    }
}

fn acceptable_newton_yield(rate: f64, residual: f64, dirty: f64) -> bool {
    rate > -1.0 && residual.is_finite() && residual.abs() <= 1e-10 * dirty
}

#[cfg(test)]
mod tests {
    use super::acceptable_newton_yield;
    use crate::fi::Bond;

    #[test]
    fn newton_yield_is_accepted_only_for_a_finite_root() {
        assert!(acceptable_newton_yield(0.05, 1e-12, 100.0));
        assert!(!acceptable_newton_yield(-1.0, 0.0, 100.0));
        assert!(!acceptable_newton_yield(0.05, f64::NAN, 100.0));
        assert!(!acceptable_newton_yield(0.05, 1.0, 100.0));
    }

    #[test]
    fn yield_bracket_expansion_stops_for_an_unrepresentable_price() -> crate::Result<()> {
        let bond = Bond::new(1.0, 0.0, 1, 1)?;
        assert!(matches!(
            bond.yield_from_price(f64::MAX),
            Err(crate::FinanceError::NoSolution { what: "bond yield" })
        ));
        Ok(())
    }
}
