//! Fully diluted share counts across common, preferred, options, warrants, and SAFEs.
//!
//! ```
//! use finance_rs::vc::cap_table::{fully_diluted, Holding, SecurityKind};
//! let shares = fully_diluted(&[Holding { shares: 100.0, kind: SecurityKind::Common }])?;
//! assert_eq!(shares, 100.0);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

use super::{nonnegative, output, FinanceError, Result};
/// Security holdings in fully diluted shares.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Holding {
    /// Shares for the holding result.
    pub shares: f64,
    /// Kind for the holding result.
    pub kind: SecurityKind,
}
#[derive(Debug, Clone, Copy, PartialEq)]
/// Enumerates security kind choices.
pub enum SecurityKind {
    /// The common option.
    Common,
    /// The preferred as converted option.
    PreferredAsConverted,
    /// The options option.
    Options,
    /// The warrants option.
    Warrants,
    /// The safe option.
    Safe,
}
/// Sum common, preferred-as-converted, options, warrants and converting SAFE shares.
///
/// # Errors
/// Returns an error for an empty holding list, negative shares, or overflow.
pub fn fully_diluted(holdings: &[Holding]) -> Result<f64> {
    if holdings.is_empty() {
        return Err(FinanceError::TooShort {
            what: "holdings",
            min: 1,
            got: 0,
        });
    }
    let mut total = 0.0;
    for h in holdings {
        total += nonnegative("shares", h.shares)?;
    }
    output("fully_diluted_shares", total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cap_table_rejects_empty_and_negative_shares() {
        assert!(fully_diluted(&[]).is_err());
        assert!(fully_diluted(&[Holding {
            shares: -1.0,
            kind: SecurityKind::Common,
        }])
        .is_err());
        assert_eq!(
            fully_diluted(&[
                Holding {
                    shares: 1.0,
                    kind: SecurityKind::Common
                },
                Holding {
                    shares: 2.0,
                    kind: SecurityKind::Safe
                },
            ])
            .unwrap(),
            3.0
        );
    }
}
