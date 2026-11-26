use std::collections::HashMap;
use std::fs;
use std::hash::{DefaultHasher, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use notify::event::ModifyKind;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::error::{Result, TrackerError};
use crate::storage::{ReconcileReport, TrackerStorage, DEFAULT_LOCK_TIMEOUT};

/// Notification emitted by [`TrackerWatcher`] when `.tracker/` files change on disk.
#[derive(Debug, Clone)]
pub struct TrackerWatchEvent {
    pub changed_paths: Vec<PathBuf>,
    pub rescan: bool,
    pub reconcile_report: Option<ReconcileReport>,
    pub reconcile_error: Option<String>,
}

enum RawSignal {
    Paths(Vec<PathBuf>),
    Rescan,
}

/// Filesystem watcher around `.tracker/` backed by `notify`.
///
/// Automatically filters out read-only access events and temporary atomic-write sibling files
/// (`.tmp.`), reconciles `.tracker/` indexes on external changes, and forwards [`TrackerWatchEvent`]
/// notifications to subscribers (such as the `libcosmic` GUI subscription loop).
pub struct TrackerWatcher {
    watcher: Option<RecommendedWatcher>,
    worker: Option<JoinHandle<()>>,
    watched_dir: PathBuf,
    shutdown: Arc<AtomicBool>,
}

impl TrackerWatcher {
    /// Starts watching `storage.tracker_dir()` recursively and automatically runs
    /// [`TrackerStorage::reconcile_indexes`] when `.tracker/` files are created, modified, or removed.
    pub fn start(storage: TrackerStorage) -> Result<(Self, Receiver<TrackerWatchEvent>)> {
        Self::start_with_options(storage, true)
    }

    /// Starts watching `storage.tracker_dir()` recursively, optionally running
    /// [`TrackerStorage::reconcile_indexes`] automatically on each change batch.
    pub fn start_with_options(
        storage: TrackerStorage,
        auto_reconcile: bool,
    ) -> Result<(Self, Receiver<TrackerWatchEvent>)> {
        let watched_dir = storage.tracker_dir().to_path_buf();
        let (tx, rx) = mpsc::channel::<TrackerWatchEvent>();
        let (raw_tx, raw_rx) = mpsc::channel::<RawSignal>();

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                let Ok(event) = res else {
                    let _ = raw_tx.send(RawSignal::Rescan);
                    return;
                };

                if event.need_rescan() || matches!(event.kind, EventKind::Other) {
                    let _ = raw_tx.send(RawSignal::Rescan);
                    return;
                }

                if !is_mutating_event_kind(&event.kind) {
                    return;
                }

                let relevant_paths: Vec<PathBuf> = event
                    .paths
                    .into_iter()
                    .filter(|p| !is_atomic_temp_file(p))
                    .collect();

                if !relevant_paths.is_empty() {
                    let _ = raw_tx.send(RawSignal::Paths(relevant_paths));
                }
            },
            Config::default(),
        )?;

        watcher.watch(&watched_dir, RecursiveMode::Recursive)?;

        let storage_for_worker = storage;
        let watched_dir_for_worker = watched_dir.clone();
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_for_worker = Arc::clone(&shutdown);
        let worker = thread::spawn(move || {
            const LOCK_SLICE: Duration = Duration::from_millis(50);

            let mut own_index_hashes: HashMap<PathBuf, u64> = HashMap::new();
            let mut pending_batch: Option<Vec<PathBuf>> = None;
            let mut pending_rescan = false;

            'worker: loop {
                if shutdown_for_worker.load(Ordering::Acquire) {
                    break;
                }

                let mut rescan = std::mem::take(&mut pending_rescan);
                let mut batch = match pending_batch.take() {
                    Some(b) => b,
                    None if rescan => Vec::new(),
                    None => match raw_rx.recv() {
                        Ok(RawSignal::Paths(b)) => b,
                        Ok(RawSignal::Rescan) => {
                            rescan = true;
                            Vec::new()
                        }
                        Err(_) => break,
                    },
                };

                if shutdown_for_worker.load(Ordering::Acquire) {
                    break;
                }

                let mut deduped = Vec::new();
                extend_unique(&mut deduped, batch);
                batch = deduped;

                // 1. Coalesce additional paths arriving within a 50ms quiet/debounce window.
                loop {
                    if shutdown_for_worker.load(Ordering::Acquire) {
                        break 'worker;
                    }
                    match raw_rx.recv_timeout(Duration::from_millis(50)) {
                        Ok(RawSignal::Paths(paths)) => extend_unique(&mut batch, paths),
                        Ok(RawSignal::Rescan) => rescan = true,
                        Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => break 'worker,
                    }
                }

                if shutdown_for_worker.load(Ordering::Acquire) {
                    break;
                }

                if !rescan && batch.is_empty() {
                    continue;
                }

                if !rescan {
                    // 2. Suppress echo batches consisting solely of derived index files (`project.toml`,
                    // `phase.toml`, `scope.toml`) whose contents match our last internal reconcile.
                    // Why: an external edit to scope.toml right after an internal reconcile was being dropped.
                    if is_own_index_echo(&batch, &own_index_hashes) {
                        continue;
                    }

                    // 3. In-flight item directory settlement: if any path refers to or resides inside
                    // an existing `.../items/<ITEM_ID>` directory whose `item.toml` is not yet in place
                    // or still has an active `.tmp.` atomic write file, poll up to 300ms (in 15ms steps).
                    let settle_start = Instant::now();
                    while settle_start.elapsed() < Duration::from_millis(300) {
                        if shutdown_for_worker.load(Ordering::Acquire) {
                            break 'worker;
                        }
                        while let Ok(more) = raw_rx.try_recv() {
                            match more {
                                RawSignal::Paths(paths) => extend_unique(&mut batch, paths),
                                RawSignal::Rescan => rescan = true,
                            }
                        }
                        if rescan || !has_unsettled_item_dirs(&batch) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(15));
                    }
                    if shutdown_for_worker.load(Ordering::Acquire) {
                        break;
                    }
                    while let Ok(more) = raw_rx.try_recv() {
                        match more {
                            RawSignal::Paths(paths) => extend_unique(&mut batch, paths),
                            RawSignal::Rescan => rescan = true,
                        }
                    }
                }

                let changed_paths = if rescan {
                    let mut paths = vec![watched_dir_for_worker.clone()];
                    extend_unique(&mut paths, batch);
                    paths
                } else {
                    batch
                };

                // 4. Reconcile indexes if enabled, record index file content hashes, drain immediate
                // index-only echo events, and emit TrackerWatchEvent.
                let (reconcile_report, reconcile_error) = if auto_reconcile {
                    // Acquire `.tracker/.lock` in short slices so shutdown exits promptly under contention.
                    let lock_start = Instant::now();
                    let reconcile_result = loop {
                        if shutdown_for_worker.load(Ordering::Acquire) {
                            break 'worker;
                        }
                        let remaining = DEFAULT_LOCK_TIMEOUT.saturating_sub(lock_start.elapsed());
                        let slice = LOCK_SLICE.min(remaining);
                        match storage_for_worker
                            .with_write_lock_timeout(slice, |s| s.reconcile_indexes())
                        {
                            Err(TrackerError::LockTimeout { path, .. }) => {
                                if shutdown_for_worker.load(Ordering::Acquire) {
                                    break 'worker;
                                }
                                let elapsed = lock_start.elapsed();
                                if elapsed >= DEFAULT_LOCK_TIMEOUT {
                                    break Err(TrackerError::LockTimeout {
                                        path,
                                        waited_ms: elapsed.as_millis() as u64,
                                    });
                                }
                            }
                            other => break other,
                        }
                    };
                    if shutdown_for_worker.load(Ordering::Acquire) {
                        break;
                    }
                    // Why: errors were silently swallowed.
                    let (report, error) = match reconcile_result {
                        Ok(r) => (Some(r), None),
                        Err(e) => (None, Some(e.to_string())),
                    };
                    own_index_hashes = hash_index_files(&watched_dir_for_worker);
                    while let Ok(more) = raw_rx.try_recv() {
                        match more {
                            RawSignal::Paths(paths) => {
                                if is_own_index_echo(&paths, &own_index_hashes) {
                                    continue;
                                }
                                let entry = pending_batch.get_or_insert_with(Vec::new);
                                extend_unique(entry, paths);
                            }
                            RawSignal::Rescan => {
                                pending_rescan = true;
                            }
                        }
                    }
                    (report, error)
                } else {
                    (None, None)
                };

                if tx
                    .send(TrackerWatchEvent {
                        changed_paths,
                        rescan,
                        reconcile_report,
                        reconcile_error,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });

        Ok((
            Self {
                watcher: Some(watcher),
                worker: Some(worker),
                watched_dir,
                shutdown,
            },
            rx,
        ))
    }

    /// Returns the `.tracker/` directory path being watched.
    pub fn watched_dir(&self) -> &Path {
        &self.watched_dir
    }
}

impl Drop for TrackerWatcher {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        drop(self.watcher.take());
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl TrackerStorage {
    /// Starts a [`TrackerWatcher`] on this `.tracker/` directory.
    pub fn watch(&self) -> Result<(TrackerWatcher, Receiver<TrackerWatchEvent>)> {
        TrackerWatcher::start(self.clone())
    }
}

fn extend_unique(target: &mut Vec<PathBuf>, incoming: Vec<PathBuf>) {
    for path in incoming {
        if !target.contains(&path) {
            target.push(path);
        }
    }
}

fn has_unsettled_item_dirs(paths: &[PathBuf]) -> bool {
    let mut checked = Vec::new();
    for path in paths {
        if let Some(item_dir) = enclosing_item_dir(path) {
            if !checked.contains(&item_dir) {
                if is_item_dir_unsettled(&item_dir) {
                    return true;
                }
                checked.push(item_dir);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some("items") && path.is_dir() {
            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let sub = entry.path();
                    if sub.is_dir() && !checked.contains(&sub) {
                        if is_item_dir_unsettled(&sub) {
                            return true;
                        }
                        checked.push(sub);
                    }
                }
            }
        }
    }
    false
}

fn enclosing_item_dir(path: &Path) -> Option<PathBuf> {
    for ancestor in path.ancestors() {
        let parent = ancestor.parent()?;
        if parent.file_name().and_then(|n| n.to_str()) == Some("items") {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn is_item_dir_unsettled(item_dir: &Path) -> bool {
    if !item_dir.is_dir() {
        return false;
    }
    if !item_dir.join("item.toml").is_file() {
        return true;
    }
    if let Ok(entries) = fs::read_dir(item_dir) {
        for entry in entries.flatten() {
            if is_atomic_temp_file(&entry.path()) {
                return true;
            }
        }
    }
    false
}

fn is_mutating_event_kind(kind: &EventKind) -> bool {
    match kind {
        EventKind::Create(_) | EventKind::Remove(_) => true,
        EventKind::Modify(modify_kind) => !matches!(modify_kind, ModifyKind::Metadata(_)),
        _ => false,
    }
}

fn is_atomic_temp_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|name| name.contains(".tmp."))
        .unwrap_or(false)
}

fn is_derived_index_file(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|n| n.to_str()),
        Some("project.toml" | "phase.toml" | "scope.toml")
    )
}

/// Resolves `path` to its canonical path when possible because FSEvents reports real paths (`/private/var/...`) while the tracker path may be given through a symlink (`/var/...`).
fn canonical_or_self(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Hashes all derived index files (`project.toml`, `phase.toml`, `scope.toml`) under `dir`.
///
/// Not a security hash; only detects our own writes.
fn hash_index_files(dir: &Path) -> HashMap<PathBuf, u64> {
    let mut hashes = HashMap::new();
    let mut stack = vec![canonical_or_self(dir)];
    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() && is_derived_index_file(&path) {
                if let Some(hash) = hash_file_bytes(&path) {
                    hashes.insert(path, hash);
                }
            }
        }
    }
    hashes
}

fn hash_file_bytes(path: &Path) -> Option<u64> {
    if !path.is_file() {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    let mut hasher = DefaultHasher::new();
    hasher.write(&bytes);
    Some(hasher.finish())
}

fn is_own_index_echo(paths: &[PathBuf], own_index_hashes: &HashMap<PathBuf, u64>) -> bool {
    !paths.is_empty()
        && paths.iter().all(|p| {
            is_derived_index_file(p)
                && match (
                    own_index_hashes.get(&canonical_or_self(p)),
                    hash_file_bytes(p),
                ) {
                    (Some(&expected), Some(actual)) => actual == expected,
                    _ => false,
                }
        })
}

#[cfg(all(test, unix))]
mod echo_path_tests {
    use std::fs;
    use std::os::unix::fs::symlink;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::{hash_index_files, is_own_index_echo};

    #[test]
    fn own_index_echo_matches_through_symlinked_path() {
        let tmp = TempDir::new().expect("tempdir");
        let real: PathBuf = tmp.path().join("real");
        let link: PathBuf = tmp.path().join("link");
        fs::create_dir_all(&real).expect("create real dir");
        fs::write(real.join("scope.toml"), "a").expect("write scope.toml");
        symlink(&real, &link).expect("create symlink");

        let hashes = hash_index_files(&real);
        assert!(is_own_index_echo(&[link.join("scope.toml")], &hashes));

        let hashes = hash_index_files(&link);
        assert!(is_own_index_echo(&[real.join("scope.toml")], &hashes));

        fs::write(real.join("scope.toml"), "b").expect("overwrite scope.toml");
        assert!(!is_own_index_echo(&[link.join("scope.toml")], &hashes));
    }

    #[test]
    fn hash_index_files_skips_directory_symlink_cycles() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path();
        fs::write(root.join("scope.toml"), "scope = 1\n").expect("write scope.toml");
        let target_phase = root.join("target_phase.toml");
        fs::write(&target_phase, "phase = 1\n").expect("write target_phase.toml");

        symlink(".", root.join("loop_dir")).expect("create self-referential dir symlink");
        symlink(&target_phase, root.join("phase.toml")).expect("create phase.toml symlink");

        let hashes = hash_index_files(root);
        let canonical_root = fs::canonicalize(root).expect("canonicalize root");

        assert_eq!(hashes.len(), 1);
        assert!(hashes.contains_key(&canonical_root.join("scope.toml")));
        assert!(!hashes.contains_key(&canonical_root.join("phase.toml")));
        assert!(!hashes
            .keys()
            .any(|p| p.components().any(|c| c.as_os_str() == "loop_dir")));
    }
}
