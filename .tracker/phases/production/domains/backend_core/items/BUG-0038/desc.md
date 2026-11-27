## Problem Description
Phase 28 review finding (test defect, origin: plan/coder, STEP 46B). In crates/backend_core/src/crypto/mod.rs:864-880, both error assertions pass a regular file (or child under a regular file), which fails inside fs::create_dir_all before fs::set_permissions is ever reached. Reverting fs::set_permissions error propagation would not fail this test.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
