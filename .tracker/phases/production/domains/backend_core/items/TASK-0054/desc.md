## Problem Description
Imported from roadmap step `STEP 29G`: Ingestion kernel heuristic drops vendor kernel names; storage test data neutral via fixed sed script (sha256-pinned) (Gemini; validate-step PASS)
- Step ID: `STEP 29G`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/ingestion/mod.rs`
  - `crates/backend_core/tests/storage_integration.rs`
  - `scripts/check-neutral.sh`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
