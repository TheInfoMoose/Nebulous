## Problem Description
Source: docs/codebase-audit-report.md (AUD-06) — Validation: CONFIRMED.

Citations:
- crates/backend_core/src/storage/mod.rs:32-40, 189-206, 221-244, 490-497, 512-515, 553, 720-766

Details:
`TrackerStorage::new` and `from_tracker_dir` preserve un-canonicalized caller paths whereas `discover` canonicalizes `start_dir`. `lock_path()` returns un-canonicalized `self.tracker_dir.join(".lock")`, which is used directly as the key in `HELD_WRITE_LOCKS` and `PROJECT_CACHE`. Nested same-thread locking across two differently-spelled `TrackerStorage` instances misses `HELD_WRITE_LOCKS` and spins for 30s (`DEFAULT_LOCK_TIMEOUT`) before returning `TrackerError::LockTimeout`.

Suggested Direction:
Derive a canonicalized lock/cache key via `canonicalize_existing_prefix(&self.tracker_dir)` in one helper shared by `holds_write_lock`, `with_write_lock_timeout`, `read_project`, and `write_project`.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
