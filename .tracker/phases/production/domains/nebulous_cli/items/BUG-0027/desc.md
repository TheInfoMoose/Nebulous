## Problem Description
Source: docs/codebase-audit-report.md (AUD-24) — Validation: CONFIRMED.

Citations:
- crates/nebulous_cli/src/commands/mod.rs:593-612, 1016, 1066-1072, 1196-1199, 1820-1844, 1891-1899, 2802-2808

Details:
1. `handle_sync` discards `sync_outcome.sha` and returns pre-sync `outcome.commit_sha`.
2. `handle_start` calls `std::env::current_dir()` instead of `resolve_context_cwd(&storage, cli.root.as_deref())` and omits `warn_if_capped(...)`.
3. `handle_move` interprets single-arg `neb move BUG-0001` as moving the active item to phase `"BUG-0001"`.
4. `handle_sync_index` omits `report.counters.task` in human output.
5. `handle_init` silently swallows `ingest_project_directory` and `register_project` errors; `ComponentCommand::Rollback` silently ignores `record_regression` errors even when `--item` was explicitly given.

Suggested Direction:
Return `sync_outcome.sha.or(Some(outcome.commit_sha))`, wire `resolve_context_cwd` and `warn_if_capped` in `handle_start`, print `task={}` in `handle_sync_index`, guard single-arg `handle_move` against item-ID tokens, and warn/propagate errors in `handle_init` and explicit `--item` `Rollback`.

## Reproduction Steps
1. Inspect `crates/nebulous_cli`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/nebulous_cli`
