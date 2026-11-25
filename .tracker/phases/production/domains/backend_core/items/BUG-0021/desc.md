## Problem Description
Source: docs/codebase-audit-report.md (AUD-18) — Validation: CONFIRMED.

Citations:
- CHANGELOG.md:18
- crates/backend_core/src/mcp/mod.rs:220, 829, 901-937, 1106-1109, 1214-1216, 1323-1343, 1459-1498, 1645-1650, 1674, 1778-1872, 1992-2047, 2164-2165, 2401-2406, 2550-2605

Details:
1. `git_fetch` defaults `dry_run` to `false`, contradicting `CHANGELOG.md:18`.
2. Only `tool_plan_import` (`937`) sets `#[serde(deny_unknown_fields)]`; the other 23 tools silently ignore misspelled/unknown parameters.
3. `read_next_jsonrpc_message` exits the header loop on EOF without requiring the blank terminator line and hits `read_exact`, crashing `serve_io` with `UnexpectedEof`.
4. `tool_upsert_component_manifest` and `tool_record_rolling_update` silently accept/default invalid enum values; `list_items`, `list_crate_boundaries`, and `select_best_active_item` skip `resolve_phase_alias`; `tool_record_rolling_update` and `tool_pin_stable_release` perform non-atomic multi-step writes; `tool_create_item` and `tool_upsert_component_manifest` leak absolute host paths; `tool_switch_project` mutates machine-wide `projects.toml`.

Suggested Direction:
Reconcile `git_fetch` `dry_run` with `CHANGELOG.md`, add `deny_unknown_fields` and strict enum/phase-alias validation across all MCP tools, fix EOF handling in `read_next_jsonrpc_message`, return workspace-relative paths, and scope `switch_project` to the session by default.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
