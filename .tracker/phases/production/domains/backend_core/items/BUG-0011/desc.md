## Problem Description
Source: docs/codebase-audit-report.md (AUD-04) — Validation: CONFIRMED.

Citations:
- crates/backend_core/src/crypto/mod.rs:314-316, 391-435
- crates/backend_core/src/git_sync/mod.rs:272-287, 311-333, 1506-1508

Details:
1. `SecureStorage::store_secret` writes `vault.enc` first, attempts `entry.set_password(secret)`, and swallows all `keyring::Error` variants returning `Ok(())`. `get_secret` reads OS keyring first and only falls back to `vault.enc` on error, so failed keyring updates silently serve stale keyring credentials.
2. `store_pat_in_keyring` / `get_pat_from_keyring` call `keyring::Entry::new` directly, and `resolve_git_auth_with_manager` falls back to `get_pat_from_keyring` even under `KeyPairManager::vault_only(...)`, bypassing `NEBULOUS_DISABLE_KEYRING`.

Suggested Direction:
On keyring update failure in `store_secret`, delete any stale keyring entry (or return an error if deletion fails) and route `git_sync` PAT access exclusively through `SecureStorage`.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
