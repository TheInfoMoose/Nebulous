use std::collections::BTreeMap;
use std::fmt;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

use crate::error::{Result, TrackerError};
use crate::models::{
    deserialize_datetime_string, deserialize_opt_datetime_string, parse_timestamp_utc,
};
use crate::storage::{normalize_relative_path_list, normalize_relative_path_str};

/// Canonical filename for per-directory Nebulous component manifests.
pub const COMPONENT_MANIFEST_FILE_NAME: &str = "nebulous.component.toml";

/// Standard component classification categories recognized during directory ingestion.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentType {
    RustCrate,
    CliCrate,
    UiCrate,
    PythonPkg,
    NodePkg,
    CCpp,
    OsImage,
    KernelModule,
    BuildScripts,
    #[default]
    Custom,
}

impl ComponentType {
    /// Returns the canonical snake_case string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::RustCrate => "rust_crate",
            Self::CliCrate => "cli_crate",
            Self::UiCrate => "ui_crate",
            Self::PythonPkg => "python_pkg",
            Self::NodePkg => "node_pkg",
            Self::CCpp => "c_cpp",
            Self::OsImage => "os_image",
            Self::KernelModule => "kernel_module",
            Self::BuildScripts => "build_scripts",
            Self::Custom => "custom",
        }
    }
}

impl fmt::Display for ComponentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ComponentType {
    type Err = TrackerError;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "rust_crate" | "rust" | "crate" => Ok(Self::RustCrate),
            "cli_crate" | "cli" => Ok(Self::CliCrate),
            "ui_crate" | "ui" | "gui" => Ok(Self::UiCrate),
            "python_pkg" | "python" => Ok(Self::PythonPkg),
            "node_pkg" | "node" | "javascript" | "typescript" => Ok(Self::NodePkg),
            "c_cpp" | "c" | "cpp" | "cxx" => Ok(Self::CCpp),
            "os_image" | "iso" | "distro" | "os" => Ok(Self::OsImage),
            "kernel_module" | "kernel" | "kmod" => Ok(Self::KernelModule),
            "build_scripts" | "scripts" | "ci" => Ok(Self::BuildScripts),
            "custom" => Ok(Self::Custom),
            other => Err(TrackerError::InvalidEnumValue {
                enum_name: "ComponentType",
                value: other.to_string(),
                expected: "rust_crate, cli_crate, ui_crate, python_pkg, node_pkg, c_cpp, os_image, kernel_module, build_scripts, custom",
            }),
        }
    }
}

impl From<ComponentType> for String {
    fn from(value: ComponentType) -> Self {
        value.as_str().to_string()
    }
}

/// `[components]` section inside `.tracker/project.toml`.
///
/// Supports both `[components]\nregistry = [...]` and direct array deserialization.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ComponentRegistry {
    #[serde(default)]
    pub registry: Vec<ComponentManifest>,
}

impl ComponentRegistry {
    /// Returns `true` if no components are registered.
    pub fn is_empty(&self) -> bool {
        self.registry.is_empty()
    }

    /// Returns the number of registered components.
    pub fn len(&self) -> usize {
        self.registry.len()
    }

    /// Looks up a component by `id` or normalized `relative_path`.
    pub fn find(&self, id_or_path: &str) -> Option<&ComponentManifest> {
        let trimmed = id_or_path.trim();
        let norm_path = normalize_relative_path_str(trimmed);
        self.registry
            .iter()
            .find(|c| c.id == trimmed || (!norm_path.is_empty() && c.relative_path == norm_path))
    }

    /// Looks up a mutable component by `id` or normalized `relative_path`.
    pub fn find_mut(&mut self, id_or_path: &str) -> Option<&mut ComponentManifest> {
        let trimmed = id_or_path.trim();
        let norm_path = normalize_relative_path_str(trimmed);
        self.registry
            .iter_mut()
            .find(|c| c.id == trimmed || (!norm_path.is_empty() && c.relative_path == norm_path))
    }

    /// Inserts or updates a [`ComponentManifest`] in the registry and keeps entries sorted by `id`.
    pub fn upsert(&mut self, mut manifest: ComponentManifest) {
        manifest.normalize_paths();
        self.registry.retain(|c| {
            c.id != manifest.id
                && (manifest.relative_path.is_empty()
                    || manifest.relative_path == "."
                    || c.relative_path != manifest.relative_path)
        });
        self.registry.push(manifest);
        self.registry.sort_by(|a, b| a.id.cmp(&b.id));
    }
}

impl Deref for ComponentRegistry {
    type Target = Vec<ComponentManifest>;

    fn deref(&self) -> &Self::Target {
        &self.registry
    }
}

impl DerefMut for ComponentRegistry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.registry
    }
}

impl<'de> Deserialize<'de> for ComponentRegistry {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ComponentRegistryRepr {
            Table {
                #[serde(default, alias = "manifest", alias = "entries")]
                registry: Vec<ComponentManifest>,
            },
            List(Vec<ComponentManifest>),
        }

        match ComponentRegistryRepr::deserialize(deserializer)? {
            ComponentRegistryRepr::Table { registry } => Ok(Self { registry }),
            ComponentRegistryRepr::List(registry) => Ok(Self { registry }),
        }
    }
}

/// Component manifest stored in `<dir>/nebulous.component.toml` and indexed in `.tracker/project.toml` `[components]`.
///
/// Note on field ordering: All scalar and primitive-array fields are declared before nested
/// table / array-of-table fields (`dependencies`, `pinned_release`, `rolling_updates`) so
/// `toml::to_string_pretty` never fails with `ValueAfterTable`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentManifest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "default_relative_path", alias = "path")]
    pub relative_path: String,
    #[serde(default = "default_component_type_str", alias = "type")]
    pub component_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub tech_stack: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub flags: Vec<String>,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<ComponentDependency>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "pinned_config",
        alias = "last_known_good"
    )]
    pub pinned_release: Option<PinnedReleaseConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rolling_updates: Vec<RollingUpdateEntry>,
}

impl Default for ComponentManifest {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            relative_path: default_relative_path(),
            component_type: default_component_type_str(),
            description: None,
            tech_stack: Vec::new(),
            tags: Vec::new(),
            flags: Vec::new(),
            files: Vec::new(),
            dependencies: Vec::new(),
            pinned_release: None,
            rolling_updates: Vec::new(),
        }
    }
}

impl ComponentManifest {
    /// Creates a new [`ComponentManifest`] with required identifiers and defaults for collections.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        relative_path: impl Into<String>,
        component_type: impl Into<String>,
    ) -> Self {
        let mut manifest = Self {
            id: id.into(),
            name: name.into(),
            relative_path: relative_path.into(),
            component_type: component_type.into(),
            ..Self::default()
        };
        manifest.normalize_paths();
        manifest
    }

    /// Deserializes a [`ComponentManifest`] from a `nebulous.component.toml` string.
    ///
    /// Supports both flat top-level component manifests and `[component]`-wrapped manifests.
    pub fn from_toml_str(input: &str) -> Result<Self> {
        #[derive(Deserialize)]
        struct WrappedComponent {
            component: ComponentManifest,
            #[serde(default)]
            tech_stack: Vec<String>,
            #[serde(default)]
            tags: Vec<String>,
            #[serde(default)]
            flags: Vec<String>,
            #[serde(default)]
            files: Vec<String>,
            #[serde(default)]
            dependencies: Vec<ComponentDependency>,
            #[serde(default, alias = "pinned_config", alias = "last_known_good")]
            pinned_release: Option<PinnedReleaseConfig>,
            #[serde(default)]
            rolling_updates: Vec<RollingUpdateEntry>,
        }

        let has_component_section = toml::from_str::<toml::Table>(input)
            .map(|t| t.contains_key("component"))
            .unwrap_or(false);

        if has_component_section {
            let wrapped = toml::from_str::<WrappedComponent>(input)?;
            let mut manifest = wrapped.component;
            if manifest.tech_stack.is_empty() && !wrapped.tech_stack.is_empty() {
                manifest.tech_stack = wrapped.tech_stack;
            }
            if manifest.tags.is_empty() && !wrapped.tags.is_empty() {
                manifest.tags = wrapped.tags;
            }
            if manifest.flags.is_empty() && !wrapped.flags.is_empty() {
                manifest.flags = wrapped.flags;
            }
            if manifest.files.is_empty() && !wrapped.files.is_empty() {
                manifest.files = wrapped.files;
            }
            if manifest.dependencies.is_empty() && !wrapped.dependencies.is_empty() {
                manifest.dependencies = wrapped.dependencies;
            }
            if manifest.pinned_release.is_none() && wrapped.pinned_release.is_some() {
                manifest.pinned_release = wrapped.pinned_release;
            }
            if manifest.rolling_updates.is_empty() && !wrapped.rolling_updates.is_empty() {
                manifest.rolling_updates = wrapped.rolling_updates;
            }
            manifest.normalize_paths();
            if manifest.id.trim().is_empty() && manifest.name.trim().is_empty() {
                return Err(TrackerError::ValidationError(
                    "ComponentManifest must specify 'id' or 'name'".to_string(),
                ));
            }
            return Ok(manifest);
        }

        let mut manifest: Self = toml::from_str(input)?;
        manifest.normalize_paths();
        if manifest.id.trim().is_empty() && manifest.name.trim().is_empty() {
            return Err(TrackerError::ValidationError(
                "ComponentManifest must specify 'id' or 'name'".to_string(),
            ));
        }
        Ok(manifest)
    }

    /// Serializes this [`ComponentManifest`] into a formatted TOML string.
    pub fn to_toml_string(&self) -> Result<String> {
        let mut normalized = self.clone();
        normalized.normalize_paths();
        Ok(toml::to_string_pretty(&normalized)?)
    }

    /// Normalizes `id`/`name` fallbacks, `relative_path`, `files`, and any nested script paths to forward slashes (`/`).
    pub fn normalize_paths(&mut self) {
        if self.id.trim().is_empty() && !self.name.trim().is_empty() {
            self.id = crate::models::registry::slugify_project_id(&self.name);
        } else if self.name.trim().is_empty() && !self.id.trim().is_empty() {
            self.name = self.id.clone();
        }
        let norm_rel = normalize_relative_path_str(&self.relative_path);
        self.relative_path = if norm_rel.is_empty() {
            ".".to_string()
        } else {
            norm_rel
        };
        self.files = normalize_relative_path_list(&self.files);
        if let Some(pin) = &mut self.pinned_release {
            if let Some(script) = &pin.build_script_path {
                let norm = normalize_relative_path_str(script);
                pin.build_script_path = if norm.is_empty() { None } else { Some(norm) };
            }
        }
        for update in &mut self.rolling_updates {
            if let Some(script) = &update.build_script_path {
                let norm = normalize_relative_path_str(script);
                update.build_script_path = if norm.is_empty() { None } else { Some(norm) };
            }
        }
    }

    /// Snapshots the current component dependency versions, commit SHA, build script, and flags
    /// into `pinned_release` (`last_known_good`).
    pub fn pin_current_release(
        &mut self,
        commit_sha: impl Into<String>,
        pinned_at: impl Into<String>,
        build_script_path: Option<String>,
        build_flags: Vec<String>,
        last_verification_run: Option<VerificationRecord>,
    ) -> PinnedReleaseConfig {
        let mut dependency_pins = BTreeMap::new();
        for dep in &self.dependencies {
            if !dep.current_version.trim().is_empty() {
                dependency_pins.insert(dep.name.clone(), dep.current_version.clone());
            } else if !dep.version_constraint.trim().is_empty() {
                dependency_pins.insert(dep.name.clone(), dep.version_constraint.clone());
            }
        }

        let effective_flags = if build_flags.is_empty() {
            self.flags.clone()
        } else {
            build_flags
        };

        let pin = PinnedReleaseConfig {
            commit_sha: commit_sha.into(),
            pinned_at: pinned_at.into(),
            build_script_path: build_script_path
                .map(|p| normalize_relative_path_str(&p))
                .filter(|p| !p.is_empty()),
            build_flags: effective_flags,
            dependency_pins,
            last_verification_run,
        };
        self.pinned_release = Some(pin.clone());
        pin
    }

    /// Restores `dependencies[*].current_version` and `flags` from `pinned_release` if present.
    pub fn restore_from_pinned_release(&mut self) -> Option<&PinnedReleaseConfig> {
        let pin = self.pinned_release.as_ref()?;
        for dep in &mut self.dependencies {
            if let Some(pinned_ver) = pin.dependency_pins.get(&dep.name) {
                dep.current_version = pinned_ver.clone();
            }
        }
        if !pin.build_flags.is_empty() {
            self.flags = pin.build_flags.clone();
        }
        self.pinned_release.as_ref()
    }
}

/// Upstream or ecosystem dependency tracked by a [`ComponentManifest`] (e.g., `apt_sid`, `kernel_xanmod`, `cargo`, `git`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentDependency {
    #[serde(alias = "package")]
    pub name: String,
    #[serde(default = "default_dependency_kind", alias = "channel")]
    pub kind: String,
    #[serde(default, alias = "constraint")]
    pub version_constraint: String,
    #[serde(default, alias = "version")]
    pub current_version: String,
    #[serde(default, alias = "source")]
    pub upstream_source: String,
}

impl ComponentDependency {
    /// Creates a new [`ComponentDependency`].
    pub fn new(
        name: impl Into<String>,
        kind: impl Into<String>,
        version_constraint: impl Into<String>,
        current_version: impl Into<String>,
        upstream_source: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind: kind.into(),
            version_constraint: version_constraint.into(),
            current_version: current_version.into(),
            upstream_source: upstream_source.into(),
        }
    }
}

/// Known-good release pin snapshot (`[pinned_release]` / `last_known_good`) inside [`ComponentManifest`].
///
/// Note on field ordering: Scalar and array-of-scalar fields precede `dependency_pins` (`BTreeMap`)
/// and `last_verification_run` (`Option<VerificationRecord>`) for clean TOML table serialization.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PinnedReleaseConfig {
    pub commit_sha: String,
    #[serde(default, deserialize_with = "deserialize_datetime_string")]
    pub pinned_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_script_path: Option<String>,
    #[serde(default)]
    pub build_flags: Vec<String>,
    #[serde(
        default,
        alias = "dependency_versions",
        deserialize_with = "deserialize_dependency_pins"
    )]
    pub dependency_pins: BTreeMap<String, String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "verification_run",
        alias = "verification_status"
    )]
    pub last_verification_run: Option<VerificationRecord>,
}

fn flatten_dependency_pins_table<E: serde::de::Error>(
    prefix: &str,
    table: toml::map::Map<String, toml::Value>,
    out: &mut BTreeMap<String, String>,
) -> std::result::Result<(), E> {
    for (k, v) in table {
        let full_key = if prefix.is_empty() {
            k
        } else {
            format!("{prefix}.{k}")
        };
        match v {
            toml::Value::String(s) => {
                out.insert(full_key, s);
            }
            toml::Value::Integer(i) => {
                out.insert(full_key, i.to_string());
            }
            toml::Value::Float(f) => {
                out.insert(full_key, f.to_string());
            }
            toml::Value::Boolean(b) => {
                out.insert(full_key, b.to_string());
            }
            toml::Value::Datetime(dt) => {
                out.insert(full_key, dt.to_string());
            }
            toml::Value::Table(nested) => {
                flatten_dependency_pins_table(&full_key, nested, out)?;
            }
            toml::Value::Array(_) => {
                return Err(E::custom(format!(
                    "expected string version or nested dotted key table for dependency pin '{full_key}', found array"
                )));
            }
        }
    }
    Ok(())
}

fn deserialize_dependency_pins<'de, D>(
    deserializer: D,
) -> std::result::Result<BTreeMap<String, String>, D::Error>
where
    D: Deserializer<'de>,
{
    let val = Option::<toml::Value>::deserialize(deserializer)?;
    let mut out = BTreeMap::new();
    match val {
        None => Ok(out),
        Some(toml::Value::Table(table)) => {
            flatten_dependency_pins_table("", table, &mut out)?;
            Ok(out)
        }
        Some(other) => Err(serde::de::Error::custom(format!(
            "expected a table for dependency_pins, found {}",
            other.type_str()
        ))),
    }
}

impl PinnedReleaseConfig {
    /// Parses `pinned_at` into a UTC `DateTime`.
    pub fn parsed_pinned_at(&self) -> Result<DateTime<Utc>> {
        parse_timestamp_utc(&self.pinned_at)
    }
}

/// QEMU, container, or test-suite verification result recorded for a component release or rolling update.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerificationRecord {
    pub passed: bool,
    #[serde(default)]
    pub exit_code: i32,
    #[serde(default)]
    pub command: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logs_excerpt: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_opt_datetime_string"
    )]
    pub verified_at: Option<String>,
}

impl VerificationRecord {
    /// Parses `verified_at` into an optional UTC `DateTime`.
    pub fn parsed_verified_at(&self) -> Result<Option<DateTime<Utc>>> {
        match self.verified_at.as_deref() {
            Some(s) if !s.trim().is_empty() => Ok(Some(parse_timestamp_utc(s)?)),
            _ => Ok(None),
        }
    }
}

/// Entry tracking an upstream rolling-release package/kernel update and its verification status.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RollingUpdateEntry {
    #[serde(default, alias = "name", alias = "package")]
    pub dependency_name: String,
    #[serde(default = "default_dependency_kind")]
    pub kind: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        alias = "from_version"
    )]
    pub previous_version: Option<String>,
    #[serde(default, alias = "to_version", alias = "version")]
    pub new_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub upstream_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_script_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "item_id")]
    pub tracked_item_id: Option<String>,
    #[serde(default = "default_rolling_status", alias = "status")]
    pub status: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_opt_datetime_string"
    )]
    pub recorded_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "notes")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_run: Option<VerificationRecord>,
}

impl RollingUpdateEntry {
    /// Parses `recorded_at` into an optional UTC `DateTime`.
    pub fn parsed_recorded_at(&self) -> Result<Option<DateTime<Utc>>> {
        match self.recorded_at.as_deref() {
            Some(s) if !s.trim().is_empty() => Ok(Some(parse_timestamp_utc(s)?)),
            _ => Ok(None),
        }
    }
}

fn default_relative_path() -> String {
    ".".to_string()
}

fn default_component_type_str() -> String {
    "custom".to_string()
}

fn default_dependency_kind() -> String {
    "cargo".to_string()
}

fn default_rolling_status() -> String {
    "pending".to_string()
}
