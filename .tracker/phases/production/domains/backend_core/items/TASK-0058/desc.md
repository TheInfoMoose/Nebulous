## Problem Description
Imported from roadmap step `STEP 26C`: ItemStub.created_at (default empty, not serialized when empty) + chronological_cmp (instant rank, then same-type number, then ID); id tie-break not total with hash + 5-digit IDs, fixed in 26D (Gemini; validate-step PASS)
- Step ID: `STEP 26C`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/models/scope.rs`
  - `crates/backend_core/src/models/item.rs`
  - `crates/backend_core/tests/storage_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
