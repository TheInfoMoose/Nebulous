## Problem Description
Audit A4 (Phase 21): `crates/backend_core/src/mcp/mod.rs:933` — `plan_import` is the only tool with `#[serde(deny_unknown_fields)]`, so it rejects Orrery's reserved `orrery` key ("unknown field `orrery`") where every other tool ignores it — drop `deny_unknown_fields` there (or document the strictness).
Migrated from the plan file's Parked list (2026-11-18).

## Reproduction Steps
1. Call MCP `plan_import` with an extra `orrery` object in args (as Orrery hooks do).
2. Observe: error "unknown field `orrery`".

## Fix Log & Agent Notes
