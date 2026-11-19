## Problem Description
`crates/backend_core/src/workflow/assist.rs:526` — the stagnant check includes `pending` items, so parked items (the neb-tracking convention for deferred findings) surface in `neb next` as "Stagnant item" after `stagnant_item_threshold_days` — either skip `pending` there or show them as "Parked" with a phase-end hint. Found while writing the neb-tracking skill.
Migrated from the plan file's Parked list (2026-11-18).

## Reproduction Steps
n/a (parked finding; see Problem Description)

## Fix Log & Agent Notes
