## Problem Description
Imported from roadmap step `STEP 47F`: Move neb done -c close+commit+rollback into backend_core under one with_write_lock and surface rollback failures with a post-close commit-failure regression test (BUG-0015 / AUD-08)
- Step ID: `STEP 47F`
- Command: `/boost`
- Roadmap order: 6
- Target files:
  - `crates/backend_core/src/workflow/lifecycle.rs`
  - `crates/nebulous_cli/src/commands/mod.rs`
  - `crates/nebulous_cli/tests/done_commit_safety.rs`

## Reproduction Steps
1. Inspect `crates/nebulous_cli`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/nebulous_cli`
