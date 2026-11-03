use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};
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
}

impl ProjectMetadata {
    /// Parses `created_at` into a UTC `DateTime`.
    pub fn parsed_created_at(&self) -> Result<DateTime<Utc>> {
        parse_timestamp_utc(&self.created_at)
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
}

impl Default for GitConfig {
    fn default() -> Self {
        Self {
            sync_mode: GitSyncMode::Manual,
            sync_interval_minutes: default_sync_interval_minutes(),
            remote: default_git_remote(),
            branch: default_git_branch(),
            auto_commit_on_close: false,
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
