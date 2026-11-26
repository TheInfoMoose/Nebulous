## Problem Description
Imported from roadmap step `STEP 45C`: Canonicalize HELD_WRITE_LOCKS and PROJECT_CACHE keys to prevent same-thread 30s self-deadlock (BUG-0013 / AUD-06)
- Step ID: `STEP 45C`
- Command: `/goal`
- Roadmap order: 3
- Target files:
  - `crates/backend_core/src/storage/mod.rs`
  - `crates/backend_core/tests/storage_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
