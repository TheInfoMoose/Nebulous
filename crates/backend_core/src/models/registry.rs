use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::crypto::nebulous_config_dir;
use crate::error::{Result, TrackerError};
use crate::models::{deserialize_datetime_string, parse_timestamp_utc};
use crate::storage::{atomic_write_str, TrackerStorage};

/// Default filename for the global multi-project registry inside [`nebulous_config_dir()`].
pub const GLOBAL_PROJECTS_FILE_NAME: &str = "projects.toml";

static REGISTRY_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn registry_lock() -> &'static Mutex<()> {
    REGISTRY_LOCK.get_or_init(|| Mutex::new(()))
}

/// Returns the default cross-platform path to `<nebulous_config_dir>/projects.toml`.
pub fn global_registry_path() -> PathBuf {
    nebulous_config_dir().join(GLOBAL_PROJECTS_FILE_NAME)
}

/// Resolves a `projects.toml` file path from either a directory or an explicit `.toml` file path.
pub fn resolve_registry_file_path(path_or_dir: impl AsRef<Path>) -> PathBuf {
    let p = path_or_dir.as_ref();
    if p.extension() == Some(OsStr::new("toml")) || p.file_name() == Some(OsStr::new(GLOBAL_PROJECTS_FILE_NAME)) {
        p.to_path_buf()
    } else {
        p.join(GLOBAL_PROJECTS_FILE_NAME)
    }
}

/// Cross-platform global project registry persisted at `<nebulous_config_dir>/projects.toml`.
///
/// Note on field ordering: `active_project_id` precedes `projects` (`[[projects]]`) so
/// `toml::to_string_pretty` always serializes top-level scalars before array-of-tables.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GlobalProjectRegistry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_project_id: Option<String>,
    #[serde(default)]
    pub projects: Vec<RegisteredProject>,
}

/// Single registered workspace entry in `projects.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredProject {
    pub id: String,
    pub name: String,
    pub root_path: String,
    pub tracker_path: String,
    #[serde(default = "default_branch_name")]
    pub default_branch: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_phase: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_datetime_string")]
    pub last_opened_at: String,
}

impl RegisteredProject {
    /// Parses `last_opened_at` into `DateTime<Utc>`.
    pub fn parsed_last_opened_at(&self) -> Result<DateTime<Utc>> {
        parse_timestamp_utc(&self.last_opened_at)
    }
}

impl GlobalProjectRegistry {
    /// Deserializes a [`GlobalProjectRegistry`] from a TOML string.
    pub fn from_toml_str(input: &str) -> Result<Self> {
        Ok(toml::from_str(input)?)
    }

    /// Serializes this [`GlobalProjectRegistry`] into a formatted TOML string.
    pub fn to_toml_string(&self) -> Result<String> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Returns the currently active [`RegisteredProject`] if `active_project_id` matches an entry.
    pub fn active_project(&self) -> Option<&RegisteredProject> {
        let active_id = self.active_project_id.as_deref()?;
        self.projects.iter().find(|p| p.id == active_id)
    }

    /// Looks up the index of a registered project with deterministic priority:
    /// 1. Exact or case-insensitive `id` match
    /// 2. Normalized or canonicalized `root_path` / `tracker_path` match
    /// 3. Case-insensitive `name` match
    fn find_project_index(&self, query: &str) -> Option<usize> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return None;
        }
        let lower = trimmed.to_ascii_lowercase();

        if let Some(idx) = self.projects.iter().position(|p| p.id == trimmed) {
            return Some(idx);
        }
        if let Some(idx) = self
            .projects
            .iter()
            .position(|p| p.id.to_ascii_lowercase() == lower)
        {
            return Some(idx);
        }

        let norm_query = normalize_query_path_str(trimmed);
        let canonical_query = Path::new(trimmed)
            .exists()
            .then(|| fs::canonicalize(trimmed).ok())
            .flatten()
            .map(|p| normalize_display_path(&p));

        if let Some(idx) = self.projects.iter().position(|p| {
            p.root_path == trimmed
                || p.tracker_path == trimmed
                || (!norm_query.is_empty()
                    && (p.root_path == norm_query || p.tracker_path == norm_query))
                || canonical_query.as_deref().is_some_and(|canon| {
                    p.root_path == canon || p.tracker_path == canon
                })
        }) {
            return Some(idx);
        }

        self.projects
            .iter()
            .position(|p| p.name.to_ascii_lowercase() == lower)
    }

    /// Looks up a registered project by `id`, `root_path` / `tracker_path`, or case-insensitive `name`.
    pub fn find_project(&self, query: &str) -> Option<&RegisteredProject> {
        let idx = self.find_project_index(query)?;
        self.projects.get(idx)
    }

    /// Looks up a mutable registered project by `id`, `root_path` / `tracker_path`, or case-insensitive `name`.
    pub fn find_project_mut(&mut self, query: &str) -> Option<&mut RegisteredProject> {
        let idx = self.find_project_index(query)?;
        self.projects.get_mut(idx)
    }

    /// Registers or updates a project in this in-memory [`GlobalProjectRegistry`].
    pub fn register_project(
        &mut self,
        root_or_tracker_path: impl AsRef<Path>,
        name_override: Option<&str>,
        tags: Vec<String>,
        set_active: bool,
    ) -> Result<RegisteredProject> {
        let raw_path = root_or_tracker_path.as_ref();
        let is_project_toml =
            raw_path.is_file() && raw_path.file_name() == Some(OsStr::new("project.toml"));
        if !raw_path.exists() || (!raw_path.is_dir() && !is_project_toml) {
            return Err(TrackerError::NotFound {
                kind: "ProjectDirectory",
                id: raw_path.display().to_string(),
            });
        }

        let target_dir = if is_project_toml {
            raw_path.parent().unwrap_or(raw_path)
        } else {
            raw_path
        };

        let canonical_input =
            fs::canonicalize(target_dir).unwrap_or_else(|_| target_dir.to_path_buf());
        let storage = TrackerStorage::new(&canonical_input);
        let project_root_buf = storage.project_root();
        let tracker_dir_buf = storage.tracker_dir().to_path_buf();

        let root_path_str = normalize_display_path(&project_root_buf);
        let tracker_path_str = normalize_display_path(&tracker_dir_buf);

        let fallback_dir_name = project_root_buf
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("project")
            .to_string();

        let (inferred_name, has_manifest_name, default_branch, active_phase) =
            match storage.read_project() {
                Ok(manifest) => (
                    manifest.project.name,
                    true,
                    manifest.git.branch,
                    Some(manifest.project.active_phase),
                ),
                Err(_) => (fallback_dir_name.clone(), false, default_branch_name(), None),
            };

        let explicit_name = name_override
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let project_name = explicit_name.clone().unwrap_or(inferred_name);

        let base_id = slugify_project_id(&project_name);
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let cleaned_tags = deduplicate_tags(tags);

        if let Some(existing) = self
            .projects
            .iter_mut()
            .find(|p| p.root_path == root_path_str || p.tracker_path == tracker_path_str)
        {
            if explicit_name.is_some()
                || existing.name.trim().is_empty()
                || (has_manifest_name && existing.name == fallback_dir_name)
            {
                existing.name = project_name;
            }
            existing.root_path = root_path_str;
            existing.tracker_path = tracker_path_str;
            existing.default_branch = default_branch;
            if active_phase.is_some() {
                existing.active_phase = active_phase;
            }
            if !cleaned_tags.is_empty() {
                existing.tags = cleaned_tags;
            }
            existing.last_opened_at = now;
            let updated = existing.clone();
            if set_active || self.active_project_id.is_none() {
                self.active_project_id = Some(updated.id.clone());
            }
            return Ok(updated);
        }

        let unique_id = allocate_unique_project_id(&self.projects, &base_id);
        let entry = RegisteredProject {
            id: unique_id.clone(),
            name: project_name,
            root_path: root_path_str,
            tracker_path: tracker_path_str,
            default_branch,
            active_phase,
            tags: cleaned_tags,
            last_opened_at: now,
        };

        self.projects.push(entry.clone());
        if set_active || self.active_project_id.is_none() {
            self.active_project_id = Some(unique_id);
        }

        Ok(entry)
    }

    /// Removes a project by `id`, `name`, or `root_path` from this registry.
    ///
    /// If the removed project was active, `active_project_id` falls back to the first remaining project.
    pub fn remove_project(&mut self, id_or_name: &str) -> Result<RegisteredProject> {
        let trimmed = id_or_name.trim();
        let idx = self
            .find_project_index(trimmed)
            .ok_or_else(|| TrackerError::NotFound {
                kind: "Project",
                id: trimmed.to_string(),
            })?;

        let removed = self.projects.remove(idx);
        if self.active_project_id.as_deref() == Some(&removed.id) {
            self.active_project_id = self.projects.first().map(|p| p.id.clone());
        }
        Ok(removed)
    }

    /// Sets the active project by `id`, `name`, or `root_path` and updates its `last_opened_at` timestamp.
    pub fn set_active_project(&mut self, id_or_name: &str) -> Result<RegisteredProject> {
        let trimmed = id_or_name.trim();
        let edited = self
            .find_project_mut(trimmed)
            .ok_or_else(|| TrackerError::NotFound {
                kind: "Project",
                id: trimmed.to_string(),
            })?;
        edited.last_opened_at = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let cloned = edited.clone();
        self.active_project_id = Some(cloned.id.clone());
        Ok(cloned)
    }
}

/// Loads the global [`GlobalProjectRegistry`] from `<nebulous_config_dir>/projects.toml`.
///
/// Returns an empty default registry if `projects.toml` does not exist yet.
pub fn load_global_registry() -> Result<GlobalProjectRegistry> {
    load_global_registry_from(global_registry_path())
}

/// Loads a [`GlobalProjectRegistry`] from an explicit `projects.toml` path or config directory.
pub fn load_global_registry_from(path_or_dir: impl AsRef<Path>) -> Result<GlobalProjectRegistry> {
    let path = resolve_registry_file_path(path_or_dir);
    let _guard = registry_lock()
        .lock()
        .map_err(|e| TrackerError::ValidationError(e.to_string()))?;
    load_registry_unlocked(&path)
}

/// Atomically saves `registry` to `<nebulous_config_dir>/projects.toml`.
pub fn save_global_registry(registry: &GlobalProjectRegistry) -> Result<()> {
    save_global_registry_to(global_registry_path(), registry)
}

/// Atomically saves `registry` to an explicit `projects.toml` path or config directory.
pub fn save_global_registry_to(
    path_or_dir: impl AsRef<Path>,
    registry: &GlobalProjectRegistry,
) -> Result<()> {
    let path = resolve_registry_file_path(path_or_dir);
    let _guard = registry_lock()
        .lock()
        .map_err(|e| TrackerError::ValidationError(e.to_string()))?;
    save_registry_unlocked(&path, registry)
}

/// Registers a project in `<nebulous_config_dir>/projects.toml` and persists the updated registry.
pub fn register_project(
    root_or_tracker_path: impl AsRef<Path>,
    name_override: Option<&str>,
    tags: Vec<String>,
    set_active: bool,
) -> Result<RegisteredProject> {
    register_project_in(
        global_registry_path(),
        root_or_tracker_path,
        name_override,
        tags,
        set_active,
    )
}

/// Registers a project in the registry at `registry_path_or_dir` and persists the updated registry.
pub fn register_project_in(
    registry_path_or_dir: impl AsRef<Path>,
    root_or_tracker_path: impl AsRef<Path>,
    name_override: Option<&str>,
    tags: Vec<String>,
    set_active: bool,
) -> Result<RegisteredProject> {
    let path = resolve_registry_file_path(registry_path_or_dir);
    let _guard = registry_lock()
        .lock()
        .map_err(|e| TrackerError::ValidationError(e.to_string()))?;
    let mut registry = load_registry_unlocked(&path)?;
    let entry = registry.register_project(root_or_tracker_path, name_override, tags, set_active)?;
    save_registry_unlocked(&path, &registry)?;
    Ok(entry)
}

/// Removes a project by `id`, `name`, or `root_path` from `<nebulous_config_dir>/projects.toml`.
pub fn remove_project(id_or_name: &str) -> Result<RegisteredProject> {
    remove_project_in(global_registry_path(), id_or_name)
}

/// Removes a project by `id`, `name`, or `root_path` from the registry at `registry_path_or_dir`.
pub fn remove_project_in(
    registry_path_or_dir: impl AsRef<Path>,
    id_or_name: &str,
) -> Result<RegisteredProject> {
    let path = resolve_registry_file_path(registry_path_or_dir);
    let _guard = registry_lock()
        .lock()
        .map_err(|e| TrackerError::ValidationError(e.to_string()))?;
    let mut registry = load_registry_unlocked(&path)?;
    let removed = registry.remove_project(id_or_name)?;
    save_registry_unlocked(&path, &registry)?;
    Ok(removed)
}

/// Sets the active project by `id`, `name`, or `root_path` in `<nebulous_config_dir>/projects.toml`.
pub fn set_active_project(id_or_name: &str) -> Result<RegisteredProject> {
    set_active_project_in(global_registry_path(), id_or_name)
}

/// Sets the active project by `id`, `name`, or `root_path` in the registry at `registry_path_or_dir`.
pub fn set_active_project_in(
    registry_path_or_dir: impl AsRef<Path>,
    id_or_name: &str,
) -> Result<RegisteredProject> {
    let path = resolve_registry_file_path(registry_path_or_dir);
    let _guard = registry_lock()
        .lock()
        .map_err(|e| TrackerError::ValidationError(e.to_string()))?;
    let mut registry = load_registry_unlocked(&path)?;
    let active = registry.set_active_project(id_or_name)?;
    save_registry_unlocked(&path, &registry)?;
    Ok(active)
}

fn load_registry_unlocked(path: &Path) -> Result<GlobalProjectRegistry> {
    if !path.is_file() {
        return Ok(GlobalProjectRegistry::default());
    }
    let raw = fs::read_to_string(path).map_err(|source| TrackerError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    if raw.trim().is_empty() {
        return Ok(GlobalProjectRegistry::default());
    }
    GlobalProjectRegistry::from_toml_str(&raw).map_err(|err| match err {
        TrackerError::TomlDeserialize { source, .. } => TrackerError::TomlDeserialize {
            path: Some(path.to_path_buf()),
            source,
        },
        other => other,
    })
}

fn save_registry_unlocked(path: &Path, registry: &GlobalProjectRegistry) -> Result<()> {
    let serialized = registry.to_toml_string()?;
    atomic_write_str(path, &serialized)
}

fn normalize_display_path(path: &Path) -> String {
    normalize_query_path_str(&path.to_string_lossy())
}

fn normalize_query_path_str(raw: &str) -> String {
    let mut s = raw.trim().replace('\\', "/");
    // Strip Windows extended-length prefix `//?/` if produced by `fs::canonicalize`
    if let Some(stripped) = s.strip_prefix("//?/UNC/") {
        s = format!("//{stripped}");
    } else if let Some(stripped) = s.strip_prefix("//?/") {
        s = stripped.to_string();
    }
    while s.len() > 1 && s.ends_with('/') && !s.ends_with(":/") {
        s.pop();
    }
    s
}

pub(crate) fn slugify_project_id(name: &str) -> String {
    let mut slug = String::with_capacity(name.len());
    let mut prev_underscore = false;
    for ch in name.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            prev_underscore = false;
        } else if !prev_underscore && !slug.is_empty() {
            slug.push('_');
            prev_underscore = true;
        }
    }
    while slug.ends_with('_') {
        slug.pop();
    }
    if slug.is_empty() {
        "project".to_string()
    } else {
        slug
    }
}

fn allocate_unique_project_id(existing: &[RegisteredProject], base_id: &str) -> String {
    if !existing.iter().any(|p| p.id == base_id) {
        return base_id.to_string();
    }
    let mut suffix = 2u32;
    loop {
        let candidate = format!("{base_id}_{suffix}");
        if !existing.iter().any(|p| p.id == candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn deduplicate_tags(tags: Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    for t in tags {
        let trimmed = t.trim().to_string();
        if !trimmed.is_empty() && !out.contains(&trimmed) {
            out.push(trimmed);
        }
    }
    out
}

fn default_branch_name() -> String {
    "main".to_string()
}
