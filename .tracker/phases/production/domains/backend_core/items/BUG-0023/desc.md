## Problem Description
Source: docs/codebase-audit-report.md (AUD-20) — Validation: CONFIRMED (minor nuance on canonicalization).

Citations:
- crates/backend_core/src/watcher/mod.rs:137-149, 170-174, 224-231, 332-375
- crates/backend_core/tests/watcher_integration.rs:26-35

Details:
1. `TrackerWatcher::drop` joins the worker thread unconditionally with no shutdown `AtomicBool` checked during the 300ms settlement loop or `reconcile_indexes()` (up to 30s lock timeout).
2. `hash_index_files` recurses via `path.is_dir()`, following directory symlinks inside `.tracker/` without cycle/visited guards (and symlinked inner paths can also miss `is_own_index_echo`).
3. `watcher_integration.rs` only tests dropping an idle watcher.

Suggested Direction:
Add an `Arc<AtomicBool>` shutdown flag checked across settlement/reconcile loops, skip symlinks via `entry.file_type()` in `hash_index_files`, and add a lock-contention drop test.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
