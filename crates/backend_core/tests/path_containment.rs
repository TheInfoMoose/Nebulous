//! STEP-12: identifier allow-list and symlink-aware path containment for `.tracker/` and
//! component manifests.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use backend_core::{
    contained_path, is_valid_identifier, ComponentManifest, IngestionOptions, TrackerError,
    TrackerStorage,
};

static COUNTER: AtomicU64 = AtomicU64::new(1);

struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "nebulous_containment_{tag}_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn identifier_allow_list_rejects_escapes_and_reserved_names() {
    for bad in [
        "C:x", "c:", "CON", "con", "Nul", "COM1", "lpt9", "a/b", "a\\b", "..", ".", "a.", "a.b",
        "", " a", "a ", "-a", "_a", "a\0b", "a:b",
    ] {
        assert!(!is_valid_identifier(bad), "{bad:?} must be rejected");
    }
    assert!(!is_valid_identifier(&"a".repeat(65)));

    for good in [
        "BUG-0001",
        "FEAT-0001",
        "pre_production",
        "production",
        "post_production",
        "inbox",
        "backend_core",
        "nebulous_cli",
        "01_core_engine",
        "console",
        "COM10",
    ] {
        assert!(is_valid_identifier(good), "{good:?} must be accepted");
    }
    assert!(is_valid_identifier(&"a".repeat(64)));
}

#[test]
fn storage_paths_reject_invalid_identifiers() {
    let temp = TempDir::new("ids");
    let storage = TrackerStorage::init_standard(temp.path(), "ids").unwrap();
    for bad in ["C:x", "CON", "a/b", "..", "a."] {
        assert!(
            matches!(
                storage.item_dir("pre_production", "inbox", bad),
                Err(TrackerError::ValidationError(_))
            ),
            "item id {bad:?}"
        );
        assert!(
            storage.scope_dir("pre_production", bad).is_err(),
            "scope id {bad:?}"
        );
        assert!(storage.phase_dir(bad).is_err(), "phase id {bad:?}");
    }
}

#[test]
fn contained_path_rejects_parent_and_absolute_components() {
    let temp = TempDir::new("lexical");
    assert!(contained_path(temp.path(), "../x").is_err());
    assert!(contained_path(temp.path(), "/etc/passwd").is_err());
    assert_eq!(
        contained_path(temp.path(), "a/b/c").unwrap(),
        temp.path().join("a/b/c")
    );
}

#[test]
fn ingestion_normalizes_dotted_on_disk_component_ids() {
    let temp = TempDir::new("dotted");
    let storage = TrackerStorage::init_standard(temp.path(), "proj").unwrap();
    let comp = temp.path().join("serde.json");
    fs::create_dir_all(&comp).unwrap();
    fs::write(
        comp.join("Cargo.toml"),
        "[package]\nname = \"serde.json\"\n",
    )
    .unwrap();
    fs::write(
        comp.join("nebulous.component.toml"),
        "id = \"my.crate\"\nname = \"my.crate\"\nrelative_path = \"serde.json\"\n",
    )
    .unwrap();

    let report = storage
        .ingest_project_directory(IngestionOptions::new().with_write_manifests(true))
        .expect("ingestion must normalize, not reject, dotted ids");
    let ids: Vec<&str> = report.components.iter().map(|c| c.id.as_str()).collect();
    assert!(ids.contains(&"my_crate"), "ids: {ids:?}");
    assert!(ids.iter().all(|id| is_valid_identifier(id)), "ids: {ids:?}");
}

#[cfg(unix)]
mod unix {
    use super::*;
    use std::os::unix::fs::symlink;

    fn assert_rejected<T: std::fmt::Debug>(result: backend_core::Result<T>, what: &str) {
        match result {
            Err(TrackerError::ValidationError(msg)) => {
                assert!(
                    msg.contains("escapes") || msg.contains("symlink"),
                    "{what}: unexpected message {msg}"
                );
            }
            other => panic!("{what}: expected containment rejection, got {other:?}"),
        }
    }

    #[test]
    fn symlinked_items_dir_rejected_on_read_and_write() {
        let temp = TempDir::new("items_link");
        let outside = TempDir::new("items_outside");
        let storage = TrackerStorage::init_standard(temp.path(), "proj").unwrap();

        // Plant a plausible item outside the tracker and point `items/` at it.
        let outside_item = outside.path().join("BUG-0001");
        fs::create_dir_all(&outside_item).unwrap();
        fs::write(outside_item.join("item.toml"), "stolen = true\n").unwrap();
        fs::write(outside_item.join("desc.md"), "secret\n").unwrap();

        let items_dir = storage.items_dir("pre_production", "inbox").unwrap();
        fs::remove_dir_all(&items_dir).unwrap();
        symlink(outside.path(), &items_dir).unwrap();

        assert_rejected(
            storage.read_item_manifest("pre_production", "inbox", "BUG-0001"),
            "read item.toml",
        );
        assert_rejected(
            storage.read_item_desc("pre_production", "inbox", "BUG-0001"),
            "read desc.md",
        );
        assert_rejected(
            storage.write_item_desc("pre_production", "inbox", "BUG-0001", "pwned"),
            "write desc.md",
        );
        assert_rejected(
            storage.write_item_desc("pre_production", "inbox", "BUG-0002", "pwned"),
            "write new item",
        );
        assert_rejected(
            storage.ensure_items_dir("pre_production", "inbox"),
            "ensure items dir",
        );
        assert_eq!(
            fs::read_to_string(outside_item.join("desc.md")).unwrap(),
            "secret\n"
        );
        assert!(!outside.path().join("BUG-0002").exists());
    }

    #[test]
    fn symlinked_desc_md_outside_tracker_rejected() {
        let temp = TempDir::new("desc_link");
        let outside = TempDir::new("desc_outside");
        let storage = TrackerStorage::init_standard(temp.path(), "proj").unwrap();

        storage
            .write_item_desc("pre_production", "inbox", "BUG-0001", "legit\n")
            .unwrap();
        let desc = storage
            .item_desc_path("pre_production", "inbox", "BUG-0001")
            .unwrap();
        let secret = outside.path().join("id_ed25519");
        fs::write(&secret, "PRIVATE KEY\n").unwrap();
        fs::remove_file(&desc).unwrap();
        symlink(&secret, &desc).unwrap();

        assert_rejected(
            storage.read_item_desc("pre_production", "inbox", "BUG-0001"),
            "read desc.md",
        );
        assert_rejected(
            storage.write_item_desc("pre_production", "inbox", "BUG-0001", "pwned"),
            "write desc.md",
        );
        assert_eq!(fs::read_to_string(&secret).unwrap(), "PRIVATE KEY\n");
        assert!(fs::symlink_metadata(&desc)
            .unwrap()
            .file_type()
            .is_symlink());

        // Even a symlink that stays inside `.tracker/` is refused as a final component.
        fs::remove_file(&desc).unwrap();
        symlink(storage.project_toml_path(), &desc).unwrap();
        assert_rejected(
            storage.read_item_desc("pre_production", "inbox", "BUG-0001"),
            "read in-tracker symlink",
        );
    }

    #[test]
    fn symlinked_timer_and_phase_dir_rejected() {
        let temp = TempDir::new("timer_link");
        let outside = TempDir::new("timer_outside");
        let storage = TrackerStorage::init_standard(temp.path(), "proj").unwrap();

        let target = outside.path().join("timer.toml");
        fs::write(&target, "item_id = \"BUG-0001\"\n").unwrap();
        symlink(&target, storage.timer_toml_path()).unwrap();
        assert_rejected(storage.read_active_timer(), "read timer");
        assert_rejected(storage.clear_active_timer(), "clear timer");
        assert!(target.exists());

        let phase_dir = storage.phase_dir("production").unwrap();
        let moved = outside.path().join("production");
        fs::rename(&phase_dir, &moved).unwrap();
        symlink(&moved, &phase_dir).unwrap();
        assert!(storage.read_phase("production").is_err());
        assert!(storage.load_index().is_err());
    }

    #[test]
    fn symlinked_component_dir_rejected_for_manifest_write() {
        let temp = TempDir::new("comp_link");
        let outside = TempDir::new("comp_outside");
        let storage = TrackerStorage::init_standard(temp.path(), "proj").unwrap();

        fs::write(outside.path().join("main.rs"), "fn main() {}\n").unwrap();
        symlink(outside.path(), temp.path().join("evil")).unwrap();

        assert_rejected(
            storage.component_manifest_path("evil"),
            "component_manifest_path",
        );
        let manifest = ComponentManifest::new("evil", "Evil", "evil", "custom");
        assert_rejected(
            storage.write_component_manifest(&manifest),
            "write_component_manifest",
        );
        assert!(!outside.path().join("nebulous.component.toml").exists());

        // A symlinked manifest file inside a real component directory is rejected too.
        fs::create_dir_all(temp.path().join("real")).unwrap();
        let foreign = outside.path().join("foreign.toml");
        fs::write(&foreign, "id = \"x\"\n").unwrap();
        symlink(&foreign, temp.path().join("real/nebulous.component.toml")).unwrap();
        let manifest = ComponentManifest::new("real", "Real", "real", "custom");
        assert_rejected(
            storage.write_component_manifest(&manifest),
            "write via symlinked manifest file",
        );
        assert_eq!(fs::read_to_string(&foreign).unwrap(), "id = \"x\"\n");
    }
}
