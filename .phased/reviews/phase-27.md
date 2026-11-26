# Phase 27 review

Range: `2e1f1808f7df11e7087c837a50c3c1ac7e0be045..8a611747e3cb5ec83c841e38b181e214ed21310a`

## Independent pass

### Findings

1. **`crates/backend_core/src/models/registry.rs:60-96,453-460,584-594` — Read-only registry lookups open `projects.lock` for write and take an exclusive flock (`regression`)**
   - **Trigger:** Calling `load_global_registry_from(&dir)` or `credential_accounts_for_root_in(&dir, root)` when `dir` exists and is read-only (e.g., `chmod 555` or a read-only config mount), or when `dir` exists without a `projects.toml`.
   - **Effect:** `with_registry_lock` always opens `projects.lock` with `OpenOptions::new().create(true).truncate(false).write(true)` and acquires an exclusive `fd_lock::RwLock::try_write()` lock. On a read-only config directory this fails with `TrackerError::Io(PermissionDenied)` where reads previously succeeded (previously only the in-process `Mutex` was taken). On a writable directory with no `projects.toml`, a read call now creates an empty `projects.lock` file on disk. Additionally, `with_registry_lock` holds the process-global `REGISTRY_LOCK` mutex across the `try_write()` sleep-poll loop (up to 30s `DEFAULT_LOCK_TIMEOUT`), blocking unrelated registry paths in the same process behind a contended lock.

2. **`crates/backend_core/src/models/registry.rs:628-630` — `load_registry_unlocked` now fails reads if persisting unsafe-project pruning fails (`regression`)**
   - **Trigger:** Calling `load_global_registry_from` on a read-only `projects.toml` (or read-only parent directory) that contains an unsafe broad-directory project entry.
   - **Effect:** Changing `let _ = save_registry_unlocked(path, &registry);` to `save_registry_unlocked(path, &registry)?;` causes a read call to fail with `TrackerError::Io` instead of returning the pruned in-memory registry when the file cannot be rewritten.

3. **`crates/nebulous_cli/src/commands/mod.rs:2687-2733` & `crates/backend_core/src/mcp/mod.rs:1630-1671,1755-1806` — Sibling component read-modify-write paths lack outer `with_write_lock` (`old bug`)**
   - **Trigger:** Concurrent invocations of `neb component pin --dep ...` (`ComponentCommand::Pin`), MCP `upsert_component_manifest`, or MCP `record_rolling_update` against the same component manifest.
   - **Effect:** `McpServer::tool_pin_stable_release` (`crates/backend_core/src/mcp/mod.rs:1990-2048`) was wrapped in `storage.with_write_lock(...)` across its `read_component_manifest` -> mutate -> `write_component_manifest` -> `pin_component_release` sequence, but its CLI twin `ComponentCommand::Pin` (`crates/nebulous_cli/src/commands/mod.rs:2687-2733`) still executes the identical read-modify-write + pin sequence without an outer write lock, and MCP `tool_upsert_component_manifest` / `tool_record_rolling_update` still perform unlocked read-modify-write cycles on component manifests and item descriptions, allowing lost updates under concurrency.

### Traced and verified clean

- `crates/backend_core/src/workflow/lifecycle.rs:970-976`: `remove_scope_stub` now delegates to `scope_manifest.items.remove_id(item_id)` (covering tasks as well as bugs/features); grepped `.items.bugs` and `.items.features` across `crates/` and confirmed no sibling manual retain sites remain.
- `crates/backend_core/src/storage/mod.rs`: `discover` and `guard_tracker_path` reject symlinked `.tracker` roots while allowing real `.tracker` under symlinked workspace ancestors; `read_tracker_file` and `with_write_lock_timeout` add `libc::O_NOFOLLOW` on Unix; `lock_key` canonicalizes existing prefix so mixed path spellings share `HELD_WRITE_LOCKS` and `PROJECT_CACHE` entries cleanly and `ReentrantLockGuard::drop` clears both; public mutators (`init`, `ensure_items_dir`, `write_project`, `write_phase`, `write_scope`, `write_item_manifest`, `write_item_desc`, `write_item_detail`, `read_active_timer` quarantine path, `write_active_timer`, `clear_active_timer`, `resolve_phase_and_scope`) now serialize via `with_write_lock`.
- `crates/backend_core/src/watcher/mod.rs`: `shutdown: Arc<AtomicBool>` and 50ms lock-slice polling allow `Drop` to stop promptly under a contended write lock; `hash_index_files` skips symlinked files and directories via `entry.file_type().is_symlink()`, preventing directory symlink cycle hangs.
- `scripts/check-neutral.sh`: excludes `STEPPED_IMPLEMENTATION_PLAN.md`, `docs/codebase-audit-report.md`, and `.tracker/` in no-arg mode; passes (`check-neutral: OK`).
- Sensitive data scan across all changed files: none found.
- New tests in `path_containment.rs`, `storage_integration.rs`, `watcher_integration.rs`, `workflow_integration.rs`, and `watcher/mod.rs`: assertions verify the real paths and behaviors (minor timing sensitivity noted in `drop_stops_promptly_under_contended_write_lock` if OS notify delivery exceeds 250ms, though not a false-failure risk).

### Not checked

- Did not compile or execute `cargo test --workspace` / `cargo clippy` during the independent pass (static diff and code trace only).
- Did not test Windows-specific code paths (`#[cfg(windows)]` / `#[cfg(not(unix))]`) where `libc::O_NOFOLLOW` is not compiled in.

<!-- SEALED: Independent pass locked -->

## Contract pass

### Step verification (Phase 27: `STEP 45A`–`STEP 45F`)
- **Deferrals / Amendments:** None (all 6 steps passed and committed cleanly; `.phased/deferred/44C.md` and `44D.md` are from Phase 26).
- **`STEP 45A` (`TASK-0124`, advances `TASK-0123`):** All contract items met. `scripts/check-neutral.sh` excludes `STEPPED_IMPLEMENTATION_PLAN.md`, `docs/codebase-audit-report.md`, and `.tracker/` in no-arg mode only; explicit-arg mode unchanged; `bash scripts/check-neutral.sh` exits `0`.
- **`STEP 45B` (`TASK-0125`, advances `BUG-0012`):** All contract items met. `remove_item_from_parent_indexes` uses `scope_manifest.items.remove_id(item_id)`; `move_task_between_phases_removes_source_stub_and_updates_summary` added.
- **`STEP 45C` (`TASK-0126`, advances `BUG-0013`):** All contract items met. `lock_key()` canonicalizes existing prefix for `HELD_WRITE_LOCKS` and `PROJECT_CACHE`; `lock_path()` unchanged; `nested_write_lock_with_mixed_path_spellings_is_reentrant` added.
- **`STEP 45D` (`TASK-0127`, advances `BUG-0010`):** All contract items met. `libc = "0.2"` added under `[target.'cfg(unix)'.dependencies]`; `discover` and `guard_tracker_path` reject symlinked `.tracker` roots; `O_NOFOLLOW` applied on Unix in `read_tracker_file` and `with_write_lock_timeout`; `symlinked_tracker_root_rejected` added.
- **`STEP 45E` (`TASK-0128`, advances `BUG-0020`):** All contract items met. Listed public `TrackerStorage` mutators, `read_active_timer` quarantine path, and MCP `tool_pin_stable_release` wrapped in `with_write_lock`; `projects.lock` cross-process lock added in `models/registry.rs`; both integration tests added.
- **`STEP 45F` (`TASK-0129`, advances `BUG-0023`):** All contract items met. `AtomicBool` shutdown signal + 50ms lock slices added to `TrackerWatcher`; `hash_index_files` skips symlinks via `entry.file_type().is_symlink()`; both regression tests added.

### Origin assignment for independent findings
- **Finding 1 (`models/registry.rs` read-only `projects.lock` write open + global mutex hold):** `plan` + `coder` — `STEP 45E` required `fd_lock::RwLock` on `projects.lock` for reads without addressing read-only config directories or shared read locks, and the implementation opened `projects.lock` with `create(true).write(true)` / `try_write()` and held `REGISTRY_LOCK` across the 30s wait. Filed as `BUG-0033` (pending).
- **Finding 2 (`models/registry.rs` `load_registry_unlocked` fails read on prune save error):** `plan` — `STEP 45E` explicitly instructed `save_registry_unlocked(path, &registry)?`. Kept as a regression (not downgraded) and tracked under `BUG-0033` (pending).
- **Finding 3 (sibling component read-modify-write paths lack outer `with_write_lock`):** `pre-existing` — `STEP 45E` scoped only `tool_pin_stable_release`; CLI `ComponentCommand::Pin` and MCP `tool_upsert_component_manifest` / `tool_record_rolling_update` were already unlocked. Filed as `BUG-0034` (pending).

### Advances items ready to close (pending user approval)
- `TASK-0123` (`STEP 45A`), `BUG-0012` (`STEP 45B`), `BUG-0013` (`STEP 45C`), `BUG-0010` (`STEP 45D`), `BUG-0020` (`STEP 45E`), `BUG-0023` (`STEP 45F`).

## Handoff

- **Verdict:** PASS WITH FINDINGS — all 6 Phase 27 steps (`STEP 45A`–`STEP 45F`) met their contracts with 0 deferrals and 0 amendments; 2 medium pending bugs filed (`BUG-0033`, `BUG-0034`).
- **New pending items (medium):**
  - `BUG-0033` (`backend_core`): `models/registry.rs` read-only lookups (`load_global_registry_from`, `credential_accounts_for_root_in`) open `projects.lock` for write / exclusive flock (failing on read-only config dirs, creating `projects.lock` on empty dirs, holding global `REGISTRY_LOCK` across 30s wait) and fail reads if unsafe-project prune save errors.
  - `BUG-0034` (`backend_core`): Sibling component read-modify-write paths (`nebulous_cli` `ComponentCommand::Pin`, MCP `tool_upsert_component_manifest`, `tool_record_rolling_update`) lack outer `with_write_lock`.
- **Deferred steps to re-split:** None in Phase 27.
- **Advances items ready to close on approval:** `TASK-0123`, `BUG-0010`, `BUG-0012`, `BUG-0013`, `BUG-0020`, `BUG-0023`.
- **Decisions for user:** Confirm closing the 6 `Advances:` items above (or keep `BUG-0020` open until `BUG-0033`/`BUG-0034` are resolved).
