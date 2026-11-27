## Problem Description
Imported from roadmap step `STEP 46B`: Add mode-aware atomic_write_bytes_with_mode (0o600), propagate ensure_secure_dir errors, reject implicit temp_dir() vault fallback, and harden Windows replace/ACL handling (BUG-0009 / AUD-02)
- Step ID: `STEP 46B`
- Command: `/boost`
- Roadmap order: 2
- Target files:
  - `crates/backend_core/src/storage/atomic.rs`
  - `crates/backend_core/src/storage/mod.rs`
  - `crates/backend_core/src/crypto/mod.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
