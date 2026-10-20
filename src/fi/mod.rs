//! Fixed income: plain-vanilla bond price and yield, accrued interest,
//! Macaulay and modified duration, convexity and DV01.
//!
//! A [`Bond`] pays `face · coupon_rate / frequency` at the end of each of its
//! remaining `periods` coupon periods and `face` with the last coupon. Yields
//! are nominal annual rates compounded `frequency` times a year (the street
//! convention), so the per-period discount rate is `i = ytm / frequency`.
//! Prices are per `face` (use `face = 100.0` for percent-of-par quotes).

pub mod excel;
pub mod mm;
mod risk;
pub use excel::FixedRateBond;
mod ytm;

pub use risk::{cashflow_risk, BondRisk};

use crate::error::{finite, output, FinanceError, Result};

/// Present value of period-end cash flows at a periodic yield.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::cashflow_price), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn cashflow_price(cashflows: &[f64], periodic_rate: f64) -> Result<f64> {
    finite("periodic_rate", periodic_rate)?;
    if periodic_rate <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "periodic_rate",
            reason: "must exceed -1",
        });
    }
    if cashflows.is_empty() {
        return Err(FinanceError::TooShort {
            what: "cashflows",
            min: 1,
            got: 0,
        });
    }
    let mut factor = 1.0 + periodic_rate;
    let mut price = 0.0;
    for amount in cashflows {
        finite("cashflow", *amount)?;
        price += amount / factor;
        factor *= 1.0 + periodic_rate;
    }
    output("bond price", price)
}

/// Derivative of [`cashflow_price`] with respect to periodic yield.
/// ```rust
/// assert_eq!(::core::mem::size_of_val(&finance_rs::fi::cashflow_price_derivative), 0);
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn cashflow_price_derivative(cashflows: &[f64], periodic_rate: f64) -> Result<f64> {
    finite("periodic_rate", periodic_rate)?;
    if periodic_rate <= -1.0 {
        return Err(FinanceError::InvalidInput {
            what: "periodic_rate",
            reason: "must exceed -1",
        });
    }
    let mut factor = 1.0 + periodic_rate;
    let mut derivative = 0.0;
    for (index, amount) in cashflows.iter().enumerate() {
        finite("cashflow", *amount)?;
        let period = period_number(index)?;
        derivative -= f64::from(period) * amount / (factor * (1.0 + periodic_rate));
        factor *= 1.0 + periodic_rate;
    }
    output("bond price derivative", derivative)
}

/// Longest supported schedule (e.g. 8,000+ years of monthly coupons).
const MAX_PERIODS: u32 = 100_000;

fn period_number(index: usize) -> Result<u32> {
    u32::try_from(index + 1).map_err(|_| FinanceError::Overflow {
        what: "period index",
    })
}

/// A fixed-coupon bullet bond with a whole number of remaining periods.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fi::Bond>() > 0);
/// ```
pub struct Bond {
    /// Redemption amount, `> 0`.
    pub face: f64,
    /// Annual coupon rate as a fraction of face, `>= 0` (e.g. `0.05`).
    pub coupon_rate: f64,
    /// Coupon periods remaining until maturity, `1..=100_000`.
    pub periods: u32,
    /// Coupons per year: 1, 2, 4 or 12.
    pub frequency: u32,
}

/// Dirty (full), clean (quoted) price and accrued interest at a settlement
/// part-way through a coupon period.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
/// ```rust
/// assert!(::core::mem::size_of::<finance_rs::fi::BondPrice>() > 0);
/// ```
pub struct BondPrice {
    /// Present value of all remaining cash flows (what the buyer pays).
    pub dirty: f64,
    /// `dirty − accrued` (what is quoted).
    pub clean: f64,
    /// Coupon accrued since the last coupon date.
    pub accrued: f64,
}

impl Bond {
    /// Validated constructor.
    ///
    /// `coupon per period = face · coupon_rate / frequency`
    ///
    /// ```rust
    /// use finance_rs::fi::Bond;
    /// let b = Bond::new(1_000.0, 0.06, 20, 2)?;
    /// assert!((b.coupon()? - 30.0).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// [`FinanceError::NonFinite`] for non-finite input,
    /// [`FinanceError::InvalidInput`] for a non-positive face, negative coupon,
    /// zero or more than 100,000 periods, or a zero coupon frequency.
    pub fn new(face: f64, coupon_rate: f64, periods: u32, frequency: u32) -> Result<Self> {
        let b = Self {
            face,
            coupon_rate,
            periods,
            frequency,
        };
        b.check()?;
        Ok(b)
    }

    /// Constructor from years to maturity; `years · frequency` must be a whole
    /// number of periods (within `1e-9`).
    ///
    /// `periods = round(years · frequency)`
    ///
    /// ```rust
    /// use finance_rs::fi::Bond;
    /// let b = Bond::from_years(100.0, 0.05, 7.5, 2)?;
    /// assert_eq!(b.periods, 15);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Bond::new`], plus [`FinanceError::InvalidInput`] when `years`
    /// is not a positive whole number of periods.
    pub fn from_years(face: f64, coupon_rate: f64, years: f64, frequency: u32) -> Result<Self> {
        finite("years", years)?;
        check_frequency(frequency)?;
        let n = years * f64::from(frequency);
        let whole = n.round();
        if whole < 1.0 || whole > f64::from(MAX_PERIODS) || (n - whole).abs() > 1e-9 * whole {
            return Err(FinanceError::InvalidInput {
                what: "years",
                reason: "must be a positive whole number of coupon periods",
            });
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let periods = whole as u32;
        Self::new(face, coupon_rate, periods, frequency)
    }

    pub(crate) fn check(&self) -> Result<()> {
        finite("face", self.face)?;
        finite("coupon_rate", self.coupon_rate)?;
        if self.face <= 0.0 {
            return Err(FinanceError::InvalidInput {
                what: "face",
                reason: "must be positive",
            });
        }
        if self.coupon_rate < 0.0 {
            return Err(FinanceError::InvalidInput {
                what: "coupon_rate",
                reason: "must be non-negative",
            });
        }
        if self.periods == 0 || self.periods > MAX_PERIODS {
            return Err(FinanceError::InvalidInput {
                what: "periods",
                reason: "must be between 1 and 100000",
            });
        }
        check_frequency(self.frequency)
    }

    /// Coupon paid each period.
    ///
    /// `C = face · coupon_rate / frequency`
    ///
    /// ```rust
    /// use finance_rs::fi::Bond;
    /// assert!((Bond::new(100.0, 0.08, 4, 4)?.coupon()? - 2.0).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Bond::new`] when the fields are invalid.
    pub fn coupon(&self) -> Result<f64> {
        self.check()?;
        output(
            "coupon",
            self.face * self.coupon_rate / f64::from(self.frequency),
        )
    }

    /// Per-period rate `ytm / frequency`, validated `> −1`.
    fn period_rate(&self, ytm: f64) -> Result<f64> {
        self.check()?;
        finite("ytm", ytm)?;
        let i = ytm / f64::from(self.frequency);
        if i <= -1.0 {
            return Err(FinanceError::InvalidInput {
                what: "ytm",
                reason: "ytm / frequency must exceed -1",
            });
        }
        Ok(i)
    }

    /// `(Σ CF_k v^k, Σ k·CF_k v^k, Σ k(k+1)·CF_k v^k)` with `v = 1/(1+i)`.
    fn moments(&self, i: f64) -> (f64, f64, f64) {
        let c = self.face * self.coupon_rate / f64::from(self.frequency);
        let v = 1.0 / (1.0 + i);
        let (mut p, mut s1, mut s2) = (0.0, 0.0, 0.0);
        let mut vk = 1.0;
        for k in 1..=self.periods {
            vk *= v;
            let cf = if k == self.periods { c + self.face } else { c };
            let kf = f64::from(k);
            let pv = cf * vk;
            p += pv;
            s1 += kf * pv;
            s2 += kf * (kf + 1.0) * pv;
        }
        (p, s1, s2)
    }

    /// Price on a coupon date (no accrued interest) at yield `ytm`.
    ///
    /// `P = Σ_{k=1..n} C/(1+y/f)^k + face/(1+y/f)^n`
    ///
    /// ```rust
    /// use finance_rs::fi::Bond;
    /// // 10-year 8% semiannual bond at a 10% yield (Fabozzi): 875.38.
    /// let p = Bond::new(1_000.0, 0.08, 20, 2)?.price(0.10)?;
    /// assert!((p - 875.3779).abs() < 1e-4);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Bond::new`], plus [`FinanceError::NonFinite`] /
    /// [`FinanceError::InvalidInput`] for a non-finite yield or
    /// `ytm / frequency <= −1`, [`FinanceError::Overflow`] on overflow.
    pub fn price(&self, ytm: f64) -> Result<f64> {
        let i = self.period_rate(ytm)?;
        output("price", self.moments(i).0)
    }

    /// Coupon accrued a fraction `accrued_fraction ∈ [0, 1)` of the way
    /// through the current period.
    ///
    /// `AI = C · accrued_fraction`
    ///
    /// ```rust
    /// use finance_rs::fi::Bond;
    /// let ai = Bond::new(100.0, 0.06, 10, 2)?.accrued_interest(0.25)?;
    /// assert!((ai - 0.75).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Bond::new`], plus [`FinanceError::InvalidInput`] for a fraction
    /// outside `[0, 1)`.
    pub fn accrued_interest(&self, accrued_fraction: f64) -> Result<f64> {
        check_fraction(accrued_fraction)?;
        output("accrued_interest", self.coupon()? * accrued_fraction)
    }

    /// Dirty and clean price a fraction `a ∈ [0, 1)` of the way through the
    /// current period, with `periods` coupons still to come (the next in
    /// `1 − a` periods).
    ///
    /// `dirty = P · (1+y/f)^a`, `accrued = C · a`, `clean = dirty − accrued`
    ///
    /// ```rust
    /// use finance_rs::fi::Bond;
    /// let b = Bond::new(100.0, 0.06, 10, 2)?;
    /// let q = b.price_with_accrued(0.06, 0.5)?;
    /// assert!((q.dirty - 100.0 * 1.03f64.sqrt()).abs() < 1e-9);
    /// assert!((q.clean - (q.dirty - 1.5)).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Bond::price`] and [`Bond::accrued_interest`].
    pub fn price_with_accrued(&self, ytm: f64, accrued_fraction: f64) -> Result<BondPrice> {
        check_fraction(accrued_fraction)?;
        let i = self.period_rate(ytm)?;
        let dirty_value = self.moments(i).0 * (1.0 + i).powf(accrued_fraction);
        let dirty = output("dirty", dirty_value)?;
        let accrued = self.accrued_interest(accrued_fraction)?;
        Ok(BondPrice {
            dirty,
            clean: dirty - accrued,
            accrued,
        })
    }

    /// Annual coupon over the (clean) price.
    ///
    /// `current_yield = face · coupon_rate / price`
    ///
    /// ```rust
    /// use finance_rs::fi::Bond;
    /// let cy = Bond::new(1_000.0, 0.08, 20, 2)?.current_yield(875.38)?;
    /// assert!((cy - 80.0 / 875.38).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    ///
    /// # Errors
    /// As [`Bond::new`], plus [`FinanceError::InvalidInput`] for a
    /// non-positive or non-finite price.
    pub fn current_yield(&self, price: f64) -> Result<f64> {
        self.check()?;
        check_price(price)?;
        output("current_yield", self.face * self.coupon_rate / price)
    }
}

fn check_frequency(frequency: u32) -> Result<()> {
    if frequency > 0 {
        Ok(())
    } else {
        Err(FinanceError::InvalidInput {
            what: "frequency",
            reason: "must be positive",
        })
    }
}

fn check_fraction(a: f64) -> Result<()> {
    finite("accrued_fraction", a)?;
    if (0.0..1.0).contains(&a) {
        Ok(())
    } else {
        Err(FinanceError::InvalidInput {
            what: "accrued_fraction",
            reason: "must be in [0, 1)",
        })
    }
}

fn check_price(price: f64) -> Result<()> {
    finite("price", price)?;
    if price > 0.0 {
        Ok(())
    } else {
        Err(FinanceError::InvalidInput {
            what: "price",
            reason: "must be positive",
        })
    }
}

#[cfg(test)]
mod tests;
