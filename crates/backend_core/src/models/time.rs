//! Jiff-based timestamp and date parsing and the only place stored timestamp strings are formatted.
//! All values are normalized to and formatted in UTC.

use crate::error::{Result, TrackerError};
use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use jiff::Timestamp;

const LOCAL_DATETIME_FORMATS: &[&str] = &[
    "%Y-%m-%dT%H:%M:%S%.f",
    "%Y-%m-%d %H:%M:%S%.f",
    "%Y-%m-%dT%H:%M:%S",
    "%Y-%m-%d %H:%M:%S",
];

/// Parses an RFC 3339 timestamp string, local TOML datetime, or `YYYY-MM-DD` date fallback (at 00:00:00Z) into `Timestamp`.
pub fn parse_timestamp_utc(value: &str) -> Result<Timestamp> {
    let trimmed = value.trim();
    if let Ok(ts) = trimmed.parse::<Timestamp>() {
        return Ok(ts);
    }
    for fmt in LOCAL_DATETIME_FORMATS {
        if let Ok(dt) = DateTime::strptime(fmt, trimmed) {
            if let Ok(ts) = TimeZone::UTC.to_timestamp(dt) {
                return Ok(ts);
            }
        }
    }
    if let Ok(date) = Date::strptime("%Y-%m-%d", trimmed) {
        if let Ok(ts) = TimeZone::UTC.to_timestamp(date.to_datetime(jiff::civil::Time::midnight()))
        {
            return Ok(ts);
        }
    }
    Err(TrackerError::InvalidDateTime {
        value: value.to_string(),
        reason: "expected RFC 3339 timestamp (e.g. 2026-11-01T21:00:00Z), local datetime, or YYYY-MM-DD date"
            .to_string(),
    })
}

/// Parses a `YYYY-MM-DD` date string (or extracts the UTC date portion of a timestamp) into `Date`.
pub fn parse_date(value: &str) -> Result<Date> {
    let trimmed = value.trim();
    if let Ok(date) = Date::strptime("%Y-%m-%d", trimmed) {
        return Ok(date);
    }
    if let Ok(ts) = trimmed.parse::<Timestamp>() {
        return Ok(ts.to_zoned(TimeZone::UTC).date());
    }
    for fmt in LOCAL_DATETIME_FORMATS {
        if let Ok(dt) = DateTime::strptime(fmt, trimmed) {
            return Ok(dt.date());
        }
    }
    Err(TrackerError::InvalidDateTime {
        value: value.to_string(),
        reason: "expected YYYY-MM-DD date or RFC 3339 / ISO 8601 timestamp".to_string(),
    })
}

/// Formats a `Timestamp` in the on-disk timestamp format (whole seconds with a `Z` suffix).
/// Never use `Timestamp`'s `Display` implementation for stored values (it prints fractional seconds).
pub fn format_timestamp(ts: Timestamp) -> String {
    ts.strftime("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Returns the current UTC timestamp formatted in the on-disk `YYYY-MM-DDTHH:MM:SSZ` format.
pub fn now_timestamp_string() -> String {
    format_timestamp(Timestamp::now())
}

/// Returns today's UTC date formatted as `YYYY-MM-DD`.
pub fn today_date_string() -> String {
    Timestamp::now().strftime("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_timestamp_utc_handles_rfc3339_z_and_offset() {
        let expected = "2026-11-01T21:00:00Z".parse::<Timestamp>().unwrap();
        assert_eq!(
            parse_timestamp_utc("2026-11-01T21:00:00Z").unwrap(),
            expected
        );
        assert_eq!(
            parse_timestamp_utc("2026-11-01T23:00:00+02:00").unwrap(),
            expected
        );
    }

    #[test]
    fn parse_timestamp_utc_handles_local_forms_and_date_fallback() {
        let expected_21 = "2026-11-01T21:00:00Z".parse::<Timestamp>().unwrap();
        assert_eq!(
            parse_timestamp_utc("2026-11-01T21:00:00").unwrap(),
            expected_21
        );
        assert_eq!(
            parse_timestamp_utc("2026-11-01 21:00:00").unwrap(),
            expected_21
        );

        let frac = parse_timestamp_utc("2026-11-01T21:00:00.5").unwrap();
        assert_eq!(format_timestamp(frac), "2026-11-01T21:00:00Z");

        let expected_midnight = "2026-11-01T00:00:00Z".parse::<Timestamp>().unwrap();
        assert_eq!(
            parse_timestamp_utc("2026-11-01").unwrap(),
            expected_midnight
        );
    }

    #[test]
    fn parse_date_handles_date_local_datetime_and_utc_converted_offset() {
        let expected = jiff::civil::date(2026, 11, 1);
        assert_eq!(parse_date("2026-11-01").unwrap(), expected);
        assert_eq!(parse_date("2026-11-01T21:00:00").unwrap(), expected);
        assert_eq!(parse_date("2026-11-02T01:00:00+02:00").unwrap(), expected);
    }

    #[test]
    fn invalid_inputs_return_invalid_datetime_error() {
        assert!(matches!(
            parse_timestamp_utc("not-a-timestamp"),
            Err(TrackerError::InvalidDateTime { .. })
        ));
        assert!(matches!(
            parse_date("not-a-date"),
            Err(TrackerError::InvalidDateTime { .. })
        ));
    }

    #[test]
    fn format_and_now_helpers_produce_canonical_strings() {
        let ts: Timestamp = "2026-11-01T21:00:00.123456Z".parse().unwrap();
        assert_eq!(format_timestamp(ts), "2026-11-01T21:00:00Z");

        let now_str = now_timestamp_string();
        assert_eq!(now_str.len(), 20);
        assert!(now_str.ends_with('Z'));
        assert!(!now_str.contains('.'));
        let parsed_now = parse_timestamp_utc(&now_str).unwrap();
        assert_eq!(format_timestamp(parsed_now), now_str);

        let today_str = today_date_string();
        assert_eq!(today_str.len(), 10);
        assert!(parse_date(&today_str).is_ok());
    }
}
