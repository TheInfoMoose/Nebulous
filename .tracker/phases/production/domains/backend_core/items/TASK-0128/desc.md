## Problem Description
Imported from roadmap step `STEP 45E`: Lock public TrackerStorage mutators, read_active_timer quarantine, tool_pin_stable_release, and projects.toml registry (projects.lock) (BUG-0020 / AUD-17)
- Step ID: `STEP 45E`
- Command: `/boost`
- Roadmap order: 5
- Target files:
  - `crates/backend_core/src/storage/mod.rs`
  - `crates/backend_core/src/models/registry.rs`
  - `crates/backend_core/src/mcp/mod.rs`
  - `crates/backend_core/tests/storage_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
