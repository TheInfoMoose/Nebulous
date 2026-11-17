## Problem Description
Imported from roadmap step `STEP 24B`: Pure planning::parse_roadmap → PlanStep; approved gap picks G1–G8 (header-name columns, | escapes, strict row/heading errors); 6 inline tests; stricter gate. Validator swapped to the hybrid-workflow skill copy mid-step (user-confirmed) (Gemini; diff + gate PASS)
- Step ID: `STEP 24B`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/planning/mod.rs`
  - `crates/backend_core/src/lib.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
