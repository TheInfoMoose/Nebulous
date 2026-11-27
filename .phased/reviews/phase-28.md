# Phase 28 review

Range: `2dddf4ff9800f096f69048c2fe3c0cc7228a892e..cd186cdc1a4c1ec615a43e42eb1fbccd3eb4fcd7`

## Independent pass

### Findings

1. **`crates/backend_core/src/git_sync/mod.rs:1013-1026` — `regression`**
   - **Trigger:** `git_credential_for_remote` is invoked with `configured_remote_url = Some("ssh://...")` (or SCP-style `git@host:org/repo.git`), `auth` containing both a `pat` and SSH credentials (`ssh_keypair`, `ssh_key_file`, or `ssh_auth_sock` — populated together by `resolve_git_auth_with_manager` whenever a bound account holds both), and `allowed_types` including both `CredentialType::USER_PASS_PLAINTEXT` and `CredentialType::SSH_KEY` (or `SSH_MEMORY` / `SSH_CUSTOM`).
   - **Effect:** Because `configured_remote_url` is `Some(...)` and `https_host_port(remote_url)` is `None`, the function immediately returns `Err("Refusing to send Personal Access Token: host mismatch with configured https:// remote")` instead of falling through to the SSH authentication branch at line 1037. At `<base>`, `pat_for_url` returned `None` for non-`https://` URLs and fell through to SSH credentials.

2. **`crates/backend_core/src/git_sync/mod.rs:1153-1158` — `new defect`**
   - **Trigger:** Calling `push_repo_with_auth` against a Git remote that configures a separate `remote.<name>.pushurl` on a different HTTPS host/port than `remote.<name>.url` (or an HTTPS `pushurl` paired with an SSH/HTTP fetch `url`).
   - **Effect:** `push_repo_with_auth` pins PAT delivery to `remote.url()` instead of `remote.pushurl().or_else(|| remote.url())`. When `libgit2` connects to `pushurl` and passes it to the credential callback, `git_credential_for_remote` rejects the PAT with a host mismatch error, breaking push authentication for valid `pushurl` remotes.

3. **`crates/backend_core/tests/credential_hygiene.rs:281-305` — `test defect`**
   - **Trigger:** Running `vault_only_manager_does_not_fall_back_to_global_keyring_store` with the old fallback (`.or_else(|| get_pat_from_keyring(account).ok())`) restored.
   - **Effect:** The test binds `"testuser/isolated"` in the isolated vault directory and asserts `resolved.pat.is_none()`, but never seeds a PAT for `"testuser/isolated"` in the global config/keyring store. Even if `resolve_git_auth_with_manager` still fell back to `get_pat_from_keyring`, it would return `None` and the test would pass with the isolation broken.

4. **`crates/backend_core/src/crypto/mod.rs:864-880` — `test defect`**
   - **Trigger:** Running `ensure_secure_dir_propagates_permission_error` with `fs::set_permissions(dir, perms)?` reverted to `let _ = fs::set_permissions(dir, perms);`.
   - **Effect:** Both error assertions (`ensure_secure_dir(&blocker)` and `ensure_secure_dir(&blocker.join("child"))`) fail inside `fs::create_dir_all` because `blocker` is a regular file, never reaching `fs::set_permissions`. The test never exercises a permission-hardening failure on an existing directory and passes even if `fs::set_permissions` errors are silently discarded.

### Traced & verified clean
- `crates/backend_core/src/agent_context/mod.rs`: `is_bidi_format`, `strip_control`, `single_line_safe`, and all un-fenced interpolations in `synthesize_bounded_prompt`; test `bounded_prompt_sanitizes_single_line_fields_and_bidi_chars` in `tests/workflow_integration.rs`.
- `crates/backend_core/src/storage/atomic.rs` and `crates/backend_core/src/storage/mod.rs`: `atomic_write_bytes_with_mode` Unix mode creation + `fchmod` + temp cleanup, Windows `replace_target` removal of non-atomic remove-then-rename fallback.
- `crates/backend_core/src/crypto/mod.rs`: `try_nebulous_config_dir` / `resolve_nebulous_config_dir_checked`, `SecureStorage::new()` erroring when config env vars are absent, `sync_keyring_on_store` stale-entry cleanup, `atomic_write_bytes_with_mode(..., Some(0o600))` for `vault.enc` and `vault.key`.
- `crates/backend_core/src/models/registry.rs`: per-path `REGISTRY_LOCKS`, `with_registry_read_lock` / `with_registry_write_lock`, read-only `persist_prune = false` vs write `persist_prune = true`, and updated/new tests in `tests/storage_integration.rs`.
- Sensitive data scan across all 9 changed files: only reserved placeholders (`example.com`, `127.0.0.1`, `[::1]`, `testuser`, synthetic token strings).

### Not checked
- Windows-only code paths (`icacls_exe_path`, `restrict_to_current_user`, Windows `replace_target` retry loop, Windows `fd_lock::RwLock::try_read` on read-only file handle) were inspected statically only; not compiled or executed on Windows.
- Live OS keyring backends (Secret Service / macOS Keychain / Windows Credential Manager) were not exercised; only unit-level `sync_keyring_on_store` and vault-only paths were checked.
- Full test suite runtime execution (only `cargo test --workspace --no-run` was run during the blind pass).

<!-- SEALED: Independent pass locked -->

## Contract pass

### Step-by-step contract check
- **STEP 46A (`TASK-0130`, `BUG-0033`):** All `Contract & Behavior`, `Must still work`, `Drops`, and `Tests` met cleanly. Existing `storage_integration.rs:2072` test updated to match the documented `Drops` behavior (read-only load no longer rewrites `projects.toml`). No deviations.
- **STEP 46B (`TASK-0131`, `BUG-0009` / `AUD-02`):** All production `Contract & Behavior` items met (`atomic_write_bytes_with_mode`, `try_nebulous_config_dir`, `SecureStorage::new()` rejection of implicit `temp_dir()`, `0o600` vault writes, `ensure_secure_dir` error propagation, Windows `replace_target` and `%SystemRoot%\System32\icacls.exe`). Finding #4 (`crypto/mod.rs:864-880`) stems from the plan's test spec testing `create_dir_all` blockers rather than a `set_permissions` failure.
- **STEP 46C (`TASK-0132`, `BUG-0011` / `AUD-04`):** All production `Contract & Behavior` items met (`sync_keyring_on_store`, removal of `FALLBACK_PAT_STORE` and direct `keyring::Entry` calls in `git_sync/mod.rs`, `resolve_git_auth_with_manager` isolation). Finding #3 (`credential_hygiene.rs:281-305`) stems from the plan's test spec omitting a seeded PAT in the global store to prove fallback suppression.
- **STEP 46D (`TASK-0133`, `BUG-0019` / `AUD-14`):** All contract items implemented as specified, including `https_host_port`, `git_credential_for_remote`, `build_auth_callbacks_for_remote`, `RemoteRedirect::None`, and both tests. However, the plan's own contract introduced Finding #1 (unconditional early `Err` on non-HTTPS remotes blocking SSH fallback) and Finding #2 (pinning `push_repo_with_auth` to `remote.url()` instead of `remote.pushurl()`). Neither finding is withdrawn or downgraded.
- **STEP 46E (`TASK-0134`, `BUG-0018` / `AUD-12`):** All `Contract & Behavior` and `Tests` met cleanly with zero findings.
- **STEP 46F (`TASK-0135`, `TASK-0110` non-breaking slice / `AUD-13`):** **Deferred** (`cd186cd`, `.phased/deferred/46F.md`). Neon correctly halted and filed `TASK-0136` (pending, high, scope lapse L2): changing `GitAuthMaterial::pat` to `Option<Zeroizing<String>>` breaks `as_deref()` assertions in `crates/backend_core/tests/workflow_integration.rs:1428,1450`, which was omitted from `STEP 46F`'s `ALLOWED_FILES`.
- **Amendments:** None.

### Finding origins & tracker items
| # | Location | Class | Origin | Severity | Filed Item |
| :-: | :--- | :--- | :--- | :--- | :--- |
| 1 | `crates/backend_core/src/git_sync/mod.rs:1013-1026` | `regression` | `plan` | `high` | `BUG-0035` (`pending`) |
| 2 | `crates/backend_core/src/git_sync/mod.rs:1153-1158` | `new defect` | `plan` | `medium` | `BUG-0036` (`pending`) |
| 3 | `crates/backend_core/tests/credential_hygiene.rs:281-305` | `test defect` | `plan` / `coder` | `low` | `BUG-0037` (`pending`) |
| 4 | `crates/backend_core/src/crypto/mod.rs:864-880` | `test defect` | `plan` / `coder` | `low` | `BUG-0038` (`pending`) |

## Handoff

- **Verdict:** **PASS WITH FINDINGS** (5/6 steps landed; 1 high regression, 1 medium defect, 2 low test defects, 1 deferred step).
- **Priority findings (filed as `pending`):**
  - High: `BUG-0035` — `git_credential_for_remote` blocks SSH fallback on non-HTTPS remotes when PAT is present (`git_sync/mod.rs:1013-1026`, regression from `STEP 46D` spec).
  - Medium: `BUG-0036` — `push_repo_with_auth` pins PAT to `remote.url()` instead of `remote.pushurl()` (`git_sync/mod.rs:1153-1158`, new defect from `STEP 46D` spec).
  - Low: `BUG-0037` — `vault_only_manager_does_not_fall_back_to_global_keyring_store` never seeds global store (`credential_hygiene.rs:281-305`, test defect).
  - Low: `BUG-0038` — `ensure_secure_dir_propagates_permission_error` only tests `create_dir_all`, not `set_permissions` (`crypto/mod.rs:864-880`, test defect).
- **Deferred step to re-stage:**
  - `STEP 46F` (`TASK-0135`, blocked by `TASK-0136` scope lapse): add `crates/backend_core/tests/workflow_integration.rs` to `ALLOWED_FILES` so PAT assertions at lines `1428,1450` can be updated for `Option<Zeroizing<String>>`; keep `TASK-0110` open until landed.
- **`Advances:` items ready to close on user approval:** `BUG-0033` (46A), `BUG-0009` (46B), `BUG-0011` (46C), `BUG-0018` (46E), and optionally `BUG-0019` (46D — HTTPS redirect fix landed; regression/defect tracked in `BUG-0035`/`BUG-0036`).
- **Decisions for user:** Confirm closing `BUG-0033`, `BUG-0009`, `BUG-0011`, `BUG-0018`, and whether to close `BUG-0019` now (with `BUG-0035`/`BUG-0036` open) or hold `BUG-0019` open until `BUG-0035`/`BUG-0036` land.
