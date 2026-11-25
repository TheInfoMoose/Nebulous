## Problem Description
Source: docs/codebase-audit-report.md (AUD-08) — Validation: CONFIRMED.

Citations:
- crates/nebulous_cli/src/commands/mod.rs:1307-1358
- crates/backend_core/src/workflow/lifecycle.rs:495-553
- crates/nebulous_cli/tests/done_commit_safety.rs:130-166

Details:
`handle_close` invokes `validate_documented_commit`, `load_item_detail_by_id`, `read_active_timer`, `close_item_reporting`, unlocked `write_item_manifest` (popping `previous_fixes`), and `git_commit_documented` without holding an outer `.tracker/.lock`. On `git_commit_documented` failure, lines `1351-1355` discard all rollback errors via `let _ = ...`. `done_commit_safety.rs` only tests pre-close validation rejection.

Suggested Direction:
Move the `done -c` close+commit+rollback flow into `backend_core` inside a single `with_write_lock` scope, surface rollback failures explicitly, and add a post-close commit-failure regression test.

## Reproduction Steps
1. Inspect `crates/nebulous_cli`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/nebulous_cli`
