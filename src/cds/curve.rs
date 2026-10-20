use crate::{Date, FinanceError, Result};

/// Market par-spread calibration point.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ParSpreadQuote {
    /// Unadjusted maturity date.
    pub maturity: Date,
    /// Par running spread as an annual decimal (100 bp = 0.01).
    pub spread: f64,
}

/// A piecewise-constant hazard rate ending at `maturity`.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct HazardNode {
    /// End date of this hazard interval.
    pub maturity: Date,
    /// Annual default intensity on the interval ending at this node.
    pub hazard_rate: f64,
}

/// Survival curve bootstrapped from CDS par spreads.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CreditCurve {
    /// Curve valuation/protection start date.
    pub valuation_date: Date,
    /// Calibrated piecewise-constant intervals.
    pub nodes: Vec<HazardNode>,
}

impl CreditCurve {
    /// Bootstrap sequential interval hazards from increasing par-spread maturities.
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn bootstrap(
        valuation_date: Date,
        quotes: &[ParSpreadQuote],
        discount_rate: f64,
        recovery: f64,
    ) -> Result<Self> {
        if !discount_rate.is_finite() {
            return Err(FinanceError::NonFinite {
                what: "discount_rate",
            });
        }
        if discount_rate <= -1.0 {
            return Err(FinanceError::InvalidInput {
                what: "discount_rate",
                reason: "must exceed -1",
            });
        }
        if !recovery.is_finite() {
            return Err(FinanceError::NonFinite { what: "recovery" });
        }
        if !(0.0..1.0).contains(&recovery) {
            return Err(FinanceError::InvalidInput {
                what: "recovery",
                reason: "must be in [0, 1)",
            });
        }
        if quotes.is_empty() {
            return Err(FinanceError::TooShort {
                what: "quotes",
                min: 1,
                got: 0,
            });
        }
        let mut nodes = Vec::with_capacity(quotes.len());
        let mut previous = valuation_date;
        for quote in quotes {
            if !quote.spread.is_finite() {
                return Err(FinanceError::NonFinite { what: "spread" });
            }
            if quote.maturity <= previous || quote.spread < 0.0 {
                return Err(FinanceError::InvalidInput {
                    what: "quotes",
                    reason: "maturities must increase and spreads must be non-negative",
                });
            }
            let mut low = 0.0;
            let mut high = 10.0;
            let mut trial = nodes.clone();
            trial.push(HazardNode {
                maturity: quote.maturity,
                hazard_rate: high,
            });
            let upper = super::pricing::par_spread_for_nodes(
                valuation_date,
                quote.maturity,
                discount_rate,
                recovery,
                &trial,
            )?;
            if upper < quote.spread {
                return Err(FinanceError::NoSolution {
                    what: "CDS hazard bootstrap",
                });
            }
            for _ in 0..80 {
                let mid = (low + high) * 0.5;
                *trial.last_mut().ok_or(FinanceError::NoSolution {
                    what: "CDS hazard bootstrap",
                })? = HazardNode {
                    maturity: quote.maturity,
                    hazard_rate: mid,
                };
                let model = super::pricing::par_spread_for_nodes(
                    valuation_date,
                    quote.maturity,
                    discount_rate,
                    recovery,
                    &trial,
                )?;
                if model < quote.spread {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            nodes.push(HazardNode {
                maturity: quote.maturity,
                hazard_rate: (low + high) * 0.5,
            });
            previous = quote.maturity;
        }
        Ok(Self {
            valuation_date,
            nodes,
        })
    }

    /// Piecewise-constant hazard at a date, extended flat beyond the last node.
    #[must_use]
    pub fn hazard_at(&self, date: Date) -> f64 {
        self.nodes
            .iter()
            .find(|node| date <= node.maturity)
            .or_else(|| self.nodes.last())
            .map_or(0.0, |node| node.hazard_rate)
    }

    /// Survival probability from curve date through `date`.
    #[must_use]
    #[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
    pub fn survival_probability(&self, date: Date) -> f64 {
        if date <= self.valuation_date {
            return 1.0;
        }
        let mut survival = 1.0;
        let mut start = self.valuation_date;
        for node in &self.nodes {
            let end = date.min(node.maturity);
            if end > start {
                survival *= (-node.hazard_rate * end.days_since(start) as f64 / 365.0).exp();
                start = end;
            }
            if date <= node.maturity {
                return survival;
            }
        }
        if let Some(last) = self.nodes.last() {
            survival *= (-last.hazard_rate * date.days_since(start) as f64 / 365.0).exp();
        }
        survival
    }

    /// Cumulative default probability from curve date through `date`.
    #[must_use]
    pub fn default_probability(&self, date: Date) -> f64 {
        1.0 - self.survival_probability(date)
    }
}
