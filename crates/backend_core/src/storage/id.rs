use std::fs;
use std::path::Path;

use rand::{rngs::OsRng, RngCore};

use crate::error::{Result, TrackerError};
use crate::models::item::is_hash_item_id;
use crate::models::project::IdScheme;
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
    ItemType::from_prefix(prefix).map(|kind| (kind, number))
}

fn record_id_in_counters(counters: &mut ProjectCounters, id: &str) {
    if let Some((item_type, number)) = parse_item_id_number(id) {
        let slot = counters.get_mut(item_type);
        *slot = (*slot).max(number);
    }
}

pub(crate) const MAX_HASH_ID_ATTEMPTS: u32 = 16;

impl TrackerStorage {
    /// Scans all phases, domain scopes (`scope.toml`), item directory names (`items/<ID>`),
    /// and authoritative `item.toml` manifests to find the highest existing `BUG-*` and `FEAT-*`
    /// sequence numbers on disk.
    pub fn scan_max_counters(&self) -> Result<ProjectCounters> {
        let mut counters = ProjectCounters::default();

        if let Ok(project) = self.read_project() {
            counters.merge_max(&project.counters);

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

    pub(crate) fn allocate_hash_id(
        &self,
        item_type: ItemType,
        mut draw: impl FnMut() -> u32,
    ) -> Result<String> {
        for _ in 0..MAX_HASH_ID_ATTEMPTS {
            let id = item_type.format_hash_id(draw());
            if !is_hash_item_id(&id) {
                continue;
            }
            match self.locate_item(&id) {
                Ok(_) => continue,
                Err(TrackerError::NotFound { .. }) => return Ok(id),
                Err(e) => return Err(e),
            }
        }

        Err(TrackerError::ValidationError(format!(
            "could not allocate a unique hash ID after {MAX_HASH_ID_ATTEMPTS} attempts"
        )))
    }

    /// Allocates the next item ID according to the project's [`IdScheme`]:
    /// - [`IdScheme::Sequential`]: allocates the next global monotonic ID (`BUG-XXXX` or `FEAT-XXXX`),
    ///   reconciling `project.toml` `[counters]` against a directory scan safety check and persisting
    ///   the updated counter atomically to `project.toml`.
    /// - [`IdScheme::Hash`]: draws random 24-bit hash IDs (`BUG-XXXXXX` or `FEAT-XXXXXX`), redrawing
    ///   all-digit or already-used IDs, and leaves `[counters]` untouched.
    pub fn allocate_next_id(&self, item_type: ItemType) -> Result<String> {
        self.with_write_lock(|_| {
            let mut project = self.read_project()?;
            if project.project.id_scheme == IdScheme::Hash {
                return self.allocate_hash_id(item_type, || OsRng.next_u32());
            }
            let scanned = self.scan_max_counters()?;

            project.counters.merge_max(&scanned);

            let slot = project.counters.get_mut(item_type);
            let next_num = slot.checked_add(1).ok_or_else(|| {
                TrackerError::ValidationError(format!(
                    "{} counter overflowed u32::MAX",
                    item_type.id_prefix()
                ))
            })?;
            *slot = next_num;

            self.write_project(&project)?;
            Ok(item_type.format_id(next_num))
        })
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
            for (_, stub) in scope_manifest.items.iter() {
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

#[cfg(test)]
mod hash_id_tests {
    use super::*;

    #[test]
    fn allocate_hash_id_redraws_all_digit_and_taken_ids() {
        let dir = tempfile::tempdir().unwrap();
        let storage = TrackerStorage::init_standard(dir.path(), "hash-test").unwrap();
        let mut p = storage.read_project().unwrap();
        p.project.id_scheme = IdScheme::Hash;
        storage.write_project(&p).unwrap();

        let taken = storage
            .create_bug("taken", None, None, crate::models::Severity::Medium, None)
            .unwrap()
            .0
            .stub
            .id;
        let taken_bits = u32::from_str_radix(&taken[4..], 16).unwrap();

        let scripted = [0x123456, taken_bits, 0x00ABCD];
        let mut calls = 0usize;
        let allocated = storage
            .allocate_hash_id(ItemType::Bug, || {
                let val = scripted[calls];
                calls += 1;
                val
            })
            .unwrap();

        assert_eq!(allocated, "BUG-00ABCD");
        assert_eq!(calls, 3);
    }

    #[test]
    fn allocate_hash_id_fails_after_max_attempts_on_repeated_invalid_draws() {
        let dir = tempfile::tempdir().unwrap();
        let storage = TrackerStorage::init_standard(dir.path(), "hash-test").unwrap();
        let mut p = storage.read_project().unwrap();
        p.project.id_scheme = IdScheme::Hash;
        storage.write_project(&p).unwrap();

        let mut calls = 0u32;
        let err = storage
            .allocate_hash_id(ItemType::Bug, || {
                calls += 1;
                0x111111
            })
            .unwrap_err();

        assert_eq!(calls, 16);
        assert!(
            matches!(err, TrackerError::ValidationError(ref msg) if msg.contains("16 attempts")),
            "expected ValidationError after 16 attempts, got {err:?}"
        );
    }
}
