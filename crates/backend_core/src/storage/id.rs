use std::fs;
use std::path::Path;

use crate::error::{Result, TrackerError};
use crate::models::{ItemManifest, ItemType, ProjectCounters, ScopeManifest};
use crate::storage::TrackerStorage;

/// Parses a canonical item identifier (e.g. `"BUG-0001"` or `"FEAT-0042"`) into its [`ItemType`]
/// and numeric sequence value.
pub fn parse_item_id_number(id: &str) -> Option<(ItemType, u32)> {
    let trimmed = id.trim();
    let (prefix, digits) = trimmed.split_once('-')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let number: u32 = digits.parse().ok()?;
    match prefix.to_ascii_uppercase().as_str() {
        "BUG" => Some((ItemType::Bug, number)),
        "FEAT" | "FEATURE" => Some((ItemType::Feature, number)),
        _ => None,
    }
}

fn record_id_in_counters(counters: &mut ProjectCounters, id: &str) {
    if let Some((item_type, number)) = parse_item_id_number(id) {
        match item_type {
            ItemType::Bug => counters.bug = counters.bug.max(number),
            ItemType::Feature => counters.feature = counters.feature.max(number),
        }
    }
}

impl TrackerStorage {
    /// Scans all phases, domain scopes (`scope.toml`), item directory names (`items/<ID>`),
    /// and authoritative `item.toml` manifests to find the highest existing `BUG-*` and `FEAT-*`
    /// sequence numbers on disk.
    pub fn scan_max_counters(&self) -> Result<ProjectCounters> {
        let mut counters = ProjectCounters::default();

        if let Ok(project) = self.read_project() {
            counters.bug = counters.bug.max(project.counters.bug);
            counters.feature = counters.feature.max(project.counters.feature);

            for phase_entry in &project.phases.registry {
                let phase_dir =
                    self.resolve_phase_path(&phase_entry.id, Some(&phase_entry.path))?;
                self.scan_phase_dir(&phase_dir, &mut counters);
            }
        }

        let phases_root = self.phases_dir();
        if phases_root.is_dir() {
            if let Ok(entries) = fs::read_dir(&phases_root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        self.scan_phase_dir(&path, &mut counters);
                    }
                }
            }
        }

        Ok(counters)
    }

    /// Allocates the next global monotonic item ID (`BUG-XXXX` or `FEAT-XXXX`), reconciling
    /// `project.toml` `[counters]` against a directory scan safety check and persisting the
    /// updated counter atomically to `project.toml`.
    pub fn allocate_next_id(&self, item_type: ItemType) -> Result<String> {
        let mut project = self.read_project()?;
        let scanned = self.scan_max_counters()?;

        project.counters.bug = project.counters.bug.max(scanned.bug);
        project.counters.feature = project.counters.feature.max(scanned.feature);

        let next_num = match item_type {
            ItemType::Bug => {
                let next = project.counters.bug.checked_add(1).ok_or_else(|| {
                    TrackerError::ValidationError("BUG counter overflowed u32::MAX".to_string())
                })?;
                project.counters.bug = next;
                next
            }
            ItemType::Feature => {
                let next = project.counters.feature.checked_add(1).ok_or_else(|| {
                    TrackerError::ValidationError("FEAT counter overflowed u32::MAX".to_string())
                })?;
                project.counters.feature = next;
                next
            }
        };

        self.write_project(&project)?;
        Ok(item_type.format_id(next_num))
    }

    fn scan_phase_dir(&self, phase_dir: &Path, counters: &mut ProjectCounters) {
        let phase_toml = phase_dir.join("phase.toml");
        if let Ok(phase_manifest) = self.read_phase_from_path(&phase_toml) {
            for scope_entry in &phase_manifest.scopes.manifest {
                if let Ok(scope_dir) =
                    self.resolve_scope_path(phase_dir, &scope_entry.id, Some(&scope_entry.path))
                {
                    scan_scope_dir(self, &scope_dir, counters);
                }
            }
        }

        let domains_dir = phase_dir.join("domains");
        if !domains_dir.is_dir() {
            return;
        }

        let Ok(scope_entries) = fs::read_dir(&domains_dir) else {
            return;
        };

        for scope_entry in scope_entries.flatten() {
            let scope_dir = scope_entry.path();
            if scope_dir.is_dir() {
                scan_scope_dir(self, &scope_dir, counters);
            }
        }
    }
}

fn scan_scope_dir(storage: &TrackerStorage, scope_dir: &Path, counters: &mut ProjectCounters) {
    if !scope_dir.is_dir() {
        return;
    }

    // Check stubs in scope.toml if readable
    let scope_toml = scope_dir.join("scope.toml");
    if let Ok(raw) = storage.read_tracker_file(&scope_toml) {
        if let Ok(scope_manifest) = ScopeManifest::from_toml_str(&raw) {
            for stub in &scope_manifest.items.bugs {
                record_id_in_counters(counters, &stub.id);
            }
            for stub in &scope_manifest.items.features {
                record_id_in_counters(counters, &stub.id);
            }
        }
    }

    // Check items/* directories and item.toml files
    let items_dir = scope_dir.join("items");
    if !items_dir.is_dir() {
        return;
    }
    let Ok(item_entries) = fs::read_dir(&items_dir) else {
        return;
    };
    for item_entry in item_entries.flatten() {
        let item_dir = item_entry.path();
        if !item_dir.is_dir() {
            continue;
        }
        if let Some(dir_name) = item_dir.file_name().and_then(|n| n.to_str()) {
            record_id_in_counters(counters, dir_name);
        }
        let item_toml = item_dir.join("item.toml");
        if let Ok(raw) = storage.read_tracker_file(&item_toml) {
            if let Ok(item_manifest) = ItemManifest::from_toml_str(&raw) {
                record_id_in_counters(counters, &item_manifest.item.id);
            }
        }
    }
}
