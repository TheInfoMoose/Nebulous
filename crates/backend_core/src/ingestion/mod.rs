use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use ignore::{DirEntry, WalkBuilder};
use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};
use crate::models::registry::slugify_project_id;
use crate::models::{
    resolve_phase_alias, ComponentManifest, ComponentType, RollingUpdateEntry, ScopeItems,
    ScopeManifest, ScopeMetadata, ScopeRegistryEntry, VerificationRecord,
    COMPONENT_MANIFEST_FILE_NAME,
};
use crate::storage::{
    atomic_write_str, is_valid_identifier, normalize_identifier, normalize_relative_path,
    normalize_relative_path_list, normalize_relative_path_str, resolve_contained_reference,
    resolve_safe_relative_path, validate_identifier, validate_safe_project_root, TrackerStorage,
    MAX_IDENTIFIER_LEN,
};

/// Directories unconditionally skipped during project directory ingestion.
pub const IGNORED_INGESTION_DIRS: &[&str] = &[
    ".git",
    ".tracker",
    "target",
    "node_modules",
    "dist",
    "build",
    "__pycache__",
    ".venv",
];

/// Ecosystem manifest filenames recognized as component boundaries during Stage 1 scanning.
pub const ECOSYSTEM_MANIFEST_FILES: &[&str] = &[
    COMPONENT_MANIFEST_FILE_NAME,
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "CMakeLists.txt",
    "Makefile",
    "Dockerfile",
];

/// Standard source directories that belong to a root-level package manifest (if the project root
/// itself contains a manifest) rather than being treated as separate functional components.
const ROOT_SOURCE_SUBDIRS: &[&str] = &["src", "include", "lib", "tests", "benches", "examples"];

/// Options controlling [`TrackerStorage::ingest_project_directory`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestionOptions {
    /// Optional subdirectory relative to `project_root` to restrict scanning.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subpath: Option<String>,
    /// When `true`, writes or updates `nebulous.component.toml` in each discovered component directory.
    #[serde(default)]
    pub write_manifests: bool,
    /// When `true`, promotes all discovered components into domain scopes in the target/active phase.
    #[serde(default)]
    pub promote_all: bool,
    /// Optional target phase ID when `promote_all` is `true` (defaults to `project.active_phase`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub promote_phase_id: Option<String>,
}

impl IngestionOptions {
    /// Creates default [`IngestionOptions`].
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_subpath(mut self, subpath: impl Into<String>) -> Self {
        let norm = normalize_relative_path_str(&subpath.into());
        self.subpath = if norm.is_empty() || norm == "." {
            None
        } else {
            Some(norm)
        };
        self
    }

    pub fn with_write_manifests(mut self, write_manifests: bool) -> Self {
        self.write_manifests = write_manifests;
        self
    }

    pub fn with_promote_all(mut self, promote_all: bool) -> Self {
        self.promote_all = promote_all;
        self
    }

    pub fn with_promote_phase(mut self, phase_id: impl Into<String>) -> Self {
        self.promote_phase_id = Some(phase_id.into());
        self
    }
}

/// Summary report returned by [`TrackerStorage::ingest_project_directory`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IngestionReport {
    pub components: Vec<ComponentManifest>,
    pub total_files_indexed: usize,
    pub manifests_written: Vec<PathBuf>,
    pub promoted_scopes: Vec<String>,
}

impl TrackerStorage {
    /// Stage 1: Walks `project_root` (or `options.subpath`) using [`ignore::WalkBuilder`],
    /// respecting `.gitignore` and skipping `.git/`, `.tracker/`, `target/`, `node_modules/`,
    /// `dist/`, `build/`, `__pycache__/`, `.venv/`, and any hidden dot-directories.
    ///
    /// Detects ecosystem manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, `CMakeLists.txt`,
    /// `Makefile`, `Dockerfile`, `nebulous.component.toml`) and top-level un-manifested functional
    /// directories (e.g. `scripts/`, `iso/`, `config/`), recording relative file paths (`files = [...]`)
    /// into `.tracker/project.toml` `[components]` without reading or parsing source file ASTs.
    pub fn ingest_project_directory(&self, options: IngestionOptions) -> Result<IngestionReport> {
        let project_root = validate_safe_project_root(&self.project_root())?;
        if !project_root.is_dir() {
            return Err(TrackerError::NotFound {
                kind: "ProjectRoot",
                id: project_root.display().to_string(),
            });
        }

        let mut norm_subpath = options
            .subpath
            .as_deref()
            .map(normalize_relative_path_str)
            .filter(|s| !s.is_empty() && s != ".");

        let scan_root = match norm_subpath.as_deref() {
            Some(rel) => {
                let target = resolve_contained_reference(&project_root, rel)?;
                if !target.is_dir() {
                    return Err(TrackerError::NotFound {
                        kind: "Directory",
                        id: rel.to_string(),
                    });
                }
                let target_canon =
                    fs::canonicalize(&target).map_err(|source| TrackerError::Io {
                        path: target.clone(),
                        source,
                    })?;
                if !target_canon.starts_with(&project_root) {
                    return Err(TrackerError::ValidationError(format!(
                        "Relative tracker path '{rel}' resolves outside the active project root '{}'",
                        project_root.display()
                    )));
                }
                validate_safe_project_root(&target_canon)?;
                if let Ok(canon_rel) = target_canon.strip_prefix(&project_root) {
                    let canon_rel_str = normalize_relative_path(canon_rel);
                    norm_subpath = if canon_rel_str.is_empty() || canon_rel_str == "." {
                        None
                    } else {
                        Some(canon_rel_str)
                    };
                }
                target_canon
            }
            None => project_root.clone(),
        };

        let mut walk_builder = WalkBuilder::new(&scan_root);
        walk_builder
            .require_git(false)
            .git_ignore(true)
            .git_global(false)
            .git_exclude(true)
            .same_file_system(true)
            .follow_links(false)
            .hidden(false);

        if scan_root != project_root {
            let root_gitignore = project_root.join(".gitignore");
            if root_gitignore.is_file() {
                let _ = walk_builder.add_ignore(root_gitignore);
            }
        }

        let scan_root_clone = scan_root.clone();
        walk_builder.filter_entry(move |entry| !should_skip_entry(entry, &scan_root_clone));

        let mut all_rel_files = Vec::new();
        let mut manifest_dirs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for entry_res in walk_builder.build() {
            let entry = match entry_res {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let file_name = entry.file_name().to_string_lossy();
            if file_name == ".gitignore" || file_name == ".ignore" {
                continue;
            }

            let rel_path = match path.strip_prefix(&project_root) {
                Ok(rel) => normalize_relative_path(rel),
                Err(_) => continue,
            };
            if rel_path.is_empty() {
                continue;
            }

            if ECOSYSTEM_MANIFEST_FILES.contains(&file_name.as_ref()) {
                let parent_rel = path
                    .parent()
                    .and_then(|p| p.strip_prefix(&project_root).ok())
                    .map(normalize_relative_path)
                    .unwrap_or_else(|| ".".to_string());
                let dir_key = if parent_rel.is_empty() {
                    ".".to_string()
                } else {
                    parent_rel
                };
                manifest_dirs
                    .entry(dir_key)
                    .or_default()
                    .insert(file_name.into_owned());
            }

            all_rel_files.push(rel_path);
        }

        all_rel_files.sort();
        all_rel_files.dedup();

        // Group files into components:
        // 1. Any file inside a non-root manifested directory belongs to the deepest manifested ancestor.
        // 2. Remaining files in standard root source subdirs (`src/`, `tests/`, etc.) belong to `.`
        //    when `.` is an actual root package.
        // 3. Remaining files in un-manifested directories belong to the shallowest directory prefix
        //    (at or below `options.subpath`) that is not a container of nested manifested packages.
        // 4. Remaining root-level files belong to `.` when `.` is an actual root package.
        let has_root_manifest = manifest_dirs.contains_key(".");
        let has_nested_manifests = manifest_dirs.keys().any(|k| k != ".");
        let root_has_nebulous_manifest = manifest_dirs
            .get(".")
            .map(|m| m.contains(COMPONENT_MANIFEST_FILE_NAME))
            .unwrap_or(false);
        let root_has_primary_sources = all_rel_files.iter().any(|f| {
            find_deepest_non_root_manifest_dir(f, &manifest_dirs).is_none()
                && (f.starts_with("src/") || f.starts_with("lib/") || f.starts_with("include/"))
        });
        let has_only_root_files =
            !all_rel_files.is_empty() && all_rel_files.iter().all(|f| !f.contains('/'));
        let root_is_package = (has_root_manifest
            && (!has_nested_manifests || root_has_nebulous_manifest || root_has_primary_sources))
            || root_has_nebulous_manifest
            || has_only_root_files;

        let mut component_files: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for dir_key in manifest_dirs.keys() {
            if dir_key != "." || root_is_package {
                component_files.entry(dir_key.clone()).or_default();
            }
        }

        for rel_file in &all_rel_files {
            if let Some(deepest_manifest_dir) =
                find_deepest_non_root_manifest_dir(rel_file, &manifest_dirs)
            {
                component_files
                    .entry(deepest_manifest_dir)
                    .or_default()
                    .push(rel_file.clone());
                continue;
            }

            if let Some((top_seg, _rest)) = rel_file.split_once('/') {
                if root_is_package && ROOT_SOURCE_SUBDIRS.contains(&top_seg) {
                    component_files
                        .entry(".".to_string())
                        .or_default()
                        .push(rel_file.clone());
                } else if let Some(dir_prefix) = find_first_non_container_dir_prefix(
                    rel_file,
                    norm_subpath.as_deref(),
                    &manifest_dirs,
                ) {
                    component_files
                        .entry(dir_prefix)
                        .or_default()
                        .push(rel_file.clone());
                }
            } else if root_is_package {
                component_files
                    .entry(".".to_string())
                    .or_default()
                    .push(rel_file.clone());
            }
        }

        let mut project = self.read_project()?;
        if options.subpath.is_none() {
            project.components.registry.retain(|c| {
                if component_files.contains_key(&c.relative_path) {
                    return true;
                }
                let dir_path = if c.relative_path == "." || c.relative_path.is_empty() {
                    project_root.clone()
                } else {
                    project_root.join(&c.relative_path)
                };
                dir_path.is_dir() && dir_path.join(COMPONENT_MANIFEST_FILE_NAME).is_file()
            });
        }

        let mut discovered_components = Vec::new();
        let mut manifests_written = Vec::new();
        let mut used_ids: BTreeSet<String> = project
            .components
            .registry
            .iter()
            .filter(|c| !component_files.contains_key(&c.relative_path))
            .map(|c| c.id.clone())
            .collect();

        // Pre-load existing on-disk and in-project manifests and reserve their IDs so newly
        // discovered directories never steal an ID from an existing component.
        let mut disk_manifests: BTreeMap<String, ComponentManifest> = BTreeMap::new();
        let mut proj_manifests: BTreeMap<String, ComponentManifest> = BTreeMap::new();
        let mut reserved_ids: BTreeMap<String, String> = BTreeMap::new();

        for comp_rel_dir in component_files.keys() {
            // Symlinked or escaping manifests are skipped (treated as absent) here; a later
            // write to the same component directory is rejected by `component_manifest_path`.
            let on_disk_manifest_path = self.component_manifest_path(comp_rel_dir).ok();
            if let Some(on_disk_manifest_path) = on_disk_manifest_path.filter(|p| p.is_file()) {
                if let Some(disk_manifest) = fs::read_to_string(&on_disk_manifest_path)
                    .ok()
                    .and_then(|raw| ComponentManifest::from_toml_str(&raw).ok())
                {
                    let id = disk_manifest.id.trim().to_string();
                    if is_valid_identifier(&id) && !used_ids.contains(&id) {
                        used_ids.insert(id.clone());
                        reserved_ids.insert(comp_rel_dir.clone(), id);
                    }
                    disk_manifests.insert(comp_rel_dir.clone(), disk_manifest);
                }
            }
            if let Some(proj_manifest) = project
                .components
                .registry
                .iter()
                .find(|c| &c.relative_path == comp_rel_dir)
                .cloned()
            {
                proj_manifests.insert(comp_rel_dir.clone(), proj_manifest);
            }
        }

        for (comp_rel_dir, proj_manifest) in &proj_manifests {
            if !reserved_ids.contains_key(comp_rel_dir) {
                let id = proj_manifest.id.trim().to_string();
                if is_valid_identifier(&id) && !used_ids.contains(&id) {
                    used_ids.insert(id.clone());
                    reserved_ids.insert(comp_rel_dir.clone(), id);
                }
            }
        }

        for (comp_rel_dir, mut files) in component_files {
            files.sort();
            files.dedup();
            // Do not list `nebulous.component.toml` itself as a tracked source file
            let filtered_files: Vec<String> = files
                .into_iter()
                .filter(|f| {
                    !f.ends_with(&format!("/{COMPONENT_MANIFEST_FILE_NAME}"))
                        && *f != COMPONENT_MANIFEST_FILE_NAME
                })
                .collect();

            let markers = manifest_dirs
                .get(&comp_rel_dir)
                .cloned()
                .unwrap_or_default();

            let existing_on_disk = disk_manifests.remove(&comp_rel_dir);
            let existing_in_project = proj_manifests.remove(&comp_rel_dir);
            let pre_reserved_id = reserved_ids.remove(&comp_rel_dir);

            let (inferred_type, inferred_stack) =
                infer_component_type_and_stack(&comp_rel_dir, &markers, &filtered_files);

            let mut manifest = if let Some(mut disk_manifest) = existing_on_disk {
                disk_manifest.relative_path = comp_rel_dir.clone();
                if !filtered_files.is_empty() {
                    disk_manifest.files = filtered_files;
                }
                for tech in inferred_stack {
                    if !disk_manifest.tech_stack.contains(&tech) {
                        disk_manifest.tech_stack.push(tech);
                    }
                }
                if (disk_manifest.component_type.trim().is_empty()
                    || disk_manifest.component_type == "custom")
                    && inferred_type != ComponentType::Custom
                {
                    disk_manifest.component_type = inferred_type.to_string();
                }
                if let Some(res_id) = pre_reserved_id {
                    disk_manifest.id = res_id;
                } else {
                    let base = if disk_manifest.id.trim().is_empty() {
                        slugify_project_id(&disk_manifest.name)
                    } else {
                        disk_manifest.id.clone()
                    };
                    disk_manifest.id =
                        allocate_unique_component_id(&used_ids, &base, &comp_rel_dir);
                }
                disk_manifest
            } else if let Some(mut proj_manifest) = existing_in_project {
                proj_manifest.relative_path = comp_rel_dir.clone();
                if !filtered_files.is_empty() {
                    proj_manifest.files = filtered_files;
                }
                for tech in inferred_stack {
                    if !proj_manifest.tech_stack.contains(&tech) {
                        proj_manifest.tech_stack.push(tech);
                    }
                }
                if let Some(res_id) = pre_reserved_id {
                    proj_manifest.id = res_id;
                } else {
                    proj_manifest.id =
                        allocate_unique_component_id(&used_ids, &proj_manifest.id, &comp_rel_dir);
                }
                proj_manifest
            } else {
                let raw_name = if comp_rel_dir == "." {
                    project.project.name.clone()
                } else {
                    comp_rel_dir
                        .rsplit('/')
                        .next()
                        .unwrap_or(&comp_rel_dir)
                        .to_string()
                };
                let base_id = slugify_project_id(&raw_name);
                let comp_id = allocate_unique_component_id(&used_ids, &base_id, &comp_rel_dir);
                ComponentManifest {
                    id: comp_id,
                    name: raw_name,
                    relative_path: comp_rel_dir.clone(),
                    component_type: inferred_type.to_string(),
                    description: None,
                    tech_stack: inferred_stack,
                    tags: Vec::new(),
                    flags: Vec::new(),
                    files: filtered_files,
                    dependencies: Vec::new(),
                    pinned_release: None,
                    rolling_updates: Vec::new(),
                }
            };

            manifest.normalize_paths();
            used_ids.insert(manifest.id.clone());

            if options.write_manifests {
                let written_path = self.write_component_file_only(&manifest)?;
                manifests_written.push(written_path);
            }

            project.components.upsert(manifest.clone());
            discovered_components.push(manifest);
        }

        discovered_components.sort_by(|a, b| a.id.cmp(&b.id));
        self.write_project(&project)?;

        let mut promoted_scopes = Vec::new();
        if options.promote_all {
            let target_phase = options.promote_phase_id.as_deref();
            for comp in &discovered_components {
                let scope = self.promote_component_to_scope(target_phase, &comp.id, None, None)?;
                promoted_scopes.push(scope.scope.id);
            }
        }

        let total_files_indexed = discovered_components.iter().map(|c| c.files.len()).sum();

        Ok(IngestionReport {
            components: discovered_components,
            total_files_indexed,
            manifests_written,
            promoted_scopes,
        })
    }

    /// Returns the filesystem path to `<relative_dir>/nebulous.component.toml` inside `project_root`.
    ///
    /// The path is resolved via [`crate::storage::contained_path`]: neither a symlinked component
    /// directory resolving outside the project root nor a symlinked manifest file is accepted, so
    /// every read and write of the component manifest is contained.
    pub fn component_manifest_path(&self, relative_dir: &str) -> Result<PathBuf> {
        let norm = normalize_relative_path_str(relative_dir);
        let rel = if norm.is_empty() || norm == "." {
            COMPONENT_MANIFEST_FILE_NAME.to_string()
        } else {
            format!("{norm}/{COMPONENT_MANIFEST_FILE_NAME}")
        };
        resolve_safe_relative_path(&self.project_root(), &rel)
    }

    /// Reads a [`ComponentManifest`] by `component_id` or `relative_path`, checking on-disk
    /// `nebulous.component.toml` first and falling back to `.tracker/project.toml` `[components]`.
    pub fn read_component_manifest(&self, id_or_rel_path: &str) -> Result<ComponentManifest> {
        let project = self.read_project()?;
        if let Some(entry) = project.components.find(id_or_rel_path) {
            let disk_path = self.component_manifest_path(&entry.relative_path)?;
            if disk_path.is_file() {
                let raw = fs::read_to_string(&disk_path).map_err(|source| TrackerError::Io {
                    path: disk_path.clone(),
                    source,
                })?;
                let mut disk_manifest =
                    ComponentManifest::from_toml_str(&raw).map_err(|err| match err {
                        TrackerError::TomlDeserialize { source, .. } => {
                            TrackerError::TomlDeserialize {
                                path: Some(disk_path),
                                source,
                            }
                        }
                        other => other,
                    })?;
                if disk_manifest.relative_path == "." && entry.relative_path != "." {
                    disk_manifest.relative_path = entry.relative_path.clone();
                }
                if disk_manifest.id.trim().is_empty() {
                    disk_manifest.id = entry.id.clone();
                }
                if disk_manifest.name.trim().is_empty() {
                    disk_manifest.name = entry.name.clone();
                }
                if disk_manifest.files.is_empty() && !entry.files.is_empty() {
                    disk_manifest.files = entry.files.clone();
                }
                disk_manifest.normalize_paths();
                return Ok(disk_manifest);
            }
            return Ok(entry.clone());
        }

        let direct_path = self.component_manifest_path(id_or_rel_path)?;
        if direct_path.is_file() {
            let raw = fs::read_to_string(&direct_path).map_err(|source| TrackerError::Io {
                path: direct_path.clone(),
                source,
            })?;
            let mut disk_manifest =
                ComponentManifest::from_toml_str(&raw).map_err(|err| match err {
                    TrackerError::TomlDeserialize { source, .. } => TrackerError::TomlDeserialize {
                        path: Some(direct_path.clone()),
                        source,
                    },
                    other => other,
                })?;
            let norm_rel = normalize_relative_path_str(id_or_rel_path);
            if disk_manifest.relative_path == "." && !norm_rel.is_empty() && norm_rel != "." {
                disk_manifest.relative_path = norm_rel;
            }
            if disk_manifest.files.is_empty() {
                if let Some(parent_dir) = direct_path.parent() {
                    disk_manifest.files = collect_directory_files(&self.project_root(), parent_dir);
                }
            }
            disk_manifest.normalize_paths();
            return Ok(disk_manifest);
        }

        Err(TrackerError::NotFound {
            kind: "Component",
            id: id_or_rel_path.to_string(),
        })
    }

    /// Creates or updates `<relative_path>/nebulous.component.toml` in the component directory
    /// and synchronizes the [`ComponentManifest`] into `.tracker/project.toml` `[components]`.
    ///
    /// Returns the path to the written `nebulous.component.toml` file.
    pub fn write_component_manifest(&self, manifest: &ComponentManifest) -> Result<PathBuf> {
        validate_identifier("Component", &manifest.id)?;
        let mut normalized = manifest.clone();
        normalized.normalize_paths();

        if normalized.files.is_empty() {
            let target_dir =
                if normalized.relative_path == "." || normalized.relative_path.is_empty() {
                    self.project_root()
                } else {
                    resolve_safe_relative_path(&self.project_root(), &normalized.relative_path)?
                };
            if target_dir.is_dir() {
                normalized.files = collect_directory_files(&self.project_root(), &target_dir);
            }
        } else {
            for f in &normalized.files {
                let _ = resolve_contained_reference(&self.project_root(), f)?;
            }
        }

        let path = self.write_component_file_only(&normalized)?;

        let mut project = self.read_project()?;
        project.components.upsert(normalized);
        self.write_project(&project)?;

        Ok(path)
    }

    fn write_component_file_only(&self, manifest: &ComponentManifest) -> Result<PathBuf> {
        let path = self.component_manifest_path(&manifest.relative_path)?;
        let serialized = manifest.to_toml_string()?;
        atomic_write_str(&path, &serialized)?;
        Ok(path)
    }

    /// Stage 2: Promotes an ingested component (and an optional subset of `selected_files`) into
    /// a Tier 3 `scope.toml` inside `phase_id` (defaulting to `project.active_phase`), binding
    /// `bound_component`, `tracked_files`, `target`, and `tech_stack`, and registering the scope
    /// in `phase.toml`.
    pub fn promote_component_to_scope(
        &self,
        phase_id: Option<&str>,
        component_id: &str,
        scope_id_override: Option<&str>,
        selected_files: Option<Vec<String>>,
    ) -> Result<ScopeManifest> {
        let component = self.read_component_manifest(component_id)?;
        let project = self.read_project()?;

        let resolved_phase = match phase_id.map(str::trim).filter(|s| !s.is_empty()) {
            Some(p) => self.ensure_canonical_phase_if_missing(p)?,
            None => resolve_phase_alias(&project.project.active_phase).to_string(),
        };

        let mut phase_manifest = self.read_phase(&resolved_phase)?;

        let resolved_scope_id = match scope_id_override.map(str::trim).filter(|s| !s.is_empty()) {
            Some(s) => {
                validate_identifier("Scope", s)?;
                s.to_string()
            }
            None => {
                validate_identifier("Scope", &component.id)?;
                component.id.clone()
            }
        };

        let tracked_files = match selected_files {
            Some(files) if !files.is_empty() => {
                for raw_file in &files {
                    let _ = resolve_contained_reference(&self.project_root(), raw_file)?;
                }
                let normalized = normalize_relative_path_list(files);
                let expanded: Vec<String> = normalized
                    .into_iter()
                    .map(|f| {
                        if component.relative_path != "."
                            && !f.starts_with(&format!("{}/", component.relative_path))
                        {
                            let prefixed = format!("{}/{f}", component.relative_path);
                            if component.files.contains(&prefixed)
                                || self.project_root().join(&prefixed).is_file()
                            {
                                return prefixed;
                            }
                        }
                        f
                    })
                    .collect();
                normalize_relative_path_list(expanded)
            }
            _ => component.files.clone(),
        };

        let scope_toml_path = self.scope_toml_path(&resolved_phase, &resolved_scope_id)?;
        let scope_manifest = if scope_toml_path.is_file() {
            let mut existing = self.read_scope(&resolved_phase, &resolved_scope_id)?;
            existing.scope.target = component.relative_path.clone();
            existing.scope.tech_stack = component.tech_stack.clone();
            existing.scope.bound_component = Some(component.id.clone());
            existing.scope.tracked_files = tracked_files;
            if existing.scope.description.is_none() {
                existing.scope.description = component.description.clone().or_else(|| {
                    Some(format!(
                        "Promoted from component '{}' ({})",
                        component.name, component.relative_path
                    ))
                });
            }
            existing
        } else {
            ScopeManifest {
                scope: ScopeMetadata {
                    id: resolved_scope_id.clone(),
                    target: component.relative_path.clone(),
                    tech_stack: component.tech_stack.clone(),
                    description: component.description.clone().or_else(|| {
                        Some(format!(
                            "Promoted from component '{}' ({})",
                            component.name, component.relative_path
                        ))
                    }),
                    bound_component: Some(component.id.clone()),
                    tracked_files,
                },
                items: ScopeItems::default(),
            }
        };

        self.write_scope(&resolved_phase, &resolved_scope_id, &scope_manifest)?;
        fs::create_dir_all(self.items_dir(&resolved_phase, &resolved_scope_id)?).map_err(
            |source| TrackerError::Io {
                path: self.tracker_dir().to_path_buf(),
                source,
            },
        )?;

        if let Some(entry) = phase_manifest
            .scopes
            .manifest
            .iter_mut()
            .find(|s| s.id == resolved_scope_id)
        {
            entry.scope_type = component.component_type.clone();
        } else {
            phase_manifest.scopes.manifest.push(ScopeRegistryEntry {
                id: resolved_scope_id.clone(),
                scope_type: component.component_type.clone(),
                path: format!("domains/{resolved_scope_id}"),
            });
        }
        self.write_phase(&resolved_phase, &phase_manifest)?;
        self.ensure_scope_across_phases(&scope_manifest.scope, Some(&component.component_type))?;

        Ok(scope_manifest)
    }

    /// Snapshots a component's current dependency versions, Git commit SHA, build script path,
    /// build flags, and optional verification record into `pinned_release` (`last_known_good`),
    /// persisting both `nebulous.component.toml` and `.tracker/project.toml`.
    pub fn pin_component_release(
        &self,
        component_id: &str,
        commit_sha: Option<&str>,
        build_script_path: Option<String>,
        build_flags: Vec<String>,
        verification: Option<VerificationRecord>,
    ) -> Result<ComponentManifest> {
        let mut manifest = self.read_component_manifest(component_id)?;
        let sha = commit_sha
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| self.git_head_short_sha());
        let now = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

        manifest.pin_current_release(sha, now, build_script_path, build_flags, verification);
        self.write_component_manifest(&manifest)?;
        Ok(manifest)
    }

    /// Records a [`RollingUpdateEntry`] on a component, updates the matching dependency's
    /// `current_version` if present, and persists `nebulous.component.toml` and `project.toml`.
    pub fn record_component_rolling_update(
        &self,
        component_id: &str,
        mut update: RollingUpdateEntry,
    ) -> Result<ComponentManifest> {
        let mut manifest = self.read_component_manifest(component_id)?;
        if update.recorded_at.is_none() {
            update.recorded_at = Some(Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
        }
        if let Some(dep) = manifest
            .dependencies
            .iter_mut()
            .find(|d| d.name == update.dependency_name)
        {
            if update.previous_version.is_none() && !dep.current_version.is_empty() {
                update.previous_version = Some(dep.current_version.clone());
            }
            if !update.new_version.is_empty() {
                dep.current_version = update.new_version.clone();
            }
        }
        manifest.rolling_updates.push(update);
        self.write_component_manifest(&manifest)?;
        Ok(manifest)
    }

    /// Records a [`VerificationRecord`] on the latest rolling update (and `pinned_release` if present)
    /// for `component_id`, persisting `nebulous.component.toml` and `project.toml`.
    pub fn record_component_verification(
        &self,
        component_id: &str,
        mut record: VerificationRecord,
    ) -> Result<ComponentManifest> {
        let mut manifest = self.read_component_manifest(component_id)?;
        if record.verified_at.is_none() {
            record.verified_at = Some(Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
        }
        if let Some(last_update) = manifest.rolling_updates.last_mut() {
            last_update.status = if record.passed {
                "passed".to_string()
            } else {
                "failed".to_string()
            };
            last_update.verification_run = Some(record.clone());
        }
        if let Some(pin) = &mut manifest.pinned_release {
            if record.passed {
                pin.last_verification_run = Some(record);
            }
        }
        self.write_component_manifest(&manifest)?;
        Ok(manifest)
    }

    /// Restores a component's dependency versions and build flags from its `pinned_release`
    /// (`last_known_good`), marks any pending/failed rolling updates as `"rolled_back"`, and
    /// persists `nebulous.component.toml` and `project.toml`.
    pub fn rollback_component_to_pinned(&self, component_id: &str) -> Result<ComponentManifest> {
        let mut manifest = self.read_component_manifest(component_id)?;
        if manifest.restore_from_pinned_release().is_none() {
            return Err(TrackerError::ValidationError(format!(
                "Component '{}' does not have a pinned_release configuration to roll back to",
                manifest.id
            )));
        }
        for update in &mut manifest.rolling_updates {
            if update.status == "pending" || update.status == "failed" {
                update.status = "rolled_back".to_string();
            }
        }
        self.write_component_manifest(&manifest)?;
        Ok(manifest)
    }
}

fn should_skip_entry(entry: &DirEntry, scan_root: &Path) -> bool {
    if entry.path() == scan_root {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
        return name.starts_with('.') || IGNORED_INGESTION_DIRS.contains(&name.as_ref());
    }
    false
}

fn collect_directory_files(project_root: &Path, target_dir: &Path) -> Vec<String> {
    let Ok(root_canon) = validate_safe_project_root(project_root) else {
        return Vec::new();
    };
    let Ok(target_canon) = fs::canonicalize(target_dir) else {
        return Vec::new();
    };
    if !target_canon.starts_with(&root_canon) || validate_safe_project_root(&target_canon).is_err()
    {
        return Vec::new();
    }

    let mut walk_builder = WalkBuilder::new(&target_canon);
    walk_builder
        .require_git(false)
        .git_ignore(true)
        .git_global(false)
        .git_exclude(true)
        .same_file_system(true)
        .follow_links(false)
        .hidden(false);

    if target_canon != root_canon {
        let root_gitignore = root_canon.join(".gitignore");
        if root_gitignore.is_file() {
            let _ = walk_builder.add_ignore(root_gitignore);
        }
    }

    let scan_root_clone = target_canon.clone();
    walk_builder.filter_entry(move |entry| !should_skip_entry(entry, &scan_root_clone));

    let mut files = Vec::new();
    for entry_res in walk_builder.build() {
        let entry = match entry_res {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy();
        if file_name == ".gitignore"
            || file_name == ".ignore"
            || file_name == COMPONENT_MANIFEST_FILE_NAME
        {
            continue;
        }
        if let Ok(rel) = path.strip_prefix(&root_canon) {
            let norm = normalize_relative_path(rel);
            if !norm.is_empty() {
                files.push(norm);
            }
        }
    }
    files.sort();
    files.dedup();
    files
}

fn find_deepest_non_root_manifest_dir(
    rel_file: &str,
    manifest_dirs: &BTreeMap<String, BTreeSet<String>>,
) -> Option<String> {
    let mut best: Option<&str> = None;
    for dir in manifest_dirs.keys() {
        if dir == "." {
            continue;
        }
        if let Some(rest) = rel_file.strip_prefix(dir.as_str()) {
            if rest.starts_with('/') {
                match best {
                    None => best = Some(dir.as_str()),
                    Some(prev) if dir.len() > prev.len() => best = Some(dir.as_str()),
                    _ => {}
                }
            }
        }
    }
    best.map(str::to_string)
}

fn find_first_non_container_dir_prefix(
    rel_file: &str,
    min_dir_prefix: Option<&str>,
    manifest_dirs: &BTreeMap<String, BTreeSet<String>>,
) -> Option<String> {
    let (parent_dir, _) = rel_file.rsplit_once('/')?;
    let mut current = String::new();
    for seg in parent_dir.split('/') {
        if seg.is_empty() {
            continue;
        }
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(seg);

        if let Some(min_prefix) = min_dir_prefix {
            if min_prefix.starts_with(&format!("{current}/")) {
                continue;
            }
        }

        let prefix_with_slash = format!("{current}/");
        let is_container_of_manifest = manifest_dirs
            .keys()
            .any(|k| k.starts_with(&prefix_with_slash));
        if !is_container_of_manifest {
            return Some(current);
        }
    }
    Some(parent_dir.to_string())
}

fn infer_component_type_and_stack(
    rel_dir: &str,
    markers: &BTreeSet<String>,
    files: &[String],
) -> (ComponentType, Vec<String>) {
    let dir_leaf = rel_dir
        .rsplit('/')
        .next()
        .unwrap_or(rel_dir)
        .to_ascii_lowercase();

    let mut stack = Vec::new();
    let mut push_tech = |t: &str| {
        let s = t.to_string();
        if !stack.contains(&s) {
            stack.push(s);
        }
    };

    if markers.contains("Cargo.toml") {
        push_tech("rust");
    }
    if markers.contains("pyproject.toml") {
        push_tech("python");
    }
    if markers.contains("package.json") {
        push_tech("node");
    }
    if markers.contains("CMakeLists.txt") {
        push_tech("c_cpp");
        push_tech("cmake");
    }
    if markers.contains("Makefile") {
        push_tech("make");
    }
    if markers.contains("Dockerfile") {
        push_tech("docker");
    }

    for f in files {
        let lower = f.to_ascii_lowercase();
        if lower.ends_with(".rs") {
            push_tech("rust");
        } else if lower.ends_with(".py") {
            push_tech("python");
        } else if lower.ends_with(".ts") || lower.ends_with(".tsx") {
            push_tech("typescript");
        } else if lower.ends_with(".js") || lower.ends_with(".jsx") {
            push_tech("javascript");
        } else if lower.ends_with(".c")
            || lower.ends_with(".cpp")
            || lower.ends_with(".cc")
            || lower.ends_with(".h")
            || lower.ends_with(".hpp")
        {
            push_tech("c_cpp");
        } else if lower.ends_with(".sh") || lower.ends_with(".bash") {
            push_tech("shell");
        }
    }

    let comp_type = if markers.contains("Cargo.toml") || stack.iter().any(|s| s == "rust") {
        if dir_leaf.ends_with("_cli") || dir_leaf == "cli" {
            ComponentType::CliCrate
        } else if dir_leaf.ends_with("_interface")
            || dir_leaf.ends_with("_ui")
            || dir_leaf.ends_with("_gui")
            || dir_leaf == "gui"
            || dir_leaf == "ui"
        {
            ComponentType::UiCrate
        } else {
            ComponentType::RustCrate
        }
    } else if markers.contains("pyproject.toml") || stack.iter().any(|s| s == "python") {
        ComponentType::PythonPkg
    } else if markers.contains("package.json")
        || stack
            .iter()
            .any(|s| s == "node" || s == "typescript" || s == "javascript")
    {
        ComponentType::NodePkg
    } else if dir_leaf.contains("kernel") || dir_leaf == "kmod" {
        ComponentType::KernelModule
    } else if matches!(
        dir_leaf.as_str(),
        "iso" | "os" | "image" | "distro" | "rootfs" | "live_build" | "live-build"
    ) {
        ComponentType::OsImage
    } else if markers.contains("CMakeLists.txt") || stack.iter().any(|s| s == "c_cpp") {
        ComponentType::CCpp
    } else if matches!(
        dir_leaf.as_str(),
        "scripts" | "ci" | "tools" | "packaging" | "build_scripts"
    ) || stack.iter().any(|s| s == "shell")
    {
        ComponentType::BuildScripts
    } else {
        ComponentType::Custom
    };

    (comp_type, stack)
}

/// Allocates a unique component ID that always satisfies the identifier allow-list.
///
/// IDs taken verbatim from an on-disk `nebulous.component.toml` (e.g. `my.crate`) or derived
/// from long directory names are normalized via [`normalize_identifier`] rather than rejected,
/// so ingestion of existing repositories keeps working.
fn allocate_unique_component_id(
    used_ids: &BTreeSet<String>,
    base_id: &str,
    rel_dir: &str,
) -> String {
    let base_id = normalize_identifier(base_id, MAX_IDENTIFIER_LEN);
    if !used_ids.contains(&base_id) {
        return base_id;
    }
    let path_slug = normalize_identifier(
        &slugify_project_id(&rel_dir.replace('/', "_")),
        MAX_IDENTIFIER_LEN,
    );
    if !path_slug.is_empty() && !used_ids.contains(&path_slug) {
        return path_slug;
    }
    // Leave room for a `_<n>` suffix within the length limit.
    let stem = normalize_identifier(&base_id, MAX_IDENTIFIER_LEN - 11);
    let mut idx = 2u32;
    loop {
        let candidate = format!("{stem}_{idx}");
        if !used_ids.contains(&candidate) {
            return candidate;
        }
        idx += 1;
    }
}
