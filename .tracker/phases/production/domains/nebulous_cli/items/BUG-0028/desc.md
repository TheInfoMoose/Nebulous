## Problem Description
Source: docs/codebase-audit-report.md (AUD-25) — Validation: CONFIRMED (note overlap with BUG-0007 on Description Enter/multiline).

Citations:
- crates/nebulous_cli/src/tui/state.rs:412-416, 634, 646, 789-797, 885-892, 903-905, 928-939, 970-981, 1106-1127, 1143, 1173, 1197-1203, 1234-1246, 1277, 1284-1291
- crates/nebulous_cli/src/tui/render.rs:172-186, 607, 673-706, 924-937
- crates/nebulous_cli/src/tui/mod.rs:87-96

Details:
1. `CANONICAL_PHASES` is hardcoded across stage headers, `1`/`2`/`3` keybindings, `CreateModal` cycling, and `MoveModal`, making custom phases inaccessible in the TUI.
2. `rebuild_view()` reads `item.toml` per item on every `/` filter keystroke, and moving the Scopes cursor calls full `self.refresh()` (reloading `TrackerIndex` + running `git2` status).
3. `submit_active_modal` discards `create_phase`/`create_scope` errors; `action_start_work` maps every error to `"No open or selected item to start."`; `action_copy_context` ignores errors.
4. `action_jump_whats_next` fails silently when a filter hides the item; `tui/mod.rs:87-91` leaks a stacked global panic hook; `state.rs:1277` uses `.unwrap()` in production code.
5. (Note: `CreateModal` Enter-submits-in-Description / single-line Description rendering is also tracked in BUG-0007.)

Suggested Direction:
Cache `recurrence_count`, call `rebuild_view()` on scope cursor movement, support dynamic `project.phases.registry` phases, surface actual errors in `status_message`, clear filter or notify on jump, replace `.unwrap()` at `state.rs:1277`, and restore the panic hook on `run_tui` exit.

## Reproduction Steps
1. Inspect `crates/nebulous_cli`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/nebulous_cli`
