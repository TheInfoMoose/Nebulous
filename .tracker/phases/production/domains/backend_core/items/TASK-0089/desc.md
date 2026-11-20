## Problem Description
Imported from roadmap step `STEP 41A`: plan import: Completed rows never create items; closed items are never retitled (Advances BUG-0003)
Advances: BUG-0003
- Step ID: `STEP 41A`
- Command: `/goal`
- Roadmap order: 1
- Target files:
  - `crates/backend_core/src/planning/import.rs`
  - `crates/backend_core/tests/plan_import.rs`
  - `crates/nebulous_cli/tests/cli_integration.rs`
  - `docs/USAGE.md`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
