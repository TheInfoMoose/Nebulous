# Phase 28 review

Range: `2dddf4ff9800f096f69048c2fe3c0cc7228a892e..9d980d88ad3a91fe5987e377750fbdb808d85041`

## Independent pass

Changed code files reviewed in full:
- `crates/backend_core/src/agent_context/mod.rs`
- `crates/backend_core/src/crypto/mod.rs`
- `crates/backend_core/src/git_sync/mod.rs`
- `crates/backend_core/src/models/registry.rs`
- `crates/backend_core/src/storage/atomic.rs`
- `crates/backend_core/src/storage/mod.rs`
- `crates/backend_core/tests/credential_hygiene.rs`
- `crates/backend_core/tests/storage_integration.rs`
- `crates/backend_core/tests/workflow_integration.rs`

### Findings

1. **`crates/backend_core/src/git_sync/mod.rs:1013-1035`** — `regression`
   - **Trigger:** Authenticating against a non-`https://` remote (e.g. `ssh://git@example.com/org/repo.git` or `git@example.com:org/repo.git`) when `auth.pat` is `Some(...)` (e.g. both a PAT and an SSH keypair are stored under the project's bound credential account) and `libgit2` invokes the credential callback with `allowed_types` containing both `USER_PASS_PLAINTEXT` and `SSH_KEY`/`SSH_MEMORY` (standard when an SSH server advertises both password and publickey auth).
   - **Effect:** `configured_hp` is `None`, so the `Some(remote_url)` arm returns `Err("Refusing to send Personal Access Token: host mismatch with configured https:// remote")` immediately instead of skipping PAT delivery and falling through to the SSH/`USERNAME`/`DEFAULT` credential branches at lines 1037–1074. At launch (`2dddf4ff`), `pat_for_url` returned `None` for non-`https://` URLs and fell through to SSH credentials.

2. **`crates/backend_core/src/git_sync/mod.rs:1154-1158`** — `regression`
   - **Trigger:** Calling `push_repo_with_auth` on a remote that configures `remote.<name>.pushurl` with a different HTTPS host/port than `remote.<name>.url` (or with a non-HTTPS fetch `url` and an `https://` `pushurl`).
   - **Effect:** `push_repo_with_auth` extracts `remote.url()` instead of `remote.pushurl()` (falling back to `remote.url()`). When `libgit2` connects to `pushurl`, `git_credential_for_remote` rejects the PAT with a host mismatch against the fetch URL. At launch (`2dddf4ff`), pushing to a remote with a distinct `pushurl` succeeded.

3. **`crates/backend_core/tests/credential_hygiene.rs:280-305`** — `test defect`
   - **Trigger:** Running `vault_only_manager_does_not_fall_back_to_global_keyring_store` against the pre-fix code where `resolve_git_auth_with_manager` still fell back to `get_pat_from_keyring`.
   - **Effect:** The test never stores a PAT for `"testuser/isolated"` in the global store before asserting `resolved.pat.is_none()`. Because no PAT exists in either store, the assertion passes even if the global fallback is still active.

4. **`crates/backend_core/src/crypto/mod.rs:864-880`** — `test defect`
   - **Trigger:** Running `ensure_secure_dir_propagates_permission_error` with `fs::set_permissions` errors still discarded (`let _ = fs::set_permissions(...)`).
   - **Effect:** Both error assertions pass paths blocked by a regular file (`blocker` and `blocker.join("child")`), which fail inside `fs::create_dir_all` (line 703) and never reach `fs::set_permissions` (line 711). The test therefore passes even when permission-setting failures are silently ignored.

### Not checked

- Windows-only code paths (`icacls_exe_path`, `restrict_to_current_user`, Windows `replace_target` retry loop) were inspected statically only and not executed on Windows.
- Live OS keyring daemon behavior (`keyring::Entry`) and real remote HTTPS/SSH Git servers were not exercised; `sync_keyring_on_store` and `git_credential_for_remote` were traced statically.
- Full workspace test execution (only `cargo test -p backend_core --no-run` was run during the independent pass).

<!-- SEALED: Independent pass locked -->

## Contract pass

### Step-by-step contract check
- **STEP 46A (`TASK-0130`, `BUG-0033` — closed):** All `Contract & Behavior`, `Must still work`, `Drops`, and `Tests` met. Existing `storage_integration.rs` assertion updated in line with the documented `Drops` (read-only load no longer rewrites `projects.toml` on disk). No deviations.
- **STEP 46B (`TASK-0131`, `BUG-0009` — closed):** All production `Contract & Behavior` items met (`atomic_write_bytes_with_mode`, `try_nebulous_config_dir`, `SecureStorage::new()` error on missing env vars, `0o600` vault writes, `ensure_secure_dir` error propagation, Windows `replace_target` and `%SystemRoot%\System32\icacls.exe`). Finding #4 (`crypto/mod.rs:864-880`) originates from the plan's test spec describing a regular-file blocker, which fails in `create_dir_all` rather than `set_permissions`.
- **STEP 46C (`TASK-0132`, `BUG-0011` — closed):** All production `Contract & Behavior` items met (`sync_keyring_on_store`, removal of `FALLBACK_PAT_STORE` and direct `keyring::Entry` calls in `git_sync/mod.rs`, `resolve_git_auth_with_manager` isolation). Finding #3 (`credential_hygiene.rs:280-305`) originates from the plan's test spec not requiring a seeded PAT in the global store to prove fallback suppression.
- **STEP 46D (`TASK-0133`, `BUG-0019` — open):** All contract items implemented as written (`https_host_port`, `git_credential_for_remote`, `build_auth_callbacks_for_remote`, `RemoteRedirect::None`, and both tests). The plan's contract directly caused Finding #1 (unconditional early `Err` on non-HTTPS remotes blocking SSH fallback) and Finding #2 (pinning `push_repo_with_auth` to `remote.url()` instead of `remote.pushurl()`). Neither finding is withdrawn or downgraded.
- **STEP 46E (`TASK-0134`, `BUG-0018` — closed):** All `Contract & Behavior` and `Tests` met with zero findings.
- **STEP 46F (`TASK-0135`, `TASK-0110` — open):** Deferred (`cd186cd`, `.phased/deferred/46F.md`). `TASK-0136` (pending, high) records the scope lapse: `crates/backend_core/tests/workflow_integration.rs` was missing from `ALLOWED_FILES` and needs PAT assertion updates for `Option<Zeroizing<String>>`.
- **Amendments:** None.

### Finding origins & duplicate check
All four independent findings match existing `pending` items already filed in `.tracker/` (from `9d980d8`); no duplicate items were created:

| # | Location | Class | Origin | Severity | Existing Item |
| :-: | :--- | :--- | :--- | :--- | :--- |
| 1 | `crates/backend_core/src/git_sync/mod.rs:1013-1035` | `regression` | `plan` | `high` | `BUG-0035` (`pending`) |
| 2 | `crates/backend_core/src/git_sync/mod.rs:1154-1158` | `regression` | `plan` | `medium` | `BUG-0036` (`pending`) |
| 3 | `crates/backend_core/tests/credential_hygiene.rs:280-305` | `test defect` | `plan` / `coder` | `low` | `BUG-0037` (`pending`) |
| 4 | `crates/backend_core/src/crypto/mod.rs:864-880` | `test defect` | `plan` / `coder` | `low` | `BUG-0038` (`pending`) |

## Handoff

- **Verdict:** **PASS WITH FINDINGS** (5/6 steps landed; 2 regressions from `STEP 46D` spec, 2 test defects, 1 deferred step). Independent second pass confirms `phase-28.md` with zero new findings beyond already-filed items.
- **Priority findings (already `pending` in `neb`):**
  - High: `BUG-0035` — `git_credential_for_remote` blocks SSH fallback on non-HTTPS remotes when PAT is present (`git_sync/mod.rs:1013-1035`).
  - Medium: `BUG-0036` — `push_repo_with_auth` pins PAT to `remote.url()` instead of `remote.pushurl()` (`git_sync/mod.rs:1154-1158`).
  - Low: `BUG-0037` — `vault_only_manager_does_not_fall_back_to_global_keyring_store` never seeds global store (`credential_hygiene.rs:280-305`).
  - Low: `BUG-0038` — `ensure_secure_dir_propagates_permission_error` only exercises `create_dir_all`, not `set_permissions` (`crypto/mod.rs:864-880`).
- **Deferred step to re-stage:**
  - `STEP 46F` (`TASK-0135`, blocked by `TASK-0136`): add `crates/backend_core/tests/workflow_integration.rs` to `ALLOWED_FILES` so PAT assertions can be updated for `Option<Zeroizing<String>>`; keep `TASK-0110` open until landed.
- **Tracker state:** `BUG-0033`, `BUG-0009`, `BUG-0011`, `BUG-0018` are already closed (`9d980d8`). `BUG-0019` and `TASK-0110` remain open.
- **Decisions for user:** Confirm whether to keep `BUG-0019` open until `BUG-0035`/`BUG-0036` land (or close `BUG-0019` now), and reply `approved` to commit `.phased/reviews/phase-28-2.md`.
