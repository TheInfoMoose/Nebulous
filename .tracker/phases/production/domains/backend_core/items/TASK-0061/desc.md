## Problem Description
Imported from roadmap step `STEP 26F`: git_pull_with_report / git_sync_with_report (+ GitPullOutcome / GitSyncOutcome; old signatures kept as wrappers); neb git pull / neb sync warn + JSON duplicate_ids; shared duplicate_id_warnings (Gemini; validate-step PASS)
- Step ID: `STEP 26F`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/git_sync/mod.rs`
  - `crates/nebulous_cli/src/commands/mod.rs`
  - `crates/backend_core/tests/workflow_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
