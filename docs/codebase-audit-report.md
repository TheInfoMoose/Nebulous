# Nebulous Codebase Audit Report (`v0.1.0` Beta)

**Scope**: Active workspace (`crates/backend_core/`, `crates/nebulous_cli/`, `scripts/`, `docs/`, `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.github/workflows/`, `mcpb/manifest.json`, `server.json`, `.orrery/`, `.phased/`, `README.md`, `CHANGELOG.md`, `STEPPED_IMPLEMENTATION_PLAN.md`, `.gitignore`, `LICENSE`).
**Excluded**: `archive/`, `target/`, `.tracker/`.
**Method**: Direct line-by-line manual code, script, config, and documentation inspection across 3 independent tracks, cross-verified against source files and non-mutating Cargo/shell checks.

---

## Executive Summary & Baseline Checks

Nebulous `0.1.0` has strong foundational engineering in several core areas—atomic path containment for tracker subpaths, bounded-prompt fencing, OSC-52 base64 terminal escaping, TUI narrow-viewport guards, and a passing 251-test workspace suite—but direct code inspection uncovered **33 verified issues** (**11 High**, **19 Medium**, **3 Low**) that existing tests and linters miss, alongside **3 remaining unverified leads**.

For a solo maintainer, the highest-leverage risks cluster around:
1. **Supply-chain & CI failure**: `Cargo.lock` locks `url 2.5.8` (`RUSTSEC-2026-0592`), causing `cargo deny` (and therefore CI/release pipelines) to fail immediately.
2. **Secret & filesystem containment gaps**: `vault.key`/`vault.enc` creation has a pre-`0600` umask window and ignored directory/ACL hardening errors; `SecureStorage` suffers keyring/vault split-brain; `guard_tracker_path` trusts a symlinked `.tracker/` root; `libgit2` default redirects can leak PATs across HTTPS hosts.
3. **Data-integrity & deadlock/hang bugs**: Moving `TASK-*` items across phases leaves ghost stubs; mixed relative/canonical `TrackerStorage` paths cause a 30s same-thread self-deadlock; `git_sync` / `git_commit_documented` / `neb done -c` have unlocked/non-atomic failure windows; `scripts/phased-step.sh` crashes on clean or `.tracker/`-only trees; `arboard` hangs indefinitely on stalled X11 sockets.
4. **Scope lapse & maintenance drag**: Roughly 1,500+ LOC of withdrawn OS/ISO rolling-release tooling and Orrery/Argon harness coupling remain embedded in `backend_core`, CLI/MCP schemas, docs, and `scripts/`, while `docs/USAGE.md` documents a removed `--all` flag.

### Non-Mutating Baseline Verification Results

| Check Command | Result | Key Detail |
|---|---|---|
| `cargo fmt --all -- --check` | **Pass** (`exit 0`) | Zero formatting diffs. |
| `cargo clippy --workspace --all-targets -- -D warnings` | **Pass** (`exit 0`) | Zero compiler/clippy warnings. |
| `cargo test --workspace` | **Pass** (`exit 0`) | 251 passed, 0 failed (`backend_core`: 76 lib + 137 integration across 9 suites; `nebulous_cli`: 12 lib + 26 integration across 3 suites). |
| `cargo deny --offline check advisories bans licenses sources` | **Fail** (`exit 1`) | `advisories FAILED` (`RUSTSEC-2026-0592` on `url 2.5.8`); `bans ok`, `licenses ok`, `sources ok`; duplicate-version warnings (`aead`, `aes`, `aes-gcm`, `sha2`, `rand`, `windows-sys`, etc.). |
| `for s in scripts/*; do bash -n "$s"; done && bash scripts/check-neutral.sh` | **Pass** (`exit 0`) | All 7 scripts syntax-valid; `check-neutral: OK (0 allowlisted file(s), 0 line(s) left to scrub)`. |

---

## Focus Area Rollup

| Focus Area | High | Medium | Low | Core Assessment |
|---|---:|---:|---:|---|
| **Security** | 4 | 6 | 1 | `RUSTSEC-2026-0592` (`url 2.5.8`) breaks `deny.toml`; `vault.key`/`vault.enc` have a pre-`0600` creation window + ignored `chmod`/ACL errors; keyring write failures create credential split-brain; `.tracker/` symlink root bypasses containment; `libgit2` initial redirects allow cross-host HTTPS PAT leakage; `agent_context` leaves single-line fields unfenced; CI actions lack SHA pinning and omit `licenses`/`sources`. *(Verified sound: OSC-52 base64 encoding, `ComponentCommand::Init` path validation, PAT HTTPS-only scheme check on non-redirected URLs, `git2` `Url::origin()` unreachability.)* |
| **Stability** | 9 | 12 | 2 | `move_item_to_phase` corrupts `TASK-*` indexes; non-canonicalized lock keys self-deadlock for 30s; `git_sync` / `git_commit_documented` / `neb done -c` have unlocked/non-atomic windows; `phased-step.sh` `check` crashes under `set -euo pipefail`; `arboard` hangs on stalled X11 sockets; `TrackerWatcher::drop` can block 30s; `reconcile`/`metrics` abort globally on one bad file; TUI/CLI suffer hash-ID clipping, silent error swallowing, and stale SHA outputs. |
| **Sustainability** | 2 | 9 | 3 | Monolithic `commands/mod.rs` (3,146 LOC), 994 KB `STEPPED_IMPLEMENTATION_PLAN.md`, `.orrery/` vs `.phased/` split, widespread doc drift (`docs/USAGE.md` `--all`, 8+ undocumented subcommands), dual RustCrypto versions from `ssh-key 0.7.0-rc.11`, decoupled Phase 25 `0.0.0` templates (`mcpb/`, `server.json`), and test-suite hazards (`env::set_var`). |
| **Scope Lapse** | 1 | 3 | 0 | Removed OS/ISO rolling-release engine (`OsImage`, `KernelModule`, `apt_sid`, `orbit_image`, 4 MCP tools) and hardcoded Orrery/Argon agent-harness glue persist across `backend_core`, CLI, MCP, docs, and `scripts/`, doubling solo-maintainer surface area. |

*(Counts reflect findings tagged with each focus area; several findings span multiple focus areas.)*

---

## Prioritized Verified Findings

Findings are ordered strictly by severity (**High** -> **Medium** -> **Low**) and prioritized within each tier by concrete risk to a solo beta maintainer.

### High Severity

#### AUD-01 — Locked `url 2.5.8` Triggers Open Vulnerability `RUSTSEC-2026-0592`, Failing `cargo deny` & CI/Release Gates
- **Severity**: High
- **Focus Area(s)**: Security, Stability, Supply Chain
- **Citations**: `Cargo.lock:3280-3282`; `crates/backend_core/Cargo.toml:14`; `crates/nebulous_cli/Cargo.toml:29`; `deny.toml:2, 8-13`; `CHANGELOG.md:34`; `.github/workflows/ci.yml:48-52`; `.github/workflows/release.yml:49-52`; `crates/backend_core/src/git_sync/mod.rs:571-595`; `git2-0.21.0/src/cred.rs:214-219, 286-287`
- **Rationale**: `cargo deny --offline check advisories bans licenses sources` fails with exit code `1` because `Cargo.lock` pins `url 2.5.8` (pulled transitively solely via `git2 0.21.0`). `url <= 2.5.8` is affected by `RUSTSEC-2026-0592` (unrecoverable stack-overflow abort in `Url::origin()` on nested `blob:` URLs, fixed in `>= 2.5.9`). Inspection of `git2-0.21.0/src/` confirms `git2` only calls `Url::parse`, `.host()`, `.port()`, `.scheme()`, and `.path()`—never `Url::origin()` or `url::quirks::origin()`—and `backend_core` parses URLs manually without `CredentialHelper`, so the panic itself is unreachable in current code. Even so, `deny.toml:13` enforces zero open advisories (`ignore = []`).
- **Impact**: Both `ci.yml` and `release.yml` fail immediately at the `cargo deny` gate, blocking releases and contradicting `CHANGELOG.md:34` and `deny.toml:2`.
- **Suggested Direction**: Run `cargo update -p url` to advance `Cargo.lock` to `url >= 2.5.9` and re-verify `cargo deny check advisories bans licenses sources`.

#### AUD-02 — Pre-`0600` Creation Window, Ignored Directory/ACL Hardening Errors, and `/tmp/nebulous` Fallback for `vault.key` & `vault.enc`
- **Severity**: High
- **Focus Area(s)**: Security, Stability
- **Citations**: `crates/backend_core/src/storage/atomic.rs:17-20, 51-68, 108-146, 164-168`; `crates/backend_core/src/crypto/mod.rs:190, 544-545, 603-604, 615-652, 656-675`
- **Rationale**:
  1. `atomic_write_bytes` (`atomic.rs:54-66, 164-168`) creates temp files using process `umask` (typically `0o644`/`0o664`), writes the full payload, and only applies `original_permissions` afterward. When `vault.key` or `vault.enc` is newly created (`original_permissions == None`), `crypto/mod.rs:544-545, 603-604` writes and renames secret material under `umask` before calling `set_secure_file_permissions` (`0o600` / Windows `icacls`).
  2. `ensure_secure_dir` (`crypto/mod.rs:666, 669-670`) ignores permission failures via `let _ = ...` and skips Windows ACL hardening when `dir.is_dir()` is already true.
  3. If `NEBULOUS_CONFIG_DIR`, `XDG_CONFIG_HOME`, `APPDATA`, `HOME`, and `USERPROFILE` are all unset/empty, `resolve_nebulous_config_dir_from` (`crypto/mod.rs:190`) falls back to `std::env::temp_dir().join("nebulous")` (world-writable `/tmp/nebulous` on Unix).
  4. On Windows, `restrict_to_current_user` (`crypto/mod.rs:616-639`) builds the principal from spoofable `USERNAME`/`USERDOMAIN` env vars and calls unqualified `"icacls"`, while `atomic.rs:142-144` falls back to non-atomic `remove_file` + `rename` (an exception explicitly noted at `atomic.rs:18-20` after line `17`'s general `"no non-atomic fallback"` claim).
- **Impact**: On multi-user Unix systems (especially if `temp_dir()` fallback or pre-existing directories are hit), `vault.key` and `vault.enc` are briefly or persistently readable by other local users; on Windows, transient lock contention during `replace_target` can delete the target file on crash.
- **Suggested Direction**: Add a mode-aware `atomic_write_bytes_with_mode` (`OpenOptionsExt::mode(0o600)` on Unix temp file creation), propagate `ensure_secure_dir` errors, disallow `temp_dir()` fallback for credential vaults without explicit `NEBULOUS_CONFIG_DIR`, and avoid non-atomic `remove_file` fallback in `atomic.rs` (use `ReplaceFileW` or fail safely).

#### AUD-03 — `.tracker/` Root Symlink Escape Bypass and TOCTOU Window in `guard_tracker_path` / `contained_path`
- **Severity**: High
- **Focus Area(s)**: Security
- **Citations**: `crates/backend_core/src/storage/mod.rs:239-244, 419-428, 1281-1384`; `crates/backend_core/tests/path_containment.rs:141-277`
- **Rationale**:
  1. `TrackerStorage::discover` (`storage/mod.rs:239-244`) probes `dir.join(".tracker").join("project.toml").is_file()` (following symlinks) and sets `tracker_dir = candidate`. `resolve_contained` (`1318-1335`) canonicalizes `base` (`self.tracker_dir`) via `canonicalize_existing_prefix(base)?` before checking `canon_target.starts_with(&canon_base)`. If `.tracker` itself in an untrusted repo is a symlink pointing outside the repo, both `canon_base` and `canon_target` resolve inside the external target, so containment passes. When `rel` is empty (`guard_tracker_path(&self.tracker_dir)` at line `517`), `has_normal` is `false`, skipping `reject_final_symlink`.
  2. `resolve_contained` returns the lexical `PathBuf` (`joined`), and callers open files/dirs using that lexical path without `O_NOFOLLOW` or directory-fd pinning (TOCTOU window). Existing `tests/path_containment.rs:141-277` never tests a symlinked `.tracker` root.
- **Impact**: Opening or auto-discovering a cloned/untrusted workspace with a symlinked `.tracker` redirects tracker reads/writes (`.lock`, `.gitignore`, `project.toml`, `items/`) outside the repository boundary.
- **Suggested Direction**: Reject `fs::symlink_metadata(&self.tracker_dir)` when `.tracker` itself is a symlink in `discover` and `guard_tracker_path`, and use `O_NOFOLLOW` / `openat`-style handles on Unix for sensitive tracker file I/O.

#### AUD-04 — `SecureStorage` Keyring-First Read + Silent Keyring Write Failure Causes Credential Split-Brain
- **Severity**: High
- **Focus Area(s)**: Security, Stability
- **Citations**: `crates/backend_core/src/crypto/mod.rs:314-316, 391-435`; `crates/backend_core/src/git_sync/mod.rs:272-287, 311-333, 1506-1508`
- **Rationale**: `SecureStorage::store_secret` (`crypto/mod.rs:392-412`) writes to `vault.enc` first, then attempts `entry.set_password(secret)` and swallows **all** `keyring::Error` variants (`398-407`), returning `Ok(())`. However, `SecureStorage::get_secret` (`420-430`) reads from the OS keyring **first** and only falls back to `vault.enc` on `Err`. If an older credential exists in the keyring and updating it fails (locked keyring, size limit, or platform rejection), `store_secret` succeeds silently while `get_secret` keeps returning the stale keyring credential. Separately, `store_pat_in_keyring` / `get_pat_from_keyring` (`git_sync/mod.rs:281-286, 313-322`) access `keyring::Entry` directly in addition to `SecureStorage`, and `resolve_git_auth_with_manager` (`1506-1508`) falls back to `.or_else(|| get_pat_from_keyring(account).ok())` even when passed `KeyPairManager::vault_only(...)`, bypassing `NEBULOUS_DISABLE_KEYRING`.
- **Impact**: Rotating a PAT or SSH key can silently revert to a revoked/stale credential while reporting success, and `NEBULOUS_DISABLE_KEYRING=1` / `vault_only` are bypassed by `git_sync` PAT resolution.
- **Suggested Direction**: On keyring update failure in `store_secret`, delete any stale keyring entry (or surface an error if deletion also fails) and route `git_sync` PAT functions exclusively through `SecureStorage`.

#### AUD-05 — `move_item_to_phase` Leaves Ghost `TASK-*` Stubs and Corrupts Source `phase.toml` Summary
- **Severity**: High
- **Focus Area(s)**: Stability
- **Citations**: `crates/backend_core/src/workflow/lifecycle.rs:935, 948, 966-983, 1035`; `crates/backend_core/src/models/scope.rs:90-94`; `crates/backend_core/tests/workflow_integration.rs:1494-1652`
- **Rationale**: `move_item_to_phase` deletes the old `items/<ITEM_ID>` directory (`934-939`) and calls `remove_item_from_parent_indexes` (`948`). Unlike `ScopeItems::remove_id` (`scope.rs:90-94`), `remove_item_from_parent_indexes` (`lifecycle.rs:973-980`) only retains out `bugs` and `features`, leaving `scope_manifest.items.tasks` untouched. `workflow_integration.rs:1529-1641` only tests moving `FEAT-0001`, never `TASK-*`.
- **Impact**: Moving any `TASK-*` item between phases deletes its source directory while leaving a dangling stub in the source `scope.toml` and inflated counts in `phase.toml`. Subsequent `load_index().find_item` resolves the stale source stub first, causing `load_item_detail_by_id` to fail with `Io(NotFound)` until `reconcile_indexes` is run.
- **Suggested Direction**: Replace `lifecycle.rs:973-980` with `scope_manifest.items.remove_id(item_id);` and add a `TASK-*` move regression test in `workflow_integration.rs`.

#### AUD-06 — Non-Canonicalized `HELD_WRITE_LOCKS` & `PROJECT_CACHE` Key Causes Same-Thread 30s Self-Deadlock
- **Severity**: High
- **Focus Area(s)**: Stability
- **Citations**: `crates/backend_core/src/storage/mod.rs:32-40, 189-206, 221-244, 490-497, 512-515, 553, 720-766`
- **Rationale**: `TrackerStorage::new` (`189-199`) and `from_tracker_dir` (`202-206`) preserve caller-supplied relative/symlinked paths, while `TrackerStorage::discover` (`221-244`) canonicalizes `start_dir`. `lock_path()` (`490-492`) returns `self.tracker_dir.join(".lock")` without canonicalization, and both `HELD_WRITE_LOCKS` (`513, 553`) and `PROJECT_CACHE` (`721-763`) key on that un-canonicalized `PathBuf`.
- **Impact**: If nested code on the same thread enters `with_write_lock` via two `TrackerStorage` instances with different path spellings (e.g., `TrackerStorage::new(".")` and `TrackerStorage::discover(".")`), `HELD_WRITE_LOCKS` misses and `fd_lock::RwLock::try_write()` spins for the full 30s `DEFAULT_LOCK_TIMEOUT` before failing with `TrackerError::LockTimeout`.
- **Suggested Direction**: Derive a canonicalized lock/cache key (via `canonicalize_existing_prefix(&self.tracker_dir)`) in a single helper used by `holds_write_lock`, `with_write_lock_timeout`, `read_project`, and `write_project`.

#### AUD-07 — `git_sync` Merge-Reconcile Error Suppression, Unchecked `duplicate_ids` Push, and `git_commit_documented` Pre-Amend Failure Window
- **Severity**: High
- **Focus Area(s)**: Stability
- **Citations**: `crates/backend_core/src/git_sync/mod.rs:801-814, 1247, 1291-1326, 1840-1914, 1934-1953`; `crates/backend_core/tests/git_safety.rs:1-426`
- **Rationale**:
  1. In `pull_repo_with_auth` (`1293`), after auto-resolving `.tracker/` index conflicts, `if storage.reconcile_indexes().is_ok()` silently discards reconciliation failures, leaving stale/conflicted index manifests in the merge commit.
  2. In `git_sync_with_report` (`1943-1945`), if `git_pull_with_report` reports `!pull_outcome.duplicate_ids.is_empty()` (colliding item IDs created on two clones), `self.git_push(auth)?` still immediately pushes the collision to the remote.
  3. In `git_commit_documented` (`1840-1914`), `repo.commit` advances `HEAD` (`1842`) *before* `write_item_detail` (`1888`), `sync_parent_indexes_for_item` (`1889`), and `initial_commit.amend` (`1897`). Any failure in `1888-1904` leaves `HEAD` moved to the un-amended commit, dirty `.tracker/` edits on disk, and `.git/index` out of sync (`sync_user_index_paths` at `1914` is skipped). Even on success, `recorded_fix_sha` written into `item.toml` and `desc.md` (`1847, 1872, 1880`) is the pre-amend commit SHA, which is orphaned by `amend()` (`1905`).
  4. `sync_user_index_paths` (`801-814`) populates `.git/index` entries with zeroed `ctime`/`mtime`/`ino`/`uid`/`gid`, leaving updated entries stat-dirty to `git` CLI.
- **Impact**: Mid-commit errors desynchronize `.git/index` from `HEAD` (risking silent reverts on the user's next `git commit`); duplicate-ID merges propagate to shared remotes; recorded fix SHAs in `item.toml`/`desc.md` do not exist in `git log`.
- **Suggested Direction**: Propagate `reconcile_indexes()` errors on merge resolution; abort `git_sync` before `git_push` when `duplicate_ids` is non-empty; compute/stage tracker docs before creating the final commit (or reset `HEAD` and resync index on failure).

#### AUD-08 — `neb done -c` (`handle_close`) Lacks Outer `.tracker/.lock` Across Close+Commit and Swallows Rollback Errors
- **Severity**: High
- **Focus Area(s)**: Stability
- **Citations**: `crates/nebulous_cli/src/commands/mod.rs:1307-1358`; `crates/backend_core/src/workflow/lifecycle.rs:495-553`; `crates/nebulous_cli/tests/done_commit_safety.rs:130-166`
- **Rationale**: `handle_close` invokes `validate_documented_commit`, `load_item_detail_by_id`, `close_item_reporting`, optional `write_item_manifest` (popping `previous_fixes`), and `git_commit_documented` as separate operations without holding `.tracker/.lock` across the transaction. If `git_commit_documented` fails after `close_item_reporting` succeeds, lines `1351-1355` execute `let _ = storage.write_item_detail(...)`, `let _ = storage.reconcile_indexes()`, and `let _ = storage.write_active_timer(&timer)`, silently ignoring rollback failures. `done_commit_safety.rs` only tests pre-close validation rejection.
- **Impact**: Concurrent watcher/MCP activity can interleave mid-close, and a failed git commit coupled with a failed rollback silently leaves the item closed with lost `previous_fixes` or timer state.
- **Suggested Direction**: Move the `done -c` close+commit+rollback flow into `backend_core` inside a single `with_write_lock` scope, surface rollback failures explicitly, and add a post-close commit-failure test.

#### AUD-09 — `scripts/phased-step.sh` `code_stamp()` Crashes Under `set -euo pipefail` on Clean/`.tracker/`-Only Trees (+ macOS/Markdown Fragility)
- **Severity**: High
- **Focus Area(s)**: Stability, Sustainability
- **Citations**: `scripts/phased-step.sh:42, 216, 219-220, 245-247, 269, 326, 548-549, 553-556, 596, 609`; `scripts/validate-step.sh:78`
- **Rationale**:
  1. `code_stamp()` (`246`) runs `fingerprint | grep -v '^[^ ]*  \.tracker/' | sha256sum | cut -d' ' -f1`. Under `set -euo pipefail` (`42`), whenever `fingerprint` emits zero non-`.tracker/` lines (e.g., a `.tracker/`-only step or a `BUG-0021` no-op rerun where `STEP_PATHS` is empty), `grep -v` exits `1` and aborts `check` (`609`: `code_stamp >"${SNAP}/pass"`). While `commit` (`549`: `[[ "$(code_stamp)" == "$(cat "${SNAP}/pass")" ]] || ...`) suppresses `errexit` inside `[[ ... ]]` on the LHS of `||`, `check` aborts first so `${SNAP}/pass` is missing (`548`) and the `BUG-0021` rerun handler (`556`) is never reached.
  2. `tracker_items()` (`269`) uses GNU-only `find -printf '%h\n'`, breaking on macOS/BSD.
  3. `ALLOWED` extraction (`phased-step.sh:216`, `validate-step.sh:78`) keeps literal Markdown backticks around paths (`` - `crates/foo.rs` ``), and `PRE_GATE`/`POST_GATE` extraction (`219-220`) reads only one line after the tag.
- **Impact**: Any `.tracker/`-only step or no-op rerun crashes `phased-step.sh check`; macOS contributors cannot use `phased-step.sh`.
- **Suggested Direction**: Replace `grep -v ...` with `{ grep -v '^[^ ]*  \.tracker/' || true; }` (or `awk`), replace `find -printf` with portable `dirname` / Python `pathlib`, and strip Markdown backticks in `ALLOWED` extraction.

#### AUD-10 — `docs/USAGE.md` Documents Removed `--all` Flag on `neb git commit` and `neb sync`
- **Severity**: High
- **Focus Area(s)**: Stability, Documentation Parity
- **Citations**: `docs/USAGE.md:166, 169`; `crates/nebulous_cli/src/args.rs:301-323, 687-717`; `crates/nebulous_cli/src/lib.rs:280`; `crates/nebulous_cli/tests/cli_integration.rs:1005-1011`
- **Rationale**: Both copy-pasteable examples in `docs/USAGE.md` (`166, 169`) pass `--all` to `neb git commit` and `neb sync`. `--all` was intentionally removed and replaced with explicit `--path <FILE1,FILE2>` (or `.tracker/`-only default commits), and unit test `lib.rs:280` explicitly asserts `--all` fails to parse.
- **Impact**: Every user or coding agent following `docs/USAGE.md` for `neb git commit` or `neb sync` immediately hits a `clap` CLI parse error (`unexpected argument '--all'`).
- **Suggested Direction**: Update `docs/USAGE.md:165-170` to show `--path <FILES>` and `.tracker/`-only default commit behavior.

#### AUD-11 — Scope Lapse: Removed OS/ISO Release-Engine Subsystem & Hardcoded Orrery/Argon Coupling Persist Across Code, Docs, and Scripts
- **Severity**: High
- **Focus Area(s)**: Scope Lapse, Sustainability
- **Citations**: `CHANGELOG.md:29`; `crates/backend_core/src/models/component.rs:19-31, 180-613`; `crates/backend_core/src/models/project.rs:24-25`; `crates/backend_core/src/mcp/mod.rs:629, 641, 702, 739, 745-812, 945-946, 1360, 1399, 1718-2149, 2635-2679, 2758`; `crates/backend_core/src/ingestion/mod.rs:763-869`; `crates/backend_core/src/planning/{mod.rs,import.rs}`; `crates/backend_core/src/watcher/mod.rs:33`; `crates/backend_core/src/error.rs:90`; `crates/nebulous_cli/src/args.rs:567, 593, 602`; `README.md:7, 34`; `docs/USAGE.md:6`; `scripts/{phased-step.sh,neon-dispatch,validate-step.sh}`; `crates/backend_core/tests/mcp_orrery_context.rs:8`; `crates/backend_core/tests/plan_import.rs:32`
- **Rationale**: Although `CHANGELOG.md:29` states the release engine and `neb release` were removed to a separate project:
  1. `models/component.rs` (~430 LOC), `mcp/mod.rs` (4 "Suite 3: Release Engineering & Self-Healing CI Tools", `"orbit_image"`, `"apt_sid"`, `"kernel_custom"`), `ingestion/mod.rs:763-869`, `args.rs` (`os_image`, `kernel_module`, `scripts/build_iso.sh`), and `README.md`/`docs/USAGE.md` (`rolling-release OS builders`) all remain active (`scripts/check-neutral.sh` does not flag `orbit_image`/`apt_sid`/`kernel_custom`).
  2. `backend_core` hardcodes Orrery/Argon workflow specifics (`orrery.step_title`, `item_id_from_orrery_step` restricted to numeric+uppercase IDs like `41B`, `planning/mod.rs` hardcoding `## Step Execution Roadmap` / `### Up Next`), `watcher/mod.rs:33` still references `libcosmic` (archived in `archive/cosmic_interface`), `error.rs:90` hardcodes the `neb` CLI binary name, and `scripts/` carries 858 LOC of external agent-orchestration scripts (`phased-step.sh`, `neon-dispatch`, `validate-step.sh`).
- **Impact**: A solo developer maintaining a `0.1.0` project-and-bug tracker carries ~1,500+ LOC of out-of-scope release-engineering, OS-distro, Orrery/Argon, and agent-harness surface area, increasing bug density and confusing beta users.
- **Suggested Direction**: Decide explicitly for `0.1.x` whether component rolling-update/pinning and Orrery/Argon glue belong in `backend_core`; if not, extract or feature-gate them (`#[cfg(feature = "release-pins")]` / external adapter), rebrand generic artifact pinning if kept, remove `libcosmic`/`orbit_image`/`apt_sid` leftovers, and separate repo-dev agent scripts from shipped `scripts/`.

---

### Medium Severity

#### AUD-12 — `agent_context::synthesize_bounded_prompt` Fails to Fence/Sanitize Untrusted Single-Line Fields
- **Severity**: Medium
- **Focus Area(s)**: Security
- **Citations**: `crates/backend_core/src/agent_context/mod.rs:16-20, 148, 153-170, 181-183, 209-219`; `crates/backend_core/src/mcp/mod.rs:1801-1803, 1943-1951`
- **Rationale**: `scope.description` (`177`) and `detail.description_markdown` (`232`) are wrapped in adaptive-length `fenced(...)` blocks, but `item.title` (`157`) only passes through `strip_control`, which preserves `\n` and `\t` (`18`) and passes Unicode bidi controls (category `Cf`). `scope.id`, `scope.target`, `tech_stack`, `fix.date`, and `fix.commit` are interpolated raw into headings, prose, blockquotes, and inline code spans.
- **Impact**: Untrusted `.tracker/` manifests (e.g., from an external repo or injected via CI log strings in `record_verification_run`) can break out of Markdown headings/inline code and inject prompt instructions into `bounded_prompt`.
- **Suggested Direction**: Add a `single_line_safe()` sanitizer collapsing `\r`/`\n`/`\t` to spaces, stripping `Cc` and bidi `Cf` characters, and escaping/stripping backticks for all single-line interpolated fields.

#### AUD-13 — Plaintext-Equivalent `vault.key` Seed, Single-Round SHA-256 KDF, Un-Zeroized PATs, and Passphrase-Stripping SSH Import
- **Severity**: Medium
- **Focus Area(s)**: Security
- **Citations**: `crates/backend_core/src/crypto/mod.rs:194-204, 233-241, 263-277, 438-441, 491, 539-542, 560-608`; `crates/backend_core/src/crypto/keys.rs:145-172, 256-259, 281`; `crates/backend_core/src/git_sync/mod.rs:25-29, 242, 338-342, 1015-1028, 1496-1524`
- **Rationale**:
  1. `vault.key` stores a raw 32-byte seed on disk alongside `vault.enc`, and `derive_key_sha256` (`crypto/mod.rs:194-204`) uses a single unsalted `Sha256` round—offering minimal brute-force resistance when `NEBULOUS_VAULT_MASTER_KEY` is a human passphrase.
  2. `FALLBACK_PAT_STORE` (`git_sync/mod.rs:25-29, 341`) caches plain `String` PATs without zeroizing on removal, and `GitAuthMaterial::pat` (`242`) is `Option<String>`, stripping `Zeroizing` from `GitAuthRequest::pat` (`1496-1501`).
  3. `KeyPairManager::import_ssh_keypair` (`keys.rs:145-172, 281`) decrypts passphrase-protected OpenSSH keys and re-serializes them unencrypted into `SecureStorage`, while `git_credential` (`git_sync/mod.rs:1015-1028`) always passes `None` as passphrase for memory and `~/.ssh/id_*` keys.
- **Impact**: Backup exposure of `~/.config/nebulous` compromises the vault; weak `NEBULOUS_VAULT_MASTER_KEY` passphrases are fast to brute-force; plain `String` PATs persist on heap; passphrase-protected `~/.ssh` keys fail silently.
- **Suggested Direction**: Use Argon2id/scrypt for `NEBULOUS_VAULT_MASTER_KEY` derivation (and prefer storing `vault.key` in the OS keyring when enabled), wrap `FALLBACK_PAT_STORE` and `GitAuthMaterial::pat` in `Zeroizing<String>`, and warn/reject when stripping SSH key passphrases.

#### AUD-14 — Cross-Host HTTPS Redirect PAT Leakage via `libgit2` Default Redirects
- **Severity**: Medium
- **Focus Area(s)**: Security
- **Citations**: `crates/backend_core/src/git_sync/mod.rs:591-595, 980-1006`; `libgit2-sys-0.18.8+1.9.7/libgit2/src/libgit2/remote.c:875, 924`; `libgit2-sys-0.18.8+1.9.7/libgit2/src/libgit2/transports/http.c:221-269`
- **Rationale**: `url_allows_pat` (`git_sync/mod.rs:591-595`) checks only whether the callback URL starts with `https://`, without pinning the host to the configured remote URL. Inspection of the vendored `libgit2` source confirms `libgit2` defaults `follow_redirects` to `GIT_REMOTE_REDIRECT_INITIAL`, follows cross-host `301`/`302`/`303`/`307`/`308` redirects on the initial request (`stream->service->initial == 1`), updates `transport->server.url` to the redirected host, and passes that redirected URL to `callbacks.credentials`.
- **Impact**: A compromised or malicious remote returning a cross-host HTTPS redirect during initial `fetch`/`pull`/`push`/`sync` can capture the user's Git PAT.
- **Suggested Direction**: Pin `pat_for_url` to the configured remote host (or set `RemoteCallbacks` / `FetchOptions` redirect policy to disallow cross-host redirects).

#### AUD-15 — Pre-Release `ssh-key 0.7.0-rc.11` Splits RustCrypto Stack Into Dual Incompatible Versions
- **Severity**: Medium
- **Focus Area(s)**: Security, Sustainability, Supply Chain
- **Citations**: `crates/backend_core/Cargo.toml:10, 18, 21-22`; `Cargo.lock:2872-2887`
- **Rationale**: `backend_core` depends on `ssh-key = "0.7.0-rc.11"`, which pulls newer RustCrypto major versions (`sha2 0.11.1`, `aes-gcm 0.11.1`, `aead 0.6.1`, `aes 0.9.3`, `rand_core 0.10.1`) alongside `backend_core`'s direct `aes-gcm 0.10`, `sha2 0.10`, and `rand 0.8` dependencies (confirmed by `cargo deny` duplicate warnings).
- **Impact**: Doubles compiled cryptographic surface area, slows builds, bloats binary size, and risks API churn from a `-rc` dependency.
- **Suggested Direction**: Pin stable `ssh-key 0.6` or upgrade direct RustCrypto deps once `ssh-key 0.7.0` ships stably.

#### AUD-16 — CI/Release Supply-Chain Pinning & `cargo deny` Scope Gaps, Over-Broad `docs/` Packaging, and `install.sh` / Template Packaging Drift
- **Severity**: Medium
- **Focus Area(s)**: Security, Sustainability, Stability
- **Citations**: `.github/workflows/ci.yml:13-66`; `.github/workflows/release.yml:16-122`; `deny.toml:1, 21-42`; `mcpb/manifest.json:5`; `server.json:6, 14-15`; `STEPPED_IMPLEMENTATION_PLAN.md:13, 10043`; `scripts/bump-version.sh:79-115`; `scripts/install.sh:25-29, 107, 114, 129-136, 150, 160, 167`
- **Rationale**:
  1. All GitHub Actions in `ci.yml` and `release.yml` use mutable tags (`@v4`, `@v2`, `@stable`) instead of commit SHAs; `ci.yml` lacks `permissions: contents: read` and `--locked` on `cargo clippy`/`test`/`run`; and both workflows (`ci.yml:52`, `release.yml:52`) run `cargo deny check advisories bans`, leaving `[licenses]` and `[sources]` in `deny.toml:21-42` unenforced in CI/release.
  2. `release.yml:78` executes `cp -r docs "${STAGING_DIR}/"`, bundling internal repo docs (including this audit report) into public release tarballs; `release.yml` also builds only `x86_64-unknown-linux-gnu`.
  3. `mcpb/manifest.json` and `server.json` are `0.0.0` / all-zero-SHA-256 templates by design for pending Phase 25 `STEP 43C`/`43D` (`STEPPED_IMPLEMENTATION_PLAN.md:13, 10043`), not forgotten releases—but they are already untracked on disk (breaking `STEP 43C`'s `PRE_GATE` in `AUD-30`) and decoupled from `bump-version.sh` and `release.yml`.
  4. `bump-version.sh` mutates `Cargo.toml`/`Cargo.lock`/`CHANGELOG.md` before checking `release-check.sh` (`112`).
  5. `install.sh:25-29, 107` strips trailing `/` when `realpath -m` is absent, so `--prefix /` on macOS/BSD yields `PREFIX=""` (installing to the intended `/bin/neb`, but printing a blank `"Installing Nebulous CLI to ..."` at `160`), while on Linux `PREFIX="/"` produces `BINDIR="//bin"` (`114`), falsely failing the `"$PATH"` check (`167`); additionally, `NEBULOUS_BIN_DIR` (`129`) overrides the install source even when `SKIP_BUILD=0` builds `target/release/neb`.
- **Impact**: Supply-chain exposure to mutable action tags and unenforced license/source rules; silent lockfile drift in CI; internal docs shipped in release tarballs; broken `install.sh` diagnostics/PATH checks and stale/untracked MCPB templates.
- **Suggested Direction**: SHA-pin workflow actions, run `cargo deny check advisories bans licenses sources`, add `permissions: { contents: read }` and `--locked` to `ci.yml`, copy only user-facing docs (`docs/USAGE.md`, `docs/ARCHITECTURE.md`) in `release.yml`, complete or defer `STEP 43C`/`43D` cleanly, and fix `install.sh` `--prefix /` and `NEBULOUS_BIN_DIR` handling.

#### AUD-17 — Unsynchronized Public `TrackerStorage` Mutators, Lockless `read_active_timer` Quarantine Race, and Process-Local-Only `projects.toml` Lock
- **Severity**: Medium
- **Focus Area(s)**: Stability
- **Citations**: `crates/backend_core/src/storage/mod.rs:274, 747, 789, 819, 853, 872, 884, 898-903, 917-930, 933, 939, 1053`; `crates/backend_core/src/models/registry.rs:19-23, 382-479, 550-570`; `crates/backend_core/src/mcp/mod.rs:1786, 1804, 1846, 1871, 1952, 2010, 2041`; `crates/backend_core/src/git_sync/mod.rs:1888-1890`
- **Rationale**:
  1. `TrackerStorage` exposes `write_project`, `write_phase`, `write_scope`, `write_item_manifest`, `write_item_desc`, `write_item_detail`, `write_active_timer`, `clear_active_timer`, `init`, and `resolve_phase_and_scope` as `pub fn` without acquiring `with_write_lock`. Exhaustive workspace inspection confirmed unlocked writes in `mcp/mod.rs` (`tool_record_rolling_update`, `tool_pin_stable_release`, and `tool_record_verification_run` at `1952` writing `desc.md` after `record_component_verification` drops its lock) and `git_sync/mod.rs:1888-1890` (`git_commit_documented` calling `write_item_detail` and `sync_parent_indexes_for_item` unlocked), whereas callers in `workflow/lifecycle.rs`, `workflow/timer.rs`, `storage/id.rs`, `storage/reconcile.rs`, and `ingestion/mod.rs` are wrapped in `with_write_lock`.
  2. `read_active_timer` (`storage/mod.rs:917-930`) renames corrupt `.timer.toml` to `.timer.toml.bak` without `with_write_lock`, racing between concurrent readers (e.g., TUI poll + CLI).
  3. `models/registry.rs:19-23, 382-479` uses an in-process `Mutex` (`REGISTRY_LOCK`) with no cross-process `fd_lock` on `projects.toml`, and `load_registry_unlocked` (`566-568`) discards write errors (`let _ = save_registry_unlocked(...)`) after pruning.
- **Impact**: Concurrent CLI, MCP, and TUI processes can overwrite `projects.toml` or `.tracker/` manifests, or hit spurious `Io(NotFound)` errors when quarantining a corrupt timer.
- **Suggested Direction**: Acquire `with_write_lock` inside public `TrackerStorage::write_*` / `resolve_phase_and_scope` / `quarantine_active_timer` methods (keeping unlocked helpers `pub(crate)`) and add a cross-process `projects.lock` `fd_lock` in `models/registry.rs`.

#### AUD-18 — MCP Protocol & Tool Gaps: `git_fetch` `dry_run` Contradicts `CHANGELOG.md`, 23/24 Tools Lack `deny_unknown_fields`, EOF Frame Crash, Non-Atomic Multi-Writes, and Path Leaks
- **Severity**: Medium
- **Focus Area(s)**: Stability, Security, Scope Lapse
- **Citations**: `CHANGELOG.md:18`; `crates/backend_core/src/mcp/mod.rs:220, 829, 937, 968-2333, 1106-1109, 1214-1216, 1323-1343, 1459-1498, 1645-1650, 1674, 1778-1872, 1992-2047, 2164-2165, 2401-2406, 2550-2605`
- **Rationale**:
  1. `git_fetch` (`829, 2164-2165`) defaults `dry_run` to `false` while performing network I/O and updating remote-tracking refs, contradicting `CHANGELOG.md:18` ("`dry_run` (default true) on tools that write or use the network").
  2. Only `tool_plan_import` (`937`) sets `#[serde(deny_unknown_fields)]`; the other 23 tools silently ignore misspelled/camelCase parameters (e.g., `dryRun: false` silently runs `dry_run: true`).
  3. `read_next_jsonrpc_message` (`2550-2605`) exits the header loop on EOF without requiring the blank terminator line and hits `reader.read_exact(&mut buf)?`, crashing `serve_io` (`220`) with `UnexpectedEof` instead of returning a clean frame error or `Ok(None)`.
  4. `tool_upsert_component_manifest` (`1648`) and `tool_record_rolling_update` (`1832-1833`) silently accept/default invalid enum values; `list_items`, `list_crate_boundaries`, and `select_best_active_item` skip `resolve_phase_alias`; `tool_record_rolling_update` and `tool_pin_stable_release` perform non-atomic multi-step disk writes; `tool_create_item` and `tool_upsert_component_manifest` return absolute host paths; `tool_switch_project` mutates machine-wide `projects.toml`.
- **Impact**: Unexpected network operations on `git_fetch`, silent agent parameter no-ops, MCP server crashes on truncated stdio frames, partial writes on error, and cross-session `switch_project` interference.
- **Suggested Direction**: Align `git_fetch` `dry_run` default with `CHANGELOG.md:18` (or update `CHANGELOG.md`), add `deny_unknown_fields` and strict enum/phase-alias validation across all MCP tools, check for the header terminator in `read_next_jsonrpc_message`, return workspace-relative paths, and isolate `switch_project` to the session unless persistence is requested.

#### AUD-19 — `arboard` Blocks Indefinitely on Stalled X11 Sockets (`neb start --copy`, `neb context --copy`, TUI `s`/`c`)
- **Severity**: Medium
- **Focus Area(s)**: Stability
- **Citations**: `crates/nebulous_cli/src/clipboard.rs:20`; `crates/nebulous_cli/src/args.rs:167-175`; `crates/nebulous_cli/src/tui/state.rs:1204, 1235`
- **Rationale**: `copy_text` (`clipboard.rs:20`) calls `arboard::Clipboard::new().and_then(|mut cb| cb.set_text(...))` synchronously on the main thread before falling back to OSC 52. While closed X11 ports or missing Wayland sockets fail fast (`0.02s`) and trigger OSC 52, testing `neb --json context TASK-0103 --copy` against a silent/stalled X11 TCP socket (`DISPLAY=127.0.0.1:<port>`) hangs past a 3-second timeout without returning.
- **Impact**: Because `neb start` defaults `--copy=true` (`args.rs:169`), any user or agent running `neb start`, `neb context --copy`, or TUI `s`/`c` (`state.rs:1204, 1235`) under stale SSH X11 forwarding or an unresponsive X server freezes indefinitely.
- **Suggested Direction**: Wrap `arboard` calls in a bounded worker-thread timeout (or prefer OSC 52 / skip blocking X11 handshake when a socket probe times out).

#### AUD-20 — `TrackerWatcher::drop` Blocks up to 30s on Contended Locks, and `hash_index_files` Follows Directory Symlinks Without Cycle Guards
- **Severity**: Medium
- **Focus Area(s)**: Stability
- **Citations**: `crates/backend_core/src/watcher/mod.rs:137-149, 170-174, 224-231, 332-375`; `crates/backend_core/tests/watcher_integration.rs:1-63`
- **Rationale**: `TrackerWatcher::drop` (`224-231`) joins the worker thread unconditionally with no shutdown `AtomicBool` checked during the 300ms settlement polling loop (`138-149`) or `reconcile_indexes()` (`170`). Meanwhile, `hash_index_files` (`332-351`) recurses using `path.is_dir()`, following symlinks without cycle/containment checks and storing non-canonical paths that miss `is_own_index_echo` (`368`). `watcher_integration.rs` tests dropping an idle watcher, not under lock contention.
- **Impact**: TUI/process shutdown can freeze up to 30s on contended `.tracker/.lock`, and a symlink loop inside `.tracker/` can hang/OOM the watcher thread.
- **Suggested Direction**: Add an `Arc<AtomicBool>` shutdown signal checked across settlement/reconcile loops and skip symlinks via `entry.file_type()` in `hash_index_files`.

#### AUD-21 — `reconcile_indexes` and `compute_metrics_at` Abort Globally on One Corrupt/Future-Schema File
- **Severity**: Medium
- **Focus Area(s)**: Stability
- **Citations**: `crates/backend_core/src/storage/reconcile.rs:64, 87, 115, 153, 279-290`; `crates/backend_core/src/workflow/metrics.rs:132-136, 147, 223-227, 237-238, 277-281`; `crates/backend_core/src/workflow/assist.rs:387-391`
- **Rationale**: `reconcile_indexes` gracefully skips/reports corrupt or newer-schema Tier 4 `item.toml` files (`279-290`), yet propagates `?` on `read_phase_from_path` and `read_scope_from_path` (`64, 87, 115, 153`). Likewise, `compute_metrics_at` (`metrics.rs:132-136, 147`) aborts globally via `?` on a single unreadable `item.toml` or bad `updated_at` (unlike `compute_whats_next_at` in `assist.rs:387-399`). `metrics.rs:223-227, 237-238` also sets `daily_pace_ratio` to the exact same value as `target_days_equivalent` and can emit `f64::INFINITY` (JSON `null`) on tiny `target_daily_hours`.
- **Impact**: One damaged or future-schema `phase.toml`/`scope.toml`/`item.toml` breaks watcher auto-reconciliation, `neb sync-index`, and `neb metrics` across the whole project.
- **Suggested Direction**: Extend skip-and-report handling to `phase.toml`/`scope.toml` in `reconcile_indexes` and to unreadable/bad-date items in `compute_metrics_at`; guard `f64::is_finite()` on pace metrics and fix/deprecate `daily_pace_ratio`.

#### AUD-22 — `compat::unknown_keys` Re-Injects Explicit Default Fields in `O(K*N)` and `write_versioned` Downgrades `schema_version`
- **Severity**: Medium
- **Focus Area(s)**: Stability, Sustainability
- **Citations**: `crates/backend_core/src/storage/compat.rs:76-94, 148-165`; `crates/backend_core/src/storage/mod.rs:682-717, 751, 797, 832, 861`
- **Rationale**: `compat::unknown_keys` (`76-94`) probes candidate keys by comparing against `known = roundtrip(old)?`. Because model fields use `#[serde(default, skip_serializing_if = "...")]` (`id_scheme = "sequential"`, `task = 0`, `tasks = []`, etc.), explicit default keys on disk are absent from `known`, yield identical `roundtrip(&probe_without)` results, and are misclassified as "unknown" and re-injected forever (at `1 + K` full TOML parse/serialize cycles). Furthermore, `read_versioned` allows schema v2 across all 4 tiers while `write_project`/`write_phase`/`write_scope` hardcode `version = 1`, silently downgrading any v2 Tier 1–3 manifest on write; `compat::migrate` (`160-165`) is an uncalled no-op stub.
- **Impact**: Hand-edited default keys persist redundantly across writes with unnecessary TOML roundtrip cost, and future v2 Tier 1–3 manifests would be downgraded to v1 on write.
- **Suggested Direction**: Track known field names or `#[serde(flatten)]` extra maps directly instead of `skip_serializing_if`-sensitive probing, define per-tier `MAX_*_SCHEMA_VERSION` constants, and remove or mark `compat::migrate` until migrations exist.

#### AUD-23 — `planning/mod.rs` & `planning/import.rs` Parser Fragility and Cross-Phase `source_ref` Hijack/Collision
- **Severity**: Medium
- **Focus Area(s)**: Stability, Scope Lapse
- **Citations**: `crates/backend_core/src/planning/mod.rs:52-77, 256-315`; `crates/backend_core/src/planning/import.rs:98-122`
- **Rationale**: `parse_roadmap` (`planning/mod.rs:71`) parses any `|`-prefixed line inside `### Up Next` / `### Completed` without tracking fenced code blocks; `split_table_row` (`259, 270`) fails to strip the trailing pipe on row-ending `\\|` (`!inner.ends_with(r"\|")`) while middle-cell `Demo\\|` silently retains the backslash (`Ok("Demo\\|")`); `parse_target_files` (`303`) splits strictly on `", "`; and `import_plan` (`import.rs:98-122`) builds `by_source_ref` globally across *all* phases. When Phase 1 already holds `STEP 1` and Phase 2 reuses `STEP 1`, `import_plan` first silently hijacks/retitles Phase 1's item instead of creating a Phase 2 item, and errors with `Duplicate source_ref` once two items in different phases share `STEP 1`.
- **Impact**: Valid Markdown examples inside plan sections break `neb plan import`, escaped pipes corrupt cells, and multi-phase plans resetting step numbers per phase silently retitle items or fail import.
- **Suggested Direction**: Track code-fence state in `parse_roadmap`, fix escaped-backslash handling in `split_table_row`, and scope `by_source_ref` in `import_plan` to the target phase (or key by `(phase, source_ref)`).

#### AUD-24 — CLI Correctness Bugs & Silent Error Swallowing (`handle_sync`, `handle_start`, `handle_move`, `handle_sync_index`, `handle_init`, `ComponentCommand::Rollback`)
- **Severity**: Medium
- **Focus Area(s)**: Stability, Sustainability
- **Citations**: `crates/nebulous_cli/src/commands/mod.rs:593-612, 1016, 1066-1072, 1196-1199, 1820-1844, 1891-1899, 2802-2808`; `crates/backend_core/src/models/project.rs:81-88`
- **Rationale**:
  1. `handle_sync` (`1826-1837`) returns the pre-sync `outcome.commit_sha` and discards `sync_outcome.sha` returned by `git_sync_with_report`.
  2. `handle_start` (`1016, 1066-1072`) calls `std::env::current_dir()` instead of `resolve_context_cwd(&storage, cli.root.as_deref())` (ignoring `--root` scope inference) and omits `warn_if_capped(...)`.
  3. `handle_move` (`1196-1199`) interprets `neb move BUG-0001` (missing stage) as moving the active item to phase `"BUG-0001"`.
  4. `handle_sync_index` (`1892-1898`) omits `counters.task` in human output.
  5. `handle_init` (`594-612`) silently swallows `ingest_project_directory` and `register_project` errors without warning; `ComponentCommand::Rollback` (`2803`) silently ignores `record_regression` errors even when `--item` was explicitly provided.
- **Impact**: Stale SHAs in `neb sync --json`, broken `--root` scope inference on `neb start`, omitted `task` counter output, confusing `neb move` errors, and silent failures hiding real errors.
- **Suggested Direction**: Return `sync_outcome.sha.or(Some(outcome.commit_sha))` in `handle_sync`, wire `resolve_context_cwd` and `warn_if_capped` in `handle_start`, print `task={}` in `handle_sync_index`, guard single-arg `handle_move` against item-ID-like tokens, and warn/propagate errors in `handle_init` and explicit `--item` `Rollback`.

#### AUD-25 — TUI Stability, UX, and Performance Issues (`tui/state.rs`, `tui/render.rs`, `tui/mod.rs`)
- **Severity**: Medium
- **Focus Area(s)**: Stability, Sustainability
- **Citations**: `crates/nebulous_cli/src/tui/state.rs:412-416, 634, 646, 789-797, 885-892, 903-905, 928-939, 970-981, 1106-1127, 1143, 1173, 1197-1203, 1234-1246, 1277, 1284-1291`; `crates/nebulous_cli/src/tui/render.rs:172-186, 607, 673-706, 924-937`; `crates/nebulous_cli/src/tui/mod.rs:87-96`
- **Rationale**:
  1. `TuiApp` hardcodes `CANONICAL_PHASES` across stage headers, keybindings, `CreateModal`, and `MoveModal`, making custom phases added via `neb phase add` inaccessible in the TUI.
  2. `rebuild_view()` (`412-416`) reads `item.toml` for every item in the stage on each `/` filter keystroke, and moving the Scopes cursor (`1143, 1173`) invokes full `self.refresh()` (reloading `TrackerIndex` + running `git2` status).
  3. `submit_active_modal` discards `create_phase`/`create_scope` errors (`634, 646`), `action_start_work` maps every error (including lock timeout/corrupt TOML) to `"No open or selected item to start."` (`1197-1203`), and `action_copy_context` (`1234-1246`) silently ignores errors.
  4. `CreateModal` submits on `Enter` even inside `Description` (`903-905`) and renders it single-line; `action_jump_whats_next` (`1285-1291`) fails silently when a filter hides the item; `tui/mod.rs:87-91` leaks a stacked global panic hook; `state.rs:1277` uses `.unwrap()` in production code.
- **Impact**: Laggy navigation on larger repos, unreachable custom phases, hidden error causes in TUI modals/actions, and panic-hook stacking.
- **Suggested Direction**: Cache `recurrence_count` instead of reading `item.toml` per filter keystroke, call `rebuild_view()` on scope cursor movement, support dynamic `project.phases.registry` phases in TUI, surface actual errors in `status_message`, replace `.unwrap()` at `state.rs:1277`, and restore the panic hook on `run_tui` exit.

#### AUD-26 — TUI Hash-ID Column Clipping (`Constraint::Length(9)`) and `CloseModal` Unicode Char-Count Truncation
- **Severity**: Medium
- **Focus Area(s)**: Stability
- **Citations**: `crates/nebulous_cli/src/tui/render.rs:403-409, 792-800`
- **Rationale**:
  1. `render_items_pane` (`403-409`) hardcodes the `ID` table column to `Constraint::Length(9)`. Sequential IDs (`BUG-0001` = 8 chars, `FEAT-0001`/`TASK-0001` = 9 chars) fit, but opt-in hash IDs (`BUG-7F3A2C` = 10 chars, `FEAT-7F3A2C`/`TASK-7F3A2C` = 11 chars) are silently truncated (e.g., `FEAT-7F3A`).
  2. In `CloseModal` (`792-800`), when `!close.summary_edited`, the pre-filled default summary is sliced via `close.summary.chars().take(max_sum_chars)` rather than terminal display width, allowing double-width Unicode item titles to overflow the input box.
- **Impact**: Projects using `id_scheme = "hash"` lose the trailing hex characters needed to distinguish items in the TUI, and wide Unicode titles can corrupt `CloseModal` layout.
- **Suggested Direction**: Size the `ID` column dynamically from the longest displayed ID (minimum `11` when hash IDs are present) and truncate unedited `CloseModal` previews by Unicode display width (`unicode-width`).

#### AUD-27 — `assist.rs` Redundant I/O / Git Walks & Fallback Quirks, `timer.rs` `u32` Truncation, and `lifecycle.rs` Double Timer Call
- **Severity**: Medium
- **Focus Area(s)**: Sustainability, Stability
- **Citations**: `crates/backend_core/src/workflow/assist.rs:287-301, 718-723, 727-748, 861-912`; `crates/backend_core/src/models/timer.rs:54-59`; `crates/backend_core/src/workflow/timer.rs:98-101, 193-195`; `crates/backend_core/src/workflow/lifecycle.rs:278-279, 364-366`
- **Rationale**: `predict_defaults` (`assist.rs:287-301`) loads `TrackerIndex` twice (via `compute_whats_next` and `resolve_active_or_inferred_item_id`) and runs `repo.statuses` with `recurse_untracked_dirs(true)` every time. `count_phase_items` (`718-723`) can emit `AdvanceStage` on an empty phase if stale `phase.toml` summary counts remain; `suggest_next_stage` (`727-748`) defaults to `"production"` at the end of custom phase lists. `ActiveTimer::elapsed_minutes_at` (`models/timer.rs:54-59`) and `timer_status_at` (`workflow/timer.rs:193-195`) cast `(seconds / 60) as u32` instead of saturating. `start_work_item` (`lifecycle.rs:278-279`) calls `switch_or_start_timer` twice (once inside `update_item_status` at `365` with `None`, and again at `279` with `note`), writing `.timer.toml` twice back-to-back when `note` is `Some(...)` and performing a redundant read/locate when `note` is `None` (`timer.rs:98-101`).
- **Impact**: Extra disk/git latency on large repos, minor edge-case misreporting on custom/empty phases or extreme timestamps, and redundant timer reads/writes on `neb start`.
- **Suggested Direction**: Share the loaded `TrackerIndex` inside `predict_defaults`, saturate `u32::try_from(...).unwrap_or(u32::MAX)` on elapsed minutes, avoid the duplicate `switch_or_start_timer` call in `start_work_item`, and return `None` from `suggest_next_stage` after the final custom phase.

#### AUD-28 — Monolithic `commands/mod.rs` (3,146 LOC), Overlapping Commands, and Silent Dead `--use` Flag
- **Severity**: Medium
- **Focus Area(s)**: Sustainability
- **Citations**: `crates/nebulous_cli/src/commands/mod.rs:662, 759, 1286, 2273`; `crates/nebulous_cli/src/args.rs:200-203, 301-323, 419-446, 523-524, 720-733`; `crates/nebulous_cli/tests/cli_integration.rs:2459-2467`
- **Rationale**: `commands/mod.rs` concentrates 28 subcommand handlers, DTOs, formatting, and rollback logic in 3,146 lines (~45% of `nebulous_cli/src`), with three `#[allow(clippy::too_many_arguments)]` suppressions. `Commands::Sync` accepts `--item`/`--path` while `GitCommand::Sync` omits both; `Commands::Stage` overlaps `PhaseCommand`; `ProjectCommand::Add` defines hidden `--use`/`--active` (`use_compat: _`) whose value is completely ignored (`neb project add <dir> --no-use --use` silently leaves `--no-use` active).
- **Impact**: High maintenance friction for a solo developer, diverging alias behavior, and misleading flag parsing.
- **Suggested Direction**: Split `commands/mod.rs` into domain submodules, reconcile `sync` / `git sync` and `stage` / `phase`, and either wire `use_compat` via clap `overrides_with("no_use")` or remove it.

#### AUD-29 — Widespread Documentation Drift & Internal Contradictions (`README.md`, `docs/USAGE.md`, `docs/ARCHITECTURE.md`, `CHANGELOG.md`)
- **Severity**: Medium
- **Focus Area(s)**: Sustainability
- **Citations**: `README.md:27, 29, 138, 141, 145`; `docs/USAGE.md:10-208, 234, 241, 246`; `docs/ARCHITECTURE.md:13-22, 49-79, 94-118`; `CHANGELOG.md:6, 18`; `crates/nebulous_cli/src/args.rs:28-391, 138`; `crates/nebulous_cli/src/tui/state.rs:484-495`; `crates/nebulous_cli/src/tui/render.rs:530-555`
- **Rationale**:
  1. `README.md:138` says TUI `1`/`2`/`3` *"Switch active stage"*, contradicting `docs/USAGE.md:241` (*"Change the viewed stage ... only"*) and `tui/state.rs:484-495` (code matches `USAGE.md`).
  2. `README.md` omits `t` (`New Task`) and `r` (`Reload`); `docs/USAGE.md:234` (the modal accessibility prose paragraph) omits `New Task (t)` even though the shortcuts table at `docs/USAGE.md:246` documents `t`; `tui/render.rs:530-555` omits `[r] Reload` in the footer legend.
  3. Out of 28 `Commands` variants in `args.rs`, `docs/USAGE.md` omits `list`, `status`, `reopen`, `timer`, `log`, `metrics`, `phase`, `scope`, and `sync-index` (only mentioned in passing), omits `neb task` from scope inference (`46`), and `args.rs:138` omits `task`/`chore` in `List --type` help text.
  4. `docs/ARCHITECTURE.md` omits `planning`, `clipboard`, `storage::compat`, `TASK-XXXX`, `id_scheme = "hash"`, `.tracker/.lock`, `.tracker/.gitignore`, and `.timer.toml.bak`; `README.md:27, 29` omits `TASK-XXXX` and `hash` IDs; `CHANGELOG.md:6` `[Unreleased]` is empty despite committed `STEP 41A–43B` work and `CHANGELOG.md:18` contradicts `git_fetch` `dry_run: false`.
- **Impact**: Contradictory user/agent guidance and undocumented core commands.
- **Suggested Direction**: Reconcile `README.md`, `docs/USAGE.md`, `docs/ARCHITECTURE.md`, `args.rs` help text, and `CHANGELOG.md` against current code.

#### AUD-30 — Workflow & Plan Drift: 994 KB `STEPPED_IMPLEMENTATION_PLAN.md`, `.orrery/` vs `.phased/` Split, and `.gitignore` Contradiction
- **Severity**: Medium
- **Focus Area(s)**: Sustainability, Scope Lapse
- **Citations**: `STEPPED_IMPLEMENTATION_PLAN.md:1-6, 11, 17, 23-31, 240-315, 3249-5040, 10043`; `.orrery/project.toml:2, 9, 24`; `.orrery/mcp/servers.toml:2, 6`; `.orrery/roles/{orchestrator,validator}.toml:6`; `.orrery/skills/repo-rules.md:1-3`; `.phased/plan-header.md:3`; `.gitignore:19`; untracked `mcpb/manifest.json`, `server.json`
- **Rationale**: `STEPPED_IMPLEMENTATION_PLAN.md` is 994 KB (10,386 lines) with a stale Phase 4 title (`1`), ~1,790 lines of withdrawn Phase 9 `neb release` specs (`3249-5040`), and external `/srv/devshare/NovOS` paths (`11, 3251, 3270`). Tracked `.orrery/*` files configure `hybrid-workflow` while untracked `.phased/plan-header.md` and `scripts/phased-step.sh` configure `phased-workflow`. `.orrery/skills/repo-rules.md:1-3` references `/workspace/` and `"release tracker"`. `.gitignore:19` ignores `STEPPED_IMPLEMENTATION_PLAN.md` even though `.orrery/project.toml:2` treats it as canonical, and `STEP 43C` `PRE_GATE` (`10043`) requires `mcpb/manifest.json` and `server.json` not to exist while both are already untracked on disk.
- **Impact**: High cognitive overhead and broken automation preconditions (`STEP 43C` `PRE_GATE` fails; `.orrery/` and `.phased/` disagree).
- **Suggested Direction**: Archive completed/withdrawn phases out of `STEPPED_IMPLEMENTATION_PLAN.md`, align `.orrery/`, `.phased/`, and `.gitignore` on a single workflow, and clean up `/workspace/` and `release tracker` references in `.orrery/skills/repo-rules.md`.

---

### Low Severity

#### AUD-31 — Minor CLI Hygiene & Input Validation (`NEB_GIT_PAT` Buffer, `--json` Warnings, Case-Sensitive Timer IDs, Invalid `--type`, Wayland Feature)
- **Severity**: Low
- **Focus Area(s)**: Security, Stability, Sustainability
- **Citations**: `crates/nebulous_cli/src/commands/mod.rs:985-1006, 1458-1580, 1485, 1527, 1775, 1785-1808, 2617-2621`; `crates/nebulous_cli/src/main.rs:17-21`; `crates/nebulous_cli/Cargo.toml:18`
- **Rationale**: `cli_git_auth` (`commands/mod.rs:1775`) reads `NEB_GIT_PAT` into a `String` and `read_pat_input` (`1802-1807`) allocates a trimmed copy before zeroizing the initial buffer; `warn_if_capped` and `note_quarantined_timer` print prose to `stderr` under `--json` (and `note_quarantined_timer` is omitted from `TimerCommand::Start`/`Stop`); `TimerCommand::Stop` (`1485`) and `TimerCommand::Status` (`1527`) compare `item_id` case-sensitively (`neb timer stop bug-0001` fails against `BUG-0001`, unlike `close_item_reporting`); `ComponentCommand::Init --type` (`2617-2621`) uses `.unwrap_or_else(|_| raw.to_string())`, silently accepting arbitrary typos (CLI counterpart to `AUD-18`); `main.rs:20` formats unescaped `{err}` in its JSON fallback; `nebulous_cli/Cargo.toml:18` enables `arboard`'s `wayland-data-control` unconditionally on non-Linux targets.
- **Impact**: Minor secret-residency gap for env-var PATs, noisy `--json` stderr, case-sensitive timer rejections, silent invalid component types, and unnecessary Linux-specific feature activation on macOS/Windows.
- **Suggested Direction**: Zeroize `env_pat` in place, normalize `item_id` case in `TimerCommand::Stop`/`Status`, reject invalid `ComponentType` values in `ComponentCommand::Init`, gate `wayland-data-control` under `[target.'cfg(target_os = "linux")'.dependencies]`, and structure/suppress warnings under `--json`.

#### AUD-32 — Minor Robustness & Path-Normalization Hygiene (`git_sync`, `ingestion`, `registry.rs`, `.gitignore`)
- **Severity**: Low
- **Focus Area(s)**: Stability, Sustainability
- **Citations**: `crates/backend_core/src/models/registry.rs:179-182, 290-294, 581-593`; `crates/backend_core/src/storage/mod.rs:473, 1525-1536`; `crates/backend_core/src/workflow/lifecycle.rs:824`; `crates/backend_core/src/git_sync/mod.rs:617-619, 1072-1079`; `crates/backend_core/src/ingestion/mod.rs:196, 338-340, 732, 884-893, 922`; `.gitignore:8, 19`; `LICENSE:1-22`
- **Rationale**:
  1. `registry.rs:293` matches existing entries by raw equality instead of `normalize_query_path_str`, risking duplicate `projects.toml` entries on legacy/Windows paths, and duplicates UNC/trailing-slash normalization with `storage/mod.rs:1525-1536`.
  2. `is_temp_atomic_path` (`git_sync/mod.rs:618`) filters any path containing `.tmp.` (hiding workspace files like `foo.tmp.rs` from `git_status`/`stage_tracker_dir`); `fetch_repo_with_auth` (`1072-1079`) retries with a fresh `attempts = 0` auth callback on any error (doubling retries from 5 to 10 even on `401`).
  3. `ingest_project_directory` (`ingestion/mod.rs:196`) checks `path.is_file()` (following symlinks); lines `338-340` discard TOML parse errors on `nebulous.component.toml`, allowing `write_manifests: true` to overwrite a broken user manifest; and `promote_component_to_scope` (`732`) calls raw `fs::create_dir_all(self.items_dir(...))` instead of `self.ensure_items_dir(...)` (`storage/mod.rs:473`, `create_tracker_dir_all` containment guard, inconsistent with `workflow/lifecycle.rs:824`).
  4. `.gitignore:8` still lists legacy `/.tracker/.keyring_fallback`. (`LICENSE` cleanly matches `Cargo.toml:12` MIT.)
- **Impact**: Minor risk of duplicate registry entries, hidden `.tmp.` workspace files in git operations, unnecessary auth retries, bypassed directory containment guard on promotion, and overwritten malformed `nebulous.component.toml`.
- **Suggested Direction**: Share path normalization in `registry.rs`/`storage/mod.rs`, match atomic temp suffixes narrowly (`.tmp.<digits>.<digits>`), skip auth retries on invalid-credentials errors, use `entry.file_type()`, `ensure_items_dir`, and surfaced manifest parse errors in ingestion, and clean up `.gitignore`.

#### AUD-33 — Wildcard Crate-Root Re-Exports (`lib.rs`) and Test-Suite Hazards / Coverage Gaps
- **Severity**: Low
- **Focus Area(s)**: Sustainability, Test Hygiene
- **Citations**: `crates/backend_core/src/lib.rs:13-21, 29-886`; `crates/backend_core/tests/storage_integration.rs:14-40, 1458, 1478`; `crates/backend_core/tests/path_containment.rs:14-37`; `crates/backend_core/src/storage/atomic.rs:194-203`; `crates/backend_core/tests/workflow_integration.rs:736-823, 1494-1652`; `crates/backend_core/tests/watcher_integration.rs:1-63`
- **Rationale**: `backend_core/src/lib.rs:13-21` exports 8 modules via `pub use <mod>::*;` and mixes 858 lines of unit tests into `lib.rs`. `storage_integration.rs:1458, 1478` mutates `NEBULOUS_CONFIG_DIR` via `std::env::set_var` / `remove_var` in parallel tests. Three hand-rolled temp-dir helpers duplicate `tempfile::TempDir`. Integration tests miss `move_item_to_phase` on `ItemType::Task`, symlinked `.tracker/` roots, `TrackerWatcher::drop` under `.lock` contention, and malformed `phase.toml`/`scope.toml` in `reconcile_indexes`.
- **Impact**: Parallel test flakiness risk (and UB under Rust 2024 rules), noisy crate-root API surface, and regression blind spots that masked AUD-03, AUD-05, AUD-20, and AUD-21.
- **Suggested Direction**: Replace wildcard re-exports with explicit exports, pass config dirs via `*_in(&cfg_dir)` helpers in `storage_integration.rs:1458`, standardize on `tempfile::TempDir`, and add the four missing regression tests.

---

## Unverified Hypotheses & Observations

The following leads were identified during manual inspection but **not verified** by execution or cross-platform testing; they are intentionally separated from Verified Findings above (Iteration 1 leads `#1`, `#2`, `#4`, `#5`, and `#6` were resolved and promoted or folded into `AUD-01`, `AUD-14`, `AUD-17`, `AUD-19`, and Verified Sound Surfaces):

1. **Windows `icacls` Behavior on Localized / Non-Domain Accounts & Keyring Size Limits** (`crates/backend_core/src/crypto/mod.rs:394-410, 615-652`): On Windows, `restrict_to_current_user` formats `%USERDOMAIN%\%USERNAME%` for `icacls`, which may behave differently on AzureAD/Microsoft accounts or localized Windows builds; OS keyring entry size limits (e.g., Windows `CRED_MAX_CREDENTIAL_BLOB_SIZE` ~2.5 KiB) may also reject large OpenSSH PEM keys and trigger AUD-04 split-brain. Unverified because the audit host runs Linux.
2. **Untested `nebulous_cli` Wrapper Paths** (`crates/nebulous_cli/tests/cli_integration.rs`): End-to-end CLI tests do not exercise `neb component promote`, `neb git fetch`/`pull`/`push`, `neb sync --item ... --path ...` (without `--commit-only`), or `neb done -c` post-close commit-failure rollback.
3. **Intent of Untracked Local Phased-Workflow Files** (`.phased/plan-header.md`, `scripts/{phased-step.sh,neon-dispatch}`): While `mcpb/manifest.json` and `server.json` are explicitly defined as Phase 25 `STEP 43C` templates (`STEPPED_IMPLEMENTATION_PLAN.md:13, 23, 10043`), whether `.phased/plan-header.md` and `scripts/{phased-step.sh,neon-dispatch}` represent an active local workflow transition or abandoned WIP could not be determined from repo files alone.

---

## Scope & Verification Coverage Summary

- **In-Scope Areas Covered**:
  - `crates/backend_core/` (all modules: `storage/`, `models/`, `watcher/`, `workflow/`, `crypto/`, `git_sync/`, `mcp/`, `ingestion/`, `planning/`, `agent_context/`, `error.rs`, `lib.rs`, `Cargo.toml`, and all 9 integration test suites).
  - `crates/nebulous_cli/` (all modules: `main.rs`, `lib.rs`, `args.rs`, `clipboard.rs`, `commands/mod.rs`, `tui/`, `Cargo.toml`, and all 3 integration test suites).
  - `scripts/` (all 7 scripts: `bump-version.sh`, `check-neutral.sh`, `install.sh`, `neon-dispatch`, `phased-step.sh`, `release-check.sh`, `validate-step.sh`).
  - `docs/` & root metadata (`docs/ARCHITECTURE.md`, `docs/USAGE.md`, `README.md`, `CHANGELOG.md`, `STEPPED_IMPLEMENTATION_PLAN.md` active/structural sections, `Cargo.toml`, `Cargo.lock`, `deny.toml`, `.gitignore`, `LICENSE`).
  - CI, packaging & workflow configs (`.github/workflows/{ci,release}.yml`, `mcpb/manifest.json`, `server.json`, `.orrery/*`, `.phased/plan-header.md`).
- **Excluded Areas**: `archive/`, `target/`, `.tracker/` (verified excluded from audit findings).
- **Verified Sound Surfaces**: OSC-52 base64 escaping (`crates/nebulous_cli/src/clipboard.rs:14-36`), TUI narrow/zero viewport bounds (`crates/nebulous_cli/src/tui/render.rs:60-63, 107-109, 593-602`), `ComponentCommand::Init` relative-path safety (`crates/backend_core/src/ingestion/mod.rs:607`), CLI `--json` dispatch parity (`crates/nebulous_cli/src/commands/mod.rs:54-326`), `git_sync` PAT HTTPS scheme / URL userinfo redaction on non-redirected URLs (`crates/backend_core/tests/credential_hygiene.rs`), fast-forward `git_pull` index preservation alongside untracked `.tracker/.lock` and staged changes (`crates/backend_core/src/git_sync/mod.rs:1208-1223`), and `git2 0.21.0` non-reachability of `Url::origin()` (`RUSTSEC-2026-0592`).
