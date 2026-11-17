## Problem Description
Imported from roadmap step `STEP 23K`: ItemMetadata::new (default status/severity, both timestamps); create_item uses ..ItemMetadata::new; 2 inline tests; approved stricter gate (Gemini; validate-step PASS)
- Step ID: `STEP 23K`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/models/item.rs`
  - `crates/backend_core/src/workflow/lifecycle.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
