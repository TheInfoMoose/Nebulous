use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};
use crate::models::scope::ItemStub;
use crate::models::{deserialize_datetime_string, parse_date, parse_timestamp_utc};

/// Tier 4: Authoritative atomic item manifest (`.../items/<ITEM_ID>/item.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemManifest {
    pub item: ItemMetadata,
    #[serde(default)]
    pub effort: ItemEffort,
    #[serde(default)]
    pub history: ItemHistory,
}

impl ItemManifest {
    /// Deserializes an `ItemManifest` from a TOML string.
    pub fn from_toml_str(input: &str) -> Result<Self> {
        Ok(toml::from_str(input)?)
    }

    /// Serializes this `ItemManifest` into a formatted TOML string.
    pub fn to_toml_string(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Creates a lightweight `ItemStub` for indexing in `scope.toml`.
    pub fn to_stub(&self) -> ItemStub {
        ItemStub {
            id: self.item.id.clone(),
            title: self.item.title.clone(),
            status: self.item.status,
            severity: self.item.severity,
            created_at: self.item.created_at.clone(),
        }
    }
}

/// Combined lazily-loaded item detail (`item.toml` + `desc.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDetail {
    pub manifest: ItemManifest,
    pub description_markdown: String,
}

/// Maximum item schema version supported by this binary.
///
/// STEP 23F sets `Task => 2` and raises `MAX_ITEM_SCHEMA_VERSION` to 2.
pub const MAX_ITEM_SCHEMA_VERSION: u32 = 1;

/// Number of uppercase hex characters in a hash item ID suffix (e.g. `7F3A2C` in `BUG-7F3A2C`).
pub const HASH_ID_HEX_LEN: usize = 6;

/// Category of tracked work item (`bug` or `feature`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemType {
    #[serde(alias = "bugs")]
    Bug,
    #[serde(alias = "feat", alias = "features")]
    Feature,
}

impl ItemType {
    /// Returns the minimum `schema_version` required for an item of this type.
    ///
    /// STEP 23F sets `Task => 2` and raises `MAX_ITEM_SCHEMA_VERSION` to 2.
    pub fn min_schema_version(self) -> u32 {
        match self {
            Self::Bug | Self::Feature => 1,
        }
    }

    /// Returns the canonical ID prefix (`BUG` or `FEAT`).
    pub fn id_prefix(self) -> &'static str {
        match self {
            Self::Bug => "BUG",
            Self::Feature => "FEAT",
        }
    }

    /// Formats a monotonic sequence number into a canonical item ID (e.g. `BUG-0001`).
    pub fn format_id(self, number: u32) -> String {
        format!("{}-{:04}", self.id_prefix(), number)
    }

    /// Formats a 24-bit value into a canonical hash item ID (e.g. `BUG-7F3A2C`).
    pub fn format_hash_id(self, bits: u32) -> String {
        format!("{}-{:06X}", self.id_prefix(), bits & 0x00FF_FFFF)
    }
}

/// Returns `true` if `id` is a valid uppercase hash item ID (`BUG-XXXXXX` or `FEAT-XXXXXX`,
/// where `XXXXXX` is 6 uppercase hex digits containing at least one `A`–`F`).
pub fn is_hash_item_id(id: &str) -> bool {
    let Some((prefix, suffix)) = id.split_once('-') else {
        return false;
    };
    if !matches!(prefix, "BUG" | "FEAT") {
        return false;
    }
    if suffix.len() != HASH_ID_HEX_LEN {
        return false;
    }
    suffix
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'A'..=b'F'))
        && suffix.bytes().any(|b| matches!(b, b'A'..=b'F'))
}

impl fmt::Display for ItemType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bug => write!(f, "bug"),
            Self::Feature => write!(f, "feature"),
        }
    }
}

impl FromStr for ItemType {
    type Err = TrackerError;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "bug" | "bugs" => Ok(Self::Bug),
            "feature" | "features" | "feat" => Ok(Self::Feature),
            other => Err(TrackerError::InvalidEnumValue {
                enum_name: "ItemType",
                value: other.to_string(),
                expected: "bug, feature (or feat)",
            }),
        }
    }
}

/// Lifecycle status of a tracked bug or feature.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    #[default]
    Open,
    #[serde(alias = "in-progress", alias = "inprogress")]
    InProgress,
    Pending,
    Closed,
    Merged,
}

impl ItemStatus {
    /// Returns `true` if the item is considered completed/resolved (`closed` or `merged`).
    pub fn is_completed(self) -> bool {
        matches!(self, Self::Closed | Self::Merged)
    }
}

impl fmt::Display for ItemStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Open => write!(f, "open"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Pending => write!(f, "pending"),
            Self::Closed => write!(f, "closed"),
            Self::Merged => write!(f, "merged"),
        }
    }
}

impl FromStr for ItemStatus {
    type Err = TrackerError;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "open" => Ok(Self::Open),
            "in_progress" | "in-progress" | "inprogress" => Ok(Self::InProgress),
            "pending" => Ok(Self::Pending),
            "closed" => Ok(Self::Closed),
            "merged" => Ok(Self::Merged),
            other => Err(TrackerError::InvalidEnumValue {
                enum_name: "ItemStatus",
                value: other.to_string(),
                expected: "open, in_progress, pending, closed, merged",
            }),
        }
    }
}

/// Severity/priority classification ordered from `Low` to `Critical`.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    #[default]
    #[serde(alias = "med")]
    Medium,
    #[serde(alias = "hi")]
    High,
    #[serde(alias = "crit")]
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

impl FromStr for Severity {
    type Err = TrackerError;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "low" => Ok(Self::Low),
            "medium" | "med" => Ok(Self::Medium),
            "high" | "hi" => Ok(Self::High),
            "critical" | "crit" => Ok(Self::Critical),
            other => Err(TrackerError::InvalidEnumValue {
                enum_name: "Severity",
                value: other.to_string(),
                expected: "low, medium, high, critical",
            }),
        }
    }
}

/// `[item]` table inside `item.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemMetadata {
    pub id: String,
    #[serde(rename = "type")]
    pub item_type: ItemType,
    pub title: String,
    #[serde(default)]
    pub status: ItemStatus,
    #[serde(default)]
    pub severity: Severity,
    pub scope_ref: String,
    #[serde(deserialize_with = "deserialize_datetime_string")]
    pub created_at: String,
    #[serde(deserialize_with = "deserialize_datetime_string")]
    pub updated_at: String,
}

impl ItemMetadata {
    /// Parses `created_at` into `DateTime<Utc>`.
    pub fn parsed_created_at(&self) -> Result<DateTime<Utc>> {
        parse_timestamp_utc(&self.created_at)
    }

    /// Parses `updated_at` into `DateTime<Utc>`.
    pub fn parsed_updated_at(&self) -> Result<DateTime<Utc>> {
        parse_timestamp_utc(&self.updated_at)
    }
}

/// `[effort]` table inside `item.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemEffort {
    #[serde(default)]
    pub logged_time_minutes: u32,
    #[serde(default)]
    pub iteration_count: u32,
}

/// `[history]` table inside `item.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemHistory {
    #[serde(default)]
    pub recurrence_count: u32,
    #[serde(default)]
    pub previous_fixes: Vec<PreviousFix>,
}

/// Single fix entry in `item.toml` `[history].previous_fixes`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PreviousFix {
    #[serde(deserialize_with = "deserialize_datetime_string")]
    pub date: String,
    pub commit: String,
    pub summary: String,
}

impl PreviousFix {
    /// Parses `date` into `NaiveDate`.
    pub fn parsed_date(&self) -> Result<NaiveDate> {
        parse_date(&self.date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_hash_id_pads_and_masks_to_24_bits() {
        assert_eq!(ItemType::Bug.format_hash_id(0xABC), "BUG-000ABC");
        assert_eq!(ItemType::Feature.format_hash_id(0xFFFF_FFFF), "FEAT-FFFFFF");
    }

    #[test]
    fn is_hash_item_id_validates_prefix_length_uppercase_and_alpha_requirement() {
        assert!(is_hash_item_id("BUG-7F3A2C"));
        assert!(is_hash_item_id("FEAT-00000A"));

        assert!(!is_hash_item_id("BUG-001234"));
        assert!(!is_hash_item_id("BUG-0012"));
        assert!(!is_hash_item_id("BUG-7f3a2c"));
        assert!(!is_hash_item_id("BUG-7F3A2"));
        assert!(!is_hash_item_id("BUG-7F3A2C1"));
        assert!(!is_hash_item_id("TASK-7F3A2C"));
        assert!(!is_hash_item_id("BUG7F3A2C"));
        assert!(!is_hash_item_id(""));
    }
}
