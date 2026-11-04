use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::models::item::{ItemStatus, Severity};

/// Tier 3: Domain/crate scope index manifest (`.../domains/<scope_id>/scope.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeManifest {
    pub scope: ScopeMetadata,
    #[serde(default)]
    pub items: ScopeItems,
}

impl ScopeManifest {
    /// Deserializes a `ScopeManifest` from a TOML string.
    pub fn from_toml_str(input: &str) -> Result<Self> {
        Ok(toml::from_str(input)?)
    }

    /// Serializes this `ScopeManifest` into a formatted TOML string.
    pub fn to_toml_string(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Returns total number of indexed bugs and features in this scope.
    pub fn total_items(&self) -> usize {
        self.items.bugs.len() + self.items.features.len()
    }
}

/// `[scope]` table inside `scope.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeMetadata {
    pub id: String,
    pub target: String,
    #[serde(default)]
    pub tech_stack: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_component: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tracked_files: Vec<String>,
}

/// `[items]` table inside `scope.toml` containing lightweight item stubs for fast index loading.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeItems {
    #[serde(default)]
    pub bugs: Vec<ItemStub>,
    #[serde(default)]
    pub features: Vec<ItemStub>,
}

/// Lightweight item summary stored in `scope.toml` `[items]`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemStub {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub status: ItemStatus,
    #[serde(default)]
    pub severity: Severity,
}
