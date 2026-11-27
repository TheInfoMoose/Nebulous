## Problem Description
Imported from roadmap step `STEP 46F`: Zeroize GitAuthMaterial::pat (Option<Zeroizing<String>>) and guard SSH passphrase stripping in KeyPairManager (TASK-0110 non-breaking slice / AUD-13)
- Step ID: `STEP 46F`
- Command: `/goal`
- Roadmap order: 6
- Target files:
  - `crates/backend_core/src/git_sync/mod.rs`
  - `crates/backend_core/src/crypto/keys.rs`
  - `crates/backend_core/tests/credential_hygiene.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
