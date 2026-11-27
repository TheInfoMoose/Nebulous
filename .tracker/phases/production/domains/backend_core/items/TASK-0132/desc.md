## Problem Description
Imported from roadmap step `STEP 46C`: Fix SecureStorage keyring/vault split-brain and route git_sync PAT functions exclusively through SecureStorage (BUG-0011 / AUD-04)
- Step ID: `STEP 46C`
- Command: `/boost`
- Roadmap order: 3
- Target files:
  - `crates/backend_core/src/crypto/mod.rs`
  - `crates/backend_core/src/git_sync/mod.rs`
  - `crates/backend_core/tests/credential_hygiene.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
