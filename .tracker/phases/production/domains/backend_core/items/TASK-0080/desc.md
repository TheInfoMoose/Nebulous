## Problem Description
Imported from roadmap step `STEP 24C`: planning::import_plan upserts tasks by source_ref (atomic via 24H); D1–D8 as approved; dry run takes no lock; Gemini-added input pre-check kept (user, 2026-11-14) (Gemini; validate-step PASS)
- Step ID: `STEP 24C`
- Command: `/boost`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/planning/import.rs`
  - `crates/backend_core/src/planning/mod.rs`
  - `crates/backend_core/src/workflow/assist.rs`
  - `crates/backend_core/tests/plan_import.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
