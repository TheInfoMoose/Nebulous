## Problem Description
Imported from roadmap step `STEP 23J`: MCP create_item / list_items text names task; list_crate_boundaries gains tasks_count; TASK-0001 in ID examples and commit-policy error; approved MCP test and stricter gate (Gemini; validate-step PASS)
- Step ID: `STEP 23J`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/mcp/mod.rs`
  - `crates/backend_core/src/git_sync/mod.rs`
  - `crates/backend_core/tests/workflow_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
