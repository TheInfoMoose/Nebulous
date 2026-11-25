## Problem Description
Source: docs/codebase-audit-report.md (AUD-26) — Validation: CONFIRMED.

Citations:
- crates/nebulous_cli/src/tui/render.rs:403-409, 792-800

Details:
1. `render_items_pane` hardcodes `Constraint::Length(9)` for the `ID` column, silently clipping 10–11 char hash IDs (`BUG-7F3A2C`, `FEAT-7F3A2C`, `TASK-7F3A2C`).
2. `render_close_modal` slices unedited default summaries via `.chars().take(max_sum_chars)` instead of terminal display width, allowing double-width Unicode titles to overflow.

Suggested Direction:
Size the `ID` column dynamically from displayed IDs (at least 11 when hash IDs are present) and truncate unedited `CloseModal` previews using Unicode display width (`unicode-width`).

## Reproduction Steps
1. Inspect `crates/nebulous_cli`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/nebulous_cli`
