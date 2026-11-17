## Problem Description
Imported from roadmap step `STEP 29H`: CLI strings and test data neutral via fixed sed script (sha256-pinned); check-neutral allowlist empty (Gemini; validate-step PASS)
- Step ID: `STEP 29H`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/nebulous_cli/src/lib.rs`
  - `crates/nebulous_cli/tests/cli_integration.rs`
  - `scripts/check-neutral.sh`

## Reproduction Steps
1. Inspect `crates/nebulous_cli`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/nebulous_cli`
