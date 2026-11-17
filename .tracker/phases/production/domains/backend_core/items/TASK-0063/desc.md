## Problem Description
Imported from roadmap step `STEP 23A`: Kind-aware helpers: ItemType::{ALL, ordinal, from_prefix, label}, ProjectCounters::{get, get_mut, merge_max}, ScopeItems::{iter, count, insert, remove_id}; id_rank uses ordinal (Gemini; validate-step PASS)
- Step ID: `STEP 23A`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/models/item.rs`
  - `crates/backend_core/src/models/project.rs`
  - `crates/backend_core/src/models/scope.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
