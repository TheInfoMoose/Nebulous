## Problem Description
Imported from roadmap step `STEP 45F`: Add AtomicBool shutdown signal to TrackerWatcher::drop (including contended lock waits) and skip symlinks in hash_index_files (BUG-0023 / AUD-20)
- Step ID: `STEP 45F`
- Command: `/goal`
- Roadmap order: 6
- Target files:
  - `crates/backend_core/src/watcher/mod.rs`
  - `crates/backend_core/tests/watcher_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
