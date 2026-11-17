## Problem Description
Imported from roadmap step `STEP 26D`: Total-order IdRank tie-break in chronological_cmp; reconcile + lifecycle sort stubs with it, own comparators removed (Gemini; validate-step PASS)
- Step ID: `STEP 26D`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/models/scope.rs`
  - `crates/backend_core/src/storage/reconcile.rs`
  - `crates/backend_core/src/workflow/lifecycle.rs`
  - `crates/backend_core/tests/storage_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
