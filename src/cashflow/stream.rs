//! [`CashFlowStream`]: dated, typed flows sorted by date, the common
//! currency between deal, fund, real-estate and tax-lot code. Every
//! valuation lowers to the slice functions in [`super`].

use super::{macaulay_duration_dated, xirr_with, xnpv_with, XirrOptions};
use crate::daycount::DayCount;
use crate::error::{finite, output, FinanceError, Result};
use crate::period::Period;
use crate::rates::Rate;
use crate::Date;

/// What a flow is, so one stream can carry contributions, distributions,
/// fees and NAV marks side by side.
///
/// ```
/// let kind = finance_rs::cashflow::FlowKind::Contribution;
/// assert_eq!(kind, finance_rs::cashflow::FlowKind::Contribution);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[non_exhaustive]
pub enum FlowKind {
    /// Capital paid in (an LP contribution, an equity check).
    Contribution,
    /// Capital paid out to the holder.
    Distribution,
    /// Interest.
    Interest,
    /// Principal repayment.
    Principal,
    /// A fee.
    Fee,
    /// A tax payment or refund.
    Tax,
    /// A net asset value mark (a terminal value, not a cash movement).
    Nav,
    /// Anything else, and the kind of netted flows of mixed kinds.
    #[default]
    Other,
}

/// One dated amount (received > 0, paid < 0).
///
/// ```
/// use finance_rs::cashflow::{CashFlow, FlowKind};
/// use finance_rs::Date;
/// let flow = CashFlow { date: Date::new(2026, 1, 1)?, amount: 10.0, kind: FlowKind::Distribution };
/// assert_eq!(flow.amount, 10.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CashFlow {
    /// When it happens.
    pub date: Date,
    /// How much (spreadsheet signs).
    pub amount: f64,
    /// What it is.
    pub kind: FlowKind,
}

impl CashFlow {
    /// A flow of `amount` on `date`.
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, FlowKind};
    /// use finance_rs::Date;
    /// let flow = CashFlow::new(Date::new(2026, 1, 1)?, 10.0, FlowKind::Distribution);
    /// assert_eq!(flow.amount, 10.0);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    pub fn new(date: impl Into<Date>, amount: f64, kind: FlowKind) -> Self {
        Self {
            date: date.into(),
            amount,
            kind,
        }
    }
}

/// Flows sorted by date (stably: same-day flows keep their input order),
/// every amount finite. Operations return new streams.
///
/// ```rust
/// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
/// use finance_rs::Date;
/// let d = |y| Date::new(y, 1, 1);
/// let s = CashFlowStream::new([
///     CashFlow::new(d(2021)?, 60.0, FlowKind::Distribution),
///     CashFlow::new(d(2020)?, -100.0, FlowKind::Contribution),
///     CashFlow::new(d(2022)?, 60.0, FlowKind::Distribution),
/// ])?;
/// // Contributions and distributions are ordered by date.
/// let r = s.xirr()?;
/// assert!(s.xnpv(r, finance_rs::daycount::DayCount::Act365F)?.abs() < 1e-9);
/// let (inflows, outflows) = s.split_signs();
/// assert_eq!((inflows.len(), outflows.len()), (2, 1));
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(try_from = "Vec<CashFlow>", into = "Vec<CashFlow>")
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CashFlowStream {
    flows: Vec<CashFlow>,
}

impl TryFrom<Vec<CashFlow>> for CashFlowStream {
    type Error = FinanceError;

    fn try_from(flows: Vec<CashFlow>) -> Result<Self> {
        Self::new(flows)
    }
}

impl From<CashFlowStream> for Vec<CashFlow> {
    fn from(s: CashFlowStream) -> Self {
        s.flows
    }
}

impl CashFlowStream {
    /// Validate (every amount finite) and sort stably by date.
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::Date;
    /// let stream = CashFlowStream::new([CashFlow::new(Date::new(2026, 1, 1)?, 10.0, FlowKind::Other)])?;
    /// assert_eq!(stream.len(), 1);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// [`FinanceError::NonFinite`] for a non-finite amount.
    pub fn new(flows: impl IntoIterator<Item = CashFlow>) -> Result<Self> {
        let mut flows: Vec<CashFlow> = flows.into_iter().collect();
        for f in &flows {
            finite("amount", f.amount)?;
        }
        flows.sort_by_key(|f| f.date);
        Ok(Self { flows })
    }

    /// From parallel date and amount slices, every flow of `kind`.
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlowStream, FlowKind};
    /// use finance_rs::Date;
    /// let dates = [Date::new(2026, 1, 1)?, Date::new(2027, 1, 1)?];
    /// let stream = CashFlowStream::from_slices(&dates, &[-100.0, 110.0], FlowKind::Other)?;
    /// assert_eq!(stream.len(), 2);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// [`FinanceError::LengthMismatch`]; as [`CashFlowStream::new`].
    pub fn from_slices<D: Copy + Into<Date>>(
        dates: &[D],
        amounts: &[f64],
        kind: FlowKind,
    ) -> Result<Self> {
        crate::error::same_len("dates", dates.len(), "amounts", amounts.len())?;
        Self::new(
            dates
                .iter()
                .zip(amounts)
                .map(|(d, a)| CashFlow::new(*d, *a, kind)),
        )
    }

    /// The flows, in date order.
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::Date;
    /// let stream = CashFlowStream::new([CashFlow::new(Date::new(2026, 1, 1)?, 10.0, FlowKind::Other)])?;
    /// assert_eq!(stream.flows()[0].amount, 10.0);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn flows(&self) -> &[CashFlow] {
        &self.flows
    }

    /// Number of flows.
    ///
    /// ```
    /// assert_eq!(finance_rs::cashflow::CashFlowStream::default().len(), 0);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        self.flows.len()
    }

    /// Whether there are no flows.
    ///
    /// ```
    /// assert!(finance_rs::cashflow::CashFlowStream::default().is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.flows.is_empty()
    }

    /// The dates, in order (for the slice APIs).
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::Date;
    /// let date = Date::new(2026, 1, 1)?;
    /// let stream = CashFlowStream::new([CashFlow::new(date, 10.0, FlowKind::Other)])?;
    /// assert_eq!(stream.dates(), vec![date]);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn dates(&self) -> Vec<Date> {
        self.flows.iter().map(|f| f.date).collect()
    }

    /// The amounts, in date order (for the slice APIs).
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::Date;
    /// let stream = CashFlowStream::new([CashFlow::new(Date::new(2026, 1, 1)?, 10.0, FlowKind::Other)])?;
    /// assert_eq!(stream.amounts(), vec![10.0]);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn amounts(&self) -> Vec<f64> {
        self.flows.iter().map(|f| f.amount).collect()
    }

    fn keep(&self, pred: impl Fn(&CashFlow) -> bool) -> Self {
        Self {
            flows: self.flows.iter().copied().filter(pred).collect(),
        }
    }

    /// Both streams' flows, sorted; on a shared date `self`'s come first.
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::Date;
    /// let date = Date::new(2026, 1, 1)?;
    /// let a = CashFlowStream::new([CashFlow::new(date, 1.0, FlowKind::Other)])?;
    /// let b = CashFlowStream::new([CashFlow::new(date, 2.0, FlowKind::Other)])?;
    /// assert_eq!(a.merge(&b).len(), 2);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn merge(&self, other: &Self) -> Self {
        let mut flows = Vec::with_capacity(self.len() + other.len());
        flows.extend_from_slice(&self.flows);
        flows.extend_from_slice(&other.flows);
        flows.sort_by_key(|f| f.date);
        Self { flows }
    }

    /// One flow per date: amounts summed, the kind kept when every netted
    /// flow shares it and [`FlowKind::Other`] otherwise. Zero nets are kept.
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::Date;
    /// let date = Date::new(2026, 1, 1)?;
    /// let stream = CashFlowStream::new([CashFlow::new(date, 2.0, FlowKind::Other), CashFlow::new(date, -1.0, FlowKind::Other)])?;
    /// assert_eq!(stream.net_by_date().amounts(), vec![1.0]);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn net_by_date(&self) -> Self {
        let mut flows: Vec<CashFlow> = Vec::with_capacity(self.len());
        for f in &self.flows {
            match flows.last_mut() {
                Some(last) if last.date == f.date => {
                    last.amount += f.amount;
                    if last.kind != f.kind {
                        last.kind = FlowKind::Other;
                    }
                }
                _ => flows.push(*f),
            }
        }
        Self { flows }
    }

    /// Every date moved by `p` ([`Date::add_period`]).
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::{Date, period::{Period, TimeUnit}};
    /// let stream = CashFlowStream::new([CashFlow::new(Date::new(2026, 1, 1)?, 1.0, FlowKind::Other)])?;
    /// assert_eq!(stream.shift(Period::new(1, TimeUnit::Years))?.len(), 1);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// [`FinanceError::InvalidInput`] when a date leaves the supported range.
    pub fn shift(&self, p: Period) -> Result<Self> {
        let flows = self
            .flows
            .iter()
            .map(|f| {
                Ok(CashFlow {
                    date: f.date.add_period(p)?,
                    ..*f
                })
            })
            .collect::<Result<Vec<_>>>()?;
        // Month-end clamping can merge but never reorder dates.
        Ok(Self { flows })
    }

    /// Every amount times `k`.
    ///
    /// # Errors
    /// [`FinanceError::NonFinite`] for a non-finite `k`,
    /// [`FinanceError::Overflow`] when an amount overflows.
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::Date;
    /// let stream = CashFlowStream::new([CashFlow::new(Date::new(2026, 1, 1)?, 10.0, FlowKind::Other)])?;
    /// assert_eq!(stream.scale(2.0)?.amounts(), vec![20.0]);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    pub fn scale(&self, k: f64) -> Result<Self> {
        finite("k", k)?;
        let flows = self
            .flows
            .iter()
            .map(|f| {
                Ok(CashFlow {
                    amount: output("scaled amount", f.amount * k)?,
                    ..*f
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { flows })
    }

    /// Only the flows of the given kinds.
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::Date;
    /// let stream = CashFlowStream::new([CashFlow::new(Date::new(2026, 1, 1)?, 10.0, FlowKind::Fee)])?;
    /// assert_eq!(stream.filter(&[FlowKind::Fee]).len(), 1);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn filter(&self, kinds: &[FlowKind]) -> Self {
        self.keep(|f| kinds.contains(&f.kind))
    }

    /// Only the flows dated within `[start, end]` (both inclusive).
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::Date;
    /// let start = Date::new(2026, 1, 1)?;
    /// let stream = CashFlowStream::new([CashFlow::new(start, 10.0, FlowKind::Other)])?;
    /// assert_eq!(stream.between(start, start).len(), 1);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn between(&self, start: Date, end: Date) -> Self {
        self.keep(|f| f.date >= start && f.date <= end)
    }

    /// `(inflows, outflows)`: the positive and the negative flows; zeros
    /// are in neither.
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::Date;
    /// let stream = CashFlowStream::new([CashFlow::new(Date::new(2026, 1, 1)?, 10.0, FlowKind::Other), CashFlow::new(Date::new(2027, 1, 1)?, -5.0, FlowKind::Other)])?;
    /// let (inflows, outflows) = stream.split_signs();
    /// assert_eq!((inflows.len(), outflows.len()), (1, 1));
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn split_signs(&self) -> (Self, Self) {
        (self.keep(|f| f.amount > 0.0), self.keep(|f| f.amount < 0.0))
    }

    /// Sum of the amounts.
    ///
    /// # Errors
    /// [`FinanceError::Overflow`] when the sum overflows.
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::Date;
    /// let stream = CashFlowStream::new([CashFlow::new(Date::new(2026, 1, 1)?, 10.0, FlowKind::Other)])?;
    /// assert_eq!(stream.total()?, 10.0);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    pub fn total(&self) -> Result<f64> {
        output("total", self.flows.iter().map(|f| f.amount).sum())
    }

    /// [`super::xnpv_with`] of the stream.
    ///
    /// # Errors
    /// As [`super::xnpv_with`].
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::{daycount::DayCount, Date};
    /// let stream = CashFlowStream::new([CashFlow::new(Date::new(2026, 1, 1)?, 100.0, FlowKind::Other)])?;
    /// assert_eq!(stream.xnpv(0.05, DayCount::Act365F)?, 100.0);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    pub fn xnpv(&self, rate: f64, day_count: DayCount) -> Result<f64> {
        xnpv_with(rate, &self.dates(), &self.amounts(), day_count)
    }

    /// [`super::xirr`] of the stream (ACT/365F, guess 0.1).
    ///
    /// # Errors
    /// As [`super::xirr`].
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::Date;
    /// let stream = CashFlowStream::new([CashFlow::new(Date::new(2026, 1, 1)?, -100.0, FlowKind::Other), CashFlow::new(Date::new(2027, 1, 1)?, 110.0, FlowKind::Other)])?;
    /// assert!(stream.xirr()?.is_finite());
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    pub fn xirr(&self) -> Result<f64> {
        self.xirr_with(XirrOptions::default())
    }

    /// [`super::xirr_with`] of the stream.
    ///
    /// # Errors
    /// As [`super::xirr_with`].
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind, XirrOptions};
    /// use finance_rs::Date;
    /// let stream = CashFlowStream::new([CashFlow::new(Date::new(2026, 1, 1)?, -100.0, FlowKind::Other), CashFlow::new(Date::new(2027, 1, 1)?, 110.0, FlowKind::Other)])?;
    /// assert!(stream.xirr_with(XirrOptions::default())?.is_finite());
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    pub fn xirr_with(&self, opts: XirrOptions) -> Result<f64> {
        xirr_with(&self.dates(), &self.amounts(), opts)
    }

    /// [`super::macaulay_duration_dated`] of the stream.
    ///
    /// # Errors
    /// As [`super::macaulay_duration_dated`].
    ///
    /// ```
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::{daycount::DayCount, Date};
    /// let stream = CashFlowStream::new([CashFlow::new(Date::new(2026, 1, 1)?, 100.0, FlowKind::Other), CashFlow::new(Date::new(2027, 1, 1)?, 110.0, FlowKind::Other)])?;
    /// assert!(stream.macaulay_duration(0.05, DayCount::Act365F)?.is_finite());
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    pub fn macaulay_duration(&self, rate: f64, day_count: DayCount) -> Result<f64> {
        macaulay_duration_dated(rate, &self.dates(), &self.amounts(), day_count)
    }

    /// Present value at `as_of` under a quoted [`Rate`]:
    /// `Σ amount_i · rate.discount_factor_between(as_of, date_i)`.
    ///
    /// ```rust
    /// use finance_rs::cashflow::{CashFlow, CashFlowStream, FlowKind};
    /// use finance_rs::rates::{Compounding, Frequency, Rate};
    /// use finance_rs::{daycount::DayCount, Date};
    /// let s = CashFlowStream::new([CashFlow::new(Date::new(2026, 1, 1)?, 110.0, FlowKind::Other)])?;
    /// let r = Rate::new(0.10, Compounding::Compounded(Frequency::Annual), DayCount::Thirty360Us);
    /// assert!((s.pv(&r, Date::new(2025, 1, 1)?)? - 100.0).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// [`FinanceError::InvalidInput`] for a flow dated before `as_of`; as
    /// [`Rate::discount_factor_between`] otherwise.
    pub fn pv(&self, rate: &Rate, as_of: Date) -> Result<f64> {
        let mut sum = 0.0;
        for f in &self.flows {
            sum += f.amount * rate.discount_factor_between(as_of, f.date)?;
        }
        output("pv", sum)
    }
}

#[cfg(test)]
#[path = "stream_tests.rs"]
mod coverage_tests;
