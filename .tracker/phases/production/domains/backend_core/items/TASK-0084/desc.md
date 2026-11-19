## Problem Description
Audit A5 (Phase 21): `crates/backend_core/src/mcp/mod.rs:24` — rmcp 3.9.0 asks for `2026-07-28`; Nebulous answers `2025-06-18` and rmcp accepts it, but Nebulous skips `2025-11-25` (rmcp's `LATEST_WITH_INITIALIZE`), the version a future initialize-only client would ask for — add `2025-11-25` to `SUPPORTED_MCP_PROTOCOL_VERSIONS`.
Migrated from the plan file's Parked list (2026-11-18).

## Reproduction Steps
n/a (parked finding; see Problem Description)

## Fix Log & Agent Notes
