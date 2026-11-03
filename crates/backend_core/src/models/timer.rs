use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::models::{deserialize_datetime_string, parse_timestamp_utc};

/// Active session timer state persisted at `.tracker/.timer.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveTimer {
    pub item_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
    #[serde(deserialize_with = "deserialize_datetime_string")]
    pub started_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl ActiveTimer {
    /// Creates a new `ActiveTimer` starting at `started_at`.
    pub fn new(
        item_id: impl Into<String>,
        phase_id: Option<String>,
        scope_id: Option<String>,
        started_at: impl Into<String>,
    ) -> Self {
        Self {
            item_id: item_id.into(),
            phase_id,
            scope_id,
            started_at: started_at.into(),
            note: None,
        }
    }

    /// Deserializes an `ActiveTimer` from a TOML string.
    pub fn from_toml_str(input: &str) -> Result<Self> {
        Ok(toml::from_str(input)?)
    }

    /// Serializes this `ActiveTimer` into a formatted TOML string.
    pub fn to_toml_string(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Parses `started_at` into `DateTime<Utc>`.
    pub fn parsed_started_at(&self) -> Result<DateTime<Utc>> {
        parse_timestamp_utc(&self.started_at)
    }

    /// Calculates elapsed whole minutes between `started_at` and `now` (clamped to `>= 0`).
    pub fn elapsed_minutes_at(&self, now: DateTime<Utc>) -> Result<u32> {
        let start = self.parsed_started_at()?;
        let seconds = (now - start).num_seconds().max(0);
        // Round to nearest minute or floor; whole elapsed minutes:
        Ok((seconds / 60) as u32)
    }
}
