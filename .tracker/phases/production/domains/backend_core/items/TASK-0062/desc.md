## Problem Description
Imported from roadmap step `STEP 26G`: MCP git_pull / git_sync results gain duplicate_ids (via 26F *_with_report; [] for commit_only); descriptions get one sentence (Gemini; validate-step PASS)
- Step ID: `STEP 26G`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/mcp/mod.rs`
  - `crates/backend_core/tests/workflow_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
