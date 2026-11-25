## Problem Description
Source: docs/codebase-audit-report.md (AUD-27) — Validation: CONFIRMED (minor nuance: git status walk runs when cwd does not match a scope prefix).

Citations:
- crates/backend_core/src/workflow/assist.rs:287-301, 718-723, 727-748, 861-912
- crates/backend_core/src/models/timer.rs:54-59
- crates/backend_core/src/workflow/timer.rs:85-101, 193-195
- crates/backend_core/src/workflow/lifecycle.rs:278-279, 364-366

Details:
1. `predict_defaults` loads `TrackerIndex` twice and `infer_scope_from_git_dirty_files` runs `repo.statuses` with `recurse_untracked_dirs(true)` when cwd is outside a scope prefix.
2. `count_phase_items` can emit `AdvanceStage` on an empty phase if stale `phase.toml` summary counts remain; `suggest_next_stage` falls back to `"production"` after the last custom phase.
3. `ActiveTimer::elapsed_minutes_at` and `timer_status_at` cast `(seconds / 60) as u32` without saturation.
4. `start_work_item` calls `switch_or_start_timer` twice (once via `update_item_status` with `None`, then again with `note`).

Suggested Direction:
Share the loaded `TrackerIndex` in `predict_defaults`, saturate `u32::try_from(...).unwrap_or(u32::MAX)` on elapsed minutes, eliminate the duplicate `switch_or_start_timer` call in `start_work_item`, and return `None` after the final custom phase.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
