## Problem Description
Imported from roadmap step `STEP 13A`: with_write_lock: reentrant fd-lock on .tracker/.lock, timeout (Gemini; validate-step PASS)
- Step ID: `STEP 13A`
- Command: `/boost`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/Cargo.toml`
  - `crates/backend_core/src/storage/mod.rs`
  - `crates/backend_core/src/error.rs`
  - `Cargo.lock`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
