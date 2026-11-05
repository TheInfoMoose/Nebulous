use std::fmt;
use std::str::FromStr;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};
use crate::models::{deserialize_opt_datetime_string, parse_date};

/// Canonical 3-stage development lifecycle phases (`(phase_id, title)`).
pub const CANONICAL_PHASES: &[(&str, &str)] = &[
    ("pre_production", "Pre-Production"),
    ("production", "Production"),
    ("post_production", "Post-Production"),
];

/// Resolves shorthand stage aliases (`pre`, `prod`, `post`, `1`, `2`, `3`, etc.)
/// to their canonical phase identifier (`pre_production`, `production`, `post_production`).
///
/// Any non-matching identifier is returned trimmed and unchanged so custom phases continue
/// to work seamlessly.
pub fn resolve_phase_alias(input: &str) -> &str {
    let trimmed = input.trim();
    if trimmed.eq_ignore_ascii_case("pre")
        || trimmed.eq_ignore_ascii_case("pre-prod")
        || trimmed.eq_ignore_ascii_case("pre_prod")
        || trimmed.eq_ignore_ascii_case("pre-production")
        || trimmed.eq_ignore_ascii_case("pre_production")
        || trimmed == "1"
    {
        "pre_production"
    } else if trimmed.eq_ignore_ascii_case("prod")
        || trimmed.eq_ignore_ascii_case("production")
        || trimmed.eq_ignore_ascii_case("active")
        || trimmed == "2"
    {
        "production"
    } else if trimmed.eq_ignore_ascii_case("post")
        || trimmed.eq_ignore_ascii_case("post-prod")
        || trimmed.eq_ignore_ascii_case("post_prod")
        || trimmed.eq_ignore_ascii_case("post-production")
        || trimmed.eq_ignore_ascii_case("post_production")
        || trimmed == "3"
    {
        "post_production"
    } else {
        trimmed
    }
}

/// Returns the human-readable title for a canonical phase ID (or alias), if `phase_id`
/// resolves to one of [`CANONICAL_PHASES`].
pub fn canonical_phase_title(phase_id: &str) -> Option<&'static str> {
    let resolved = resolve_phase_alias(phase_id);
    CANONICAL_PHASES
        .iter()
        .find(|(id, _)| *id == resolved)
        .map(|(_, title)| *title)
}

/// Tier 2: Phase index manifest (`.tracker/phases/<phase_id>/phase.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseManifest {
    pub phase: PhaseMetadata,
    #[serde(default)]
    pub scopes: ScopeRegistry,
    #[serde(default)]
    pub summary: PhaseSummary,
}

impl PhaseManifest {
    /// Deserializes a `PhaseManifest` from a TOML string.
    pub fn from_toml_str(input: &str) -> Result<Self> {
        Ok(toml::from_str(input)?)
    }

    /// Serializes this `PhaseManifest` into a formatted TOML string.
    pub fn to_toml_string(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }
}

/// Lifecycle status of a project phase.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    #[default]
    Planned,
    #[serde(alias = "in-progress", alias = "inprogress")]
    InProgress,
    #[serde(alias = "complete", alias = "done")]
    Completed,
    Blocked,
}

impl fmt::Display for PhaseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Planned => write!(f, "planned"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Blocked => write!(f, "blocked"),
        }
    }
}

impl FromStr for PhaseStatus {
    type Err = TrackerError;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "planned" => Ok(Self::Planned),
            "in_progress" | "in-progress" | "inprogress" => Ok(Self::InProgress),
            "completed" | "complete" | "done" => Ok(Self::Completed),
            "blocked" => Ok(Self::Blocked),
            other => Err(TrackerError::InvalidEnumValue {
                enum_name: "PhaseStatus",
                value: other.to_string(),
                expected: "planned, in_progress, completed, blocked",
            }),
        }
    }
}

/// `[phase]` table inside `phase.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseMetadata {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub status: PhaseStatus,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_opt_datetime_string"
    )]
    pub start_date: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_opt_datetime_string"
    )]
    pub end_date: Option<String>,
}

impl PhaseMetadata {
    /// Parses `start_date` if present into `NaiveDate`.
    pub fn parsed_start_date(&self) -> Result<Option<NaiveDate>> {
        self.start_date.as_deref().map(parse_date).transpose()
    }

    /// Parses `end_date` if present into `NaiveDate`.
    pub fn parsed_end_date(&self) -> Result<Option<NaiveDate>> {
        self.end_date.as_deref().map(parse_date).transpose()
    }
}

/// `[scopes]` table inside `phase.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeRegistry {
    #[serde(default)]
    pub manifest: Vec<ScopeRegistryEntry>,
}

/// Single domain scope entry inside `phase.toml` `[scopes].manifest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeRegistryEntry {
    pub id: String,
    #[serde(rename = "type", default = "default_scope_type")]
    pub scope_type: String,
    pub path: String,
}

/// `[summary]` table inside `phase.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseSummary {
    #[serde(default)]
    pub total_items: u32,
    #[serde(default)]
    pub completed_items: u32,
}

fn default_scope_type() -> String {
    "rust_crate".to_string()
}
