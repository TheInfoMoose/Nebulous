## Problem Description
crates/backend_core/src/crypto/keys.rs:197-209,394-402 — KeyPairManager::import_ssh_keypair_with_policy guards passphrase stripping, but KeyPairManager::import_ssh_key_file and the module-level import_ssh_keypair helper unconditionally delegate to allow_strip_passphrase=true with no policy-guarded variant.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
