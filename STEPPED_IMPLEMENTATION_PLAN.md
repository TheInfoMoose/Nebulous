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

**Phase goal (Phase 26):** Unblock `cargo deny`/CI (`url >= 2.5.9`), fix `scripts/phased-step.sh` under `set -euo pipefail` + macOS/backticks, align `.orrery/skills/repo-rules.md` and `.gitignore` with the phased workflow, fix broken `--all` doc examples in `docs/USAGE.md`, and remove `std::env::set_var`/`remove_var` hazards in `storage_integration.rs`.

**Side findings, bugs and scope lapses** become **pending neb items**, reviewed once at phase end. Every Up Next row has a neb task (`source_ref` = step ID); commits start with `[<TASK-ID>](<scope>)`.

**Regression aversion policy (2026-11-25):** Every bug fix must ship with a dedicated regression test proving the failure before/after. Routine and high-risk work are split across separate phases; breaking API/protocol changes and large structural refactors stay out of bug-fix phases.

### Up Next

| # | Step | Command | Scope | Target Files | Waits on |
| :---: | :--- | :---: | :--- | :--- | :--- |
| 1 | **STEP 44A** | `/goal` | Update `url` (`>= 2.5.9`) in `Cargo.lock` to resolve `RUSTSEC-2026-0592` (`BUG-0008` / `AUD-01`) | `Cargo.lock` | — |
| 2 | **STEP 44B** | `/goal` | Fix `scripts/phased-step.sh` `code_stamp()` under `set -euo pipefail`, portable `find`, and backtick stripping in `ALLOWED` (`BUG-0016` / `AUD-09`) | `scripts/phased-step.sh`, `scripts/validate-step.sh` | — |
| 3 | **STEP 44C** | `/goal` | Align `.orrery/skills/repo-rules.md` and `.gitignore` with phased workflow (`TASK-0115` / `AUD-30`) | `.orrery/skills/repo-rules.md`, `.gitignore` | — |
| 4 | **STEP 44D** | `/goal` | Fix `docs/USAGE.md` removed `--all` examples on `neb git commit` and `neb sync` (`BUG-0017` / `AUD-10`) | `docs/USAGE.md` | — |
| 5 | **STEP 44E** | `/goal` | Replace `std::env::set_var`/`remove_var` in `storage_integration.rs` with `*_in`/`*_from`/`*_to` helpers (`TASK-0116` test-hazard slice / `AUD-33`) | `crates/backend_core/tests/storage_integration.rs` | — |

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


### Future phases

Outlines only (goal, done criteria, and step titles). `/phase-plan` writes the contract step blocks just before each phase starts.

- **Phase 27 — Storage Containment, Locking & Core Data Integrity (High-Risk Core)**
  - **Goal:** Eliminate `.tracker/` index corruption, 30s same-thread lock deadlocks, symlink root escapes, unsynchronized mutators, and watcher shutdown/symlink hangs, each guarded by a regression test.
  - **Done when:** Moving `TASK-*` items across phases leaves zero ghost stubs; mixed relative/canonical `TrackerStorage` paths never self-deadlock; symlinked `.tracker/` roots are rejected; all public `TrackerStorage` mutators, timer quarantine, and `projects.toml` writes hold cross-process file locks; `TrackerWatcher::drop` stops promptly under lock contention and skips directory symlinks.
  - **Steps:**
    - Fix `move_item_to_phase` to remove `TASK-*` stubs via `ScopeItems::remove_id` with regression test (`BUG-0012` / `AUD-05`)
    - Canonicalize `HELD_WRITE_LOCKS` and `PROJECT_CACHE` keys to prevent same-thread 30s self-deadlock (`BUG-0013` / `AUD-06`)
    - Reject symlinked `.tracker/` roots in `discover` and `guard_tracker_path` (+ Unix `O_NOFOLLOW` hardening) with containment tests (`BUG-0010` / `AUD-03`)
    - Lock public `TrackerStorage` mutators, `read_active_timer` quarantine, and `projects.toml` registry (`projects.lock`) (`BUG-0020` / `AUD-17`)
    - Add `AtomicBool` shutdown signal to `TrackerWatcher::drop` and skip symlinks in `hash_index_files` (`BUG-0023` / `AUD-20`)

- **Phase 28 — Secret Vault, Keyring & Security Hardening (High-Risk Security)**
  - **Goal:** Close vault file creation permission windows, keyring/vault credential split-brain, cross-host HTTPS PAT leakage, prompt-injection gaps, and un-zeroized PAT residency without breaking existing user vaults.
  - **Done when:** `vault.key`/`vault.enc` are created with `0o600` atomically, directory/ACL hardening errors propagate, `/tmp/nebulous` vault fallback is rejected, and Windows replacement avoids destructive remove-then-rename; `store_secret` prevents stale keyring reads and `git_sync` respects `vault_only`/`NEBULOUS_DISABLE_KEYRING`; `libgit2` blocks cross-host HTTPS PAT redirects; `agent_context` sanitizes all single-line untrusted fields; `git_sync` PAT buffers are zeroized and SSH passphrase stripping warns/refuses safely.
  - **Steps:**
    - Add mode-aware `atomic_write_bytes_with_mode` (`0o600`), propagate `ensure_secure_dir` errors, disallow `temp_dir()` vault fallback, and harden Windows replace/ACL handling (`BUG-0009` / `AUD-02`)
    - Fix `SecureStorage` keyring/vault split-brain and route `git_sync` PAT functions exclusively through `SecureStorage` (`BUG-0011` / `AUD-04`)
    - Pin `libgit2` PAT credentials and redirect policy to the configured remote host (`BUG-0019` / `AUD-14`)
    - Sanitize all single-line untrusted fields in `agent_context::synthesize_bounded_prompt` (`BUG-0018` / `AUD-12`)
    - Zeroize `FALLBACK_PAT_STORE` and `GitAuthMaterial::pat`, and guard SSH passphrase stripping (`TASK-0110` non-breaking slice / `AUD-13`)

- **Phase 29 — Git Sync & Documented Commit Atomicity (High-Risk Git)**
  - **Goal:** Make `git_sync`, `git_commit_documented`, and `neb done -c` transactional, lock-safe, and regression-tested against merge-reconcile errors, duplicate-ID pushes, and post-close commit failures.
  - **Done when:** Merge-reconcile errors propagate; `git_sync` aborts before `git_push` on `duplicate_ids`; `git_commit_documented` records reachable commit SHAs, keeps `.git/index` stat-clean, rolls back cleanly on failure, and supports opt-in `allow_empty_ok`; `neb done -c` executes close+commit+rollback inside a single `with_write_lock` in `backend_core` and surfaces rollback errors.
  - **Steps:**
    - Propagate merge-reconcile errors, block `git_push` on `duplicate_ids`, and fix `git_commit_documented` pre-amend window, fix SHA, and index metadata (`BUG-0014` / `AUD-07`)
    - Add opt-in `allow_empty_ok` (default `false`) to `git_commit_documented` for idempotent Orrery reruns (`BUG-0004`)
    - Move `neb done -c` close+commit+rollback into `backend_core` under one `with_write_lock` and surface rollback failures with a commit-failure regression test (`BUG-0015` / `AUD-08`)

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
    - Atomize/lock multi-write MCP handlers, return workspace-relative paths, align `git_fetch` `dry_run` contract, and isolate `switch_project` (`BUG-0021` / `AUD-18`)

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

**Open before future phases:**
1. **Before Phase 28 (`AUD-13` / `TASK-0110` KDF migration):** Should `NEBULOUS_VAULT_MASTER_KEY` move from single-round SHA-256 to Argon2id/scrypt via a versioned vault envelope (backward-compatible read of v1 vaults, upgrade on write), or stay parked?
2. **Before Phase 34 (`AUD-11` / `TASK-0109` Suite-3 MCP tools):** Should the 4 Suite-3 release-pin MCP tools (`record_rolling_update`, `record_verification_run`, `pin_stable_release`, `rollback_to_pinned_release`) be neutralized in place (non-breaking) or feature-gated/removed?

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
