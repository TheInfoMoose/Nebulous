use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use notify::event::ModifyKind;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::error::Result;
use crate::storage::{ReconcileReport, TrackerStorage};

/// Notification emitted by [`TrackerWatcher`] when `.tracker/` files change on disk.
#[derive(Debug, Clone)]
pub struct TrackerWatchEvent {
    pub changed_paths: Vec<PathBuf>,
    pub reconcile_report: Option<ReconcileReport>,
}

/// Filesystem watcher around `.tracker/` backed by `notify`.
///
/// Automatically filters out read-only access events and temporary atomic-write sibling files
/// (`.tmp.`), reconciles `.tracker/` indexes on external changes, and forwards [`TrackerWatchEvent`]
/// notifications to subscribers (such as the `libcosmic` GUI subscription loop).
pub struct TrackerWatcher {
    _watcher: RecommendedWatcher,
    watched_dir: PathBuf,
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
        let (raw_tx, raw_rx) = mpsc::channel::<Vec<PathBuf>>();

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                let Ok(event) = res else {
                    return;
                };

                if !is_mutating_event_kind(&event.kind) {
                    return;
                }

                let relevant_paths: Vec<PathBuf> = event
                    .paths
                    .into_iter()
                    .filter(|p| !is_atomic_temp_file(p))
                    .collect();

                if !relevant_paths.is_empty() {
                    let _ = raw_tx.send(relevant_paths);
                }
            },
            Config::default(),
        )?;

        watcher.watch(&watched_dir, RecursiveMode::Recursive)?;

        let storage_for_worker = storage;
        thread::spawn(move || {
            let mut last_internal_reconcile: Option<Instant> = None;
            let mut pending_batch: Option<Vec<PathBuf>> = None;

            loop {
                let mut batch = match pending_batch.take() {
                    Some(b) => b,
                    None => match raw_rx.recv() {
                        Ok(b) => b,
                        Err(_) => break,
                    },
                };

                let mut deduped = Vec::new();
                extend_unique(&mut deduped, batch);
                batch = deduped;

                // 1. Coalesce additional paths arriving within a 50ms quiet/debounce window.
                while let Ok(more) = raw_rx.recv_timeout(Duration::from_millis(50)) {
                    extend_unique(&mut batch, more);
                }

                if batch.is_empty() {
                    continue;
                }

                // 2. Suppress echo batches consisting solely of derived index files (`project.toml`,
                // `phase.toml`, `scope.toml`) within 1000ms of an internal auto-reconcile.
                let all_are_index_files = batch.iter().all(|p| is_derived_index_file(p));
                if all_are_index_files
                    && last_internal_reconcile
                        .map(|last| last.elapsed() < Duration::from_millis(1000))
                        .unwrap_or(false)
                {
                    continue;
                }

                // 3. In-flight item directory settlement: if any path refers to or resides inside
                // an existing `.../items/<ITEM_ID>` directory whose `item.toml` is not yet in place
                // or still has an active `.tmp.` atomic write file, poll up to 300ms (in 15ms steps).
                let settle_start = Instant::now();
                while settle_start.elapsed() < Duration::from_millis(300) {
                    while let Ok(more) = raw_rx.try_recv() {
                        extend_unique(&mut batch, more);
                    }
                    if !has_unsettled_item_dirs(&batch) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(15));
                }
                while let Ok(more) = raw_rx.try_recv() {
                    extend_unique(&mut batch, more);
                }

                // 4. Reconcile indexes if enabled, record completion timestamp, drain immediate
                // index-only echo events, and emit TrackerWatchEvent.
                let reconcile_report = if auto_reconcile {
                    let report = storage_for_worker.reconcile_indexes().ok();
                    last_internal_reconcile = Some(Instant::now());
                    while let Ok(more) = raw_rx.try_recv() {
                        if more.iter().all(|p| is_derived_index_file(p)) {
                            continue;
                        }
                        let entry = pending_batch.get_or_insert_with(Vec::new);
                        extend_unique(entry, more);
                    }
                    report
                } else {
                    None
                };

                if tx
                    .send(TrackerWatchEvent {
                        changed_paths: batch,
                        reconcile_report,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });

        Ok((
            Self {
                _watcher: watcher,
                watched_dir,
            },
            rx,
        ))
    }

    /// Returns the `.tracker/` directory path being watched.
    pub fn watched_dir(&self) -> &Path {
        &self.watched_dir
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
