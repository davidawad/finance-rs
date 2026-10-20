//! The one error type every fallible function returns.

use std::fmt;

/// Result alias used across the crate.
pub type Result<T> = std::result::Result<T, FinanceError>;

/// Why a computation could not produce a number.
///
/// Every public function validates its input and returns one of these rather
/// than panicking or silently returning `NaN`.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::FinanceError>() > 0);
/// ```
pub enum FinanceError {
    /// A series that needs at least `min` observations had fewer.
    TooShort {
        /// Argument name.
        what: &'static str,
        /// Minimum length required.
        min: usize,
        /// Length supplied.
        got: usize,
    },
    /// Two series that must be aligned have different lengths.
    LengthMismatch {
        /// Name of the first series.
        left: &'static str,
        /// Name of the second series.
        right: &'static str,
        /// Length of the first series.
        left_len: usize,
        /// Length of the second series.
        right_len: usize,
    },
    /// An input was `NaN` or infinite.
    NonFinite {
        /// Argument name.
        what: &'static str,
    },
    /// An input was outside its domain (negative period count, rate <= -100%, ...).
    InvalidInput {
        /// Argument name.
        what: &'static str,
        /// Human-readable constraint that was violated.
        reason: &'static str,
    },
    /// The result is undefined: a zero denominator (zero volatility, zero
    /// debt service, ...).
    Undefined {
        /// What would have been divided by zero.
        what: &'static str,
    },
    /// Cash flows never change sign, so no rate of return exists.
    NoSignChange,
    /// No real root exists in the searched domain.
    NoSolution {
        /// Which solver failed.
        what: &'static str,
    },
    /// An iterative solver ran out of iterations before converging.
    NoConvergence {
        /// Which solver failed.
        what: &'static str,
        /// Iterations spent.
        iterations: usize,
    },
    /// The computed value overflowed to infinity or `NaN`.
    Overflow {
        /// What overflowed.
        what: &'static str,
    },
}

impl fmt::Display for FinanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooShort { what, min, got } => {
                write!(f, "{what}: need at least {min} values, got {got}")
            }
            Self::LengthMismatch {
                left,
                right,
                left_len,
                right_len,
            } => write!(
                f,
                "{left} has {left_len} values but {right} has {right_len}"
            ),
            Self::NonFinite { what } => write!(f, "{what}: value is NaN or infinite"),
            Self::InvalidInput { what, reason } => write!(f, "{what}: {reason}"),
            Self::Undefined { what } => write!(f, "undefined: {what} is zero"),
            Self::NoSignChange => f.write_str("cash flows never change sign"),
            Self::NoSolution { what } => write!(f, "{what}: no real solution"),
            Self::NoConvergence { what, iterations } => {
                write!(f, "{what}: no convergence after {iterations} iterations")
            }
            Self::Overflow { what } => write!(f, "{what}: result overflowed"),
        }
    }
}

// `core::error::Error` (stable since 1.81) keeps the error no_std-ready;
// `std::error::Error` is the same trait.
impl core::error::Error for FinanceError {}

/// Fail with [`FinanceError::NonFinite`] unless `x` is finite.
pub(crate) fn finite(what: &'static str, x: f64) -> Result<f64> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(FinanceError::NonFinite { what })
    }
}

/// Fail unless every value is finite.
pub(crate) fn all_finite(what: &'static str, xs: &[f64]) -> Result<()> {
    if xs.iter().all(|x| x.is_finite()) {
        Ok(())
    } else {
        Err(FinanceError::NonFinite { what })
    }
}

/// Fail unless `xs` has at least `min` values.
pub(crate) fn min_len(what: &'static str, xs: &[f64], min: usize) -> Result<()> {
    if xs.len() >= min {
        Ok(())
    } else {
        Err(FinanceError::TooShort {
            what,
            min,
            got: xs.len(),
        })
    }
}

/// Fail unless both series have the same length.
pub(crate) fn same_len(left: &'static str, a: usize, right: &'static str, b: usize) -> Result<()> {
    if a == b {
        Ok(())
    } else {
        Err(FinanceError::LengthMismatch {
            left,
            right,
            left_len: a,
            right_len: b,
        })
    }
}

/// Map a non-finite computed value to [`FinanceError::Overflow`].
pub(crate) fn output(what: &'static str, x: f64) -> Result<f64> {
    if x.is_finite() {
        Ok(x)
    } else {
        Err(FinanceError::Overflow { what })
    }
}

/// `num / den`, or [`FinanceError::Undefined`] when `den` is zero.
pub(crate) fn div(what: &'static str, num: f64, den: f64) -> Result<f64> {
    if den == 0.0 {
        return Err(FinanceError::Undefined { what });
    }
    output(what, num / den)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_variant_has_stable_display_text() {
        let cases = [
            (
                FinanceError::TooShort {
                    what: "returns",
                    min: 2,
                    got: 1,
                },
                "returns: need at least 2 values, got 1",
            ),
            (
                FinanceError::LengthMismatch {
                    left: "returns",
                    right: "benchmark",
                    left_len: 2,
                    right_len: 1,
                },
                "returns has 2 values but benchmark has 1",
            ),
            (
                FinanceError::NonFinite { what: "rate" },
                "rate: value is NaN or infinite",
            ),
            (
                FinanceError::InvalidInput {
                    what: "rate",
                    reason: "must be positive",
                },
                "rate: must be positive",
            ),
            (
                FinanceError::Undefined { what: "volatility" },
                "undefined: volatility is zero",
            ),
            (FinanceError::NoSignChange, "cash flows never change sign"),
            (
                FinanceError::NoSolution { what: "IRR" },
                "IRR: no real solution",
            ),
            (
                FinanceError::NoConvergence {
                    what: "Newton",
                    iterations: 12,
                },
                "Newton: no convergence after 12 iterations",
            ),
            (
                FinanceError::Overflow { what: "NPV" },
                "NPV: result overflowed",
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(error.to_string(), expected);
        }
    }

    #[test]
    fn shared_validation_helpers_cover_success_and_failure_paths() {
        assert_eq!(finite("x", 1.0), Ok(1.0));
        assert!(matches!(
            finite("x", f64::NAN),
            Err(FinanceError::NonFinite { .. })
        ));
        assert!(all_finite("xs", &[1.0, 2.0]).is_ok());
        assert!(matches!(
            all_finite("xs", &[1.0, f64::INFINITY]),
            Err(FinanceError::NonFinite { .. })
        ));
        assert!(min_len("xs", &[1.0, 2.0], 2).is_ok());
        assert!(matches!(
            min_len("xs", &[1.0], 2),
            Err(FinanceError::TooShort { .. })
        ));
        assert!(same_len("a", 2, "b", 2).is_ok());
        assert!(matches!(
            same_len("a", 2, "b", 1),
            Err(FinanceError::LengthMismatch { .. })
        ));
        assert_eq!(output("x", 1.0), Ok(1.0));
        assert!(matches!(
            output("x", f64::INFINITY),
            Err(FinanceError::Overflow { .. })
        ));
        assert_eq!(div("x", 6.0, 2.0), Ok(3.0));
        assert!(matches!(
            div("x", 1.0, 0.0),
            Err(FinanceError::Undefined { .. })
        ));
        assert!(matches!(
            div("x", f64::MAX, 0.5),
            Err(FinanceError::Overflow { .. })
        ));
    }
}
