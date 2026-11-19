## Problem Description
`neb plan import` (dry run 2026-11-18) wants to RETITLE TASK-0035/0036/0041/0042/0046/0054 back to their roadmap row text — those titles were reworded on purpose in STEP 33C to pass `scripts/check-neutral.sh`, so `--write` would undo that and fail the neutrality check. It also wants to CREATE ~70 items for long-completed steps. Suggested fix: only retitle items whose title still matches the last imported text (or add `--no-retitle`), and skip creating items for Completed rows unless `--backfill` is passed.
Migrated from the plan file's Parked list (2026-11-18).

## Reproduction Steps
1. `neb plan import STEPPED_IMPLEMENTATION_PLAN.md` in Nebulous.
2. Observe: RETITLE lines for the six neutral-reworded tasks and CREATE lines for completed steps.

## Fix Log & Agent Notes
