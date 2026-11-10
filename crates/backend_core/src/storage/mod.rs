pub mod atomic;
pub mod compat;
pub mod id;
pub mod reconcile;

use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use chrono::Utc;

pub use atomic::{atomic_write_bytes, atomic_write_str};
pub use id::parse_item_id_number;
pub use reconcile::ReconcileReport;

/// Advisory lock file name inside `.tracker/`.
pub const LOCK_FILE_NAME: &str = ".lock";

/// Default timeout when waiting to acquire the exclusive `.tracker/.lock` file lock.
pub const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(30);

thread_local! {
    static HELD_WRITE_LOCKS: RefCell<HashSet<PathBuf>> = RefCell::new(HashSet::new());
}

struct ReentrantLockGuard {
    path: PathBuf,
}

impl ReentrantLockGuard {
    fn acquire(path: PathBuf) -> Self {
        HELD_WRITE_LOCKS.with(|held| {
            held.borrow_mut().insert(path.clone());
        });
        Self { path }
    }
}

impl Drop for ReentrantLockGuard {
    fn drop(&mut self) {
        HELD_WRITE_LOCKS.with(|held| {
            held.borrow_mut().remove(&self.path);
        });
    }
}

use crate::error::{Result, TrackerError};
use crate::models::{
    canonical_phase_title, resolve_phase_alias, ActiveTimer, ComponentRegistry, GitConfig,
    ItemDetail, ItemManifest, ItemStub, ItemType, MetricsConfig, PhaseManifest, PhaseMetadata,
    PhaseRegistry, PhaseRegistryEntry, PhaseStatus, PhaseSummary, ProjectCounters, ProjectManifest,
    ProjectMetadata, ScopeItems, ScopeManifest, ScopeMetadata, ScopeRegistry, ScopeRegistryEntry,
    CANONICAL_PHASES,
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

    /// Looks up a loaded [`PhaseIndex`] by phase ID (or canonical phase alias).
    pub fn phase(&self, phase_id: &str) -> Option<&PhaseIndex> {
        let resolved = resolve_phase_alias(phase_id);
        self.phases
            .iter()
            .find(|p| p.phase_id == resolved || p.phase_id == phase_id)
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

    /// Initializes a new `.tracker/` directory hierarchy at `root_or_tracker_dir` with all 3
    /// canonical stages (`pre_production`, `production`, `post_production`), setting
    /// `active_phase = "pre_production"` (`InProgress`, others `Planned`) and creating the
    /// default `inbox` scope in all 3 stages.
    pub fn init_standard(
        root_or_tracker_dir: impl AsRef<Path>,
        project_name: impl Into<String>,
    ) -> Result<Self> {
        Self::init(
            root_or_tracker_dir,
            project_name,
            "pre_production",
            "Pre-Production",
        )
    }

    /// Initializes a new `.tracker/` directory hierarchy at `root_or_tracker_dir` with a default
    /// phase and `inbox` scope.
    ///
    /// Resolves `initial_phase_id` via [`resolve_phase_alias`]. When `initial_phase_id` resolves
    /// to one of the 3 canonical stages (`pre_production`, `production`, `post_production`), all
    /// 3 canonical stages are scaffolded with their `inbox` scopes out of the box and
    /// `active_phase` is set to the resolved stage.
    pub fn init(
        root_or_tracker_dir: impl AsRef<Path>,
        project_name: impl Into<String>,
        initial_phase_id: impl Into<String>,
        initial_phase_title: impl Into<String>,
    ) -> Result<Self> {
        let storage = Self::new(root_or_tracker_dir);
        validate_safe_project_root(&storage.project_root())?;
        let raw_phase_id = initial_phase_id.into();
        let phase_id = resolve_phase_alias(&raw_phase_id).to_string();
        let raw_phase_title = initial_phase_title.into();
        validate_identifier("Phase", &phase_id)?;

        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let today = Utc::now().format("%Y-%m-%d").to_string();

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

        let is_canonical = canonical_phase_title(&phase_id).is_some();
        let stages_to_scaffold: Vec<(String, String)> = if is_canonical {
            CANONICAL_PHASES
                .iter()
                .map(|(cid, ctitle)| {
                    let title = if *cid == phase_id
                        && !raw_phase_title.trim().is_empty()
                        && raw_phase_title.trim() != "Core Engine & Initial Scope"
                    {
                        raw_phase_title.trim().to_string()
                    } else {
                        (*ctitle).to_string()
                    };
                    ((*cid).to_string(), title)
                })
                .collect()
        } else {
            let title = if raw_phase_title.trim().is_empty() {
                phase_id.clone()
            } else {
                raw_phase_title
            };
            vec![(phase_id.clone(), title)]
        };

        let registry_entries: Vec<PhaseRegistryEntry> = stages_to_scaffold
            .iter()
            .map(|(sid, stitle)| PhaseRegistryEntry {
                id: sid.clone(),
                title: stitle.clone(),
                path: format!("phases/{sid}"),
            })
            .collect();

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
                registry: registry_entries,
            },
            metrics: MetricsConfig::default(),
            git: GitConfig::default(),
            components: ComponentRegistry::default(),
        };

        storage.write_project(&project)?;
        let gitignore_path = storage.tracker_dir.join(".gitignore");
        if !gitignore_path.exists() {
            storage.write_tracker_file(&gitignore_path, &format!("{LOCK_FILE_NAME}\n"))?;
        }

        for (sid, stitle) in &stages_to_scaffold {
            let phase = PhaseManifest {
                phase: PhaseMetadata {
                    id: sid.clone(),
                    title: stitle.clone(),
                    status: if *sid == phase_id {
                        PhaseStatus::InProgress
                    } else {
                        PhaseStatus::Planned
                    },
                    start_date: Some(today.clone()),
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

            storage.write_phase(sid, &phase)?;
            storage.write_scope(sid, "inbox", &inbox_scope)?;
            storage.ensure_items_dir(sid, "inbox")?;
        }

        Ok(storage)
    }

    /// Ensures that if `phase_id` (after alias resolution) is one of [`CANONICAL_PHASES`] and its
    /// `phase.toml` does not yet exist on disk, it is automatically scaffolded with `inbox`.
    pub(crate) fn ensure_canonical_phase_if_missing(&self, phase_id: &str) -> Result<String> {
        let resolved = resolve_phase_alias(phase_id).to_string();
        validate_identifier("Phase", &resolved)?;
        if let Some(canonical_title) = canonical_phase_title(&resolved) {
            if !self.phase_toml_path(&resolved)?.is_file() {
                let _ = self.create_phase(&resolved, canonical_title, false)?;
            }
        }
        Ok(resolved)
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

    /// Proves `path` (which must be lexically beneath `tracker_dir`) stays inside `.tracker/` on
    /// disk and is not a symlink, via [`contained_path`].
    pub(crate) fn guard_tracker_path(&self, path: &Path) -> Result<PathBuf> {
        let rel = path.strip_prefix(&self.tracker_dir).map_err(|_| {
            TrackerError::ValidationError(format!(
                "Path '{}' is not inside tracker directory '{}'",
                path.display(),
                self.tracker_dir.display()
            ))
        })?;
        contained_path(&self.tracker_dir, rel)
    }

    /// Reads a UTF-8 file under `.tracker/` after containment checks.
    pub(crate) fn read_tracker_file(&self, path: &Path) -> Result<String> {
        let path = self.guard_tracker_path(path)?;
        fs::read_to_string(&path).map_err(|source| TrackerError::Io { path, source })
    }

    /// Atomically writes a UTF-8 file under `.tracker/` after containment checks.
    pub(crate) fn write_tracker_file(&self, path: &Path, contents: &str) -> Result<()> {
        let path = self.guard_tracker_path(path)?;
        atomic_write_str(&path, contents)
    }

    /// Creates a directory (and parents) under `.tracker/` after containment checks.
    pub(crate) fn create_tracker_dir_all(&self, path: &Path) -> Result<()> {
        let path = self.guard_tracker_path(path)?;
        fs::create_dir_all(&path).map_err(|source| TrackerError::Io { path, source })
    }

    /// Creates a new directory under `.tracker/` after containment checks, failing with
    /// [`TrackerError::AlreadyExists`] if `path` already exists.
    pub(crate) fn create_tracker_dir_new(&self, path: &Path) -> Result<()> {
        let path = self.guard_tracker_path(path)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| TrackerError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
        fs::create_dir(&path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::AlreadyExists {
                let id = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_string();
                TrackerError::AlreadyExists { kind: "Item", id }
            } else {
                TrackerError::Io { path, source }
            }
        })
    }

    /// Creates `items/` for `(phase_id, scope_id)` under `.tracker/` after containment checks.
    pub fn ensure_items_dir(&self, phase_id: &str, scope_id: &str) -> Result<PathBuf> {
        let dir = self.items_dir(phase_id, scope_id)?;
        self.create_tracker_dir_all(&dir)?;
        Ok(dir)
    }

    /// Returns `.tracker/project.toml`.
    pub fn project_toml_path(&self) -> PathBuf {
        self.tracker_dir.join("project.toml")
    }

    /// Returns `.tracker/.timer.toml`.
    pub fn timer_toml_path(&self) -> PathBuf {
        self.tracker_dir.join(".timer.toml")
    }

    /// Returns `.tracker/.lock`.
    pub fn lock_path(&self) -> PathBuf {
        self.tracker_dir.join(LOCK_FILE_NAME)
    }

    /// Acquires an exclusive, thread-reentrant advisory lock on `.tracker/.lock` with
    /// [`DEFAULT_LOCK_TIMEOUT`] and executes `f`.
    pub fn with_write_lock<T>(&self, f: impl FnOnce(&Self) -> Result<T>) -> Result<T> {
        self.with_write_lock_timeout(DEFAULT_LOCK_TIMEOUT, f)
    }

    /// Acquires an exclusive, thread-reentrant advisory lock on `.tracker/.lock`, waiting up to
    /// `timeout` before returning [`TrackerError::LockTimeout`], and executes `f`.
    pub fn with_write_lock_timeout<T>(
        &self,
        timeout: Duration,
        f: impl FnOnce(&Self) -> Result<T>,
    ) -> Result<T> {
        let lock_path = self.lock_path();
        if HELD_WRITE_LOCKS.with(|held| held.borrow().contains(&lock_path)) {
            return f(self);
        }

        self.create_tracker_dir_all(&self.tracker_dir)?;
        let guarded_lock_path = self.guard_tracker_path(&lock_path)?;
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&guarded_lock_path)
            .map_err(|source| TrackerError::Io {
                path: guarded_lock_path.clone(),
                source,
            })?;

        let mut rw_lock = fd_lock::RwLock::new(file);
        let start = Instant::now();
        let _fd_guard = loop {
            match rw_lock.try_write() {
                Ok(guard) => break guard,
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    let elapsed = start.elapsed();
                    if elapsed >= timeout {
                        return Err(TrackerError::LockTimeout {
                            path: guarded_lock_path,
                            waited_ms: elapsed.as_millis() as u64,
                        });
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(source) => {
                    return Err(TrackerError::Io {
                        path: guarded_lock_path,
                        source,
                    });
                }
            }
        };

        let _reentrancy_guard = ReentrantLockGuard::acquire(lock_path);
        self.ensure_lock_gitignore()?;
        f(self)
    }

    fn ensure_lock_gitignore(&self) -> Result<()> {
        let gitignore_path = self.tracker_dir.join(".gitignore");
        if gitignore_path.is_file() {
            let existing = self.read_tracker_file(&gitignore_path)?;
            if existing.lines().any(|line| line == LOCK_FILE_NAME) {
                return Ok(());
            }
            let mut updated = existing;
            if !updated.is_empty() && !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push_str(LOCK_FILE_NAME);
            updated.push('\n');
            self.write_tracker_file(&gitignore_path, &updated)
        } else {
            self.write_tracker_file(&gitignore_path, &format!("{LOCK_FILE_NAME}\n"))
        }
    }

    /// Returns `.tracker/phases`.
    pub fn phases_dir(&self) -> PathBuf {
        self.tracker_dir.join("phases")
    }

    /// Returns `.tracker/phases/<phase_id>` (or the custom relative path configured in `project.toml`).
    pub fn phase_dir(&self, phase_id: &str) -> Result<PathBuf> {
        let resolved = resolve_phase_alias(phase_id);
        validate_identifier("Phase", resolved)?;
        if let Ok(project) = self.read_project() {
            if let Some(entry) = project
                .phases
                .registry
                .iter()
                .find(|p| p.id == resolved || p.id == phase_id)
            {
                return self.resolve_phase_path(&entry.id, Some(&entry.path));
            }
        }
        Ok(self.phases_dir().join(resolved))
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

    /// Reads and parses a versioned TOML manifest under `.tracker/`.
    fn read_versioned<T: serde::de::DeserializeOwned>(&self, path: &Path) -> Result<T> {
        let raw = self.read_tracker_file(path)?;
        let mut table: toml::Table =
            toml::from_str(&raw).map_err(|err| attach_toml_path(err.into(), path.to_path_buf()))?;
        compat::check_readable(&table, path, crate::models::item::MAX_ITEM_SCHEMA_VERSION)?;
        table.remove(compat::SCHEMA_KEY);
        T::deserialize(toml::Value::Table(table))
            .map_err(|err| attach_toml_path(err.into(), path.to_path_buf()))
    }

    /// Writes a versioned TOML manifest under `.tracker/`, preserving unknown keys from any existing file.
    fn write_versioned<T: serde::Serialize>(
        &self,
        path: &Path,
        value: &T,
        version: u32,
    ) -> Result<()> {
        let mut new = toml::Table::try_from(value)?;
        let mut old_had_key = false;
        if path.is_file() {
            let raw = self.read_tracker_file(path)?;
            let old: toml::Table = toml::from_str(&raw)
                .map_err(|err| attach_toml_path(err.into(), path.to_path_buf()))?;
            compat::check_readable(&old, path, crate::models::item::MAX_ITEM_SCHEMA_VERSION)?;
            old_had_key = old.contains_key(compat::SCHEMA_KEY);
            compat::merge_unknown(&mut new, &old);
        }
        compat::stamp(&mut new, version, old_had_key);
        let serialized = toml::to_string_pretty(&new)?;
        self.write_tracker_file(path, &serialized)
    }

    /// Reads and parses Tier 1 `.tracker/project.toml`.
    pub fn read_project(&self) -> Result<ProjectManifest> {
        let path = self.project_toml_path();
        if !path.is_file() {
            return Err(TrackerError::TrackerRootNotFound(self.tracker_dir.clone()));
        }
        let mut manifest: ProjectManifest = self.read_versioned(&path)?;
        normalize_project_manifest_paths(&mut manifest);
        Ok(manifest)
    }

    /// Atomically writes Tier 1 `.tracker/project.toml`.
    pub fn write_project(&self, manifest: &ProjectManifest) -> Result<()> {
        let mut normalized = manifest.clone();
        normalize_project_manifest_paths(&mut normalized);
        self.write_versioned(&self.project_toml_path(), &normalized, 1)
    }

    /// Reads and parses Tier 2 `.tracker/phases/<phase_id>/phase.toml`.
    pub fn read_phase(&self, phase_id: &str) -> Result<PhaseManifest> {
        let resolved = resolve_phase_alias(phase_id);
        let path = self.phase_toml_path(resolved)?;
        if !path.is_file() {
            return Err(TrackerError::NotFound {
                kind: "Phase",
                id: resolved.to_string(),
            });
        }
        self.read_phase_from_path(&path)
    }

    pub(crate) fn read_phase_from_path(&self, path: &Path) -> Result<PhaseManifest> {
        let mut manifest: PhaseManifest = self.read_versioned(path)?;
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
        self.write_versioned(path, &normalized, 1)
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
        let mut manifest: ScopeManifest = self.read_versioned(path)?;
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
        self.write_versioned(path, &normalized, 1)
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
        self.read_versioned(&path)
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
        let version = manifest.item.item_type.min_schema_version();
        self.write_versioned(&path, manifest, version)
    }

    /// Reads Tier 4 `items/<item_id>/desc.md`.
    pub fn read_item_desc(&self, phase_id: &str, scope_id: &str, item_id: &str) -> Result<String> {
        let path = self.item_desc_path(phase_id, scope_id, item_id)?;
        self.read_tracker_file(&path)
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
        self.write_tracker_file(&path, description_markdown)
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
        let raw = self.read_tracker_file(&path)?;
        let timer = ActiveTimer::from_toml_str(&raw).map_err(|err| attach_toml_path(err, path))?;
        Ok(Some(timer))
    }

    /// Atomically writes `.tracker/.timer.toml`.
    pub fn write_active_timer(&self, timer: &ActiveTimer) -> Result<()> {
        let serialized = timer.to_toml_string()?;
        self.write_tracker_file(&self.timer_toml_path(), &serialized)
    }

    /// Removes `.tracker/.timer.toml` if it exists.
    pub fn clear_active_timer(&self) -> Result<()> {
        let path = self.guard_tracker_path(&self.timer_toml_path())?;
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
            Some(p) => self.ensure_canonical_phase_if_missing(p)?,
            None => {
                let active = resolve_phase_alias(project.project.active_phase.trim());
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
                self.ensure_items_dir(&resolved_phase, &resolved_scope)?;
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

/// Maximum length of a phase / scope / item / component identifier.
pub const MAX_IDENTIFIER_LEN: usize = 64;

/// Windows device names that cannot be used as file or directory names on any Windows volume.
const WINDOWS_RESERVED_NAMES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "COM0", "COM1", "COM2", "COM3", "COM4",
    "COM5", "COM6", "COM7", "COM8", "COM9", "LPT0", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6",
    "LPT7", "LPT8", "LPT9",
];

/// Returns `true` if `id` matches `^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$` and is not a Windows
/// reserved device name (case-insensitive).
///
/// The allow-list guarantees an identifier is always exactly one plain path component on every
/// platform: no separators, no `.`/`..`, no drive prefixes (`C:x`), no trailing dots or spaces,
/// and no NUL.
pub fn is_valid_identifier(id: &str) -> bool {
    let bytes = id.as_bytes();
    if bytes.is_empty() || bytes.len() > MAX_IDENTIFIER_LEN || !bytes[0].is_ascii_alphanumeric() {
        return false;
    }
    if !bytes[1..]
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
    {
        return false;
    }
    !WINDOWS_RESERVED_NAMES
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(id))
}

pub(crate) fn validate_identifier(kind: &'static str, id: &str) -> Result<()> {
    if !is_valid_identifier(id) {
        return Err(TrackerError::ValidationError(format!(
            "Invalid {kind} identifier '{id}': must match ^[A-Za-z0-9][A-Za-z0-9_-]{{0,63}}$ \
             and must not be a reserved Windows device name"
        )));
    }
    Ok(())
}

/// Normalizes an arbitrary label (e.g. an on-disk component id such as `my.crate`) into a valid
/// identifier: invalid input is slugified (lowercase ASCII alphanumerics joined by `_`),
/// truncated to `max_len` (capped at [`MAX_IDENTIFIER_LEN`]), and suffixed with `_` if it would
/// collide with a Windows reserved name. Already-valid identifiers that fit are returned as-is.
pub(crate) fn normalize_identifier(raw: &str, max_len: usize) -> String {
    let max_len = max_len.clamp(1, MAX_IDENTIFIER_LEN);
    if raw.len() <= max_len && is_valid_identifier(raw) {
        return raw.to_string();
    }
    let mut slug = crate::models::registry::slugify_project_id(raw);
    slug.truncate(max_len);
    while slug.ends_with('_') && slug.len() > 1 {
        slug.pop();
    }
    if !is_valid_identifier(&slug) {
        // Only reachable for reserved names (slugify output is otherwise always valid).
        if slug.len() >= max_len {
            slug.pop();
        }
        slug.push('_');
    }
    slug
}

/// Resolves `rel` beneath `base` and proves the result stays inside `base` on disk.
///
/// - `rel` must consist only of normal components (no `..`, root, or drive/UNC prefix).
/// - The deepest existing ancestor of `base.join(rel)` is canonicalized (following any
///   symlinked directories) and must start with the canonicalized `base`.
/// - The final component must not itself be a symlink (checked with `symlink_metadata`), so a
///   planted `desc.md -> ~/.ssh/id_ed25519` is rejected for both reads and writes.
///
/// Returns the lexical path `base.join(rel)` (not the canonical form) so callers keep the
/// tracker-relative paths they display and compare.
pub fn contained_path(base: &Path, rel: impl AsRef<Path>) -> Result<PathBuf> {
    resolve_contained(base, rel.as_ref(), true)
}

/// Like [`contained_path`] but permits the final component to be a symlink as long as it
/// resolves inside `base`. Only for validating references (e.g. tracked file lists or an
/// ingestion scan root) that are never opened for reading or writing tracker content.
pub(crate) fn contained_reference_path(base: &Path, rel: impl AsRef<Path>) -> Result<PathBuf> {
    resolve_contained(base, rel.as_ref(), false)
}

fn resolve_contained(base: &Path, rel: &Path, reject_final_symlink: bool) -> Result<PathBuf> {
    let escape = |why: &str| {
        TrackerError::ValidationError(format!(
            "Path '{}' escapes '{}': {why}",
            rel.display(),
            base.display()
        ))
    };

    let mut joined = base.to_path_buf();
    let mut has_normal = false;
    for comp in rel.components() {
        match comp {
            Component::Normal(part) => {
                joined.push(part);
                has_normal = true;
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(escape(
                    "must be relative without '..' or a root/drive prefix",
                ));
            }
        }
    }

    let canon_base = canonicalize_existing_prefix(base)?;
    let canon_target = canonicalize_existing_prefix(&joined)?;
    if !canon_target.starts_with(&canon_base) {
        return Err(escape(&format!(
            "resolves to '{}' outside '{}'",
            canon_target.display(),
            canon_base.display()
        )));
    }

    if reject_final_symlink && has_normal {
        if let Ok(meta) = fs::symlink_metadata(&joined) {
            if meta.file_type().is_symlink() {
                return Err(escape("final path component is a symlink"));
            }
        }
    }

    Ok(joined)
}

/// Canonicalizes the deepest existing ancestor of `path` and re-appends the non-existent tail.
fn canonicalize_existing_prefix(path: &Path) -> Result<PathBuf> {
    let io_err = |source: std::io::Error| TrackerError::Io {
        path: path.to_path_buf(),
        source,
    };

    let mut existing = path;
    let mut tail: Vec<&OsStr> = Vec::new();
    loop {
        let probe = if existing.as_os_str().is_empty() {
            Path::new(".")
        } else {
            existing
        };
        match fs::symlink_metadata(probe) {
            Ok(meta) => {
                let mut canon = fs::canonicalize(probe).map_err(|source| {
                    if meta.file_type().is_symlink() {
                        TrackerError::ValidationError(format!(
                            "Dangling or unresolvable symlink '{}'",
                            probe.display()
                        ))
                    } else {
                        io_err(source)
                    }
                })?;
                for part in tail.iter().rev() {
                    canon.push(part);
                }
                return Ok(canon);
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                let (Some(name), Some(parent)) = (existing.file_name(), existing.parent()) else {
                    return Err(TrackerError::ValidationError(format!(
                        "Cannot resolve path '{}' (no existing ancestor)",
                        path.display()
                    )));
                };
                tail.push(name);
                existing = parent;
            }
            Err(err) => return Err(io_err(err)),
        }
    }
}

/// Lexically validates a slash- or backslash-separated tracker-relative path and resolves it
/// beneath `base` via [`contained_path`] (symlink-aware containment, final symlink rejected).
pub(crate) fn resolve_safe_relative_path(base: &Path, rel: &str) -> Result<PathBuf> {
    contained_path(base, lexical_relative_path(rel)?)
}

/// Like [`resolve_safe_relative_path`] but delegates to [`contained_reference_path`].
pub(crate) fn resolve_contained_reference(base: &Path, rel: &str) -> Result<PathBuf> {
    contained_reference_path(base, lexical_relative_path(rel)?)
}

fn lexical_relative_path(rel: &str) -> Result<PathBuf> {
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
    let mut resolved = PathBuf::new();

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

#[cfg(test)]
mod identifier_tests {
    use super::*;

    #[test]
    fn normalize_identifier_maps_invalid_labels_to_valid_ids() {
        assert_eq!(normalize_identifier("backend_core", 64), "backend_core");
        assert_eq!(normalize_identifier("my.crate", 64), "my_crate");
        assert_eq!(normalize_identifier("serde.json-2", 64), "serde_json_2");
        assert_eq!(normalize_identifier("CON", 64), "con_");
        assert_eq!(normalize_identifier("..", 64), "project");
        let long = normalize_identifier(&"x".repeat(100), 53);
        assert_eq!(long.len(), 53);
        for raw in ["my.crate", "CON", "..", "C:x", "a.", &"y".repeat(80)] {
            assert!(is_valid_identifier(&normalize_identifier(raw, 64)), "{raw}");
        }
    }
}

#[cfg(test)]
mod lock_tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;

    #[test]
    fn write_lock_creates_lock_file_and_gitignore_entry() {
        let dir = tempfile::tempdir().expect("tempdir");
        let storage = TrackerStorage::init_standard(dir.path(), "t").expect("init_standard");
        let gitignore = storage.tracker_dir().join(".gitignore");
        assert!(gitignore.is_file());

        // Also verify with_write_lock restores or appends `.lock` when .gitignore is modified.
        fs::write(&gitignore, "# tracker ignore\n").expect("write custom gitignore");
        let val = storage
            .with_write_lock(|_| Ok(42))
            .expect("with_write_lock");
        assert_eq!(val, 42);
        assert!(storage.lock_path().is_file());
        let contents = fs::read_to_string(&gitignore).expect("read gitignore");
        assert!(contents.lines().any(|line| line == LOCK_FILE_NAME));
    }

    #[test]
    fn write_lock_is_reentrant_on_same_thread() {
        let dir = tempfile::tempdir().expect("tempdir");
        let storage = TrackerStorage::init_standard(dir.path(), "t").expect("init_standard");
        let res = storage
            .with_write_lock(|outer| outer.with_write_lock(|_| Ok("nested_ok")))
            .expect("nested with_write_lock");
        assert_eq!(res, "nested_ok");
    }

    #[test]
    fn write_lock_times_out_across_contending_threads() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        let storage_a = TrackerStorage::init_standard(&root, "t").expect("init_standard");

        let (locked_tx, locked_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();

        let handle_a = thread::spawn(move || {
            storage_a
                .with_write_lock(|_| {
                    locked_tx.send(()).expect("notify locked");
                    release_rx.recv().expect("wait release");
                    Ok(())
                })
                .expect("thread A lock");
        });

        locked_rx.recv().expect("wait for thread A to hold lock");

        let root_b = root.clone();
        let handle_b = thread::spawn(move || {
            let storage_b = TrackerStorage::new(&root_b);
            storage_b.with_write_lock_timeout(Duration::from_millis(200), |_| Ok(()))
        });

        let res_b = handle_b.join().expect("join B");
        match res_b {
            Err(TrackerError::LockTimeout { path, waited_ms }) => {
                assert_eq!(path, TrackerStorage::new(&root).lock_path());
                assert!(waited_ms >= 200, "waited_ms={waited_ms}");
            }
            other => panic!("expected LockTimeout, got {other:?}"),
        }

        release_tx.send(()).expect("release thread A");
        handle_a.join().expect("join A");
    }

    #[test]
    fn write_lock_propagates_error_and_releases_lock() {
        let dir = tempfile::tempdir().expect("tempdir");
        let storage = TrackerStorage::init_standard(dir.path(), "t").expect("init_standard");

        let err = storage
            .with_write_lock::<()>(|_| Err(TrackerError::ValidationError("boom".to_string())))
            .expect_err("expected error from closure");
        match err {
            TrackerError::ValidationError(msg) => assert_eq!(msg, "boom"),
            other => panic!("unexpected error: {other:?}"),
        }

        let after = storage
            .with_write_lock(|_| Ok("recovered"))
            .expect("subsequent lock after error");
        assert_eq!(after, "recovered");
    }
}
