## Problem Description
Imported from roadmap step `STEP 15B`: Versioned reads/writes for all 4 tiers; per-type item schema; toml preserve_order (Gemini; validate-step PASS; merge defect fixed by 15E)
- Step ID: `STEP 15B`
- Command: `/boost`
- Roadmap order: completed
- Target files:
  - `Cargo.toml`
  - `crates/backend_core/src/storage/mod.rs`
  - `crates/backend_core/src/models/item.rs`
  - `crates/backend_core/tests/storage_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
