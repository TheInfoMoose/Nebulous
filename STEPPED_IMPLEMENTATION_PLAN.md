# Nebulous Project Plan

Nebulous is a Git-backed, CLI/TUI-first and MCP-enabled project, bug, and task tracker (`crates/backend_core` + `crates/nebulous_cli`). Completed Phases 1–24 and partial Phase 25 (`STEP 43A–43B`) history is archived in [archive/STEPPED_IMPLEMENTATION_PLAN_phases_1_24.md](archive/STEPPED_IMPLEMENTATION_PLAN_phases_1_24.md).

---

## Step Execution Roadmap

**Workflow:** phased

**Base gate:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

**Forbidden paths:** `/srv/devshare/NovOS`

**Planning files:** `docs/codebase-audit-report.md`, `archive/STEPPED_IMPLEMENTATION_PLAN_phases_1_24.md`

**How to read this:** work goes strictly top to bottom through **Up Next**, one step at a time. The **#** column is the only order. Step IDs are kept because commits and notes cite them. A step whose "Waits on" names an Open Question or a **[USER]** action doesn't start until that's resolved. `neb plan import` reads the **Up Next** and **Completed** tables, so keep their columns as they are.

**Who runs what (phased workflow):**
- **Hydrogen (`/project-plan`, `/phase-plan`, `/phase-review`), fresh session each:** direction, contract steps (no pre-written code), and code-first phase review.
- **Orrery TUI:** Gemini Hydrogen orchestrates, Gemini Neon implements, Gemini Hydrogen validates. Hooks gate, commit, and defer. Nothing failing is committed.
- **Neon (`/phase-issue`), fresh session:** fixes and restages a stopped or deferred step, with the user's approval.
- **Neon** never commits, never edits this file or the forbidden paths, and files bugs and scope lapses as **pending** neb items.
- **Repeated failures** or a scope lapse defer the step. A step whose PRE_GATE then fails is deferred without running. Too many deferrals (`max(2, ceil(steps/3))`) stop the phase.

**Phase goal (Phase 29):** Resolve Phase 28 security fallout (SSH fallback `BUG-0035`, `pushurl` pinning `BUG-0036`, re-staged `TASK-0110` zeroize/passphrase-guard, test defects `BUG-0037`/`BUG-0038`) and make `git_sync`, `git_commit_documented`, and `neb done -c` transactional, lock-safe, and idempotent (`BUG-0014`, `BUG-0004`, `BUG-0015`).

**Side findings, bugs and scope lapses** become **pending neb items**, reviewed once at phase end. Every Up Next row has a neb task (`source_ref` = step ID); commits start with `[<TASK-ID>](<scope>)`.

**Regression aversion policy (2026-11-25):** Every bug fix must ship with a dedicated regression test proving the failure before/after. Routine and high-risk work are split across separate phases; breaking API/protocol changes and large structural refactors stay out of bug-fix phases.

### Up Next

| # | Step | Command | Scope | Target Files | Waits on |
| :---: | :--- | :---: | :--- | :--- | :--- |
| 1 | **STEP 47A** | `/goal` | Fix `git_credential_for_remote` SSH fallback on non-HTTPS remotes and pin `push_repo_with_auth` to `remote.pushurl()` (`BUG-0035`, `BUG-0036`; closes `BUG-0019`) | `crates/backend_core/src/git_sync/mod.rs`, `crates/backend_core/tests/credential_hygiene.rs` | — |
| 2 | **STEP 47B** | `/goal` | Zeroize `GitAuthMaterial::pat` (`Option<Zeroizing<String>>`) and guard SSH passphrase stripping in `KeyPairManager` (re-split of `STEP 46F`, `TASK-0135`, `TASK-0136`, `TASK-0110`) | `crates/backend_core/src/git_sync/mod.rs`, `crates/backend_core/src/crypto/keys.rs`, `crates/backend_core/tests/credential_hygiene.rs`, `crates/backend_core/tests/workflow_integration.rs` | `STEP 47A` |
| 3 | **STEP 47C** | `/goal` | Fix Phase 28 test defects: seed global vault PAT in `vault_only_manager_does_not_fall_back_to_global_keyring_store` and test `set_permissions` failure in `ensure_secure_dir_propagates_permission_error` (`BUG-0037`, `BUG-0038`) | `crates/backend_core/tests/credential_hygiene.rs`, `crates/backend_core/src/crypto/mod.rs` | — |
| 4 | **STEP 47D** | `/boost` | Propagate merge-reconcile errors, block `git_push` on `duplicate_ids`, and fix `git_commit_documented` pre-amend window, reachable fix SHA, and `.git/index` stat metadata (`BUG-0014` / `AUD-07`) | `crates/backend_core/src/git_sync/mod.rs`, `crates/backend_core/tests/git_safety.rs`, `crates/backend_core/tests/workflow_integration.rs`, `crates/nebulous_cli/tests/done_commit_safety.rs`, `crates/nebulous_cli/tests/cli_integration.rs` | `STEP 47B` |
| 5 | **STEP 47E** | `/goal` | Add opt-in `allow_empty_ok` (default `false`) to `DocumentedCommitInput`, `git_commit_documented`, and MCP `git_commit_documented` for idempotent Orrery reruns (`BUG-0004`) | `crates/backend_core/src/git_sync/mod.rs`, `crates/backend_core/src/mcp/mod.rs`, `crates/nebulous_cli/src/commands/mod.rs`, `crates/backend_core/tests/workflow_integration.rs` | `STEP 47D` |
| 6 | **STEP 47F** | `/boost` | Move `neb done -c` close+commit+rollback into `backend_core` under one `with_write_lock` and surface rollback failures with a post-close commit-failure regression test (`BUG-0015` / `AUD-08`) | `crates/backend_core/src/workflow/lifecycle.rs`, `crates/nebulous_cli/src/commands/mod.rs`, `crates/nebulous_cli/tests/done_commit_safety.rs` | `STEP 47E` |

### Completed

Rows 1–12 are recorded from git history; their `POST_GATE`s weren't re-run when this table was added.

| Step ID | Status | Command | Scope | Target Files |
| :--- | :---: | :---: | :--- | :--- |
| **STEP 1–7** | **COMPLETED** (`513a0ab`, `97e6c36`) | — | Archive COSMIC GUI, 3-stage lifecycle, assist, CLI shortcuts, amber TUI, watcher, CI | `archive/`, `crates/backend_core/`, `crates/nebulous_cli/`, `.github/workflows/ci.yml` |
| **STEP 8** | **COMPLETED** (`c21238c`) | — | Supply-chain remediation, advisory gate | `Cargo.toml`, `Cargo.lock`, `deny.toml` |
| **STEP 9** | **COMPLETED** (`ccd2032`) | — | Release workflow ships `neb` only | `.github/workflows/release.yml` |
| **STEP 10** | **COMPLETED** (`0d44ea7`) | — | Git operation safety | `crates/backend_core/src/git_sync/mod.rs`, `crates/nebulous_cli/src/commands/mod.rs`, `crates/backend_core/src/mcp/mod.rs` |
| **STEP 11** | **COMPLETED** (`5917e22`) | — | Credential hygiene | `crates/backend_core/src/git_sync/mod.rs`, `crates/backend_core/src/crypto/mod.rs`, `crates/nebulous_cli/src/args.rs` |
| **STEP 12** | **COMPLETED** (`75d3815`) | — | Path containment, identifier allow-list, durable atomic writes | `crates/backend_core/src/storage/`, `crates/backend_core/src/ingestion/mod.rs` |
| **STEP 13A** | **COMPLETED** (`51d7dbc`) | `/boost` | `with_write_lock`: reentrant fd-lock on `.tracker/.lock`, timeout (Gemini; validate-step PASS) | `crates/backend_core/Cargo.toml`, `crates/backend_core/src/storage/mod.rs`, `crates/backend_core/src/error.rs`, `Cargo.lock` |
| **STEP 13B** | **COMPLETED** (`ef0bb57`) | `/goal` | Lock every lifecycle + timer mutation; new item dir fails on AlreadyExists (Gemini; validate-step PASS) | `crates/backend_core/src/workflow/lifecycle.rs`, `crates/backend_core/src/workflow/timer.rs`, `crates/backend_core/src/storage/mod.rs` |
| **STEP 13C** | **COMPLETED** (`f06eafa`) | `/goal` | Lock reconcile + ID allocation; concurrency test (Gemini; validate-step PASS) | `crates/backend_core/src/storage/reconcile.rs`, `crates/backend_core/src/storage/id.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 15A** | **COMPLETED** (`c35acb7`) | `/goal` | `storage/compat.rs`: schema check, unknown-key merge, stamp, migrate hook (Gemini; validate-step PASS) | `crates/backend_core/src/storage/compat.rs` (NEW), `crates/backend_core/src/storage/mod.rs`, `crates/backend_core/src/error.rs` |
| **STEP 15B** | **COMPLETED** (`6535c47`) | `/boost` | Versioned reads/writes for all 4 tiers; per-type item schema; toml `preserve_order` (Gemini; validate-step PASS; merge defect fixed by 15E) | `Cargo.toml`, `crates/backend_core/src/storage/mod.rs`, `crates/backend_core/src/models/item.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 15E** | **COMPLETED** (`1357aba`) | `/boost` | Fix 15B: merge only truly unknown keys (Gemini; validate-step PASS) | `crates/backend_core/src/storage/compat.rs`, `crates/backend_core/src/storage/mod.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 15C** | **COMPLETED** (`13a8a1e`) | `/goal` | Reconcile skips too-new items and reports them (Gemini; validate-step PASS) | `crates/backend_core/src/storage/reconcile.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 15D** | **COMPLETED** (`fb2966d`) | `/goal` | `project add` scaffolds the 3 canonical stages; `--no-use` (Gemini; validate-step PASS) | `crates/nebulous_cli/src/args.rs`, `crates/nebulous_cli/src/commands/mod.rs`, `crates/nebulous_cli/tests/cli_integration.rs` |
| **STEP 25A** | **COMPLETED** (`80ea914`) | `/goal` | Release module skeleton: exit codes, JSON envelope (Gemini; validate-step PASS) | `crates/backend_core/src/release/mod.rs` (NEW), `crates/backend_core/src/lib.rs` |
| **STEP 25B** | **COMPLETED** (`5014dd4`) | `/goal` | Debian version ordering + dpkg-verified case table (Gemini; validate-step PASS; 75 cases re-checked with dpkg) | `crates/backend_core/src/release/debver.rs` (NEW), `crates/backend_core/src/release/testdata/debver_cases.txt` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 25C** | **COMPLETED** (`b2ffedd`) | `/goal` | ReleasePaths + config.toml, unrelaxable forward-only policy and agent markers (Gemini; validate-step PASS) | `crates/backend_core/src/release/config.rs` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 25D** | **COMPLETED** (`6529c58`) | `/goal` | User-only gate: tty, agent markers, confirmation word (Gemini + 1 correction for a spec defect: env::var → var_os; validate-step PASS) | `crates/backend_core/src/release/gate.rs` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 25E** | **COMPLETED** (`faca60f`) | `/goal` | dpkg status / os-release parsers, inventory types (Gemini; validate-step PASS) | `crates/backend_core/src/release/inventory.rs` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 25F** | **COMPLETED** (`d5d3d82`) | `/goal` | Extracted-root helpers: confined symlink resolution, sha256, workspace crate versions, custom binary scan (Gemini; validate-step PASS) | `crates/backend_core/src/release/rootfs.rs` (NEW), `crates/backend_core/src/release/mod.rs`, `crates/backend_core/Cargo.toml` |
| **STEP 25G** | **COMPLETED** (`8a614c8`) | `/goal` | `inventory_from_root`: os-release, dpkg, live-boot + COSMIC synthetic packages, Calamares, boot (Gemini; validate-step PASS) | `crates/backend_core/src/release/rootfs.rs`, `crates/backend_core/Cargo.toml` |
| **STEP 25H** | **COMPLETED** (`1f9358e`) | `/boost` | `extract_iso_inventory`: unprivileged xorriso + unsquashfs into a self-deleting temp dir; tempfile moved to [dependencies] (Gemini; validate-step PASS; ignored round-trip test also passed) | `crates/backend_core/src/release/iso.rs` (NEW), `crates/backend_core/src/release/mod.rs`, `crates/backend_core/Cargo.toml` |
| **STEP 25I** | **COMPLETED** (`49e6b28`) | `/goal` | Golden v2 model (`Golden`, `GoldenComponent`, `Origin`), v1/v2 reader, deterministic atomic writer returning sha256 (Gemini; validate-step PASS) | `crates/backend_core/src/release/golden.rs` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 27A** | **COMPLETED** (`1edb895`) | `/goal` | `InstalledPackage.depends` + `parse_depends` from dpkg `Depends:`, for kernel-line membership (Gemini; validate-step PASS) | `crates/backend_core/src/release/inventory.rs`, `crates/backend_core/src/release/rootfs.rs` |
| **STEP 25J** | **COMPLETED** (`aa89910`) | `/boost` | `group_components` (kernel lines via Depends, source grouping), ABI rename, placeholder-aware `find_match`, `binary_changes` (Gemini; validate-step PASS) | `crates/backend_core/src/release/component.rs` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 25K** | **COMPLETED** (`fd2dae3`) | `/boost` | `approvals.toml` read/append with version-scoped `covers`; `evaluate_against_golden` by component, custom binaries, Calamares (Gemini; validate-step PASS) | `crates/backend_core/src/release/approvals.rs` (NEW), `crates/backend_core/src/release/variance.rs` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 25L** | **COMPLETED** (`d9b164b`) | `/boost` | `manifest.toml` model + reader; `verify_repo_invariants`: custom tools, Calamares, workspace crate versions, stage-script tokens (Gemini; validate-step PASS) | `crates/backend_core/src/release/manifest.rs` (NEW), `crates/backend_core/src/release/repo_invariants.rs` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 27B** | **COMPLETED** (`1c49362`) | `/goal` | `evaluate_against_manifest` (os-release, packages, custom binaries + flags, Calamares), `evaluate_variance` (manifest + optional golden), shared live-binary index (Gemini; validate-step PASS) | `crates/backend_core/src/release/variance.rs` |
| **STEP 25M** | **COMPLETED** (`77030e5`) | `/goal` | Loop-state item model, Python-compatible error/diff fingerprints, `record_attempt` (warn + 3-strike lock), `guardrail_state`, `unlock`; diagnostics preserved (Gemini; validate-step PASS) | `crates/backend_core/src/release/loopstate.rs` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 25N** | **COMPLETED** (`7afc457`) | `/goal` | `Phase`, failure ledger with component-class-only `record_failure`; release record model, reader, no-overwrite writer, listing, snapshot/cache lookups (Gemini; validate-step PASS) | `crates/backend_core/src/release/ledger.rs` (NEW), `crates/backend_core/src/release/records.rs` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 27C** | **COMPLETED** (`7fdba6c`) | `/boost` | Holds: one per component at golden (highest blocked version), all-last-good, pins-from-release, hold-file render (Gemini; validate-step PASS) | `crates/backend_core/src/release/holds.rs` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 25O** | **COMPLETED** (`81b283e`) | `/boost` | Candidate model, `build_candidate` (changed set, held_version_present, holds in force), `record_phase` order + identity checks, flake guard (Gemini; validate-step PASS; identity rule fixed by 27E) | `crates/backend_core/src/release/candidate.rs` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 27D** | **COMPLETED** (`3216ebf`) | `/boost` | `IsoIdentity` (extra live layers, production squashfs sha256), `extract_iso`, `read_test_overlay_manifest` (Gemini; validate-step PASS incl. ignored round-trips) | `crates/backend_core/src/release/iso.rs` |
| **STEP 27E** | **COMPLETED** (`d359c70`) | `/boost` | Fix 25O: `build_candidate` refuses test ISOs, stores `squashfs_sha256`; VM phases tie through the overlay claim + guest measurement; `parse_verdict_overlay` (Gemini; validate-step PASS) | `crates/backend_core/src/release/candidate.rs` |
| **STEP 25P** | **COMPLETED** (`a5aa9a4`) | `/boost` | Promote: `prepare_promote` (production ISO, identity, phases, bisect, variance; golden, record, cleared holds) + `commit_promote` (record → golden → remove candidate, rollback) (Gemini; validate-step PASS) | `crates/backend_core/src/release/promote.rs` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 25Q** | **COMPLETED** (`49b8a90`) | `/boost` | Bisect: `bisect.toml` model + I/O, `start_bisect`, `next_round` (test set + hold file), `record_round` (literal halving, culprit, interaction, cap); ledger entries returned (Gemini; validate-step PASS) | `crates/backend_core/src/release/bisect.rs` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 25R** | **COMPLETED** (`34ae5d1`) | `/boost` | Import from .nebulous: `plan_import` (validate sources, golden v1 → v2 placeholders, config.toml from `[guardrails]`, verbatim manifest/approvals/items, Conflict if any destination exists) + `commit_import` (new files only, remove on failure); `config.rs` gains `pub validate_config` + `render_config` (Gemini; validate-step PASS) | `crates/backend_core/src/release/import.rs` (NEW), `crates/backend_core/src/release/mod.rs`, `crates/backend_core/src/release/config.rs` |
| **STEP 25S** | **COMPLETED** (`3cb452c`) | `/boost` | Status summary (`read_status`, `status_json`, `render_status_text`, §4.8 keys) + built-in self-test runner (14 §2.9 cases, embedded dpkg table, panic-safe) (Gemini; validate-step PASS) | `crates/backend_core/src/release/status.rs` (NEW), `crates/backend_core/src/release/selftest.rs` (NEW), `crates/backend_core/src/release/mod.rs` |
| **STEP 25T** | **COMPLETED** (`ef8b77b`) | `/goal` | `ReleaseCommand` (17 subcommands) + `BisectCommand` + `StageArg`/`PhaseArg`/`ResultArg`/`HoldFormat` with `From` into backend types; parse/refusal tests (Gemini; validate-step PASS) | `crates/nebulous_cli/src/args.rs` |
| **STEP 25U** | **COMPLETED** (`d147bf6`) | `/boost` | `neb release` plumbing (`ReleaseCtx`, `ReleaseRun`, `render`, `from_error`) + baseline handlers: self-test, status, holds, pins-from-release, extract-golden (refuses a test ISO), verify-variance, record-attempt, check-guardrail (Gemini; POST_GATE PASS; validate-step flagged the plan hash because Phase 11 was added mid-run by the user + another agent, accepted) | `crates/nebulous_cli/src/commands/release/mod.rs` (NEW), `crates/nebulous_cli/src/commands/release/baseline.rs` (NEW), `crates/nebulous_cli/src/commands/mod.rs` |
| **STEP 29B** | **COMPLETED** (`0f341ed`) | `/goal` | Remove the release CLI: `args.rs` + `commands/mod.rs` restored to `fb2966d`, `commands/release/` deleted (Gemini; validate-step PASS) | `crates/nebulous_cli/src/args.rs`, `crates/nebulous_cli/src/commands/mod.rs`, `crates/nebulous_cli/src/commands/release/` (DEL) |
| **STEP 29C** | **COMPLETED** (`370c66f`) | `/goal` | Remove the release engine: `backend_core/src/release/` deleted (24 files); `lib.rs`, `backend_core/Cargo.toml`, `Cargo.lock` restored to `fb2966d`, so `crates/`, `Cargo.toml`, `Cargo.lock` equal `fb2966d` (Gemini; validate-step PASS) | `crates/backend_core/src/release/` (DEL), `crates/backend_core/src/lib.rs`, `crates/backend_core/Cargo.toml`, `Cargo.lock` |
| **STEP 29D** | **COMPLETED** (`41b48d2`) | `/goal` | Neutrality ratchet: `scripts/check-neutral.sh` (exact-count allowlist, 9 files / 182 lines) + CI step after checkout (Gemini; validate-step PASS) | `scripts/check-neutral.sh` (NEW), `.github/workflows/ci.yml` |
| **STEP 29E** | **COMPLETED** (`17e3e4d`) | `/goal` | Docs scrub: README + USAGE use a neutral "Orbit" example; both leave the allowlist; missing argument files fail (Gemini; validate-step PASS) | `README.md`, `docs/USAGE.md`, `scripts/check-neutral.sh` |
| **STEP 29F** | **COMPLETED** (`ec1a3d5`) | `/goal` | Backend source strings neutral: vault fixtures, MCP Suite 3 comments + tool descriptions, component doc comments; tool names/schemas/defaults unchanged (Gemini; validate-step PASS) | `crates/backend_core/src/lib.rs`, `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/src/models/component.rs`, `scripts/check-neutral.sh` |
| **STEP 29G** | **COMPLETED** (`c68bf2a`) | `/goal` | Ingestion kernel heuristic drops `xanmod`; storage test data neutral via fixed sed script (sha256-pinned) (Gemini; validate-step PASS) | `crates/backend_core/src/ingestion/mod.rs`, `crates/backend_core/tests/storage_integration.rs`, `scripts/check-neutral.sh` |
| **STEP 29H** | **COMPLETED** (`55f72c7`) | `/goal` | CLI strings and test data neutral via fixed sed script (sha256-pinned); check-neutral allowlist empty (Gemini; validate-step PASS) | `crates/nebulous_cli/src/lib.rs`, `crates/nebulous_cli/tests/cli_integration.rs`, `scripts/check-neutral.sh` |
| **STEP 26A** | **COMPLETED** (`8c8d113`) | `/goal` | `IdScheme` setting (`[project] id_scheme`, default sequential, not serialized when sequential) + `format_hash_id` / `is_hash_item_id` / `HASH_ID_HEX_LEN` (Gemini; validate-step PASS) | `crates/backend_core/src/models/project.rs`, `crates/backend_core/src/models/item.rs`, `crates/backend_core/src/storage/mod.rs` |
| **STEP 26B** | **COMPLETED** (`ce5ef72`) | `/boost` | `allocate_next_id` hash branch + `allocate_hash_id` (OsRng, redraw all-digit/taken, 16-attempt cap; counters untouched) (Gemini; validate-step PASS) | `crates/backend_core/src/storage/id.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 26C** | **COMPLETED** (`5f84a65`) | `/goal` | `ItemStub.created_at` (default empty, not serialized when empty) + `chronological_cmp` (instant rank, then same-type number, then ID); id tie-break not total with hash + 5-digit IDs, fixed in 26D (Gemini; validate-step PASS) | `crates/backend_core/src/models/scope.rs`, `crates/backend_core/src/models/item.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 26D** | **COMPLETED** (`fad0602`) | `/goal` | Total-order `IdRank` tie-break in `chronological_cmp`; reconcile + lifecycle sort stubs with it, own comparators removed (Gemini; validate-step PASS) | `crates/backend_core/src/models/scope.rs`, `crates/backend_core/src/storage/reconcile.rs`, `crates/backend_core/src/workflow/lifecycle.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 26E** | **COMPLETED** (`81954bd`) | `/goal` | `ReconcileReport.duplicate_ids` (all phases/scopes, parsed manifests only); `neb sync-index` warns in text + JSON (Gemini; validate-step PASS) | `crates/backend_core/src/storage/reconcile.rs`, `crates/nebulous_cli/src/commands/mod.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 26F** | **COMPLETED** (`bf5eff1`) | `/goal` | `git_pull_with_report` / `git_sync_with_report` (+ `GitPullOutcome` / `GitSyncOutcome`; old signatures kept as wrappers); `neb git pull` / `neb sync` warn + JSON `duplicate_ids`; shared `duplicate_id_warnings` (Gemini; validate-step PASS) | `crates/backend_core/src/git_sync/mod.rs`, `crates/nebulous_cli/src/commands/mod.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 26G** | **COMPLETED** (`bfd515f`) | `/goal` | MCP `git_pull` / `git_sync` results gain `duplicate_ids` (via 26F `*_with_report`; `[]` for commit_only); descriptions get one sentence (Gemini; validate-step PASS) | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 23A** | **COMPLETED** (`72aa885`) | `/goal` | Kind-aware helpers: `ItemType::{ALL, ordinal, from_prefix, label}`, `ProjectCounters::{get, get_mut, merge_max}`, `ScopeItems::{iter, count, insert, remove_id}`; `id_rank` uses `ordinal` (Gemini; validate-step PASS) | `crates/backend_core/src/models/item.rs`, `crates/backend_core/src/models/project.rs`, `crates/backend_core/src/models/scope.rs` |
| **STEP 23B** | **COMPLETED** (`6c5f2ec`) | `/goal` | Storage ID parsing, counter max/allocation and stub bucketing use the 23A helpers; reconcile sorts via `insert`; no behaviour change, tests unmodified (Gemini; validate-step PASS) | `crates/backend_core/src/storage/id.rs`, `crates/backend_core/src/storage/reconcile.rs`, `crates/backend_core/src/storage/mod.rs` |
| **STEP 23C** | **COMPLETED** (`fe56597`) | `/goal` | Lifecycle, assist and metrics stub walks use `ScopeItems::iter`; `sync_parent_indexes_for_item` is `remove_id` + `insert`; bug-first ranking unchanged; tests unmodified (Gemini; validate-step PASS) | `crates/backend_core/src/workflow/lifecycle.rs`, `crates/backend_core/src/workflow/assist.rs`, `crates/backend_core/src/workflow/metrics.rs` |
| **STEP 26H** | **COMPLETED** (`666ae5d`) | `/goal` | `cmp_item_ids` (IdRank total order) in scope.rs; assist's non-total `compare_item_ids` removed, 4 What's Next tie-breaks use it; exhaustive totality test (Gemini; validate-step PASS) | `crates/backend_core/src/models/scope.rs`, `crates/backend_core/src/workflow/assist.rs` |
| **STEP 23D** | **COMPLETED** (`16305dd`) | `/goal` | MCP + CLI scope listing `bugs_count` / `features_count` use `ScopeItems::count`; no output change (Gemini; validate-step PASS) | `crates/backend_core/src/mcp/mod.rs`, `crates/nebulous_cli/src/commands/mod.rs` |
| **STEP 23E** | **COMPLETED** (`b5ff0a4`) | `/goal` | TUI scope totals, open counts and listing use `ScopeItems::iter`; capture title uses `ItemType::label`; crate-wide per-kind grep clean outside models (Gemini; validate-step PASS) | `crates/nebulous_cli/src/tui/state.rs`, `crates/nebulous_cli/src/tui/render.rs` |
| **STEP 23F** | **COMPLETED** (`ab356f0`) | `/boost` | `ItemType::Task` (`TASK-XXXX`, schema v2, MAX 2), `task` counter (skipped when 0), `tasks` scope list; `is_hash_item_id` accepts TASK; storage_integration 6 approved line edits (Gemini; validate-step PASS) | `crates/backend_core/src/models/item.rs`, `crates/backend_core/src/models/project.rs`, `crates/backend_core/src/models/scope.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 23G** | **COMPLETED** (`54f351f`) | `/goal` | Storage integration tests: tasks allocate TASK-0001 alongside bugs/features, reconcile restores the task counter, concurrent mixed-type creators get 150 unique IDs; additions-only (Gemini; validate-step PASS) | `crates/backend_core/tests/storage_integration.rs` |
| **STEP 23H** | **COMPLETED** (`09ceebf`) | `/goal` | `neb task` (alias `chore`), TASKS column + `tasks_count` in scope list, TASK-XXXX in ID doc examples; CLI test incl. approved `tasks_count` assertion and stricter gate (Gemini; validate-step PASS) | `crates/nebulous_cli/src/args.rs`, `crates/nebulous_cli/src/commands/mod.rs`, `crates/nebulous_cli/tests/cli_integration.rs` |
| **STEP 23I** | **COMPLETED** (`0f454ee`) | `/goal` | TUI `t` opens task capture (NEW TASK CAPTURE); footer + 3 empty-state hints list `[t]`; approved headless TUI test and stricter gate (Gemini; validate-step PASS) | `crates/nebulous_cli/src/tui/state.rs`, `crates/nebulous_cli/src/tui/render.rs`, `crates/nebulous_cli/tests/cli_integration.rs` |
| **STEP 23J** | **COMPLETED** (`d00798c`) | `/goal` | MCP `create_item` / `list_items` text names `task`; `list_crate_boundaries` gains `tasks_count`; TASK-0001 in ID examples and commit-policy error; approved MCP test and stricter gate (Gemini; validate-step PASS) | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/src/git_sync/mod.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 23K** | **COMPLETED** (`5090aef`) | `/goal` | `ItemMetadata::new` (default status/severity, both timestamps); `create_item` uses `..ItemMetadata::new`; 2 inline tests; approved stricter gate (Gemini; validate-step PASS) | `crates/backend_core/src/models/item.rs`, `crates/backend_core/src/workflow/lifecycle.rs` |
| **STEP 23L** | **COMPLETED** (`b376420`) | `/goal` | The five integration-test `ItemMetadata` literals use `..ItemMetadata::new`, keeping only non-default fields; values unchanged (Gemini; validate-step PASS) | `crates/backend_core/tests/storage_integration.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 24A** | **COMPLETED** (`b505013`) | `/goal` | `ItemMetadata.source_ref: Option<String>` (skipped when None), `new()` sets None; 2 inline TOML tests; approved stricter gate (Gemini; validate-step PASS) | `crates/backend_core/src/models/item.rs` |
| **STEP 24B** | **COMPLETED** (`6355a69`) | `/goal` | Pure `planning::parse_roadmap` → `PlanStep`; approved gap picks G1–G8 (header-name columns, `\|` escapes, strict row/heading errors); 6 inline tests; stricter gate. Validator swapped to the hybrid-workflow skill copy mid-step (user-confirmed) (Gemini; diff + gate PASS) | `crates/backend_core/src/planning/mod.rs`, `crates/backend_core/src/lib.rs` |
| **STEP 24G** | **COMPLETED** (`2b8954d`) | `/goal` | CLI, TUI and 2 MCP `CreateItemInput` literals use `..CreateItemInput::new`; no behaviour change (Gemini; validate-step PASS) | `crates/nebulous_cli/src/commands/mod.rs`, `crates/nebulous_cli/src/tui/state.rs`, `crates/backend_core/src/mcp/mod.rs` |
| **STEP 24H** | **COMPLETED** (`ca8ae95`) | `/goal` | `CreateItemInput.source_ref` + `with_source_ref`; `create_item` writes it in the same `item.toml` write; bug/feature helpers use `..CreateItemInput::new` (Gemini; validate-step PASS) | `crates/backend_core/src/workflow/lifecycle.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 24C** | **COMPLETED** (`ffffbf0`) | `/boost` | `planning::import_plan` upserts tasks by `source_ref` (atomic via 24H); D1–D8 as approved; dry run takes no lock; Gemini-added input pre-check kept (user, 2026-11-14) (Gemini; validate-step PASS) | `crates/backend_core/src/planning/import.rs`, `crates/backend_core/src/planning/mod.rs`, `crates/backend_core/src/workflow/assist.rs`, `crates/backend_core/tests/plan_import.rs` |
| **STEP 24D** | **COMPLETED** (`13d314d`) | `/goal` | `neb plan import <PATH> [--phase] [--write]` (dry run default); CREATE/RETITLE/CLOSE lines, UNCHANGED count, footer; `--json` ImportReport; G1–G6 + stricter gate approved (Gemini; validate-step PASS) | `crates/nebulous_cli/src/args.rs`, `crates/nebulous_cli/src/commands/mod.rs`, `crates/nebulous_cli/tests/cli_integration.rs` |
| **STEP 24F** | **COMPLETED** (`094182e`) | `/goal` | MCP `plan_import {path, phase?, dry_run = true}` (path contained to project root, unknown args rejected); 4 MCP tests; G1–G4 + stricter gate approved (Gemini; validate-step PASS) | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/plan_import.rs` |
| **STEP 24E** | **COMPLETED** (`495c5de`) | `/goal` | USAGE: `neb task`/`chore` + type guidance, §6 Plan Import, §7 Item IDs, `t` key row, MCP `plan_import`; README Quick Start lines (Gemini; 1 correction: output-order wording; validate-step PASS) | `docs/USAGE.md`, `README.md` |
| **[USER] SEED** | **COMPLETED** (2026-11-15) | — | Seeded both trackers with `neb plan import --write`: Nebulous 83 tasks (8 open, 75 closed, TASK-0001…0083); NovOS 64 (14 open, 50 closed, hash IDs). Re-runs report UNCHANGED (user) | `.tracker/` (Nebulous), `.tracker/` (NovOS) |
| **STEP 14A** | **COMPLETED** (`dd4ce08`) | `/goal` | One `tool_handler` name→fn table for `call_tool` and `is_known_tool`; duplicate list removed; 2 inline tests (Gemini; validate-step PASS) | `crates/backend_core/src/mcp/mod.rs` |
| **STEP 14B** | **COMPLETED** (`ba47e0f`) | `/goal` | MCP stdio framing capped at 8 MiB, `Content-Length` in any header position, bad frames → `-32600` and keep serving; 5 tests in new `mcp_protocol.rs` (Gemini; strike 1 corrected; validate-step PASS) | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/mcp_protocol.rs` (NEW) |
| **STEP 14C** | **COMPLETED** (`04a206b`) | `/goal` | JSON-RPC: id-less requests get no reply and aren't run; `parse_request` keeps a string/number `id` on `-32600`, requires `jsonrpc` `"2.0"`; 7 tests (Gemini; validate-step PASS first try) | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/mcp_protocol.rs` |
| **STEP 14D** | **COMPLETED** (`d8236bc`) | `/goal` | `initialize` negotiates `protocolVersion` from `SUPPORTED_MCP_PROTOCOL_VERSIONS`; unknown or missing → `2025-06-18` (new default); 3 tests (Gemini; validate-step PASS first try) | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/mcp_protocol.rs` |
| **STEP 14E** | **COMPLETED** (`d4d7815`) | `/goal` | `tools/call` result is only `content` + `structuredContent` + `isError` (success and error); 20 CLI reads moved to `structuredContent`, key sets pinned (Gemini; validate-step PASS first try) | `crates/backend_core/src/mcp/mod.rs`, `crates/nebulous_cli/tests/cli_integration.rs` |
| **STEP 14F** | **COMPLETED** (`38744c8`) | `/goal` | `list_projects` read-only (no auto-register on an empty registry); dead `register_in_registry` removed; 2 tests (Gemini; validate-step PASS first try) | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/mcp_protocol.rs` |
| **STEP 14G** | **COMPLETED** (`7300eb5`) | `/boost` | `dry_run` (default true) on `git_pull` / `git_push` / `git_sync`; 1 new test, existing MCP git test opts out (Gemini; validate-step PASS first try) | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 14H** | **COMPLETED** (`6dea9ee`) | `/boost` | `dry_run` (default true) on `rollback_to_pinned_release`, writing `ingest_project_directory`, `switch_project`; 3 new tests, existing CLI MCP tests opt out (Gemini; validate-step PASS first try) | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/mcp_protocol.rs`, `crates/nebulous_cli/tests/cli_integration.rs` |
| **STEP 14I** | **COMPLETED** (`2aa4867`) | `/goal` | Bounded agent context: `desc.md` 16 KiB, scope description / fix summaries 1 KiB, fenced untrusted text, control chars stripped; 3 tests (Gemini; validate-step PASS first try) | `crates/backend_core/src/agent_context/mod.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 16A** | **COMPLETED** (`bdaf510`) | `/goal` | Write-lock-scoped `project.toml` cache; `create_item` ≤2 disk reads; 3 tests (Gemini; validate-step PASS first try) | `crates/backend_core/src/storage/mod.rs` |
| **STEP 16B** | **COMPLETED** (`dd16e2a`) | `/goal` | One index load per What's Next (`infer_scope_with_index`); `scan_max_counters` scans each phase once; 2 tests. Spec item 3 dropped (my spec error, Gemini halted correctly); passed on correction | `crates/backend_core/src/workflow/assist.rs`, `crates/backend_core/src/storage/id.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 17A** | **COMPLETED** (`e04c79b`) | `/boost` | Watcher: rescan/Other/errors trigger full reconcile; reconcile errors surfaced; join worker on drop; 2 tests (Gemini; validate-step PASS first try) | `crates/backend_core/src/watcher/mod.rs`, `crates/backend_core/tests/watcher_integration.rs` (NEW) |
| **STEP 17B** | **COMPLETED** (`2391979`) | `/goal` | Reconcile skips + reports malformed `item.toml` (`skipped_malformed`); 1 test (Gemini; validate-step PASS first try) | `crates/backend_core/src/storage/reconcile.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 17C** | **COMPLETED** (`7970fdc`) | `/goal` | Watcher echo suppression by content hash (1000ms window removed); 1 test (Gemini; validate-step PASS first try) | `crates/backend_core/src/watcher/mod.rs`, `crates/backend_core/tests/watcher_integration.rs` |
| **STEP 17D** | **COMPLETED** (`05710ad`) | `/goal` | TUI live reload via `TrackerWatcher`; reconcile errors in status bar; wall-clock timer minutes; 1 test (Gemini; validate-step PASS first try) | `crates/nebulous_cli/src/tui/mod.rs`, `crates/nebulous_cli/src/tui/render.rs`, `crates/nebulous_cli/tests/cli_integration.rs` |
| **STEP 18A** | **COMPLETED** (`0c57ef6`) | `/goal` | Idempotent timer stop (`effort.last_timer_session`) + parent index sync on stop; 2 tests (Gemini; validate-step PASS first try) | `crates/backend_core/src/models/item.rs`, `crates/backend_core/src/workflow/timer.rs`, `crates/backend_core/src/workflow/lifecycle.rs`, tests |
| **STEP 18B** | **COMPLETED** (`7b66f0e`) | `/goal` | Corrupt `.timer.toml` quarantined to `.bak` and read as no timer; orphan stop quarantines + returns NotFound; 2 tests (Gemini; validate-step PASS first try) | `crates/backend_core/src/storage/mod.rs`, `crates/backend_core/src/workflow/timer.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 18C** | **COMPLETED** (`e2be59f`) | `/goal` | `metrics.max_session_minutes` (default 480, 0 = off) caps one stop; `capped_from` in result + JSON; CLI warning; 1 test (Gemini; validate-step PASS first try) | `crates/backend_core/src/models/project.rs`, `crates/backend_core/src/workflow/timer.rs`, `crates/nebulous_cli/src/commands/mod.rs`, tests |
| **STEP 19A** | **COMPLETED** (`aa28130`) | `/boost` | `resolve_start_target` + `start_work_item(.., cwd)` → `StartedWork { .., stopped_previous }`; CLI/TUI report switched timer; 1 new test (Gemini; validate-step PASS first try) | `crates/backend_core/src/workflow/lifecycle.rs`, `crates/nebulous_cli/src/commands/mod.rs`, `crates/nebulous_cli/src/tui/state.rs`, tests |
| **STEP 19B** | **COMPLETED** (`1e3b561`) | `/goal` | `neb start --dry-run` (text + JSON, no lock/writes); start/move/done zero-ID help reworded; 2 tests (Gemini; validate-step PASS first try) | `crates/nebulous_cli/src/args.rs`, `crates/nebulous_cli/src/commands/mod.rs`, `crates/nebulous_cli/src/lib.rs`, tests |
| **STEP 19C** | **COMPLETED** (`ff88902`) | `/goal` | Recurring + stagnant recommendations filtered to `active_phase_id`; 1 test (Gemini; validate-step PASS first try) | `crates/backend_core/src/workflow/assist.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 21A** | **COMPLETED** (`e58462d`) | `/goal` | `restore_terminal()` shared by cleanup guard + panic hook set in `run_tui` (Gemini; validate-step PASS first try) | `crates/nebulous_cli/src/tui/mod.rs` |
| **STEP 21B** | **COMPLETED** (`a269355`) | `/goal` | `TuiApp.index` + `rebuild_view()`; filter keys rebuild in memory; `r` reloads; 1 test (Gemini; validate-step PASS first try) | `crates/nebulous_cli/src/tui/state.rs`, `crates/nebulous_cli/tests/cli_integration.rs` |
| **STEP 21C** | **COMPLETED** (`b24312b`) | `/goal` | `clipboard::copy_text` (arboard + wayland-data-control → OSC 52 on stderr tty), `osc52_sequence`; 1 test (Gemini; validate-step PASS first try) | `crates/nebulous_cli/src/clipboard.rs` (NEW), `crates/nebulous_cli/src/lib.rs`, `crates/nebulous_cli/Cargo.toml`, `Cargo.lock` |
| **STEP 21D** | **COMPLETED** (`7dce91c`) | `/goal` | `try_copy_to_clipboard`, `action_start_work`, `action_copy_context` use `copy_text`; TUI status reports System / OSC 52 / unavailable (Gemini; validate-step PASS first try) | `crates/nebulous_cli/src/commands/mod.rs`, `crates/nebulous_cli/src/tui/state.rs` |
| **STEP 21E** | **COMPLETED** (`c892d32`) | `/goal` | `CreateModalState.confirm_new_scope`; unknown scope needs second Enter, scope edits reset it; 1 new test + palette test gets 2nd Enter (Gemini; validate-step PASS first try) | `crates/nebulous_cli/src/tui/state.rs`, `crates/nebulous_cli/tests/cli_integration.rs` |
| **STEP 21F** | **COMPLETED** (`0470d3c`) | `/goal` | All 4 TUI `set_active_phase` calls removed (switch_stage, create, move, jump-to-next; last one implied by POST_GATE, not listed in item 2); 1 test (Gemini; validate-step PASS first try) | `crates/nebulous_cli/src/tui/state.rs`, `crates/nebulous_cli/tests/cli_integration.rs` |
| **STEP 22A** | **COMPLETED** (`f4abeb8`) | `/goal` | Removed non-auth fetch/push/pull_repo, DocumentedCommit aliases, `with_include_workspace`, `export_bounded_prompt`; Mcp arm → `unreachable!` (Gemini; validate-step PASS first try) | `crates/backend_core/src/git_sync/mod.rs`, `crates/backend_core/src/agent_context/mod.rs`, `crates/nebulous_cli/src/commands/mod.rs` |
| **STEP 22B** | **COMPLETED** (`d9da615`) | `/goal` | README zero-ID + dry-run + Linux-only binaries; USAGE key table + timer cap/quarantine; ARCHITECTURE watcher (Gemini; validate-step PASS first try) | `README.md`, `docs/USAGE.md`, `docs/ARCHITECTURE.md` |
| **STEP 22C** | **COMPLETED** (`27740ca`) | `/goal` | `realpath -m` probe, `${SUDO}` on install/rm, refuse build under sudo (Gemini; validate-step PASS first try; ~/.local regression parked) | `scripts/install.sh` |
| **STEP 30A** | **COMPLETED** (`65bf216`) | `/goal` | `StartedWork` re-exported in `workflow/mod.rs` and crate root (Parked 5; Gemini; validate-step PASS first try) | `crates/backend_core/src/workflow/mod.rs`, `crates/backend_core/src/lib.rs` |
| **STEP 30B** | **COMPLETED** (`83ecf69`) | `/goal` | Mcp arm returns `ValidationError`; 1 test (Parked 6; Gemini; validate-step PASS first try) | `crates/nebulous_cli/src/commands/mod.rs`, `crates/nebulous_cli/src/lib.rs` |
| **STEP 30C** | **COMPLETED** (`82f68ee`) | `/goal` | `nearest_existing_dir` sudo check; README `--system` without sudo; ARCHITECTURE drops `export_bounded_prompt` (Parked 7–9; Gemini; validate-step PASS first try) | `scripts/install.sh`, `README.md`, `docs/ARCHITECTURE.md` |
| **STEP 31A** | **COMPLETED** (`7b53d27`) | `/goal` | All 7 ingestion/component writers run inside `with_write_lock`; lock-wait test (audit 1; Gemini; validate-step PASS first try) | `crates/backend_core/src/ingestion/mod.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 31B** | **COMPLETED** (`9b48550`) | `/goal` | `StatusChange`; `update_item_status_reporting` / `close_item_reporting` stop timers via `stop_timer` (cap applies, no inline add) (audit 2; Gemini; validate-step PASS first try) | `crates/backend_core/src/workflow/lifecycle.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 31C** | **COMPLETED** (`221e3e6`) | `/goal` | `StatusChange` re-exported; `quarantined_timer_path()` + test (audit 3; Gemini; validate-step PASS first try) | `crates/backend_core/src/workflow/mod.rs`, `crates/backend_core/src/lib.rs`, `crates/backend_core/src/storage/mod.rs` |
| **STEP 31D** | **COMPLETED** (`acb907c`) | `/goal` | `warn_if_capped` on status/close; `note_quarantined_timer` on next/start/timer status; TUI close cap note + quarantine banner (audit 2, 3; Gemini; validate-step PASS first try) | `crates/nebulous_cli/src/commands/mod.rs`, `crates/nebulous_cli/src/tui/state.rs`, `crates/nebulous_cli/tests/cli_integration.rs` |
| **STEP 32A** | **COMPLETED** (`4aa9a8d`) | `/goal` | `with_vault_lock` (fd-lock on `vault.lock`) replaces the in-process Mutex for the 4 vault fns + vault.key creation; lock-wait test (Parked 5; Gemini; validate-step PASS first try) | `crates/backend_core/src/crypto/mod.rs` |
| **STEP 32B** | **COMPLETED** (`9c9df31`) | `/goal` | MCP `update_item_status` / `close_item_with_fix` use `*_reporting` and add `stopped_timer` (item_id, logged_minutes, capped_from); 2 tests (Parked 8; Gemini; validate-step PASS first try) | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 32C** | **COMPLETED** (`6ca51e5`) | `/goal` | Doc comments + ARCHITECTURE: Windows vault files rely on inherited `%APPDATA%` ACLs; `NEBULOUS_CONFIG_DIR` outside the profile isn't protected (docs only, documented gap) (Parked 4; Gemini; validate-step PASS first try) | `crates/backend_core/src/crypto/mod.rs`, `docs/ARCHITECTURE.md` |
| **STEP 32D** | **COMPLETED** (`02d3773`) | `/goal` | CI matrix ubuntu/macos/windows, `fail-fast: false`; neutrality, apt, cargo-deny, fmt Linux-only (Parked 3; Gemini; validate-step PASS first try) | `.github/workflows/ci.yml` |
| **[USER] CI** | **COMPLETED** (pushed by user 2026-11-16) | — | Matrix ran: macOS fails `storage_integration.rs:1894` (`/etc` → `/private/etc`); Windows log pasted was a duplicate of macOS; findings parked | — |
| **STEP 33A** | **COMPLETED** (`83ccde7`) | `/goal` | `UNSAFE_SYSTEM_DIRS` adds `/Applications`, `/private/etc`, `/private/var`, `/private/var/tmp`; 4 test asserts (Parked 1; Gemini; validate-step PASS first try) | `crates/backend_core/src/storage/mod.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 33B** | **COMPLETED** (`a5db7cb`) | `/goal` | CI test step `cargo test --workspace --no-fail-fast` (Parked 3; Gemini; validate-step PASS first try) | `.github/workflows/ci.yml` |
| **STEP 33C** | **COMPLETED** (`a240588`) | `/goal` | 12 tracker titles reworded neutrally (item.toml, desc.md, scope.toml identical); `.timer.toml.bak` ignored; `.tracker/` committed (195 files) by Claude (Parked 4; Gemini; validate-step PASS first try) | `.tracker/…/backend_core/`, `.gitignore` |
| **STEP 33D** | **COMPLETED** (`f59ac0f`) | `/boost` | `restrict_to_current_user` via `icacls /inheritance:r /grant:r`; files every save, dirs when created; Windows-only test; docs updated (Parked 5; Gemini; validate-step PASS first try; Windows code clippy-probed on Linux; PROVEN by Windows CI run 5: `windows_vault_file_acl_is_user_only` passed) | `crates/backend_core/src/crypto/mod.rs`, `docs/ARCHITECTURE.md` |
| **[USER] CI (Phase 15)** | **COMPLETED** (pushed by user 2026-11-16) | — | macOS: `/etc` fixed, `/home` now fails (autofs); Windows: clippy `result_large_err` blocks all tests; findings parked | — |
| **STEP 34A** | **COMPLETED** (`243defb`) | `/goal` | `TomlDeserialize.source: Box<toml::de::Error>` (From impl + registry.rs); message test (Parked 1; Gemini; validate-step PASS first try; Windows clippy effect unproven until CI) | `crates/backend_core/src/error.rs`, `crates/backend_core/src/models/registry.rs` |
| **STEP 34B** | **COMPLETED** (`ca09dd2`) | `/goal` | `lexical_project_root_candidate` split out; `UNSAFE_SYSTEM_DIRS` checked on lexical + canonical path; unit test (Parked 2; Gemini; validate-step PASS first try; macOS effect unproven until CI) | `crates/backend_core/src/storage/mod.rs` |
| **STEP 34C** | **COMPLETED** (`4a38df2`) | `/goal` | `authors.workspace = true` in both member crates (Parked 3; Gemini; validate-step PASS first try) | `crates/backend_core/Cargo.toml`, `crates/nebulous_cli/Cargo.toml` |
| **STEP 34D** | **COMPLETED** (`ebb8682`) | `/goal` | Config-dir tree lists `vault.key` and `vault.lock` (Parked 5; Gemini; validate-step PASS first try) | `docs/ARCHITECTURE.md` |
| **[USER] CI (Phase 16)** | **COMPLETED** (pushed by user 2026-11-16) | — | macOS: all passed (34B fixed `/home`). Windows: `result_large_err` gone (34A works); clippy now fails on 1 unused import from my 34B spec; finding parked | — |
| **STEP 35A** | **COMPLETED** (`59dfb46`) | `/goal` | Storage `mod tests` gated `#[cfg(all(test, unix))]`; per-test gate removed (Parked 1; Gemini; validate-step PASS first try; Windows clippy effect unproven until CI) | `crates/backend_core/src/storage/mod.rs` |
| **[USER] CI (Phase 17)** | **COMPLETED** (pushed by user 2026-11-16) | — | Windows: storage test-module error gone (35A works); clippy now stops at `path_containment.rs:9` unused `ComponentManifest`; finding parked | — |
| **STEP 36A** | **COMPLETED** (`b395ad8`) | `/goal` | `ComponentManifest` import moved into `#[cfg(unix)] mod unix` (Parked 1; Gemini; validate-step PASS first try; Windows clippy effect unproven until CI) | `crates/backend_core/tests/path_containment.rs` |
| **[USER] CI (Phase 18)** | **COMPLETED** (pushed by user 2026-11-16) | — | Windows: clippy passes; first test run; lib tests (incl. 33D ACL test) pass; `storage_integration` (Windows dirs, expected) and `workflow_integration` git test fail; findings parked | — |
| **STEP 37A** | **COMPLETED** (`1a7458b`) | `/goal` | `UNSAFE_WINDOWS_DIRS` + `is_windows_system_dir` (any drive, any case) on lexical + canonical path; portable unit test (Gemini; fmt/clippy/tests PASS; POST_GATE's cargo deny failed on RUSTSEC-2026-0543/0544 published after start, unrelated to the diff → 37D) | `crates/backend_core/src/storage/mod.rs` |
| **STEP 37D** | **COMPLETED** (`01954e1`) | `/goal` | libz-sys 1.1.31 → 1.1.33 (RUSTSEC-2026-0544); RUSTSEC-2026-0543 (chrono) ignored with dated reason until Phase 20 (user-approved workaround; Gemini; validate-step PASS first try) | `Cargo.lock`, `deny.toml` |
| **STEP 37B** | **COMPLETED** (`7111965`) | `/goal` | Unix path asserts + `/tmp` ingest in `#[cfg(unix)]`; `#[cfg(windows)]` loop over 6 system dirs; registry pruning → `#[cfg(unix)] fn test_registry_prunes_poisoned_unix_roots`; `testuser` (Gemini; fmt/clippy/tests/deny/neutral PASS; POST_GATE's `sed` range check was a Claude gate bug — ended at `^    };` and ran into the new test; re-checked with `^}`: 0 hits) | `crates/backend_core/tests/storage_integration.rs` |
| **STEP 37C** | **COMPLETED** (`bda0f69`) | `/goal` | `core.autocrlf=false` on `repo` and `cloned_repo` in `test_git_fetch_ahead_behind_and_workspace_dirty_tracking` (Gemini; validate-step PASS first try) | `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 37E** | **COMPLETED** (`4263657`) | `/goal` | `canonical_or_self`; `hash_index_files` walks the canonical dir; `is_own_index_echo` looks up canonical event paths; `#[cfg(all(test, unix))] mod echo_path_tests` (symlink both ways + changed content) (Gemini; validate-step PASS first try) | `crates/backend_core/src/watcher/mod.rs` |
| **[USER] CI (Phase 19)** | **COMPLETED** (2026-11-17) | — | Run 7: ubuntu, macOS and Windows all green (first fully green matrix) | — |
| **STEP 38A** | **COMPLETED** (`a25eefd`) | `/goal` | jiff 0.2.39 (`default-features = false`, `std`) in workspace + both crates; locked at 0.2.39; optional defmt/portable-atomic/jiff-static are lock-only (not built); side effect: `os_pipe` 1.2.3's windows-sys edge re-resolved 0.48 → 0.61.2 (already locked; Windows-only) (Gemini; validate-step PASS first try) | `Cargo.toml`, both crate `Cargo.toml`s, `Cargo.lock` |
| **STEP 38B** | **COMPLETED** (`e6e6f98`) | `/goal` | `models/time.rs`: jiff `parse_timestamp_utc` / `parse_date` (same parse order and error strings as the chrono versions) + `format_timestamp` (strftime, whole seconds) / `now_timestamp_string` / `today_date_string`, 5 unit tests; `models/mod.rs` gained only `pub mod time;` (Gemini; validate-step PASS first try) | `models/time.rs` (new), `models/mod.rs` |
| **STEP 38C** | **COMPLETED** (`21ddc27`) | `/goal` | lifecycle.rs (5 dates + `now_rfc3339` body), storage/mod.rs (1 + 1), git_sync/mod.rs (1 date) build now/today strings via `now_timestamp_string()` / `today_date_string()`; chrono imports gone from all three (Gemini; validate-step PASS first try) | `workflow/lifecycle.rs`, `storage/mod.rs`, `git_sync/mod.rs` |
| **STEP 38D** | **COMPLETED** (`f73986d`) | `/goal` | ingestion/mod.rs (3 timestamps), mcp/mod.rs (2 timestamps + 1 date) via the helpers; `tui_timer_renders_wall_clock_minutes` builds `started_at` with `format_timestamp(Timestamp::now() - SignedDuration::from_mins(7))`; no chrono in the three files (Gemini; validate-step PASS first try) | `ingestion/mod.rs`, `mcp/mod.rs`, `cli_integration.rs` |
| **STEP 38E** | **COMPLETED** (`4f39ec1`) | `/goal` | `PhaseMetadata::parsed_start_date` / `parsed_end_date` → `Option<jiff::civil::Date>`, `ProjectMetadata::parsed_created_at` → `jiff::Timestamp`, scope.rs `CreatedAtRank::Parsed(jiff::Timestamp)`; all via `models::time` (Gemini; validate-step PASS first try) | `models/phase.rs`, `models/project.rs`, `models/scope.rs` |
| **STEP 38F** | **COMPLETED** (`8dced6d`) | `/goal` | registry.rs `parsed_last_opened_at` → `jiff::Timestamp` + 2 now strings via `now_timestamp_string()`; component.rs `parsed_pinned_at` / `parsed_verified_at` / `parsed_recorded_at` → jiff `Timestamp`; both parse via `models::time`, no chrono (Gemini; validate-step PASS first try) | `models/registry.rs`, `models/component.rs` |
| **STEP 38G** | **COMPLETED** (`e0c9c67`) | `/boost` | item/timer models, workflow timer/metrics/assist, TUI render, lib.rs tests and workflow_integration.rs on `jiff::Timestamp` / `civil::Date` / `SignedDuration`; `duration_since(..).as_secs()` (`.max(0)` kept, `/ 86_400` truncates like `num_days`); test inputs/expectations unchanged; chrono left only in `models/mod.rs` (Gemini; validate-step PASS first try) | 8 files (see step) |
| **STEP 38H** | **COMPLETED** (`503c541`) | `/goal` | `models/mod.rs`: chrono import, `LOCAL_DATETIME_FORMATS` and the chrono parsers deleted; `pub use time::{parse_date, parse_timestamp_utc};`; unused `crate::error` import dropped; no chrono left in any .rs file (Gemini; validate-step PASS first try) | `models/mod.rs` |
| **STEP 38I** | **COMPLETED** (`b4fc26b`) | `/goal` | chrono removed from the 3 manifests; Cargo.lock only lost chrono + its iana-time-zone / windows-core subtree (102 deletions, no additions); deny.toml back to `ignore = []`; `cargo deny check advisories bans` passes with zero exceptions (Gemini; validate-step PASS first try) | `Cargo.toml`, both crate `Cargo.toml`s, `Cargo.lock`, `deny.toml` |
| **[USER] CI (Phase 20)** | **COMPLETED** (2026-11-17) | — | Pushed through `b4fc26b`: ubuntu, macOS and Windows all green; chrono gone, `cargo deny` passes with zero exceptions | — |
| **STEP 39A** | **COMPLETED** (`1e2b95e`) | `/goal` | Zero-ID error no longer runs What's Next; both messages point to `neb next`; test asserts `neb next` (Gemini; validate-step PASS first try) | `workflow/assist.rs`, `workflow_integration.rs` |
| **STEP 39B** | **COMPLETED** (`f08985a`) | `/goal` | `list_items` mentions tasks; `switch_project`, `ingest_project_directory`, `rollback_to_pinned_release` `dry_run` text describes each tool's own dry run; 3 `git_*` strings unchanged (Gemini; validate-step PASS first try) | `mcp/mod.rs` |
| **STEP 39C** | **COMPLETED** (`941b9c9`) | `/goal` | `CHANGELOG.md` (0.1.0 notes under `## [Unreleased]`, text exact to spec) + `scripts/release-check.sh` (tag = workspace version, dated non-empty section, prints trimmed body; exit 2 usage / 1 mismatch) (Gemini; validate-step PASS first try) | `CHANGELOG.md`, `scripts/release-check.sh` |
| **STEP 39D** | **COMPLETED** (`9a9f17b`) | `/goal` | `release.yml`: release guard right after checkout writes `dist/RELEASE_NOTES.md`; `neb --version` must equal the tag; CHANGELOG.md in the archive; publish uses `body_path` + `prerelease` for `v0.*` (Gemini; validate-step PASS first try) | `.github/workflows/release.yml` |
| **STEP 39E** | **COMPLETED** (`94b924f`) | `/goal` | `scripts/bump-version.sh [--dry-run] X.Y.Z` (both version strings, `cargo update --workspace`, fresh Unreleased + dated section, runs release-check, prints commit/tag/push; never runs them) + README "Releasing". First check FAILed on a Claude gate bug (locale-dependent `sort` in the file-list compare); gate fixed to `LC_ALL=C sort`, plan hash otherwise unchanged, re-check PASS. No executor strike. | `scripts/bump-version.sh`, `README.md` |
| **[USER] CI (Phase 21)** | **COMPLETED** (2026-11-18) | — | Pushed through `94b924f`: ubuntu, macOS and Windows all green (run 44315324411). The intermediate `9a9f17b` run failed on Windows only, in the watcher self-loop test (parked as a flake) | — |
| **[CLAUDE] AUDIT (Phase 21)** | **COMPLETED** (2026-11-18) | — | Orrery ↔ Nebulous audit: 2 blockers (A1 hook/gate args, A2 `git_commit_documented` commits without summary), 1 should-fix, 3 nice-to-have; see "Phase 21 audit report" | this file |
| **STEP 40A** | **COMPLETED** (`bc00dd3`) | `/goal` | MCP `git_commit_documented` rejects a missing/blank `summary` and takes `dry_run` (default true); real commits report `dry_run: false`; CLI MCP session test passes `dry_run: false`; new test covers bare Orrery payload, blank, dry run, real commit (Gemini; validate-step PASS first try) | `mcp/mod.rs`, `workflow_integration.rs`, `cli_integration.rs` |
| **STEP 40B** | **COMPLETED** (`2426306`) | `/goal` | Watcher self-loop assertion waits for 500 ms of quiet within 5 s instead of any event in 250 ms; gate ran the test 3× (Gemini; validate-step PASS first try) | `workflow_integration.rs` |
| **STEP 40C** | **COMPLETED** (`4da9cee`) | `/goal` | `release.yml` deny gate now `check advisories bans`, matching ci.yml (Gemini; validate-step PASS first try) | `.github/workflows/release.yml` |
| **[USER] CI (Phase 22)** | **COMPLETED** (2026-11-18) | — | Pushed through `4da9cee`: ubuntu, macOS and Windows all green (run 44367918082; macOS first went unrun for lack of a GitHub runner and passed on re-run). First green Windows run of 40B's watcher test | — |
| **[USER] ORRERY A1** | **COMPLETED** (Orrery `899bdea`, 2026-11-18) | — | Orrery Steps 89–92: configured `args` on hooks/gates merged with the reserved `orrery` object, CLI `--args`, live merged-args test, README; user's NEB-1 acceptance passed | Orrery repo |
| **[CLAUDE] LIVE (Phase 22)** | **COMPLETED** (2026-11-18) | — | Orrery hook args drive a real Nebulous write: `dry_run: false` committed `.tracker/` only; omitted → dry run; bare payload → "summary must not be empty". `v0.1.0` gate cleared; see "Phase 22 live check" | this file |
| **Release v0.1.0** | **RELEASED** (`520d576`, 2026-11-18) | — | `bump-version.sh 0.1.0` dated the CHANGELOG section; tag pushed. Release run 44386889068 green on first run (guard, `--version`, package, publish); GitHub pre-release with the changelog section as notes and 2 assets (`.tar.gz` 4,527,620 B + `.sha256`). CI run 44386889328 green on all three OSes. Asset check: `sha256sum -c` OK (`d4e85696…0543`), archive holds `neb`, README, LICENSE, CHANGELOG (byte-identical to the tag), install.sh, docs/; released `neb --version` = `neb 0.1.0`; released binary rejects Orrery's bare payload ("summary must not be empty", no commit) | `CHANGELOG.md` |
| **STEP 41A** | **COMPLETED** (`484058c`) | `/goal` | Plan import: Completed rows never create items; closed/merged items never retitled; 1 new test, 3→2 count asserts (BUG-0003 closed; Gemini; validate-step PASS first try) | `crates/backend_core/src/planning/import.rs`, `crates/backend_core/tests/plan_import.rs`, `crates/nebulous_cli/tests/cli_integration.rs`, `docs/USAGE.md` |
| **STEP 41B** | **COMPLETED** (`b601308`) | `/goal` | MCP `plan_import` accepts and ignores the reserved `orrery` object; other unknown args still rejected; schema lists `orrery`; 1 test (BUG-0001 closed; Gemini; validate-step PASS first try) | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/plan_import.rs`, `docs/USAGE.md` |
| **STEP 41C** | **COMPLETED** (`6a0e7a6`) | `/goal` | MCP `update_item_status` / `close_item_with_fix`: `item_id` optional, resolved from `orrery.step_title` via `source_ref` (0 / >1 / unparseable → error); 8 tests (FEAT-0001 closed; side finding TASK-0093 pending; Gemini; validate-step PASS first try) | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/mcp_orrery_context.rs` |
| **STEP 41D** | **COMPLETED** (`63feb6f`) | `/goal` | README says 24 MCP tools; USAGE: `--root` optional (cwd discovery first) and the Orrery step-context section (Gemini; validate-step PASS first try) | `README.md`, `docs/USAGE.md` |
| **[USER] INSTALL (Phase 23)** | **COMPLETED** (user, 2026-11-20) | — | `./scripts/install.sh --user` at `e6104af`; installed `neb` serves `update_item_status` required `["status"]`, `close_item_with_fix` `["summary"]`, `plan_import` lists `orrery` | — |
| **[CLAUDE] REGISTER** | **COMPLETED** (`922509b`) | — | `orrery mcp add-stdio --id neb --command neb --arg mcp`; `orrery mcp list` → `neb: 24 tools`; read-only `list_items` through Orrery `a4b6f7a`'s live client with merged `orrery` args passed (built from a `git archive` of Orrery HEAD, because Orrery's checkout had uncommitted non-compiling edits); `project.toml` unchanged, `.git/hooks` samples only. orrery-setup §5 and neb-tracking §6 corrected (no `neb project switch` needed) | `.orrery/mcp/servers.toml` |
| **[USER] CI (Phase 23)** | **COMPLETED** (pushed by user 2026-11-20) | — | Run 44663931999 (`e6104af`, through 41D) and 44664160960 (`922509b`) green on ubuntu/macOS/Windows | — |
| **[CLAUDE] LIVE (Phase 23)** | **COMPLETED** (2026-11-20) | — | All 5 runs passed; see "Phase 23 live check". Real `on_step_start` hook `neb:update_item_status {"status":"in_progress"}` flipped the step's task in ~1 s; cost one Haiku orchestrator turn ($0.0022 list) | this plan file |
| **STEP 42A** | **COMPLETED** (`625c573`) | `/goal` | MCP `2025-11-25` supported and default; guard test keeps every `inputSchema` an object schema with 2020-12 keywords (TASK-0084 closed; Gemini; validate-step PASS first try) | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/mcp_protocol.rs` |
| **STEP 42B** | **COMPLETED** (`a5fa574`) | `/goal` | MCP `git_fetch` `dry_run` (default false): status + `would` without contacting a remote, works with no `origin`; non-dry result unchanged; 1 test (BUG-0002 closed; Gemini; validate-step PASS first try) | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 42C** | **COMPLETED** (`937b855`) | `/goal` | USAGE Part 3: `claude mcp add` (user/project scope), `agy mcp add` (global; `--root` to pin), protocol versions note (Gemini; validate-step PASS first try) | `docs/USAGE.md` |
| **[USER] INSTALL (Phase 24)** | **COMPLETED** (user, 2026-11-20) | — | `./scripts/install.sh --user` at `47ff52f`; installed `neb` answers `2025-11-25` and `git_fetch` has `dry_run` | — |
| **[USER] REGISTER** | **COMPLETED** (user, 2026-11-20) | — | `claude mcp add --scope user nebulous -- neb mcp` (✔ Connected) and `agy mcp add nebulous neb mcp` (enabled) | your client configs |
| **[USER] CI (Phase 24)** | **COMPLETED** (pushed by user 2026-11-20) | — | Run 44725634691 (`ef70c89`, 42A+42B) and 44726771948 (`47ff52f`, 42C) green on ubuntu/macOS/Windows | — |
| **[CLAUDE] LIVE (Phase 24)** | **COMPLETED** (2026-11-20) | — | 5/5 runs passed; see "Phase 24 live check". Side finding TASK-0100 pending (agy headless needs a `mcp(nebulous/<tool>)` allow rule) | this plan file |
| **STEP 43A** | **COMPLETED** (`2d340d8`) | `/goal` | `neb init` ignores the timer files (Gemini; validate-step PASS; Advances TASK-0088, closed) | `crates/backend_core/src/storage/mod.rs` |
| **STEP 43B** | **COMPLETED** (`bca1277`) | `/goal` | Docs list only tested MCP clients; agy headless allow rule (Gemini; validate-step PASS; Advances TASK-0095, TASK-0100, closed) | `README.md`, `docs/USAGE.md` |
| **STEP 44A** | **COMPLETED** (`ee1f2a6`) | `/goal` | Update `url` (`>= 2.5.9`) in `Cargo.lock` to resolve `RUSTSEC-2026-0592` (`BUG-0008` / `AUD-01`; `TASK-0117` closed) | `Cargo.lock` |
| **STEP 44B** | **COMPLETED** (`97a5781`) | `/goal` | Fix `scripts/phased-step.sh` `code_stamp()` under `set -euo pipefail`, portable `find`, and backtick stripping in `ALLOWED` (`BUG-0016` / `AUD-09`; `TASK-0118` closed) | `scripts/phased-step.sh`, `scripts/validate-step.sh` |
| **STEP 44C** | **COMPLETED** (`7c12853`) | `/goal` | Align `.orrery/skills/repo-rules.md` and `.gitignore` with phased workflow (applied host-side because Orrery masks `.orrery/`; `TASK-0115`, `TASK-0119` closed) | `.orrery/skills/repo-rules.md`, `.gitignore` |
| **STEP 44D** | **COMPLETED** (`c20aa19`) | `/goal` | Fix `docs/USAGE.md` removed `--all` examples on `neb git commit` and `neb sync` (`BUG-0017` / `AUD-10`; `TASK-0120` closed) | `docs/USAGE.md` |
| **STEP 44E** | **COMPLETED** (`2db073f`) | `/goal` | Replace `std::env::set_var`/`remove_var` in `storage_integration.rs` with `*_in`/`*_from`/`*_to` helpers (`TASK-0116` test-hazard slice / `AUD-33`; `TASK-0121` closed) | `crates/backend_core/tests/storage_integration.rs` |
| **STEP 45A** | **COMPLETED** (`d23d44e`) | `/goal` | Exclude planning, audit, and tracker prose in `scripts/check-neutral.sh` while keeping source, tests, scripts, and user docs enforced (`TASK-0123`; `TASK-0124` closed) | `scripts/check-neutral.sh` |
| **STEP 45B** | **COMPLETED** (`2bf9236`) | `/goal` | Fix `move_item_to_phase` to remove `TASK-*` stubs via `ScopeItems::remove_id` with regression test (`BUG-0012` / `AUD-05`; `TASK-0125` closed) | `crates/backend_core/src/workflow/lifecycle.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| **STEP 45C** | **COMPLETED** (`4abb0e3`) | `/goal` | Canonicalize `HELD_WRITE_LOCKS` and `PROJECT_CACHE` keys to prevent same-thread 30s self-deadlock (`BUG-0013` / `AUD-06`; `TASK-0126` closed) | `crates/backend_core/src/storage/mod.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 45D** | **COMPLETED** (`f94325e`) | `/boost` | Reject symlinked `.tracker/` roots in `discover` and `guard_tracker_path` with Unix `O_NOFOLLOW` hardening and containment tests (`BUG-0010` / `AUD-03`; `TASK-0127` closed) | `crates/backend_core/Cargo.toml`, `Cargo.lock`, `crates/backend_core/src/storage/mod.rs`, `crates/backend_core/tests/path_containment.rs` |
| **STEP 45E** | **COMPLETED** (`1002a15`) | `/boost` | Lock public `TrackerStorage` mutators, `read_active_timer` quarantine, `tool_pin_stable_release`, and `projects.toml` registry (`projects.lock`) (`BUG-0020` / `AUD-17`; `TASK-0128` closed) | `crates/backend_core/src/storage/mod.rs`, `crates/backend_core/src/models/registry.rs`, `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 45F** | **COMPLETED** (`17474a8`) | `/goal` | Add `AtomicBool` shutdown signal to `TrackerWatcher::drop` (including contended lock waits) and skip symlinks in `hash_index_files` (`BUG-0023` / `AUD-20`; `TASK-0129` closed) | `crates/backend_core/src/watcher/mod.rs`, `crates/backend_core/tests/watcher_integration.rs` |
| **STEP 46A** | **COMPLETED** (`68bbb7c`) | `/goal` | Fix read-only registry lookups (`projects.lock` shared/read-only open, per-path lock map, no prune write on read) (`BUG-0033`, `TASK-0130` closed) | `crates/backend_core/src/models/registry.rs`, `crates/backend_core/tests/storage_integration.rs` |
| **STEP 46B** | **COMPLETED** (`da89c2f`) | `/boost` | Add mode-aware `atomic_write_bytes_with_mode` (`0o600`), propagate `ensure_secure_dir` errors, reject implicit `temp_dir()` vault fallback, and harden Windows replace/ACL handling (`BUG-0009` / `AUD-02`, `TASK-0131` closed) | `crates/backend_core/src/storage/atomic.rs`, `crates/backend_core/src/storage/mod.rs`, `crates/backend_core/src/crypto/mod.rs` |
| **STEP 46C** | **COMPLETED** (`635e541`) | `/boost` | Fix `SecureStorage` keyring/vault split-brain and route `git_sync` PAT functions exclusively through `SecureStorage` (`BUG-0011` / `AUD-04`, `TASK-0132` closed) | `crates/backend_core/src/crypto/mod.rs`, `crates/backend_core/src/git_sync/mod.rs`, `crates/backend_core/tests/credential_hygiene.rs` |
| **STEP 46D** | **COMPLETED** (`c1be53e`) | `/boost` | Pin `libgit2` PAT credentials and disable cross-host redirects (`RemoteRedirect::None`) to the configured HTTPS remote host (`BUG-0019` / `AUD-14`; `TASK-0133` closed; follow-up `BUG-0035`/`BUG-0036` in `STEP 47A`) | `crates/backend_core/src/git_sync/mod.rs`, `crates/backend_core/tests/credential_hygiene.rs` |
| **STEP 46E** | **COMPLETED** (`5c9a9b0`) | `/goal` | Sanitize all single-line untrusted fields and Unicode bidi format chars in `agent_context::synthesize_bounded_prompt` (`BUG-0018` / `AUD-12`, `TASK-0134` closed) | `crates/backend_core/src/agent_context/mod.rs`, `crates/backend_core/tests/workflow_integration.rs` |


### Future phases

Outlines only (goal, done criteria, and step titles). `/phase-plan` writes the contract step blocks just before each phase starts.

- **Phase 30 — Backend Resilience, Schema Compat, Planning & Assist Fixes (Medium-Risk Core)**
  - **Goal:** Ensure single corrupt/future-schema files, multi-phase plans, and edge-case timers/paths never crash or corrupt core backend operations.
  - **Done when:** `reconcile_indexes` and `compute_metrics_at` skip-and-report bad `phase.toml`/`scope.toml`/`item.toml` files and guard non-finite pace ratios; `compat` stops re-injecting default fields or downgrading `schema_version`; `parse_roadmap` handles code fences and escaped pipes; `import_plan` scopes `source_ref` per phase and skips malformed `item.toml`; `assist`/`timer`/`lifecycle` remove redundant I/O, `u32` truncation, and double timer writes; minor `registry`/`git_sync`/`ingestion` path and error edge cases are fixed.
  - **Steps:**
    - Skip-and-report bad `phase.toml`/`scope.toml`/`item.toml` in `reconcile_indexes` and `compute_metrics_at`, and guard non-finite pace metrics (`BUG-0024` / `AUD-21`)
    - Fix `compat::unknown_keys` default-field re-injection and per-tier `MAX_*_SCHEMA_VERSION` writes (`BUG-0025` / `AUD-22`)
    - Harden `parse_roadmap` code-fence/escape handling, scope `import_plan` `source_ref` per phase, and skip malformed `item.toml` in `source_ref` scans (`BUG-0026` / `AUD-23`, `TASK-0093`)
    - Share `TrackerIndex` in `predict_defaults`, saturate timer elapsed minutes, remove duplicate timer call in `start_work_item`, and fix `suggest_next_stage` (`BUG-0030` / `AUD-27`)
    - Fix `registry.rs`/`storage` path normalization, `is_temp_atomic_path`, auth retry doubling, `ingestion` symlink/parse/`ensure_items_dir` hygiene, and `.gitignore` legacy entry (`BUG-0032` / `AUD-32`)

- **Phase 31 — MCP Protocol & Tool Contract Hardening (Medium-Risk MCP)**
  - **Goal:** Harden MCP framing, parameter validation, atomicity, path privacy, and session isolation without breaking valid existing clients or Orrery hooks.
  - **Done when:** Truncated stdio frames return clean frame errors instead of crashing `serve_io`; all 24 tools enforce `deny_unknown_fields` (preserving `orrery` support), strict enum validation, and `resolve_phase_alias`; multi-step MCP writes are locked/atomic; responses emit workspace-relative paths; `git_fetch` `dry_run` matches docs/changelog; `switch_project` isolates session state unless persistence is requested.
  - **Steps:**
    - Fix `read_next_jsonrpc_message` EOF/terminator handling so truncated stdio frames never crash `serve_io` (`BUG-0021` / `AUD-18`)
    - Enforce `deny_unknown_fields`, strict enum checks, and `resolve_phase_alias` across all MCP tools (`BUG-0021` / `AUD-18`)
    - Atomize/lock multi-write MCP handlers (including `BUG-0034` sibling component RMW locks), return workspace-relative paths, align `git_fetch` `dry_run` contract, and isolate `switch_project` (`BUG-0021` / `AUD-18`, `BUG-0034`)

- **Phase 32 — CLI & TUI Bug Fixes & Stability (Routine-to-Medium UI/CLI)**
  - **Goal:** Fix clipboard X11 hangs, CLI output/error-swallowing bugs, user-reported TUI input bugs, and TUI performance, layout, and panic-hook defects with headless regression tests.
  - **Done when:** `arboard` never hangs on stalled X11 sockets; CLI `sync`/`start`/`move`/`sync-index`/`init`/`timer`/`component init` handle SHAs, `--root`, case-insensitive IDs, errors, and `--json` cleanly; TUI description input wraps text and supports line breaks, severity selection highlights cleanly, hash IDs never clip, `CloseModal` uses Unicode display width, custom phases work, `/` filtering avoids per-keystroke disk reads, and the panic hook restores on exit.
  - **Steps:**
    - Bound `arboard` clipboard calls with a worker-thread timeout and OSC 52 fallback (`BUG-0022` / `AUD-19`)
    - Fix CLI correctness and error-swallowing bugs in `handle_sync`, `handle_start`, `handle_move`, `handle_sync_index`, `handle_init`, and `Rollback` (`BUG-0027` / `AUD-24`)
    - Fix CLI hygiene: zeroize `NEB_GIT_PAT`, `--json` stderr warnings, case-insensitive timer IDs, strict `ComponentCommand::Init --type`, Linux-gated Wayland feature, and `main.rs` JSON escaping (`BUG-0031` / `AUD-31`)
    - Fix TUI input modals: description text wrapping, description line-break input, and severity field highlight display (`BUG-0005`, `BUG-0006`, `BUG-0007`)
    - Fix TUI performance, dynamic phases, error surfacing, panic-hook restoration, dynamic hash-ID column width, and `CloseModal` Unicode truncation (`BUG-0028` / `AUD-25`, `BUG-0029` / `AUD-26`)

- **Phase 33 — CI/Release Supply-Chain Hardening, Packaging & Documentation Parity (Routine)**
  - **Goal:** Harden GitHub workflows and installer/release scripts and reconcile all documentation against the verified codebase.
  - **Done when:** All GitHub Actions are SHA-pinned with minimal permissions and `--locked`; CI and release enforce `cargo deny check advisories bans licenses sources`; release tarballs bundle only `docs/USAGE.md` and `docs/ARCHITECTURE.md`; `bump-version.sh` validates before mutating; `install.sh` handles `--prefix /` and `NEBULOUS_BIN_DIR` cleanly; `README.md`, `docs/USAGE.md`, `docs/ARCHITECTURE.md`, `args.rs`, TUI footer, and `CHANGELOG.md` are fully aligned.
  - **Steps:**
    - SHA-pin GitHub Actions, enforce `cargo deny` licenses/sources, add `--locked`/permissions, restrict release docs packaging, and fix `bump-version.sh` and `install.sh` (`TASK-0112` / `AUD-16`)
    - Reconcile `README.md`, `docs/USAGE.md`, `docs/ARCHITECTURE.md`, `args.rs` help text, TUI footer `[r] Reload`, and `CHANGELOG.md` (`TASK-0114` / `AUD-29`)

- **Phase 34 — Dedicated Scope-Lapse Cleanup (Late / Isolated)**
  - **Goal:** Clean up leftover OS/ISO rolling-release artifacts and evaluate Orrery/Argon coupling in isolation after all bug fixes are stable and CI-green.
  - **Done when:** User decision on Suite-3 MCP release-pin tools (neutralize-in-place vs feature-gate/remove) is executed with zero unapproved regressions; OS-distro leftovers (`orbit_image`, `apt_sid`, `kernel_custom`, `libcosmic`) are removed; `scripts/check-neutral.sh` guards against re-introduction.
  - **Steps:**
    - Remove leftover OS/ISO rolling-release identifiers and execute approved Suite-3/Orrery coupling cleanup (`TASK-0109` / `AUD-11`)

- **Phase 35 — MCP Registry & `.mcpb` Release (`v0.2.0`, Resuming Deferred Phase 25)**
  - **Goal:** Resume and ship the deferred Phase 25 MCP Registry (`io.github.TheInfoMoose/nebulous`) and Linux x86_64 `.mcpb` bundle release as `v0.2.0` on a fully audited, green codebase.
  - **Done when:** `mcpb/manifest.json` and `server.json` are committed; `release.yml` packs, hashes, smoke-tests, and publishes `.mcpb` behind `mcp-registry` approval; `CHANGELOG.md`, `README.md`, and `docs/USAGE.md` document `v0.2.0` bundling; CI matrix and live registry listing pass.
  - **Steps:**
    - Commit MCP bundle manifest and `server.json` templates (`TASK-0103` / deferred `STEP 43C`)
    - Update `release.yml` to pack, hash, and smoke-test `.mcpb` (`TASK-0104` / deferred `STEP 43D`)
    - Add `publish-registry` job behind `mcp-registry` approval (`TASK-0105` / deferred `STEP 43E`)
    - Update `CHANGELOG.md`, `README.md`, and `docs/USAGE.md` for `v0.2.0` release (`TASK-0106` / deferred `STEP 43F`)
    - User CI matrix, `mcp-registry` environment, `v0.2.0` tag/release, and bundle/listing verification (`TASK-0107`, `TASK-0108`)

### Open Questions

**Recorded decisions (2026-11-25, user):**
- **Workflow adoption:** Adopted `**Workflow:** phased`, archived Phases 1–24 step blocks to `archive/STEPPED_IMPLEMENTATION_PLAN_phases_1_24.md`, and scheduled `.orrery/`/phased alignment + `AUD-09`/`BUG-0016` first in Phase 26.
- **Priority & Phase 25:** Priority shifted to audit bug fixes and regression aversion; remaining Phase 25 (`STEP 43C–43F` + `v0.2.0` MCP Registry release) is deferred to Phase 35.
- **Refactor boundary:** Bug fixes, security/stability hardening, and regression tests run first; pure structural refactors (`AUD-28` `commands/mod.rs` split, `AUD-15` `ssh-key` RC, `AUD-33` `lib.rs` wildcard re-exports) stay parked.
- **Scope lapse (`AUD-11`):** Deferred to isolated Phase 34 so bug-fix phases remain strictly non-breaking and regression-averse.

**Recorded decisions (2026-11-26, user):**
- **Phase 28 scope:** Fold `BUG-0033` (Phase 27 read-only registry regression) into Phase 28 alongside the 5 Security Hardening steps (6 steps total); defer `BUG-0034` (sibling component RMW write locks) to Phase 30/31.
- **`AUD-13` / `TASK-0110` KDF migration:** Keep `NEBULOUS_VAULT_MASTER_KEY` KDF migration parked; Phase 28 executes only the non-breaking zeroize + SSH passphrase guard slice.

**Recorded decisions (2026-11-27, user):**
- **Phase 29 scope:** Combined 6-step Phase 29 folding Phase 28 fallout (`BUG-0035`/`BUG-0036`, re-split `STEP 46F` as `STEP 47B`, and test defects `BUG-0037`/`BUG-0038`) ahead of Git Sync & Documented Commit Atomicity (`BUG-0014`, `BUG-0004`, `BUG-0015`).

**Open before future phases:**
1. **Before Phase 34 (`AUD-11` / `TASK-0109` Suite-3 MCP tools):** Should the 4 Suite-3 release-pin MCP tools (`record_rolling_update`, `record_verification_run`, `pin_stable_release`, `rollback_to_pinned_release`) be neutralized in place (non-breaking) or feature-gated/removed?

### Parked

Tracked in neb: `neb list --status pending`. Reviewed at phase end; only picked items become steps.

**Explicitly parked for regression aversion (2026-11-25):**
- `TASK-0113` (`AUD-28`): Split monolithic `commands/mod.rs` (3,146 LOC), reconcile overlapping `sync`/`stage` commands, and wire/drop hidden `--use` flag.
- `TASK-0111` (`AUD-15`): Pre-release `ssh-key 0.7.0-rc.11` dual RustCrypto stack (revisit when `ssh-key 0.7.0` is stable).
- `TASK-0116` (`AUD-33` API-surface slice): Replace wildcard `pub use <mod>::*;` re-exports in `crates/backend_core/src/lib.rs` (test-suite `env::set_var` hazards and missing regression tests are scheduled in Phases 26–30).
- `TASK-0085` (`jiff` 1.0), `TASK-0086` (pending items in stagnant warnings), `TASK-0094` (MCP `2026-07-28` `server/discover` handshake).

---

## Phase 26 Step Contracts

### STEP 44A: Update `url` (`>= 2.5.9`) in `Cargo.lock` (`RUSTSEC-2026-0592`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 44A: Update `url` in `Cargo.lock` to `>= 2.5.9` so `RUSTSEC-2026-0592` is resolved and `cargo deny` passes cleanly (`BUG-0008` / `AUD-01`).

[SCOPE_LOCK]
ALLOWED_FILES:
  - Cargo.lock
READ_ONLY_REFS:
  - deny.toml:1-42
  - Cargo.toml:1-60
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not modify Cargo.toml, deny.toml, or any source file. Do not add advisory ignores to deny.toml. Never touch /srv/devshare/NovOS.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal: Advance `url` in `Cargo.lock` from `2.5.8` to `>= 2.5.9` so `RUSTSEC-2026-0592` is eliminated and `cargo deny check advisories bans licenses sources` exits 0 with zero ignores.

Contract & Behavior:
  1. `Cargo.lock`: run `cargo update -p url` (and only `url`) so every `url` package entry in `Cargo.lock` has version `>= 2.5.9`. Do not bump unrelated dependencies.
  2. `deny.toml` remains unchanged (`ignore = []`).

Must still work:
  - Workspace formatting, clippy (`-D warnings`), and all unit/integration tests (`cargo test --workspace`).
  - `git2` remote URL parsing and credential hygiene tests.

Drops: none

Tests:
  - Lockfile check: parse `Cargo.lock` and verify every `url` package entry has semantic version `>= (2, 5, 9)` -> passes.
  - Advisory gate: `cargo deny check advisories bans licenses sources` -> exits 0 with zero advisories.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && python3 -c 'import tomllib; pkgs=[p["version"] for p in tomllib.loads(open("Cargo.lock").read())["package"] if p["name"]=="url"]; assert pkgs and all(tuple(int(x) for x in v.split(".")) >= (2, 5, 9) for v in pkgs), pkgs' && cargo deny check advisories bans licenses sources

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 44B: Fix `scripts/phased-step.sh` `code_stamp()`, portable `find`, and `ALLOWED` backtick stripping

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 44B: Make `scripts/phased-step.sh` survive clean/`.tracker/`-only trees under `set -euo pipefail`, remove GNU-only `find -printf`, and strip Markdown backticks in `ALLOWED_FILES` extraction (`BUG-0016` / `AUD-09`).

[SCOPE_LOCK]
ALLOWED_FILES:
  - scripts/phased-step.sh
  - scripts/validate-step.sh
READ_ONLY_REFS:
  - docs/codebase-audit-report.md:124-134 (AUD-09)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not change step-block tag names, commit message format, or amendment rules. Never touch /srv/devshare/NovOS.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal: Ensure `scripts/phased-step.sh` never aborts on clean or `.tracker/`-only trees, runs portably on macOS/BSD `find`, and strips surrounding backticks from `ALLOWED_FILES` entries in both `scripts/phased-step.sh` and `scripts/validate-step.sh`.

Contract & Behavior:
  1. `scripts/phased-step.sh` (`code_stamp()`): under `set -euo pipefail`, must exit 0 and output a 64-char SHA-256 hex digest even when `fingerprint` produces zero non-`.tracker/` lines (empty output or `.tracker/`-only paths). Non-`.tracker/` lines must still be hashed identically.
  2. `scripts/phased-step.sh` (`tracker_items()`): replace GNU-only `find ... -printf '%h\n'` with a POSIX/BSD-portable implementation that emits the same sorted unique item IDs from `.tracker/phases/*/domains/*/items/*/item.toml`.
  3. `scripts/phased-step.sh` and `scripts/validate-step.sh` (`ALLOWED` extraction): strip leading/trailing backticks (`` ` ``) from each extracted path token so `` - `crates/foo.rs` `` resolves to `crates/foo.rs`, while leaving plain paths, globs, and `(NEW)` handling unchanged.

Must still work:
  - `bash -n scripts/phased-step.sh` and `bash -n scripts/validate-step.sh`.
  - Normal `phased-step.sh` `start`, `check`, `commit`, `defer`, `status`, and `new-phase` flows on steps with code changes.
  - Existing unquoted `ALLOWED_FILES` paths, globs, and `Cargo.lock` auto-inclusion.

Drops: none

Tests:
  - `code_stamp_tracker_only`: invoke `code_stamp()` under `set -euo pipefail` with a `.tracker/`-only `fingerprint()` output -> exits 0 and emits a 64-character SHA-256 hex digest.
  - `tracker_items_portable`: verify `scripts/phased-step.sh` contains no `-printf` flag and parses `.tracker` item IDs cleanly.
  - `allowed_backtick_strip`: feed `` - `crates/backend_core/src/lib.rs` (NEW) `` into the `ALLOWED` extraction pipeline -> emits `crates/backend_core/src/lib.rs` without backticks.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && bash -n scripts/phased-step.sh && bash -n scripts/validate-step.sh && ! grep -q -- '-printf' scripts/phased-step.sh && bash -c 'set -euo pipefail; eval "$(sed -n "/^code_stamp()/,/^}/p" scripts/phased-step.sh)"; fingerprint() { printf "deadbeef  .tracker/project.toml\n"; }; stamp="$(code_stamp)"; test ${#stamp} -eq 64'

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 44C: Align `.orrery/skills/repo-rules.md` and `.gitignore` with phased workflow

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 44C: Update `.orrery/skills/repo-rules.md` and `.gitignore` to remove stale `/workspace/`, `"release tracker"`, and `"Argon block"` drift and align with `**Workflow:** phased` (`TASK-0115` / `AUD-30`).

[SCOPE_LOCK]
ALLOWED_FILES:
  - .orrery/skills/repo-rules.md
  - .gitignore
READ_ONLY_REFS:
  - .orrery/project.toml:1-40
  - .phased/plan-header.md:1-47
  - docs/codebase-audit-report.md:331-338 (AUD-30)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not modify Rust source files, STEPPED_IMPLEMENTATION_PLAN.md, or .tracker/. Never touch /srv/devshare/NovOS.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal: Align `.orrery/skills/repo-rules.md` with the phased workflow and neutral project identity, and ensure `.gitignore` does not ignore the canonical `STEPPED_IMPLEMENTATION_PLAN.md` file.

Contract & Behavior:
  1. `.orrery/skills/repo-rules.md`:
     - Describe Nebulous neutrally as a CLI/TUI-first, local-first, Git-backed project, bug, and task tracker (`neb`), removing `"release tracker"`.
     - Replace `/workspace/docs/USAGE.md`, `/workspace/docs/ARCHITECTURE.md`, and `/workspace/STEPPED_IMPLEMENTATION_PLAN.md` with repo-relative paths (`docs/USAGE.md`, `docs/ARCHITECTURE.md`, `STEPPED_IMPLEMENTATION_PLAN.md`).
     - Replace `"Argon block"` with `"step block"` and note that Neon files side findings/bugs as pending `neb` items rather than editing `.tracker/` by hand.
     - Retain all existing rules (gates, neutrality check, no edits outside repo or to plan file, placeholder test data).
  2. `.gitignore`: ensure `STEPPED_IMPLEMENTATION_PLAN.md` is not ignored (remove the `# Dev md files` / `STEPPED_IMPLEMENTATION_PLAN.md` ignore entry if still present) and keep all Rust/tracker/OS ignore rules intact.

Must still work:
  - `bash scripts/check-neutral.sh` exits 0.
  - Existing `.gitignore` rules for `/target/`, `**/*.rs.bk`, `/.tracker/.timer.toml`, `/.tracker/.timer.toml.bak`, OS, and editor files.

Drops: none

Tests:
  - `repo_rules_phased_alignment`: inspect `.orrery/skills/repo-rules.md` -> no `/workspace/`, no `release tracker`, no `Argon block`, and references repo-relative `STEPPED_IMPLEMENTATION_PLAN.md` and `docs/USAGE.md`.
  - `gitignore_allows_plan`: `git check-ignore STEPPED_IMPLEMENTATION_PLAN.md` -> exits 1 (not ignored).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && bash scripts/check-neutral.sh && ! grep -qE '/workspace/|release tracker|Argon block' .orrery/skills/repo-rules.md && grep -q 'STEPPED_IMPLEMENTATION_PLAN.md' .orrery/skills/repo-rules.md && ! git check-ignore -q STEPPED_IMPLEMENTATION_PLAN.md

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 44D: Fix `docs/USAGE.md` removed `--all` examples on `neb git commit` and `neb sync`

* **Recommended Command:** `/goal`
* **Restaged:** 1

```text
[STEP_ID]
STEP 44D: Replace removed `--all` flags on `neb git commit` and `neb sync` in `docs/USAGE.md` with valid `--path` and `.tracker/`-only examples (`BUG-0017` / `AUD-10`).

[SCOPE_LOCK]
ALLOWED_FILES:
  - docs/USAGE.md
READ_ONLY_REFS:
  - crates/nebulous_cli/src/args.rs:300-324 (Commands::Sync)
  - crates/nebulous_cli/src/args.rs:686-717 (GitCommand::Commit)
  - crates/nebulous_cli/src/lib.rs:275-290 (cli_rejects_removed_all_flag)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not modify CLI argument parsing or unrelated sections of docs/USAGE.md. Never touch /srv/devshare/NovOS.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal: Ensure every `neb git commit` and `neb sync` example in `docs/USAGE.md` uses supported flags (`--path <FILE>` or `.tracker/`-only default) and never passes the removed `--all` flag.

Contract & Behavior:
  1. `docs/USAGE.md` (§5 Git Operations, lines 165–170):
     - Replace `--all` on the `neb git commit` example with an explicit project-relative `--path <FILE>` example (e.g., `--path crates/backend_core/src/models/item.rs`).
     - Replace `--all` on the `neb sync` example with an explicit `--path <FILE>` example, and note (in comment or example) that omitting `--path` stages and commits `.tracker/` metadata only.
  2. Zero occurrences of `--all` remain anywhere in `docs/USAGE.md`.
  3. Keep neutrality clean on `docs/USAGE.md` (`bash scripts/check-neutral.sh docs/USAGE.md` passes).

Must still work:
  - All other sections and command examples in `docs/USAGE.md`.
  - `bash scripts/check-neutral.sh docs/USAGE.md`.

Drops: none

Tests:
  - `usage_git_examples_valid`: verify `docs/USAGE.md` has zero `--all` occurrences and includes `--path` examples for both `neb git commit` and `neb sync`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && bash scripts/check-neutral.sh docs/USAGE.md && ! grep -q -- '--all' docs/USAGE.md && grep -qE 'neb git commit.*--path' docs/USAGE.md && grep -qE 'neb sync.*--path' docs/USAGE.md

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 44E: Replace `std::env::set_var`/`remove_var` in `storage_integration.rs` with explicit-path helpers

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 44E: Replace `std::env::set_var`/`remove_var("NEBULOUS_CONFIG_DIR")` in `crates/backend_core/tests/storage_integration.rs` with `*_in`/`*_from`/`*_to` registry helpers (`TASK-0116` test-hazard slice / `AUD-33`).

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/registry.rs:377-479 (load_global_registry_from, save_global_registry_to, register_project_in, remove_project_in, set_active_project_in)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not modify production code in crates/backend_core/src/ or weaken any existing test assertions. Never touch /srv/devshare/NovOS.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal: Eliminate process-global `NEBULOUS_CONFIG_DIR` environment variable mutation in `crates/backend_core/tests/storage_integration.rs` so parallel integration tests run without env-race hazards.

Contract & Behavior:
  1. `crates/backend_core/tests/storage_integration.rs` (`test_ingestion_subpath_duplicate_leaf_ids_and_promote_all`, lines 1456–1479):
     - Remove `std::env::set_var("NEBULOUS_CONFIG_DIR", &cfg_dir)` and `std::env::remove_var("NEBULOUS_CONFIG_DIR")`.
     - Replace `register_project`, `load_global_registry`, `save_global_registry`, `set_active_project`, and `remove_project` with `register_project_in(&cfg_dir, ...)`, `load_global_registry_from(&cfg_dir)`, `save_global_registry_to(&cfg_dir, &loaded_reg)`, `set_active_project_in(&cfg_dir, ...)`, and `remove_project_in(&cfg_dir, ...)`.
  2. Preserve every existing assertion in `test_ingestion_subpath_duplicate_leaf_ids_and_promote_all` (`reg_entry.id == "dupleafworkspace"`, `active_project_id`, `switched.id`, `removed.id`, and final `projects.is_empty()`).
  3. Zero occurrences of `set_var` or `remove_var` remain in `crates/backend_core/tests/storage_integration.rs`.

Must still work:
  - All tests in `crates/backend_core/tests/storage_integration.rs` and the full workspace test suite (`cargo test --workspace`).

Drops: none

Tests:
  - `test_ingestion_subpath_duplicate_leaf_ids_and_promote_all`: runs `register_project_in`, `load_global_registry_from`, `save_global_registry_to`, `set_active_project_in`, and `remove_project_in` against `&cfg_dir` -> passes under `cargo test -p backend_core --test storage_integration` with zero environment mutation.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ! grep -qE 'set_var|remove_var' crates/backend_core/tests/storage_integration.rs && grep -q 'register_project_in' crates/backend_core/tests/storage_integration.rs && grep -q 'load_global_registry_from' crates/backend_core/tests/storage_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

## Phase 27 Step Contracts

### STEP 45A: Exclude planning, audit, and tracker prose in `scripts/check-neutral.sh`

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 45A: Exclude `STEPPED_IMPLEMENTATION_PLAN.md`, `docs/codebase-audit-report.md`, and `.tracker/` from default repo-wide scans in `scripts/check-neutral.sh` so Ubuntu CI passes while keeping source, tests, scripts, and user docs enforced (`TASK-0123`).

[SCOPE_LOCK]
ALLOWED_FILES:
  - scripts/check-neutral.sh
READ_ONLY_REFS:
  - .github/workflows/ci.yml (Ubuntu neutrality gate invocation)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not relax `PATTERN`, add entries to `ALLOWLIST`, or weaken explicit-file checks when arguments are passed to `scripts/check-neutral.sh`. Never touch the forbidden path listed in the plan header.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal: Restore a passing repo-wide `bash scripts/check-neutral.sh` gate on Ubuntu CI without weakening neutrality enforcement on Rust source, tests, scripts, CI workflows, or user-facing documentation (`README.md`, `docs/USAGE.md`, `docs/ARCHITECTURE.md`, `CHANGELOG.md`).

Contract & Behavior:
  1. `scripts/check-neutral.sh` (default `no_args == 1` `git ls-files` filter):
     - Extend the `grep -vE` exclusion filter to skip `^STEPPED_IMPLEMENTATION_PLAN\.md$`, `^docs/codebase-audit-report\.md$`, and `^\.tracker/` in addition to the existing `^archive/`, `^Cargo\.lock$`, and `^scripts/check-neutral\.sh$` exclusions.
     - Keep `PATTERN` and the empty `ALLOWLIST` unchanged.
     - Keep explicit-argument mode (`bash scripts/check-neutral.sh <file> ...`) unchanged so passing any file path directly still scans that file and fails on missing files or hits.
  2. Running `bash scripts/check-neutral.sh` with no arguments from repo root must exit `0` and print `check-neutral: OK (0 allowlisted file(s), 0 line(s) left to scrub)`.

Must still work:
  - `bash scripts/check-neutral.sh README.md docs/USAGE.md` exits `0`.
  - `bash scripts/check-neutral.sh` fails (non-zero) when given an explicit file containing a forbidden token or a nonexistent path.
  - All Rust source, tests, scripts, workflows, and user docs remain scanned in default no-arg mode.

Drops: none

Tests:
  - Shell verification in `POST_GATE`: `bash -n scripts/check-neutral.sh` and `bash scripts/check-neutral.sh` exit `0` on the current repository.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && bash -n scripts/check-neutral.sh && bash scripts/check-neutral.sh

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 45B: Fix `move_item_to_phase` to remove `TASK-*` stubs via `ScopeItems::remove_id`

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 45B: Fix `remove_item_from_parent_indexes` in `crates/backend_core/src/workflow/lifecycle.rs` to use `ScopeItems::remove_id` so moving `TASK-*` items leaves zero ghost stubs, with a regression test (`BUG-0012` / `AUD-05`).

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/workflow/lifecycle.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/scope.rs:85-105 (ScopeItems::remove_id)
  - crates/backend_core/src/workflow/lifecycle.rs:920-1043 (move_item_to_phase, remove_item_from_parent_indexes, sync_parent_indexes_for_item)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not change public signatures on `TrackerStorage`, `ScopeItems`, or `ItemLocation`. Never touch the forbidden path listed in the plan header.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal: Moving a `TASK-*` item between phases removes its stub from the source `scope.toml`, updates the source `phase.toml` summary counts accurately, inserts it into the target phase, and keeps subsequent lookups working without requiring `reconcile_indexes`.

Contract & Behavior:
  1. `crates/backend_core/src/workflow/lifecycle.rs` (`remove_item_from_parent_indexes`):
     - Replace the manual `.bugs.retain(...)` and `.features.retain(...)` calls with `scope_manifest.items.remove_id(item_id);` so all `ItemType` variants (`Bug`, `Feature`, `Task`) are removed uniformly, matching `sync_parent_indexes_for_item`.
     - Preserve writing the updated `scope.toml` and recomputing the source `phase.toml` summary via `recompute_phase_summary_with_scope`.

Must still work:
  - Moving `BUG-*` and `FEAT-*` items across phases (existing move tests in `workflow_integration.rs`).
  - Active timer `phase_id` update when moving the item with an active timer.

Drops: none

Tests:
  - `move_task_between_phases_removes_source_stub_and_updates_summary` in `crates/backend_core/tests/workflow_integration.rs`: create a `TASK-*` item in `pre_production`/`inbox` via `create_item`, call `move_item_to_phase(&task_id, "production")` -> source `scope.toml` `items.tasks` does not contain `task_id`, source `phase.toml` `summary.total_items == 0`, target `scope.toml` `items.tasks` contains `task_id`, target `phase.toml` `summary.total_items == 1`, and `load_item_detail_by_id(&task_id)` succeeds with `phase_id == "production"`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'remove_id(item_id)' crates/backend_core/src/workflow/lifecycle.rs && grep -q 'fn move_task_between_phases_removes_source_stub_and_updates_summary' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 45C: Canonicalize `HELD_WRITE_LOCKS` and `PROJECT_CACHE` keys

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 45C: Canonicalize the thread-local `HELD_WRITE_LOCKS` and `PROJECT_CACHE` key in `crates/backend_core/src/storage/mod.rs` so mixed relative/unnormalized/canonical `TrackerStorage` instances never self-deadlock on the same thread (`BUG-0013` / `AUD-06`).

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/mod.rs
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs:32-70 (HELD_WRITE_LOCKS, PROJECT_CACHE, ReentrantLockGuard)
  - crates/backend_core/src/storage/mod.rs:489-556 (lock_path, holds_write_lock, with_write_lock_timeout)
  - crates/backend_core/src/storage/mod.rs:719-767 (read_project, write_project)
  - crates/backend_core/src/storage/mod.rs:1339-1384 (canonicalize_existing_prefix)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not change the public return value of `TrackerStorage::lock_path(&self)` or weaken `PROJECT_DISK_READS` caching tests. Never touch the forbidden path listed in the plan header.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal: Nested same-thread `with_write_lock` / `with_write_lock_timeout` calls across two `TrackerStorage` instances pointing to the same `.tracker/` directory via different path spellings (e.g., canonical `discover` vs relative or `./` unnormalized `new`/`from_tracker_dir`) re-enter immediately without timing out and share the same `PROJECT_CACHE` entry.

Contract & Behavior:
  1. `crates/backend_core/src/storage/mod.rs`:
     - Add a single internal helper (e.g. `fn lock_key(&self) -> PathBuf`) that derives the canonicalized lock/cache key from `canonicalize_existing_prefix(&self.tracker_dir)` (falling back to `lexical_normalize_path(&self.tracker_dir)` if canonicalization errors) joined with `LOCK_FILE_NAME`.
     - Use that canonical key consistently in `holds_write_lock`, `with_write_lock_timeout` (both the reentrancy check and `ReentrantLockGuard::acquire`), `read_project`, and `write_project` (cache insert and error-path removal).
     - Keep `pub fn lock_path(&self) -> PathBuf` returning `self.tracker_dir.join(LOCK_FILE_NAME)` unchanged.

Must still work:
  - Existing lock timeout between distinct threads (`write_lock_times_out_when_contended`), error release (`write_lock_propagates_error_and_releases_lock`), and `project.toml` cache tests (`create_item_reads_project_toml_at_most_twice`, `project_cache_is_dropped_when_lock_releases`, `write_inside_lock_updates_cache`).
  - Locking before `.tracker/` exists on disk (`create_tracker_dir_all` inside `with_write_lock_timeout`).

Drops: none

Tests:
  - `nested_write_lock_with_mixed_path_spellings_is_reentrant` in `crates/backend_core/tests/storage_integration.rs`: initialize a tracker in a temp dir, construct `storage_canon = TrackerStorage::discover(temp.path())` and `storage_unnorm = TrackerStorage::new(temp.path().join("."))` (plus a `from_tracker_dir` variant with `.`), verify `storage_canon.with_write_lock(|_| storage_unnorm.with_write_lock_timeout(Duration::from_millis(200), |s| { let mut p = s.read_project()?; p.project.name = "updated".to_string(); s.write_project(&p)?; Ok(()) }))` succeeds without `LockTimeout` and `storage_canon.read_project()` inside the outer lock observes `"updated"`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'fn nested_write_lock_with_mixed_path_spellings_is_reentrant' crates/backend_core/tests/storage_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 45D: Reject symlinked `.tracker/` roots and harden Unix tracker opens with `O_NOFOLLOW`

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 45D: Reject symlinked `.tracker/` roots in `TrackerStorage::discover` and `guard_tracker_path`, harden Unix tracker file/lock opens with `O_NOFOLLOW`, and add containment regression tests (`BUG-0010` / `AUD-03`).

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/Cargo.toml
  - Cargo.lock
  - crates/backend_core/src/storage/mod.rs
  - crates/backend_core/tests/path_containment.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs:212-250 (TrackerStorage::discover)
  - crates/backend_core/src/storage/mod.rs:417-470 (guard_tracker_path, read_tracker_file, write_tracker_file, create_tracker_dir_*)
  - crates/backend_core/src/storage/mod.rs:507-556 (with_write_lock_timeout)
  - crates/backend_core/src/storage/mod.rs:1281-1384 (contained_path, resolve_contained, canonicalize_existing_prefix)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not weaken existing path containment checks or reject trackers whose ancestor directories (e.g. macOS `/var -> /private/var`) contain symlinks when `.tracker` itself is a real directory. Never touch the forbidden path listed in the plan header.

[PRE_GATE]
grep -q 'nested_write_lock_with_mixed_path_spellings_is_reentrant' crates/backend_core/tests/storage_integration.rs

[MUTATION_SPEC]
Goal: A symlinked `.tracker` root directory can no longer bypass `discover` or `guard_tracker_path` containment checks, and Unix tracker file reads and lock opens refuse final-component symlinks at the kernel open boundary via `O_NOFOLLOW`.

Contract & Behavior:
  1. `crates/backend_core/Cargo.toml` + `Cargo.lock`:
     - Add `libc = "0.2"` under `[target.'cfg(unix)'.dependencies]` (already present in `Cargo.lock`; do not add any other crate or change unrelated versions).
  2. `crates/backend_core/src/storage/mod.rs`:
     - `TrackerStorage::discover`: check `fs::symlink_metadata` on the `.tracker` candidate (both when `start_dir` itself names a symlinked `.tracker` before canonicalization and when `dir.join(".tracker")` is probed during ancestor walk). If the `.tracker` entry itself is a symlink, return `Err(TrackerError::ValidationError(...))` mentioning `"symlink"` rather than accepting or silently following it.
     - `guard_tracker_path`: before delegating to `contained_path`, check `fs::symlink_metadata(&self.tracker_dir)`; if it exists and `file_type().is_symlink()`, return `Err(TrackerError::ValidationError(...))` mentioning `"symlink"`.
     - Unix `O_NOFOLLOW` hardening: in `read_tracker_file` and `with_write_lock_timeout` (opening `guarded_lock_path`), apply `OpenOptionsExt::custom_flags(libc::O_NOFOLLOW)` under `#[cfg(unix)]` so a race swapping the final component for a symlink fails at `open(2)` instead of following it. Ensure `#[cfg(not(unix))]` builds cleanly with zero unused-import warnings.

Must still work:
  - Legitimate `.tracker/` directories located under symlinked ancestor directories (such as macOS `/var/folders/...` -> `/private/var/folders/...` or a workspace symlink pointing to a repo containing a real `.tracker/` directory).
  - All existing tests in `crates/backend_core/tests/path_containment.rs` and `cargo test --workspace`.

Drops: none

Tests:
  - `symlinked_tracker_root_rejected` in `crates/backend_core/tests/path_containment.rs` (`#[cfg(unix)]`): initialize a real tracker in an outside temp dir, create a workspace dir with `.tracker` symlinked to that outside `.tracker`, and verify:
    1. `TrackerStorage::discover(workspace.path())` returns `Err(TrackerError::ValidationError(_))` mentioning `"symlink"`.
    2. `TrackerStorage::discover(workspace.path().join(".tracker"))` returns `Err(TrackerError::ValidationError(_))` mentioning `"symlink"`.
    3. `TrackerStorage::new(workspace.path())` rejects `read_project()`, `write_item_desc(...)`, and `with_write_lock(...)` with `Err(TrackerError::ValidationError(_))` mentioning `"symlink"`, leaving the outside tracker unmodified.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'O_NOFOLLOW' crates/backend_core/src/storage/mod.rs && grep -q 'fn symlinked_tracker_root_rejected' crates/backend_core/tests/path_containment.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 45E: Lock public `TrackerStorage` mutators, timer quarantine, `tool_pin_stable_release`, and `projects.toml`

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 45E: Acquire `.tracker/.lock` inside public `TrackerStorage` mutators, `read_active_timer` quarantine, and MCP `tool_pin_stable_release`, and add cross-process `projects.lock` `fd_lock` serialization to `models/registry.rs` (`BUG-0020` / `AUD-17`).

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/mod.rs
  - crates/backend_core/src/models/registry.rs
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/crypto/mod.rs:55-95 (with_vault_lock fd_lock pattern)
  - crates/backend_core/src/storage/mod.rs:274-390 (init)
  - crates/backend_core/src/storage/mod.rs:472-478,746-946,1053-1131 (public mutators, read_active_timer, resolve_phase_and_scope)
  - crates/backend_core/src/models/registry.rs:16-40,374-576 (registry_lock, load/save/register/remove/set_active/credential helpers)
  - crates/backend_core/src/mcp/mod.rs:1971-2050 (tool_pin_stable_release)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not change public method signatures on `TrackerStorage`, `GlobalProjectRegistry`, or MCP tool schemas. Never touch the forbidden path listed in the plan header.

[PRE_GATE]
grep -q 'symlinked_tracker_root_rejected' crates/backend_core/tests/path_containment.rs

[MUTATION_SPEC]
Goal: All public `TrackerStorage` mutators, corrupt timer quarantine, MCP release pinning, and global `projects.toml` operations are serialized across processes with file locks instead of racing unlocked or relying solely on an in-process mutex.

Contract & Behavior:
  1. `crates/backend_core/src/storage/mod.rs`:
     - Acquire `with_write_lock` inside public mutators: `init` (wrapping the filesystem writes after validation), `ensure_items_dir`, `write_project`, `write_phase`, `write_scope`, `write_item_manifest`, `write_item_desc`, `write_item_detail`, `write_active_timer`, `clear_active_timer`, and `resolve_phase_and_scope`. Keep `pub(crate)` internal helpers unlocked where callers already hold the lock or re-enter cleanly via reentrant `with_write_lock`.
     - In `read_active_timer`: keep the happy path (missing or valid `.timer.toml`) lock-free; when parsing fails, acquire `with_write_lock`, re-check `.timer.toml` (return `Ok(None)` if removed or `Ok(Some(timer))` if a concurrent writer replaced it with valid TOML), and only then call `quarantine_active_timer()` under the lock.
  2. `crates/backend_core/src/models/registry.rs`:
     - Export `pub const GLOBAL_PROJECTS_LOCK_FILE_NAME: &str = "projects.lock";`.
     - Serialize all registry file operations (`load_global_registry_from`, `save_global_registry_to`, `register_project_in`, `remove_project_in`, `set_active_project_in`, `credential_accounts_for_root_in`, `bind_credential_account_in`) with both the in-process `registry_lock()` mutex and a cross-process `fd_lock::RwLock` on sibling `projects.lock` (using `DEFAULT_LOCK_TIMEOUT` and returning `TrackerError::LockTimeout` on contention, matching `with_vault_lock`).
     - For read-only loads (`load_global_registry_from`, `credential_accounts_for_root_in`), if the registry file and its parent directory do not exist yet, return default/empty without creating the parent directory on disk.
     - In `load_registry_unlocked`, when `registry.prune_unsafe_projects()` returns `true`, propagate any error from `save_registry_unlocked(path, &registry)?` instead of discarding it with `let _ = ...`.
  3. `crates/backend_core/src/mcp/mod.rs`:
     - Wrap `tool_pin_stable_release`'s `read_component_manifest`, optional `write_component_manifest`, and `pin_component_release` sequence in a single `storage.with_write_lock(|storage| ...)`.

Must still work:
  - Reentrant nested lock calls inside `create_item`, `reconcile_indexes`, `move_item_to_phase`, `init_standard`, and `PROJECT_DISK_READS` caching (`create_item_reads_project_toml_at_most_twice`).
  - Corrupt `.timer.toml` quarantine tests (`workflow_integration.rs`) and `path_containment.rs` symlink rejections.
  - All existing `registry` and MCP tests (`cli_integration.rs`, `workflow_integration.rs`, `mcp_protocol.rs`).

Drops: none

Tests:
  - `public_mutator_waits_for_write_lock` in `crates/backend_core/tests/storage_integration.rs`: hold `.tracker/.lock` on thread A for ~200ms while thread B calls `storage.write_item_desc(...)` -> thread B blocks until thread A releases the lock, then succeeds and writes the description.
  - `registry_writes_serialize_on_projects_lock` in `crates/backend_core/tests/storage_integration.rs`: hold an exclusive `fd_lock::RwLock` on `<cfg_dir>/projects.lock` on thread A for ~200ms while thread B calls `register_project_in(&cfg_dir, ...)` -> thread B waits until thread A releases `projects.lock`, then succeeds and persists the project entry.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'GLOBAL_PROJECTS_LOCK_FILE_NAME' crates/backend_core/src/models/registry.rs && grep -q 'fn public_mutator_waits_for_write_lock' crates/backend_core/tests/storage_integration.rs && grep -q 'fn registry_writes_serialize_on_projects_lock' crates/backend_core/tests/storage_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 45F: Add `AtomicBool` shutdown signal to `TrackerWatcher::drop` and skip symlinks in `hash_index_files`

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 45F: Add an `Arc<AtomicBool>` shutdown signal to `TrackerWatcher::drop` so dropping under lock contention or settlement exits promptly, and skip symlinks in `hash_index_files` (`BUG-0023` / `AUD-20`).

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/watcher/mod.rs
  - crates/backend_core/tests/watcher_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs:505-556 (with_write_lock_timeout)
  - crates/backend_core/src/storage/reconcile.rs:34-40 (reconcile_indexes)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not change `TrackerWatcher` public method signatures or weaken `echo_path_tests::own_index_echo_matches_through_symlinked_path` (watching through a symlinked root ancestor like macOS `/var`). Never touch the forbidden path listed in the plan header.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal: Dropping a `TrackerWatcher` always terminates its worker thread promptly (well under 2 seconds) even when `.tracker/.lock` is held by another thread/process or an item directory is mid-settlement, and `hash_index_files` never follows symlinks or loops on directory symlink cycles inside `.tracker/`.

Contract & Behavior:
  1. `crates/backend_core/src/watcher/mod.rs`:
     - Store `shutdown: Arc<AtomicBool>` on `TrackerWatcher` (initialized `false`), set `shutdown.store(true, Ordering::Release)` in `Drop::drop` before dropping `self.watcher` and joining `self.worker`.
     - Check `shutdown.load(Ordering::Acquire)` in the worker loop: on every iteration, during the 50ms debounce loop, during the 300ms settlement loop, and while acquiring `.tracker/.lock` for `auto_reconcile`.
     - For `auto_reconcile`, acquire the write lock in short slices via `storage_for_worker.with_write_lock_timeout(Duration::from_millis(50..100), |s| s.reconcile_indexes())` up to `DEFAULT_LOCK_TIMEOUT` total elapsed, checking `shutdown` on each `Err(TrackerError::LockTimeout { .. })`: if `shutdown` is set, exit the worker loop immediately without waiting the full 30s; if `DEFAULT_LOCK_TIMEOUT` elapses without shutdown, report `reconcile_error = Some(err.to_string())` as before.
     - In `hash_index_files`: inspect `entry.file_type()` (which does not follow symlinks) instead of `path.is_dir()`; skip entries where `file_type.is_symlink()`, recurse only when `file_type.is_dir()`, and hash derived index files only when `file_type.is_file()`.

Must still work:
  - `own_index_echo_matches_through_symlinked_path` (watching a directory accessed via a symlinked ancestor path still matches echo hashes).
  - `reconcile_error_is_reported`, `drop_joins_worker_promptly`, and `external_scope_edit_after_reconcile_is_processed` in `watcher_integration.rs`.

Drops: none

Tests:
  - `drop_stops_promptly_under_contended_write_lock` in `crates/backend_core/tests/watcher_integration.rs`: start `TrackerWatcher`, hold `.tracker/.lock` on a background thread (via `storage.with_write_lock`), trigger a filesystem write under `.tracker/` so the watcher attempts `reconcile_indexes`, sleep briefly so the worker enters lock contention, then `drop(watcher)` -> `drop` returns in `< 2s` while the lock is still held.
  - `hash_index_files_skips_directory_symlink_cycles` in `crates/backend_core/src/watcher/mod.rs` (`#[cfg(all(test, unix))]`): create a temp dir with `scope.toml` and a self-referential directory symlink (`loop_dir -> .`) plus a symlinked `phase.toml` -> `hash_index_files` terminates promptly, includes the real `scope.toml`, and does not include or recurse through either symlink.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'AtomicBool' crates/backend_core/src/watcher/mod.rs && grep -q 'fn drop_stops_promptly_under_contended_write_lock' crates/backend_core/tests/watcher_integration.rs && grep -q 'fn hash_index_files_skips_directory_symlink_cycles' crates/backend_core/src/watcher/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

## Phase 28 Step Contracts

### STEP 46A: Fix read-only registry lookups, per-path lock map, and read-only prune handling (`BUG-0033`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 46A: Make read-only registry lookups avoid write-opening projects.lock, isolate per-path in-process locks, and never fail reads on prune persistence

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/registry.rs
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/registry.rs:20-96,446-637
  - crates/backend_core/tests/storage_integration.rs:3145-3188 (registry_writes_serialize_on_projects_lock)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not change GlobalProjectRegistry TOML schema or public function signatures.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal: Read-only global registry lookups (`load_global_registry_from`, `credential_accounts_for_root_in`) succeed on read-only config directories without creating `projects.lock` or writing to disk, and lock contention on one registry directory never blocks unrelated registry directories in the same process.

Contract & Behavior:
  1. `crates/backend_core/src/models/registry.rs`:
     - Replace the single process-global `REGISTRY_LOCK: OnceLock<Mutex<()>>` held across `fd_lock` polling with a per-registry-path lock table (keyed by normalized/canonicalized lock path, e.g. `Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>`) so acquiring a lock on `dir_a/projects.lock` never blocks operations on `dir_b/projects.lock` in the same process.
     - Split read vs. write locking:
       - Write callers (`save_global_registry_to`, `register_project_in`, `remove_project_in`, `set_active_project_in`, `bind_credential_account_in`) continue creating parent directories, opening `projects.lock` for write, and taking an exclusive `fd_lock::RwLock::try_write()` lock with `DEFAULT_LOCK_TIMEOUT`.
       - Read-only callers (`load_global_registry_from`, `credential_accounts_for_root_in`) must NOT create parent directories and must NOT create or write-open `projects.lock`. If `projects.toml` does not exist (`!path.is_file()`), return the empty default immediately without touching `projects.lock`. If `projects.lock` exists, open it read-only (`OpenOptions::new().read(true)`) and take a shared `fd_lock::RwLock::try_read()` lock (with `DEFAULT_LOCK_TIMEOUT` poll loop); if `projects.lock` is absent (`ErrorKind::NotFound`), proceed directly to reading `projects.toml`.
     - Update `load_registry_unlocked` to take a `persist_prune: bool` flag (or separate read/write helper): write callers pass `true` (persisting pruned unsafe entries under the exclusive lock), while read-only callers pass `false` (returning the pruned in-memory `GlobalProjectRegistry` without writing to disk).

Must still work:
  - `registry_writes_serialize_on_projects_lock` in `storage_integration.rs`.
  - All existing `load_global_registry*`, `save_global_registry*`, `register_project*`, `remove_project*`, `set_active_project*`, `credential_accounts_for_root_in`, and `bind_credential_account_in` behavior.

Drops: Read-only `load_global_registry_from` no longer rewrites `projects.toml` on disk when pruning unsafe broad-directory entries (pruning still applies in memory on every read and persists on the next write).

Tests:
  - `read_only_registry_lookup_succeeds_without_creating_lock_file` in `crates/backend_core/tests/storage_integration.rs`: call `load_global_registry_from` on an empty writable dir -> returns empty default and `projects.lock` does not exist; on Unix, populate a dir with `projects.toml` containing one valid project and one unsafe broad-directory entry (`/`), chmod the dir `0o555` (and restore `0o755` before cleanup) -> both `load_global_registry_from` and `credential_accounts_for_root_in` succeed, return only the valid project (unsafe entry pruned in memory), and do not create `projects.lock`.
  - `registry_lock_contention_does_not_block_unrelated_registry_dir` in `crates/backend_core/tests/storage_integration.rs`: hold an exclusive `fd_lock` on `dir_a/projects.lock` while a background thread blocks inside `register_project_in(&dir_a, ...)` for ~300ms -> calling `register_project_in(&dir_b, ...)` on a separate directory completes in `< 150ms` while `dir_a` is still contended.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'try_read' crates/backend_core/src/models/registry.rs && grep -q 'fn read_only_registry_lookup_succeeds_without_creating_lock_file' crates/backend_core/tests/storage_integration.rs && grep -q 'fn registry_lock_contention_does_not_block_unrelated_registry_dir' crates/backend_core/tests/storage_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 46B: Add mode-aware `atomic_write_bytes_with_mode`, propagate `ensure_secure_dir` errors, reject implicit `temp_dir()` vault fallback, and harden Windows replace/ACL handling (`BUG-0009` / `AUD-02`)

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 46B: Create vault.key and vault.enc with atomic 0600 permissions, propagate dir/ACL hardening errors, reject implicit temp_dir() vault fallback, and harden Windows replace/icacls

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/atomic.rs
  - crates/backend_core/src/storage/mod.rs
  - crates/backend_core/src/crypto/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/lib.rs:722-785 (test_cross_platform_config_dir_resolution_and_path_normalization)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not break `nebulous_config_dir()` or `resolve_nebulous_config_dir_from(...)` signatures or behavior for non-vault callers.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal: `vault.key` and `vault.enc` are never created on disk with wider-than-`0o600` permissions, `ensure_secure_dir` surfaces permission/ACL failures instead of ignoring them, `SecureStorage::new()` refuses to fall back to shared `temp_dir()/nebulous`, and Windows atomic replacement never falls back to destructive `remove_file` + `rename` or PATH-resolved `icacls`.

Contract & Behavior:
  1. `crates/backend_core/src/storage/atomic.rs` and `crates/backend_core/src/storage/mod.rs`:
     - Add `pub fn atomic_write_bytes_with_mode(target_path: &Path, contents: &[u8], unix_mode: Option<u32>) -> Result<()>` and re-export it from `crates/backend_core/src/storage/mod.rs`.
     - Implement `atomic_write_bytes` as `atomic_write_bytes_with_mode(target_path, contents, None)`.
     - On Unix, when `unix_mode` is `Some(mode)`, create the unique temp file with `OpenOptionsExt::mode(mode)` at open time AND apply `temp_file.set_permissions(fs::Permissions::from_mode(mode))` before `sync_all()` so process umask cannot widen or alter the final mode and no pre-`0o600` window exists. When `unix_mode` is `None`, preserve the existing file's permissions verbatim as before.
     - On Windows (`#[cfg(windows)]` `replace_target`), remove the `fs::remove_file(target_path)` + `fs::rename(...)` fallback; keep clearing `FILE_ATTRIBUTE_READONLY` and retrying `fs::rename`, returning the error and leaving the existing target intact if all retries fail.
  2. `crates/backend_core/src/crypto/mod.rs`:
     - Add `pub fn resolve_nebulous_config_dir_checked(nebulous_override: Option<&str>, xdg_config_home: Option<&str>, appdata: Option<&str>, home: Option<&str>, target_os: &str) -> Result<PathBuf>` and `pub fn try_nebulous_config_dir() -> Result<PathBuf>`: follow the same precedence as `resolve_nebulous_config_dir_from`, but return `Err(TrackerError::ValidationError(...))` instead of `std::env::temp_dir().join("nebulous")` when all config/home variables are unset or empty.
     - Update `SecureStorage::new()` to call `try_nebulous_config_dir()?` so credential vaults never silently fall back to `/tmp/nebulous` unless the user explicitly sets `NEBULOUS_CONFIG_DIR`. Keep `resolve_nebulous_config_dir_from` and `nebulous_config_dir()` unchanged for existing callers.
     - Update `save_vault_unlocked` and `load_or_create_vault_master_key` to write `vault.enc` and `vault.key` via `atomic_write_bytes_with_mode(..., Some(0o600))` (retaining `set_secure_file_permissions` so Windows ACLs are still applied).
     - Update `ensure_secure_dir` to propagate `fs::set_permissions` errors on Unix (`TrackerError::Io`) and on Windows call `restrict_to_current_user(dir, true)?` unconditionally (even if `dir` already existed), propagating errors.
     - On Windows (`#[cfg(windows)]` `restrict_to_current_user`), resolve `icacls.exe` from `%SystemRoot%\System32\icacls.exe` (falling back to `C:\Windows\System32\icacls.exe` when `SystemRoot` is unset/empty) instead of unqualified `"icacls"`.

Must still work:
  - `preserves_original_mode_across_atomic_write` and `failed_rename_leaves_target_untouched_and_no_temp_files` in `atomic.rs`.
  - `test_cross_platform_config_dir_resolution_and_path_normalization` in `lib.rs`.
  - `SecureStorage::vault_only(temp_dir.path())` and `SecureStorage::with_config_dir(...)` with explicit paths in tests.
  - `windows_vault_file_acl_is_user_only` on Windows.

Drops: On Windows, `atomic_write_bytes` no longer falls back to non-atomic `remove_file` + `rename` when `MoveFileEx` fails after retries (leaves the existing target intact and returns the I/O error instead).

Tests:
  - `atomic_write_bytes_with_mode_creates_file_with_0600` in `crates/backend_core/src/storage/atomic.rs` (`#[cfg(all(test, unix))]`): write a brand-new file with `atomic_write_bytes_with_mode(&target, b"secret", Some(0o600))` -> file exists with contents `b"secret"` and mode `0o600`.
  - `vault_files_created_with_0600_mode` in `crates/backend_core/src/crypto/mod.rs` (`#[cfg(unix)]`): initialize `SecureStorage::vault_only(&dir)` and call `store_secret("testuser", "s3cret")` -> `dir` mode is `0o700` and both `vault.key` and `vault.enc` have mode `0o600`.
  - `ensure_secure_dir_propagates_permission_error` in `crates/backend_core/src/crypto/mod.rs` (`#[cfg(unix)]`): verify `ensure_secure_dir` tightens an existing `0o755` dir to `0o700`, and returns `Err` when given a path blocked by a regular file or unwritable parent.
  - `try_nebulous_config_dir_rejects_missing_env_vars` in `crates/backend_core/src/crypto/mod.rs`: `resolve_nebulous_config_dir_checked(None, None, None, None, "linux")` returns `Err(TrackerError::ValidationError(_))`, while `resolve_nebulous_config_dir_checked(Some("/tmp/custom"), None, None, None, "linux")` returns `Ok(PathBuf::from("/tmp/custom"))`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'pub fn atomic_write_bytes_with_mode' crates/backend_core/src/storage/atomic.rs && grep -q 'pub fn try_nebulous_config_dir' crates/backend_core/src/crypto/mod.rs && grep -q 'fn atomic_write_bytes_with_mode_creates_file_with_0600' crates/backend_core/src/storage/atomic.rs && grep -q 'fn vault_files_created_with_0600_mode' crates/backend_core/src/crypto/mod.rs && grep -q 'fn try_nebulous_config_dir_rejects_missing_env_vars' crates/backend_core/src/crypto/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 46C: Fix `SecureStorage` keyring/vault split-brain and route `git_sync` PAT functions exclusively through `SecureStorage` (`BUG-0011` / `AUD-04`)

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 46C: Prevent stale OS keyring reads on SecureStorage::store_secret failure and route git_sync PAT operations exclusively through SecureStorage

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/crypto/mod.rs
  - crates/backend_core/src/git_sync/mod.rs
  - crates/backend_core/tests/credential_hygiene.rs
READ_ONLY_REFS:
  - crates/backend_core/src/crypto/mod.rs:380-469
  - crates/backend_core/src/git_sync/mod.rs:20-35,270-365,1474-1542,1956-1990
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not change `store_pat_in_keyring`, `get_pat_from_keyring`, `delete_pat_from_keyring`, or `SecureStorage::store_secret` public signatures.

[PRE_GATE]
grep -q 'atomic_write_bytes_with_mode' crates/backend_core/src/crypto/mod.rs

[MUTATION_SPEC]
Goal: Updating a secret in `SecureStorage` can never silently leave a stale OS keyring entry that shadows the newly written `vault.enc` value, and `git_sync` PAT helpers/resolution never bypass `SecureStorage`, `KeyPairManager::vault_only`, or `NEBULOUS_DISABLE_KEYRING`.

Contract & Behavior:
  1. `crates/backend_core/src/crypto/mod.rs`:
     - Extract a testable helper (e.g. `fn sync_keyring_on_store(set_res: std::result::Result<(), keyring::Error>, delete_fn: impl FnOnce() -> std::result::Result<(), keyring::Error>) -> Result<()>`) called from `SecureStorage::store_secret` when `self.use_keyring` is `true` after `self.write_vault_entry(...)`:
       - On `Ok(())`: succeed.
       - On `Err(keyring::Error::PlatformFailure(_) | keyring::Error::NoStorageAccess(_))`: attempt `delete_fn()` best-effort; if `delete_fn()` returns `Ok(())`, `Err(NoEntry)`, `Err(NoStorageAccess(_))`, or `Err(PlatformFailure(_))`, return `Ok(())` (headless/unavailable keyring, vault is authoritative). Otherwise return `Err(TrackerError::KeyringError(...))`.
       - On any other `set_password` error (e.g. `TooLong`, `BadEncoding`, `Invalid`, `Ambiguous`): call `delete_fn()` to clear any stale keyring entry; if `delete_fn()` returns `Ok(())` or `Err(NoEntry)`, return `Ok(())` (stale entry removed so `get_secret` reads the updated vault); if `delete_fn()` fails, return `Err(TrackerError::KeyringError(...))` so `get_secret` never silently serves a stale keyring secret.
  2. `crates/backend_core/src/git_sync/mod.rs`:
     - Remove direct `keyring::Entry::new` calls and the static plaintext `FALLBACK_PAT_STORE` / `combine_pat_store_results` cache from `git_sync/mod.rs`.
     - Route `store_pat_in_keyring`, `get_pat_from_keyring`, and `delete_pat_from_keyring` exclusively through `SecureStorage::new()` (`store_secret`, `get_secret_string`, `delete_secret`), preserving token trimming, empty-token `ValidationError`, and never echoing the token in error messages.
     - In `TrackerStorage::resolve_git_auth_with_manager`: when `manager` is `Some(mgr)`, resolve the PAT exclusively from `mgr` (do NOT fall back to `get_pat_from_keyring(account)` when `mgr` does not have it, so `KeyPairManager::vault_only` never touches the OS keyring or global config dir). Only call `get_pat_from_keyring(account)` when `manager` is `None`.
     - Replace the obsolete `combine_pat_store_results` unit tests at the bottom of `git_sync/mod.rs` with unit tests verifying `store_pat_in_keyring` rejects empty/whitespace tokens without echoing secrets.

Must still work:
  - `SecureStorage::vault_only` and `KeyPairManager::vault_only` round-trips in `lib.rs` and `workflow_integration.rs`.
  - `TrackerStorage::store_pat`, `get_pat`, `delete_pat`, and `resolve_git_auth`.

Drops: `store_pat_in_keyring` no longer writes to a process-local `FALLBACK_PAT_STORE` or succeeds via OS keyring alone when `SecureStorage` vault initialization/write fails; `SecureStorage` (`vault.enc` + optional OS keyring) is the single PAT store.

Tests:
  - `store_secret_keyring_failure_clears_stale_or_errors` in `crates/backend_core/src/crypto/mod.rs`: exercises `sync_keyring_on_store` across (a) `set_password` `NoStorageAccess` + `delete` `NoStorageAccess` -> `Ok(())`, (b) `set_password` `TooLong` + `delete` `Ok(())` -> `Ok(())`, (c) `set_password` `TooLong` + `delete` `Err(Invalid(...))` -> `Err(TrackerError::KeyringError(_))`.
  - `vault_only_manager_does_not_fall_back_to_global_keyring_store` in `crates/backend_core/tests/credential_hygiene.rs`: bind an account to a temp project in an isolated `vault_only` `KeyPairManager` without storing a PAT in that vault, call `storage.resolve_git_auth_with_manager(Some(&manager), &GitAuthRequest::default())` -> `resolved.pat` is `None` and no OS keyring lookup occurs.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'fn sync_keyring_on_store' crates/backend_core/src/crypto/mod.rs && grep -q 'fn store_secret_keyring_failure_clears_stale_or_errors' crates/backend_core/src/crypto/mod.rs && grep -q 'fn vault_only_manager_does_not_fall_back_to_global_keyring_store' crates/backend_core/tests/credential_hygiene.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 46D: Pin `libgit2` PAT credentials and disable cross-host redirects to the configured HTTPS remote host (`BUG-0019` / `AUD-14`)

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 46D: Pin HTTPS PAT delivery to the configured remote host and port and set RemoteRedirect::None on libgit2 fetch and push options

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/git_sync/mod.rs
  - crates/backend_core/tests/credential_hygiene.rs
READ_ONLY_REFS:
  - crates/backend_core/src/git_sync/mod.rs:585-610,955-1170
  - crates/backend_core/tests/credential_hygiene.rs:120-215
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not break `git_credential`, `build_auth_callbacks`, or `build_pat_callbacks` existing signatures.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal: `libgit2` fetch, pull, and push operations never follow off-site HTTP/HTTPS redirects (`RemoteRedirect::None`) and never send a PAT to any host or port other than the configured `https://` remote URL.

Contract & Behavior:
  1. `crates/backend_core/src/git_sync/mod.rs`:
     - Add an `https_host_port(url: &str) -> Option<(String, u16)>` helper that parses `https://` URLs (case-insensitive scheme), strips any `userinfo@` in the authority, handles bracketed IPv6 literals (`[::1]:8443`), lowercases the host, defaults the port to `443` when omitted, and returns `None` for non-`https://` or malformed/empty-host URLs.
     - Add `pub fn git_credential_for_remote(auth: &GitAuthMaterial, configured_remote_url: Option<&str>, url: &str, username_from_url: Option<&str>, allowed_types: CredentialType) -> std::result::Result<Cred, git2::Error>` and `pub fn build_auth_callbacks_for_remote<'a>(auth: GitAuthMaterial, configured_remote_url: Option<String>) -> RemoteCallbacks<'a>`:
       - Keep `pub fn git_credential` and `pub fn build_auth_callbacks` delegating with `None` for backward compatibility.
       - When `configured_remote_url` is `Some(remote_url)`, offer `auth.pat` ONLY if `https_host_port(remote_url)` and `https_host_port(url)` are both `Some` and equal (exact normalized host + port match). If they do not match and `USER_PASS_PLAINTEXT` was requested with a PAT configured, refuse with a `git2::Error` naming host mismatch / configured remote.
     - In `fetch_repo_with_auth` (both initial `fetch_opts` and `retry_opts`), `push_repo_with_auth` (`push_opts`), and `pull_repo_with_auth` (`fetch_opts`):
       - Call `.follow_redirects(git2::RemoteRedirect::None)` on every `FetchOptions` and `PushOptions`.
       - Pass `remote.url().map(str::to_string)` into `build_auth_callbacks_for_remote(auth, remote_url)`.

Must still work:
  - `pat_is_offered_only_to_https_urls`, `http_remote_never_receives_pat`, and `vault_ssh_key_is_used_from_memory_and_ssh_cache_is_never_created` in `credential_hygiene.rs`.
  - `build_pat_callbacks` and `build_auth_callbacks` callers in `workflow_integration.rs`.

Drops: Off-site HTTP/HTTPS redirects during `git fetch`, `git pull`, and `git push` are no longer followed automatically (`git2::RemoteRedirect::None`).

Tests:
  - `pat_is_pinned_to_configured_https_remote_host_and_port` in `crates/backend_core/tests/credential_hygiene.rs`: calls `git_credential_for_remote` with configured remote `https://git.example.com/org/repo.git` -> accepts `https://GIT.EXAMPLE.COM/org/repo.git` and `https://git.example.com:443/org/repo.git`, and rejects `https://evil.example.com/org/repo.git`, `https://sub.git.example.com/org/repo.git`, `https://git.example.com:8443/org/repo.git`, and `http://git.example.com/org/repo.git`.
  - `fetch_disables_cross_host_redirects` in `crates/backend_core/tests/credential_hygiene.rs`: spawns two local TCP listeners (`origin` returning `302 Found` with `Location: http://127.0.0.1:<target_port>/org/repo.git`, and `target` recording any connections), runs `fetch_repo_with_auth` against `origin` -> `origin` is contacted, `fetch_repo_with_auth` returns `Err`, and `target` receives zero requests.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'pub fn git_credential_for_remote' crates/backend_core/src/git_sync/mod.rs && grep -q 'pub fn build_auth_callbacks_for_remote' crates/backend_core/src/git_sync/mod.rs && grep -q 'RemoteRedirect::None' crates/backend_core/src/git_sync/mod.rs && grep -q 'fn pat_is_pinned_to_configured_https_remote_host_and_port' crates/backend_core/tests/credential_hygiene.rs && grep -q 'fn fetch_disables_cross_host_redirects' crates/backend_core/tests/credential_hygiene.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 46E: Sanitize all single-line untrusted fields and Unicode bidi format chars in `agent_context::synthesize_bounded_prompt` (`BUG-0018` / `AUD-12`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 46E: Sanitize all single-line untrusted fields and strip Unicode bidi format characters in agent_context::synthesize_bounded_prompt

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/agent_context/mod.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/agent_context/mod.rs:1-246
  - crates/backend_core/tests/workflow_integration.rs:2487-2555
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not change `AgentContextBundle` fields, `MAX_DESC_BYTES`, `MAX_SHORT_TEXT_BYTES`, or the `synthesize_bounded_prompt` signature.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal: Untrusted single-line metadata fields (`item.title`, `phase_id`, `scope.id`, `scope.target`, `tech_stack`, `fix.date`, `fix.commit`, `fix.summary`, paths, `item.id`) cannot inject newlines, Markdown headings/code-span breakouts, control characters, or Unicode bidi overrides into synthesized agent prompts.

Contract & Behavior:
  1. `crates/backend_core/src/agent_context/mod.rs`:
     - Add a Unicode bidi format character check covering `U+061C`, `U+200E`, `U+200F`, `U+202A..=U+202E`, and `U+2066..=U+2069`, and strip those characters inside `strip_control` (used by fenced `desc.md` and `scope.scope.description`) as well as in `single_line_safe`.
     - Add `fn single_line_safe(s: &str) -> String`:
       - Maps `\r`, `\n`, and `\t` to `' '`.
       - Strips all `c.is_control()` characters and all Unicode bidi format characters listed above.
       - Strips backticks (`` ` ``) so untrusted strings cannot break out of inline Markdown code spans.
     - Apply `single_line_safe` to every single-line interpolated field in `synthesize_bounded_prompt`: `item.id`, `item.title`, `phase_id`, `scope.scope.id`, `scope.scope.target` (both `- **Target Path:**` and `- **Boundary Constraint:**`), every entry of `scope.scope.tech_stack`, `fix.date`, `fix.commit`, `fix.summary` (capping `fix.summary` to `MAX_SHORT_TEXT_BYTES` and collapsing any truncation newline to a space so the blockquote item stays single-line), `scope_rel_path`, `item_rel_path`, and `desc_rel_path`.
     - Keep `desc.md` and `scope.scope.description` wrapped in adaptive `fenced(...)` as before.

Must still work:
  - `test_agent_context_exporter_refs_and_bounded_prompt`, `bounded_prompt_caps_desc_md`, `bounded_prompt_strips_control_chars`, and `bounded_prompt_fence_outlasts_backticks` in `workflow_integration.rs`.

Drops: none

Tests:
  - `bounded_prompt_sanitizes_single_line_fields_and_bidi_chars` in `crates/backend_core/tests/workflow_integration.rs`: constructs a `ScopeManifest` and `ItemDetail` containing `\n## INJECTED`, `\r`, `\t`, backticks, `\x1b[31m`, and bidi override `\u{202E}` across `item.title`, `scope.id`, `scope.target`, `tech_stack`, `fix.date`, `fix.commit`, `fix.summary`, and `desc.md`; calls `synthesize_bounded_prompt(...)` -> output contains no `\x1b`, no `\u{202E}`, no line starting with `## INJECTED`, and every single-line heading/bullet/blockquote line stays on a single line without backtick code-span breakout.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'fn single_line_safe' crates/backend_core/src/agent_context/mod.rs && grep -q 'fn bounded_prompt_sanitizes_single_line_fields_and_bidi_chars' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 46F: Zeroize `GitAuthMaterial::pat` and guard SSH passphrase stripping (`TASK-0110` non-breaking slice / `AUD-13`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 46F: Wrap GitAuthMaterial::pat in Zeroizing<String> and add an explicit passphrase-stripping policy guard on SSH key import

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/git_sync/mod.rs
  - crates/backend_core/src/crypto/keys.rs
  - crates/backend_core/tests/credential_hygiene.rs
READ_ONLY_REFS:
  - crates/backend_core/src/git_sync/mod.rs:238-270,590-608,957-1010,1486-1526
  - crates/backend_core/src/crypto/keys.rs:19-64,124-205,249-335
  - crates/backend_core/src/lib.rs:652-693
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not change vault.enc format, vault.key KDF (parked), or break existing `KeyPairManager::import_ssh_keypair` callers.

[PRE_GATE]
grep -q 'build_auth_callbacks_for_remote' crates/backend_core/src/git_sync/mod.rs && grep -q 'fn sync_keyring_on_store' crates/backend_core/src/crypto/mod.rs

[MUTATION_SPEC]
Goal: HTTPS PATs remain inside `Zeroizing<String>` throughout `GitAuthMaterial` and auth resolution, and callers importing passphrase-protected OpenSSH keys can refuse passphrase stripping or observe `passphrase_stripped: true` on the returned record.

Contract & Behavior:
  1. `crates/backend_core/src/git_sync/mod.rs`:
     - Change `GitAuthMaterial.pat` from `Option<String>` to `Option<Zeroizing<String>>` (using `crate::crypto::Zeroizing`).
     - Update `GitAuthMaterial::has_pat`, `pat_for_url`, `build_pat_callbacks`, and `TrackerStorage::resolve_git_auth_with_manager` to keep PATs in `Zeroizing<String>` (using `mgr.get_pat(account)` instead of `mgr.get_pat_string(account)` and wrapping `request.pat` / fallback `get_pat_from_keyring` results in `Zeroizing::new`).
     - Keep `Debug` redaction (`"<REDACTED>"`) intact on `GitAuthMaterial`.
  2. `crates/backend_core/src/crypto/keys.rs`:
     - Add `pub passphrase_stripped: bool` to `SshKeyPairRecord` (set `true` only when an encrypted OpenSSH private key was decrypted and persisted unencrypted inside `SecureStorage`; `false` for generated keys, unencrypted imports, and `get_ssh_keypair` reads).
     - Add `pub fn import_ssh_keypair_with_policy(&self, account: &str, private_key_openssh: &str, passphrase: Option<&str>, allow_strip_passphrase: bool) -> Result<SshKeyPairRecord>`:
       - When `outer.is_encrypted()` and `!allow_strip_passphrase`, return `Err(TrackerError::ValidationError(...))` refusing to strip the passphrase without decrypting or storing the key.
       - When `outer.is_encrypted()` and `allow_strip_passphrase`, decrypt with `passphrase`, persist into `SecureStorage`, and return `SshKeyPairRecord` with `passphrase_stripped: true`.
       - When `!outer.is_encrypted()`, import normally with `passphrase_stripped: false`.
     - Keep `KeyPairManager::import_ssh_keypair` delegating to `import_ssh_keypair_with_policy(..., true)` for backward compatibility.
  3. `crates/backend_core/tests/credential_hygiene.rs`:
     - Update the two existing `GitAuthMaterial { pat: Some(...) }` test literals to `Some(Zeroizing::new(SECRET_PAT.to_string()))`.

Must still work:
  - All existing tests in `lib.rs` (including passphrase-encrypted key import via `import_ssh_keypair`), `credential_hygiene.rs`, and `workflow_integration.rs`.

Drops: none

Tests:
  - `import_ssh_keypair_with_policy_guards_passphrase_stripping` in `crates/backend_core/tests/credential_hygiene.rs`: builds/uses an encrypted OpenSSH Ed25519 test key (comment `testuser@example.com`), calls `import_ssh_keypair_with_policy("testuser/strict", &enc_pem, Some("test-passphrase"), false)` -> returns `Err(TrackerError::ValidationError(_))` and does not store the key; calling with `allow_strip_passphrase = true` -> returns `Ok(record)` with `record.passphrase_stripped == true`; importing an unencrypted key with `allow_strip_passphrase = false` -> returns `Ok(record)` with `record.passphrase_stripped == false`.
  - `git_auth_material_keeps_pat_zeroizing_and_redacts_debug` in `crates/backend_core/tests/credential_hygiene.rs`: constructs `GitAuthMaterial` with `Some(Zeroizing::new(...))`, verifies `has_pat()` is `true` and `format!("{auth:?}")` contains `"<REDACTED>"` and does not contain the secret token.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'pub pat: Option<Zeroizing<String>>' crates/backend_core/src/git_sync/mod.rs && grep -q 'pub fn import_ssh_keypair_with_policy' crates/backend_core/src/crypto/keys.rs && grep -q 'pub passphrase_stripped: bool' crates/backend_core/src/crypto/keys.rs && grep -q 'fn import_ssh_keypair_with_policy_guards_passphrase_stripping' crates/backend_core/tests/credential_hygiene.rs && grep -q 'fn git_auth_material_keeps_pat_zeroizing_and_redacts_debug' crates/backend_core/tests/credential_hygiene.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

## Phase 29 Step Contracts

### STEP 47A: Fix `git_credential_for_remote` SSH fallback and `push_repo_with_auth` `pushurl` pinning (`BUG-0035`, `BUG-0036`; closes `BUG-0019`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 47A: Allow SSH fallback on non-HTTPS remotes when a PAT is present and pin push_repo_with_auth to remote.pushurl() with fallback to remote.url()

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/git_sync/mod.rs
  - crates/backend_core/tests/credential_hygiene.rs
READ_ONLY_REFS:
  - crates/backend_core/src/git_sync/mod.rs:1000-1178
  - crates/backend_core/tests/credential_hygiene.rs:307-360
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not relax HTTPS PAT host/port pinning for configured https:// remotes or change RemoteRedirect::None.

[PRE_GATE]
grep -q 'pub fn git_credential_for_remote' crates/backend_core/src/git_sync/mod.rs

[MUTATION_SPEC]
Goal: Non-HTTPS remotes (such as SSH remotes advertising both password and publickey auth) fall through to SSH credentials even when a PAT is stored alongside an SSH keypair, and `push_repo_with_auth` pins PAT delivery to `remote.pushurl()` when configured (falling back to `remote.url()`).

Contract & Behavior:
  1. `crates/backend_core/src/git_sync/mod.rs` (`git_credential_for_remote`):
     - When `allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT)`, `auth.pat` is non-empty, and `configured_remote_url` is `Some(remote_url)`:
       - If `https_host_port(remote_url)` is `Some(configured_hp)`:
         - Return `Cred::userpass_plaintext(user, token)` when `https_host_port(url) == Some(configured_hp)`.
         - Otherwise return `Err(git2::Error::from_str("Refusing to send Personal Access Token: host mismatch with configured https:// remote"))`.
       - If `https_host_port(remote_url)` is `None` (the configured remote is non-HTTPS, e.g. `ssh://`, `git@...`, `http://`):
         - Do NOT send the PAT and do NOT return a host-mismatch error early; fall through to the SSH (`SSH_KEY | SSH_MEMORY | SSH_CUSTOM`), `USERNAME`, and `DEFAULT` branches.
         - If no subsequent branch satisfies `allowed_types`, keep the existing final error (`"No usable Git credentials: a Personal Access Token is only sent to https:// remotes"` when `USER_PASS_PLAINTEXT` was requested and `auth.has_pat()`, else `"No usable Git credentials for this remote"`).
  2. `crates/backend_core/src/git_sync/mod.rs` (`push_repo_with_auth` + helper `effective_push_remote_url`):
     - Extract a helper `fn effective_push_remote_url(remote: &git2::Remote<'_>) -> Option<String>` that returns `remote.pushurl()` when configured, otherwise `remote.url()`.
     - Call `effective_push_remote_url(&remote)` in `push_repo_with_auth` and pass the result to `build_auth_callbacks_for_remote`.

Must still work:
  - `pat_is_pinned_to_configured_https_remote_host_and_port` and `pat_callbacks_refuse_cross_host_redirects` in `credential_hygiene.rs` (configured `https://` remotes still reject cross-host, wrong-port, `http://`, and `ssh://` URLs with the host-mismatch error).

Drops: none

Tests:
  - `ssh_remote_falls_back_to_ssh_when_pat_is_present` in `crates/backend_core/tests/credential_hygiene.rs`: constructs `GitAuthMaterial` with both a non-empty `pat` and an in-memory Ed25519 `ssh_keypair` (generated via `KeyPairManager::vault_only` with `testuser/ssh`), calls `git_credential_for_remote(&auth, Some("ssh://git@example.com/org/repo.git"), "ssh://git@example.com/org/repo.git", Some("git"), CredentialType::USER_PASS_PLAINTEXT | CredentialType::SSH_MEMORY | CredentialType::SSH_KEY)` -> returns `Ok(cred)` (SSH credential, not host-mismatch error); also verifies `git_credential_for_remote(&auth, Some("http://example.com/org/repo.git"), "http://example.com/org/repo.git", None, CredentialType::USER_PASS_PLAINTEXT)` -> returns `Err` refusing to send PAT over non-HTTPS.
  - `push_remote_url_prefers_pushurl_over_fetch_url` in `crates/backend_core/src/git_sync/mod.rs`: creates a temp Git repo with remote `origin` (`url = https://fetch.example.com/org/repo.git`, `pushurl = https://push.example.com:8443/org/repo.git`) and remote `upstream` (`url = https://fetch.example.com/org/repo.git`, no `pushurl`) -> `effective_push_remote_url` returns `Some("https://push.example.com:8443/org/repo.git")` for `origin` and `Some("https://fetch.example.com/org/repo.git")` for `upstream`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'fn effective_push_remote_url' crates/backend_core/src/git_sync/mod.rs && grep -q 'fn push_remote_url_prefers_pushurl_over_fetch_url' crates/backend_core/src/git_sync/mod.rs && grep -q 'fn ssh_remote_falls_back_to_ssh_when_pat_is_present' crates/backend_core/tests/credential_hygiene.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 47B: Zeroize `GitAuthMaterial::pat` and guard SSH passphrase stripping (re-split of `STEP 46F`, `TASK-0135`, `TASK-0136`, `TASK-0110`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 47B: Wrap GitAuthMaterial::pat in Zeroizing<String>, update all PAT callers/tests, and add an explicit passphrase-stripping policy guard on SSH key import

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/git_sync/mod.rs
  - crates/backend_core/src/crypto/keys.rs
  - crates/backend_core/tests/credential_hygiene.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/git_sync/mod.rs:230-262,935-951,1005-1036,1529-1570
  - crates/backend_core/src/crypto/keys.rs:19-64,124-205,249-335
  - crates/backend_core/tests/workflow_integration.rs:1418-1455
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not change vault.enc format, vault.key KDF (parked), or break existing `KeyPairManager::import_ssh_keypair` callers.

[PRE_GATE]
grep -q 'fn ssh_remote_falls_back_to_ssh_when_pat_is_present' crates/backend_core/tests/credential_hygiene.rs

[MUTATION_SPEC]
Goal: HTTPS PATs remain inside `Zeroizing<String>` throughout `GitAuthMaterial` and auth resolution, and callers importing passphrase-protected OpenSSH keys can refuse passphrase stripping or observe `passphrase_stripped: true` on the returned record.

Contract & Behavior:
  1. `crates/backend_core/src/git_sync/mod.rs`:
     - Change `GitAuthMaterial.pat` from `Option<String>` to `Option<Zeroizing<String>>` (using `crate::crypto::Zeroizing`).
     - Update `GitAuthMaterial::has_pat`, `git_credential_for_remote`, `build_pat_callbacks`, and `TrackerStorage::resolve_git_auth_with_manager` to keep PATs in `Zeroizing<String>` (using `mgr.get_pat(account)` instead of `mgr.get_pat_string(account)` and wrapping `request.pat` / `get_pat_from_keyring` results in `Zeroizing::new`).
     - Keep `Debug` redaction (`"<REDACTED>"`) intact on `GitAuthMaterial`.
  2. `crates/backend_core/src/crypto/keys.rs`:
     - Add `pub passphrase_stripped: bool` to `SshKeyPairRecord` (set `true` only when an encrypted OpenSSH private key was decrypted and persisted unencrypted inside `SecureStorage`; `false` for generated keys, unencrypted imports, and `get_ssh_keypair` reads).
     - Add `pub fn import_ssh_keypair_with_policy(&self, account: &str, private_key_openssh: &str, passphrase: Option<&str>, allow_strip_passphrase: bool) -> Result<SshKeyPairRecord>`:
       - When `outer.is_encrypted()` and `!allow_strip_passphrase`, return `Err(TrackerError::ValidationError(...))` refusing to strip the passphrase without decrypting or storing the key.
       - When `outer.is_encrypted()` and `allow_strip_passphrase`, decrypt with `passphrase`, persist into `SecureStorage`, and return `SshKeyPairRecord` with `passphrase_stripped: true`.
       - When `!outer.is_encrypted()`, import normally with `passphrase_stripped: false`.
     - Keep `KeyPairManager::import_ssh_keypair` delegating to `import_ssh_keypair_with_policy(..., true)` for backward compatibility.
  3. `crates/backend_core/tests/credential_hygiene.rs` & `crates/backend_core/tests/workflow_integration.rs`:
     - Update all `GitAuthMaterial { pat: Some(...) }` test literals in `credential_hygiene.rs` to `Some(Zeroizing::new(...))`.
     - Update the two `resolved.pat.as_deref()` / `explicit.pat.as_deref()` assertions in `workflow_integration.rs` (around lines 1428 and 1450) for `Option<Zeroizing<String>>`.

Must still work:
  - All existing tests in `lib.rs` (including passphrase-encrypted key import via `import_ssh_keypair`), `credential_hygiene.rs`, and `workflow_integration.rs`.

Drops: none

Tests:
  - `import_ssh_keypair_with_policy_guards_passphrase_stripping` in `crates/backend_core/tests/credential_hygiene.rs`: builds an encrypted OpenSSH Ed25519 test key (comment `testuser@example.com`), calls `import_ssh_keypair_with_policy("testuser/strict", &enc_pem, Some("test-passphrase"), false)` -> returns `Err(TrackerError::ValidationError(_))` and does not store the key; calling with `allow_strip_passphrase = true` -> returns `Ok(record)` with `record.passphrase_stripped == true`; importing an unencrypted key with `allow_strip_passphrase = false` -> returns `Ok(record)` with `record.passphrase_stripped == false`.
  - `git_auth_material_keeps_pat_zeroizing_and_redacts_debug` in `crates/backend_core/tests/credential_hygiene.rs`: constructs `GitAuthMaterial` with `Some(Zeroizing::new(...))`, verifies `has_pat()` is `true` and `format!("{auth:?}")` contains `"<REDACTED>"` and does not contain the secret token.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'pub pat: Option<Zeroizing<String>>' crates/backend_core/src/git_sync/mod.rs && grep -q 'pub fn import_ssh_keypair_with_policy' crates/backend_core/src/crypto/keys.rs && grep -q 'pub passphrase_stripped: bool' crates/backend_core/src/crypto/keys.rs && grep -q 'fn import_ssh_keypair_with_policy_guards_passphrase_stripping' crates/backend_core/tests/credential_hygiene.rs && grep -q 'fn git_auth_material_keeps_pat_zeroizing_and_redacts_debug' crates/backend_core/tests/credential_hygiene.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 47C: Fix Phase 28 test defects (`BUG-0037`, `BUG-0038`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 47C: Seed a global-store PAT in vault_only_manager_does_not_fall_back_to_global_keyring_store and directly test set_permissions error propagation in ensure_secure_dir_propagates_permission_error

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/tests/credential_hygiene.rs
  - crates/backend_core/src/crypto/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/tests/credential_hygiene.rs:280-306
  - crates/backend_core/src/crypto/mod.rs:700-722,862-881
  - crates/backend_core/src/git_sync/mod.rs:1529-1569
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not alter production vault/keyring semantics or weaken any existing assertions.

[PRE_GATE]
true

[MUTATION_SPEC]
Goal: Both Phase 28 regression tests actually fail if their guarded regressions are re-introduced: `vault_only_manager_does_not_fall_back_to_global_keyring_store` proves isolation against a real seeded global PAT, and `ensure_secure_dir_propagates_permission_error` proves `set_permissions` failure propagation after `create_dir_all` succeeds.

Contract & Behavior:
  1. `crates/backend_core/tests/credential_hygiene.rs` (`vault_only_manager_does_not_fall_back_to_global_keyring_store`):
     - Create a separate temporary `global_cfg` directory, seed a PAT (`"ghp_global_fallback_should_not_leak"`) under `"testuser/isolated"` into `SecureStorage::vault_only(&global_cfg)`, and point `NEBULOUS_CONFIG_DIR` to `global_cfg` for the duration of the test (restored unconditionally by a `Drop` guard).
     - Verify the seeded global lookup path is live (e.g. `SecureStorage::vault_only(&global_cfg).get_secret_string("testuser/isolated")` returns the seeded PAT), then call `storage.resolve_git_auth_with_manager(Some(&manager), &GitAuthRequest::default())` where `manager` is `KeyPairManager::vault_only(&vault_dir)` with no PAT stored.
     - Assert `resolved.pat.is_none()`, proving the isolated manager never falls back to `NEBULOUS_CONFIG_DIR` / `get_pat_from_keyring`.
  2. `crates/backend_core/src/crypto/mod.rs` (`ensure_secure_dir` + `ensure_secure_dir_with` + `ensure_secure_dir_propagates_permission_error`):
     - Refactor `ensure_secure_dir` to delegate to an internal helper `fn ensure_secure_dir_with(dir: &Path, set_perms: impl FnOnce(&Path) -> Result<()>) -> Result<()>` that runs `fs::create_dir_all(dir)` and then propagates `set_perms(dir)?`.
     - In `ensure_secure_dir_propagates_permission_error`:
       - Keep the existing `0o755` -> `0o700` tightening and regular-file blocker checks.
       - Call `ensure_secure_dir_with(&loose_dir, |p| Err(TrackerError::Io { path: p.to_path_buf(), source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "simulated chmod failure") }))` on an already-existing directory where `fs::create_dir_all` succeeds -> assert `Err(TrackerError::Io { .. })` is returned.
       - On Unix, when `fs::metadata(dir.path()).uid() != fs::metadata("/").uid()` (non-root), also call `ensure_secure_dir(Path::new("/"))` -> assert `Err(TrackerError::Io { .. })` (real `fs::set_permissions` `EPERM` after `create_dir_all` succeeds).

Must still work:
  - All tests in `credential_hygiene.rs` and `crypto::tests` pass deterministically regardless of whether `cargo test` runs as root or non-root and without polluting the host OS keyring or config directory.

Drops: none

Tests:
  - `vault_only_manager_does_not_fall_back_to_global_keyring_store` in `crates/backend_core/tests/credential_hygiene.rs`: seeds `"testuser/isolated"` in `global_cfg` pointed to by `NEBULOUS_CONFIG_DIR`, resolves auth with an empty `vault_only(&vault_dir)` manager -> `resolved.pat.is_none()`.
  - `ensure_secure_dir_propagates_permission_error` in `crates/backend_core/src/crypto/mod.rs`: verifies `ensure_secure_dir_with` propagates `PermissionDenied` on an existing dir (plus real `ensure_secure_dir(Path::new("/"))` `Err` on Unix when non-root).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'fn ensure_secure_dir_with' crates/backend_core/src/crypto/mod.rs && grep -q 'ghp_global_fallback_should_not_leak' crates/backend_core/tests/credential_hygiene.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 47D: Propagate merge-reconcile errors, block `git_push` on `duplicate_ids`, and fix `git_commit_documented` atomicity, reachable SHA, and index stat metadata (`BUG-0014` / `AUD-07`)

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 47D: Propagate merge-reconcile failures, abort git_sync before push on duplicate_ids, make git_commit_documented record reachable fix SHAs without pre-advancing HEAD, and populate real stat metadata in sync_user_index_paths

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/git_sync/mod.rs
  - crates/backend_core/tests/git_safety.rs
  - crates/backend_core/tests/workflow_integration.rs
  - crates/nebulous_cli/tests/done_commit_safety.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/git_sync/mod.rs:757-804,1268-1375,1846-1998
  - crates/backend_core/tests/workflow_integration.rs:1213-1286
  - crates/nebulous_cli/tests/done_commit_safety.rs:64-112
  - crates/nebulous_cli/tests/cli_integration.rs:2090-2106
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not change `DocumentedCommitInput` fields yet (handled in STEP 47E) or alter `git_pull` / `git_sync` return types.

[PRE_GATE]
grep -q 'pub pat: Option<Zeroizing<String>>' crates/backend_core/src/git_sync/mod.rs

[MUTATION_SPEC]
Goal: Merge-reconcile failures never silently commit stale indexes, `git_sync_with_report` never pushes duplicate item IDs to a remote, `git_commit_documented` never advances `HEAD` before tracker docs are written and always records a reachable `recorded_fix_sha` in Git history, and `.git/index` entries updated by `sync_user_index_paths` carry real file stat metadata.

Contract & Behavior:
  1. `crates/backend_core/src/git_sync/mod.rs` (`pull_repo_with_auth`):
     - When `resolved_derived_index_conflict` is `true`, run `TrackerStorage::discover(repo_path.as_ref())?` and `storage.reconcile_indexes()?` BEFORE creating the merge commit on `HEAD` (propagating any error via `?` instead of swallowing it with `if let Ok` / `.is_ok()`).
     - Re-stage the reconciled derived index paths into the private index seeded from `tree`, compute the final tree, commit ONCE with parents `&[&local_commit, &remote_commit]`, and sync the user index for the reconciled paths. If `reconcile_indexes()` fails, `HEAD` remains at `local_commit`.
  2. `crates/backend_core/src/git_sync/mod.rs` (`git_sync_with_report`):
     - After `let pull_outcome = self.git_pull_with_report(auth)?;`, if `!pull_outcome.duplicate_ids.is_empty()`, return `Err(TrackerError::GitError(...))` mentioning `"duplicate"` and the duplicate item IDs WITHOUT calling `self.git_push(auth)`.
  3. `crates/backend_core/src/git_sync/mod.rs` (`git_commit_documented`):
     - When `prepared.loaded_item` is `Some((loc, detail))` and `prepared.auto_append_to_desc` is `true`:
       - Create the primary commit object with `update_ref = None` (`repo.commit(None, &sig, &sig, &commit_message, &tree, parents)`), so `HEAD` does NOT move yet.
       - Compute `recorded_fix_sha = short_sha_for_oid(&repo, initial_commit_oid)`.
       - Snapshot `detail` before mutating. Update `history.previous_fixes` and `desc.md` with `recorded_fix_sha`, write via `write_item_detail` and `sync_parent_indexes_for_item`, stage `.tracker/` into the private index, write `doc_tree`, and create the follow-up doc commit with `update_ref = Some("HEAD")`, parent `&[&initial_commit]`, and message `format!("[{item_id}]({scope_label}) docs: record fix {recorded_fix_sha}")`.
       - If any step between `write_item_detail` and advancing `HEAD` fails, roll back `.tracker/` (`write_item_detail(&loc.phase_id, &loc.scope_id, &snapshot)` and `sync_parent_indexes_for_item`) and return `Err(...)` with `HEAD` untouched.
       - Set `outcome.commit_sha` to the `HEAD` (doc commit) short SHA, `outcome.recorded_fix_sha` to the reachable `HEAD~1` (primary commit) short SHA, and `outcome.changed_files` to `changed_files_between(&repo, parent_tree.as_ref(), &doc_tree)?`.
     - When `loaded_item` is `None` or `!auto_append_to_desc`, create a single commit directly on `Some("HEAD")` as before.
  4. `crates/backend_core/src/git_sync/mod.rs` (`sync_user_index_paths`):
     - Populate `IndexEntry` stat metadata (`ctime`, `mtime`, `file_size`, and on Unix `dev`, `ino`, `uid`, `gid`) from `fs::symlink_metadata(workdir.join(path))` when the file exists on disk (falling back to `0` / `blob_size` only if metadata cannot be read) so committed files are stat-clean to external `git` CLI (`git diff-index --cached HEAD` / `git status`).
  5. Test assertion updates for reachable 2-commit `HEAD`/`HEAD~1`:
     - Update `workflow_integration.rs:1280-1284`, `done_commit_safety.rs:94-98`, and `cli_integration.rs:2100-2104` to assert that the `recorded_fix_sha` commit (`head_commit.parent(0)`) has `outcome.message` and `head_commit` is the doc-recording commit whose tree contains both workspace and `.tracker/` updates.

Must still work:
  - `pre_staged_hunk_survives_git_commit_documented`, `nested_project_root_commit_stages_tracker`, `dirty_tracker_edit_survives_merge_attempt`, and all CLI/MCP documented-commit tests.

Drops:
  - When `auto_append_to_desc` is enabled and an `item_id` is linked, `git_commit_documented` now creates two commits (`HEAD~1` = primary commit with `outcome.message` and reachable SHA `recorded_fix_sha`; `HEAD` = `.tracker/` doc-recording commit) instead of amending a single commit (which previously orphaned `recorded_fix_sha`).

Tests:
  - `git_sync_aborts_before_push_when_pull_reports_duplicate_ids` in `crates/backend_core/tests/git_safety.rs`: sets up two workspaces sharing a bare remote where a pull introduces a duplicate item ID in two directories, runs `git_sync_with_report` -> returns `Err(TrackerError::GitError(msg))` containing `"duplicate"` and the duplicate ID, and the remote ref is NOT updated with local commits.
  - `git_commit_documented_records_reachable_fix_sha_and_stat_clean_index` in `crates/backend_core/tests/git_safety.rs`: runs `git_commit_documented` for `BUG-0001` with a workspace file -> verifies `repo.revparse_single(&outcome.recorded_fix_sha)` resolves to `head.parent(0)` (reachable in Git history), `head` tree contains updated `item.toml`/`desc.md` + workspace file, and the user `.git/index` entry for the committed workspace file has non-zero `mtime` matching disk metadata.
  - `pull_propagates_merge_reconcile_error_without_advancing_head` in `crates/backend_core/tests/git_safety.rs`: sets up a divergent derived-index conflict where `reconcile_indexes` fails (e.g. corrupt `project.toml` or unreadable phase manifest), runs `pull_repo_with_auth` -> returns `Err(_)` and `HEAD` remains at the pre-pull commit.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'fn git_sync_aborts_before_push_when_pull_reports_duplicate_ids' crates/backend_core/tests/git_safety.rs && grep -q 'fn git_commit_documented_records_reachable_fix_sha_and_stat_clean_index' crates/backend_core/tests/git_safety.rs && grep -q 'fn pull_propagates_merge_reconcile_error_without_advancing_head' crates/backend_core/tests/git_safety.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 47E: Add opt-in `allow_empty_ok` (default `false`) to `git_commit_documented` and MCP (`BUG-0004`)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 47E: Add opt-in allow_empty_ok (default false) to DocumentedCommitInput, git_commit_documented, and MCP git_commit_documented so idempotent reruns on a clean tree succeed

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/git_sync/mod.rs
  - crates/backend_core/src/mcp/mod.rs
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/git_sync/mod.rs:42-189,1846-1970
  - crates/backend_core/src/mcp/mod.rs:855-872,2264-2321
  - crates/nebulous_cli/src/commands/mod.rs:1309-1318,1826-1834,2982-2990
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not change default behavior when `allow_empty_ok` is omitted or `false`.

[PRE_GATE]
grep -q 'fn git_commit_documented_records_reachable_fix_sha_and_stat_clean_index' crates/backend_core/tests/git_safety.rs

[MUTATION_SPEC]
Goal: Callers (including Orrery's `on_step_pass` MCP hook `neb:git_commit_documented`) can pass `allow_empty_ok: true` to succeed cleanly when there are no changes to commit, while strict callers omitting `allow_empty_ok` or passing `false` keep the existing `ValidationError`.

Contract & Behavior:
  1. `crates/backend_core/src/git_sync/mod.rs`:
     - Add `#[serde(default)] pub allow_empty_ok: bool` (defaults to `false`) to `DocumentedCommitInput`, plus builder `pub fn with_allow_empty_ok(mut self, allow_empty_ok: bool) -> Self`.
     - Add `#[serde(default)] pub nothing_to_commit: bool` to `DocumentedCommitOutcome` (`false` when a commit was created, `true` when `allow_empty_ok` short-circuited an empty commit).
     - In `TrackerStorage::git_commit_documented`: when the staged tree matches `parent_commit.tree_id()` or `changed_files.is_empty()`:
       - If `!input.allow_empty_ok`, keep returning `Err(TrackerError::ValidationError("No changes to commit in workspace or .tracker/".to_string()))`.
       - If `input.allow_empty_ok`, create NO commit, do NOT mutate `item.toml`/`desc.md` or `.git/index`, and return `Ok(DocumentedCommitOutcome { commit_sha: head_sha.clone(), recorded_fix_sha: head_sha, message: prepared.commit_message, item_id: prepared.item_id, scope_id: prepared.resolved_scope, changed_files: Vec::new(), documentation_updated: false, nothing_to_commit: true })` (where `head_sha` is `short_sha_for_oid(&repo, parent.id())` when `parent_commit` is `Some`, else empty string).
  2. `crates/backend_core/src/mcp/mod.rs`:
     - Add `#[serde(default)] allow_empty_ok: bool` to `tool_git_commit_documented`'s `Args` struct and forward it to `DocumentedCommitInput`.
     - Add `"allow_empty_ok"` (`"type": "boolean"`, `"default": false`) to `git_commit_documented`'s `input_schema` properties.
  3. `crates/nebulous_cli/src/commands/mod.rs`:
     - Supply `allow_empty_ok: false` in the three existing `DocumentedCommitInput { ... }` struct literals (`handle_close`, `handle_sync`, `GitCommand::Commit`).

Must still work:
  - Existing strict tests expecting `ValidationError("No changes to commit in workspace or .tracker/")` when `allow_empty_ok` is omitted or `false`, plus MCP `git_commit_documented` dry-run and schema tests.

Drops: none

Tests:
  - `git_commit_documented_allow_empty_ok_is_opt_in_via_api_and_mcp` in `crates/backend_core/tests/workflow_integration.rs`: on a clean repo with committed `.tracker/`, calls `git_commit_documented` with default `allow_empty_ok: false` -> returns `Err(TrackerError::ValidationError(_))`; calls with `with_allow_empty_ok(true)` -> returns `Ok(outcome)` with `outcome.nothing_to_commit == true`, `outcome.changed_files.is_empty()`, `!outcome.documentation_updated`, and `HEAD` unchanged; invokes MCP `git_commit_documented` with `{"summary":"orrery step pass","dry_run":false,"allow_empty_ok":true}` -> succeeds with `nothing_to_commit == true` and `HEAD` unchanged.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'pub allow_empty_ok: bool' crates/backend_core/src/git_sync/mod.rs && grep -q 'pub nothing_to_commit: bool' crates/backend_core/src/git_sync/mod.rs && grep -q 'allow_empty_ok' crates/backend_core/src/mcp/mod.rs && grep -q 'fn git_commit_documented_allow_empty_ok_is_opt_in_via_api_and_mcp' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 47F: Move `neb done -c` close+commit+rollback into `backend_core` under one `with_write_lock` and surface rollback failures (`BUG-0015` / `AUD-08`)

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 47F: Execute neb done -c close+commit+rollback inside a single backend_core with_write_lock and surface any rollback errors

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/workflow/lifecycle.rs
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/nebulous_cli/tests/done_commit_safety.rs
READ_ONLY_REFS:
  - crates/backend_core/src/workflow/lifecycle.rs:493-554
  - crates/nebulous_cli/src/commands/mod.rs:1290-1360
  - crates/nebulous_cli/tests/done_commit_safety.rs:1-166
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not change `neb done` CLI flags, JSON output schema, or non-`-c` close behavior.

[PRE_GATE]
grep -q 'pub allow_empty_ok: bool' crates/backend_core/src/git_sync/mod.rs

[MUTATION_SPEC]
Goal: `neb done -c` executes validation, snapshotting, close, manifest adjustment, documented commit, and rollback inside a single `TrackerStorage::with_write_lock` in `backend_core`, never discarding rollback errors.

Contract & Behavior:
  1. `crates/backend_core/src/workflow/lifecycle.rs`:
     - Add `pub fn close_item_and_commit(&self, item_id: &str, summary: impl Into<String>, commit_override: Option<&str>, merged: bool, paths: &[String]) -> Result<(StatusChange, DocumentedCommitOutcome)>`:
       - Runs the entire flow inside a single `self.with_write_lock(|_| { ... })`.
       - Resolves `normalized_id` and `loc`, builds `DocumentedCommitInput` (`item_id`, `scope_id`, `summary`, `tracker_only: paths.is_empty()`, `paths`, `allow_empty_ok: false`), and calls `self.validate_documented_commit(&commit_input)?` before any mutation.
       - Snapshots `(loc, detail_snapshot)` and `timer_snapshot = self.read_active_timer()?`.
       - Calls `self.close_item_reporting(&normalized_id, &summary_str, commit_override, merged)?`.
       - If `auto_append_to_desc` is enabled and `detail.manifest.history.previous_fixes` is non-empty, pops the placeholder fix entry and writes the manifest via `self.write_item_manifest(...)`.
       - Calls `self.git_commit_documented(commit_input)`.
       - On `Ok(commit)`: reloads the updated `ItemDetail` via `self.load_item_detail_by_id(&normalized_id)?.1` and returns `Ok((StatusChange { detail, stopped_timer: change.stopped_timer }, commit))`.
       - On `Err(commit_err)`: runs an explicit rollback helper (`rollback_close_and_commit`) that restores `write_item_detail(&loc.phase_id, &loc.scope_id, &detail_snapshot)`, `reconcile_indexes()`, and (when `timer_snapshot` is `Some`) `write_active_timer(&timer)`, collecting EVERY rollback error instead of ignoring them with `let _ = ...`.
       - If all rollback steps succeed, returns `Err(commit_err)`. If any rollback step fails, returns `Err(TrackerError::GitError(format!("{commit_err}; rollback also failed: ...")))` including both the commit error and all rollback error messages.
  2. `crates/nebulous_cli/src/commands/mod.rs` (`handle_close`):
     - When `commit_changes` is `true`, delegate to `storage.close_item_and_commit(&resolved_id, summary, commit, merged, paths)?`, call `warn_if_capped(change.stopped_timer.as_ref())`, and pass `(change.detail, Some(commit_outcome.commit_sha), summary, json)` to `render_close`.
     - Remove the unlocked rollback block (`let _ = ...`) from `handle_close`.

Must still work:
  - Existing tests in `done_commit_safety.rs` (`done_commit_does_not_commit_untracked_secret_env`, `policy_rejected_done_commit_leaves_item_open_without_fix_entry`) and `cli_integration.rs` `neb done -c` tests.

Drops: none

Tests:
  - `post_close_commit_failure_rolls_back_item_and_timer` in `crates/nebulous_cli/tests/done_commit_safety.rs`: starts a timer on `BUG-0001`, creates a Git ref lockfile (`.git/<head_ref_name>.lock`) so `validate_documented_commit` passes but `git_commit_documented` fails when advancing `HEAD`, runs `neb done BUG-0001 -m "Trigger post-close commit failure" -c` -> exits non-zero, `BUG-0001` is rolled back to its pre-close status with zero `previous_fixes`, `HEAD` is unchanged, and the active timer on `BUG-0001` is restored.
  - `close_item_and_commit_waits_for_write_lock` in `crates/backend_core/src/workflow/lifecycle.rs` (unit test): holds `.tracker/.lock` on a background thread for ~300ms while calling `close_item_and_commit` on another thread -> proves `close_item_and_commit` blocks on the outer `with_write_lock`.
  - `close_item_and_commit_surfaces_rollback_failure` in `crates/backend_core/src/workflow/lifecycle.rs` (unit test): invokes the rollback helper when restoring item/timer state fails -> asserts the returned `Err(TrackerError::GitError(msg))` contains both the original commit failure and `"rollback"`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && grep -q 'pub fn close_item_and_commit' crates/backend_core/src/workflow/lifecycle.rs && grep -q 'close_item_and_commit' crates/nebulous_cli/src/commands/mod.rs && grep -q 'fn post_close_commit_failure_rolls_back_item_and_timer' crates/nebulous_cli/tests/done_commit_safety.rs && grep -q 'fn close_item_and_commit_waits_for_write_lock' crates/backend_core/src/workflow/lifecycle.rs && grep -q 'fn close_item_and_commit_surfaces_rollback_failure' crates/backend_core/src/workflow/lifecycle.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```
