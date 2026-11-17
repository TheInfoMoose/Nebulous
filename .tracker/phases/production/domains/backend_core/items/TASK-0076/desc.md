## Problem Description
Imported from roadmap step `STEP 24A`: ItemMetadata.source_ref: Option<String> (skipped when None), new() sets None; 2 inline TOML tests; approved stricter gate (Gemini; validate-step PASS)
- Step ID: `STEP 24A`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/models/item.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
