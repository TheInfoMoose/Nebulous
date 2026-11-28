## Problem Description
Imported from roadmap step `STEP 47E`: Add opt-in allow_empty_ok (default false) to DocumentedCommitInput, git_commit_documented, and MCP git_commit_documented for idempotent Orrery reruns (BUG-0004)
- Step ID: `STEP 47E`
- Command: `/goal`
- Roadmap order: 5
- Target files:
  - `crates/backend_core/src/git_sync/mod.rs`
  - `crates/backend_core/src/mcp/mod.rs`
  - `crates/nebulous_cli/src/commands/mod.rs`
  - `crates/backend_core/tests/workflow_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
