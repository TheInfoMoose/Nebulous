//! STEP-10: `neb done -c` commit scoping and atomicity.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use backend_core::{ItemStatus, Severity, TrackerStorage};
use git2::{Repository, Signature};
use tempfile::TempDir;

fn run_neb(root: &Path, cfg: &Path, args: &[&str]) -> Output {
    Command::new(PathBuf::from(env!("CARGO_BIN_EXE_neb")))
        .arg("--root")
        .arg(root)
        .args(args)
        .env("NEBULOUS_CONFIG_DIR", cfg)
        .output()
        .expect("should execute neb binary")
}

/// Repo with a committed `.tracker/` holding open BUG-0001 and a committed `src/lib.rs`.
fn setup() -> (TempDir, Repository, TrackerStorage) {
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Done Tester").unwrap();
        cfg.set_str("user.email", "done@nebulous.local").unwrap();
    }
    let storage = TrackerStorage::init(dir.path(), "DoneSafety", "01_core_engine", "Core").unwrap();
    storage
        .create_scope(
            Some("01_core_engine"),
            "backend_core",
            "src",
            vec!["rust".to_string()],
            None,
        )
        .unwrap();
    storage
        .create_bug(
            "Bug to close",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::Medium,
            None,
        )
        .unwrap();
    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "pub fn a() {}\n").unwrap();
    {
        let mut idx = repo.index().unwrap();
        idx.add_all([".tracker", "src"], git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        idx.write().unwrap();
        let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
        let sig = Signature::now("Done Tester", "done@nebulous.local").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
    }
    (dir, repo, storage)
}

#[test]
fn done_commit_does_not_commit_untracked_secret_env() {
    let (dir, repo, storage) = setup();
    let cfg = dir.path().join("cfg");
    fs::write(dir.path().join("secret.env"), "API_TOKEN=do-not-commit\n").unwrap();
    fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn a() { /* fixed */ }\n",
    )
    .unwrap();

    let out = run_neb(
        dir.path(),
        &cfg,
        &[
            "done",
            "BUG-0001",
            "-m",
            "Fix lib entrypoint safely",
            "-c",
            "--path",
            "src/lib.rs",
        ],
    );
    assert!(
        out.status.success(),
        "done -c failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let head = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(
        head.summary().unwrap().unwrap(),
        "[BUG-0001](backend_core) Fix lib entrypoint safely"
    );
    let tree = head.tree().unwrap();
    assert!(tree.get_path(Path::new("secret.env")).is_err());
    assert!(tree.get_path(Path::new("src/lib.rs")).is_ok());
    assert!(dir.path().join("secret.env").is_file());
    assert!(repo
        .status_file(Path::new("secret.env"))
        .unwrap()
        .contains(git2::Status::WT_NEW));

    let (_, detail) = storage.load_item_detail_by_id("BUG-0001").unwrap();
    assert_eq!(detail.manifest.item.status, ItemStatus::Closed);
    assert_eq!(detail.manifest.history.previous_fixes.len(), 1);

    // Without `--path`, `done -c` is a `.tracker/`-only commit: untracked files stay out.
    let (dir2, repo2, _storage2) = setup();
    fs::write(dir2.path().join("secret.env"), "API_TOKEN=x\n").unwrap();
    let out2 = run_neb(
        dir2.path(),
        &dir2.path().join("cfg"),
        &["done", "BUG-0001", "-m", "Close tracker only", "-c"],
    );
    assert!(
        out2.status.success(),
        "{}",
        String::from_utf8_lossy(&out2.stderr)
    );
    let tree2 = repo2.head().unwrap().peel_to_tree().unwrap();
    assert!(tree2.get_path(Path::new("secret.env")).is_err());
}

#[test]
fn policy_rejected_done_commit_leaves_item_open_without_fix_entry() {
    let (dir, repo, storage) = setup();
    let cfg = dir.path().join("cfg");
    let head_before = repo.head().unwrap().target().unwrap();
    let (_, before) = storage.load_item_detail_by_id("BUG-0001").unwrap();

    // `min_summary_length` defaults to 12, so "short" is rejected by the commit policy.
    let out = run_neb(dir.path(), &cfg, &["done", "BUG-0001", "-m", "short", "-c"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("at least 12"));

    let (_, after) = storage.load_item_detail_by_id("BUG-0001").unwrap();
    assert_eq!(after.manifest.item.status, before.manifest.item.status);
    assert_ne!(after.manifest.item.status, ItemStatus::Closed);
    assert!(after.manifest.history.previous_fixes.is_empty());
    assert_eq!(repo.head().unwrap().target().unwrap(), head_before);
    assert!(!storage.git_status().unwrap().tracker_dirty);

    // A glob `--path` is likewise rejected before the item is touched.
    let glob = run_neb(
        dir.path(),
        &cfg,
        &[
            "done",
            "BUG-0001",
            "-m",
            "Glob path must be rejected",
            "-c",
            "--path",
            "src/*",
        ],
    );
    assert!(!glob.status.success());
    let (_, after_glob) = storage.load_item_detail_by_id("BUG-0001").unwrap();
    assert_ne!(after_glob.manifest.item.status, ItemStatus::Closed);
    assert!(after_glob.manifest.history.previous_fixes.is_empty());
}
