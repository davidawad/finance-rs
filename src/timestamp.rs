//! ISO-8601 timestamps and elapsed calendar days, including sub-day fractions.
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime};

/// An ISO timestamp with its original timezone awareness preserved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Timestamp {
    /// A timestamp carrying an explicit UTC offset.
    Offset(DateTime<FixedOffset>),
    /// A local timestamp without an offset.
    Naive(NaiveDateTime),
}

/// Parse the ISO-8601 subset accepted by the prediction metric's Python
/// predecessor. A trailing `Z` is treated as `+00:00`.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
/// # Panics
/// This cannot panic for a valid calendar date because midnight is always valid.
#[allow(clippy::expect_used)] // Every chrono NaiveDate has a valid midnight.
pub fn parse_iso(raw: &str) -> Result<Timestamp, String> {
    let s = raw.replace('Z', "+00:00");
    for format in [
        "%Y-%m-%dT%H:%M:%S%.f%:z",
        "%Y-%m-%d %H:%M:%S%.f%:z",
        "%Y-%m-%dT%H:%M%:z",
        "%Y-%m-%d %H:%M%:z",
    ] {
        if let Ok(value) = DateTime::parse_from_str(&s, format) {
            return Ok(Timestamp::Offset(value));
        }
    }
    for format in [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
    ] {
        if let Ok(value) = NaiveDateTime::parse_from_str(&s, format) {
            return Ok(Timestamp::Naive(value));
        }
    }
    if let Ok(value) = NaiveDate::parse_from_str(&s, "%Y-%m-%d") {
        // Every valid calendar date has a valid midnight.
        let midnight = value.and_hms_opt(0, 0, 0).expect("midnight is valid");
        return Ok(Timestamp::Naive(midnight));
    }
    Err(format!(
        "Invalid isoformat string: {}",
        python_string_repr(raw)
    ))
}

fn python_string_repr(value: &str) -> String {
    if value.contains('\'') && !value.contains('"') {
        format!("\"{value}\"")
    } else {
        format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
    }
}

/// Return elapsed fractional days, rejecting mixed naive and offset timestamps.
/// # Errors
/// Returns an error when inputs are invalid or the calculation cannot be completed.
#[allow(clippy::cast_precision_loss)] // Financial algorithms intentionally use f64 for bounded sample counts and indices.
pub fn elapsed_days_between(as_of: &str, expires_at: &str) -> Result<f64, String> {
    let expiry = parse_iso(expires_at)?;
    let now = parse_iso(as_of)?;
    let delta = match (expiry, now) {
        (Timestamp::Offset(end), Timestamp::Offset(start)) => end.signed_duration_since(start),
        (Timestamp::Naive(end), Timestamp::Naive(start)) => end.signed_duration_since(start),
        _ => return Err("can't subtract offset-naive and offset-aware datetimes".into()),
    };
    let micros = delta.num_microseconds().ok_or("time span overflows")?;
    Ok(micros as f64 / 1e6 / 86400.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn preserves_subday_and_offset_semantics() {
        assert_eq!(
            elapsed_days_between("2024-01-01T00:00:00Z", "2024-01-01T12:00:00Z"),
            Ok(0.5)
        );
        assert_eq!(
            elapsed_days_between("2024-01-01T00:00:00+02:00", "2024-01-01T00:00:00Z"),
            Ok(1.0 / 12.0)
        );
        assert_eq!(
            elapsed_days_between("2024-01-01", "2024-01-01T12:00:00"),
            Ok(0.5)
        );
        assert_eq!(
            elapsed_days_between("2024-01-01", "2024-01-01T12:00:00Z"),
            Err("can't subtract offset-naive and offset-aware datetimes".into())
        );
    }

    proptest! {
        #[test]
        fn arbitrary_inputs_never_panic(a in any::<String>(), b in any::<String>()) {
            let _ = elapsed_days_between(&a, &b);
        }
    }
}
