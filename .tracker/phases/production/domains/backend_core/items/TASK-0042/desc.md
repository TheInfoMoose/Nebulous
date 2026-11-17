## Problem Description
Imported from roadmap step `STEP 27E`: Fix 25O: build_candidate refuses test ISOs, stores the root-image sha256; VM phases tie through the overlay claim + guest measurement; parse_verdict_overlay (Gemini; validate-step PASS)
- Step ID: `STEP 27E`
- Command: `/boost`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/release/candidate.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
