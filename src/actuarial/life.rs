//! Life-contingency functions on caller-supplied qx rates (Bowers convention).
use crate::{FinanceError, Result};

/// Annual mortality probabilities, with `qx[0]` applying at `age`.
///
/// ```
/// let table = finance_rs::actuarial::life::LifeTable::new(40, vec![0.1, 0.2])?;
/// assert_eq!(table.age(), 40);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct LifeTable {
    age: u32,
    qx: Vec<f64>,
}
impl LifeTable {
    /// Construct a table after checking every one-year death probability.
    ///
    /// ```
    /// let table = finance_rs::actuarial::life::LifeTable::new(40, vec![0.1, 0.2])?;
    /// assert_eq!(table.qx(), &[0.1, 0.2]);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn new(age: u32, qx: Vec<f64>) -> Result<Self> {
        if qx.is_empty() {
            return Err(FinanceError::TooShort {
                what: "qx",
                min: 1,
                got: 0,
            });
        }
        if qx.iter().any(|q| !q.is_finite()) {
            return Err(FinanceError::NonFinite { what: "qx" });
        }
        if qx.iter().any(|q| !(0.0..=1.0).contains(q)) {
            return Err(FinanceError::InvalidInput {
                what: "qx",
                reason: "each probability must be between zero and one",
            });
        }
        Ok(Self { age, qx })
    }
    /// Returns the age value.
    ///
    /// ```
    /// let table = finance_rs::actuarial::life::LifeTable::new(40, vec![0.1])?;
    /// assert_eq!(table.age(), 40);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn age(&self) -> u32 {
        self.age
    }
    /// Returns the qx value.
    ///
    /// ```
    /// let table = finance_rs::actuarial::life::LifeTable::new(40, vec![0.1, 0.2])?;
    /// assert_eq!(table.qx(), &[0.1, 0.2]);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn qx(&self) -> &[f64] {
        &self.qx
    }
    /// Number surviving to each integer age, starting with lx=1 at issue age.
    ///
    /// ```
    /// let table = finance_rs::actuarial::life::LifeTable::new(40, vec![0.1, 0.2])?;
    /// let lx = table.lx();
    /// assert_eq!((lx[0], lx[1]), (1.0, 0.9));
    /// assert!((lx[2] - 0.72).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    #[must_use]
    pub fn lx(&self) -> Vec<f64> {
        let mut out = Vec::with_capacity(self.qx.len() + 1);
        let mut l = 1.0;
        out.push(l);
        for q in &self.qx {
            l *= 1.0 - q;
            out.push(l);
        }
        out
    }
    /// One-year survival probability at attained age.
    ///
    /// ```
    /// let table = finance_rs::actuarial::life::LifeTable::new(40, vec![0.1])?;
    /// assert!((table.px(40)? - 0.9).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn px(&self, attained_age: u32) -> Result<f64> {
        Ok(1.0 - self.q(attained_age)?)
    }
    /// t-year survival probability from age x.
    ///
    /// ```
    /// let table = finance_rs::actuarial::life::LifeTable::new(40, vec![0.1, 0.2])?;
    /// assert!((table.tpx(40, 2)? - 0.72).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn tpx(&self, x: u32, t: u32) -> Result<f64> {
        let mut p = 1.0;
        for k in 0..t {
            p *= 1.0
                - self.q(x.checked_add(k).ok_or(FinanceError::InvalidInput {
                    what: "age",
                    reason: "age overflow",
                })?)?;
        }
        Ok(p)
    }
    /// t-year death probability from age x.
    ///
    /// ```
    /// let table = finance_rs::actuarial::life::LifeTable::new(40, vec![0.1, 0.2])?;
    /// assert!((table.tqx(40, 2)? - 0.28).abs() < 1e-12);
    /// # Ok::<(), finance_rs::FinanceError>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn tqx(&self, x: u32, t: u32) -> Result<f64> {
        Ok(1.0 - self.tpx(x, t)?)
    }
    pub(crate) fn q(&self, attained: u32) -> Result<f64> {
        let i = attained
            .checked_sub(self.age)
            .ok_or(FinanceError::InvalidInput {
                what: "age",
                reason: "age precedes the mortality table",
            })? as usize;
        self.qx.get(i).copied().ok_or(FinanceError::InvalidInput {
            what: "age",
            reason: "age is outside the mortality table",
        })
    }
    fn survival_to(&self, x: u32, k: u32) -> Result<f64> {
        self.tpx(x, k)
    }
}
fn rate_ok(rate: f64) -> Result<()> {
    if rate.is_finite() && rate > -1.0 {
        Ok(())
    } else {
        Err(FinanceError::InvalidInput {
            what: "rate",
            reason: "must be finite and greater than -1",
        })
    }
}
/// Curtate life expectancy `e_x = Σ _kp_x`.
///
/// ```
/// use finance_rs::actuarial::life::{expectancy, LifeTable};
/// let table = LifeTable::new(40, vec![0.1, 0.2])?;
/// assert!((expectancy(&table, 40)? - 1.62).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_possible_truncation)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
pub fn expectancy(table: &LifeTable, x: u32) -> Result<f64> {
    let n = table.qx.len() as u32;
    let mut s = 0.0;
    for k in 1..=n {
        s += table.survival_to(x, k)?;
    }
    Ok(s)
}
/// Whole-life annuity-due `ä_x = Σ v^k _kp_x`.
///
/// ```
/// use finance_rs::actuarial::life::{annuity_due, LifeTable};
/// let table = LifeTable::new(40, vec![0.1, 0.2])?;
/// assert!(annuity_due(&table, 40, 0.03)? > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
pub fn annuity_due(table: &LifeTable, x: u32, rate: f64) -> Result<f64> {
    rate_ok(rate)?;
    let n = table.qx.len() as u32;
    let mut s = 0.0;
    for k in 0..n {
        s += table.survival_to(x, k)? / (1.0 + rate).powi(k as i32);
    }
    Ok(s)
}
/// n-year temporary life annuity-due.
///
/// ```
/// use finance_rs::actuarial::life::{temporary_annuity, LifeTable};
/// let table = LifeTable::new(40, vec![0.1, 0.2])?;
/// assert!((temporary_annuity(&table, 40, 2, 0.0)? - 1.9).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_possible_wrap)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
pub fn temporary_annuity(table: &LifeTable, x: u32, n: u32, rate: f64) -> Result<f64> {
    rate_ok(rate)?;
    let mut s = 0.0;
    for k in 0..n {
        s += table.survival_to(x, k)? / (1.0 + rate).powi(k as i32);
    }
    Ok(s)
}
/// Whole-life insurance payable at the end of year of death.
///
/// ```
/// use finance_rs::actuarial::life::{whole_life_insurance, LifeTable};
/// let table = LifeTable::new(40, vec![0.1, 0.2])?;
/// assert!((whole_life_insurance(&table, 40, 0.0)? - 0.28).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
pub fn whole_life_insurance(table: &LifeTable, x: u32, rate: f64) -> Result<f64> {
    rate_ok(rate)?;
    let n = table.qx.len() as u32;
    let mut s = 0.0;
    for k in 0..n {
        s += table.survival_to(x, k)? * table.q(x + k)? / (1.0 + rate).powi((k + 1) as i32);
    }
    Ok(s)
}
/// n-year term insurance payable at the end of year of death.
///
/// ```
/// use finance_rs::actuarial::life::{term_insurance, LifeTable};
/// let table = LifeTable::new(40, vec![0.1, 0.2])?;
/// assert!(term_insurance(&table, 40, 2, 0.03)? > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_possible_wrap)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
pub fn term_insurance(table: &LifeTable, x: u32, n: u32, rate: f64) -> Result<f64> {
    rate_ok(rate)?;
    let mut s = 0.0;
    for k in 0..n {
        s += table.survival_to(x, k)? * table.q(x + k)? / (1.0 + rate).powi((k + 1) as i32);
    }
    Ok(s)
}
/// n-year endowment insurance (term benefit plus maturity benefit).
///
/// ```
/// use finance_rs::actuarial::life::{endowment, LifeTable};
/// let table = LifeTable::new(40, vec![0.1, 0.2])?;
/// assert!((endowment(&table, 40, 2, 0.0)? - 1.0).abs() < 1e-12);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_possible_wrap)] // Inputs are range-checked by this algorithm before narrowing to its domain type.
pub fn endowment(table: &LifeTable, x: u32, n: u32, rate: f64) -> Result<f64> {
    rate_ok(rate)?;
    Ok(term_insurance(table, x, n, rate)? + table.survival_to(x, n)? / (1.0 + rate).powi(n as i32))
}
/// Net annual level premium for a unit whole-life benefit.
///
/// ```
/// use finance_rs::actuarial::life::{net_level_premium, LifeTable};
/// let table = LifeTable::new(40, vec![0.1, 0.2])?;
/// assert!(net_level_premium(&table, 40, 0.0)? > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn net_level_premium(table: &LifeTable, x: u32, rate: f64) -> Result<f64> {
    let a = annuity_due(table, x, rate)?;
    // The k=0 term is one, so a valid nonempty mortality table has a > 0.
    Ok(whole_life_insurance(table, x, rate)? / a)
}
/// Prospective reserve under the equivalence principle after t years.
///
/// ```
/// use finance_rs::actuarial::life::{reserve_prospective, LifeTable};
/// let table = LifeTable::new(40, vec![0.1, 0.2])?;
/// let reserve = reserve_prospective(&table, 40, 0, 0.03)?;
/// assert!(reserve.is_finite());
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn reserve_prospective(table: &LifeTable, x: u32, t: u32, rate: f64) -> Result<f64> {
    let attained = x.checked_add(t).ok_or(FinanceError::InvalidInput {
        what: "age",
        reason: "age overflow",
    })?;
    let premium = net_level_premium(table, x, rate)?;
    let a = annuity_due(table, attained, rate)?;
    let insurance = whole_life_insurance(table, attained, rate)?;
    Ok(insurance - premium * a)
}
/// Level immediate annuity payout, reduced by an explicit proportional load.
///
/// ```
/// use finance_rs::actuarial::life::{spia_payout, LifeTable};
/// let table = LifeTable::new(65, vec![0.1, 0.2, 0.3])?;
/// assert!(spia_payout(100_000.0, &table, 65, 0.03, 0.02)? > 0.0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn spia_payout(premium: f64, table: &LifeTable, x: u32, rate: f64, load: f64) -> Result<f64> {
    if !premium.is_finite() || premium < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "premium",
            reason: "must be finite and nonnegative",
        });
    }
    if !load.is_finite() || load < 0.0 {
        return Err(FinanceError::InvalidInput {
            what: "load",
            reason: "must be finite and nonnegative",
        });
    }
    let factor = annuity_due(table, x, rate)? * (1.0 + load);
    // The k=0 annuity term is one and load is constrained to be nonnegative.
    Ok(premium / factor)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn textbook_survival_and_values() {
        let t = LifeTable::new(40, vec![0.1, 0.2]).unwrap();
        assert!((t.lx()[2] - 0.72).abs() < 1e-12);
        assert!((t.tpx(40, 2).unwrap() - 0.72).abs() < 1e-12);
        assert!((temporary_annuity(&t, 40, 2, 0.0).unwrap() - 1.9).abs() < 1e-12);
        assert!((whole_life_insurance(&t, 40, 0.0).unwrap() - 0.28).abs() < 1e-12);
        assert!((endowment(&t, 40, 2, 0.0).unwrap() - 1.0).abs() < 1e-12);
        assert!((net_level_premium(&t, 40, 0.0).unwrap() - 0.28 / 1.9).abs() < 1e-12);
        assert!(expectancy(&t, 41).is_err());
        assert!(t.px(100).is_err());
    }

    #[test]
    fn life_table_contracts_and_payouts_cover_remaining_paths() {
        let t = LifeTable::new(40, vec![0.0, 0.5, 1.0]).unwrap();
        assert_eq!(t.age(), 40);
        assert_eq!(t.qx(), &[0.0, 0.5, 1.0]);
        assert_eq!(t.px(40).unwrap(), 1.0);
        assert_eq!(t.tqx(40, 2).unwrap(), 0.5);
        let boundary = LifeTable::new(u32::MAX, vec![0.0, 0.0]).unwrap();
        assert!(matches!(
            boundary.tpx(u32::MAX, 2),
            Err(FinanceError::InvalidInput { what: "age", .. })
        ));
        assert!(LifeTable::new(40, vec![]).is_err());
        assert!(LifeTable::new(40, vec![f64::NAN]).is_err());
        assert!(LifeTable::new(40, vec![-0.1]).is_err());
        assert!(LifeTable::new(40, vec![1.1]).is_err());
        assert!(t.px(39).is_err());
        assert!(annuity_due(&t, 40, f64::NAN).is_err());
        assert!(temporary_annuity(&t, 40, 1, -1.0).is_err());
        assert!(whole_life_insurance(&t, 40, -1.0).is_err());
        assert!(term_insurance(&t, 40, 1, -1.0).is_err());
        assert!(endowment(&t, 40, 1, -1.0).is_err());
        assert!(net_level_premium(&t, 40, -1.0).is_err());
        assert!(reserve_prospective(&t, u32::MAX, 1, 0.0).is_err());
        assert!(reserve_prospective(&t, 40, 0, 0.0).is_ok());
        assert!(spia_payout(-1.0, &t, 40, 0.0, 0.0).is_err());
        assert!(spia_payout(1.0, &t, 40, 0.0, -0.1).is_err());
        assert!(spia_payout(1.0, &t, 40, -1.0, 0.0).is_err());
        assert!(spia_payout(100.0, &t, 40, 0.0, 0.1).unwrap() > 0.0);
    }
}
