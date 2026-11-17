## Problem Description
Imported from roadmap step `STEP 26A`: IdScheme setting ([project] id_scheme, default sequential, not serialized when sequential) + format_hash_id / is_hash_item_id / HASH_ID_HEX_LEN (Gemini; validate-step PASS)
- Step ID: `STEP 26A`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/models/project.rs`
  - `crates/backend_core/src/models/item.rs`
  - `crates/backend_core/src/storage/mod.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
