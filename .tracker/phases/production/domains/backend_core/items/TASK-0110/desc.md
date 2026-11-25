## Problem Description
Source: docs/codebase-audit-report.md (AUD-13) — Validation: CONFIRMED (minor nuance on SSH_AUTH_SOCK fallback).

Citations:
- crates/backend_core/src/crypto/mod.rs:194-204, 561-566, 596-606
- crates/backend_core/src/crypto/keys.rs:145-172, 256-259, 281
- crates/backend_core/src/git_sync/mod.rs:25-29, 242, 338-342, 1015-1035, 1496-1524

Details:
1. `vault.key` stores a raw 32-byte seed alongside `vault.enc`, and `NEBULOUS_VAULT_MASTER_KEY` uses single-round unsalted SHA-256 (`derive_key_sha256`).
2. `FALLBACK_PAT_STORE` caches plain `String` PATs without zeroizing on removal; `GitAuthMaterial::pat` (`Option<String>`) and `get_pat_string` strip `Zeroizing`.
3. `KeyPairManager::import_ssh_keypair` decrypts passphrase-protected OpenSSH keys and re-serializes them unencrypted into `SecureStorage`, while `git_credential` passes `None` passphrase for memory and `~/.ssh/id_*` keys (falling through to `SSH_AUTH_SOCK` if available).

Suggested Direction:
Use Argon2id/scrypt for `NEBULOUS_VAULT_MASTER_KEY` derivation (and prefer storing `vault.key` in OS keyring when enabled), wrap `FALLBACK_PAT_STORE` and `GitAuthMaterial::pat` in `Zeroizing<String>`, and warn or reject when stripping SSH key passphrases.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
