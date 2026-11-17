## Problem Description
Imported from roadmap step `STEP 25H`: extract_iso_inventory: unprivileged ISO + root-image extraction into a self-deleting temp dir; tempfile moved to [dependencies] (Gemini; validate-step PASS; ignored round-trip test also passed)
- Step ID: `STEP 25H`
- Command: `/boost`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/release/iso.rs`
  - `crates/backend_core/src/release/mod.rs`
  - `crates/backend_core/Cargo.toml`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
