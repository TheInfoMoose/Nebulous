## Problem Description
Imported from roadmap step `STEP 23L`: The five integration-test ItemMetadata literals use ..ItemMetadata::new, keeping only non-default fields; values unchanged (Gemini; validate-step PASS)
- Step ID: `STEP 23L`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/tests/storage_integration.rs`
  - `crates/backend_core/tests/workflow_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
