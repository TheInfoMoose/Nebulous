## Problem Description
Imported from roadmap step `STEP 25S`: Status summary (read_status, status_json, render_status_text, §4.8 keys) + built-in self-test runner (14 §2.9 cases, embedded package-version table, panic-safe) (Gemini; validate-step PASS)
- Step ID: `STEP 25S`
- Command: `/boost`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/release/status.rs`
  - `crates/backend_core/src/release/selftest.rs`
  - `crates/backend_core/src/release/mod.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
