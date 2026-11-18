pub mod component;
pub mod item;
pub mod phase;
pub mod project;
pub mod registry;
pub mod scope;
pub mod time;
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
pub use time::{parse_date, parse_timestamp_utc};
pub use timer::ActiveTimer;

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
