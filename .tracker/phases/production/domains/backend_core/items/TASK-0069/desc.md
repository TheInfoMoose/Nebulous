## Problem Description
Imported from roadmap step `STEP 23F`: ItemType::Task (TASK-XXXX, schema v2, MAX 2), task counter (skipped when 0), tasks scope list; is_hash_item_id accepts TASK; storage_integration 6 approved line edits (Gemini; validate-step PASS)
- Step ID: `STEP 23F`
- Command: `/boost`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/models/item.rs`
  - `crates/backend_core/src/models/project.rs`
  - `crates/backend_core/src/models/scope.rs`
  - `crates/backend_core/tests/storage_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
