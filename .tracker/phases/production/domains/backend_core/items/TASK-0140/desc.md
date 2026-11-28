## Problem Description
Imported from roadmap step `STEP 47D`: Propagate merge-reconcile errors, block git_push on duplicate_ids, and fix git_commit_documented pre-amend window, reachable fix SHA, and .git/index stat metadata (BUG-0014 / AUD-07)
- Step ID: `STEP 47D`
- Command: `/boost`
- Roadmap order: 4
- Target files:
  - `crates/backend_core/src/git_sync/mod.rs`
  - `crates/backend_core/tests/git_safety.rs`
  - `crates/backend_core/tests/workflow_integration.rs`
  - `crates/nebulous_cli/tests/done_commit_safety.rs`
  - `crates/nebulous_cli/tests/cli_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
