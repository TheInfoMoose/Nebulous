## Problem Description
Imported from roadmap step `STEP 47C`: Fix Phase 28 test defects: seed global vault PAT in vault_only_manager_does_not_fall_back_to_global_keyring_store and test set_permissions failure in ensure_secure_dir_propagates_permission_error (BUG-0037, BUG-0038)
- Step ID: `STEP 47C`
- Command: `/goal`
- Roadmap order: 3
- Target files:
  - `crates/backend_core/tests/credential_hygiene.rs`
  - `crates/backend_core/src/crypto/mod.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
