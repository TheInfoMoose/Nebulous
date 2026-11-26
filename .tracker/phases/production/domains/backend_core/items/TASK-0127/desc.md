## Problem Description
Imported from roadmap step `STEP 45D`: Reject symlinked .tracker/ roots in discover and guard_tracker_path with Unix O_NOFOLLOW hardening and containment tests (BUG-0010 / AUD-03)
- Step ID: `STEP 45D`
- Command: `/boost`
- Roadmap order: 4
- Target files:
  - `crates/backend_core/Cargo.toml`
  - `Cargo.lock`
  - `crates/backend_core/src/storage/mod.rs`
  - `crates/backend_core/tests/path_containment.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
