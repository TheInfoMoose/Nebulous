## Problem Description
Imported from roadmap step `STEP 24H`: CreateItemInput.source_ref + with_source_ref; create_item writes it in the same item.toml write; bug/feature helpers use ..CreateItemInput::new (Gemini; validate-step PASS)
- Step ID: `STEP 24H`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/workflow/lifecycle.rs`
  - `crates/backend_core/tests/workflow_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
