//! Brinson performance attribution and multi-period linking.
use crate::error::{all_finite, same_len, FinanceError, Result};

/// Single category's attribution effects; effects sum to active return.
///
/// ```rust
/// use finance_rs::attribution::BrinsonEffect;
/// let effect = BrinsonEffect { allocation: 0.01, selection: 0.02, interaction: 0.0 };
/// assert!((effect.allocation + effect.selection + effect.interaction - 0.03).abs() < 1e-12);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrinsonEffect {
    /// Allocation for the brinson effect result.
    pub allocation: f64,
    /// Selection for the brinson effect result.
    pub selection: f64,
    /// Interaction for the brinson effect result.
    pub interaction: f64,
}

fn validate(wp: &[f64], wb: &[f64], rp: &[f64], rb: &[f64]) -> Result<()> {
    let n = wp.len();
    for (name, x) in [("wb", wb), ("rp", rp), ("rb", rb)] {
        same_len("wp", n, name, x.len())?;
    }
    if n == 0 {
        return Err(FinanceError::TooShort {
            what: "categories",
            min: 1,
            got: 0,
        });
    }
    for (name, x) in [("wp", wp), ("wb", wb), ("rp", rp), ("rb", rb)] {
        all_finite(name, x)?;
    }
    Ok(())
}

/// Brinson-Hood-Beebower attribution by category.
///
/// ```rust
/// use finance_rs::attribution::brinson_hood_beebower;
/// let effects = brinson_hood_beebower(&[0.6], &[0.5], &[0.10], &[0.08])?;
/// assert!((effects[0].allocation - 0.008).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn brinson_hood_beebower(
    wp: &[f64],
    wb: &[f64],
    rp: &[f64],
    rb: &[f64],
) -> Result<Vec<BrinsonEffect>> {
    validate(wp, wb, rp, rb)?;
    Ok((0..wp.len())
        .map(|i| BrinsonEffect {
            allocation: (wp[i] - wb[i]) * rb[i],
            selection: wb[i] * (rp[i] - rb[i]),
            interaction: (wp[i] - wb[i]) * (rp[i] - rb[i]),
        })
        .collect())
}

/// Brinson-Fachler attribution by category, using benchmark total return `Rb`.
///
/// ```rust
/// use finance_rs::attribution::brinson_fachler;
/// let effects = brinson_fachler(&[0.6], &[0.5], &[0.10], &[0.08])?;
/// assert!((effects[0].allocation - 0.004).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn brinson_fachler(
    wp: &[f64],
    wb: &[f64],
    rp: &[f64],
    rb: &[f64],
) -> Result<Vec<BrinsonEffect>> {
    validate(wp, wb, rp, rb)?;
    let total: f64 = wb.iter().zip(rb).map(|(w, r)| w * r).sum();
    Ok((0..wp.len())
        .map(|i| BrinsonEffect {
            allocation: (wp[i] - wb[i]) * (rb[i] - total),
            selection: wb[i] * (rp[i] - rb[i]),
            interaction: (wp[i] - wb[i]) * (rp[i] - rb[i]),
        })
        .collect())
}

/// Period-level effects for linking; each row carries portfolio and benchmark return.
///
/// ```rust
/// use finance_rs::attribution::LinkedPeriod;
/// let period = LinkedPeriod { portfolio_return: 0.10, benchmark_return: 0.08, effects: [0.01, 0.01, 0.0] };
/// assert!((period.effects.iter().sum::<f64>() - (period.portfolio_return - period.benchmark_return)).abs() < 1e-12);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkedPeriod {
    /// Portfolio return for the linked period result.
    pub portfolio_return: f64,
    /// Benchmark return for the linked period result.
    pub benchmark_return: f64,
    /// Effects for the linked period result.
    pub effects: [f64; 3],
}

/// Public submodule for link.
///
/// ```
/// use finance_rs::attribution::{link, LinkedPeriod};
/// let linked = link::carino(&[LinkedPeriod { portfolio_return: 0.10, benchmark_return: 0.08, effects: [0.01, 0.01, 0.0] }])?;
/// assert!((linked.iter().sum::<f64>() - 0.02).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
pub mod link {
    use super::{all_finite, FinanceError, LinkedPeriod, Result};
    fn validate(p: &[LinkedPeriod]) -> Result<()> {
        if p.is_empty() {
            return Err(FinanceError::TooShort {
                what: "periods",
                min: 1,
                got: 0,
            });
        }
        for x in p {
            all_finite(
                "period returns",
                &[
                    x.portfolio_return,
                    x.benchmark_return,
                    x.effects[0],
                    x.effects[1],
                    x.effects[2],
                ],
            )?;
        }
        Ok(())
    }
    fn link(p: &[LinkedPeriod], method: u8) -> Result<[f64; 3]> {
        validate(p)?;
        let total_p = p.iter().fold(1.0, |a, x| a * (1.0 + x.portfolio_return)) - 1.0;
        let total_b = p.iter().fold(1.0, |a, x| a * (1.0 + x.benchmark_return)) - 1.0;
        let mut out = [0.0; 3];
        for x in p {
            let gap = x.portfolio_return - x.benchmark_return;
            let factor = if method == 0 && (x.portfolio_return - x.benchmark_return).abs() > 1e-14 {
                ((1.0 + x.portfolio_return).ln() - (1.0 + x.benchmark_return).ln()) / gap
            } else if method == 1 {
                1.0 / (1.0 + x.benchmark_return)
            } else {
                1.0
            };
            for (o, e) in out.iter_mut().zip(x.effects) {
                *o += e * factor;
            }
        }
        let sum = out.iter().sum::<f64>();
        let active = total_p - total_b;
        if sum.abs() > 1e-14 {
            for v in &mut out {
                *v *= active / sum;
            }
        }
        crate::error::output("linked attribution", out[0])?;
        Ok(out)
    }
    /// Carino log-return linking, normalized to compounded active return.
    ///
    /// ```rust
    /// use finance_rs::attribution::{link, LinkedPeriod};
    /// let p = [LinkedPeriod { portfolio_return: 0.10, benchmark_return: 0.0, effects: [0.04, 0.03, 0.03] }];
    /// let linked = link::carino(&p)?;
    /// assert!((linked.iter().sum::<f64>() - 0.10).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn carino(periods: &[LinkedPeriod]) -> Result<[f64; 3]> {
        link(periods, 0)
    }
    /// Menchero linking with benchmark-relative scaling, normalized to active return.
    ///
    /// ```rust
    /// use finance_rs::attribution::{link, LinkedPeriod};
    /// let p = [LinkedPeriod { portfolio_return: 0.10, benchmark_return: 0.0, effects: [0.04, 0.03, 0.03] }];
    /// let linked = link::menchero(&p)?;
    /// assert!((linked.iter().sum::<f64>() - 0.10).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn menchero(periods: &[LinkedPeriod]) -> Result<[f64; 3]> {
        link(periods, 1)
    }
    /// GRAP arithmetic linking, normalized to compounded active return.
    ///
    /// ```rust
    /// use finance_rs::attribution::{link, LinkedPeriod};
    /// let p = [LinkedPeriod { portfolio_return: 0.10, benchmark_return: 0.0, effects: [0.04, 0.03, 0.03] }];
    /// let linked = link::grap(&p)?;
    /// assert!((linked.iter().sum::<f64>() - 0.10).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn grap(periods: &[LinkedPeriod]) -> Result<[f64; 3]> {
        link(periods, 2)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn linking_rejects_empty_periods() {
            assert!(carino(&[]).is_err());
            assert!(menchero(&[]).is_err());
            assert!(grap(&[]).is_err());
        }

        #[test]
        fn linking_handles_equal_period_returns_and_zero_effects() {
            let equal = [LinkedPeriod {
                portfolio_return: 0.1,
                benchmark_return: 0.1,
                effects: [0.0; 3],
            }];
            assert_eq!(carino(&equal).unwrap(), [0.0; 3]);
            assert_eq!(menchero(&equal).unwrap(), [0.0; 3]);
            assert_eq!(grap(&equal).unwrap(), [0.0; 3]);

            let differing = [LinkedPeriod {
                portfolio_return: 0.2,
                benchmark_return: 0.1,
                effects: [0.1, 0.0, 0.0],
            }];
            let linked = carino(&differing).unwrap();
            assert!((linked.iter().sum::<f64>() - 0.1).abs() < 1e-12);
            assert!(menchero(&differing).is_ok());
            assert!(grap(&differing).is_ok());
            let invalid = [LinkedPeriod {
                portfolio_return: f64::NAN,
                ..equal[0]
            }];
            assert!(carino(&invalid).is_err());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brinson_methods_match_category_effects_and_reject_invalid_rows() {
        let wp = [0.6, 0.4];
        let wb = [0.5, 0.5];
        let rp = [0.12, 0.04];
        let rb = [0.10, 0.02];
        let hood_beebower = brinson_hood_beebower(&wp, &wb, &rp, &rb).unwrap();
        let fachler = brinson_fachler(&wp, &wb, &rp, &rb).unwrap();
        assert_eq!(hood_beebower.len(), 2);
        assert_eq!(fachler.len(), 2);
        for (effect, active) in hood_beebower.iter().zip([0.022, 0.006]) {
            assert!(
                (effect.allocation + effect.selection + effect.interaction - active).abs() < 1e-12
            );
        }
        for (effect, active) in fachler.iter().zip([0.016, 0.012]) {
            assert!(
                (effect.allocation + effect.selection + effect.interaction - active).abs() < 1e-12
            );
        }
        assert!(brinson_hood_beebower(&[], &[], &[], &[]).is_err());
        assert!(brinson_hood_beebower(&[0.5], &[], &[0.1], &[0.1]).is_err());
        assert!(brinson_fachler(&[f64::NAN], &[0.5], &[0.1], &[0.1]).is_err());
    }
}
