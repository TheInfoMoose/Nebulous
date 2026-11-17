## Problem Description
Imported from roadmap step `STEP 25B`: Package version ordering + reference-verified case table (Gemini; validate-step PASS; 75 cases re-checked against the reference tool)
- Step ID: `STEP 25B`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/release/debver.rs`
  - `crates/backend_core/src/release/testdata/debver_cases.txt`
  - `crates/backend_core/src/release/mod.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
