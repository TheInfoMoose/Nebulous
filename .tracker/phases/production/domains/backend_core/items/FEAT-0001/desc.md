## Problem Description
`crates/backend_core/src/mcp/mod.rs:635` — `close_item_with_fix` / `update_item_status` require `item_id`, but Orrery hook `args` are static per project, and the per-step data arrives only in the reserved `orrery` object (`step_title` like `"3F: …"`, `step_number`), which neb ignores. So an `on_step_start` / `on_step_pass` hook can't target the step's item. Done when: with `item_id` absent, both tools resolve the single item whose `source_ref` = `STEP <ID>` parsed from `orrery.step_title` (error if 0 or >1 match), and the orrery-setup skill can wire `on_step_start` → in_progress and `on_step_pass` → close.
Migrated from the plan file's Parked list (2026-11-18).

## Reproduction Steps
n/a (parked finding; see Problem Description)

## Fix Log & Agent Notes
