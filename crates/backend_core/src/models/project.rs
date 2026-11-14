use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};
use crate::models::component::ComponentRegistry;
use crate::models::item::ItemType;
use crate::models::{deserialize_datetime_string, parse_timestamp_utc};

/// Tier 1: Root project manifest (`.tracker/project.toml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectManifest {
    pub project: ProjectMetadata,
    #[serde(default)]
    pub counters: ProjectCounters,
    #[serde(default)]
    pub phases: PhaseRegistry,
    #[serde(default)]
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub git: GitConfig,
    #[serde(default, skip_serializing_if = "ComponentRegistry::is_empty")]
    pub components: ComponentRegistry,
}

impl ProjectManifest {
    /// Deserializes a `ProjectManifest` from a TOML string.
    pub fn from_toml_str(input: &str) -> Result<Self> {
        Ok(toml::from_str(input)?)
    }

    /// Serializes this `ProjectManifest` into a formatted TOML string.
    pub fn to_toml_string(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }
}

/// `[project]` table inside `.tracker/project.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectMetadata {
    pub name: String,
    #[serde(default = "default_project_version")]
    pub version: String,
    pub active_phase: String,
    #[serde(default = "default_scope_name")]
    pub default_scope: String,
    #[serde(deserialize_with = "deserialize_datetime_string")]
    pub created_at: String,
    #[serde(default, skip_serializing_if = "IdScheme::is_sequential")]
    pub id_scheme: IdScheme,
}

impl ProjectMetadata {
    /// Parses `created_at` into a UTC `DateTime`.
    pub fn parsed_created_at(&self) -> Result<DateTime<Utc>> {
        parse_timestamp_utc(&self.created_at)
    }
}

/// Item ID allocation scheme (`sequential` allocates `BUG-0001`-style counters;
/// `hash` allocates `BUG-7F3A2C`-style random IDs). A missing key defaults to `Sequential`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdScheme {
    #[default]
    Sequential,
    Hash,
}

impl IdScheme {
    /// Returns `true` if this scheme is `Sequential`.
    pub fn is_sequential(&self) -> bool {
        matches!(self, Self::Sequential)
    }
}

/// `[counters]` table inside `.tracker/project.toml` for monotonic ID generation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectCounters {
    #[serde(default)]
    pub bug: u32,
    #[serde(default, alias = "feat")]
    pub feature: u32,
}

impl ProjectCounters {
    /// Returns the counter value for `kind`.
    pub fn get(&self, kind: ItemType) -> u32 {
        match kind {
            ItemType::Bug => self.bug,
            ItemType::Feature => self.feature,
        }
    }

    /// Returns a mutable reference to the counter for `kind`.
    pub fn get_mut(&mut self, kind: ItemType) -> &mut u32 {
        match kind {
            ItemType::Bug => &mut self.bug,
            ItemType::Feature => &mut self.feature,
        }
    }

    /// Updates each counter in `self` to at least the corresponding value in `other`.
    pub fn merge_max(&mut self, other: &ProjectCounters) {
        for &kind in ItemType::ALL {
            let v = other.get(kind);
            let slot = self.get_mut(kind);
            *slot = (*slot).max(v);
        }
    }
}

/// `[phases]` table inside `.tracker/project.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseRegistry {
    #[serde(default)]
    pub registry: Vec<PhaseRegistryEntry>,
}

/// Single phase registration entry in `project.toml` `[phases].registry`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseRegistryEntry {
    pub id: String,
    pub title: String,
    pub path: String,
}

/// `[metrics]` table inside `.tracker/project.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_target_daily_hours")]
    pub target_daily_hours: f64,
    #[serde(default = "default_stagnant_threshold_days")]
    pub stagnant_item_threshold_days: u32,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            target_daily_hours: default_target_daily_hours(),
            stagnant_item_threshold_days: default_stagnant_threshold_days(),
        }
    }
}

/// Synchronization mode for `.tracker/` Git operations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GitSyncMode {
    #[default]
    Manual,
    #[serde(alias = "auto")]
    Scheduled,
}

impl fmt::Display for GitSyncMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Manual => write!(f, "manual"),
            Self::Scheduled => write!(f, "scheduled"),
        }
    }
}

impl FromStr for GitSyncMode {
    type Err = TrackerError;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "manual" => Ok(Self::Manual),
            "scheduled" | "auto" => Ok(Self::Scheduled),
            other => Err(TrackerError::InvalidEnumValue {
                enum_name: "GitSyncMode",
                value: other.to_string(),
                expected: "manual, scheduled",
            }),
        }
    }
}

/// `[git]` table inside `.tracker/project.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitConfig {
    #[serde(default)]
    pub sync_mode: GitSyncMode,
    #[serde(default = "default_sync_interval_minutes")]
    pub sync_interval_minutes: u32,
    #[serde(default = "default_git_remote")]
    pub remote: String,
    #[serde(default = "default_git_branch")]
    pub branch: String,
    #[serde(default)]
    pub auto_commit_on_close: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh_key_account: Option<String>,
    #[serde(default)]
    pub commit_policy: CommitPolicyConfig,
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            sync_mode: GitSyncMode::Manual,
            sync_interval_minutes: default_sync_interval_minutes(),
            remote: default_git_remote(),
            branch: default_git_branch(),
            auto_commit_on_close: false,
            ssh_key_account: None,
            commit_policy: CommitPolicyConfig::default(),
        }
    }
}

/// `[git.commit_policy]` table inside `.tracker/project.toml` enforcing documented commits.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitPolicyConfig {
    #[serde(default = "default_true")]
    pub require_item_id: bool,
    #[serde(default = "default_true")]
    pub require_scope: bool,
    #[serde(default = "default_true")]
    pub require_summary: bool,
    #[serde(default = "default_min_summary_length")]
    pub min_summary_length: usize,
    #[serde(default = "default_true")]
    pub auto_append_to_desc: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ssh_key_account: Option<String>,
}

impl Default for CommitPolicyConfig {
    fn default() -> Self {
        Self {
            require_item_id: default_true(),
            require_scope: default_true(),
            require_summary: default_true(),
            min_summary_length: default_min_summary_length(),
            auto_append_to_desc: default_true(),
            ssh_key_account: None,
        }
    }
}

fn default_project_version() -> String {
    "0.1.0".to_string()
}

fn default_scope_name() -> String {
    "inbox".to_string()
}

fn default_target_daily_hours() -> f64 {
    6.0
}

fn default_stagnant_threshold_days() -> u32 {
    4
}

fn default_sync_interval_minutes() -> u32 {
    30
}

fn default_git_remote() -> String {
    "origin".to_string()
}

fn default_git_branch() -> String {
    "main".to_string()
}

fn default_true() -> bool {
    true
}

fn default_min_summary_length() -> usize {
    12
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE_PROJECT_TOML: &str = r#"[project]
name = "test-proj"
active_phase = "production"
created_at = "2026-11-09T12:00:00Z"
"#;

    #[test]
    fn id_scheme_defaults_to_sequential_and_parses_hash() {
        let seq = ProjectManifest::from_toml_str(BASE_PROJECT_TOML).unwrap();
        assert_eq!(seq.project.id_scheme, IdScheme::Sequential);
        assert!(seq.project.id_scheme.is_sequential());

        let hash_toml = format!("{BASE_PROJECT_TOML}id_scheme = \"hash\"\n");
        let hash = ProjectManifest::from_toml_str(&hash_toml).unwrap();
        assert_eq!(hash.project.id_scheme, IdScheme::Hash);
        assert!(!hash.project.id_scheme.is_sequential());
    }

    #[test]
    fn id_scheme_skips_serializing_when_sequential_and_emits_when_hash() {
        let mut manifest = ProjectManifest::from_toml_str(BASE_PROJECT_TOML).unwrap();
        let seq_serialized = manifest.to_toml_string().unwrap();
        assert!(!seq_serialized.contains("id_scheme"));

        manifest.project.id_scheme = IdScheme::Hash;
        let hash_serialized = manifest.to_toml_string().unwrap();
        assert!(hash_serialized.contains("id_scheme = \"hash\""));
    }

    #[test]
    fn id_scheme_rejects_invalid_value() {
        let bad_toml = format!("{BASE_PROJECT_TOML}id_scheme = \"bogus\"\n");
        assert!(ProjectManifest::from_toml_str(&bad_toml).is_err());
    }

    #[test]
    fn project_counters_get_and_merge_max_per_kind() {
        let mut counters = ProjectCounters { bug: 3, feature: 7 };
        *counters.get_mut(ItemType::Bug) += 1;
        assert_eq!(counters.get(ItemType::Bug), 4);
        assert_eq!(counters.get(ItemType::Feature), 7);

        counters.merge_max(&ProjectCounters { bug: 2, feature: 9 });
        assert_eq!(counters, ProjectCounters { bug: 4, feature: 9 });
    }
}
