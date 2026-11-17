## Problem Description
Imported from roadmap step `STEP 24D`: neb plan import <PATH> [--phase] [--write] (dry run default); CREATE/RETITLE/CLOSE lines, UNCHANGED count, footer; --json ImportReport; G1–G6 + stricter gate approved (Gemini; validate-step PASS)
- Step ID: `STEP 24D`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/nebulous_cli/src/args.rs`
  - `crates/nebulous_cli/src/commands/mod.rs`
  - `crates/nebulous_cli/tests/cli_integration.rs`

## Reproduction Steps
1. Inspect `crates/nebulous_cli`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/nebulous_cli`
