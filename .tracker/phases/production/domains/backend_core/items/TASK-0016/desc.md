## Problem Description
Imported from roadmap step `STEP 13B`: Lock every lifecycle + timer mutation; new item dir fails on AlreadyExists (Gemini; validate-step PASS)
- Step ID: `STEP 13B`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/workflow/lifecycle.rs`
  - `crates/backend_core/src/workflow/timer.rs`
  - `crates/backend_core/src/storage/mod.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
