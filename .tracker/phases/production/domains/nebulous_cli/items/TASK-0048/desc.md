## Problem Description
Imported from roadmap step `STEP 25U`: neb release plumbing (ReleaseCtx, ReleaseRun, render, from_error) + baseline handlers: self-test, status, holds, pins-from-release, extract-golden (refuses a test ISO), verify-variance, record-attempt, check-guardrail (Gemini; POST_GATE PASS; validate-step flagged the plan hash because Phase 11 was added mid-run by the user + another agent, accepted)
- Step ID: `STEP 25U`
- Command: `/boost`
- Roadmap order: completed
- Target files:
  - `crates/nebulous_cli/src/commands/release/mod.rs`
  - `crates/nebulous_cli/src/commands/release/baseline.rs`
  - `crates/nebulous_cli/src/commands/mod.rs`

## Reproduction Steps
1. Inspect `crates/nebulous_cli`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/nebulous_cli`
