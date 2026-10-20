use super::{Date, FinanceError, Result};
use std::fmt;

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::NaiveDate> for Date {
    fn from(d: chrono::NaiveDate) -> Self {
        use chrono::Datelike;
        Self {
            year: d.year(),
            month: d.month(),
            day: d.day(),
        }
    }
}

#[cfg(feature = "chrono")]
impl TryFrom<Date> for chrono::NaiveDate {
    type Error = FinanceError;
    fn try_from(d: Date) -> Result<Self> {
        Self::from_ymd_opt(d.year, d.month, d.day).ok_or(FinanceError::InvalidInput {
            what: "date",
            reason: "outside chrono's range",
        })
    }
}
