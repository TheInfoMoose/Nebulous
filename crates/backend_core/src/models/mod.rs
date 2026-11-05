pub mod component;
pub mod item;
pub mod phase;
pub mod project;
pub mod registry;
pub mod scope;
pub mod timer;

pub use component::{
    ComponentDependency, ComponentManifest, ComponentRegistry, ComponentType, PinnedReleaseConfig,
    RollingUpdateEntry, VerificationRecord, COMPONENT_MANIFEST_FILE_NAME,
};
pub use item::{
    ItemDetail, ItemEffort, ItemHistory, ItemManifest, ItemMetadata, ItemStatus, ItemType,
    PreviousFix, Severity,
};
pub use phase::{
    canonical_phase_title, resolve_phase_alias, PhaseManifest, PhaseMetadata, PhaseStatus,
    PhaseSummary, ScopeRegistry, ScopeRegistryEntry, CANONICAL_PHASES,
};
pub use project::{
    GitConfig, GitSyncMode, MetricsConfig, PhaseRegistry, PhaseRegistryEntry, ProjectCounters,
    ProjectManifest, ProjectMetadata,
};
pub use registry::{
    global_registry_path, load_global_registry, load_global_registry_from, register_project,
    register_project_in, remove_project, remove_project_in, resolve_registry_file_path,
    save_global_registry, save_global_registry_to, set_active_project, set_active_project_in,
    GlobalProjectRegistry, RegisteredProject, GLOBAL_PROJECTS_FILE_NAME,
};
pub use scope::{ItemStub, ScopeItems, ScopeManifest, ScopeMetadata};
pub use timer::ActiveTimer;

use crate::error::{Result, TrackerError};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use serde::{de::Error as DeError, Deserialize, Deserializer};

/// Deserializes either a quoted string or a native unquoted TOML date/datetime into a `String`.
pub fn deserialize_datetime_string<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = toml::Value::deserialize(deserializer)?;
    match value {
        toml::Value::String(s) => Ok(s),
        toml::Value::Datetime(dt) => Ok(dt.to_string()),
        other => Err(D::Error::custom(format!(
            "expected a date/datetime string or TOML datetime, found {}",
            other.type_str()
        ))),
    }
}

/// Deserializes an optional quoted string or native unquoted TOML date/datetime into `Option<String>`.
pub fn deserialize_opt_datetime_string<'de, D>(
    deserializer: D,
) -> std::result::Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<toml::Value>::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(toml::Value::String(s)) => Ok(Some(s)),
        Some(toml::Value::Datetime(dt)) => Ok(Some(dt.to_string())),
        Some(other) => Err(D::Error::custom(format!(
            "expected an optional date/datetime string or TOML datetime, found {}",
            other.type_str()
        ))),
    }
}

const LOCAL_DATETIME_FORMATS: &[&str] = &[
    "%Y-%m-%dT%H:%M:%S%.f",
    "%Y-%m-%d %H:%M:%S%.f",
    "%Y-%m-%dT%H:%M:%S",
    "%Y-%m-%d %H:%M:%S",
];

/// Parses an RFC 3339 timestamp string, local TOML datetime, or `YYYY-MM-DD` date fallback (at 00:00:00Z) into `DateTime<Utc>`.
pub fn parse_timestamp_utc(value: &str) -> Result<DateTime<Utc>> {
    let trimmed = value.trim();
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.with_timezone(&Utc));
    }
    for fmt in LOCAL_DATETIME_FORMATS {
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(trimmed, fmt) {
            return Ok(naive_dt.and_utc());
        }
    }
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        if let Some(naive_dt) = date.and_hms_opt(0, 0, 0) {
            return Ok(naive_dt.and_utc());
        }
    }
    Err(TrackerError::InvalidDateTime {
        value: value.to_string(),
        reason: "expected RFC 3339 timestamp (e.g. 2026-11-01T21:00:00Z), local datetime, or YYYY-MM-DD date"
            .to_string(),
    })
}

/// Parses a `YYYY-MM-DD` date string (or extracts the date portion of a timestamp) into `NaiveDate`.
pub fn parse_date(value: &str) -> Result<NaiveDate> {
    let trimmed = value.trim();
    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        return Ok(date);
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.with_timezone(&Utc).date_naive());
    }
    for fmt in LOCAL_DATETIME_FORMATS {
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(trimmed, fmt) {
            return Ok(naive_dt.date());
        }
    }
    Err(TrackerError::InvalidDateTime {
        value: value.to_string(),
        reason: "expected YYYY-MM-DD date or RFC 3339 / ISO 8601 timestamp".to_string(),
    })
}
