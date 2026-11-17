## Problem Description
Imported from roadmap step `STEP 26B`: allocate_next_id hash branch + allocate_hash_id (OsRng, redraw all-digit/taken, 16-attempt cap; counters untouched) (Gemini; validate-step PASS)
- Step ID: `STEP 26B`
- Command: `/boost`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/storage/id.rs`
  - `crates/backend_core/tests/storage_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
