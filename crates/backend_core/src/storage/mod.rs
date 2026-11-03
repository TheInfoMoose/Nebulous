pub mod atomic;
pub mod id;
pub mod reconcile;

use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};

use chrono::Utc;

pub use atomic::{atomic_write_bytes, atomic_write_str};
pub use id::parse_item_id_number;
pub use reconcile::ReconcileReport;

use crate::error::{Result, TrackerError};
use crate::models::{
    ActiveTimer, GitConfig, ItemDetail, ItemManifest, ItemStub, ItemType, MetricsConfig,
    PhaseManifest, PhaseMetadata, PhaseRegistry, PhaseRegistryEntry, PhaseStatus, PhaseSummary,
    ProjectCounters, ProjectManifest, ProjectMetadata, ScopeItems, ScopeManifest, ScopeMetadata,
    ScopeRegistry, ScopeRegistryEntry,
};
use crate::storage::reconcile::infer_default_scope_type;

/// Top-level in-memory index loaded at boot without reading any `items/*/desc.md` or `items/*/item.toml`.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackerIndex {
    pub project: ProjectManifest,
    pub phases: Vec<PhaseIndex>,
}

impl TrackerIndex {
    /// Returns the currently active [`PhaseIndex`] according to `project.active_phase`.
    pub fn active_phase(&self) -> Option<&PhaseIndex> {
        self.phase(&self.project.project.active_phase)
    }

    /// Looks up a loaded [`PhaseIndex`] by phase ID.
    pub fn phase(&self, phase_id: &str) -> Option<&PhaseIndex> {
        self.phases.iter().find(|p| p.phase_id == phase_id)
    }

    /// Looks up a loaded [`ScopeIndex`] by phase ID and scope ID.
    pub fn scope(&self, phase_id: &str, scope_id: &str) -> Option<&ScopeIndex> {
        self.phase(phase_id)?
            .scopes
            .iter()
            .find(|s| s.scope_id == scope_id)
    }

    /// Searches all indexed phases and scopes for an item stub with `item_id`.
    pub fn find_item(&self, item_id: &str) -> Option<ItemLocation> {
        for phase in &self.phases {
            for scope in &phase.scopes {
                for bug in &scope.manifest.items.bugs {
                    if bug.id == item_id {
                        return Some(ItemLocation {
                            phase_id: phase.phase_id.clone(),
                            scope_id: scope.scope_id.clone(),
                            item_type: ItemType::Bug,
                            stub: bug.clone(),
                        });
                    }
                }
                for feat in &scope.manifest.items.features {
                    if feat.id == item_id {
                        return Some(ItemLocation {
                            phase_id: phase.phase_id.clone(),
                            scope_id: scope.scope_id.clone(),
                            item_type: ItemType::Feature,
                            stub: feat.clone(),
                        });
                    }
                }
            }
        }
        None
    }

    /// Returns all indexed item locations across all phases and scopes.
    pub fn all_items(&self) -> Vec<ItemLocation> {
        let mut out = Vec::new();
        for phase in &self.phases {
            for scope in &phase.scopes {
                for bug in &scope.manifest.items.bugs {
                    out.push(ItemLocation {
                        phase_id: phase.phase_id.clone(),
                        scope_id: scope.scope_id.clone(),
                        item_type: ItemType::Bug,
                        stub: bug.clone(),
                    });
                }
                for feat in &scope.manifest.items.features {
                    out.push(ItemLocation {
                        phase_id: phase.phase_id.clone(),
                        scope_id: scope.scope_id.clone(),
                        item_type: ItemType::Feature,
                        stub: feat.clone(),
                    });
                }
            }
        }
        out
    }
}

/// Indexed phase entry loaded from Tier 2 `phase.toml` and its child Tier 3 `scope.toml` files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseIndex {
    pub phase_id: String,
    pub manifest: PhaseManifest,
    pub scopes: Vec<ScopeIndex>,
}

impl PhaseIndex {
    /// Looks up a [`ScopeIndex`] within this phase by `scope_id`.
    pub fn scope(&self, scope_id: &str) -> Option<&ScopeIndex> {
        self.scopes.iter().find(|s| s.scope_id == scope_id)
    }
}

/// Indexed domain scope entry loaded from Tier 3 `scope.toml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeIndex {
    pub scope_id: String,
    pub manifest: ScopeManifest,
}

/// Location and lightweight stub metadata for an indexed item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemLocation {
    pub phase_id: String,
    pub scope_id: String,
    pub item_type: ItemType,
    pub stub: ItemStub,
}

/// Core 4-tier `.tracker/` persistence, lazy loading, ID allocation, and index reconciliation engine.
#[derive(Debug, Clone)]
pub struct TrackerStorage {
    tracker_dir: PathBuf,
}

impl TrackerStorage {
    /// Creates a `TrackerStorage` instance pointing at `.tracker/`.
    ///
    /// Accepts either the `.tracker` directory itself or its parent workspace root directory.
    pub fn new(root_or_tracker_dir: impl AsRef<Path>) -> Self {
        let path = root_or_tracker_dir.as_ref();
        let tracker_dir = if path.file_name() == Some(OsStr::new(".tracker"))
            || path.join("project.toml").is_file()
        {
            path.to_path_buf()
        } else {
            path.join(".tracker")
        };
        Self { tracker_dir }
    }

    /// Creates a `TrackerStorage` directly from an explicit `.tracker` directory path.
    pub fn from_tracker_dir(tracker_dir: impl Into<PathBuf>) -> Self {
        Self {
            tracker_dir: tracker_dir.into(),
        }
    }

    /// Discovers a `.tracker/` directory by walking up ancestor directories from `start_dir`.
    pub fn discover(start_dir: impl AsRef<Path>) -> Result<Self> {
        let start = start_dir.as_ref();
        let mut current = Some(start);
        while let Some(dir) = current {
            if dir.file_name() == Some(OsStr::new(".tracker")) && dir.join("project.toml").is_file()
            {
                return Ok(Self {
                    tracker_dir: dir.to_path_buf(),
                });
            }
            let candidate = dir.join(".tracker");
            if candidate.join("project.toml").is_file() {
                return Ok(Self {
                    tracker_dir: candidate,
                });
            }
            current = dir.parent();
        }
        Err(TrackerError::TrackerRootNotFound(start.to_path_buf()))
    }

    /// Initializes a new `.tracker/` directory hierarchy at `root_or_tracker_dir` with a default
    /// phase and `inbox` scope.
    pub fn init(
        root_or_tracker_dir: impl AsRef<Path>,
        project_name: impl Into<String>,
        initial_phase_id: impl Into<String>,
        initial_phase_title: impl Into<String>,
    ) -> Result<Self> {
        let storage = Self::new(root_or_tracker_dir);
        let phase_id = initial_phase_id.into();
        let phase_title = initial_phase_title.into();
        validate_identifier("Phase", &phase_id)?;

        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let today = Utc::now().format("%Y-%m-%d").to_string();

        let project = ProjectManifest {
            project: ProjectMetadata {
                name: project_name.into(),
                version: "0.1.0".to_string(),
                active_phase: phase_id.clone(),
                default_scope: "inbox".to_string(),
                created_at: now,
            },
            counters: ProjectCounters::default(),
            phases: PhaseRegistry {
                registry: vec![PhaseRegistryEntry {
                    id: phase_id.clone(),
                    title: phase_title.clone(),
                    path: format!("phases/{phase_id}"),
                }],
            },
            metrics: MetricsConfig::default(),
            git: GitConfig::default(),
        };

        let phase = PhaseManifest {
            phase: PhaseMetadata {
                id: phase_id.clone(),
                title: phase_title,
                status: PhaseStatus::InProgress,
                start_date: Some(today),
                end_date: None,
            },
            scopes: ScopeRegistry {
                manifest: vec![ScopeRegistryEntry {
                    id: "inbox".to_string(),
                    scope_type: "inbox".to_string(),
                    path: "domains/inbox".to_string(),
                }],
            },
            summary: PhaseSummary::default(),
        };

        let inbox_scope = ScopeManifest {
            scope: ScopeMetadata {
                id: "inbox".to_string(),
                target: ".tracker".to_string(),
                tech_stack: Vec::new(),
                description: Some("Default unscoped triage inbox".to_string()),
            },
            items: ScopeItems::default(),
        };

        storage.write_project(&project)?;
        storage.write_phase(&phase_id, &phase)?;
        storage.write_scope(&phase_id, "inbox", &inbox_scope)?;
        fs::create_dir_all(storage.items_dir(&phase_id, "inbox")?).map_err(|source| {
            TrackerError::Io {
                path: storage.tracker_dir.clone(),
                source,
            }
        })?;

        Ok(storage)
    }

    /// Returns the path to the `.tracker/` directory.
    pub fn tracker_dir(&self) -> &Path {
        &self.tracker_dir
    }

    /// Returns the workspace root directory (parent of `.tracker/`).
    pub fn project_root(&self) -> PathBuf {
        self.tracker_dir
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.tracker_dir.clone())
    }

    /// Returns `.tracker/project.toml`.
    pub fn project_toml_path(&self) -> PathBuf {
        self.tracker_dir.join("project.toml")
    }

    /// Returns `.tracker/.timer.toml`.
    pub fn timer_toml_path(&self) -> PathBuf {
        self.tracker_dir.join(".timer.toml")
    }

    /// Returns `.tracker/phases`.
    pub fn phases_dir(&self) -> PathBuf {
        self.tracker_dir.join("phases")
    }

    /// Returns `.tracker/phases/<phase_id>` (or the custom relative path configured in `project.toml`).
    pub fn phase_dir(&self, phase_id: &str) -> Result<PathBuf> {
        validate_identifier("Phase", phase_id)?;
        if let Ok(project) = self.read_project() {
            if let Some(entry) = project.phases.registry.iter().find(|p| p.id == phase_id) {
                return self.resolve_phase_path(phase_id, Some(&entry.path));
            }
        }
        Ok(self.phases_dir().join(phase_id))
    }

    /// Returns `.tracker/phases/<phase_id>/phase.toml`.
    pub fn phase_toml_path(&self, phase_id: &str) -> Result<PathBuf> {
        Ok(self.phase_dir(phase_id)?.join("phase.toml"))
    }

    /// Returns `.tracker/phases/<phase_id>/domains/<scope_id>` (or the custom relative path configured in `phase.toml`).
    pub fn scope_dir(&self, phase_id: &str, scope_id: &str) -> Result<PathBuf> {
        validate_identifier("Scope", scope_id)?;
        let phase_dir = self.phase_dir(phase_id)?;
        let phase_toml = phase_dir.join("phase.toml");
        if phase_toml.is_file() {
            if let Ok(phase_manifest) = self.read_phase_from_path(&phase_toml) {
                if let Some(entry) = phase_manifest
                    .scopes
                    .manifest
                    .iter()
                    .find(|s| s.id == scope_id)
                {
                    return self.resolve_scope_path(&phase_dir, scope_id, Some(&entry.path));
                }
            }
        }
        Ok(phase_dir.join("domains").join(scope_id))
    }

    /// Returns `.tracker/phases/<phase_id>/domains/<scope_id>/scope.toml`.
    pub fn scope_toml_path(&self, phase_id: &str, scope_id: &str) -> Result<PathBuf> {
        Ok(self.scope_dir(phase_id, scope_id)?.join("scope.toml"))
    }

    /// Returns `.tracker/phases/<phase_id>/domains/<scope_id>/items`.
    pub fn items_dir(&self, phase_id: &str, scope_id: &str) -> Result<PathBuf> {
        Ok(self.scope_dir(phase_id, scope_id)?.join("items"))
    }

    /// Returns `.tracker/phases/<phase_id>/domains/<scope_id>/items/<item_id>`.
    pub fn item_dir(&self, phase_id: &str, scope_id: &str, item_id: &str) -> Result<PathBuf> {
        validate_identifier("Item", item_id)?;
        Ok(self.items_dir(phase_id, scope_id)?.join(item_id))
    }

    /// Returns `.tracker/phases/<phase_id>/domains/<scope_id>/items/<item_id>/item.toml`.
    pub fn item_toml_path(&self, phase_id: &str, scope_id: &str, item_id: &str) -> Result<PathBuf> {
        Ok(self
            .item_dir(phase_id, scope_id, item_id)?
            .join("item.toml"))
    }

    /// Returns `.tracker/phases/<phase_id>/domains/<scope_id>/items/<item_id>/desc.md`.
    pub fn item_desc_path(&self, phase_id: &str, scope_id: &str, item_id: &str) -> Result<PathBuf> {
        Ok(self.item_dir(phase_id, scope_id, item_id)?.join("desc.md"))
    }

    pub(crate) fn resolve_phase_path(
        &self,
        phase_id: &str,
        configured_rel_path: Option<&str>,
    ) -> Result<PathBuf> {
        validate_identifier("Phase", phase_id)?;
        if let Some(rel) = configured_rel_path {
            if !rel.trim().is_empty() {
                return resolve_safe_relative_path(&self.tracker_dir, rel);
            }
        }
        Ok(self.phases_dir().join(phase_id))
    }

    pub(crate) fn resolve_scope_path(
        &self,
        phase_dir: &Path,
        scope_id: &str,
        configured_rel_path: Option<&str>,
    ) -> Result<PathBuf> {
        validate_identifier("Scope", scope_id)?;
        if let Some(rel) = configured_rel_path {
            if !rel.trim().is_empty() {
                return resolve_safe_relative_path(phase_dir, rel);
            }
        }
        Ok(phase_dir.join("domains").join(scope_id))
    }

    /// Reads and parses Tier 1 `.tracker/project.toml`.
    pub fn read_project(&self) -> Result<ProjectManifest> {
        let path = self.project_toml_path();
        if !path.is_file() {
            return Err(TrackerError::TrackerRootNotFound(self.tracker_dir.clone()));
        }
        let raw = fs::read_to_string(&path).map_err(|source| TrackerError::Io {
            path: path.clone(),
            source,
        })?;
        ProjectManifest::from_toml_str(&raw).map_err(|err| attach_toml_path(err, path))
    }

    /// Atomically writes Tier 1 `.tracker/project.toml`.
    pub fn write_project(&self, manifest: &ProjectManifest) -> Result<()> {
        let serialized = manifest.to_toml_string()?;
        atomic_write_str(&self.project_toml_path(), &serialized)
    }

    /// Reads and parses Tier 2 `.tracker/phases/<phase_id>/phase.toml`.
    pub fn read_phase(&self, phase_id: &str) -> Result<PhaseManifest> {
        let path = self.phase_toml_path(phase_id)?;
        if !path.is_file() {
            return Err(TrackerError::NotFound {
                kind: "Phase",
                id: phase_id.to_string(),
            });
        }
        self.read_phase_from_path(&path)
    }

    pub(crate) fn read_phase_from_path(&self, path: &Path) -> Result<PhaseManifest> {
        let raw = fs::read_to_string(path).map_err(|source| TrackerError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        PhaseManifest::from_toml_str(&raw).map_err(|err| attach_toml_path(err, path.to_path_buf()))
    }

    /// Atomically writes Tier 2 `.tracker/phases/<phase_id>/phase.toml`.
    pub fn write_phase(&self, phase_id: &str, manifest: &PhaseManifest) -> Result<()> {
        let path = self.phase_toml_path(phase_id)?;
        self.write_phase_to_path(&path, manifest)
    }

    pub(crate) fn write_phase_to_path(&self, path: &Path, manifest: &PhaseManifest) -> Result<()> {
        let serialized = manifest.to_toml_string()?;
        atomic_write_str(path, &serialized)
    }

    /// Reads and parses Tier 3 `.tracker/phases/<phase_id>/domains/<scope_id>/scope.toml`.
    pub fn read_scope(&self, phase_id: &str, scope_id: &str) -> Result<ScopeManifest> {
        let path = self.scope_toml_path(phase_id, scope_id)?;
        if !path.is_file() {
            return Err(TrackerError::NotFound {
                kind: "Scope",
                id: scope_id.to_string(),
            });
        }
        self.read_scope_from_path(&path)
    }

    pub(crate) fn read_scope_from_path(&self, path: &Path) -> Result<ScopeManifest> {
        let raw = fs::read_to_string(path).map_err(|source| TrackerError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        ScopeManifest::from_toml_str(&raw).map_err(|err| attach_toml_path(err, path.to_path_buf()))
    }

    /// Atomically writes Tier 3 `.tracker/phases/<phase_id>/domains/<scope_id>/scope.toml`.
    pub fn write_scope(
        &self,
        phase_id: &str,
        scope_id: &str,
        manifest: &ScopeManifest,
    ) -> Result<()> {
        let path = self.scope_toml_path(phase_id, scope_id)?;
        self.write_scope_to_path(&path, manifest)
    }

    pub(crate) fn write_scope_to_path(&self, path: &Path, manifest: &ScopeManifest) -> Result<()> {
        let serialized = manifest.to_toml_string()?;
        atomic_write_str(path, &serialized)
    }

    /// Reads and parses Tier 4 `items/<item_id>/item.toml`.
    pub fn read_item_manifest(
        &self,
        phase_id: &str,
        scope_id: &str,
        item_id: &str,
    ) -> Result<ItemManifest> {
        let path = self.item_toml_path(phase_id, scope_id, item_id)?;
        if !path.is_file() {
            return Err(TrackerError::NotFound {
                kind: "Item",
                id: item_id.to_string(),
            });
        }
        let raw = fs::read_to_string(&path).map_err(|source| TrackerError::Io {
            path: path.clone(),
            source,
        })?;
        ItemManifest::from_toml_str(&raw).map_err(|err| attach_toml_path(err, path))
    }

    /// Atomically writes Tier 4 `items/<item_id>/item.toml`.
    pub fn write_item_manifest(
        &self,
        phase_id: &str,
        scope_id: &str,
        item_id: &str,
        manifest: &ItemManifest,
    ) -> Result<()> {
        let path = self.item_toml_path(phase_id, scope_id, item_id)?;
        let serialized = manifest.to_toml_string()?;
        atomic_write_str(&path, &serialized)
    }

    /// Reads Tier 4 `items/<item_id>/desc.md`.
    pub fn read_item_desc(&self, phase_id: &str, scope_id: &str, item_id: &str) -> Result<String> {
        let path = self.item_desc_path(phase_id, scope_id, item_id)?;
        fs::read_to_string(&path).map_err(|source| TrackerError::Io { path, source })
    }

    /// Atomically writes Tier 4 `items/<item_id>/desc.md`.
    pub fn write_item_desc(
        &self,
        phase_id: &str,
        scope_id: &str,
        item_id: &str,
        description_markdown: &str,
    ) -> Result<()> {
        let path = self.item_desc_path(phase_id, scope_id, item_id)?;
        atomic_write_str(&path, description_markdown)
    }

    /// Atomically writes both `item.toml` and `desc.md` for an [`ItemDetail`].
    pub fn write_item_detail(
        &self,
        phase_id: &str,
        scope_id: &str,
        detail: &ItemDetail,
    ) -> Result<()> {
        let item_id = &detail.manifest.item.id;
        self.write_item_manifest(phase_id, scope_id, item_id, &detail.manifest)?;
        self.write_item_desc(phase_id, scope_id, item_id, &detail.description_markdown)?;
        Ok(())
    }

    /// Reads `.tracker/.timer.toml` if an active timer is currently running.
    pub fn read_active_timer(&self) -> Result<Option<ActiveTimer>> {
        let path = self.timer_toml_path();
        if !path.is_file() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&path).map_err(|source| TrackerError::Io {
            path: path.clone(),
            source,
        })?;
        let timer = ActiveTimer::from_toml_str(&raw).map_err(|err| attach_toml_path(err, path))?;
        Ok(Some(timer))
    }

    /// Atomically writes `.tracker/.timer.toml`.
    pub fn write_active_timer(&self, timer: &ActiveTimer) -> Result<()> {
        let serialized = timer.to_toml_string()?;
        atomic_write_str(&self.timer_toml_path(), &serialized)
    }

    /// Removes `.tracker/.timer.toml` if it exists.
    pub fn clear_active_timer(&self) -> Result<()> {
        let path = self.timer_toml_path();
        if path.exists() {
            fs::remove_file(&path).map_err(|source| TrackerError::Io { path, source })?;
        }
        Ok(())
    }

    /// Loads only the top-level index files (`project.toml`, `phase.toml`, and `scope.toml`)
    /// without reading any `items/*/desc.md` or `items/*/item.toml` files.
    pub fn load_index(&self) -> Result<TrackerIndex> {
        let project = self.read_project()?;
        let mut phases = Vec::with_capacity(project.phases.registry.len());

        for phase_entry in &project.phases.registry {
            let phase_dir = self.resolve_phase_path(&phase_entry.id, Some(&phase_entry.path))?;
            let phase_toml_path = phase_dir.join("phase.toml");
            if !phase_toml_path.is_file() {
                return Err(TrackerError::NotFound {
                    kind: "Phase",
                    id: phase_entry.id.clone(),
                });
            }
            let phase_manifest = self.read_phase_from_path(&phase_toml_path)?;

            let mut scopes = Vec::with_capacity(phase_manifest.scopes.manifest.len());
            for scope_entry in &phase_manifest.scopes.manifest {
                let scope_dir =
                    self.resolve_scope_path(&phase_dir, &scope_entry.id, Some(&scope_entry.path))?;
                let scope_toml_path = scope_dir.join("scope.toml");
                if !scope_toml_path.is_file() {
                    return Err(TrackerError::NotFound {
                        kind: "Scope",
                        id: scope_entry.id.clone(),
                    });
                }
                let scope_manifest = self.read_scope_from_path(&scope_toml_path)?;
                scopes.push(ScopeIndex {
                    scope_id: scope_entry.id.clone(),
                    manifest: scope_manifest,
                });
            }

            phases.push(PhaseIndex {
                phase_id: phase_entry.id.clone(),
                manifest: phase_manifest,
                scopes,
            });
        }

        Ok(TrackerIndex { project, phases })
    }

    /// Lazily loads the full [`ItemDetail`] (`item.toml` + `desc.md`) for a specific item on demand.
    pub fn load_item_detail(
        &self,
        phase_id: &str,
        scope_id: &str,
        item_id: &str,
    ) -> Result<ItemDetail> {
        let manifest = self.read_item_manifest(phase_id, scope_id, item_id)?;
        let description_markdown = self.read_item_desc(phase_id, scope_id, item_id)?;
        Ok(ItemDetail {
            manifest,
            description_markdown,
        })
    }

    /// Locates an item by its ID across all phases and scopes (checking top-level indexes first,
    /// then falling back to a directory scan if not yet indexed).
    pub fn locate_item(&self, item_id: &str) -> Result<ItemLocation> {
        validate_identifier("Item", item_id)?;
        let index = self.load_index()?;
        if let Some(loc) = index.find_item(item_id) {
            return Ok(loc);
        }

        // Fallback: check on-disk items directories in case an item was created prior to index reconciliation
        for phase in &index.phases {
            for scope in &phase.scopes {
                let item_toml = self.item_toml_path(&phase.phase_id, &scope.scope_id, item_id)?;
                if item_toml.is_file() {
                    let manifest =
                        self.read_item_manifest(&phase.phase_id, &scope.scope_id, item_id)?;
                    return Ok(ItemLocation {
                        phase_id: phase.phase_id.clone(),
                        scope_id: scope.scope_id.clone(),
                        item_type: manifest.item.item_type,
                        stub: manifest.to_stub(),
                    });
                }
            }
        }

        Err(TrackerError::NotFound {
            kind: "Item",
            id: item_id.to_string(),
        })
    }

    /// Locates an item by ID and lazily loads its full [`ItemDetail`] (`item.toml` + `desc.md`).
    pub fn load_item_detail_by_id(&self, item_id: &str) -> Result<(ItemLocation, ItemDetail)> {
        let loc = self.locate_item(item_id)?;
        let detail = self.load_item_detail(&loc.phase_id, &loc.scope_id, item_id)?;
        Ok((loc, detail))
    }

    /// Resolves the target `(phase_id, scope_id)` for an operation, falling back to
    /// `project.active_phase` when `phase_id` is `None`/empty and `project.default_scope`
    /// (defaulting to `"inbox"`) when `scope_id` is `None`/empty.
    ///
    /// If the resolved scope is the project's `default_scope` (or `"inbox"`) and its directory
    /// does not yet exist in the target phase, it is automatically bootstrapped and registered
    /// in `phase.toml`.
    pub fn resolve_phase_and_scope(
        &self,
        phase_id: Option<&str>,
        scope_id: Option<&str>,
    ) -> Result<(String, String)> {
        let project = self.read_project()?;

        let resolved_phase = match phase_id.map(str::trim).filter(|s| !s.is_empty()) {
            Some(p) => {
                validate_identifier("Phase", p)?;
                p.to_string()
            }
            None => {
                let active = project.project.active_phase.trim();
                validate_identifier("Phase", active)?;
                active.to_string()
            }
        };

        // Ensure phase exists
        let mut phase_manifest = self.read_phase(&resolved_phase)?;

        let default_scope = {
            let trimmed = project.project.default_scope.trim();
            if trimmed.is_empty() {
                "inbox".to_string()
            } else {
                trimmed.to_string()
            }
        };

        let resolved_scope = match scope_id.map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => {
                validate_identifier("Scope", s)?;
                s.to_string()
            }
            None => {
                validate_identifier("Scope", &default_scope)?;
                default_scope.clone()
            }
        };

        let scope_toml = self.scope_toml_path(&resolved_phase, &resolved_scope)?;
        if !scope_toml.is_file() {
            if resolved_scope == default_scope || resolved_scope == "inbox" {
                let inbox_manifest = ScopeManifest {
                    scope: ScopeMetadata {
                        id: resolved_scope.clone(),
                        target: ".tracker".to_string(),
                        tech_stack: Vec::new(),
                        description: Some("Default unscoped triage inbox".to_string()),
                    },
                    items: ScopeItems::default(),
                };
                self.write_scope(&resolved_phase, &resolved_scope, &inbox_manifest)?;
                fs::create_dir_all(self.items_dir(&resolved_phase, &resolved_scope)?).map_err(
                    |source| TrackerError::Io {
                        path: scope_toml.clone(),
                        source,
                    },
                )?;
            } else {
                return Err(TrackerError::NotFound {
                    kind: "Scope",
                    id: resolved_scope,
                });
            }
        }

        if !phase_manifest
            .scopes
            .manifest
            .iter()
            .any(|s| s.id == resolved_scope)
        {
            phase_manifest.scopes.manifest.push(ScopeRegistryEntry {
                id: resolved_scope.clone(),
                scope_type: infer_default_scope_type(&resolved_scope),
                path: format!("domains/{resolved_scope}"),
            });
            self.write_phase(&resolved_phase, &phase_manifest)?;
        }

        Ok((resolved_phase, resolved_scope))
    }
}

pub(crate) fn validate_identifier(kind: &'static str, id: &str) -> Result<()> {
    let trimmed = id.trim();
    if trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains('\0')
    {
        return Err(TrackerError::ValidationError(format!(
            "Invalid {kind} identifier '{id}'"
        )));
    }
    Ok(())
}

fn resolve_safe_relative_path(base: &Path, rel: &str) -> Result<PathBuf> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        return Err(TrackerError::ValidationError(format!(
            "Relative tracker path '{rel}' must not be absolute"
        )));
    }
    for comp in rel_path.components() {
        match comp {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(TrackerError::ValidationError(format!(
                    "Relative tracker path '{rel}' must not contain parent traversal ('..')"
                )));
            }
        }
    }
    Ok(base.join(rel_path))
}

fn attach_toml_path(err: TrackerError, path: PathBuf) -> TrackerError {
    match err {
        TrackerError::TomlDeserialize { source, .. } => TrackerError::TomlDeserialize {
            path: Some(path),
            source,
        },
        other => other,
    }
}
