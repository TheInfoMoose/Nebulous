## Problem Description
Imported from roadmap step `STEP 45B`: Fix move_item_to_phase to remove TASK-* stubs via ScopeItems::remove_id with regression test (BUG-0012 / AUD-05)
- Step ID: `STEP 45B`
- Command: `/goal`
- Roadmap order: 2
- Target files:
  - `crates/backend_core/src/workflow/lifecycle.rs`
  - `crates/backend_core/tests/workflow_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
