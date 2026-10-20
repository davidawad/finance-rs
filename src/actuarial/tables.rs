//! IRS Publication 590-B (2025) appendix B life expectancy factors.
//!
//! The single-life, uniform-lifetime, and joint-life values are IRS regulatory
//! data. SOA mortality rates are exposed as a source-attributed dataset.

use std::sync::OnceLock;

/// Age-indexed IRS factors, with the first value corresponding to age 0 or 72.
///
/// ```
/// let factors: finance_rs::actuarial::tables::AgeFactors =
///     finance_rs::actuarial::tables::irs_uniform_lifetime();
/// assert_eq!(factors[0], 27.4);
/// ```
pub type AgeFactors = &'static [f64];
/// IRS Table I, Single Life Expectancy, age 0 through 120+ (Pub. 590-B, 2025).
///
/// ```
/// let factors = finance_rs::actuarial::tables::irs_single_life();
/// assert_eq!(factors[72], 17.2);
/// ```
#[must_use]
pub fn irs_single_life() -> AgeFactors {
    &[
        84.6, 83.7, 82.8, 81.8, 80.8, 79.8, 78.8, 77.9, 76.9, 75.9, 74.9, 73.9, 72.9, 71.9, 70.9,
        69.9, 69.0, 68.0, 67.0, 66.0, 65.0, 64.1, 63.1, 62.1, 61.1, 60.2, 59.2, 58.2, 57.3, 56.3,
        55.3, 54.4, 53.4, 52.5, 51.5, 50.5, 49.6, 48.6, 47.7, 46.7, 45.7, 44.8, 43.8, 42.9, 41.9,
        41.0, 40.0, 39.0, 38.1, 37.1, 36.2, 35.3, 34.3, 33.4, 32.5, 31.6, 30.6, 29.8, 28.9, 28.0,
        27.1, 26.2, 25.4, 24.5, 23.7, 22.9, 22.0, 21.2, 20.4, 19.6, 18.8, 18.0, 17.2, 16.4, 15.6,
        14.8, 14.1, 13.3, 12.6, 11.9, 11.2, 10.5, 9.9, 9.3, 8.7, 8.1, 7.6, 7.1, 6.6, 6.1, 5.7, 5.3,
        4.9, 4.6, 4.3, 4.0, 3.7, 3.4, 3.2, 3.0, 2.8, 2.6, 2.5, 2.3, 2.2, 2.1, 2.1, 2.1, 2.0, 2.0,
        2.0, 2.0, 2.0, 2.0, 1.9, 1.9, 1.8, 1.8, 1.6, 1.4, 1.1, 1.0,
    ]
}
/// IRS Table III, Uniform Lifetime, ages 72 through 120 (Pub. 590-B, 2025).
///
/// ```
/// let factors = finance_rs::actuarial::tables::irs_uniform_lifetime();
/// assert_eq!(factors[0], 27.4);
/// ```
#[must_use]
pub fn irs_uniform_lifetime() -> AgeFactors {
    &[
        27.4, 26.5, 25.5, 24.6, 23.7, 22.9, 22.0, 21.1, 20.2, 19.4, 18.5, 17.7, 16.8, 16.0, 15.2,
        14.4, 13.7, 12.9, 12.2, 11.5, 10.8, 10.1, 9.5, 8.9, 8.4, 7.8, 7.3, 6.8, 6.4, 6.0, 5.6, 5.2,
        4.9, 4.6, 4.3, 4.1, 3.9, 3.7, 3.5, 3.4, 3.3, 3.1, 3.0, 2.9, 2.8, 2.7, 2.5, 2.3, 2.0,
    ]
}
/// IRS Table II (Joint and Last Survivor) data supplied as a row-major matrix.
///
/// ```
/// use finance_rs::actuarial::tables::JointLastSurvivor;
/// let table = JointLastSurvivor::new(2, 2, vec![72.0, 71.5, 71.5, 71.0])?;
/// assert_eq!(table.factor(1, 1), Some(71.0));
/// # Ok::<(), &'static str>(())
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct JointLastSurvivor {
    owner_ages: u8,
    beneficiary_ages: u8,
    values: Vec<f64>,
}
impl JointLastSurvivor {
    /// Construct from the published matrix, with row and column counts.
    ///
    /// ```
    /// let table = finance_rs::actuarial::tables::JointLastSurvivor::new(1, 1, vec![70.0])?;
    /// assert_eq!(table.factor(0, 0), Some(70.0));
    /// # Ok::<(), &'static str>(())
    /// ```
    /// # Errors
    /// Returns an error when inputs are invalid or the calculation cannot be completed.
    pub fn new(
        owner_ages: u8,
        beneficiary_ages: u8,
        values: Vec<f64>,
    ) -> Result<Self, &'static str> {
        let expected = usize::from(owner_ages) * usize::from(beneficiary_ages);
        if owner_ages == 0
            || beneficiary_ages == 0
            || values.len() != expected
            || values.iter().any(|v| !v.is_finite() || *v <= 0.0)
        {
            return Err("joint lifetime matrix dimensions or factors are invalid");
        }
        Ok(Self {
            owner_ages,
            beneficiary_ages,
            values,
        })
    }
    /// Read a factor by zero-based owner and beneficiary age offset.
    ///
    /// ```
    /// use finance_rs::actuarial::tables::JointLastSurvivor;
    /// let table = JointLastSurvivor::new(1, 1, vec![70.0])?;
    /// assert_eq!(table.factor(0, 0), Some(70.0));
    /// # Ok::<(), &'static str>(())
    /// ```
    #[must_use]
    pub fn factor(&self, owner_age: u8, beneficiary_age: u8) -> Option<f64> {
        if owner_age >= self.owner_ages || beneficiary_age >= self.beneficiary_ages {
            return None;
        }
        self.values
            .get(
                usize::from(owner_age) * usize::from(self.beneficiary_ages)
                    + usize::from(beneficiary_age),
            )
            .copied()
    }
}
/// Build a joint-life table from explicitly supplied IRS-published matrix data.
///
/// ```
/// let table = finance_rs::actuarial::tables::irs_joint_last_survivor(1, 1, vec![70.0])?;
/// assert_eq!(table.factor(0, 0), Some(70.0));
/// # Ok::<(), &'static str>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn irs_joint_last_survivor(
    owner_ages: u8,
    beneficiary_ages: u8,
    values: Vec<f64>,
) -> Result<JointLastSurvivor, &'static str> {
    JointLastSurvivor::new(owner_ages, beneficiary_ages, values)
}
/// 2025 IRS Table II, with owner and spouse ages 20 through 120.
///
/// The publication omits cells above the diagonal; joint-and-last-survivor
/// factors are symmetric, so those lookups use the published transposed cell.
/// Source: <https://www.irs.gov/publications/p590b>, Appendix B, Table II.
///
/// ```
/// let table = finance_rs::actuarial::tables::irs_joint_last_survivor_2025();
/// assert_eq!(table.factor(20, 20), Some(72.0));
/// ```
#[derive(Debug)]
pub struct IrsJointLastSurvivor2025 {
    values: Vec<(u8, u8, f64)>,
}
impl IrsJointLastSurvivor2025 {
    /// Look up a published divisor by attained owner and spouse ages.
    ///
    /// ```
    /// let table = finance_rs::actuarial::tables::irs_joint_last_survivor_2025();
    /// assert_eq!(table.factor(20, 20), Some(72.0));
    /// ```
    #[must_use]
    pub fn factor(&self, owner_age: u8, spouse_age: u8) -> Option<f64> {
        self.lookup(owner_age, spouse_age)
            .or_else(|| self.lookup(spouse_age, owner_age))
    }
    fn lookup(&self, owner_age: u8, spouse_age: u8) -> Option<f64> {
        self.values
            .binary_search_by_key(&(owner_age, spouse_age), |(owner, spouse, _)| {
                (*owner, *spouse)
            })
            .ok()
            .and_then(|index| self.values.get(index).map(|(_, _, value)| *value))
    }
    /// Number of published matrix cells.
    ///
    /// ```
    /// let table = finance_rs::actuarial::tables::irs_joint_last_survivor_2025();
    /// assert_eq!(table.len(), 10_201);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        101 * 101
    }
    /// Whether the table has no published cells.
    ///
    /// ```
    /// let table = finance_rs::actuarial::tables::irs_joint_last_survivor_2025();
    /// assert!(!table.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}
/// Bundled IRS Publication 590-B (2025) Table II matrix.
///
/// ```
/// assert!(!finance_rs::actuarial::tables::irs_joint_last_survivor_2025().is_empty());
/// ```
pub fn irs_joint_last_survivor_2025() -> &'static IrsJointLastSurvivor2025 {
    static TABLE: OnceLock<IrsJointLastSurvivor2025> = OnceLock::new();
    TABLE.get_or_init(|| {
        let values = parse_joint_table(include_str!("data/irs-joint-last-survivor-2025.csv"));
        IrsJointLastSurvivor2025 { values }
    })
}

fn parse_joint_table(text: &str) -> Vec<(u8, u8, f64)> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split(',');
            let owner = fields.next()?.parse().ok()?;
            let spouse = fields.next()?.parse().ok()?;
            let factor = fields.next()?.parse().ok()?;
            Some((owner, spouse, factor))
        })
        .collect()
}
/// Sex basis for the SOA 2012 Individual Annuity Mortality Basic table.
///
/// ```
/// let sex = finance_rs::actuarial::tables::IamSex::Female;
/// let table = finance_rs::actuarial::tables::soa_2012_iam(sex)?;
/// assert_eq!(table.age(), 0);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IamSex {
    /// Male.
    Male,
    /// Female.
    Female,
}
/// Return the SOA 2012 IAM Basic qx table, ages 0 through 120, on ANB basis.
///
/// Source: Society of Actuaries Mortality and Other Rate Tables,
/// <https://mort.soa.org/ViewTable.aspx?TableIdentity=2581> (male) and
/// <https://mort.soa.org/ViewTable.aspx?TableIdentity=2582> (female).
/// Values are annual probabilities.
///
/// ```
/// use finance_rs::actuarial::tables::{soa_2012_iam, IamSex};
/// let mortality = soa_2012_iam(IamSex::Male)?;
/// assert_eq!(mortality.qx().len(), 121);
/// # Ok::<(), finance_rs::FinanceError>(())
/// ```
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
pub fn soa_2012_iam(sex: IamSex) -> crate::Result<super::life::LifeTable> {
    let column = match sex {
        IamSex::Male => 1,
        IamSex::Female => 2,
    };
    let qx = include_str!("data/soa-2012-iam-basic.csv")
        .lines()
        .filter_map(|line| line.split(',').nth(column)?.parse::<f64>().ok())
        .collect();
    super::life::LifeTable::new(0, qx)
}
/// SOA table data citation for attribution in generated reports.
///
/// ```
/// assert_eq!(finance_rs::actuarial::tables::SOA_2012_IAM_BASIC_VINTAGE, "SOA 2012 IAM Basic");
/// ```
pub const SOA_2012_IAM_BASIC_VINTAGE: &str = "SOA 2012 IAM Basic";

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    #[test]
    fn irs_factors_match_publication_2025() {
        assert_eq!(irs_single_life()[0], 84.6);
        assert_eq!(irs_single_life()[72], 17.2);
        assert_eq!(irs_uniform_lifetime()[0], 27.4);
        assert_eq!(irs_uniform_lifetime()[1], 26.5);
        assert_eq!(irs_uniform_lifetime()[48], 2.0);
        assert_eq!(
            JointLastSurvivor::new(2, 2, vec![72.0, 71.5, 71.5, 71.0])
                .unwrap()
                .factor(1, 1),
            Some(71.0)
        );

        let joint = irs_joint_last_survivor_2025();
        assert_eq!(joint.len(), 10_201);
        assert!(!joint.is_empty());
        assert_eq!(joint.factor(20, 20), Some(72.0));
        assert_eq!(joint.factor(72, 60), Some(28.8));
        assert_eq!(joint.factor(20, 121), None);
        assert_eq!(joint.factor(20, 30), joint.factor(30, 20));
        let male = soa_2012_iam(IamSex::Male).unwrap();
        let female = soa_2012_iam(IamSex::Female).unwrap();
        assert_eq!(male.qx().len(), 121);
        assert_eq!(male.qx()[0], 0.001_783);
        assert_eq!(female.qx()[0], 0.001_801);
        assert_eq!(male.qx()[120], 0.4);
        assert_eq!(
            parse_joint_table("owner,spouse,factor\n20,21,70\n22,bad,68\n23,24\n"),
            vec![(20, 21, 70.0)]
        );
        assert!(JointLastSurvivor::new(1, 1, vec![1.0])
            .unwrap()
            .factor(1, 0)
            .is_none());
        assert!(IrsJointLastSurvivor2025 { values: vec![] }.is_empty());
    }

    proptest! {
        #[test]
        fn irs_joint_table_is_symmetric_and_finite(owner in 20_u8..=120, spouse in 20_u8..=120) {
            let table = irs_joint_last_survivor_2025();
            let direct = table.factor(owner, spouse);
            prop_assert_eq!(direct, table.factor(spouse, owner));
            prop_assert!(direct.is_some_and(|factor| factor.is_finite() && factor > 0.0));
        }

        #[test]
        fn soa_iam_table_contains_valid_annual_probabilities(age in 0_usize..=120) {
            for sex in [IamSex::Male, IamSex::Female] {
                let qx = soa_2012_iam(sex).unwrap().qx()[age];
                prop_assert!(qx.is_finite() && (0.0..=1.0).contains(&qx));
            }
        }

    }
}
