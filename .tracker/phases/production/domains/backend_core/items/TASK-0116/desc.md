## Problem Description
Source: docs/codebase-audit-report.md (AUD-33) — Validation: CONFIRMED.

Citations:
- crates/backend_core/src/lib.rs:13-21, 29-887
- crates/backend_core/tests/storage_integration.rs:14-40, 1458, 1478
- crates/backend_core/tests/path_containment.rs:14-37; storage/atomic.rs:194-203
- crates/backend_core/tests/workflow_integration.rs:1494-1652; tests/watcher_integration.rs:26-35

Details:
`backend_core/src/lib.rs` exports 8 modules via `pub use <mod>::*;` and mixes 859 lines of inline unit tests. `storage_integration.rs:1458, 1478` mutates `NEBULOUS_CONFIG_DIR` via `std::env::set_var`/`remove_var` in parallel tests. Three hand-rolled temp-dir helpers duplicate `tempfile::TempDir`. Regression tests are missing for `TASK-*` phase moves, symlinked `.tracker/` roots, `TrackerWatcher::drop` under `.lock` contention, and malformed `phase.toml`/`scope.toml` in `reconcile_indexes`.

Suggested Direction:
Replace wildcard re-exports with explicit exports, pass config dirs via `*_in(&cfg_dir)` helpers in `storage_integration.rs`, standardize on `tempfile::TempDir`, and add the four missing regression tests.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
