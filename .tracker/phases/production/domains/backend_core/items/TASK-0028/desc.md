## Problem Description
Imported from roadmap step `STEP 25F`: Extracted-root helpers: confined symlink resolution, sha256, workspace crate versions, custom binary scan (Gemini; validate-step PASS)
- Step ID: `STEP 25F`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/release/rootfs.rs`
  - `crates/backend_core/src/release/mod.rs`
  - `crates/backend_core/Cargo.toml`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
