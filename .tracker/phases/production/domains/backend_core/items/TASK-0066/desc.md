## Problem Description
Imported from roadmap step `STEP 26H`: cmp_item_ids (IdRank total order) in scope.rs; assist's non-total compare_item_ids removed, 4 What's Next tie-breaks use it; exhaustive totality test (Gemini; validate-step PASS)
- Step ID: `STEP 26H`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/models/scope.rs`
  - `crates/backend_core/src/workflow/assist.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
