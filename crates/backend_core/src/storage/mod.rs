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
    ActiveTimer, ComponentRegistry, GitConfig, ItemDetail, ItemManifest, ItemStub, ItemType,
    MetricsConfig, PhaseManifest, PhaseMetadata, PhaseRegistry, PhaseRegistryEntry, PhaseStatus,
    PhaseSummary, ProjectCounters, ProjectManifest, ProjectMetadata, ScopeItems, ScopeManifest,
    ScopeMetadata, ScopeRegistry, ScopeRegistryEntry,
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
    ///
    /// Skips any `.tracker/` directory located directly inside an unsafe broad root (such as
    /// `/`, `$HOME`, or `/tmp`) so a stray `~/.tracker` never hijacks unrelated workspaces.
    pub fn discover(start_dir: impl AsRef<Path>) -> Result<Self> {
        let start = start_dir.as_ref();
        let abs = if start.is_relative() {
            std::env::current_dir()
                .map(|cwd| cwd.join(start))
                .unwrap_or_else(|_| start.to_path_buf())
        } else {
            start.to_path_buf()
        };
        let resolved_start = if abs.exists() {
            fs::canonicalize(&abs).unwrap_or_else(|_| lexical_normalize_path(&abs))
        } else {
            lexical_normalize_path(&abs)
        };

        let mut current = Some(resolved_start.as_path());
        while let Some(dir) = current {
            if dir.file_name() == Some(OsStr::new(".tracker")) && dir.join("project.toml").is_file()
            {
                let parent = dir.parent().unwrap_or(dir);
                if !is_unsafe_broad_directory(parent) {
                    return Ok(Self {
                        tracker_dir: dir.to_path_buf(),
                    });
                }
            }
            if !is_unsafe_broad_directory(dir) {
                let candidate = dir.join(".tracker");
                if candidate.join("project.toml").is_file() {
                    return Ok(Self {
                        tracker_dir: candidate,
                    });
                }
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
        validate_safe_project_root(&storage.project_root())?;
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
            components: ComponentRegistry::default(),
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
                bound_component: None,
                tracked_files: Vec::new(),
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
        let mut manifest =
            ProjectManifest::from_toml_str(&raw).map_err(|err| attach_toml_path(err, path))?;
        normalize_project_manifest_paths(&mut manifest);
        Ok(manifest)
    }

    /// Atomically writes Tier 1 `.tracker/project.toml`.
    pub fn write_project(&self, manifest: &ProjectManifest) -> Result<()> {
        let mut normalized = manifest.clone();
        normalize_project_manifest_paths(&mut normalized);
        let serialized = normalized.to_toml_string()?;
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
        let mut manifest = PhaseManifest::from_toml_str(&raw)
            .map_err(|err| attach_toml_path(err, path.to_path_buf()))?;
        normalize_phase_manifest_paths(&mut manifest);
        Ok(manifest)
    }

    /// Atomically writes Tier 2 `.tracker/phases/<phase_id>/phase.toml`.
    pub fn write_phase(&self, phase_id: &str, manifest: &PhaseManifest) -> Result<()> {
        let path = self.phase_toml_path(phase_id)?;
        self.write_phase_to_path(&path, manifest)
    }

    pub(crate) fn write_phase_to_path(&self, path: &Path, manifest: &PhaseManifest) -> Result<()> {
        let mut normalized = manifest.clone();
        normalize_phase_manifest_paths(&mut normalized);
        let serialized = normalized.to_toml_string()?;
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
        let mut manifest = ScopeManifest::from_toml_str(&raw)
            .map_err(|err| attach_toml_path(err, path.to_path_buf()))?;
        normalize_scope_manifest_paths(&mut manifest);
        Ok(manifest)
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
        let mut normalized = manifest.clone();
        normalize_scope_manifest_paths(&mut normalized);
        let serialized = normalized.to_toml_string()?;
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
                        bound_component: None,
                        tracked_files: Vec::new(),
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

/// Normalizes any relative path string (e.g. `phases\01_core`, `.\domains\backend_core\`, or
/// component file lists) to canonical forward slashes (`/`) across Linux, macOS, and Windows.
pub fn normalize_relative_path_str(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed == "." {
        return ".".to_string();
    }

    let unified = trimmed.replace('\\', "/");
    let has_dot_segment = unified.split('/').any(|seg| seg == ".");
    let parts: Vec<&str> = unified
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != ".")
        .collect();

    if parts.is_empty() {
        if has_dot_segment {
            ".".to_string()
        } else {
            String::new()
        }
    } else {
        parts.join("/")
    }
}

/// Normalizes a relative [`Path`] to canonical forward slashes (`/`).
pub fn normalize_relative_path(path: impl AsRef<Path>) -> String {
    normalize_relative_path_str(&path.as_ref().to_string_lossy())
}

/// Normalizes a list of relative file or component paths to forward slashes (`/`),
/// omitting empty entries and deduplicating while preserving order.
pub fn normalize_relative_path_list<I, S>(paths: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut out = Vec::new();
    for p in paths {
        let norm = normalize_relative_path_str(p.as_ref());
        if !norm.is_empty() && !out.contains(&norm) {
            out.push(norm);
        }
    }
    out
}

pub(crate) fn normalize_project_manifest_paths(manifest: &mut ProjectManifest) {
    for entry in &mut manifest.phases.registry {
        entry.path = normalize_relative_path_str(&entry.path);
    }
    for comp in &mut manifest.components.registry {
        comp.normalize_paths();
    }
}

pub(crate) fn normalize_phase_manifest_paths(manifest: &mut PhaseManifest) {
    for entry in &mut manifest.scopes.manifest {
        entry.path = normalize_relative_path_str(&entry.path);
    }
}

pub(crate) fn normalize_scope_manifest_paths(manifest: &mut ScopeManifest) {
    manifest.scope.target = normalize_relative_path_str(&manifest.scope.target);
    manifest.scope.tracked_files = normalize_relative_path_list(&manifest.scope.tracked_files);
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

pub(crate) fn resolve_safe_relative_path(base: &Path, rel: &str) -> Result<PathBuf> {
    let trimmed = rel.trim();
    if trimmed.is_empty() {
        return Err(TrackerError::ValidationError(
            "Relative tracker path must not be empty".to_string(),
        ));
    }

    let has_windows_drive_prefix = trimmed.len() >= 2
        && trimmed.as_bytes()[0].is_ascii_alphabetic()
        && trimmed.as_bytes()[1] == b':';

    if trimmed.starts_with('/')
        || trimmed.starts_with('\\')
        || has_windows_drive_prefix
        || Path::new(trimmed).is_absolute()
    {
        return Err(TrackerError::ValidationError(format!(
            "Relative tracker path '{rel}' must not be absolute"
        )));
    }

    let unified = trimmed.replace('\\', "/");
    let mut resolved = base.to_path_buf();

    for seg in unified.split('/') {
        if seg.is_empty() || seg == "." {
            continue;
        }
        if seg == ".." || seg.contains('\0') {
            return Err(TrackerError::ValidationError(format!(
                "Relative tracker path '{rel}' must not contain parent traversal ('..')"
            )));
        }
        for comp in Path::new(seg).components() {
            match comp {
                Component::Normal(part) => resolved.push(part),
                Component::CurDir => {}
                Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                    return Err(TrackerError::ValidationError(format!(
                        "Relative tracker path '{rel}' must not contain parent traversal ('..')"
                    )));
                }
            }
        }
    }

    Ok(resolved)
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

const UNSAFE_SYSTEM_DIRS: &[&str] = &[
    "/bin",
    "/boot",
    "/dev",
    "/etc",
    "/home",
    "/lib",
    "/lib32",
    "/lib64",
    "/media",
    "/mnt",
    "/opt",
    "/proc",
    "/root",
    "/run",
    "/sbin",
    "/srv",
    "/sys",
    "/tmp",
    "/usr",
    "/var",
    "/Users",
    "/System",
    "/Library",
    "/Volumes",
    "/private",
    "/private/tmp",
    "/var/tmp",
];

fn lexical_normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::Prefix(p) => out.push(p.as_os_str()),
            Component::RootDir => out.push(comp.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = out.pop();
            }
            Component::Normal(part) => out.push(part),
        }
    }
    out
}

fn normalize_comparable_path_str(path: &Path) -> String {
    let mut s = path.to_string_lossy().replace('\\', "/");
    if let Some(stripped) = s.strip_prefix("//?/UNC/") {
        s = format!("//{stripped}");
    } else if let Some(stripped) = s.strip_prefix("//?/") {
        s = stripped.to_string();
    }
    while s.len() > 1 && s.ends_with('/') {
        s.pop();
    }
    s
}

fn resolve_project_root_candidate(path: &Path) -> PathBuf {
    if path.as_os_str().is_empty() {
        return PathBuf::new();
    }
    let abs = if path.is_relative() {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    };

    let mut norm = lexical_normalize_path(&abs);
    if norm.file_name() == Some(OsStr::new("project.toml")) {
        if let Some(parent) = norm.parent() {
            norm = parent.to_path_buf();
        }
    }
    if norm.file_name() == Some(OsStr::new(".tracker")) {
        if let Some(parent) = norm.parent() {
            norm = parent.to_path_buf();
        }
    }

    if norm.exists() {
        if let Ok(mut canon) = fs::canonicalize(&norm) {
            if canon.file_name() == Some(OsStr::new(".tracker")) {
                if let Some(parent) = canon.parent() {
                    canon = parent.to_path_buf();
                }
            }
            return canon;
        }
    }
    norm
}

/// Returns `true` if `path` (or its parent when `path` is `.tracker`) resolves to an unsafe broad
/// directory such as a filesystem root (`/`, `C:\`), the user's home directory (`$HOME` /
/// `%USERPROFILE%`), a top-level system tree (`/usr`, `/etc`, `/var`, `/proc`, `/sys`, `/dev`,
/// `/home`), or the top-level temporary directory (`/tmp`) itself.
pub fn is_unsafe_broad_directory(path: &Path) -> bool {
    let resolved = resolve_project_root_candidate(path);
    if resolved.as_os_str().is_empty() || resolved.parent().is_none() {
        return true;
    }

    let s = normalize_comparable_path_str(&resolved);
    if s.is_empty()
        || s == "/"
        || (s.len() == 2 && s.as_bytes()[0].is_ascii_alphabetic() && s.as_bytes()[1] == b':')
    {
        return true;
    }

    if UNSAFE_SYSTEM_DIRS.contains(&s.as_str()) {
        return true;
    }
    if s.starts_with("/proc/") || s.starts_with("/sys/") || s.starts_with("/dev/") {
        return true;
    }

    let temp_dir = std::env::temp_dir();
    let temp_norm_s = normalize_comparable_path_str(&lexical_normalize_path(&temp_dir));
    let temp_canon_s = fs::canonicalize(&temp_dir)
        .map(|p| normalize_comparable_path_str(&p))
        .unwrap_or_else(|_| temp_norm_s.clone());

    if s == temp_norm_s || s == temp_canon_s {
        return true;
    }

    if let Some(home_os) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let home_path = PathBuf::from(home_os);
        if !home_path.as_os_str().is_empty() {
            let home_norm = lexical_normalize_path(&home_path);
            let home_norm_s = normalize_comparable_path_str(&home_norm);
            let home_canon_s = fs::canonicalize(&home_path)
                .map(|p| normalize_comparable_path_str(&p))
                .unwrap_or_else(|_| home_norm_s.clone());

            let temp_norm_prefix = format!("{temp_norm_s}/");
            let temp_canon_prefix = format!("{temp_canon_s}/");
            let home_in_temp = home_norm_s == temp_norm_s
                || home_norm_s.starts_with(&temp_norm_prefix)
                || home_canon_s == temp_canon_s
                || home_canon_s.starts_with(&temp_canon_prefix);

            if !home_in_temp && (s == home_norm_s || s == home_canon_s) {
                return true;
            }
        }
    }

    false
}

/// Validates that `path` is a safe, specific project root directory (rejecting `/`, `$HOME`,
/// top-level OS directories, and `/tmp` itself), returning the normalized/canonicalized path.
pub fn validate_safe_project_root(path: &Path) -> Result<PathBuf> {
    let resolved = resolve_project_root_candidate(path);
    if is_unsafe_broad_directory(path) {
        return Err(TrackerError::ValidationError(format!(
            "Refusing to use broad system or home directory '{}' as a Nebulous project root. Please select a specific project directory.",
            resolved.display()
        )));
    }
    Ok(resolved)
}
