## Problem Description
Imported from roadmap step `STEP 18`: Idempotent timer stop, orphan recovery, idle cap
- Step ID: `STEP 18`
- Command: `/goal`
- Roadmap order: 5
- Target files:
  - `crates/backend_core/src/workflow/timer.rs`
  - `crates/backend_core/src/models/timer.rs`
  - `crates/backend_core/src/storage/mod.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
