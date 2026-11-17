## Problem Description
Imported from roadmap step `STEP 25P`: Promote: prepare_promote (production ISO, identity, phases, bisect, variance; golden, record, cleared holds) + commit_promote (record → golden → remove candidate, rollback) (Gemini; validate-step PASS)
- Step ID: `STEP 25P`
- Command: `/boost`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/release/promote.rs`
  - `crates/backend_core/src/release/mod.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
