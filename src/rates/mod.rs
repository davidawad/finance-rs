//! Interest rates with their quote basis: [`Frequency`], [`Compounding`],
//! [`Rate`] (QuantLib `InterestRate` semantics) and the scalar conversions
//! between quote bases (Excel `EFFECT`/`NOMINAL`, ffn `annualize`/
//! `deannualize`, Fisher).
//!
//! Units: rates are decimals (`0.05`), never percent; `t` is in years.
//!
//! ```
//! use finance_rs::daycount::DayCount;
//! use finance_rs::rates::{Compounding, Frequency, Rate};
//! let r = Rate::new(0.05, Compounding::Compounded(Frequency::Semiannual), DayCount::Act365F);
//! assert!((r.compound_factor(2.0)? - 1.025f64.powi(4)).abs() < 1e-15);
//! let cont = r.equivalent(Compounding::Continuous, 1.0)?;
//! assert!((cont.value - 2.0 * 1.025f64.ln()).abs() < 1e-15);
//! # Ok::<(), finance_rs::FinanceError>(())
//! ```

// "QuantLib", "VaR" are product names and abbreviations, not code.
#![allow(clippy::doc_markdown)]

mod rate;

pub use rate::Rate;

use crate::error::{finite, output, FinanceError, Result};

fn quote_denominator(denominator: f64) -> Result<(f64, f64)> {
    finite("fraction_denominator", denominator)?;
    if denominator < 1.0 || denominator.fract() != 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "fraction_denominator",
            reason: "must be a positive integer",
        });
    }
    let decimal_scale = 10.0_f64.powf(denominator.log10().ceil());
    Ok((denominator, decimal_scale))
}

/// Convert a fractional financial quote to a decimal quote (Excel `DOLLARDE`).
/// For example, `1.02` with denominator `16` represents `1 + 2/16`.
/// ```rust
/// assert!(finance_rs::rates::effective(0.12, 12.0).unwrap() > 0.12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn dollarde(value: f64, fraction_denominator: f64) -> Result<f64> {
    finite("value", value)?;
    let (denominator, scale) = quote_denominator(fraction_denominator)?;
    let sign = value.signum();
    let abs = value.abs();
    let whole = abs.trunc();
    let fractional_digits = (abs.fract() * scale).round();
    output("dollarde", sign * (whole + fractional_digits / denominator))
}

/// Convert a decimal quote to a fractional financial quote (Excel `DOLLARFR`).
/// ```rust
/// assert!(finance_rs::rates::effective(0.12, 12.0).unwrap() > 0.12);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn dollarfr(value: f64, fraction_denominator: f64) -> Result<f64> {
    finite("value", value)?;
    let (denominator, scale) = quote_denominator(fraction_denominator)?;
    let sign = value.signum();
    let abs = value.abs();
    let whole = abs.trunc();
    let fractional_digits = (abs.fract() * denominator).round();
    output("dollarfr", sign * (whole + fractional_digits / scale))
}

/// How many times a year something happens (QuantLib `Frequency`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
/// ```rust
/// assert!(finance_rs::rates::effective(0.12, 12.0).unwrap() > 0.12);
/// ```
pub enum Frequency {
    /// A single payment at maturity (no periods per year).
    Once,
    /// 1 per year.
    Annual,
    /// 2 per year.
    Semiannual,
    /// 4 per year.
    Quarterly,
    /// 12 per year.
    Monthly,
    /// 52 per year.
    Weekly,
    /// 365 per year.
    Daily,
    /// Any other whole number per year (`PerYear(0)` behaves like `Once`).
    PerYear(u16),
}

impl Frequency {
    /// Periods per year, or `None` for [`Frequency::Once`].
    ///
    /// ```
    /// use finance_rs::rates::Frequency;
    /// assert_eq!(Frequency::Quarterly.per_year(), Some(4));
    /// assert_eq!(Frequency::Once.per_year(), None);
    /// ```
    #[must_use]
    pub fn per_year(self) -> Option<u16> {
        match self {
            Self::Once | Self::PerYear(0) => None,
            Self::Annual => Some(1),
            Self::Semiannual => Some(2),
            Self::Quarterly => Some(4),
            Self::Monthly => Some(12),
            Self::Weekly => Some(52),
            Self::Daily => Some(365),
            Self::PerYear(n) => Some(n),
        }
    }

    fn periods(self) -> Result<f64> {
        self.per_year()
            .map(f64::from)
            .ok_or(FinanceError::InvalidInput {
                what: "frequency",
                reason: "compounding needs at least one period per year",
            })
    }
}

/// How a rate accrues over time `t` (QuantLib `Compounding`), with `m`
/// periods per year:
///
/// | basis | compound factor |
/// |---|---|
/// | `Simple` | `1 + r·t` |
/// | `Compounded(f)` | `(1 + r/m)^(m·t)` |
/// | `Continuous` | `e^(r·t)` |
/// | `SimpleThenCompounded(f)` | simple for `t ≤ 1/m`, compounded after |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
/// ```rust
/// assert!(finance_rs::rates::effective(0.12, 12.0).unwrap() > 0.12);
/// ```
pub enum Compounding {
    /// Simple interest.
    Simple,
    /// Compounded `m` times a year.
    Compounded(Frequency),
    /// Continuously compounded.
    Continuous,
    /// Simple up to one period, compounded beyond (money-market quotes).
    SimpleThenCompounded(Frequency),
}

impl Compounding {
    /// The basis actually applied at `t`: `SimpleThenCompounded` resolves to
    /// simple or compounded, and a frequency to its periods per year.
    fn at(self, t: f64) -> Result<Basis> {
        Ok(match self {
            Self::Simple => Basis::Simple,
            Self::Continuous => Basis::Continuous,
            Self::Compounded(f) => Basis::Compounded(f.periods()?),
            Self::SimpleThenCompounded(f) => {
                let m = f.periods()?;
                if t <= 1.0 / m {
                    Basis::Simple
                } else {
                    Basis::Compounded(m)
                }
            }
        })
    }

    /// `compound factor − 1` at `rate` over `t ≥ 0` years, computed with
    /// `ln_1p`/`exp_m1` so small rates and times keep their precision.
    ///
    /// # Errors
    /// As [`Compounding::compound_factor`].
    /// ```rust
    /// assert!(finance_rs::rates::effective(0.12, 12.0).unwrap() > 0.12);
    /// ```
    pub fn growth(self, rate: f64, t: f64) -> Result<f64> {
        finite("rate", rate)?;
        finite("t", t)?;
        if t < 0.0 {
            return Err(FinanceError::InvalidInput {
                what: "t",
                reason: "must be non-negative",
            });
        }
        let g = match self.at(t)? {
            Basis::Simple => rate * t,
            Basis::Continuous => (rate * t).exp_m1(),
            Basis::Compounded(m) => {
                if rate / m <= -1.0 {
                    return Err(FinanceError::InvalidInput {
                        what: "rate",
                        reason: "rate per period must be greater than -100%",
                    });
                }
                ((rate / m).ln_1p() * (m * t)).exp_m1()
            }
        };
        if g <= -1.0 {
            return Err(FinanceError::InvalidInput {
                what: "rate",
                reason: "compound factor must be positive",
            });
        }
        output("compound factor", g)
    }

    /// Growth of 1 at `rate` over `t ≥ 0` years (QuantLib
    /// `InterestRate::compoundFactor`).
    ///
    /// ```
    /// use finance_rs::rates::{Compounding, Frequency};
    /// assert!((Compounding::Simple.compound_factor(0.04, 0.5)? - 1.02).abs() < 1e-15);
    /// let q = Compounding::Compounded(Frequency::Quarterly).compound_factor(0.04, 1.0)?;
    /// assert!((q - 1.01f64.powi(4)).abs() < 1e-15);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// [`FinanceError::InvalidInput`] for `t < 0`, a [`Frequency::Once`]
    /// compounding, a per-period rate at or below −100%, or a non-positive
    /// factor; [`FinanceError::NonFinite`] / [`FinanceError::Overflow`].
    pub fn compound_factor(self, rate: f64, t: f64) -> Result<f64> {
        let g = self.growth(rate, t)?;
        output("compound factor", 1.0 + g)
    }

    /// The rate on this basis whose compound factor over `t > 0` years is
    /// `compound` (QuantLib `InterestRate::impliedRate`); `compound = 1`
    /// implies 0 for any `t`.
    ///
    /// ```
    /// use finance_rs::rates::Compounding;
    /// let r = Compounding::Continuous.implied_rate(2f64.exp(), 4.0)?;
    /// assert!((r - 0.5).abs() < 1e-15);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// [`FinanceError::InvalidInput`] for `compound ≤ 0` or `t ≤ 0` (with
    /// `compound ≠ 1`) and for a [`Frequency::Once`] compounding.
    #[allow(clippy::float_cmp)] // exactly 1 implies exactly 0
    pub fn implied_rate(self, compound: f64, t: f64) -> Result<f64> {
        finite("compound", compound)?;
        finite("t", t)?;
        if compound <= 0.0 {
            return Err(FinanceError::InvalidInput {
                what: "compound",
                reason: "must be positive",
            });
        }
        if compound == 1.0 {
            return if t >= 0.0 {
                Ok(0.0)
            } else {
                Err(FinanceError::InvalidInput {
                    what: "t",
                    reason: "must be non-negative",
                })
            };
        }
        if t <= 0.0 {
            return Err(FinanceError::InvalidInput {
                what: "t",
                reason: "must be positive",
            });
        }
        let r = match self.at(t)? {
            Basis::Simple => (compound - 1.0) / t,
            Basis::Continuous => compound.ln() / t,
            Basis::Compounded(m) => (compound.ln() / (m * t)).exp_m1() * m,
        };
        output("implied rate", r)
    }
}

/// A [`Compounding`] resolved at a given `t`.
enum Basis {
    Simple,
    Continuous,
    /// Periods per year.
    Compounded(f64),
}

fn positive_periods(m: f64) -> Result<f64> {
    finite("periods_per_year", m)?;
    if m <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "periods_per_year",
            reason: "must be positive",
        });
    }
    Ok(m)
}

fn above_minus_one(what: &'static str, r: f64) -> Result<f64> {
    finite(what, r)?;
    if r <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what,
            reason: "must be greater than -100%",
        });
    }
    Ok(r)
}

/// Effective annual rate of a nominal rate compounded `m` times a year:
/// `(1 + i/m)^m − 1` (Excel `EFFECT`, without its truncation of `m`).
///
/// ```
/// use finance_rs::rates::effective;
/// assert!((effective(0.0525, 4.0)? - 0.053_542_667_370_758_7).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for `m ≤ 0` or `i/m ≤ −1`.
pub fn effective(nominal: f64, periods_per_year: f64) -> Result<f64> {
    let m = positive_periods(periods_per_year)?;
    above_minus_one(
        "nominal / periods_per_year",
        finite("nominal", nominal)? / m,
    )?;
    output("effective", ((nominal / m).ln_1p() * m).exp_m1())
}

/// Nominal rate compounded `m` times a year with effective annual rate `e`:
/// `m·((1 + e)^(1/m) − 1)` (Excel `NOMINAL`).
///
/// ```
/// use finance_rs::rates::nominal;
/// assert!((nominal(0.053_543, 4.0)? - 0.052_500_319_868_355_1).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for `m ≤ 0` or `e ≤ −1`.
pub fn nominal(effective: f64, periods_per_year: f64) -> Result<f64> {
    let m = positive_periods(periods_per_year)?;
    let e = above_minus_one("effective", effective)?;
    output("nominal", (e.ln_1p() / m).exp_m1() * m)
}

/// Per-period rate equivalent to the annual rate `annual`:
/// `(1 + R)^(1/m) − 1`. This is exactly how `perf` de-annualizes `rf`
/// (quantstats `to_excess_returns`).
///
/// ```
/// use finance_rs::rates::periodic_from_annual;
/// assert!((periodic_from_annual(0.126_825_030_131_969_8, 12.0)? - 0.01).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for `m ≤ 0` or `annual ≤ −1`.
pub fn periodic_from_annual(annual: f64, periods_per_year: f64) -> Result<f64> {
    let m = positive_periods(periods_per_year)?;
    let r = above_minus_one("annual", annual)?;
    output("periodic rate", (1.0 + r).powf(1.0 / m) - 1.0)
}

/// De-annualize: the per-period return compounding to `annual` over `m`
/// periods, `(1 + R)^(1/m) − 1` (ffn `deannualize`). Same as
/// [`periodic_from_annual`].
///
/// # Errors
/// As [`periodic_from_annual`].
/// ```rust
/// assert!(finance_rs::rates::effective(0.12, 12.0).unwrap() > 0.12);
/// ```
pub fn deannualize(annual: f64, periods_per_year: f64) -> Result<f64> {
    periodic_from_annual(annual, periods_per_year)
}

/// Annualize a per-period return: `(1 + r)^m − 1` (ffn `annualize` with a
/// duration of one period).
///
/// ```
/// use finance_rs::rates::annualize;
/// assert!((annualize(0.01, 12.0)? - 0.126_825_030_131_969_7).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for `m ≤ 0` or `r ≤ −1`.
pub fn annualize(periodic: f64, periods_per_year: f64) -> Result<f64> {
    let m = positive_periods(periods_per_year)?;
    let r = above_minus_one("periodic", periodic)?;
    output("annualized rate", (r.ln_1p() * m).exp_m1())
}

/// Continuously compounded rate equivalent to `r` compounded `m` times a
/// year: `m·ln(1 + r/m)`.
///
/// # Errors
/// [`FinanceError::InvalidInput`] for `m ≤ 0` or `r/m ≤ −1`.
/// ```rust
/// assert!(finance_rs::rates::effective(0.12, 12.0).unwrap() > 0.12);
/// ```
pub fn continuous_from_discrete(rate: f64, periods_per_year: f64) -> Result<f64> {
    let m = positive_periods(periods_per_year)?;
    above_minus_one("rate / periods_per_year", finite("rate", rate)? / m)?;
    output("continuous rate", m * (rate / m).ln_1p())
}

/// Rate compounded `m` times a year equivalent to the continuously
/// compounded `rc`: `m·(e^(rc/m) − 1)`.
///
/// ```
/// use finance_rs::rates::{continuous_from_discrete, discrete_from_continuous};
/// let rc = continuous_from_discrete(0.06, 2.0)?;
/// assert!((discrete_from_continuous(rc, 2.0)? - 0.06).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] for `m ≤ 0`; [`FinanceError::Overflow`].
pub fn discrete_from_continuous(rate: f64, periods_per_year: f64) -> Result<f64> {
    let m = positive_periods(periods_per_year)?;
    finite("rate", rate)?;
    output("discrete rate", m * (rate / m).exp_m1())
}

/// Real rate by the exact Fisher relation `(1 + i)/(1 + π) − 1`.
///
/// ```
/// use finance_rs::rates::real_rate;
/// assert!((real_rate(0.08, 0.03)? - 0.048_543_689_320_388_35).abs() < 1e-15);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
///
/// # Errors
/// [`FinanceError::InvalidInput`] when either rate is at or below −100%.
pub fn real_rate(nominal: f64, inflation: f64) -> Result<f64> {
    let i = above_minus_one("nominal", nominal)?;
    let p = above_minus_one("inflation", inflation)?;
    output("real rate", (i - p) / (1.0 + p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_then_compounded_switches_after_one_period() -> Result<()> {
        let c = Compounding::SimpleThenCompounded(Frequency::Semiannual);
        assert_eq!(
            c.compound_factor(0.06, 0.25)?,
            Compounding::Simple.compound_factor(0.06, 0.25)?
        );
        let late = Compounding::Compounded(Frequency::Semiannual).compound_factor(0.06, 2.0)?;
        assert_eq!(c.compound_factor(0.06, 2.0)?, late);
        Ok(())
    }

    #[test]
    fn bad_bases_are_errors() {
        assert!(Compounding::Compounded(Frequency::Once)
            .compound_factor(0.05, 1.0)
            .is_err());
        assert!(Compounding::Simple.compound_factor(0.05, -1.0).is_err());
        assert!(Compounding::Simple.compound_factor(-2.0, 1.0).is_err());
        assert!(Compounding::Simple.implied_rate(0.0, 1.0).is_err());
        assert!(effective(0.05, 0.0).is_err());
        assert!(real_rate(0.05, -1.0).is_err());
    }

    #[test]
    fn frequencies_and_implied_quotes_cover_each_compounding_basis() -> Result<()> {
        assert_eq!(Frequency::Daily.per_year(), Some(365));
        assert_eq!(Frequency::PerYear(7).per_year(), Some(7));
        let compoundings = [
            Compounding::Simple,
            Compounding::Continuous,
            Compounding::Compounded(Frequency::Quarterly),
        ];
        for compounding in compoundings {
            let factor = compounding.compound_factor(0.08, 2.0)?;
            assert!((compounding.implied_rate(factor, 2.0)? - 0.08).abs() < 1e-12);
        }
        assert_eq!(Compounding::Simple.implied_rate(1.0, 1.0)?, 0.0);
        assert!(Compounding::Simple.implied_rate(1.0, -0.1).is_err());
        Ok(())
    }
}
