## Problem Description
Imported from roadmap step `STEP 47B`: Zeroize GitAuthMaterial::pat (Option<Zeroizing<String>>) and guard SSH passphrase stripping in KeyPairManager (re-split of STEP 46F, TASK-0135, TASK-0136, TASK-0110)
- Step ID: `STEP 47B`
- Command: `/goal`
- Roadmap order: 2
- Target files:
  - `crates/backend_core/src/git_sync/mod.rs`
  - `crates/backend_core/src/crypto/keys.rs`
  - `crates/backend_core/tests/credential_hygiene.rs`
  - `crates/backend_core/tests/workflow_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
