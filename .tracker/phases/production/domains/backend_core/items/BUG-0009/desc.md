## Problem Description
Source: docs/codebase-audit-report.md (AUD-02) — Validation: CONFIRMED.

Citations:
- crates/backend_core/src/storage/atomic.rs:17-20, 51-68, 108-146, 164-168
- crates/backend_core/src/crypto/mod.rs:148-191, 544-545, 603-604, 615-652, 656-675

Details:
1. `atomic_write_bytes` creates temp files under process umask before renaming, so newly created `vault.key` and `vault.enc` are written and renamed before `set_secure_file_permissions` (`0o600` / Windows `icacls`).
2. `ensure_secure_dir` ignores permission/ACL failures via `let _ = ...` and skips Windows ACL hardening when `dir.is_dir()` is already true.
3. `resolve_nebulous_config_dir_from` falls back to `std::env::temp_dir().join("nebulous")` when config/home env vars are unset/empty.
4. On Windows, `restrict_to_current_user` builds the principal from spoofable `USERNAME`/`USERDOMAIN` env vars and invokes unqualified `"icacls"`, while `atomic.rs:142-144` falls back to non-atomic `remove_file` + `rename`.

Suggested Direction:
Add mode-aware `atomic_write_bytes_with_mode` (`OpenOptionsExt::mode(0o600)` on Unix), propagate `ensure_secure_dir` errors, disallow `temp_dir()` fallback for credential vaults without explicit `NEBULOUS_CONFIG_DIR`, and avoid non-atomic `remove_file` fallback on Windows.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
