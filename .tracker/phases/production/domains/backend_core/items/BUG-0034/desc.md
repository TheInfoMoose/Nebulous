## Problem Description
Phase 27 review (Finding 3): crates/nebulous_cli/src/commands/mod.rs:2687-2733 (ComponentCommand::Pin, exact CLI twin of MCP tool_pin_stable_release) and crates/backend_core/src/mcp/mod.rs:1630-1671,1755-1806 (tool_upsert_component_manifest, tool_record_rolling_update) perform unlocked read_component_manifest -> mutate -> write_component_manifest / pin_component_release cycles, allowing lost updates under concurrency.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
