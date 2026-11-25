## Problem Description
Source: docs/codebase-audit-report.md (AUD-17) — Validation: CONFIRMED (nuance on tool_pin_stable_release lock split).

Citations:
- crates/backend_core/src/storage/mod.rs:274, 747, 789, 819, 853, 872, 884, 898-903, 917-930, 933, 939, 1053
- crates/backend_core/src/models/registry.rs:19-23, 382-479, 550-570
- crates/backend_core/src/mcp/mod.rs:1786, 1804, 1952, 1990-2041
- crates/backend_core/src/git_sync/mod.rs:1888-1890

Details:
1. Public `TrackerStorage` mutators (`write_*`, `init`, `resolve_phase_and_scope`) do not acquire `with_write_lock`; unlocked calls exist in `mcp/mod.rs` (`1804`, `1952`) and `git_sync/mod.rs:1888-1890`, while `tool_pin_stable_release` splits read/write/pin across separate lock acquisitions.
2. `read_active_timer` renames corrupt `.timer.toml` to `.timer.toml.bak` without `with_write_lock`.
3. `models/registry.rs` uses only an in-process `Mutex` (`REGISTRY_LOCK`) with no cross-process `fd_lock` on `projects.toml`, and `load_registry_unlocked` discards prune save errors (`let _ = ...`).

Suggested Direction:
Acquire `with_write_lock` inside public `TrackerStorage` mutators and timer quarantine (keeping unlocked helpers `pub(crate)`), and add a cross-process `projects.lock` `fd_lock` in `models/registry.rs`.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
