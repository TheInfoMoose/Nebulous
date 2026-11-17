## Problem Description
Imported from roadmap step `STEP 25Q`: Bisect: bisect.toml model + I/O, start_bisect, next_round (test set + hold file), record_round (literal halving, culprit, interaction, cap); ledger entries returned (Gemini; validate-step PASS)
- Step ID: `STEP 25Q`
- Command: `/boost`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/release/bisect.rs`
  - `crates/backend_core/src/release/mod.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
