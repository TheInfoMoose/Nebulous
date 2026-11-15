use std::fs;
use std::time::{Duration, Instant};

use backend_core::models::item::Severity;
use backend_core::storage::TrackerStorage;
use backend_core::watcher::TrackerWatcher;
use tempfile::tempdir;

#[test]
fn reconcile_error_is_reported() {
    let dir = tempdir().expect("create tempdir");
    let storage =
        TrackerStorage::init_standard(dir.path(), "test-project").expect("init standard tracker");
    let (_watcher, rx) = TrackerWatcher::start(storage.clone()).expect("start watcher");

    fs::write(storage.project_toml_path(), "not = [valid").expect("overwrite project.toml");

    let event = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("receive watch event within 5s");
    assert!(event.reconcile_error.is_some());
    assert!(event.reconcile_report.is_none());
}

#[test]
fn drop_joins_worker_promptly() {
    let dir = tempdir().expect("create tempdir");
    let storage =
        TrackerStorage::init_standard(dir.path(), "test-project").expect("init standard tracker");
    let (watcher, _rx) = TrackerWatcher::start(storage).expect("start watcher");

    let start = Instant::now();
    drop(watcher);
    assert!(start.elapsed() < Duration::from_secs(2));
}

#[test]
fn external_scope_edit_after_reconcile_is_processed() {
    let dir = tempdir().expect("create tempdir");
    let storage =
        TrackerStorage::init_standard(dir.path(), "test-project").expect("init standard tracker");
    let (_watcher, rx) = TrackerWatcher::start(storage.clone()).expect("start watcher");

    storage
        .create_bug("test bug", None, None, Severity::Medium, None)
        .expect("create BUG-0001");

    let first_event = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("receive first event within 5s");
    assert!(first_event.reconcile_report.is_some());

    let scope_toml_path = storage
        .scope_toml_path("pre_production", "inbox")
        .expect("inbox scope.toml path");
    let existing = fs::read_to_string(&scope_toml_path).expect("read scope.toml");
    fs::write(&scope_toml_path, format!("{existing}\n# external\n")).expect("rewrite scope.toml");

    let second_event = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("receive second event within 5s");
    assert!(second_event.reconcile_report.is_some());
}
