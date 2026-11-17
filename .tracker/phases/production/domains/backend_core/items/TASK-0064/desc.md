## Problem Description
Imported from roadmap step `STEP 23B`: Storage ID parsing, counter max/allocation and stub bucketing use the 23A helpers; reconcile sorts via insert; no behaviour change, tests unmodified (Gemini; validate-step PASS)
- Step ID: `STEP 23B`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/storage/id.rs`
  - `crates/backend_core/src/storage/reconcile.rs`
  - `crates/backend_core/src/storage/mod.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
