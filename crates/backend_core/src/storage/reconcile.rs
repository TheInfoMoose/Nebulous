use std::cmp::Ordering;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

use crate::error::Result;
use crate::models::{
    ItemManifest, ItemStub, ItemType, PhaseRegistryEntry, ProjectCounters, ScopeRegistryEntry,
};
use crate::storage::id::parse_item_id_number;
use crate::storage::TrackerStorage;

/// Summary report returned by [`TrackerStorage::reconcile_indexes`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReconcileReport {
    pub phases_reconciled: usize,
    pub scopes_reconciled: usize,
    pub total_items: u32,
    pub completed_items: u32,
    pub counters: ProjectCounters,
}

impl TrackerStorage {
    /// Reconciles all `.tracker/` index files (`project.toml`, `phase.toml`, `scope.toml`)
    /// using `items/*/item.toml` as the authoritative source of truth.
    ///
    /// - Discovers any unregistered phase directories containing `phase.toml` and scope
    ///   directories containing `scope.toml`.
    /// - Rebuilds `scope.toml` `[items]` (`bugs` and `features` stubs) from `items/*/item.toml`.
    /// - Recomputes `phase.toml` `[summary]` (`total_items` and `completed_items`).
    /// - Reconciles `project.toml` `[counters]` so monotonic IDs never collide with existing items.
    pub fn reconcile_indexes(&self) -> Result<ReconcileReport> {
        let mut project = self.read_project()?;
        let mut max_counters = project.counters.clone();

        // Discover any phase directories on disk not yet listed in `project.phases.registry`
        let phases_root = self.phases_dir();
        if phases_root.is_dir() {
            let mut known_phase_ids: HashSet<String> = project
                .phases
                .registry
                .iter()
                .map(|p| p.id.clone())
                .collect();

            let mut dir_names: Vec<String> = fs::read_dir(&phases_root)
                .map_err(|source| crate::error::TrackerError::Io {
                    path: phases_root.clone(),
                    source,
                })?
                .flatten()
                .filter(|e| e.path().is_dir() && e.path().join("phase.toml").is_file())
                .filter_map(|e| e.file_name().into_string().ok())
                .collect();
            dir_names.sort();

            for dir_name in dir_names {
                let phase_toml_path = phases_root.join(&dir_name).join("phase.toml");
                let phase_manifest = self.read_phase_from_path(&phase_toml_path)?;
                if !known_phase_ids.contains(&phase_manifest.phase.id) {
                    known_phase_ids.insert(phase_manifest.phase.id.clone());
                    project.phases.registry.push(PhaseRegistryEntry {
                        id: phase_manifest.phase.id,
                        title: phase_manifest.phase.title,
                        path: format!("phases/{dir_name}"),
                    });
                }
            }
        }

        let mut report = ReconcileReport::default();
        let mut valid_phases = Vec::with_capacity(project.phases.registry.len());

        for mut phase_entry in project.phases.registry.drain(..) {
            let phase_dir = self.resolve_phase_path(&phase_entry.id, Some(&phase_entry.path))?;
            let phase_toml_path = phase_dir.join("phase.toml");
            if !phase_toml_path.is_file() {
                continue;
            }

            let mut phase_manifest = self.read_phase_from_path(&phase_toml_path)?;
            // Keep project phase title in sync with authoritative phase.toml title
            phase_entry.title = phase_manifest.phase.title.clone();

            // Discover any domain scope directories on disk not yet in `phase.scopes.manifest`
            let domains_dir = phase_dir.join("domains");
            if domains_dir.is_dir() {
                let mut known_scope_ids: HashSet<String> = phase_manifest
                    .scopes
                    .manifest
                    .iter()
                    .map(|s| s.id.clone())
                    .collect();

                let mut scope_dirs: Vec<String> = fs::read_dir(&domains_dir)
                    .map_err(|source| crate::error::TrackerError::Io {
                        path: domains_dir.clone(),
                        source,
                    })?
                    .flatten()
                    .filter(|e| e.path().is_dir() && e.path().join("scope.toml").is_file())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect();
                scope_dirs.sort();

                for dir_name in scope_dirs {
                    let scope_toml_path = domains_dir.join(&dir_name).join("scope.toml");
                    let scope_manifest = self.read_scope_from_path(&scope_toml_path)?;
                    if !known_scope_ids.contains(&scope_manifest.scope.id) {
                        known_scope_ids.insert(scope_manifest.scope.id.clone());
                        phase_manifest.scopes.manifest.push(ScopeRegistryEntry {
                            id: scope_manifest.scope.id.clone(),
                            scope_type: infer_default_scope_type(&scope_manifest.scope.id),
                            path: format!("domains/{dir_name}"),
                        });
                    }
                }
            }

            let mut phase_total_items: u32 = 0;
            let mut phase_completed_items: u32 = 0;
            let mut valid_scopes = Vec::with_capacity(phase_manifest.scopes.manifest.len());

            for scope_entry in phase_manifest.scopes.manifest.drain(..) {
                let scope_dir =
                    self.resolve_scope_path(&phase_dir, &scope_entry.id, Some(&scope_entry.path))?;
                let scope_toml_path = scope_dir.join("scope.toml");
                if !scope_toml_path.is_file() {
                    continue;
                }

                let mut scope_manifest = self.read_scope_from_path(&scope_toml_path)?;
                let items_dir = scope_dir.join("items");

                let (bugs, features, scope_completed) =
                    collect_authoritative_items(&items_dir, &mut max_counters)?;

                let scope_item_count = (bugs.len() + features.len()) as u32;
                phase_total_items += scope_item_count;
                phase_completed_items += scope_completed;

                scope_manifest.items.bugs = bugs;
                scope_manifest.items.features = features;

                self.write_scope_to_path(&scope_toml_path, &scope_manifest)?;
                report.scopes_reconciled += 1;
                valid_scopes.push(scope_entry);
            }

            phase_manifest.scopes.manifest = valid_scopes;
            phase_manifest.summary.total_items = phase_total_items;
            phase_manifest.summary.completed_items = phase_completed_items;

            self.write_phase_to_path(&phase_toml_path, &phase_manifest)?;

            report.phases_reconciled += 1;
            report.total_items += phase_total_items;
            report.completed_items += phase_completed_items;
            valid_phases.push(phase_entry);
        }

        project.phases.registry = valid_phases;
        if !project
            .phases
            .registry
            .iter()
            .any(|p| p.id == project.project.active_phase)
        {
            if let Some(first) = project.phases.registry.first() {
                project.project.active_phase = first.id.clone();
            }
        }

        project.counters.bug = project.counters.bug.max(max_counters.bug);
        project.counters.feature = project.counters.feature.max(max_counters.feature);
        report.counters = project.counters.clone();

        self.write_project(&project)?;

        Ok(report)
    }
}

fn collect_authoritative_items(
    items_dir: &Path,
    max_counters: &mut ProjectCounters,
) -> Result<(Vec<ItemStub>, Vec<ItemStub>, u32)> {
    let mut bugs = Vec::new();
    let mut features = Vec::new();
    let mut completed_count: u32 = 0;

    if !items_dir.is_dir() {
        return Ok((bugs, features, completed_count));
    }

    let entries = fs::read_dir(items_dir).map_err(|source| crate::error::TrackerError::Io {
        path: items_dir.to_path_buf(),
        source,
    })?;

    let mut item_dirs: Vec<std::path::PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    item_dirs.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    let mut parsed_items: Vec<(bool, ItemManifest)> = Vec::new();

    for item_dir in item_dirs {
        let dir_name = item_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();

        if let Some((kind, num)) = parse_item_id_number(&dir_name) {
            match kind {
                ItemType::Bug => max_counters.bug = max_counters.bug.max(num),
                ItemType::Feature => max_counters.feature = max_counters.feature.max(num),
            }
        }

        let item_toml_path = item_dir.join("item.toml");
        if !item_toml_path.is_file() {
            continue;
        }

        let raw = fs::read_to_string(&item_toml_path).map_err(|source| {
            crate::error::TrackerError::Io {
                path: item_toml_path.clone(),
                source,
            }
        })?;
        let manifest = ItemManifest::from_toml_str(&raw).map_err(|err| match err {
            crate::error::TrackerError::TomlDeserialize { source, .. } => {
                crate::error::TrackerError::TomlDeserialize {
                    path: Some(item_toml_path),
                    source,
                }
            }
            other => other,
        })?;

        if let Some((kind, num)) = parse_item_id_number(&manifest.item.id) {
            match kind {
                ItemType::Bug => max_counters.bug = max_counters.bug.max(num),
                ItemType::Feature => max_counters.feature = max_counters.feature.max(num),
            }
        }

        let is_canonical_dir = dir_name == manifest.item.id;
        parsed_items.push((is_canonical_dir, manifest));
    }

    // Prefer canonical directory names (`items/<ITEM_ID>`) if duplicate IDs exist across subdirectories
    parsed_items.sort_by(|(canon_a, _), (canon_b, _)| canon_b.cmp(canon_a));

    let mut seen_ids = HashSet::new();
    for (_, manifest) in parsed_items {
        if !seen_ids.insert(manifest.item.id.clone()) {
            continue;
        }

        if manifest.item.status.is_completed() {
            completed_count += 1;
        }

        let stub = manifest.to_stub();
        match manifest.item.item_type {
            ItemType::Bug => bugs.push(stub),
            ItemType::Feature => features.push(stub),
        }
    }

    bugs.sort_by(compare_item_stubs);
    features.sort_by(compare_item_stubs);

    Ok((bugs, features, completed_count))
}

fn compare_item_stubs(a: &ItemStub, b: &ItemStub) -> Ordering {
    match (parse_item_id_number(&a.id), parse_item_id_number(&b.id)) {
        (Some((type_a, num_a)), Some((type_b, num_b))) if type_a == type_b => {
            num_a.cmp(&num_b).then_with(|| a.id.cmp(&b.id))
        }
        _ => a.id.cmp(&b.id),
    }
}

pub(crate) fn infer_default_scope_type(scope_id: &str) -> String {
    match scope_id {
        "inbox" => "inbox".to_string(),
        s if s.ends_with("_cli") || s == "cli" => "cli_crate".to_string(),
        s if s.ends_with("_interface") || s.ends_with("_ui") || s == "gui" => {
            "ui_crate".to_string()
        }
        _ => "rust_crate".to_string(),
    }
}
