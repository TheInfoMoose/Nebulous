# Phase 29 review

Range: `0f753331ef0ee175c364e1f628484788d7d94936..e2c5081f4cf91827b75e526b3ab8b80defdf0b6c`

## Independent pass

### Findings

1. **`crates/backend_core/src/git_sync/mod.rs:1409-1455` (`pull_repo_with_auth`) — `new defect`**
   - **Trigger:** A 3-way merge pull resolves a derived-index conflict (`resolved_derived_index_conflict == true`), writes the merged tree to the working directory via `repo.checkout_tree(tree.as_object(), Some(CheckoutBuilder::default().safe()))`, and then `storage.reconcile_indexes()?` (or private-index staging) fails (e.g., corrupt `.tracker/project.toml`).
   - **Effect:** `HEAD` stays at the pre-pull commit, but the working directory is left mutated with the merged tree (remote files and unreconciled `our` derived indexes) with no rollback. Even after fixing the underlying error, a subsequent pull fails with `"Uncommitted local changes in '...' would be overwritten by pull"`. The accompanying test (`crates/backend_core/tests/git_safety.rs:421-480`) only asserts `HEAD == head_before` and does not check workdir cleanliness.

2. **`crates/backend_core/tests/credential_hygiene.rs:285-323` (`vault_only_manager_does_not_fall_back_to_global_keyring_store`) — `test defect`**
   - **Trigger:** Running `cargo test --test credential_hygiene` with the default parallel test runner.
   - **Effect:** The test mutates process-global `NEBULOUS_CONFIG_DIR` via `std::env::set_var` / `remove_var` without synchronization while `vault_ssh_key_is_never_written_to_disk` (`crates/backend_core/tests/credential_hygiene.rs:227`) concurrently reads `nebulous_config_dir()` in the same binary, creating a flaky race condition (and unsafe concurrent `setenv`/`getenv` on POSIX).

3. **`crates/backend_core/src/git_sync/mod.rs:2058-2063` (`git_commit_documented`) — `new defect`**
   - **Trigger:** Calling `git_commit_documented` with `auto_append_to_desc = true` when `HEAD` is advanced concurrently between reading `parent_commit` (line 1941) and updating the ref (lines 2058-2063).
   - **Effect:** Updating the branch via `repo.reference(symbolic, doc_commit_oid, true, &doc_message)` (`force = true`) unconditionally overwrites the ref without checking that `HEAD` still matches `parent_commit` (unlike `repo.commit(Some("HEAD"), ...)`), which can silently drop a concurrent commit on the same branch. Additionally, if `doc_result` fails (lines 2070-2077), rollback errors from `write_item_detail` / `sync_parent_indexes_for_item` are silently discarded via `let _ =`.

4. **`crates/backend_core/src/crypto/keys.rs:197-209,394-402` (`import_ssh_key_file`, free `import_ssh_keypair`) — `new defect`**
   - **Trigger:** Importing a passphrase-protected OpenSSH private key through `KeyPairManager::import_ssh_key_file` or the module-level `import_ssh_keypair` helper.
   - **Effect:** `KeyPairManager::import_ssh_keypair_with_policy` guards passphrase stripping via `allow_strip_passphrase`, but neither `import_ssh_key_file` nor the free `import_ssh_keypair` helper offers a policy-guarded variant; both unconditionally pass `allow_strip_passphrase = true` via `import_ssh_keypair`.

5. **`crates/backend_core/src/workflow/lifecycle.rs:604-624` (`close_item_and_commit`) — `new defect`**
   - **Trigger:** `close_item_and_commit` calls `git_commit_documented`, which advances `HEAD` (lines 2060/2084) and then fails in `sync_user_index_paths` (line 2090, e.g., `.git/index.lock` held).
   - **Effect:** Because `git_commit_documented` returns `Err` after `HEAD` has already moved, `close_item_and_commit` invokes `rollback_close_and_commit` and restores `.tracker/` on disk to its pre-close open state, leaving `HEAD` with the closed item while the working tree is rolled back to open.

### What was traced
- All 11 changed code/test files in `0f753331..e2c5081f`: `crypto/keys.rs`, `crypto/mod.rs`, `git_sync/mod.rs`, `mcp/mod.rs`, `workflow/lifecycle.rs`, `nebulous_cli/src/commands/mod.rs`, and all 5 test files.
- Verified `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo test --workspace` all exit `0`.
- Verified `with_write_lock` re-entrancy (`HELD_WRITE_LOCKS`), `git_credential_for_remote` HTTPS PAT pinning and SSH fallback, `effective_push_remote_url` pushurl precedence, `allow_empty_ok` opt-in behavior in Rust API and MCP, `git_sync_with_report` abort-before-push on duplicate IDs, and absence of real sensitive data in code/tests.

### What was not checked
- Windows-specific paths (`#[cfg(windows)]` in `ensure_secure_dir`, `#[cfg(not(unix))]` in `index_stat_for_path`) — reviewed statically only, not executed on Windows.
- Live network Git HTTPS/SSH push/pull against an external server (verified only against local bare repos and local mock listeners).
- TUI layout/rendering (no UI files changed in this phase).

<!-- SEALED: Independent pass locked -->

## Contract pass

### Step-by-step verification (`STEP 47A`–`STEP 47F`)
- **STEP 47A (`TASK-0137`, `d4eae09`):** All `Contract & Behavior`, `Must still work`, and test requirements met. Scope lock respected. Advances `BUG-0035`, `BUG-0036`, and `BUG-0019`.
- **STEP 47B (`TASK-0138`, `f84d167`):** All `Contract & Behavior`, `Must still work`, and test requirements met. Scope lock respected. Advances `TASK-0135`, `TASK-0136`, and partially advances `TASK-0110` (`vault.key` KDF hardening remains parked/open).
- **STEP 47C (`TASK-0139`, `4104180`):** All `Contract & Behavior`, `Must still work`, and test requirements met. Scope lock respected. Advances `BUG-0037` and `BUG-0038`.
- **STEP 47D (`TASK-0140`, `f7d61d8`):** All `Contract & Behavior`, `Must still work`, and test requirements met with one spec-driven deviation the validator accepted:
  - **Deviation (Contract 3):** The plan asked for `repo.commit(Some("HEAD"), ..., &[&initial_commit])` for the follow-up doc commit, which `libgit2` rejects because `HEAD` still points to `parent_commit` rather than `initial_commit`. The coder created the doc commit with `update_ref = None` and moved the branch ref via `repo.reference(symbolic, doc_commit_oid, true, &doc_message)?` (`force = true`). This achieves the intended 2-commit reachable history but omits a CAS check on `HEAD` (Finding 3 / `BUG-0041`).
  - Advances `BUG-0014`.
- **STEP 47E (`TASK-0141`, `0356f8e`):** All `Contract & Behavior`, `Must still work`, and test requirements met. Scope lock respected. Advances `BUG-0004`.
- **STEP 47F (`TASK-0142`, `b365ef8`):** All `Contract & Behavior`, `Must still work`, and test requirements met. Scope lock respected. Advances `BUG-0015`.

### Deferrals & amendments
- **Deferrals:** None in Phase 29 (`.phased/deferred/` contains only prior-phase files `44C.md`, `44D.md`, `46F.md`).
- **Amendments:** None.

### Independent finding origins & filed items
| # | Finding | Class | Origin | Filed Item | Severity |
| :-: | :--- | :--- | :--- | :--- | :--- |
| 1 | `pull_repo_with_auth` leaves dirty workdir on `reconcile_indexes` failure after `checkout_tree` | `new defect` | `plan` | `BUG-0039` | `medium` |
| 2 | Unsynchronized `NEBULOUS_CONFIG_DIR` `env::set_var` race in `credential_hygiene.rs` | `test defect` | `plan` | `BUG-0040` | `low` |
| 3 | `git_commit_documented` `force=true` ref update without HEAD CAS check + discarded doc rollback errors | `new defect` | `coder` (triggered by `plan` spec) | `BUG-0041` | `low` |
| 4 | `import_ssh_key_file` & free `import_ssh_keypair` lack `allow_strip_passphrase` policy variant | `new defect` | `plan` | `BUG-0042` | `low` |
| 5 | `close_item_and_commit` rolls back `.tracker/` if `sync_user_index_paths` fails after `HEAD` advances | `new defect` | `plan` | `BUG-0043` | `low` |

No independent findings were withdrawn or downgraded.

## Handoff

- **Verdict:** PASS with follow-ups (0 regressions, 0 high/critical defects; 1 medium, 4 low pending items filed).
- **Steps:** All 6 steps (`STEP 47A`–`STEP 47F`, `TASK-0137`–`TASK-0142`) completed; 0 deferrals, 0 amendments.
- **New pending items (by priority):**
  - **Medium:** `BUG-0039` (`pull_repo_with_auth` leaves dirty workdir on `reconcile_indexes` failure after `checkout_tree`).
  - **Low:** `BUG-0040` (unsynchronized `NEBULOUS_CONFIG_DIR` `env::set_var` race in `credential_hygiene.rs`; pair with `TASK-0116`), `BUG-0041` (`git_commit_documented` `force=true` ref update & discarded doc rollback errors), `BUG-0042` (`import_ssh_key_file` / free `import_ssh_keypair` lack policy guard), `BUG-0043` (`close_item_and_commit` rolls back `.tracker/` if `sync_user_index_paths` fails after `HEAD` advances).
- **Deferred steps to re-split:** None.
- **`Advances:` items ready to close on user approval:** `BUG-0035`, `BUG-0036`, `BUG-0019`, `TASK-0135`, `TASK-0136`, `BUG-0037`, `BUG-0038`, `BUG-0014`, `BUG-0004`, `BUG-0015`. (`TASK-0110` stays open for parked `vault.key` KDF hardening.)
- **Decisions for user:** Approve closing the 10 `Advances:` items above; decide whether to fold `BUG-0039`–`BUG-0043` into the next phase or keep low-severity items parked.
