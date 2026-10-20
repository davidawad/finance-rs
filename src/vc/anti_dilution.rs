//! Full-ratchet and weighted-average anti-dilution conversion-price adjustments.
//!
//! ```
//! use finance_rs::vc::anti_dilution::full_ratchet;
//! assert_eq!(full_ratchet(2.0, 1.5)?, 1.5);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

use super::{nonnegative, output, positive, Result};
/// Full-ratchet conversion price after a down round.
///
/// # Errors
/// Returns an error for non-finite or non-positive conversion prices.
pub fn full_ratchet(cp: f64, new_price: f64) -> Result<f64> {
    positive("cp", cp)?;
    positive("new_price", new_price)?;
    Ok(cp.min(new_price))
}
/// Weighted-average conversion price using broad-based capitalization.
///
/// # Errors
/// Returns an error for invalid inputs or a zero `A + C` denominator.
pub fn broad_based(cp1: f64, a: f64, b: f64, c: f64) -> Result<f64> {
    weighted(cp1, a, b, c)
}
/// Weighted-average conversion price using narrow-based capitalization (caller supplies narrow A).
///
/// # Errors
/// Returns an error for invalid inputs or a zero `A + C` denominator.
pub fn narrow_based(cp1: f64, a: f64, b: f64, c: f64) -> Result<f64> {
    weighted(cp1, a, b, c)
}
fn weighted(cp1: f64, a: f64, b: f64, c: f64) -> Result<f64> {
    positive("cp1", cp1)?;
    nonnegative("A", a)?;
    nonnegative("B", b)?;
    nonnegative("C", c)?;
    positive("A + C", a + c)?;
    output("cp2", cp1 * (a + b) / (a + c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anti_dilution_validates_each_term_and_caps_ratchet() {
        assert_eq!(full_ratchet(2.0, 3.0).unwrap(), 2.0);
        assert_eq!(broad_based(2.0, 10.0, 5.0, 10.0).unwrap(), 1.5);
        assert_eq!(narrow_based(2.0, 10.0, 5.0, 10.0).unwrap(), 1.5);
        assert!(full_ratchet(0.0, 1.0).is_err());
        assert!(full_ratchet(1.0, f64::NAN).is_err());
        assert!(broad_based(0.0, 1.0, 1.0, 1.0).is_err());
        assert!(broad_based(1.0, -1.0, 1.0, 1.0).is_err());
        assert!(broad_based(1.0, 1.0, f64::INFINITY, 1.0).is_err());
        assert!(broad_based(1.0, 1.0, 1.0, -1.0).is_err());
        assert!(broad_based(1.0, 0.0, 1.0, 0.0).is_err());
    }
}
