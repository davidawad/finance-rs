use crate::{Date, FinanceError, Result};

/// CDS contract family and reference portfolio.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum CdsKind {
    /// Single-name contract on one reference entity.
    SingleName {
        /// Legal or market identifier of the reference entity.
        reference_entity: String,
    },
    /// Standardized index contract.
    Index {
        /// Index family, such as `CDX.NA.IG` or `iTraxx Europe`.
        index: String,
        /// Published index series number.
        series: u16,
        /// Published index version number.
        version: u8,
    },
}

/// Standard North American running coupon, in basis points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum StandardCoupon {
    /// Investment-grade coupon, 100 bp per year.
    InvestmentGrade,
    /// High-yield coupon, 500 bp per year.
    HighYield,
}

impl StandardCoupon {
    /// Coupon as an annual decimal rate.
    #[must_use]
    pub const fn rate(self) -> f64 {
        match self {
            Self::InvestmentGrade => 0.01,
            Self::HighYield => 0.05,
        }
    }

    /// Coupon in basis points.
    #[must_use]
    pub const fn basis_points(self) -> u16 {
        match self {
            Self::InvestmentGrade => 100,
            Self::HighYield => 500,
        }
    }
}

/// Standard CDS coupon cash amount for an ACT/360 accrual period.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn standard_coupon_amount(
    coupon: StandardCoupon,
    notional: f64,
    accrual_days: u32,
) -> Result<f64> {
    if !notional.is_finite() {
        return Err(FinanceError::NonFinite { what: "notional" });
    }
    if notional <= 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "notional",
            reason: "must be positive",
        });
    }
    let amount = coupon.rate() * notional * f64::from(accrual_days) / 360.0;
    if amount.is_finite() {
        Ok(amount)
    } else {
        Err(FinanceError::Overflow { what: "CDS coupon" })
    }
}

/// Standard single-name or index CDS terms.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CdsInstrument {
    /// Contract family and reference name.
    pub kind: CdsKind,
    /// Protection start date (inclusive).
    pub effective: Date,
    /// Unadjusted maturity date, normally an IMM 20th.
    pub maturity: Date,
    /// Notional in currency units.
    pub notional: f64,
    /// Running coupon as an annual decimal (100 bp = 0.01).
    pub coupon: f64,
    /// Assumed recovery fraction in `[0, 1)`.
    pub recovery: f64,
    /// Upfront in currency units, positive when paid by the protection buyer.
    pub upfront: f64,
}

impl CdsInstrument {
    /// Build and validate a CDS contract.
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn new(
        kind: CdsKind,
        effective: Date,
        maturity: Date,
        notional: f64,
        coupon: f64,
        recovery: f64,
        upfront: f64,
    ) -> Result<Self> {
        let instrument = Self {
            kind,
            effective,
            maturity,
            notional,
            coupon,
            recovery,
            upfront,
        };
        instrument.validate()?;
        Ok(instrument)
    }

    /// Validate contract fields.
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn validate(&self) -> Result<()> {
        match &self.kind {
            CdsKind::SingleName { reference_entity } if reference_entity.trim().is_empty() => {
                return Err(FinanceError::InvalidInput {
                    what: "CDS single-name reference",
                    reason: "must not be empty",
                });
            }
            CdsKind::Index {
                index,
                series,
                version,
            } if index.trim().is_empty() || *series == 0 || *version == 0 => {
                return Err(FinanceError::InvalidInput {
                    what: "CDS index reference",
                    reason: "name, series, and version must be present and positive",
                });
            }
            _ => {}
        }
        for (what, value) in [
            ("notional", self.notional),
            ("coupon", self.coupon),
            ("recovery", self.recovery),
            ("upfront", self.upfront),
        ] {
            if !value.is_finite() {
                return Err(FinanceError::NonFinite { what });
            }
        }
        if self.effective >= self.maturity {
            return Err(FinanceError::InvalidInput {
                what: "maturity",
                reason: "must follow effective date",
            });
        }
        if self.notional <= 0.0 {
            return Err(FinanceError::InvalidInput {
                what: "notional",
                reason: "must be positive",
            });
        }
        if self.coupon < 0.0 {
            return Err(FinanceError::InvalidInput {
                what: "coupon",
                reason: "must be non-negative",
            });
        }
        if !(0.0..1.0).contains(&self.recovery) {
            return Err(FinanceError::InvalidInput {
                what: "recovery",
                reason: "must be in [0, 1)",
            });
        }
        Ok(())
    }
}

/// First quarterly March/June/September/December 20th strictly after `date`.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn next_imm_date(date: Date) -> Result<Date> {
    let year = date.year();
    for month in [3, 6, 9, 12] {
        let candidate = Date::new(year, month, 20)?;
        if candidate > date {
            return Ok(candidate);
        }
    }
    Date::new(year.saturating_add(1), 3, 20)
}
