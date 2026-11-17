## Problem Description
Imported from roadmap step `STEP 24G`: CLI, TUI and 2 MCP CreateItemInput literals use ..CreateItemInput::new; no behaviour change (Gemini; validate-step PASS)
- Step ID: `STEP 24G`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/nebulous_cli/src/commands/mod.rs`
  - `crates/nebulous_cli/src/tui/state.rs`
  - `crates/backend_core/src/mcp/mod.rs`

## Reproduction Steps
1. Inspect `crates/nebulous_cli`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/nebulous_cli`
