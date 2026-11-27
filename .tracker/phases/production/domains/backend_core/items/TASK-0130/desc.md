## Problem Description
Imported from roadmap step `STEP 46A`: Fix read-only registry lookups (projects.lock shared/read-only open, per-path lock map, no prune write on read) (BUG-0033)
- Step ID: `STEP 46A`
- Command: `/goal`
- Roadmap order: 1
- Target files:
  - `crates/backend_core/src/models/registry.rs`
  - `crates/backend_core/tests/storage_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
