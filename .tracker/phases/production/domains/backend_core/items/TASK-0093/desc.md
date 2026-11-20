## Problem Description
`crates/backend_core/src/mcp/mod.rs` `item_id_from_orrery_step` (STEP 41C) and `crates/backend_core/src/planning/import.rs:98` (`import_plan`'s `by_source_ref` scan) call `read_item_manifest(...)?` on every indexed item, so one malformed or too-new `item.toml` anywhere makes step-context resolution and plan import fail for the whole tracker, while reconcile (STEP 17B) skips and reports such items. Suggested fix: skip items whose manifest fails to parse in both scans (optionally list them in the error/report). Found validating STEP 41C (2026-11-20).

## Reproduction Steps
1. Corrupt one item.toml in a tracker that has a step task.
2. Call MCP update_item_status with only orrery.step_title: the call fails with the parse error instead of resolving the step's item.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
