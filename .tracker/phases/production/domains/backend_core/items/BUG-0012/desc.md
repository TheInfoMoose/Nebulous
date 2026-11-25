## Problem Description
Source: docs/codebase-audit-report.md (AUD-05) — Validation: CONFIRMED.

Citations:
- crates/backend_core/src/workflow/lifecycle.rs:934-939, 948, 966-983, 1035
- crates/backend_core/src/models/scope.rs:90-94
- crates/backend_core/tests/workflow_integration.rs:1494-1652

Details:
After deleting `old_item_dir`, `remove_item_from_parent_indexes` only retains out `scope_manifest.items.bugs` and `.features`, leaving `.tasks` untouched (unlike `ScopeItems::remove_id`). Moving any `TASK-*` item between phases leaves a dangling stub in the source `scope.toml` and inflated counts in `phase.toml`, causing subsequent lookups to fail with `Io(NotFound)` until `reconcile_indexes` runs.

Suggested Direction:
Replace `lifecycle.rs:973-980` with `scope_manifest.items.remove_id(item_id);` and add a `TASK-*` move regression test in `workflow_integration.rs`.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
