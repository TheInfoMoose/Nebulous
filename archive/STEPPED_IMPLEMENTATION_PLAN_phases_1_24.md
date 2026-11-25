# Nebulous Phase 4: CLI-First Architecture, 3-Stage Lifecycle, Predictive Assist & Retro Amber TUI Plan

This document defines the stepped, Argon-structured implementation plan to archive the COSMIC desktop GUI (`crates/cosmic_interface`), refocus Nebulous wholly on the `neb` CLI/TUI (`crates/nebulous_cli` + `crates/backend_core`), simplify development phasing to the 3 canonical stages (`pre_production`, `production`, `post_production`), add predictive field inference and zero-ID workflow automation, and build a retro Amber-on-Black phosphor terminal interface (`ratatui` + `crossterm`) with an unclipped ASCII Nebula + `NEBULOUS` centerpiece header and responsive scaling locks.

---

## Step Execution Roadmap

**How to read this:** work goes strictly top to bottom through **Up Next**, one step at a time. The **#** column is the only order; step IDs are kept because commits and notes cite them. A step whose "Waits on" names an Open Question or a **[USER]** action doesn't start until that's resolved. A step moves to **Completed** only after its `POST_GATE` exits 0. `neb plan import` (STEP 24B–24D) reads these two tables, so keep their columns as they are.

**Who runs what (2026-11-09):** Gemini executes one step at a time from its Argon block. It never commits, never edits this file, and never edits anything under `/srv/devshare/NovOS` (reading the paths a step lists under READ_ONLY_REFS is fine). Per step: `scripts/validate-step.sh start <ID>` before Gemini begins (snapshots HEAD, this file's hash and the dirty tree), then `scripts/validate-step.sh check <ID>` afterwards (no commits, this file untouched, nothing outside ALLOWED_FILES, POST_GATE exits 0). Claude also reads the diff against MUTATION_SPEC, then commits and moves the row to **Completed**. Step IDs belong to this repo; a NovOS step is always written "NovOS STEP …".

**Phase goal (Phase 25, frozen; approved by user 2026-11-20):** Nebulous is listed in the official MCP Registry as `io.github.TheInfoMoose/nebulous`, installable as a Linux x86_64 MCP bundle (`.mcpb`) attached to the GitHub release. `mcpb/manifest.json` (MCPB 0.3, `neb mcp --root ${user_config.root}`) and `server.json` are committed templates. `release.yml` packs the bundle with pinned `@anthropic-ai/mcpb@2.1.2`, fills in version, asset URL and SHA-256, smoke-tests it and attaches it. A `publish-registry` job behind the `mcp-registry` environment (your approval) re-checks the live asset's hash and publishes with cosign-verified `mcp-publisher` v1.8.8 over GitHub OIDC. Also: TASK-0088 (`neb init` ignores timer files), TASK-0095 (docs name only tested clients), TASK-0100 (agy headless allow rule), and the CHANGELOG for Phases 23–25. You create the environment, cut v0.2.0 and approve the publish; Claude checks the bundle before approval and the listing after. Nothing else. (Phase 24: COMPLETED 2026-11-20, CI green on ubuntu/macOS/Windows at `47ff52f`; Claude Code and agy live check passed. Phase 23: COMPLETED 2026-11-20, CI green on ubuntu/macOS/Windows at `922509b`; Orrery live check passed. Phase 22: COMPLETED 2026-11-18, CI green on ubuntu/macOS/Windows at `4da9cee`; live check passed; v0.1.0 released 2026-11-18. Phase 21: COMPLETED 2026-11-18, CI green on ubuntu/macOS/Windows at `94b924f`; audit report written. Phase 20: COMPLETED 2026-11-17, CI green on ubuntu/macOS/Windows. Phase 19: COMPLETED 2026-11-17. Phase 5–7 Completion and Phase 12: COMPLETED 2026-11-15. Phases 13–18: COMPLETED 2026-11-16.)

**Side findings** become **pending neb items** (`neb status <ID> pending`), not new steps or mid-phase questions. The user reviews them once, when Up Next is empty. Every Up Next row has a neb task (`source_ref` = step ID); commits start with `[<TASK-ID>](<scope>)`.

**Orrery (2026-11-18):** steps can also run through Orrery (`.orrery/`, Agents mode, `orrery run --step <position>`): the `hybrid:step_start` hook does `neb start` + `validate-step.sh start`, `agy-impl` executes the Argon block, `claude-val` reads the diff, and the `hybrid:step_check` gate runs `validate-step.sh check` on the host. Failures go back to the implementer automatically, up to 2 retries. Claude still reads the diff, commits and moves the row. See the `orrery-setup` skill.

### Up Next

| # | Step | Command | Scope | Target Files | Waits on |
| :---: | :--- | :---: | :--- | :--- | :--- |
| 1 | **STEP 43C** | `/goal` | MCP bundle manifest and `server.json` templates | `mcpb/manifest.json` (NEW), `server.json` (NEW) | — |
| 2 | **STEP 43D** | `/goal` | Release packs, hashes and smoke-tests the `.mcpb` | `.github/workflows/release.yml` | — |
| 3 | **STEP 43E** | `/goal` | Release publishes to the MCP Registry after approval | `.github/workflows/release.yml` | — |
| 4 | **STEP 43F** | `/goal` | CHANGELOG, README Releasing, USAGE bundle section | `CHANGELOG.md`, `README.md`, `docs/USAGE.md` | — |
| 5 | **[USER] CI (Phase 25)** | — | Push `main` (incl. `d9a20a1`); Claude reads the CI matrix | — | **[USER]** runs "User Step: Push and Report CI Matrix (Phase 25)". |
| 6 | **[USER] ENV (Phase 25)** | — | Create the `mcp-registry` environment with you as required reviewer | repo settings | **[USER]** runs "User Step: Create the `mcp-registry` Environment". |
| 7 | **[USER] RELEASE (Phase 25)** | — | Bump to 0.2.0, commit, tag, push | `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md` | **[USER]** runs "User Step: Release v0.2.0" after the CI and ENV rows. |
| 8 | **[CLAUDE] BUNDLE (Phase 25)** | — | Check the released `.mcpb` before you approve `publish-registry` | — | The RELEASE row's release run reaches `publish-registry` (waiting). |
| 9 | **[CLAUDE] LISTING (Phase 25)** | — | Verify the registry listing | this plan file | You approve `publish-registry` and it goes green. |

NovOS **STEP 13B** was Phase 9 here (STEP 25A–25U, 27A–27E, all COMPLETED). On 2026-11-11 the release engine moved to NovOS (Phase 11 — Separation, end of this file); 29B–29H removed it from Nebulous and made Nebulous neutral (COMPLETED).

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

### Open Questions

**Resolved 2026-11-15 (user):** OQ14-1 to OQ14-5 (STEP 14 re-split) take their defaults: drop the flattened `tools/call` keys (14E); destructive MCP tools default to `dry_run: true` (14G/14H); a request without `jsonrpc: "2.0"` is rejected with `-32600` (14C); supported protocol versions `2025-06-18`, `2025-03-26`, `2024-11-05`, unknown → `2025-06-18` (14D); agent-context caps `desc.md` 16 KiB, scope description and each fix summary 1 KiB, with a truncation marker, applied everywhere the prompt is used (14I).

**Resolved 2026-11-14 (user):** 24C D3: `source_ref` is written atomically with the item, never patched after creation ("data collision potentially causing automation fault, I can't"). New prerequisite steps 24G (CreateItemInput literals → `..CreateItemInput::new`) and 24H (`CreateItemInput.source_ref`) run before 24C. D1 (match scopes, not components) and D2 (reuse `match_file_to_best_scope`) accepted.

**Resolved 2026-11-11 (user):** OQ8–OQ13 (neutral release engine) are withdrawn. The release engine moves to NovOS instead; see "Phase 11 — Separation" at the end of this file.

**Resolved 2026-11-09 (user):**
- **Execution model:** Claude writes and re-splits steps, validates each finished step and commits; Gemini executes one step at a time, never commits, never edits this file. Gemini does not touch NovOS.
- **OQ1:** Claude re-splits STEP 13–22 into Argon steps before they run, starting with STEP 13. **Superseded 2026-11-15:** STEP 16–22 were re-split all at once (see "Phase 5–7 Completion" at the end of this file); no more just-in-time blocks.
- **OQ2:** no hook port. A post-hoc `scripts/validate-step.sh <STEP_ID>` (scope check + POST_GATE) replaces the hooks for this repo.
- **OQ3:** keep STEP 15 at #2; STEP 23F waits on it.
- **OQ4:** add a new step before 24A that gives `ItemMetadata` a constructor and moves the six struct-literal sites onto it; 24A then stays within 3 files.
- **OQ5:** default accepted as written below.
- **OQ6a:** yes. A `plan_import` MCP tool (`dry_run` default `true`) becomes its own small sub-step after 24D.
- **OQ6b:** NovOS pulls STEP 13A (`docs/NEB_RELEASE_CONTRACT.md`, not yet written) forward. The `neb release` steps then go in right after STEP 13 + 15, before the 23x series.
- **OQ7:** stale. Both trackers are already `active_phase = "production"`.

Original questions, kept for reference:

1. **Steps 13–22 predate the current criteria.** They have no `ALLOWED_FILES` / `READ_ONLY_REFS` / `FORBIDDEN_SCOPE`, no single-command gates, and most touch more than 3 files. Recommendation: re-split each one with `/plan-argon` before it runs, starting with STEP 13 since it's first in the queue. Until then the roadmap rows above copy their old scope lists.
2. **`/goal`, `/boost`, `/plan-argon` and the step-lock hooks only exist in NovOS's `.claude/`.** This repo has no `.claude/`. The commands in this plan assume they get ported (skills at minimum; hooks optional). Port before STEP 13, or run steps by hand following the same four-phase gates?
3. **STEP 15 before STEP 23F?** Without `schema_version`, a `neb` binary older than 23F hits a hard error on any `type = "task"` item. Default: yes, keep STEP 15 at #2. Answer "Skip 15 for now" to drop that wait, and every machine has to upgrade `neb` before touching a tracker that has tasks.
4. **STEP 24A touches 4 files.** Adding a field to `ItemMetadata` breaks its six struct-literal sites (one in `lifecycle.rs:117`, five in two test files). Each needs a one-line `source_ref: None`. Accept the exception, or add a constructor first as its own step?
5. **Plan import field ownership.** Default: the plan sets title, scope and type at creation and updates the title on re-import. A plan row that is COMPLETED closes an open item, but import never reopens a closed item and never changes stage, status (other than that close), severity, effort or `desc.md`. New items land in `--phase` (default `active_phase`). Confirm or adjust.
6. **MCP + NovOS STEP 13B.** (a) Fold a `plan_import` MCP tool (`dry_run` default `true`) into STEP 14 when it gets re-split? (b) NovOS STEP 13B (`neb release` per `docs/NEB_RELEASE_CONTRACT.md` in NovOS) is waiting on this repo with no step here. Where should it go in this queue?
7. **Both trackers have `active_phase = "pre_production"`.** Both projects are past planning. Switch to `production` before seeding (`neb phase use production`), or seed into pre-production on purpose?

---

### Parked

Tracked in neb since 2026-11-18: `neb list --status pending`. Reviewed at phase end; only picked items become steps. Migrated: A3 → Orrery tracker; A4 → BUG-0001, A5 → TASK-0084, A6 → BUG-0002, jiff 1.0 → TASK-0085.

#### Archive (pre-neb list, kept for history; do not add here)

_Phase 21 review (2026-11-18): user picked audit blockers A1 (→ Orrery `args`, user carries it) + A2 (→ STEP 40A), the Windows watcher flake (→ 40B) and the release.yml deny mismatch (→ 40C). Audit A3–A6 and jiff 1.0 stay parked._
_Side findings, one line each: `file:line — problem — suggested fix`. Reviewed once at phase end; only items the user picks become steps._
- **→ neb (A3 Orrery; A4 BUG-0001, A5 TASK-0084, A6 BUG-0002)** (Phase 21 audit, 2026-11-18) Audit A3 (Orrery: positional step numbers, no start-at-step), A4 (`plan_import` deny_unknown_fields), A5 (MCP `2025-11-25`), A6 (`git_fetch` dry run without origin) — see "Phase 21 audit report".
- **→ STEP 40B** `crates/backend_core/tests/workflow_integration.rs:811` — Windows-only flake: CI run 44313520994 (`9a9f17b`) failed "watcher must not loop on its own reconciled index writes"; the diff since green `1e2b95e` touches no watcher/storage code (MCP strings, changelog, scripts, release.yml). The 100 ms drain then 250 ms quiet check races late ReadDirectoryChangesW / debouncer events — suggested fix: assert no *reconcile-producing* event within the window (or drain until 500 ms of quiet) instead of "no event at all".

_Phase 5–7 review (2026-11-15): user picked the last 5 items (→ Phase 12); the first 4 stay parked._

_Phase 13 review (2026-11-16): user picked the vault lock, Windows permissions, CI matrix and MCP cap items (→ Phase 14); the rest stay parked._

_Phase 14 review (2026-11-16): user picked macOS roots, CI no-fail-fast, .tracker commit and Windows ACLs (→ Phase 15); the rest stay parked._

_Phase 15 review (2026-11-16): user picked Windows clippy, macOS /home, authors warning and config-dir docs (→ Phase 16); the rest stay parked._

_Phase 16 review (2026-11-16): user picked the Windows unused-import blocker only (→ Phase 17); the rest stay parked._

_Phase 17 review (2026-11-16): user picked the path_containment unused-import blocker only (→ Phase 18); the rest stay parked._

_Phase 18 review (2026-11-16): user supplied both run-5 logs (→ Phase 19: Windows dirs + test split, autocrlf); the rest stay parked._

_Phase 19 review (2026-11-17): user picked chrono → jiff only (→ Phase 20); the rest stay parked._

_Phase 20 review (2026-11-17): user picked assist wording, MCP dry_run text, list_items description, beta + release workflow (guard, bump script, changelog) and the Orrery ↔ Nebulous audit (→ Phase 21); jiff 1.0 stays parked._

- **→ STEP 39A** `crates/backend_core/src/workflow/assist.rs:246,270` — `resolve_active_or_inferred_item_id` runs a full What's Next (incl. git status) just to build an error suggestion — suggest `neb next` in the text instead (changes error wording).
- **→ STEP 31C/31D** `crates/backend_core/src/storage/mod.rs:839` (`read_active_timer`, after 18B) — corrupt/orphan timer is quarantined silently; backend_core has no warning channel — return a recovery notice the CLI/TUI can print.
- **→ STEP 39B** `crates/backend_core/src/mcp/mod.rs` (tools_list: `switch_project`, `ingest_project_directory`, `rollback_to_pinned_release`) — their `dry_run` schema text says "reports git status … without network access or commits" (copied from 14G per my spec) — reword per tool, e.g. "reports what would change without writing".
- **→ STEP 31B/31D** `crates/backend_core/src/workflow/lifecycle.rs:240,269` — `neb done` / status changes stop timers internally, so the 18C cap warning only prints for `neb timer stop` — surface `capped_from` from those paths too.
- **→ STEP 30A** `crates/backend_core/src/workflow/mod.rs:7`, `crates/backend_core/src/lib.rs:25` — 19A spec asked to re-export `StartedWork` but these files weren't in its ALLOWED_FILES (my spec) — add `StartedWork` to both `pub use` lists.
- **→ STEP 30B** `crates/nebulous_cli/src/commands/mod.rs:322` (after 22A) — `execute_to_string` is `pub` (re-exported `lib.rs:10`), so a library caller passing `Commands::Mcp` now panics via `unreachable!` (my spec) — return an error like "use execute() for mcp" instead.
- **→ STEP 30C** `docs/ARCHITECTURE.md:68` (after 22A/22B) — `agent_context` bullet still lists `export_bounded_prompt`, deleted in 22A; my 22B spec missed it — drop it from the list.
- **→ STEP 30C** `scripts/install.sh:111` (after 22C, REGRESSION from my spec) — user mode with no `~/.local` yet: neither `~/.local/bin` nor `~/.local` is writable, so SUDO=sudo creates a root-owned `~/.local/bin` — test writability of the nearest existing ancestor of BINDIR instead.
- **→ STEP 30C** `README.md:72` (after 22C) — still says `sudo ./scripts/install.sh --system`, which now exits with the run-without-sudo refusal — change to `./scripts/install.sh --system`.
- **→ STEP 32D** (audit 2026-11-15, should-fix) `.github/workflows/ci.yml` — CI runs only on ubuntu-latest while `atomic.rs:107` and `crypto/mod.rs:595` have Windows-only branches — add a macOS/Windows matrix or document Linux-only.
- **→ STEP 32C** (audit 2026-11-15, should-fix) `crates/backend_core/src/crypto/mod.rs:595` — `set_secure_file_permissions` is a no-op on non-unix, so the vault master key file has default ACLs on Windows — restrict the ACL, or document it.
- **→ STEP 32A** (audit 2026-11-15, should-fix) `crates/backend_core/src/crypto/mod.rs:440` — `vault_lock` is an in-process Mutex; two `neb` processes storing secrets at once can lose an entry — use a file lock around vault read-modify-write.
- **→ STEP 39C–39E** (audit 2026-11-15; user 2026-11-16) `Cargo.toml:10` — `0.1.0` was hard-coded by hand during test builds; no version automation exists. User convention: `0.0.x` = alpha/pre-alpha, `0.1.0` = beta, `1.0.0` = release (then major.minor). Tags are on `v0.0.x` — pick the current stage, set the version to match, and decide whether to add release/version automation.
- **→ STEP 39B** (audit 2026-11-15, nice-to-have) `crates/backend_core/src/mcp/mod.rs:~571` — `list_items` description says "bugs and features"; tasks exist too — reword.
- **→ STEP 32B** (Phase 13 scope) `crates/backend_core/src/mcp/mod.rs:1353,1384` — MCP `update_item_status` / `close_item_with_fix` don't report a capped session — add `capped_from` to their JSON.
- **→ STEP 34D** (after 32A) `docs/ARCHITECTURE.md:88` — config-dir tree lists `vault.enc` but not the new `vault.lock` — add a line `vault.lock  # advisory lock for vault read-modify-write`.
- **→ STEP 33D** (after 32C, deferred by plan) `crates/backend_core/src/crypto/mod.rs` (`set_secure_file_permissions`, `ensure_secure_dir`) — Windows still sets no ACLs (32C only documented it) — once Windows CI is green, restrict the DACL to the current user via `windows-sys`.
- **→ resolved (CI run 7 all green)** (CI 2026-11-16 run 5, Windows) FIRST WINDOWS TEST RUN: clippy passes (34A/35A/36A worked); every target except two passes, including backend_core lib tests, so 33D's `windows_vault_file_acl_is_user_only` PASSED.
- **→ STEP 37A/37B** (CI 2026-11-16 run 5, Windows, expected) `crates/backend_core/tests/storage_integration.rs:1894` — `/etc` assertion fails: on Windows `Path::new("/etc")` has no drive prefix, so it isn't absolute and resolves to `<drive>:\etc`, and `UNSAFE_SYSTEM_DIRS` has no Windows entries — this is the parked Windows-system-dirs item: gate the Unix-path assertions `#[cfg(unix)]`, add Windows dirs (`C:\Windows`, `C:\Program Files`, `C:\Program Files (x86)`, `C:\ProgramData`, `C:\Users`) matched case-insensitively, with `#[cfg(windows)]` assertions.
- **→ resolved by Phase 20 (38A–38I), CI green 2026-11-17** (2026-11-17, user decision: NEXT PHASE) chrono → jiff migration — RUSTSEC-2026-0543: chrono is unmaintained, no fixed version; ignored in deny.toml by 37D as a temporary exception. Phase 20 replaces chrono in the 20 files that use it (Utc, DateTime, Duration, TimeZone, NaiveDate, SecondsFormat; serde on stored timestamps) and removes the ignore entry.
- **→ STEP 37C** (CI 2026-11-16 run 5, Windows, CONFIRMED) `crates/backend_core/tests/workflow_integration.rs:1079` — `test_git_fetch_ahead_behind_and_workspace_dirty_tracking`: left `"pub fn remote_version() {}\r\n"`, right `"...\n"` — the runner's global `core.autocrlf=true` converts on checkout; set `core.autocrlf=false` on the test's repos.
- **→ STEP 37B** (found 2026-11-16) `crates/backend_core/tests/storage_integration.rs:1936` — test fixture has `name = "theinfomoose"` (a real username) — use `testuser`.
- **→ STEP 36A** (CI 2026-11-16 run 4, Windows, BLOCKER) `crates/backend_core/tests/path_containment.rs:9` — `ComponentManifest` is imported at the top but used only inside `#[cfg(unix)] mod unix` (:258, :270), so it's unused on Windows and clippy -D warnings fails (35A's fix worked; this is the next target clippy reached) — move the import into `mod unix` (`use backend_core::ComponentManifest;`). Static sweep 2026-11-16 of every other cfg(unix)/cfg(windows) site (lib.rs:489, cli_integration.rs:1839, storage_integration.rs:2011, atomic.rs, crypto/mod.rs) found no other import or local used only under a gate; a cross-target clippy isn't possible here (no Windows target/mingw).
- **→ STEP 35A** (CI 2026-11-16 run 3, Windows, BLOCKER; my 34B spec) `crates/backend_core/src/storage/mod.rs:1799-1802` — `mod tests` is `#[cfg(test)]` but its only test is `#[cfg(unix)]`, so on Windows `use super::*` is unused and clippy `-D warnings` fails (1 error; the 349 `result_large_err` errors are gone, so 34A worked) — make the module `#[cfg(all(test, unix))]` and drop the per-test `#[cfg(unix)]`. Still no Windows tests have run (33D's ACL test unproven).
- **→ STEP 34A** (CI 2026-11-16 run 2, Windows, BLOCKER) `crates/backend_core/src/error.rs:21-25` — clippy `result_large_err` fails 349× on Windows only: `TomlDeserialize { path: Option<PathBuf>, source: toml::de::Error }` reaches the 128-byte limit because Windows `PathBuf` is larger — the clippy step fails, so NO Windows tests ran (33D's ACL test unproven) — box it: `source: Box<toml::de::Error>` and `Box::new(..)` at the ~9 construction sites. Not `#[allow]`.
- **→ STEP 34B** (CI 2026-11-16 run 2, macOS) `crates/backend_core/src/storage/mod.rs:1553` (`storage_integration.rs:1896`) — 33A fixed `/etc`; next failure is `/home`, which macOS canonicalizes to an autofs path (likely `/System/Volumes/Data/home`, a guess) — check BOTH the lexical path and the canonical path against `UNSAFE_SYSTEM_DIRS`, so a listed dir is rejected wherever it resolves.
- **→ STEP 34C** (CI 2026-11-16 run 2, macOS, warning) `Cargo.toml:13` — newer cargo warns `workspace.package.authors` is unused (`cargo::unused_workspace_package_fields`) — inherit it in member crates (`authors.workspace = true`) or drop it.
- **→ STEP 33A** (CI 2026-11-16, macOS, REAL BUG) `crates/backend_core/src/storage/mod.rs:1453,1553` — `is_unsafe_broad_directory` canonicalizes first, and on macOS `/etc`, `/tmp`, `/var` are symlinks to `/private/...`, which aren't in `UNSAFE_SYSTEM_DIRS` — macOS lets them be project roots (fails `storage_integration.rs:1894`) — add `/private/etc`, `/private/tmp`, `/private/var` plus macOS top-level dirs (`/System`, `/Library`, `/Applications`, `/Users`, `/Volumes`).
- **→ STEP 37A/37B** (CI 2026-11-16, Windows, unverified) same test/line reported failing, but the pasted Windows log matches the macOS one byte-for-byte (thread id 42614, 8.58s) — get the real Windows log. Separately: `UNSAFE_SYSTEM_DIRS` has no Windows system dirs (`C:\Windows`, `C:\Program Files`, `C:\Users`) and the test's `/etc`-style assertions are Unix-only — split the test by `cfg` and add Windows dirs (guess until the real log is in).
- **→ STEP 33B** (CI 2026-11-16) `.github/workflows/ci.yml` — `cargo test --workspace` stops at the first failing test binary, so macOS/Windows results for the other test binaries are unknown — use `cargo test --workspace --no-fail-fast` to see every failure per OS.
- **→ STEP 33C** (user 2026-11-16) `.tracker/` — meant to be committed (README: "Git-backed"; `.gitignore:5-7` only excludes runtime files) but never was. 25 files fail `check-neutral.sh`: TASK-0024/27/29/30/32/34/35/36/41/42/46/54 (`item.toml` + `desc.md`) and `phases/production/domains/backend_core/scope.toml` (12 lines), all titles of the Phase 9 release-engine steps (25A–25U) that moved out in 29B–29H — reword those titles neutrally, add `/.tracker/.timer.toml.bak` to `.gitignore`, then commit `.tracker/`.
- **→ neb TASK-0085** (Phase 20 planning, 2026-11-17) `Cargo.toml` — Phase 20 pins jiff 0.2.39 (newest release ≥ 2 weeks old); jiff 1.0.0 shipped 2026-11-15 — once it's ≥ 2 weeks old, move to `jiff = "1"` (check the 1.0 changelog for renamed APIs first).
- **→ Phase 21 [CLAUDE] AUDIT** (user 2026-11-16) Orrery ↔ Nebulous audit — `/srv/devshare/Orrery` stays a separate repo (like NovOS). Audit BOTH repos: they complement each other, no unnecessary overlap, and each delivers its intended utility alone and together. Known touch points: Orrery's `rmcp` client ↔ `neb mcp` protocol-version handshake; the Nebulous tool names/args Orrery's lifecycle hooks call; `### STEP` plan-format parity; shared palette. Read-only audit first, before any step.
- **→ STEP 40C** (Phase 21 planning, 2026-11-17) `.github/workflows/release.yml` ("Supply-chain advisory gate") — runs `cargo deny` with `command: check advisories`, while CI runs `check advisories bans` — use `check advisories bans` so a release can't ship a banned crate CI would reject.


## Root Cause & Architectural Evaluation

1. **GUI Overhead & Disconnected Workflow (`crates/cosmic_interface` & `packaging/`)**:
   - Maintaining the heavy COSMIC desktop application (`crates/cosmic_interface`) and desktop packaging (`packaging/`, `scripts/build-appimage.sh`) split engineering focus and introduced friction instead of a unified terminal workflow where developers and coding agents actually operate.
2. **Rigid, Manual Phasing & Scope Duplication (`crates/backend_core/src/storage/mod.rs` & `workflow/lifecycle.rs`)**:
   - Previously, `neb init` only scaffolded a single arbitrary phase (`01_core_engine`), and adding a domain scope (`neb scope add`) only registered it inside a single phase. In rapid agentic workflows, development naturally flows across three concrete stages:
     - `pre_production` (`pre` — Planning & Design)
     - `production` (`prod` — Active Implementation)
     - `post_production` (`post` — Verification, Polish & Release)
   - Domain scopes (crates/modules) must be mirrored automatically across all 3 canonical stages so items can be created or moved between stages (`neb move` / TUI `m`) without recreating scope manifests.
3. **High-Friction Manual Bookkeeping (`crates/nebulous_cli/src/args.rs` & `commands/mod.rs`)**:
   - Every command previously required explicit IDs, scope flags, and multi-step sequences (`neb init` -> `neb ingest --write-manifests --promote-all` -> `neb project add`, or `neb status <ID> in_progress` -> `neb timer start <ID>` -> `neb context <ID> --copy`, or `neb timer stop` -> `neb close <ID>`).
   - Required fields (`scope`, `phase`, `item_id`, `commit`) lacked predictive inference from the active working directory, modified Git files, active timer, or sole `in_progress` item.
4. **Retro Amber-on-Black TUI with ASCII Nebula Header & Non-Clipping Scaling Locks (`crates/nebulous_cli/src/tui/`)**:
   - The interactive TUI (`neb`) requires a classic CRT Amber-on-Black monochrome phosphor palette (`#000000` background, `#FFB000` primary amber, `#FFD866` high-intensity gold, `#996300` muted amber borders, `#FF5500` critical alert orange-amber).
   - The top header must display an ASCII art cosmic nebula starfield/swirl with ASCII-art `NEBULOUS` centered inside it, guaranteed never to clip or deform (`Wrap` disabled on the logo, centered horizontally using fixed-width tiers and a minimum terminal dimension lock).
   - All menus, tables, and modal input forms (`New Bug`, `New Feature`, `Done/Close`, `Move Stage`) must enforce minimum height/width locks (`Constraint::Min`), viewport auto-scrolling, and responsive pane collapsing (`>= 110` cols 3-pane, `80..109` cols 2-pane + tabbed drawer, `< 80` cols or `< 24` rows minimum-viewport lock guard) so required fields (`Title`, `Scope`, `Stage`, `Severity`, `Summary`) are always visible, unclipped, and accessible.
5. **CI Failure in `test_watcher_detects_external_changes_and_auto_reconciles_indexes` (`crates/backend_core/src/watcher/mod.rs`, `crates/backend_core/src/storage/reconcile.rs`, `crates/backend_core/tests/workflow_integration.rs`) & Archived GUI CI Step (`.github/workflows/ci.yml`)**:
   - **Root Cause 1 — Blocking `notify`'s `inotify` Event Loop Before Recursive Subdirectory Watch Registration**:
     - In `notify 6.1.1` (`src/inotify.rs` lines 273–365), when `write_item_detail` calls `fs::create_dir_all(".../items/BUG-0007")`, the Linux kernel emits `IN_CREATE | IN_ISDIR`. `notify` queues `.../items/BUG-0007` into a deferred `add_watches` vector (line 284), then synchronously invokes `self.event_handler.handle_event(Ok(ev))` (line 343) **before** calling `self.add_watch(path, true, false)` (line 364).
     - Because `TrackerWatcher` executed `storage.reconcile_indexes()` synchronously inside the `RecommendedWatcher` callback:
       1. When `handle_event` fired on `EventKind::Create(CreateKind::Folder)` (`.../items/BUG-0007`), the writer thread (`write_item_detail`) was still inside `temp_file.sync_all()` for `.item.toml.tmp.<pid>.<seq>`, so `items/BUG-0007/item.toml` did not exist yet.
       2. `reconcile_indexes()` scanned `items/BUG-0007`, saw `!item_toml_path.is_file()`, skipped `BUG-0007`, and performed 4 synchronous `atomic_write_str` (`sync_all()`) writes across `scope.toml`, `phase.toml`, and `project.toml` — blocking `notify`'s event loop thread at line 343 before line 364 could register the `inotify` watch on `items/BUG-0007`.
       3. While `notify`'s thread was blocked inside `reconcile_indexes()`, `write_item_detail` finished renaming `item.toml` and `desc.md` inside the unwatched `items/BUG-0007` directory. When `handle_event` finally returned and `notify` registered the watch on `items/BUG-0007`, all file writes inside `items/BUG-0007` had already completed, so no subsequent `inotify` event was ever emitted and `scope.items.bugs.len()` remained `0` (failing line 776 of `workflow_integration.rs`).
   - **Root Cause 2 — Unconditional Index Rewrites in `reconcile_indexes()` (`crates/backend_core/src/storage/reconcile.rs`)**:
     - `TrackerStorage::reconcile_indexes` unconditionally calls `write_scope_to_path`, `write_phase_to_path`, and `write_project` on every run even when the computed manifests are identical to the on-disk manifests, generating unnecessary `sync_all()` disk latency and `inotify` echo events.
   - **Root Cause 3 — Stale `cosmic_interface` Step in `.github/workflows/ci.yml`**:
     - `.github/workflows/ci.yml` line 61 still executes `cargo run -p cosmic_interface -- --self-test`, which will fail immediately once `cargo test --workspace` passes because `cosmic_interface` was archived to `archive/cosmic_interface` in Step 1.

---

## Stepped Implementation Plan (Argon Prompts)

### Step 1: Archive COSMIC GUI & Packaging Artifacts (`Cargo.toml` & `archive/`)

**Recommended Slash Command:** `/goal`

```markdown
[STEP_ID]
STEP-01: Archive COSMIC GUI & Packaging Artifacts (`Cargo.toml` & `archive/`)

[SCOPE_LOCK]
Allowed files/directories to modify or move:
- `Cargo.toml`
- `crates/cosmic_interface` -> `archive/cosmic_interface`
- `packaging` -> `archive/packaging`
- `scripts/build-appimage.sh` -> `archive/scripts/build-appimage.sh`
- `scripts/install.sh`
Do not modify `crates/backend_core` or `crates/nebulous_cli` source logic in this step.

[PRE_GATE]
- Verify workspace layout and confirm `crates/cosmic_interface` exists.

[MUTATION_SPEC]
1. **Archive GUI & Desktop Packaging**:
   - Create `archive/` and `archive/scripts/`.
   - Move `crates/cosmic_interface` to `archive/cosmic_interface`.
   - Move `packaging/` to `archive/packaging`.
   - Move `scripts/build-appimage.sh` to `archive/scripts/build-appimage.sh`.
2. **Update Workspace `Cargo.toml`**:
   - Remove `"crates/cosmic_interface"` from `[workspace] members` so the active workspace members are strictly `["crates/backend_core", "crates/nebulous_cli"]`.
   - Add `exclude = ["archive/cosmic_interface"]` to `[workspace]` in `Cargo.toml` so Cargo never traverses the archived GUI crate.
3. **Update `scripts/install.sh`**:
   - Update `scripts/install.sh` so it builds and installs only the `neb` CLI binary (`cargo build --release -p nebulous_cli`) without building or installing desktop GUI icons/appstream/desktop files.

[POST_GATE]
- Run `cargo fmt --all`
- Run `cargo fmt --all -- --check`
- Run `cargo clippy --workspace --all-targets -- -D warnings`
- Run `cargo test --workspace`
- Verify `bash -n scripts/install.sh` exits `0`.

[HALT_INVARIANT]
- Halt immediately after all `[POST_GATE]` checks pass and report results. Do not proceed to Step 2 until instructed.
```

---

### Step 2: 3-Stage Canonical Phasing (`pre_production`, `production`, `post_production`), Cross-Stage Scope Mirroring & Item Stage Movement (`backend_core`)

**Recommended Slash Command:** `/goal`

```markdown
[STEP_ID]
STEP-02: 3-Stage Canonical Phasing, Cross-Stage Scope Mirroring & Item Stage Movement (`backend_core`)

[SCOPE_LOCK]
Allowed files to modify:
- `crates/backend_core/src/models/phase.rs`
- `crates/backend_core/src/models/mod.rs`
- `crates/backend_core/src/storage/mod.rs`
- `crates/backend_core/src/workflow/lifecycle.rs`
- `crates/backend_core/src/ingestion/mod.rs`
- `crates/backend_core/tests/storage_integration.rs`
- `crates/backend_core/tests/workflow_integration.rs`
Do not modify `crates/nebulous_cli` in this step.

[PRE_GATE]
- Run `cargo test -p backend_core` and confirm baseline tests pass.

[MUTATION_SPEC]
1. **Canonical 3-Stage Definitions & Alias Resolution (`crates/backend_core/src/models/phase.rs` & `storage/mod.rs`)**:
   - Define `pub const CANONICAL_PHASES: &[(&str, &str)] = &[("pre_production", "Pre-Production"), ("production", "Production"), ("post_production", "Post-Production")];`.
   - Implement `pub fn resolve_phase_alias(input: &str) -> &str` in `models/phase.rs` (and re-export in `models/mod.rs`):
     - Maps `"pre"`, `"pre-prod"`, `"pre_prod"`, `"pre-production"`, `"pre_production"`, `"1"` -> `"pre_production"`.
     - Maps `"prod"`, `"production"`, `"active"`, `"2"` -> `"production"`.
     - Maps `"post"`, `"post-prod"`, `"post_prod"`, `"post-production"`, `"post_production"`, `"3"` -> `"post_production"`.
     - Otherwise returns `input.trim()` unchanged so custom phases continue to work seamlessly.
   - Update `TrackerStorage` phase lookup/resolution methods (`phase_dir`, `read_phase`, `set_active_phase`, `resolve_phase_and_scope`, `create_scope`, etc.) to normalize phase identifiers through `resolve_phase_alias` when a canonical alias is supplied.
2. **Multi-Stage Scaffolding in `TrackerStorage::init` (`crates/backend_core/src/storage/mod.rs`)**:
   - Add `TrackerStorage::init_standard(root_or_tracker_dir: impl AsRef<Path>, project_name: impl Into<String>) -> Result<Self>` which initializes `.tracker/` with all 3 canonical stages (`pre_production`, `production`, `post_production`) registered in `project.toml`, sets `active_phase = "pre_production"` (status `InProgress`, others `Planned`), and creates the default `inbox` scope in all 3 stages.
   - In `TrackerStorage::init(root_or_tracker_dir, project_name, initial_phase_id, initial_phase_title)`:
     - Resolve `initial_phase_id` via `resolve_phase_alias`.
     - Always ensure the 3 canonical stages (`pre_production`, `production`, `post_production`) are scaffolded with their `inbox` scope; if a custom `initial_phase_id` (e.g. `"01_core_engine"`) is passed by existing callers/tests, also scaffold and set that custom phase as `active_phase` so all existing tests remain 100% backward-compatible while new projects get the 3 canonical stages out of the box.
3. **Cross-Stage Scope Mirroring (`crates/backend_core/src/workflow/lifecycle.rs` & `ingestion/mod.rs`)**:
   - Update `TrackerStorage::create_scope` (and add `TrackerStorage::ensure_scope_across_phases`): when a scope is created or promoted from a component (`promote_component_to_scope`), automatically mirror the empty `scope.toml` (with identical `ScopeMetadata`: `id`, `target`, `tech_stack`, `description`, `bound_component`, `tracked_files`) and `items/` directory across all registered phases in `project.toml` where that scope does not yet exist.
4. **Single-Call Item Stage Movement (`crates/backend_core/src/workflow/lifecycle.rs`)**:
   - Implement `pub fn move_item_to_phase(&self, item_id: &str, target_phase_id: &str) -> Result<(ItemLocation, ItemDetail)>` on `TrackerStorage`:
     - Resolves `target_phase_id` via `resolve_phase_alias`.
     - Locates the item (`loc`), ensures `target_phase_id` exists, and ensures `loc.scope_id` exists in `target_phase_id` (copying scope metadata from `loc.phase_id` if missing).
     - Moves `item.toml` and `desc.md` to the target phase's scope `items/<ITEM_ID>` directory, removes the old item directory, updates `updated_at`, and atomically reconciles both the source phase/scope and target phase/scope indexes.
5. **Tests (`storage_integration.rs` & `workflow_integration.rs`)**:
   - Add tests verifying `resolve_phase_alias` (`pre`/`prod`/`post`), 3-stage scaffolding on `init_standard` / `init`, automatic scope mirroring across all phases on `create_scope` and `promote_component_to_scope`, and `move_item_to_phase`.

[POST_GATE]
- Run `cargo fmt --all`
- Run `cargo fmt --all -- --check`
- Run `cargo clippy --workspace --all-targets -- -D warnings`
- Run `cargo test --workspace`

[HALT_INVARIANT]
- Halt immediately after all `[POST_GATE]` checks pass and report results. Do not proceed to Step 3 until instructed.
```

---

### Step 3: Assistive/Predictive Engine ("What's Next", Smart Field Inference & Zero-ID Workflow Automation) (`backend_core`)

**Recommended Slash Command:** `/goal`

```markdown
[STEP_ID]
STEP-03: Assistive/Predictive Engine & Zero-ID Workflow Automation (`backend_core`)

[SCOPE_LOCK]
Allowed files to modify or create:
- `crates/backend_core/src/workflow/assist.rs` (new)
- `crates/backend_core/src/workflow/mod.rs`
- `crates/backend_core/src/workflow/lifecycle.rs`
- `crates/backend_core/src/workflow/timer.rs`
- `crates/backend_core/src/lib.rs`
- `crates/backend_core/tests/workflow_integration.rs`
Do not modify `crates/nebulous_cli` in this step.

[PRE_GATE]
- Run `cargo test -p backend_core` and confirm all tests pass.

[MUTATION_SPEC]
1. **Create `crates/backend_core/src/workflow/assist.rs`** (and export from `workflow/mod.rs` and `lib.rs`):
   - **Predictive Context & Field Inference (`PredictiveDefaults`)**:
     - `TrackerStorage::infer_scope_from_context(&self, cwd: Option<&Path>) -> Result<String>`:
       1. If `cwd` is inside a scope's `target` directory (longest prefix match among scopes in the active phase), return that `scope_id`.
       2. Otherwise inspect dirty/modified files in Git (`git2::Repository::open(project_root)`) and match modified file paths against registered scope `target` paths (most modified files wins).
       3. Otherwise if an active timer is running (`.timer.toml`) with a `scope_id`, return that `scope_id`.
       4. Otherwise if the active phase has exactly one non-`inbox` scope, return that scope.
       5. Fall back to `project.default_scope` (`"inbox"`).
     - `TrackerStorage::resolve_active_or_inferred_item_id(&self, explicit_id: Option<&str>) -> Result<String>`:
       1. If `explicit_id` is `Some(id)` and non-empty, normalize case (`id.trim().to_ascii_uppercase()`) and return it.
       2. If `.timer.toml` has an active timer, return `timer.item_id`.
       3. If there is a single `ItemStatus::InProgress` item in the active phase (or across the project if none in active phase), return its ID.
       4. Otherwise return a descriptive `TrackerError::ValidationError` explaining that no active timer or single `in_progress` item was found and suggesting the top recommended item ID.
   - **"What's Next" Recommendation Engine (`WhatsNextReport`, `NextActionKind`, `NextRecommendation`)**:
     - `TrackerStorage::compute_whats_next(&self, cwd: Option<&Path>) -> Result<WhatsNextReport>`:
       - Evaluates current project state and produces a ranked list of actionable recommendations (`recommendations: Vec<NextRecommendation>`) plus `inferred_scope: String`, `active_phase: String`, and `active_timer: Option<TimerStatus>`:
         1. **Unscoped Workspace Warning**: If the workspace has crates/components on disk not yet promoted to scopes (only `inbox` exists), suggest running `neb init` / `neb ingest --promote-all`.
         2. **Active Timer / In-Progress Focus**: If an item is currently `in_progress` or being timed, surface it as the primary focus (`neb done` when finished, or `neb context --copy`).
         3. **Recurring Critical/High Bugs**: Rank open items with `recurrence_count > 0` first (highest severity, then highest recurrence).
         4. **Stagnant Items**: Surface open/in-progress/pending items exceeding `stagnant_item_threshold_days`.
         5. **Highest-Severity Open Items in Active Stage (prioritizing inferred scope)**: Rank remaining `open` items in `active_phase` by `(scope == inferred_scope, severity DESC, item_type == Bug, id ASC)`.
         6. **Stage Advancement Hint**: If `active_phase` has `total_items > 0` and all items in `active_phase` are completed (`closed` or `merged`), recommend advancing to the next stage (`pre_production` -> `production` -> `post_production`, or moving ready items forward).
         7. **Missing Spec Guidance**: For the top recommended item, inspect `desc.md`; if it still contains the default placeholder (`"Describe the problem or specification for:"` or `"TODO: Add reproduction"`), include a field-completion hint (`missing_fields: Vec<String>`) prompting the user/agent to fill reproduction steps or acceptance criteria.
2. **Automated Status + Timer Transitions (`crates/backend_core/src/workflow/lifecycle.rs` & `timer.rs`)**:
   - Implement `TrackerStorage::start_work_item(&self, item_id: Option<&str>, note: Option<String>) -> Result<(ItemLocation, ItemDetail, ActiveTimer, AgentContextBundle)>`:
     - Resolves `item_id` (if `None`, picks the top recommended `open` item from `compute_whats_next` or active item).
     - If another timer is currently running for a *different* item, automatically stops that previous timer and logs its elapsed minutes (`stop_timer()`) instead of failing with `TimerAlreadyRunning`.
     - Transitions the target item's status to `ItemStatus::InProgress` (`update_item_status`).
     - Starts (or keeps) the active timer for the target item (`.timer.toml`).
     - Builds and returns the `AgentContextBundle` so the caller can immediately display/copy the prompt.
   - Update `TrackerStorage::close_item` and `TrackerStorage::update_item_status`:
     - When `update_item_status` transitions an item to `ItemStatus::InProgress` and no timer is running (or a different item's timer is running), automatically start/switch the session timer onto that item.
     - When `close_item` or `update_item_status(..., Closed | Merged)` completes an item and `.timer.toml` is timing that `item_id`, automatically stop the timer and add the elapsed minutes to `effort.logged_time_minutes` before writing the closed manifest.
3. **Tests (`crates/backend_core/tests/workflow_integration.rs`)**:
   - Add integration tests covering `infer_scope_from_context`, `resolve_active_or_inferred_item_id`, `compute_whats_next` ranking & stage-advancement hints, and automatic timer start/switch/stop during `start_work_item` and `close_item`.

[POST_GATE]
- Run `cargo fmt --all`
- Run `cargo fmt --all -- --check`
- Run `cargo clippy --workspace --all-targets -- -D warnings`
- Run `cargo test --workspace`

[HALT_INVARIANT]
- Halt immediately after all `[POST_GATE]` checks pass and report results. Do not proceed to Step 4 until instructed.
```

---

### Step 4: One-Command Bootstrap, Zero-ID Ergonomic CLI Shortcuts & Predictive Output (`nebulous_cli`)

**Recommended Slash Command:** `/goal`

```markdown
[STEP_ID]
STEP-04: One-Command Bootstrap, Zero-ID Ergonomic CLI Shortcuts & Predictive Output (`nebulous_cli`)

[SCOPE_LOCK]
Allowed files to modify:
- `crates/nebulous_cli/src/args.rs`
- `crates/nebulous_cli/src/commands/mod.rs`
- `crates/nebulous_cli/src/lib.rs`
- `crates/nebulous_cli/tests/cli_integration.rs`
Do not add TUI rendering code in this step (TUI is isolated to Step 5).

[PRE_GATE]
- Run `cargo test --workspace` and confirm all tests pass.

[MUTATION_SPEC]
1. **Update CLI Subcommands & Shortcuts (`crates/nebulous_cli/src/args.rs`)**:
   - Make `Cli.command` `Option<Commands>` so running bare `neb` (or `neb tui` / `neb ui`) is valid and dispatches to the interactive dashboard (in Step 5; in Step 4, if `command` is `None` in `execute_to_string`, return the status + "What's Next" dashboard summary).
   - **`Commands::Init`**:
     - Default `--phase-id` to `"pre_production"` and `--phase-title` to `"Pre-Production"`.
     - Add `--no-ingest: bool` (default `false`) and `--no-register: bool` (default `false`). By default, `neb init` scaffolds all 3 canonical stages (`pre_production`, `production`, `post_production`), automatically scans and promotes discovered workspace components into domain scopes across all 3 stages, and registers the project in `projects.toml` in a single step.
   - **Zero-ID Optional `<ID>` on Action Subcommands**:
     - `Commands::Show { id: Option<String> }`: if `id` is omitted, targets the active timer / `in_progress` item (or top "What's Next" item).
     - `Commands::Close { id: Option<String>, ... }` (with `#[command(visible_alias = "done")]`): if `id` is omitted, resolves via `resolve_active_or_inferred_item_id`. Add `--commit-changes` (`-c`) flag to optionally create a documented Git commit `[<ID>](<scope>) <summary>` in the same step.
     - `Commands::Context { id: Option<String>, ... }`: if `id` is omitted, resolves via `resolve_active_or_inferred_item_id`.
   - **New High-Flow Subcommands**:
     - `Commands::Start { id: Option<String>, #[arg(short, long)] note: Option<String>, #[arg(long, default_value_t = true)] copy: bool }`: calls `storage.start_work_item(id.as_deref(), note)`, copies the bounded agent context to clipboard (best-effort if headless), and prints the active item + agent prompt summary.
     - `Commands::Next`: runs `storage.compute_whats_next(cwd)` and prints (or outputs JSON for) the ranked "What's Next" queue, active timer, inferred scope, missing spec hints, and stage advancement recommendation.
     - `Commands::Move { id: Option<String>, stage: String }` (where `stage` can be positional or `--to`; if only one arg is given, treat it as `stage` and infer `id` from active item, or if two args `<ID> <STAGE>` are given, move `<ID>` to `<STAGE>`): moves an item across stages (`pre` / `prod` / `post`).
     - `Commands::Stage { stage: Option<String> }`: quick alias for `neb phase use <stage>` (when `stage` is given, e.g. `neb stage prod`) or `neb phase list` (when omitted).
2. **Predictive Scope Inference & "What's Next" Footer in `crates/nebulous_cli/src/commands/mod.rs`**:
   - In `handle_create_item` (`neb bug` / `neb feat`): when `--scope` is omitted, automatically infer the scope via `storage.infer_scope_from_context(cwd)` instead of dumping everything into `inbox` whenever the developer is inside or modifying a known crate/component.
   - Append a concise `"Next Suggested Action"` hint line to human-readable outputs of `init`, `bug`, `feat`, `start`, `close`, and `list` so the CLI continuously guides flow.
3. **Tests (`crates/nebulous_cli/tests/cli_integration.rs` & `src/lib.rs`)**:
   - Add integration tests verifying:
     - One-command `neb init` (scaffolds `pre_production`, `production`, `post_production`, auto-ingests workspace crates into scopes across all 3 stages, and registers project).
     - Predictive scope inference on `neb bug` / `neb feat` when `--scope` is omitted.
     - Zero-ID workflow: `neb start` -> `neb context` -> `neb move prod` -> `neb done -m "Implemented feature"`.
     - `neb next` JSON and human output.

[POST_GATE]
- Run `cargo fmt --all`
- Run `cargo fmt --all -- --check`
- Run `cargo clippy --workspace --all-targets -- -D warnings`
- Run `cargo test --workspace`

[HALT_INVARIANT]
- Halt immediately after all `[POST_GATE]` checks pass and report results. Do not proceed to Step 5 until instructed.
```

---

### Step 5: Retro Amber-on-Black TUI, Centered ASCII Nebula + `NEBULOUS` Header & Non-Clipping Scaling Locks (`nebulous_cli` & Docs)

**Recommended Slash Command:** `/boost`

```markdown
[STEP_ID]
STEP-05: Retro Amber-on-Black TUI, Centered ASCII Nebula + NEBULOUS Header & Non-Clipping Scaling Locks (`nebulous_cli` & Docs)

[SCOPE_LOCK]
Allowed files to modify or create:
- `Cargo.toml`
- `crates/nebulous_cli/Cargo.toml`
- `crates/nebulous_cli/src/tui/mod.rs` (new)
- `crates/nebulous_cli/src/tui/theme.rs` (new)
- `crates/nebulous_cli/src/tui/state.rs` (new)
- `crates/nebulous_cli/src/tui/render.rs` (new)
- `crates/nebulous_cli/src/args.rs`
- `crates/nebulous_cli/src/commands/mod.rs`
- `crates/nebulous_cli/src/lib.rs`
- `crates/nebulous_cli/tests/cli_integration.rs`
- `README.md`
- `docs/USAGE.md`
- `docs/ARCHITECTURE.md`

[PRE_GATE]
- Run `cargo test --workspace` and confirm all Step 1–4 tests pass.

[MUTATION_SPEC]
1. **Dependencies (`Cargo.toml` & `crates/nebulous_cli/Cargo.toml`)**:
   - Add `ratatui = "0.29"` and `crossterm = "0.28"` to `crates/nebulous_cli/Cargo.toml` (using the versions already cached in the local Cargo registry).
2. **Retro Amber-on-Black CRT Theme & Centered ASCII Nebula Header (`crates/nebulous_cli/src/tui/theme.rs`)**:
   - Define the strict retro Amber-on-Black phosphor palette:
     - `BG_BLACK`: `Color::Rgb(0, 0, 0)` (pure CRT obsidian background painted across the entire terminal buffer so host terminal themes never bleed through).
     - `PANEL_BG`: `Color::Rgb(12, 8, 0)` (subtle warm dark phosphor canvas for modals/active panels).
     - `AMBER_PRIMARY`: `Color::Rgb(255, 176, 0)` (`#FFB000` — classic P3 phosphor amber for primary text, active borders, and ASCII logo).
     - `AMBER_BRIGHT`: `Color::Rgb(255, 216, 102)` (`#FFD866` — high-intensity gold for focused items, active stage tabs, and `NEBULOUS` center glyphs).
     - `AMBER_DIM`: `Color::Rgb(153, 99, 0)` (`#996300` — muted amber for inactive borders, nebula outer dust rings, and secondary labels).
     - `AMBER_DARK`: `Color::Rgb(92, 58, 0)` (`#5C3A00` — deep amber for subtle grid dividers and outer starfield).
     - `AMBER_ALERT`: `Color::Rgb(255, 85, 0)` (`#FF5500` — high-contrast orange-amber for `critical`/`high` bugs, recurring regressions, and validation locks).
   - **Centered ASCII Art Nebula + `NEBULOUS` Banner (`select_nebula_banner(width: u16, height: u16)`)**:
     - Provide 2 aspect-locked, non-deforming ASCII art nebula banners with ASCII-art `NEBULOUS` embedded directly in the center of the swirling nebula cloud (`·`, `*`, `✦`, `░`, `▒`, `▓`, `(`, `)`):
       - **Full Banner (`5 lines × 76 cols`, used when `width >= 78` and `height >= 26`)**:
         Multi-line block-letter `N E B U L O U S` (`███╗   ██╗...` or box-drawing ASCII art) flanked and framed by symmetric swirling cosmic nebula dust/stars (`*  . ·  ░▒▓ (`, `) ▓▒░  · .  *`), styled with `AMBER_DIM` on the outer nebula cloud and bold `AMBER_BRIGHT` / `AMBER_PRIMARY` on the central `NEBULOUS` glyphs.
       - **Compact Banner (`3 lines × 60 cols`, used when `width < 78` or `height < 26`)**:
         3-line nebula swirl framing centered ASCII `╔═╗ N E B U L O U S ╔═╗` art so the logo never wraps, clips, or distorts at smaller terminal sizes.
     - Render the banner using horizontal centering padding (`Alignment::Center` with `Wrap` strictly disabled) inside a locked-height header block (`Constraint::Length(banner_height + 2)`).
3. **Scaling Locks, Viewport Guard & Non-Clipping Menu/Field Layout (`crates/nebulous_cli/src/tui/render.rs`)**:
   - **Minimum Terminal Lock Guard (`MIN_TERM_WIDTH = 64`, `MIN_TERM_HEIGHT = 20`)**:
     - If the terminal is resized below `64×20`, render a centered Amber-on-Black scaling lock screen displaying the compact Nebula logo, current vs. required dimensions (`Current: WxH | Minimum Locked: 64x20`), and active item status without clipping or panicking.
   - **Responsive Breakpoint Locks for Main Workspace**:
     - **Wide Mode (`width >= 110`)**: 3-pane horizontal layout with minimum width locks:
       - Left Scopes Menu: `Constraint::Min(22)` / `Constraint::Length(26)`
       - Center Items Table: `Constraint::Min(44)` (with column width locks: `ID` `Length(9)`, `Sev` `Length(6)`, `Status` `Length(11)`, `Title` `Min(18)`)
       - Right Details & Agent Context Inspector: `Constraint::Min(38)`
     - **Medium Mode (`64 <= width < 110`)**: 2-pane adaptive layout to prevent column/field crushing:
       - Top/Main row splits Left Scopes Menu (`Constraint::Length(22)`) and Center Items Table (`Constraint::Min(40)`).
       - Bottom Inspector Drawer (`Constraint::Min(7)`) displays the selected item's `desc.md` / Fix History / Agent Context with scroll support (`PgUp`/`PgDn` or `J`/`K`).
   - **Modal & Required Field Accessibility Locks (`render_create_modal`, `render_close_modal`, `render_move_modal`)**:
     - Modal rects use `locked_centered_rect(min_width, min_height, max_width, area)` which clamps modal dimensions to `min_width.min(area.width)` and `min_height.min(area.height)` instead of naive percentages that crush fields on small screens.
     - Required fields in `New Bug` / `New Feature` (`[1] Title *REQUIRED*`, `[2] Scope *REQUIRED*`, `[3] Stage *REQUIRED*`, `[4] Severity *REQUIRED*`, `[5] Description`) each have a dedicated locked row (`Constraint::Length(3)` per single-line field, `Constraint::Min(4)` for description).
     - If terminal height is tight, the modal automatically scrolls the focused field into the visible viewport window and renders horizontal cursor-window scrolling (`input_scroll_offset`) for long text inputs so characters are never clipped off-screen.
     - Attempting to submit (`Enter`) when a required field (`Title` or `Summary`) is empty triggers an inline `AMBER_ALERT` `[REQUIRED FIELD LOCK]` indicator on that field and keeps focus locked on the missing required field.
4. **TUI State & Predictive Controller (`crates/nebulous_cli/src/tui/state.rs` & `mod.rs`)**:
   - Implement `TuiApp` state machine backed by `TrackerStorage`:
     - **Header & Stage Bar**: Centered ASCII Nebula + `NEBULOUS` header, `[1] Pre-Production | [2] Production | [3] Post-Production` stage selector tabs, live timer badge (`⏱ <ID> <elapsed>m`), and `"WHAT'S NEXT"` predictive recommendation banner.
     - **Assistive Keyboard Hotkeys**:
       - `1` / `2` / `3`: Switch active stage (`pre_production` / `production` / `post_production`).
       - `Tab` / `BackTab` or `h` / `l`: Cycle pane focus (`Scopes` <-> `Items` <-> `Details`) or modal fields.
       - `j` / `k` or `Up` / `Down`: Navigate menus/items without clipping selection (auto-adjusting `ListState` / `TableState` offset).
       - `b` (New Bug) / `f` (New Feature): Opens predictive creation modal with `Scope` pre-filled via `infer_scope_from_context`, `Stage` pre-filled to active stage, and `Severity` pre-filled to `Medium`.
       - `s` (Start Work): Calls `start_work_item` on selected/recommended item, starts/switches timer, marks `in_progress`, and copies agent context.
       - `d` (Done / Close): Opens close modal pre-filled with `"Resolved <ID>: <title>"` and Git `HEAD` SHA.
       - `m` (Move Stage): Opens 1-key stage move modal (`1` Pre-Production, `2` Production, `3` Post-Production).
       - `c` (Copy Context), `i` (Ingest & Promote Workspace), `n` (Jump to "What's Next" item), `/` (Filter items), `q` / `Esc` (Quit / Close modal).
   - Provide `render_to_buffer(app: &mut TuiApp, width: u16, height: u16) -> ratatui::buffer::Buffer` and `render_to_string(app: &mut TuiApp, width: u16, height: u16) -> String` using `ratatui::backend::TestBackend`.
5. **Documentation & Headless Verification (`README.md`, `docs/`, `cli_integration.rs`)**:
   - Update `README.md`, `docs/USAGE.md`, and `docs/ARCHITECTURE.md` to document the Amber-on-Black TUI, 3-stage workflow, predictive assist, and CLI shortcuts.
   - Add headless `TestBackend` integration tests in `crates/nebulous_cli/tests/cli_integration.rs` verifying:
     1. Amber-on-Black color cells (`Color::Rgb(0, 0, 0)` background and `#FFB000`/`#FFD866` foregrounds) are applied across the rendered `Buffer`.
     2. The ASCII Nebula header and centered `NEBULOUS` art render completely unclipped across both wide (`120×36`) and compact (`80×24`) terminal dimensions.
     3. All required modal fields (`Title`, `Scope`, `Stage`, `Severity`) remain visible and functional at compact dimensions (`80×24`), and empty-title submission triggers the `[REQUIRED FIELD LOCK]` guard.
     4. Resizing below `64×20` engages the non-deforming minimum-size scaling lock view cleanly without panicking.

[POST_GATE]
- Run `cargo fmt --all`
- Run `cargo fmt --all -- --check`
- Run `cargo clippy --workspace --all-targets -- -D warnings`
- Run `cargo test --workspace`

[HALT_INVARIANT]
- Halt immediately after all `[POST_GATE]` checks pass and report final verification results.
```

---

### Step 6: Non-Blocking `TrackerWatcher` Worker, Item-Creation Settlement & Write-On-Change Index Reconciliation (`backend_core`)

**Recommended Slash Command:** `/goal`

```markdown
[STEP_ID]
STEP-06: Non-Blocking TrackerWatcher Worker, Item-Creation Settlement & Write-On-Change Index Reconciliation (`backend_core`)

[SCOPE_LOCK]
Allowed files to modify:
- `crates/backend_core/src/storage/reconcile.rs`
- `crates/backend_core/src/watcher/mod.rs`
- `crates/backend_core/tests/workflow_integration.rs`
Do not modify `crates/nebulous_cli` in this step.

[PRE_GATE]
- Inspect `crates/backend_core/src/watcher/mod.rs`, `crates/backend_core/src/storage/reconcile.rs`, and `crates/backend_core/tests/workflow_integration.rs`.

[MUTATION_SPEC]
1. **Write-On-Change Idempotency in `TrackerStorage::reconcile_indexes` (`crates/backend_core/src/storage/reconcile.rs`)**:
   - Before mutating `project`, clone `let original_project = project.clone();`.
   - Inside the phase loop, clone `let original_phase_manifest = phase_manifest.clone();` after reading `phase.toml`.
   - Inside the scope loop, clone `let original_scope_manifest = scope_manifest.clone();` after reading `scope.toml`.
   - Only call `self.write_scope_to_path(&scope_toml_path, &scope_manifest)?` when `scope_manifest != original_scope_manifest`.
   - Only call `self.write_phase_to_path(&phase_toml_path, &phase_manifest)?` when `phase_manifest != original_phase_manifest`.
   - Only call `self.write_project(&project)?` when `project != original_project`.
   - This guarantees that when `.tracker/` indexes are already synchronized with on-disk `items/*/item.toml` files, `reconcile_indexes()` performs zero disk writes (`atomic_write_str` / `sync_all()`), eliminating redundant `fsync` stalls and preventing infinite `inotify` self-trigger loops.
2. **Decouple `notify` Event Loop from Reconciliation & Settle In-Flight Item Writes (`crates/backend_core/src/watcher/mod.rs`)**:
   - In `TrackerWatcher::start_with_options`:
     - Do **not** run `storage.reconcile_indexes()` synchronously inside the `RecommendedWatcher::new(...)` callback. In `notify 6.1.1` (`src/inotify.rs` lines 284, 343, 364), `handle_event` is invoked on `EventKind::Create(CreateKind::Folder)` **before** `self.add_watch(path, true, false)` registers the recursive `inotify` watch on the newly created directory (`items/<ITEM_ID>`).
     - Instead, have the `RecommendedWatcher` callback immediately filter out non-mutating events (also excluding `EventKind::Modify(notify::event::ModifyKind::Metadata(_))` permission/timestamp touches from `atomic_write_bytes`) and `.tmp.` paths, and forward `relevant_paths` non-blockingly over an internal `mpsc::channel::<Vec<PathBuf>>` (`raw_tx`) to a dedicated background worker thread.
     - On the worker thread:
       1. Receive the initial `paths` batch from `raw_rx`, then coalesce any additional paths arriving within a `50ms` quiet/debounce window (`while let Ok(more) = raw_rx.recv_timeout(Duration::from_millis(50))`).
       2. If all coalesced paths are derived index files (`project.toml`, `phase.toml`, `scope.toml`) and an internal `auto_reconcile` completed within the last `1000ms`, discard the echo batch and continue.
       3. **In-Flight Item Directory Settlement**: If any coalesced path refers to or resides inside an item directory (`.../items/<ITEM_ID>`) that exists on disk, check whether `<item_dir>/item.toml` is missing or whether `<item_dir>` still contains an active `.tmp.` atomic write file. If so, poll up to `300ms` (in `15ms` intervals) until `<item_dir>/item.toml` is present and no `.tmp.` sibling file remains in `<item_dir>`, draining any additional `raw_rx` paths into the batch.
       4. Run `storage.reconcile_indexes()` when `auto_reconcile` is `true`, record the completion `Instant`, drain any immediate index-only echo events from `raw_rx`, and send the `TrackerWatchEvent` to `tx`.
3. **Harden `test_watcher_detects_external_changes_and_auto_reconciles_indexes` (`crates/backend_core/tests/workflow_integration.rs`)**:
   - Wait for a `TrackerWatchEvent` (within the 5-second timeout) whose `reconcile_report` reflects `total_items >= 1` (or poll subsequent events until the reconciled state is reached), drain trailing events, assert `scope.items.bugs[0].id == "BUG-0007"` and `phase.summary.total_items == 1`, and verify no self-trigger loop events occur over the subsequent `250ms` window.

[POST_GATE]
- Run `cargo fmt --all`
- Run `cargo fmt --all -- --check`
- Run `OPENSSL_DIR=/tmp/openssl-dev/usr cargo clippy --workspace --all-targets -- -D warnings` (or without `OPENSSL_DIR` if system `libssl-dev` is installed)
- Run `OPENSSL_DIR=/tmp/openssl-dev/usr cargo test --workspace`

[HALT_INVARIANT]
- Halt immediately after all `[POST_GATE]` checks pass and report results. Do not proceed to Step 7 until instructed.
```

---

### Step 7: Update GitHub Actions CI Workflow for CLI/TUI Architecture (`.github/workflows/ci.yml`)

**Recommended Slash Command:** `/goal`

```markdown
[STEP_ID]
STEP-07: Update GitHub Actions CI Workflow for CLI/TUI Architecture (`.github/workflows/ci.yml`)

[SCOPE_LOCK]
Allowed files to modify:
- `.github/workflows/ci.yml`
Do not modify Rust source files in this step.

[PRE_GATE]
- Inspect `.github/workflows/ci.yml` and confirm `crates/cosmic_interface` was archived in Step 1.

[MUTATION_SPEC]
1. **Update `.github/workflows/ci.yml`**:
   - Update the job name from `Format, Clippy, Test & Headless Self-Test (Linux x86_64)` to `Format, Clippy, Test & CLI Smoke Check (Linux x86_64)`.
   - Prune obsolete Wayland/X11/Mesa GUI packages from the `Install Linux system dependencies` step, keeping only the required build and Secret Service / OpenSSL dependencies (`build-essential`, `pkg-config`, `cmake`, `clang`, `libssl-dev`, `libdbus-1-dev`).
   - Replace the obsolete `Run headless COSMIC GUI self-test` (`cargo run -p cosmic_interface -- --self-test`) step at line 60–61 (which fails because `cosmic_interface` was archived in Step 1) with a CLI/TUI binary verification step (`cargo run -p nebulous_cli -- --help`).

[POST_GATE]
- Run `cargo fmt --all -- --check`
- Run `OPENSSL_DIR=/tmp/openssl-dev/usr cargo clippy --workspace --all-targets -- -D warnings`
- Run `OPENSSL_DIR=/tmp/openssl-dev/usr cargo test --workspace`
- Run `OPENSSL_DIR=/tmp/openssl-dev/usr cargo run -p nebulous_cli -- --help`

[HALT_INVARIANT]
- Halt immediately after all `[POST_GATE]` checks pass and report final verification results.
```

---
---

# Phase 5–7: Supply-Chain Remediation, Hardening, Correctness & Polish

Source: full-project review of `97e6c36` (2026-11-05). Gates at review time were green: `fmt`, `clippy -D warnings`, and `cargo test --workspace` (56 tests). Feature breadth is ahead of hardening. Risk is concentrated in four areas:
- git operations exposed to agents through MCP;
- no cross-process locking;
- symlink/path trust in git-synced `.tracker/` content;
- vulnerable dependencies and no schema versioning.

## Review Findings (Root Causes)

### A. Supply chain (RustSec/GHSA via OSV.dev + NIST NVD, 397 registry crates)

| Crate (locked) | Advisories | Worst | Fixed in |
|---|---|---|---|
| `git2 0.19.0` | RUSTSEC-2026-0008 / GHSA-j39j-6gw9-jw6h, RUSTSEC-2026-0183, RUSTSEC-2026-0184 | UB | `0.21.0` |
| `libgit2-sys 0.17.0+1.8.1` | CVE-2026-53583 (TLS IP-SAN check inverted), CVE-2026-53584 (submodule traversal), CVE-2026-53585 (delta alloc), CVE-2026-53586 (cred sent after offsite redirect), CVE-2026-53587 (pkt-line over-read) | CVSS 7.5 | `0.18.8+1.9.7` |
| `russh 0.64.1` | CVE-2026-116794 (High), CVE-2026-116773, CVE-2026-116774, CVE-2026-116696 | High | `0.65` (remove) |
| `pageant 0.2.4` (via russh) | CVE-2026-116775 | Low | remove with russh |
| `lru 0.12.5` (via ratatui 0.29) | RUSTSEC-2026-0002, RUSTSEC-2026-0253 (UAF) | UB | `0.18.2` → ratatui `0.30.2` |
| `paste 1.0.15` (via ratatui 0.29) | RUSTSEC-2024-0436 (unmaintained) | info | dropped by ratatui `0.30` |

- **Not affected:**
  - CVE-2026-5917: libssh2 isn't compiled.
  - AWS-LC CVE-2026-3336/3337/3338/4428: bundled 5.7.0, and those paths are unused.
  - Host `libssl3t64 3.0.13-0ubuntu3.16`: distro-maintained.
- **Functional defect:** `git2` is built with `features = ["https"]` only, so there is no SSH transport. Every SSH-key auth path in `git_sync` is unreachable for `git@`/`ssh://` remotes.

### B. Security
1. **Workspace commits stage the whole repo.** `git_commit_documented` with no `paths` stages `add_all(["*"])`, which includes untracked files (`git_sync/mod.rs:1448-1451`); reached by `neb done -c` and MCP. `paths` are only checked for `..`, so globs pass.
2. **Commits reset the user's index.** `read_tree(HEAD)` runs before and after the commit (`git_sync/mod.rs:519-535`, `:1571-1580`), discarding staged work.
3. **`git_pull` fast-forward moves the wrong branch.** It force-sets `refs/heads/<configured>` plus `set_head` regardless of HEAD (`:959-967`). Push targets the configured branch.
4. **`git_pull` merge forces checkout** (`CheckoutBuilder::force()`, `:1016`), overwriting uncommitted `.tracker/` edits.
5. **Credential handling.**
   - A file-path `auth_override` is read and imported into the vault (`:1137-1152`).
   - The credential account is keyed from git-synced `project.toml`.
   - The plaintext private key is exported to `~/.config/nebulous/ssh_cache` and never removed (`:672-690`).
   - A PAT is offered over `http://`; `--pat` goes on argv; `git_status` returns `remote_url` verbatim.
6. **No symlink containment.** `validate_identifier` / `resolve_safe_relative_path` (`storage/mod.rs:952-1016`) are lexical only, and the canonical-prefix check exists only in ingestion.
7. **MCP framing has no bounds:** `vec![0u8; content_len]` with no cap (`mcp/mod.rs:2293`) and an unbounded `read_line`.
8. **Vault and writes.**
   - `vault.key` is a plaintext seed next to `vault.enc`, and creating it races between processes.
   - `set_readonly(false)` makes files world-writable (`storage/atomic.rs:39-46`).
   - No parent-dir fsync after rename.
   - `validate_identifier` allows `:` (a Windows path escape).
9. **Agent context is unbounded.** It embeds `desc.md` and verification logs verbatim (stored prompt injection). Destructive MCP tools have no `dry_run` or confirm.

### C. Functionality / correctness
1. **No cross-process lock.** `allocate_next_id` (`storage/id.rs:252-278`) and all manifest and timer read-modify-writes can race, giving duplicate IDs and overwritten items.
2. **`release.yml` is broken.** It runs and copies `target/release/nebulous`, which doesn't exist (`:64,74`).
3. **`neb done -c` closes the item and mutates its fix entry before committing.** A policy failure leaves the item closed.
4. **Zero-ID `start` vs `next`.** `start` uses `compute_whats_next(None)` without the CWD (`lifecycle.rs:214`). The stagnant and recurring lists ignore the active stage, and the timer is switched silently.
5. **Watcher gaps.**
   - It drops `EventKind::Other` (rescan/overflow).
   - The 1s echo suppression hides real edits.
   - `reconcile_indexes().ok()` swallows errors, and one bad TOML aborts reconcile.
   - The thread is detached.
   - It isn't wired into the TUI or CLI.
6. **Timer.**
   - A crash between log and clear double-logs.
   - A corrupt or orphaned `.timer.toml` blocks most commands.
   - There's no elapsed cap.
7. **No `schema_version` and no unknown-field preservation** in any tracker TOML.
8. **CLI quirks.**
   - `project add` scaffolds the legacy `01_core_engine`.
   - `project add --use` can't be disabled.
   - `project switch` is MCP-only.
   - `init` swallows ingest errors.
   - The duplicate `Commands::Mcp` arm is dead.

### D. Efficiency / usability
- **Repeated TOML parsing.** `load_index` / `locate_item` / `phase_dir` re-parse TOML per call: about 10 reads per `create_item`, and about 4 index rebuilds per MCP `get_active_task`. Assist runs a full git status, including untracked files, per call.
- **TUI.**
  - `refresh()` re-reads every `item.toml` plus git status on each filter keystroke.
  - No panic hook.
  - The timer display is frozen.
  - Clipboard failures are reported as success.
  - arboard is missing `wayland-data-control`.
  - Mistyped scopes are auto-created.
- **Docs drift and other leftovers.**
  - The README claims zero-ID fallback for done/show/context/move, and claims watcher auto-reconcile.
  - ARCHITECTURE says `log_time`; the function is `log_effort`.
  - Windows/macOS builds are claimed.
  - Dead code: `fetch_repo`, `push_repo`, `pull_repo`, `export_bounded_prompt`, the `DocumentedCommit*` aliases.
  - `install.sh --system` builds as root.

## Execution Strategy
- Run each phase in a **fresh session** seeded from this file.
- Phase 5 steps run **sequentially**, because they overlap on `Cargo.toml`, `git_sync`, `storage` and `mcp`.
- STEP-09 (`release.yml`) and STEP-22 (docs) touch no shared Rust files and may run in parallel sessions.

**Standard Rust `[POST_GATE]`** (referenced below as `RUST_GATE`):
- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo deny check advisories bans` (from STEP-08 onward)

**Permanent constraint:** zero open RustSec/GHSA/NVD advisories against locked crates. Every step that touches `Cargo.toml`/`Cargo.lock` must keep `cargo deny check advisories` at exit 0.

---

## Phase 5 — Supply Chain & Hardening

### Step 8: Supply-Chain Remediation & Advisory Gate (`Cargo.toml`, `crypto/keys.rs`, `tui/`, CI)

**Recommended Slash Command:** `/goal`

```markdown
[STEP_ID]
STEP-08: Supply-Chain Remediation (git2/libgit2, russh removal, ratatui 0.30) & cargo-deny Advisory Gate

[SCOPE_LOCK]
Allowed files to modify:
- `Cargo.toml`, `Cargo.lock`, `crates/backend_core/Cargo.toml`, `crates/nebulous_cli/Cargo.toml`
- `crates/backend_core/src/crypto/keys.rs`, `crates/backend_core/src/git_sync/mod.rs` (API migration only)
- `crates/nebulous_cli/src/tui/*.rs` (ratatui/crossterm API migration only)
- `deny.toml` (new), `.github/workflows/ci.yml`
No behavioural changes beyond dependency/API migration.

[PRE_GATE]
- `git status` clean; confirm locked versions: git2 0.19.0, libgit2-sys 0.17.0+1.8.1, russh 0.64.1, ratatui 0.29.0, lru 0.12.5, paste 1.0.15.
- Install the gate tool locally if absent: `cargo install --locked cargo-deny`.

[MUTATION_SPEC]
1. git2: bump to `0.21` in `backend_core` (features `["https", "ssh"]` — SSH transport is currently missing,
   making all SSH auth in `git_sync` dead) and in `nebulous_cli` dev-deps; `cargo update -p libgit2-sys`
   so the lock resolves `>= 0.18.8+1.9.7` (fixes CVE-2026-53583..53587 and CVE-2026-5917 once libssh2 is linked).
   Migrate any git2 0.19→0.21 API breaks in `git_sync/mod.rs`.
2. Remove `russh` (and transitive `pageant`, `aws-lc-rs`, `aws-lc-sys`): replace its use at
   `crypto/keys.rs:152` (encrypted-key import) with `ssh_key::PrivateKey::from_openssh` + `.decrypt(passphrase)`
   and delete the redundant russh validation pass at `crypto/keys.rs:321`. Keep existing key-import tests green.
3. Move `crossterm`/`ratatui` in `nebulous_cli` to `.workspace = true`; bump workspace to `ratatui = "0.30.2"`,
   `crossterm = "0.29"`; migrate TUI API breaks (resolves lru RUSTSEC-2026-0002/0253, drops paste RUSTSEC-2024-0436).
4. Add `deny.toml`: `[advisories]` deny vulnerabilities + unmaintained (workspace scope), yanked = deny;
   `[bans] multiple-versions = "warn"`; `[licenses]` allow the current MIT/Apache-2.0/BSD/ISC/Unicode/Zlib set.
5. CI: add a step using pinned `EmbarkStudios/cargo-deny-action@v2` running `check advisories bans`.

[POST_GATE]
- RUST_GATE
- `cargo tree -i russh` and `cargo tree -i lru@0.12.5` and `cargo tree -i paste` report no match.
- `cargo tree -i libgit2-sys` shows `+1.9.7` (or newer); `cargo tree -i libssh2-sys` present.
- Independent cross-check: OSV batch query (`https://api.osv.dev/v1/querybatch`) of every registry package in `Cargo.lock` returns 0 vulns.

[HALT_INVARIANT]
- Halt after POST_GATE and report the before/after advisory table. Do not proceed to Step 9.
```

### Step 9: Fix Release Workflow (`.github/workflows/release.yml`)

```markdown
[STEP_ID]
STEP-09: Release Workflow — Ship `neb` Only

[SCOPE_LOCK]
- `.github/workflows/release.yml` only.

[MUTATION_SPEC]
1. Build with `cargo build -p nebulous_cli --release --locked`.
2. Replace `./target/release/nebulous --self-test` with `./target/release/neb --help`; remove `cp target/release/nebulous`.
3. Remove obsolete GUI apt packages (keep build-essential, pkg-config, cmake, clang, libssl-dev, libdbus-1-dev).
4. Scope `permissions: contents: write` to the publish job only; default workflow permissions to `contents: read`.
5. Add `cargo deny check advisories` before build.

[POST_GATE]
- YAML parses (`python3 -c "import yaml,sys;yaml.safe_load(open(sys.argv[1]))" .github/workflows/release.yml`).
- `cargo build -p nebulous_cli --release --locked && ./target/release/neb --help`.

[HALT_INVARIANT]
- Halt and report.
```

### Step 10: Git Operation Safety (`git_sync`, `commands/mod.rs`, `mcp`)

```markdown
[STEP_ID]
STEP-10: Git Safety — Scoped Staging, Isolated Index, HEAD-Branch Pull/Push, Safe Checkout, Atomic `done -c`

[SCOPE_LOCK]
- `crates/backend_core/src/git_sync/mod.rs`, `crates/backend_core/src/mcp/mod.rs` (tool schemas/defaults),
  `crates/nebulous_cli/src/commands/mod.rs`, `crates/nebulous_cli/src/args.rs`, related tests.

[MUTATION_SPEC]
1. `DocumentedCommitInput.tracker_only` defaults to `true` (CLI + MCP schema). Workspace commits require explicit
   `paths`; reject any path containing glob metacharacters (`* ? [ ] { }`) or that resolves outside the workdir;
   never stage untracked files unless each path is listed explicitly.
2. Build commits from a private in-memory index (`Index::new()` seeded from HEAD tree, add blobs, `write_tree_to(&repo)`);
   never call `repo.index()?.write()` — the user's staging area must be byte-identical before/after.
3. `git_pull`/`git_push`: operate on HEAD's current branch; if it differs from `[git] branch`, return an error
   naming both. Fast-forward only updates HEAD's ref. Replace `CheckoutBuilder::force()` with `.safe()`;
   on conflict, abort and report without writing. Fail closed if the dirty-status check errors.
4. `neb done -c`: run policy validation and commit first; only on success close the item and append the fix
   (or snapshot and roll back item.toml/desc.md on failure).
5. Pathspecs must be computed relative to the repo workdir (support project roots nested in a repo).

[POST_GATE]
- RUST_GATE
- New tests: untracked `secret.env` not committed by `done -c`; pre-staged hunk survives `git_commit_documented`;
  pull on non-configured branch errors and leaves refs unchanged; dirty `.tracker` edit survives merge attempt;
  policy-rejected `done -c` leaves item open with no fix entry; nested-root commit stages `.tracker`.

[HALT_INVARIANT]
- Halt and report.
```

### Step 11: Credential Hygiene (`git_sync`, `crypto`, CLI args)

```markdown
[STEP_ID]
STEP-11: Credential Hygiene — No Key Files on Disk, Local-Only Account Binding, HTTPS-Only PAT

[SCOPE_LOCK]
- `crates/backend_core/src/git_sync/mod.rs`, `crates/backend_core/src/crypto/{mod.rs,keys.rs}`,
  `crates/backend_core/src/models/registry.rs`, `crates/backend_core/src/mcp/mod.rs`, `crates/nebulous_cli/src/args.rs`, tests.

[MUTATION_SPEC]
1. Remove file-path interpretation of `auth_override`; accept only a vault account name defined in the local registry.
2. Bind credential accounts to the local project registry entry (not `project.toml` `name`/`ssh_key_account`, which are git-synced).
3. Use `Cred::ssh_key_from_memory` exclusively; delete the `ssh_cache` export path and purge any existing `ssh_cache/` on startup.
4. Offer PAT credentials only when the remote URL scheme is `https`.
5. `--pat` reads from stdin (`--pat-stdin`) or `NEB_GIT_PAT`; remove the argv value form.
6. Redact userinfo in any `remote_url` returned by `git_status`/MCP.
7. `store_pat_in_keyring` returns an error when both keyring and vault writes fail.

[POST_GATE]
- RUST_GATE
- Tests: `ssh_cache` never created; `http://` remote never receives PAT callback; `git_status` output contains no userinfo.

[HALT_INVARIANT]
- Halt and report.
```

### Step 12: Path Containment (`storage`, `ingestion`)

```markdown
[STEP_ID]
STEP-12: Unified Path Containment & Identifier Allow-List

[SCOPE_LOCK]
- `crates/backend_core/src/storage/{mod.rs,atomic.rs}`, `crates/backend_core/src/ingestion/mod.rs`, tests.

[MUTATION_SPEC]
1. `validate_identifier`: allow-list `^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$`; reject reserved Windows names.
2. Add `contained_path(base, rel) -> Result<PathBuf>`: canonicalize the deepest existing ancestor and require
   `starts_with(canonical base)`; reject symlinked final components on read and write (`symlink_metadata`).
   Route every `.tracker/` read/write and `component_manifest_path`/`write_component_manifest` through it.
3. `atomic.rs`: preserve original mode instead of `set_readonly(false)`; fsync the parent dir after rename on Unix;
   remove the non-atomic truncate fallback; remove the delete-then-rename path on Unix.

[POST_GATE]
- RUST_GATE
- Tests: symlinked `items/` and symlinked `desc.md` rejected on read and write; `C:x` / `CON` / `a.b` IDs rejected;
  file mode preserved across atomic write.

[HALT_INVARIANT]
- Halt and report.
```

### Step 13: Cross-Process Locking (re-split 2026-11-09 into STEP 13A–13C)

The original STEP 13 touched 7 files in one step. Re-split per Open Question 1. Its item 3 (cross-process lock on `vault.key` creation) is **deferred**: `crypto/mod.rs:50` already serializes vault access in-process, and no queued step adds a second vault-writing process. Re-add it as its own step if one appears.

### STEP 13A: Advisory Write Lock (`with_write_lock`)

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 13A: TrackerStorage::with_write_lock: reentrant, exclusive fd-lock on .tracker/.lock with a timeout

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/Cargo.toml
  - crates/backend_core/src/storage/mod.rs
  - crates/backend_core/src/error.rs
  (Cargo.lock changes only as cargo's own result of the new dependency.)
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs:143-175 (TrackerStorage, new, from_tracker_dir), :236-360 (init), :394-404 (write_tracker_file, create_tracker_dir_all)
  - crates/backend_core/src/error.rs:9-90 (TrackerError variants)
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §1.3 (neb release reuses this lock; a timeout maps to exit 7). Read only; never edit anything under /srv/devshare/NovOS.
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No caller is wrapped in this step (13B and 13C do that).

[PRE_GATE]
! grep -q 'fn with_write_lock' crates/backend_core/src/storage/mod.rs

[MUTATION_SPEC]
1. Cargo.toml: add `fd-lock = "4"` to [dependencies], in alphabetical order.
2. error.rs: new variant `#[error("timed out after {waited_ms} ms waiting for tracker lock {path}")] LockTimeout { path: PathBuf, waited_ms: u64 }` (use `path.display()` in the message the same way the `Io` variant does).
3. storage/mod.rs:
   a. `pub const LOCK_FILE_NAME: &str = ".lock";` and `pub const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(30);`
   b. `pub fn lock_path(&self) -> PathBuf` returns `self.tracker_dir.join(LOCK_FILE_NAME)`.
   c. `pub fn with_write_lock<T>(&self, f: impl FnOnce(&Self) -> Result<T>) -> Result<T>` calls `with_write_lock_timeout(DEFAULT_LOCK_TIMEOUT, f)`.
   d. `pub fn with_write_lock_timeout<T>(&self, timeout: Duration, f: impl FnOnce(&Self) -> Result<T>) -> Result<T>`:
      - Reentrancy: a `thread_local!` `RefCell<HashSet<PathBuf>>` holds the lock paths this thread already holds. If `lock_path()` is in the set, run `f(self)` directly and return its result.
      - Otherwise open the lock file with `OpenOptions::new().create(true).truncate(false).write(true)` and wrap it in `fd_lock::RwLock`. Loop on `try_write()`: on `ErrorKind::WouldBlock`, sleep 50 ms and retry until `timeout` has elapsed, then return `Err(TrackerError::LockTimeout { .. })`. Any other io error becomes `TrackerError::Io { path, source }`.
      - While holding the lock: insert the path into the set, run `f(self)`, remove the path. Removal must also happen on error and on panic, so use a drop-guard struct. The fd-lock guard drops after it, which releases the lock.
      - Ensure `.tracker/.gitignore` lists `.lock`: if the file is missing, create it with `.lock\n`; if it exists without a line equal to `.lock`, append one. Do this inside the lock, through `write_tracker_file`.
   e. init (:236): after the tracker directory exists, create `.tracker/.gitignore` containing `.lock\n` if it does not exist.
4. Inline `#[cfg(test)] mod lock_tests` in storage/mod.rs. Build each tracker with `TrackerStorage::init_standard(tempdir, "t")`.
   - After one `with_write_lock` call, `lock_path()` exists and `.tracker/.gitignore` contains a `.lock` line.
   - A nested `with_write_lock` inside `with_write_lock` on the same storage returns Ok (no deadlock).
   - Thread A holds the lock (it waits on a channel inside the closure). Thread B builds its own `TrackerStorage::new(root)` and calls `with_write_lock_timeout(200 ms)`, which returns `LockTimeout`. Then A is released.
   - The closure's `Err` comes back unchanged, and a following call succeeds (the lock was released).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans licenses && grep -q 'pub fn with_write_lock' crates/backend_core/src/storage/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 13B: Lock Every Workflow Mutation

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 13B: Wrap every lifecycle and timer mutation in with_write_lock; a new item dir fails on AlreadyExists

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/workflow/lifecycle.rs
  - crates/backend_core/src/workflow/timer.rs
  - crates/backend_core/src/storage/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs `with_write_lock` (STEP 13A), :394-410 (guard_tracker_path, create_tracker_dir_all), :480-484 (item_dir)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No pub fn signature changes. No behaviour change other than taking the lock and the AlreadyExists check.

[PRE_GATE]
grep -q 'pub fn with_write_lock' crates/backend_core/src/storage/mod.rs && ! grep -q 'with_write_lock' crates/backend_core/src/workflow/lifecycle.rs

[MUTATION_SPEC]
1. lifecycle.rs: wrap the whole existing body of each of these 13 pub fns as `self.with_write_lock(|_| { <existing body> })`. The body keeps using `self`; do not move bodies into new functions. The fns: create_item (:102), start_work_item (:206), update_item_status (:252), reopen_item (:315), record_regression (:369), close_item (:415), update_item_metadata (:476), update_item_description (:512), create_phase (:527), set_active_phase (:621), create_scope (:637), ensure_scope_across_phases (:704), move_item_to_phase (:772). create_bug and create_feature only delegate to create_item, so leave them unwrapped. The lock is reentrant, so nested calls are fine.
2. timer.rs: same treatment for start_timer_at (:37), switch_or_start_timer (:69), stop_timer_at (:111) and log_effort (:165). start_timer and stop_timer only delegate; timer_status and timer_status_at are read-only. Leave all four unchanged.
3. storage/mod.rs: `pub(crate) fn create_tracker_dir_new(&self, path: &Path) -> Result<()>`: run `guard_tracker_path`, `create_dir_all` the parent, then `fs::create_dir(path)`. Map `ErrorKind::AlreadyExists` to `TrackerError::AlreadyExists { kind: "Item", id: <final path component> }` and other errors to `TrackerError::Io`.
4. lifecycle.rs create_item: immediately before `self.write_item_detail(&phase_id, &scope_id, &detail)?;`, add `self.create_tracker_dir_new(&self.item_dir(&phase_id, &scope_id, &item_id)?)?;`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && test "$(grep -c 'with_write_lock' crates/backend_core/src/workflow/lifecycle.rs)" -ge 13 && test "$(grep -c 'with_write_lock' crates/backend_core/src/workflow/timer.rs)" -ge 4

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 13C: Lock Reconcile + ID Allocation; Concurrency Test

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 13C: Lock reconcile_indexes and allocate_next_id; two-process-style concurrency test

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/reconcile.rs
  - crates/backend_core/src/storage/id.rs
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/workflow/lifecycle.rs:162-200 (create_bug signature)
  - existing create_bug calls in crates/backend_core/tests/storage_integration.rs (copy their argument style)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.

[PRE_GATE]
grep -q 'with_write_lock' crates/backend_core/src/workflow/lifecycle.rs

[MUTATION_SPEC]
1. reconcile.rs reconcile_indexes (:32): wrap the body as `self.with_write_lock(|_| { .. })`.
2. id.rs allocate_next_id (:69): same. It already runs under create_item's lock (reentrant); standalone callers are now locked too. scan_max_counters stays unwrapped (read-only).
3. storage_integration.rs: new test `concurrent_creators_get_unique_ids`. Run `init_standard` in a tempdir, then spawn 2 threads. Each thread builds its own `TrackerStorage::new(root)` and creates 50 bugs (titles `a0..a49` / `b0..b49`). Join both, then assert 100 distinct ids and 100 item directories in total under the inbox scope's items dir of the active phase.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && cargo test -p backend_core --test storage_integration concurrent_creators_get_unique_ids

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### Step 14: MCP Server Robustness (`mcp`, `agent_context`)

**Superseded 2026-11-15:** re-split into STEP 14A–14I (see "STEP 14 Re-split" before Phase 8). Kept for reference.

```markdown
[STEP_ID]
STEP-14: MCP Framing Bounds, JSON-RPC Compliance, Destructive-Tool Guards, Bounded Agent Context

[SCOPE_LOCK]
- `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/src/agent_context/mod.rs`, CLI MCP tests.

[MUTATION_SPEC]
1. Cap `Content-Length` and line reads at 8 MiB; reply `-32600` and continue on overflow. Parse headers in any order.
2. Never respond to messages without `id`; keep the request id on error responses where readable; require `jsonrpc == "2.0"`.
3. Negotiate `protocolVersion` against a supported list; respond with the server's chosen version.
4. Nest tool payloads under `structuredContent` instead of flattening into the result object.
5. Add `dry_run` (default `true`) to `git_push`, `git_pull`, `git_sync`, `rollback_to_pinned_release`,
   `ingest_project_directory` (when writing), `switch_project`; `list_projects` must be read-only.
6. Agent context: cap each embedded field (e.g. desc 16 KiB, logs 4 KiB), fence untrusted text in labelled blocks,
   strip control characters.
7. Remove the duplicate `is_known_tool` list (derive from the dispatch table).

[POST_GATE]
- RUST_GATE
- Tests: 1 GiB `Content-Length` returns error without allocation; notification without id yields no output;
  destructive tools default to dry-run.

[HALT_INVARIANT]
- Halt and report.
```

---

## Phase 6 — Correctness & Performance

### Step 15: Schema Versioning & Forward Compatibility (re-split 2026-11-09 into STEP 15A–15D)

Redesigned while re-splitting. The original put `schema_version` and a `#[serde(flatten)] extra` field on all four manifest structs, which breaks about 24 struct-literal sites in 6 files. Instead, versioning and unknown-key preservation live in the storage read/write layer (read the on-disk table, merge its unknown keys into the new one, write). The structs don't change. This depends on STEP 13: read-merge-write is only safe under the lock. Purpose (Open Question 3): a neb binary that predates STEP 23F refuses a task item with a clear UnsupportedSchema error, instead of failing to parse or dropping fields.

### STEP 15A: `storage/compat.rs` (pure)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 15A: Pure schema helpers: read/check schema_version, recursive unknown-key merge, stamp, migrate hook

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/compat.rs (NEW)
  - crates/backend_core/src/storage/mod.rs (only the `pub mod compat;` line)
  - crates/backend_core/src/error.rs
READ_ONLY_REFS:
  - crates/backend_core/src/error.rs:9-90
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No read/write path uses these helpers yet (that's 15B).

[PRE_GATE]
grep -q 'pub fn with_write_lock' crates/backend_core/src/storage/mod.rs && test ! -e crates/backend_core/src/storage/compat.rs

[MUTATION_SPEC]
1. error.rs: `#[error("{path} has schema_version {found}; this neb supports up to {supported}. Upgrade neb.")] UnsupportedSchema { path: PathBuf, found: u32, supported: u32 }` (use path.display() like the Io variant).
2. compat.rs, all using `toml::Table`:
   - `pub const SCHEMA_KEY: &str = "schema_version";`
   - `pub fn read_schema_version(raw: &Table) -> u32`: missing key → 1; an integer ≥ 1 → that value; anything else → 1.
   - `pub fn check_readable(raw: &Table, path: &Path, supported: u32) -> Result<u32>`: found > supported → `Err(UnsupportedSchema)`; else `Ok(found)`.
   - `pub fn merge_unknown(new: &mut Table, old: &Table)`: for each `(k, v)` in `old`: if `new` lacks `k`, insert a clone; if both values are tables, recurse; otherwise keep `new`'s value. Arrays are never merged element-wise (new wins); state this limitation in the doc comment.
   - `pub fn stamp(table: &mut Table, version: u32, old_had_key: bool)`: set `schema_version = version` when `version > 1 || old_had_key`; otherwise remove the key, so v1 files stay as they are today.
   - `pub fn migrate(table: Table, from: u32, to: u32) -> Result<Table>`: returns `table` unchanged when `from >= to`. There are no migrations yet; the doc says new ones go here as `match (from, to)` arms.
3. Inline tests: unknown top-level key kept; unknown key inside a nested table kept; new value wins on conflict; old array replaced by new array; check_readable(found 3, supported 2) → UnsupportedSchema; missing key reads as 1; stamp(1, false) leaves no key; stamp(2, false) writes 2.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn merge_unknown' crates/backend_core/src/storage/compat.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 15B: Versioned Reads/Writes for All Four Tiers

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 15B: Route project/phase/scope/item reads and writes through compat; items carry a per-type minimum schema

[SCOPE_LOCK]
ALLOWED_FILES:
  - Cargo.toml (workspace root; the `toml` line only)
  - crates/backend_core/src/storage/mod.rs
  - crates/backend_core/src/models/item.rs
  - crates/backend_core/tests/storage_integration.rs
  (Cargo.lock changes only as cargo's own result of the feature change.)
READ_ONLY_REFS:
  - crates/backend_core/src/storage/compat.rs (STEP 15A)
  - crates/backend_core/src/storage/mod.rs:673-800 (read_project, write_project, read_phase(_from_path), write_phase_to_path, read_scope(_from_path), write_scope_to_path, read_item_manifest, write_item_manifest), attach_toml_path
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. The models' `from_toml_str` / `to_toml_string` stay as they are (other callers use them). No manifest struct gains a field.

[PRE_GATE]
grep -q 'pub fn merge_unknown' crates/backend_core/src/storage/compat.rs

[MUTATION_SPEC]
0. Workspace Cargo.toml: change `toml = "0.8"` to `toml = { version = "0.8", features = ["preserve_order"] }`. Why: without it `toml::Table` is a BTreeMap, so routing writes through a Table (item 2) would sort every key alphabetically and rewrite the layout of every existing tracker file. With it, `to_string_pretty(&Table::try_from(&m)?)` is byte-identical to `to_string_pretty(&m)` (verified 2026-11-09 against toml 0.8.23), and merged unknown keys go at the end of their table.
1. item.rs: `pub const MAX_ITEM_SCHEMA_VERSION: u32 = 1;` and `impl ItemType { pub fn min_schema_version(self) -> u32 { match self { Self::Bug | Self::Feature => 1 } } }`. The match is exhaustive with no `_` arm, so any new variant has to pick its version. Doc: "STEP 23F sets Task => 2 and raises MAX_ITEM_SCHEMA_VERSION to 2."
2. storage/mod.rs private helpers:
   - `fn read_versioned<T: DeserializeOwned>(&self, path: &Path) -> Result<T>`: `read_tracker_file`, parse into `toml::Table`, `compat::check_readable(&t, path, MAX_ITEM_SCHEMA_VERSION)?`, remove `SCHEMA_KEY`, then `T::deserialize(toml::Value::Table(t))`. Map parse errors with the existing `attach_toml_path`.
   - `fn write_versioned<T: Serialize>(&self, path: &Path, value: &T, version: u32) -> Result<()>`: `toml::Table::try_from(value)?`. If `path` is a file: parse the old table, `check_readable` it (refusing to overwrite a newer file), note whether it had `SCHEMA_KEY`, and `merge_unknown(&mut new, &old)`. Then `stamp(&mut new, version, old_had_key)`, `toml::to_string_pretty(&new)?` and `write_tracker_file`.
3. Route the existing fns through these helpers, keeping any post-processing they do after parsing:
   - reads: read_project, read_phase_from_path, read_scope_from_path, read_item_manifest;
   - writes: write_project, write_phase_to_path, write_scope_to_path (version 1) and write_item_manifest (version `manifest.item.item_type.min_schema_version()`).
4. storage_integration.rs tests:
   - Hand-add `future_flag = true` at the top level and `future_field = "x"` under `[item]` to a bug's item.toml. Both survive `update_item_status` (assert on the file text).
   - An unknown top-level key in project.toml survives `set_active_phase`.
   - An item.toml with `schema_version = 9` makes `read_item_manifest` return UnsupportedSchema; `write_item_manifest` over it returns UnsupportedSchema and the file is byte-identical afterwards.
   - A freshly created bug's item.toml has no `schema_version` line.
   - Layout unchanged: after `init_standard` and creating one bug, the text of `project.toml` equals `read_project()?.to_toml_string()?`, and the text of the bug's item.toml equals `read_item_manifest(..)?.to_toml_string()?` (the models' serializers, which this step does not touch, define the pre-15B layout).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'fn write_versioned' crates/backend_core/src/storage/mod.rs && grep -q 'preserve_order' Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 15E: Merge Only Truly Unknown Keys (fix for STEP 15B)

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 15E: write_versioned keeps only keys this binary does not know; cleared fields and legacy aliases are no longer resurrected

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/compat.rs
  - crates/backend_core/src/storage/mod.rs
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs `read_versioned`, `write_versioned` (STEP 15B)
  - crates/backend_core/src/models/scope.rs:36-44 (`description`, `bound_component`, `tracked_files`: skipped when None/empty)
  - crates/backend_core/src/models/component.rs:155-175 (ComponentRegistry accepts legacy keys `manifest` / `entries` for `registry`)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No model or serde attribute changes. `merge_unknown`, `stamp`, `check_readable` keep their signatures and behaviour.

[PRE_GATE]
grep -q 'fn write_versioned' crates/backend_core/src/storage/mod.rs && ! grep -q 'pub fn unknown_keys' crates/backend_core/src/storage/compat.rs

[MUTATION_SPEC]
Problem: STEP 15B merges every key that is in the old file but missing from the new table. A field the code just cleared (None, or an empty Vec skipped on write, e.g. scope `bound_component`, project `components`) is missing from the new table too, so the old value comes back. A legacy alias key (`[components] manifest = [...]`) is re-added next to the canonical `registry`, and the next read fails with a duplicate-field error.
1. compat.rs: `pub fn unknown_keys<F>(old: &Table, roundtrip: F) -> Result<Table> where F: Fn(&Table) -> Result<Table>`.
   - `let known = roundtrip(old)?;`. `roundtrip` deserializes a table into the model and serializes it back, so `known` holds exactly the keys the model understood.
   - Candidates: every key path present in `old` but absent from `known`. Recurse only where the value is a table in both `old` and `known`.
   - For each candidate, probe: clone `old`, remove that path, and call `roundtrip` on the clone. If the result equals `known`, the key is unknown, so keep it. Otherwise the model consumed it (an alias), so drop it.
   - Return a Table containing only the kept paths with their `old` values, creating intermediate tables as needed.
   - Doc comment: explain the probe and state the cost (one extra roundtrip per candidate; usually zero candidates).
2. storage/mod.rs write_versioned: bound becomes `T: Serialize + DeserializeOwned`. After `check_readable(&old, ..)` and recording `old_had_key`, remove `SCHEMA_KEY` from `old`, then `let extra = compat::unknown_keys(&old, |t| Ok(toml::Table::try_from(T::deserialize(toml::Value::Table(t.clone()))?)?))?;` and `merge_unknown(&mut new, &extra)` (replacing the current `merge_unknown(&mut new, &old)`). A deserialize failure of `old` goes through `attach_toml_path` like read_versioned.
3. compat.rs inline tests, using a private test struct `S { a: u32, #[serde(default, skip_serializing_if = "Option::is_none")] b: Option<String>, #[serde(default, alias = "old_c")] c: Vec<u32>, #[serde(default)] n: Inner }` with `Inner { x: u32 }` (both `#[derive(Serialize, Deserialize, Default)]`), and a roundtrip closure built from it:
   - old `{a=1, b="x", zz=true}` → `{zz=true}` (b is known, so it is not returned).
   - old `{a=1, old_c=[1]}` → `{}` (alias consumed).
   - old `{a=1, n={x=1, yy=2}}` → `{n={yy=2}}` (nested unknown).
   - old `{a=1}` → `{}`.
4. storage_integration.rs:
   - Clearing is not undone: set a scope's `bound_component = Some("x")` with `write_scope`, then `None` with `write_scope`; scope.toml text has no `bound_component` line and `read_scope` returns None.
   - The STEP 15B unknown-key tests still pass unchanged (do not edit them).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'compat::unknown_keys' crates/backend_core/src/storage/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 15C: Reconcile Skips Too-New Items

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 15C: reconcile_indexes skips (and reports) item files with an unsupported schema instead of failing

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/reconcile.rs
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/reconcile.rs:14-21 (ReconcileReport), :32-206 (reconcile_indexes), :208-300 (collect_authoritative_items; item.toml is read at :256-266)
  - crates/backend_core/src/storage/mod.rs `read_versioned` (STEP 15B; private, but callable from reconcile.rs because it is a child module of storage)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. CLI/TUI display of the new field is out of scope.

[PRE_GATE]
grep -q 'fn write_versioned' crates/backend_core/src/storage/mod.rs

[MUTATION_SPEC]
1. reconcile.rs:
   a. `ReconcileReport` gains `pub skipped_unsupported: Vec<String>`. It derives Default and nothing builds it with a struct literal, so no other construction changes.
   b. collect_authoritative_items today reads item.toml with `read_tracker_file` + `ItemManifest::from_toml_str` (:256-266). That path never checks schema_version (the models ignore unknown keys), so a too-new item would be indexed. Replace those lines with `storage.read_versioned::<ItemManifest>(&item_toml_path)`, which already attaches the path to parse errors.
   c. Add a parameter `skipped: &mut Vec<String>`. On `Err(TrackerError::UnsupportedSchema { .. })`, push the item.toml path relative to `storage.tracker_dir()` (components joined with `/`, e.g. `phases/pre_production/domains/inbox/items/BUG-0009/item.toml`) and `continue`. Every other error still propagates with `?`. Keep the dir-name counter update above it, so a skipped item's ID is still never reused.
   d. reconcile_indexes passes `&mut report.skipped_unsupported` and sorts it before returning. A skipped item is not indexed and not counted in `[summary]`; reconcile never rewrites item.toml.
2. storage_integration.rs: `init_standard`, create two bugs, then prepend `schema_version = 9\n` to the second bug's item.toml. reconcile_indexes returns Ok; `skipped_unsupported` equals exactly that one relative path; the scope's `items.bugs` holds only the first bug; `counters.bug` is still >= the second bug's number; the second bug's item.toml is byte-identical afterwards.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'skipped_unsupported' crates/backend_core/src/storage/reconcile.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 15D: Canonical `project add`, `--no-use`

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 15D: neb project add scaffolds the 3 canonical stages (same path as init); --no-use replaces the no-op --use

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/args.rs
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs:252-266 (init_standard)
  - crates/nebulous_cli/src/args.rs:457-472 (ProjectCommand::Add; the `Add` at :391 is a different command, leave it alone)
  - crates/nebulous_cli/src/commands/mod.rs:2079-2104 (ProjectCommand::Add handler)
  - existing `project add` tests in crates/nebulous_cli/tests/cli_integration.rs (copy how they isolate the global projects.toml)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.

[PRE_GATE]
grep -q '"01_core_engine"' crates/nebulous_cli/src/commands/mod.rs

[MUTATION_SPEC]
1. commands/mod.rs ProjectCommand::Add: replace `TrackerStorage::init(path, &init_name, "01_core_engine", "Core Engine & Initial Scope")` with `TrackerStorage::init_standard(path, &init_name)`.
2. args.rs ProjectCommand::Add: replace `set_active` (`--use`, `default_value_t = true`, which can never be turned off) with `#[arg(long = "no-use")] no_use: bool`, doc "Register without making it the active project." Keep `--use` / `--active` working for existing scripts as a hidden, ignored flag: `#[arg(long = "use", alias = "active", hide = true)] use_compat: bool` (plain `alias`: a hidden arg's aliases are hidden anyway). In the handler's `ProjectCommand::Add { .. }` pattern, bind `no_use` and ignore the old flag with `use_compat: _`, then pass `!*no_use` where `*set_active` was passed.
3. cli_integration.rs:
   - `neb project add <tmp>` on a directory without `.tracker/` creates phases pre_production, production and post_production, with active_phase pre_production;
   - `--no-use` leaves the previously active project active;
   - `--use` is still accepted.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && ! grep -q '"01_core_engine"' crates/nebulous_cli/src/commands/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### Step 16: Index Snapshot Caching

**Superseded 2026-11-15** by STEP 16A–16B in "Phase 5–7 Completion" at the end of this file. Kept for history.

```markdown
[STEP_ID]
STEP-16: Per-Operation `IndexSnapshot` & Cached Git Status in Assist

[SCOPE_LOCK]
- `crates/backend_core/src/storage/{mod.rs,id.rs}`, `crates/backend_core/src/workflow/{assist.rs,lifecycle.rs}`, `crates/backend_core/src/mcp/mod.rs`.

[MUTATION_SPEC]
1. `IndexSnapshot` (project + phases + scopes, loaded once) passed through `locate_item`, `phase_dir`, `scope_dir`,
   `compute_whats_next`, and MCP handlers; remove repeated `load_index` calls per operation.
2. `scan_max_counters`: scan each phase once.
3. Assist: compute git dirty files once per snapshot; exclude untracked recursion unless needed; never from error paths.

[POST_GATE]
- RUST_GATE; add a test asserting `create_item` reads `project.toml` at most twice (counting wrapper or tracing).

[HALT_INVARIANT]
- Halt and report.
```

### Step 17: Watcher Completion & TUI Live Reload

**Superseded 2026-11-15** by STEP 17A–17D in "Phase 5–7 Completion" at the end of this file. Kept for history.

```markdown
[STEP_ID]
STEP-17: Watcher Rescan Handling, Error Surfacing, Thread Join & TUI Integration

[SCOPE_LOCK]
- `crates/backend_core/src/watcher/mod.rs`, `crates/backend_core/src/storage/reconcile.rs`, `crates/nebulous_cli/src/tui/{mod.rs,state.rs,render.rs}`.

[MUTATION_SPEC]
1. Treat `EventKind::Other` / `need_rescan()` and watcher errors as a full reconcile trigger.
2. Echo suppression compares content hashes of index files written by reconcile instead of a 1s time window.
3. Surface reconcile errors in `TrackerWatchEvent`; reconcile skips (and reports) individual malformed TOML files.
4. Store the worker `JoinHandle`; join on `Drop`.
5. TUI subscribes to the watcher: refresh on events; render the active timer from wall clock each tick.

[POST_GATE]
- RUST_GATE; tests: malformed item.toml reported but others indexed; external `scope.toml` edit is reconciled.

[HALT_INVARIANT]
- Halt and report.
```

### Step 18: Timer Robustness

**Superseded 2026-11-15** by STEP 18A–18C in "Phase 5–7 Completion" at the end of this file. Kept for history.

```markdown
[STEP_ID]
STEP-18: Idempotent Timer Stop, Orphan/Corrupt Recovery, Idle Cap

[SCOPE_LOCK]
- `crates/backend_core/src/workflow/timer.rs`, `crates/backend_core/src/models/timer.rs`, `crates/backend_core/src/storage/mod.rs`, CLI timer commands.

[MUTATION_SPEC]
1. Stop: record a `session_id` in the timer and in the item's log entry; skip logging if that session is already recorded.
2. Corrupt or orphaned `.timer.toml` → warn, move it to `.timer.toml.bak`, continue.
3. Configurable `max_session_minutes` (default 480); longer sessions require `--force` or are capped with a warning.
4. Stop triggers parent index sync.

[POST_GATE]
- RUST_GATE; tests: double stop logs once; corrupt timer file does not block `neb list`.

[HALT_INVARIANT]
- Halt and report.
```

### Step 19: Zero-ID Consistency

**Superseded 2026-11-15** by STEP 19A–19C in "Phase 5–7 Completion" at the end of this file. Kept for history.

```markdown
[STEP_ID]
STEP-19: `start` Uses Same Ranking Context as `next`; Stage-Scoped Recommendations; Dry-Run

[SCOPE_LOCK]
- `crates/backend_core/src/workflow/{assist.rs,lifecycle.rs}`, `crates/nebulous_cli/src/{args.rs,commands/mod.rs}`, `crates/nebulous_cli/src/tui/state.rs`.

[MUTATION_SPEC]
1. Pass CWD into `compute_whats_next` from `start` (CLI, TUI, MCP).
2. Stagnant/recurring candidate lists filter to the active stage by default.
3. `neb start --dry-run` prints the chosen item; switching timers prints "stopped timer on X".
4. Align help text (`args.rs:128,187`) with actual zero-ID fallback behaviour.

[POST_GATE]
- RUST_GATE; test: `start` with no ID picks the same item as `next`'s #1 under identical state.

[HALT_INVARIANT]
- Halt and report.
```

---

## Phase 7 — Slim & Polish

### Step 20: Vault Decision

**WITHDRAWN 2026-11-15 (user: drop it).** Not planned; kept for history only.

```markdown
[STEP_ID]
STEP-20: Vault — Keyring-Primary, Passphrase-Protected Fallback

[SCOPE_LOCK]
- `crates/backend_core/src/crypto/mod.rs`, `crates/backend_core/Cargo.toml`, tests.

[MUTATION_SPEC]
1. Store secrets in the OS keyring only when available; use the vault only as an explicit fallback.
2. When `NEBULOUS_VAULT_MASTER_KEY` is a passphrase, derive with Argon2id (stored salt); never keep a plaintext
   key file alongside the vault unless the user opts in (`vault.key` mode documented as obfuscation-only).
3. Never fall back to `/tmp/nebulous` when `$HOME` is unset; error instead.

[POST_GATE]
- RUST_GATE

[HALT_INVARIANT]
- Halt and report.
```

### Step 21: TUI Polish

**Superseded 2026-11-15** by STEP 21A–21F in "Phase 5–7 Completion" at the end of this file. Kept for history.

```markdown
[STEP_ID]
STEP-21: Panic Hook, Debounced Filter, Honest Clipboard, Scope Confirmation

[SCOPE_LOCK]
- `crates/nebulous_cli/src/tui/*.rs`, `crates/nebulous_cli/Cargo.toml`, `crates/nebulous_cli/src/commands/mod.rs` (clipboard).

[MUTATION_SPEC]
1. Install a panic hook that restores the terminal before printing the panic.
2. Filter keystrokes filter the in-memory item list; disk refresh only on watcher events or explicit `r`.
3. Clipboard: enable arboard `wayland-data-control`; fall back to OSC 52; report failure in the status bar.
4. Creating an item with an unknown scope prompts for confirmation instead of auto-creating.
5. Stage switching (`1/2/3`) is view-only; persist `active_phase` only via `neb stage`.

[POST_GATE]
- RUST_GATE

[HALT_INVARIANT]
- Halt and report.
```

### Step 22: Docs Reconciliation, Dead Code & Installer

**Superseded 2026-11-15** by STEP 22A–22C in "Phase 5–7 Completion" at the end of this file. Kept for history.

```markdown
[STEP_ID]
STEP-22: Docs Match Behaviour; Remove Dead Code; Portable install.sh

[SCOPE_LOCK]
- `README.md`, `docs/USAGE.md`, `docs/ARCHITECTURE.md`, `scripts/install.sh`,
  dead code in `crates/backend_core/src/git_sync/mod.rs`, `crates/backend_core/src/agent_context/mod.rs`,
  `crates/nebulous_cli/src/commands/mod.rs`.

[MUTATION_SPEC]
1. Fix the documented claims: zero-ID scope, watcher integration, `log_effort`, platforms actually shipped,
   `q` behaviour inside modals.
2. Remove `fetch_repo`, `push_repo`, `pull_repo`, `DocumentedCommitReport`/`DocumentedCommitResult` aliases,
   `export_bounded_prompt`, duplicate `Commands::Mcp` arm, and unused `with_include_workspace`.
3. install.sh: build as the invoking user, then `sudo install` only the binary for `--system`;
   replace `realpath -m` with a portable fallback.

[POST_GATE]
- RUST_GATE; `bash -n scripts/install.sh`; `shellcheck scripts/install.sh` if available.

[HALT_INVARIANT]
- Halt and report final verification results.
```

---

# STEP 14 Re-split — MCP Robustness (2026-11-15)

The original STEP 14 (Phase 6, "### Step 14: MCP Server Robustness") predates the Argon format: 7 items across `mcp/mod.rs` and `agent_context/mod.rs`. It is replaced by 14A–14I below, one concern each. Full Argon blocks are written (and dry-run) when each step reaches #1, the same "Commit, write X" flow as 23–24; the outlines here fix scope and order only. **Superseded 2026-11-15:** 14G–14I blocks written together with the Phase 5–7 completion; no more just-in-time blocks.

**State at HEAD `495c5de` (what's already true vs. still needed):**
| Original item | Today | Step |
| :--- | :--- | :--- |
| 1. Framing bounds, headers in any order | Not done. `read_next_jsonrpc_message` (`mcp/mod.rs:2329-2370`) allocates `vec![0u8; content_len]` uncapped, `read_line` is unbounded, `Content-Length` must be the first header, and a bad header returns an io error that ends `serve_io`. | 14B |
| 2. No reply without `id`; keep `id` on errors; require `jsonrpc == "2.0"` | Partly. Only `notifications/*`, `initialized`, `cancelled` are silent without `id` (`:333-340`); any other id-less method gets a reply with `id: null`. Invalid-shape requests always answer `id: null`. `jsonrpc` defaults to `"2.0"` when missing and is never checked (`:28-36`). | 14C |
| 3. Negotiate `protocolVersion` | Not done: `initialize` echoes whatever the client sends (`:346-371`). | 14D |
| 4. Payload only under `structuredContent` | Partly: `structuredContent` exists, but the payload is **also** flattened into the result object (`:413-431`), and error results flatten `status`/`error`. 7 assertions in `cli_integration.rs:1332-1364` read the flattened keys. | 14E |
| 5. `dry_run` (default true) on destructive tools; `list_projects` read-only | Not done. No destructive tool has `dry_run`. `list_projects` registers the workspace in `projects.toml` as a side effect (`:1470-1490`). | 14F, 14G, 14H |
| 6. Bounded agent context | Not done. `synthesize_bounded_prompt` (`agent_context/mod.rs:95-185`) embeds `desc.md`, scope description and fix summaries verbatim, unfenced. (No verification logs are embedded today, so there's no log cap to add.) Shared by MCP, `neb context`, `neb start` and the TUI `c` key. | 14I |
| 7. Derive `is_known_tool` | Not done: a second hand-kept list (`:578-605`); 24F had to add `plan_import` to both. | 14A |

## Steps (in order)

| Step | Cmd | Scope | Files | Waits on |
| :--- | :---: | :--- | :--- | :--- |
| 14A | `/goal` | `is_known_tool` = name is in `tools_list()`; test that every listed tool dispatches (none hits "Unknown MCP tool") | `mcp/mod.rs` | — |
| 14B | `/goal` | Framing: 8 MiB cap on `Content-Length` and on a line; overflow and bad headers → `-32600` reply, drain, keep serving; headers in any order | `mcp/mod.rs`, `tests/mcp_protocol.rs` (NEW) | — |
| 14C | `/goal` | JSON-RPC: no reply to any id-less message (batch included); keep a readable `id` on invalid-request errors; `jsonrpc` must be `"2.0"` | `mcp/mod.rs`, `tests/mcp_protocol.rs` | OQ14-3 |
| 14D | `/goal` | `initialize` picks from a supported-versions list; unknown → server's newest | `mcp/mod.rs`, `tests/mcp_protocol.rs` | OQ14-4 |
| 14E | `/goal` | `tools/call` result = `content` + `structuredContent` + `isError` only (no flattened keys); update the 20 CLI reads to `structuredContent` (was "7"; recounted 2026-11-15) | `mcp/mod.rs`, `nebulous_cli/tests/cli_integration.rs` | OQ14-1 |
| 14F | `/goal` | `list_projects` read-only (no registry write) | `mcp/mod.rs`, `tests/mcp_protocol.rs` | — |
| 14G | `/boost` | `dry_run` on `git_push`, `git_pull`, `git_sync`: dry run reports status and what would happen, no network, no commit | `mcp/mod.rs`, `tests/workflow_integration.rs` | OQ14-2 |
| 14H | `/boost` | `dry_run` on `rollback_to_pinned_release`, `ingest_project_directory` (writing modes), `switch_project` | `mcp/mod.rs`, `tests/mcp_protocol.rs` | OQ14-2 |
| 14I | `/goal` | Agent context: cap `desc.md` (16 KiB) and other free text, fence it in labelled blocks, strip control characters | `agent_context/mod.rs`, `tests/workflow_integration.rs` | OQ14-5 |

## Open questions for the re-split (all resolved 2026-11-15: defaults accepted)

- **OQ14-1 (14E) Drop the flattened result keys?** Spec-following MCP clients (Claude Code, Cursor, Codex) read `content` / `structuredContent`, so they're unaffected. Anything that reads top-level keys of a `tools/call` result breaks, including the 7 existing CLI test assertions (which 14E updates). Default: yes, drop them.
- **OQ14-2 (14G, 14H) `dry_run` default for destructive tools.** The original spec says default `true`. That changes behaviour for any agent that calls `git_sync` / `git_push` today: the call would report and do nothing until it passes `dry_run: false`. Options: (a) default `true` as specified; (b) default `false`, opt-in preview only (no behaviour change). Default: (a).
- **OQ14-3 (14C) Reject a missing `jsonrpc` field?** Today a request without `"jsonrpc"` is accepted. Requiring `"2.0"` follows the JSON-RPC spec but rejects sloppy clients. Default: reject (`-32600`).
- **OQ14-4 (14D) Supported protocol versions.** Proposed list: `2025-06-18`, `2025-03-26`, `2024-11-05` (current default). The server already returns `structuredContent` (a 2025-06-18 feature). Unknown versions get `2025-06-18`. Default: that list.
- **OQ14-5 (14I) Bounds and where they apply.** Proposed: `desc.md` 16 KiB, scope description and each fix summary 1 KiB, truncated with a visible `[… truncated N bytes]` marker. The same prompt is used by `neb context`, `neb start` and the TUI copy, so their output changes too (one bounded prompt everywhere). Default: as proposed, applied everywhere.


### STEP 14A: One Tool Dispatch Table

* **Recommended Command:** `/goal`

**Written 2026-11-15** against HEAD `495c5de` (STEP 14 re-split, original item 7). Dry-run (scratchpad only): 1 file, +80/−57, gate below exit 0.

**Differs from the original re-split row (approved 2026-11-15):** the row says "`is_known_tool` = name is in `tools_list()`". This block instead derives both `is_known_tool` and `call_tool` from one name → handler table (`tool_handler`), which is the original STEP 14 wording ("derive from the dispatch table"). Why: with the `tools_list()` approach, proving "every listed tool dispatches" needs actually calling all 24 tools (network git calls, registry writes); with the table, the test checks it without calling any tool, and `tools_list()` (which builds 24 JSON schemas) isn't rebuilt on every `tools/call`.

**Current code (cited for the spec):**
- `mcp/mod.rs:24` `pub const DEFAULT_MCP_PROTOCOL_VERSION`; `:577-606` `/// Returns \`true\` if \`name\` is a supported MCP tool.` `pub fn is_known_tool(name: &str) -> bool { matches!(name.trim(), "get_active_task" | … | "git_sync") }` (24 names); `:396` its only caller (`tools/call`, returns `-32602 Unknown tool`).
- `:934-966` `pub fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value> { match name.trim() { // Suite 1 … "get_active_task" => self.tool_get_active_task(arguments), … // Suite 4 … "git_sync" => self.tool_git_sync(arguments), other => Err(TrackerError::ValidationError(format!("Unknown MCP tool '{other}'"))) } }`.
- All 24 `tool_*` methods have the signature `fn(&mut self, args: Value) -> Result<Value>`.
- `mcp/mod.rs` has no `#[cfg(test)]` module yet; `McpServer::new(None)` builds a server with no storage.

```text
[STEP_ID]
STEP 14A: call_tool and is_known_tool share one name→handler table; the duplicate tool-name list is removed

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:577-606 (is_known_tool), :934-966 (call_tool), :396 (tools/call caller)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No tool is added, removed, renamed or changed; tool_* bodies, tools_list() and the JSON-RPC layer stay as they are. Error texts ("Unknown MCP tool '<name>'", "Unknown tool: '<name>'") are unchanged.

[PRE_GATE]
grep -q 'pub fn is_known_tool' crates/backend_core/src/mcp/mod.rs && ! grep -q 'fn tool_handler' crates/backend_core/src/mcp/mod.rs && test "$(grep -c '"git_sync"' crates/backend_core/src/mcp/mod.rs)" -eq 3

[MUTATION_SPEC]
1. After `DEFAULT_MCP_PROTOCOL_VERSION`: `/// Signature shared by every MCP tool handler.` `type ToolHandler = fn(&mut McpServer, Value) -> Result<Value>;`
2. `fn tool_handler(name: &str) -> Option<ToolHandler>` (private, after `call_tool`), doc saying it is the single dispatch table read by both `call_tool` and `is_known_tool`: `let handler: ToolHandler = match name.trim() { <the same 24 arms, same order and suite comments, each "name" => Self::tool_<name>,> _ => return None }; Some(handler)`.
3. `call_tool` becomes `match Self::tool_handler(name) { Some(handler) => handler(self, arguments), None => Err(TrackerError::ValidationError(format!("Unknown MCP tool '{}'", name.trim()))) }`.
4. `is_known_tool` becomes `Self::tool_handler(name).is_some()`; its doc mentions the table. The 24-name `matches!` list is deleted.
5. New `#[cfg(test)] mod tests { use super::*; … }` at the end of mcp/mod.rs:
   a. `every_listed_tool_has_a_handler_and_names_are_unique`: for every `McpServer::tools_list()` entry, `is_known_tool(&tool.name)` is true and the name is unique; `tools_list().len() == 24`.
   b. `unknown_tool_is_rejected_by_is_known_tool_and_call_tool`: `is_known_tool("no_such_tool")` is false; `is_known_tool("  plan_import  ")` is true (names are trimmed); `McpServer::new(None).call_tool("no_such_tool", json!({}))` errors with a message containing `Unknown MCP tool 'no_such_tool'`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'fn tool_handler(name: &str) -> Option<ToolHandler>' crates/backend_core/src/mcp/mod.rs && test "$(grep -c '"git_sync"' crates/backend_core/src/mcp/mod.rs)" -eq 2 && test "$(grep -c '"plan_import"' crates/backend_core/src/mcp/mod.rs)" -eq 2 && test "$(cargo test -p backend_core --lib -- mcp::tests:: 2>&1 | grep -c 'test result: ok. 2 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

**Decisions the contract leaves open:**
- **The test pins 24 tools.** Adding or removing a tool later means updating that number in the same step, which is the point: it forces the step to say so.


### STEP 14B: MCP Framing Bounds

* **Recommended Command:** `/goal`

**Written 2026-11-15** against HEAD `dd4ce08` (STEP 14 re-split, original item 1). Dry-run (scratchpad only): `mcp/mod.rs` +167/−58 plus the new test file; gate below exit 0.

**Behaviour change to know about:** today a bad `Content-Length` header returns an io error that **ends `neb mcp`**. After this step the server replies `-32600` and keeps serving.

**Decisions the spec leaves open (approved 2026-11-15):**
- **G1 a rejected frame's reply has `id: null`** (the body was never read, so there's no id to keep; 14C handles ids for bodies that do parse).
- **G2 what counts as a header line:** `Name: value` where `Name` is letters, digits and `-` only. Anything else (every JSON line starts with `{` or `[`) is a newline-delimited message, exactly as today.
- **G3 every rejection keeps serving:** over-long line, `Content-Length` over the cap, unparseable `Content-Length`, a header block with no `Content-Length`, a body that isn't UTF-8. The reply uses the same framing the client used.
- **G4 nothing over the cap is buffered:** an over-long line is consumed up to its newline and discarded; an oversized body is skipped with `io::copy(take(len), sink())`. A client that claims a huge length and sends less simply reaches EOF.
- **G5 header lines are capped too** (same 8 MiB limit), and `MAX_MCP_MESSAGE_BYTES` is `pub` so tests and callers can use it.

**Current code (cited for the spec):**
- `mcp/mod.rs:2` `use std::io::{self, BufRead, Write};`; `:24` `DEFAULT_MCP_PROTOCOL_VERSION`; `:26` `/// Signature shared by every MCP tool handler.` (14A).
- `:226-258` `pub fn serve_io<R: BufRead, W: Write>(&mut self, mut reader: R, mut writer: W) -> Result<()>`: `while let Some((raw_message, used_content_length)) = read_next_jsonrpc_message(&mut reader)…?` → `handle_raw_message` → writes `Content-Length: N\r\n\r\n…` when the request was framed, else one line.
- `:2316-2361` `fn read_next_jsonrpc_message<R: BufRead>(reader: &mut R) -> io::Result<Option<(String, bool)>>`: unbounded `read_line`; only a first line starting `content-length:` starts a frame; following header lines are skipped; `vec![0u8; content_len]` with no cap; parse/UTF-8 failures are `io::Error`s.
- `JsonRpcResponse::error(id, code, message, data)` at `:64`. `lib.rs:18` `pub use mcp::*;`. `McpServer::new(None)` serves `ping` with no storage.

```text
[STEP_ID]
STEP 14B: MCP stdio framing is bounded at 8 MiB, accepts headers in any order, and answers bad frames with -32600 instead of exiting

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/mcp_protocol.rs (NEW)
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:226-258 (serve_io), :2316-2361 (read_next_jsonrpc_message), :64 (JsonRpcResponse::error)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. handle_raw_message, handle_request and every tool stay as they are (JSON-RPC rules are 14C). Messages under the cap are read and answered exactly as today, in both framings.

[PRE_GATE]
grep -q 'fn tool_handler' crates/backend_core/src/mcp/mod.rs && grep -q 'vec!\[0u8; content_len\]' crates/backend_core/src/mcp/mod.rs && test ! -e crates/backend_core/tests/mcp_protocol.rs

[MUTATION_SPEC]
1. Before the 14A `ToolHandler` alias: `/// Largest JSON-RPC message accepted on stdio …` `pub const MAX_MCP_MESSAGE_BYTES: usize = 8 * 1024 * 1024;` (G5).
2. Private `enum IncomingFrame { Message { body: String, framed: bool }, Rejected { reason: String, framed: bool } }` next to the reader, with a doc on each variant.
3. `fn read_bounded_line<R: BufRead>(reader: &mut R, limit: usize) -> io::Result<Option<(Vec<u8>, bool)>>`: reads via `fill_buf` / `consume` up to the newline; `None` at EOF with nothing read; the bool is true when the line was longer than `limit`, in which case the rest of the line is consumed and dropped, never stored (G4).
4. `fn is_header_line(line: &str) -> bool`: true when the part before the first `:` is non-empty and only ASCII letters, digits and `-` (G2).
5. `read_next_jsonrpc_message` returns `io::Result<Option<IncomingFrame>>`:
   - lines via `read_bounded_line(.., MAX_MCP_MESSAGE_BYTES)`; an over-long line → `Rejected { framed: false }`; blank lines are skipped;
   - a non-header line → `Message { body: trimmed, framed: false }`;
   - a header line starts a header block read until a blank line or EOF; `Content-Length` is found case-insensitively in any position;
   - no `Content-Length`, or one that doesn't parse as `usize` → `Rejected { framed: true }`;
   - length over the cap → skip exactly `len` bytes with `io::copy(&mut io::Read::take(&mut *reader, len as u64), &mut io::sink())`, then `Rejected { framed: true }` (G4);
   - otherwise read the body (`read_exact`); non-UTF-8 → `Rejected { framed: true }`, else `Message { body, framed: true }`.
   Real io errors (a failed read) still propagate as `io::Error`.
6. `serve_io`: `Message` → `handle_raw_message` as today; `Rejected { reason, framed }` → `JsonRpcResponse::error(Value::Null, -32600, format!("Invalid Request: {reason}"), None)` (G1); both are written in the frame's own framing (`Content-Length: N\r\n\r\n…` when framed, else one line) and flushed; the loop continues (G3).
7. tests/mcp_protocol.rs (NEW): helpers `serve(input: Vec<u8>) -> String` (runs `McpServer::new(None).serve_io(Cursor::new(input), &mut Vec)` and asserts it returns Ok), `framed(body) -> Vec<u8>`, and `framed_bodies(out) -> Vec<Value>`; `const PING = {"jsonrpc":"2.0","id":7,"method":"ping"}`:
   a. `oversized_content_length_is_rejected_without_allocating`: input `Content-Length: 1073741824\r\n\r\n` (no body) → exactly one framed reply, `error.code == -32600`, message contains `exceeds`.
   b. `invalid_content_length_is_rejected_and_serving_continues`: `Content-Length: abc\r\n\r\n` then a framed PING → two framed replies: `-32600`, then `id == 7` with no `error`.
   c. `headers_are_accepted_in_any_order`: `Content-Type: application/vscode-jsonrpc; charset=utf-8\r\n` before `Content-Length` → one framed reply with `id == 7`, no `error`.
   d. `overlong_line_is_rejected_and_the_next_line_is_served`: `MAX_MCP_MESSAGE_BYTES + 1` bytes of `x`, `\n`, then PING and `\n` → two line replies: `-32600`, then `id == 7`.
   e. `non_content_length_header_lines_are_ignored` (strike-1 correction, 2026-11-15): `Content-Length: <len>\r\nbogus\r\n\r\n` then PING as the body → exactly one framed reply with `id == 7`, no `error`, no unframed output. Inside a header block, any line that is not `Content-Length` (over-long, non-UTF-8, not `Name: value`) is skipped, as today.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'pub const MAX_MCP_MESSAGE_BYTES: usize = 8 \* 1024 \* 1024;' crates/backend_core/src/mcp/mod.rs && ! grep -q 'vec!\[0u8; content_len\]' crates/backend_core/src/mcp/mod.rs && test "$(cargo test -p backend_core --test mcp_protocol 2>&1 | grep -c 'test result: ok. 5 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 14C: JSON-RPC Request Rules

* **Recommended Command:** `/goal`

**Written 2026-11-15** against HEAD `ba47e0f` (STEP 14 re-split, original item 2; OQ14-3 resolved: reject a missing `jsonrpc`). Dry-run (scratch worktree, removed): `mcp/mod.rs` +42/−34, `tests/mcp_protocol.rs` +84; gate below exit 0; the same POST_GATE exits 1 on an untouched tree.

**Behaviour changes to know about:**
- An id-less request for a method that isn't a notification (`ping`, `tools/call`, …) is now **neither answered nor run**. Today it runs and gets a reply with `id: null`.
- A request without `"jsonrpc": "2.0"` now gets `-32600`. Every request in this repo's tests and every spec-following MCP client already sends it.

**Decisions the spec leaves open (approve before hand-off):**
- **C1 id-less = not dispatched.** JSON-RPC says a notification gets no reply. It doesn't require running it, and MCP defines no request method as a notification. A `tools/call` whose result and errors nobody can see shouldn't run (e.g. a `git_push`). Every id-less method `handle_request` knows today (`notifications/*`, `initialized`, `cancelled`) is already a no-op, so nothing is lost.
- **C2 `"id": null` counts as no id.** `JsonRpcRequest.id` is `Option<Value>` with `#[serde(default)]`, so serde reads `null` as `None`. Telling the two apart needs a custom deserializer for a form the JSON-RPC spec discourages. Kept as is.
- **C3 a "readable" id is a string or number.** That's the JSON-RPC rule. An id of any other type (object, array, bool) gets `-32600` with `id: null`.
- **C4 invalid requests are always answered**, id or not (JSON-RPC: "Invalid Request" replies with `id: null` when there's no readable id). The silence rule applies only to well-formed requests. A batch gets one error per invalid entry, and no reply at all when every entry is a notification (already true).
- **C5 the `jsonrpc` check sits at the wire layer** (`handle_raw_message`, via the new `parse_request`). The `JsonRpcRequest` struct keeps its `#[serde(default = "default_jsonrpc_version")]`, so direct callers of the public `handle_request` are unaffected.
- **C6 the error reply is boxed** (`Result<JsonRpcRequest, Box<JsonRpcResponse>>`). Clippy's `result_large_err` rejects the unboxed 152-byte `Err` under `-D warnings`. This is the cause fix, not an `#[allow]`.

**Current code (cited for the spec):**
- `mcp/mod.rs:33-43` `pub struct JsonRpcRequest { #[serde(default = "default_jsonrpc_version")] pub jsonrpc: String, #[serde(default, skip_serializing_if = "Option::is_none")] pub id: Option<Value>, pub method: String, … params: Option<Value> }`; `:67` `JsonRpcResponse::error(id, code, message, data)`.
- `:278-346` `pub fn handle_raw_message(&mut self, raw_json: &str) -> Option<String>`: `-32700` on bad JSON; batch loop `:308-323` does `match serde_json::from_value::<JsonRpcRequest>(item) { Ok(req) => …handle_request…, Err(err) => responses.push(JsonRpcResponse::error(Value::Null, -32600, format!("Invalid Request: {err}"), None)) }`; empty `responses` → `None`; the single path `:331-341` is the same `from_value` with an `id: null` `-32600`.
- `:349-361` `pub fn handle_request(…)`: `// Standard MCP notifications …` `if request.id.is_none() && (method.starts_with("notifications/") || method == "initialized" || method == "cancelled") { return None; }` then `let req_id = request.id.clone().unwrap_or(Value::Null);` (the file's only `unwrap_or(Value::Null)`).
- `:392-398` `"notifications/initialized" | "initialized" => { if request.id.is_some() { Some(success(req_id, json!({}))) } else { None } }`.
- `:2520` `fn default_jsonrpc_version() -> String`, right after `fn non_empty` at `:2516`.
- `tests/mcp_protocol.rs` (14B): `use backend_core::{McpServer, MAX_MCP_MESSAGE_BYTES}; use serde_json::Value;`, helpers `serve`, `framed`, `framed_bodies`, `const PING`, 5 `#[test]`s.

```text
[STEP_ID]
STEP 14C: id-less JSON-RPC requests get no reply and are not run; invalid requests keep a string/number id; jsonrpc must be "2.0"

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/mcp_protocol.rs
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:33-43 (JsonRpcRequest), :278-346 (handle_raw_message), :349-361 and :392-398 (handle_request), :2516-2522 (non_empty, default_jsonrpc_version)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. JsonRpcRequest, JsonRpcResponse and their serde attributes stay as they are. serve_io, the 14B framing code, every handle_request method arm other than "notifications/initialized" | "initialized", and every tool stay as they are. The -32700 parse error and the "empty batch" -32600 reply are unchanged. The 5 existing tests in mcp_protocol.rs are unchanged.

[PRE_GATE]
grep -q 'pub const MAX_MCP_MESSAGE_BYTES' crates/backend_core/src/mcp/mod.rs && grep -q 'method == "cancelled"' crates/backend_core/src/mcp/mod.rs && ! grep -q 'fn parse_request' crates/backend_core/src/mcp/mod.rs && test "$(grep -c '^#\[test\]' crates/backend_core/tests/mcp_protocol.rs)" -eq 5

[MUTATION_SPEC]
1. mcp/mod.rs, new private free fn directly above `fn default_jsonrpc_version`, with a doc saying it checks one incoming message and, on failure, returns the -32600 reply carrying the message's id when that id is a string or number, otherwise null:
   `fn parse_request(message: Value) -> std::result::Result<JsonRpcRequest, Box<JsonRpcResponse>>` (C6), in this order:
   a. `let id = match message.get("id") { None | Some(Value::Null) => Value::Null, Some(id @ (Value::String(_) | Value::Number(_))) => id.clone(), Some(_) => return Err(Box::new(JsonRpcResponse::error(Value::Null, -32600, "Invalid Request: id must be a string, number or null", None))) };` (C3)
   b. `if message.get("jsonrpc").and_then(Value::as_str) != Some("2.0")` → `Err(Box::new(JsonRpcResponse::error(id, -32600, "Invalid Request: jsonrpc must be \"2.0\"", None)))` (OQ14-3, C5).
   c. `serde_json::from_value(message).map_err(|err| Box::new(JsonRpcResponse::error(id, -32600, format!("Invalid Request: {err}"), None)))`.
2. handle_raw_message batch loop: `match parse_request(item) { Ok(req) => { if let Some(resp) = self.handle_request(req) { responses.push(resp); } } Err(resp) => responses.push(*resp), }`. The empty-batch reply and `responses.is_empty() → None` stay (C4).
3. handle_raw_message single path: `let request = match parse_request(parsed) { Ok(req) => req, Err(resp) => return serde_json::to_string(&resp).ok(), };`.
4. handle_request: delete the `// Standard MCP notifications …` comment, the `if request.id.is_none() && (…) { return None; }` block and `let req_id = request.id.clone().unwrap_or(Value::Null);`. In their place: `// A request without \`id\` is a JSON-RPC notification: no reply, and it is not dispatched.` then `let req_id = request.id.clone()?;` (C1, C2). `let method = …` stays above it; `let params = …` stays below it.
5. handle_request `"notifications/initialized" | "initialized"` arm becomes `Some(JsonRpcResponse::success(req_id, json!({})))` (the `if request.id.is_some()` check is now always true).
6. tests/mcp_protocol.rs, appended after the 14B tests: helper `fn reply(raw: &str) -> Option<Value>` = `McpServer::new(None).handle_raw_message(raw).map(|out| serde_json::from_str(&out).expect("reply should be valid JSON"))`. Then 7 tests (`-32600` means `["error"]["code"] == -32600`):
   a. `idless_requests_get_no_reply`: `reply(..)` is None for `{"jsonrpc":"2.0","method":"ping"}`, for `{"jsonrpc":"2.0","method":"tools/call","params":{"name":"no_such_tool"}}`, and for `{"jsonrpc":"2.0","id":null,"method":"ping"}`.
   b. `notification_only_batch_gets_no_reply`: `[{"jsonrpc":"2.0","method":"notifications/initialized"},{"jsonrpc":"2.0","method":"ping"}]` → None.
   c. `mixed_batch_replies_only_to_requests_with_id`: `[{"jsonrpc":"2.0","method":"ping"},{"jsonrpc":"2.0","id":"a","method":"ping"}]` → an array of 1 reply, `id == "a"`, no `error`.
   d. `invalid_request_keeps_readable_id`: `{"jsonrpc":"2.0","id":9}` → -32600, `id == 9`; `{"jsonrpc":"2.0","id":"x","method":5}` → -32600, `id == "x"`; `{"jsonrpc":"2.0","method":5}` → -32600, `id == null` (C4: answered even without an id).
   e. `missing_or_wrong_jsonrpc_is_rejected`: `{"id":3,"method":"ping"}` → -32600, `id == 3`, message contains `jsonrpc`; `{"jsonrpc":"1.0","id":4,"method":"ping"}` → -32600, `id == 4`; batch `[{"id":5,"method":"ping"}]` → array of 1, -32600, `id == 5`.
   f. `unreadable_id_is_rejected_with_null_id`: for each of `{"jsonrpc":"2.0","id":{"a":1},"method":"ping"}` and `{"jsonrpc":"2.0","id":true,"method":"ping"}` → -32600, `id == null`.
   g. `invalid_batch_entries_each_get_an_error`: `[1,{"jsonrpc":"2.0","id":2,"method":"ping"}]` → array of 2: [0] -32600 with `id == null`, [1] `id == 2` with no `error`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'fn parse_request(message: Value) -> std::result::Result<JsonRpcRequest, Box<JsonRpcResponse>>' crates/backend_core/src/mcp/mod.rs && grep -q 'let req_id = request.id.clone()?;' crates/backend_core/src/mcp/mod.rs && ! grep -q 'method == "cancelled"' crates/backend_core/src/mcp/mod.rs && ! grep -q 'unwrap_or(Value::Null)' crates/backend_core/src/mcp/mod.rs && test "$(cargo test -p backend_core --test mcp_protocol 2>&1 | grep -c 'test result: ok. 12 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 14D: MCP Protocol Version Negotiation

* **Recommended Command:** `/goal`

**Written 2026-11-15** against HEAD `04a206b` (STEP 14 re-split, original item 3; OQ14-4 resolved: supported `2025-06-18`, `2025-03-26`, `2024-11-05`, unknown → `2025-06-18`). Dry-run (scratch worktree, removed): `mcp/mod.rs` +25/−8, `tests/mcp_protocol.rs` +51/−1; gate below exit 0; its markers fail on an untouched tree.

**Behaviour change to know about:** `initialize` no longer echoes whatever version the client sends. A client asking for an unknown version, or sending none, now gets `2025-06-18`. Today an unknown version is echoed back, and a missing one gets `2024-11-05`.

**Decisions the spec leaves open (approve before hand-off):**
- **D1 `DEFAULT_MCP_PROTOCOL_VERSION` becomes `"2025-06-18"`** and means "what unknown or missing gets". It's `pub`, but nothing outside `mcp/mod.rs` reads it (checked across `crates/`, `docs/` and `README.md`). Keeping it at `2024-11-05` would leave a constant named "default" that no code path returns.
- **D2 a missing, empty or non-string `protocolVersion` counts as unknown → newest.** The MCP spec says clients MUST send it, so this only affects broken clients, which today get `2024-11-05`.
- **D3 the requested version is trimmed before matching** (`" 2025-03-26 "` → `2025-03-26`), the same leniency today's `.filter(|s| !s.trim().is_empty())` gives.
- **D4 no other `initialize` output changes.** `capabilities` stays as is. The server already sends `structuredContent`, which is a 2025-06-18 feature, so advertising 2025-06-18 is accurate. Per-version capability differences are out of scope.

**Current code (cited for the spec):**
- `mcp/mod.rs:23-24` `/// Default MCP protocol version advertised by the Nebulous stdio server.` `pub const DEFAULT_MCP_PROTOCOL_VERSION: &str = "2024-11-05";` (re-exported by `lib.rs:18` `pub use mcp::*;`).
- `:342-371` the `"initialize" =>` arm: `let requested_version = params.get("protocolVersion").and_then(Value::as_str).filter(|s| !s.trim().is_empty()).unwrap_or(DEFAULT_MCP_PROTOCOL_VERSION);` then `json!({ "protocolVersion": requested_version, "capabilities": {…}, "serverInfo": {…} })` (`:352`).
- `:2495` `fn parse_request` (14C), `:2528` `fn default_jsonrpc_version`.
- `tests/mcp_protocol.rs` (14B/14C): `use backend_core::{McpServer, MAX_MCP_MESSAGE_BYTES};`, helper `fn reply(raw: &str) -> Option<Value>` (runs `McpServer::new(None).handle_raw_message` and parses the reply), 12 `#[test]`s.
- `nebulous_cli/tests/cli_integration.rs:1111` sends `"protocolVersion": "2024-11-05"` but never asserts the reply's version (`:1287-1288` check `id` and `serverInfo.name` only). It is unaffected.

```text
[STEP_ID]
STEP 14D: initialize answers with the client's protocolVersion when supported, otherwise the newest supported version (2025-06-18)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/mcp_protocol.rs
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:23-24 (DEFAULT_MCP_PROTOCOL_VERSION), :342-371 (initialize arm), :2495-2530 (parse_request, default_jsonrpc_version)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. The initialize reply's "capabilities" and "serverInfo" stay exactly as they are. Every other handle_request arm, parse_request, serve_io, the framing code and every tool stay as they are. The 12 existing tests in mcp_protocol.rs are unchanged (only its `use backend_core::{…}` line grows).

[PRE_GATE]
grep -q 'fn parse_request' crates/backend_core/src/mcp/mod.rs && grep -q 'requested_version' crates/backend_core/src/mcp/mod.rs && grep -q 'pub const DEFAULT_MCP_PROTOCOL_VERSION: &str = "2024-11-05";' crates/backend_core/src/mcp/mod.rs && ! grep -q 'SUPPORTED_MCP_PROTOCOL_VERSIONS' crates/backend_core/src/mcp/mod.rs && test "$(grep -c '^#\[test\]' crates/backend_core/tests/mcp_protocol.rs)" -eq 12

[MUTATION_SPEC]
1. mcp/mod.rs:23-24 is replaced by two consts:
   `/// MCP protocol versions this server speaks, newest first.` `pub const SUPPORTED_MCP_PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];`
   `/// Version \`initialize\` answers with when the client asks for one not in [\`SUPPORTED_MCP_PROTOCOL_VERSIONS\`] (or none): the newest supported version.` `pub const DEFAULT_MCP_PROTOCOL_VERSION: &str = "2025-06-18";` (D1)
2. New private free fn directly above `fn default_jsonrpc_version`, with a doc saying it picks the version initialize answers with: the trimmed requested version when it is in SUPPORTED_MCP_PROTOCOL_VERSIONS, otherwise DEFAULT_MCP_PROTOCOL_VERSION:
   `fn negotiate_protocol_version(requested: Option<&str>) -> &'static str { requested.map(str::trim).and_then(|wanted| SUPPORTED_MCP_PROTOCOL_VERSIONS.iter().copied().find(|version| *version == wanted)).unwrap_or(DEFAULT_MCP_PROTOCOL_VERSION) }` (D2, D3)
3. initialize arm: `let requested_version = …;` becomes `let protocol_version = negotiate_protocol_version(params.get("protocolVersion").and_then(Value::as_str));`, and `"protocolVersion": requested_version,` becomes `"protocolVersion": protocol_version,`. Nothing else in the arm changes (D4).
4. tests/mcp_protocol.rs: the import becomes `use backend_core::{McpServer, DEFAULT_MCP_PROTOCOL_VERSION, MAX_MCP_MESSAGE_BYTES, SUPPORTED_MCP_PROTOCOL_VERSIONS};` (as rustfmt lays it out). Appended after the 14C tests: helper `fn initialize_version(params: &str) -> Value` that sends `{"jsonrpc":"2.0","id":1,"method":"initialize","params":<params>}` through `reply` and returns `["result"]["protocolVersion"]`. Then 3 tests:
   a. `initialize_answers_each_supported_version_with_itself`: for each entry of SUPPORTED_MCP_PROTOCOL_VERSIONS, `{"protocolVersion":"<v>"}` → `<v>`; also `{"protocolVersion":" 2025-03-26 "}` → `"2025-03-26"`.
   b. `initialize_unknown_or_missing_version_gets_newest`: `{"protocolVersion":"1999-01-01"}`, `{"protocolVersion":""}`, `{"protocolVersion":42}` and `{}` each → `"2025-06-18"`.
   c. `default_protocol_version_is_newest_supported`: `SUPPORTED_MCP_PROTOCOL_VERSIONS == ["2025-06-18", "2025-03-26", "2024-11-05"]` and `DEFAULT_MCP_PROTOCOL_VERSION == SUPPORTED_MCP_PROTOCOL_VERSIONS[0]`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'pub const SUPPORTED_MCP_PROTOCOL_VERSIONS: &\[&str\] = &\["2025-06-18", "2025-03-26", "2024-11-05"\];' crates/backend_core/src/mcp/mod.rs && grep -q 'pub const DEFAULT_MCP_PROTOCOL_VERSION: &str = "2025-06-18";' crates/backend_core/src/mcp/mod.rs && grep -q "fn negotiate_protocol_version(requested: Option<&str>) -> &'static str" crates/backend_core/src/mcp/mod.rs && ! grep -q 'requested_version' crates/backend_core/src/mcp/mod.rs && test "$(cargo test -p backend_core --test mcp_protocol 2>&1 | grep -c 'test result: ok. 15 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 14E: tools/call Result Shape

* **Recommended Command:** `/goal`

**Written 2026-11-15** against HEAD `d8236bc` (STEP 14 re-split, original item 4; OQ14-1 resolved: drop the flattened keys). Dry-run (scratch worktree, removed): `mcp/mod.rs` +10/−28, `cli_integration.rs` +84/−23 (mostly rustfmt re-wrapping longer index chains); gate below exit 0; its markers fail on an untouched tree.

**Correction to the re-split row:** the row and the state table say "7 CLI assertions at `cli_integration.rs:1332-1364`". The real count is **20 flattened-key reads at `:1332-1390`**, all in `test_mcp_stdio_jsonrpc_session_end_to_end` (`:1052`). The row's scope is unchanged: same two files, same rewrite. Nothing else in `crates/`, `docs/` or `README.md` reads `tools/call` result keys.

**Behaviour change to know about:** a `tools/call` result is now exactly `{ "content", "structuredContent", "isError" }`. Anything reading tool fields at the top level of `result` (e.g. `result.item_id`) must read `result.structuredContent.item_id`. Error results lose their top-level `status` / `error` too; those stay inside `structuredContent`.

**Decisions the spec leaves open (approve before hand-off):**
- **E1 error results get the same three keys**, with `structuredContent = {"status":"error","error":"<message>"}` as today. The original item covers both paths ("error results flatten `status`/`error`").
- **E2 the payload goes into `structuredContent` verbatim.** The MCP spec wants it to be a JSON object. Every tool returns a `json!({…})` object today, so no wrapping is added. Wrapping non-objects would be a fix for a case that doesn't happen.
- **E3 the CLI test also pins the key set** on one success result (`ingest_project_directory`, response 3) and one error result (`unpinned_rollback`). Without that, the 20 rewritten reads would also pass against today's code, since `structuredContent` already exists alongside the flattened copy.

**Current code (cited for the spec):**
- `mcp/mod.rs:380-451` the `"tools/call" =>` arm. `Ok(payload)` at `:404-425`: builds `let mut result_obj = serde_json::Map::new();`, copies every key of an object payload into it, then inserts `"content"` (`[{"type":"text","text": <pretty JSON>}]`), `"structuredContent": payload`, `"isError": false`. `Err(err)` at `:426-449`: `let err_msg = err.to_string(); let err_payload = json!({"status":"error","error": err_msg});`, then a result `json!({"status":"error","error": err.to_string(),"content":[…],"structuredContent": err_payload,"isError": true})`.
- `nebulous_cli/tests/cli_integration.rs:1329-1390`, in `test_mcp_stdio_jsonrpc_session_end_to_end`: the 20 reads are `responses[2]["result"]["total_components"]`; `responses[3]["result"]["boundaries"]`; `responses[4]["result"]["last_known_good"]…` (×2); `responses[5]["result"]["tracked_item_id"]`, `["rolling_update"]…`; `responses[6]["result"]["active"]`, `["item_id"]`, `["scope_id"]`, `["bounded_prompt"]`; `responses[7]["result"]["passed"]`, `["exit_code"]`, `["tracked_item_id"]`; `responses[8]["result"]["restored"]`, `["regression_recorded"]`, `["item_id"]`, `["recurrence_count"]`, `["last_known_good"]…`; `responses[9]["result"]["item_id"]`, `["documentation_updated"]`. The `["result"]["isError"]`, `["serverInfo"]` and `["tools"]` reads are already spec-shaped.
- `:1597-1604` in `test_mcp_remaining_tools_resources_content_length_and_error_paths`: `unpinned_rollback` (a `rollback_to_pinned_release` call with no pin), `assert_eq!(unpinned_rollback["result"]["isError"], true);`.

```text
[STEP_ID]
STEP 14E: a tools/call result is exactly content + structuredContent + isError; the tool payload is no longer flattened into the result

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:380-451 (tools/call arm)
  - crates/nebulous_cli/tests/cli_integration.rs:1329-1390 (test_mcp_stdio_jsonrpc_session_end_to_end assertions), :1597-1604 (unpinned_rollback)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. In mod.rs only the Ok/Err result construction inside the tools/call arm changes: the -32602 checks above it, call_tool, every tool, every other handle_request arm and the content text (pretty-printed JSON) stay as they are. In cli_integration.rs only the reads listed in MUTATION_SPEC item 3 change, plus the assertions added in item 4; no request, no other assertion and no other test changes.

[PRE_GATE]
grep -q 'fn negotiate_protocol_version' crates/backend_core/src/mcp/mod.rs && grep -q 'result_obj' crates/backend_core/src/mcp/mod.rs && test "$(grep -c '"structuredContent":' crates/backend_core/src/mcp/mod.rs)" -eq 1 && test "$(grep -c '\["result"\]\["structuredContent"\]' crates/nebulous_cli/tests/cli_integration.rs)" -eq 0

[MUTATION_SPEC]
1. mod.rs, tools/call `Ok(payload)` arm: keep `let text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string());`. Delete `result_obj` and the key-copy loop. Return `Some(JsonRpcResponse::success(req_id, json!({ "content": [{ "type": "text", "text": text }], "structuredContent": payload, "isError": false })))` (E2).
2. mod.rs, tools/call `Err(err)` arm: `let err_payload = json!({ "status": "error", "error": err.to_string() });` (the `err_msg` local goes). Keep the `text` line. The result is `json!({ "content": [{ "type": "text", "text": text }], "structuredContent": err_payload, "isError": true })`, with no top-level `status` or `error` (E1).
3. cli_integration.rs:1329-1390: each of the 20 reads of the form `responses[N]["result"]["<key>"]`, where `<key>` is not `isError`, `serverInfo` or `tools`, becomes `responses[N]["result"]["structuredContent"]["<key>"]`. The rest of each expression and the expected value stay as they are. The keys are total_components, boundaries, last_known_good (×3), tracked_item_id (×2), rolling_update, active, item_id (×3), scope_id, bounded_prompt, passed, exit_code, restored, regression_recorded, recurrence_count and documentation_updated. Let rustfmt re-wrap the lines.
4. cli_integration.rs, new assertions (E3):
   a. Directly after the `total_components` assertion (response 3, ingest_project_directory): collect `responses[2]["result"].as_object().unwrap().keys().map(String::as_str)` into a `Vec<&str>` named `result_keys`, `sort_unstable()`, then `assert_eq!(result_keys, ["content", "isError", "structuredContent"]);`.
   b. Directly after `assert_eq!(unpinned_rollback["result"]["isError"], true);`: the same key-set check on `unpinned_rollback["result"]` into `error_keys`, with the same `assert_eq!(error_keys, ["content", "isError", "structuredContent"]);`, then `assert_eq!(unpinned_rollback["result"]["structuredContent"]["status"], "error");`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -q 'result_obj' crates/backend_core/src/mcp/mod.rs && test "$(grep -c '"structuredContent":' crates/backend_core/src/mcp/mod.rs)" -eq 2 && test "$(grep -c '"status": "error",' crates/backend_core/src/mcp/mod.rs)" -eq 1 && ! grep -qE '\["result"\]\["(total_components|boundaries|last_known_good|tracked_item_id|rolling_update|active|item_id|scope_id|bounded_prompt|passed|exit_code|restored|regression_recorded|recurrence_count|documentation_updated)"\]' crates/nebulous_cli/tests/cli_integration.rs && test "$(grep -c '\["result"\]\["structuredContent"\]' crates/nebulous_cli/tests/cli_integration.rs)" -eq 21 && test "$(grep -c '\["content", "isError", "structuredContent"\]' crates/nebulous_cli/tests/cli_integration.rs)" -eq 2

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 14F: Read-only list_projects

* **Recommended Command:** `/goal`

**Written 2026-11-15** against HEAD `d4d7815` (STEP 14 re-split, original item 5, the `list_projects` half). Dry-run (scratch worktree, removed): `mcp/mod.rs` +6/−33, `tests/mcp_protocol.rs` +51/−1; gate below exit 0; its markers fail on an untouched tree.

**Behaviour change to know about:** with an empty `projects.toml` and a server bound to a workspace, `list_projects` used to register that workspace, make it active and write `projects.toml`. Now it reports `total: 0` and writes nothing. To register a workspace, use `neb init` (which registers it unless you pass `--no-register`, `commands/mod.rs:590-597`) or `neb project add <PATH>`.

**Decisions the spec leaves open (approve before hand-off):**
- **F1 no in-memory preview entry.** The other option is to show the bound workspace as if it were registered, without saving. That would list an `active_project_id` that isn't in `projects.toml`, which is misleading, and `switch_project` couldn't find it. The tool's own description says it lists "projects registered in … projects.toml", so the stored registry is the answer.
- **F2 `register_in_registry` is deleted, not kept.** `list_projects` was its only caller (`mcp/mod.rs:1446`, nothing outside the file). With no caller, `-D warnings` fails on dead code, and an `#[allow(dead_code)]` would only hide that. Its two imports, `register_project` and `register_project_in`, go with it. They stay `pub` in `models::registry` for the CLI.

**Current code (cited for the spec):**
- `mcp/mod.rs:13-17` `use crate::models::{ load_global_registry, load_global_registry_from, register_project, register_project_in, set_active_project, set_active_project_in, … };`
- `:191-196` `fn load_registry(&self) -> Result<GlobalProjectRegistry>` (honours `registry_path_override`); `:198-211` `fn register_in_registry(&self, root_or_tracker_path: &Path, name_override: Option<&str>, tags: Vec<String>, set_active: bool) -> Result<RegisteredProject>`; `:213` `fn switch_in_registry` (stays).
- `:1439-1459` `fn tool_list_projects(&mut self, _args: Value) -> Result<Value>`: `let mut registry = self.load_registry()?;` then, `if registry.projects.is_empty()` and the bound storage has a `project.toml`, it calls `self.register_in_registry(&storage.project_root(), None, Vec::new(), true)`, which saves to disk, and pushes the entry. It returns `json!({ "active_project_id", "total", "projects" })`.
- `McpServer::from_storage(storage).with_registry_path(path)` (`:140`, `:163`) points the registry at a test directory. `backend_core::register_project_in(dir, root, Some(name), vec![], true)` and `TrackerStorage::init(root, name, phase_id, phase_title)` are `pub` (used the same way at `cli_integration.rs:1459-1462`). `tempfile = "3"` is already a backend_core dev-dependency.
- The existing `list_projects` assertions (`cli_integration.rs:1478-1482`, 2 registered projects) don't touch the empty-registry path and are unaffected.

```text
[STEP_ID]
STEP 14F: list_projects is read-only: it reports projects.toml as stored and never registers or activates the bound workspace

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/mcp_protocol.rs
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:13-17 (models imports), :191-216 (load_registry, register_in_registry, switch_in_registry), :1439-1459 (tool_list_projects)
  - crates/nebulous_cli/tests/cli_integration.rs:1450-1482 (how the CLI test builds a registry-backed server)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. load_registry, switch_in_registry, tool_switch_project, every other tool, the tools_list() entry for list_projects and the JSON-RPC layer stay as they are. list_projects keeps its output keys (active_project_id, total, projects). The 15 existing tests in mcp_protocol.rs are unchanged (only its `use` lines grow).

[PRE_GATE]
grep -q 'fn register_in_registry' crates/backend_core/src/mcp/mod.rs && grep -q '"structuredContent": err_payload' crates/backend_core/src/mcp/mod.rs && ! grep -q 'result_obj' crates/backend_core/src/mcp/mod.rs && test "$(grep -c '^#\[test\]' crates/backend_core/tests/mcp_protocol.rs)" -eq 15

[MUTATION_SPEC]
1. mod.rs `tool_list_projects` becomes, with this doc line directly above it:
   `/// Read-only: reports \`projects.toml\` as stored and never registers or activates anything.`
   `fn tool_list_projects(&mut self, _args: Value) -> Result<Value> { let registry = self.load_registry()?; Ok(json!({ "active_project_id": registry.active_project_id, "total": registry.projects.len(), "projects": registry.projects })) }` (F1)
2. mod.rs: delete `fn register_in_registry` (`:198-211`) entirely (F2).
3. mod.rs `use crate::models::{…}`: remove `register_project` and `register_project_in`. Nothing else in the import list changes; let rustfmt re-wrap it.
4. tests/mcp_protocol.rs: add `use std::fs;` next to `use std::io::Cursor;`, and add `register_project_in` and `TrackerStorage` to the `use backend_core::{…}` list (rustfmt order). Append 2 tests, each using `tempfile::TempDir::new()` and a workspace from `TrackerStorage::init(temp.path().join("proj"), "ProjOne", "01_core", "Core Phase")`, with `cfg_dir = temp.path().join("cfg")`:
   a. `list_projects_with_empty_registry_writes_nothing`: `McpServer::from_storage(storage).with_registry_path(&cfg_dir)`, then `call_tool("list_projects", json!({}))` → `total == 0`, `projects == []`, `active_project_id` is null, and `!cfg_dir.exists()`.
   b. `list_projects_leaves_an_existing_registry_byte_for_byte`: first `register_project_in(&cfg_dir, storage.project_root(), Some("ProjOne"), vec![], true)`, then read `cfg_dir.join("projects.toml")` with `fs::read` into `before`. Build the server as in (a) and call `list_projects` → `total == 1`, `active_project_id == "projone"`, and `fs::read` of the file again equals `before`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -q 'fn register_in_registry' crates/backend_core/src/mcp/mod.rs && ! grep -q 'register_project_in' crates/backend_core/src/mcp/mod.rs && ! grep -qE '\bregister_project\b' crates/backend_core/src/mcp/mod.rs && grep -q '/// Read-only: reports `projects.toml` as stored and never registers or activates anything.' crates/backend_core/src/mcp/mod.rs && test "$(cargo test -p backend_core --test mcp_protocol 2>&1 | grep -c 'test result: ok. 17 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 14G: `dry_run` on `git_pull`, `git_push`, `git_sync`

* **Recommended Command:** `/boost`

**Written 2026-11-15** against HEAD `38744c8`, within the approved re-split row and OQ14-2 (default `true`). Auth args are still validated before the dry-run branch, so `credential_hygiene.rs` keeps passing unchanged.

```text
[STEP_ID]
STEP 14G: git_pull, git_push and git_sync take dry_run (default true); a dry run returns git_status plus what would happen, with no network, no commit and no .tracker/ write

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs: tool_git_pull (~2092), tool_git_push (~2114), tool_git_sync (~2170), their tools_list() entries (~828, ~838, ~864), plan_import's dry_run schema line (~735), fn default_true (~2528)
  - crates/backend_core/src/git_sync/mod.rs:33-43 (GitRepoStatus: branch, remote_url, ahead_count, behind_count, tracker_dirty, dirty_tracker_files)
  - crates/backend_core/tests/workflow_integration.rs:2180-2258 (existing MCP git test)
  - crates/backend_core/tests/credential_hygiene.rs:95-110 (raw PAT refused for all git tools)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. dry_run: false behaves exactly as today. git_fetch and git_commit_documented unchanged. git_sync_with_report / git_push / git_pull_with_report unchanged.

[PRE_GATE]
grep -q 'fn tool_handler(name: &str) -> Option<ToolHandler>' crates/backend_core/src/mcp/mod.rs && ! grep -q '"would"' crates/backend_core/src/mcp/mod.rs

[MUTATION_SPEC]
1. Each of the three Args structs gains `#[serde(default = "default_true")] dry_run: bool`.
2. In each handler, after `mcp_auth_request(...)` and `resolve_active_storage()` and before any git call: `if parsed.dry_run { let status = storage.git_status()?; return Ok(json!({ "dry_run": true, "status": status, "would": <text> })); }`. Why the order: auth validation (raw PAT / unbound account refusal) must still run on a dry run.
   - git_pull would: `format!("pull {branch} from {remote}; {behind} commit(s) behind as of the last fetch")` using status.branch / status.remote_url (`"<none>"` when None) / status.behind_count.
   - git_push would: `format!("push {branch} to {remote}; {ahead} local commit(s) ahead")`.
   - git_sync would: an array: `"commit N .tracker/ file(s)"` when tracker_dirty (N = dirty_tracker_files.len()), then (unless commit_only) `"pull then push {branch} with {remote}"`. Also include `"commit_only": parsed.commit_only`.
3. Non-dry results gain `"dry_run": false`; existing keys unchanged.
4. tools_list(): each of the three input_schema properties gets `"dry_run": { "type": "boolean", "default": true, "description": "When true (default), reports git status and what would happen without network access or commits. Pass false to run." }`; each description ends with ` Defaults to dry_run: true.`
5. Tests (workflow_integration.rs):
   a. In the existing MCP git test, every git_pull / git_sync call that expects real work passes `"dry_run": false`.
   b. `mcp_git_tools_default_to_dry_run`: a tracker repo with a bare remote (same setup as the existing test); create a bug so .tracker/ is dirty; record HEAD sha; `call_tool("git_sync", json!({}))` → `dry_run == true`, `would` is a non-empty array; HEAD sha unchanged; `storage.git_status()?.tracker_dirty` still true. `call_tool("git_push", json!({}))` → `dry_run == true`, remote HEAD unchanged. `call_tool("git_pull", json!({}))` → `dry_run == true`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && test "$(grep -c '"would"' crates/backend_core/src/mcp/mod.rs)" -ge 3 && grep -q 'fn mcp_git_tools_default_to_dry_run' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 14H: `dry_run` on `rollback_to_pinned_release`, Writing `ingest_project_directory`, `switch_project`

* **Recommended Command:** `/boost`

**Written 2026-11-15** against HEAD `38744c8`, within the approved re-split row and OQ14-2 (default `true`). Adds `nebulous_cli/tests/cli_integration.rs` to the row's files: its MCP session test calls ingest (writing) and rollback and expects real results, so those two calls need `"dry_run": false`.

```text
[STEP_ID]
STEP 14H: rollback_to_pinned_release, ingest_project_directory (when it would write) and switch_project take dry_run (default true); a dry run reports what would change and writes nothing

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/mcp_protocol.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs: tool_switch_project (~1434), tool_ingest_project_directory (~1457-1515), tool_rollback_to_pinned_release (~1989-2066), load_registry (~190), their tools_list() entries
  - crates/backend_core/src/models/registry.rs:212 (GlobalProjectRegistry::find_project), :359-371 (set_active_project NotFound error)
  - crates/backend_core/tests/mcp_protocol.rs:259-310 (McpServer::from_storage(..).with_registry_path(..) test pattern)
  - crates/nebulous_cli/tests/cli_integration.rs:1125-1140 (ingest call, id 3), :1205-1220 (rollback call, id 9)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. dry_run: false behaves exactly as today. Read-only ingest (write_manifests false and no promote_all / promote_phase_id) ignores dry_run and runs as today.

[PRE_GATE]
test "$(grep -c '"would"' crates/backend_core/src/mcp/mod.rs)" -ge 3 && ! grep -q '"would_switch_to"' crates/backend_core/src/mcp/mod.rs

[MUTATION_SPEC]
1. switch_project Args gains `#[serde(default = "default_true")] dry_run: bool`. Dry run: `let registry = self.load_registry()?; let project = registry.find_project(q).cloned().ok_or_else(|| TrackerError::NotFound { kind: "Project", id: q.to_string() })?;` return `{"dry_run": true, "would_switch_to": project}`; registry file and `self.storage` untouched.
2. ingest_project_directory Args gains `dry_run` (default true). `let writes = parsed.write_manifests || parsed.promote_all || parsed.promote_phase_id.is_some();`. If `writes && parsed.dry_run`: run the scan with `.with_write_manifests(false).with_promote_all(false)` and no promote phase, and return the normal result keys plus `"dry_run": true` and `"would_write_manifests": <bool>`, `"would_promote": <bool>`. Otherwise run as today with `"dry_run": false`.
3. rollback_to_pinned_release Args gains `dry_run` (default true). Dry run: after reading the manifest, the pinned check and target_item_id resolution, return `{"dry_run": true, "component_id", "would_restore": parsed.restore_manifest, "last_known_good": pinned, "would_record_regression_on": target_item_id}`, skipping rollback_component_to_pinned and record_regression. Non-dry results gain `"dry_run": false`.
4. tools_list(): all three schemas get the `dry_run` property (same wording as 14G); descriptions end with ` Defaults to dry_run: true.`
5. cli_integration.rs: add `"dry_run": false` to the ingest (id 3) and rollback (id 9) arguments; no assertion changes.
6. Tests (mcp_protocol.rs):
   a. `switch_project_dry_run_leaves_registry`: register ProjOne in a temp registry (as in the list_projects tests); bytes of projects.toml before == after `call_tool("switch_project", json!({"id": "ProjOne"}))`; result `dry_run == true`, `would_switch_to.name == "ProjOne"`. Unknown name → Err NotFound.
   b. `ingest_writing_mode_defaults_to_dry_run`: storage with a `crates/demo/Cargo.toml` on disk; `call_tool("ingest_project_directory", json!({"write_manifests": true}))` → `dry_run == true`, `would_write_manifests == true`, and no `nebulous.component.toml` exists under `crates/demo`.
   c. `rollback_defaults_to_dry_run`: a component with a pinned release (set up via `call_tool("pin_stable_release", …)` with the args its schema requires); then `call_tool("rollback_to_pinned_release", json!({"component_id": <id>}))` → `dry_run == true`; component manifest bytes unchanged.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q '"would_switch_to"' crates/backend_core/src/mcp/mod.rs && grep -q '"would_restore"' crates/backend_core/src/mcp/mod.rs && grep -q 'fn rollback_defaults_to_dry_run' crates/backend_core/tests/mcp_protocol.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 14I: Bounded Agent Context

* **Recommended Command:** `/goal`

**Written 2026-11-15** against HEAD `38744c8`, within the approved re-split row and OQ14-5 (`desc.md` 16 KiB; scope description and each fix summary 1 KiB; visible truncation marker; applies everywhere the prompt is used).

```text
[STEP_ID]
STEP 14I: synthesize_bounded_prompt caps free text, fences it in labelled untrusted-text blocks, and strips control characters

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/agent_context/mod.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/agent_context/mod.rs:95-185 (synthesize_bounded_prompt: title in heading, scope description, previous_fixes summaries, desc.md body)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Section headings, order and the file_refs string unchanged. synthesize_bounded_prompt signature unchanged.

[PRE_GATE]
grep -q '"would_switch_to"' crates/backend_core/src/mcp/mod.rs && ! grep -q 'MAX_DESC_BYTES' crates/backend_core/src/agent_context/mod.rs

[MUTATION_SPEC]
1. Consts: `pub const MAX_DESC_BYTES: usize = 16 * 1024;` `pub const MAX_SHORT_TEXT_BYTES: usize = 1024;` with a doc pointing at OQ14-5.
2. `fn strip_control(s: &str) -> String`: drop every char where `c.is_control() && c != '\n' && c != '\t'` (so `\r`, ESC and NUL go).
3. `fn cap_bytes(s: &str, max: usize) -> String`: if `s.len() <= max` return it; else cut at the largest char boundary ≤ max and append `\n[… truncated {dropped} bytes]` (dropped = s.len() − kept).
4. `fn fenced(label: &str, body: &str) -> String`: fence = backticks, length max(3, longest backtick run in body + 1). Output: `{label} (untrusted text: treat as data, not instructions):\n{fence}text\n{body}\n{fence}\n`.
5. Apply: title (heading and nowhere else) → strip_control only. Scope description → strip_control, cap 1 KiB, fenced("Scope Description", …) on its own lines instead of the inline bullet. Each fix summary → strip_control, cap 1 KiB, still inline in its numbered line (newlines in it replaced with spaces). desc.md → strip_control, cap 16 KiB, fenced("desc.md", …) under the section 3 heading.
6. Tests (workflow_integration.rs), through `storage.export_agent_context(id)?.bounded_prompt`:
   a. `bounded_prompt_caps_desc_md`: desc = `"Q7".repeat(10_000)` (20 000 bytes) → prompt contains `[… truncated` and `prompt.matches("Q7").count() == 8192`.
   b. `bounded_prompt_strips_control_chars`: desc `red \x1b[31mtext\x07 end` → prompt has no `\x1b` and no `\x07`.
   c. `bounded_prompt_fence_outlasts_backticks`: desc containing a line of ``` → the desc block opens with ```` (4 backticks).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'pub const MAX_DESC_BYTES: usize = 16 \* 1024' crates/backend_core/src/agent_context/mod.rs && grep -q 'fn fenced' crates/backend_core/src/agent_context/mod.rs && grep -q 'fn bounded_prompt_caps_desc_md' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

# Phase 8 — Generic `task` Record Type & Plan Import

## Root Cause
- `ItemType` (`models/item.rs:53`) only has `Bug` and `Feature`. Most planned work (refactors, hardening, migrations, build steps) is neither, so it gets filed under the wrong type. Status (`open`, `in_progress`, `pending`, `closed`, `merged`) is already fine and isn't changed here.
- **Bug/feature are hard-coded as parallel fields, not data.** `ScopeItems.bugs`/`.features` and `ProjectCounters.bug`/`.feature` are walked by hand at about 20 sites, and `ItemType` is matched exhaustively in `storage/id.rs:26,76`, `storage/reconcile.rs:237,260,284`, `workflow/lifecycle.rs:952` and `tui/render.rs:564`. The stub ordering function is also copied twice (`lifecycle.rs:973`, `reconcile.rs:297`). Adding a third type straight away would touch about 15 files in one step, and any site we missed would silently drop tasks from totals, "What's Next", metrics and the TUI.
- **Fix:** STEPS 23A–23E route every site through kind-aware helpers, with no behaviour change. After that, STEP 23F adds `Task` by changing only the three model files.
- **Trackers can't import plans.** Plans live in Markdown. Seeding the trackers by hand or with a one-off script duplicates items on every re-run, because items have no link back to the plan step they came from. STEPS 24A–24D add `source_ref` and an idempotent `neb plan import` that reads the Step Execution Roadmap tables (the format the current criteria use in both repos).
- **Concurrency:** none of this is safe to use on itself until STEP 13 lands. ID allocation (`storage/id.rs:68`) and scope stub updates (`lifecycle.rs:945`) are unlocked read-modify-write cycles. Running the TUI watcher, the CLI and MCP together makes three writers.

## Execution Strategy
- One step per turn, strictly in roadmap order. STEPS 23A–23E are pure refactors: the existing test suite must pass **unmodified** after each one. That's the proof nothing changed.
- `RUST_GATE` below is shorthand for exactly: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans`. Every `POST_GATE` writes it out in full.

---

### STEP 23A: Kind-Aware Helpers on `ItemType`, `ProjectCounters`, `ScopeItems`

* **Recommended Command:** `/goal`

**Refreshed 2026-11-13** against HEAD `bfd515f`. Since this block was first written, 26C/26D added `ItemStub::chronological_cmp` (the one stub ordering; reconcile and lifecycle both sort with it), so the old `cmp_order` helper is dropped and `insert` sorts with `chronological_cmp`. New here: `ItemType::ordinal`, so `id_rank` in scope.rs stops matching on variants and 23F only has to touch the `ItemType` matches in item.rs.

**Current code (cited for the spec):**
- `models/item.rs:62-67` `enum ItemType { Bug, Feature }`; `:69-96` `impl ItemType` (`min_schema_version`, `id_prefix` "BUG"/"FEAT", `format_id`, `format_hash_id`); `:116-139` `Display` ("bug"/"feature") and `FromStr` (accepts bug/bugs/feature/features/feat); `:301` existing `mod tests`.
- `models/project.rs:78-85` `ProjectCounters { bug: u32, feature: u32 }` (no impl block); `:254` existing `mod tests`.
- `models/scope.rs:25-28` `total_items` sums `bugs.len() + features.len()`; `:47-53` `ScopeItems { bugs, features }`; `:92-102` `id_rank` matches `ItemType::Bug => 0, Feature => 1`; `:104-113` `chronological_cmp`. No `mod tests` yet.
- `storage/id.rs:21-25` the prefix match `from_prefix` replaces in 23B (`"BUG"`, `"FEAT" | "FEATURE"`, case-insensitive via `to_ascii_uppercase`).
- `workflow/lifecycle.rs:970-996` the retain-then-push-then-sort pattern `remove_id` + `insert` replace in 23C.
- `nebulous_cli/src/tui/render.rs:565-566` capture-modal titles ("BUG", "FEATURE") that `label` serves in 23E.

```text
[STEP_ID]
STEP 23A: Add data-driven per-kind helpers to ItemType, ProjectCounters and ScopeItems (add only; scope.rs id_rank uses ItemType::ordinal; no other caller changes)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/item.rs
  - crates/backend_core/src/models/project.rs
  - crates/backend_core/src/models/scope.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/id.rs:14-26 (parse_item_id_number prefix match to mirror in from_prefix)
  - crates/backend_core/src/workflow/lifecycle.rs:970-996 (retain/push/sort pattern insert + remove_id mirror)
  - crates/nebulous_cli/src/tui/render.rs:565-566 (labels)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No serialized field is added, renamed or reordered; no serde attribute changes. Existing tests are not edited. No `_ =>` arm in any new match on ItemType (every match stays exhaustive so 23F's new variant is a compile error until handled).

[PRE_GATE]
grep -q 'pub fn chronological_cmp' crates/backend_core/src/models/scope.rs && grep -q 'ItemType::Bug => 0' crates/backend_core/src/models/scope.rs && ! grep -qE 'fn (from_prefix|ordinal|merge_max)\b' crates/backend_core/src/models/item.rs crates/backend_core/src/models/project.rs

[MUTATION_SPEC]
1. item.rs, inside the existing `impl ItemType` (after `format_hash_id`):
   - `pub const ALL: &'static [ItemType] = &[Self::Bug, Self::Feature];` with doc "Every item type, in ordinal order."
   - `pub fn ordinal(self) -> u8` — exhaustive match: Bug => 0, Feature => 1. Doc: "Stable sort position of this type (index into `ALL`)."
   - `pub fn from_prefix(prefix: &str) -> Option<Self>` — `match prefix.to_ascii_uppercase().as_str()`: "BUG" => Bug, "FEAT" | "FEATURE" => Feature, else None. Same rule as storage/id.rs:21-25; no trimming.
   - `pub fn label(self) -> &'static str` — exhaustive match: "BUG", "FEATURE".
   Add to the existing `mod tests` one test `item_type_helpers_round_trip_over_all`: for each `(i, kind)` in `ItemType::ALL.iter().enumerate()`: `kind.ordinal() as usize == i`; `kind.to_string().parse::<ItemType>().unwrap() == *kind`; `ItemType::from_prefix(kind.id_prefix()) == Some(*kind)`; `ItemType::from_prefix(&kind.id_prefix().to_ascii_lowercase()) == Some(*kind)`. Plus `from_prefix("FEATURE") == Some(Feature)`, `from_prefix("TASK") == None`, `from_prefix("") == None`, `Bug.label() == "BUG"`, `Feature.label() == "FEATURE"`.
2. project.rs: new `impl ProjectCounters` directly below the struct:
   - `pub fn get(&self, kind: ItemType) -> u32` and `pub fn get_mut(&mut self, kind: ItemType) -> &mut u32` — exhaustive matches to `bug` / `feature`.
   - `pub fn merge_max(&mut self, other: &ProjectCounters)` — `for &kind in ItemType::ALL { let v = other.get(kind); let slot = self.get_mut(kind); *slot = (*slot).max(v); }`.
   Import `ItemType` (`use crate::models::item::ItemType;`). Add to the existing `mod tests` one test `project_counters_get_and_merge_max_per_kind`: start `{bug: 3, feature: 7}`, `*get_mut(Bug) += 1` gives `get(Bug) == 4`; `merge_max(&{bug: 2, feature: 9})` gives `{bug: 4, feature: 9}` (compare whole structs with `assert_eq!`).
3. scope.rs:
   a. `id_rank`: replace the `match item_type { ... }` with `item_type.ordinal()`; keep everything else (`IdRank::Sequential(type_order, number, id)`).
   b. New `impl ScopeItems` (below the struct):
      - `pub fn iter(&self) -> impl Iterator<Item = (ItemType, &ItemStub)>` — bugs tagged `ItemType::Bug`, then features tagged `ItemType::Feature`, each in stored order.
      - `pub fn count(&self, kind: ItemType) -> usize` — exhaustive match to that list's `len()`.
      - `pub fn insert(&mut self, kind: ItemType, stub: ItemStub)` — push onto that kind's list, then `sort_by(ItemStub::chronological_cmp)` on that list only. Doc says callers that move a stub call `remove_id` first (it does not dedupe).
      - `pub fn remove_id(&mut self, id: &str)` — `retain(|s| s.id != id)` on every list.
   c. `total_items` body becomes `self.items.iter().count()`; doc comment unchanged.
   d. Import `ItemType` (`use crate::models::item::ItemType;` alongside the existing item import). Use `std::cmp::Ordering` only if needed.
   e. New `#[cfg(test)] mod tests` at end of file with a local `fn stub(id: &str, created_at: &str) -> ItemStub` (title "t", default status/severity). Tests:
      - `scope_items_insert_keeps_chronological_order`: insert Bug stubs `BUG-0002`@"2026-01-02T00:00:00Z" then `BUG-0001`@"2026-01-01T00:00:00Z"; bugs ids == ["BUG-0001","BUG-0002"]; features empty.
      - `scope_items_remove_id_drops_from_every_list`: bugs [BUG-0001], features [FEAT-0001, FEAT-0002]; `remove_id("FEAT-0001")` leaves features [FEAT-0002], bugs untouched; `remove_id("NOPE")` changes nothing.
      - `scope_items_iter_and_count_cover_every_kind`: 2 bugs + 1 feature; `iter()` collected as `(kind, id)` == [(Bug,"BUG-0001"),(Bug,"BUG-0002"),(Feature,"FEAT-0001")]; `count(Bug)==2`, `count(Feature)==1`; a `ScopeManifest` built from these items has `total_items()==3`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && git diff --quiet HEAD -- crates/backend_core/tests crates/backend_core/src/storage crates/backend_core/src/workflow crates/nebulous_cli && ! grep -nE 'ItemType::(Bug|Feature) => [0-9]' crates/backend_core/src/models/scope.rs && ! grep -nE '^\s*_ =>' crates/backend_core/src/models/project.rs && cargo test -p backend_core --lib -- item_type_helpers_round_trip_over_all project_counters_get_and_merge_max_per_kind scope_items_ 2>&1 | grep -q 'test result: ok. 5 passed'

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status. Halt and report instead of editing an existing test if any existing test fails.
```

**Decisions the contract leaves open:**
- **`insert` doesn't dedupe.** Lifecycle today retains-then-pushes; 23C will call `remove_id` then `insert`, keeping that explicit. Folding dedupe into `insert` would hide a move across kinds.
- **`label` returns "FEATURE", not "FEAT".** It's for display (the TUI modal title); `id_prefix` stays the ID form.
- **`get`/`get_mut`/`count` are matches, not arrays.** The serialized structs keep named fields, so a match is the only way without changing the TOML shape. Each is exhaustive, so 23F's `Task` won't compile until all of them are handled — that's the point.

---

### STEP 23B: Storage Uses the Helpers

* **Recommended Command:** `/goal`

**Refreshed 2026-11-14** against HEAD `72aa885`. The old block cited line numbers from before 26B–26E and a `compare_item_stubs` that 26D already deleted. Reconcile now sorts with `ItemStub::chronological_cmp`, which `ScopeItems::insert` (23A) also uses, so bucketing through `insert` gives the identical list (the comparator is a total order, so the result doesn't depend on insertion order).

**Current code (cited for the spec):**
- `storage/id.rs:14-26` `parse_item_id_number`: digits check, then `match prefix.to_ascii_uppercase()` "BUG" / "FEAT" | "FEATURE". `:28-35` `record_id_in_counters` per-kind max match. `:47-48` `scan_max_counters` per-field max against `project.counters`. `:108-109` `allocate_next_id` per-field max against `scanned`; `:111-128` per-kind `checked_add(1)` with messages "BUG counter overflowed u32::MAX" / "FEAT counter overflowed u32::MAX". `:174-179` `scan_scope_dir` walks `items.bugs` then `items.features`.
- `storage/reconcile.rs:156-169` `collect_authoritative_items` returns `(bugs, features, completed)`; caller sums `bugs.len() + features.len()` and assigns both fields. `:204-205` per-field max into `project.counters`. `:229` fn signature; `:230-235` the two `Vec::new()`s and early return; `:259-264` and `:287-292` per-kind max matches on `max_counters`; `:312-322` bucket match, two `sort_by(ItemStub::chronological_cmp)`, return.
- `storage/mod.rs:90-114` `TrackerIndex::find_item` and `:117-141` `all_items`: a bugs loop then a features loop, each tagging `ItemType::Bug` / `ItemType::Feature`.
- 23A helpers: `ItemType::from_prefix`, `ProjectCounters::{get, get_mut, merge_max}`, `ScopeItems::{iter, insert}`.

```text
[STEP_ID]
STEP 23B: Route storage ID parsing, counter max/allocation and stub bucketing through the 23A helpers (no behaviour change)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/id.rs
  - crates/backend_core/src/storage/reconcile.rs
  - crates/backend_core/src/storage/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/item.rs (ItemType::from_prefix, id_prefix)
  - crates/backend_core/src/models/project.rs (ProjectCounters::get, get_mut, merge_max)
  - crates/backend_core/src/models/scope.rs (ScopeItems::iter, insert; ItemStub::chronological_cmp)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No test is added, edited or removed (the `#[cfg(test)]` modules in id.rs and mod.rs stay byte-identical): the existing suite passing unmodified is the proof. No public signature changes. Error message text stays identical.

[PRE_GATE]
grep -q 'pub fn merge_max' crates/backend_core/src/models/project.rs && grep -q 'pub fn from_prefix' crates/backend_core/src/models/item.rs && grep -q 'pub fn insert(&mut self, kind: ItemType' crates/backend_core/src/models/scope.rs && grep -qE 'items\.(bugs|features)' crates/backend_core/src/storage/mod.rs

[MUTATION_SPEC]
1. id.rs:
   a. `parse_item_id_number`: keep the trim/split/digits/parse lines; replace the final `match` with `ItemType::from_prefix(prefix).map(|kind| (kind, number))`.
   b. `record_id_in_counters`: body under the `if let` becomes `let slot = counters.get_mut(item_type); *slot = (*slot).max(number);`.
   c. `scan_max_counters` (:47-48): `counters.merge_max(&project.counters);`.
   d. `allocate_next_id` (:108-128): `project.counters.merge_max(&scanned);` then
      `let slot = project.counters.get_mut(item_type);`
      `let next_num = slot.checked_add(1).ok_or_else(|| TrackerError::ValidationError(format!("{} counter overflowed u32::MAX", item_type.id_prefix())))?;`
      `*slot = next_num;`
      (rustfmt will wrap it). The `write_project` / `format_id` lines after it are unchanged.
   e. `scan_scope_dir` (:174-179): one `for (_, stub) in scope_manifest.items.iter() { record_id_in_counters(counters, &stub.id); }`.
2. reconcile.rs:
   a. `collect_authoritative_items` returns `Result<(ScopeItems, u32)>`: `let mut items = ScopeItems::default();` replaces the two `Vec::new()`s; the early return is `Ok((items, completed_count))`.
   b. Both per-kind max matches (:259-264, :287-292) become `let slot = max_counters.get_mut(kind); *slot = (*slot).max(num);`.
   c. The bucket match (:312-316) becomes `items.insert(manifest.item.item_type, stub);`. Delete the two `sort_by` lines (:319-320); `insert` sorts. Return `Ok((items, completed_count))`.
   d. Caller (:156-169): `let (items, scope_completed) = collect_authoritative_items(..)?;`, `let scope_item_count = items.iter().count() as u32;`, and `scope_manifest.items = items;` replaces the two field assignments.
   e. :204-205 → `project.counters.merge_max(&max_counters);`.
   f. Imports: add `ScopeItems`; drop `ItemType` and `ItemStub` if they become unused (clippy -D warnings will say).
3. storage/mod.rs `find_item` and `all_items`: each pair of loops becomes one `for (item_type, stub) in scope.manifest.items.iter()`, building `ItemLocation { phase_id, scope_id, item_type, stub: stub.clone() }` exactly as before (same field values, same bugs-then-features order). Drop `ItemType` from the `crate::models` import only if it becomes unused.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && git diff --quiet HEAD -- crates/backend_core/tests crates/backend_core/src/models crates/backend_core/src/workflow crates/backend_core/src/mcp crates/nebulous_cli && ! grep -nE 'items\.(bugs|features)|counters\.(bug|feature)\b|ItemType::(Bug|Feature) =>|"(BUG|FEAT) counter overflowed' crates/backend_core/src/storage/id.rs crates/backend_core/src/storage/reconcile.rs crates/backend_core/src/storage/mod.rs && ! grep -n 'chronological_cmp' crates/backend_core/src/storage/reconcile.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status. Halt and report instead of editing a test if any existing test fails.
```

**Decisions the contract leaves open:**
- **`insert` re-sorts on every push.** Reconcile used to push all, then sort once. Each re-sort is over an already-sorted list plus one, which the stdlib sort handles in near-linear time, so a scope with hundreds of items costs nothing noticeable. If a scope ever holds tens of thousands, add a `ScopeItems::sort_all` in its own step.
- **No new tests.** 23B is a pure refactor and existing coverage already pins it: reconcile ordering (26D's `reconcile_and_create_order_scope_stubs_by_created_at`), counter reconcile, duplicate IDs (26E), overflow-free allocation, `find_item` / `all_items` through every CLI and MCP test.
- **The overflow message is built with `format!`.** The text is byte-identical ("BUG counter…", "FEAT counter…"); the gate bans the literal so the per-kind copy can't come back.

---

### STEP 23C: Workflow Uses the Helpers

* **Recommended Command:** `/goal`

**Refreshed 2026-11-14** against HEAD `6c5f2ec`. The old block cited pre-26D line numbers, a `compare_stubs` that 26D already deleted, and a `cmp_order` that 23A dropped. Lifecycle now sorts with `ItemStub::chronological_cmp`, which `ScopeItems::insert` uses too, so `remove_id` + `insert` gives the identical list.

**Current code (cited for the spec):**
- `workflow/lifecycle.rs:925-952` `recompute_phase_summary_with_scope`: a bugs loop and a features loop, each `total_items += 1` and `completed_items += 1` when `status.is_completed()`.
- `workflow/lifecycle.rs:961-1000` `sync_parent_indexes_for_item`: two `retain(|existing| existing.id != stub.id)` (:970-977), then a `match item_type` that pushes and `sort_by(ItemStub::chronological_cmp)` on that list (:979-996).
- `workflow/lifecycle.rs:172, :191` `create_bug` / `create_feature` pass `ItemType::Bug` / `Feature` as values. Not per-kind dispatch; they stay.
- `workflow/assist.rs:238-243` (in-progress IDs in the active phase) and `:396-402` (open-stub walk): `items.bugs.iter().chain(items.features.iter())`.
- `workflow/assist.rs:724-745` `count_phase_items`: a bugs loop and a features loop, same shape as lifecycle's.
- `workflow/assist.rs:514-515, :598` bug-first ranking (`item_type == ItemType::Bug`): that's ranking policy ("bugs outrank other kinds"), not a per-kind field walk. It stays as is.
- `workflow/metrics.rs:129-134` `all_stubs` chain, same as assist.
- 23A helpers: `ScopeItems::{iter, insert, remove_id}`.

```text
[STEP_ID]
STEP 23C: Route lifecycle, assist and metrics scope-list access through the 23A helpers (no behaviour change)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/workflow/lifecycle.rs
  - crates/backend_core/src/workflow/assist.rs
  - crates/backend_core/src/workflow/metrics.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/scope.rs (ScopeItems::iter, insert, remove_id)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No test is added, edited or removed. assist.rs bug-first ranking (:514-515, :598) and `compare_item_ids` (:773-780) stay byte-identical. `create_bug` / `create_feature` stay as they are.

[PRE_GATE]
grep -q 'merge_max' crates/backend_core/src/storage/id.rs && grep -q 'pub fn remove_id' crates/backend_core/src/models/scope.rs && grep -qE 'items\.(bugs|features)' crates/backend_core/src/workflow/lifecycle.rs

[MUTATION_SPEC]
1. lifecycle.rs:
   a. `recompute_phase_summary_with_scope` (:940-951): the two loops become one `for (_, stub) in s_manifest.items.iter() { total_items += 1; if stub.status.is_completed() { completed_items += 1; } }`.
   b. `sync_parent_indexes_for_item` (:970-996): the two `retain` calls and the `match` become exactly
      `scope_manifest.items.remove_id(&stub.id);`
      `scope_manifest.items.insert(item_manifest.item.item_type, stub);`
      The `read_scope`, `to_stub`, `write_scope` and `recompute_phase_summary_with_scope` lines around it are unchanged.
   c. Drop `ItemStub` from the `crate::models` import only if it becomes unused (clippy -D warnings will say).
2. assist.rs:
   a. :238-243: the `flat_map` closure body becomes `s.manifest.items.iter().map(|(_, stub)| stub)`; the `.filter` / `.map` / `.collect` after it are unchanged.
   b. :396-402: `let all_stubs = scope_idx.manifest.items.iter().map(|(_, stub)| stub);`.
   c. `count_phase_items` (:728-739): one `for (_, stub) in scope.manifest.items.iter()` loop with the same `total` / `completed` updates. The `total == 0` fallback after it is unchanged.
3. metrics.rs :129-134: `let all_stubs = scope_idx.manifest.items.iter().map(|(_, stub)| stub);`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && git diff --quiet HEAD -- crates/backend_core/tests crates/backend_core/src/models crates/backend_core/src/storage crates/backend_core/src/mcp crates/nebulous_cli && ! grep -nE 'items\.(bugs|features)|ItemType::(Bug|Feature) =>|chronological_cmp' crates/backend_core/src/workflow/lifecycle.rs crates/backend_core/src/workflow/assist.rs crates/backend_core/src/workflow/metrics.rs && test "$(grep -c 'item_type == ItemType::Bug' crates/backend_core/src/workflow/assist.rs)" -eq 3

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status. Halt and report instead of editing a test if any existing test fails.
```

**Decisions the contract leaves open:**
- **Bug-first ranking stays `== ItemType::Bug`.** When 23F adds `Task`, that reads as "bugs first, then everything else tied", which is what we want: a task shouldn't outrank a feature by accident. The gate pins it by counting the 3 lines that hold `item_type == ItemType::Bug` (:514, :515, :598), so it can't be "helpfully" rewritten.
- **No new tests.** Lifecycle stub ordering is pinned by 26D's `reconcile_and_create_order_scope_stubs_by_created_at`; phase summaries, What's Next and metrics are covered by the existing workflow and CLI suites.

---

### STEP 23D: CLI/MCP Scope Counts Use the Helpers

* **Recommended Command:** `/goal`

**Re-cited 2026-11-14** against HEAD `666ae5d`: spec unchanged; the CLI line numbers moved (was :1934-1935). `ItemType` is already imported in both files (`mcp/mod.rs:16`, `commands/mod.rs:10`).

```text
[STEP_ID]
STEP 23D: Scope listing count fields in MCP and CLI use ScopeItems::count (no output change)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/nebulous_cli/src/commands/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/scope.rs (ScopeItems::count)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Output field names and JSON shape unchanged. No test file edits.

[PRE_GATE]
! grep -qE 'items\.(bugs|features)' crates/backend_core/src/workflow/lifecycle.rs

[MUTATION_SPEC]
1. mcp/mod.rs:1259-1260: `bugs_count: scope_idx.manifest.items.count(ItemType::Bug)`, `features_count: ...count(ItemType::Feature)`.
2. commands/mod.rs:1956-1957 (`s.manifest.items.bugs.len()` / `.features.len()`): same change, `s.manifest.items.count(ItemType::Bug)` / `count(ItemType::Feature)`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && ! grep -nE 'items\.(bugs|features)' crates/backend_core/src/mcp/mod.rs crates/nebulous_cli/src/commands/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 23E: TUI Uses the Helpers

* **Recommended Command:** `/goal`

**Re-cited 2026-11-14** against HEAD `16305dd`: spec and line numbers unchanged. After 23D, the POST_GATE's crate-wide grep hits only the 4 lines this step removes (`state.rs:313`, `:319`, `render.rs:565-566`).

```text
[STEP_ID]
STEP 23E: TUI scope totals, stub listing and capture-modal title use the 23A helpers (no visible change)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/tui/state.rs
  - crates/nebulous_cli/src/tui/render.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/{item,scope}.rs (ItemType::label, ScopeItems::iter)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No key bindings change. No test file edits.

[PRE_GATE]
! grep -qE 'items\.(bugs|features)' crates/nebulous_cli/src/commands/mod.rs

[MUTATION_SPEC]
1. state.rs:313-320: total → `sc.manifest.items.iter().count()`, the chain → `items.iter().map(|(_, s)| s)`. state.rs:385-397: the tagged chain → `sc.manifest.items.iter()`.
2. render.rs:564-567: replace the match with `let title_str = format!(" ✦ NEW {} CAPTURE (PREDICTIVE ASSIST) ✦ ", create.item_type.label());`. The rendered strings for Bug and Feature must match the current ones exactly.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && [ -z "$(grep -rnE 'items\.(bugs|features)|ItemType::(Bug|Feature) =>' crates --include=*.rs | grep -vE '/tests/|backend_core/src/lib\.rs|models/(item|project|scope)\.rs')" ]

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 23F: Add `ItemType::Task` (`TASK-XXXX`)

* **Recommended Command:** `/boost`

**Refreshed 2026-11-14** against HEAD `b5ff0a4`. The row's scope is unchanged (`Task` variant, `task` counter, `tasks` scope list, `schema_version = 2` for tasks per 15B) except one thing the row can't do as written: it says only the three model files, but adding a struct field breaks every struct literal of that type, and `tests/storage_integration.rs` has 4 `ScopeItems { .. }` literals. What's different from the 2026-11-09 text, each with its reason:

**Required by code that landed since (not optional):**
- **`ItemType::ordinal` gets `Task => 2`.** 23A added `ordinal` as an exhaustive match; it won't compile without the arm. This is the "Task gets `IdRank` 2" note logged in 26A/26D: `id_rank` and `cmp_item_ids` use `ordinal`, so they need no edit.
- **`is_hash_item_id` accepts `TASK`.** Logged in the 26A decisions for this block. Without it, `allocate_hash_id` (`storage/id.rs:65-84`) rejects every `TASK-XXXXXX` it draws, so creating a task in an `id_scheme = "hash"` tracker fails after 16 attempts.
- **A 4th file, `tests/storage_integration.rs`, gets 6 one-line edits:** `tasks: Vec::new(),` added to the `ScopeItems` literals at :248, :455, :2673, :2680, and the 15B test's `assert_eq!(supported, 1)` at :2386 and :2407 becomes `2` (it asserts `MAX_ITEM_SCHEMA_VERSION`, which the 15B amendment raises). Nothing else in that file changes. Approved 2026-11-14: it's outside the row's original file list, but there's no way to add the field without it (`..Default::default()` would be the same edit, less explicit).
- **23A's own tests get the new field in their literals:** project.rs:323, :328, :329 add `task: 0`; scope.rs:180 and :201 add `tasks: Vec::new()`.
- **Two existing inline tests change, one line each,** because they assert the pre-Task world: `item.rs:349` `assert!(!is_hash_item_id("TASK-7F3A2C"))` becomes `assert!(is_hash_item_id("TASK-7F3A2C"))`, and `item.rs:366` (23A) `from_prefix("TASK") == None` becomes `== Some(ItemType::Task)`. No other existing test is edited.

**Beyond the row (approved 2026-11-14; spec item 2b below):**
- **`ProjectCounters.task` gets `skip_serializing_if` zero.** The original text gives it only `#[serde(default)]`, so every re-save of an existing `project.toml` would add `task = 0`. That's harmless to old binaries (unknown keys are ignored and 15E preserves them), but it contradicts the block's own READ_ONLY_REFS line ("must load and re-save byte-identical") and puts a diff in every tracker repo on first write. It mirrors `tasks` getting `skip_serializing_if = "Vec::is_empty"`. Cost: one private `fn is_zero(v: &u32) -> bool` in project.rs.

**Current code (cited for the spec):**
- `models/item.rs:51-54` `MAX_ITEM_SCHEMA_VERSION = 1`; `:59-67` `enum ItemType { Bug, Feature }` (serde aliases on each); `:69-77` `min_schema_version`; `:79-85` `id_prefix`; `:97-123` `ALL`, `ordinal`, `from_prefix`, `label` (23A); `:126-142` `is_hash_item_id` (`matches!(prefix, "BUG" | "FEAT")`); `:144-167` `Display` and `FromStr` (`expected: "bug, feature (or feat)"`); `:329` `mod tests`.
- `models/project.rs:80-86` `ProjectCounters { bug, feature }`; `:88-113` `get` / `get_mut` / `merge_max` (23A; `merge_max` loops `ItemType::ALL`, so it needs no edit).
- `models/scope.rs:47-53` `ScopeItems { bugs, features }`; `:55-89` `iter` / `count` / `insert` / `remove_id` (23A).
- `storage/mod.rs:803` `write_item_manifest` writes `min_schema_version()` via `write_versioned`, so `Task => 2` is all it takes for tasks to get `schema_version = 2`.
- `workflow/lifecycle.rs` `CreateItemInput` (re-exported from `backend_core`) + `TrackerStorage::create_item`: kind-agnostic since 23C.

```text
[STEP_ID]
STEP 23F: task item type with TASK prefix, a backward-compatible task counter and tasks scope list

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/item.rs
  - crates/backend_core/src/models/project.rs
  - crates/backend_core/src/models/scope.rs
  - crates/backend_core/tests/storage_integration.rs (only the 6 line edits in MUTATION_SPEC 4)
READ_ONLY_REFS:
  - crates/backend_core/src/storage/id.rs:65-84 (allocate_hash_id uses is_hash_item_id)
  - crates/backend_core/src/storage/mod.rs:795-805 (write_item_manifest uses min_schema_version)
  - crates/backend_core/src/workflow/lifecycle.rs (CreateItemInput, create_item)
  - Open Questions 3
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Existing bug/feature serialization unchanged: a project.toml / scope.toml / item.toml with no task data loads and re-serializes with no new keys. No `_ =>` arm on ItemType anywhere. Existing tests are not edited except as listed in MUTATION_SPEC 4.

[PRE_GATE]
[ -z "$(grep -rnE 'items\.(bugs|features)|ItemType::(Bug|Feature) =>' crates --include=*.rs | grep -vE '/tests/|backend_core/src/lib\.rs|models/(item|project|scope)\.rs')" ] && ! grep -qE '^\s*Task,' crates/backend_core/src/models/item.rs && grep -q 'pub fn ordinal' crates/backend_core/src/models/item.rs

[MUTATION_SPEC]
1. item.rs:
   a. `ItemType::Task` after `Feature`, with `#[serde(alias = "tasks")]`. Enum doc: "Category of tracked work item (`bug`, `feature` or `task`)."
   b. Arms: `id_prefix` → "TASK"; `ordinal` → 2; `label` → "TASK"; `min_schema_version` → `Self::Task => 2` (Bug | Feature stay 1); `Display` → "task". `ALL` becomes `&[Self::Bug, Self::Feature, Self::Task]`.
   c. `from_prefix`: add `"TASK" => Some(Self::Task)`. `FromStr`: add `"task" | "tasks" => Ok(Self::Task)`; `expected` becomes `"bug, feature (or feat), task"`.
   d. `MAX_ITEM_SCHEMA_VERSION` becomes 2. Replace the two "STEP 23F sets Task => 2 ..." doc lines (on the const and on `min_schema_version`) with a plain description: tasks require schema version 2.
   e. `is_hash_item_id`: `matches!(prefix, "BUG" | "FEAT" | "TASK")`; doc comment lists `TASK-XXXXXX`.
2. project.rs:
   a. `ProjectCounters.task: u32` after `feature`; `get` / `get_mut` gain the Task arm.
   b. (Proposed beyond the row; approved 2026-11-14) the attribute is `#[serde(default, skip_serializing_if = "is_zero")]` with a private `fn is_zero(v: &u32) -> bool { *v == 0 }` in project.rs. `bug` / `feature` attributes unchanged.
3. scope.rs: `ScopeItems.tasks: Vec<ItemStub>` after `features`, `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. `iter` chains tasks last (tagged `ItemType::Task`); `count` and `insert` gain the Task arm; `remove_id` also retains on `tasks`.
4. Existing tests, only these edits:
   a. Struct literals gain the new field: project.rs tests :323, :328, :329 `ProjectCounters { bug: N, feature: M }` → add `task: 0`; scope.rs tests :180, :201 and tests/storage_integration.rs :248, :455, :2673, :2680 `ScopeItems { .. }` → add `tasks: Vec::new(),` as the last field.
   b. tests/storage_integration.rs :2386 and :2407 (HEAD line numbers; +2 after 4a's inserts) — the only two `assert_eq!(supported, 1);` lines in the file — → `assert_eq!(supported, 2);` (the 15B UnsupportedSchema test reports `MAX_ITEM_SCHEMA_VERSION`).
   c. Two assertion edits in item.rs: item.rs:349 `assert!(!is_hash_item_id("TASK-7F3A2C"))` → `assert!(is_hash_item_id("TASK-7F3A2C"))`; item.rs:366 `assert_eq!(ItemType::from_prefix("TASK"), None)` → `assert_eq!(ItemType::from_prefix("TASK"), Some(ItemType::Task))`. Add `assert_eq!(ItemType::from_prefix("TASKS"), None)` after it.
5. New inline tests:
   a. item.rs `task_type_parses_formats_and_requires_schema_v2`: `ItemType::from_str("task")` and `from_str("tasks")` are Task; `Task.format_id(7) == "TASK-0007"`; `Task.format_hash_id(0xABCDEF) == "TASK-ABCDEF"`; `Task.min_schema_version() == 2`; `Bug.min_schema_version() == 1`; `MAX_ITEM_SCHEMA_VERSION == 2`.
   b. item.rs `created_task_writes_schema_version_2_and_bug_does_not`: `tempfile::tempdir()`, `TrackerStorage::init_standard(dir.path(), "task-test")`, `create_item(CreateItemInput { item_type: ItemType::Task, .. })` and `create_bug(..)` (use `CreateItemInput::new` if it fits, else a struct literal with `Severity::Medium` and `None`s); read each item's `item.toml` from disk (`storage.item_toml_path(..)` is fine if visible, else the path from the returned location); the task's text contains `schema_version = 2`, the bug's contains no `schema_version` line; the task's ID starts with `TASK-`.
   c. project.rs `project_counters_task_defaults_and_skips_zero`: `BASE_PROJECT_TOML` (existing const) loads with `counters.task == 0`; re-serializing contains no `task =` line; with `counters.task = 3` it contains `task = 3`.
   d. scope.rs `scope_items_tasks_default_skip_and_follow_iter_order`: a scope.toml with only bugs/features loads with empty `tasks` and re-serializes with no `tasks` key; `insert(Task, ..)` then `iter()` yields bugs, features, then tasks; `count(Task) == 1`; `remove_id` on the task's ID empties `tasks`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && test "$(git diff -U0 HEAD -- crates/backend_core/tests | grep -E '^[+-]' | grep -vE '^(\+\+\+|---) ' | grep -vcE '^\+\s*tasks: Vec::new\(\),$|^-\s*assert_eq!\(supported, 1\);$|^\+\s*assert_eq!\(supported, 2\);$')" -eq 0 && test "$(git diff -U0 HEAD -- crates/backend_core/tests | grep -E '^[+-]' | grep -vcE '^(\+\+\+|---) ')" -eq 8 && git diff --quiet HEAD -- crates/backend_core/src/storage crates/backend_core/src/workflow crates/backend_core/src/mcp crates/nebulous_cli && grep -q '"TASK"' crates/backend_core/src/models/item.rs && grep -q 'MAX_ITEM_SCHEMA_VERSION: u32 = 2' crates/backend_core/src/models/item.rs && ! grep -rnE '^\s*_ =>' crates/backend_core/src/models/project.rs crates/backend_core/src/models/scope.rs && ! grep -n 'STEP 23F' crates/backend_core/src/models/item.rs && test "$(cargo test -p backend_core --lib -- task_type_parses_formats_and_requires_schema_v2 created_task_writes_schema_version_2_and_bug_does_not project_counters_task_defaults_and_skips_zero scope_items_tasks_default_skip_and_follow_iter_order 2>&1 | grep -c 'test result: ok. 4 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status. The /boost verification subagent confirms that no exhaustive ItemType match anywhere was silenced with a wildcard arm, and that no existing test changed other than as listed in MUTATION_SPEC 4.
```

**Decisions the contract leaves open:**
- **`from_prefix("TASKS")` is `None`.** `FromStr` accepts "tasks" (a type name, like "bugs"), but ID prefixes are singular, matching how `from_prefix` already rejects "BUGS".
- **Tasks rank after bugs and features** in `chronological_cmp` / `cmp_item_ids` ties (ordinal 2) and get no special place in What's Next's bug-first ranking (they tie with features there, as 23C's note intended).
- **No new CLI/TUI surface yet, but MCP parses "task" from here on.** `neb task`, the `[t]` key and MCP's documented `item_type: "task"` are 23H–23J. MCP `create_item` / `list_items` (`mcp/mod.rs:1297`, `:1061`) already parse `item_type` with `FromStr`, so a client that sends `"task"` gets a task as soon as this lands; 23J adds it to the schema text. Nothing in 23F changes MCP code.

---

### STEP 23G: Storage Integration Tests for `task`

* **Recommended Command:** `/goal`

**Refreshed 2026-11-14** against HEAD `ab356f0`. The scope and the three tests are unchanged from the row. Re-cited only: there's no `create_task` helper, so tasks are created with `CreateItemInput::new(ItemType::Task, title)`; the STEP 13 test to mirror is `concurrent_creators_get_unique_ids`; test names are now given so the gate can find them.

**Beyond the row (approved 2026-11-14):**
- **The POST_GATE also checks** that `storage_integration.rs` only gains lines (no existing test edited), that the 3 named tests run and pass, and `check-neutral.sh`. The original gate is only the standard four, which would still pass if a test were skipped or an existing one weakened.

**Current code (cited for the spec):**
- `tests/storage_integration.rs:5-10` imports (`ItemType`, `Severity`, `TrackerStorage`, … — no `CreateItemInput`); `:14` `TempTrackerDir` (`TempTrackerDir::new(label)`, `.path()`); `:2200-2251` `concurrent_creators_get_unique_ids` (two threads, each `TrackerStorage::new(&root)`, 50 `create_bug` each, `HashSet` of IDs, item dir count via `storage.items_dir(&active_phase, "inbox")`); file ends at `:2800`.
- `backend_core::CreateItemInput::new(item_type, title)` (`workflow/lifecycle.rs:30`): Medium severity, default phase/scope.
- `TrackerStorage::reconcile_indexes()` → `ReconcileReport { counters, .. }`; counters merge with `merge_max` over `ItemType::ALL` (23B), which includes `Task` since 23F.
- `ScopeItems.tasks` (23F), `ProjectCounters.task` (23F).

```text
[STEP_ID]
STEP 23G: Integration tests prove task IDs allocate, reconcile and stay unique under concurrent mixed-type creation

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/tests/storage_integration.rs:2200-2251 (concurrent_creators_get_unique_ids, the STEP 13 pattern)
  - crates/backend_core/src/workflow/lifecycle.rs:30 (CreateItemInput::new)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Use only temp dirs (TempTrackerDir) and placeholder titles. Existing tests and the top-of-file `use` block stay byte-identical: bring `CreateItemInput` in with a `use backend_core::CreateItemInput;` inside each new test fn.

[PRE_GATE]
grep -q '"TASK"' crates/backend_core/src/models/item.rs && grep -q 'pub tasks: Vec<ItemStub>' crates/backend_core/src/models/scope.rs && ! grep -q 'fn task_items_allocate_alongside_bugs_and_features' crates/backend_core/tests/storage_integration.rs

[MUTATION_SPEC]
Append three tests at the end of tests/storage_integration.rs:
1. `task_items_allocate_alongside_bugs_and_features`: fresh `init_standard` tracker; create a bug (`create_bug`), a feature (`create_feature`) and a task (`create_item(CreateItemInput::new(ItemType::Task, ..))`), all in the default scope. IDs are `BUG-0001`, `FEAT-0001`, `TASK-0001`. `read_scope` of the task's phase/scope has `items.tasks` with exactly the task's ID, and the task's `ItemLocation.item_type == ItemType::Task`. `read_project().counters.task == 1`.
2. `reconcile_restores_task_counter`: create one task; set `project.counters` to `Default::default()` via `read_project` / `write_project`; `reconcile_indexes()` → `report.counters.task == 1` and `read_project().counters.task == 1`.
3. `concurrent_mixed_type_creators_get_unique_ids`: same shape as `concurrent_creators_get_unique_ids`: two threads, each with its own `TrackerStorage::new(&root)`, each creating 25 bugs, 25 features and 25 tasks (interleaved in one loop: bug, feature, task per iteration). Afterwards: 150 distinct IDs; 150 dirs under `items_dir(&active_phase, "inbox")`; `read_project().counters` has bug, feature and task all `== 50`; exactly 50 IDs start with each of `BUG-`, `FEAT-`, `TASK-`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && test "$(git diff -U0 HEAD -- crates/backend_core/tests/storage_integration.rs | grep -E '^-' | grep -vc '^--- ')" -eq 0 && test "$(cargo test -p backend_core --test storage_integration -- task_items_allocate_alongside_bugs_and_features reconcile_restores_task_counter concurrent_mixed_type_creators_get_unique_ids 2>&1 | grep -c 'test result: ok. 3 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status. Halt and report instead of changing non-test code if a test exposes a bug.
```

**Decisions the contract leaves open:**
- **Halt on a real bug instead of fixing it.** If a test finds that tasks don't allocate, reconcile or stay unique, that's a 23F/23B defect and gets its own step; 23G only adds tests.

---

### STEP 23H: `neb task` Command

* **Recommended Command:** `/goal`

**Refreshed 2026-11-14** against HEAD `54f351f`. The scope, the three files and the three CLI checks are unchanged from the row. Re-cited only: line numbers moved (the second ID-example doc comment is `:638`, not `:635`; the scope table is `:1970-1987`, not `:1952`); a fresh `neb init` has only the `inbox` scope, so the test adds a `docs` scope before `neb task -s docs`; `neb show --json` nests the item, so the row's `"type": "task"` check reads `["item"]["type"]`; the test gets a name so the gate can find it.

**Beyond the row (approved 2026-11-14):**
1. **The POST_GATE also checks** that `cli_integration.rs` only gains lines (no existing test edited), that the named test runs and passes, and `check-neutral.sh`. Same shape as the gate you approved for 23G.
2. **The test also asserts the new column's data:** after the task is created, `neb scope list --json` reports `tasks_count == 1` for `docs`. The row adds `tasks_count` but none of its three checks would notice if it were wired to the wrong kind.

**Current code (cited for the spec):**
- `args.rs:78-100` `Commands::Feat { title, scope, phase, severity, desc }` with `#[command(visible_alias = "feature")]`; the shape to mirror. ID-example doc comments: `:279` (`Sync.item`) and `:638` (`Commit.item`), both "(`BUG-XXXX` or `FEAT-XXXX`)".
- `commands/mod.rs:96-116` `Commands::Feat` dispatch → `handle_create_item(&storage, ItemType::Feature, ..)` (`:626`).
- `commands/mod.rs:1907-1915` `ScopeListEntryJson { id, target, tech_stack, bugs_count, features_count, total_items }`; `:1956-1957` counts via `items.count(ItemType::Bug / Feature)` (23D); `:1970-1987` text table: header `"{:<18} {:<26} {:<6} {:<6} TECH STACK"` with `"BUGS", "FEATS"`, a `"-".repeat(78)` rule, and rows in the same format.
- `tests/cli_integration.rs:166` `run_neb_json(args, cwd)` (prepends `--json`); `:152` `run_neb_ok`; `:193-300` the lifecycle test to mirror (`init`, `scope add`, `bug`/`feat` → `["id"]`, `["type"]`); close via `done` alias (`args.rs:188`) with `-m`, JSON `["status"] == "closed"` (`:378`). File ends at `:2403`.
- `src/lib.rs:74` parses `Commands::Feat`; not in ALLOWED_FILES and needs no change (no exhaustive match there).

```text
[STEP_ID]
STEP 23H: neb task quick-capture, TASKS column in scope listing, task-aware ID examples

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/args.rs
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/nebulous_cli/src/args.rs:78-100 (Commands::Feat, the shape to mirror)
  - crates/nebulous_cli/src/commands/mod.rs:96-116 (Feat dispatch), :1907-1987 (scope listing)
  - crates/nebulous_cli/tests/cli_integration.rs:193-300 (lifecycle test pattern)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No change to `bug` / `feat` behaviour or flags. Existing tests in cli_integration.rs stay byte-identical; the new test is appended at the end. Temp dirs and placeholder titles only.

[PRE_GATE]
grep -q '"TASK"' crates/backend_core/src/models/item.rs && ! grep -qE '^\s*Task \{' crates/nebulous_cli/src/args.rs && ! grep -q 'tasks_count' crates/nebulous_cli/src/commands/mod.rs

[MUTATION_SPEC]
1. args.rs:
   a. Directly after `Feat { .. },` add `Commands::Task { title, scope, phase, severity, desc }`, same field types, attributes and defaults as `Feat`, with `#[command(visible_alias = "chore")]` and doc "Quick-capture a generic change/work item (`TASK-XXXX`)." Field docs say "task" where Feat's say "feature".
   b. The doc comments at :279 and :638 read "(`BUG-XXXX`, `FEAT-XXXX` or `TASK-XXXX`)".
2. commands/mod.rs:
   a. After the `Commands::Feat` arm, a `Commands::Task` arm identical to it except `ItemType::Task`.
   b. `ScopeListEntryJson` gains `tasks_count: usize` after `features_count`, set to `s.manifest.items.count(ItemType::Task)`.
   c. The text table header becomes `"{:<18} {:<26} {:<6} {:<6} {:<6} TECH STACK"` with `"BUGS", "FEATS", "TASKS"`; each row prints `s.tasks_count` in the new column; the rule becomes `"-".repeat(85)`.
3. cli_integration.rs: append `#[test] fn test_task_quick_capture_show_and_close()`: TempDir; `neb init --name TaskCliTest`; `neb scope add docs --target docs`; `neb --json task "Untrack build dir" -s docs` → `["id"] == "TASK-0001"`, `["type"] == "task"`, `["scope_id"] == "docs"`; `neb --json show TASK-0001` → `["item"]["type"] == "task"` (show nests the item; see `:409`); `neb --json done TASK-0001 -m "Untracked build dir"` → `["status"] == "closed"`.
   Before `done`: `neb --json scope list` → the `docs` entry has `["tasks_count"] == 1`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && test "$(git diff -U0 HEAD -- crates/nebulous_cli/tests/cli_integration.rs | grep -E '^-' | grep -vc '^--- ')" -eq 0 && test "$(cargo test -p nebulous_cli --test cli_integration -- test_task_quick_capture_show_and_close 2>&1 | grep -c 'test result: ok. 1 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status. Halt and report instead of changing backend code if the test exposes a backend bug.
```

**Decisions the contract leaves open:**
- **`chore` alias, not `change`:** kept from the row.
- **Rule widened 78 → 85:** the new `{:<6} ` column adds 7 characters; widening keeps the rule as long as the header.

---

### STEP 23I: TUI `[t]` Task Capture

* **Recommended Command:** `/goal`

**Refreshed 2026-11-14** against HEAD `09ceebf`. The row's scope is unchanged: a `t` key that opens the capture modal for a task, and a hint that lists it. Re-cited only: the b/f arms are at `state.rs:790-795` (not `:803-808`); the hint text the spec rewrote is the detail-pane empty state at `render.rs:494`. `open_create_modal` and the submit path are already type-generic (`state.rs:486`, `:642-648`), and the modal title comes from `ItemType::label` (23E), so it reads "NEW TASK CAPTURE" with no extra code. `'t'` isn't bound anywhere today. No test locks any hint string.

**Beyond the row (approved 2026-11-14):**
1. **Footer lists `[t] Task` too.** The block's STEP_ID says "footer hint lists it", but its spec only touches `render.rs:494`, which is the detail pane, not the footer. The footer is `render.rs:517` (status-message variant) and `:526-528` (the span list). It grows from 111 to 120 characters, so it still fits at 120 columns; at 80 it was already cut off before this change.
2. **The other two empty-state hints mention `[t]`:** `render.rs:211` (What's Next bar) and `:346` (empty item table).
3. **A headless TUI test** in `tests/cli_integration.rs` (a 3rd file): `t` opens a modal titled "NEW TASK CAPTURE", and submitting a title creates `TASK-0001`; the idle render at 120×36 shows `[t] Task`. Without it the gate is only a grep.
4. **Stricter POST_GATE** (needs 3): `cli_integration.rs` only gains lines, the named test passes, and `check-neutral.sh`. Same shape as 23G/23H.

**Current code (cited for the spec):**
- `tui/state.rs:790-795` `KeyCode::Char('b') => { self.open_create_modal(ItemType::Bug); }` and the same for `'f'` / `Feature`; `:486` `open_create_modal(item_type)`; `:642-648` submit builds `CreateItemInput { item_type: create.item_type, .. }`.
- `tui/render.rs:494` `"Press [b] to capture a Bug, [f] to capture a Feature, or [1/2/3] to switch stages."`; `:211` `"All clear in active stage — press [b] New Bug or [f] New Feature."`; `:346` `"No items in this view — press [b] Bug or [f] Feature"`; `:517` `"  │  [1/2/3] Stage [b] Bug [f] Feat [s] Start [d] Done [m] Move [q] Quit"`; `:526-528` `Span::styled("[f]", style_bright())`, `Span::styled(" Feat ", style_base())`.
- `tests/cli_integration.rs:2044-2160` the TUI test pattern: `TuiApp::new(storage, cwd)`, `render_to_string(&mut app, w, h)`, `press_char`, `type_text`, `press_key(KeyCode::Enter)`, `app.modal`, `app.items`. File ends at `:2457`.

```text
[STEP_ID]
STEP 23I: TUI t key opens the task capture modal; footer and empty-state hints list it

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/tui/state.rs
  - crates/nebulous_cli/src/tui/render.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/nebulous_cli/src/tui/state.rs:790-795 (b/f bindings)
  - crates/nebulous_cli/tests/cli_integration.rs:2044-2215 (TUI test pattern)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No other key changes; the modal, submit path and existing hint wording outside the listed strings stay as they are. Existing tests stay byte-identical; the new test is appended at the end.

[PRE_GATE]
grep -q '"TASK"' crates/backend_core/src/models/item.rs && ! grep -q "KeyCode::Char('t')" crates/nebulous_cli/src/tui/state.rs && ! grep -q '\[t\] Task' crates/nebulous_cli/src/tui/render.rs

[MUTATION_SPEC]
1. state.rs: directly after the `'f'` arm, `KeyCode::Char('t') => { self.open_create_modal(ItemType::Task); }`.
2. render.rs:494: the hint becomes "Press [b] Bug, [f] Feature, [t] Task, or [1/2/3] to switch stages."
3. render.rs footer: `:517` reads "  │  [1/2/3] Stage [b] Bug [f] Feat [t] Task [s] Start [d] Done [m] Move [q] Quit"; after the `" Feat "` span at `:528`, add `Span::styled("[t]", style_bright())` and `Span::styled(" Task ", style_base())`.
4. render.rs:211 → "All clear in active stage — press [b] New Bug, [f] New Feature or [t] New Task."; render.rs:346 → "No items in this view — press [b] Bug, [f] Feature or [t] Task".
5. cli_integration.rs: append `#[test] fn test_tui_t_key_opens_task_capture_and_creates_task()` with `use crossterm::event::KeyCode;` and `use nebulous_cli::tui::{render_to_string, TuiApp};` inside the fn: TempDir; `TrackerStorage::init_standard(.., "TuiTaskTest")`; `TuiApp::new(storage, None)`; `render_to_string(&mut app, 120, 36)` contains "[t] Task"; `press_char('t')`; `render_to_string(&mut app, 80, 24)` contains "NEW TASK CAPTURE"; `type_text("Untrack build dir")`; `press_key(KeyCode::Enter)`; `app.modal.is_none()`; `app.items` contains `TASK-0001`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q "KeyCode::Char('t')" crates/nebulous_cli/src/tui/state.rs && bash scripts/check-neutral.sh && test "$(git diff -U0 HEAD -- crates/nebulous_cli/tests/cli_integration.rs | grep -E '^-' | grep -vc '^--- ')" -eq 0 && test "$(cargo test -p nebulous_cli --test cli_integration -- test_tui_t_key_opens_task_capture_and_creates_task 2>&1 | grep -c 'test result: ok. 1 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

**Decisions the contract leaves open:**
- **Footer at 80 columns:** it already overflows (111 characters) before this step; 23I doesn't fix that. STEP 21 owns TUI polish.

---

### STEP 23J: MCP `item_type: "task"` and Commit-Policy Text

* **Recommended Command:** `/goal`

**Refreshed 2026-11-14** against HEAD `0f454ee`. The row's scope is unchanged: MCP `create_item` documents `task`, the MCP scope listing reports `tasks_count`, and item-ID examples and the commit-policy error list TASK IDs. Re-cited only: line numbers moved (`create_item` is `:660-670`; the counts are `:106-107` and `:1259-1260`; the default is `:2351`; git_sync's are `:52`, `:186` and `:1803`). The "scope listing" is the `list_crate_boundaries` tool (`CrateBoundaryEntry`, `:94-111`), which returns each entry under both `boundaries` and `scopes`. `create_item` and `list_items` already accept `"task"` through `ItemType::from_str` since 23F, so this step is text plus one count field.

**Beyond the row (approved 2026-11-14):**
1. **The `list_items` filter description** (`mcp/mod.rs:631`, "Optional item type filter (bug, feature).") also lists `task`. The filter already works for tasks; only its description is stale.
2. **An MCP test** in `tests/workflow_integration.rs` (a 3rd file): `create_item` with `item_type: "task"` returns `TASK-0001` / `"type": "task"`; `list_items` with the `task` filter returns it; `list_crate_boundaries` reports `tasks_count == 1` for `inbox`. Without it the gate is only a grep for `'task'`.
3. **Stricter POST_GATE** (needs 2): `workflow_integration.rs` only gains lines, the named test passes, and `check-neutral.sh`. Same shape as 23G–23I.

**Current code (cited for the spec):**
- `mcp/mod.rs:106-107` `pub bugs_count: usize, pub features_count: usize,` in `CrateBoundaryEntry`; `:1259-1260` `bugs_count: scope_idx.manifest.items.count(ItemType::Bug), features_count: …(ItemType::Feature),`.
- `mcp/mod.rs:661` `"Creates a new bug (BUG-XXXX) or feature (FEAT-XXXX) item in the specified or default phase and scope."`; `:665` `"Item type: 'bug' or 'feature' (default 'bug')."`; `:643` and `:892` contain `BUG-0001 or FEAT-0001`; `:631` `"Optional item type filter (bug, feature)."`; `:2351-2352` `default_item_type_str() -> "bug"`.
- `git_sync/mod.rs:52` `` (`BUG-XXXX` or `FEAT-XXXX`) ``; `:186` `` (`BUG-XXXX` / `FEAT-XXXX`) ``; `:1803` `"Commit policy requires an item_id (e.g. BUG-0001 or FEAT-0001) when committing workspace code changes"`. No test matches that string.
- `tests/workflow_integration.rs:2144-2212` the MCP test pattern (`use backend_core::McpServer; use serde_json::json;` inside the fn, `McpServer::new(Some(storage))`, `server.call_tool(name, json!({..}))`). File ends at `:2251`.

```text
[STEP_ID]
STEP 23J: MCP create_item accepts and documents task; scope listing reports tasks_count; commit-policy text lists TASK IDs

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/src/git_sync/mod.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:623-670 (list_items / create_item definitions), :1259-1260 (scope counts), :2351 (default item type)
  - crates/backend_core/tests/workflow_integration.rs:2144-2212 (MCP test pattern)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Default item_type stays "bug". Tool names, argument names and parsing are unchanged. Step 14 hardening is out of scope. Existing tests stay byte-identical; the new test is appended at the end.

[PRE_GATE]
grep -q 'tasks_count' crates/nebulous_cli/src/commands/mod.rs && ! grep -q 'tasks_count' crates/backend_core/src/mcp/mod.rs

[MUTATION_SPEC]
1. mcp/mod.rs:
   a. `CrateBoundaryEntry` gains `pub tasks_count: usize,` after `features_count`, set at `:1260` to `scope_idx.manifest.items.count(ItemType::Task)`.
   b. `:661` → "Creates a new bug (BUG-XXXX), feature (FEAT-XXXX) or task (TASK-XXXX) item in the specified or default phase and scope."; `:665` → "Item type: 'bug', 'feature' or 'task' (default 'bug')."
   c. Both `BUG-0001 or FEAT-0001` examples (`:643`, `:892`) → `BUG-0001, FEAT-0001 or TASK-0001`.
   d. `:631` → "Optional item type filter (bug, feature, task)."
2. git_sync/mod.rs: `:52` → `` (`BUG-XXXX`, `FEAT-XXXX` or `TASK-XXXX`) ``; `:186` → `` (`BUG-XXXX` / `FEAT-XXXX` / `TASK-XXXX`) ``; `:1803` → "Commit policy requires an item_id (e.g. BUG-0001, FEAT-0001 or TASK-0001) when committing workspace code changes".
3. workflow_integration.rs: append `#[test] fn mcp_create_item_accepts_task_and_reports_tasks_count()` with `use backend_core::McpServer; use serde_json::json;` inside the fn: TempDir; `TrackerStorage::init_standard(.., "McpTaskTest")`; `McpServer::new(Some(storage))`; `call_tool("create_item", json!({"title": "Untrack build dir", "item_type": "task"}))` → `["id"] == "TASK-0001"`, `["type"] == "task"`; `call_tool("list_items", json!({"item_type": "task"}))` → `["total"] == 1`, `["items"][0]["id"] == "TASK-0001"`; `call_tool("list_crate_boundaries", json!({}))` → `["scopes"][0]["scope_id"] == "inbox"`, `["scopes"][0]["tasks_count"] == 1`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q "'task'" crates/backend_core/src/mcp/mod.rs && bash scripts/check-neutral.sh && test "$(git diff -U0 HEAD -- crates/backend_core/tests/workflow_integration.rs | grep -E '^-' | grep -vc '^--- ')" -eq 0 && test "$(cargo test -p backend_core --test workflow_integration -- mcp_create_item_accepts_task_and_reports_tasks_count 2>&1 | grep -c 'test result: ok. 1 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

**Decisions the contract leaves open:**
- **The component-tracking tool's own `item_type`** (`mcp/mod.rs:1684`, default `"feature"`, parsed at `:1774` with `unwrap_or(Feature)`) already accepts `"task"` and has no schema text naming types, so it's untouched.

---

### STEP 23K: `ItemMetadata::new` Constructor (lifecycle site)

* **Recommended Command:** `/goal`

**Refreshed 2026-11-14** against HEAD `d00798c`. The row's scope is unchanged: add `ItemMetadata::new` with its two inline tests, and move `create_item`'s literal onto it. Re-cited only: `ItemMetadata` is now `item.rs:276-302` (not `:196-226`); `create_item`'s literal is `lifecycle.rs:116-125`; the inline tests go at the top of `item.rs`'s `mod tests` (`:339`) and are named so the gate can find them. `ItemStatus::default()` is `Open` and `Severity::default()` is `Medium` (`#[default]` at `:182` and `:235`), so the call site keeps `status: ItemStatus::Open` explicit, as the spec says, and behaviour is identical. The other five literal sites are all in test files (`workflow_integration.rs:330, :366, :388, :739`; `storage_integration.rs:180`), which is 23L.

**Beyond the row (approved 2026-11-14):**
1. **Stricter POST_GATE:** `item.rs` only gains lines (nothing existing edited), the 2 named inline tests pass, and `check-neutral.sh`. The row's gate is the standard four plus a grep, which would still pass with the tests missing.

**Current code (cited for the spec):**
- `models/item.rs:276-290` `pub struct ItemMetadata { id: String, item_type: ItemType (serde "type"), title: String, status: ItemStatus (#[serde(default)]), severity: Severity (#[serde(default)]), scope_ref: String, created_at: String, updated_at: String }`; `:292-302` `impl ItemMetadata { parsed_created_at, parsed_updated_at }`; `:338-340` `#[cfg(test)] mod tests { use super::*;`.
- `workflow/lifecycle.rs:113` `let now = now_rfc3339();`; `:116-125` `item: ItemMetadata { id: item_id.clone(), item_type: input.item_type, title: title.clone(), status: ItemStatus::Open, severity: input.severity, scope_ref: scope_id.clone(), created_at: now.clone(), updated_at: now },`.

```text
[STEP_ID]
STEP 23K: Add ItemMetadata::new; create_item builds its literal with struct-update syntax (Open Question 4)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/item.rs
  - crates/backend_core/src/workflow/lifecycle.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/item.rs:276-302 (ItemMetadata)
  - crates/backend_core/src/workflow/lifecycle.rs:113-125 (create_item `item: ItemMetadata {` literal)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Test-file literals are STEP 23L. No existing line in item.rs changes; lifecycle.rs changes only the create_item literal.

[PRE_GATE]
! grep -q '\.\.ItemMetadata::new' crates/backend_core/src/workflow/lifecycle.rs && ! grep -q 'pub fn new(' crates/backend_core/src/models/item.rs

[MUTATION_SPEC]
1. item.rs, first item in `impl ItemMetadata`: `pub fn new(id: impl Into<String>, item_type: ItemType, title: impl Into<String>, scope_ref: impl Into<String>, timestamp: impl Into<String>) -> Self`. status = `ItemStatus::default()`, severity = `Severity::default()`, created_at = updated_at = timestamp. Doc: "Build literals as `ItemMetadata { field: x, ..ItemMetadata::new(..) }` so adding a field never breaks a call site." Inline tests at the top of `mod tests`:
   a. `item_metadata_new_sets_every_field`: `new("TASK-0001", ItemType::Task, "Sample task", "inbox", "2026-01-01T00:00:00Z")` → every field as described (status Open, severity Medium, both timestamps equal).
   b. `item_metadata_struct_update_keeps_other_fields`: `ItemMetadata { severity: Severity::Critical, ..base.clone() }` keeps every other field of `base` (severity is Critical; setting it back to Medium gives a value equal to `base`).
2. lifecycle.rs create_item: `item: ItemMetadata { status: ItemStatus::Open, severity: input.severity, ..ItemMetadata::new(item_id.clone(), input.item_type, title.clone(), scope_id.clone(), now) },`, with the behaviour identical to today.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q '\.\.ItemMetadata::new' crates/backend_core/src/workflow/lifecycle.rs && bash scripts/check-neutral.sh && test "$(git diff -U0 HEAD -- crates/backend_core/src/models/item.rs | grep -E '^-' | grep -vc '^--- ')" -eq 0 && test "$(cargo test -p backend_core --lib -- models::item::tests::item_metadata_new_sets_every_field models::item::tests::item_metadata_struct_update_keeps_other_fields 2>&1 | grep -c 'test result: ok. 2 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

**Decisions the contract leaves open:**
- **Explicit `status: ItemStatus::Open` at the call site** even though it equals the default: kept from the row, so `create_item` still states the status it writes.

---

### STEP 23L: Test Literals Use `ItemMetadata::new`

* **Recommended Command:** `/goal`

**Refreshed 2026-11-14** against HEAD `5090aef`. Re-cited only; scope, files, spec and gate are unchanged from the block. The five literals are still 1 + 4, at the lines below. All five have an `updated_at` that differs from `created_at`, so each keeps `updated_at` explicitly. `new()` defaults are status `Open` and severity `Medium` (23K), so `status: ItemStatus::Open` (`workflow_integration.rs:334`) and `severity: Severity::Medium` (`:371`) drop out; `storage_integration.rs:180` takes `status` / `severity` as fn parameters, so both stay. `new()` takes `impl Into<String>`, so `&str` params and string literals go in without `.to_string()`. No proposals beyond the row.

**Current code (cited for the spec):**
- `tests/storage_integration.rs:180-189` in `sample_item_detail(id: &str, item_type, title: &str, status, severity, scope_ref: &str)`: `id: id.to_string(), item_type, title: title.to_string(), status, severity, scope_ref: scope_ref.to_string(), created_at: "2026-11-02T10:00:00Z"…, updated_at: "2026-11-02T12:00:00Z"…`.
- `tests/workflow_integration.rs:330-339` BUG-0001 (Open, High, backend_core, created 2026-11-01T12:00:00Z, updated 2026-11-04T12:00:00Z with a trailing comment); `:366-375` FEAT-0001 (InProgress, Medium, inbox, created 2026-11-01T12:00:00Z, updated 2026-11-06T12:00:00Z, comment); `:388-397` BUG-0002 (Closed, Critical, backend_core, created 2026-11-01T00:00:00Z, updated 2026-11-01T12:00:00Z, comment); `:739-748` BUG-0007 (Closed, High, backend_core, created 2026-11-02T10:00:00Z, updated 2026-11-02T11:00:00Z).
- `models/item.rs` `ItemMetadata::new(id, item_type, title, scope_ref, timestamp)` (23K).

```text
[STEP_ID]
STEP 23L: Convert the five ItemMetadata literals in the integration tests to struct-update syntax

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/tests/storage_integration.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/item.rs ItemMetadata::new (STEP 23K)
  - crates/backend_core/tests/storage_integration.rs:180-189; crates/backend_core/tests/workflow_integration.rs:330-339, :366-375, :388-397, :739-748 (the five literals)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No test changes what it asserts. Only the five `item: ItemMetadata { .. }` literals change; trailing comments on `updated_at` stay.

[PRE_GATE]
grep -q '\.\.ItemMetadata::new' crates/backend_core/src/workflow/lifecycle.rs && test "$(grep -c 'item: ItemMetadata {' crates/backend_core/tests/storage_integration.rs)" -eq 1 && test "$(grep -c 'item: ItemMetadata {' crates/backend_core/tests/workflow_integration.rs)" -eq 4 && ! grep -q '\.\.ItemMetadata::new' crates/backend_core/tests/workflow_integration.rs

[MUTATION_SPEC]
1. Every `item: ItemMetadata {` literal in the two files (1 + 4, cited above) becomes `ItemMetadata { <only the fields that differ from new()'s defaults>, ..ItemMetadata::new(id, type, title, scope_ref, timestamp) }`. Use the literal's created_at as the timestamp; keep updated_at explicitly if it differs (it does in all five). The values each test sees stay identical. Concretely:
   - storage_integration.rs:180: keeps `status`, `severity`, `updated_at`; `..ItemMetadata::new(id, item_type, title, scope_ref, "2026-11-02T10:00:00Z")`.
   - workflow_integration.rs BUG-0001: keeps `severity: Severity::High`, `updated_at`.
   - FEAT-0001: keeps `status: ItemStatus::InProgress`, `updated_at`.
   - BUG-0002 and BUG-0007: keep `status`, `severity`, `updated_at`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && test "$(grep -c '\.\.ItemMetadata::new' crates/backend_core/tests/storage_integration.rs)" -ge 1 && test "$(grep -c '\.\.ItemMetadata::new' crates/backend_core/tests/workflow_integration.rs)" -ge 4

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 24A: `source_ref` on Items

* **Recommended Command:** `/goal`

**Refreshed 2026-11-14** against HEAD `b376420`. The row's scope is unchanged: one file, one optional field, `new()` sets it to None, two inline tests. Re-cited only: `ItemMetadata` is `item.rs:276-290` and `new()` is `:294-312`; every literal outside item.rs uses `..ItemMetadata::new` (23K `lifecycle.rs:116`, 23L the five test sites), so the field compiles everywhere with no other edit; the dry-run confirmed it. The tests use `ItemManifest::from_toml_str` / `to_toml_string` (`:23-30`), the same path `item.toml` goes through, and are named so the gate can find them.

**Beyond the row (approved 2026-11-14):**
1. **Stricter POST_GATE:** `item.rs` only gains lines, the 2 named inline tests pass, and `check-neutral.sh`. Same shape as 23K. The row's gate is the standard four plus a grep, which would still pass with the tests missing.

**Current code (cited for the spec):**
- `models/item.rs:276-290` `pub struct ItemMetadata { .., #[serde(deserialize_with = "deserialize_datetime_string")] pub updated_at: String, }`; `:294-312` `pub fn new(..) -> Self { Self { .., created_at: timestamp.clone(), updated_at: timestamp } }`; `:21-30` `ItemManifest::from_toml_str` / `to_toml_string` (`toml::to_string_pretty`); `:360` `mod tests { use super::*;`.
- Every `ItemMetadata { .. }` literal outside item.rs ends in `..ItemMetadata::new(..)`: `workflow/lifecycle.rs:116`, `tests/storage_integration.rs:180`, `tests/workflow_integration.rs:330, :367, :390, :743`.

```text
[STEP_ID]
STEP 24A: Optional source_ref on ItemMetadata links an item to the plan step it was imported from

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/item.rs
READ_ONLY_REFS:
  - Open Question 4 (resolved 2026-11-09: STEP 23K/23L moved every literal to `..ItemMetadata::new`, so this step is one file)
  - crates/backend_core/src/models/item.rs:21-30 (ItemManifest TOML helpers), :276-312 (ItemMetadata, new)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No existing line in item.rs changes. No schema-version change (the field is optional and skipped when None).

[PRE_GATE]
grep -q '"TASK"' crates/backend_core/src/models/item.rs && grep -q 'tasks_count' crates/backend_core/src/mcp/mod.rs && grep -q '\.\.ItemMetadata::new' crates/backend_core/src/workflow/lifecycle.rs && ! grep -q 'source_ref' crates/backend_core/src/models/item.rs

[MUTATION_SPEC]
1. item.rs: `ItemMetadata.source_ref: Option<String>` as the last field (after `updated_at`) with `#[serde(default, skip_serializing_if = "Option::is_none")]`, doc "Plan step this item was imported from (e.g. `STEP 23A`); set only by `neb plan import`." Inline tests at the top of `mod tests`, using a placeholder item.toml const (`[item]` with id `TASK-0001`, type `task`, title, status, severity, scope_ref `inbox`, created_at/updated_at `2026-01-01T00:00:00Z`, no source_ref):
   a. `item_source_ref_absent_loads_none_and_is_not_serialized`: `ItemManifest::from_toml_str` → `item.source_ref == None`; `to_toml_string()` output has no `source_ref`.
   b. `item_source_ref_round_trips`: set `Some("STEP 23A")`, `to_toml_string` → `from_toml_str` gives `Some("STEP 23A")` and a manifest equal to the original.
2. `ItemMetadata::new` sets `source_ref: None`. No other site changes (amended 2026-11-09: every literal uses `..ItemMetadata::new` since STEP 23K/23L).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub source_ref: Option<String>' crates/backend_core/src/models/item.rs && bash scripts/check-neutral.sh && test "$(git diff -U0 HEAD -- crates/backend_core/src/models/item.rs | grep -E '^-' | grep -vc '^--- ')" -eq 0 && test "$(cargo test -p backend_core --lib -- models::item::tests::item_source_ref_absent_loads_none_and_is_not_serialized models::item::tests::item_source_ref_round_trips 2>&1 | grep -c 'test result: ok. 2 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status. If adding the field breaks any file other than item.rs, halt and report (it means a literal was missed in 23K/23L).
```

**Decisions the contract leaves open:**
- **No `ItemStub` / scope-index field:** the row puts `source_ref` on the item only; 24C reads it from item.toml.

---

### STEP 24B: Roadmap-Table Parser

* **Recommended Command:** `/goal`

**Refreshed 2026-11-14** against HEAD `b505013`. The row's scope is unchanged: a new pure `planning` module with `PlanStep` + `parse_roadmap`, `pub mod planning;` in lib.rs, the five inline tests. Re-cited: lib.rs declares modules alphabetically at `:1-10` (`planning` goes after `models`); `TrackerError::ValidationError(String)` is `error.rs:83`; both real plans have `## Step Execution Roadmap` → `### Up Next` (6 columns) → `### Completed` (5 columns), followed by another `###` or `##` heading. A dry-run parser (scratchpad only, not committed) read this repo's plan as 81 steps (68 completed, 13 up next after skipping `[USER] SEED`) and NovOS's as 64 (50 completed), once G5 below was in.

**Gaps the spec leaves (filled; approved 2026-11-14):**
- **G1 `title`** = the cleaned Scope cell.
- **G2 `order`** = the Up Next `#` cell as `u32`; a non-number → ValidationError. Completed rows → None.
- **G3 columns are found by header name** (`#`, `Step` / `Step ID`, `Command`, `Scope`, `Target Files`, `Waits on`), so column order can change; a missing column → ValidationError naming it.
- **G4 a row whose cell count differs from the header** → ValidationError quoting the row, instead of guessing.
- **G5 `\|` inside a cell is a literal pipe.** NovOS's Completed table has one (`STEP 9G`, `curl \| sh`); without this, that plan fails to parse under G4.
- **G6 an empty or `—` Command / Target Files cell** → `None` / empty Vec.
- **G7 only `(NEW)` / `(DEL)` are stripped** (as the spec says). Other notes stay in the file string: NovOS has `target/ (index only)` and `… (+ baseline)`. 24C decides what to do with them.
- **G8 a section ends** at the next `###` heading, or a `##` heading; the roadmap section ends at the next `##`.

**Beyond the row (approved 2026-11-14):**
1. **A 6th inline test** `parse_roadmap_escaped_pipe_stays_in_one_cell` for G5.
2. **Stricter POST_GATE:** lib.rs only gains lines, the `planning::tests` suite shows `6 passed`, and `check-neutral.sh`. The row's gate is the standard four plus a grep, which would still pass with no tests.

**Current code (cited for the spec):**
- `backend_core/src/lib.rs:1-10` `pub mod agent_context; … pub mod models; pub mod storage; pub mod watcher; pub mod workflow;`; `:12-21` `pub use` lines (planning gets none; 24C/24D use `backend_core::planning::…`).
- `backend_core/src/error.rs:83` `ValidationError(String)`; `crate::error::{Result, TrackerError}`.
- No `planning` module or directory exists yet.

```text
[STEP_ID]
STEP 24B: Pure parser turns a plan file's Step Execution Roadmap tables into PlanStep records

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/planning/mod.rs (NEW)
  - crates/backend_core/src/lib.rs
READ_ONLY_REFS:
  - STEPPED_IMPLEMENTATION_PLAN.md "Step Execution Roadmap" (this repo; Up Next = `# | Step | Command | Scope | Target Files | Waits on`, Completed = `Step ID | Status | Command | Scope | Target Files`)
  - /srv/devshare/NovOS/STEPPED_IMPLEMENTATION_PLAN.md "Step Execution Roadmap" (same columns; Up Next rows may say **COMPLETED** in "Waits on"; one Completed cell contains `\|`)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No file I/O and no storage access in this module (tests included). Test fixtures use synthetic rows only (no copied plan text, real hosts or names; check-neutral must pass). No `pub use` re-export in lib.rs.

[PRE_GATE]
grep -q 'pub source_ref: Option<String>' crates/backend_core/src/models/item.rs && test ! -e crates/backend_core/src/planning && ! grep -q 'mod planning' crates/backend_core/src/lib.rs

[MUTATION_SPEC]
1. planning/mod.rs: `#[derive(Debug, Clone, PartialEq, Eq)] pub struct PlanStep { pub step_id: String, pub title: String, pub command: Option<String>, pub target_files: Vec<String>, pub order: Option<u32>, pub completed: bool }` and `pub fn parse_roadmap(markdown: &str) -> Result<Vec<PlanStep>>` (`crate::error::{Result, TrackerError}`). Rules:
   - Only tables under `### Up Next` and `### Completed` within `## Step Execution Roadmap`. A section ends at the next `###` or `##` heading; the roadmap ends at the next `##` (G8). Up Next rows come first, in table order, then Completed rows.
   - Columns are located by header name: `#`, `Step` or `Step ID`, `Command`, `Scope`, `Target Files`, `Waits on` (`#` and `Waits on` only in Up Next). Missing column → ValidationError naming it (G3). Header and `:---` separator rows are not steps.
   - Split rows on `|`, treating `\|` as a literal `|` inside a cell (G5). A row whose cell count differs from the header → ValidationError quoting the row (G4).
   - Clean every cell: strip `**` and backticks, trim. `step_id` = the cleaned Step cell (ranges such as "STEP 1–7" stay one ID). `title` = the cleaned Scope cell (G1).
   - `target_files`: split the cleaned cell on ", ", strip a trailing `(NEW)` / `(DEL)` from each and trim; other parenthetical notes stay (G7). Empty or "—" → empty Vec (G6).
   - `command`: None for "—" or empty (G6).
   - `order`: Up Next `#` parsed as u32, non-number → ValidationError (G2); Completed → None.
   - `completed` = Completed table row, or Up Next row whose cleaned "Waits on" starts with "COMPLETED".
   - Skip rows whose cleaned Step cell starts with "[USER]".
   - Missing `## Step Execution Roadmap`, `### Up Next` or `### Completed` → `TrackerError::ValidationError` naming the missing heading. Duplicate step_id across both tables → ValidationError naming it.
2. lib.rs: `pub mod planning;` directly after `pub mod models;`.
3. Inline tests in `planning::tests`, on one synthetic fixture (Up Next: two steps + a `[USER]` row, one step's Waits on `**COMPLETED**`, one `(NEW)` file; Completed: one `STEP 1–3` range row with a `(DEL)` file and `—` command; followed by `### Open Questions` and a `##` heading):
   a. `parse_roadmap_reads_up_next_and_completed`: the fixture parses to the exact expected Vec.
   b. `parse_roadmap_completed_in_waits_on_marks_completed`.
   c. `parse_roadmap_skips_user_rows`.
   d. `parse_roadmap_missing_heading_errors`: no roadmap heading → error names `## Step Execution Roadmap`; `### Completed` renamed → error names `### Completed`.
   e. `parse_roadmap_duplicate_step_id_errors`: error names the duplicate ID.
   f. `parse_roadmap_escaped_pipe_stays_in_one_cell`: a Scope cell containing `` `curl \| sh` `` parses as one cell whose title contains `curl | sh`, and the row's other cells are unaffected.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q '^pub mod planning;' crates/backend_core/src/lib.rs && bash scripts/check-neutral.sh && test "$(git diff -U0 HEAD -- crates/backend_core/src/lib.rs | grep -E '^-' | grep -vc '^--- ')" -eq 0 && test "$(cargo test -p backend_core --lib -- planning::tests:: 2>&1 | grep -c 'test result: ok. 6 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

**Decisions the contract leaves open:**
- **Error over guessing (G2, G4):** 24C/24D show the error and import nothing, so a malformed plan never creates half its steps.
- **Annotation cleanup (G7) is 24C's call**, since it's where file paths turn into scope matches.

---

### STEP 24G: `CreateItemInput` Literals Use `..CreateItemInput::new`

* **Recommended Command:** `/goal`

**Added 2026-11-14** (user decision on 24C D3: `source_ref` must be written atomically with the item, so `CreateItemInput` gets the field first). Same pattern as 23K/23L: move the struct-literal sites outside lifecycle.rs onto struct-update syntax so 24H can add a field without touching them. No behaviour change. Dry-run (scratchpad only): 3 files, +4/−8, POST_GATE exit 0.

**Current code (cited for the spec):**
- `workflow/lifecycle.rs:18-25` `pub struct CreateItemInput { item_type, title: String, phase_id: Option<String>, scope_id: Option<String>, severity: Severity, description: Option<String> }`; `:30` `CreateItemInput::new(item_type, title: impl Into<String>)` (severity Medium, others None).
- `nebulous_cli/src/commands/mod.rs:675-682` `CreateItemInput { item_type, title: title.to_string(), phase_id: phase.map(str::to_string), scope_id: resolved_scope, severity, description: desc.map(str::to_string) }`.
- `nebulous_cli/src/tui/state.rs:642-649` `CreateItemInput { item_type: create.item_type, title: create.title.trim().to_string(), phase_id: Some(stage_id.clone()), scope_id: Some(scope_id), severity: create.severity, description: desc_opt }`.
- `backend_core/src/mcp/mod.rs:1302-1309` `CreateItemInput { item_type, title: parsed.title, phase_id: non_empty(..), scope_id: non_empty(..), severity, description: non_empty(..) }`; `:1790-1797` `CreateItemInput { item_type: kind, title, phase_id: Some(target_phase), scope_id: target_scope, severity: sev, description: Some(desc) }`.
- The other two literals (`lifecycle.rs:172`, `:191`) are 24H, which edits that file anyway.

```text
[STEP_ID]
STEP 24G: The four CreateItemInput literals outside lifecycle.rs use struct-update syntax over CreateItemInput::new

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/nebulous_cli/src/tui/state.rs
  - crates/backend_core/src/mcp/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/workflow/lifecycle.rs:18-60 (CreateItemInput, new, with_*)
  - crates/nebulous_cli/src/commands/mod.rs:675-682; crates/nebulous_cli/src/tui/state.rs:642-649; crates/backend_core/src/mcp/mod.rs:1302-1309, :1790-1797 (the four literals)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Only the four `CreateItemInput { .. }` literals change; every value each site passes stays identical. No new field, no builder-chain rewrite.

[PRE_GATE]
! grep -q 'source_ref' crates/backend_core/src/workflow/lifecycle.rs && test "$(grep -c 'CreateItemInput {' crates/nebulous_cli/src/commands/mod.rs crates/nebulous_cli/src/tui/state.rs crates/backend_core/src/mcp/mod.rs | awk -F: '{s+=$2} END {print s}')" -eq 4 && ! grep -rq '\.\.CreateItemInput::new' crates/

[MUTATION_SPEC]
1. Each of the four literals becomes `CreateItemInput { phase_id: <same>, scope_id: <same>, severity: <same>, description: <same>, ..CreateItemInput::new(<item_type>, <title>) }`, dropping its `item_type` and `title` lines:
   a. commands/mod.rs:675 → `..CreateItemInput::new(item_type, title)` (`title` is `&str`; `new` takes `impl Into<String>`).
   b. tui/state.rs:642 → `..CreateItemInput::new(create.item_type, create.title.trim())`.
   c. mcp/mod.rs:1302 → `..CreateItemInput::new(item_type, parsed.title)`.
   d. mcp/mod.rs:1790 → `..CreateItemInput::new(kind, title)`.
   `severity` stays explicit at every site (new() would default it to Medium).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && test "$(grep -c '\.\.CreateItemInput::new' crates/nebulous_cli/src/commands/mod.rs crates/nebulous_cli/src/tui/state.rs crates/backend_core/src/mcp/mod.rs | awk -F: '{s+=$2} END {print s}')" -eq 4 && test "$(grep -c 'CreateItemInput {' crates/nebulous_cli/src/commands/mod.rs crates/nebulous_cli/src/tui/state.rs crates/backend_core/src/mcp/mod.rs | awk -F: '{s+=$2} END {print s}')" -eq 4

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 24H: `CreateItemInput.source_ref`

* **Recommended Command:** `/goal`

**Added 2026-11-14** (user decision on 24C D3). `create_item` writes `source_ref` in the same `item.toml` write as the rest of the item, so 24C's import can never leave a task without its `source_ref` (which the next import would duplicate). Dry-run (scratchpad only, on top of 24G): 2 files, +33/−4, gate exit 0.

**Current code (cited for the spec):**
- `workflow/lifecycle.rs:18-25` `CreateItemInput` fields (`description: Option<String>` last); `:30-39` `new()` (`description: None` last); `:56-59` `with_description`; `:116-126` `item: ItemMetadata { status: ItemStatus::Open, severity: input.severity, ..ItemMetadata::new(item_id.clone(), input.item_type, title.clone(), scope_id.clone(), now) }` (23K); `:172-179` `create_bug` and `:191-198` `create_feature` literals `CreateItemInput { item_type: ItemType::Bug|Feature, title: title.into(), phase_id: .., scope_id: .., severity, description: .. }`.
- `models/item.rs` `ItemMetadata.source_ref: Option<String>` (24A).
- `tests/workflow_integration.rs:9-14` already imports `CreateItemInput`, `ItemType`, `TrackerStorage`; `TempDir` at `:7`. File ends at `:2288`.

```text
[STEP_ID]
STEP 24H: CreateItemInput gains source_ref; create_item writes it in the same item.toml write

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/workflow/lifecycle.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/workflow/lifecycle.rs:18-60 (CreateItemInput), :100-160 (create_item), :165-200 (create_bug, create_feature)
  - crates/backend_core/src/models/item.rs (ItemMetadata.source_ref, 24A)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No other change to create_item's behaviour; no validation or trimming of source_ref. Existing tests stay byte-identical; the new test is appended at the end.

[PRE_GATE]
grep -q '\.\.CreateItemInput::new' crates/backend_core/src/mcp/mod.rs && grep -q 'pub source_ref: Option<String>' crates/backend_core/src/models/item.rs && ! grep -q 'source_ref' crates/backend_core/src/workflow/lifecycle.rs

[MUTATION_SPEC]
1. `CreateItemInput`: last field `pub source_ref: Option<String>,` with doc "Plan step the item is imported from; written to `item.source_ref` in the same write as the rest of `item.toml`."; `new()` sets `source_ref: None`.
2. After `with_description`: `pub fn with_source_ref(mut self, source_ref: impl Into<String>) -> Self { self.source_ref = Some(source_ref.into()); self }`.
3. create_item's metadata literal (`:116`) gains `source_ref: input.source_ref.clone(),` before `..ItemMetadata::new(..)`. Nothing else in create_item changes.
4. `create_bug` / `create_feature` literals become `CreateItemInput { phase_id: .., scope_id: .., severity, description: .., ..CreateItemInput::new(ItemType::Bug|Feature, title) }` (same values).
5. workflow_integration.rs: append `#[test] fn create_item_with_source_ref_writes_it_in_item_toml()`: TempDir + `init_standard(.., "SourceRefTest")`; `create_item(CreateItemInput::new(ItemType::Task, "Imported task").with_source_ref("STEP 9Z"))` → returned `detail.manifest.item.source_ref == Some("STEP 9Z")` and `read_item_manifest(loc.phase_id, loc.scope_id, id)` on disk has the same; `create_item(CreateItemInput::new(ItemType::Task, "Hand-made task"))` → `source_ref == None`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'pub fn with_source_ref' crates/backend_core/src/workflow/lifecycle.rs && grep -q 'source_ref: input.source_ref.clone()' crates/backend_core/src/workflow/lifecycle.rs && test "$(git diff -U0 HEAD -- crates/backend_core/tests/workflow_integration.rs | grep -E '^-' | grep -vc '^--- ')" -eq 0 && test "$(cargo test -p backend_core --test workflow_integration -- create_item_with_source_ref_writes_it_in_item_toml 2>&1 | grep -c 'test result: ok. 1 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 24C: Idempotent Plan Import

* **Recommended Command:** `/boost`

**Refreshed 2026-11-14** against HEAD `6355a69` (lifecycle.rs line numbers re-cited at `ca8ae95` after 24H); **revised the same day** after the user chose D3 (b): 24G/24H run first and give `CreateItemInput` an atomic `source_ref`. The row's scope is unchanged: `import_plan` upserts one task per plan step by `source_ref`, closes on COMPLETED, retitles, never reopens/moves/re-scopes, has a no-write dry run, and gets the seven integration tests. Re-cited:
- **The write lock is thread-reentrant now** (`storage/mod.rs:475-490`, `HELD_WRITE_LOCKS`): calling `create_item` / `close_item` / `update_item_metadata` inside `with_write_lock` just runs them. The old item 4's "no lock-free inner operations → halt" clause no longer applies.
- **`source_ref` isn't in scope-index stubs** (`ItemStub`, `models/scope.rs:99-108`), so matching reads every `item.toml` (`read_item_manifest`, `storage/mod.rs:778`).
- **`create_item` sets `source_ref` atomically** once 24H lands (`CreateItemInput::with_source_ref`).
- **Components aren't in `models/project.rs`.** They're `ComponentRegistry` (`models/component.rs:93`), and items live in scopes, which have `target` / `tracked_files`. The codebase's file→scope matcher is `match_file_to_best_scope` (`workflow/assist.rs:919`, private).
- **Two things a dry run must not call:** `resolve_phase_and_scope` (`storage/mod.rs:970`) writes (it can create a canonical phase and bootstrap the default scope), and `with_write_lock` creates `.tracker/.lock`.
- A dry-run implementation (scratchpad only, on top of dry-run 24G + 24H) passed this block's full POST_GATE; an earlier version imported this repo's plan (81 created, then 81 unchanged on re-run) and NovOS's (64, then 64) into temp trackers.

**Decisions the spec leaves open or gets wrong (all approved 2026-11-14):**
- **D1 scope matching uses scopes, not components:** each target file goes to the scope `match_file_to_best_scope` picks (longest `target` prefix, or an exact `tracked_files` entry) in the import phase; the scope with the most files wins; a tie or no match → `default_scope`. The spec's "component relative_path" doesn't map to a scope unless the component was promoted, and scopes are what items live in.
- **D2 reuse the existing matcher:** `fn match_file_to_best_scope` in `workflow/assist.rs` becomes `pub(crate) fn` (one word, 4th file). The alternative is a ~30-line copy in import.rs that can drift.
- **D3 `source_ref` is written with the item (user, 2026-11-14):** `create_item(CreateItemInput::new(..)..with_source_ref(step_id))`, one `item.toml` write. Prerequisites 24G (literal sites → `..CreateItemInput::new`) and 24H (the field). No post-create patch, so no window where a task exists without its `source_ref`.
- **D4 report entries pair step and item:** `ImportEntry { step_id, item_id: Option<String> }`, with `item_id` None only for a dry-run creation (no ID is allocated without writing). `ImportReport` and `ImportEntry` derive `Serialize` (24D `--json` and 24F return it). The row's `Vec<String>` can't carry "paired with its step_id".
- **D5 the import phase must already exist:** both modes return `NotFound` for an unknown phase instead of auto-creating one (only the write path could, so dry run and write would disagree).
- **D6 two items with the same `source_ref`** → ValidationError naming both, before anything is written.
- **D7 a new step that's already COMPLETED** is created then closed (summary "Completed per plan roadmap"), and is reported under `created` only.
- **D8 the dry run takes no lock** and calls no write path (so `.lock` isn't created and `.tracker/` stays byte-identical).

**Beyond the row (approved 2026-11-14):**
1. **Stricter POST_GATE:** `planning/mod.rs` only gains lines, the `assist.rs` diff is exactly the one `pub(crate)` line, `plan_import` shows `7 passed`, and `check-neutral.sh`.

**Current code (cited for the spec):**
- `storage/mod.rs:475` `with_write_lock(&self, f: impl FnOnce(&Self) -> Result<T>)` (reentrant); `:109` `TrackerIndex::all_items() -> Vec<ItemLocation>`; `:144` `ScopeIndex { scope_id, manifest }`; `:778` `read_item_manifest(phase, scope, id)`; `:795` `write_item_manifest(phase, scope, id, &manifest)`; `:866` `load_index()`; `:957` `load_item_detail_by_id(id)`; `:1053` `pub fn normalize_relative_path_str`.
- `workflow/lifecycle.rs:18-68` `CreateItemInput::new(item_type, title).with_phase(..).with_scope(..).with_description(..).with_source_ref(..)` (`with_source_ref` at `:64`, 24H); `:108` `create_item(input) -> Result<(ItemLocation, ItemDetail)>` (wraps the description in the standard desc.md boilerplate); `:432` `close_item(id, summary, commit_override: Option<&str>, merged: bool)`; `:495` `update_item_metadata(id, title: Option<String>, severity: Option<Severity>)`; `:664` `create_scope(phase, id, target, tech_stack, description)`.
- `models/item.rs:193` `ItemStatus::is_completed()` (Closed | Merged); `models/phase.rs:22` `resolve_phase_alias`; `project.default_scope` (`models/project.rs:47-48`, empty → `"inbox"`).
- `workflow/assist.rs:919-965` `fn match_file_to_best_scope(scopes: &[ScopeIndex], default_scope: &str, rel_file: &str) -> Option<(String, usize)>` (skips `inbox` and `default_scope`).
- `planning/mod.rs` (24B) `PlanStep`, `parse_roadmap`; `use crate::error::{Result, TrackerError};` at `:3`.

```text
[STEP_ID]
STEP 24C: import_plan upserts task items by source_ref under the tracker write lock, with a no-write dry run

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/planning/import.rs (NEW)
  - crates/backend_core/src/planning/mod.rs
  - crates/backend_core/src/workflow/assist.rs   [D2]
  - crates/backend_core/tests/plan_import.rs (NEW)
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs:109 (all_items), :475-490 (with_write_lock, reentrant), :778-806 (read/write_item_manifest), :957 (load_item_detail_by_id), :970-1010 (resolve_phase_and_scope: writes; not for dry run), :1053 (normalize_relative_path_str)
  - crates/backend_core/src/workflow/lifecycle.rs:18-68 (CreateItemInput, with_source_ref at :64), :108 (create_item), :432 (close_item), :495 (update_item_metadata), :664 (create_scope)
  - crates/backend_core/src/workflow/assist.rs:919-965 (match_file_to_best_scope)
  - crates/backend_core/src/models/item.rs:193 (is_completed); models/phase.rs:22 (resolve_phase_alias)
  - Open Question 5 (field ownership)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. assist.rs changes only `fn match_file_to_best_scope` → `pub(crate) fn match_file_to_best_scope`. import.rs never calls `write_item_manifest` (it only reads manifests for matching). Import never reopens an item, never moves stage or scope, never edits severity, effort or an existing desc.md. The dry run takes no lock and calls nothing that writes (including resolve_phase_and_scope). Tests use temp dirs and synthetic plan text only.

[PRE_GATE]
grep -q 'pub fn parse_roadmap' crates/backend_core/src/planning/mod.rs && grep -q 'pub fn with_source_ref' crates/backend_core/src/workflow/lifecycle.rs && grep -q 'fn with_write_lock' crates/backend_core/src/storage/mod.rs && grep -q '^fn match_file_to_best_scope' crates/backend_core/src/workflow/assist.rs && test ! -e crates/backend_core/src/planning/import.rs

[MUTATION_SPEC]
1. import.rs types: `pub const PLAN_CLOSE_SUMMARY: &str = "Completed per plan roadmap";` `#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct ImportEntry { pub step_id: String, pub item_id: Option<String> }` and `#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)] pub struct ImportReport { pub created: Vec<ImportEntry>, pub retitled: Vec<ImportEntry>, pub closed: Vec<ImportEntry>, pub unchanged: Vec<ImportEntry> }` (D4). Entries follow the order of `steps`.
2. `pub fn import_plan(storage: &TrackerStorage, steps: &[PlanStep], phase_id: Option<&str>, write: bool) -> Result<ImportReport>`: `write == true` → `storage.with_write_lock(|s| run(s, ..))`; `write == false` → `run` directly, with no lock (D8). One private `run(storage, steps, phase_id, write)` computes the report for both modes; every mutation sits behind `if write`.
3. Setup in `run`: `load_index()`; phase = `resolve_phase_alias` of `phase_id` (trimmed, non-empty) else of `active_phase`; `index.phase(&phase)` missing → `TrackerError::NotFound { kind: "Phase", id }` (D5); default scope = `project.default_scope` trimmed, empty → `"inbox"`. Do not call `resolve_phase_and_scope`.
4. Matching: for every `index.all_items()` location, `read_item_manifest`; build a map `source_ref → (item id, title, status)`. A second item with the same `source_ref` → ValidationError naming both IDs and the ref (D6).
5. No match → `created`. With `write`: scope = D1 (`pick_scope` below); `create_item(CreateItemInput::new(ItemType::Task, title.trim()).with_phase(phase).with_scope(scope).with_description(desc).with_source_ref(step_id))` (D3; one write, never a separate `write_item_manifest`), where desc names the step ID, command (or "—"), roadmap order (or "completed") and each target file; then, if `step.completed`, `close_item(id, PLAN_CLOSE_SUMMARY, None, false)` (D7). `item_id` = the new ID (write) or None (dry run).
6. Existing match: title (trimmed) differs → `update_item_metadata(id, Some(title), None)` and push to `retitled`. `step.completed && !status.is_completed()` → `close_item(id, PLAN_CLOSE_SUMMARY, None, false)` and push to `closed`. Neither → `unchanged`. Nothing else is ever changed: no reopen, no stage or scope move.
7. `fn pick_scope(scopes: &[ScopeIndex], default_scope: &str, target_files: &[String]) -> String` (D1): per file, `normalize_relative_path_str`, skip empty, `match_file_to_best_scope(scopes, default_scope, &norm)` → count per scope ID; the single scope with the highest count wins; a tie for highest or no match → `default_scope`.
8. assist.rs: `fn match_file_to_best_scope(` → `pub(crate) fn match_file_to_best_scope(` (D2). Nothing else.
9. mod.rs: `pub mod import;` and `pub use import::{import_plan, ImportEntry, ImportReport};` after the `use crate::error` line.
10. tests/plan_import.rs (TempDir + `TrackerStorage::init_standard`; steps from `parse_roadmap` on synthetic markdown with Up Next rows STEP 2A `src/widget.rs` (NEW) and STEP 2B `src/app.rs`, and Completed row STEP 1A):
   a. `first_write_run_creates_tasks_with_source_ref`: created = [2A, 2B, 1A] in order, other lists empty; 2A is a `TASK-` item titled from the Scope cell with `source_ref == Some("STEP 2A")`, status Open, desc mentions `src/widget.rs`; 1A is Closed.
   b. `second_run_reports_everything_unchanged`: the same steps again → created/retitled/closed empty, unchanged = [2A, 2B, 1A].
   c. `retitled_row_retitles_only`: rename 2B's Scope cell → retitled = [2B] only; the item's new title is set; status, severity, scope and desc.md are unchanged.
   d. `row_flipped_to_completed_closes`: 2B's Waits on → `**COMPLETED**` → closed = [2B]; the item is Closed.
   e. `manually_closed_item_stays_closed_while_row_is_open`: `close_item` 2A by hand; re-import with 2A still open → 2A in `unchanged`, still Closed.
   f. `dry_run_leaves_tracker_byte_identical`: every file under `.tracker/` (path → bytes) is identical before and after a dry run; dry created = [2A, 2B, 1A] with all `item_id == None`; a following write run creates the same step IDs.
   g. `target_files_under_a_scope_land_in_that_scope`: `create_scope(None, "core", "crates/core", vec![], None)`; a step with `crates/core/src/a.rs`, `crates/core/src/b.rs`, `README.md` → the item's `scope_ref == "core"`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && cargo test -p backend_core --test plan_import && bash scripts/check-neutral.sh && test "$(git diff -U0 HEAD -- crates/backend_core/src/planning/mod.rs | grep -E '^-' | grep -vc '^--- ')" -eq 0 && test "$(git diff --numstat HEAD -- crates/backend_core/src/workflow/assist.rs)" = "$(printf '1\t1\tcrates/backend_core/src/workflow/assist.rs')" && grep -q '^pub(crate) fn match_file_to_best_scope' crates/backend_core/src/workflow/assist.rs && test "$(cargo test -p backend_core --test plan_import 2>&1 | grep -c 'test result: ok. 7 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status. The /boost verification subagent re-reads import.rs against Open Question 5 and confirms no path reopens, moves or re-scopes an item, and that the dry-run path takes no lock and calls nothing that writes.
```

---

### STEP 24D: `neb plan import`

* **Recommended Command:** `/goal`

**Refreshed 2026-11-14** against HEAD `ffffbf0`. The row's scope is unchanged: `neb plan import <PATH> [--phase] [--write]`, dry run by default, text lines per action, `--json` prints the ImportReport, and the four CLI checks. Re-cited: subcommand groups follow `Commands::Scope { #[command(subcommand)] action: ScopeCommand }` (`args.rs:332-335`, enum at `:439`); dispatch is in `execute_to_string` (`commands/mod.rs:274-277` for Scope); errors reach `main.rs`, which prints `Error: <msg>` (or JSON on stderr) and exits non-zero; `ImportReport` / `ImportEntry` serialize (24C). A dry-run implementation (scratchpad only) passed the gate below and, on this repo's real plan, printed 83 `CREATE (new)` lines + the footer, created TASK-0001…0083 with `--write`, and reported `UNCHANGED 83` on re-run.

**Gaps the spec leaves (filled; approved 2026-11-14):**
- **G1 the enum is `PlanCommand`** (singular), matching `ScopeCommand`, `PhaseCommand`, etc., not the spec's `PlanCommands`.
- **G2 a dry-run creation prints `CREATE (new) ← STEP 2A`**, because no ID is allocated without writing (24C D4).
- **G3 text layout:** `CREATE` lines, then `RETITLE`, then `CLOSE` (each in plan order, `<VERB> <ID> ← <STEP>`), then `UNCHANGED <n>`, then the dry-run footer only when not writing.
- **G4 `<PATH>` is relative to the current directory** (normal CLI behaviour; `--root` only picks the tracker). A read error → `TrackerError::Io` naming the path.
- **G5 `--json` prints the ImportReport as-is** (no extra `dry_run` key; a dry run shows `item_id: null` on creations).
- **G6 the CLI test checks "writes nothing" via `neb --json list` total 0**; the byte-identical check already lives in 24C's backend test.

**Beyond the row (approved 2026-11-14):**
1. **Stricter POST_GATE:** `args.rs` and `cli_integration.rs` only gain lines, the 2 named tests pass, `check-neutral.sh`, and a grep for `PlanCommand::Import` in commands/mod.rs. The row's gate is the standard four only, which passes on an untouched tree.

**Current code (cited for the spec):**
- `args.rs:1` `use std::path::PathBuf;`; `:332-335` `Scope { #[command(subcommand)] action: ScopeCommand }`; `:438-439` `#[derive(Debug, Clone, Subcommand)] pub enum ScopeCommand {`.
- `commands/mod.rs:5-12` `use backend_core::{..}`; `:14` `use serde::Serialize;`; `:16-19` `use crate::args::{Cli, Commands, ComponentCommand, GitCommand, PhaseCommand, ProjectCommand, ScopeCommand, TimerCommand};`; `:274-277` Scope dispatch (`resolve_storage(cli.root.as_deref())?` then `handle_scope(&storage, action, cli.json)`); `:523` `to_json_pretty`; `:1953` `fn handle_scope(storage, action, json) -> Result<String>`. `std::fs` is not imported at module level (use the full path).
- `backend_core::planning::{import_plan, parse_roadmap, ImportEntry, ImportReport}` (24B/24C).
- `tests/cli_integration.rs:141-175` `run_neb`, `run_neb_ok`, `run_neb_json` (prepends `--json`); file ends at `:2479`.

```text
[STEP_ID]
STEP 24D: neb plan import <PATH> [--phase <ID>] [--write]; dry run by default

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/args.rs
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/planning/mod.rs (parse_roadmap), crates/backend_core/src/planning/import.rs (import_plan, ImportReport, ImportEntry)
  - crates/nebulous_cli/src/args.rs:332-335, :438-439 (Scope / ScopeCommand pattern)
  - crates/nebulous_cli/src/commands/mod.rs:274-277 (Scope dispatch), :523 (to_json_pretty), :1953 (handle_scope)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No MCP tool in this step (Open Question 6; that's 24F). No change to existing commands, output or tests. Test plan files are synthetic.

[PRE_GATE]
grep -q 'pub fn import_plan' crates/backend_core/src/planning/import.rs && ! grep -q 'PlanCommand' crates/nebulous_cli/src/args.rs

[MUTATION_SPEC]
1. args.rs:
   a. After `Scope { .. },` in `Commands`: `/// Import roadmap steps from a stepped plan file as task items. Dry run unless --write.` `Plan { #[command(subcommand)] action: PlanCommand },` (G1).
   b. Before `pub enum ScopeCommand`: `#[derive(Debug, Clone, Subcommand)] pub enum PlanCommand { Import { path: PathBuf, #[arg(short, long)] phase: Option<String>, #[arg(long)] write: bool } }`, each with a one-line doc (path = plan Markdown file; phase = phase for new tasks, default `active_phase`; write = apply the changes, otherwise nothing is written).
2. commands/mod.rs:
   a. Import `PlanCommand` in the `crate::args` use list and `use backend_core::planning::{import_plan, parse_roadmap, ImportEntry};`.
   b. Dispatch after the Scope arm: `Some(Commands::Plan { action }) => { let storage = resolve_storage(cli.root.as_deref())?; handle_plan(&storage, action, cli.json) }`.
   c. `fn handle_plan(storage: &TrackerStorage, action: &PlanCommand, json: bool) -> Result<String>`: `std::fs::read_to_string(path)` (error → `TrackerError::Io { path, source }`, G4); `parse_roadmap`; `import_plan(storage, &steps, phase.as_deref(), *write)`. `json` → `to_json_pretty(&report)` (G5). Otherwise G3: one `"<VERB> <ID> ← <STEP>"` line per entry for CREATE (`report.created`), RETITLE, CLOSE in that order, `<ID>` = `item_id` or `(new)` (G2); then `UNCHANGED <report.unchanged.len()>`; then, only when not writing, `dry run: nothing written; re-run with --write`. Return the text trimmed of trailing whitespace.
3. cli_integration.rs, appended at the end, each in a TempDir with `neb --root <tmp> init --name PlanCliTest` and a synthetic plan file in the temp dir (Up Next STEP 2A, STEP 2B; Completed STEP 1A):
   a. `test_plan_import_dry_run_write_and_rerun`: `plan import <file>` stdout contains `CREATE (new) ← STEP 2A` and the dry-run footer, and `--json list` then has `total == 0` (G6); `--json plan import <file> --write` → `created` has 3 entries, `created[0].step_id == "STEP 2A"`, `created[0].item_id` starts with `TASK-`; a second `plan import <file> --write` stdout has no `CREATE`, contains `UNCHANGED 3`, and no `dry run`.
   b. `test_plan_import_missing_roadmap_section_fails_with_heading`: a plan file without the roadmap heading → exit non-zero, stderr contains `## Step Execution Roadmap`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'PlanCommand::Import' crates/nebulous_cli/src/commands/mod.rs && test "$(git diff -U0 HEAD -- crates/nebulous_cli/tests/cli_integration.rs crates/nebulous_cli/src/args.rs | grep -E '^-' | grep -vc '^--- ')" -eq 0 && test "$(cargo test -p nebulous_cli --test cli_integration -- test_plan_import_dry_run_write_and_rerun test_plan_import_missing_roadmap_section_fails_with_heading 2>&1 | grep -c 'test result: ok. 2 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 24F: MCP `plan_import` Tool

* **Recommended Command:** `/goal`

**Refreshed 2026-11-15** against HEAD `13d314d`. The row's scope is unchanged: an MCP `plan_import` tool `{path, phase?, dry_run = true}` wrapping `planning::import_plan`, registered in the three lists, `path` contained to the project root, and the three tests. Re-cited: the three lists are `is_known_tool` (`mcp/mod.rs:578-605`), `tools_list` (`:608-916`) and `call_tool` (`:919-950`); tools parse a local `#[derive(Deserialize)] struct Args` with `serde_json::from_value` (`create_item`, `:1280-1297`); storage comes from `self.resolve_active_storage()?` (`:161`); `contained_path(base, rel)` is `storage/mod.rs:1198` and rejects `..` with "escapes"; the JSON-RPC layer (`:388-420`) wraps whatever object `call_tool` returns and adds `content`. No test counts the tools (`cli_integration.rs:1290-1330` checks names only). A dry-run implementation (scratchpad only) passed the gate below: 2 files, +119/−1.

**Gaps the spec leaves (filled; approved 2026-11-15):**
- **G1 placement:** in Suite 2 (Project & Ingestion), right after `promote_component_to_scope` in all three lists.
- **G2 strict arguments:** `Args` has `#[serde(deny_unknown_fields)]`, so the code matches the schema's `additionalProperties: false`. A typo such as `"write": true` is an error, not a silent dry run. `phase` also accepts `phase_id`, like the other tools.
- **G3 payload:** the ImportReport serialized as-is (`serde_json::to_value(report)`, error → ValidationError, as `:542` does); no extra `dry_run` key. A dry run shows `item_id: null` on creations.
- **G4 reading:** `std::fs::read_to_string` on the contained path; a read error → `TrackerError::Io` naming it. An empty `phase` string counts as none (`non_empty`, `:2389`).

**Beyond the row (approved 2026-11-15):**
1. **A 4th test** `mcp_plan_import_rejects_unknown_arguments` for G2.
2. **Stricter POST_GATE:** `plan_import.rs` only gains lines, `"plan_import"` appears exactly 3 times in mcp/mod.rs (one per list), the `plan_import` suite shows `11 passed` (7 from 24C + 4), and `check-neutral.sh`.

**Current code (cited for the spec):**
- `mcp/mod.rs:19` `use crate::storage::{normalize_relative_path_str, ItemLocation, TrackerStorage};`; `:593` `| "promote_component_to_scope"`; `:768-780` its `McpToolDefinition`, followed by `// Suite 3` at `:781`; `:935` `"promote_component_to_scope" => self.tool_promote_component_to_scope(arguments),`; `:953` `fn tool_get_active_task` (first tool fn); `:2389` `fn non_empty`; `:2413` `fn default_true`.
- `storage/mod.rs:389` `project_root()`; `:1198` `pub fn contained_path(base: &Path, rel: impl AsRef<Path>) -> Result<PathBuf>`.
- `tests/plan_import.rs:11` `SAMPLE_PLAN` (3 steps), `:29` `init_test_tracker()`, `:36` `snapshot_directory(root)`; file ends at `:382`.

```text
[STEP_ID]
STEP 24F: MCP tool plan_import {path, phase?, dry_run = true} wrapping planning::import (Open Question 6a)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/plan_import.rs
READ_ONLY_REFS:
  - crates/backend_core/src/planning/mod.rs (parse_roadmap), crates/backend_core/src/planning/import.rs (import_plan, ImportReport)
  - crates/backend_core/src/mcp/mod.rs:578-605 (is_known_tool), :608-916 (tools_list), :919-950 (call_tool), :1280-1297 (tool_create_item Args pattern), :161 (resolve_active_storage), :2389 (non_empty), :2413 (default_true)
  - crates/backend_core/src/storage/mod.rs:389 (project_root), :1198 (contained_path)
  - crates/backend_core/tests/plan_import.rs:11-61 (SAMPLE_PLAN, init_test_tracker, snapshot_directory)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No other tool changes. Existing tests in plan_import.rs stay byte-identical; new tests are appended at the end.

[PRE_GATE]
grep -q 'pub fn import_plan' crates/backend_core/src/planning/import.rs && ! grep -q '"plan_import"' crates/backend_core/src/mcp/mod.rs

[MUTATION_SPEC]
1. mcp/mod.rs registration (G1), each directly after `promote_component_to_scope`:
   a. `is_known_tool`: `| "plan_import"`.
   b. `tools_list`: `McpToolDefinition { name: "plan_import", description: <says it imports the roadmap tables as task items matched by source_ref (create, retitle, close; never reopens, moves or re-scopes) and that it is a dry run unless dry_run is false>, input_schema: json!({ "type": "object", "properties": { "path": string ("Plan Markdown file, relative to the project root."), "phase": string (optional, defaults to active_phase), "dry_run": boolean (default true, "default": true) }, "required": ["path"], "additionalProperties": false }) }`.
   c. `call_tool`: `"plan_import" => self.tool_plan_import(arguments),`.
2. `fn tool_plan_import(&mut self, args: Value) -> Result<Value>` placed before `tool_get_active_task`: local `#[derive(Debug, Deserialize)] #[serde(deny_unknown_fields)] struct Args { path: String, #[serde(default, alias = "phase_id")] phase: Option<String>, #[serde(default = "default_true")] dry_run: bool }` (G2); parse error → `ValidationError("Invalid plan_import args: {e}")`; `let storage = self.resolve_active_storage()?;` `contained_path(&storage.project_root(), &parsed.path)?`; `std::fs::read_to_string` (error → `TrackerError::Io`, G4); `parse_roadmap`; `import_plan(&storage, &steps, non_empty(parsed.phase.as_deref()), !parsed.dry_run)`; return `serde_json::to_value(report)` with errors mapped to ValidationError (G3).
3. mcp/mod.rs imports: `use crate::planning::{import_plan, parse_roadmap};` and `contained_path` added to the `crate::storage` use.
4. tests/plan_import.rs, appended: helper `mcp_tracker_with_plan()` = `init_test_tracker()` + write `SAMPLE_PLAN` to `<tmp>/PLAN.md`; each test uses `backend_core::McpServer` and `serde_json::json` inside the fn and `McpServer::new(Some(storage))`:
   a. `mcp_plan_import_defaults_to_dry_run`: `call_tool("plan_import", {"path": "PLAN.md"})` → `.tracker/` byte-identical (`snapshot_directory`); `created` has 3 entries, `created[0].step_id == "STEP 2A"`, every `item_id` null.
   b. `mcp_plan_import_dry_run_false_creates_tasks`: `{"path": "PLAN.md", "dry_run": false}` → 3 created; `created[0].item_id` starts with `TASK-` and that item's `source_ref == Some("STEP 2A")`.
   c. `mcp_plan_import_rejects_path_outside_project_root`: `{"path": "../outside.md"}` → Err whose message contains `escapes`.
   d. `mcp_plan_import_rejects_unknown_arguments`: `{"path": "PLAN.md", "write": true}` → Err whose message contains `write`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q '"plan_import"' crates/backend_core/src/mcp/mod.rs && bash scripts/check-neutral.sh && test "$(git diff -U0 HEAD -- crates/backend_core/tests/plan_import.rs | grep -E '^-' | grep -vc '^--- ')" -eq 0 && test "$(grep -c '"plan_import"' crates/backend_core/src/mcp/mod.rs)" -eq 3 && test "$(cargo test -p backend_core --test plan_import 2>&1 | grep -c 'test result: ok. 11 passed')" -eq 1

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 24E: Docs for `task` and Plan Import

* **Recommended Command:** `/goal`

**Refreshed 2026-11-15** against HEAD `094182e`. The row's scope is unchanged: USAGE.md documents `neb task`, the TUI `[t]` key, `neb plan import` (format, dry run, `--write`, idempotency, field ownership), the MCP `plan_import` tool and hash item IDs; README gets one line each for `neb task` and `neb plan import`. Re-cited:
- **The enum is `PlanCommand`** (24D G1), so the old PRE_GATE grep for `PlanCommands` would fail; fixed below.
- **README has no command-overview list.** The commands live in the Quick Start §1 code block (`README.md:96-106`, after `neb feat`), so the two lines go there.
- **Facts checked against code:** `[project] id_scheme = "hash"` (`models/project.rs:52`, missing key = sequential, test `:301`); hash IDs are `<PREFIX>-` + 6 uppercase hex digits (`ItemType::format_hash_id`, `models/item.rs:97`), redrawn when all digits, so they always contain a letter (`storage/id.rs:65`, test `:192`); the duplicate-ID warning ("item ID … exists in more than one item directory …") prints from `neb sync-index`, `neb sync` and `neb git pull` (`commands/mod.rs:1758, :1795, :2848`).
- **Field ownership as finally decided:** OQ5 plus 24C D1, D5 and D7 (a step already COMPLETED is created then closed; new tasks land in `--phase`, default `active_phase`; the scope is the one matching the most target files, a tie or none → `default_scope`).
- No code changes, so there's no dry-run; the gate greps below were checked to be satisfiable against the spec wording.

**Gaps the spec leaves (filled; approved 2026-11-15):**
- **G1 placement in USAGE.md:** `neb task` / `chore` and the bug/feature/task guidance go in Part 1 §2 (`:44-73`, title gains `neb task`, plus one `neb task` line in its code block); a new **§6 Plan Import (`neb plan import`)** and **§7 Item IDs** go after §5 (before `## Part 2`, `:164`); the `t` key gets its own row in the Keyboard Shortcuts table (`:193-207`, after the `b` / `f` row); the MCP `plan_import` tool gets a short subsection at the end of Part 3 (after the Codex snippet, `:236`).
- **G2 examples stay neutral** (generic file names and step IDs; `scripts/check-neutral.sh` scans docs/ and README.md with an empty allowlist).

**Beyond the row (approved 2026-11-15):**
1. **Stricter POST_GATE:** `check-neutral.sh`, plus greps that each required topic landed: `chore`, `plan_import`, `id_scheme = "hash"`, `source_ref`, `--write` and a `` `t` `` table row in USAGE.md, and `neb task` in README.md. The row's gate only checks two phrases in USAGE.md and one in README.md.

**Current docs (cited for the spec):**
- `docs/USAGE.md:44` `### 2. Predictive Capture & Zero-ID Daily Flow (\`neb bug\`, \`neb feat\`, …)`; `:52-55` the capture code block (`neb bug …`, `neb feat …`); `:144` `### 5.Git Operations …`; `:164` `---` before `## Part 2`; `:193-207` Keyboard Shortcuts table (`| \`b\` / \`f\` | Open predictive \`New Bug\` / \`New Feature\` modal |`); `:213` `## Part 3: Model Context Protocol Server`; file ends at `:236`.
- `README.md:100-102` `# Capture a bug or feature …` / `neb bug …` / `neb feat …`.
- CLI facts: `neb task <TITLE> [-s] [-p] [--severity] [-d]`, alias `chore` (`args.rs`, `Commands::Task`, 23H); `neb plan import <PATH> [-p/--phase] [--write]`, `PATH` relative to the current directory, output lines `CREATE <ID|(new)> ← <STEP>`, `RETITLE`, `CLOSE`, `UNCHANGED <n>`, dry-run footer, `--json` ImportReport (`PlanCommand::Import`, 24D); MCP `plan_import {path (relative to project root), phase?, dry_run = true}`, unknown arguments rejected (24F); TUI `t` opens `NEW TASK CAPTURE` (23I).

```text
[STEP_ID]
STEP 24E: USAGE and README document the task type, the [t] key and neb plan import

[SCOPE_LOCK]
ALLOWED_FILES:
  - docs/USAGE.md
  - README.md
READ_ONLY_REFS:
  - crates/nebulous_cli/src/args.rs (Commands::Task, PlanCommand::Import)
  - crates/backend_core/src/mcp/mod.rs (plan_import tool definition)
  - crates/backend_core/src/models/project.rs:52-70 (id_scheme / IdScheme); crates/backend_core/src/models/item.rs:97 (format_hash_id)
  - Open Question 5 + STEP 24C D1, D5, D7 (field ownership as finally decided)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. STEP 22 docs reconciliation stays separate: don't rewrite, renumber or reword existing sections beyond the insertions below. Examples stay neutral (no real project, host or person names).

[PRE_GATE]
grep -q 'PlanCommand' crates/nebulous_cli/src/args.rs && grep -q '"plan_import"' crates/backend_core/src/mcp/mod.rs && ! grep -q 'neb plan import' docs/USAGE.md

[MUTATION_SPEC]
1. USAGE.md Part 1 §2 (G1): heading adds `neb task`; one `neb task "<title>"` line in its capture code block; a short list of when to use each type: bug = something broken; feature = new capability for the user; task = any other planned change (alias `neb chore`).
2. USAGE.md new `### 6. Plan Import (\`neb plan import\`)` after §5: the roadmap tables it reads (`## Step Execution Roadmap` → `### Up Next` with `# | Step | Command | Scope | Target Files | Waits on`, `### Completed` with `Step ID | Status | Command | Scope | Target Files`; `[USER]` rows skipped; COMPLETED in Waits on counts as done); dry run by default and `--write` to apply; `--phase`; the output lines; idempotency via `source_ref` (re-running reports UNCHANGED); field ownership: the plan sets title, type and scope at creation and updates the title on re-import, a COMPLETED row closes an open item, import never reopens a closed item and never changes stage, scope, severity, effort or `desc.md`; scope = the scope matching the most target files, tie or none → `default_scope`.
3. USAGE.md new `### 7. Item IDs`: sequential by default; `[project] id_scheme = "hash"` in project.toml opts in (missing key = sequential); format `BUG-7F3A2C` (prefix + 6 hex digits, always containing a letter); existing IDs never change; `neb sync-index` (and `neb sync` / `neb git pull`) warn when an ID exists in more than one item directory.
4. USAGE.md Keyboard Shortcuts: a row `| \`t\` | Open predictive \`New Task\` modal |` after the `b` / `f` row.
5. USAGE.md end of Part 3: a `plan_import` subsection: arguments `path` (relative to the project root), `phase`, `dry_run` (default true), unknown arguments rejected, returns the same report as `neb plan import --json`.
6. README.md Quick Start §1 code block: after the `neb feat` line, `neb task "<title>"` and `neb plan import STEPPED_IMPLEMENTATION_PLAN.md` (each with a one-line `#` comment like the lines around them).

[POST_GATE]
grep -q 'neb plan import' docs/USAGE.md && grep -q 'neb task' docs/USAGE.md && grep -q 'neb plan import' README.md && grep -q 'neb task' README.md && grep -q 'chore' docs/USAGE.md && grep -q 'plan_import' docs/USAGE.md && grep -q 'id_scheme = "hash"' docs/USAGE.md && grep -q 'source_ref' docs/USAGE.md && grep -q -- '--write' docs/USAGE.md && grep -q '^| `t` |' docs/USAGE.md && bash scripts/check-neutral.sh

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

# Phase 9 — `neb release` (NovOS STEP 13B)

Implements `/srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md` (NovOS STEP 13A, amended 2026-11-09). Every step reads the contract section it names. Nothing under `/srv/devshare/NovOS` is ever edited from this repo. NovOS resumes at its STEP 13C once `neb release --help | grep -q verify-variance && neb release self-test` passes.

Layout: one module `crates/backend_core/src/release/` (one file per concern, declared in `release/mod.rs`); the CLI is `crates/nebulous_cli/src/commands/release.rs`, and the MCP tools go in `crates/backend_core/src/mcp/release_tools.rs`. The existing component pin/rollback tools (`mcp/mod.rs` "Suite 3", `models/component.rs` PinnedReleaseConfig) track source components and are a separate concept. Leave them alone.

STEP 25A–25U and 27A–27E have Argon blocks (25I, 27A, 25J, 25K, 25L, 27B, 25M, 25N, 27C, 25O, 27D, 27E, 25P, 25Q, 25R and 25S written 2026-11-10; 25T and 25U written 2026-11-11; the old 25U (all handlers) was split into 25U, 27F, 27G and 27H so no step needs a stub; 27B is the manifest half of the ISO check, split from 25K; 27C is holds + pins, split from 25N; 27D/27E add the production-vs-test ISO identity that 25O lacked (NovOS STEP 15F derives the test ISO); 27A captures dpkg `Depends:`, which 25J's kernel lines need. 25F–25H written 2026-11-09: the old single ISO-extraction step was split in three, and the later steps shifted two letters, old 25G–25X → 25I–25Z). Claude writes the next blocks in small batches, just before they run, so each spec can cite the real code the previous steps produced. A roadmap row whose "Waits on" says "Argon block" is not runnable yet.

### STEP 25A: Release Module Skeleton — Exit Codes + JSON Envelope

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 25A: backend_core::release module: ExitCode enum (contract §1.1), JSON envelope (§1.2), Violation type

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/mod.rs (NEW)
  - crates/backend_core/src/lib.rs
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §1.1–§1.2 (lines 17-47)
  - /srv/devshare/NovOS/scripts/nebulous-ctl.py:37-41 (exit code values)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Nothing under /srv/devshare/NovOS is edited.

[PRE_GATE]
test ! -e crates/backend_core/src/release/mod.rs && grep -q 'fn write_versioned' crates/backend_core/src/storage/mod.rs

[MUTATION_SPEC]
1. lib.rs: `pub mod release;`, in alphabetical order among the existing `pub mod` lines.
2. release/mod.rs:
   - Module doc: "`neb release`: replaces NovOS scripts/nebulous-ctl.py and scripts/mcp_nebulous_ci.py. Spec: /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md."
   - `#[repr(i32)] #[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum ExitCode { Ok = 0, Error = 1, VarianceLock = 2, LoopLock = 3, PermissionDenied = 4, NotReady = 5, BisectStop = 6, StateConflict = 7 }` with `pub fn code(self) -> i32` and `pub fn name(self) -> &'static str` returning the contract's names (`EXIT_OK` … `EXIT_STATE_CONFLICT`). Also `pub fn from_error(e: &TrackerError) -> ExitCode`: `LockTimeout` → StateConflict, `UnsupportedSchema` → StateConflict, everything else → Error.
   - `pub const ENVELOPE_SCHEMA: &str = "neb.release/v1";` and status constants `STATUS_PASS = "PASS"`, `HARD_STOP_VARIANCE_LOCK`, `HARD_STOP_LOOP_LOCK`, `HARD_STOP_PERMISSION_DENIED` (each value equals its name).
   - `pub fn envelope(command: &str, status: &str, exit: ExitCode, body: serde_json::Value) -> serde_json::Value`: returns an object with `schema`, `command`, `status` and `exit_code`, plus every key of `body` when it is an object (envelope keys win on a clash). A non-object, non-null body goes under `data`.
   - `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct Violation { #[serde(rename = "type")] pub kind: String, pub item: String, pub detail: String }` with `pub fn new(kind, item, detail)`.
   - `#[derive(Debug, Clone)] pub struct Outcome { pub exit: ExitCode, pub status: String, pub body: serde_json::Value }` with `pub fn envelope(&self, command: &str) -> serde_json::Value`.
3. Inline tests: codes 0..=7 and names match the contract table; the envelope carries the 4 fields plus body keys; envelope keys override body keys; a Violation serializes its kind as `type`; from_error maps LockTimeout to 7.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'BisectStop = 6' crates/backend_core/src/release/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25B: Debian Version Comparison

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 25B: release::debver: pure Rust Debian version ordering (Policy §5.6.12) with an embedded dpkg-verified case table

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/debver.rs (NEW)
  - crates/backend_core/src/release/testdata/debver_cases.txt (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod debver;`)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/scripts/nebulous-ctl.py:65-191 (the pure comparator to port), :1282-1573 (self-tests: copy every version-compare case)
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §5.3
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. debver never shells out at runtime.

[PRE_GATE]
test -f crates/backend_core/src/release/mod.rs && test ! -e crates/backend_core/src/release/debver.rs && command -v dpkg

[MUTATION_SPEC]
1. debver.rs: `pub fn parse(v: &str) -> (u64, &str, &str)` (epoch, upstream, revision) per `_split_debian_version`, and `pub fn compare(a: &str, b: &str) -> std::cmp::Ordering`, a faithful port of `compare_debian_versions_pure`: `~` sorts before everything (even end of string), letters sort before non-letters, and digit runs compare numerically without integer overflow (strip leading zeros, compare length, then lexically).
2. testdata/debver_cases.txt: one case per line, `<a> <op> <b>`, with op in `lt`, `eq`, `gt`; `#` lines are comments. At least 60 cases:
   - every compare case from the Python self-tests;
   - kernel ABI versions (`7.2.15-x64v3-xanmod1` vs `7.3.3-x64v3-xanmod1`);
   - t64 transitions, epochs (`1:0.9` > `2.0`), tildes (`1.0~rc1` < `1.0`), and revisions (`1.0-1` < `1.0-1+b1`);
   - leading zeros, and a 25-digit number.
   Derive every op from the host's dpkg (`dpkg --compare-versions "$a" lt "$b"`, else `eq`, else `gt`). The file's first comment line records that command and the dpkg version (`dpkg --version | head -1`).
3. Inline test: `include_str!("testdata/debver_cases.txt")`. Every case's `compare` matches its op, and swapping the pair gives the reverse.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && test "$(grep -cvE '^(#|$)' crates/backend_core/src/release/testdata/debver_cases.txt)" -ge 60

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25C: Release Paths + `config.toml`

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 25C: release::config: ReleasePaths for every §1.3 file, ReleaseConfig (§5.4) with an unrelaxable forward-only policy (§5.2)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/config.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod config;`)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §1.3 (lines 49-65), §5.1 agent markers (lines 389-396), §5.2 (lines 398-404), §5.4 config.toml example (lines 414-437)
  - crates/backend_core/src/storage/mod.rs tracker_dir(), `pub fn is_valid_identifier` (:1155), `pub fn contained_path` (:1215); both are `pub`, so release/ calls them as `crate::storage::is_valid_identifier` / `crate::storage::contained_path`
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES.

[PRE_GATE]
test -f crates/backend_core/src/release/debver.rs && test ! -e crates/backend_core/src/release/config.rs

[MUTATION_SPEC]
1. `pub struct ReleasePaths { root: PathBuf }` with `pub fn new(storage: &TrackerStorage) -> Self` (root = `storage.tracker_dir().join("release")`). Methods returning PathBuf, named after the §1.3 files: `manifest`, `golden`, `approvals`, `releases_dir`, `candidate`, `failures`, `bisect`, `loop_state_dir`, `config`. Plus `pub fn loop_state_item(&self, id: &str) -> Result<PathBuf>`, which rejects ids that fail `is_valid_identifier` with ValidationError. There is no lock-file path: release commands use `TrackerStorage::with_write_lock`.
2. Serde structs whose field names equal the TOML keys, each struct carries the container attribute `#[serde(default)]` (on the struct, not per field), so a key missing inside a table takes that struct's Default value: `ReleaseConfig { guardrails: Guardrails, harness: Harness, components: Components }`, with `Guardrails` including `agent_env_markers: Vec<String>`, plus `Harness` and `Components`. `Default` reproduces the §5.4 example exactly, including the 5 default agent markers.
3. `pub fn load_config(paths: &ReleasePaths) -> Result<ReleaseConfig>`: a missing file gives `Default`. Otherwise parse, then validate: `variance_policy` must be "asymmetric_forward_only", and `allow_package_removal`, `allow_version_downgrade` and `allow_custom_tool_disable` must be false. Any violation → `ValidationError("release guardrail <key> is relaxed in <path>; the forward-only policy cannot be changed")`. Also (contract §5.1): `agent_env_markers` present but empty → `ValidationError("release guardrail agent_env_markers is empty in <path>; the user-only gate cannot be configured away")`. A missing `agent_env_markers` key takes the 5 defaults through `#[serde(default)]`. A ValidationError maps to exit 1 through `ExitCode::from_error`, which is the contract's startup refusal.
4. Inline tests: a missing file gives defaults; the §5.4 example parses equal to Default; each of the 4 relaxed keys errors with its key name; `agent_env_markers = []` errors; a `[guardrails]` table that sets only `loop_max_retries_hard_stop = 5` still has the 5 default markers; `loop_state_item("../x")` errors.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'agent_env_markers' crates/backend_core/src/release/config.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25D: User-Only Gate (tty + agent markers + confirmation word)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 25D: release::gate: require_user() for bless-golden, approve-variance, unlock-loop, import, bisect abort (contract §5.1)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/gate.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod gate;`)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §5.1
  - /srv/devshare/NovOS/scripts/nebulous-ctl.py:1262-1281 (the gate being replaced)
  - crates/backend_core/src/release/mod.rs ExitCode, HARD_STOP_PERMISSION_DENIED
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No new dependency (use std::io::IsTerminal).

[PRE_GATE]
test -f crates/backend_core/src/release/config.rs && test ! -e crates/backend_core/src/release/gate.rs

[MUTATION_SPEC]
1. `pub trait Terminal { fn stdin_is_tty(&self) -> bool; fn stdout_is_tty(&self) -> bool; fn var(&self, key: &str) -> Option<String>; fn prompt(&mut self, message: &str) -> std::io::Result<String>; }` and `pub struct RealTerminal;` implementing it with `std::io::IsTerminal`, `std::env::var`, and an `eprint!` prompt read with `stdin().read_line`.
2. `pub enum DenyReason { NotTty, AgentMarker(String), NotConfirmed }` and `pub struct GateDenied { pub reason: DenyReason }`, with Display ("HARD_STOP_PERMISSION_DENIED: <reason>") and `pub fn exit(&self) -> ExitCode` returning PermissionDenied.
3. `pub fn require_user(term: &mut dyn Terminal, markers: &[String], action: &str, confirm_word: &str) -> Result<(), GateDenied>`. Checks run in this order, stopping at the first failure:
   - stdin and stdout are both ttys, else NotTty;
   - no marker has a non-empty value, else AgentMarker(name);
   - the prompt "Type '<confirm_word>' to <action>: " returns the word (trimmed, exact), else NotConfirmed.
4. Inline tests with a FakeTerminal:
   - no tty → NotTty, and prompt is never called;
   - CLAUDECODE=1 → AgentMarker("CLAUDECODE");
   - a marker set to an empty string doesn't count;
   - a wrong word → NotConfirmed; the right word → Ok;
   - a custom marker list is honoured.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn require_user' crates/backend_core/src/release/gate.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25E: dpkg status + os-release Parsers, Inventory Types

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 25E: release::inventory: pure parsers for /var/lib/dpkg/status and /etc/os-release, plus the IsoInventory data types (no extraction yet)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/inventory.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod inventory;`)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/scripts/nebulous-ctl.py:291-323 (parse_os_release, parse_dpkg_status), :324-540 (extract_iso_inventory: which fields the inventory holds)
  - /srv/devshare/NovOS/.nebulous/golden-snapshot.toml (v1 table names and value shapes; read only, it is a protected file)
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §4.1 (component id/version rows)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No process spawning in this step (that's 25F).

[PRE_GATE]
test -f crates/backend_core/src/release/gate.rs && test ! -e crates/backend_core/src/release/inventory.rs

[MUTATION_SPEC]
1. `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct InstalledPackage { pub name: String, pub version: String, pub source: String, pub source_version: Option<String>, pub arch: Option<String> }`. `source` is the `Source:` name with any ` (ver)` suffix stripped (that ver goes into `source_version`); when `Source:` is absent, `source` = `name`.
2. `pub fn parse_dpkg_status(text: &str) -> BTreeMap<String, InstalledPackage>`: stanzas are separated by blank lines, and a line starting with a space or tab continues the previous field. Keep a stanza only when it has `Package:`, `Version:` and `Status:` and the Status value's whitespace-separated words include exactly `installed` (the Python rule: `"installed" in status.split()`). So `install ok installed` and `hold ok installed` are kept; `deinstall ok config-files` and `install ok half-installed` are dropped. `Architecture:` fills `arch`. A later stanza with the same Package name replaces the earlier one, as in the Python dict.
3. `pub fn parse_os_release(text: &str) -> BTreeMap<String, String>`: KEY=VALUE lines, with surrounding single or double quotes stripped and comments and blank lines skipped.
4. Data types only (all `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`), mirroring the dict `extract_iso_inventory` returns (nebulous-ctl.py:518-538) minus its `schema_version` (the golden writer owns that, STEP 25I):
   - `IsoMeta { path: String, size_bytes: u64, vmlinuz_sha256: String, initrd_sha256: String, extracted_at: String }`. A missing kernel or initrd hashes to `""` in the Python; keep that, not Option.
   - `BootInfo { files: Vec<String> }` (sorted).
   - `CalamaresModules { configs: Vec<String>, custom_modules: Vec<String>, presence: BTreeMap<String, bool> }` (lists sorted).
   - `CustomBinary { path: String, present: bool, sha256: String, size_bytes: u64, version: String, is_symlink: bool, flags: Vec<String> }`. The Python falls back to version "0.1.0", so version is never absent.
   - `IsoInventory { iso: IsoMeta, os_release: BTreeMap<String, String>, boot: BootInfo, calamares_modules: CalamaresModules, custom_binaries: BTreeMap<String, CustomBinary>, packages: BTreeMap<String, InstalledPackage> }`. Field names equal the v1 golden table names.
5. Inline tests: a fixture status file with 4 stanzas (one deinstalled, one with `Source: foo (1.2-1)`, one with a multi-line Description, one plain) gives the 3 kept packages with correct source / source_version; quoted os-release values come back unquoted.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn parse_dpkg_status' crates/backend_core/src/release/inventory.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25F: Extracted-Root Helpers + Custom Binary Scan

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 25F: release/rootfs.rs: symlink resolution confined to an extracted root, sha256, workspace crate versions, and the /usr/local/{bin,sbin} custom binary scan

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/rootfs.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod rootfs;`)
  - crates/backend_core/Cargo.toml (only `regex = "1"` in [dependencies], alphabetical)
  (Cargo.lock changes only as cargo's own result; regex 1.13.1 is already locked through fancy-regex, so no new crate is added.)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/scripts/nebulous-ctl.py:257-289 (sha256_file, _resolve_in_extract_root), :433-490 (crate_versions map at :433-446, the custom binary loop at :448-490)
  - crates/backend_core/src/release/inventory.rs:36-45 (CustomBinary)
  - crates/backend_core/src/error.rs:11 (Io), :83 (ValidationError)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No process spawning (STEP 25H). Nothing under /srv/devshare/NovOS is edited.

[PRE_GATE]
test -f crates/backend_core/src/release/inventory.rs && test ! -e crates/backend_core/src/release/rootfs.rs

[MUTATION_SPEC]
1. `pub fn resolve_in_root(root: &Path, entry: &Path) -> PathBuf`: a faithful port of `_resolve_in_extract_root`. Normalize paths lexically (resolve `.` and `..` without touching the filesystem; write a private `normalize_lexically` helper). While `current` is a symlink (`fs::symlink_metadata`): stop if it was already visited; read the link; an absolute target resolves against `root` (`root.join(target stripped of its leading /)`), a relative one against `current`'s parent; normalize. If the result is not inside `root`, use `root.join(candidate.file_name())`, as the Python does. Never follow a link to a host path.
2. `pub fn sha256_file(path: &Path) -> Result<String>`: streaming `sha2::Sha256` over 64 KiB reads, lowercase hex. I/O errors → `TrackerError::Io { path, source }`.
3. `pub fn workspace_crate_versions(project_root: &Path) -> BTreeMap<String, String>`: read `project_root/Cargo.toml`; for each string in `workspace.members` (literal paths, no glob expansion, as in the Python), read `<member>/Cargo.toml` and take `package.name` and `package.version`. A string version is used as is. `version = { workspace = true }` uses `[workspace.package].version` when that is a string (the Python would record the table's text; this is a deliberate fix). Any other shape, or a missing or unparsable file, skips that member. A missing or unparsable root Cargo.toml gives an empty map. This function never errors.
4. `pub fn scan_custom_binaries(root: &Path, crate_versions: &BTreeMap<String, String>) -> Result<BTreeMap<String, CustomBinary>>`: port the loop at nebulous-ctl.py:448-490. For `usr/local/bin` then `usr/local/sbin` under `root` (skip a missing dir): entries sorted by name (lossy UTF-8). `is_symlink` comes from the raw entry's symlink_metadata. `resolved = resolve_in_root(root, raw)`; skip if `resolved` is not a regular file. Fields:
   - `path = "/<rel_dir>/<name>"`, `present = true`, `sha256` and `size_bytes` of `resolved`;
   - `version`: `crate_versions[name]`, else `crate_versions[resolved file name]`, else `"0.1.0"`; then, if the file's bytes match `regex::bytes::Regex` `(?-u)\b<regex::escape(name)>\s+v?(\d+\.\d+\.\d+(?:[-+~][0-9A-Za-z._-]+)?)\b`, capture group 1 replaces it (lossy UTF-8);
   - `flags = ["--pre-packages"]` when the bytes contain `--pre-packages`, else empty.
   A read error on `resolved` while probing keeps the defaults (the Python's `except OSError: pass`). An entry name in sbin that also exists in bin replaces it (later wins, as in the Python dict). Note in a doc comment that `(?-u)` makes `\b` ASCII-only, while Python's latin1 text treats non-ASCII letters as word characters; the difference only matters for non-ASCII bytes around the name.
5. Inline tests (tempdir root):
   - an absolute symlink `usr/local/bin/link -> /usr/local/bin/tool-a` resolves to `root/usr/local/bin/tool-a`; a relative symlink that climbs out (`../../../../../etc/passwd`) resolves to `root/passwd`; a two-link cycle terminates.
   - scan: `tool-a` (text `tool-a v1.2.3-rc1 --pre-packages`) → version `1.2.3-rc1`, flags `["--pre-packages"]`; `tool-b` (no version text) with crate map `{tool-b: 0.4.0}` → `0.4.0`; `tool-c` (no text, no crate entry) → `0.1.0`; `link` → `is_symlink = true` with tool-a's sha256; an escaping symlink in sbin is skipped (its target does not exist inside root); a sbin entry with the same name as a bin entry wins.
   - sha256_file of `b"abc"` is `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad`.
   - workspace_crate_versions: a fixture workspace with one member using a string version and one using `version.workspace = true` (with `[workspace.package] version = "2.0.0"`) gives both; a member with a missing Cargo.toml is skipped; no root Cargo.toml → empty map.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn scan_custom_binaries' crates/backend_core/src/release/rootfs.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25G: Inventory From an Extracted Root

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 25G: rootfs::inventory_from_root builds an IsoInventory from an already-extracted ISO root, including the live-boot and COSMIC synthetic package entries

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/rootfs.rs
  - crates/backend_core/Cargo.toml (only `flate2 = "1"` in [dependencies], alphabetical)
  (Cargo.lock changes only as cargo's own result; flate2 1.1.10 is already locked through png, so no new crate is added.)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/scripts/nebulous-ctl.py:49-58 (COSMIC_EPOCH_COMPONENTS), :388-431 (status check :388-392, os-release :394-404, dpkg status :406-409, live-boot :411-422, COSMIC :424-431), :492-538 (Calamares :492-507, boot :509-516, returned dict :518-538)
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §4.1 (component id and version come from Source)
  - crates/backend_core/src/release/inventory.rs:6-56 (types), :138 parse_dpkg_status, :165 parse_os_release
  - crates/backend_core/src/release/rootfs.rs resolve_in_root, workspace_crate_versions, scan_custom_binaries (STEP 25F)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No process spawning (STEP 25H).

[PRE_GATE]
grep -q 'pub fn scan_custom_binaries' crates/backend_core/src/release/rootfs.rs && ! grep -q 'pub fn inventory_from_root' crates/backend_core/src/release/rootfs.rs

[MUTATION_SPEC]
1. `pub const COSMIC_EPOCH_COMPONENTS: [&str; 8]`, the same 8 names in the same order as the Python.
2. `pub fn inventory_from_root(root: &Path, iso: IsoMeta, project_root: &Path) -> Result<IsoInventory>`, in this order:
   - `root/var/lib/dpkg/status` must be a regular file, else `ValidationError("var/lib/dpkg/status is missing from the extracted ISO root <root>")`.
   - os_release: `resolve_in_root(root, root/etc/os-release)`; if that is not a file, use `root/usr/lib/os-release`; if neither is a file, an empty map. Read as lossy UTF-8 and `parse_os_release`.
   - packages: `parse_dpkg_status` of the status file (lossy UTF-8).
   - live-boot: only if `packages` has no `live-boot`, `root/usr/share/doc/live-boot/changelog.gz` is a regular file, and `root/usr/bin/live-boot` exists as a file or a symlink (symlink_metadata, never followed). Decompress with `flate2::read::GzDecoder`, take the first line (lossy, trimmed) and the text between the first `(` and the next `)`, trimmed. If found, insert `InstalledPackage { name: "live-boot", version, source: "live-boot", source_version: None, arch: None }`. A gzip read error → `TrackerError::Io` (the Python raises here too).
   - COSMIC: if `packages` has `cosmic-epoch-monorepo` with version V, then for each name in COSMIC_EPOCH_COMPONENTS that is not already in `packages` and exists as `root/usr/bin/<name>` (file or symlink, symlink_metadata), insert `InstalledPackage { name, version: V, source: "cosmic-epoch-monorepo", source_version: Some(V), arch: None }`. Source is the monorepo because it ships those binaries; that makes them members of the monorepo component under §4.1. Doc comment: why.
   - custom_binaries: `scan_custom_binaries(root, &workspace_crate_versions(project_root))?`.
   - calamares_modules: `configs` = every entry name in `root/etc/calamares/modules`, sorted; `custom_modules` = entry names in `root/usr/lib/calamares/modules` that are directories, sorted; `presence[name] = true` for every name in either list. Missing dirs give empty lists.
   - boot: `files` = entry names in `root/boot` that are regular files or symlinks, sorted.
   - Return `IsoInventory { iso, os_release, boot, calamares_modules, custom_binaries, packages }`.
3. `#[cfg(test)] pub(crate) fn write_fixture_root(dir: &Path)` (STEP 25H's round-trip test reuses it) writes a fixture root:
   - var/lib/dpkg/status with `cosmic-epoch-monorepo 1.0.0~alpha7-1`, `libfoo1` (Source `foo (1.2-1)`) and `base-files 14`, all installed;
   - usr/bin/cosmic-comp and usr/bin/cosmic-term (files) and usr/bin/cosmic-panel (a symlink to cosmic-comp);
   - usr/bin/live-boot plus usr/share/doc/live-boot/changelog.gz whose first line is `live-boot (1:20250815) unstable; urgency=medium` (write it with `flate2::write::GzEncoder`);
   - usr/lib/os-release (`NAME="NovOS"`, `ID=novos`) and etc/os-release as a relative symlink to `../usr/lib/os-release`;
   - usr/local/bin/tool-a (`tool-a v1.2.3`);
   - etc/calamares/modules/{welcome.conf,users.conf} and usr/lib/calamares/modules/limine-bootloader/ (a dir);
   - boot/vmlinuz-7.3.3-x64v3-xanmod1 (file) and boot/vmlinuz (a symlink to it).
4. Inline tests using write_fixture_root:
   - the 3 dpkg packages, plus live-boot `1:20250815`, plus cosmic-comp, cosmic-term and cosmic-panel at `1.0.0~alpha7-1` with source `cosmic-epoch-monorepo`; cosmic-files (no binary) is absent;
   - os_release NAME is `NovOS` through the symlink;
   - calamares configs `[users.conf, welcome.conf]`, custom_modules `[limine-bootloader]`, presence has all 3;
   - boot files `[vmlinuz, vmlinuz-7.3.3-x64v3-xanmod1]`; custom_binaries has tool-a at `1.2.3`;
   - a dpkg-registered live-boot is not replaced by the changelog version;
   - a root without var/lib/dpkg/status → ValidationError.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn inventory_from_root' crates/backend_core/src/release/rootfs.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25H: ISO Extraction via xorriso + unsquashfs

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 25H: release/iso.rs: extract_iso_inventory runs xorriso and unsquashfs unprivileged into a temp dir and returns the IsoInventory

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/iso.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod iso;`)
  - crates/backend_core/Cargo.toml (move `tempfile = "3"` from [dev-dependencies] to [dependencies], alphabetical; the tests keep using it)
  (Cargo.lock changes only as cargo's own result; tempfile is already locked.)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/scripts/nebulous-ctl.py:324-386 (extract_iso_inventory: xorriso call :342-362, hashes :364-365, unsquashfs targets :368-380, unsquashfs call :381-386), :518-525 (the returned `iso` table)
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §2.1 (exit 1 on an unreadable ISO or a failed extraction), §5.3 (only xorriso and unsquashfs, unprivileged, temp dir; no loop mounts, no root)
  - crates/backend_core/src/release/rootfs.rs inventory_from_root, sha256_file, COSMIC_EPOCH_COMPONENTS, `#[cfg(test)] write_fixture_root` (STEP 25G)
  - crates/backend_core/src/release/inventory.rs:15-22 (IsoMeta)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No sudo, no mount, no losetup. Spawn only `xorriso`, `unsquashfs`, and (tests only) `mksquashfs`.

[PRE_GATE]
grep -q 'pub fn inventory_from_root' crates/backend_core/src/release/rootfs.rs && test ! -e crates/backend_core/src/release/iso.rs

[MUTATION_SPEC]
1. `pub fn extract_iso_inventory(iso: &Path, project_root: &Path) -> Result<IsoInventory>`:
   - `iso` must be a regular file, else `ValidationError("ISO file not found: <iso>")`. `iso_abs = fs::canonicalize(iso)` (the Python records the resolved path).
   - Temp dir: `tempfile::Builder::new().prefix("neb-iso-").tempdir()`; it is removed when dropped, on success and on error.
   - Run `xorriso -osirrox on -indev <iso_abs> -extract /live/filesystem.squashfs <tmp>/filesystem.squashfs -extract /live/vmlinuz <tmp>/vmlinuz -extract /live/initrd.img <tmp>/initrd.img` with `std::process::Command` (args as separate items, never a shell string), capturing output. A spawn error of kind NotFound → `ValidationError("xorriso is not installed (needed to read the ISO)")`; any other spawn error → `Io`; a non-zero exit → `ValidationError("xorriso failed on <iso>: <last 20 lines of stderr>")`.
   - `vmlinuz_sha256` / `initrd_sha256` = `sha256_file` when that file exists, else `""`.
   - Run `unsquashfs -d <tmp>/extract <tmp>/filesystem.squashfs` followed by the exact target list from nebulous-ctl.py:368-380 (the 9 fixed paths, then `usr/bin/<name>` for each COSMIC_EPOCH_COMPONENTS entry, then `boot`), in that order. As in the Python, its exit status is ignored (some targets may not exist); only a spawn failure errors, with the same NotFound message shape naming unsquashfs. The missing-status check in inventory_from_root catches a failed extraction.
   - `IsoMeta { path: iso_abs as lossy string, size_bytes: metadata len, vmlinuz_sha256, initrd_sha256, extracted_at: chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Micros, false) }` (this gives the Python's `+00:00` isoformat shape).
   - Return `rootfs::inventory_from_root(&<tmp>/extract, meta, project_root)`.
2. Doc comment on the fn: unprivileged; no mounts; the temp dir is deleted afterwards; contract §5.3.
3. Tests in iso.rs:
   - a missing ISO path → ValidationError containing "ISO file not found";
   - a 4 KiB file of zero bytes passed as the ISO → Err (ValidationError, from xorriso failing or not being installed), and no `neb-iso-*` dir is left in `std::env::temp_dir()` that was created by this call (compare the set of `neb-iso-*` entries before and after);
   - `#[ignore = "needs xorriso and squashfs-tools; run: cargo test -p backend_core iso_round_trip -- --ignored"]` round trip: `write_fixture_root` into a tempdir; `mksquashfs <root> <isodir>/live/filesystem.squashfs -noappend -quiet`; write `<isodir>/live/vmlinuz` (`b"kernel"`) and `<isodir>/live/initrd.img` (`b"initrd"`); `xorriso -as mkisofs -quiet -o <tmp>/t.iso <isodir>`; then extract_iso_inventory gives the same `packages` as `inventory_from_root` on the fixture root directly, `vmlinuz_sha256 == sha256 of b"kernel"`, and `iso.path` is absolute.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn extract_iso_inventory' crates/backend_core/src/release/iso.rs && ! grep -nE '"(sudo|mount|losetup)"' crates/backend_core/src/release/iso.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25I: Golden v1 Reader, Golden v2 Model + Writer

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 25I: release/golden.rs: Golden v2 model (contract §4.1), a reader that accepts v1 and v2, a deterministic atomic writer

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/golden.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod golden;`, alphabetical: after `gate`)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §1.3 (lines 51-65), §4.1 (lines 205-235: component rules and the v2 example), §6.4 (line 463: v1 → v2 is import's job, STEP 25R)
  - /srv/devshare/NovOS/scripts/nebulous-ctl.py:518-538 (the v1 inventory dict), :541-586 (write_golden_snapshot_toml: the v1 file layout, `[packages]` is name = version)
  - crates/backend_core/src/release/inventory.rs:5-54 (IsoMeta, BootInfo, CalamaresModules, CustomBinary, IsoInventory)
  - crates/backend_core/src/release/rootfs.rs sha256_file (:114), inventory_from_root (:333), `#[cfg(test)] write_fixture_root` (:507)
  - crates/backend_core/src/storage/atomic.rs:94 (atomic_write_str), crates/backend_core/src/error.rs:91 (UnsupportedSchema)
  - crates/backend_core/src/release/config.rs:175-193 (load_config: the read + TomlDeserialize { path: Some } pattern to copy)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No component grouping here (STEP 25J); no v1 → v2 conversion (STEP 25R). Nothing takes the tracker lock here; callers do.

[PRE_GATE]
test ! -e crates/backend_core/src/release/golden.rs && grep -q 'pub struct IsoInventory' crates/backend_core/src/release/inventory.rs && grep -q 'pub fn atomic_write_str' crates/backend_core/src/storage/atomic.rs

[MUTATION_SPEC]
1. Types (all `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]` unless stated):
   - `pub const GOLDEN_SCHEMA_VERSION: u32 = 2;`
   - `#[derive(.., Copy, PartialOrd, Ord, ..)] #[serde(rename_all = "snake_case")] pub enum Origin { Debian, ThirdParty }` with `pub fn as_str(self) -> &'static str` ("debian" / "third_party").
   - `pub struct GoldenComponent { pub version: String, pub origin: Origin, #[serde(default, skip_serializing_if = "is_false")] pub kernel_line: bool, #[serde(default, skip_serializing_if = "is_false")] pub source_unknown: bool, pub binaries: BTreeMap<String, String> }` (`fn is_false(b: &bool) -> bool` private).
   - `pub struct Golden { pub schema_version: u32, pub iso: IsoMeta, pub os_release: BTreeMap<String, String>, pub boot: BootInfo, pub calamares_modules: CalamaresModules, pub custom_binaries: BTreeMap<String, CustomBinary>, pub packages: BTreeMap<String, String>, #[serde(default)] pub components: BTreeMap<String, GoldenComponent> }`. Field order as written: `schema_version` first so it renders above every table.
2. `impl Golden { pub fn from_inventory(inv: &IsoInventory, components: BTreeMap<String, GoldenComponent>) -> Self }`: schema_version = GOLDEN_SCHEMA_VERSION, the five v1 tables cloned from `inv`, `packages` = each package name → its binary `version`.
3. `pub fn parse_golden(text: &str, path: &Path) -> Result<Golden>`:
   - Parse to `toml::Table` first (error → `TomlDeserialize { path: Some(path) }`). `schema_version`: missing → 1 (v1 files written by hand may lack it); not an integer, or < 1 → `ValidationError("<path>: schema_version must be a positive integer")`; > GOLDEN_SCHEMA_VERSION → `UnsupportedSchema { path, found, supported: GOLDEN_SCHEMA_VERSION }` (exit 7 via ExitCode::from_error).
   - Then deserialize the table into `Golden` (same TomlDeserialize mapping). The returned `schema_version` is the file's value, so callers can tell a v1 golden (components empty) from a v2 one.
4. `pub fn read_golden(path: &Path) -> Result<Option<Golden>>`: `Ok(None)` when the file does not exist (the Python treats a missing golden as "no golden checks"); an I/O error → `Io { path }`; else `parse_golden`.
5. `pub fn render_golden(golden: &Golden) -> Result<String>`:
   - Refuse with `ValidationError("golden has packages but no components; convert it with neb release import")` when `packages` is non-empty and `components` is empty (writing such a file would silently disable component checks).
   - Two header lines `# NovOS golden inventory (.tracker/release/golden.toml), schema v2` and `# Written by neb release. Do not edit by hand.`, a blank line, then `toml::to_string` of a copy with `schema_version = GOLDEN_SCHEMA_VERSION`. Output must be byte-identical for equal input (BTreeMaps only; no timestamps added here).
6. `pub fn write_golden(path: &Path, golden: &Golden) -> Result<String>`: render; `create_dir_all` on the parent (Io on error); `crate::storage::atomic_write_str(path, &text)`; return the lowercase hex sha256 of the bytes written (use `sha2` as rootfs.rs does). Doc comment: the caller holds `with_write_lock`; the return value is the release record's `golden_sha256` (§4.6).
7. Tests in golden.rs (no real paths or hostnames: ISO path `/tmp/neb-test/NovOS-sid-amd64.iso`, os-release URLs under `https://example.com/`):
   - v1 parse: an inline fixture in the exact v1 layout of nebulous-ctl.py:541-586 (header comments, `schema_version = 1`, `[iso]`, `[os_release]`, `[boot]`, `[calamares_modules]` + `[calamares_modules.presence]` with quoted keys like `"fstab.conf" = true`, `[custom_binaries]` as inline tables with all 7 fields, one with `flags = ["--pre-packages"]`, `[packages]` with `linux-image-amd64 = "7.3.3-1"` and a quoted key `"linux-image-7.3.3+deb14-amd64" = "7.3.3-1"`) → schema_version 1, components empty, those values read back.
   - v1 without a `schema_version` line parses as 1.
   - round trip: inventory_from_root on write_fixture_root + `Golden::from_inventory` with two components (one `Origin::Debian`; one `Origin::ThirdParty` with `kernel_line: true` and 2 binaries) → render → parse_golden equals the original; rendering twice gives identical strings; the text contains `schema_version = 2` and `origin = "third_party"` and no `source_unknown`; setting `source_unknown = true` on one component makes `source_unknown = true` appear.
   - `schema_version = 3` → UnsupportedSchema, and `ExitCode::from_error` gives StateConflict; `schema_version = "two"` → ValidationError.
   - read_golden on a missing path → Ok(None).
   - render_golden refuses packages-without-components; write_golden's returned sha equals `rootfs::sha256_file(path)`, and the parent dir has no leftover temp file (only `golden.toml` in a fresh tempdir).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn write_golden' crates/backend_core/src/release/golden.rs && grep -q 'GOLDEN_SCHEMA_VERSION: u32 = 2' crates/backend_core/src/release/golden.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 27A: Capture `Depends:` on Installed Packages

* **Recommended Command:** `/goal`

Why its own step (added 2026-11-10): STEP 25J groups a kernel line as the meta package plus the `linux-image-<abi>` / `linux-headers-<abi>` packages it depends on (contract §4.1). Versions can't stand in for Depends: in the 2026-11-08 golden the XanMod meta is `linux-xanmod-x64v3 = "7.3.3-xanmod1-0"` while its image is `7.3.3-x64v3-xanmod1-0~20261107.g2ef8115`. The 25x letters are used up, so this step takes the free ID 27A; the **#** column is still the order.

```text
[STEP_ID]
STEP 27A: InstalledPackage.depends: package names from the dpkg Depends field

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/inventory.rs
  - crates/backend_core/src/release/rootfs.rs (only `depends: Vec::new()` in its InstalledPackage literals)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §4.1 (line 211, "Kernel line")
  - crates/backend_core/src/release/inventory.rs:5-12 (InstalledPackage), :85-138 (flush_stanza), :140-163 (parse_dpkg_status: continuation lines are joined with '\n')
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. write_fixture_root's dpkg status stays as it is (rootfs tests count its 7 packages).

[PRE_GATE]
test -f crates/backend_core/src/release/golden.rs && ! grep -q 'pub depends' crates/backend_core/src/release/inventory.rs

[MUTATION_SPEC]
1. InstalledPackage gains a last field `#[serde(default, skip_serializing_if = "Vec::is_empty")] pub depends: Vec<String>`.
2. flush_stanza reads `Depends` (case-insensitive, like the other keys) and fills `depends` with `pub fn parse_depends(raw: &str) -> Vec<String>`: split on `,`, then each part on `|` (every alternative counts); per alternative, trim, keep the text before the first whitespace or `(`, then drop a `:<arch>` suffix (`libc6:any` → `libc6`); skip empty names; sort and dedup. Continuation lines (the '\n' the parser joins in) are whitespace and need no special case. `Pre-Depends` is not read.
3. Every other InstalledPackage literal (rootfs.rs live-boot and COSMIC synthetic packages, and the test literals in both files) gets `depends: Vec::new()`.
4. Tests in inventory.rs:
   - parse_depends on `linux-image-7.3.3-x64v3-xanmod1 (= 7.3.3-x64v3-xanmod1-0~20261107.g2ef8115), linux-headers-7.3.3-x64v3-xanmod1 | foo, libc6:any (>= 2.36)` → `["foo", "libc6", "linux-headers-7.3.3-x64v3-xanmod1", "linux-image-7.3.3-x64v3-xanmod1"]`; on `""` → empty.
   - a status stanza with a Depends line split over a continuation line → both names land in `depends`; a stanza without Depends → empty.
   - `serde_json::to_value` of a package with empty depends has no `depends` key.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub depends: Vec<String>' crates/backend_core/src/release/inventory.rs && grep -q 'pub fn parse_depends' crates/backend_core/src/release/inventory.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25J: Component Grouping, Kernel Lines, ABI Rename, Imported-Placeholder Match

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 25J: release/component.rs: group an inventory into §4.1 components, ABI-rename matching, placeholder-aware golden → new matching

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/component.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod component;`, alphabetical: before `config`)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §4.1 (lines 205-235), §6.4 (line 463)
  - crates/backend_core/src/release/golden.rs Golden, GoldenComponent, Origin, Golden::from_inventory (STEP 25I)
  - crates/backend_core/src/release/inventory.rs InstalledPackage (incl. `depends`, STEP 27A), IsoInventory
  - crates/backend_core/src/release/config.rs:156-172 (Components: kernel_lines, third_party)
  - crates/backend_core/src/release/debver.rs:29 (compare)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No variance verdicts or violations here (STEP 25K); this step only groups and matches.

[PRE_GATE]
test ! -e crates/backend_core/src/release/component.rs && grep -q 'pub struct GoldenComponent' crates/backend_core/src/release/golden.rs && grep -q 'pub depends: Vec<String>' crates/backend_core/src/release/inventory.rs

[MUTATION_SPEC]
1. `pub fn abi_key(name: &str) -> String`: drop one trailing `t64`, then replace every run of ASCII digits with `#`. `pub fn is_abi_rename(old: &str, new: &str) -> bool`: `old != new && abi_key(old) == abi_key(new)`.
2. `pub fn group_components(packages: &BTreeMap<String, InstalledPackage>, cfg: &Components) -> BTreeMap<String, GoldenComponent>`:
   a. Kernel lines first, in `cfg.kernel_lines` order, for each name that is installed: members = the meta plus every name in its `depends` that is installed, starts with `linux-image-` or `linux-headers-`, is not itself a kernel_lines entry, and is not already claimed by an earlier line. id = meta name; version = the meta's binary version; `kernel_line: true`; binaries = member name → binary version. Members are claimed and skipped in (b).
   b. Every other package joins the component named by its `source` (InstalledPackage already falls back to the package name). Per binary the candidate version is `source_version` when Some, else `version`; the component version is the highest candidate by `debver::compare`. binaries = name → binary `version`.
   c. origin = `Origin::ThirdParty` when the component id or any of its binary names is in `cfg.third_party`, else `Origin::Debian` (contract: apt origin data is not captured yet; the config list is the fallback). `source_unknown: false`.
3. `pub fn golden_from_inventory(inv: &IsoInventory, cfg: &Components) -> Golden` = `Golden::from_inventory(inv, group_components(&inv.packages, cfg))`.
4. Matching a golden component to the new set:
   - `pub struct ComponentMatch<'a> { pub id: &'a str, pub component: &'a GoldenComponent, pub version: &'a str }`.
   - `pub fn find_match<'a>(golden_id: &str, golden: &GoldenComponent, new: &'a BTreeMap<String, GoldenComponent>) -> Option<ComponentMatch<'a>>`: for `golden.source_unknown` (an imported placeholder, id = a binary name), the first new component in id order whose `binaries` contains `golden_id`, with `version` = that binary's version; otherwise `new.get(golden_id)` with `version` = its component version. None when nothing matches.
5. `pub struct BinaryChanges { pub abi_renames: Vec<(String, String)>, pub dropped: Vec<String> }` and `pub fn binary_changes(old: &GoldenComponent, new: &GoldenComponent) -> BinaryChanges`: vanished = old binaries not in new, appeared = new binaries not in old (both sorted); each vanished name pairs with the first still-unpaired appeared name where `is_abi_rename` holds → `abi_renames (from, to)`; unpaired vanished names → `dropped`. Appeared names with no partner are additions and not reported.
6. Tests in component.rs (InstalledPackage literals with `depends`; versions from the 2026-11-08 golden):
   - is_abi_rename true for `linux-image-7.2.15-x64v3-xanmod1` → `linux-image-7.3.3-x64v3-xanmod1`, `libfoo1` → `libfoo2`, `libpcap0.8` → `libpcap0.8t64`; false for `libfoo1` → `libbar1` and for identical names.
   - XanMod line: meta `linux-xanmod-x64v3` 7.3.3-xanmod1-0 depending on `linux-image-7.3.3-x64v3-xanmod1` and `linux-headers-7.3.3-x64v3-xanmod1` (both 7.3.3-x64v3-xanmod1-0~20261107.g2ef8115) → one component, version 7.3.3-xanmod1-0, kernel_line, 3 binaries; neither member appears in another component; with `third_party = ["linux-xanmod-x64v3"]` its origin is ThirdParty.
   - Debian line: `linux-image-amd64` (source `linux-signed-amd64`) 7.3.3-1 depending on `linux-image-7.3.3+deb14-amd64` → a 2-binary kernel line; `linux-headers-amd64` (source `linux`, not depended on) stays in component `linux`.
   - Source grouping: two binaries with `Source: foo (1.2-1)` and versions 1.2-1 and 1.2-1+b1 → component `foo`, version 1.2-1 (the source version); two binaries of source `bar` with no source_version at 2.0-1 and 2.0-2 → version 2.0-2.
   - find_match: placeholder `libpcap0.8` (source_unknown, version 1.10.5-2) finds new component `libpcap` containing `libpcap0.8` and returns that binary's version; a placeholder whose binary is gone → None; a normal component matches by id.
   - binary_changes: the XanMod 7.2.15 → 7.3.3 pair gives 2 renames and nothing dropped; `libpcap0.8` → `libpcap0.8t64` is a rename; a vanished `libfoo-doc` with no partner is dropped.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn group_components' crates/backend_core/src/release/component.rs && grep -q 'pub fn binary_changes' crates/backend_core/src/release/component.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25K: Approvals + Asymmetric Variance by Component

* **Recommended Command:** `/boost`

Scope note (2026-11-10): this step ports only the golden half of `evaluate_asymmetric_variance` (nebulous-ctl.py:958-1049). The manifest half (:846-956: os-release, manifest packages, binaries, Calamares) needs the manifest model from STEP 25L and is added there. Known edge from 25J, left as is: a package whose `Source:` equals a kernel-line meta name is merged into that kernel-line component; no real XanMod or Debian package does this.

```text
[STEP_ID]
STEP 25K: release/approvals.rs (approvals.toml read/append, version-scoped cover check) + release/variance.rs (forward-only check of a live Golden against golden, by component)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/approvals.rs (NEW)
  - crates/backend_core/src/release/variance.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod approvals;` and `pub mod variance;`, alphabetical)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §1.2 (line 47, violation shape), §1.3 (line 57, approvals.toml), §2.2 (lines 84-92), §2.6 (lines 123-130), §4.1 (lines 205-235), §5.2 (lines 398-404)
  - /srv/devshare/NovOS/scripts/nebulous-ctl.py:593-608 (load_approved_exceptions), :810-823 (_index_custom_binaries), :958-1049 (golden checks: the detail strings to copy), :1586-1643 (cmd_verify_variance: counts come from the live inventory), :1715-1743 (approve-variance writer)
  - crates/backend_core/src/release/component.rs find_match (:148), binary_changes (:180), golden_from_inventory (:137)
  - crates/backend_core/src/release/golden.rs Golden (:47), GoldenComponent (:36)
  - crates/backend_core/src/release/mod.rs Violation (:93, `Violation::new(kind, item, detail)`)
  - crates/backend_core/src/release/debver.rs:29 (compare), crates/backend_core/src/storage/atomic.rs:94 (atomic_write_str)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No manifest checks (STEP 25L), no holds / `held[]` (STEP 25N), no CLI or JSON envelope (STEP 25U), no tty gate (approve-variance's handler applies gate.rs). Nothing takes the tracker lock here; callers do.

[PRE_GATE]
test ! -e crates/backend_core/src/release/variance.rs && test ! -e crates/backend_core/src/release/approvals.rs && grep -q 'pub fn binary_changes' crates/backend_core/src/release/component.rs && grep -q 'pub fn find_match' crates/backend_core/src/release/component.rs

[MUTATION_SPEC]
1. approvals.rs:
   - `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct Approval { pub item_name: String, pub reason: String, #[serde(default = "default_true")] pub approved: bool, #[serde(default)] pub timestamp: String, #[serde(default, skip_serializing_if = "Option::is_none")] pub version: Option<String> }`.
   - `#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)] pub struct Approvals { #[serde(default)] pub approvals: Vec<Approval> }`.
   - `pub fn read_approvals(path: &Path) -> Result<Approvals>`: missing file → `Approvals::default()`; I/O error → Io; parse error → `TomlDeserialize { path: Some }`. (The Python swallows a parse error and returns no approvals; neb reports it instead, exit 1.)
   - `impl Approvals { pub fn covers(&self, item: &str, target_version: Option<&str>) -> bool }`: true when some entry has `approved`, a non-blank `reason`, `item_name == item`, and either `version` is None (open-ended, as today) or `target_version == Some(version)`. The target version is the version the check is about: the live version for a downgrade, the golden version for a removal, None for a Calamares module (so a versioned approval never covers a module).
   - `pub fn render_approvals(a: &Approvals) -> Result<String>`: header `# NovOS approved variance exceptions (.tracker/release/approvals.toml)`, `# Written by neb release approve-variance. Remove an entry by hand to revoke it.`, a blank line, then `toml::to_string(a)`.
   - `pub fn append_approval(path: &Path, approval: Approval) -> Result<()>`: read_approvals, push, render, `create_dir_all` on the parent, `atomic_write_str`. Doc comment: the caller holds `with_write_lock` and has passed the user-only gate (§5.1).
2. variance.rs types (`#[derive(Debug, Clone, PartialEq, Eq, Serialize)]`):
   - `pub struct Upgrade { pub item: String, pub from_version: String, pub to_version: String }`
   - `pub struct AbiRename { pub component: String, pub from_package: String, pub to_package: String }`
   - `pub struct VarianceReport { pub violations: Vec<Violation>, pub upgrades: Vec<Upgrade>, pub abi_renames: Vec<AbiRename>, pub checked_packages_count: usize, pub checked_binaries_count: usize, pub checked_components_count: usize }` with `pub fn is_pass(&self) -> bool` (no violations). Counts are the live side's `packages`, `custom_binaries` and `components` lengths, as cmd_verify_variance counts the live inventory.
3. `pub fn evaluate_against_golden(live: &Golden, golden: &Golden, approvals: &Approvals) -> Result<VarianceReport>` (`live` is built by `component::golden_from_inventory`):
   - Refuse with `ValidationError("golden.toml is schema v1 (no components); run neb release import")` when `golden.components` is empty and `golden.packages` is not. Without this, a v1 golden would pass every component check.
   - Components, in golden id order. `m = find_match(id, gcomp, &live.components)`.
     - None → `golden_component_removed`, item = id, detail `Component '<id>' (golden=<ver>) was removed from live OS`; skipped when `approvals.covers(id, Some(gcomp.version))`.
     - Some → compare `m.version` with `gcomp.version` (debver::compare). Less → `golden_component_downgrade`, detail `Component '<id>' downgraded: live=<m.version> < golden=<ver>`, skipped when `covers(id, Some(m.version))`. Greater → `Upgrade { item: id, from_version: gcomp.version, to_version: m.version }`. Equal → nothing.
     - Binaries, only when `!gcomp.source_unknown` (a placeholder holds one binary and was matched on it): `binary_changes(gcomp, m.component)`; each rename → `AbiRename { component: id, from_package, to_package }`; each dropped name → `golden_binary_package_dropped`, item = the binary name, detail `Binary package '<name>' of component '<id>' vanished and is not an ABI rename`, skipped when `covers(name, Some(<golden binary version>))`.
   - Custom binaries: port :992-1034. Index the live binaries by map key, by basename of the key and by `path` (all three point at the same entry). For each golden (key, meta): skip when `covers` holds for the key, its basename or `meta.path` with target `Some(meta.version)` for a removal or `Some(live.version)` for a downgrade. Live entry missing (lookup by key, then basename, then path) or `!present` → `golden_binary_removed`; lower version → `golden_binary_downgrade`; higher → Upgrade with `item` = the key. Detail strings as in the Python.
   - Calamares: each golden `presence` entry that is true, where the live presence is not true and `!covers(name, None)` → `golden_calamares_removed`, detail as in the Python.
   - Order: components, then custom binaries, then Calamares; BTreeMap order inside each. Same input → same report.
4. Tests (approvals.rs and variance.rs; build `Golden` values by hand with `Golden::from_inventory` on a default-ish IsoInventory or struct literals):
   - approvals: missing file → empty; an approval with no `version` covers any target; one with `version = "105-1"` covers only `Some("105-1")` and never `None`; `approved = false` or a blank reason never covers; append_approval twice → read_approvals returns both in order, and `approved` defaults to true when the key is absent in a hand-written file.
   - XanMod 7.2.15 → 7.3.3 kernel line (meta 7.2.15-xanmod1-0 → 7.3.3-xanmod1-0, image/headers renamed) → PASS, one Upgrade for `linux-xanmod-x64v3`, two AbiRename entries.
   - downgrade `dracut` 110-1 → 105-1 → one `golden_component_downgrade`; with an open approval → PASS; with an approval for `version = "104-1"` → still the violation.
   - removed component `btrfs-progs` → `golden_component_removed`; approved → PASS.
   - a surviving component that lost `libfoo-doc` → `golden_binary_package_dropped` with item `libfoo-doc`; `libpcap0.8` → `libpcap0.8t64` → AbiRename and PASS.
   - imported placeholder `libpcap0.8` (source_unknown) matched by live component `libpcap` at a higher binary version → Upgrade, no violation; at a lower one → downgrade violation with item `libpcap0.8`.
   - custom binary missing / `present = false` / lower version / higher version → removed / removed / downgrade / Upgrade; approval by basename covers a golden key given as a full path.
   - Calamares module present in golden, absent live → `golden_calamares_removed`.
   - v1 golden (packages, no components) → ValidationError.
   - the report's three counts equal the live side's map lengths.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn evaluate_against_golden' crates/backend_core/src/release/variance.rs && grep -q 'pub fn covers' crates/backend_core/src/release/approvals.rs && grep -q 'golden_binary_package_dropped' crates/backend_core/src/release/variance.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25L: Manifest Model + Repo Invariants

* **Recommended Command:** `/boost`

Scope note (2026-11-10): re-split. This step adds the `manifest.toml` model and the repo-level checks (`verify_repo_invariants`, nebulous-ctl.py:649-807). The manifest half of the ISO check (:846-956) is STEP 27B, which uses this step's `Manifest`. Approvals here have no version to scope by, so every check passes `None` to `covers` except the crate-version check, which passes the crate's version.

```text
[STEP_ID]
STEP 25L: release/manifest.rs (manifest.toml model + reader) + release/repo_invariants.rs (custom tools, Calamares, workspace crate versions, stage-script tokens)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/manifest.rs (NEW)
  - crates/backend_core/src/release/repo_invariants.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod manifest;` and `pub mod repo_invariants;`, alphabetical)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §1.3 (line 55, manifest.toml: schema unchanged), §2.2 (lines 84-92)
  - /srv/devshare/NovOS/.nebulous/manifest.toml (the real file: the shape to parse; [kernels] and other unknown keys are ignored)
  - /srv/devshare/NovOS/custom-tools/manifest.yaml (the `tools:` list the YAML scan reads)
  - /srv/devshare/NovOS/scripts/nebulous-ctl.py:610-646 (parse_custom_tools_manifest_yaml, _strip_comments, _has_token), :649-807 (verify_repo_invariants: the detail strings to copy)
  - crates/backend_core/src/release/golden.rs:83 (parse_golden: the schema_version pattern to copy), :118 (read_golden: the Option pattern to copy)
  - crates/backend_core/src/release/approvals.rs:32 (Approvals::covers)
  - crates/backend_core/src/release/rootfs.rs:149 (workspace_crate_versions)
  - crates/backend_core/src/release/debver.rs:29 (compare), crates/backend_core/src/release/mod.rs Violation (`Violation::new(kind, item, detail)`)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No manifest-vs-ISO checks (STEP 27B), no edits to variance.rs, no CLI (STEP 25U). No YAML crate: the tool scan is a line scan, as in the Python. No new dependencies.

[PRE_GATE]
test ! -e crates/backend_core/src/release/manifest.rs && test ! -e crates/backend_core/src/release/repo_invariants.rs && grep -q 'pub fn covers' crates/backend_core/src/release/approvals.rs && grep -q 'pub fn workspace_crate_versions' crates/backend_core/src/release/rootfs.rs

[MUTATION_SPEC]
1. manifest.rs (`#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]`, every field `#[serde(default)]`, no deny_unknown_fields):
   - `pub const MANIFEST_SCHEMA_VERSION: u32 = 1;`
   - `pub struct OsSpec { pub required: bool, pub distro_id: Option<String>, pub distro_version: Option<String> }` (required defaults false).
   - `pub struct PackageSpec { pub required: bool, pub min_version: String }`: required defaults true, min_version defaults "0" (write a manual `Default` or `#[serde(default = "...")]` per field so a bare `{}` gives true / "0").
   - `pub struct ManifestPackages { pub core: BTreeMap<String, PackageSpec>, pub cosmic: BTreeMap<String, PackageSpec> }`.
   - `pub struct BinarySpec { pub required: bool, pub min_version: String, pub required_flags: Vec<String> }`: required defaults true, min_version defaults "0.1.0".
   - `pub struct CalamaresSpec { pub required: bool, pub required_configs: Vec<String>, pub custom_modules: Vec<String> }` (required defaults false).
   - `pub struct Manifest { pub schema_version: u32, pub os: OsSpec, pub packages: ManifestPackages, pub custom_binaries: BTreeMap<String, BinarySpec>, pub calamares_modules: CalamaresSpec }`.
   - `pub fn parse_manifest(text: &str, path: &Path) -> Result<Manifest>`: same schema_version handling as parse_golden (missing → 1; not a positive integer → ValidationError; > MANIFEST_SCHEMA_VERSION → UnsupportedSchema); TOML errors → `TomlDeserialize { path: Some }`.
   - `pub fn read_manifest(path: &Path) -> Result<Option<Manifest>>`: missing file → None; other I/O → Io.
2. repo_invariants.rs helpers (private unless noted):
   - `fn strip_comments(text: &str) -> String`: each line cut at its first `#`, joined with `\n` (port of _strip_comments).
   - `pub fn has_token(text: &str, token: &str) -> bool`: true when `token` occurs with neither the byte before nor the byte after in `[A-Za-z0-9_.-]` (start/end of text count as boundaries). The `regex` crate has no look-behind, so do it by hand: loop over `text.match_indices(token)` and check both neighbours. An empty token → false.
   - `struct Tool { name: String, enabled: bool }` and `fn scan_tools(text: &str) -> Vec<Tool>`: port of parse_custom_tools_manifest_yaml, keeping only `name` and `enabled`. Skip blank lines and lines starting with `#` (after trim). A line starting with `- name:` starts a new tool (value trimmed, quotes `"`/`'` stripped). Inside a tool, a line `enabled: <v>` (not starting with `-`) sets enabled = `v.trim().eq_ignore_ascii_case("true")`. enabled defaults false.
3. `pub fn verify_repo_invariants(manifest: &Manifest, approvals: &Approvals, repo_root: &Path) -> Result<Vec<Violation>>`. Order and detail strings as nebulous-ctl.py:649-807; `ok(item)` below means `approvals.covers(item, None)`. Unreadable files that exist → Io error; missing optional files are handled as listed.
   - (1) Custom tools: read `custom-tools/manifest.yaml` (missing → no tools). No `novos-custom-tools` tool → `repo_custom_tool_missing`; present but not enabled → `repo_custom_tool_disabled`; both skipped when `ok("novos-custom-tools")`.
   - (2) Calamares under `calamares/modules/`: each `required_configs` entry not a file → `repo_calamares_config_missing`; each `custom_modules` entry not a dir → `repo_calamares_module_missing`; skipped when `ok(name)`. If `netinstall.yaml` is a file and `!ok("netinstall.yaml")`: after strip_comments, missing `- name:` or missing `packages:` → `repo_calamares_netinstall_invalid`.
   - (3) Crates: read `Cargo.toml` (missing → skip the whole check; parse error → TomlDeserialize). For each string in `workspace.members`, a missing `<member>/Cargo.toml` → `repo_cargo_member_missing` with item = member and detail `Workspace member Cargo.toml missing: <repo_root>/<member>/Cargo.toml`. Versions come from `rootfs::workspace_crate_versions(repo_root)`; for each (name, version) where `manifest.custom_binaries` has `/usr/local/bin/<name>`: `debver::compare(version, min_version) == Less` and `!approvals.covers(name, Some(version))` → `repo_cargo_version_downgrade`. (Deviation, kept on purpose: the Python crashes on `version.workspace = true`; workspace_crate_versions resolves it.)
   - (4) Stage scripts: the text is every `scripts/stages/*.sh`, sorted by file name, joined with `\n`, then strip_comments; then, when `custom-tools/manifest.yaml` exists, `\n` + its strip_comments text. No `glob` crate: `read_dir` + filter `.sh` + sort. Missing dir → empty text.
     - each `packages.core` name with `!has_token` and `!ok(name)` → `repo_stage_package_missing`, detail `Core package '<name>' missing from scripts/stages/*.sh`;
     - each `packages.cosmic` name, same, but also passes when `has_token(text, "cosmic-epoch-monorepo")`; detail `COSMIC package '<name>' ...`;
     - each `custom_binaries` (path, spec): basename missing as a token and `!ok(path)` and `!ok(basename)` → `repo_stage_binary_missing`, item = path; then each `required_flags` flag not a token and `!ok(path)` → `repo_stage_flag_missing`, item `<path>:<flag>`.
   - BTreeMap order inside each section (the Python used insertion order; neb is sorted and deterministic).
4. Tests (tempdir repo trees built in the test; placeholder names only, e.g. `testtool`, `example-pkg`; no real hostnames or paths):
   - manifest: parse the real-shaped sample (os, packages.core/cosmic, custom_binaries with required_flags, calamares_modules, an unknown `[kernels]` table) → fields as expected; `{}` package spec → required true, min "0"; binary spec defaults → "0.1.0"; schema_version 2 → UnsupportedSchema; missing file → None.
   - has_token: `dracut` in `apt install dracut btrfs-progs` → true; `dracut` in `dracut-core` → false; `btrfs` in `btrfs-progs` → false; token at start and end of text → true; `--pre-packages` in `novos-setup --pre-packages` → true.
   - strip_comments: a package named only in a `# dracut` comment is not a token.
   - scan_tools: two tools, quoted name, `enabled: true` / `enabled: false`, a nested `- "/usr/local/bin/x"` list item ignored.
   - a clean tree (tool enabled, all configs/modules present, valid netinstall.yaml, member present with version ≥ min, all names in a stage script) → no violations.
   - one violation of each `repo_*` type, each from a tree that breaks one rule; each one skipped by an open approval for its item.
   - crate version 0.0.9 < min 0.1.0 → downgrade; approval with `version = "0.0.9"` → skipped; `version = "0.0.8"` → still reported.
   - COSMIC package absent but `cosmic-epoch-monorepo` present → no violation.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn verify_repo_invariants' crates/backend_core/src/release/repo_invariants.rs && grep -q 'pub fn read_manifest' crates/backend_core/src/release/manifest.rs && grep -q 'repo_stage_flag_missing' crates/backend_core/src/release/repo_invariants.rs && git diff --quiet -- crates/backend_core/Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 27B: Manifest Half of the ISO Variance Check

* **Recommended Command:** `/goal`

Why its own step (2026-11-10): split from STEP 25K, whose golden half was already a full step, and it needs STEP 25L's `Manifest`. All 25x letters are taken, so it continues the 27x series after 27A.

```text
[STEP_ID]
STEP 27B: release/variance.rs: evaluate_against_manifest (os-release, packages, custom binaries + flags, Calamares) and evaluate_variance (manifest + optional golden in one report)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/variance.rs
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §2.2 (lines 84-92), §5.2 (lines 398-404)
  - /srv/devshare/NovOS/scripts/nebulous-ctl.py:810-823 (_index_custom_binaries), :846-956 (manifest checks: the detail strings to copy)
  - crates/backend_core/src/release/variance.rs:48 (evaluate_against_golden), :129-136 (the live custom-binary index), :222 (report construction)
  - crates/backend_core/src/release/manifest.rs (Manifest, OsSpec, PackageSpec, BinarySpec, CalamaresSpec)
  - crates/backend_core/src/release/approvals.rs:32 (covers)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No changes to evaluate_against_golden's behaviour or its tests; the index refactor below must leave them passing unchanged. No repo invariants (STEP 25L did those), no CLI (STEP 25U).

[PRE_GATE]
test -f crates/backend_core/src/release/manifest.rs && grep -q 'pub fn evaluate_against_golden' crates/backend_core/src/release/variance.rs && ! grep -q 'fn evaluate_against_manifest' crates/backend_core/src/release/variance.rs

[MUTATION_SPEC]
1. Move the live index built at variance.rs:129-136 into `fn index_custom_binaries(bins: &BTreeMap<String, CustomBinary>) -> BTreeMap<&str, &CustomBinary>` (key, basename of key, and non-empty `path`, as now) and call it from evaluate_against_golden. No behaviour change.
2. `pub fn evaluate_against_manifest(live: &Golden, manifest: &Manifest, approvals: &Approvals) -> Vec<Violation>`. Port of :846-956; detail strings exactly as the Python. Order: os, packages core then cosmic, custom binaries, Calamares; BTreeMap order inside each.
   - os: only when `manifest.os.required`, `live.os_release` is non-empty and `!approvals.covers("os_release", None)`. `distro_id` set, live `ID` present and different → `manifest_os_id_mismatch` (item `os_release.ID`). `distro_version` set, live `VERSION_ID` present and `debver::compare(live, min) == Less` → `manifest_os_version_downgrade` (item `os_release.VERSION_ID`).
   - packages: skip specs with `required == false`. Name not in `live.packages` and `!covers(name, None)` → `manifest_package_missing`. Present with `compare(live_ver, min_version) == Less` and `!covers(name, Some(live_ver))` → `manifest_package_downgrade`.
   - custom binaries: skip `required == false`. Look up by full path, then basename, in `index_custom_binaries(&live.custom_binaries)`. `ok(t)` = `covers(path, t) || covers(basename, t)`. Missing or `!present` and `!ok(None)` → `manifest_binary_missing`. Otherwise: `compare(live.version, min) == Less` and `!ok(Some(live.version))` → `manifest_binary_downgrade`; each required flag not in the live entry's `flags` and `!ok(None)` → `manifest_binary_flag_missing` (the Python checks only when `flags` exists; neb's CustomBinary always has it).
   - Calamares: only when `manifest.calamares_modules.required`. Each `required_configs` name whose `live.calamares_modules.presence` value is not true and `!covers(name, None)` → `manifest_calamares_missing`; each `custom_modules` name, same → `manifest_calamares_custom_missing`.
   - Note in a doc comment: the Python skipped a whole item on any approval by name; neb scopes a versioned approval to the downgrade target, as in STEP 25K.
3. `pub fn evaluate_variance(live: &Golden, manifest: &Manifest, golden: Option<&Golden>, approvals: &Approvals) -> Result<VarianceReport>`: violations = evaluate_against_manifest, then (when golden is Some) evaluate_against_golden's violations appended, with its upgrades and abi_renames. With golden None: no upgrades or renames. Counts from `live` as in evaluate_against_golden. A v1 golden still returns evaluate_against_golden's ValidationError.
4. Tests (in variance.rs `mod tests`, reusing `empty_golden`, `custom_bin` and `approval` already there; add a `fn manifest_with(...)` or build `Manifest` literals):
   - a live Golden meeting every manifest spec → no violations.
   - one violation for each `manifest_*` type (os id, os version, package missing, package downgrade, binary missing, binary `present = false` → missing, binary downgrade, flag missing, Calamares config, Calamares custom module).
   - `required = false` package or binary absent → no violation; `os.required = false` → no os checks; `calamares_modules.required = false` → no Calamares checks.
   - open approval by basename covers a missing binary keyed by full path; approval `version = "254"` covers a `systemd-sysv` downgrade to live 254 and not to 253.
   - evaluate_variance: manifest violations come before golden ones; with golden None the upgrades list is empty; a golden upgrade still appears in `upgrades` when there are manifest violations.
   - the existing evaluate_against_golden tests pass unchanged.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn evaluate_against_manifest' crates/backend_core/src/release/variance.rs && grep -q 'pub fn evaluate_variance' crates/backend_core/src/release/variance.rs && grep -q 'manifest_binary_flag_missing' crates/backend_core/src/release/variance.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25M: Loop-State Breaker (record-attempt, guardrail state, unlock)

* **Recommended Command:** `/goal`

Scope note (2026-11-10): the library half only. The handlers (STEP 25U) pick the item path with `ReleasePaths::loop_state_item`, take `with_write_lock`, supply the timestamp and the diff fingerprint, apply the user-only gate to unlock, and combine the guardrail state with STEP 25L's repo invariants for `check-guardrail`. Two deliberate differences from the Python: `[[diagnostics]]` survive a record-attempt (the Python's `write_item_toml` drops them; contract §1.3 says the schema keeps them), and a missing item file is created on the first attempt.

```text
[STEP_ID]
STEP 25M: release/loopstate.rs: loop-state item model, fingerprints, record_attempt (warn + 3-strike lock), guardrail_state, unlock

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/loopstate.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod loopstate;`, alphabetical)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §1.3 (line 62, loop-state file), §2.3 (lines 94-103), §2.4 (lines 105-112), §2.7 (lines 132-139)
  - /srv/devshare/NovOS/.nebulous/items/item-001-docker-qemu-ci.toml (a real item: the shape to parse)
  - /srv/devshare/NovOS/scripts/nebulous-ctl.py:190-195 (ANSI_ESCAPE_RE, strip_ansi), :1054-1092 (normalize_error_fingerprint, compute_git_diff_fingerprint), :1158-1259 (record_item_attempt: the exact rules and the negative-constraint text), :1746-1756 (unlock)
  - crates/backend_core/src/release/config.rs:82 (loop_state_item), :103 (Guardrails: loop_repeat_warn_threshold, loop_max_retries_hard_stop)
  - crates/backend_core/src/storage/atomic.rs:94 (atomic_write_str)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No CLI, no JSON envelope, no exit-code mapping (STEP 25U), no MCP diagnostics writer (STEP 25W), no lock taking, no user-only gate. No new dependencies: `regex` and `sha2` are already in Cargo.toml; there is no `hex` crate, so format the digest bytes with `{:02x}`.

[PRE_GATE]
test ! -e crates/backend_core/src/release/loopstate.rs && grep -q 'pub fn loop_state_item' crates/backend_core/src/release/config.rs

[MUTATION_SPEC]
1. Types (`#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`):
   - `#[serde(rename_all = "kebab-case")] pub enum Stage { Build, LiveBoot, InstallE2e }` (serialized `build`, `live-boot`, `install-e2e`), with `as_str`.
   - `#[serde(rename_all = "lowercase")] pub enum AttemptStatus { Pass, Fail }`.
   - `pub struct Attempt { pub timestamp: String, pub stage: String, pub status: String, pub error_summary: String, pub error_fingerprint: String, pub diff_fingerprint: String, pub modified_files: Vec<String>, pub log_file: String }`, every field `#[serde(default)]`. Stage and status stay `String` here so an imported attempt with an unexpected value still loads.
   - `pub struct ItemSection { pub id: String, pub phase_id: String, pub title: String, pub status: String, pub loop_locked: bool, pub retry_count: u32, pub negative_constraints: Vec<String>, #[serde(default, skip_serializing)] pub attempts: Vec<Attempt> }`, all `#[serde(default)]` (legacy files kept attempts under `[item]`, nebulous-ctl.py:1176).
   - `pub struct LoopItem { pub item: ItemSection, #[serde(default)] pub attempts: Vec<Attempt>, #[serde(default, skip_serializing_if = "Vec::is_empty")] pub diagnostics: Vec<toml::Table> }`. Diagnostics are kept verbatim; STEP 25W types them.
   - `pub struct DiffFingerprint { pub fingerprint: String, pub modified_files: Vec<String> }`.
   - `pub struct AttemptInput { pub stage: Stage, pub status: AttemptStatus, pub error_summary: String, pub log_file: Option<PathBuf>, pub diff: DiffFingerprint, pub timestamp: String }`.
   - `pub struct RecordOutcome { pub item_id: String, pub retry_count: u32, pub loop_locked: bool, pub error_fingerprint: String, pub diff_fingerprint: String, pub negative_constraints: Vec<String> }`.
   - `pub struct GuardrailState { pub item_id: String, pub status: String, pub loop_locked: bool, pub retry_count: u32, pub negative_constraints: Vec<String> }`.
2. Pure helpers:
   - `pub fn strip_ansi(text: &str) -> String`: regex `\x1b(?:[@-Z\\-_]|\[[0-?]*[ -/]*[@-~])`, built once (`std::sync::LazyLock` or `OnceLock`).
   - `pub fn normalize_error_fingerprint(summary: &str, log_text: &str) -> String`: port of :1054-1065 exactly: strip_ansi + trim both, take summary if non-empty else log, lowercase, apply the four regex replacements in order (timestamps → `<ts>`, `0x…` → `<hex>`, `/tmp/…` → `/tmp/<tmp>`, `pid…` → `pid=<pid>`), collapse whitespace runs to one space, trim, sha256, first 16 hex chars.
   - `fn log_tail(path: &Path) -> String`: last 40 lines, read as bytes and decoded with `String::from_utf8_lossy`, then strip_ansi. Missing or unreadable file → empty string (the Python ignores OSError here).
   - `fn summary_from(error_summary: &str, tail: &str) -> String`: strip_ansi + trim; if empty and the tail has non-blank lines: the last trimmed line matching `(?i)\b(error|fatal|panic|fail|failed|exception)\b`, else the last non-blank trimmed line.
3. `pub fn git_diff_fingerprint(repo_root: &Path) -> DiffFingerprint`: runs `git diff HEAD` and `git diff --name-only HEAD` with `std::process::Command` (`current_dir(repo_root)`), not git2: the fingerprint must match the Python's byte-for-byte so imported attempts compare equal. Trimmed diff empty → `clean`; else first 16 hex of sha256 of the trimmed diff text (stdout decoded lossily). Names = non-blank trimmed lines. Spawn failure → (`clean`, []), as the Python.
4. File I/O:
   - `pub fn read_loop_item(path: &Path) -> Result<Option<LoopItem>>`: missing → None; parse error → `TomlDeserialize { path: Some }`. After parsing, if top-level `attempts` is empty, move `item.attempts` into it.
   - `pub fn render_loop_item(li: &LoopItem) -> Result<String>`: header lines `# ======…` (78 `=`), `# Nebulous Tiered Work Item: <id>`, `# ======…`, blank line, then `toml::to_string(li)`.
   - `pub fn write_loop_item(path: &Path, li: &LoopItem) -> Result<()>`: `create_dir_all(parent)`, `atomic_write_str`. Doc comment: the caller holds `with_write_lock`.
5. `pub fn record_attempt(path: &Path, input: AttemptInput, guardrails: &Guardrails) -> Result<RecordOutcome>`: port of :1158-1259.
   - Load the item; None → a new item with `id` = the file stem, `status = "in_progress"`, everything else default.
   - tail = log_tail(log_file) when given; summary = summary_from; err_fp = normalize_error_fingerprint(summary, tail) for Fail, `none` for Pass.
   - `same_diff_prior` = any prior attempt with status `fail` whose diff_fingerprint equals the new one, when the new one is neither empty nor `none` (`clean` counts, as in the Python).
   - Push the Attempt (stage/status `as_str`, log_file as a display string or "").
   - Pass: retry_count = 0; status `in_progress` unless loop_locked.
   - Fail: retry_count += 1; `same_err` = count of `fail` attempts (including the new one) with this err_fp; if `same_err >= loop_repeat_warn_threshold || same_diff_prior`, add the negative-constraint text from :1222-1227 exactly (`mod_str` = modified files joined by `, `, or `working-tree` when empty) unless already present. If retry_count >= loop_max_retries_hard_stop: status `blocked_loop_hard_stop`, loop_locked = true.
   - Write the item; return RecordOutcome from the written state. A locked item that records a Pass stays locked (the handler maps loop_locked to exit 3).
6. `pub fn guardrail_state(path: &Path) -> Result<Option<GuardrailState>>`: read-only view of `[item]`.
7. `pub fn unlock(path: &Path) -> Result<LoopItem>`: missing → `NotFound { kind: "loop-state item", id: <file stem> }`; else loop_locked = false, retry_count = 0, status `in_progress`, attempts, diagnostics and negative_constraints kept; write; return the item.
8. Tests (tempdir; placeholder text only, no real hosts or user paths):
   - parse a copy of the real item shape (four attempts, `negative_constraints = []`) inline as a test string; round-trip render → read gives an equal LoopItem; a `[[diagnostics]]` table survives a record_attempt.
   - legacy file with attempts under `[item]` → moved to top level.
   - strip_ansi removes `\x1b[31m` and `\x1b[0m`.
   - normalize_error_fingerprint: two summaries that differ only by an ISO timestamp, a `0x7ffd1234` address, a `/tmp/build.ab12` path and `pid=4242` vs `pid 99` give the same fingerprint; different text gives a different one; result is 16 hex chars. Two fixed vectors, computed with nebulous-ctl.py's normalize_error_fingerprint on 2026-11-10: summary `E: Unable to locate package example-pkg at 2026-11-08T13:02:11Z pid=4242 0x7ffd1234 /tmp/build.ab12`, log `` → `fe0a0a3a04fb63c1`; summary ``, log `Timed out after 1200s waiting for guest readiness` → `3b9c45640c421370` (the same value as the real item file's first attempt).
   - summary_from: empty summary + tail ending in `ok` after `E: build failed` → `E: build failed`; no keyword lines → last non-blank line.
   - three Fail attempts with the same summary and thresholds 2/3: after #1 no constraint, unlocked, retry 1; after #2 one constraint, retry 2; after #3 locked, status `blocked_loop_hard_stop`, retry 3, still one constraint.
   - same diff fingerprint on two different errors → constraint added on the second (same_diff_prior).
   - Pass after one Fail → retry 0, status in_progress; Pass on a locked item → still locked.
   - missing file → record_attempt creates it with id = stem.
   - unlock: clears lock and retry, keeps attempts; missing → NotFound.
   - git_diff_fingerprint in a fresh `git init` tempdir with one commit (commit with `git -c user.name=testuser -c user.email=testuser@example.com commit`): clean tree → `clean`; one modified file → 16-hex fingerprint and that file name. Skip the test (return early with a note) if `git` isn't on PATH.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn record_attempt' crates/backend_core/src/release/loopstate.rs && grep -q 'pub fn unlock' crates/backend_core/src/release/loopstate.rs && grep -q 'blocked_loop_hard_stop' crates/backend_core/src/release/loopstate.rs && git diff --quiet -- crates/backend_core/Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25N: Failure Ledger + Release Records

* **Recommended Command:** `/goal`

Scope note (2026-11-10): re-split. Holds need the release records (a Debian hold's `snapshot` is "the DEBIAN_SNAPSHOT of the release record that last carried held_version", contract §4.4), and nothing reads or writes records yet. So this step adds the ledger and the record model. Holds, the hold file and pins-from-release are STEP 27C. There is no Python to port: the source is contract §4.3, §4.4 and §4.6. `RecordComponent` gains a `binaries` table that the §4.6 example doesn't show, so `pins-from-release` (27C) can fill the hold file's `packages`. Extra keys are allowed by the record's reader, and promote (25P) is its only writer.

```text
[STEP_ID]
STEP 25N: release/ledger.rs (Phase, failures.toml model, component-class-only record_failure) + release/records.rs (release record model, reader, writer, listing, snapshot/cache lookups)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/ledger.rs (NEW)
  - crates/backend_core/src/release/records.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod ledger;` and `pub mod records;`, alphabetical)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §1.3 (lines 49-66), §4 intro (lines 197-203), §4.3 (lines 246-270), §4.4 (lines 272-301, for what 27C will need), §4.5 (line 305, phase names), §4.6 (lines 316-348, release record)
  - crates/backend_core/src/release/golden.rs:21 (Origin), :83 (parse_golden: schema_version pattern), :155 (write_golden: atomic write + sha256 pattern)
  - crates/backend_core/src/release/approvals.rs:46 (read_approvals: missing-file pattern), :78 (append_approval: read-push-render-write pattern)
  - crates/backend_core/src/release/config.rs:58 (releases_dir), :66 (failures)
  - crates/backend_core/src/release/iso.rs:6 (chrono usage), crates/backend_core/src/storage/atomic.rs:94 (atomic_write_str)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No holds or hold file (STEP 27C), no candidate or phase recording (STEP 25O), no promote transaction (STEP 25P), no bisect (STEP 25Q), no CLI (STEP 25U). No lock taking: callers hold `with_write_lock`. No new dependencies.

[PRE_GATE]
test ! -e crates/backend_core/src/release/ledger.rs && test ! -e crates/backend_core/src/release/records.rs && grep -q 'pub enum Origin' crates/backend_core/src/release/golden.rs && grep -q 'pub fn failures' crates/backend_core/src/release/config.rs

[MUTATION_SPEC]
1. ledger.rs types (`#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`):
   - `#[derive(Copy, PartialOrd, Ord)] #[serde(rename_all = "kebab-case")] pub enum Phase { Build, VmLiveBoot, VmInstallE2e }` (`build`, `vm-live-boot`, `vm-install-e2e`), with `pub const ALL: [Phase; 3]` in order, `pub fn as_str(self) -> &'static str`, `pub fn next(self) -> Option<Phase>`, and `impl FromStr` (unknown → ValidationError naming the three valid values). STEP 25O and 25Q reuse this enum.
   - `#[serde(rename_all = "kebab-case")] pub enum FailureSource { Bisect, SingleChange, Interaction }`.
   - `pub struct Failure { pub component: String, pub version: String, pub phase: Phase, pub class: String, pub classifier_rule: String, pub log: String, pub recorded_at: String, pub source: FailureSource, #[serde(default)] pub members: Vec<String> }`.
   - `#[derive(Default)] pub struct Ledger { #[serde(default, rename = "failure")] pub failures: Vec<Failure> }` (so the file is `[[failure]]` tables, as §4.3).
   - `pub enum RecordFailureOutcome { Recorded(Failure), RejectedClass { class: String } }` (no serde).
   - `pub const COMPONENT_CLASS: &str = "component";`
2. ledger.rs functions:
   - `pub fn read_ledger(path: &Path) -> Result<Ledger>`: missing → default; I/O → Io; parse → `TomlDeserialize { path: Some }`.
   - `pub fn render_ledger(l: &Ledger) -> Result<String>`: header `# NovOS failure ledger (.tracker/release/failures.toml)`, `# Written by neb release record-failure and neb release bisect. Delete an entry by hand to release its hold.`, blank line, `toml::to_string(l)`.
   - `pub fn record_failure(path: &Path, entry: Failure) -> Result<RecordFailureOutcome>`: `entry.class != COMPONENT_CLASS` → `RejectedClass` and nothing is written (infra and unknown never reach the ledger, §4.9). Blank `component`, `version` or `classifier_rule` (after trim) → ValidationError naming the field. Otherwise read, push (duplicates are kept: one entry per occurrence), render, `create_dir_all(parent)`, `atomic_write_str`, return `Recorded(entry)`. Doc comment: caller holds `with_write_lock`; `recorded_at` comes from the caller.
   - `impl Ledger { pub fn has_exact(&self, component: &str, version: &str) -> bool; pub fn for_component<'a>(&'a self, component: &'a str) -> impl Iterator<Item = &'a Failure> + 'a }`.
3. records.rs:
   - `pub const RELEASE_RECORD_SCHEMA_VERSION: u32 = 1;`
   - `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)] pub struct RecordComponent { pub version: String, pub origin: Origin, #[serde(default, skip_serializing_if = "BTreeMap::is_empty")] pub binaries: BTreeMap<String, String> }`.
   - `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] pub struct ReleaseRecord` with **scalar fields first, then tables**, in this order (TOML can't put a plain value after a table): `schema_version: u32`, `version: String`, `promoted_at: String`, `promoted_by: String`, `#[serde(default)] reason: String`, `snapshot: String`, `iso_sha256: String`, `golden_sha256: String`, `#[serde(default)] hold_file_sha256: String`, `candidate_id: String`, `#[serde(default)] bisect_rounds: u32`, `#[serde(default)] phases: BTreeMap<String, String>`, `#[serde(default)] components: BTreeMap<String, RecordComponent>`, `#[serde(default)] third_party_cache: BTreeMap<String, Vec<String>>`, `#[serde(default, skip_serializing_if = "Option::is_none")] candidate: Option<toml::Table>` (doc: the archived candidate.toml, written by STEP 25P). All `pub`.
   - `pub fn record_file_name(version: &str, promoted_at: &chrono::DateTime<chrono::Utc>) -> Result<String>`: `<version>-<YYYYMMDDTHHMMSSZ>.toml`. `version` must be non-empty and only `[A-Za-z0-9._+~-]`, else ValidationError (it becomes a file name).
   - `pub fn parse_record(text: &str, path: &Path) -> Result<ReleaseRecord>`: schema_version handling as parse_golden (missing → ValidationError here, since every record is written by neb; non-positive → ValidationError; > 1 → UnsupportedSchema).
   - `pub fn read_record(path: &Path) -> Result<ReleaseRecord>` (missing → Io NotFound, it is a named record).
   - `pub fn render_record(r: &ReleaseRecord) -> Result<String>`: header `# NovOS release record (written by <promoted_by>)`, blank line, `toml::to_string(r)`.
   - `pub fn write_record(path: &Path, r: &ReleaseRecord) -> Result<String>`: refuse with `AlreadyExists { kind: "release record", id: <file name> }` if the path exists; `create_dir_all(parent)`; atomic write; return the sha256 hex of the written text (as write_golden).
   - `pub fn list_records(dir: &Path) -> Result<Vec<(PathBuf, ReleaseRecord)>>`: missing dir → empty; only `*.toml` files; a record that fails to parse is an error (never skipped); sorted oldest first by `promoted_at`, then file name.
   - `pub fn latest_record(dir: &Path) -> Result<Option<(PathBuf, ReleaseRecord)>>`: last of list_records.
   - `pub fn last_carrying<'a>(records: &'a [(PathBuf, ReleaseRecord)], component: &str, version: &str) -> Option<&'a ReleaseRecord>`: the newest record whose `components[component].version == version`.
   - `pub fn snapshot_for(records, component, version) -> String`: last_carrying's `snapshot`, or "" (§4.4: a hold with no snapshot is still emitted).
   - `pub fn cache_paths_for(records, component, version) -> Vec<String>`: last_carrying's `third_party_cache[component]`, or empty.
4. Tests (tempdir; placeholder names such as `example-pkg`, snapshot `20261101T000000Z`; no real paths):
   - Phase: serde names, `ALL` order, `next` (build → vm-live-boot → vm-install-e2e → None), `"vm-live-boot".parse()` ok, `"live-boot".parse()` → ValidationError.
   - record_failure: `class = "infra"` and `"unknown"` → RejectedClass and the file is not created; a component entry → Recorded and `read_ledger` returns it; two appends keep both in order (same component and version, different phase); blank version → ValidationError; rendered file has `[[failure]]` and the header; `members` round-trips for an interaction entry.
   - has_exact / for_component.
   - records: record_file_name(`2026.10`, 2026-11-09T12:00:00Z) = `2026.10-20261109T120000Z.toml`; `../x` and "" → ValidationError.
   - write_record → read_record round-trip with phases, two components (one with binaries), third_party_cache and no candidate; the returned sha256 matches the file; a second write to the same path → AlreadyExists; schema_version 2 → UnsupportedSchema.
   - list_records: three records written out of order → sorted by promoted_at; a non-toml file is ignored; a corrupt `.toml` → error; missing dir → empty.
   - last_carrying / snapshot_for / cache_paths_for: `example-pkg` 1.0-1 in records A (snapshot S1) and B (S2), 1.1-1 in C → snapshot_for(1.0-1) = S2; unknown version → ""; cache_paths_for a third_party component returns its list.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn record_failure' crates/backend_core/src/release/ledger.rs && grep -q 'RejectedClass' crates/backend_core/src/release/ledger.rs && grep -q 'pub fn snapshot_for' crates/backend_core/src/release/records.rs && git diff --quiet -- crates/backend_core/Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 27C: Holds, Hold File, Pins From a Release

* **Recommended Command:** `/boost`

Why its own step (2026-11-10): split from STEP 25N (see its scope note). It continues the 27x series. Design choices the contract leaves open:
- **One hold per component, not per ledger entry.** Its `blocked_version` is the highest active ledger version. The consumer installs the archive's version only when it is newer than `blocked_version`, so the highest failed version is the one that matters.
- **A ledger entry whose component isn't in golden produces no hold**, since there's no last-good version to hold at. That covers a brand-new component, or one golden still holds as an imported placeholder under a binary name. `candidate` (25O) still refuses an ISO carrying that exact failed version (`held_version_present`).

```text
[STEP_ID]
STEP 27C: release/holds.rs: Hold / HoldFile model, ledger holds against golden, all-last-good holds, pins-from-release, hold-file render

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/holds.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod holds;`, alphabetical)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §2.8 (lines 141-151, pins-from-release), §4.3 (line 267, hold semantics), §4.4 (lines 272-301, holds + hold file format)
  - crates/backend_core/src/release/ledger.rs (Ledger, Failure), crates/backend_core/src/release/records.rs (ReleaseRecord, RecordComponent, snapshot_for, cache_paths_for) — both from STEP 25N
  - crates/backend_core/src/release/golden.rs:21 (Origin), :36 (GoldenComponent), :47 (Golden)
  - crates/backend_core/src/release/debver.rs:29 (compare)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No file writes: holds are read-only (§4.4); the CLI prints the rendered hold file (STEP 25U). No bisect holds (`reason = "bisect"` is emitted by STEP 25Q; this step only defines the variant). No archive queries. No new dependencies.

[PRE_GATE]
test ! -e crates/backend_core/src/release/holds.rs && grep -q 'pub fn snapshot_for' crates/backend_core/src/release/records.rs && grep -q 'pub struct Ledger' crates/backend_core/src/release/ledger.rs

[MUTATION_SPEC]
1. Types (`#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`):
   - `#[derive(Copy)] #[serde(rename_all = "kebab-case")] pub enum HoldReason { Failure, Bisect, LastGood, Pin }`.
   - `#[serde(untagged)] pub enum CachePath { One(String), Many(Vec<String>) }` with `pub fn from_paths(paths: Vec<String>) -> Self` (0 → `One("")`, 1 → `One(path)`, more → `Many`), as §4.4 ("array when several").
   - `pub struct Hold { pub component: String, pub held_version: String, pub origin: Origin, pub packages: BTreeMap<String, String>, pub snapshot: String, pub cache_path: CachePath, pub reason: HoldReason, pub blocked_version: String }` (field order as §4.4).
   - `pub const HOLD_FILE_SCHEMA_VERSION: u32 = 1;` and `pub struct HoldFile { pub schema_version: u32, pub generated_at: String, pub generated_by: String, #[serde(default, rename = "hold")] pub holds: Vec<Hold> }`.
2. `fn make_hold(id: &str, version: &str, origin: Origin, packages: BTreeMap<String, String>, reason: HoldReason, blocked_version: &str, records: &[(PathBuf, ReleaseRecord)]) -> Hold`: Debian → `snapshot = snapshot_for(records, id, version)`, `cache_path = One("")`; ThirdParty → `snapshot = ""`, `cache_path = CachePath::from_paths(cache_paths_for(records, id, version))`.
3. `pub fn ledger_holds(ledger: &Ledger, golden: &Golden, records: &[(PathBuf, ReleaseRecord)]) -> Vec<Hold>`:
   - An entry is active when `golden.components` has its component and `debver::compare(&golden_version, &entry.version) != Greater` (once golden is newer the hold drops out, §4.4).
   - Group active entries by component; `blocked_version` = the highest version (debver::compare) among them.
   - Each hold: `held_version` = golden version, `origin` and `packages` from the golden component (`binaries`), `reason = Failure`, via make_hold.
   - Sorted by component id. Entries whose component isn't in golden are skipped (see the scope note).
4. `pub fn all_last_good(golden: &Golden, records: &[(PathBuf, ReleaseRecord)]) -> Vec<Hold>`: one hold per golden component, `reason = LastGood`, `blocked_version = ""`, component id order.
5. `pub fn pins_from_release(record: &ReleaseRecord, components: Option<&[String]>) -> Result<Vec<Hold>>`: the named components (or all of `record.components` when None), each pinned to the record's version: `held_version` and `origin` from the RecordComponent, `packages` from its `binaries`, `snapshot` = `record.snapshot` for Debian ("" for third_party), `cache_path` = `from_paths(record.third_party_cache[id])` for ThirdParty, `reason = Pin`, `blocked_version = ""`. A named component missing from the record → ValidationError naming every missing one (contract §2.8: exit 1, names it). Output in component id order; duplicates in `components` produce one hold.
6. `pub fn hold_file(holds: Vec<Hold>, generated_at: &str, generated_by: &str) -> HoldFile` and `pub fn render_hold_file(h: &HoldFile) -> Result<String>` (`toml::to_string`, no header comment: the file is machine input and goes to stdout).
7. Tests (struct literals for Golden, Ledger and ReleaseRecord; placeholder names such as `example-pkg`, `example-kernel`; snapshots like `20261101T000000Z`):
   - ledger entry at 2.5.5-1+1, golden 2.5.2-1+2 → one Failure hold, held 2.5.2-1+2, blocked 2.5.5-1+1, packages = golden binaries, snapshot from the newest record carrying 2.5.2-1+2.
   - golden now newer (2.5.6-1) than the entry → no hold; golden equal to the entry → hold still active.
   - two entries for one component (2.5.5-1, 2.5.6-1; plus a second phase of 2.5.5-1) → one hold with blocked 2.5.6-1.
   - entry for a component absent from golden → no hold.
   - third_party component with two cache paths → `cache_path` renders as an array; with one → a string; with none → `""`; Debian with no carrying record → `snapshot = ""`.
   - all_last_good: one LastGood hold per golden component, blocked "".
   - pins_from_release: all components; a named subset; a missing name → ValidationError containing that name; Debian gets the record's snapshot.
   - render_hold_file → parse back with `toml::from_str::<HoldFile>` equals the input; the text contains `[[hold]]`, `reason = "last-good"` and `schema_version = 1`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn ledger_holds' crates/backend_core/src/release/holds.rs && grep -q 'pub fn pins_from_release' crates/backend_core/src/release/holds.rs && grep -q 'pub fn all_last_good' crates/backend_core/src/release/holds.rs && git diff --quiet -- crates/backend_core/Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25O: Candidate, Record-Phase, Flake Guard

* **Recommended Command:** `/boost`

Scope note (2026-11-10). Library half only: the handler (STEP 25U) extracts the ISO, hashes it and the hold file, reads repo HEAD and the clock, takes `with_write_lock`, and maps outcomes to exit codes. Decisions the contract leaves open:
- **The candidate stores no inventory.** promote (25P) re-inventories the `--iso` it is given, after checking its sha256. This keeps `candidate.toml` small, which matters because it gets archived into the release record.
- **Identical inputs (§4.5).** Every phase must match the candidate's `hold_file_sha256`, `snapshot` and `repo_head`. The VM phases must also match `iso_sha256`. A `build` **fail** may carry a different ISO sha256 or none, since a fresh build makes a new ISO. A `build` **pass** with a different sha256 is refused; the pipeline makes a new candidate from the new ISO.
- **A phase can't be re-recorded once a later phase has a result,** and a `fail_confirmed` phase is final for that candidate. Both are refused, which maps to exit 5.
- **`next` after a confirmed failure:** `bisect` when more than one component changed, `record-failure` when exactly one did, `stop` when none did (a failure with no changes needs the user).
- **`DEBIAN_SNAPSHOT` is not in NovOS `config/build.env` today** (only `NOVOS_PIN_SNAPSHOT`). The default reader errors clearly until NovOS adds it; `--snapshot` works regardless.

```text
[STEP_ID]
STEP 25O: release/candidate.rs: candidate.toml model, build_candidate (changed set, held_version_present, holds in force), read/write, record_phase with order + identity checks and the flake guard

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/candidate.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod candidate;`, alphabetical)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §4 intro (lines 197-203), §4.2 (lines 237-244), §4.3 (lines 262-270, class rule), §4.5 (lines 303-314), §4.9 (lines 377-385)
  - /srv/devshare/NovOS/config/build.env (shell `export KEY="value"` lines; no DEBIAN_SNAPSHOT yet)
  - crates/backend_core/src/release/component.rs:142 (ComponentMatch: `id` is the live component id), :148 (find_match)
  - crates/backend_core/src/release/variance.rs:250 (evaluate_variance, VarianceReport)
  - crates/backend_core/src/release/ledger.rs:14 (Phase: ALL, next, as_str), :94 (has_exact), COMPONENT_CLASS
  - crates/backend_core/src/release/holds.rs:110 (ledger_holds), HoldReason
  - crates/backend_core/src/release/golden.rs:21 (Origin), Golden, GoldenComponent
  - crates/backend_core/src/release/loopstate.rs (read/render/write pattern with a header and atomic_write_str)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No ISO extraction, hashing of files, git calls or clock reads (all inputs are passed in). No promote (STEP 25P), no bisect (STEP 25Q), no ledger writes (the pipeline calls record-failure), no CLI (STEP 25U). No new dependencies.

[PRE_GATE]
test ! -e crates/backend_core/src/release/candidate.rs && grep -q 'pub fn ledger_holds' crates/backend_core/src/release/holds.rs && grep -q 'pub fn evaluate_variance' crates/backend_core/src/release/variance.rs && grep -q 'pub enum Phase' crates/backend_core/src/release/ledger.rs

[MUTATION_SPEC]
1. Types (`#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]` unless noted):
   - `pub const CANDIDATE_SCHEMA_VERSION: u32 = 1;`
   - `#[derive(Copy, Default)] #[serde(rename_all = "snake_case")] pub enum PhaseState { #[default] Pending, Pass, FailUnconfirmed, FailConfirmed }` (`pending`, `pass`, `fail_unconfirmed`, `fail_confirmed`, as §4.5).
   - `#[derive(Default)] pub struct Phases { pub build: PhaseState, #[serde(rename = "vm-live-boot")] pub vm_live_boot: PhaseState, #[serde(rename = "vm-install-e2e")] pub vm_install_e2e: PhaseState }` with `pub fn get(&self, p: Phase) -> PhaseState` and `pub fn set(&mut self, p: Phase, s: PhaseState)`. (A struct, not a map keyed by the enum, so the TOML keys are plain strings.)
   - `pub struct ChangedComponent { pub component: String, pub origin: Origin, #[serde(default, skip_serializing_if = "Option::is_none")] pub from_version: Option<String>, pub to_version: String, #[serde(default)] pub kernel_line: bool }` (None = new component; JSON shows it as null via the handler).
   - `pub struct HoldInForce { pub component: String, pub held_version: String, pub reason: HoldReason }`.
   - `#[derive(Copy)] #[serde(rename_all = "lowercase")] pub enum PhaseResult { Pass, Fail }`.
   - `pub struct PhaseRecord { pub phase: Phase, pub result: PhaseResult, pub state: PhaseState, #[serde(default)] pub class: String, #[serde(default)] pub classifier_rule: String, #[serde(default)] pub logs: Vec<String>, #[serde(default)] pub iso_sha256: String, #[serde(default)] pub verdict: String, #[serde(default)] pub flaked: bool, pub recorded_at: String }`.
   - `pub struct Candidate`, **scalars first, then tables** (as records.rs): `schema_version: u32`, `candidate_id: String`, `created_at: String`, `iso_path: String`, `iso_sha256: String`, `snapshot: String`, `#[serde(default)] artifact_cache: String`, `#[serde(default)] hold_file: String`, `#[serde(default)] hold_file_sha256: String`, `repo_head: String`, `unchanged_count: usize`, `#[serde(default)] phases: Phases`, `#[serde(default)] changed: Vec<ChangedComponent>`, `#[serde(default)] holds_in_force: Vec<HoldInForce>`, `#[serde(default)] history: Vec<PhaseRecord>`. All `pub`.
   - Inputs (no serde): `pub struct CandidateInput<'a> { pub live: &'a Golden, pub iso_path: &'a str, pub iso_sha256: &'a str, pub snapshot: &'a str, pub artifact_cache: &'a str, pub hold_file: &'a str, pub hold_file_sha256: &'a str, pub repo_head: &'a str, pub now: chrono::DateTime<chrono::Utc> }` and `pub struct PhaseInput { pub phase: Phase, pub result: PhaseResult, pub logs: Vec<String>, pub iso_sha256: Option<String>, pub hold_file_sha256: String, pub snapshot: String, pub repo_head: String, pub class: Option<String>, pub classifier_rule: Option<String>, pub verdict: Option<String>, pub recorded_at: String }`.
   - Outcomes (no serde on the enums; `Serialize` on PhaseOutcome): `pub enum CandidateOutcome { Created { candidate: Candidate, report: VarianceReport }, VarianceLock { violations: Vec<Violation>, report: VarianceReport } }`; `pub enum RefusalReason { NoCandidate, OutOfOrder, IsoMismatch, InputsMismatch, PhaseFailed }` with `as_str` (`no_candidate`, `out_of_order`, `iso_mismatch`, `inputs_mismatch`, `phase_failed`); `pub struct PhaseOutcome { pub status: String /* PASS | FAIL_UNCONFIRMED | FAIL_CONFIRMED */, pub phase: Phase, pub candidate_id: String, pub phases: Phases, pub next: String, pub changed_count: usize, pub rerun_required: bool, pub flaked: bool }`; `pub enum RecordPhaseResult { Recorded(PhaseOutcome), RejectedClass { class: String }, Refused { reason: RefusalReason, detail: String } }`.
2. `pub fn candidate_id(iso_sha256: &str, now: &DateTime<Utc>) -> String`: first 12 chars of the sha256 (all of it if shorter), `-`, `%Y%m%dT%H%M%SZ`.
3. `pub fn build_candidate(input: &CandidateInput, manifest: &Manifest, golden: Option<&Golden>, approvals: &Approvals, ledger: &Ledger, records: &[(PathBuf, ReleaseRecord)]) -> Result<CandidateOutcome>` (pure; writes nothing):
   - `report = evaluate_variance(input.live, manifest, golden, approvals)?`.
   - Golden version per live component: for each golden (id, gcomp), `find_match(id, gcomp, &live.components)`; a match maps `m.id` → `gcomp.version`. Keep the first mapping per live id (golden id order).
   - For each live component (id order): no mapping → changed with `from_version: None`; `compare(live.version, from) == Greater` → changed with `Some(from)`; `Equal` → `unchanged_count += 1`; `Less` → neither (it is a downgrade, already in `report.violations` unless approved). `origin` and `kernel_line` come from the live component. With golden None every component is new.
   - held_version_present: each changed component with `ledger.has_exact(id, to_version)` → `Violation::new("held_version_present", id, "Component '<id>' <to_version> is held by the failure ledger but is present in the ISO; the hold file was not applied")`.
   - violations = report.violations then the held ones. Non-empty → `VarianceLock`.
   - Else `Created` with candidate_id, created_at = `now` RFC 3339 (`SecondsFormat::Secs`, `Z`), the input strings, `phases` all Pending, `changed`, `unchanged_count`, `holds_in_force` = `ledger_holds(ledger, g, records)` mapped to (component, held_version, reason) when golden is Some, else empty, `history` empty.
4. `pub fn read_build_env_snapshot(repo_root: &Path) -> Result<String>`: read `config/build.env`; the first line that, after trim and an optional leading `export `, starts with `DEBIAN_SNAPSHOT=`; value trimmed, one pair of surrounding `"` or `'` stripped. Missing file, missing key or empty value → `ValidationError("DEBIAN_SNAPSHOT is not set in config/build.env; pass --snapshot")`.
5. File I/O: `pub fn read_candidate(path) -> Result<Option<Candidate>>` (missing → None; schema_version > 1 → UnsupportedSchema; parse error → TomlDeserialize), `pub fn render_candidate(c) -> Result<String>` (header `# NovOS release candidate (.tracker/release/candidate.toml)`, `# Written by neb release candidate and neb release record-phase.`, blank, toml), `pub fn write_candidate(path, c) -> Result<()>` (create_dir_all + atomic_write_str; replaces any previous candidate, §4.2). Doc comments: the caller holds `with_write_lock`; an active bisect is not touched.
6. `pub fn apply_phase(c: &mut Candidate, input: PhaseInput) -> Result<RecordPhaseResult>` (pure), checks in this order:
   - (a) Fail without `class` → ValidationError; `class != COMPONENT_CLASS` → `RejectedClass` (c unchanged). Fail without a non-blank `classifier_rule` → ValidationError.
   - (b) Order: every phase before `input.phase` (Phase::ALL order) must be Pass, and every phase after it must be Pending → else `Refused(OutOfOrder)`, detail naming the blocking phase and its state.
   - (c) The phase's current state is FailConfirmed → `Refused(PhaseFailed)`.
   - (d) Identity: hold_file_sha256, snapshot or repo_head differ from the candidate's → `Refused(InputsMismatch)` naming the field. For VM phases `iso_sha256` must be `Some` and equal → else `Refused(IsoMismatch)`. For Build: Pass with `Some(sha) != c.iso_sha256` → `IsoMismatch`; Pass with None is accepted; Fail may differ or be None.
   - (e) Transition. Pass: state → Pass; `flaked` = the old state was FailUnconfirmed; status `PASS`; next = the next phase's `as_str`, or `promote` after vm-install-e2e. Fail from Pending or Pass → FailUnconfirmed, status `FAIL_UNCONFIRMED`, `rerun_required = true`, next `rerun`. Fail from FailUnconfirmed → FailConfirmed, status `FAIL_CONFIRMED`, next per the scope note (`bisect` if changed > 1, `record-failure` if 1, `stop` if 0).
   - (f) Append a PhaseRecord (the new state, class/rule or "", logs, iso sha or "", verdict or "", flaked, recorded_at). Return `Recorded` with `phases` = the updated copy and `changed_count = c.changed.len()`.
7. `pub fn record_phase(path: &Path, input: PhaseInput) -> Result<RecordPhaseResult>`: read_candidate; None → `Refused(NoCandidate)`; apply_phase; write only when the result is Recorded.
8. Tests (struct-literal Golden values, as variance.rs's `empty_golden`; placeholder ids like `example-pkg`, `example-lib`; fake sha strings such as `"a".repeat(64)`):
   - candidate_id format; created_at is RFC 3339 with `Z`.
   - build_candidate: one upgraded, one new, one equal, one approved-away downgrade → changed has the upgrade (`from_version` Some) and the new one (None), unchanged_count 1, phases all pending; a kernel_line live component keeps `kernel_line = true`.
   - an imported placeholder golden component (`source_unknown`) matched by a live component under a different id → mapped, not reported as new.
   - a changed component whose to_version is in the ledger → VarianceLock with `held_version_present`; an unapproved removal → VarianceLock with the variance violation; holds_in_force lists ledger_holds output when golden is Some.
   - golden None → every live component is new.
   - read_build_env_snapshot: `export DEBIAN_SNAPSHOT="20261101T000000Z"` → value; without `export` and with single quotes → value; missing key → the ValidationError text; missing file → same.
   - write → read round-trip equals; schema_version 2 → UnsupportedSchema.
   - flake guard: build pass → next `vm-live-boot`; vm-live-boot fail → FAIL_UNCONFIRMED, rerun_required; second fail → FAIL_CONFIRMED with next `bisect` for 2 changed, `record-failure` for 1, `stop` for 0; a third record on that phase → Refused(PhaseFailed).
   - unconfirmed fail then pass → PASS with flaked true; history has both records.
   - vm-install-e2e before vm-live-boot passed → Refused(OutOfOrder); re-recording build after vm-live-boot has a result → Refused(OutOfOrder).
   - VM phase with a different or missing iso sha → IsoMismatch; build fail with a different iso sha → accepted; build pass with a different one → IsoMismatch; different hold_file_sha256 → InputsMismatch.
   - class `infra` → RejectedClass and the candidate is unchanged; Fail with no class → ValidationError.
   - record_phase with no candidate file → Refused(NoCandidate) and no file is created; a Refused result leaves the file byte-identical.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn build_candidate' crates/backend_core/src/release/candidate.rs && grep -q 'pub fn record_phase' crates/backend_core/src/release/candidate.rs && grep -q 'held_version_present' crates/backend_core/src/release/candidate.rs && grep -q 'FAIL_CONFIRMED' crates/backend_core/src/release/candidate.rs && git diff --quiet -- crates/backend_core/Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 27D: ISO Identity (Production vs Test Overlay)

* **Recommended Command:** `/boost`

Scope note (2026-11-10). NovOS ships two ISOs. The **production ISO** is what gets released. The **test ISO** is derived from it by NovOS STEP 15F (`scripts/make-test-overlay-iso.py`): the same ISO plus one extra live-boot layer, `/live/zz-novos-test.squashfs` (qemu-guest-agent and `/usr/share/novos-test/overlay.json`). The production `/live/filesystem.squashfs` is byte-identical in both. `extract_iso_inventory` reads only `filesystem.squashfs`, so today the two ISOs give identical inventories and a test ISO could become a candidate or golden. This step adds the identity facts the release flow needs, all in `iso.rs`:
- **Extra live layers.** live-boot stacks every `*.squashfs` in `/live`, so any layer besides `filesystem.squashfs` means "not production", whatever it is called.
- **The production squashfs sha256.** This is what ties a test ISO to its production ISO. `overlay.json` records it (`make-test-overlay-iso.py:664`), and the VM harness measures it in the guest (`test-docker-qemu.py:1137-1146`).
- **The test ISO's overlay claim.** `read_test_overlay_manifest` gives the host-side tie for VM failures that never got far enough to measure in the guest.
- `IsoMeta` and `IsoInventory` don't change, so golden files and every test literal stay as they are. STEP 27E (candidate) and STEP 25P/25U (promote, bless-golden, extract-golden) consume these facts.

xorriso behaviour (checked 2026-11-10 with xorriso and a mksquashfs fixture): `xorriso -indev <iso> -find /live -name '*.squashfs' -type f` prints one path per line on stdout, single-quoted (`'/live/zz-novos-test.squashfs'`), and exits 0. A `-find` on a missing path exits non-zero (5). Progress and info lines go to stderr.

```text
[STEP_ID]
STEP 27D: release/iso.rs: IsoIdentity (extra live layers, production squashfs sha256), extract_iso (inventory + identity), read_test_overlay_manifest

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/iso.rs
READ_ONLY_REFS:
  - crates/backend_core/src/release/iso.rs:29 (extract_iso_inventory), :56 (squashfs_path in the temp dir), :61-87 (xorriso -extract call and its NotFound mapping), :89-102 (stderr tail on failure), :227 (iso_round_trip ignored test)
  - crates/backend_core/src/release/rootfs.rs:114 (sha256_file), write_fixture_root
  - crates/backend_core/src/release/inventory.rs:17 (IsoMeta; not changed)
  - /srv/devshare/NovOS/scripts/make-test-overlay-iso.py:57 (LAYER_ISO_PATH), :656-667 (overlay.json fields)
  - /srv/devshare/NovOS/scripts/test-docker-qemu.py:1087-1113 (xorriso -find check, quote stripping)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. IsoMeta, IsoInventory and Golden do not change. No mounts, no sudo, no new dependencies (serde_json and tempfile are already in [dependencies]). Do not change what extract_iso_inventory returns.

[PRE_GATE]
grep -q 'pub fn extract_iso_inventory' crates/backend_core/src/release/iso.rs && ! grep -q 'IsoIdentity' crates/backend_core/src/release/iso.rs && grep -q 'pub fn sha256_file' crates/backend_core/src/release/rootfs.rs

[MUTATION_SPEC]
1. Constants: `pub const PRODUCTION_SQUASHFS_ISO_PATH: &str = "/live/filesystem.squashfs";`, `pub const TEST_OVERLAY_LAYER_ISO_PATH: &str = "/live/zz-novos-test.squashfs";`, `pub const TEST_OVERLAY_MANIFEST_REL: &str = "usr/share/novos-test/overlay.json";`. Use the first in the existing `-extract` call instead of the string literal.
2. Types (`#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`):
   - `pub struct IsoIdentity { pub squashfs_sha256: String, pub extra_live_layers: Vec<String> }` with `pub fn is_production(&self) -> bool` = `extra_live_layers.is_empty() && !squashfs_sha256.is_empty()`. Doc comment: the production squashfs is byte-identical in a test ISO; `extra_live_layers` (sorted ISO paths) is what tells them apart.
   - `pub struct ExtractedIso { pub inventory: IsoInventory, pub identity: IsoIdentity }`.
   - `pub struct OverlayManifest { pub schema: u32, #[serde(default)] pub production_iso: String, pub production_filesystem_squashfs_sha256: String }` (unknown keys such as `packages` are ignored; no `deny_unknown_fields`).
3. `pub fn parse_live_squashfs_listing(stdout: &str) -> Vec<String>` (pure): each non-blank line, trimmed, one pair of surrounding `'` stripped; keep paths that start with `/live/` and end with `.squashfs`; drop `PRODUCTION_SQUASHFS_ISO_PATH`; sort and dedup.
4. `fn list_extra_live_layers(iso_abs: &Path) -> Result<Vec<String>>`: run `xorriso -indev <iso> -find /live -name '*.squashfs' -type f` (separate args; no shell). NotFound on spawn → the same ValidationError text as the existing xorriso spawn. Non-zero exit → `ValidationError("xorriso could not list /live in <iso>: <stderr tail>")`, using the same 20-line stderr tail as the existing failure path (factor it into a private `fn stderr_tail(stderr: &[u8]) -> String` and use it in both places). Success → `parse_live_squashfs_listing(stdout)`.
5. `pub fn extract_iso(iso: &Path, project_root: &Path) -> Result<ExtractedIso>`: the current body of `extract_iso_inventory`, plus: after the canonicalize/metadata checks call `list_extra_live_layers`; after the xorriso extract succeeds, `squashfs_sha256 = rootfs::sha256_file(&squashfs_path)?` (computed before unsquashfs, while the temp dir is alive). Returns the inventory and `IsoIdentity { squashfs_sha256, extra_live_layers }`. Doc comment: the release flow calls this; a test ISO's inventory is identical to its production ISO's, so callers that must not take a test ISO check `identity.is_production()`.
6. `pub fn extract_iso_inventory(iso, project_root) -> Result<IsoInventory>` becomes `extract_iso(iso, project_root).map(|e| e.inventory)`; signature and doc comment unchanged.
7. `pub fn read_test_overlay_manifest(iso: &Path) -> Result<Option<OverlayManifest>>`: same file check and canonicalize as extract; `list_extra_live_layers`; if `TEST_OVERLAY_LAYER_ISO_PATH` is not in the list → `Ok(None)`. Else, in a `neb-iso-` tempdir: `xorriso -osirrox on -indev <iso> -extract <TEST_OVERLAY_LAYER_ISO_PATH> <tmp>/layer.squashfs` (failure → ValidationError with the stderr tail), then `unsquashfs -d <tmp>/extract <tmp>/layer.squashfs <TEST_OVERLAY_MANIFEST_REL>` (NotFound → the existing unsquashfs ValidationError text). Read `<tmp>/extract/<TEST_OVERLAY_MANIFEST_REL>`; missing → `ValidationError("<iso> has <layer> but no /<TEST_OVERLAY_MANIFEST_REL>")`; `serde_json::from_str` failure → `ValidationError("<iso>: invalid overlay.json: <err>")`; empty `production_filesystem_squashfs_sha256` → ValidationError naming the field. Doc comment: this is the host-side tie between a test ISO and its production ISO (NovOS STEP 15F), used by record-phase for VM phases.
8. Tests (existing tests unchanged and still passing):
   - `parse_live_squashfs_listing`: the two-line quoted output `'/live/filesystem.squashfs'` + `'/live/zz-novos-test.squashfs'` → `["/live/zz-novos-test.squashfs"]`; production-only output → empty; unquoted lines, blank lines, a non-`/live/` path and a `.img` file are handled (kept / skipped as the rule says); output is sorted and deduped.
   - `IsoIdentity::is_production`: empty layers + sha → true; a layer → false; empty sha → false.
   - `read_test_overlay_manifest` on a missing path → ValidationError "ISO file not found".
   - `#[ignore = "needs xorriso and squashfs-tools; run: cargo test -p backend_core iso_identity_round_trip -- --ignored"]` `iso_identity_round_trip`: build a production ISO as `iso_round_trip` does → `extract_iso` gives `identity.is_production()`, `squashfs_sha256 == sha256_file(<the filesystem.squashfs written>)`, and `read_test_overlay_manifest` → None. Then add `/live/zz-novos-test.squashfs` (mksquashfs of a dir holding `usr/share/novos-test/overlay.json` = `{"schema":1,"production_iso":"novos.iso","production_filesystem_squashfs_sha256":"<that sha>","packages":[]}`) and rebuild the ISO → `extra_live_layers == ["/live/zz-novos-test.squashfs"]`, not production, the same `squashfs_sha256`, the same inventory packages, and `read_test_overlay_manifest` → Some with that sha.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo test -p backend_core iso_ -- --ignored && cargo deny check advisories bans && grep -q 'pub fn extract_iso(' crates/backend_core/src/release/iso.rs && grep -q 'pub fn read_test_overlay_manifest' crates/backend_core/src/release/iso.rs && grep -q 'pub fn is_production' crates/backend_core/src/release/iso.rs && git diff --quiet -- crates/backend_core/Cargo.toml crates/backend_core/src/release/inventory.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 27E: Candidate Identity Uses the Production Squashfs (fix for STEP 25O)

* **Recommended Command:** `/boost`

Scope note (2026-11-10). STEP 25O assumed one ISO, so it ties the VM phases to the candidate by comparing the ISO sha256. But the VM phases boot the **test ISO** (NovOS STEP 15F), which has a different sha256 from the production ISO the candidate was made from. This step fixes identity only. The changed set, holds, phase order, class rule and flake guard stay as 25O wrote them.
- **Candidate creation refuses a test ISO.** `build_candidate` returns a `ValidationError` (exit 1) when `!identity.is_production()`. It's an error and not a `Violation` so that no approval can wave it away. The candidate stores the production `squashfs_sha256`.
- **The build phase is unchanged.** It is still tied to the production ISO's sha256.
- **The VM phases are tied through the production squashfs sha256 from two sources:**
  - the **overlay claim**: the test ISO's own `overlay.json`, read host-side by STEP 27D's `read_test_overlay_manifest`;
  - the **guest measurement**: the verdict's `test_overlay.production_filesystem_squashfs_sha256` and `test_overlay.matches_manifest` (`test-docker-qemu.py:1137-1146`, written under the `"test_overlay"` key at `:2841`; null when the guest agent never answered).
- **Rules.** A **pass** needs the claim, the measurement and `matches_manifest == true`, with both hashes equal to the candidate's. A **fail** needs the claim. If a measurement is present it must match, and `matches_manifest` must not be false. That lets an early-boot failure (no guest agent, so no measurement) still count toward the flake guard, as long as the test ISO was derived from this candidate.
- The VM phases' `iso_sha256` is now the test ISO's sha. It is recorded in history and never compared.
- No real `candidate.toml` exists yet (the handler is STEP 25U), so the new required field doesn't bump `CANDIDATE_SCHEMA_VERSION`.

```text
[STEP_ID]
STEP 27E: release/candidate.rs: refuse test ISOs, store squashfs_sha256, VM phases tie through the overlay claim + guest measurement; parse_verdict_overlay

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/candidate.rs
READ_ONLY_REFS:
  - crates/backend_core/src/release/candidate.rs:131 (Candidate), :157 (CandidateInput), :170 (PhaseInput), :108 (PhaseRecord), :248 (build_candidate), :598-636 (the identity match to replace), test helpers sample_input / pass_phase_input / fail_phase_input, test identity_and_iso_mismatch_checks
  - crates/backend_core/src/release/iso.rs (IsoIdentity, is_production, OverlayManifest — from STEP 27D)
  - /srv/devshare/NovOS/scripts/test-docker-qemu.py:1115-1147 (collect_test_overlay_record), :2841 ("test_overlay" key of the verdict)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No file reads in the library besides read_candidate (the handler reads the verdict file and the test ISO). Do not change the changed-set, holds, order, PhaseFailed, class or flake-guard logic. No new dependencies.

[PRE_GATE]
grep -q 'pub fn is_production' crates/backend_core/src/release/iso.rs && grep -q 'pub fn record_phase' crates/backend_core/src/release/candidate.rs && ! grep -q 'squashfs_sha256' crates/backend_core/src/release/candidate.rs

[MUTATION_SPEC]
1. `CandidateInput` gains `pub identity: &'a IsoIdentity` (import `crate::release::iso::IsoIdentity`).
2. `Candidate` gains `pub squashfs_sha256: String` directly after `iso_sha256` (required, no serde default). `build_candidate` sets it from `input.identity.squashfs_sha256`.
3. `build_candidate`, first thing (before `evaluate_variance`): if `!input.identity.extra_live_layers.is_empty()` → `Err(ValidationError(format!("{} is a test ISO (extra live layers: {}); make the candidate from the production ISO", input.iso_path, layers.join(", "))))`; else if `input.identity.squashfs_sha256` is empty → `Err(ValidationError(format!("{}: production filesystem.squashfs sha256 is missing; extract the ISO with extract_iso", input.iso_path)))`.
4. `#[derive(Debug, Clone, Default, PartialEq, Eq)] pub struct VerdictOverlay { pub production_filesystem_squashfs_sha256: Option<String>, pub matches_manifest: Option<bool> }` and `pub fn parse_verdict_overlay(json: &str) -> Result<VerdictOverlay>` (pure, `serde_json::Value`): invalid JSON or a non-object top level → `ValidationError("verdict is not a JSON object: <err>")`; `test_overlay` missing or null → default; otherwise take `production_filesystem_squashfs_sha256` when it is a non-empty string and `matches_manifest` when it is a bool (anything else → None).
5. `PhaseInput` gains `pub overlay_claim: Option<String>` (the test ISO's overlay.json `production_filesystem_squashfs_sha256`) and `pub verdict_overlay: VerdictOverlay`. Doc comment on `iso_sha256`: the production ISO for build; the test ISO for VM phases (recorded, not compared).
6. `PhaseRecord` gains `#[serde(default)] pub overlay_claim: String` and `#[serde(default)] pub squashfs_sha256: String` (the guest measurement), after `iso_sha256`; filled with the input values or "".
7. Replace the `Phase::VmLiveBoot | Phase::VmInstallE2e` arm of check (d) (`apply_phase`, currently :599-620). All refusals are `Refused(IsoMismatch)` with a detail naming the phase and the failing source:
   - `overlay_claim` None → "VM phase '<p>' needs the test ISO's overlay claim (overlay.json production_filesystem_squashfs_sha256)"; claim != `c.squashfs_sha256` → "test ISO was derived from production squashfs '<claim>', candidate is '<c>'".
   - `verdict_overlay.matches_manifest == Some(false)` → "guest measured a production squashfs that does not match the test ISO's overlay.json".
   - measurement Some and != `c.squashfs_sha256` → "guest measured production squashfs '<m>', candidate is '<c>'".
   - result Pass and (measurement None or `matches_manifest != Some(true)`) → "VM pass for '<p>' needs the guest measurement and matches_manifest = true in the verdict".
   - Fail with no measurement (and the claim matching) is accepted.
   The `Phase::Build` arm stays exactly as it is.
8. Tests: update `sample_input` (production `IsoIdentity` with `squashfs_sha256 = "d".repeat(64)`, held in a local or a `static`/helper the tests can borrow) and the phase-input helpers (VM helpers pass the matching claim and a matching measurement with `matches_manifest: Some(true)`; build helpers pass `None` / default). Every existing test keeps its assertions except `identity_and_iso_mismatch_checks`, which is rewritten to:
   - VM pass with a different test-ISO `iso_sha256` but matching claim + measurement → Recorded PASS (the test ISO sha is not compared); history keeps that iso sha, the claim and the measurement.
   - VM phase with no claim → IsoMismatch; a claim for another squashfs → IsoMismatch.
   - VM pass with no measurement → IsoMismatch; with `matches_manifest: Some(false)` → IsoMismatch; with `matches_manifest: None` → IsoMismatch.
   - VM fail with the claim and no measurement → Recorded FAIL_UNCONFIRMED; VM fail with a mismatched measurement → IsoMismatch.
   - build fail with a different iso sha → accepted; build pass with a different one → IsoMismatch; a different hold_file_sha256 → InputsMismatch (kept from 25O).
   - New: `build_candidate` with `extra_live_layers = ["/live/zz-novos-test.squashfs"]` → ValidationError whose text contains that path, even when the variance would otherwise pass; with an empty `squashfs_sha256` → ValidationError; a created candidate has `squashfs_sha256 == "d".repeat(64)` and it survives the write → read round-trip.
   - New: `parse_verdict_overlay` on `{"test_overlay": {"production_filesystem_squashfs_sha256": "<sha>", "matches_manifest": true}}` → both Some; `{"test_overlay": null}` and `{}` → default; `matches_manifest: "yes"` → None; `"[]"` and `"not json"` → ValidationError.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn parse_verdict_overlay' crates/backend_core/src/release/candidate.rs && grep -q 'pub squashfs_sha256: String' crates/backend_core/src/release/candidate.rs && grep -q 'is a test ISO' crates/backend_core/src/release/candidate.rs && grep -q 'overlay_claim' crates/backend_core/src/release/candidate.rs && git diff --quiet -- crates/backend_core/Cargo.toml crates/backend_core/src/release/iso.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25P: Promote (Golden + Release Record + Hold Clearing, All or Nothing)

* **Recommended Command:** `/boost`

Scope note (2026-11-10). Library half only, same split as 25O. The handler (STEP 25U) does the rest:
- extracts `--iso` with `extract_iso` and hashes it;
- reads the candidate, golden, manifest, approvals, ledger, records and bisect state;
- lists the artifact-cache directory;
- takes `with_write_lock` and maps outcomes to exit codes.

Decisions the contract leaves open:
- **Two halves.** `prepare_promote` is pure: it checks preconditions, builds the new golden and the release record in memory, and writes nothing. `commit_promote` does the writes with compensating rollback. That way every refusal path is provably write-free, and the rollback can be tested by making a target path unwritable.
- **"All or nothing" (§4.6, §4.9) across three files** is done in this order:
  1. Write the release record. It's a new file that refuses to overwrite.
  2. Replace `golden.toml`.
  3. Remove `candidate.toml`.
  - If step 2 fails, remove the record. If step 3 fails, restore the old golden bytes (or remove golden if there was none) and remove the record.
  - If a rollback step itself fails, the error says so and names the paths.
- **"Clear resolved holds" writes nothing.** Holds are derived from the ledger plus golden (STEP 27C), and the contract says only the user un-records a ledger entry. So writing the new golden is what clears them. `cleared_holds` reports every `holds_in_force` entry from the candidate whose component has no hold under `ledger_holds(ledger, new_golden, records)`.
- **A test ISO is refused** with a `ValidationError` (exit 1) before anything else, like `build_candidate` (STEP 27E). An ISO sha256 that isn't the candidate's is `NotReady` (exit 5, §1.1).
- **The production squashfs sha256 rides in the archived candidate** (`record.candidate.squashfs_sha256`). `ReleaseRecord` doesn't change, because a new field would mean editing its literals in three other files.
- **The bisect check is an input.** `bisect.toml` arrives with STEP 25Q, so the handler passes `active_bisect_candidate: Option<&str>` (None until 25Q) and `bisect_rounds`.
- **`third_party_cache` is computed by a pure helper** from a listing of `.deb` file names in the candidate's `artifact_cache`, which the handler supplies.

```text
[STEP_ID]
STEP 25P: release/promote.rs: prepare_promote (preconditions, new golden, release record, cleared holds), commit_promote (record → golden → remove candidate, with rollback), third_party_cache_from_listing

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/promote.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod promote;`, alphabetical)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §1.1 (exit 5), §4.4 (hold applies while golden is not newer), §4.6 (promote + release record), §4.9 (all or nothing)
  - crates/backend_core/src/release/candidate.rs:172 (Candidate: iso_sha256, squashfs_sha256, snapshot, artifact_cache, hold_file_sha256, candidate_id, phases, holds_in_force), :86 (HoldInForce), :57 (Phases::get), :36 (PhaseState::as_str)
  - crates/backend_core/src/release/iso.rs:46 (ExtractedIso), :40 (IsoIdentity::is_production)
  - crates/backend_core/src/release/component.rs:137 (golden_from_inventory), config.rs:158 (Components)
  - crates/backend_core/src/release/variance.rs:250 (evaluate_variance), :30 (VarianceReport)
  - crates/backend_core/src/release/holds.rs:110 (ledger_holds), :253 (empty_golden test helper pattern)
  - crates/backend_core/src/release/golden.rs:133 (render_golden), :155 (write_golden returns the sha256 of the bytes written), Origin
  - crates/backend_core/src/release/records.rs:30 (ReleaseRecord, RecordComponent), :60 (record_file_name), :132 (write_record: AlreadyExists, returns sha256), :255 (sample_record)
  - crates/backend_core/src/release/ledger.rs:21 (Phase::ALL), :23 (Phase::as_str)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES (ReleaseRecord, Golden, Candidate do not change). No ISO extraction, file hashing of the ISO, directory listing, git, clock reads or lock taking in this module (all passed in). No ledger writes, no hold-file writes, no bisect (STEP 25Q), no CLI (STEP 25U). No new dependencies (sha2, toml, chrono are already in [dependencies]).

[PRE_GATE]
test ! -e crates/backend_core/src/release/promote.rs && grep -q 'pub squashfs_sha256: String' crates/backend_core/src/release/candidate.rs && grep -q 'pub struct ExtractedIso' crates/backend_core/src/release/iso.rs && grep -q 'pub fn write_record' crates/backend_core/src/release/records.rs && grep -q 'pub fn golden_from_inventory' crates/backend_core/src/release/component.rs

[MUTATION_SPEC]
1. Constants and types:
   - `pub const PROMOTED_BY: &str = "neb release promote";`
   - `pub struct PromoteInput<'a> { pub candidate: Option<&'a Candidate>, pub extracted: &'a ExtractedIso, pub iso_sha256: &'a str, pub components: &'a Components, pub manifest: &'a Manifest, pub golden: Option<&'a Golden>, pub approvals: &'a Approvals, pub ledger: &'a Ledger, pub records: &'a [(PathBuf, ReleaseRecord)], pub active_bisect_candidate: Option<&'a str>, pub bisect_rounds: u32, pub cache_deb_names: &'a [String], pub now: DateTime<Utc> }` (`#[derive(Debug, Clone)]`, no serde).
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum NotReadyReason { NoCandidate, IsoMismatch, PhasesIncomplete, BisectActive }` with `as_str` (`no_candidate`, `iso_mismatch`, `phases_incomplete`, `bisect_active`).
   - `#[derive(Debug, Clone, PartialEq)] pub struct PromotePlan { pub golden: Golden, pub golden_text: String, pub golden_sha256: String, pub record: ReleaseRecord, pub record_file_name: String, pub cleared_holds: Vec<HoldInForce> }`.
   - `#[derive(Debug, Clone, PartialEq)] pub enum PreparedPromote { Ready(Box<PromotePlan>), NotReady { reason: NotReadyReason, detail: String }, VarianceLock { violations: Vec<Violation>, report: VarianceReport } }`.
   - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct Promoted { pub record_path: PathBuf, pub record_sha256: String, pub golden_sha256: String, pub cleared_holds: Vec<HoldInForce> }`.
2. `pub fn third_party_cache_from_listing(golden: &Golden, artifact_cache: &str, deb_names: &[String]) -> BTreeMap<String, Vec<String>>` (pure): empty `artifact_cache` → empty map. For each component with `origin == Origin::ThirdParty`, for each (binary, version) in `binaries`: a file name matches when it ends with `.deb` and starts with `<binary>_<v>_`, where `<v>` is the version with a leading `<digits>:` epoch removed, or the version with every `:` replaced by `%3a`. Value = `<artifact_cache without trailing '/'>/<file name>`, sorted and deduped. Components with no match are omitted.
3. `pub fn prepare_promote(input: &PromoteInput) -> Result<PreparedPromote>` (pure; writes nothing). Checks in this order:
   - (a) `!input.extracted.identity.is_production()` → `Err(ValidationError(format!("{} is not a production ISO (extra live layers: {}; squashfs sha256: '{}'); promote takes the production ISO", inv.iso.path, layers joined by ", " or "none", sha)))`.
   - (b) `candidate` None → `NotReady(NoCandidate, "no active candidate (.tracker/release/candidate.toml does not exist)")`.
   - (c) `input.iso_sha256 != c.iso_sha256` → `NotReady(IsoMismatch)` naming both; `identity.squashfs_sha256 != c.squashfs_sha256` → `NotReady(IsoMismatch)` naming both.
   - (d) the first phase in `Phase::ALL` whose state is not Pass → `NotReady(PhasesIncomplete, "phase '<p>' is '<state>'; promote needs all three phases to pass")`.
   - (e) `active_bisect_candidate` Some(id) with `id != c.candidate_id` → `NotReady(BisectActive)` naming that id.
   - (f) `new_golden = golden_from_inventory(&input.extracted.inventory, input.components)`; `report = evaluate_variance(&new_golden, manifest, golden, approvals)?`; non-empty `report.violations` → `VarianceLock { violations: report.violations.clone(), report }`.
   - (g) `version = new_golden.os_release["VERSION_ID"]` trimmed; missing or empty → `Err(ValidationError("ISO os-release has no VERSION_ID; cannot name the release record"))`. `record_file_name = records::record_file_name(&version, &now)?`.
   - (h) `golden_text = render_golden(&new_golden)?`; `golden_sha256` = lowercase hex sha256 of `golden_text` bytes (sha2, the same way `write_golden` computes it).
   - (i) `record = ReleaseRecord { schema_version: RELEASE_RECORD_SCHEMA_VERSION, version, promoted_at: now RFC 3339 (SecondsFormat::Secs, Z), promoted_by: PROMOTED_BY, reason: "", snapshot: c.snapshot, iso_sha256: c.iso_sha256, golden_sha256, hold_file_sha256: c.hold_file_sha256, candidate_id: c.candidate_id, bisect_rounds, phases: {each Phase::ALL as_str → state as_str}, components: new_golden.components mapped to RecordComponent { version, origin, binaries }, third_party_cache: third_party_cache_from_listing(&new_golden, &c.artifact_cache, cache_deb_names), candidate: Some(the candidate as a toml::Table via toml::Value::try_from(c)?; a non-table value → ValidationError) }`.
   - (j) `cleared_holds` = each `c.holds_in_force` entry whose `component` has no hold in `ledger_holds(ledger, &new_golden, records)` (candidate order).
   - Return `Ready(Box::new(PromotePlan { … }))`.
4. `pub fn commit_promote(plan: &PromotePlan, golden_path: &Path, releases_dir: &Path, candidate_path: &Path) -> Result<Promoted>`. Doc comment: the caller holds `with_write_lock` across `prepare_promote` and this call; an active bisect is not touched. Steps:
   - Read the old golden bytes (`fs::read_to_string`; NotFound → None; other errors → Io) before any write.
   - `record_path = releases_dir.join(&plan.record_file_name)`; `record_sha256 = write_record(&record_path, &plan.record)?` (AlreadyExists or any error → return it; nothing else was written).
   - `write_golden(golden_path, &plan.golden)`: on Err → remove `record_path`, return the error. On Ok with a sha != `plan.golden_sha256` → roll back (below) and `Err(ValidationError("golden.toml render changed between prepare and commit"))`.
   - `fs::remove_file(candidate_path)`: on Err → roll back, return `Io` for the candidate path.
   - Rollback (after golden was written): restore the old golden text with `crate::storage::atomic_write_str`, or `remove_file(golden_path)` when there was none; then `remove_file(record_path)`. If any rollback step fails, return `ValidationError(format!("promote failed: {original}; rollback also failed: {rollback_err}; check {golden_path} and {record_path}"))` instead of the original error.
   - Return `Promoted { record_path, record_sha256, golden_sha256: plan.golden_sha256.clone(), cleared_holds: plan.cleared_holds.clone() }`.
5. Tests (struct-literal fixtures as candidate.rs / holds.rs do; placeholder ids `example-pkg`, `example-kernel-tool`; fake shas `"a".repeat(64)`; an `IsoInventory` with `os_release` `VERSION_ID = "2026.11"` and two `InstalledPackage`s, one listed in `Components.third_party`; a `Candidate` literal with all phases Pass whose `iso_sha256` / `squashfs_sha256` match the input; `Manifest::default()`; empty approvals and ledger unless stated):
   - third_party_cache_from_listing: epoch version `1:2.0-1` matches `tool_2.0-1_amd64.deb` and `tool_1%3a2.0-1_amd64.deb`, not `tool_2.0-10_amd64.deb` or `toolbox_2.0-1_amd64.deb`; a debian-origin component is skipped; an empty artifact_cache → empty map; a trailing `/` is not doubled.
   - prepare_promote happy path → Ready: record `version == "2026.11"`, file name `2026.11-<UTC>.toml`, `promoted_by == PROMOTED_BY`, phases all `pass`, `golden_sha256` equals the sha of `render_golden(&plan.golden)`, `record.candidate` has `squashfs_sha256`, components carry versions and origins.
   - a test-overlay identity → ValidationError containing `/live/zz-novos-test.squashfs`; an empty squashfs sha → ValidationError.
   - no candidate → NotReady(NoCandidate); a different iso sha → NotReady(IsoMismatch); vm-install-e2e Pending → NotReady(PhasesIncomplete) naming `vm-install-e2e`; a bisect for another candidate → NotReady(BisectActive); a bisect for this candidate → Ready.
   - an unapproved removal against an old golden → VarianceLock; missing VERSION_ID → ValidationError.
   - cleared_holds: a candidate hold on `example-pkg` (ledger entry at the blocked version) is cleared when the new inventory carries a newer version, and kept when it carries the golden version.
   - commit_promote happy path in a tempdir: record file exists and parses with `read_record`, golden file's sha256 equals `plan.golden_sha256`, candidate file is gone, `Promoted.record_sha256` equals the sha of the record file bytes.
   - commit rollback: (1) `golden_path` is an existing directory → Err, the record file does not exist, the candidate file is unchanged; (2) `candidate_path` is a directory (remove_file fails) with an old golden present → Err, golden.toml is byte-identical to the old one, the record file does not exist; (3) with no old golden and the same candidate failure → golden.toml does not exist afterwards; (4) a record file of the same name already exists → AlreadyExists and golden/candidate untouched.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn prepare_promote' crates/backend_core/src/release/promote.rs && grep -q 'pub fn commit_promote' crates/backend_core/src/release/promote.rs && grep -q 'pub fn third_party_cache_from_listing' crates/backend_core/src/release/promote.rs && grep -q 'rollback also failed' crates/backend_core/src/release/promote.rs && git diff --quiet -- crates/backend_core/Cargo.toml crates/backend_core/src/release/records.rs crates/backend_core/src/release/golden.rs crates/backend_core/src/release/candidate.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25Q: Bisect (Deterministic Halving, Interaction, Cap)

* **Recommended Command:** `/boost`

Scope note (2026-11-10). Library half only, same split as 25O/25P. The handler (STEP 25U) does the rest:
- takes `with_write_lock` and the §5.1 gate for `abort`;
- reads candidate, golden, ledger, records and `bisect.toml`;
- writes the round's hold file to disk and the ledger entries this module returns (via `record_failure`, **before** writing `bisect.toml`);
- maps outcomes to exit codes and JSON.

Decisions the contract leaves open:
- **Rules 2–3 are read literally.** A fails → S := A without testing B; B is tested only when A passes. Each round is a full Stage 1–6 build plus VM phases, so this saves a round per level, and a second culprit hiding in B still fails the pipeline's rerun-without-culprit, which starts a new bisect. Consequence: rule 5 ("both fail") can't occur and the queue holds one suspect set in practice. `queue[]` stays in the format because §4.7 lists it.
- **The cap is a backstop.** Literal halving needs at most `2·ceil(log2 N)` rounds, below the cap of `2·ceil(log2 N) + 2`, so the cap only fires on a hand-edited or future state. Tests drive it with a state literal whose `cap` is small.
- **New components (no golden version) can't be held at golden**, because a hold file can only pin a version, not remove a package. They're excluded from the suspect set and listed as `always_applied`; N counts only bisectable components. N ≤ 1 → not ready (exit 5).
- **Evidence for ledger entries.** Each suspect set carries the `classifier_rule` and `log` of the failure that put it under suspicion: the candidate's confirming `FailConfirmed` record for the starting set, then the failing round. A culprit's entry uses the round that isolated it. An interaction entry uses the set's own evidence.
- **Only `status = "active"` counts as an active bisect.** A finished bisect (culprit, interaction, cap reached) stays in `bisect.toml` for `status`, but doesn't block `promote` of the rerun candidate or a new `bisect start`.
- **`bisect next` on a round still pending re-plans that round in place** (same number, same `started_at`, so the same hold file if the ledger hasn't changed) instead of starting another.
- **Round 1 is the first bisect build.** Round 0 is the candidate run, as in the contract's `round-0/build.log` example.
- The module computes; it writes only `bisect.toml` (own I/O functions, like `candidate.rs`). Ledger entries come back as values.

```text
[STEP_ID]
STEP 25Q: release/bisect.rs: BisectState model and I/O, start_bisect, next_round (test set + hold file), record_round (halving, culprit, interaction, cap), active_bisect_candidate

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/bisect.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod bisect;`, alphabetical)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §4.3 (ledger entry, source, members), §4.4 (hold file), §4.7 (bisect), §1.1 (exit 5, 6, 7)
  - crates/backend_core/src/release/candidate.rs:172 (Candidate: candidate_id, phases, changed, history), :75 (ChangedComponent: component, from_version, to_version), :145 (PhaseRecord: phase, state, classifier_rule, logs), :27 (PhaseState), :94 (PhaseResult), :467 (read_candidate: schema check pattern), :528 (write_candidate: parent dir + atomic write pattern)
  - crates/backend_core/src/release/holds.rs:53 (Hold), :110 (ledger_holds), :153 (all_last_good), :230 (hold_file), :239 (render_hold_file), :17 (HoldReason::Bisect)
  - crates/backend_core/src/release/ledger.rs:10 (COMPONENT_CLASS), :14 (Phase, ALL), :57 (FailureSource), :74 (Failure)
  - crates/backend_core/src/release/golden.rs (Golden, GoldenComponent), records.rs:30 (ReleaseRecord)
  - crates/backend_core/src/release/promote.rs:104 (sha256_hex pattern), :28 (PromoteInput.active_bisect_candidate, bisect_rounds)
  - crates/backend_core/src/release/mod.rs:36 (ExitCode)
  - crates/backend_core/src/storage/atomic.rs:94 (atomic_write_str)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES (candidate.rs, holds.rs, ledger.rs, promote.rs do not change). No ledger writes (entries are returned), no hold-file writes, no lock taking, no tty gate, no clock reads (`now` is passed in), no CLI (STEP 25T/25U). No new dependencies.

[PRE_GATE]
test ! -e crates/backend_core/src/release/bisect.rs && grep -q 'pub fn all_last_good' crates/backend_core/src/release/holds.rs && grep -q 'pub fn ledger_holds' crates/backend_core/src/release/holds.rs && grep -q 'pub enum PhaseResult' crates/backend_core/src/release/candidate.rs && grep -q 'active_bisect_candidate' crates/backend_core/src/release/promote.rs

[MUTATION_SPEC]
1. Constants and types (all fields `pub`; serde derives on the state types; **scalars and plain arrays before tables in every struct**, as TOML needs):
   - `pub const BISECT_SCHEMA_VERSION: u32 = 1;` `pub const BISECT_GENERATED_BY: &str = "neb release bisect";`
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")] pub enum BisectStatus { Active, Culprit, Interaction, CapReached }` with `as_str` (`active`, `culprit`, `interaction`, `cap_reached`), `pub fn status_code(self) -> &'static str` (`BISECT_ROUND_RECORDED`, `BISECT_CULPRIT`, `BISECT_INTERACTION`, `BISECT_CAP_REACHED`) and `pub fn exit_code(self) -> ExitCode` (Active, Culprit → Ok; Interaction, CapReached → BisectStop).
   - `#[serde(rename_all = "snake_case")] pub enum RoundResult { Pending, Pass, Fail }` with `as_str`.
   - `pub struct Round { round: u32, started_at: String, result: RoundResult, hold_file_sha256: String, #[serde(default)] classifier_rule: String, #[serde(default)] log: String, #[serde(default)] recorded_at: String, testing: Vec<String>, held: Vec<String> }`.
   - `pub struct Suspect { #[serde(default, skip_serializing_if = "Option::is_none")] a_round: Option<u32>, classifier_rule: String, log: String, members: Vec<String> }` (doc: `a_round` is the round that tested the first half and passed; `None` means the first half is next).
   - `pub struct Culprit { component: String, version: String, round: u32 }`.
   - `pub struct BisectState { schema_version: u32, candidate_id: String, failing_phase: Phase, cap: u32, round: u32, status: BisectStatus, started_at: String, always_applied: Vec<String>, #[serde(default)] interaction: Vec<String>, #[serde(default)] culprits: Vec<Culprit>, #[serde(default)] queue: Vec<Suspect>, #[serde(default)] rounds: Vec<Round>, versions: BTreeMap<String, String> }` (`versions`: component → `to_version` for every bisectable component; `round` = `rounds.len()`). Methods: `pub fn is_active(&self) -> bool` (status == Active), `pub fn rounds_completed(&self) -> u32` (rounds whose result is not Pending).
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum BisectNotReady { NoCandidate, NoConfirmedFailure, TooFewComponents, NotActive, NoRoundInProgress }` with `as_str` (`no_candidate`, `no_confirmed_failure`, `too_few_components`, `not_active`, `no_round_in_progress`).
   - `pub enum StartOutcome { Started { state: Box<BisectState>, n: usize, cap: u32 }, NotReady { reason: BisectNotReady, detail: String }, Conflict { detail: String } }`.
   - `pub struct RoundPlan { round: u32, testing: Vec<String>, held: Vec<String>, hold_file_text: String, hold_file_sha256: String }`; `pub enum NextOutcome { Round(RoundPlan), NotReady { reason: BisectNotReady, detail: String }, CapReached }`.
   - `pub struct RoundInput { result: PhaseResult, class: Option<String>, classifier_rule: String, log: String, now: DateTime<Utc> }`; `pub enum RecordOutcome { Recorded { status: BisectStatus, next: &'static str, failures: Vec<Failure> }, RejectedClass { class: String }, NotReady { reason: BisectNotReady, detail: String } }`.
   - Derive `Debug, Clone, PartialEq` (plus `Eq` where every field allows it) on all of these.
2. Helpers (pure):
   - `pub fn bisect_cap(n: usize) -> u32`: `2 * ceil(log2 n) + 2` in integer arithmetic (n = 2 → 4, 3 → 6, 4 → 6, 5 → 8, 8 → 8, 9 → 10); n < 2 → 2.
   - `pub fn split(members: &[String]) -> (Vec<String>, Vec<String>)`: A = first `ceil(len/2)`, B = the rest.
   - `pub fn active_bisect_candidate(state: Option<&BisectState>) -> Option<&str>`: the candidate_id when the state is active, else None (this is what STEP 25U passes to `PromoteInput.active_bisect_candidate`).
   - a private `sha256_hex(&[u8]) -> String` (lowercase hex, same as promote.rs:104).
3. `pub fn start_bisect(candidate: Option<&Candidate>, existing: Option<&BisectState>, now: DateTime<Utc>) -> StartOutcome` (pure), checks in order:
   - candidate None → NotReady(NoCandidate, "no active candidate (.tracker/release/candidate.toml does not exist)").
   - `existing` is active → Conflict naming its candidate_id and round ("a bisect is already active for candidate '<id>' at round <n>; finish it or run neb release bisect abort"). A finished `existing` is replaced.
   - failing phase = the first `Phase::ALL` whose state is FailConfirmed; none → NotReady(NoConfirmedFailure, naming every phase's state).
   - evidence = the last `history` record with that phase and `state == FailConfirmed`: its `classifier_rule`, and its first log or "" if it has none.
   - bisectable = `changed` entries with `from_version` Some, ids sorted; `always_applied` = ids with `from_version` None, sorted; `versions` = bisectable id → to_version.
   - `n = bisectable.len()`; n ≤ 1 → NotReady(TooFewComponents, "<n> bisectable changed component(s); bisect needs at least 2" plus, when `always_applied` is non-empty, "; new components cannot be held at golden: <ids>").
   - Started: state with `schema_version`, candidate_id, failing_phase, `cap = bisect_cap(n)`, `round = 0`, Active, `started_at` = now RFC 3339 (Secs, Z), always_applied, versions, `queue = [Suspect { a_round: None, evidence, members: bisectable }]`, empty rounds/culprits/interaction.
4. `pub fn next_round(state: &mut BisectState, golden: &Golden, ledger: &Ledger, records: &[(PathBuf, ReleaseRecord)], now: DateTime<Utc>) -> Result<NextOutcome>`:
   - status CapReached → CapReached. Any other non-active status → NotReady(NotActive, naming the status). Active but `rounds_completed() >= cap` → set status CapReached and return CapReached.
   - If the last round is Pending, re-plan it in place: keep its number and `started_at`, recompute testing/held/holds below and overwrite its `testing`, `held` and `hold_file_sha256`. Otherwise push a new Round (`round = rounds.len() + 1`, `started_at` = now RFC 3339, Pending) and set `state.round` to it.
   - S = `queue[0]`; `(a, b) = split(&S.members)`; testing = `a` when `S.a_round` is None, else `b`. held = every key of `versions` not in testing (sorted).
   - Holds: `ledger_holds(ledger, golden, records)` minus those whose component is in held, plus `all_last_good(golden, records)` filtered to held with `reason = HoldReason::Bisect`; sort by component. A held component missing from `golden.components` → `Err(ValidationError("bisect: held component '<id>' is not in golden.toml"))`.
   - `hold_file_text = render_hold_file(&hold_file(holds, &round.started_at, BISECT_GENERATED_BY))?`, sha = sha256_hex of its bytes. Return `Round(RoundPlan { … })`.
5. `pub fn record_round(state: &mut BisectState, input: RoundInput) -> Result<RecordOutcome>`:
   - Not active → NotReady(NotActive). No rounds, or last round not Pending → NotReady(NoRoundInProgress, "no bisect round in progress; run neb release bisect next").
   - Fail: `class` must be Some(COMPONENT_CLASS); anything else → RejectedClass { class (or "" when None) } with the state unchanged (round not consumed). Blank `classifier_rule` on a fail → ValidationError.
   - Store result, classifier_rule, log, recorded_at (now RFC 3339) on the round.
   - Advance (S = `queue[0]`, `(a, b) = split(&S.members)`):
     - Testing A (`S.a_round` None), fail → narrow(a). Pass → `S.a_round = Some(round)`, status stays Active.
     - Testing B, fail → narrow(b). Pass → interaction: `state.interaction = S.members`, one Failure per member (`version = versions[member]`, `phase = failing_phase`, `class = COMPONENT_CLASS`, S's `classifier_rule` and `log`, `recorded_at` = now, `source = Interaction`, `members = S.members`), clear the queue, status Interaction.
     - narrow(half): if half has one member → culprit: push `Culprit { component, version, round }`, one Failure (`source = Bisect`, `members = []`, this round's classifier_rule and log), remove `queue[0]`; status Culprit when the queue is empty. Otherwise replace `queue[0]` with `Suspect { a_round: None, this round's rule and log, members: half }`.
   - Then, if status is still Active and `rounds_completed() >= cap` → status CapReached, no failures for the unresolved set.
   - `next`: Active → "next", Culprit → "rerun-without-culprit", Interaction and CapReached → "stop". Return `Recorded { status, next, failures }`.
6. I/O (same patterns as candidate.rs:467/:528):
   - `pub fn read_bisect(path) -> Result<Option<BisectState>>`: NotFound → None; schema_version above BISECT_SCHEMA_VERSION → UnsupportedSchema; non-positive-integer schema_version → ValidationError.
   - `pub fn render_bisect(&BisectState) -> Result<String>` with a two-line `#` header naming the file and its writer.
   - `pub fn write_bisect(path, &BisectState) -> Result<()>`: create parent, `atomic_write_str`. Doc: the caller holds `with_write_lock` and writes the returned ledger entries first.
   - `pub fn remove_bisect(path) -> Result<bool>`: true when removed, false when it did not exist (the `abort` body; the §5.1 gate is the handler's).
7. Tests (struct-literal fixtures; placeholder ids `a-pkg`, `b-pkg`, `ppp`, `z-pkg`, `new-pkg`; fake shas `"a".repeat(64)`; a Golden with those components at golden versions):
   - `bisect_cap` table above; `split` of 3 → (2, 1), of 4 → (2, 2).
   - start: no candidate → NoCandidate; an active existing state → Conflict; a CapReached existing state → Started; no FailConfirmed phase → NoConfirmedFailure; one bisectable + `new-pkg` → TooFewComponents naming `new-pkg`; happy path with `changed` out of order plus `new-pkg` and vm-live-boot FailConfirmed → members sorted `[a-pkg, b-pkg, ppp, z-pkg]`, `always_applied == [new-pkg]`, failing_phase VmLiveBoot, evidence from the confirming history record, cap 6.
   - next: round 1 tests `[a-pkg, b-pkg]`, holds `ppp` and `z-pkg` at golden with reason bisect; a ledger hold on an unrelated golden component is kept; a ledger hold on a held component is replaced by the bisect hold; a second call while pending returns round 1 again with the same sha; a held component missing from golden → ValidationError; a finished state → NotActive.
   - record: no round → NoRoundInProgress; fail with class `infra` → RejectedClass and the round is still Pending.
   - ppp culprit: round 1 `[a-pkg, b-pkg]` pass → round 2 `[ppp, z-pkg]` fail → round 3 `[ppp]` fail → Culprit, next `rerun-without-culprit`, one Failure `ppp` at its to_version (`2.5.5-1+1`), source Bisect, phase vm-live-boot, round 3's rule and log; `culprits[0].round == 3`.
   - culprit via the second half: …round 3 `[ppp]` pass → round 4 `[z-pkg]` fail → Culprit `z-pkg`.
   - interaction: n = 2, round 1 `[a-pkg]` pass, round 2 `[b-pkg]` pass → Interaction, exit_code BisectStop, two Failures with `members == [a-pkg, b-pkg]`, source Interaction, the starting evidence.
   - cap: a state literal with `cap = 1` → after one passing round on A, CapReached with no failures; next_round → CapReached.
   - `active_bisect_candidate`: Active → Some(id); Culprit → None.
   - I/O: write then read round-trips; missing file → None; schema_version 99 → UnsupportedSchema; remove existing → true, missing → false.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn start_bisect' crates/backend_core/src/release/bisect.rs && grep -q 'pub fn next_round' crates/backend_core/src/release/bisect.rs && grep -q 'pub fn record_round' crates/backend_core/src/release/bisect.rs && grep -q 'pub fn active_bisect_candidate' crates/backend_core/src/release/bisect.rs && git diff --quiet -- crates/backend_core/Cargo.toml crates/backend_core/src/release/candidate.rs crates/backend_core/src/release/holds.rs crates/backend_core/src/release/ledger.rs crates/backend_core/src/release/promote.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25R: Import from `.nebulous` (Golden v1 → v2 Placeholders, Loop-State, Guardrails)

* **Recommended Command:** `/boost`

Scope note (2026-11-10). Library half only, same split as 25O–25Q. The handler (STEP 25U) adds the §5.1 user-only gate (confirmation word `import`), takes `with_write_lock`, and maps outcomes to exit codes. Source is contract §1.3 ("Replaces" column), §4.1 ("Imported placeholder"), §5.4 and §6.4. NovOS's real `.nebulous/` has `golden-snapshot.toml` (v1), `manifest.toml`, `project.toml`, `items/item-001-docker-qemu-ci.toml` and `phases/`; no `approvals.toml`.

Decisions the contract leaves open:
- **Two halves, like promote.** `plan_import` reads and validates every source and builds every output in memory; `commit_import` writes them. Every refusal path is write-free.
- **Import runs once.** If any destination already exists, the plan is a `Conflict` listing them (exit 7) and nothing is written. No `--force`: the user deletes by hand. Because every destination is new, rollback on a failed write is just "remove what this call wrote".
- **Copied verbatim, after validation:** `manifest.toml`, `approvals.toml` (when present) and each `items/*.toml`. Their schema is unchanged (§1.3), so a byte copy keeps comments and any keys our structs don't model. Each is parsed first with its existing reader so a bad file stops the import.
- **Converted:** golden (v1 → v2 placeholders, rendered by `render_golden`) and `config.toml` (built from `project.toml [guardrails]` plus defaults for `agent_env_markers`, `[harness]` and `[components]`).
- **Golden conversion rule:** convert when `components` is empty; a source that already has components is kept as-is. Each binary in `[packages]` becomes component `<binary>` with `version` = its version, `source_unknown = true`, `binaries = {<binary> = <version>}`, `kernel_line = false` (v1 has no Depends data), and `origin = third_party` only when the name is in the config's `[components.third_party]` (empty by default, so XanMod stays `debian` until the user edits config.toml; the first promote replaces placeholders anyway).
- **A relaxed guardrail in `project.toml` refuses the import** through the same check `load_config` applies (`validate_config`), so import can never write a config.toml that `load_config` would then reject. That needs two small additions in `config.rs`: `validate_config` becomes `pub`, and a `render_config` writer.
- **Not imported:** `phases/` and `project.toml [project]` / `[hierarchy]`. The report lists skipped top-level entries.

```text
[STEP_ID]
STEP 25R: release/import.rs: plan_import (validate .nebulous sources, golden v1 → v2 placeholders, config from [guardrails], verbatim copies), commit_import (write all, remove on failure); config.rs: pub validate_config, render_config

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/import.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod import;`, alphabetical)
  - crates/backend_core/src/release/config.rs (only: `fn validate_config` → `pub fn validate_config`, and the new `pub fn render_config` below)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §1.3, §4.1 (Imported placeholder), §5.4, §6.4
  - /srv/devshare/NovOS/.nebulous/ (shape only: project.toml [guardrails], items/*.toml, golden v1 tables). Do not copy its data into tests: it has real paths and hashes.
  - crates/backend_core/src/release/config.rs:31 (ReleasePaths: golden, manifest, approvals, config, loop_state_dir, loop_state_item), :95 (ReleaseConfig), :103 (Guardrails, serde default), :158 (Components), :175 (load_config), :195 (validate_config), :254 (test ReleasePaths via TrackerStorage::init_standard)
  - crates/backend_core/src/release/golden.rs:36 (GoldenComponent), :47 (Golden), :83 (parse_golden: v1 without components), :133 (render_golden), :13 (GOLDEN_SCHEMA_VERSION)
  - crates/backend_core/src/release/manifest.rs:119 (parse_manifest)
  - crates/backend_core/src/release/approvals.rs:46 (read_approvals)
  - crates/backend_core/src/release/loopstate.rs:117 (LoopItem), :288 (read_loop_item)
  - crates/backend_core/src/release/promote.rs:342 (rollback error wording pattern)
  - crates/backend_core/src/storage/atomic.rs:94 (atomic_write_str)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES; in config.rs nothing beyond the two changes named (no behaviour change to load_config or validate_config). golden.rs, manifest.rs, approvals.rs, loopstate.rs do not change. No tty gate, no lock taking, no CLI (STEP 25T/25U), no deleting or editing anything under the source directory, no import of `phases/`. No new dependencies.

[PRE_GATE]
test ! -e crates/backend_core/src/release/import.rs && grep -q '^fn validate_config' crates/backend_core/src/release/config.rs && grep -q 'pub fn read_loop_item' crates/backend_core/src/release/loopstate.rs && grep -q 'pub fn parse_manifest' crates/backend_core/src/release/manifest.rs && grep -q 'pub source_unknown: bool' crates/backend_core/src/release/golden.rs

[MUTATION_SPEC]
1. config.rs:
   - `fn validate_config` → `pub fn validate_config` (signature and body unchanged; add a one-line doc comment).
   - `pub fn render_config(cfg: &ReleaseConfig) -> Result<String>`: `validate_config(cfg, Path::new("config.toml"))?`, then a two-line `#` header ("# NovOS release configuration (.tracker/release/config.toml)" / "# Written by neb release import; edit by hand afterwards. The forward-only guardrails cannot be relaxed.") followed by `toml::to_string(cfg)?`.
   - One test: `render_config(&ReleaseConfig::default())` parses back with `toml::from_str` to the same value; a config with `allow_package_removal = true` → ValidationError.
2. import.rs constants and types (all fields `pub`):
   - Source names: `pub const SRC_GOLDEN: &str = "golden-snapshot.toml"`, `SRC_MANIFEST = "manifest.toml"`, `SRC_APPROVALS = "approvals.toml"`, `SRC_PROJECT = "project.toml"`, `SRC_ITEMS = "items"`.
   - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct ImportFile { dest: PathBuf, text: String }`.
   - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct ImportReport { golden_schema_from: u32, golden_converted: bool, placeholders: usize, components: usize, manifest_copied: bool, approvals_copied: bool, approvals: usize, guardrails_from_project: bool, loop_items: Vec<String>, skipped: Vec<String> }`.
   - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct ImportPlan { files: Vec<ImportFile>, report: ImportReport }` (files in write order: golden, manifest, approvals when present, config, loop items by id).
   - `#[derive(Debug, Clone, PartialEq, Eq)] pub enum ImportOutcome { Ready(Box<ImportPlan>), Conflict { existing: Vec<PathBuf> } }`.
3. `pub fn golden_v1_to_v2(golden: Golden, components: &Components) -> (Golden, usize)` (pure): when `golden.components` is non-empty return it with `schema_version = GOLDEN_SCHEMA_VERSION` and 0. Otherwise build one placeholder per `[packages]` entry as described in the decisions above, set `schema_version = GOLDEN_SCHEMA_VERSION`, and return the count.
4. `pub fn config_from_project(project_toml: &str, path: &Path) -> Result<(ReleaseConfig, bool)>`: parse as `toml::Table` (error → TomlDeserialize with `path`). `guardrails` absent → `(ReleaseConfig::default(), false)`. Present and a table → deserialize it into `Guardrails` (serde defaults fill missing keys such as `agent_env_markers`), config = default with those guardrails, `validate_config(&cfg, path)?`, return `(cfg, true)`. Present but not a table → ValidationError naming `path`.
5. `pub fn plan_import(from: &Path, paths: &ReleasePaths) -> Result<ImportOutcome>` (reads only):
   - `from` not a directory → ValidationError naming it.
   - Golden: `from/SRC_GOLDEN` missing → ValidationError naming the file; else `parse_golden`; record `golden_schema_from` = its schema_version; `config_from_project` first (step below) so `golden_v1_to_v2` gets its `components`; text = `render_golden(&converted)?`.
   - Manifest: `from/SRC_MANIFEST` missing → ValidationError; else read text, `parse_manifest(&text, path)?`, copy text verbatim.
   - Approvals: `from/SRC_APPROVALS` present → `read_approvals` (count entries), copy verbatim; absent → no file, `approvals_copied = false`.
   - Project: `from/SRC_PROJECT` missing → ValidationError; else `config_from_project`; text = `render_config(&cfg)?`.
   - Items: when `from/SRC_ITEMS` is a directory, every `*.toml` in it sorted by file name: `read_loop_item`; id = `item.id` trimmed, or the file stem when blank; `dest = paths.loop_state_item(&id)?` (invalid id → its ValidationError); a duplicate id → ValidationError naming both files; copy text verbatim.
   - `skipped` = names of top-level entries in `from` other than the five source names, sorted (directories get a trailing `/`).
   - Conflict: collect every planned `dest` that already exists (`Path::exists`, symlinks included via `symlink_metadata`), sorted; non-empty → `Conflict { existing }`.
   - Else `Ready(Box::new(ImportPlan { files, report }))`.
6. `pub fn commit_import(plan: &ImportPlan) -> Result<Vec<PathBuf>>`. Doc comment: the caller holds `with_write_lock` and has passed the §5.1 gate; `plan_import` and this call run under the same lock. For each file in order: create the parent directory, then `atomic_write_str`. Before writing each file re-check `symlink_metadata(dest)` → exists → failure with `ValidationError("<dest> appeared after planning; import writes only new files")`. On any failure: remove every file this call wrote (reverse order) and return the original error; if a removal fails, return `ValidationError(format!("import failed: {original}; rollback also failed: {rollback_err}; check {paths}"))`. Directories created along the way may stay. Return the written paths.
7. Tests (tempdir fixtures written by the test; placeholder data only: package names `example-pkg`, `example-tool`, `libexample1`; paths under `/tmp/neb-test/`; fake shas `"a".repeat(64)`; ReleasePaths via `TrackerStorage::init_standard(tmp, "test-proj")` as config.rs:254 does). A helper writes a source dir with a v1 golden (3 packages, no `[components]`), a minimal manifest, a project.toml whose `[guardrails]` sets `loop_repeat_warn_threshold = 4`, `items/item-001-example.toml` with `[item] id = "item-001-example"` and one `[[attempts]]`, and a `phases/` directory:
   - golden_v1_to_v2: 3 placeholders, each `source_unknown`, `binaries == {name: version}`, `kernel_line == false`; `example-tool` listed in `third_party` → ThirdParty, others Debian; a golden that already has components is unchanged apart from schema_version and returns 0.
   - config_from_project: threshold 4 carried, `agent_env_markers` defaulted; no `[guardrails]` → default and false; `allow_package_removal = true` → ValidationError containing `allow_package_removal`; `guardrails = 1` → ValidationError.
   - happy path: plan Ready with 4 files (no approvals) and `skipped == ["phases/"]`; commit; `read_golden(paths.golden())` → schema 2, 3 components all placeholders; `load_config(&paths)` → threshold 4 and default harness; manifest and item files byte-identical to their sources; item lands at `paths.loop_state_item("item-001-example")`; report counts match.
   - with an `approvals.toml` source (one approval) → 5 files, copy byte-identical, `approvals == 1`.
   - conflict: pre-create `paths.golden()` → Conflict listing it; manifest/config/loop-state not created.
   - refusals write nothing: missing golden source (error names `golden-snapshot.toml`), missing manifest, relaxed guardrail, item with id `bad id`, two items with the same id; `from` is a file.
   - commit rollback: plan Ready, then create a regular file at `paths.loop_state_dir()` so the item write fails → Err, and golden.toml, manifest.toml and config.toml do not exist afterwards.
   - commit re-check: plan Ready, then create `paths.manifest()` → Err naming it, and golden.toml (written first) was removed.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn plan_import' crates/backend_core/src/release/import.rs && grep -q 'pub fn commit_import' crates/backend_core/src/release/import.rs && grep -q 'pub fn golden_v1_to_v2' crates/backend_core/src/release/import.rs && grep -q 'pub fn validate_config' crates/backend_core/src/release/config.rs && grep -q 'pub fn render_config' crates/backend_core/src/release/config.rs && git diff --quiet -- crates/backend_core/Cargo.toml crates/backend_core/src/release/golden.rs crates/backend_core/src/release/manifest.rs crates/backend_core/src/release/approvals.rs crates/backend_core/src/release/loopstate.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25S: Status Summary + Built-in Self-Test Runner

* **Recommended Command:** `/boost`

Scope note (2026-11-10). Library half only, same split as 25O–25R. The handlers (STEP 25U) print the status (text or `--json` envelope) and map the self-test report to exit 0/1. Source is contract §2.9 (self-test cases) and §4.8 (status keys). Both modules only read: status reads `.tracker/release/`, the self-test builds in-memory fixtures plus throwaway temp dirs. No network, no ISO, no root.

Decisions the contract leaves open:
- **Status holds = what the next candidate would hold.** `holds[]` is `ledger_holds(ledger, golden, records)`, not the candidate's `holds_in_force` (that one is shown as a count under `candidate`). With no golden, or a v1 golden (no components), `holds` is empty.
- **Golden "version" is `os_release.VERSION_ID`**, the same field promote names release records by (promote.rs:234). `sha256` is the hash of the file bytes on disk, via `rootfs::sha256_file`. `schema_version` is shown, so a v1 golden says "run import" by itself.
- **Bisect is shown whenever `bisect.toml` exists**, with an `active` flag, so a finished culprit/interaction/cap state stays visible until the user aborts it.
- **Self-test cases are plain functions, not `#[test]`s.** `#[test]` code isn't in the binary, so `selftest.rs` carries its own small fixtures (placeholder data only). Each case returns `Result<(), String>`; the runner wraps each call in `catch_unwind`, so one panicking case is reported as a failure and the rest still run. The default panic hook still prints the panic message to stderr; that's acceptable.
- **The Debian version table is embedded** with `include_str!("testdata/debver_cases.txt")`, the dpkg-generated table from STEP 25B (75 cases). That table already holds the `NebulousCtlSelfTests` cases, so one case covers both parts of §2.9's first bullet.
- **Case names are stable kebab-case strings** in a `pub const SELF_TEST_CASES`, so the CLI and 25V's tests can name a failing case.

```text
[STEP_ID]
STEP 25S: release/status.rs: read_status (golden, candidate, bisect, holds, failures_count, last_release) + render_status_text; release/selftest.rs: run_self_test over the §2.9 cases with a panic-safe runner

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/status.rs (NEW)
  - crates/backend_core/src/release/selftest.rs (NEW)
  - crates/backend_core/src/release/mod.rs (only `pub mod selftest;` and `pub mod status;`, alphabetical)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §2.9 (line 154), §4.8 (line 373)
  - crates/backend_core/src/release/config.rs:31 (ReleasePaths: golden, releases_dir, candidate, failures, bisect), :103 (Guardrails::default: hard stop 3), :10 (DEFAULT_AGENT_ENV_MARKERS), :254 (test ReleasePaths via TrackerStorage::init_standard)
  - crates/backend_core/src/release/golden.rs:36 (GoldenComponent), :47 (Golden), :118 (read_golden)
  - crates/backend_core/src/release/rootfs.rs:114 (sha256_file)
  - crates/backend_core/src/release/candidate.rs:48 (Phases), :172 (Candidate), :467 (read_candidate)
  - crates/backend_core/src/release/bisect.rs:117 (BisectState, is_active, rounds_completed), :265 (start_bisect), :376 (next_round), :527 (record_round), :649 (read_bisect); test fixtures :752 (sample_golden), :805 (sample_candidate); scenarios :1116 (ppp culprit by halving), :1405 (interaction, cap reached)
  - crates/backend_core/src/release/ledger.rs:74 (Failure), :88 (Ledger), :116 (read_ledger: missing → empty)
  - crates/backend_core/src/release/records.rs:30 (ReleaseRecord), :166 (list_records), :208 (latest_record)
  - crates/backend_core/src/release/holds.rs:53 (Hold), :64 (HoldFile), :110 (ledger_holds), :230 (hold_file), :239 (render_hold_file); scenario :330
  - crates/backend_core/src/release/variance.rs:277 (evaluate_against_golden); scenarios :510 (XanMod kernel-line upgrade with ABI renames), :694 (t64 rename vs dropped package)
  - crates/backend_core/src/release/component.rs:31 (is_abi_rename)
  - crates/backend_core/src/release/loopstate.rs:134 (AttemptInput), :347 (record_attempt); scenario :698 (three fails, lock on third)
  - crates/backend_core/src/release/gate.rs:6 (Terminal), :77 (require_user); scenarios :167 (no tty), :187 (CLAUDECODE marker)
  - crates/backend_core/src/release/promote.rs:29 (PromoteInput), :46 (NotReadyReason), :156 (prepare_promote); fixtures :447 (sample_extracted_iso), :511 (sample_candidate); scenario :753 (precondition checks)
  - crates/backend_core/src/release/debver.rs:29 (compare); table parsing at the `dpkg_verified_case_table_matches_and_swaps_symmetrically` test
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No changes to any existing release module (fixtures are re-created in selftest.rs, not exported from test modules). No file writes outside a `tempfile::TempDir` the case creates. No reading the real environment (gate cases use a fake Terminal, never `RealTerminal` or `std::env`). No network, no subprocesses, no ISO. No CLI or envelope (STEP 25T/25U). No new dependencies (`tempfile`, `serde_json`, `chrono` are already in [dependencies]).

[PRE_GATE]
test ! -e crates/backend_core/src/release/status.rs && test ! -e crates/backend_core/src/release/selftest.rs && test -f crates/backend_core/src/release/testdata/debver_cases.txt && grep -q 'pub fn ledger_holds' crates/backend_core/src/release/holds.rs && grep -q 'pub fn read_bisect' crates/backend_core/src/release/bisect.rs && grep -q 'pub fn prepare_promote' crates/backend_core/src/release/promote.rs && grep -q 'pub fn sha256_file' crates/backend_core/src/release/rootfs.rs

[MUTATION_SPEC]
1. status.rs types (all fields `pub`; `#[derive(Debug, Clone, PartialEq, Eq, Serialize)]`; field names are the JSON keys):
   - `StatusGolden { version: Option<String>, sha256: String, schema_version: u32, components_count: usize, packages_count: usize }`.
   - `StatusCandidate { candidate_id: String, created_at: String, iso_sha256: String, phases: Phases, changed_count: usize, holds_in_force_count: usize }` (`Phases` already serializes as `build` / `vm-live-boot` / `vm-install-e2e`).
   - `StatusBisect { candidate_id: String, status: String (BisectStatus::as_str), active: bool, failing_phase: String (Phase::as_str), round: u32, cap: u32, rounds_completed: u32, culprits: Vec<Culprit> }`.
   - `StatusRelease { file: String (file name only), version: String, promoted_at: String, candidate_id: String, iso_sha256: String }`.
   - `ReleaseStatus { golden: Option<StatusGolden>, candidate: Option<StatusCandidate>, bisect: Option<StatusBisect>, holds: Vec<Hold>, failures_count: usize, last_release: Option<StatusRelease> }`.
2. `pub fn read_status(paths: &ReleasePaths) -> Result<ReleaseStatus>` (reads only, no lock):
   - golden: `read_golden(&paths.golden())?`; when Some, `sha256 = sha256_file(&paths.golden())?`, `version` = trimmed non-empty `os_release["VERSION_ID"]` or None.
   - candidate: `read_candidate(&paths.candidate())?` mapped as above.
   - bisect: `read_bisect(&paths.bisect())?` mapped as above.
   - ledger: `read_ledger(&paths.failures())?`; `failures_count = failures.len()`.
   - records: `list_records(&paths.releases_dir())?` when the directory exists, else empty; `last_release` = the last element (same as `latest_record`).
   - holds: `ledger_holds(&ledger, golden, &records)` when golden is Some, else empty.
   - Any reader error propagates unchanged (a corrupt file is an error, not a blank field).
3. `pub fn status_json(s: &ReleaseStatus) -> Result<serde_json::Value>` = `serde_json::to_value(s)`, with a serialization error mapped to `TrackerError::ValidationError(format!("status serialization failed: {e}"))` (error.rs has no serde_json variant; don't add one).
4. `pub fn render_status_text(s: &ReleaseStatus) -> String`, one line per section, in this order, each `label: value`:
   - `golden: <version or "(no VERSION_ID)"> sha256 <first 12 of sha256> schema v<n>, <components_count> components` or `golden: none`; when schema_version < 2 append `  (schema v1: run neb release import)`.
   - `candidate: <candidate_id> build=<state> vm-live-boot=<state> vm-install-e2e=<state>, <changed_count> changed` or `candidate: none` (`PhaseState::as_str`).
   - `bisect: <status> round <round>/<cap> on <failing_phase>` (append ` (inactive)` when not active) or `bisect: none`.
   - `holds: <n>` then one indented line per hold: `  <component> held at <held_version> (blocked <blocked_version>, <reason>)` (`HoldReason::as_str`).
   - `failures: <failures_count>`.
   - `last release: <version> promoted <promoted_at> (<file>)` or `last release: none`.
   - Lines joined with `\n`, trailing newline.
5. selftest.rs public API:
   - `#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct CaseResult { name: &'static str, passed: bool, detail: String }` (detail empty on pass).
   - `#[derive(Debug, Clone, PartialEq, Eq, Serialize)] pub struct SelfTestReport { cases: Vec<CaseResult> }` with `pub fn all_passed(&self) -> bool` and `pub fn failed(&self) -> Vec<&CaseResult>`.
   - `pub const SELF_TEST_CASES: &[&str]` listing the case names below, in run order.
   - `pub fn run_self_test() -> SelfTestReport` = private `run_cases(&CASES)` where `CASES: &[(&str, fn() -> std::result::Result<(), String>)]` pairs each name with its function.
   - Private `fn run_cases(cases: &[(&'static str, fn() -> std::result::Result<(), String>)]) -> SelfTestReport`: for each, `std::panic::catch_unwind(AssertUnwindSafe(f))`; `Ok(Ok(()))` → passed; `Ok(Err(msg))` → failed with msg; `Err(payload)` → failed with `panicked: <payload as &str or String, else "unknown panic">`. Never stops early.
   - Cases use `?`-style early returns with a short `format!` message naming what differed (a private `fn ensure(cond: bool, msg: impl FnOnce() -> String) -> Result<(), String>` helper is fine). No `unwrap`/`expect` in case bodies: map errors with `.map_err(|e| e.to_string())`.
6. Cases (name → what it proves). Fixtures: package/component names `example-pkg`, `example-lib`, `a-pkg`, `b-pkg`, `c-pkg`, `d-pkg`, `libexample1`; ISO paths under `/tmp/neb-test/`; shas `"a".repeat(64)` / `"b".repeat(64)`; candidate ids like `aaaaaaaaaaaa-20261110T120000Z`; a fixed `Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0)`. One private `fn golden_with(components: &[(&str, &str)]) -> Golden` (one binary per component, same name and version, Debian, as bisect.rs:752) and one `fn candidate_with(changed: &[(&str, Option<&str>, &str)]) -> Candidate` (as bisect.rs:805, all phases Pass unless the case sets them) are shared by the cases.
   - `debver-table`: every non-comment line of the embedded table, `compare(a, b)` equals the op and `compare(b, a)` equals its reverse; at least 60 cases counted.
   - `variance-upgrade-passes`: golden `example-pkg 1.0-1`, live `1.1-1` → `evaluate_against_golden` passes with exactly one upgrade.
   - `variance-downgrade-blocks`: live `0.9-1` → one violation of type `golden_component_downgrade`.
   - `variance-removal-blocks-unless-approved`: live without the component → one `golden_component_removed`; with an `Approval { item_name: "example-pkg", version: Some("1.0-1"), approved: true, .. }` → pass.
   - `abi-rename-kernel`: `is_abi_rename("linux-image-7.2.15-x64v3-xanmod1", "linux-image-7.3.3-x64v3-xanmod1")` is true; the variance.rs:510 scenario (kernel-line component 7.2.15 → 7.3.3 with renamed image/headers binaries) passes with the upgrade and its ABI renames and no violations. Placeholder version suffixes only (no real build dates or git hashes).
   - `abi-rename-t64`: `is_abi_rename("libexample1", "libexample1t64")` is true and `is_abi_rename("libexample1", "libother1")` is false; a component whose only binary goes `libexample1` → `libexample1t64` at the same version passes with one `AbiRename` and no violations (variance.rs:694, libpcap half).
   - `loop-breaker-three-strikes`: in a `TempDir`, three `record_attempt` calls on `<tmp>/item-001-example.toml` with the same failing `error_summary` and `Guardrails::default()` → after 1 not locked, after 3 `loop_locked == true` and `retry_count == 3` (loopstate.rs:698).
   - `gate-refuses-no-tty`: a private fake `Terminal` (stdin not a tty) → `require_user` returns `DenyReason::NotTty` and the fake's prompt was never called.
   - `gate-refuses-agent-marker`: fake with both ttys true and `CLAUDECODE=1` (markers = `DEFAULT_AGENT_ENV_MARKERS`) → `DenyReason::AgentMarker("CLAUDECODE")`, prompt never called.
   - `hold-file-generation`: ledger with one `Failure { component: "example-pkg", version: "1.1-1", .. }`, golden `example-pkg 1.0-1` → `ledger_holds` gives one hold (held `1.0-1`, blocked `1.1-1`); `render_hold_file(&hold_file(holds, ..))` parses back with `toml::from_str::<HoldFile>` to the same holds; with golden at `1.2-1` the hold is no longer active (holds.rs:330 rule).
   - `bisect-halving`: candidate changes `a-pkg`, `b-pkg`, `c-pkg`, `d-pkg` (each 1.0-1 → 1.1-1, vm-live-boot FailConfirmed); `start_bisect` → Started; drive `next_round` / `record_round` with fail (`class: Some("component")`) when the tested set contains `c-pkg` and pass otherwise → terminates with status Culprit naming `c-pkg` within `cap` rounds, and the returned failures include one `source = bisect` entry for `c-pkg 1.1-1` (bisect.rs:1116 pattern). The driver loop is bounded by `cap + 1` iterations and fails the case if exceeded.
   - `bisect-interaction`: two changes, both rounds pass → status Interaction, members both (bisect.rs:1405 first half).
   - `bisect-cap`: the cap-reached half of bisect.rs:1405 → status CapReached and `next_round` then returns `NextOutcome::CapReached`.
   - `promote-preconditions`: with the promote.rs:447/:511-style fixtures, `prepare_promote` returns `NotReady` with `NoCandidate` (candidate None), `PhasesIncomplete` (one phase Pending), `BisectActive` (`active_bisect_candidate` = the candidate id) and `IsoMismatch` (different `iso_sha256`), per promote.rs:753.
7. Tests:
   - status.rs (tempdir + `TrackerStorage::init_standard(tmp, "test-proj")` as config.rs:254): empty tree → all None, empty holds, 0 failures, and the text is exactly the six "none"/0 lines; a written golden (VERSION_ID `2026.11`, one component), a candidate, a ledger entry matching the golden component and one release record → every field populated, `holds.len() == 1`, `golden.sha256 == sha256_file(...)`, `status_json` has exactly the keys `golden`, `candidate`, `bisect`, `holds`, `failures_count`, `last_release`, and `render_status_text` contains the hold line; a v1 golden → text contains `run neb release import` and `holds` is empty; a corrupt `candidate.toml` → Err.
   - selftest.rs: `run_self_test()` → `all_passed()`, and the case names equal `SELF_TEST_CASES` in order; `run_cases` with one passing, one `Err("boom")` and one panicking fn → three results, the last two failed with details `boom` and starting `panicked:`, and `failed().len() == 2`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn read_status' crates/backend_core/src/release/status.rs && grep -q 'pub fn render_status_text' crates/backend_core/src/release/status.rs && grep -q 'pub fn run_self_test' crates/backend_core/src/release/selftest.rs && grep -q 'pub const SELF_TEST_CASES' crates/backend_core/src/release/selftest.rs && grep -q 'catch_unwind' crates/backend_core/src/release/selftest.rs && grep -q 'include_str!("testdata/debver_cases.txt")' crates/backend_core/src/release/selftest.rs && ! grep -q 'RealTerminal\|std::env::var' crates/backend_core/src/release/selftest.rs && git diff --quiet -- crates/backend_core/Cargo.toml crates/backend_core/src/release/bisect.rs crates/backend_core/src/release/promote.rs crates/backend_core/src/release/holds.rs crates/backend_core/src/release/variance.rs crates/backend_core/src/release/gate.rs crates/backend_core/src/release/loopstate.rs crates/backend_core/src/release/debver.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25T: `neb release` Clap Subcommands

* **Recommended Command:** `/goal`

Scope note (2026-11-11). Argument definitions only: `ReleaseCommand`, `BisectCommand` and the small value enums in `args.rs`, with parse tests. Source is contract §2.1–2.9, §2.10, §4.2–4.8 and §6.1's `--help` list (17 subcommands). The handlers, exit codes and envelope are STEP 25U.

Decisions the contract leaves open:
- **`Commands::Release` is not added here.** `commands/mod.rs` matches `Commands` exhaustively (no `_` arm), so a new variant can't compile without a handler arm, and a stub arm would just be a placeholder for 25U to tear out. 25T defines the enums; 25U adds the one `Commands::Release { action: ReleaseCommand }` variant, its dispatch arm and the `lib.rs` re-export together. So 25U's row gains `args.rs` (that variant only) and `lib.rs`. `nebulous_cli` is a lib crate (`pub mod args`), so the unused-for-now `pub` enums don't trip `dead_code`.
- **CLI-side value enums with `From` into the backend types.** backend_core has no clap dependency and shouldn't get one, so args.rs defines `#[derive(ValueEnum)]` mirrors (`StageArg`, `PhaseArg`, `ResultArg`, `HoldFormat`) whose clap names match the contract's spellings, plus `From` impls into `loopstate::Stage`, `ledger::Phase`, `candidate::PhaseResult` and `loopstate::AttemptStatus`. A typo then fails at parse time with clap's list of valid values.
- **Clap enforces what the contract states as argument rules; the handler enforces the rest.** In clap: required args, `record-phase fail` and `bisect record fail` require `--class` and `--classifier-rule`, and `verify-variance` takes exactly one of `--iso` or `--repo-only`. In the handler (25U): `extract-golden --output` being required without `--json` (`--json` is a global flag on `Cli` that this enum can't see), the `--class component` value rule (§4.3: `infra`/`unknown` → exit 1 `REJECTED_CLASS` with a JSON body, which a clap error can't give), and resolving `--item` ids vs paths.
- **Ids and paths stay `String` where the contract lets them be either** (`--item <id|path>`, `--release <record path|latest>`): the handler decides.

```text
[STEP_ID]
STEP 25T: nebulous_cli args.rs: ReleaseCommand (17 subcommands), BisectCommand, StageArg/PhaseArg/ResultArg/HoldFormat value enums with From into backend types; parse tests

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/args.rs (new enums and a `#[cfg(test)] mod tests` at the end; `Cli` and `Commands` do not change)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §2.1–2.9 (lines 73-161), §2.10 (line 163), §4.2 (line 241), §4.3 (record-failure args, line 264), §4.4 (line 276), §4.5 (line 309), §4.6, §4.7 (line 350), §4.8, §6.1 (line 442: the `--help` list)
  - crates/nebulous_cli/src/args.rs:28 (Commands; existing doc-comment and `#[arg]` style), :608 (GitCommand: `value_delimiter = ','` example)
  - crates/nebulous_cli/src/lib.rs:1 (`pub mod args`), :20 (`Cli::command().debug_assert()` test)
  - crates/nebulous_cli/src/commands/mod.rs:55 (exhaustive `match` on Commands: why no variant is added here)
  - crates/backend_core/src/release/loopstate.rs:42 (Stage: build, live-boot, install-e2e), :60 (AttemptStatus)
  - crates/backend_core/src/release/ledger.rs:14 (Phase: build, vm-live-boot, vm-install-e2e)
  - crates/backend_core/src/release/candidate.rs:94 (PhaseResult)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No change to `Cli`, `Commands` or any existing enum; no `Commands::Release` variant (STEP 25U). No handler code, no I/O, no exit codes, no envelope. No clap dependency or derive added to backend_core. No new dependencies (clap `derive` already includes `ValueEnum`).

[PRE_GATE]
! grep -q 'ReleaseCommand' crates/nebulous_cli/src/args.rs && ! grep -qE '^ +Release \{' crates/nebulous_cli/src/args.rs && grep -q 'pub enum Stage' crates/backend_core/src/release/loopstate.rs && grep -q 'pub enum PhaseResult' crates/backend_core/src/release/candidate.rs && grep -q 'impl FromStr for Phase' crates/backend_core/src/release/ledger.rs && grep -q 'pub fn run_self_test' crates/backend_core/src/release/selftest.rs

[MUTATION_SPEC]
1. Value enums (`#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]`, clap's default kebab-case names; check each spelling below in a test):
   - `StageArg { Build, LiveBoot, InstallE2e }` → `build`, `live-boot`, `install-e2e`; `impl From<StageArg> for backend_core::release::loopstate::Stage`.
   - `PhaseArg { Build, VmLiveBoot, VmInstallE2e }` → `build`, `vm-live-boot`, `vm-install-e2e`; `impl From<PhaseArg> for backend_core::release::ledger::Phase`.
   - `ResultArg { Pass, Fail }` → `pass`, `fail`; `impl From<ResultArg> for candidate::PhaseResult` and `impl From<ResultArg> for loopstate::AttemptStatus`.
   - `HoldFormat { HoldFile }` → `hold-file`.
2. `#[derive(Debug, Clone, Subcommand)] pub enum ReleaseCommand`, one doc comment per variant (one line, saying what it does and, for the five user-only ones, ending "User-only: needs an interactive terminal."). Field types are `PathBuf` for file paths, `String` otherwise. Variants, with clap names from the derive (kebab-case):
   - `ExtractGolden { #[arg(long)] iso: PathBuf, #[arg(long)] output: Option<PathBuf> }`.
   - `VerifyVariance { #[arg(long, required_unless_present = "repo_only", conflicts_with = "repo_only")] iso: Option<PathBuf>, #[arg(long)] repo_only: bool, #[arg(long)] manifest: Option<PathBuf>, #[arg(long)] golden: Option<PathBuf>, #[arg(long)] approvals: Option<PathBuf> }`.
   - `RecordAttempt { #[arg(long)] item: String, #[arg(long)] stage: StageArg, #[arg(long)] status: ResultArg, #[arg(long, default_value = "")] error_summary: String, #[arg(long)] log_file: Option<PathBuf> }`.
   - `CheckGuardrail { #[arg(long)] item: String }`.
   - `BlessGolden { #[arg(long)] iso: PathBuf, #[arg(long)] reason: String }`.
   - `ApproveVariance { #[arg(long)] item_name: String, #[arg(long)] reason: String, #[arg(long)] version: Option<String> }`.
   - `UnlockLoop { #[arg(long)] item: String }`.
   - `PinsFromRelease { #[arg(long, default_value = "latest")] release: String, #[arg(long, value_delimiter = ',')] components: Vec<String>, #[arg(long, value_enum, default_value_t = HoldFormat::HoldFile)] format: HoldFormat }`.
   - `SelfTest`.
   - `Candidate { #[arg(long)] iso: PathBuf, #[arg(long)] snapshot: Option<String>, #[arg(long)] artifact_cache: Option<PathBuf>, #[arg(long)] hold_file: Option<PathBuf> }`.
   - `RecordPhase { phase: PhaseArg (positional), result: ResultArg (positional), #[arg(long = "log", required = true)] logs: Vec<PathBuf> (repeatable `--log a --log b`), #[arg(long)] iso: PathBuf, #[arg(long, required_if_eq("result", "fail"))] class: Option<String>, #[arg(long, required_if_eq("result", "fail"))] classifier_rule: Option<String>, #[arg(long)] verdict: Option<PathBuf> }`.
   - `RecordFailure { #[arg(long)] component: String, #[arg(long)] version: String, #[arg(long)] phase: PhaseArg, #[arg(long)] class: String, #[arg(long)] classifier_rule: String, #[arg(long)] log: String, #[arg(long, value_delimiter = ',')] members: Vec<String> }`.
   - `Holds { #[arg(long)] all_last_good: bool, #[arg(long, value_enum)] format: Option<HoldFormat> }`.
   - `Promote { #[arg(long)] iso: PathBuf }`.
   - `Bisect { #[command(subcommand)] action: BisectCommand }`.
   - `Status`.
   - `Import { #[arg(long)] from: PathBuf }`.
3. `#[derive(Debug, Clone, Subcommand)] pub enum BisectCommand { Start, Next, Record { result: ResultArg (positional), #[arg(long, required_if_eq("result", "fail"))] class: Option<String>, #[arg(long, required_if_eq("result", "fail"))] classifier_rule: Option<String>, #[arg(long)] log: Option<String> }, Status, Abort }`, each with a one-line doc comment (`Abort` is user-only).
4. Tests at the end of args.rs, through a test-only wrapper `#[derive(Debug, Parser)] #[command(name = "release")] struct ReleaseCli { #[command(subcommand)] cmd: ReleaseCommand }` (placeholder paths under `/tmp/neb-test/`):
   - `ReleaseCli::command().debug_assert()`.
   - The wrapper's subcommand names are exactly the 17 of contract §6.1, in any order: extract-golden, verify-variance, record-attempt, check-guardrail, bless-golden, approve-variance, unlock-loop, pins-from-release, self-test, candidate, record-phase, record-failure, holds, promote, bisect, status, import (read them from `ReleaseCli::command().get_subcommands()`).
   - One successful parse per subcommand, asserting the fields: e.g. `record-attempt --item item-001-example --stage live-boot --status fail` → `StageArg::LiveBoot`, `ResultArg::Fail`, `error_summary == ""`; `pins-from-release` → `release == "latest"`, empty components, `HoldFile`; `pins-from-release --components a-pkg,b-pkg` → two components; `record-failure ... --members a-pkg,b-pkg` → two members; `record-phase vm-live-boot fail --log /tmp/neb-test/a.log --log /tmp/neb-test/b.log --iso /tmp/neb-test/c.iso --class component --classifier-rule example-rule` → two logs; `bisect record pass` and `bisect abort`; `verify-variance --repo-only`.
   - Refusals (`try_parse_from` is Err): `extract-golden` without `--iso`; `verify-variance` with neither, and with both `--iso` and `--repo-only`; `record-phase build fail --log x --iso y` without `--class`; `bisect record fail` without `--class`; `record-attempt ... --stage boot` (not a valid value); `record-phase` with no `--log`; `import` without `--from`.
   - `From` mappings: every `StageArg`, `PhaseArg` and `ResultArg` value converts to the backend variant whose `as_str()` equals the clap name (`PossibleValue::get_name()` via `ValueEnum::to_possible_value`).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub enum ReleaseCommand' crates/nebulous_cli/src/args.rs && grep -q 'pub enum BisectCommand' crates/nebulous_cli/src/args.rs && grep -q 'impl From<PhaseArg> for' crates/nebulous_cli/src/args.rs && ! grep -qE '^ +Release \{' crates/nebulous_cli/src/args.rs && git diff --quiet -- crates/nebulous_cli/src/commands/mod.rs crates/nebulous_cli/src/lib.rs crates/nebulous_cli/Cargo.toml crates/backend_core/Cargo.toml crates/backend_core/src

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 25U: `neb release` Plumbing + Baseline Handlers

* **Recommended Command:** `/boost`

Scope note (2026-11-11). The old 25U row ("all handlers, exit codes, envelope") was split four ways so that no step needs a stub:
- **25U (this step):** the shared plumbing plus the eight handlers that read state or touch only loop-state.
- **27F:** lifecycle handlers (candidate, record-phase, record-failure, promote).
- **27G:** bisect and the user-only handlers.
- **27H:** wires `Commands::Release`, the dispatcher and `main.rs` exit codes.

Until 27H, the handlers are `pub fn`s called from unit tests. `nebulous_cli` is a lib crate, so they don't trip `dead_code`. Source is contract §1.1–1.3, §2.1–2.4, §2.8, §2.9, §4.4 and §4.8.

Decisions the contract leaves open:
- **A handler returns a `ReleaseRun`, never `Err`.** A run is `{exit, status, body, text, text_to_stderr}`. Library errors become a run through `ReleaseRun::from_error`:
  - `ExitCode::from_error` picks the code: 7 for a lock timeout or schema mismatch, 1 otherwise.
  - The status is `STATE_CONFLICT` or `ERROR`, the body is `{"error": "<message>"}`, and the text is `Error: <message>` on stderr.
  - That way every exit path has an envelope (§1.2), and 27H's dispatcher only has to print and exit.
- **Output rule, in `render`.**
  - With `--json`: stdout is the pretty-printed envelope plus a newline, and stderr is the text (§1.2).
  - Without `--json`: the text goes to stdout, or to stderr when `text_to_stderr` is set (errors).
  - For `holds --format hold-file` and `pins-from-release` without `--json`, the text *is* the TOML hold file, so `build-novos.sh` can redirect stdout straight into a file.
- **Read-only reports use status `PASS`** (self-test, status, holds, pins). A failed self-test uses `SELF_TEST_FAILED` with exit 1.
- **`check-guardrail`'s item status goes in `item_status`, not `status`.** The Python put the item's own status (`in_progress`) in `status`, but here `status` belongs to the envelope (`envelope()` drops a body key named `status`). The envelope status is `HARD_STOP_LOOP_LOCK` (exit 3), else `HARD_STOP_VARIANCE_LOCK` (exit 2), else `PASS`. The other §2.4 keys stay as they are. 13C/25W callers read `item_status`.
- **`--item <id|path>`:** a value containing `/` or ending in `.toml` is a path, taken as given. Otherwise it's an id, resolved with `ReleasePaths::loop_state_item(id)`, which rejects bad ids.
- **`extract-golden`:**
  - It refuses a test ISO (`!identity.is_production()` → exit 1). Same rule as candidate and bless-golden: a test ISO's inventory is identical, so only the identity check tells them apart.
  - `--output` is required without `--json`.
  - It refuses an `--output` that resolves to `.tracker/release/golden.toml` (§2.1: extracting is not blessing).
  - With `--json` and no `--output`, the rendered TOML goes in the envelope as `golden_toml`, with `output: null`.
- **`verify-variance`:**
  - Repo invariants always run. The ISO check runs too when `--iso` is given (clap already makes `--iso` and `--repo-only` exclusive).
  - It does not refuse a test ISO: post-build variance runs on whatever was built, and the inventories are identical.
  - `held[]` = the ledger holds (`ledger_holds` against golden) whose component is in the live inventory at exactly the hold's `held_version`, as `{component, version}`. It's empty without a golden.
  - A missing manifest is exit 1 (`manifest.toml missing; run neb release import`). A missing golden is allowed: the ISO is checked against the manifest only.
- **`holds` and `pins-from-release`:**
  - `holds` without a golden is exit 1 (`golden.toml missing; run neb release import`).
  - The hold files are stamped `generated_at` = now (RFC 3339, seconds, `Z`) and `generated_by` = `"neb release holds"` / `"neb release pins-from-release"`.
- **`self-test` needs no workspace.** It runs before `.tracker/` exists (NovOS `verify-deps.sh`), so it doesn't open a `ReleaseCtx`.
- **Locking.** Only `record-attempt` writes in this step. It runs inside `storage.with_write_lock`, so a lock timeout becomes exit 7 through `from_error`. Nothing else here takes the lock.
- **The clock is an input.** `ReleaseCtx.now: DateTime<Utc>` is set by the caller (27H: `Utc::now()`), so tests are deterministic.

```text
[STEP_ID]
STEP 25U: nebulous_cli commands/release: ReleaseCtx, ReleaseRun, render (envelope / stderr rule), from_error; handlers self-test, status, holds, pins-from-release, extract-golden, verify-variance, record-attempt, check-guardrail

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/commands/release/mod.rs (NEW)
  - crates/nebulous_cli/src/commands/release/baseline.rs (NEW)
  - crates/nebulous_cli/src/commands/mod.rs (only `pub mod release;`, placed right after the `use crate::args::{…};` block at the top; the file has no other module items besides `mod tests`)
READ_ONLY_REFS:
  - /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md §1.1 (line 19), §1.2 (line 34), §1.3 (lock rule, line 65), §2.1–2.4 (lines 73-112), §2.8 (line 141), §2.9 (line 154), §4.4 (line 272), §4.8 (line 373)
  - crates/nebulous_cli/src/args.rs (ReleaseCommand fields from STEP 25T: StageArg, ResultArg, HoldFormat and their From impls)
  - crates/nebulous_cli/src/commands/mod.rs:294 (resolve_storage)
  - crates/nebulous_cli/src/main.rs (current exit handling; 27H changes it, not this step)
  - crates/backend_core/src/release/mod.rs:38 (ExitCode, from_error), :77 (envelope), :105 (Violation), :127 (Outcome)
  - crates/backend_core/src/storage/mod.rs:406 (project_root), :492 (with_write_lock, 30 s)
  - crates/backend_core/src/release/config.rs:31 (ReleasePaths::new, loop_state_item), :175 (load_config)
  - crates/backend_core/src/release/iso.rs:34 (IsoIdentity::is_production), :138 (extract_iso)
  - crates/backend_core/src/release/component.rs:137 (golden_from_inventory)
  - crates/backend_core/src/release/golden.rs:118 (read_golden), :133 (render_golden), :155 (write_golden)
  - crates/backend_core/src/release/manifest.rs:159 (read_manifest)
  - crates/backend_core/src/release/approvals.rs:46 (read_approvals)
  - crates/backend_core/src/release/repo_invariants.rs:95 (verify_repo_invariants); test fixtures :340 (sample_manifest), :371 (write_clean_repo)
  - crates/backend_core/src/release/variance.rs:16 (Upgrade), :23 (AbiRename), :250 (evaluate_variance)
  - crates/backend_core/src/release/holds.rs:110 (ledger_holds), :153 (all_last_good), :175 (pins_from_release), :230 (hold_file), :239 (render_hold_file)
  - crates/backend_core/src/release/records.rs:109 (read_record), :166 (list_records), :208 (latest_record)
  - crates/backend_core/src/release/ledger.rs:116 (read_ledger)
  - crates/backend_core/src/release/loopstate.rs:134 (AttemptInput), :236 (git_diff_fingerprint), :347 (record_attempt), :449 (guardrail_state)
  - crates/backend_core/src/release/status.rs (read_status, status_json, render_status_text — STEP 25S)
  - crates/backend_core/src/release/selftest.rs (run_self_test, SelfTestReport — STEP 25S)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No `Commands::Release` variant, no dispatcher over `ReleaseCommand`, no change to `execute`/`execute_to_string`/`main.rs`/`lib.rs` (STEP 27H). No lifecycle, bisect or user-only handlers (27F/27G), no tty gate. No change to backend_core. No `std::process::exit`, no printing from handlers (they return text). No new dependencies.

[PRE_GATE]
test ! -e crates/nebulous_cli/src/commands/release && grep -q 'pub enum ReleaseCommand' crates/nebulous_cli/src/args.rs && grep -q 'pub fn read_status' crates/backend_core/src/release/status.rs && grep -q 'pub fn run_self_test' crates/backend_core/src/release/selftest.rs && grep -q 'pub fn extract_iso(' crates/backend_core/src/release/iso.rs && grep -q 'pub fn from_error' crates/backend_core/src/release/mod.rs && ! grep -q 'pub mod release' crates/nebulous_cli/src/commands/mod.rs

[MUTATION_SPEC]
1. release/mod.rs (`pub mod baseline;`):
   - `#[derive(Debug, Clone, PartialEq)] pub struct ReleaseRun { pub exit: ExitCode, pub status: String, pub body: serde_json::Value, pub text: String, pub text_to_stderr: bool }` (`ExitCode` is `backend_core::release::ExitCode`), with:
     - `pub fn new(exit, status: impl Into<String>, body: Value, text: impl Into<String>) -> Self` (`text_to_stderr = false`).
     - `pub fn from_error(e: &TrackerError) -> Self`: exit `ExitCode::from_error(e)`; status `"STATE_CONFLICT"` when that is StateConflict, else `"ERROR"`; body `{"error": e.to_string()}`; text `"Error: <e>"`; `text_to_stderr = true`.
   - `#[derive(Debug, Clone, PartialEq, Eq)] pub struct Rendered { pub stdout: String, pub stderr: String, pub code: i32 }` and `pub fn render(command: &str, run: &ReleaseRun, json: bool) -> Rendered`:
     - json: stdout = `serde_json::to_string_pretty(&envelope(command, &run.status, run.exit, run.body.clone()))` + `\n`; stderr = text.
     - human: the text goes to stdout, or to stderr when `text_to_stderr`, and the other stream is empty.
     - Non-empty text always ends with `\n` (add one when missing). `code = run.exit.code()`.
   - `pub struct ReleaseCtx { pub storage: TrackerStorage, pub paths: ReleasePaths, pub config: ReleaseConfig, pub json: bool, pub now: DateTime<Utc> }` with:
     - `pub fn open(root: Option<&Path>, json: bool, now: DateTime<Utc>) -> Result<Self>` = `resolve_storage(root)?`, `ReleasePaths::new`, `load_config(&paths)?`.
     - `pub fn project_root(&self) -> PathBuf` (storage.project_root()).
     - `pub fn timestamp(&self) -> String` (`now.to_rfc3339_opts(SecondsFormat::Secs, true)`).
   - `pub fn resolve_item_path(paths: &ReleasePaths, item: &str) -> Result<PathBuf>` per the `--item` rule above.
   - A private helper `fn run_or_error(f: impl FnOnce() -> Result<ReleaseRun>) -> ReleaseRun` (Err → `from_error`) used by every handler, so handler bodies can use `?`.
2. release/baseline.rs, each `pub fn` returning `ReleaseRun` (the command names in comments are the envelope `command` 27H will pass):
   - `handle_self_test() -> ReleaseRun` (`self-test`):
     - Runs `run_self_test()`. Exit 0 / status `PASS` when all pass, else exit 1 / `SELF_TEST_FAILED`.
     - Body `{"cases": [...], "passed": n, "failed": n}`.
     - Text: one line per case, `ok   <name>` or `FAIL <name>: <detail>`, then `<passed>/<total> passed`.
   - `handle_status(ctx: &ReleaseCtx) -> ReleaseRun` (`status`): `read_status(&ctx.paths)`. Body = `status_json` (its object keys merge into the envelope), text = `render_status_text`, status `PASS`.
   - `handle_holds(ctx, all_last_good: bool, format: Option<HoldFormat>) -> ReleaseRun` (`holds`):
     - Reads golden (missing → error per the decision above), the ledger and `list_records`.
     - Holds = `all_last_good(golden, records)` or `ledger_holds(ledger, golden, records)`.
     - Body `{"holds": [...], "count": n}`.
     - Text: the rendered hold file when `format == Some(HoldFile)`, else a table with one line per hold (`<component>  held <held_version>  blocked <blocked_version>  <reason>`), or `no holds`.
   - `handle_pins_from_release(ctx, release: &str, components: &[String]) -> ReleaseRun` (`pins-from-release`):
     - `"latest"` → `latest_record(releases_dir)`; None → error `no release records in <dir>`. Anything else → `read_record(Path::new(release))`.
     - `pins_from_release(&record, (!components.is_empty()).then_some(components))`; its missing-component error passes through.
     - Body `{"holds": [...], "release_record": <file name>}`. Text = the rendered hold file.
   - `handle_extract_golden(ctx, iso: &Path, output: Option<&Path>) -> ReleaseRun` (`extract-golden`):
     - Refusal order: no output and not json → error `--output is required without --json`.
     - Then `extract_iso(iso, &project_root)`. A test ISO → error naming the ISO and its `extra_live_layers`.
     - Then `golden_from_inventory(&ex.inventory, &ctx.config.components)`.
     - With an output: refuse when it is the golden path (compare `fs::canonicalize` of the output's parent joined with its file name against `canonicalize(paths.root())` joined with `golden.toml`; when either parent doesn't exist, compare the paths as given). Otherwise `write_golden(output, &g)`.
     - Body: `packages_count`, `components_count`, `custom_binaries_count`, `output` (string or null), and `golden_toml` only when there's no output. Status `EXTRACTED`.
   - `handle_verify_variance(ctx, iso: Option<&Path>, repo_only: bool, manifest: Option<&Path>, golden: Option<&Path>, approvals: Option<&Path>) -> ReleaseRun` (`verify-variance`):
     - The overrides default to `ctx.paths.{manifest,golden,approvals}()`.
     - Violations = `verify_repo_invariants(&manifest, &approvals, &project_root)`, plus, with an ISO, `evaluate_variance(&live, &manifest, golden.as_ref(), &approvals)` (live = `golden_from_inventory` of `extract_iso`).
     - Status `PASS` / exit 0, or `HARD_STOP_VARIANCE_LOCK` / exit 2.
     - Body: `repo_only` (true when no ISO), `checked_packages_count`, `checked_binaries_count`, `checked_components_count` (0 when repo-only), `upgraded_count`, `upgrades`, `abi_renames`, `held`, `violations`.
     - Text: one line per violation (`<type>: <item>: <detail>`), then `variance: PASS` or `variance: HARD_STOP_VARIANCE_LOCK (<n> violations)`.
   - `handle_record_attempt(ctx, item: &str, stage: StageArg, status: ResultArg, error_summary: &str, log_file: Option<&Path>) -> ReleaseRun` (`record-attempt`):
     - Runs inside `ctx.storage.with_write_lock`: `resolve_item_path`, then `AttemptInput { stage.into(), status.into(), error_summary, log_file, diff: git_diff_fingerprint(&project_root), timestamp: ctx.timestamp() }`, then `record_attempt(path, input, &ctx.config.guardrails)`.
     - Status `ATTEMPT_RECORDED` / exit 0, or `HARD_STOP_LOOP_LOCK` / exit 3 when `loop_locked`.
     - Body = the `RecordOutcome` fields.
     - Text: `item <id>: retry <n>, locked <bool>`, plus one line per negative constraint.
   - `handle_check_guardrail(ctx, item: &str) -> ReleaseRun` (`check-guardrail`):
     - `guardrail_state(path)`; None → error `loop-state item not found: <path>`.
     - Repo violations as in verify-variance (manifest required).
     - Status and exit per the decision above.
     - Body: `item_id`, `item_status`, `loop_locked`, `retry_count`, `negative_constraints`, `variance_status` (`PASS`/`HARD_STOP_VARIANCE_LOCK`), `variance_violations`.
     - Text: `GUARDRAIL_STATE: item=<id> status=<item_status> locked=<bool> retry_count=<n> negative_constraints=<n> variance=<variance_status>` (the Python line).
3. Tests. They live in the two files' `#[cfg(test)]` modules, use `tempfile` and build the workspace with `TrackerStorage::init_standard(tmp, "test-proj")` and `ReleaseCtx { .. }` directly, with a fixed `now`. Placeholder data only (`example-pkg`, `/tmp/neb-test/`, `"a".repeat(64)`). No ISO tooling: ISO paths are only tested on their refusal paths.
   - mod.rs:
     - `render` json vs human, the stderr rule, the trailing newline.
     - `from_error` with a `LockTimeout` gives exit 7 / `STATE_CONFLICT`, and with a `ValidationError` gives exit 1 / `ERROR`.
     - `resolve_item_path` with an id, with a `.toml` path, and with `bad id` (Err).
   - baseline.rs:
     - **self-test:** exit 0 / `PASS`, 14 cases.
     - **status:** an empty tree gives `PASS`, and the envelope after `render(.., true)` has the six §4.8 keys.
     - **holds:** no golden → exit 1. A golden with one ledger entry → count 1, the table line, and the hold-file text parses as `HoldFile`. `all_last_good` → one hold per golden component.
     - **pins-from-release:**
       - `latest` with no records → exit 1.
       - A written record → its holds.
       - An unknown component → exit 1, naming it.
     - **extract-golden:**
       - No output and not json → exit 1, and the ISO is never touched (pass a nonexistent ISO path and check that the error is the `--output` one).
       - Output == the golden path → exit 1.
       - A nonexistent ISO with a valid output → exit 1.
     - **verify-variance --repo-only:**
       - A missing manifest → exit 1.
       - A clean repo tree → exit 0 / `PASS`. Re-create the repo_invariants.rs:340/:371 fixture in the test: it's private to backend_core.
       - The same tree with one required custom tool removed → exit 2, and the violations are non-empty.
     - **record-attempt:** in a git-less tempdir, three identical `fail`s on `item-001-example` give exit 0, 0, then 3 / `HARD_STOP_LOOP_LOCK`, and the file lands at `paths.loop_state_item("item-001-example")`.
     - **check-guardrail:**
       - A missing item → exit 1.
       - The locked item from the record-attempt case → exit 3, with `item_status` present and no body `status` key.
       - An unlocked item with a clean repo → exit 0.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub fn render' crates/nebulous_cli/src/commands/release/mod.rs && grep -q 'pub fn from_error' crates/nebulous_cli/src/commands/release/mod.rs && grep -q 'pub fn handle_verify_variance' crates/nebulous_cli/src/commands/release/baseline.rs && grep -q 'pub fn handle_check_guardrail' crates/nebulous_cli/src/commands/release/baseline.rs && grep -q 'is_production' crates/nebulous_cli/src/commands/release/baseline.rs && ! grep -rq 'process::exit\|println!\|eprintln!' crates/nebulous_cli/src/commands/release && ! grep -qE '^ +Release \{' crates/nebulous_cli/src/args.rs && git diff --quiet -- crates/nebulous_cli/src/args.rs crates/nebulous_cli/src/main.rs crates/nebulous_cli/src/lib.rs crates/nebulous_cli/Cargo.toml crates/backend_core

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### User Step: Seed Trackers

**[USER]**. Run after STEP 24E is COMPLETED, with no TUI or `neb mcp` open on either repo, after answering Open Question 7. Review each dry run before running `--write`.

```bash
cd /srv/devshare/Nebulous && neb plan import STEPPED_IMPLEMENTATION_PLAN.md
```

```bash
cd /srv/devshare/Nebulous && neb plan import STEPPED_IMPLEMENTATION_PLAN.md --write
```

```bash
cd /srv/devshare/NovOS && neb plan import STEPPED_IMPLEMENTATION_PLAN.md
```

```bash
cd /srv/devshare/NovOS && neb plan import STEPPED_IMPLEMENTATION_PLAN.md --write
```

---

# Phase 10 — Hash Item IDs (opt-in, 2026-11-09)

Why: `.tracker/` is shared through git (NovOS tracks it). Two clones that create items while apart both allocate the next sequential number, for example `BUG-0013`, and collide on merge. STEP 13A–C only serialize writers inside one checkout. Hash IDs need no coordination and work offline, and an ID never changes once given.

Decisions (user, 2026-11-09):
- Format `BUG-7F3A2C`: the type prefix plus 6 uppercase hex digits (24 random bits from `rand::rngs::OsRng`, already a dependency). Uppercase because 9 call sites uppercase item IDs.
- Opt-in per project: `[project] id_scheme = "hash"` in project.toml. A missing key means `"sequential"`, so existing trackers and the ~340 hard-coded `BUG-0001`-style test IDs stay valid. The user adds the line to NovOS's project.toml.
- Existing IDs never change. Sequential and hash IDs coexist in one tracker.
- A hash suffix always contains at least one letter A–F (all-digit draws are redrawn), so a hash ID can never be read as a sequential one.
- Item lists sort by `created_at`, then by sequential number (both sequential, same type), then by ID string.
- A same-checkout duplicate is detected and redrawn. A cross-clone duplicate can't be detected when the item is created, so reconcile reports duplicate IDs after a merge (STEP 26E).

---

### STEP 26A: `IdScheme` Setting + Hash ID Format Helpers

* **Recommended Command:** `/goal`

Scope note (refreshed 2026-11-12). This block was first written 2026-11-09, before Phases 9 and 11. I re-checked it against HEAD `55f72c7`. The code it touches has been back to its pre-Phase 9 shape since 29C, so its design is unchanged. Only the line references, the gates and the open decisions are new.
- **`ProjectMetadata`:** five fields, no `Default` derive. The only struct literal of it in the workspace is in `storage/mod.rs` `init`, so adding a field breaks nothing else.
- **`ItemType`:** has only `Bug` and `Feature` (`Task` comes in 23F).
- **Inline tests:** neither `project.rs` nor `item.rs` has a `#[cfg(test)]` module yet, so this step adds one to each.
- **Layout guard:** the STEP 15B layout guard is `versioned_reads_and_writes_preserve_unknown_keys_and_layout` (`tests/storage_integration.rs:2250`). It has to stay green, which proves that a Sequential project never writes the new key.

```text
[STEP_ID]
STEP 26A: [project] id_scheme (sequential | hash, default sequential) and ItemType hash-ID helpers

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/project.rs
  - crates/backend_core/src/models/item.rs
  - crates/backend_core/src/storage/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/project.rs:39-57 (ProjectMetadata + its impl)
  - crates/backend_core/src/models/item.rs:55-86 (ItemType: min_schema_version, id_prefix, format_id)
  - crates/backend_core/src/storage/mod.rs:333-340 (`init`, the only `ProjectMetadata { .. }` literal)
  - crates/backend_core/src/storage/id.rs:8-22 (parse_item_id_number: digits only, so it already returns None for a suffix containing A-F)
  - crates/backend_core/src/models/mod.rs:21-24 (explicit project.rs re-export list; not edited)
  - crates/backend_core/tests/storage_integration.rs:2250 (`versioned_reads_and_writes_preserve_unknown_keys_and_layout`, must stay green)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No ID allocation change (STEP 26B). `format_id` and `parse_item_id_number` keep their behaviour. No re-export of IdScheme from models/mod.rs. No other edits to storage/mod.rs beyond the one field in `init`. No commits (Claude commits).

[PRE_GATE]
! grep -rq 'IdScheme\|id_scheme\|format_hash_id\|is_hash_item_id\|HASH_ID_HEX_LEN' crates && test "$(grep -rn 'ProjectMetadata {' crates | grep -v 'pub struct\|impl ProjectMetadata' | wc -l)" -eq 1 && grep -q 'project: ProjectMetadata {' crates/backend_core/src/storage/mod.rs && ! grep -q 'cfg(test)' crates/backend_core/src/models/project.rs && ! grep -q 'cfg(test)' crates/backend_core/src/models/item.rs && grep -q 'fn versioned_reads_and_writes_preserve_unknown_keys_and_layout' crates/backend_core/tests/storage_integration.rs && grep -q 'Self::Bug | Self::Feature => 1,' crates/backend_core/src/models/item.rs

[MUTATION_SPEC]
1. project.rs:
   - Add, right after `impl ProjectMetadata { .. }`: `#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "snake_case")] pub enum IdScheme { #[default] Sequential, Hash }`, with `impl IdScheme { pub fn is_sequential(&self) -> bool }` (true for Sequential). The doc comment says what each scheme allocates (`BUG-0001`-style counters vs. `BUG-7F3A2C`-style random IDs) and that a missing key means Sequential.
   - ProjectMetadata gains, as its last field (after `created_at`): `#[serde(default, skip_serializing_if = "IdScheme::is_sequential")] pub id_scheme: IdScheme`. Sequential is never written, so existing project.toml files keep their layout.
   - Don't re-export it. models/mod.rs re-exports project.rs items through an explicit list, and that file isn't in ALLOWED_FILES. Callers name it `crate::models::project::IdScheme` (`models::project` is a `pub mod`).
2. item.rs:
   - `pub const HASH_ID_HEX_LEN: usize = 6;` above `ItemType`.
   - In `impl ItemType`, after `format_id`: `pub fn format_hash_id(self, bits: u32) -> String` returning `format!("{}-{:06X}", self.id_prefix(), bits & 0x00FF_FFFF)`.
   - Free function `pub fn is_hash_item_id(id: &str) -> bool`. It's true when all of these hold:
     - the part before the first `-` is `BUG` or `FEAT`;
     - the part after it is exactly `HASH_ID_HEX_LEN` chars, all in `0-9A-F`;
     - at least one of those chars is in `A-F`.
     It's false for everything else, including lowercase input (callers uppercase first). It doesn't trim.
3. storage/mod.rs `init`: the `ProjectMetadata` literal gets `id_scheme: crate::models::project::IdScheme::Sequential,` as its last field. Nothing else in the file changes.
4. Inline tests. A new `#[cfg(test)] mod tests` at the end of project.rs and at the end of item.rs:
   - project.rs:
     - (a) a minimal `[project]` table (`name`, `active_phase`, `created_at`) without `id_scheme` parses through `ProjectManifest::from_toml_str` to `IdScheme::Sequential`, and the same table with `id_scheme = "hash"` parses to `IdScheme::Hash`;
     - (b) `to_toml_string` of that manifest emits no `id_scheme` line with Sequential, and contains `id_scheme = "hash"` with Hash;
     - (c) `id_scheme = "bogus"` fails to parse.
   - item.rs:
     - (d) `ItemType::Bug.format_hash_id(0xABC)` == `"BUG-000ABC"`;
     - (e) `ItemType::Feature.format_hash_id(0xFFFF_FFFF)` == `"FEAT-FFFFFF"` (masked);
     - (f) `is_hash_item_id` is true for `BUG-7F3A2C` and `FEAT-00000A`, and false for `BUG-001234` (all digits), `BUG-0012`, `BUG-7f3a2c`, `BUG-7F3A2`, `BUG-7F3A2C1`, `TASK-7F3A2C`, `BUG7F3A2C` and `""`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && scripts/check-neutral.sh >/dev/null && grep -q 'pub enum IdScheme' crates/backend_core/src/models/project.rs && grep -q 'skip_serializing_if = "IdScheme::is_sequential"' crates/backend_core/src/models/project.rs && grep -q 'pub fn format_hash_id' crates/backend_core/src/models/item.rs && grep -q 'pub fn is_hash_item_id' crates/backend_core/src/models/item.rs && grep -q 'pub const HASH_ID_HEX_LEN: usize = 6;' crates/backend_core/src/models/item.rs && test "$(git diff --numstat -- crates/backend_core/src/storage/mod.rs | awk '{print $1" "$2}')" = "1 0" && cargo test -p backend_core --test storage_integration versioned_reads_and_writes_preserve_unknown_keys_and_layout 2>&1 | grep -q 'test result: ok. 1 passed' && git diff --quiet -- crates/backend_core/src/models/mod.rs crates/backend_core/src/storage/id.rs crates/nebulous_cli Cargo.toml Cargo.lock

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

Decisions the contract leaves open:
- **`is_hash_item_id` knows only `BUG` and `FEAT`.** When 23F adds `ItemType::Task`, it should also extend this check to `TASK`. I'll put that in the 23F block when it's written. 26B–26E don't depend on it.
- **`IdRank` in `scope.rs` (STEP 26D) also needs `Task`.** Its exhaustive `type_order` match stops compiling when 23F adds the variant. The 23F block must give `Task` rank `2`.
- **An all-digit draw makes a valid-looking `format_hash_id` result that `is_hash_item_id` rejects.** That's on purpose: 26B redraws until `is_hash_item_id` is true, so a hash ID can never be read as a sequential one.
- **No CLI or MCP surface for the setting.** You turn it on by adding `id_scheme = "hash"` under `[project]` in project.toml by hand, as decided 2026-11-09.

---

### STEP 26B: Allocate Hash IDs When `id_scheme = "hash"`

* **Recommended Command:** `/boost`

Scope note (refreshed 2026-11-12). This block was first written 2026-11-09. I re-checked it against HEAD `8c8d113` (26A). Fixes to the old version:
- **Wrong field path:** the field is `project.project.id_scheme`. The old text said `project.id_scheme`.
- **No regex in the tests:** `regex` left `backend_core` in 29C, so the ID-format check is written out by hand.
- **Exact paths:** the block now names the import paths and the helpers the tests use.

Facts the step relies on, checked against the code:
- `create_item` (`workflow/lifecycle.rs:114`) calls `allocate_next_id` inside its own `with_write_lock`. The lock is reentrant (13A), so the hash path runs under the same lock as the check for an existing ID.
- `locate_item` (`storage/mod.rs:944`) checks the index first, then each indexed phase/scope's item dir. It returns `TrackerError::NotFound { kind: "Item", .. }` when the ID is free.
- `parse_item_id_number` (`id.rs:10`) takes digits only, so `scan_max_counters` already ignores hash IDs. Mixing the two schemes leaves the counters alone.
- `rand = "0.8"` is already a dependency. `crypto/keys.rs:4` shows the import style.

```text
[STEP_ID]
STEP 26B: allocate_next_id draws a unique 24-bit hash ID under the write lock when the project uses id_scheme = "hash"

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/id.rs
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/id.rs:66-99 (allocate_next_id, already inside with_write_lock)
  - crates/backend_core/src/storage/mod.rs:944-975 (locate_item; NotFound when free)
  - crates/backend_core/src/workflow/lifecycle.rs:114 (create_item -> allocate_next_id) and :165 (`create_bug(title, phase_id, scope_id, severity, description)`)
  - crates/backend_core/src/models/item.rs `ItemType::format_hash_id`, `is_hash_item_id` (STEP 26A); crates/backend_core/src/models/project.rs `IdScheme` (STEP 26A)
  - crates/backend_core/src/crypto/keys.rs:4 (`use rand::{rngs::OsRng, RngCore};`)
  - crates/backend_core/tests/storage_integration.rs:14-35 (`TempTrackerDir` helper) and :347-380 (existing sequential allocation tests, must stay green unchanged)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No new dependency (Cargo.toml and Cargo.lock untouched). The sequential path of allocate_next_id is unchanged byte for byte: same reads, same counter arithmetic, same write. Existing tests aren't edited; new ones are appended at the end of storage_integration.rs. No commits (Claude commits).

[PRE_GATE]
grep -q 'pub fn format_hash_id' crates/backend_core/src/models/item.rs && grep -q 'pub fn is_hash_item_id' crates/backend_core/src/models/item.rs && grep -q 'pub enum IdScheme' crates/backend_core/src/models/project.rs && ! grep -q 'fn allocate_hash_id\|MAX_HASH_ID_ATTEMPTS\|cfg(test)' crates/backend_core/src/storage/id.rs && grep -q '^rand = "0.8"' crates/backend_core/Cargo.toml && sed -n 114p crates/backend_core/src/workflow/lifecycle.rs | grep -q 'self.allocate_next_id(input.item_type)' && grep -q 'pub fn locate_item' crates/backend_core/src/storage/mod.rs && test "$(wc -l < crates/backend_core/tests/storage_integration.rs)" -eq 2530

[MUTATION_SPEC]
1. id.rs imports: add `use rand::{rngs::OsRng, RngCore};` and `use crate::models::item::is_hash_item_id;` and `use crate::models::project::IdScheme;`.
2. id.rs: a module-level `pub(crate) const MAX_HASH_ID_ATTEMPTS: u32 = 16;` (above `impl TrackerStorage`), and in `impl TrackerStorage`:
   - `pub(crate) fn allocate_hash_id(&self, item_type: ItemType, mut draw: impl FnMut() -> u32) -> Result<String>`. It tries up to MAX_HASH_ID_ATTEMPTS times. Each try:
     - `let id = item_type.format_hash_id(draw());`
     - if `!is_hash_item_id(&id)` (an all-digit draw), try again;
     - `match self.locate_item(&id)`: `Ok(_)` means taken, so try again; `Err(TrackerError::NotFound { .. })` means free, so return `Ok(id)`; any other `Err(e)` returns `Err(e)`.
     After the last try, return `Err(TrackerError::ValidationError(format!("could not allocate a unique hash ID after {MAX_HASH_ID_ATTEMPTS} attempts")))`.
   - allocate_next_id: the first statement inside the existing `with_write_lock` closure is still `let mut project = self.read_project()?;`. Right after it, add `if project.project.id_scheme == IdScheme::Hash { return self.allocate_hash_id(item_type, || OsRng.next_u32()); }`. That path never touches `[counters]` and never writes project.toml. Everything after it is the existing sequential code, unchanged.
   - Rewrite the allocate_next_id doc comment to describe both schemes: sequential reconciles and persists `[counters]` as today; hash draws random 24-bit IDs, redrawing all-digit or already-used ones, and leaves `[counters]` alone.
3. Inline tests in id.rs: `#[cfg(test)] mod hash_id_tests` with `use super::*;`. Each test uses `tempfile::tempdir()` and `TrackerStorage::init_standard(dir.path(), "hash-test")`, then sets hash mode with `let mut p = storage.read_project().unwrap(); p.project.id_scheme = IdScheme::Hash; storage.write_project(&p).unwrap();`.
   - (a) Create one bug with `storage.create_bug("taken", None, None, crate::models::Severity::Medium, None)` and call its ID `taken` (`.0.stub.id`; `ItemStub.id` is a `String`). Its bits are `u32::from_str_radix(&taken[4..], 16).unwrap()`. Call `allocate_hash_id(ItemType::Bug, draw)` with a scripted draw that returns, in order, `0x123456` (all digits, rejected), `taken`'s bits (taken, rejected), then `0x00ABCD`. It returns `"BUG-00ABCD"`, and the draw was called exactly 3 times (a counter captured by the closure).
   - (b) A draw that always returns `0x111111` makes allocate_hash_id fail with `TrackerError::ValidationError` after exactly 16 calls.
4. storage_integration.rs: append two tests at the end, using `TempTrackerDir` and `TrackerStorage::init_standard`. Name types by full path: `backend_core::models::project::IdScheme` and `backend_core::models::item::is_hash_item_id` (add `use` lines inside the test functions).
   - (c) `hash_scheme_allocates_unique_hash_ids_and_leaves_counters`:
     - read `[counters]` (a clone of `read_project().unwrap().counters`), switch to Hash, and create 200 bugs with `create_bug`;
     - every ID has length 10, starts with `"BUG-"`, has 6 chars after it in `0-9A-F`, and `is_hash_item_id` is true;
     - all 200 are distinct (a `HashSet` of size 200);
     - `read_project().unwrap().counters` equals the clone from before.
   - (d) `sequential_and_hash_ids_coexist`:
     - create a bug in sequential mode and assert it's `"BUG-0001"`;
     - switch to Hash and create one more, which passes `is_hash_item_id`;
     - `locate_item` finds both;
     - switch back to `IdScheme::Sequential`, create a bug, and assert it's `"BUG-0002"`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && scripts/check-neutral.sh >/dev/null && grep -q 'fn allocate_hash_id' crates/backend_core/src/storage/id.rs && grep -q 'MAX_HASH_ID_ATTEMPTS: u32 = 16' crates/backend_core/src/storage/id.rs && grep -q 'project.project.id_scheme == IdScheme::Hash' crates/backend_core/src/storage/id.rs && cargo test -p backend_core --lib hash_id_tests 2>&1 | grep -q 'test result: ok. 2 passed' && cargo test -p backend_core --test storage_integration hash_scheme_allocates_unique_hash_ids_and_leaves_counters 2>&1 | grep -q 'test result: ok. 1 passed' && cargo test -p backend_core --test storage_integration sequential_and_hash_ids_coexist 2>&1 | grep -q 'test result: ok. 1 passed' && test "$(git diff --numstat -- crates/backend_core/tests/storage_integration.rs | awk '{print $2}')" -eq 0 && git diff --quiet -- crates/backend_core/src/storage/mod.rs crates/backend_core/src/workflow crates/backend_core/src/models crates/nebulous_cli Cargo.toml Cargo.lock

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

Decisions the contract leaves open:
- **The collision check uses `locate_item`, not a directory walk.** It sees the index plus each indexed scope's item dir, which is the same reach the existing code has. It can't see an ID created in another clone that hasn't been merged in yet. 26E's duplicate report covers that case after a merge.
- **16 attempts.** With 24 bits there are about 16.7M IDs, and about 59% of draws are usable (6 hex digits with at least one A–F). So failing 16 times in a row needs a tracker that's mostly full. The error says so instead of looping forever.
- **`draw` is a parameter** so tests can script it. Production passes `|| OsRng.next_u32()`.

---

### STEP 26C: `created_at` on `ItemStub` + Chronological Comparator

* **Recommended Command:** `/goal`

Scope note (refreshed 2026-11-12). This block was first written 2026-11-09. I re-checked it against HEAD `ce5ef72` (26B). Changes from the old version:
- **Comparator:** it now sorts by the parsed instant, not the raw string. `created_at` isn't always the same format:
  - `now_rfc3339()` writes `%Y-%m-%dT%H:%M:%SZ`;
  - a hand-edited item can carry an offset (`+02:00`) or a TOML datetime, which `deserialize_datetime_string` passes through as written.
  Comparing raw strings would put those in the wrong order.
- **Total order:** the comparator ranks values into empty, parseable and unparseable. Since Rust 1.81, `sort_by` may panic on a comparator that isn't a total order (we're on 1.99), so it has to be one.
- **The `allocate_hash_id` doc comment** (the nit from 26B) is left out: `id.rs` isn't in this step's files.

Checked:
- The only `ItemStub { .. }` literals are `to_stub` (`item.rs:34`) and `storage_integration.rs:249` and `:455`.
- **Dry-run:** I added the field, filled it in `to_stub` and patched the two literals in a scratch worktree, and `cargo test --workspace` passed in full. Adding the field breaks nothing outside the allowed files, CLI JSON included.
- `reconcile.rs:316` `compare_item_stubs` and `lifecycle.rs:1003` `compare_stubs` are today's comparators (sequence number for the same type, then ID). The new method has to give the same order for stubs whose `created_at` is equal. Switching callers over is 26D.

```text
[STEP_ID]
STEP 26C: ItemStub carries created_at; ItemStub::chronological_cmp orders by created_at instant, then same-type sequence number, then ID

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/scope.rs
  - crates/backend_core/src/models/item.rs
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/scope.rs:55-64 (ItemStub)
  - crates/backend_core/src/models/item.rs:32-40 (to_stub) and :249 (ItemMetadata.created_at: String)
  - crates/backend_core/src/models/mod.rs:81 (`pub fn parse_timestamp_utc(value: &str) -> Result<DateTime<Utc>>`)
  - crates/backend_core/src/storage/mod.rs:16 (`pub use id::parse_item_id_number;`)
  - crates/backend_core/src/storage/reconcile.rs:316-323 and crates/backend_core/src/workflow/lifecycle.rs:1003-1010 (today's comparators; not edited)
  - crates/backend_core/tests/storage_integration.rs:249 and :455 (the two ItemStub literals)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Callers keep their own comparators (STEP 26D). No other changes to existing tests besides adding the field to the two literals; new tests are appended at the end of storage_integration.rs. No commits (Claude commits).

[PRE_GATE]
! grep -q 'fn chronological_cmp\|pub created_at' crates/backend_core/src/models/scope.rs && test "$(grep -rn 'ItemStub {' crates | grep -v 'pub struct\|impl ItemStub\|fn to_stub' | wc -l)" -eq 3 && grep -q 'fn compare_item_stubs' crates/backend_core/src/storage/reconcile.rs && grep -q 'fn compare_stubs' crates/backend_core/src/workflow/lifecycle.rs && grep -q 'pub fn parse_timestamp_utc' crates/backend_core/src/models/mod.rs && ! grep -q 'cfg(test)' crates/backend_core/src/models/scope.rs

[MUTATION_SPEC]
1. scope.rs:
   - ItemStub gains, as its last field (after `severity`): `#[serde(default, skip_serializing_if = "String::is_empty")] pub created_at: String`. Stubs in existing scope.toml files read as empty until reconcile rebuilds them, and an empty value is never written.
   - `impl ItemStub { pub fn chronological_cmp(&self, other: &Self) -> std::cmp::Ordering }`, with a doc comment that states the order. It compares in this order:
     1. `created_at`, through a private key fn mapping each value to one of three ranks:
        - rank 0: empty (after trim). Pre-26C stubs sort first.
        - rank 1: parses with `crate::models::parse_timestamp_utc`, ordered by the parsed `DateTime<Utc>`.
        - rank 2: non-empty but unparseable, ordered by the trimmed string.
        Compare the ranks first, then compare within the rank. That makes it a total order.
     2. On a tie: if both IDs parse with `crate::storage::parse_item_id_number` to the same `ItemType`, compare the numbers.
     3. Then `id` strings.
     That's exactly today's `compare_item_stubs` when `created_at` ties.
2. item.rs `to_stub`: add `created_at: self.item.created_at.clone(),` as the last field. Nothing else in item.rs changes.
3. storage_integration.rs:
   - add `created_at: String::new(),` as the last field of the two ItemStub literals (:249, :455).
   - Append `fn item_stub_chronological_cmp_orders_by_instant_then_sequence_then_id` at the end of the file. It builds stubs with a small local helper `stub(id, created_at)` (title "t", `ItemStatus::Open`, `Severity::Medium`) and sorts with `v.sort_by(|a, b| a.chronological_cmp(b))`. Assert each resulting id order:
     - (a) `BUG-00ABCD` @ `2026-01-02T00:00:00Z`, `BUG-FFFFFA` @ `2026-01-01T00:00:00Z`, `BUG-0002` @ `""` → `BUG-0002`, `BUG-FFFFFA`, `BUG-00ABCD`.
     - (b) the same `created_at` (`2026-01-01T00:00:00Z`) on `BUG-0010` and `BUG-0002` → `BUG-0002`, `BUG-0010` (numeric, not string order).
     - (c) offsets compare as instants: `BUG-0001` @ `2026-01-01T10:00:00+02:00` (08:00Z) and `BUG-0002` @ `2026-01-01T09:00:00Z` → `BUG-0001`, `BUG-0002`.
     - (d) unparseable after parseable: `BUG-0001` @ `not-a-date` and `BUG-0002` @ `2026-01-01T00:00:00Z` → `BUG-0002`, `BUG-0001`.
     - (e) a mixed type tie: `FEAT-0001` and `BUG-0002` at the same `created_at` → `BUG-0002`, `FEAT-0001` (string order).
4. Round-trip check, appended in the same test: an `ItemStub` with an empty `created_at`, put in a `ScopeItems` and serialized with `toml::to_string`, doesn't contain `created_at`. With a non-empty value it does. (Use `toml`, which is already a backend_core dependency.)

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && scripts/check-neutral.sh >/dev/null && grep -q 'pub fn chronological_cmp' crates/backend_core/src/models/scope.rs && grep -q 'skip_serializing_if = "String::is_empty")\]' crates/backend_core/src/models/scope.rs && grep -q 'created_at: self.item.created_at.clone(),' crates/backend_core/src/models/item.rs && test "$(git diff --numstat -- crates/backend_core/src/models/item.rs | awk '{print $1" "$2}')" = "1 0" && test "$(git diff --numstat -- crates/backend_core/tests/storage_integration.rs | awk '{print $2}')" -eq 0 && cargo test -p backend_core --test storage_integration item_stub_chronological_cmp_orders_by_instant_then_sequence_then_id 2>&1 | grep -q 'test result: ok. 1 passed' && git diff --quiet -- crates/backend_core/src/storage crates/backend_core/src/workflow crates/backend_core/src/mcp crates/nebulous_cli Cargo.toml Cargo.lock

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

Decisions the contract leaves open:
- **Empty sorts first.** Old stubs have no `created_at` until reconcile rebuilds them from item.toml (26D does that on every reconcile). Until then, they stay ahead of anything new.
- **No cached parse.** The comparator parses on every call. Scope lists are small (tens to hundreds of items), so caching isn't worth the extra code.
- **`ScopeItems` is the serialization wrapper for the round-trip check,** because a bare struct serializes the same way. If `toml::to_string` on a bare `ItemStub` is simpler, either is fine.

---

### STEP 26D: Total-Order ID Tie-Break + Index Ordering Uses `chronological_cmp`

* **Recommended Command:** `/goal`

Scope note (refreshed 2026-11-13, against HEAD `5f84a65`, 26C). Changes from the 2026-11-09 block:
- **Adds the 26C comparator fix (user, 2026-11-13).** The ID tie-break in `chronological_cmp` (and today's `compare_item_stubs` / `compare_stubs`, which it copies) isn't a total order once hash IDs mix with 5+ digit sequential IDs. With equal `created_at`: `BUG-9999 < BUG-10000` (numeric), `BUG-10000 < BUG-5ABCDE` (string), `BUG-5ABCDE < BUG-9999` (string). That's a cycle, so `sort_by` may panic. I reproduced it on 26C. This step replaces the tie-break with one ranked key, before any caller sorts with it.
- **4 files, on purpose.** The fix (`scope.rs`) and the caller switch can't be split. Switching first would put real sorts on a comparator that can panic; fixing first would leave a step with no caller. The tests go in `storage_integration.rs` because none of the three source files has a test module.
- **No 1.1 s sleep.** The old test slept to get two `created_at` seconds. The new one writes chosen `created_at` values into item.toml with `write_item_manifest`, including a `+02:00` offset, so it's deterministic and instant.
- **Dry-run.** I made this exact change in a throwaway worktree. fmt, clippy `-D warnings` and `cargo test --workspace` all passed. No existing test changed or failed, and the worktree is removed.

Current code (HEAD `5f84a65`):
- `scope.rs:56-66` ItemStub (last field `created_at`), `:68-83` `CreatedAtRank` + `created_at_rank`, `:85-104` `impl ItemStub { chronological_cmp }`. Its `.then_with(|| { match (crate::storage::parse_item_id_number(..), ..) { .. } })` is the tie-break being replaced.
- `reconcile.rs:310-311` `bugs.sort_by(compare_item_stubs); features.sort_by(compare_item_stubs);`, `:316-323` `fn compare_item_stubs`. `use std::cmp::Ordering;` (line 1) is used only by it. `parse_item_id_number` (line 10) is still used at :251 and :279, so it stays.
- `lifecycle.rs:986` / `:990` `sort_by(compare_stubs)` inside `sync_parent_indexes_for_item`, `:1003-1010` `fn compare_stubs`. `use std::cmp::Ordering;` (line 1) and `use crate::storage::id::parse_item_id_number;` (line 14) are used only by it.
- `ItemType` (`models/item.rs:62`) doesn't derive `Ord`, and `item.rs` isn't in scope, so the rank maps it to a `u8`.

```text
[STEP_ID]
STEP 26D: chronological_cmp breaks ties with a total-order ID rank; reconcile and lifecycle sort scope item stubs with ItemStub::chronological_cmp

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/scope.rs
  - crates/backend_core/src/storage/reconcile.rs
  - crates/backend_core/src/workflow/lifecycle.rs
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/id.rs `parse_item_id_number` (returns Option<(ItemType, u32)>; None for hash IDs and anything not all digits after the dash)
  - crates/backend_core/src/models/item.rs `ItemType` (Bug, Feature; no Ord)
  - crates/backend_core/src/storage/mod.rs `read_item_manifest`, `write_item_manifest`, `read_scope`
  - crates/backend_core/tests/storage_integration.rs `sequential_and_hash_ids_coexist` (how a test switches a project to IdScheme::Hash)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. `CreatedAtRank` / `created_at_rank` stay as they are. assist.rs `compare_item_ids` stays as it is (it is only a last tie-break). No edits to existing tests; new tests are appended at the end of storage_integration.rs. No commits (Claude commits).

[PRE_GATE]
grep -q 'fn chronological_cmp' crates/backend_core/src/models/scope.rs && grep -q 'enum CreatedAtRank' crates/backend_core/src/models/scope.rs && ! grep -q 'IdRank' crates/backend_core/src/models/scope.rs && [ "$(grep -c 'sort_by(compare_item_stubs)' crates/backend_core/src/storage/reconcile.rs)" = 2 ] && [ "$(grep -c 'sort_by(compare_stubs)' crates/backend_core/src/workflow/lifecycle.rs)" = 2 ] && grep -q 'fn item_stub_chronological_cmp_orders_by_instant_then_sequence_then_id' crates/backend_core/tests/storage_integration.rs

[MUTATION_SPEC]
1. scope.rs: add a private ID rank right after `created_at_rank`, and use it as the tie-break:
   - `#[derive(PartialEq, Eq, PartialOrd, Ord)] enum IdRank<'a> { Sequential(u8, u32, &'a str), Other(&'a str) }`. The variant order matters: every sequential ID sorts before every non-sequential one.
   - `fn id_rank(id: &str) -> IdRank<'_>`: if `crate::storage::parse_item_id_number(id)` is `Some((item_type, number))`, return `Sequential(type_order, number, id)`, where `type_order` is an exhaustive `match` (`crate::models::ItemType::Bug => 0`, `Feature => 1`, no `_` arm, so a new ItemType variant fails to compile here). Otherwise return `Other(id)`.
   - In `chronological_cmp`, replace the whole `.then_with(|| { match (...) { ... } })` with `.then_with(|| id_rank(&self.id).cmp(&id_rank(&other.id)))`. Update the doc comment to state the new tie-break: sequential IDs first (by type, then number, then ID string), then all other IDs by string.
2. reconcile.rs: delete `fn compare_item_stubs`. The two calls become `bugs.sort_by(ItemStub::chronological_cmp);` and `features.sort_by(ItemStub::chronological_cmp);`. Delete `use std::cmp::Ordering;`. Keep the `parse_item_id_number` import (still used).
3. lifecycle.rs: delete `fn compare_stubs`. The two calls become `sort_by(ItemStub::chronological_cmp)`. Delete `use std::cmp::Ordering;` and `use crate::storage::id::parse_item_id_number;`.
4. storage_integration.rs: append two tests at the end of the file.
   - `fn chronological_cmp_is_total_with_mixed_hash_and_wide_sequential_ids`: a local helper `stub(id)` (title "t", `ItemStatus::Open`, `Severity::Medium`, `created_at` "2026-01-01T00:00:00Z"). With a = `BUG-9999`, b = `BUG-10000`, c = `BUG-5ABCDE`: assert `a.chronological_cmp(&b)`, `b.chronological_cmp(&c)` and `a.chronological_cmp(&c)` are all `std::cmp::Ordering::Less`. (On 26C the last one was `Greater`, which made the cycle.)
   - `fn reconcile_and_create_order_scope_stubs_by_created_at`:
     - `TempTrackerDir::new("chrono_order")`, then `TrackerStorage::init_standard(temp.path(), "ChronoProject")`, then switch the project to `backend_core::models::project::IdScheme::Hash` (read_project / set `project.project.id_scheme` / write_project).
     - Create 3 bugs with `create_bug(format!("bug {i}"), None, None, Severity::Medium, None)`. Pass `format!(..)` by value; clippy rejects `&format!`. After each one, read its manifest with `read_item_manifest(&loc.phase_id, &loc.scope_id, &loc.stub.id)`, set `manifest.item.created_at`, and write it back with `write_item_manifest`. The values, in creation order: `2026-03-01T00:00:00Z`, `2026-01-01T10:00:00+02:00`, `2026-02-01T00:00:00Z`.
     - Call `reconcile_indexes()`, then `read_scope` on that phase and scope. Assert that `items.bugs` IDs equal [bug 1, bug 2, bug 0], which is instant order.
     - Then create a 4th bug (no created_at edit: it gets the real current time, later than all three) and re-read the scope without reconciling. Assert that `items.bugs` IDs equal [bug 1, bug 2, bug 0, bug 3]. This proves the lifecycle sync path sorts the same way reconcile does.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'enum IdRank' crates/backend_core/src/models/scope.rs && ! grep -q 'crate::storage::parse_item_id_number(&self.id)' crates/backend_core/src/models/scope.rs && ! grep -q 'fn compare_item_stubs' crates/backend_core/src/storage/reconcile.rs && ! grep -q 'fn compare_stubs' crates/backend_core/src/workflow/lifecycle.rs && [ "$(grep -c 'sort_by(ItemStub::chronological_cmp)' crates/backend_core/src/storage/reconcile.rs)" = 2 ] && [ "$(grep -c 'sort_by(ItemStub::chronological_cmp)' crates/backend_core/src/workflow/lifecycle.rs)" = 2 ] && ! grep -q 'std::cmp::Ordering' crates/backend_core/src/storage/reconcile.rs crates/backend_core/src/workflow/lifecycle.rs && grep -q 'fn chronological_cmp_is_total_with_mixed_hash_and_wide_sequential_ids' crates/backend_core/tests/storage_integration.rs && grep -q 'fn reconcile_and_create_order_scope_stubs_by_created_at' crates/backend_core/tests/storage_integration.rs && [ "$(git diff HEAD --numstat -- crates/backend_core/tests/storage_integration.rs | awk '{print $2}')" = 0 ] && git diff --quiet HEAD -- crates/backend_core/src/storage/id.rs crates/backend_core/src/storage/mod.rs crates/backend_core/src/models/item.rs crates/backend_core/src/workflow/assist.rs crates/backend_core/src/mcp crates/nebulous_cli Cargo.toml Cargo.lock crates/backend_core/Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

Decisions the contract leaves open:
- **Sequential projects reorder only where `created_at` disagrees with the number.** Items created normally get increasing timestamps, so their order doesn't change. Items whose item.toml carries an older `created_at` (hand-edited, imported) move to their instant. That's the point of 26.
- **Pre-26C stubs (empty `created_at`) stay first until a reconcile.** When the lifecycle sync rewrites one item's stub, that one stub gains a timestamp and moves behind the empty ones. A `reconcile_indexes` run fills them all in from item.toml. That's a one-time transitional reshuffle, and it never panics.
- **`IdRank` doesn't cover `TASK`.** When 23F adds `ItemType::Task`, the exhaustive match fails to compile, which forces the 23F block to give it a rank (`2`). I'll note that in 23F alongside the `is_hash_item_id` extension.
- **The `allocate_hash_id` doc nit (26B) still waits.** `id.rs` isn't in scope here either.

---

### STEP 26E: Reconcile Reports Duplicate Item IDs

* **Recommended Command:** `/goal`

Scope note (refreshed 2026-11-13, against HEAD `fad0602`, 26D). Changes from the 2026-11-09 block:
- **The command is `neb sync-index`** (`args.rs:299` `#[command(name = "sync-index")]`), not "neb reconcile". I fixed the name here, in the Up Next row and in the 23-series docs step.
- **The CLI warning gets a test.** The old block had none for `commands/mod.rs`. That file has an inline `mod tests` (`:2828`) that already builds trackers with `tempfile` + `init_standard`, so a direct `handle_sync_index` test fits with no new files.
- **The backend test reuses `copy_dir_recursive`** (`storage_integration.rs:2048`) instead of a hand copy.
- **Dry-run.** I made this exact change in a throwaway worktree. fmt, clippy `-D warnings` and `cargo test --workspace` all passed, plus both new tests. `create_bug(.., None, ..)` lands in `pre_production`/`inbox`, which I confirmed. The worktree is removed.

Current code (HEAD `fad0602`):
- `reconcile.rs:1` `use std::collections::HashSet;`. `:13-21` `ReconcileReport` (derives `Default`, so there are no struct literals anywhere; last field `skipped_unsupported`). `:36` `let mut max_counters`. `:154-159` the one `collect_authoritative_items(self, &items_dir, &mut max_counters, &mut report.skipped_unsupported)?` call. `:204` `report.skipped_unsupported.sort();`. `:215-220` the `collect_authoritative_items` signature. `:285` `let is_canonical_dir = ...`, right after a manifest parses successfully.
- `commands/mod.rs:1730-1737` `SyncIndexOutputJson`, `:1739-1761` `handle_sync_index` (JSON branch, then `Ok(format!(..))`).
- Other `reconcile_indexes` callers (`git_sync/mod.rs:1352`, `:1642`, `watcher/mod.rs:130`, `commands/mod.rs:1239`) ignore or pass the report on. They keep compiling unchanged.

```text
[STEP_ID]
STEP 26E: ReconcileReport.duplicate_ids lists every item ID found in more than one item directory, across all phases and scopes; neb sync-index warns about them

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/reconcile.rs
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/reconcile.rs `collect_authoritative_items` (its `seen_ids` dedup is per scope only, and silently keeps the canonical directory)
  - crates/backend_core/src/storage/mod.rs `item_dir`, `items_dir`
  - crates/backend_core/tests/storage_integration.rs `copy_dir_recursive`, `TempTrackerDir`
  - crates/nebulous_cli/src/commands/mod.rs `mod tests` (tempfile + init_standard pattern)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Reconcile doesn't rename, move or delete anything; it only reports. Indexing output (scope.toml stubs, phase summaries, counters) is unchanged. Text and JSON output for a tracker with no duplicates is byte-for-byte unchanged, apart from the new JSON key. No edits to existing tests. No commits (Claude commits).

[PRE_GATE]
grep -q 'pub skipped_unsupported: Vec<String>' crates/backend_core/src/storage/reconcile.rs && ! grep -q 'duplicate_ids' crates/backend_core/src/storage/reconcile.rs crates/nebulous_cli/src/commands/mod.rs && [ "$(grep -c 'collect_authoritative_items(' crates/backend_core/src/storage/reconcile.rs)" = 2 ] && grep -q 'struct SyncIndexOutputJson' crates/nebulous_cli/src/commands/mod.rs && grep -q '^mod tests' crates/nebulous_cli/src/commands/mod.rs && grep -q '^fn copy_dir_recursive' crates/backend_core/tests/storage_integration.rs

[MUTATION_SPEC]
1. reconcile.rs:
   - The import becomes `use std::collections::{HashMap, HashSet};`.
   - `ReconcileReport` gains `pub duplicate_ids: Vec<String>` as its last field.
   - `reconcile_indexes`: next to `max_counters`, `let mut id_counts: HashMap<String, u32> = HashMap::new();`. Pass `&mut id_counts` as a new last argument to the `collect_authoritative_items` call. After `report.skipped_unsupported.sort();`, set `report.duplicate_ids` to every key with a count above 1, sorted.
   - `collect_authoritative_items` gains a last parameter `id_counts: &mut HashMap<String, u32>`. Right before `let is_canonical_dir = ...` (so only successfully parsed manifests count, and UnsupportedSchema skips don't), add `*id_counts.entry(manifest.item.id.clone()).or_insert(0) += 1;`. Nothing else in the function changes.
2. commands/mod.rs:
   - `SyncIndexOutputJson` gains `duplicate_ids: Vec<String>` as its last field, filled with `report.duplicate_ids` in the JSON branch.
   - Text branch: build the existing summary into a `let mut out = format!(..)` (same format string and arguments), then for each ID push `\nwarning: item ID <ID> exists in more than one item directory (likely a merge from two clones); rename one by hand`, and return `Ok(out)`.
   - Append to `mod tests`: `fn sync_index_warns_about_duplicate_item_ids`.
     - `tempfile::tempdir()`, then `TrackerStorage::init_standard(tmp.path(), "Dup")`, then `create_bug("dup bug", None, None, Severity::Medium, None)`.
     - `handle_sync_index(&storage, false)` doesn't contain `warning:`.
     - Make the duplicate: `create_dir_all` on `storage.items_dir("production", "inbox")?.join(&loc.stub.id)`, and `std::fs::copy` the original's `item_dir(..).join("item.toml")` into it.
     - The text output now contains `warning: item ID <id> exists in more than one item directory`.
     - The JSON output parsed with `serde_json::from_str::<serde_json::Value>` has `["duplicate_ids"] == serde_json::json!([id])`.
3. storage_integration.rs: append `fn reconcile_reports_duplicate_item_ids_across_phases` at the end of the file.
   - `TempTrackerDir::new("dup_ids")`, then `init_standard(temp.path(), "DupProject")`, then create a bug (it lands in `pre_production`/`inbox`).
   - `reconcile_indexes()` has an empty `duplicate_ids`.
   - `copy_dir_recursive(&storage.item_dir(&loc.phase_id, &loc.scope_id, &loc.stub.id)?, &storage.items_dir("production", "inbox")?.join(&loc.stub.id))`.
   - `reconcile_indexes()` returns Ok with `duplicate_ids == vec![loc.stub.id.clone()]`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub duplicate_ids: Vec<String>' crates/backend_core/src/storage/reconcile.rs && grep -q 'id_counts: &mut HashMap<String, u32>' crates/backend_core/src/storage/reconcile.rs && grep -q 'duplicate_ids: report.duplicate_ids' crates/nebulous_cli/src/commands/mod.rs && grep -q 'exists in more than one item directory (likely a merge from two clones); rename one by hand' crates/nebulous_cli/src/commands/mod.rs && grep -q 'fn sync_index_warns_about_duplicate_item_ids' crates/nebulous_cli/src/commands/mod.rs && grep -q 'fn reconcile_reports_duplicate_item_ids_across_phases' crates/backend_core/tests/storage_integration.rs && [ "$(git diff HEAD --numstat -- crates/backend_core/tests/storage_integration.rs | awk '{print $2}')" = 0 ] && git diff --quiet HEAD -- crates/backend_core/src/storage/mod.rs crates/backend_core/src/storage/id.rs crates/backend_core/src/models crates/backend_core/src/workflow crates/backend_core/src/git_sync crates/backend_core/src/watcher crates/backend_core/src/mcp crates/nebulous_cli/src/args.rs crates/nebulous_cli/tests Cargo.toml Cargo.lock crates/backend_core/Cargo.toml crates/nebulous_cli/Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

Decisions the contract leaves open:
- **The ID match is exact.** `BUG-7f3a2c` and `BUG-7F3A2C` count as different IDs. Every writer produces uppercase (`format_hash_id` uses `{:06X}`), so a case-only clash would mean a hand edit.
- **A same-scope duplicate counts too.** Two directories in one scope with the same `item.id` (a canonical dir plus a stray copy) get reported. Indexing still keeps only the canonical one, as today.
- **Only `neb sync-index` surfaces the warning.** `neb sync` pulls and then runs `reconcile_indexes` (`git_sync/mod.rs:1642`, `let _ =`), and that's the most likely moment a cross-clone duplicate shows up. Today it drops the report, and so do the watcher and the MCP paths. Surfacing it there is a separate step with its own files; this one stays at 3.

---


### STEP 26F: Pull and Sync Surface Duplicate Item IDs

* **Recommended Command:** `/goal`

Scope note (written 2026-11-13, against HEAD `81954bd`, 26E). This is a new step, added after 26E: `neb sync` and `neb git pull` reconcile after every pull that brings in commits, which is exactly when a duplicate from another clone shows up. Both throw the report away today.
- **Additive API.** `git_pull` and `git_sync` keep their signatures and become one-line wrappers over new `*_with_report` methods. Their other callers stay unchanged: the MCP tools (`mcp/mod.rs:2117`, `:2201`), `git_safety.rs`, `workflow_integration.rs` and `credential_hygiene.rs`.
- **One warning text.** 26E's text moves into a `duplicate_id_warnings` helper, and sync-index, git pull and sync all use it. sync-index output is byte-for-byte unchanged.
- **Test without a merge race.** Workspace 1 commits the same item ID in two phases and pushes, and workspace 2 pulls it. That tests the plumbing deterministically. Making two clones race to the same ID would hinge on git2 merge details that aren't this step's subject.
- **Dry-run.** I made this exact change in a throwaway worktree. fmt, clippy `-D warnings` and `cargo test --workspace` all passed, plus both new tests. The worktree is removed.

Current code (HEAD `81954bd`):
- `git_sync/mod.rs:157` is the doc line of `DocumentedCommitOutcome`; the new structs go right above it. `lib.rs` has `pub use git_sync::*;`, so they're exported with no lib.rs edit. `:1630-1645` is `git_pull` (`let _ = self.reconcile_indexes()?;` at `:1642`). `:1976-1987` is `git_sync` (`let _ = self.git_pull(auth)?;` at `:1983`).
- `commands/mod.rs:1631-1636` `SyncOutputJson`. `:1679-1728` `handle_sync` (`let _ = storage.git_sync(..)` at `:1703`, `storage.git_sync(..)?` at `:1709`). `:1762-1767` is 26E's warning loop in `handle_sync_index`. `:2696-2701` `GitPullOutputJson`. `:2756-2772` is the `GitCommand::Pull` arm. `:2836` is `mod tests`.
- `workflow_integration.rs:1-14` imports (`fs`, `Path`, `Repository`, `TempDir`, `GitAuthRequest`, `Severity`, `TrackerStorage`, all already there). `:514-620` `test_git_sync_status_commit_push_pull_and_keyring` is the reference two-workspace setup (bare remote, clone, fixing the branch head).

```text
[STEP_ID]
STEP 26F: git_pull_with_report / git_sync_with_report keep the post-pull reconcile's duplicate_ids; neb git pull and neb sync print the duplicate-ID warning and add duplicate_ids to their JSON

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/git_sync/mod.rs
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/reconcile.rs `ReconcileReport.duplicate_ids` (STEP 26E)
  - crates/backend_core/tests/workflow_integration.rs `test_git_sync_status_commit_push_pull_and_keyring` (two-workspace setup)
  - crates/backend_core/src/storage/mod.rs `item_dir`, `items_dir`
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. `git_pull` and `git_sync` keep their exact signatures and return values. MCP tools stay unchanged. `pull_repo_with_auth`, `git_push` and `git_commit_tracker` stay unchanged. No edits to existing tests. No commits (Claude commits).

[PRE_GATE]
grep -q 'let _ = self.reconcile_indexes()?;' crates/backend_core/src/git_sync/mod.rs && grep -q 'let _ = self.git_pull(auth)?;' crates/backend_core/src/git_sync/mod.rs && ! grep -q 'GitPullOutcome\|GitSyncOutcome\|_with_report' crates/backend_core/src/git_sync/mod.rs crates/nebulous_cli/src/commands/mod.rs && grep -q 'pub use git_sync::\*;' crates/backend_core/src/lib.rs && [ "$(grep -c 'storage.git_sync(' crates/nebulous_cli/src/commands/mod.rs)" = 2 ] && [ "$(grep -c 'exists in more than one item directory (likely a merge from two clones); rename one by hand' crates/nebulous_cli/src/commands/mod.rs)" = 1 ] && grep -q 'struct GitPullOutputJson' crates/nebulous_cli/src/commands/mod.rs && grep -q 'struct SyncOutputJson' crates/nebulous_cli/src/commands/mod.rs

[MUTATION_SPEC]
1. git_sync/mod.rs:
   - Right above the `DocumentedCommitOutcome` doc comment, add two documented structs, each `#[derive(Debug, Clone, Default, PartialEq, Eq)]`:
     - `pub struct GitPullOutcome { pub updated: bool, pub duplicate_ids: Vec<String> }`;
     - `pub struct GitSyncOutcome { pub sha: Option<String>, pub duplicate_ids: Vec<String> }`.
     Document every field. `duplicate_ids` is empty when nothing was pulled.
   - `pub fn git_pull_with_report(&self, auth: &GitAuthRequest) -> Result<GitPullOutcome>`: the current body of `git_pull`, except that when `updated` it stores `self.reconcile_indexes()?.duplicate_ids` instead of discarding the report. `git_pull` becomes `Ok(self.git_pull_with_report(auth)?.updated)`, with its doc comment kept.
   - `pub fn git_sync_with_report(&self, message: Option<&str>, auth: &GitAuthRequest) -> Result<GitSyncOutcome>`: the current body of `git_sync`, except that inside the remote-exists branch it takes `duplicate_ids` from `self.git_pull_with_report(auth)?` (otherwise empty), and it returns `GitSyncOutcome { sha: <the existing committed_sha.or_else(..) expression>, duplicate_ids }`. `git_sync` becomes `Ok(self.git_sync_with_report(message, auth)?.sha)`, with its doc comment kept.
   - Each new method gets a one-line doc: "Same as [`Self::git_pull`] / [`Self::git_sync`], but keeps the post-pull reconcile's duplicate item IDs."
2. commands/mod.rs:
   - Add a documented `fn duplicate_id_warnings(ids: &[String]) -> String` below `handle_sync_index`. It returns the concatenation of one `\nwarning: item ID {id} exists in more than one item directory (likely a merge from two clones); rename one by hand` per ID, or an empty String when there are none. `handle_sync_index`'s loop becomes `out.push_str(&duplicate_id_warnings(&report.duplicate_ids));`, so the warning text appears exactly once in the file.
   - `handle_sync`:
     - `let mut duplicate_ids = Vec::new();` before `let sha = ..`.
     - The `:1703` call becomes `duplicate_ids = storage.git_sync_with_report(message, auth)?.duplicate_ids;`.
     - The `:1709` branch calls `git_sync_with_report`, assigns its `duplicate_ids`, and evaluates to its `sha`.
     - `SyncOutputJson` gains `duplicate_ids: Vec<String>` as its last field.
     - The text result is the existing message (both match arms unchanged) followed by `duplicate_id_warnings(&duplicate_ids)`.
   - The `GitCommand::Pull` arm:
     - Call `storage.git_pull_with_report(..)`.
     - `GitPullOutputJson` gains `duplicate_ids: Vec<String>` as its last field; fill it, and `updated`, from the outcome.
     - The "Pulled remote changes ..." text gets `duplicate_id_warnings(&outcome.duplicate_ids)` appended. "Already up to date." is unchanged.
   - Append to `mod tests`: `fn duplicate_id_warnings_emits_one_line_per_id`.
     - `duplicate_id_warnings(&[])` is `""`.
     - For `["BUG-0001", "FEAT-00ABCD"]`, the output contains `\nwarning: item ID ` exactly twice, and contains `item ID <each> exists in more than one item directory`.
3. workflow_integration.rs: append `fn git_pull_and_sync_with_report_surface_duplicate_item_ids` at the end of the file.
   - **Setup:**
     - `TempDir`, then `TrackerStorage::init_standard(dir.path(), "DupPullProject")`, then `Repository::init`.
     - Set user.name `testuser` and user.email `testuser@example.com` on both repos.
     - Bare remote `origin`. Run `git_commit_tracker(Some("chore: init tracker"))` before reading `repo.head()`, which needs a commit.
     - Set `project.git.remote` / `.branch`, `write_project`, commit, `git_push`.
     - Clone the bare repo, and fix the clone's branch head the same way `test_git_sync_status_commit_push_pull_and_keyring` does.
     - `cloned = TrackerStorage::new(clone_dir.path())`.
   - **Idle pull:** `cloned.git_pull_with_report(..)` has `!updated` and an empty `duplicate_ids`.
   - **Duplicate pull:**
     - Workspace 1 runs `create_bug("dup bug", None, None, Severity::Medium, None)`.
     - Copy that item's directory files into `items_dir("production", "inbox")?.join(&id)` with a small local `copy_dir(src, dst)` helper (`create_dir_all` + `fs::copy` per entry; item dirs are flat).
     - `git_commit_tracker`, then `git_push`.
     - `cloned.git_pull_with_report(..)` has `updated` and `duplicate_ids == vec![id]`.
   - **Sync path:**
     - Workspace 1 creates a second bug, commits and pushes.
     - `cloned.git_sync_with_report(None, ..)` has `sha.is_some()` and `duplicate_ids == vec![id]`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && grep -q 'pub struct GitPullOutcome' crates/backend_core/src/git_sync/mod.rs && grep -q 'pub struct GitSyncOutcome' crates/backend_core/src/git_sync/mod.rs && grep -q 'pub fn git_pull(&self, auth: &GitAuthRequest) -> Result<bool>' crates/backend_core/src/git_sync/mod.rs && grep -q 'pub fn git_sync(&self, message: Option<&str>, auth: &GitAuthRequest) -> Result<Option<String>>' crates/backend_core/src/git_sync/mod.rs && grep -q 'Ok(self.git_pull_with_report(auth)?.updated)' crates/backend_core/src/git_sync/mod.rs && grep -q 'Ok(self.git_sync_with_report(message, auth)?.sha)' crates/backend_core/src/git_sync/mod.rs && ! grep -q 'let _ = self.reconcile_indexes()?;' crates/backend_core/src/git_sync/mod.rs && ! grep -q 'let _ = self.git_pull(auth)?;' crates/backend_core/src/git_sync/mod.rs && ! grep -q 'storage.git_sync(' crates/nebulous_cli/src/commands/mod.rs && grep -q 'git_pull_with_report' crates/nebulous_cli/src/commands/mod.rs && [ "$(grep -c 'exists in more than one item directory (likely a merge from two clones); rename one by hand' crates/nebulous_cli/src/commands/mod.rs)" = 1 ] && grep -q 'fn duplicate_id_warnings(ids: &\[String\]) -> String' crates/nebulous_cli/src/commands/mod.rs && grep -q 'fn duplicate_id_warnings_emits_one_line_per_id' crates/nebulous_cli/src/commands/mod.rs && grep -q 'fn git_pull_and_sync_with_report_surface_duplicate_item_ids' crates/backend_core/tests/workflow_integration.rs && [ "$(git diff HEAD --numstat -- crates/backend_core/tests/workflow_integration.rs | awk '{print $2}')" = 0 ] && git diff --quiet HEAD -- crates/backend_core/src/lib.rs crates/backend_core/src/mcp crates/backend_core/src/storage crates/backend_core/src/models crates/backend_core/src/workflow crates/backend_core/tests/git_safety.rs crates/backend_core/tests/storage_integration.rs crates/backend_core/tests/credential_hygiene.rs crates/nebulous_cli/src/args.rs crates/nebulous_cli/tests Cargo.toml Cargo.lock crates/backend_core/Cargo.toml crates/nebulous_cli/Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

Decisions the contract leaves open:
- **The MCP tools don't report duplicates yet.** `git_pull` and `git_sync` over MCP (`mcp/mod.rs:2117`, `:2201`) still return their old JSON. Adding `duplicate_ids` there is `mcp/mod.rs` plus its test file. It's small, but it's another file set, so it's not in this step. Your call whether it gets a row.
- **A pull that brings nothing doesn't reconcile, so it doesn't warn.** That matches today's behaviour (reconcile only runs on `updated`). `neb sync-index` always checks.
- **The warning goes into the returned String.** It's on stdout with the summary, the same as 26E. Moving warnings to stderr would be a CLI-wide change, not this one.

---


### STEP 26G: MCP `git_pull` / `git_sync` Report Duplicate Item IDs

* **Recommended Command:** `/goal`

Scope note (written 2026-11-13, against HEAD `bf5eff1`, 26F). This is the last piece of the duplicate-ID chain: an agent pulling or syncing over MCP sees the same `duplicate_ids` that `neb` users see since 26F.
- **2 files.** `mcp/mod.rs` and a test. The MCP git tools are tested today only for credential refusal (`credential_hygiene.rs:94-117`), which never reaches a real pull. `workflow_integration.rs` already has 26F's two-workspace setup, so the new test lives there, next to it.
- **The JSON only grows.** It gains one key; every existing key keeps its name and value. The tool descriptions gain one sentence each, so an agent knows the field exists. The input schemas don't change.
- **`commit_only` returns `duplicate_ids: []`.** No pull means no reconcile. The key is always present, so callers don't have to branch on its absence.
- **Dry-run.** I made this exact change in a throwaway worktree. fmt, clippy `-D warnings`, `cargo test --workspace`, the new test and `scripts/check-neutral.sh` all passed. The worktree is removed.

Current code (HEAD `bf5eff1`):
- `mcp/mod.rs:866-875` `git_pull` McpToolDefinition (its description ends "...would be overwritten."). `:902-913` `git_sync` McpToolDefinition (description "Stages and commits dirty .tracker/ state, then pulls and pushes if a remote is configured.").
- `:2104-2123` `tool_git_pull` (`let updated = storage.git_pull(&auth)?;` at `:2117`, then `json!({"updated", "status"})`).
- `:2181-2213` `tool_git_sync` (`let sha = if parsed.commit_only { git_commit_tracker } else { git_sync }` at `:2197-2202`; `json!` ending `"tracker_dirty_after": tracker_dirty_after` at `:2211`).
- `git_pull_with_report` / `git_sync_with_report` (26F, `git_sync/mod.rs`) return `GitPullOutcome { updated, duplicate_ids }` / `GitSyncOutcome { sha, duplicate_ids }`.
- `workflow_integration.rs:2042-2143` `git_pull_and_sync_with_report_surface_duplicate_item_ids` (26F) is the setup to mirror. `backend_core::McpServer` (`McpServer::new(Some(TrackerStorage::new(..)))`, `.call_tool(name, json!(..)) -> Result<Value>`) is exported through `pub use mcp::*;`. `serde_json` is a normal backend_core dependency.

```text
[STEP_ID]
STEP 26G: MCP git_pull and git_sync results include duplicate_ids from the post-pull reconcile; existing result keys unchanged

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/git_sync/mod.rs `git_pull_with_report`, `git_sync_with_report`, `GitPullOutcome`, `GitSyncOutcome` (STEP 26F)
  - crates/backend_core/tests/workflow_integration.rs `git_pull_and_sync_with_report_surface_duplicate_item_ids` (setup to mirror)
  - crates/backend_core/tests/credential_hygiene.rs `McpServer::new` / `call_tool` usage
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No tool renamed, added or removed. Input schemas stay unchanged. Existing result keys keep their names and values. The `mcp_auth_request` refusal paths stay unchanged. No edits to existing tests. No commits (Claude commits).

[PRE_GATE]
[ "$(grep -c 'let updated = storage.git_pull(&auth)?;' crates/backend_core/src/mcp/mod.rs)" = 1 ] && [ "$(grep -c 'storage.git_sync(non_empty(parsed.message.as_deref()), &auth)?' crates/backend_core/src/mcp/mod.rs)" = 1 ] && ! grep -q 'duplicate_ids\|_with_report' crates/backend_core/src/mcp/mod.rs && grep -q 'pub fn git_pull_with_report' crates/backend_core/src/git_sync/mod.rs && grep -q 'pub fn git_sync_with_report' crates/backend_core/src/git_sync/mod.rs && grep -q '^fn git_pull_and_sync_with_report_surface_duplicate_item_ids' crates/backend_core/tests/workflow_integration.rs && grep -q 'Stages and commits dirty .tracker/ state, then pulls and pushes if a remote is configured."' crates/backend_core/src/mcp/mod.rs

[MUTATION_SPEC]
1. mcp/mod.rs, tool definitions:
   - `git_pull`: append one sentence to the description, right after "...would be overwritten.": ` The result's \`duplicate_ids\` lists item IDs the post-pull reconcile found in more than one item directory (likely a merge from two clones).`
   - `git_sync`: append one sentence to the description: ` The result's \`duplicate_ids\` lists item IDs the post-pull reconcile found in more than one item directory (empty with commit_only or when nothing was pulled).`
   - The input schemas stay unchanged.
2. mcp/mod.rs `tool_git_pull`: call `storage.git_pull_with_report(&auth)?` instead of `git_pull`. The JSON is `{"updated": outcome.updated, "status": status, "duplicate_ids": outcome.duplicate_ids}`.
3. mcp/mod.rs `tool_git_sync`: replace `let sha = if .. { .. } else { .. };` with `let (sha, duplicate_ids) = ...`:
   - the commit_only arm gives `(storage.git_commit_tracker(..)?, Vec::new())`;
   - the other arm calls `storage.git_sync_with_report(non_empty(parsed.message.as_deref()), &auth)?` and gives `(outcome.sha, outcome.duplicate_ids)`.
   The JSON gains `"duplicate_ids": duplicate_ids` after `"tracker_dirty_after"`.
4. workflow_integration.rs: append `fn mcp_git_pull_and_sync_report_duplicate_item_ids` at the end of the file. It has local `use backend_core::McpServer;` and `use serde_json::json;`, and the same local `copy_dir` helper as the 26F test. Mirror that test's setup with project name `"McpDupProject"`, user `testuser` / `testuser@example.com`, and a bare remote plus clone. Then:
   - `server = McpServer::new(Some(TrackerStorage::new(clone_dir.path())))`.
   - **Idle pull:** `call_tool("git_pull", json!({}))` gives `["updated"] == json!(false)` and `["duplicate_ids"] == json!([])`.
   - **Duplicate pull:**
     - Workspace 1 creates a bug, copies its dir into `items_dir("production", "inbox")?.join(&id)`, then commits and pushes.
     - `call_tool("git_pull", json!({}))` gives `["updated"] == json!(true)`, `["duplicate_ids"] == json!([id])`, and `status` present.
   - **commit_only:** `call_tool("git_sync", json!({"commit_only": true}))` gives `["duplicate_ids"] == json!([])`.
   - **Sync:**
     - Workspace 1 creates a second bug, then commits and pushes.
     - `call_tool("git_sync", json!({}))` gives `["committed_or_head_sha"].is_string()` and `["duplicate_ids"] == json!([id])`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'storage.git_pull_with_report(&auth)?' crates/backend_core/src/mcp/mod.rs && grep -q 'git_sync_with_report(' crates/backend_core/src/mcp/mod.rs && ! grep -q 'storage.git_pull(&auth)?' crates/backend_core/src/mcp/mod.rs && ! grep -q 'storage.git_sync(non_empty' crates/backend_core/src/mcp/mod.rs && [ "$(grep -c '"duplicate_ids":' crates/backend_core/src/mcp/mod.rs)" = 2 ] && [ "$(grep -c "The result's \`duplicate_ids\` lists item IDs the post-pull reconcile found in more than one item directory" crates/backend_core/src/mcp/mod.rs)" = 2 ] && grep -q '"tracker_dirty_after": tracker_dirty_after,' crates/backend_core/src/mcp/mod.rs && grep -q 'fn mcp_git_pull_and_sync_report_duplicate_item_ids' crates/backend_core/tests/workflow_integration.rs && [ "$(git diff HEAD --numstat -- crates/backend_core/tests/workflow_integration.rs | awk '{print $2}')" = 0 ] && git diff --quiet HEAD -- crates/backend_core/src/git_sync crates/backend_core/src/storage crates/backend_core/src/models crates/backend_core/src/workflow crates/backend_core/src/lib.rs crates/backend_core/tests/credential_hygiene.rs crates/backend_core/tests/git_safety.rs crates/backend_core/tests/storage_integration.rs crates/nebulous_cli Cargo.toml Cargo.lock crates/backend_core/Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

Decisions the contract leaves open:
- **The 26F and 26G tests share ~40 lines of setup.** I didn't extract a helper, because that would mean editing 26F's test and the gate forbids touching existing tests. If a third two-workspace test ever appears, extract the helper then, in its own step.
- **MCP has no reconcile / sync-index tool** (I checked `mcp/mod.rs` for `reconcile` and `sync_index`: no matches). So pull and sync are the only MCP paths that reconcile. The watcher's `reconcile_report` already carries the full report, `duplicate_ids` included.

---


### STEP 26H: Total-Order `compare_item_ids` in What's Next (fix for the 26D pattern in assist)

* **Recommended Command:** `/goal`

**Why (found 2026-11-14 while writing 23C, from reading the code):** `workflow/assist.rs:756-763` `compare_item_ids` compares by number only when both IDs parse as sequential and the same type, and falls back to the raw string otherwise. That's the same non-total order 26D fixed in `chronological_cmp`: `BUG-9999 < BUG-10000` (numbers), `BUG-10000 < BUG-5ABCDE` (string, "1" < "5"), `BUG-5ABCDE < BUG-9999` (string, "5" < "9"). It's the last tie-break in 4 `sort_by` calls in `next_actions` (in-progress, recurring, stagnant, active-stage lists). Rust 1.81+ `sort_by` may panic on a comparator that isn't a total order, so a tracker with `id_scheme = "hash"` plus any 5-digit sequential IDs can crash `neb next` / the TUI recommendations when severity and the earlier keys tie. Hash IDs are opt-in and 5-digit counters are rare, so the risk is low but real.

**Fix:** one ID comparator for the whole crate. `scope.rs` already has the total one (`IdRank` via `id_rank`, 26D + 23A). Expose it as `pub(crate) fn cmp_item_ids`, have `chronological_cmp` use it, delete assist's copy.

**Behaviour change (intended, tie-break only):** among items that tie on every earlier key, sequential IDs now always sort before hash/other IDs (was: plain string order between them). Two corner cases also move: lowercase sequential IDs (`bug-0001`) now rank by type like uppercase ones, not by ASCII (`FEAT-…` used to sort before `bug-…`). Nothing else in the ranking changes.

**Current code (cited for the spec):**
- `models/scope.rs:122-135` private `enum IdRank` + `fn id_rank`; `:137-146` `chronological_cmp`'s `.then_with(|| id_rank(&self.id).cmp(&id_rank(&other.id)))`; `:148` existing `#[cfg(test)] mod tests` (23A) with a `stub(id, created_at)` helper.
- `workflow/assist.rs:1` `use std::cmp::Ordering;` and `:14` `use crate::storage::id::parse_item_id_number;` (both only used by `compare_item_ids`); `:463, :507, :545, :588` the 4 call sites `.then_with(|| compare_item_ids(&a.id, &b.id))`; `:756-763` `fn compare_item_ids`. `assist.rs` has no test module.
- `tests/storage_integration.rs:2689` 26D's `chronological_cmp_is_total_with_mixed_hash_and_wide_sequential_ids` (the same 3 IDs; stays untouched).

```text
[STEP_ID]
STEP 26H: One total-order item ID comparator (scope.rs cmp_item_ids); assist's non-total compare_item_ids is removed

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/scope.rs
  - crates/backend_core/src/workflow/assist.rs
READ_ONLY_REFS:
  - crates/backend_core/tests/storage_integration.rs:2689-2707 (26D totality test, same IDs)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No existing test is edited. `IdRank` / `id_rank` stay private. The earlier sort keys in the 4 assist `sort_by` calls (phase, scope, severity, recurrence, bug-first) stay byte-identical; only the final `compare_item_ids` call changes.

[PRE_GATE]
grep -q '^fn compare_item_ids' crates/backend_core/src/workflow/assist.rs && test "$(grep -c 'compare_item_ids(&a.id, &b.id)' crates/backend_core/src/workflow/assist.rs)" -eq 4 && grep -q '^fn id_rank' crates/backend_core/src/models/scope.rs

[MUTATION_SPEC]
1. scope.rs:
   a. Below `fn id_rank`, add
      `/// Total order on item IDs: sequential IDs (by type ordinal, then number, then ID string) before all other IDs (by string).`
      `pub(crate) fn cmp_item_ids(a: &str, b: &str) -> std::cmp::Ordering { id_rank(a).cmp(&id_rank(b)) }`
   b. `chronological_cmp`'s last line becomes `.then_with(|| cmp_item_ids(&self.id, &other.id))`. Its doc comment is unchanged.
   c. In the existing `mod tests`, add `cmp_item_ids_is_a_total_order_over_mixed_ids`: IDs `["BUG-9999", "BUG-10000", "BUG-5ABCDE", "FEAT-0001", "bug-0002", "BUG-ABCDEF", "NOTANID"]`.
      - Explicit: `cmp_item_ids("BUG-9999","BUG-10000") == Less`, `("BUG-10000","BUG-5ABCDE") == Less`, `("BUG-9999","BUG-5ABCDE") == Less`.
      - Exhaustive over all ordered pairs and triples of that list: antisymmetry (`cmp(a,b) == cmp(b,a).reverse()`), `cmp(a,a) == Equal`, and transitivity (if `cmp(a,b) != Greater && cmp(b,c) != Greater` then `cmp(a,c) != Greater`).
2. assist.rs:
   a. Delete `fn compare_item_ids` (:756-763) and its doc comment if any.
   b. The 4 call sites become `.then_with(|| cmp_item_ids(&a.id, &b.id))`.
   c. Imports: add `use crate::models::scope::cmp_item_ids;`; remove `use crate::storage::id::parse_item_id_number;` and `use std::cmp::Ordering;` (:1) — both are only used by `compare_item_ids` and become unused; clippy -D warnings confirms.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && git diff --quiet HEAD -- crates/backend_core/tests crates/backend_core/src/storage crates/backend_core/src/mcp crates/nebulous_cli && ! grep -n 'compare_item_ids' crates/backend_core/src/workflow/assist.rs && test "$(grep -c 'cmp_item_ids(&a.id, &b.id)' crates/backend_core/src/workflow/assist.rs)" -eq 4 && test "$(grep -c 'item_type == ItemType::Bug' crates/backend_core/src/workflow/assist.rs)" -eq 3 && ! grep -n 'pub(crate) enum IdRank\|pub(crate) fn id_rank\|pub fn id_rank' crates/backend_core/src/models/scope.rs && cargo test -p backend_core --lib cmp_item_ids_is_a_total_order_over_mixed_ids 2>&1 | grep -q 'test result: ok. 1 passed'

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status. Halt and report instead of editing an existing test if any existing test fails.
```

**Decisions the contract leaves open:**
- **Unit test only, no end-to-end `next_actions` test.** A comparator that isn't total only *may* panic, depending on input size and order, so an e2e test wouldn't reliably catch a regression. The exhaustive pair/triple check on the comparator itself does, and the gate proves assist calls that comparator at all 4 sites.
- **`cmp_item_ids` lives in `scope.rs`, not `id.rs`.** `IdRank` is already there and stays private; moving it would touch a third file for no gain. When 23F adds `Task`, `ordinal()` gives it rank 2 with no change here.

---

# Phase 11 — Separation (approved 2026-11-11)

**Status: COMPLETED on the Nebulous side 2026-11-11** (29B `0f341ed` … 29H `55f72c7`). The release engine is out (`crates/`, `Cargo.toml` and `Cargo.lock` equaled `fb2966d` after 29C). `scripts/check-neutral.sh` runs in CI with an empty allowlist. Still open:
- the NovOS-side handoff N1–N6 below, for a NovOS session to run;
- the `default_rolling_dep_kind` = `"apt_sid"` question under Steps;
- Phase 12 (generic self-healing), to be planned when it's reached.

## Why

Nebulous is planning, tracker and MCP. NovOS uses it, but Nebulous is never dedicated to NovOS. Phase 9 (NovOS STEP 13B) built NovOS's release engine inside Nebulous, and it is NovOS-shaped all the way down to its data model:
- `Golden` carries `iso`, `boot` and `calamares_modules`.
- Variance checks Calamares and custom binaries.
- `candidate.rs` is built around the test-overlay ISO.
- `repo_invariants.rs` reads NovOS's repo layout.
- `import.rs` reads `.nebulous/`.

Making it generic in place would be a rewrite: the withdrawn 26-step "neutral release engine" plan. Moving it is cheap, because it barely touches the rest of Nebulous:
- **Four touchpoints:** `TrackerError`, `atomic_write_str`, `TrackerStorage` and `is_valid_identifier`.
- **Every commit after `fb2966d` is Phase 9.** `git diff --stat fb2966d HEAD -- crates Cargo.toml Cargo.lock` shows 31 files, +19,529 and −7.
- **NovOS isn't using it yet.** Its live callers are still `scripts/nebulous-ctl.py` and `scripts/mcp_nebulous_ci.py`, so moving the code breaks nothing.

## Decisions (user, 2026-11-11)

- **All of Phase 9 moves to NovOS as is.** It becomes a `novos-release` crate and binary in NovOS's own Cargo workspace. No code is thrown away: the 17.3k-line engine with its 143 tests and the 2.1k-line CLI with its 15 tests all move.
- **NovOS owns its release state.** It goes in a NovOS directory with its own lock, not in Nebulous's `.tracker/`.
- **Nebulous gets generic self-healing later**, in its own phase (Phase 12). That covers item attempts, the loop breaker and the user-only gate, built as tracker features. NovOS adopts them when they exist.
- **Nothing in Nebulous names NovOS** or any distro. A CI ratchet enforces it.

## Withdrawn 2026-11-11: release engine moves to NovOS

- **Phase 9 rows withdrawn:**
  - **27F, 27G, 27H** (lifecycle, bisect and user-only handlers, wiring)
  - **25V** (CLI integration tests)
  - **25W, 25X, 25Y** (MCP release tools)
  - **25Z** (release docs)
  - Their specs carry over to NovOS (see "Handoff to NovOS" below).
- **Old Phase 11 withdrawn:** 28A–28Z (neutral release engine), and Open Questions 8–13. Only its scrub steps survive, as 29D–29H.
- **History stays.** Phase 9's Completed rows and Argon blocks are kept as the record of what moved.

## Steps

- **29A (Claude, plan only): COMPLETED 2026-11-11.** This section, the roadmap edit and the handoff appendix.
- **29B `/goal`: remove the release CLI. COMPLETED (`0f341ed`).**
  - `args.rs` and `commands/mod.rs` go back to their `fb2966d` content.
  - `commands/release/` is deleted.
- **29C `/goal`: remove the release engine. COMPLETED (`370c66f`).**
  - `crates/backend_core/src/release/` is deleted.
  - `backend_core/src/lib.rs`, `backend_core/Cargo.toml` and `Cargo.lock` go back to `fb2966d`, which drops `regex` and `flate2`.
  - Afterwards, `crates/`, `Cargo.toml` and `Cargo.lock` equal `fb2966d`.
- **29D `/goal`: neutrality ratchet. COMPLETED (`41b48d2`).**
  - New `scripts/check-neutral.sh` and a CI step.
  - Forbidden tokens (case-insensitive) under `crates/`, `README.md` and `docs/`: `novos`, `debian`, `dpkg`, `squashfs`, `xorriso`, `calamares`, `xanmod`, `qemu`.
  - The allowlist starts as today's offenders and only shrinks.
- **29E `/goal`: docs scrub** (`README.md`, `docs/USAGE.md`). COMPLETED (`17e3e4d`).
- **29F `/goal`: backend source strings** (`backend_core/src/lib.rs`, `mcp/mod.rs`, `models/component.rs`). Tool names and schemas stay unchanged. COMPLETED (`ec1a3d5`).
- **29G `/goal`: ingestion heuristic and storage test data** (`backend_core/src/ingestion/mod.rs`, `tests/storage_integration.rs`). COMPLETED (`c68bf2a`).
- **29H `/goal`: CLI strings and test data** (`nebulous_cli/src/lib.rs`, `tests/cli_integration.rs`). After this the allowlist is empty. COMPLETED (`55f72c7`).
- **Re-split 2026-11-11 (29D re-count):** two offenders the first count missed (`ingestion/mod.rs`, `models/component.rs`). The old 29F was split so each step stays at 2–3 files, and the old 29G became 29H. Each scrub step also edits the allowlist in `scripts/check-neutral.sh`.
- **Open (user), found writing 29F:** `default_rolling_dep_kind()` in `backend_core/src/mcp/mod.rs` defaults `record_rolling_update`'s `kind` to `"apt_sid"`, a Debian channel. The ratchet doesn't catch it (`apt` isn't a token), and changing it changes what the tool writes. Decide whether to make it neutral (e.g. `"system"`) as its own step, or leave it.

Argon blocks are written one at a time, just before each step runs.

## Handoff to NovOS

This is for a NovOS session to put into NovOS's own plan and run under NovOS's rules. Nothing under `/srv/devshare/NovOS` is edited from this repo.

- **Source.** Take it from the Nebulous repo at commit **`d147bf6`**:
  - `crates/backend_core/src/release/`: 23 files plus `testdata/debver_cases.txt`, 17,346 lines, 143 tests.
  - `crates/nebulous_cli/src/args.rs` lines 686–1545: `StageArg`, `PhaseArg`, `ResultArg`, `HoldFormat`, `ReleaseCommand`, `BisectCommand`, and their 4 tests.
  - `crates/nebulous_cli/src/commands/release/{mod,baseline}.rs`: 1,237 lines, 11 tests.
  - The Argon blocks for 25B–25U and 27A–27E in this file are the design record.
- **N1. `crates/novos-release` (lib).**
  - Replace the four Nebulous touchpoints:
    - `crate::error::TrackerError` → a local `Error` enum with only the variants used. Keep `LockTimeout` and `UnsupportedSchema` → exit 7 in `ExitCode::from_error`.
    - `crate::storage::atomic_write_str` → a local copy (write tmp, fsync, rename).
    - `TrackerStorage` (`config.rs` `ReleasePaths::new`, and tests) plus `is_valid_identifier` → a `ReleaseRoot { repo_root, state_dir }` with an fd-lock write lock and a 30 s timeout.
  - Dependencies: chrono, serde, serde_json, toml, sha2, regex, flate2, tempfile, fd-lock, thiserror.
  - Done when `cargo test -p novos-release` runs the same 143 tests, and fmt and clippy are clean.
- **N2. State dir.**
  - Change the `ReleasePaths` root from `.tracker/release` to a NovOS-owned directory (NovOS picks it, e.g. `.novos/release/`), with its own lock file.
  - Import still reads `.nebulous/`.
- **N3. The `novos-release` binary.**
  - Port the CLI files above.
  - Then write the remaining handlers. The withdrawn rows are their specs:
    - **27F:** candidate (refuses a test ISO; snapshot from build.env), record-phase (verdict via `parse_verdict_overlay`, plus the test ISO's `read_test_overlay_manifest` for VM phases), record-failure (class rule), promote (prepare + commit under the lock).
    - **27G:** bisect start/next/record/status, plus the user-only handlers behind the §5.1 gate: bless-golden, approve-variance, unlock-loop, import, bisect abort.
    - **27H:** dispatch, and `main` exiting with the release code.
    - **25V:** CLI integration tests.
  - The 25U block's "Decisions" carry over:
    - `ReleaseRun`, plus the envelope/stderr rule;
    - `item_status` in check-guardrail;
    - extract-golden refusing a test ISO and the golden path.
- **N4. MCP.** NovOS decides whether to keep `scripts/mcp_nebulous_ci.py` and point it at the binary, or add `novos-release mcp`. The withdrawn 25W–25Y rows describe the tools.
- **N5. Contract and plan.**
  - `docs/NEB_RELEASE_CONTRACT.md` becomes NovOS's own release contract: the command name, the state dir, and the envelope `schema` string.
  - In NovOS's plan, 13B becomes N1–N4, 13C switches callers to `novos-release`, and 13D retires the Python tools.
  - Update `guard-bash.sh` and `guard-edit.sh` for the new command name and state dir, and protect the new `config.toml` (it holds the guardrails and harness commands).
- **N6. Nebulous stays NovOS's tracker.** `.tracker/`, `neb` planning and `neb mcp` keep working for NovOS unchanged.
- **Verification (NovOS):**
  - `cargo test -p novos-release` gives 143 lib tests + 15 CLI tests.
  - `novos-release self-test` exits 0.
  - `novos-release verify-variance --repo-only` agrees with `nebulous-ctl.py verify-variance --repo-only` on the NovOS tree.

---

### STEP 29B: Remove the Release CLI

* **Recommended Command:** `/goal`

Scope note (2026-11-11). This is the first removal step of Phase 11 (Separation). It deletes the CLI half of Phase 9, which is moving to NovOS from commit `d147bf6` (see "Handoff to NovOS"). It is a pure revert: both edited files go back to their `fb2966d` content, the last commit before Phase 9. That's why the gate compares against that commit instead of grepping. The backend half is 29C.

```text
[STEP_ID]
STEP 29B: remove the release CLI: args.rs and commands/mod.rs back to fb2966d, delete commands/release/

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/args.rs
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/nebulous_cli/src/commands/release/*
READ_ONLY_REFS:
  - `git show fb2966d:crates/nebulous_cli/src/args.rs` and `git show fb2966d:crates/nebulous_cli/src/commands/mod.rs` (the target content)
  - `git diff fb2966d HEAD -- crates/nebulous_cli` (what Phase 9 added: the release enums, their tests, `pub mod release;`, commands/release/)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Nothing under crates/backend_core (STEP 29C). No other edits to args.rs or commands/mod.rs beyond restoring them. No commits (Claude commits).

[PRE_GATE]
test -d crates/nebulous_cli/src/commands/release && grep -q 'pub enum ReleaseCommand' crates/nebulous_cli/src/args.rs && grep -q '^pub mod release;' crates/nebulous_cli/src/commands/mod.rs && git cat-file -e fb2966d:crates/nebulous_cli/src/args.rs && test "$(git log --format=%h fb2966d..HEAD -- crates/nebulous_cli/src/args.rs crates/nebulous_cli/src/commands/mod.rs | wc -l)" -eq 2

[MUTATION_SPEC]
1. `git checkout fb2966d -- crates/nebulous_cli/src/args.rs crates/nebulous_cli/src/commands/mod.rs` (no other change to either file).
2. Delete the directory: `git rm -r -q crates/nebulous_cli/src/commands/release`.
3. Nothing else. backend_core still has `pub mod release` and must still build: the CLI simply stops using it.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && test ! -e crates/nebulous_cli/src/commands/release && git diff --quiet fb2966d -- crates/nebulous_cli/src/args.rs crates/nebulous_cli/src/commands/mod.rs && ! grep -rq 'release::' crates/nebulous_cli/src && git diff --quiet HEAD -- crates/backend_core Cargo.toml Cargo.lock

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 29C: Remove the Release Engine

* **Recommended Command:** `/goal`

Scope note (2026-11-11). This is the second and last removal step of Phase 11 (Separation). 29B already removed the CLI half (`0f341ed`). This step deletes `crates/backend_core/src/release/`, which is moving to NovOS from commit `d147bf6`, and restores the three files Phase 9 touched to their `fb2966d` content:
- `backend_core/src/lib.rs`: drops `pub mod release;`.
- `backend_core/Cargo.toml`: drops `flate2` and `regex`, and moves `tempfile` back to `[dev-dependencies]`, where it was before Phase 9.
- `Cargo.lock`: drops `flate2` and `regex` from backend_core's own dependency list. Both crates stay in the lock as transitive dependencies, as they were at `fb2966d`.

Like 29B, it's a pure revert. Once it lands, `crates/`, `Cargo.toml` and `Cargo.lock` all equal `fb2966d`.

```text
[STEP_ID]
STEP 29C: remove the release engine: delete backend_core/src/release/, restore lib.rs, backend_core/Cargo.toml and Cargo.lock to fb2966d

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/release/*
  - crates/backend_core/src/lib.rs
  - crates/backend_core/Cargo.toml
  - Cargo.lock
READ_ONLY_REFS:
  - `git show fb2966d:crates/backend_core/src/lib.rs`, `git show fb2966d:crates/backend_core/Cargo.toml`, `git show fb2966d:Cargo.lock` (the target content)
  - `git diff fb2966d HEAD -- crates Cargo.toml Cargo.lock` (should show only the release/ directory plus these three files before this step)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No edits beyond restoring the three files and deleting the directory. Nothing under crates/nebulous_cli. No `cargo update` (Cargo.lock comes from fb2966d as is). No commits (Claude commits).

[PRE_GATE]
test -d crates/backend_core/src/release && grep -q '^pub mod release;' crates/backend_core/src/lib.rs && ! grep -rq 'release::' crates/nebulous_cli/src && git diff --quiet fb2966d -- crates/nebulous_cli && test -z "$(git diff --name-only fb2966d HEAD -- crates Cargo.toml Cargo.lock | grep -v '^crates/backend_core/src/release/' | grep -vx 'crates/backend_core/src/lib.rs' | grep -vx 'crates/backend_core/Cargo.toml' | grep -vx 'Cargo.lock')" && test -z "$(grep -rl 'crate::release\|release::' crates/backend_core/src crates/backend_core/tests --include=*.rs | grep -v '^crates/backend_core/src/release/')"

[MUTATION_SPEC]
1. `git checkout fb2966d -- crates/backend_core/src/lib.rs crates/backend_core/Cargo.toml Cargo.lock`.
2. Delete the directory: `git rm -r -q crates/backend_core/src/release` (testdata included).
3. Nothing else.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && test ! -e crates/backend_core/src/release && git diff --quiet fb2966d -- crates Cargo.toml Cargo.lock

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

### STEP 29D: Neutrality Ratchet

* **Recommended Command:** `/goal`

Scope note (2026-11-11). Phase 11 (Separation) wants nothing in Nebulous to name NovOS or any distro. This step adds the check that holds that line, without fixing any text yet. 29E–29H scrub the offenders and shrink the allowlist to empty.

Re-counted after 29C (`370c66f`), as matching lines per file (`grep -ciIE`). That's 182 lines across 9 files:

| File | Lines | Scrubbed in |
| :--- | ---: | :--- |
| `README.md` | 2 | 29E |
| `docs/USAGE.md` | 14 | 29E |
| `crates/backend_core/src/lib.rs` | 8 | 29F |
| `crates/backend_core/src/mcp/mod.rs` | 7 | 29F |
| `crates/backend_core/src/models/component.rs` | 2 | 29F |
| `crates/backend_core/src/ingestion/mod.rs` | 1 | 29G |
| `crates/backend_core/tests/storage_integration.rs` | 105 | 29G |
| `crates/nebulous_cli/src/lib.rs` | 2 | 29H |
| `crates/nebulous_cli/tests/cli_integration.rs` | 41 | 29H |

The scan covers every tracked file except `archive/` (the retired GUI, kept as history), `Cargo.lock` (third-party crate names) and the script itself. That's wider than "crates/, README, docs", and it costs nothing: today no other tracked file has a hit. Only tracked files are scanned, so `target/` and `.tracker/` never count.

```text
[STEP_ID]
STEP 29D: neutrality ratchet: scripts/check-neutral.sh (forbidden distro tokens, exact-count allowlist) + CI step

[SCOPE_LOCK]
ALLOWED_FILES:
  - scripts/check-neutral.sh
  - .github/workflows/ci.yml
READ_ONLY_REFS:
  - scripts/install.sh (house style: `#!/usr/bin/env bash`, `set -euo pipefail`)
  - .github/workflows/ci.yml (the `verify` job's step list)
  - The table in this block's scope note (the allowlist, verbatim)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Don't fix any offender: no edits under crates/, README.md or docs/ (29E–29H do that). No changes to the existing CI steps. No commits (Claude commits).

[PRE_GATE]
test ! -e scripts/check-neutral.sh && ! grep -q 'check-neutral' .github/workflows/ci.yml && grep -q 'name: Checkout repository' .github/workflows/ci.yml && test "$(git ls-files | grep -vE '^archive/|^Cargo\.lock$' | xargs grep -ciIE 'novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu' | grep -v ':0$' | awk -F: '{s+=$NF; n++} END {print n" "s}')" = "9 182"

[MUTATION_SPEC]
1. scripts/check-neutral.sh (NEW, mode 755). It starts with `#!/usr/bin/env bash` and `set -euo pipefail`, then a header comment: what it enforces, and "the allowlist only shrinks; when you remove hits, lower or delete the entry in the same change".
   - `cd "$(git rev-parse --show-toplevel)"`.
   - `PATTERN='novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu'`, matched with `grep -ciIE` (case-insensitive, skip binary files, count matching lines).
   - The allowlist is a heredoc read into an associative array, one `<count> <path>` per line, holding exactly the 9 rows of the scope-note table.
   - Files to check:
     - With no arguments: `git ls-files`, minus paths under `archive/`, `Cargo.lock` and `scripts/check-neutral.sh`. Skip paths that don't exist on disk (deleted but not staged).
     - With arguments (`check-neutral.sh FILE...`): exactly those files, still judged against the allowlist by path as given. This mode exists so the gate can test the script on temp files.
   - Per file, n = the count:
     - n > 0 and not allowlisted → fail. Print `<path>: <n> line(s) name a distro (not allowlisted)`, then that file's `grep -niIE` lines.
     - allowlisted at m and n > m → fail. Print `<path>: <n> lines, allowlist says <m>: remove the new ones`, then the file's matching lines.
     - allowlisted at m and n < m → fail. Print `<path>: <n> lines, allowlist says <m>: lower the entry to <n>`, or `delete the entry` when n = 0.
   - With no arguments only: an allowlist path that isn't in the scanned set → fail, `<path>: allowlisted but not tracked: delete the entry`.
   - Exit 1 if anything failed. Otherwise exit 0 and print `check-neutral: OK (<k> allowlisted file(s), <t> line(s) left to scrub)`. With an empty allowlist that is `(0 allowlisted file(s), 0 line(s) left to scrub)`.
   - Error output goes to stderr, the OK line to stdout. Plain bash 4+ and coreutils/grep only.
2. .github/workflows/ci.yml: in the `verify` job, add one step right after "Checkout repository": `- name: Neutrality check (no distro-specific names)` with `run: bash scripts/check-neutral.sh`. Nothing else in the file changes.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash -n scripts/check-neutral.sh && test -x scripts/check-neutral.sh && scripts/check-neutral.sh | grep -q '^check-neutral: OK (9 allowlisted file(s), 182 line(s) left to scrub)$' && scripts/check-neutral.sh README.md docs/USAGE.md && t="$(mktemp)" && printf 'Built on NovOS\n' > "$t" && ! scripts/check-neutral.sh "$t" 2>/dev/null && printf 'neutral text\n' > "$t" && scripts/check-neutral.sh "$t" && rm -f "$t" && grep -q 'run: bash scripts/check-neutral.sh' .github/workflows/ci.yml && test "$(git diff --numstat -- .github/workflows/ci.yml | awk '{print $2}')" -eq 0 && git diff --quiet -- crates README.md docs Cargo.toml Cargo.lock

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

Decisions the contract leaves open:
- **Exact counts, not a ceiling.** A drop that isn't written into the allowlist also fails. That's what makes it a ratchet: every scrub step has to lower its entries, so the list can't go stale.
- **Counts are matching lines, not occurrences.** A line with two tokens counts once.
- **A file with no entry has an allowance of 0.** Once 29H lands, the heredoc is empty and the script still runs.

---

### STEP 29E: Docs Scrub

* **Recommended Command:** `/goal`

Scope note (2026-11-11). This is the first scrub step of Phase 11 (Separation). The 29D ratchet (`41b48d2`) allowlists 16 lines in the two user-facing docs: README.md has 2 and docs/USAGE.md has 14. This step rewrites them with a made-up neutral example, a rolling-release OS called **Orbit** (the name isn't used anywhere in the repo today). Then it takes both entries out of the allowlist in `scripts/check-neutral.sh`.

It also folds in the gap found while validating 29D. In argument mode, a path that doesn't exist currently passes, because grep errors and the empty count is read as 0. After this step it fails.

What these docs describe doesn't change: the same commands, flags, flag order and line count. Only the example values change.

The README's MCP suite name becomes "Release Engineering & Self-Healing CI/CD". 29F renames the matching comment in `mcp/mod.rs` to the same words.

```text
[STEP_ID]
STEP 29E: docs scrub: README.md and docs/USAGE.md use a neutral "Orbit" example; drop both from the check-neutral allowlist; missing argument files fail

[SCOPE_LOCK]
ALLOWED_FILES:
  - README.md
  - docs/USAGE.md
  - scripts/check-neutral.sh
READ_ONLY_REFS:
  - `scripts/check-neutral.sh` (the allowlist heredoc, lines 21-31, and the per-file loop, lines 49-72)
  - `grep -niIE 'novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu' README.md docs/USAGE.md` (the 16 lines to rewrite)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No other wording changes in README.md or docs/USAGE.md (don't fix the tool count, headings or anything else you notice). Don't add or remove lines in either doc. No changes under crates/ (29F–29H). No commits (Claude commits).

[PRE_GATE]
test "$(grep -ciIE 'novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu' README.md)" -eq 2 && test "$(grep -ciIE 'novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu' docs/USAGE.md)" -eq 14 && test "$(wc -l < README.md)" -eq 180 && test "$(wc -l < docs/USAGE.md)" -eq 236 && grep -q '^14 docs/USAGE.md$' scripts/check-neutral.sh && grep -q '^2 README.md$' scripts/check-neutral.sh && ! grep -rqi 'orbit' README.md docs crates && scripts/check-neutral.sh >/dev/null

[MUTATION_SPEC]
1. README.md, line 7: delete ` (such as **NovOS**)`, so the line reads `... built in Rust for developers, rolling-release OS builders, and AI-assisted coding workflows. ...`. Nothing else on the line changes.
2. README.md, line 34: replace `NovOS Self-Healing CI/CD` with `Release Engineering & Self-Healing CI/CD`.
3. docs/USAGE.md: make these exact replacements, each on its current line only:
   - line 85: `--name "NovOS" --tags os,debian-sid,xanmod` → `--name "Orbit" --tags os,rolling,custom-kernel`
   - lines 88 and 91: `"NovOS"` → `"Orbit"`
   - every `novos_iso` (lines 120, 128, 134, and twice on 139) → `orbit_image`
   - line 121: `"NovOS Live ISO"` → `"Orbit Live Image"`
   - line 123: `--stack debian-sid,xanmod,live-build` → `--stack rolling-base,custom-kernel,image-builder`
   - lines 125 and 130: `--flavour=xanmod` → `--flavour=rt`
   - line 131: `--dep linux-xanmod-x64v3=6.11.6-x64v3-xanmod1,systemd=256.7-1` → `--dep linux-image-rt=6.11.6-rt1,systemd=256.7-1`
   - line 133: `when an upstream Sid/XanMod update breaks the build` → `when an upstream rolling update breaks the build`
   - line 136: `in QEMU check` → `in the VM boot check`
4. scripts/check-neutral.sh:
   - delete the heredoc lines `2 README.md` and `14 docs/USAGE.md`; the other 7 stay as they are;
   - at the top of the per-file loop, before the grep, add `if [[ ! -f "$file" ]]; then echo "${file}: no such file" >&2; failed=1; continue; fi`.
   - Nothing else changes.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash -n scripts/check-neutral.sh && scripts/check-neutral.sh | grep -q '^check-neutral: OK (7 allowlisted file(s), 166 line(s) left to scrub)$' && ! grep -qiIE 'novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu' README.md docs/USAGE.md && ! scripts/check-neutral.sh /nonexistent/check-neutral-probe 2>/dev/null && test "$(wc -l < README.md)" -eq 180 && test "$(wc -l < docs/USAGE.md)" -eq 236 && test "$(git diff --numstat -- README.md | awk '{print $1" "$2}')" = "2 2" && test "$(git diff --numstat -- docs/USAGE.md | awk '{print $1" "$2}')" = "14 14" && grep -q '^neb component promote orbit_image --scope orbit_image$' docs/USAGE.md && grep -q 'Release Engineering & Self-Healing CI/CD' README.md && git diff --quiet -- crates Cargo.toml Cargo.lock .github

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

Decisions the contract leaves open:
- **Why "Orbit" and not a real distro.** Any real distro name would only swap one dependency for another, and the ratchet would have to allow it. A made-up name keeps the example concrete without naming anything real.
- **`--type os_image` and the `iso` directory stay.** They're Nebulous's own generic component types, not a distro's.

---

### STEP 29F: Backend Source Strings Neutral

* **Recommended Command:** `/goal`

Scope note (2026-11-11). This is the second scrub step of Phase 11 (Separation). The ratchet (`17e3e4d`) allowlists 17 lines across three backend_core source files:

| File | Lines | What they are |
| :--- | ---: | :--- |
| `src/lib.rs` | 8 | Vault test fixtures in `mod tests`: the secret key `project:novos:pat`, the fake token `glpat_novos_rolling_token` and the SSH account `novos/upstream` |
| `src/mcp/mod.rs` | 7 | Two `// Suite 3` comments, plus the description text in five tool schemas |
| `src/models/component.rs` | 2 | Doc comments on `ComponentDependency` and `VerificationRecord` |

This step changes only text: comments, description strings and test fixture values. The neutral words match 29E, so "Orbit" stands in for NovOS and the suite is "Release Engineering & Self-Healing CI".

These stay unchanged: tool names, schema keys and types, the `required` lists, and every default value. That includes `default_rolling_dep_kind()` returning `"apt_sid"` (`mcp/mod.rs`, near the end of the file). Changing a default is a behavior change, and the ratchet doesn't match that token anyway.

I checked that no test asserts on any of the strings being changed.

```text
[STEP_ID]
STEP 29F: backend source strings neutral: lib.rs vault fixtures, mcp/mod.rs suite comments + tool descriptions, models/component.rs doc comments; drop all three from the check-neutral allowlist

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/lib.rs
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/src/models/component.rs
  - scripts/check-neutral.sh
READ_ONLY_REFS:
  - `grep -niIE 'novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu' crates/backend_core/src/lib.rs crates/backend_core/src/mcp/mod.rs crates/backend_core/src/models/component.rs` (the 17 lines)
  - scripts/check-neutral.sh (the allowlist heredoc)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Change only the substrings listed in MUTATION_SPEC. No tool names, schema keys, types, `required` lists or default values (`default_rolling_dep_kind` keeps `"apt_sid"`). No lines added or removed. Nothing under crates/backend_core/tests or crates/nebulous_cli (29G, 29H). No commits (Claude commits).

[PRE_GATE]
P='novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu'; test "$(grep -ciIE "$P" crates/backend_core/src/lib.rs)" -eq 8 && test "$(grep -ciIE "$P" crates/backend_core/src/mcp/mod.rs)" -eq 7 && test "$(grep -ciIE "$P" crates/backend_core/src/models/component.rs)" -eq 2 && test "$(grep -c 'NovOS' crates/backend_core/src/lib.rs)" -eq 0 && grep -q '^8 crates/backend_core/src/lib.rs$' scripts/check-neutral.sh && grep -q '^7 crates/backend_core/src/mcp/mod.rs$' scripts/check-neutral.sh && grep -q '^2 crates/backend_core/src/models/component.rs$' scripts/check-neutral.sh && grep -q 'fn default_rolling_dep_kind' crates/backend_core/src/mcp/mod.rs && scripts/check-neutral.sh >/dev/null

[MUTATION_SPEC]
1. crates/backend_core/src/lib.rs (all in `mod tests`): replace every `novos` with `orbit`. All 8 matches are lowercase. That gives:
   - `project:orbit:pat` (lines 483, 518, 526, 536, 559);
   - `glpat_orbit_rolling_token` (lines 483, 520, 561);
   - `"orbit/upstream"` (line 652).
2. crates/backend_core/src/mcp/mod.rs: make these exact substring replacements.
   - line 752: `(e.g. novos_iso or backend_core)` → `(e.g. orbit_image or backend_core)`
   - lines 780 and 935: `// Suite 3: NovOS Release Engineering & Self-Healing CI Tools` → `// Suite 3: Release Engineering & Self-Healing CI Tools`
   - line 783: `(e.g. Debian Sid package or XanMod kernel delta)` → `(e.g. a rolling distro package or custom kernel delta)`
   - line 788: `(e.g. linux-xanmod-x64v3 or systemd)` → `(e.g. linux-image-rt or systemd)`
   - line 789: `(e.g. apt_sid, kernel_xanmod, cargo, git)` → `(e.g. apt_sid, kernel_custom, cargo, git)`
   - line 803: `Records QEMU, container, or CI test-suite` → `Records VM, container, or CI test-suite`
3. crates/backend_core/src/models/component.rs:
   - line 412: `` `kernel_xanmod` `` → `` `kernel_custom` ``
   - line 542: `/// QEMU, container, or test-suite` → `/// VM, container, or test-suite`
4. scripts/check-neutral.sh: delete the three heredoc lines `8 crates/backend_core/src/lib.rs`, `7 crates/backend_core/src/mcp/mod.rs` and `2 crates/backend_core/src/models/component.rs`. Nothing else in the script changes.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && scripts/check-neutral.sh | grep -q '^check-neutral: OK (4 allowlisted file(s), 149 line(s) left to scrub)$' && ! grep -qiIE 'novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu' crates/backend_core/src/lib.rs crates/backend_core/src/mcp/mod.rs crates/backend_core/src/models/component.rs && test "$(git diff --numstat -- crates/backend_core/src/lib.rs | awk '{print $1" "$2}')" = "8 8" && test "$(git diff --numstat -- crates/backend_core/src/mcp/mod.rs | awk '{print $1" "$2}')" = "7 7" && test "$(git diff --numstat -- crates/backend_core/src/models/component.rs | awk '{print $1" "$2}')" = "2 2" && test "$(git diff --numstat -- scripts/check-neutral.sh | awk '{print $1" "$2}')" = "0 3" && grep -q '"apt_sid".to_string()' crates/backend_core/src/mcp/mod.rs && test "$(git diff -U0 -- crates/backend_core/src/mcp/mod.rs | grep -c '^+.*name: "')" -eq 0 && git diff --quiet -- crates/backend_core/tests crates/nebulous_cli Cargo.toml Cargo.lock README.md docs .github

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

Decisions the contract leaves open:
- **`apt_sid` stays.** It's the default `kind` for `record_rolling_update` (`default_rolling_dep_kind`), and the description at line 789 still lists it as an example, because it's a real value the tool writes. Making the default neutral (for example `"system"`) is a schema/behavior change, so it isn't part of the scrub. That's for you to decide (see the note under Phase 11 Steps).
- **The fake token keeps its `glpat_` prefix.** The test exercises vault storage, not token format. Only the name inside it changes.

---

### STEP 29G: Ingestion Heuristic and Storage Test Data Neutral

* **Recommended Command:** `/goal`

Scope note (2026-11-11). This is the third scrub step of Phase 11 (Separation). The ratchet (`ec1a3d5`) allowlists 106 lines across two files:
- **`crates/backend_core/src/ingestion/mod.rs`, 1 line, line 1061.** The component-type heuristic classifies a directory as `ComponentType::KernelModule` when its name contains `"kernel"`, contains `"xanmod"` or equals `"kmod"`. The `"xanmod"` check is the one distro-specific rule in Nebulous's code, and it goes. This is a small behavior change: a directory called `xanmod` with no `kernel` in its name no longer counts as a kernel module, and falls through to the other rules. No test depends on it.
- **`crates/backend_core/tests/storage_integration.rs`, 105 lines.** Test fixtures: the project registry test (`NovOS`, `novos_2`), the component-pinning test (`test_novos_component_manifest_…`, Debian Sid/XanMod packages and `deb.xanmod.org`/`deb.debian.org` URLs), the hand-written TOML fixtures, and the `scripts/qemu` subpath ingestion test.

That's too many lines to list one by one, so the change is a fixed `sed` script. It's in this block word for word.
- **I've dry-run it on a copy:** 0 hits left, 105 lines changed, the line count unchanged at 2530, and `rustfmt --check` clean.
- **The replacements keep their lengths** (`novos`→`orbit`, `xanmod`→`custom`, `qemu`→`virt`), so rustfmt has nothing to rewrap.
- **Test-data hosts become `packages.example.com`**, which is reserved for examples. The current real hosts `deb.xanmod.org` and `deb.debian.org` in test fixtures are gone.
- **The gate pins the result** to the sha256 from the dry-run, so the outcome has to match exactly.

Every assertion changes together with the value it checks: `"novos"` becomes `"orbit"`, `"novos_2"` becomes `"orbit_2"`, `"novos_live_iso"` becomes `"orbit_live_iso"`, and the subpath id `"qemu"` becomes `"virt"`. So the tests prove the same things. There are still 22 `#[test]` functions; one of them gets a new name.

```text
[STEP_ID]
STEP 29G: ingestion kernel heuristic drops "xanmod"; storage_integration.rs fixtures neutral via a fixed sed script; drop both from the check-neutral allowlist

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/ingestion/mod.rs
  - crates/backend_core/tests/storage_integration.rs
  - scripts/check-neutral.sh
READ_ONLY_REFS:
  - crates/backend_core/src/ingestion/mod.rs lines 1053-1072 (the component-type `if` chain)
  - `grep -niIE 'novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu' crates/backend_core/tests/storage_integration.rs` (the 105 lines)
  - scripts/check-neutral.sh (the allowlist heredoc)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Don't hand-edit storage_integration.rs: apply only the sed script below, then change nothing else (don't run `cargo fmt` on it; it's already formatted). In ingestion/mod.rs, change only line 1061. No changes under crates/nebulous_cli (29H). Don't add the sed script file to the repo. No commits (Claude commits).

[PRE_GATE]
P='novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu'; test "$(grep -ciIE "$P" crates/backend_core/src/ingestion/mod.rs)" -eq 1 && grep -q '^    } else if dir_leaf.contains("kernel") || dir_leaf.contains("xanmod") || dir_leaf == "kmod" {$' crates/backend_core/src/ingestion/mod.rs && test "$(grep -ciIE "$P" crates/backend_core/tests/storage_integration.rs)" -eq 105 && test "$(wc -l < crates/backend_core/tests/storage_integration.rs)" -eq 2530 && test "$(grep -c '#\[test\]' crates/backend_core/tests/storage_integration.rs)" -eq 22 && ! grep -qi 'orbit\|virt' crates/backend_core/tests/storage_integration.rs && grep -q '^1 crates/backend_core/src/ingestion/mod.rs$' scripts/check-neutral.sh && grep -q '^105 crates/backend_core/tests/storage_integration.rs$' scripts/check-neutral.sh && scripts/check-neutral.sh >/dev/null

[MUTATION_SPEC]
1. crates/backend_core/src/ingestion/mod.rs line 1061: delete ` || dir_leaf.contains("xanmod")`, so the line reads exactly
       } else if dir_leaf.contains("kernel") || dir_leaf == "kmod" {
   (4 leading spaces, as now). Nothing else in the file changes.
2. crates/backend_core/tests/storage_integration.rs: write this sed script to a temp file (each line starts at `s#`: drop the 7-space indent shown here; otherwise verbatim) outside the repo (e.g. `f="$(mktemp)"`). Note the `\&` on line 3, which keeps the `&` literal. Then run `sed -E -i -f "$f" crates/backend_core/tests/storage_integration.rs` once, and delete the temp file.

       s#https://deb\.xanmod\.org#https://packages.example.com/kernel#g
       s#http://deb\.debian\.org/debian sid#http://packages.example.com/base rolling#g
       s#Debian Sid & XanMod Rolling Release#Orbit Base \& Custom Rolling Release#g
       s#Debian Sid \+ XanMod kernel#a rolling base + custom kernel#g
       s#debian_sid#rolling_base#g
       s#XanMod kernel#custom kernel#g
       s#XanMod#Custom#g
       s#xanmod#custom#g
       s#NovOS#Orbit#g
       s#novos#orbit#g
       s#QEMU#VM#g
       s#qemu#virt#g
3. scripts/check-neutral.sh: delete the two heredoc lines `1 crates/backend_core/src/ingestion/mod.rs` and `105 crates/backend_core/tests/storage_integration.rs`. Nothing else in the script changes.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && test "$(sha256sum crates/backend_core/tests/storage_integration.rs | cut -d' ' -f1)" = "e1ac873748b081adb69d7272a1aac8ce624b6a3a2a86a4b233de70b8d6a2da57" && grep -q '^    } else if dir_leaf.contains("kernel") || dir_leaf == "kmod" {$' crates/backend_core/src/ingestion/mod.rs && test "$(git diff --numstat -- crates/backend_core/src/ingestion/mod.rs | awk '{print $1" "$2}')" = "1 1" && test "$(git diff --numstat -- scripts/check-neutral.sh | awk '{print $1" "$2}')" = "0 2" && scripts/check-neutral.sh | grep -q '^check-neutral: OK (2 allowlisted file(s), 43 line(s) left to scrub)$' && git diff --quiet -- crates/nebulous_cli crates/backend_core/src/lib.rs crates/backend_core/src/mcp crates/backend_core/src/models Cargo.toml Cargo.lock README.md docs .github

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

Decisions the contract leaves open:
- **Why `sed` and a hash, not a line list.** 105 lines across about 20 substitution patterns would be a long list, and easy to get wrong. A fixed script plus the hash of the result is exact and short. If the hash doesn't match, Gemini stops and reports. It doesn't hand-fix.
- **Not tokens, so left alone:** `Sid`/`sid`, `live-build`, `dracut`, `initramfs`, `apt_sid`.
  - `Sid`/`sid` in `rolling-sid`, `01_sid_base` and `tags = ["…", "sid"]`.
  - `live-build` in a filename.
  - `apt_sid`, which is the open question about `default_rolling_dep_kind`, logged under Phase 11 Steps.
- **The `kmod` and `kernel` rules stay.** They're generic Linux words, not a distro.

---

### STEP 29H: CLI Strings and Test Data Neutral (Allowlist Empty)

* **Recommended Command:** `/goal`

Scope note (2026-11-11). This is the last scrub step of Phase 11 (Separation). The ratchet (`c68bf2a`) allowlists 43 lines in the CLI crate:
- **`crates/nebulous_cli/src/lib.rs`, 2 lines.** Clap parse tests at lines 223 and 252: `/tmp/novos`, `"NovOS"` and `novos_iso`.
- **`crates/nebulous_cli/tests/cli_integration.rs`, 41 lines.** The project-registry test (`novos_alpha`, `NovOS Alpha`), the git signature `NovOS Bot <bot@novos.dev>`, the component pin/rollback test (XanMod packages, `kernel_xanmod`, the QEMU reasons), and the MCP JSON-RPC fixtures (`deb.xanmod.org`, `qemu-system-x86_64 -cdrom novos.iso`, `--squashfs-zstd`).

Same method as 29G: a fixed sed script applied to both files, with the results pinned by sha256. I've dry-run it on copies:

| File | Hits left | Lines changed | Line count | rustfmt |
| :--- | ---: | ---: | :--- | :--- |
| `lib.rs` | 0 | 2 | 311 (unchanged) | clean |
| `cli_integration.rs` | 0 | 41 | 2403 (unchanged) | clean |

The words match 29E–29G:
- `novos` becomes `orbit` and `NovOS` becomes `Orbit`.
- `xanmod` becomes `custom`. `XanMod kernel` becomes `custom kernel`, and a bare `XanMod` becomes `Kernel` (same length, so "Restore Kernel 6.12.4-1 pin").
- `QEMU` becomes `VM` and `qemu` becomes `virt`.
- `squashfs` becomes `rootfs`.
- The test hosts `deb.xanmod.org` and `bot@novos.dev` become `packages.example.com/kernel` and `bot@example.com`.

Every assertion changes together with the value it checks.

After this step the allowlist is empty. `check-neutral.sh` then enforces zero across every tracked file outside `archive/`. That finishes Phase 11 on the Nebulous side.

```text
[STEP_ID]
STEP 29H: CLI strings and test data neutral via a fixed sed script; check-neutral allowlist empty

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/lib.rs
  - crates/nebulous_cli/tests/cli_integration.rs
  - scripts/check-neutral.sh
READ_ONLY_REFS:
  - `grep -niIE 'novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu' crates/nebulous_cli/src/lib.rs crates/nebulous_cli/tests/cli_integration.rs` (the 43 lines)
  - scripts/check-neutral.sh (the allowlist heredoc, now 2 lines)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Don't hand-edit lib.rs or cli_integration.rs: apply only the sed script below, then change nothing else (don't run `cargo fmt` on them; they're already formatted). Don't add the sed script file to the repo. No commits (Claude commits).

[PRE_GATE]
P='novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu'; test "$(grep -ciIE "$P" crates/nebulous_cli/src/lib.rs)" -eq 2 && test "$(grep -ciIE "$P" crates/nebulous_cli/tests/cli_integration.rs)" -eq 41 && test "$(wc -l < crates/nebulous_cli/src/lib.rs)" -eq 311 && test "$(wc -l < crates/nebulous_cli/tests/cli_integration.rs)" -eq 2403 && ! grep -qi 'orbit\|virt' crates/nebulous_cli/src/lib.rs crates/nebulous_cli/tests/cli_integration.rs && test "$(sed -n "/^done <<'EOF'$/,/^EOF$/p" scripts/check-neutral.sh | wc -l)" -eq 4 && grep -q '^2 crates/nebulous_cli/src/lib.rs$' scripts/check-neutral.sh && grep -q '^41 crates/nebulous_cli/tests/cli_integration.rs$' scripts/check-neutral.sh && scripts/check-neutral.sh >/dev/null

[MUTATION_SPEC]
1. Write this sed script to a temp file outside the repo (e.g. `f="$(mktemp)"`). Each line starts at `s#`: drop the 7-space indent shown here, and copy it otherwise verbatim. Run `sed -E -i -f "$f" crates/nebulous_cli/src/lib.rs crates/nebulous_cli/tests/cli_integration.rs` once, then delete the temp file.

       s#https://deb\.xanmod\.org#https://packages.example.com/kernel#g
       s#bot@novos\.dev#bot@example.com#g
       s#XanMod kernel#custom kernel#g
       s#XanMod#Kernel#g
       s#xanmod#custom#g
       s#squashfs#rootfs#g
       s#NovOS#Orbit#g
       s#novos#orbit#g
       s#QEMU#VM#g
       s#qemu#virt#g
2. scripts/check-neutral.sh: delete the two heredoc lines `2 crates/nebulous_cli/src/lib.rs` and `41 crates/nebulous_cli/tests/cli_integration.rs`. The heredoc is then empty: `done <<'EOF'` is followed directly by `EOF`. Keep both of those lines, and the `while read` loop above them. Nothing else in the script changes.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && test "$(sha256sum crates/nebulous_cli/src/lib.rs | cut -d' ' -f1)" = "8865b9d17b71cb87edabcf6531c4b069822e01eee17ca11cbda63efed3856c98" && test "$(sha256sum crates/nebulous_cli/tests/cli_integration.rs | cut -d' ' -f1)" = "31fea69f860ac85f86dc0e30724c363c883cc3f89118c72556e98a3ac6c5d601" && test "$(git diff --numstat -- scripts/check-neutral.sh | awk '{print $1" "$2}')" = "0 2" && scripts/check-neutral.sh | grep -q '^check-neutral: OK (0 allowlisted file(s), 0 line(s) left to scrub)$' && test -z "$(git ls-files | grep -vE '^archive/|^Cargo\.lock$|^scripts/check-neutral\.sh$' | xargs grep -liIE 'novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu')" && t="$(mktemp)" && printf 'see xorriso\n' > "$t" && ! scripts/check-neutral.sh "$t" 2>/dev/null && rm -f "$t" && git diff --quiet -- crates/backend_core crates/nebulous_cli/src/args.rs crates/nebulous_cli/src/commands Cargo.toml Cargo.lock README.md docs .github

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

Decisions the contract leaves open:
- **`bot@example.com` instead of a `.test` address.** It's already the convention in this file: the test at line 651 uses `test@example.com`.
- **`orbit_iso`, not `orbit_image`, in `lib.rs`.** It's a clap parse test of a free-form id, so a mechanical rename is enough. The USAGE doc's `orbit_image` is prose for readers, and it doesn't have to match.
- **Not tokens, so left alone:** `sid` (in the tags `distro,sid,custom`), `initramfs`, `-cdrom`.

---

# Phase 5–7 Completion — Re-split of STEP 16–22 (written 2026-11-15)

**Phase goal (frozen, user 2026-11-15):** every remaining Up Next row runs in this one phase. Rows over the step threshold are split into sub-steps with the same scope. No new behaviour beyond the original STEP 16–22 rows. STEP 20 is dropped (user).

**Written against HEAD `38744c8`.** READ_ONLY_REFS name functions and types first; line numbers are as of `38744c8` and drift as earlier steps land (14G–14I touch `mcp/mod.rs` and `agent_context/mod.rs`; sub-steps touch each other's files). Find code by the named item, not by line.

**Gate used in every POST_GATE below** (the validator needs it written out on one line each time):
`cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh`

**Where the sub-steps differ from the original rows** (one list, so you can check it once):
- **16:** the "per-operation snapshot" is a write-lock-scoped `project.toml` cache (16A) instead of threading an `IndexSnapshot` parameter through every storage API. Same effect for `create_item` (the row's own test: ≤2 reads); far smaller diff. Read-only paths outside a lock still re-read.
- **16B item 3 dropped (2026-11-15):** untracked-dir recursion stays on; the ranking needs per-file counts (the row's own "unless needed").
- **16:** "never [run git status] from error paths": `resolve_active_or_inferred_item_id` already calls `compute_whats_next` at most once per call (two exclusive branches). Removing it would change error text, so it's left alone (Parked).
- **17:** split four ways (17A watcher, 17B reconcile, 17C echo hashing, 17D TUI).
- **18:** the session key is `<item_id>@<started_at>`, not a new random `session_id`; `started_at` is already unique per session. "Warn" on corrupt/orphan timer = keep a `.timer.toml.bak`; `backend_core` has no logging channel, so no printed warning (Parked). Over-cap sessions are capped with a warning; no `--force` flag (the row said "or").
- **19:** no MCP caller of `start_work_item` exists, so MCP isn't touched.
- **22:** no doc mentions `log_effort`, so nothing to fix there.

---

### STEP 16A: Write-Lock-Scoped `project.toml` Cache

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 16A: While a thread holds the tracker write lock, read_project() parses project.toml once and serves the cached copy; create_item reads it from disk at most twice

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs:25-48 (HELD_WRITE_LOCKS, ReentrantLockGuard), :475-530 (with_write_lock_timeout), :694-709 (read_project, write_project), :1566-1600 (mod lock_tests)
  - crates/backend_core/src/workflow/lifecycle.rs:108-169 (create_item), :170-185 (create_bug)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Public signatures of read_project / write_project / with_write_lock* unchanged. No caching outside a held write lock.

[PRE_GATE]
grep -q 'static HELD_WRITE_LOCKS' crates/backend_core/src/storage/mod.rs && ! grep -q 'PROJECT_CACHE' crates/backend_core/src/storage/mod.rs

[MUTATION_SPEC]
1. Next to HELD_WRITE_LOCKS in the same thread_local! block: `static PROJECT_CACHE: RefCell<HashMap<PathBuf, ProjectManifest>> = RefCell::new(HashMap::new());` with a doc: holds the parsed project.toml, keyed by lock path, only while this thread holds that tracker's write lock. Why: inside the lock no other process can write project.toml, and create_item otherwise re-parses it ~6 times (resolve_phase_and_scope, phase_dir via read_phase/scope_dir/item_dir, allocate_next_id, scan_max_counters).
2. `impl Drop for ReentrantLockGuard`: also remove `self.path` from PROJECT_CACHE.
3. Private helper `fn holds_write_lock(&self) -> bool` = HELD_WRITE_LOCKS contains `self.lock_path()`.
4. read_project: if holds_write_lock() and PROJECT_CACHE has `self.lock_path()`, return a clone. Otherwise run the existing disk read; on success, if holds_write_lock(), insert a clone.
5. write_project: after write_versioned succeeds and holds_write_lock(), insert `normalized` into PROJECT_CACHE. If write_versioned fails, remove the entry, then return the error.
6. `#[cfg(test)] thread_local! { static PROJECT_DISK_READS: Cell<u32> = const { Cell::new(0) }; }`; read_project's disk branch increments it under `#[cfg(test)]`.
7. Tests, in `mod lock_tests` (init with `TrackerStorage::init_standard(dir.path(), "t")`):
   a. `create_item_reads_project_toml_at_most_twice`: reset PROJECT_DISK_READS to 0; `storage.create_bug("x", None, None, Severity::Medium, None)` succeeds; counter ≤ 2.
   b. `project_cache_is_dropped_when_lock_releases`: `with_write_lock(|s| s.read_project())`; then outside the lock, rewrite project.toml on disk with `fs::read_to_string` + replace `name = "t"` → `name = "u"` + `fs::write`; `storage.read_project()?.project.name == "u"`.
   c. `write_inside_lock_updates_cache`: inside one with_write_lock: read, set `project.name = "v"`, write_project, read again → `"v"`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'static PROJECT_CACHE' crates/backend_core/src/storage/mod.rs && grep -q 'fn create_item_reads_project_toml_at_most_twice' crates/backend_core/src/storage/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 16B: One Index Load and One Git Status per What's-Next Report

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 16B: compute_whats_next_at reuses its index for scope inference, and scan_max_counters scans each phase dir once

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/workflow/assist.rs
  - crates/backend_core/src/storage/id.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/workflow/assist.rs:133-210 (infer_scope_from_context), :317-340 (compute_whats_next_at), :866-917 (infer_scope_from_git_dirty_files), :919 (match_file_to_best_scope)
  - crates/backend_core/src/storage/id.rs:37-62 (scan_max_counters), :115-145 (scan_phase_dir)
  - crates/backend_core/src/workflow/lifecycle.rs:664-671 (create_scope signature: phase_id, scope_id, target, tech_stack, description)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Public signatures of infer_scope_from_context, compute_whats_next(_at), scan_max_counters unchanged. Ranking rules unchanged.

[PRE_GATE]
grep -q 'static PROJECT_CACHE' crates/backend_core/src/storage/mod.rs && ! grep -q 'fn infer_scope_with_index' crates/backend_core/src/workflow/assist.rs

[MUTATION_SPEC]
1. assist.rs: move the body of infer_scope_from_context (everything after `let index = self.load_index()?;`) into private `fn infer_scope_with_index(&self, index: &TrackerIndex, cwd: Option<&Path>) -> Result<String>`. infer_scope_from_context becomes `let index = self.load_index()?; self.infer_scope_with_index(&index, cwd)`.
2. compute_whats_next_at: `let inferred_scope = self.infer_scope_with_index(&index, cwd)?;`. Why: it was loading the whole index twice per report.
3. **Dropped 2026-11-15 (spec error, found by Gemini's halt):** `recurse_untracked_dirs(false)` collapses an untracked tree to one `crates/` entry that matches no `crates/x` target, and the "most modified files" ranking needs per-file counts (breaks `test_infer_scope_from_context_all_priority_levels`). The row's "unless needed" applies: recursion stays `true`. infer_scope_from_git_dirty_files is unchanged.
4. id.rs scan_max_counters: keep a `HashSet<PathBuf>` of phase dirs already scanned (key: `fs::canonicalize(&dir).unwrap_or(dir.clone())`); the registered-phase loop inserts, the `phases_dir()` loop skips dirs already in the set. Why: registered phases under `.tracker/phases/` were scanned twice.
5. Tests (workflow_integration.rs):
   a. `whats_next_inferred_scope_matches_infer_scope_from_context`: scope `api` with target `crates/api`; cwd = `<root>/crates/api`; `compute_whats_next(Some(cwd)).inferred_scope == infer_scope_from_context(Some(cwd))? == "api"`.
   b. **Dropped with item 3** (it only passed through the single-scope fallback). Remove the test if it was added.
   c. `scan_max_counters_unchanged_with_registered_phases`: create BUG-0001 and FEAT-0001; `scan_max_counters()?` gives bug 1, feature 1.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'fn infer_scope_with_index' crates/backend_core/src/workflow/assist.rs && grep -q 'recurse_untracked_dirs(true)' crates/backend_core/src/workflow/assist.rs && ! grep -q 'fn untracked_new_dir_still_infers_scope' crates/backend_core/tests/workflow_integration.rs && grep -q 'fn scan_max_counters_unchanged_with_registered_phases' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 17A: Watcher Rescan Trigger, Error Surfacing, Join on Drop

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 17A: notify rescan/Other events and watcher errors trigger a full reconcile, reconcile errors reach TrackerWatchEvent, and dropping TrackerWatcher joins its worker thread

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/watcher/mod.rs
  - crates/backend_core/tests/watcher_integration.rs (NEW)
READ_ONLY_REFS:
  - crates/backend_core/src/watcher/mod.rs:13-165 (TrackerWatchEvent, TrackerWatcher, start_with_options worker loop), :240-246 (is_mutating_event_kind)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. The 50ms debounce, 300ms settlement and 1000ms echo window stay as they are (17C replaces the echo window). start / start_with_options signatures unchanged.

[PRE_GATE]
grep -q 'fn infer_scope_with_index' crates/backend_core/src/workflow/assist.rs && ! grep -q 'reconcile_error' crates/backend_core/src/watcher/mod.rs

[MUTATION_SPEC]
1. TrackerWatchEvent gains `pub rescan: bool` and `pub reconcile_error: Option<String>`.
2. Private `enum RawSignal { Paths(Vec<PathBuf>), Rescan }`; the raw channel carries RawSignal. In the notify callback: `Err(_)` → send Rescan; `event.need_rescan() || matches!(event.kind, EventKind::Other)` → send Rescan; otherwise the existing filtering, sending Paths.
3. Worker: a batch that contains any Rescan sets `rescan = true`, skips echo suppression and settlement, and always reconciles when auto_reconcile; its `changed_paths` is `vec![watched_dir.clone()]` plus any real paths coalesced with it.
4. Replace `storage_for_worker.reconcile_indexes().ok()` with a match: Ok(r) → (Some(r), None); Err(e) → (None, Some(e.to_string())). Why: errors were silently swallowed.
5. TrackerWatcher fields become `watcher: Option<RecommendedWatcher>`, `worker: Option<JoinHandle<()>>`, `watched_dir`. `impl Drop`: take and drop `watcher` first (this drops the callback's raw sender, so the worker's `recv()` errors and the loop exits), then take and `join()` the worker, ignoring the join result.
6. Tests (tests/watcher_integration.rs, NEW; tempdir + `TrackerStorage::init_standard`; wait on `rx.recv_timeout(Duration::from_secs(5))`):
   a. `reconcile_error_is_reported`: start watcher; overwrite project.toml with `not = [valid`; the first event has `reconcile_error.is_some()` and `reconcile_report.is_none()`.
   b. `drop_joins_worker_promptly`: start watcher, drop it; the drop completes in < 2s (measure with Instant).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'pub reconcile_error: Option<String>' crates/backend_core/src/watcher/mod.rs && grep -q 'enum RawSignal' crates/backend_core/src/watcher/mod.rs && grep -q 'impl Drop for TrackerWatcher' crates/backend_core/src/watcher/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 17B: Reconcile Skips and Reports Malformed `item.toml`

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 17B: reconcile_indexes skips an item.toml that fails to parse, lists it in ReconcileReport.skipped_malformed, and still indexes every other item

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/reconcile.rs
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/reconcile.rs:13-22 (ReconcileReport), :221-300 (collect_authoritative_items, UnsupportedSchema arm)
  - crates/backend_core/src/error.rs:21 (TrackerError::TomlDeserialize)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Other error kinds (Io etc.) still propagate. skipped_unsupported keeps its meaning.

[PRE_GATE]
grep -q 'enum RawSignal' crates/backend_core/src/watcher/mod.rs && ! grep -q 'skipped_malformed' crates/backend_core/src/storage/reconcile.rs

[MUTATION_SPEC]
1. ReconcileReport gains `pub skipped_malformed: Vec<String>` (tracker-relative paths, `/`-joined).
2. Private `fn tracker_relative(storage: &TrackerStorage, path: &Path) -> String` holding the strip_prefix/components/join code now inline in the UnsupportedSchema arm; that arm uses it.
3. collect_authoritative_items gains a `malformed: &mut Vec<String>` parameter; new arm `Err(TrackerError::TomlDeserialize { .. }) => { malformed.push(tracker_relative(storage, &item_toml_path)); continue; }`. The caller passes `&mut report.skipped_malformed` (or a local Vec moved into the report, matching how skipped_unsupported is handled).
4. Tests (storage_integration.rs): `reconcile_skips_malformed_item_toml`: create BUG-0001 and BUG-0002; overwrite BUG-0002's item.toml with `not = [valid`; `reconcile_indexes()` is Ok; `skipped_malformed.len() == 1` and it ends with `BUG-0002/item.toml`; `load_index()?.find_item("BUG-0001")` is Some.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'pub skipped_malformed: Vec<String>' crates/backend_core/src/storage/reconcile.rs && grep -q 'fn reconcile_skips_malformed_item_toml' crates/backend_core/tests/storage_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 17C: Echo Suppression by Content Hash

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 17C: The watcher suppresses an index-only batch only when every file's content matches what the last internal reconcile left, replacing the 1000ms time window

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/watcher/mod.rs
  - crates/backend_core/tests/watcher_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/watcher/mod.rs (worker loop step 2 echo check and step 4 drain; is_derived_index_file)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Rescan handling from 17A unchanged.

[PRE_GATE]
grep -q 'pub reconcile_error: Option<String>' crates/backend_core/src/watcher/mod.rs && grep -q 'Duration::from_millis(1000)' crates/backend_core/src/watcher/mod.rs

[MUTATION_SPEC]
1. Private `fn hash_index_files(dir: &Path) -> HashMap<PathBuf, u64>`: recursive read_dir under `dir`; for each file where `is_derived_index_file` is true, hash its bytes with `std::hash::DefaultHasher`. Doc: not a security hash; only detects our own writes.
2. Worker: replace `last_internal_reconcile: Option<Instant>` with `own_index_hashes: HashMap<PathBuf, u64>`, set to `hash_index_files(&watched_dir)` right after each internal reconcile.
3. Echo check: an all-index-file batch is suppressed only if, for every path, the file exists and its current hash equals `own_index_hashes[path]`. Remove the 1000ms window. Why: an external edit to scope.toml right after an internal reconcile was being dropped.
4. Test (watcher_integration.rs): `external_scope_edit_after_reconcile_is_processed`: start watcher; create BUG-0001 via storage; wait for its event; within 200ms rewrite the inbox scope.toml with an extra comment line `# external`; a second event arrives within 5s with `reconcile_report.is_some()`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'fn hash_index_files' crates/backend_core/src/watcher/mod.rs && ! grep -q 'Duration::from_millis(1000)' crates/backend_core/src/watcher/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 17D: TUI Live Reload and Wall-Clock Timer

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 17D: The TUI refreshes on watcher events, shows reconcile errors in the status bar, and renders the active timer's minutes from the wall clock each draw

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/tui/mod.rs
  - crates/nebulous_cli/src/tui/render.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/nebulous_cli/src/tui/mod.rs:68-120 (run_tui loop, 250ms poll)
  - crates/nebulous_cli/src/tui/render.rs:104-110, :186-190 (timer text uses elapsed_minutes)
  - crates/backend_core/src/models/timer.rs:48-60 (parsed_started_at, elapsed_minutes_at)
  - crates/backend_core/src/watcher/mod.rs (TrackerWatcher::start, TrackerWatchEvent)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. tui/state.rs unchanged; TuiApp::refresh is called as is.

[PRE_GATE]
grep -q 'fn hash_index_files' crates/backend_core/src/watcher/mod.rs && ! grep -q 'TrackerWatcher' crates/nebulous_cli/src/tui/mod.rs

[MUTATION_SPEC]
1. run_tui, after TuiApp::new: `let watch = TrackerWatcher::start(app.storage.clone()).ok();` (keep the TrackerWatcher alive for the loop; on Err set `app.status_message = Some("Live reload unavailable: <err>")`).
2. Each loop iteration before draw: drain `rx.try_recv()`; if any event arrived, `app.refresh()` once (Err → status message); if an event has `reconcile_error: Some(e)`, status message `Reconcile error: {e}`.
3. render.rs: private `fn live_minutes(t: &TimerStatus) -> u32 { t.timer.elapsed_minutes_at(Utc::now()).unwrap_or(t.elapsed_minutes) }`; both timer text sites use it. Why: the cached value only changed on refresh.
4. Test (cli_integration.rs, next to the existing TUI tests): `tui_timer_renders_wall_clock_minutes`: build a TuiApp, set `app.active_timer` to a TimerStatus whose `timer.started_at` is 7 minutes ago and `elapsed_minutes: 0`; `render_to_string(&mut app, 120, 40)` contains `7m`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'TrackerWatcher::start' crates/nebulous_cli/src/tui/mod.rs && grep -q 'fn live_minutes' crates/nebulous_cli/src/tui/render.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 18A: Idempotent Timer Stop and Parent Index Sync

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 18A: Stopping a timer records its session key on the item and never logs the same session twice; stop also syncs scope/phase indexes

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/item.rs
  - crates/backend_core/src/workflow/timer.rs
  - crates/backend_core/src/workflow/lifecycle.rs
  - crates/backend_core/tests/workflow_integration.rs
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/item.rs:329-336 (ItemEffort, derives Default)
  - crates/backend_core/src/workflow/timer.rs:121-150 (stop_timer_at)
  - crates/backend_core/src/workflow/lifecycle.rs:136-139 (ItemEffort literal), :965 (sync_parent_indexes_for_item)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. ActiveTimer and .timer.toml format unchanged. StoppedTimerSession fields unchanged.

[PRE_GATE]
grep -q 'fn live_minutes' crates/nebulous_cli/src/tui/render.rs && ! grep -q 'last_timer_session' crates/backend_core/src/models/item.rs

[MUTATION_SPEC]
1. ItemEffort gains `#[serde(default, skip_serializing_if = "Option::is_none")] pub last_timer_session: Option<String>` with a doc: `<item_id>@<started_at>` of the last session logged. The 6 `ItemEffort { … }` literals (lifecycle.rs 1, workflow_integration.rs 4, storage_integration.rs 1) add `last_timer_session: None`.
2. stop_timer_at: `let session_key = format!("{}@{}", timer.item_id, timer.started_at);`. If `detail.manifest.effort.last_timer_session.as_deref() == Some(&session_key)`: don't add minutes, clear the timer, return StoppedTimerSession with `elapsed_minutes: 0`. Why: a crash between writing item.toml and removing .timer.toml made the next stop log the session again.
3. Otherwise: add minutes, set `last_timer_session = Some(session_key)`, write the manifest, then `self.sync_parent_indexes_for_item(&loc.phase_id, &loc.scope_id, &detail.manifest)?`, then clear the timer.
4. Tests (workflow_integration.rs):
   a. `stop_twice_for_same_session_logs_once`: create BUG-0001; `start_timer_at("BUG-0001", t0, None)` → keep the returned timer; `stop_timer_at(t0 + 30m)` → logged 30; `write_active_timer(&timer)` (same started_at); `stop_timer_at(t0 + 30m)` → `elapsed_minutes == 0`, item logged still 30.
   b. `stop_syncs_scope_stub`: after a stop, `load_index()?.find_item("BUG-0001").unwrap().stub == detail.manifest.to_stub()`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'pub last_timer_session: Option<String>' crates/backend_core/src/models/item.rs && grep -q 'fn stop_twice_for_same_session_logs_once' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 18B: Corrupt or Orphaned Timer Is Quarantined

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 18B: An unparseable .timer.toml is moved to .timer.toml.bak and treated as no timer; stopping a timer whose item is gone moves it aside and returns NotFound

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/mod.rs
  - crates/backend_core/src/workflow/timer.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs:839-863 (read_active_timer, write_active_timer, clear_active_timer)
  - crates/backend_core/src/workflow/timer.rs (stop_timer_at, load_item_detail_by_id call)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. A readable timer behaves exactly as before.

[PRE_GATE]
grep -q 'pub last_timer_session: Option<String>' crates/backend_core/src/models/item.rs && ! grep -q 'fn quarantine_active_timer' crates/backend_core/src/storage/mod.rs

[MUTATION_SPEC]
1. storage/mod.rs: `pub(crate) fn quarantine_active_timer(&self) -> Result<PathBuf>`: guarded paths for `.timer.toml` and `.timer.toml.bak`; `fs::rename` (overwrites an old .bak); returns the .bak path.
2. read_active_timer: if `ActiveTimer::from_toml_str` fails, call quarantine_active_timer and return `Ok(None)`. Doc: why, and that the .bak keeps the data for the user. (No printed warning: backend_core has no logging channel; Parked.)
3. timer.rs stop_timer_at: if `load_item_detail_by_id` returns `Err(TrackerError::NotFound { .. })`, call quarantine_active_timer, then return that same error.
4. Tests:
   a. `corrupt_timer_file_does_not_block_reads`: write `garbage = [` to .timer.toml; `compute_whats_next(None)` is Ok; `timer_status()? == None`; `.timer.toml.bak` exists and contains `garbage`.
   b. `orphan_timer_stop_quarantines`: create BUG-0001, start its timer, `fs::remove_dir_all` its item dir, run `reconcile_indexes()`; `stop_timer()` is `Err(NotFound)`; .timer.toml gone, .bak present.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'fn quarantine_active_timer' crates/backend_core/src/storage/mod.rs && grep -q 'fn corrupt_timer_file_does_not_block_reads' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 18C: Session Length Cap

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 18C: metrics.max_session_minutes (default 480) caps the minutes a single stop logs; StoppedTimerSession.capped_from records the real length and `neb timer stop` prints a warning

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/project.rs
  - crates/backend_core/src/workflow/timer.rs
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/project.rs:139-154 (MetricsConfig, Default, default_* fns)
  - crates/backend_core/src/workflow/timer.rs:17-24 (StoppedTimerSession), stop_timer_at
  - crates/nebulous_cli/src/commands/mod.rs:1368-1420 (handle_timer, TimerCommand::Stop output)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No --force flag. 0 means "no cap".

[PRE_GATE]
grep -q 'fn quarantine_active_timer' crates/backend_core/src/storage/mod.rs && ! grep -q 'max_session_minutes' crates/backend_core/src/models/project.rs

[MUTATION_SPEC]
1. MetricsConfig: `#[serde(default = "default_max_session_minutes")] pub max_session_minutes: u32` (fn returns 480; doc: 0 disables). Default impl sets it.
2. StoppedTimerSession: `#[serde(default, skip_serializing_if = "Option::is_none")] pub capped_from: Option<u32>`.
3. stop_timer_at (normal path, before adding minutes): read `self.read_project()?.metrics.max_session_minutes`; if `max > 0 && elapsed > max` → `capped_from = Some(elapsed)`, `elapsed = max`.
4. commands/mod.rs, timer stop text output: when `capped_from` is Some(n), add a line `Warning: session ran {n}m; logged {max}m (metrics.max_session_minutes in project.toml).` JSON output carries the field automatically.
5. Test: `long_session_is_capped`: start at t0, stop at t0 + 600m → `elapsed_minutes == 480`, `capped_from == Some(600)`; with `max_session_minutes = 0` written to project.toml → 600, None.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'pub max_session_minutes: u32' crates/backend_core/src/models/project.rs && grep -q 'pub capped_from: Option<u32>' crates/backend_core/src/workflow/timer.rs && grep -q 'fn long_session_is_capped' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 19A: `start` Ranks With the Caller's CWD and Reports a Switched Timer

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 19A: start_work_item takes the caller's cwd so zero-ID start picks the same item as `neb next`, and returns a StartedWork struct that includes the timer it stopped

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/workflow/lifecycle.rs
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/nebulous_cli/src/tui/state.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/workflow/lifecycle.rs:208-250 (start_work_item)
  - crates/nebulous_cli/src/commands/mod.rs:948-1010 (StartOutputJson, handle_start)
  - crates/nebulous_cli/src/tui/state.rs:1160-1176 (action_start_work)
  - crates/backend_core/tests/workflow_integration.rs:1950-1995 (existing start_work_item test)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Ranking logic in assist.rs unchanged. MCP has no caller; don't add one.

[PRE_GATE]
grep -q 'pub max_session_minutes: u32' crates/backend_core/src/models/project.rs && ! grep -q 'pub struct StartedWork' crates/backend_core/src/workflow/lifecycle.rs

[MUTATION_SPEC]
1. lifecycle.rs: `pub struct StartedWork { pub location: ItemLocation, pub detail: ItemDetail, pub timer: ActiveTimer, pub context: AgentContextBundle, pub stopped_previous: Option<StoppedTimerSession> }` (Debug, Clone), re-exported wherever start_work_item's types are.
2. `pub fn resolve_start_target(&self, item_id: Option<&str>, cwd: Option<&Path>) -> Result<String>`: the existing resolved_id match, with `self.compute_whats_next(cwd)`.
3. `start_work_item(&self, item_id: Option<&str>, note: Option<String>, cwd: Option<&Path>) -> Result<StartedWork>`: uses resolve_start_target; `stopped_previous = Some(self.stop_timer()?)` when it stops a different item's timer.
4. commands/mod.rs handle_start: pass `std::env::current_dir().ok().as_deref()`; when stopped_previous is Some(s), text output starts with `Stopped timer on {s.timer.item_id} ({s.elapsed_minutes}m logged).`; StartOutputJson gains `#[serde(skip_serializing_if = "Option::is_none")] stopped_previous_item: Option<String>`.
5. tui/state.rs action_start_work: pass `self.cwd.as_deref()`; read fields from StartedWork; status message prefixed with `Stopped timer on X. ` when stopped_previous is Some.
6. Tests: update the existing test for the new signature/struct and assert `stopped_previous` item is `BUG-0001` on the switch; new `zero_id_start_matches_next_with_cwd`: scopes `api` (target `crates/api`) and `web` (target `crates/web`), one open bug each at equal severity; cwd = `<root>/crates/web`; `compute_whats_next(Some(cwd))?.top_open_item_id()` equals `start_work_item(None, None, Some(cwd))?.detail.manifest.item.id`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'pub struct StartedWork' crates/backend_core/src/workflow/lifecycle.rs && grep -q 'pub fn resolve_start_target' crates/backend_core/src/workflow/lifecycle.rs && grep -q 'fn zero_id_start_matches_next_with_cwd' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 19B: `neb start --dry-run` and Honest Zero-ID Help Text

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 19B: `neb start --dry-run` prints the item start would pick without changing anything, and zero-ID help text says what each command actually falls back to

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/args.rs
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/nebulous_cli/src/lib.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/nebulous_cli/src/args.rs:158-175 (Commands::Start), :181 (Move doc), :210 (Done doc)
  - crates/backend_core/src/workflow/assist.rs:215-290 (resolve_active_or_inferred_item_id: timer → single in_progress → error with suggestion)
  - crates/backend_core/src/workflow/lifecycle.rs (resolve_start_target from 19A)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No behaviour change to any command except the new flag.

[PRE_GATE]
grep -q 'pub fn resolve_start_target' crates/backend_core/src/workflow/lifecycle.rs && ! grep -q 'dry_run' crates/nebulous_cli/src/args.rs

[MUTATION_SPEC]
1. Commands::Start: `/// Print the item \`neb start\` would pick, without starting a timer or changing status.` `#[arg(long)] dry_run: bool`.
2. handle_start gains `dry_run: bool`; when true, before any write: `resolve_start_target(id, cwd)` → `locate_item` → text `Would start {id} ({phase}/{scope}): {title}`; JSON `{"dry_run": true, "id", "phase_id", "scope_id", "title"}`. No lock taken.
3. Help text: Start `id` doc → "Optional item ID. If omitted: the top open item from `neb next`, else the top recommendation, else the item with the active timer or the single in_progress item." Move (:181) and Done (:210) docs: replace "the active or top inferred item" with "the item with the active timer, or the single in_progress item". Any other `top inferred` in args.rs gets the same wording.
4. Tests: lib.rs `parses_start_dry_run` (`neb start --dry-run` → `dry_run == true`); cli_integration.rs `start_dry_run_changes_nothing`: init, one bug, run `neb start --dry-run` → stdout contains `Would start BUG-0001`; `.tracker/.timer.toml` does not exist; the bug's status is still `open`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'dry_run: bool' crates/nebulous_cli/src/args.rs && ! grep -q 'top inferred' crates/nebulous_cli/src/args.rs && grep -q 'fn start_dry_run_changes_nothing' crates/nebulous_cli/tests/cli_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 19C: Stagnant and Recurring Recommendations Stay in the Active Stage

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 19C: What's Next lists recurring-bug and stagnant-item recommendations only for items in the active stage

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/workflow/assist.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/workflow/assist.rs:488-570 (recurring_open, stagnant filters), active_phase_id in compute_whats_next_at
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Other recommendation kinds and sort orders unchanged.

[PRE_GATE]
grep -q 'dry_run: bool' crates/nebulous_cli/src/args.rs && grep -q 'let mut stagnant' crates/backend_core/src/workflow/assist.rs

[MUTATION_SPEC]
1. recurring_open filter adds `&& c.phase_id == active_phase_id`.
2. stagnant filter adds `&& c.phase_id == active_phase_id`. Why (one comment above both): items in other stages were showing up as "stagnant"/"recurring" work for the current stage.
3. Test `stagnant_and_recurring_are_stage_scoped`: active phase `production`; one open bug in production and one in post_production; `compute_whats_next_at(None, now + 365 days)`: StagnantItem recommendations include the production bug and not the post_production one.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && test "$(grep -c 'c.phase_id == active_phase_id' crates/backend_core/src/workflow/assist.rs)" -ge 2 && grep -q 'fn stagnant_and_recurring_are_stage_scoped' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 21A: TUI Panic Hook Restores the Terminal

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 21A: A panic inside the TUI restores the terminal (raw mode off, alternate screen left) before the panic message prints

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/tui/mod.rs
READ_ONLY_REFS:
  - crates/nebulous_cli/src/tui/mod.rs:60-70 (TerminalCleanupGuard), :68-80 (run_tui setup)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Loop behaviour unchanged.

[PRE_GATE]
grep -q 'fn live_minutes' crates/nebulous_cli/src/tui/render.rs && ! grep -q 'set_hook' crates/nebulous_cli/src/tui/mod.rs

[MUTATION_SPEC]
1. `fn restore_terminal()`: `let _ = disable_raw_mode(); let _ = execute!(io::stdout(), LeaveAlternateScreen);`. TerminalCleanupGuard::drop calls it.
2. run_tui, before enable_raw_mode: `let previous = std::panic::take_hook(); std::panic::set_hook(Box::new(move |info| { restore_terminal(); previous(info); }));`. Doc: why (the panic text was printed into the alternate screen and lost, leaving the shell in raw mode). Both calls are idempotent, so the guard running after the hook is harmless.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'std::panic::set_hook' crates/nebulous_cli/src/tui/mod.rs && grep -q 'fn restore_terminal' crates/nebulous_cli/src/tui/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 21B: TUI Filter Works In Memory

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 21B: Typing in the TUI filter rebuilds the item list from the loaded index without reading disk; disk reload happens on refresh (watcher event, `r`, or actions)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/tui/state.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/nebulous_cli/src/tui/state.rs:209-233 (TuiApp), :278-~460 (refresh), :744-870 (handle_key, filter arms at ~845-860)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Visible list contents for a given index + filter unchanged.

[PRE_GATE]
grep -q 'std::panic::set_hook' crates/nebulous_cli/src/tui/mod.rs && ! grep -q 'fn rebuild_view' crates/nebulous_cli/src/tui/state.rs

[MUTATION_SPEC]
1. TuiApp gains `index: TrackerIndex` (set in new() via the first refresh; initialise from `storage.load_index()?`).
2. Split refresh: `pub fn refresh(&mut self) -> Result<()>` loads index + whats_next + timer (as now), stores `self.index`, then calls `self.rebuild_view()`. `fn rebuild_view(&mut self)` holds everything that builds scopes/items/selection/inspector from `self.index` (no load_index / compute_whats_next inside).
3. The four filter-edit arms (Enter, Esc, Backspace, Char) call `self.rebuild_view()` instead of `self.refresh()?`. Why: every keystroke re-read the whole tracker.
4. Normal mode: if `KeyCode::Char('r')` isn't bound, bind it to `self.refresh()?` with status `Reloaded.`.
5. Test `filter_typing_does_not_reload_disk`: TuiApp with BUG-0001; create BUG-0002 through storage behind its back; press `/` then `B`: items don't contain BUG-0002; press Esc, then `r`: items contain BUG-0002.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'fn rebuild_view' crates/nebulous_cli/src/tui/state.rs && grep -q 'fn filter_typing_does_not_reload_disk' crates/nebulous_cli/tests/cli_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 21C: Clipboard Module With Wayland and OSC 52 Fallback

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 21C: New nebulous_cli::clipboard::copy_text tries arboard (with Wayland data-control) and falls back to OSC 52 on a terminal, returning which path worked or an error

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/clipboard.rs (NEW)
  - crates/nebulous_cli/src/lib.rs
  - crates/nebulous_cli/Cargo.toml
  - Cargo.lock
READ_ONLY_REFS:
  - crates/nebulous_cli/Cargo.toml:17 (arboard = "3")
  - crates/nebulous_cli/src/commands/mod.rs:535-543 (try_copy_to_clipboard, current behaviour)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Callers are switched in 21D, not here. OSC 52 is never written to stdout (it would corrupt --json output).

[PRE_GATE]
grep -q 'fn rebuild_view' crates/nebulous_cli/src/tui/state.rs && ! test -f crates/nebulous_cli/src/clipboard.rs

[MUTATION_SPEC]
1. Cargo.toml: `arboard = { version = "3", features = ["wayland-data-control"] }`; add `base64 = "0.22"` (same version backend_core already uses). cargo deny must stay clean.
2. clipboard.rs: `pub enum CopyMethod { System, Osc52 }`; `pub fn copy_text(text: &str) -> Result<CopyMethod, String>`: arboard set_text → Ok(System); on error, if `std::io::stderr().is_terminal()` write `osc52_sequence(text)` to stderr and flush → Ok(Osc52); else Err(arboard error text).
3. `pub fn osc52_sequence(text: &str) -> String` = `"\x1b]52;c;" + STANDARD base64(text) + "\x07"`.
4. lib.rs: `pub mod clipboard;`.
5. Tests in clipboard.rs: `osc52_sequence("hi") == "\x1b]52;c;aGk=\x07"`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'pub fn copy_text' crates/nebulous_cli/src/clipboard.rs && grep -q 'wayland-data-control' crates/nebulous_cli/Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 21D: Callers Use `copy_text` and Report the Result Honestly

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 21D: CLI and TUI copy through clipboard::copy_text, and the TUI status bar says whether the copy worked instead of always claiming success

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/nebulous_cli/src/tui/state.rs
READ_ONLY_REFS:
  - crates/nebulous_cli/src/commands/mod.rs:535-543 (try_copy_to_clipboard)
  - crates/nebulous_cli/src/tui/state.rs (action_start_work arboard call + "agent context copied" text; action_copy_context)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No direct `arboard::` use remains in either file.

[PRE_GATE]
grep -q 'pub fn copy_text' crates/nebulous_cli/src/clipboard.rs && grep -q 'arboard::' crates/nebulous_cli/src/tui/state.rs

[MUTATION_SPEC]
1. commands/mod.rs try_copy_to_clipboard: body uses `crate::clipboard::copy_text`; Ok(_) → true; Err(e) → existing eprintln warning with `e`, false.
2. tui/state.rs action_start_work: copy via copy_text; status ends with `agent context copied` on Ok(System), `agent context sent via OSC 52` on Ok(Osc52), `clipboard unavailable: <err>` on Err. (Keep 19A's `Stopped timer on X.` prefix.)
3. action_copy_context: same three outcomes in its message.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -q 'arboard::' crates/nebulous_cli/src/tui/state.rs && ! grep -q 'arboard::' crates/nebulous_cli/src/commands/mod.rs && grep -q 'clipboard::copy_text' crates/nebulous_cli/src/tui/state.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 21E: Creating With an Unknown Scope Asks First

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 21E: Submitting the TUI create modal with a scope that doesn't exist in the chosen stage asks for a second Enter before creating the scope

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/tui/state.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/nebulous_cli/src/tui/state.rs (CreateModalState; create-submit block that calls `self.storage.create_scope(Some(&stage_id), &scope_id, &scope_id, Vec::new(), None)` ~line 624)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. `inbox` never needs confirmation. Phase auto-creation for canonical stages unchanged.

[PRE_GATE]
grep -q 'clipboard::copy_text' crates/nebulous_cli/src/tui/state.rs && ! grep -q 'confirm_new_scope' crates/nebulous_cli/src/tui/state.rs

[MUTATION_SPEC]
1. CreateModalState gains `pub confirm_new_scope: Option<String>` (default None in every constructor).
2. Submit: if the scope is unknown and `confirm_new_scope != Some(scope_id)`: set it to Some(scope_id), keep the modal open, status `Scope '<id>' doesn't exist in <stage>. Press Enter again to create it, or edit the scope.`, return. On the second Enter (matching), create the scope and the item as now. Editing the scope field clears confirm_new_scope.
3. Test `unknown_scope_needs_second_enter`: open create modal, title `x`, scope `newscope`; Enter → modal still open, `read_scope(stage, "newscope")` is Err; Enter again → item created, scope exists.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'confirm_new_scope' crates/nebulous_cli/src/tui/state.rs && grep -q 'fn unknown_scope_needs_second_enter' crates/nebulous_cli/tests/cli_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 21F: Stage Switching in the TUI Is View-Only

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 21F: Pressing 1/2/3 (and finishing a create or move) changes only the TUI's viewed stage; project.toml active_phase changes only through `neb stage`

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/tui/state.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/nebulous_cli/src/tui/state.rs (switch_stage ~469; `set_active_phase` calls after create ~669 and move ~729)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. backend set_active_phase unchanged.

[PRE_GATE]
grep -q 'confirm_new_scope' crates/nebulous_cli/src/tui/state.rs && grep -q 'set_active_phase' crates/nebulous_cli/src/tui/state.rs

[MUTATION_SPEC]
1. switch_stage: remove the set_active_phase / create_phase calls; it sets `self.active_stage` and refreshes. Status `Viewing <stage> (active stage unchanged; use \`neb stage\`)`. If the stage has no phase yet, refresh's existing fallback applies.
2. Remove the `let _ = self.storage.set_active_phase(...)` lines after create and after move.
3. Test `tui_stage_keys_do_not_change_active_phase`: project active_phase `production`; TuiApp; press `3` → `app.active_stage == "post_production"`; `read_project()?.project.active_phase == "production"`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -q 'set_active_phase' crates/nebulous_cli/src/tui/state.rs && grep -q 'fn tui_stage_keys_do_not_change_active_phase' crates/nebulous_cli/tests/cli_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 22A: Remove Dead Code

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 22A: Delete unused git helpers, alias types, export_bounded_prompt, with_include_workspace, and the unreachable Commands::Mcp dispatch arm

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/git_sync/mod.rs
  - crates/backend_core/src/agent_context/mod.rs
  - crates/nebulous_cli/src/commands/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/git_sync/mod.rs:138 (with_include_workspace), :204-207 (DocumentedCommitReport/Result aliases), :1069 (fetch_repo), :1139 (push_repo), :1198 (pull_repo)
  - crates/backend_core/src/agent_context/mod.rs:89 (export_bounded_prompt)
  - crates/nebulous_cli/src/commands/mod.rs:25-28 (Mcp early return in execute), :310-314 (Mcp match arm)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Keep every *_with_auth function. Keep the early return in execute().

[PRE_GATE]
! grep -q 'set_active_phase' crates/nebulous_cli/src/tui/state.rs && grep -q 'pub fn fetch_repo(' crates/backend_core/src/git_sync/mod.rs

[MUTATION_SPEC]
1. git_sync/mod.rs: delete `fetch_repo`, `push_repo`, `pull_repo` (the non-`_with_auth` ones), the two type aliases, and `with_include_workspace`. Each has 0 callers at 38744c8; if any caller appears, halt and report instead of deleting.
2. agent_context/mod.rs: delete `export_bounded_prompt` (0 callers).
3. commands/mod.rs: the `Some(Commands::Mcp)` arm in the dispatch match becomes `Some(Commands::Mcp) => unreachable!("neb mcp is dispatched at the top of execute()"),`. Why: execute() returns early for Mcp, so the arm's body never ran.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -qE 'pub fn (fetch|push|pull)_repo\(' crates/backend_core/src/git_sync/mod.rs && ! grep -q 'DocumentedCommitReport' crates/backend_core/src/git_sync/mod.rs && ! grep -q 'fn export_bounded_prompt' crates/backend_core/src/agent_context/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 22B: Docs Match Behaviour

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 22B: README, USAGE and ARCHITECTURE describe zero-ID fallbacks, shipped platforms, TUI keys and live reload as the code now behaves

[SCOPE_LOCK]
ALLOWED_FILES:
  - README.md
  - docs/USAGE.md
  - docs/ARCHITECTURE.md
READ_ONLY_REFS:
  - crates/nebulous_cli/src/args.rs (zero-ID help text from 19B; --dry-run)
  - .github/workflows/release.yml (builds on ubuntu-latest only)
  - crates/nebulous_cli/src/tui/state.rs (handle_key: q quits only outside modals; 1/2/3 view-only; r reload)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Prose only; no new sections beyond what each item below needs.

[PRE_GATE]
! grep -q 'fn export_bounded_prompt' crates/backend_core/src/agent_context/mod.rs

[MUTATION_SPEC]
1. README.md:25 zero-ID bullet: `neb start` falls back to the top `neb next` item; `done`/`show`/`context`/`move` use the active timer or the single in_progress item, else stop and suggest the top item. Mention `neb start --dry-run`.
2. README.md:78 heading/text: prebuilt release binaries are Linux only; macOS/Windows are build-from-source.
3. USAGE.md TUI key table: `q` quits (inside a modal it types text); `Esc` closes the modal or quits; `1`/`2`/`3` change the viewed stage only (`neb stage` changes the active stage); add `r` reload; note the list refreshes on external `.tracker/` changes.
4. USAGE.md timer section: `metrics.max_session_minutes` (default 480, 0 = off) caps one session; a corrupt `.timer.toml` is moved to `.timer.toml.bak`.
5. ARCHITECTURE.md watcher bullet (:71): add that rescan events trigger a full reconcile, malformed item.toml files are skipped and reported, and the TUI subscribes for live reload.

[POST_GATE]
grep -q 'dry-run' README.md && grep -q 'max_session_minutes' docs/USAGE.md && grep -q 'live reload' docs/ARCHITECTURE.md && grep -q '.timer.toml.bak' docs/USAGE.md

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 22C: Portable `install.sh`, Builds as the Invoking User

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 22C: install.sh builds as the invoking user, uses sudo only for the install/uninstall file operations when the target isn't writable, refuses to build as root via sudo, and drops the GNU-only realpath -m

[SCOPE_LOCK]
ALLOWED_FILES:
  - scripts/install.sh
READ_ONLY_REFS:
  - scripts/install.sh:20-30 (abs_path), :95-105 (PREFIX selection), :128-145 (build + install), uninstall branch
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Flags and their meaning unchanged.

[PRE_GATE]
grep -q 'realpath -m' scripts/install.sh

[MUTATION_SPEC]
1. abs_path: keep the existing `/`-prefixed and CALLER_PWD branches; use `realpath -m` only when `realpath -m / >/dev/null 2>&1` succeeds. Why: BSD/macOS realpath has no -m.
2. After PREFIX/BINDIR are set: `SUDO=""`; if `[[ ! -w "$(dirname "${BINDIR}")" && ! -w "${BINDIR}" ]] && [[ "${EUID}" -ne 0 ]]` then `SUDO="sudo"`. All `install -d`, `install -m`, and uninstall `rm` calls are prefixed with `${SUDO}`.
3. Before building: if `[[ "${EUID}" -eq 0 && -n "${SUDO_USER:-}" && "${SKIP_BUILD}" -eq 0 ]]`, print `Run install.sh without sudo; it calls sudo itself for the install step.` to stderr and exit 1. Why: running under sudo built target/ as root.

[POST_GATE]
bash -n scripts/install.sh && { ! command -v shellcheck >/dev/null || shellcheck scripts/install.sh; } && grep -q 'SUDO_USER' scripts/install.sh && grep -q 'realpath -m / ' scripts/install.sh

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

---

# Phase 12 — Phase 5–7 Spec Leftovers (written 2026-11-15)

**Phase goal (frozen; approved by user 2026-11-15):** fix the five Parked items the user picked at the Phase 5–7 review (Parked 5–9). Each restores what an earlier step meant to do; no new behaviour. Parked 1–4 stay parked.

| Step | Parked item | Files |
| :--- | :--- | :--- |
| 30A | 5 — `StartedWork` not re-exported (19A) | `crates/backend_core/src/workflow/mod.rs`, `crates/backend_core/src/lib.rs` |
| 30B | 6 — `execute_to_string(Mcp)` panics (22A) | `crates/nebulous_cli/src/commands/mod.rs`, `crates/nebulous_cli/src/lib.rs` |
| 30C | 7, 8, 9 — install.sh root-owned `~/.local/bin` regression (22C); README sudo line (22C); ARCHITECTURE stale `export_bounded_prompt` (22B) | `scripts/install.sh`, `README.md`, `docs/ARCHITECTURE.md` |

### STEP 30A: Re-export StartedWork

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 30A: StartedWork is re-exported next to the other lifecycle types in workflow/mod.rs and the backend_core crate root

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/workflow/mod.rs
  - crates/backend_core/src/lib.rs
READ_ONLY_REFS:
  - crates/backend_core/src/workflow/mod.rs:7 (pub use lifecycle::{...})
  - crates/backend_core/src/lib.rs:22-27 (pub use workflow::{...})
  - crates/backend_core/src/workflow/lifecycle.rs (pub struct StartedWork, from 19A)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No other names added or removed.

[PRE_GATE]
grep -q 'pub struct StartedWork' crates/backend_core/src/workflow/lifecycle.rs && ! grep -q 'StartedWork' crates/backend_core/src/workflow/mod.rs

[MUTATION_SPEC]
1. workflow/mod.rs:7: `pub use lifecycle::{format_boilerplate_description, CreateItemInput, StartedWork};`
2. lib.rs `pub use workflow::{...}`: add `StartedWork` to the list (let rustfmt place it).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'StartedWork' crates/backend_core/src/workflow/mod.rs && grep -q 'StartedWork' crates/backend_core/src/lib.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 30B: execute_to_string Returns an Error for Mcp

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 30B: execute_to_string returns a ValidationError for Commands::Mcp instead of panicking

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/nebulous_cli/src/lib.rs
READ_ONLY_REFS:
  - crates/nebulous_cli/src/commands/mod.rs:24-28 (execute() early return for Mcp), :322 (Mcp arm, `unreachable!` since 22A)
  - crates/nebulous_cli/src/lib.rs:10 (pub use commands::{execute, execute_to_string, ...}), :14 (mod tests)
  - crates/backend_core/src/error.rs:82-83 (TrackerError::ValidationError(String))
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. execute()'s early return unchanged; `neb mcp` still runs the server.

[PRE_GATE]
grep -q 'unreachable!("neb mcp' crates/nebulous_cli/src/commands/mod.rs

[MUTATION_SPEC]
1. commands/mod.rs Mcp arm: `Some(Commands::Mcp) => Err(TrackerError::ValidationError("neb mcp runs a stdio server; call execute(), not execute_to_string()".to_string())),`. One comment above it: why (execute_to_string is pub and re-exported, so a library caller must get an error, not a panic).
2. lib.rs mod tests: `execute_to_string_mcp_is_error`: `Cli::try_parse_from(["neb", "mcp"])`, then `assert!(matches!(execute_to_string(&cli), Err(TrackerError::ValidationError(_))))`. Import `backend_core::TrackerError` in the test module if it isn't in scope.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -q 'unreachable!("neb mcp' crates/nebulous_cli/src/commands/mod.rs && grep -q 'fn execute_to_string_mcp_is_error' crates/nebulous_cli/src/lib.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 30C: install.sh Sudo Check Uses Nearest Existing Directory; Docs Fixed

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 30C: install.sh decides on sudo from the nearest existing ancestor of BINDIR (no root-owned ~/.local/bin), README drops `sudo` from the --system example, ARCHITECTURE drops the deleted export_bounded_prompt

[SCOPE_LOCK]
ALLOWED_FILES:
  - scripts/install.sh
  - README.md
  - docs/ARCHITECTURE.md
READ_ONLY_REFS:
  - scripts/install.sh:20-30 (abs_path), :110-113 (SUDO check from 22C), :135-139 (refuse build under sudo)
  - README.md:66-76 (install examples)
  - docs/ARCHITECTURE.md:68 (agent_context bullet)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Flags, PREFIX selection, and the build-under-sudo refusal unchanged. No other doc edits.

[PRE_GATE]
grep -qF '! -w "$(dirname "${BINDIR}")"' scripts/install.sh && grep -q 'export_bounded_prompt' docs/ARCHITECTURE.md

[MUTATION_SPEC]
1. install.sh, directly below abs_path: `nearest_existing_dir() {` / `local d="$1"` / `while [[ ! -e "${d}" && "${d}" != "/" ]]; do d="$(dirname "${d}")"; done` / `echo "${d}"` / `}` (closing brace at column 0).
2. install.sh SUDO check becomes `if [[ ! -w "$(nearest_existing_dir "${BINDIR}")" && "${EUID}" -ne 0 ]]; then`. One comment: why (with no ~/.local yet, the old check used sudo and created a root-owned ~/.local/bin).
3. README.md:71-72: comment `# Or install system-wide to /usr/local/bin (asks for sudo only for the copy)`; command `./scripts/install.sh --system`.
4. docs/ARCHITECTURE.md:68: remove `, \`export_bounded_prompt\`` from the agent_context list.

[POST_GATE]
bash -n scripts/install.sh && { ! command -v shellcheck >/dev/null || shellcheck scripts/install.sh; } && grep -q 'nearest_existing_dir "${BINDIR}"' scripts/install.sh && ( t="$(mktemp -d)" && source <(sed -n '/^nearest_existing_dir()/,/^}/p' scripts/install.sh) && [[ "$(nearest_existing_dir "${t}/.local/bin")" == "${t}" ]] ) && ! grep -qF 'sudo ./scripts/install.sh' README.md && ! grep -q 'export_bounded_prompt' docs/ARCHITECTURE.md

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

# Phase 13 — Production Must-Fix (written 2026-11-15)

**Phase goal (frozen; approved by user 2026-11-15):** fix the three must-fix items from the 2026-11-15 audit. (1) Ingestion and component writes take the tracker write lock. (2) Every path that stops a timer goes through `stop_timer`, so the session cap and 18A's no-double-log apply, and the CLI and TUI say when the cap applied. (3) The CLI and TUI say when a timer was quarantined to `.timer.toml.bak`. Nothing else. MCP output is unchanged (reporting the cap over MCP goes to Parked).

| Step | Fixes | Files |
| :--- | :--- | :--- |
| 31A | Audit 1: unlocked project.toml / component read-modify-write | `crates/backend_core/src/ingestion/mod.rs`, `crates/backend_core/tests/storage_integration.rs` |
| 31B | Audit 2 (backend): close and status paths use `stop_timer`; `StatusChange` reports the stopped session | `crates/backend_core/src/workflow/lifecycle.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| 31C | Re-export `StatusChange`; `quarantined_timer_path()` for audit 3 | `crates/backend_core/src/workflow/mod.rs`, `crates/backend_core/src/lib.rs`, `crates/backend_core/src/storage/mod.rs` |
| 31D | Audit 2 + 3 (output): CLI and TUI print the cap warning and the quarantine note | `crates/nebulous_cli/src/commands/mod.rs`, `crates/nebulous_cli/src/tui/state.rs`, `crates/nebulous_cli/tests/cli_integration.rs` |

### STEP 31A: Ingestion and Component Writes Take the Tracker Lock

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 31A: Every public ingestion/component function that writes runs inside with_write_lock, so a concurrent CLI/MCP/TUI writer can't lose its project.toml or component manifest change

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/ingestion/mod.rs
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/ingestion/mod.rs:117 (ingest_project_directory), :592 (write_component_manifest: read_project → upsert → write_project, no lock), :633 (promote_component_to_scope), :754 (pin_component_release), :777 (record_component_rolling_update), :805 (record_component_verification), :834 (rollback_component_to_pinned)
  - crates/backend_core/src/storage/mod.rs:497-540 (with_write_lock: thread-reentrant via HELD_WRITE_LOCKS, DEFAULT_LOCK_TIMEOUT 30s)
  - crates/backend_core/tests/storage_integration.rs:1227 (ComponentManifest::new usage), :2207 (thread::spawn test pattern)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No behaviour change besides taking the lock. Read-only functions (component_manifest_path, read_component_manifest) unchanged.

[PRE_GATE]
grep -q 'pub fn write_component_manifest' crates/backend_core/src/ingestion/mod.rs && test "$(grep -c 'with_write_lock' crates/backend_core/src/ingestion/mod.rs)" -eq 0

[MUTATION_SPEC]
1. Wrap the whole body of each of these 7 functions in `self.with_write_lock(|_| { ... })`: ingest_project_directory, write_component_manifest, promote_component_to_scope, pin_component_release, record_component_rolling_update, record_component_verification, rollback_component_to_pinned. Signatures unchanged. Nested calls are fine (the lock is reentrant).
2. One doc line on write_component_manifest: why (it reads project.toml, upserts the component, and writes it back; without the lock a concurrent writer's change is lost).
3. Test `component_write_waits_for_tracker_lock` in storage_integration.rs: init a tracker; clone the storage into a thread that calls `with_write_lock(|_| { tx.send(()).unwrap(); std::thread::sleep(Duration::from_millis(400)); Ok(()) })`; main thread waits on `rx`, starts an `Instant`, calls `write_component_manifest(&ComponentManifest::new(...))`; assert it succeeded and elapsed >= 300ms; join the thread.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && test "$(grep -c 'with_write_lock' crates/backend_core/src/ingestion/mod.rs)" -ge 7 && grep -q 'fn component_write_waits_for_tracker_lock' crates/backend_core/tests/storage_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 31B: Close and Status Changes Stop Timers Through stop_timer

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 31B: close_item and update_item_status stop timers through stop_timer (session cap + no double-log), and new *_reporting variants return the stopped session

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/workflow/lifecycle.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/workflow/lifecycle.rs:296-320 (update_item_status: `let _ = self.stop_timer()?` on InProgress switch; inline elapsed add + clear_active_timer on completion), :465-492 (close_item: same inline add)
  - crates/backend_core/src/workflow/timer.rs (stop_timer / stop_timer_at: cap via metrics.max_session_minutes, session key, writes item + syncs indexes; StoppedTimerSession.capped_from)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. update_item_status and close_item keep their signatures and return the same ItemDetail. MCP, CLI and TUI callers unchanged in this step.

[PRE_GATE]
grep -q 'pub fn close_item(' crates/backend_core/src/workflow/lifecycle.rs && ! grep -q 'pub struct StatusChange' crates/backend_core/src/workflow/lifecycle.rs

[MUTATION_SPEC]
1. lifecycle.rs: `#[derive(Debug, Clone)] pub struct StatusChange { pub detail: ItemDetail, pub stopped_timer: Option<StoppedTimerSession> }` with a doc line.
2. `pub fn update_item_status_reporting(&self, item_id: &str, new_status: ItemStatus) -> Result<StatusChange>` holds the current body. Changes inside it: (a) the InProgress switch stores `stopped_timer = Some(self.stop_timer()?)`; (b) on a completed status, BEFORE loading the detail: if the active timer's item_id equals `self.locate_item(&normalized_id)?.stub.id`, `stopped_timer = Some(self.stop_timer()?)`; then load the detail as now. Delete the inline `elapsed_minutes` add and its `clear_active_timer` call. `update_item_status` becomes `self.update_item_status_reporting(item_id, new_status).map(|c| c.detail)`.
3. `pub fn close_item_reporting(&self, item_id, summary, commit_override, merged) -> Result<StatusChange>` (same params as close_item) holds close_item's body, with the same change as 2(b): stop through stop_timer before loading the detail; delete the inline add. `close_item` delegates and returns `.detail`.
4. Why (one comment at 2(b)): the inline add skipped the session cap and the no-double-log key.
5. Tests:
   a. `close_applies_session_cap`: create BUG-0001; `start_timer_at("BUG-0001", Utc::now() - Duration::minutes(600), None)`; `close_item_reporting(.., "fixed", None, false)` → `stopped_timer.unwrap().capped_from.is_some()`, `detail.manifest.effort.logged_time_minutes == 480`, `.timer.toml` gone.
   b. `status_switch_reports_stopped_timer`: create BUG-0001 and BUG-0002; start BUG-0001's timer 600 minutes ago; `update_item_status_reporting("BUG-0002", InProgress)` → `stopped_timer` item_id is BUG-0001 and `capped_from.is_some()`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'pub struct StatusChange' crates/backend_core/src/workflow/lifecycle.rs && grep -q 'pub fn close_item_reporting' crates/backend_core/src/workflow/lifecycle.rs && ! grep -q 'elapsed_minutes_at(Utc::now())' crates/backend_core/src/workflow/lifecycle.rs && grep -q 'fn close_applies_session_cap' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 31C: Re-export StatusChange; quarantined_timer_path

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 31C: StatusChange is re-exported like StartedWork, and TrackerStorage::quarantined_timer_path() reports a .timer.toml.bak left by a quarantine

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/workflow/mod.rs
  - crates/backend_core/src/lib.rs
  - crates/backend_core/src/storage/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/workflow/mod.rs:7 (pub use lifecycle::{..., StartedWork})
  - crates/backend_core/src/lib.rs:22-27 (pub use workflow::{...}), :30 (mod tests)
  - crates/backend_core/src/storage/mod.rs:892-920 (quarantine_active_timer, read_active_timer)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Quarantine behaviour from 18B unchanged.

[PRE_GATE]
grep -q 'pub struct StatusChange' crates/backend_core/src/workflow/lifecycle.rs && ! grep -q 'fn quarantined_timer_path' crates/backend_core/src/storage/mod.rs

[MUTATION_SPEC]
1. workflow/mod.rs and lib.rs: add `StatusChange` next to `StartedWork` in both `pub use` lists.
2. storage/mod.rs, below quarantine_active_timer: `pub fn quarantined_timer_path(&self) -> Option<PathBuf>`: `Some(path)` when `.tracker/.timer.toml.bak` is a file, else `None`. Doc: it stays Some until the user deletes the file, so callers can keep telling them.
3. Test in lib.rs `mod tests`: `quarantined_timer_path_reports_bak`: init a temp tracker; `quarantined_timer_path()` is None; write `garbage = [` to `.timer.toml`; `read_active_timer()` is `Ok(None)`; `quarantined_timer_path()` is Some and the file exists.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'StatusChange' crates/backend_core/src/workflow/mod.rs && grep -q 'StatusChange' crates/backend_core/src/lib.rs && grep -q 'pub fn quarantined_timer_path' crates/backend_core/src/storage/mod.rs && grep -q 'fn quarantined_timer_path_reports_bak' crates/backend_core/src/lib.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 31D: CLI and TUI Report the Cap and the Quarantine

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 31D: neb done / neb status warn when the session cap applied, neb next / start / timer status note a quarantined timer, and the TUI shows both

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/nebulous_cli/src/commands/mod.rs
  - crates/nebulous_cli/src/tui/state.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/nebulous_cli/src/commands/mod.rs:1067 (handle_next), :985 (handle_start), :1211 (handle_status → update_item_status), :1253-1290 (handle_close → close_item, two call sites), :1483 (TimerCommand::Status), the 18C stop warning text in handle_timer
  - crates/nebulous_cli/src/tui/state.rs:718-740 (close modal submit → close_item, status_message), TuiApp::new
  - backend_core: update_item_status_reporting, close_item_reporting, StatusChange.stopped_timer (31B/31C), quarantined_timer_path (31C)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. JSON output structs unchanged. Notes and warnings go to stderr, never stdout. MCP unchanged.

[PRE_GATE]
grep -q 'pub fn quarantined_timer_path' crates/backend_core/src/storage/mod.rs && ! grep -q 'close_item_reporting' crates/nebulous_cli/src/commands/mod.rs

[MUTATION_SPEC]
1. commands/mod.rs helper `fn warn_if_capped(stopped: Option<&StoppedTimerSession>)`: when `capped_from` is Some(n), `eprintln!("Warning: {item} session ran {n}m; logged {logged}m (metrics.max_session_minutes in project.toml).")` with logged = `elapsed_minutes`.
2. handle_status uses `update_item_status_reporting`; both handle_close call sites use `close_item_reporting`; each calls warn_if_capped on `stopped_timer`, then uses `.detail` as before.
3. Helper `fn note_quarantined_timer(storage: &TrackerStorage)`: when `quarantined_timer_path()` is Some(p), `eprintln!("Note: a corrupt or orphaned timer was moved to {p}. Check it, log any lost time with `neb log`, then delete the file.")` (p shown relative to the project root). Called at the end of handle_next, handle_start, and TimerCommand::Status, before returning.
4. tui/state.rs: the close modal uses `close_item_reporting`; when capped, append ` Session capped: ran {n}m, logged {logged}m.` to its status message. TuiApp::new: when `quarantined_timer_path()` is Some, set the initial status_message to `Timer quarantined to .tracker/.timer.toml.bak — check it, then delete it.`
5. Test `quarantined_timer_note_on_next` in cli_integration.rs: init a temp tracker; write `garbage = [` to `.tracker/.timer.toml`; run `neb --root <dir> next` via `run_neb` (returns Output); it succeeds, stderr contains `.timer.toml.bak`, stdout does not.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'close_item_reporting' crates/nebulous_cli/src/commands/mod.rs && grep -q 'update_item_status_reporting' crates/nebulous_cli/src/commands/mod.rs && grep -q 'quarantined_timer_path' crates/nebulous_cli/src/tui/state.rs && grep -q 'fn quarantined_timer_note_on_next' crates/nebulous_cli/tests/cli_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```


# Phase 14 — Parked Review Picks: Vault Lock, MCP Cap, Windows (written 2026-11-16)

**Phase goal (frozen; approved by user 2026-11-16):** fix Parked items 3, 4, 5 and 8 from the Phase 13 review. (5) The vault's read-modify-write and vault.key creation take a cross-process file lock. (8) MCP `update_item_status` / `close_item_with_fix` report a stopped (and capped) timer. (4) The Windows vault-permissions behaviour is documented honestly. (3) CI builds and tests on Linux, macOS and Windows. Nothing else.

| Step | Fixes | Files |
| :--- | :--- | :--- |
| 32A | Parked 5: `vault_lock` is in-process only; two `neb` processes can lose a vault entry or create two different vault.key files | `crates/backend_core/src/crypto/mod.rs` |
| 32B | Parked 8: MCP status/close don't report the stopped timer or `capped_from` | `crates/backend_core/src/mcp/mod.rs`, `crates/backend_core/tests/workflow_integration.rs` |
| 32C | Parked 4: `set_secure_file_permissions` is a silent no-op on Windows — document what protects the files there (inherited `%APPDATA%` ACLs) and the gap (a `NEBULOUS_CONFIG_DIR` override outside the profile). Docs-only: real ACL code waits until Windows CI is green | `crates/backend_core/src/crypto/mod.rs`, `docs/ARCHITECTURE.md` |
| 32D | Parked 3: CI matrix over ubuntu/macos/windows | `.github/workflows/ci.yml` |
| [USER] CI | Push and report the matrix results | — |

### STEP 32A: Vault Read-Modify-Write Takes a Cross-Process File Lock

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 32A: The in-process vault Mutex is replaced by an fd-lock on <config_dir>/vault.lock, so two neb processes can't lose a vault entry or race vault.key creation

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/crypto/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/crypto/mod.rs:6 (use std::sync::{Mutex, OnceLock}), :48-52 (VAULT_FILE_LOCK, vault_lock), :431-474 (list_vault_keys, write_vault_entry, read_vault_entry, remove_vault_entry: each takes vault_lock), :523-569 (load_or_create_vault_master_key: takes vault_lock after the env-var early return), :287 (vault_only)
  - crates/backend_core/src/storage/mod.rs:503-545 (with_write_lock_timeout: fd_lock::RwLock::new(file), try_write loop, 50ms sleep, TrackerError::LockTimeout { path, waited_ms }), :26 (DEFAULT_LOCK_TIMEOUT)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Public SecureStorage API and vault file format unchanged. No new dependencies (fd-lock 4 is already a backend_core dependency).

[PRE_GATE]
grep -q 'fn vault_lock' crates/backend_core/src/crypto/mod.rs && ! grep -q 'fn with_vault_lock' crates/backend_core/src/crypto/mod.rs

[MUTATION_SPEC]
1. crypto/mod.rs: `const VAULT_LOCK_FILE_NAME: &str = "vault.lock";` next to the other file-name consts.
2. New `fn with_vault_lock<T>(config_dir: &Path, f: impl FnOnce() -> Result<T>) -> Result<T>`: `ensure_secure_dir(config_dir)?`; open `config_dir.join(VAULT_LOCK_FILE_NAME)` with `create(true).truncate(false).write(true)` (map errors to TrackerError::Io); `fd_lock::RwLock::new(file)`; the same `try_write` loop as with_write_lock_timeout (WouldBlock → sleep 50ms, give up after `crate::storage::DEFAULT_LOCK_TIMEOUT` with `TrackerError::LockTimeout { path, waited_ms }`; other errors → TrackerError::Io); then run `f()` while the guard is held. Doc line: why (the vault is a read-modify-write of one encrypted file; an in-process Mutex doesn't stop a second neb process).
3. list_vault_keys, write_vault_entry, read_vault_entry, remove_vault_entry: replace the `vault_lock().lock()` guard with `with_vault_lock(&self.config_dir, || { ...existing body... })`.
4. load_or_create_vault_master_key: keep the env-var early return outside the lock; replace the vault_lock guard with `with_vault_lock(parent, || { ...rest of body... })` where `parent = key_path.parent()` (if None, return TrackerError::ValidationError("vault.key path has no parent directory")). Why: two processes creating vault.key at once would each write a different key, and one process's vault becomes undecryptable.
5. Delete VAULT_FILE_LOCK, vault_lock(), and the now-unused Mutex/OnceLock imports.
6. Test: add `#[cfg(test)] mod tests` at the end of crypto/mod.rs with `vault_write_waits_for_vault_file_lock`: `let dir = tempfile::tempdir().unwrap();` `let storage = SecureStorage::vault_only(dir.path()).unwrap();` (create it BEFORE the thread). Spawn a thread that calls `with_vault_lock(&path, || { tx.send(()).unwrap(); std::thread::sleep(Duration::from_millis(400)); Ok(()) })`; main waits on rx, starts an Instant, calls `storage.store_secret("testuser", "s3cret")`; assert Ok, elapsed >= 300ms, `storage.get_secret_string("testuser").unwrap() == "s3cret"`, and `dir.path().join("vault.lock").is_file()`; join the thread. Why this proves the file lock: with the Mutex gone, only the fd-lock can make the second call wait.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'fn with_vault_lock' crates/backend_core/src/crypto/mod.rs && ! grep -q 'fn vault_lock' crates/backend_core/src/crypto/mod.rs && grep -q 'fn vault_write_waits_for_vault_file_lock' crates/backend_core/src/crypto/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 32B: MCP Status and Close Report the Stopped Timer

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 32B: MCP update_item_status and close_item_with_fix call the *_reporting functions and add a stopped_timer object (item_id, logged_minutes, capped_from) to their JSON

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:1339-1365 (tool_update_item_status), :1367-1398 (tool_close_item_with_fix), :885 (pub fn call_tool), :133 (McpServer::new(Option<TrackerStorage>))
  - crates/backend_core/src/workflow/lifecycle.rs (StatusChange { detail, stopped_timer }, update_item_status_reporting, close_item_reporting)
  - crates/backend_core/src/workflow/timer.rs (StoppedTimerSession: timer.item_id, elapsed_minutes, capped_from)
  - crates/backend_core/tests/plan_import.rs:391-411 (McpServer::new(Some(storage)).call_tool(name, json!(...)) test pattern)
  - crates/backend_core/tests/workflow_integration.rs (close_applies_session_cap: start_timer_at 600 minutes ago pattern)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Existing JSON keys of both tools unchanged (the new key is additive). Tool schemas in tools_list unchanged.

[PRE_GATE]
grep -q 'pub fn close_item_reporting' crates/backend_core/src/workflow/lifecycle.rs && ! grep -q 'close_item_reporting' crates/backend_core/src/mcp/mod.rs

[MUTATION_SPEC]
1. mcp/mod.rs: private helper `fn stopped_timer_json(stopped: Option<&StoppedTimerSession>) -> Value`: `Value::Null` for None, else `json!({ "item_id": s.timer.item_id, "logged_minutes": s.elapsed_minutes, "capped_from": s.capped_from })`.
2. tool_update_item_status: call `storage.update_item_status_reporting(...)`, take `change.detail` for the existing keys, add `"stopped_timer": stopped_timer_json(change.stopped_timer.as_ref())`.
3. tool_close_item_with_fix: same with `storage.close_item_reporting(...)`.
4. Test `mcp_close_reports_capped_timer` in workflow_integration.rs: init_temp_tracker; create BUG-0001; `start_timer_at("BUG-0001", Utc::now() - chrono::Duration::minutes(600), None)`; `McpServer::new(Some(storage)).call_tool("close_item_with_fix", json!({ "item_id": "BUG-0001", "summary": "fixed" }))` → `res["stopped_timer"]["item_id"] == "BUG-0001"`, `res["stopped_timer"]["logged_minutes"] == 480`, `res["stopped_timer"]["capped_from"].is_u64()`.
5. Test `mcp_status_without_timer_reports_null`: create BUG-0001, no timer; `call_tool("update_item_status", json!({ "item_id": "BUG-0001", "status": "in_progress" }))` → Ok and `res["stopped_timer"].is_null()`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && test "$(grep -c '_reporting(' crates/backend_core/src/mcp/mod.rs)" -ge 2 && grep -q 'fn mcp_close_reports_capped_timer' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 32C: Document Vault File Protection on Windows

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 32C: set_secure_file_permissions and ARCHITECTURE say plainly that Windows relies on the inherited %APPDATA% ACLs, and that a NEBULOUS_CONFIG_DIR outside the user profile is not protected

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/crypto/mod.rs
  - docs/ARCHITECTURE.md
READ_ONLY_REFS:
  - crates/backend_core/src/crypto/mod.rs (set_secure_file_permissions: cfg(unix) chmod 0600, cfg(not(unix)) `let _ = path;`; ensure_secure_dir: cfg(unix) 0700), :80-150 (nebulous_config_dir: Windows %APPDATA%\nebulous, NEBULOUS_CONFIG_DIR override, temp_dir fallback)
  - docs/ARCHITECTURE.md:53 (SecureStorage bullet: "0600 Unix permissions"), :88 (vault.enc tree line)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No code behaviour change: doc comments and docs only. This is a documented gap, not a fix: real Windows ACL code is deferred until Windows CI (32D) is green.

[PRE_GATE]
grep -q 'fn with_vault_lock' crates/backend_core/src/crypto/mod.rs && ! grep -q 'inherited' crates/backend_core/src/crypto/mod.rs

[MUTATION_SPEC]
1. crypto/mod.rs: doc comment on set_secure_file_permissions: on Unix it sets 0600; on Windows it does nothing, and the file is protected only by the ACLs it inherits from its directory. Under the default `%APPDATA%\nebulous` those allow only the user, SYSTEM and Administrators. A `NEBULOUS_CONFIG_DIR` outside the user profile gets that directory's ACLs instead.
2. crypto/mod.rs: one-line doc on ensure_secure_dir: same note (0700 on Unix; inherited ACLs on Windows).
3. docs/ARCHITECTURE.md:53: change "`0600` Unix permissions" to "`0600` on Unix; on Windows, the inherited `%APPDATA%` ACLs (point `NEBULOUS_CONFIG_DIR` only at a directory inside your user profile)".
4. docs/ARCHITECTURE.md:88: change the trailing comment "(0600)" to "(0600 on Unix)".

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'inherited' crates/backend_core/src/crypto/mod.rs && grep -q 'NEBULOUS_CONFIG_DIR' docs/ARCHITECTURE.md

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 32D: CI Matrix Over Linux, macOS and Windows

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 32D: CI runs fmt/clippy/test/CLI smoke on ubuntu-latest, macos-latest and windows-latest; Linux-only steps stay Linux-only

[SCOPE_LOCK]
ALLOWED_FILES:
  - .github/workflows/ci.yml
READ_ONLY_REFS:
  - .github/workflows/ci.yml (single `verify` job on ubuntu-latest: checkout, check-neutral.sh, apt-get deps, dtolnay/rust-toolchain@stable, Swatinem/rust-cache@v2, cargo-deny-action, fmt, clippy, test, `cargo run -p nebulous_cli -- --help`)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No step removed; no code changes to make other OSes pass (failures are reported by the [USER] CI step and reviewed at phase end).

[PRE_GATE]
grep -q 'inherited' crates/backend_core/src/crypto/mod.rs && ! grep -q 'windows-latest' .github/workflows/ci.yml

[MUTATION_SPEC]
1. Job `verify`: `strategy: { fail-fast: false, matrix: { os: [ubuntu-latest, macos-latest, windows-latest] } }`, `runs-on: ${{ matrix.os }}`, name `Format, Clippy, Test & CLI Smoke Check (${{ matrix.os }})`.
2. `if: runner.os == 'Linux'` on: the neutrality check, the apt-get step, and the cargo-deny step (cargo-deny-action is a Docker action and only runs on Linux; the neutrality check is OS-independent so one run is enough).
3. fmt: `if: runner.os == 'Linux'` too (formatting doesn't vary by OS). clippy, test and the CLI smoke step run on all three.
4. Keep everything else byte-for-byte (env, action versions, step names).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'windows-latest' .github/workflows/ci.yml && grep -q 'macos-latest' .github/workflows/ci.yml && grep -q 'fail-fast: false' .github/workflows/ci.yml && test "$(grep -c "runner.os == 'Linux'" .github/workflows/ci.yml)" -ge 4

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### User Step: Push and Report CI Matrix

**[USER]**. After 32D is COMPLETED. Push `main`, then paste the per-OS result (or the first failing step's log) for each of the three matrix jobs. macOS/Windows failures go to Parked for the Phase 14 review; they are not fixed in this phase.

```bash
cd /srv/devshare/Nebulous && git push
```


# Phase 15 — Phase 14 Review Picks: macOS Roots, CI Visibility, Tracker Commit, Windows ACLs (written 2026-11-16)

**Phase goal (frozen; approved by user 2026-11-16):** fix Parked items 1, 3, 4 and 5 from the Phase 14 review. (1) macOS system directories that resolve under `/private` are rejected as project roots. (3) CI runs every test binary on every OS, so all failures show. (4) `.tracker/` is neutral and committed. (5) Windows vault files and directories get a user-only ACL. Nothing else. Windows system directories (Parked 2) stay parked.

| Step | Fixes | Files |
| :--- | :--- | :--- |
| 33A | Parked 1: `/etc`, `/var`, `/var/tmp` resolve to `/private/...` on macOS; not in `UNSAFE_SYSTEM_DIRS` | `crates/backend_core/src/storage/mod.rs`, `crates/backend_core/tests/storage_integration.rs` |
| 33B | Parked 3: `cargo test` stops at the first failing binary | `.github/workflows/ci.yml` |
| 33C | Parked 4: 25 `.tracker/` files name distro tools; `.timer.toml.bak` not ignored. Claude commits `.tracker/` after validation | `.tracker/` (12 items + backend_core `scope.toml`), `.gitignore` |
| 33D | Parked 5: Windows ACL via `icacls` (no new dependency). `/boost`: Linux gates can't compile `cfg(windows)` code, so only Windows CI proves it | `crates/backend_core/src/crypto/mod.rs`, `docs/ARCHITECTURE.md` |
| [USER] CI | Push and report the matrix (now `--no-fail-fast`) | — |

**Why `icacls` and not `windows-sys` (33D):** both are written blind from Linux. `icacls` is one documented command with no new dependency or `unsafe` FFI; a wrong flag fails loudly with a non-zero exit, which the code turns into an error. Its cost is one process spawn per vault save.

### STEP 33A: Reject macOS /private System Directories as Project Roots

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 33A: UNSAFE_SYSTEM_DIRS adds /private/etc, /private/var, /private/var/tmp and /Applications, so macOS's canonicalized /etc, /var and /var/tmp are rejected like on Linux

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/mod.rs
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs:1453-1481 (UNSAFE_SYSTEM_DIRS: already has /Users, /System, /Library, /Volumes, /private, /private/tmp, /var/tmp), :1515-1546 (resolve_project_root_candidate: canonicalizes when the path exists, so macOS /etc becomes /private/etc), :1553 (is_unsafe_broad_directory)
  - crates/backend_core/tests/storage_integration.rs:1884-1900 (test_safe_project_root_validation_poisoned_registry_pruning_and_hidden_dir_skip; line 1894 `/etc` assertion fails on macOS CI)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No logic change to is_unsafe_broad_directory or resolve_project_root_candidate; only the const list grows. No Windows directories (Parked 2 stays parked).

[PRE_GATE]
grep -q '"/private/tmp",' crates/backend_core/src/storage/mod.rs && ! grep -q '"/private/etc",' crates/backend_core/src/storage/mod.rs

[MUTATION_SPEC]
1. storage/mod.rs UNSAFE_SYSTEM_DIRS: add "/Applications", "/private/etc", "/private/var", "/private/var/tmp" next to the existing macOS entries. Why: on macOS /etc, /var and /var/tmp are symlinks into /private, and the candidate is canonicalized before the list check, so the plain entries never match there.
2. One comment line above the macOS entries saying that (canonicalized symlink targets).
3. Test: in test_safe_project_root_validation_poisoned_registry_pruning_and_hidden_dir_skip, right after the existing step-1 assertions, add `assert!(is_unsafe_broad_directory(Path::new(p)))` for each of "/private/etc", "/private/var", "/private/var/tmp", "/Applications". These paths don't exist on Linux, so they're matched lexically there and canonicalized on macOS; both must reject.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q '"/private/etc",' crates/backend_core/src/storage/mod.rs && grep -q '"/private/var/tmp",' crates/backend_core/src/storage/mod.rs && grep -q 'Path::new("/private/etc")' crates/backend_core/tests/storage_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 33B: CI Runs Every Test Binary

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 33B: CI's test step uses --no-fail-fast, so a failure in one test binary doesn't hide the results of the others on any OS

[SCOPE_LOCK]
ALLOWED_FILES:
  - .github/workflows/ci.yml
READ_ONLY_REFS:
  - .github/workflows/ci.yml (step "Run unit and integration tests (cargo test)": `run: cargo test --workspace`)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Only that one `run:` line changes.

[PRE_GATE]
grep -q '"/private/etc",' crates/backend_core/src/storage/mod.rs && ! grep -q 'no-fail-fast' .github/workflows/ci.yml

[MUTATION_SPEC]
1. ci.yml test step: `run: cargo test --workspace` → `run: cargo test --workspace --no-fail-fast`. Why: cargo stops after the first failing test binary; macOS CI showed one failure and hid the rest.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'cargo test --workspace --no-fail-fast' .github/workflows/ci.yml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 33C: Neutral Tracker Titles; Ignore the Quarantined Timer

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 33C: The 12 release-engine item titles in .tracker/ are reworded without distro-tool names (item.toml, desc.md and scope.toml kept identical), and .gitignore ignores .tracker/.timer.toml.bak, so .tracker/ can be committed and pass check-neutral

[SCOPE_LOCK]
ALLOWED_FILES:
  - .gitignore
  - .tracker/phases/production/domains/backend_core/scope.toml
  - .tracker/phases/production/domains/backend_core/items/TASK-00*/item.toml
  - .tracker/phases/production/domains/backend_core/items/TASK-00*/desc.md
READ_ONLY_REFS:
  - .gitignore:5-7 (Nebulous tracker runtime state block)
  - scripts/check-neutral.sh:11 (PATTERN='novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu', case-insensitive)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Only the 12 tasks below (TASK-0024, 0027, 0029, 0030, 0032, 0034, 0035, 0036, 0041, 0042, 0046, 0054); no other line in any file. Edit the files directly: do NOT run any `neb` command against this repo's tracker (it writes .lock/.timer files and re-syncs indexes).

[PRE_GATE]
grep -q 'no-fail-fast' .github/workflows/ci.yml && grep -rqiE 'dpkg|calamares' .tracker/phases/production/domains/backend_core/scope.toml

[MUTATION_SPEC]
1. For each task, replace the title text in three places with the NEW text below: `title = "..."` in items/TASK-XXXX/item.toml (line 6); the text after "Imported from roadmap step `STEP NNx`: " in items/TASK-XXXX/desc.md (line 2; keep that prefix); and the matching `title = "..."` line in scope.toml. Only the changed words differ from the old title; everything else (punctuation, the "(Gemini; ...)" suffix) stays.
   - TASK-0024 (25B): "Package version ordering + reference-verified case table (Gemini; validate-step PASS; 75 cases re-checked against the reference tool)"
   - TASK-0027 (25E): "Package status / os-release parsers, inventory types (Gemini; validate-step PASS)"
   - TASK-0029 (25G): "inventory_from_root: os-release, package database, live-boot + COSMIC synthetic packages, installer, boot (Gemini; validate-step PASS)"
   - TASK-0030 (25H): "extract_iso_inventory: unprivileged ISO + root-image extraction into a self-deleting temp dir; tempfile moved to [dependencies] (Gemini; validate-step PASS; ignored round-trip test also passed)"
   - TASK-0032 (27A): "InstalledPackage.depends + parse_depends from package Depends:, for kernel-line membership (Gemini; validate-step PASS)"
   - TASK-0034 (25K): "approvals.toml read/append with version-scoped covers; evaluate_against_golden by component, custom binaries, installer (Gemini; validate-step PASS)"
   - TASK-0035 (25L): "manifest.toml model + reader; verify_repo_invariants: custom tools, installer, workspace crate versions, stage-script tokens (Gemini; validate-step PASS)"
   - TASK-0036 (27B): "evaluate_against_manifest (os-release, packages, custom binaries + flags, installer), evaluate_variance (manifest + optional golden), shared live-binary index (Gemini; validate-step PASS)"
   - TASK-0041 (27D): "IsoIdentity (extra live layers, production root-image sha256), extract_iso, read_test_overlay_manifest (Gemini; validate-step PASS incl. ignored round-trips)"
   - TASK-0042 (27E): "Fix 25O: build_candidate refuses test ISOs, stores the root-image sha256; VM phases tie through the overlay claim + guest measurement; parse_verdict_overlay (Gemini; validate-step PASS)"
   - TASK-0046 (25S): "Status summary (read_status, status_json, render_status_text, §4.8 keys) + built-in self-test runner (14 §2.9 cases, embedded package-version table, panic-safe) (Gemini; validate-step PASS)"
   - TASK-0054 (29G): "Ingestion kernel heuristic drops vendor kernel names; storage test data neutral via fixed sed script (sha256-pinned) (Gemini; validate-step PASS)"
2. .gitignore: add `/.tracker/.timer.toml.bak` on the line after `/.tracker/.timer.toml`. Why: STEP 18B quarantines a corrupt or orphaned timer there; it's runtime state like .timer.toml.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -rIqiE 'novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu' .tracker && grep -qx '/.tracker/.timer.toml.bak' .gitignore && test "$(grep -c 'installer' .tracker/phases/production/domains/backend_core/scope.toml)" -ge 4

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 33D: Windows Vault Files and Directories Get a User-Only ACL

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 33D: On Windows, set_secure_file_permissions and ensure_secure_dir remove inherited ACEs and grant only the current user via icacls; docs say so

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/crypto/mod.rs
  - docs/ARCHITECTURE.md
READ_ONLY_REFS:
  - crates/backend_core/src/crypto/mod.rs:610-623 (ensure_secure_dir: create_dir_all, cfg(unix) chmod 0700 with errors ignored), :625-645 (set_secure_file_permissions: cfg(unix) chmod 0600 with errors returned as TrackerError::Io; cfg(not(unix)) no-op), :56 (with_vault_lock calls ensure_secure_dir on every vault operation), :539-545 (save_vault_unlocked), :601-604 (vault.key creation), end of file (mod tests)
  - docs/ARCHITECTURE.md:53 (SecureStorage bullet, 32C wording "on Windows, the inherited `%APPDATA%` ACLs ..."), :88 (vault.enc tree line "(0600 on Unix)")
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No new dependencies, no Cargo.toml/Cargo.lock change, no `unsafe`. Unix behaviour unchanged. This code is cfg(windows): the Linux gates cannot compile it, so keep it short and exactly as specified; Windows CI is the verifier.

[PRE_GATE]
grep -q 'cargo test --workspace --no-fail-fast' .github/workflows/ci.yml && ! grep -q 'fn restrict_to_current_user' crates/backend_core/src/crypto/mod.rs

[MUTATION_SPEC]
1. crypto/mod.rs: `#[cfg(windows)] fn restrict_to_current_user(path: &Path, is_dir: bool) -> Result<()>`:
   - user = `std::env::var("USERNAME")`, error → `TrackerError::KeyringError("USERNAME is not set; cannot restrict ACL on <path>")`.
   - principal = `format!("{domain}\\{user}")` when `std::env::var("USERDOMAIN")` is Ok and non-empty after trim, else user.
   - grant = `format!("{principal}:(OI)(CI)F")` when is_dir, else `format!("{principal}:F")`.
   - `std::process::Command::new("icacls").arg(path).arg("/inheritance:r").arg("/grant:r").arg(&grant).output()`; spawn error → `TrackerError::Io { path, source }`; non-success status → `TrackerError::KeyringError(format!("icacls failed on {}: {}", path.display(), String::from_utf8_lossy(&output.stderr).trim()))`.
   - Doc line: why (Windows doesn't honour Unix modes; without this the files carry whatever ACLs their directory has, including a NEBULOUS_CONFIG_DIR outside the profile).
2. set_secure_file_permissions: replace the `#[cfg(not(unix))]` no-op with `#[cfg(windows)] { restrict_to_current_user(path, false)?; }` plus `#[cfg(not(any(unix, windows)))] { let _ = path; }`.
3. ensure_secure_dir: record `let existed = dir.is_dir();` before create_dir_all; add `#[cfg(windows)] if !existed { let _ = restrict_to_current_user(dir, true); }`. Why only when newly created: with_vault_lock calls ensure_secure_dir on every vault operation, and spawning icacls each time is wasteful; errors ignored to match the Unix branch. Silence unused `existed` on unix with `#[cfg(not(windows))] let _ = existed;`.
4. Doc comments on both functions: replace the 32C "does nothing / inherited ACLs" text with: Unix 0600 (file) / 0700 (dir); Windows: inherited ACEs removed and full control granted to the current user only (directories: only when newly created).
5. Test in crypto `mod tests`: `#[cfg(windows)] #[test] fn windows_vault_file_acl_is_user_only()`: tempdir; write a file; `set_secure_file_permissions(&file).unwrap()`; run `icacls <file>`; stdout (lossy) must contain the USERNAME value and must not contain "(I)" (icacls's inherited-ACE marker).
6. docs/ARCHITECTURE.md:53: replace the Windows clause with "on Windows, an ACL granting only the current user (set with `icacls`; inheritance removed)". :88: "(0600 on Unix)" → "(0600 on Unix; user-only ACL on Windows)".

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'fn restrict_to_current_user' crates/backend_core/src/crypto/mod.rs && grep -q '/inheritance:r' crates/backend_core/src/crypto/mod.rs && grep -q 'fn windows_vault_file_acl_is_user_only' crates/backend_core/src/crypto/mod.rs && grep -q 'icacls' docs/ARCHITECTURE.md && ! grep -q 'inherited `%APPDATA%` ACLs' docs/ARCHITECTURE.md

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### User Step: Push and Report CI Matrix (Phase 15)

**[USER]**. After 33D is COMPLETED and Claude has committed. Push `main`, then paste each OS job's log separately (ubuntu, macos, windows). With `--no-fail-fast`, each log ends with a list of every failing test. The Windows log is the only proof that 33D works (`windows_vault_file_acl_is_user_only`).

```bash
cd /srv/devshare/Nebulous && git push
```


# Phase 16 — Phase 15 Review Picks: Windows Clippy, macOS /home, Cargo Warning, Config-Dir Docs (written 2026-11-16)

**Phase goal (frozen; approved by user 2026-11-16):** fix Parked items 1, 2, 3 and 5 from the Phase 15 review. (1) Windows clippy passes, so Windows tests run (and 33D's ACL test finally executes). (2) A listed system directory is rejected whether the path is given as listed or resolves to it, fixing macOS `/home`. (3) No `workspace.package.authors` warning. (5) ARCHITECTURE lists every config-dir file. Nothing else. Windows system directories stay parked until Windows tests run.

| Step | Fixes | Files |
| :--- | :--- | :--- |
| 34A | Parked 1: `TomlDeserialize` hits clippy's 128-byte `result_large_err` limit on Windows | `crates/backend_core/src/error.rs`, `crates/backend_core/src/models/registry.rs` |
| 34B | Parked 2: macOS canonicalizes `/home` to an autofs path not in `UNSAFE_SYSTEM_DIRS` | `crates/backend_core/src/storage/mod.rs` |
| 34C | Parked 3: `workspace.package.authors` unused | `crates/backend_core/Cargo.toml`, `crates/nebulous_cli/Cargo.toml` |
| 34D | Parked 5: config-dir tree omits `vault.key` and `vault.lock` | `docs/ARCHITECTURE.md` |
| [USER] CI | Push and report each OS's log | — |

**Verification limits:** 34A's effect (Windows size) and 34B's effect (macOS `/home`) can't be observed on Linux. The Linux gates prove nothing broke; the CI run proves the fix.

### STEP 34A: Box the TOML Deserialize Error

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 34A: TrackerError::TomlDeserialize stores Box<toml::de::Error>, so the enum stays under clippy's 128-byte result_large_err limit on Windows (where PathBuf is larger)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/error.rs
  - crates/backend_core/src/models/registry.rs
READ_ONLY_REFS:
  - crates/backend_core/src/error.rs:20-25 (TomlDeserialize { path: Option<PathBuf>, #[source] source: toml::de::Error }), :98-102 (impl From<toml::de::Error> for TrackerError)
  - crates/backend_core/src/models/registry.rs:562-565 (map_err building TomlDeserialize with `source`)
  - crates/backend_core/src/storage/mod.rs:1443-1450 (attach_toml_path), crates/backend_core/src/ingestion/mod.rs:531-535, :565-568 (re-wrap `source` unchanged: they move the field, so they keep compiling with a Box and need no edit); storage/reconcile.rs:285, lib.rs:421,436 (match `{ .. }`, no edit)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No `#[allow(clippy::result_large_err)]` anywhere (that would hide the size problem). Display text and the `path` field unchanged. No other variant changes.

[PRE_GATE]
grep -q 'source: toml::de::Error,' crates/backend_core/src/error.rs

[MUTATION_SPEC]
1. error.rs TomlDeserialize: `source: toml::de::Error` → `source: Box<toml::de::Error>` (keep `#[source]` and the `#[error(...)]` text). One comment line: why (on Windows PathBuf is larger and this variant reached clippy's 128-byte result_large_err limit, failing CI).
2. error.rs From<toml::de::Error>: `Self::TomlDeserialize { path: None, source: Box::new(source) }`.
3. registry.rs:562: `source: Box::new(source)`.
4. Fix any other construction site the compiler reports in these two files the same way. If the compiler reports one in another file, halt and report it (do not edit outside ALLOWED_FILES).
5. Test in error.rs: add `#[cfg(test)] mod tests` with `toml_deserialize_error_keeps_message`: `let err: TrackerError = toml::from_str::<toml::Value>("a = [").unwrap_err().into();` → `matches!(err, TrackerError::TomlDeserialize { path: None, .. })` and `err.to_string()` starts with "TOML deserialization error: ".

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'source: Box<toml::de::Error>' crates/backend_core/src/error.rs && ! grep -rq 'result_large_err' crates && grep -q 'fn toml_deserialize_error_keeps_message' crates/backend_core/src/error.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 34B: Check the Lexical Path Against the System-Dir List Too

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 34B: is_unsafe_broad_directory checks UNSAFE_SYSTEM_DIRS against the lexical path as well as the canonical one, so a listed directory is rejected wherever it resolves (macOS /home → autofs)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs:1517-1551 (resolve_project_root_candidate: empty → PathBuf::new(); absolutize via current_dir; lexical_normalize_path; strip trailing project.toml then .tracker; then canonicalize if it exists, stripping .tracker again), :1557-1575 (is_unsafe_broad_directory: `let s = normalize_comparable_path_str(&resolved)`, root checks, `UNSAFE_SYSTEM_DIRS.contains(&s.as_str())`), :1617 (validate_safe_project_root, second caller)
  - crates/backend_core/tests/storage_integration.rs:1896 (`/home` assertion failing on macOS CI)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. UNSAFE_SYSTEM_DIRS contents unchanged. resolve_project_root_candidate's return value unchanged for every input. The temp-dir and $HOME checks unchanged.

[PRE_GATE]
grep -q '"/private/etc",' crates/backend_core/src/storage/mod.rs && ! grep -q 'fn lexical_project_root_candidate' crates/backend_core/src/storage/mod.rs

[MUTATION_SPEC]
1. storage/mod.rs: split resolve_project_root_candidate. New `fn lexical_project_root_candidate(path: &Path) -> PathBuf` holds everything up to and including the two strip steps (the empty-path early return, absolutize, lexical_normalize_path, strip project.toml, strip .tracker) and returns `norm`. resolve_project_root_candidate becomes: `let norm = lexical_project_root_candidate(path); if norm.as_os_str().is_empty() { return norm; }` then the existing canonicalize block unchanged.
2. is_unsafe_broad_directory: right after the existing `if UNSAFE_SYSTEM_DIRS.contains(&s.as_str()) { return true; }`, add `let lexical_s = normalize_comparable_path_str(&lexical_project_root_candidate(path)); if UNSAFE_SYSTEM_DIRS.contains(&lexical_s.as_str()) { return true; }`. Comment: why (macOS resolves some listed dirs, e.g. /home, to paths outside the list; checking the path as given rejects them wherever they point).
3. Update the doc comment of is_unsafe_broad_directory: system trees are matched both as given and as resolved.
4. Test in storage/mod.rs `mod tests` (add one if absent): `lexical_candidate_strips_tracker_and_project_toml`: `lexical_project_root_candidate(Path::new("/srv/x/.tracker/project.toml"))` == `PathBuf::from("/srv/x")`, and `lexical_project_root_candidate(Path::new("/home/../home"))` == `PathBuf::from("/home")`. (`#[cfg(unix)]` on the test, since these are Unix paths.)

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'fn lexical_project_root_candidate' crates/backend_core/src/storage/mod.rs && grep -q 'lexical_s' crates/backend_core/src/storage/mod.rs && grep -q 'fn lexical_candidate_strips_tracker_and_project_toml' crates/backend_core/src/storage/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 34C: Member Crates Inherit workspace.package.authors

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 34C: backend_core and nebulous_cli inherit authors from the workspace, so cargo's unused_workspace_package_fields warning goes away

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/Cargo.toml
  - crates/nebulous_cli/Cargo.toml
READ_ONLY_REFS:
  - Cargo.toml:8-13 ([workspace.package]: version, edition, license, authors = ["Nebulous Contributors"])
  - crates/backend_core/Cargo.toml:1-5, crates/nebulous_cli/Cargo.toml:1-5 ([package] with version/edition/license `.workspace = true`, no authors)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Root Cargo.toml and Cargo.lock unchanged.

[PRE_GATE]
grep -q 'lexical_s' crates/backend_core/src/storage/mod.rs && ! grep -q 'authors.workspace' crates/backend_core/Cargo.toml

[MUTATION_SPEC]
1. Both member Cargo.toml files: add `authors.workspace = true` on the line after `license.workspace = true`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -qx 'authors.workspace = true' crates/backend_core/Cargo.toml && grep -qx 'authors.workspace = true' crates/nebulous_cli/Cargo.toml && git diff --quiet -- Cargo.lock Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 34D: Config-Dir Tree Lists vault.key and vault.lock

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 34D: ARCHITECTURE's config-dir tree lists vault.key and vault.lock next to vault.enc

[SCOPE_LOCK]
ALLOWED_FILES:
  - docs/ARCHITECTURE.md
READ_ONLY_REFS:
  - docs/ARCHITECTURE.md:85-88 (config-dir tree: projects.toml, `└── vault.enc ... (0600 on Unix; user-only ACL on Windows)`)
  - crates/backend_core/src/crypto/mod.rs (VAULT_KEY_FILE_NAME "vault.key": base64 seed, skipped when NEBULOUS_VAULT_MASTER_KEY is set; VAULT_LOCK_FILE_NAME "vault.lock": fd-lock held by with_vault_lock)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Only the tree block changes.

[PRE_GATE]
grep -q 'authors.workspace' crates/nebulous_cli/Cargo.toml && ! grep -q 'vault.lock' docs/ARCHITECTURE.md

[MUTATION_SPEC]
1. docs/ARCHITECTURE.md tree: `└── vault.enc` becomes `├── vault.enc` (comment unchanged); then add, aligned to the same comment column:
   `├── vault.key                                         # Vault master-key seed, base64 (0600 on Unix; user-only ACL on Windows; not used when NEBULOUS_VAULT_MASTER_KEY is set)`
   `└── vault.lock                                        # Advisory lock held during vault read-modify-write and vault.key creation`

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q '├── vault.key' docs/ARCHITECTURE.md && grep -q '└── vault.lock' docs/ARCHITECTURE.md && grep -q '├── vault.enc' docs/ARCHITECTURE.md

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### User Step: Push and Report CI Matrix (Phase 16)

**[USER]**. After 34D is COMPLETED and Claude has committed. Push `main`, then paste each OS's failing-test list (or "passed"). Windows: clippy should pass and the test step should run; look for `windows_vault_file_acl_is_user_only` and any `storage_integration` failures (expected: the Unix-path project-root assertions — Parked, Windows system dirs).

```bash
cd /srv/devshare/Nebulous && git push
```


# Phase 17 — Phase 16 Review Pick: Unblock Windows Clippy (written 2026-11-16)

**Phase goal (frozen; approved by user 2026-11-16):** fix Parked item 1 from the Phase 16 review: the storage test module compiles cleanly on Windows, so Windows clippy passes and Windows tests run for the first time. Nothing else; Windows test failures that run reveals go to Parked.

| Step | Fixes | Files |
| :--- | :--- | :--- |
| 35A | Parked 1: `use super::*` unused on Windows (my 34B spec gated the test, not the module) | `crates/backend_core/src/storage/mod.rs` |
| [USER] CI | Push and report each OS's log | — |

### STEP 35A: Gate the Unix-Only Storage Test Module

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 35A: storage/mod.rs `mod tests` is #[cfg(all(test, unix))] and its test drops the per-test #[cfg(unix)], so Windows clippy has no unused import

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs:1799-1816 (`#[cfg(test)] mod tests { use super::*; #[test] #[cfg(unix)] fn lexical_candidate_strips_tracker_and_project_toml() ... }`; added by 34B)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Test body and the other test module (`mod lock_tests`) unchanged. No `#[allow(unused_imports)]`.

[PRE_GATE]
grep -q 'fn lexical_candidate_strips_tracker_and_project_toml' crates/backend_core/src/storage/mod.rs && ! grep -q 'cfg(all(test, unix))' crates/backend_core/src/storage/mod.rs

[MUTATION_SPEC]
1. The `#[cfg(test)]` line directly above `mod tests {` (the module holding lexical_candidate_strips_tracker_and_project_toml) becomes `#[cfg(all(test, unix))]`. Why: the module's only test uses Unix paths; gating only the test left `use super::*` unused on Windows, which clippy -D warnings rejects.
2. Delete the `#[cfg(unix)]` line on that test (the module gate covers it).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q '#\[cfg(all(test, unix))\]' crates/backend_core/src/storage/mod.rs && test "$(grep -c '#\[cfg(unix)\]' crates/backend_core/src/storage/mod.rs)" -lt "$(git show HEAD:crates/backend_core/src/storage/mod.rs | grep -c '#\[cfg(unix)\]')"

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### User Step: Push and Report CI Matrix (Phase 17)

**[USER]**. After 35A is COMPLETED and Claude has committed. Push `main`, then paste the Windows failing-test list (macOS and Linux: "passed" is enough). This is the first Windows run that gets past clippy: look for `windows_vault_file_acl_is_user_only`, and expect `storage_integration` project-root failures (Parked: Windows system dirs).

```bash
cd /srv/devshare/Nebulous && git push
```


# Phase 18 — Phase 17 Review Pick: Unblock Windows Clippy, Take 2 (written 2026-11-16)

**Phase goal (frozen; approved by user 2026-11-16):** fix Parked item 1 from the Phase 17 review: `path_containment.rs` compiles cleanly on Windows, so Windows clippy passes and Windows tests run. Nothing else; failures that run reveals go to Parked.

| Step | Fixes | Files |
| :--- | :--- | :--- |
| 36A | Parked 1: `ComponentManifest` imported at file top, used only in `#[cfg(unix)] mod unix` | `crates/backend_core/tests/path_containment.rs` |
| [USER] CI | Push and report each OS's log | — |

### STEP 36A: Move the Unix-Only Import Into mod unix

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 36A: path_containment.rs imports ComponentManifest inside #[cfg(unix)] mod unix instead of at the top, so it isn't an unused import on Windows

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/tests/path_containment.rs
READ_ONLY_REFS:
  - crates/backend_core/tests/path_containment.rs:8-11 (top `use backend_core::{contained_path, is_valid_identifier, ComponentManifest, IngestionOptions, TrackerError, TrackerStorage};`), :124-127 (`#[cfg(unix)] mod unix { use super::*; use std::os::unix::fs::symlink;`), :258, :270 (ComponentManifest::new, both inside mod unix)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No test bodies change. No `#[allow(unused_imports)]`.

[PRE_GATE]
grep -q 'cfg(all(test, unix))' crates/backend_core/src/storage/mod.rs && grep -q 'is_valid_identifier, ComponentManifest, IngestionOptions' crates/backend_core/tests/path_containment.rs

[MUTATION_SPEC]
1. Remove `ComponentManifest` from the top-level `use backend_core::{...}` list.
2. Inside `mod unix`, after `use std::os::unix::fs::symlink;`, add `use backend_core::ComponentManifest;`. Why: it's only used in the Unix-only module; at the top it is unused on Windows and clippy -D warnings rejects it.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -q 'is_valid_identifier, ComponentManifest' crates/backend_core/tests/path_containment.rs && grep -q '^    use backend_core::ComponentManifest;' crates/backend_core/tests/path_containment.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### User Step: Push and Report CI Matrix (Phase 18)

**[USER]**. After 36A is COMPLETED and Claude has committed. Push `main`, then paste the Windows result (macOS and Linux: "passed" is enough). If clippy passes, paste the failing-test list; look for `windows_vault_file_acl_is_user_only`.

```bash
cd /srv/devshare/Nebulous && git push
```


# Phase 19 — Phase 18 Review Picks: Windows System Dirs, Windows Line Endings (written 2026-11-16)

**Phase goal (frozen; approved by user 2026-11-16, incl. testuser rename):** fix the two Windows test failures from CI run 5. (1) Windows system directories are rejected as project roots, and the project-root test runs Unix-path checks only on Unix and Windows-path checks only on Windows. (2) The git fetch/pull test is independent of the runner's `core.autocrlf`. (3) Added 2026-11-17 with user approval: RUSTSEC-2026-0544 fixed and RUSTSEC-2026-0543 temporarily ignored so the gates pass. (4) Added 2026-11-17 with user approval (macOS CI run 6): the watcher recognises its own index writes through symlinked paths. Nothing else.

| Step | Fixes | Files |
| :--- | :--- | :--- |
| 37A | Windows system dirs not in the blocklist | `crates/backend_core/src/storage/mod.rs` |
| 37D | Added 2026-11-17 (user-approved mid-phase): RUSTSEC-2026-0544 (libz-sys, fix available) and RUSTSEC-2026-0543 (chrono unmaintained, temporary ignore) fail every gate | `Cargo.lock`, `deny.toml` |
| 37B | Project-root test asserts Unix paths on Windows; registry-pruning fixture is Unix-only | `crates/backend_core/tests/storage_integration.rs` |
| 37C | Git test compares checkout bytes; runner's `core.autocrlf=true` turns `\n` into `\r\n` | `crates/backend_core/tests/workflow_integration.rs` |
| 37E | Added 2026-11-17 (user-approved mid-phase; macOS CI run 6): watcher echo check misses its own index writes when the tracker path is a symlink — FSEvents reports `/private/var/...` | `crates/backend_core/src/watcher/mod.rs` |
| [USER] CI | Push and report each OS's log | — |

**Windows-only code rule (learned in Phases 17–18):** the Linux gates never compile `#[cfg(windows)]` code and never see what `#[cfg(unix)]` hides from Windows. Every name used only inside a gated block or test must be defined or imported inside that block or test.

### STEP 37A: Reject Windows System Directories as Project Roots

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 37A: is_unsafe_broad_directory rejects <drive>:\Windows, Program Files, Program Files (x86), ProgramData and Users (case-insensitive, any drive), on the lexical and canonical path

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs:1453-1485 (UNSAFE_SYSTEM_DIRS), :1499-1515 (normalize_comparable_path_str: backslashes → '/', strips `//?/` and `//?/UNC/`, trims trailing '/'), :1562-1595 (is_unsafe_broad_directory: `s` = canonical comparable string; `lexical_s` = lexical comparable string, added by 34B), :1799 (`#[cfg(all(test, unix))] mod tests`)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. UNSAFE_SYSTEM_DIRS unchanged. Exact-match policy unchanged (subdirectories such as C:\Windows\System32 or C:\Users\<name> are NOT added). No `#[cfg(windows)]` on the new function: it is pure string logic and must be tested on Linux.

[PRE_GATE]
grep -q 'lexical_s' crates/backend_core/src/storage/mod.rs && ! grep -q 'fn is_windows_system_dir' crates/backend_core/src/storage/mod.rs

[MUTATION_SPEC]
1. Below UNSAFE_SYSTEM_DIRS: `const UNSAFE_WINDOWS_DIRS: &[&str] = &["windows", "program files", "program files (x86)", "programdata", "users"];` with a doc line (top-level Windows system trees, matched under any drive letter, case-insensitively, because Windows paths are case-insensitive).
2. `fn is_windows_system_dir(s: &str) -> bool`: `s` is a comparable string from normalize_comparable_path_str. Return false unless `s.len() >= 3`, byte 0 is ASCII alphabetic, byte 1 is b':' and byte 2 is b'/'. Otherwise return `UNSAFE_WINDOWS_DIRS.contains(&s[3..].to_ascii_lowercase().as_str())`.
3. is_unsafe_broad_directory: right after the `lexical_s` check, add `if is_windows_system_dir(&s) || is_windows_system_dir(&lexical_s) { return true; }`. Doc comment: add the Windows trees to the list of rejected directories.
4. Test: new portable module `#[cfg(test)] mod windows_dir_tests { use super::is_windows_system_dir; #[test] fn windows_system_dirs_match_any_drive_and_case() {...} }` (import the one function by name, not `super::*`). True: "C:/Windows", "c:/windows", "D:/Program Files (x86)", "C:/ProgramData", "C:/Users". False: "C:/Users/testuser", "C:/Windows/System32", "/Windows", "C:", "C:/", "1:/Windows".

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'fn is_windows_system_dir' crates/backend_core/src/storage/mod.rs && grep -q 'is_windows_system_dir(&lexical_s)' crates/backend_core/src/storage/mod.rs && grep -q 'use super::is_windows_system_dir;' crates/backend_core/src/storage/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 37D: Clear the 2026-11-17 RustSec Advisories

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 37D: libz-sys is updated past RUSTSEC-2026-0544, and RUSTSEC-2026-0543 (chrono unmaintained, no fixed version) is ignored in deny.toml with a dated reason until the Phase 20 chrono migration, so cargo deny passes again

[SCOPE_LOCK]
ALLOWED_FILES:
  - Cargo.lock
  - deny.toml
READ_ONLY_REFS:
  - deny.toml:1-13 (header "Permanent constraint: zero open RustSec/GHSA advisories against locked crates."; `[advisories]` with `ignore = []`)
  - .git/validate-step/37A/post_gate.log (RUSTSEC-2026-0543 chrono 0.4.45 unmaintained, "No safe upgrade is available!"; RUSTSEC-2026-0544 libz-sys 1.1.31 heap buffer overflow, "Upgrade to >=1.1.33")
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No Cargo.toml change. In Cargo.lock only libz-sys may change version (`cargo update -p libz-sys`, nothing broader). Only RUSTSEC-2026-0543 is ignored; vulnerabilities are never ignored. This ignore is a user-approved workaround (2026-11-17): it hides that chrono gets no more fixes, until Phase 20 replaces it.

[PRE_GATE]
grep -q 'fn is_windows_system_dir' crates/backend_core/src/storage/mod.rs && grep -q '^ignore = \[\]' deny.toml

[MUTATION_SPEC]
1. Run `cargo update -p libz-sys` (libz-sys 1.1.31 → 1.1.33 or later). Check `git diff Cargo.lock` shows only the libz-sys entry changing.
2. deny.toml `[advisories]`: replace `ignore = []` with
   `ignore = [`
   `    # TEMPORARY exception to the zero-advisories constraint, approved by the user 2026-11-17: chrono is unmaintained`
   `    # and no fixed version exists. Remove this entry in Phase 20 (chrono -> jiff migration).`
   `    { id = "RUSTSEC-2026-0543", reason = "chrono unmaintained, no fixed version; temporary until the Phase 20 chrono -> jiff migration (user-approved 2026-11-17)" },`
   `]`

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'RUSTSEC-2026-0543' deny.toml && ! grep -q 'RUSTSEC-2026-0544' deny.toml && ! grep -A1 'name = "libz-sys"' Cargo.lock | grep -q 'version = "1.1.31"' && git diff --quiet -- Cargo.toml crates/backend_core/Cargo.toml crates/nebulous_cli/Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 37B: Split the Project-Root Test by OS

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 37B: test_safe_project_root_validation_... checks Unix paths only on Unix and Windows system dirs only on Windows; the Unix-only registry-pruning part moves to its own #[cfg(unix)] test

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/tests/storage_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/tests/storage_integration.rs:1885-2048 (test_safe_project_root_validation_poisoned_registry_pruning_and_hidden_dir_skip). Step 1 :1891-1916 (path asserts, HOME block, 33A's /private + /Applications asserts). Step 2 :1918-1968 (temp, cfg_dir, valid_proj, storage, register_project_in "/" and "/tmp", poisoned TOML with root_path "/home", load_global_registry_from, parsed_reg). Step 3 :1970-2007 (unsafe_storage on "/tmp"; hidden-dir ingestion of valid_proj). Step 4 :2009-2047 (already #[cfg(unix)] symlink block; uses temp, valid_proj, storage).
  - CI run 5 (Windows): panicked at :1894 `is_unsafe_broad_directory(Path::new("/etc"))` — on Windows "/etc" has no drive prefix and resolves to <drive>:\etc.
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No assertion is deleted or weakened: each one keeps running on Unix exactly as now. Steps 3 (hidden dirs) and 4 unchanged except the one /tmp line in step 3.

[PRE_GATE]
grep -q 'RUSTSEC-2026-0543' deny.toml && ! grep -q 'fn test_registry_prunes_poisoned_unix_roots' crates/backend_core/tests/storage_integration.rs

[MUTATION_SPEC]
1. Step 1: wrap these in one `#[cfg(unix)] { ... }` block: the asserts for "/etc", "/usr", "/home", "/tmp"; `validate_safe_project_root(Path::new("/tmp"))`; `TrackerStorage::init(Path::new("/tmp"), ...)`; and the four asserts "/private/etc", "/private/var", "/private/var/tmp", "/Applications". Keep portable (unchanged, outside any gate): "/", "/.tracker", `std::env::temp_dir()`, `validate_safe_project_root(Path::new("/"))`, `TrackerStorage::init(Path::new("/"), ...)`, and the HOME block.
2. Step 1: add `#[cfg(windows)] { for p in [r"C:\Windows", r"c:\windows", r"C:\Program Files", r"C:\Program Files (x86)", r"C:\ProgramData", r"C:\Users"] { assert!(is_unsafe_broad_directory(Path::new(p)), "{p} must be rejected"); } assert!(validate_safe_project_root(Path::new(r"C:\Windows")).is_err()); }`.
3. Move the registry part of step 2 into a new test directly below this one: `#[cfg(unix)] #[test] fn test_registry_prunes_poisoned_unix_roots()`, with its own `use backend_core::{load_global_registry_from, register_project_in, GlobalProjectRegistry};` and its own setup: `let temp = TempTrackerDir::new("registry_pruning");`, `cfg_dir` (create_dir_all), `valid_proj` (create_dir_all) and `TrackerStorage::init(&valid_proj, "ValidWorkspace", "01_init", "Init Phase").unwrap();`. Moved verbatim: the two register_project_in asserts, poisoned_toml, the fs::write of projects.toml, the load_global_registry_from asserts, persisted_raw, parsed_reg. Why Unix-only: the fixture's poisoned root is "/home", and on Windows the `{}`-formatted paths put backslashes into TOML basic strings, where `\U...` is an escape.
4. The original test keeps `temp`, `valid_proj` and `storage` (steps 3 and 4 use them) and drops `cfg_dir`. Remove load_global_registry_from, register_project_in and GlobalProjectRegistry from its `use` list (it keeps is_unsafe_broad_directory, validate_safe_project_root, IngestionOptions).
5. Step 3: wrap `let unsafe_storage = TrackerStorage::new(Path::new("/tmp"));` and its assert in `#[cfg(unix)] { ... }`.
6. Windows-only code rule: every name used only inside a gated block or test is defined or imported inside it. Check each gated block before finishing; the Linux gates can't see this.
7. In the moved fixture, `name = "theinfomoose"` → `name = "testuser"` (a real username in test data; reserved placeholder per repo rules).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'fn test_registry_prunes_poisoned_unix_roots' crates/backend_core/tests/storage_integration.rs && grep -q 'r"C:\\Program Files (x86)"' crates/backend_core/tests/storage_integration.rs && ! sed -n '/fn test_safe_project_root_validation_poisoned_registry_pruning_and_hidden_dir_skip/,/^    };/p' crates/backend_core/tests/storage_integration.rs | grep -q 'register_project_in' && ! grep -q 'theinfomoose' crates/backend_core/tests/storage_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 37C: Git Test Ignores the Runner's core.autocrlf

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 37C: test_git_fetch_ahead_behind_and_workspace_dirty_tracking sets core.autocrlf=false on both of its repos, so checked-out bytes match on Windows runners whose global gitconfig sets autocrlf=true

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/tests/workflow_integration.rs:907-915 (workspace-1 `repo` config block: user.name, user.email), :955-960 (`cloned_repo` config block), :1079 (assert_eq on checked-out conflict.rs; Windows CI: left "...\r\n", right "...\n")
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No assertion changes; no other test changes; no product code changes. This is test isolation, not a workaround: converting line endings on checkout is git's documented autocrlf behaviour on the user's machine, and the test asserts exact bytes.

[PRE_GATE]
grep -q 'fn test_registry_prunes_poisoned_unix_roots' crates/backend_core/tests/storage_integration.rs && ! grep -q 'core.autocrlf' crates/backend_core/tests/workflow_integration.rs

[MUTATION_SPEC]
1. In that test's `repo` config block and its `cloned_repo` config block, add `cfg.set_str("core.autocrlf", "false").unwrap();` after the user.email line. Comment once (at the first): why (Windows runners set core.autocrlf=true globally; the test compares checked-out bytes).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && test "$(grep -c 'core.autocrlf", "false"' crates/backend_core/tests/workflow_integration.rs)" -eq 2

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 37E: Watcher Recognises Its Own Index Writes Through Symlinked Paths

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 37E: the watcher's echo check compares canonical paths, so its own reconciled index writes are suppressed when the tracker path runs through a symlink (macOS /var -> /private/var, where FSEvents reports real paths)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/watcher/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/watcher/mod.rs:53 (watched_dir as given), :174 (own_index_hashes = hash_index_files(&watched_dir_for_worker)), :327-346 (hash_index_files keys = paths under `dir` as given), :358-367 (is_own_index_echo looks up event paths verbatim)
  - crates/backend_core/tests/workflow_integration.rs:735-812 (test_watcher_detects_external_changes_and_auto_reconciles_indexes; macOS CI 2026-11-17 panicked at :810 "watcher must not loop on its own reconciled index writes")
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No changes to the integration test, its timeouts, or its assertions. No change to `TrackerWatcher::watched_dir()` (it keeps returning the path as given), the debounce/settle timings, event filtering, or reconcile logic. No new dependencies.

[PRE_GATE]
grep -q 'core.autocrlf", "false"' crates/backend_core/tests/workflow_integration.rs && ! grep -q 'fn canonical_or_self' crates/backend_core/src/watcher/mod.rs

[MUTATION_SPEC]
1. Add a private helper `fn canonical_or_self(path: &Path) -> PathBuf` returning `fs::canonicalize(path)` or, on error, `path.to_path_buf()`. Doc comment (one line): why — FSEvents reports real paths (/private/var/...) while the tracker path may be given through a symlink (/var/...).
2. In `hash_index_files`, start the walk from `canonical_or_self(dir)` instead of `dir`, so every key is a canonical path.
3. In `is_own_index_echo`, look up `own_index_hashes.get(&canonical_or_self(p))` instead of `get(p)`. Leave `is_derived_index_file(p)` and `hash_file_bytes(p)` as they are.
4. Add at the end of the file `#[cfg(all(test, unix))] mod echo_path_tests` with its own imports inside the module (`use super::{hash_index_files, is_own_index_echo};`, `use std::fs;`, `use std::os::unix::fs::symlink;`, `use std::path::PathBuf;`, and `tempfile::TempDir`). One test `own_index_echo_matches_through_symlinked_path`:
   a. TempDir; create `real/`; write `real/scope.toml` = "a"; `symlink(&real, &link)` where link = tmp/"link".
   b. hashes = hash_index_files(&real); assert `is_own_index_echo(&[link.join("scope.toml")], &hashes)` (event path via symlink, keys via real dir).
   c. hashes = hash_index_files(&link); assert `is_own_index_echo(&[real.join("scope.toml")], &hashes)` (keys via symlink, event path via real dir).
   d. Overwrite `real/scope.toml` with "b"; assert `!is_own_index_echo(&[link.join("scope.toml")], &hashes)` (changed content is not an echo).
   Every name the module uses is imported inside it (Windows-only code rule, inverted: nothing unix-only leaks into shared scope).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'fn canonical_or_self' crates/backend_core/src/watcher/mod.rs && grep -q 'own_index_hashes.get(&canonical_or_self(p))' crates/backend_core/src/watcher/mod.rs && grep -q '#\[cfg(all(test, unix))\]' crates/backend_core/src/watcher/mod.rs && cargo test -p backend_core --lib own_index_echo_matches_through_symlinked_path 2>&1 | grep -q 'test result: ok. 1 passed'

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### User Step: Push and Report CI Matrix (Phase 19)

**[USER]**. After 37E is COMPLETED and Claude has committed. Push `main`, then report each OS ("passed", or the `panicked at` line plus the assertion lines under it for each failure).

```bash
cd /srv/devshare/Nebulous && git push
```


# Phase 20 — Phase 19 Review Pick: chrono → jiff (written 2026-11-17)

**Phase goal (frozen; approved by user 2026-11-17):** replace chrono with jiff 0.2.39 (UTC only, `default-features = false`) across `backend_core` and `nebulous_cli` without changing any timestamp or date string written to disk, then remove chrono and the RUSTSEC-2026-0543 ignore so `cargo deny` passes with zero exceptions. Nothing else.

**Why it's safe for stored data:** every stored timestamp is a `String` field; chrono's `serde` feature is unused. Only the strings we produce matter, and they come from one format (`%Y-%m-%dT%H:%M:%SZ`) and one date format (`%Y-%m-%d`). 38B puts both in helpers with format tests; every later step writes through them.

**Why jiff 0.2.39:** newest release at least two weeks old (2026-11-02). jiff 1.0.0 and 0.2.40–0.2.46 were all published 2026-11-10 or later. Upgrading to 1.0 is parked.

**Order (expand → migrate → contract):** 38B adds `models/time.rs` (the jiff parsers + format helpers) next to the old chrono parsers, so every later step compiles on its own. 38H deletes the old parsers; 38I removes the dependency.

| Step | Does | Files |
| :--- | :--- | :--- |
| 38A | Add jiff 0.2.39 (chrono stays) | `Cargo.toml`, `crates/backend_core/Cargo.toml`, `crates/nebulous_cli/Cargo.toml`, `Cargo.lock` |
| 38B | New `models/time.rs`: jiff parsers + format helpers + tests | `crates/backend_core/src/models/time.rs` (new), `crates/backend_core/src/models/mod.rs` |
| 38C | "now" strings → helpers (1/2) | `workflow/lifecycle.rs`, `storage/mod.rs`, `git_sync/mod.rs` |
| 38D | "now" strings → helpers (2/2) | `ingestion/mod.rs`, `mcp/mod.rs`, `nebulous_cli/tests/cli_integration.rs` |
| 38E | Leaf parsers → jiff types (1/2) | `models/phase.rs`, `models/project.rs`, `models/scope.rs` |
| 38F | Leaf parsers → jiff types (2/2) | `models/registry.rs`, `models/component.rs` |
| 38G | `now`-taking API → `jiff::Timestamp` (one closed set; `/boost`) | `models/item.rs`, `models/timer.rs`, `workflow/timer.rs`, `workflow/metrics.rs`, `workflow/assist.rs`, `nebulous_cli/src/tui/render.rs`, `lib.rs`, `tests/workflow_integration.rs` |
| 38H | Delete the chrono parsers; re-export the jiff ones | `crates/backend_core/src/models/mod.rs` |
| 38I | Remove chrono and the deny ignore | `Cargo.toml`, both crate `Cargo.toml`s, `Cargo.lock`, `deny.toml` |
| [USER] CI | Push and report each OS | — |

**Why 38G has 8 files:** `item.rs::parsed_updated_at` feeds `metrics.rs` and `assist.rs`; `assist.rs` calls `timer_status_at(now)`; `timer_status_at` and `elapsed_minutes_at` are called from `render.rs`, `lib.rs` and `workflow_integration.rs`. Changing any one type breaks the rest, so they must move together. Every edit in it is a row of the mapping table below.

### Phase 20 mapping table (chrono → jiff)

| chrono | jiff |
| :--- | :--- |
| `chrono::Utc::now()` | `jiff::Timestamp::now()` |
| `DateTime<Utc>` | `jiff::Timestamp` |
| `NaiveDate` | `jiff::civil::Date` |
| `NaiveDateTime` | `jiff::civil::DateTime` |
| `x.format("%Y-%m-%dT%H:%M:%SZ").to_string()` | `crate::models::time::format_timestamp(x)` (outside backend_core: `backend_core::time::format_timestamp`) |
| `Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()` | `now_timestamp_string()` |
| `Utc::now().format("%Y-%m-%d").to_string()` | `today_date_string()` |
| `x.to_rfc3339_opts(SecondsFormat::Secs, true)` | `format_timestamp(x)` |
| `(a - b).num_seconds()` | `a.duration_since(b).as_secs()` |
| `(a - b).num_days()` | `a.duration_since(b).as_secs() / 86_400` (truncates toward zero, like `num_days`) |
| `chrono::Duration::minutes(n)` | `jiff::SignedDuration::from_mins(n)` |
| `chrono::Duration::days(n)` | `jiff::SignedDuration::from_hours(n * 24)` |
| `t ± Duration` | `t ± SignedDuration` (`Timestamp` implements `Add`/`Sub<SignedDuration>`) |
| `Utc.with_ymd_and_hms(Y, M, D, h, m, s).unwrap()` | `"YYYY-MM-DDThh:mm:ssZ".parse::<jiff::Timestamp>().unwrap()` |
| `NaiveDate::from_ymd_opt(Y, M, D).unwrap()` | `jiff::civil::date(Y, M, D)` |

Never: `Timestamp::to_string()` / `format!("{ts}")` for stored values (prints fractional seconds, changing the on-disk format); `Timestamp - Timestamp` (gives a `Span`, not a duration).

### STEP 38A: Add jiff 0.2.39

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 38A: the workspace depends on jiff 0.2.39 (std only, no time-zone database) in backend_core and nebulous_cli, locked at exactly 0.2.39; chrono is unchanged

[SCOPE_LOCK]
ALLOWED_FILES:
  - Cargo.toml
  - crates/backend_core/Cargo.toml
  - crates/nebulous_cli/Cargo.toml
  - Cargo.lock
READ_ONLY_REFS:
  - Cargo.toml:21 (`chrono = { version = "0.4", features = ["serde"] }` in [workspace.dependencies])
  - crates/backend_core/Cargo.toml:12, crates/nebulous_cli/Cargo.toml:21 (`chrono.workspace = true`)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES; no .rs changes; anything under archive/. Don't remove or change chrono. Don't run `cargo update` without `-p jiff --precise 0.2.39` (no other crate may change in Cargo.lock).

[PRE_GATE]
grep -q 'RUSTSEC-2026-0543' deny.toml && ! grep -q '^jiff' Cargo.toml

[MUTATION_SPEC]
1. Cargo.toml [workspace.dependencies]: add after the chrono line `jiff = { version = "0.2.39", default-features = false, features = ["std"] }`. Comment on the line above (one line): why — UTC only; no tz database or system time zone needed.
2. crates/backend_core/Cargo.toml and crates/nebulous_cli/Cargo.toml [dependencies]: add `jiff.workspace = true` directly after `chrono.workspace = true`.
3. Run `cargo update -p jiff --precise 0.2.39`. Cargo.lock gains jiff (and nothing else changes version).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q '^jiff = { version = "0.2.39", default-features = false, features = \["std"\] }' Cargo.toml && grep -q '^jiff.workspace = true' crates/backend_core/Cargo.toml && grep -q '^jiff.workspace = true' crates/nebulous_cli/Cargo.toml && cargo tree -p backend_core -i jiff --depth 0 | grep -q '^jiff v0.2.39' && grep -q 'name = "chrono"' Cargo.lock

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 38B: models/time.rs — jiff Parsers and Format Helpers

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 38B: crates/backend_core/src/models/time.rs holds the jiff versions of parse_timestamp_utc and parse_date (same accepted inputs, same errors) plus format_timestamp / now_timestamp_string / today_date_string, all unit-tested; the chrono parsers in models/mod.rs stay until 38H

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/time.rs
  - crates/backend_core/src/models/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/mod.rs:1-7 (`pub mod` list), :35 (chrono import), :73-78 (`LOCAL_DATETIME_FORMATS`), :80-101 (`parse_timestamp_utc`: RFC 3339 → local datetime formats → `YYYY-MM-DD` at 00:00:00Z; error `TrackerError::InvalidDateTime { value, reason }`), :103-124 (`parse_date`: `YYYY-MM-DD` → RFC 3339 date in UTC → local datetime date)
  - crates/backend_core/src/error.rs (`TrackerError::InvalidDateTime`)
  - STEPPED_IMPLEMENTATION_PLAN.md § "Phase 20 mapping table (chrono → jiff)"
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. In models/mod.rs change nothing except adding `pub mod time;` — the chrono parsers, `LOCAL_DATETIME_FORMATS` and the chrono import stay. No `pub use time::...` yet (38H does that; adding it now would clash with the old names).

[PRE_GATE]
grep -q '^jiff.workspace = true' crates/backend_core/Cargo.toml && test ! -e crates/backend_core/src/models/time.rs

[MUTATION_SPEC]
1. models/mod.rs: add `pub mod time;` to the `pub mod` list (alphabetical: after `scope`, before `timer`).
2. New models/time.rs. Module doc (2 lines): jiff-based timestamp/date parsing and the only place stored timestamp strings are formatted; all values are UTC.
   Imports: `use jiff::civil::{Date, DateTime}; use jiff::tz::TimeZone; use jiff::Timestamp; use crate::error::{Result, TrackerError};`
3. `const LOCAL_DATETIME_FORMATS: &[&str]` — same four strings as models/mod.rs:73-78 (jiff strptime uses the same `%Y %m %d %H %M %S %.f` directives).
4. `pub fn parse_timestamp_utc(value: &str) -> Result<Timestamp>` — trim, then in order, first success wins:
   a. `trimmed.parse::<Timestamp>()` (RFC 3339 with `Z` or numeric offset; converted to UTC).
   b. for each fmt in LOCAL_DATETIME_FORMATS: `DateTime::strptime(fmt, trimmed)` then `TimeZone::UTC.to_timestamp(dt)`.
   c. `Date::strptime("%Y-%m-%d", trimmed)` then `TimeZone::UTC.to_timestamp(date.to_datetime(jiff::civil::Time::midnight()))`.
   Otherwise `Err(TrackerError::InvalidDateTime { value: value.to_string(), reason: <exactly the reason string at models/mod.rs:97-98> })`.
5. `pub fn parse_date(value: &str) -> Result<Date>` — trim, then in order: `Date::strptime("%Y-%m-%d", trimmed)`; `trimmed.parse::<Timestamp>()` → `.to_zoned(TimeZone::UTC).date()`; each LOCAL fmt `DateTime::strptime` → `.date()`. Otherwise InvalidDateTime with exactly the reason string at models/mod.rs:120.
6. `pub fn format_timestamp(ts: Timestamp) -> String` → `ts.strftime("%Y-%m-%dT%H:%M:%SZ").to_string()`. Doc: the on-disk timestamp format; whole seconds, `Z` suffix; never use `Timestamp`'s Display for stored values (it prints fractional seconds).
   `pub fn now_timestamp_string() -> String` → `format_timestamp(Timestamp::now())`.
   `pub fn today_date_string() -> String` → `Timestamp::now().strftime("%Y-%m-%d").to_string()` (UTC date).
7. `#[cfg(test)] mod tests` (use super::*), one test per bullet:
   - `"2026-11-01T21:00:00Z"` and `"2026-11-01T23:00:00+02:00"` both parse to the same Timestamp as `"2026-11-01T21:00:00Z".parse::<Timestamp>().unwrap()`.
   - local forms `"2026-11-01T21:00:00"`, `"2026-11-01 21:00:00"`, `"2026-11-01T21:00:00.5"` parse (the first two equal 21:00:00Z; the third's `format_timestamp` is `"2026-11-01T21:00:00Z"`), and `"2026-11-01"` parses to `"2026-11-01T00:00:00Z"`.
   - parse_date: `"2026-11-01"`, `"2026-11-01T21:00:00"`, `"2026-11-02T01:00:00+02:00"` all give `jiff::civil::date(2026, 11, 1)` (the last proves the UTC conversion).
   - `parse_timestamp_utc("not-a-timestamp")` and `parse_date("not-a-date")` are `Err(TrackerError::InvalidDateTime { .. })` (use `matches!`).
   - `format_timestamp("2026-11-01T21:00:00.123456Z".parse().unwrap()) == "2026-11-01T21:00:00Z"`; `now_timestamp_string()` has len 20, ends with 'Z', contains no '.', and round-trips through `parse_timestamp_utc`; `today_date_string()` has len 10 and parses with `parse_date`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q '^pub mod time;' crates/backend_core/src/models/mod.rs && grep -q 'pub fn parse_timestamp_utc(value: &str) -> Result<Timestamp>' crates/backend_core/src/models/time.rs && grep -q 'pub fn parse_date(value: &str) -> Result<Date>' crates/backend_core/src/models/time.rs && grep -q 'pub fn format_timestamp(ts: Timestamp) -> String' crates/backend_core/src/models/time.rs && grep -q 'pub fn now_timestamp_string() -> String' crates/backend_core/src/models/time.rs && grep -q 'pub fn today_date_string() -> String' crates/backend_core/src/models/time.rs && ! grep -q 'chrono' crates/backend_core/src/models/time.rs && grep -q 'use chrono::' crates/backend_core/src/models/mod.rs && test "$(cargo test -p backend_core --lib models::time 2>&1 | grep -o 'test result: ok. [0-9]* passed' | grep -o '[0-9]*' | head -1)" -ge 5

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 38C: "Now" Strings Through the Helpers (1/2)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 38C: lifecycle.rs, storage/mod.rs and git_sync/mod.rs build their "now" timestamp and "today" date strings with now_timestamp_string() / today_date_string() and no longer import chrono

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/workflow/lifecycle.rs
  - crates/backend_core/src/storage/mod.rs
  - crates/backend_core/src/git_sync/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/workflow/lifecycle.rs:4 (`use chrono::Utc;`), :344, :400, :451, :526, :643 (`Utc::now().format("%Y-%m-%d").to_string()`), :1046 (`Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()`)
  - crates/backend_core/src/storage/mod.rs:16 (`use chrono::Utc;`), :283 (timestamp), :284 (date)
  - crates/backend_core/src/git_sync/mod.rs:5 (`use chrono::Utc;`), :1856 (date)
  - crates/backend_core/src/models/time.rs (`now_timestamp_string`, `today_date_string`)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No other edits in these files: same variable names, same control flow, no new comments.

[PRE_GATE]
grep -q 'pub fn today_date_string' crates/backend_core/src/models/time.rs && grep -q 'use chrono::Utc;' crates/backend_core/src/workflow/lifecycle.rs

[MUTATION_SPEC]
1. In each file replace `use chrono::Utc;` with `use crate::models::time::{...};` importing only the helpers that file uses.
2. Replace every `Utc::now().format("%Y-%m-%d").to_string()` with `today_date_string()` and every `Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()` with `now_timestamp_string()` (lifecycle.rs: 5 dates + 1 timestamp; storage/mod.rs: 1 + 1; git_sync/mod.rs: 1 date).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -nE 'chrono::|use chrono|Utc::' crates/backend_core/src/workflow/lifecycle.rs crates/backend_core/src/storage/mod.rs crates/backend_core/src/git_sync/mod.rs && test "$(cat crates/backend_core/src/workflow/lifecycle.rs crates/backend_core/src/storage/mod.rs crates/backend_core/src/git_sync/mod.rs | grep -c 'today_date_string()')" -eq 7 && test "$(cat crates/backend_core/src/workflow/lifecycle.rs crates/backend_core/src/storage/mod.rs | grep -c 'now_timestamp_string()')" -eq 2

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 38D: "Now" Strings Through the Helpers (2/2)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 38D: ingestion/mod.rs and mcp/mod.rs build their "now" strings with the helpers, and the CLI TUI timer test builds its started_at with jiff; none of the three uses chrono

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/ingestion/mod.rs
  - crates/backend_core/src/mcp/mod.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/ingestion/mod.rs:5 (`use chrono::Utc;`), :781, :799, :829 (timestamps)
  - crates/backend_core/src/mcp/mod.rs:6 (`use chrono::Utc;`), :1777 (date), :1851, :1906 (timestamps)
  - crates/nebulous_cli/tests/cli_integration.rs:2546-2558 (`tui_timer_renders_wall_clock_minutes`: `use chrono::{Duration, Utc};`, `(Utc::now() - Duration::minutes(7)).format("%Y-%m-%dT%H:%M:%SZ").to_string()`)
  - crates/backend_core/src/models/time.rs; STEPPED_IMPLEMENTATION_PLAN.md § "Phase 20 mapping table (chrono → jiff)"
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No other edits in these files.

[PRE_GATE]
! grep -q 'use chrono' crates/backend_core/src/workflow/lifecycle.rs && grep -q 'use chrono::Utc;' crates/backend_core/src/mcp/mod.rs

[MUTATION_SPEC]
1. ingestion/mod.rs, mcp/mod.rs: replace `use chrono::Utc;` with `use crate::models::time::{...};` (only the helpers used); each `Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()` → `now_timestamp_string()`, each `Utc::now().format("%Y-%m-%d").to_string()` → `today_date_string()`.
2. cli_integration.rs, inside `tui_timer_renders_wall_clock_minutes` only: replace `use chrono::{Duration, Utc};` with `use backend_core::time::format_timestamp;` and `use jiff::{SignedDuration, Timestamp};`; `started_at` becomes `format_timestamp(Timestamp::now() - SignedDuration::from_mins(7))`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -nE 'chrono::|use chrono|Utc::' crates/backend_core/src/ingestion/mod.rs crates/backend_core/src/mcp/mod.rs crates/nebulous_cli/tests/cli_integration.rs && test "$(cat crates/backend_core/src/ingestion/mod.rs crates/backend_core/src/mcp/mod.rs | grep -c 'now_timestamp_string()')" -eq 5 && grep -q 'today_date_string()' crates/backend_core/src/mcp/mod.rs && grep -q 'SignedDuration::from_mins(7)' crates/nebulous_cli/tests/cli_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 38E: Leaf Parsers to jiff Types (1/2)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 38E: PhaseMetadata::parsed_start_date / parsed_end_date return Option<jiff::civil::Date>, ProjectMetadata::parsed_created_at returns jiff::Timestamp, and scope.rs orders stubs by jiff::Timestamp, all through models::time

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/phase.rs
  - crates/backend_core/src/models/project.rs
  - crates/backend_core/src/models/scope.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/phase.rs:4 (`use chrono::NaiveDate;`), :8 (imports `parse_date` from crate::models), :147-155
  - crates/backend_core/src/models/project.rs:4 (`use chrono::{DateTime, Utc};`), :10 (imports `parse_timestamp_utc`), :56-59
  - crates/backend_core/src/models/scope.rs:113 (`Parsed(chrono::DateTime<chrono::Utc>)`), :121 (`crate::models::parse_timestamp_utc(trimmed)`)
  - crates/backend_core/src/lib.rs:78, :141 (callers use only `.is_ok()` / `.is_some()` — no change needed)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Doc comments: update only the type names they mention. Ordering logic in scope.rs unchanged (jiff::Timestamp is Ord).

[PRE_GATE]
! grep -q 'use chrono' crates/backend_core/src/mcp/mod.rs && grep -q 'use chrono::NaiveDate;' crates/backend_core/src/models/phase.rs

[MUTATION_SPEC]
1. phase.rs: drop `use chrono::NaiveDate;`; take `parse_date` from `crate::models::time` (keep `deserialize_opt_datetime_string` from `crate::models`); both methods return `Result<Option<jiff::civil::Date>>` (import `jiff::civil::Date`).
2. project.rs: drop the chrono import; take `parse_timestamp_utc` from `crate::models::time`; `parsed_created_at` returns `Result<jiff::Timestamp>`; doc says "UTC `Timestamp`".
3. scope.rs: the variant becomes `Parsed(jiff::Timestamp)`; the call becomes `crate::models::time::parse_timestamp_utc(trimmed)`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -nE 'chrono::|use chrono' crates/backend_core/src/models/phase.rs crates/backend_core/src/models/project.rs crates/backend_core/src/models/scope.rs && grep -q 'Parsed(jiff::Timestamp)' crates/backend_core/src/models/scope.rs && grep -q 'models::time' crates/backend_core/src/models/phase.rs && grep -q 'models::time' crates/backend_core/src/models/project.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 38F: Leaf Parsers to jiff Types (2/2)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 38F: registry.rs and component.rs parse through models::time and return jiff::Timestamp, and registry.rs builds its "now" strings with now_timestamp_string(); neither imports chrono

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/registry.rs
  - crates/backend_core/src/models/component.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/registry.rs:6 (`use chrono::{DateTime, Utc};`), :11 (imports `parse_timestamp_utc`), :79-82 (`parsed_last_opened_at`), :287, :367 (`Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()`)
  - crates/backend_core/src/models/component.rs:6 (`use chrono::{DateTime, Utc};`), :10-12 (imports incl. `parse_timestamp_utc`), :536-539, :563-568, :608-613
  - crates/backend_core/tests/storage_integration.rs:876, :1839-1849 (callers use only `.is_ok()` / `.unwrap().is_some()` — no change needed)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Doc comments: update only the type names. Keep `deserialize_*` imports from `crate::models`.

[PRE_GATE]
grep -q 'Parsed(jiff::Timestamp)' crates/backend_core/src/models/scope.rs && grep -q 'use chrono::{DateTime, Utc};' crates/backend_core/src/models/registry.rs

[MUTATION_SPEC]
1. registry.rs: drop the chrono import; take `parse_timestamp_utc` and `now_timestamp_string` from `crate::models::time`; `parsed_last_opened_at` returns `Result<jiff::Timestamp>`; the two `Utc::now().format(...)` strings → `now_timestamp_string()`.
2. component.rs: drop the chrono import; take `parse_timestamp_utc` from `crate::models::time`; `parsed_pinned_at` → `Result<jiff::Timestamp>`, `parsed_verified_at` and `parsed_recorded_at` → `Result<Option<jiff::Timestamp>>`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -nE 'chrono::|use chrono|Utc::|DateTime<' crates/backend_core/src/models/registry.rs crates/backend_core/src/models/component.rs && test "$(grep -c 'now_timestamp_string()' crates/backend_core/src/models/registry.rs)" -eq 2 && grep -q 'models::time' crates/backend_core/src/models/component.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 38G: The now-Taking API Moves to jiff::Timestamp

* **Recommended Command:** `/boost`

```text
[STEP_ID]
STEP 38G: item parsers, the timer model and workflow, metrics, What's Next, the TUI and their tests use jiff::Timestamp / jiff::civil::Date and jiff::SignedDuration; no chrono is left in any .rs file except models/mod.rs's old parsers (38H)

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/item.rs
  - crates/backend_core/src/models/timer.rs
  - crates/backend_core/src/workflow/timer.rs
  - crates/backend_core/src/workflow/metrics.rs
  - crates/backend_core/src/workflow/assist.rs
  - crates/nebulous_cli/src/tui/render.rs
  - crates/backend_core/src/lib.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - STEPPED_IMPLEMENTATION_PLAN.md § "Phase 20 mapping table (chrono → jiff)" — every edit in this step is one of its rows
  - crates/backend_core/src/models/item.rs:4, :318-326 (parsed_created_at / parsed_updated_at), :360-363 (parsed_date)
  - crates/backend_core/src/models/timer.rs:1, :48-58 (parsed_started_at, elapsed_minutes_at)
  - crates/backend_core/src/workflow/timer.rs:1, :35, :42, :54, :113, :117, :161, :185, :189, :194
  - crates/backend_core/src/workflow/metrics.rs:3, :101, :105, :147-148, :304
  - crates/backend_core/src/workflow/assist.rs:6, :318, :325, :338, :410-411
  - crates/nebulous_cli/src/tui/render.rs:2, :51
  - crates/backend_core/src/lib.rs:33 (`use chrono::TimeZone;`), :361-370 (timer elapsed test), :375-392 (parser test: switch its import to `crate::models::time::{parse_date, parse_timestamp_utc}`)
  - crates/backend_core/tests/workflow_integration.rs:5, :281-325, :1534, :1850-1852, :1985, :2021, :2529-2621, :2687, :2710, :2736, :2768
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No logic changes: same clamping (`.max(0)`), same minute/day arithmetic, same test inputs and expected values (only their types change). Public function names unchanged; only `DateTime<Utc>` parameters/returns become `jiff::Timestamp` and `NaiveDate` becomes `jiff::civil::Date`. Don't touch models/mod.rs.

[PRE_GATE]
! grep -q 'use chrono' crates/backend_core/src/models/registry.rs && grep -q 'use chrono::{DateTime, Utc};' crates/backend_core/src/workflow/metrics.rs

[MUTATION_SPEC]
1. models/item.rs: drop the chrono import; parse via `crate::models::time::{parse_date, parse_timestamp_utc}`; parsed_created_at / parsed_updated_at → `Result<jiff::Timestamp>`, parsed_date → `Result<jiff::civil::Date>`.
2. models/timer.rs: parsed_started_at → `Result<jiff::Timestamp>` via models::time; `elapsed_minutes_at(&self, now: jiff::Timestamp)`; seconds = `now.duration_since(start).as_secs().max(0)`.
3. workflow/timer.rs: `Utc::now()` → `jiff::Timestamp::now()`; `started_at` / `stopped_at` / `now` parameters → `jiff::Timestamp`; the two `.format("%Y-%m-%dT%H:%M:%SZ").to_string()` → `format_timestamp(..)` from models::time; elapsed = `now.duration_since(start).as_secs().max(0) as u64`.
4. workflow/metrics.rs: `now` parameters → `jiff::Timestamp`; `Utc::now()` → `Timestamp::now()`; `days_since = now.duration_since(updated_dt).as_secs() / 86_400`.
5. workflow/assist.rs: same for `compute_whats_next_at` and line 411: `.map(|dt| now.duration_since(dt).as_secs() / 86_400)`.
6. tui/render.rs: `use chrono::Utc;` → `use jiff::Timestamp;`; `elapsed_minutes_at(Timestamp::now())`.
7. lib.rs tests: drop `use chrono::TimeZone;`; `later` / `earlier` / expected values via `.parse::<jiff::Timestamp>()` / `jiff::civil::date(..)` (mapping table); parser test imports from `crate::models::time`.
8. workflow_integration.rs: replace `use chrono::{TimeZone, Utc};` with `use jiff::{SignedDuration, Timestamp};` and `use backend_core::time::format_timestamp;` (only what's used); rewrite each chrono expression with its mapping-table row. :1852 becomes `format_timestamp(eval_now - SignedDuration::from_hours(10 * 24))`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && test -z "$(grep -rlE 'chrono::|use chrono' crates --include=*.rs | grep -v '^crates/backend_core/src/models/mod.rs$')" && ! grep -rn 'Timestamp::now().to_string()' crates --include=*.rs && grep -q 'now: jiff::Timestamp\|now: Timestamp' crates/backend_core/src/workflow/metrics.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 38H: Delete the chrono Parsers

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 38H: models/mod.rs no longer contains chrono code; parse_timestamp_utc and parse_date are re-exported from models::time so backend_core::{parse_timestamp_utc, parse_date} still resolve

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/models/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/models/mod.rs:35 (chrono import), :73-78 (LOCAL_DATETIME_FORMATS), :80-124 (old parsers)
  - crates/backend_core/src/models/time.rs (replacements)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Keep `deserialize_datetime_string` / `deserialize_opt_datetime_string` and everything else in mod.rs unchanged.

[PRE_GATE]
test -z "$(grep -rlE 'chrono::|use chrono' crates --include=*.rs | grep -v '^crates/backend_core/src/models/mod.rs$')" && grep -q 'use chrono::' crates/backend_core/src/models/mod.rs

[MUTATION_SPEC]
1. Delete the chrono import, `LOCAL_DATETIME_FORMATS`, and the chrono `parse_timestamp_utc` and `parse_date` (with their doc comments).
2. Add `pub use time::{parse_date, parse_timestamp_utc};` with the other `pub use` lines.
3. If `TrackerError` becomes unused in mod.rs, trim the `use crate::error::...` line to what's still used.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -rnE 'chrono::|use chrono' crates --include=*.rs && grep -q '^pub use time::{parse_date, parse_timestamp_utc};' crates/backend_core/src/models/mod.rs && ! grep -q 'LOCAL_DATETIME_FORMATS' crates/backend_core/src/models/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 38I: Remove chrono and the Advisory Ignore

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 38I: chrono is gone from every manifest and from Cargo.lock, and deny.toml has no ignore entries, so cargo deny passes with zero exceptions (restores the zero-advisories constraint)

[SCOPE_LOCK]
ALLOWED_FILES:
  - Cargo.toml
  - crates/backend_core/Cargo.toml
  - crates/nebulous_cli/Cargo.toml
  - Cargo.lock
  - deny.toml
READ_ONLY_REFS:
  - Cargo.toml:21 (chrono workspace dep), crates/backend_core/Cargo.toml:12, crates/nebulous_cli/Cargo.toml:21
  - deny.toml (the `ignore = [ ... ]` list holding only RUSTSEC-2026-0543 and its two TEMPORARY comment lines)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES; anything under archive/ (excluded from the workspace; its manifest isn't built). Don't change any other dependency; Cargo.lock may only lose chrono and crates nothing else needs (cargo does this itself on the next build).

[PRE_GATE]
! grep -rnE 'chrono::|use chrono' crates --include=*.rs && grep -q 'RUSTSEC-2026-0543' deny.toml

[MUTATION_SPEC]
1. Delete the chrono line from Cargo.toml [workspace.dependencies] and `chrono.workspace = true` from both crate manifests.
2. Run `cargo build --workspace` so Cargo.lock drops chrono.
3. deny.toml: delete the RUSTSEC-2026-0543 entry and its two TEMPORARY comment lines, leaving `ignore = []`. Keep the "Permanent constraint: zero open RustSec/GHSA advisories" comment.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -q 'chrono' Cargo.toml crates/backend_core/Cargo.toml crates/nebulous_cli/Cargo.toml && ! grep -q 'name = "chrono"' Cargo.lock && ! grep -q 'RUSTSEC-2026-0543' deny.toml && grep -qE '^ignore = \[\]' deny.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### User Step: Push and Report CI Matrix (Phase 20)

**[USER]**. After 38I is COMPLETED and Claude has committed. Push `main`, then report each OS ("passed", or the `panicked at` line plus the assertion lines under it for each failure).

```bash
cd /srv/devshare/Nebulous && git push
```


# Phase 21 — Phase 20 Review Picks: Beta Prep (written 2026-11-17)

**Phase goal (frozen once approved):** prepare Nebulous for its `v0.1.0` beta. Fix three parked wording items (zero-ID error suggestion, MCP `dry_run` text, MCP `list_items` description). Add a changelog. Make releases version-checked: the tag must match `Cargo.toml`, the notes come from the dated changelog section, and `0.x` releases are marked pre-release. Add a bump script. End with a read-only Orrery ↔ Nebulous cooperation audit whose findings feed Phase 22. Phase 21 cuts no tag; `v0.1.0` waits until the audit's blockers are fixed.

**User picks (2026-11-17):** Parked items 1 (assist wording), 2 (MCP `dry_run` text), 4 (`list_items` description), 3 (beta + release workflow: tag/version guard, bump script, changelog) and 6 (Orrery ↔ Nebulous audit, run before any release). Item 5 (jiff 1.0) stays parked.

**Already true:** `Cargo.toml` `[workspace.package] version = "0.1.0"` (beta under the user's convention). `.github/workflows/release.yml` already builds, packages and publishes the Linux x86_64 archive on `v*` tags. Phase 21 adds the guard, the notes and the pre-release flag to it, and doesn't rebuild it.

**Release flow after Phase 21:** notable changes go under `## [Unreleased]` in `CHANGELOG.md` → `scripts/bump-version.sh X.Y.Z` sets both version strings in `Cargo.toml`, runs `cargo update --workspace`, renames `## [Unreleased]` to `## [X.Y.Z] - <UTC date>` (with a fresh empty `## [Unreleased]` above it), runs `release-check.sh`, then prints the git commands → the user commits, tags and pushes → `release.yml` runs `scripts/release-check.sh vX.Y.Z` before building. That check fails on a version mismatch or a missing, undated or empty section, and its output becomes the release notes.

| Step | Does | Files |
| :--- | :--- | :--- |
| 39A | Zero-ID error suggests `neb next` instead of running What's Next | `workflow/assist.rs`, `tests/workflow_integration.rs` |
| 39B | MCP `dry_run` text for 3 tools + `list_items` description | `mcp/mod.rs` |
| 39C | `CHANGELOG.md` + `scripts/release-check.sh` | `CHANGELOG.md` (NEW), `scripts/release-check.sh` (NEW) |
| 39D | `release.yml`: guard, version check, notes, pre-release | `.github/workflows/release.yml` |
| 39E | `scripts/bump-version.sh` + README "Releasing" | `scripts/bump-version.sh` (NEW), `README.md` |
| [USER] CI | Push and report each OS | — |
| [CLAUDE] AUDIT | Orrery ↔ Nebulous cooperation audit (read-only) | — (report goes in this file) |

### STEP 39A: Zero-ID Error Suggests `neb next`

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 39A: resolve_active_or_inferred_item_id no longer runs a full What's Next to build its error text; both error messages tell the user to pass an item ID or run `neb next`

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/workflow/assist.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/workflow/assist.rs:214-297 (`resolve_active_or_inferred_item_id`; doc item 4 at :219; multi-in-progress branch :248-259; `suggestion_suffix` :274-283; the two final `Err(..)` :285-296)
  - crates/backend_core/tests/workflow_integration.rs:1895-1902 (asserts the error contains "BUG-0003", the old top recommendation)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Steps 1–3 of the resolution order and the success paths stay unchanged. Don't touch `compute_whats_next`, `predict_defaults` or any other function.

[PRE_GATE]
grep -q 'Top recommended item: {top_id}' crates/backend_core/src/workflow/assist.rs

[MUTATION_SPEC]
1. assist.rs, multi-in-progress-in-active-phase branch (:248-259): delete the `let suggested = self.compute_whats_next(None)...;` statement. The message becomes exactly:
   "No active timer found and multiple in_progress items exist in '{}' ({}). Pass one of these IDs explicitly, or run `neb next` for ranked recommendations."
   with the same two arguments (`active_phase.phase_id`, `in_progress_in_active.join(", ")`).
2. assist.rs :274-283: replace the whole `let suggestion_suffix = match self.compute_whats_next(None)... };` with
   `let suggestion_suffix = " Pass an item ID explicitly, or run `neb next` for ranked recommendations.";`
   Both `Err(..)` format strings after it stay as they are.
   Why: the error path ran a full What's Next (index load, scoring, git status) only to name one item ID.
3. assist.rs:219, doc item 4: replace "and suggesting the top recommended item ID." with "and pointing the user to `neb next`."
4. workflow_integration.rs:1895-1902: the comment becomes `// 2. When no timer or in_progress item exists, returns ValidationError pointing to `neb next``, and the `matches!` guard becomes `if msg.contains("neb next")`. Keep the `expect_err` text.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! sed -n '/pub fn resolve_active_or_inferred_item_id/,/^    }$/p' crates/backend_core/src/workflow/assist.rs | grep -q 'compute_whats_next' && test "$(grep -c 'run `neb next` for ranked recommendations' crates/backend_core/src/workflow/assist.rs)" -eq 2 && grep -q 'msg.contains("neb next")' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 39B: MCP `dry_run` and `list_items` Descriptions Say What the Tools Do

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 39B: the dry_run descriptions of switch_project, ingest_project_directory and rollback_to_pinned_release describe each tool's own dry run, and list_items mentions tasks

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:571 (`list_items` description), :676 (switch_project dry_run), :691 (ingest_project_directory dry_run), :806 (rollback_to_pinned_release dry_run)
  - crates/backend_core/src/mcp/mod.rs:1457-1471 (switch_project dry run: returns `would_switch_to`), :1504-1506 and :1548-1558 (ingest: dry only when a write flag is set), :2088-2096 (rollback dry run: `would_restore`, `last_known_good`, `would_record_regression_on`)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Only these four string literals change. The git_pull / git_push / git_sync dry_run strings (:837, :848, :877) stay as they are. No schema keys, defaults or handler code change.

[PRE_GATE]
grep -q 'Lists tracked bugs and features with optional' crates/backend_core/src/mcp/mod.rs

[MUTATION_SPEC]
1. :571 list_items description becomes exactly: "Lists tracked bugs, features and tasks with optional phase, scope, status, type, stagnant, and recurring filters."
2. :676 switch_project `dry_run` description becomes exactly: "When true (default), returns the project it would switch to without changing the registry. Pass false to switch."
3. :691 ingest_project_directory `dry_run` description becomes exactly: "When true (default) and write_manifests, promote_all or promote_phase_id is set, scans and reports what would be written without writing manifests or promoting scopes. A scan with no write flags never writes. Pass false to write."
4. :806 rollback_to_pinned_release `dry_run` description becomes exactly: "When true (default), returns the pinned release it would restore and the item it would record a regression on, without writing. Pass false to roll back."

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && test "$(grep -c 'reports git status and what would happen' crates/backend_core/src/mcp/mod.rs)" -eq 3 && grep -q 'Lists tracked bugs, features and tasks with optional' crates/backend_core/src/mcp/mod.rs && grep -q 'returns the project it would switch to without changing the registry' crates/backend_core/src/mcp/mod.rs && grep -q 'A scan with no write flags never writes' crates/backend_core/src/mcp/mod.rs && grep -q 'returns the pinned release it would restore' crates/backend_core/src/mcp/mod.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 39C: Changelog and the Release Check Script

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 39C: CHANGELOG.md exists with the 0.1.0 beta notes under ## [Unreleased], and scripts/release-check.sh <tag> verifies the tag against Cargo.toml and prints that version's dated changelog section

[SCOPE_LOCK]
ALLOWED_FILES:
  - CHANGELOG.md (NEW)
  - scripts/release-check.sh (NEW)
READ_ONLY_REFS:
  - Cargo.toml:9-10 (`[workspace.package]` / `version = "0.1.0"`)
  - scripts/check-neutral.sh (style reference: `set -euo pipefail`, `cd "$(git rev-parse --show-toplevel)"`)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Don't change Cargo.toml or any workflow. The script never writes files.

[PRE_GATE]
test ! -e CHANGELOG.md && test ! -e scripts/release-check.sh && grep -q '^version = "0.1.0"' Cargo.toml

[MUTATION_SPEC]
1. CHANGELOG.md, exactly this content (Keep a Changelog layout):
   # Changelog

   All notable changes to Nebulous are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
   Versions: `0.0.x` = alpha, `0.1.x` = beta, `1.0.0` = first stable release.

   ## [Unreleased]

   First beta (0.1.0).

   ### Added
   - Task items (`TASK-XXXX`): `neb task` and the TUI `t` key.
   - `neb plan import` and the MCP `plan_import` tool import roadmap tables from a stepped plan file (dry run by default).
   - Opt-in hash item IDs (`id_scheme = "hash"`).
   - Schema versions on all four tracker tiers. Unknown keys are preserved, and items written by a newer version are skipped and reported.
   - Timer: per-session cap, idempotent stop, and quarantine of a corrupt or orphaned timer file.
   - MCP: `dry_run` (default true) on tools that write or use the network, and protocol version negotiation.
   - TUI: live reload, wall-clock timer, and clipboard copy with Wayland and OSC 52 fallback.

   ### Changed
   - Every tracker write takes a cross-process lock on `.tracker/.lock`, and vault writes take their own file lock.
   - MCP `tools/call` results carry `content`, `structuredContent` and `isError`, and stdio framing recovers from bad frames.
   - Dates and times use jiff instead of chrono. Stored formats are unchanged.
   - When no item can be inferred, the error suggests `neb next` instead of naming a recommendation.
   - CI runs on Linux, macOS and Windows.

   ### Removed
   - The release engine and `neb release` (moved to a separate project).

   ### Security
   - System directories can't be project roots on Linux, macOS (including `/private/...`) or Windows.
   - Vault files get user-only permissions on Windows (ACL), as on Unix.
   - `cargo deny` passes with zero open RustSec advisories.

   Releases v0.0.1–v0.0.4 predate this file; see their GitHub Release notes.

2. scripts/release-check.sh (NEW, bash; `chmod +x`). Usage: `release-check.sh [--root DIR] <tag>`. Starts with `set -euo pipefail`.
   a. Parse args: optional `--root DIR` (default: `git rev-parse --show-toplevel`), then exactly one tag. Wrong arg count, or a tag not matching `^v[0-9]+\.[0-9]+\.[0-9]+$` → usage message to stderr, exit 2.
   b. VERSION="${tag#v}". Read the `version = "..."` line inside the `[workspace.package]` table of "$ROOT/Cargo.toml" with awk (from the `[workspace.package]` line to the next line starting with `[`). If it isn't VERSION → stderr `release-check: tag <tag> does not match Cargo.toml version <cargo version>`, exit 1.
   c. In "$ROOT/CHANGELOG.md" find the line that exactly matches `## [VERSION] - YYYY-MM-DD` (dots in VERSION escaped; date `[0-9]{4}-[0-9]{2}-[0-9]{2}`). Missing → stderr `release-check: CHANGELOG.md has no dated '## [VERSION] - YYYY-MM-DD' section`, exit 1.
   d. The body is every line after that heading up to (not including) the next line starting with `## [`, or the end of the file. Leading and trailing blank lines are dropped. A body with no non-blank line → stderr `release-check: CHANGELOG.md section for VERSION is empty`, exit 1.
   e. Print the body to stdout and exit 0. Write no files.
   Why: release.yml (39D) and bump-version.sh (39E) both call this, so the rules live in one place and can be tested locally.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && bash scripts/check-neutral.sh CHANGELOG.md scripts/release-check.sh && bash -n scripts/release-check.sh && test -x scripts/release-check.sh && grep -qx '## \[Unreleased\]' CHANGELOG.md && T="$(mktemp -d)" && printf '[workspace]\nmembers = []\n\n[workspace.package]\nversion = "1.2.3"\nedition = "2021"\n\n[workspace.dependencies]\nx = { version = "9.9.9" }\n' > "$T/Cargo.toml" && printf '# Changelog\n\n## [Unreleased]\n\n- Pending.\n\n## [1.2.3] - 2026-01-02\n\n- Added a thing.\n\n## [1.2.2] - 2026-01-01\n\n- Old.\n' > "$T/CHANGELOG.md" && test "$(bash scripts/release-check.sh --root "$T" v1.2.3)" = "- Added a thing." && ! bash scripts/release-check.sh --root "$T" v1.2.2 2>/dev/null && ! bash scripts/release-check.sh --root "$T" v9.9.9 2>/dev/null && { bash scripts/release-check.sh --root "$T" 1.2.3 2>/dev/null; test $? -eq 2; } && { bash scripts/release-check.sh --root "$T" 2>/dev/null; test $? -eq 2; } && printf '# Changelog\n\n## [1.2.3] - 2026-01-02\n\n\n## [1.2.2] - 2026-01-01\n\n- Old.\n' > "$T/CHANGELOG.md" && ! bash scripts/release-check.sh --root "$T" v1.2.3 2>/dev/null && ! bash scripts/release-check.sh v0.1.0 2>/dev/null && rm -rf "$T"

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 39D: Release Workflow Checks the Tag and Publishes Changelog Notes

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 39D: release.yml refuses a tag that doesn't match Cargo.toml or has no dated changelog section before it builds anything, checks `neb --version`, publishes the changelog section as the release notes, and marks 0.x releases as pre-release

[SCOPE_LOCK]
ALLOWED_FILES:
  - .github/workflows/release.yml
READ_ONLY_REFS:
  - .github/workflows/release.yml (whole file: job `build-linux-x86_64` steps "Checkout repository" → "Upload release archive and checksum"; job `publish` → "Publish GitHub Release" with `generate_release_notes: true`)
  - scripts/release-check.sh (39C: `release-check.sh <tag>` prints the notes, exits non-zero on mismatch)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Keep the trigger, permissions, jobs, the dependency install, the toolchain, the cache, the deny and verification steps, the packaging and the smoke test as they are, except for the edits below.

[PRE_GATE]
test -x scripts/release-check.sh && grep -q 'generate_release_notes: true' .github/workflows/release.yml

[MUTATION_SPEC]
1. build-linux-x86_64: a new step directly after "Checkout repository":
       - name: Release guard (tag matches Cargo.toml, dated CHANGELOG section)
         run: |
           mkdir -p dist
           bash scripts/release-check.sh "${GITHUB_REF_NAME}" > dist/RELEASE_NOTES.md
   Why: a bad tag fails in seconds, before the dependency install and the build.
2. "Verify release binary (`neb --help`)": rename it to "Verify release binary (`neb --help`, `neb --version`)" and add a second run line:
       test "$(./target/release/neb --version)" = "neb ${GITHUB_REF_NAME#v}"
   (the step's `run:` becomes a `|` block holding both lines).
3. Packaging step: `cp README.md LICENSE "${STAGING_DIR}/"` becomes `cp README.md LICENSE CHANGELOG.md "${STAGING_DIR}/"`.
4. "Upload release archive and checksum": add `dist/RELEASE_NOTES.md` as a third line under `path:`.
5. publish → "Publish GitHub Release" `with:`: delete `generate_release_notes: true`; add `body_path: dist/RELEASE_NOTES.md` and `prerelease: ${{ startsWith(github.ref_name, 'v0.') }}`. `files:` stays the two archive patterns, so the notes file isn't uploaded as an asset.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && python3 -c 'import sys,yaml; yaml.safe_load(open(".github/workflows/release.yml"))' && grep -qF 'bash scripts/release-check.sh "${GITHUB_REF_NAME}" > dist/RELEASE_NOTES.md' .github/workflows/release.yml && grep -qF 'test "$(./target/release/neb --version)" = "neb ${GITHUB_REF_NAME#v}"' .github/workflows/release.yml && grep -q 'cp README.md LICENSE CHANGELOG.md' .github/workflows/release.yml && grep -q 'body_path: dist/RELEASE_NOTES.md' .github/workflows/release.yml && grep -qF "prerelease: \${{ startsWith(github.ref_name, 'v0.') }}" .github/workflows/release.yml && ! grep -q 'generate_release_notes' .github/workflows/release.yml && test "$(grep -n 'Release guard' .github/workflows/release.yml | cut -d: -f1)" -lt "$(grep -n 'Install Linux system dependencies' .github/workflows/release.yml | cut -d: -f1)"

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 39E: Version Bump Script and README "Releasing"

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 39E: scripts/bump-version.sh [--dry-run] X.Y.Z sets the workspace version, refreshes Cargo.lock, dates the changelog's Unreleased section and prints the commit/tag commands without running them; README documents the release flow

[SCOPE_LOCK]
ALLOWED_FILES:
  - scripts/bump-version.sh (NEW)
  - README.md
READ_ONLY_REFS:
  - Cargo.toml:9-10 (`[workspace.package]` `version = "0.1.0"`), :16 (`backend_core = { path = "crates/backend_core", version = "0.1.0" }`; the second version string)
  - CHANGELOG.md (39C: `## [Unreleased]` heading)
  - scripts/release-check.sh (39C: `release-check.sh [--root DIR] <tag>`)
  - README.md:171 (`## Verification & Testing`), :184 (`## License`)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. The script never runs git commit, git tag or git push. It changes only those two version strings in Cargo.toml (no other dependency in Cargo.lock), and in CHANGELOG.md only the `## [Unreleased]` heading line. The step's own diff must not change Cargo.toml, Cargo.lock or CHANGELOG.md.

[PRE_GATE]
test -x scripts/release-check.sh && grep -qx '## \[Unreleased\]' CHANGELOG.md && grep -q 'body_path: dist/RELEASE_NOTES.md' .github/workflows/release.yml

[MUTATION_SPEC]
1. scripts/bump-version.sh (NEW, bash, `chmod +x`, `set -euo pipefail`, `cd "$(git rev-parse --show-toplevel)"`). Usage: `bump-version.sh [--dry-run] X.Y.Z`.
   a. Args: optional `--dry-run`, then exactly one version matching `^[0-9]+\.[0-9]+\.[0-9]+$`. Anything else → usage to stderr, exit 2.
   b. CURRENT = the `version` in Cargo.toml's `[workspace.package]` table (same awk rule as release-check.sh). If NEW != CURRENT and NEW sorts before CURRENT (`sort -V`) → stderr `bump-version: X.Y.Z is lower than the current CURRENT`, exit 1.
   c. CHANGELOG.md must have exactly one line `## [Unreleased]` and at least one non-blank line between it and the next `## [` line (or the end of the file); otherwise stderr `bump-version: CHANGELOG.md needs a non-empty ## [Unreleased] section`, exit 1. If a line starting with `## [X.Y.Z]` already exists → stderr `bump-version: CHANGELOG.md already has a X.Y.Z section`, exit 1.
   d. DATE="$(date -u +%Y-%m-%d)". `--dry-run`: print `Cargo.toml: CURRENT -> X.Y.Z` (or `Cargo.toml: already X.Y.Z` when equal) and `CHANGELOG.md: ## [Unreleased] -> ## [X.Y.Z] - DATE`, write nothing, exit 0.
   e. Real run: if NEW != CURRENT, rewrite Cargo.toml (awk into a temp file, then `mv`; no `sed -i`) so that the `version = "CURRENT"` line inside `[workspace.package]` and the `version = "CURRENT"` inside the `backend_core = { path = "crates/backend_core", ... }` line read NEW, then run `cargo update --workspace`. Rewrite CHANGELOG.md the same way: the `## [Unreleased]` line becomes the three lines `## [Unreleased]`, an empty line, and `## [X.Y.Z] - DATE`.
   f. Run `bash scripts/release-check.sh "vX.Y.Z" > /dev/null`. On failure, exit 1 with stderr `bump-version: release-check failed; fix the files above before tagging`.
   g. Print exactly:
        Next (run these yourself):
          git add Cargo.toml Cargo.lock CHANGELOG.md
          git commit -m "Release vX.Y.Z"
          git tag vX.Y.Z
          git push origin main vX.Y.Z
2. README.md: a new section directly above `## License`:
   ## Releasing

   Versions: `0.0.x` = alpha, `0.1.x` = beta, `1.0.0` = first stable release. Add notable changes under `## [Unreleased]` in `CHANGELOG.md` as they land.

   1. Preview with `scripts/bump-version.sh --dry-run X.Y.Z`, then run `scripts/bump-version.sh X.Y.Z`. It sets the version in `Cargo.toml`, refreshes `Cargo.lock`, dates the changelog section, and prints the commit and tag commands. It never commits, tags or pushes.
   2. Run the printed commands. Pushing the `vX.Y.Z` tag starts `.github/workflows/release.yml`.
   3. Before building, the release workflow runs `scripts/release-check.sh vX.Y.Z`. That check stops the release if the tag doesn't match `Cargo.toml` or the changelog has no dated, non-empty `## [X.Y.Z]` section. `0.x` releases are published as pre-releases, with the changelog section as the release notes.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && bash scripts/check-neutral.sh scripts/bump-version.sh && bash -n scripts/bump-version.sh && test -x scripts/bump-version.sh && bash scripts/bump-version.sh --dry-run 0.1.1 | grep -q 'Cargo.toml: 0.1.0 -> 0.1.1' && bash scripts/bump-version.sh --dry-run 0.1.1 | grep -q '## \[0.1.1\] - [0-9]' && bash scripts/bump-version.sh --dry-run 0.1.0 | grep -q 'Cargo.toml: already 0.1.0' && ! bash scripts/bump-version.sh --dry-run 0.0.9 2>/dev/null && { bash scripts/bump-version.sh 1.2 2>/dev/null; test $? -eq 2; } && git diff --quiet -- Cargo.toml Cargo.lock CHANGELOG.md && W="$(mktemp -d)/wt" && git worktree add -q --detach "$W" HEAD && cp scripts/bump-version.sh "$W/scripts/" && (cd "$W" && bash scripts/bump-version.sh 0.1.1 | grep -q 'git tag v0.1.1' && grep -q '^version = "0.1.1"' Cargo.toml && grep -q 'backend_core = { path = "crates/backend_core", version = "0.1.1" }' Cargo.toml && grep -qx '## \[0.1.1\] - [0-9-]*' CHANGELOG.md && grep -qx '## \[Unreleased\]' CHANGELOG.md && test "$(git diff --name-only | LC_ALL=C sort | tr '\n' ' ')" = "CHANGELOG.md Cargo.lock Cargo.toml " && test -z "$(git tag -l v0.1.1)") && git worktree remove --force "$W" && grep -q '^## Releasing' README.md && grep -q 'scripts/bump-version.sh --dry-run X.Y.Z' README.md

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### User Step: Push and Report CI Matrix (Phase 21)

**[USER]**. After 39E is COMPLETED and Claude has committed. Push `main`, then report each OS ("passed", or the `panicked at` line plus the assertion lines under it for each failure). Don't push a tag. `release.yml` runs only on tags, and the first tag waits for the audit.

```bash
cd /srv/devshare/Nebulous && git push
```

### Claude Step: Orrery ↔ Nebulous Cooperation Audit

**[CLAUDE]**, read-only, after the Phase 21 CI is green. I edit nothing in either repo except this plan file. Orrery-side fixes go into Orrery's own plan, and the user applies them there.

1. **MCP handshake:** compare the `protocolVersion` Orrery's `rmcp` client sends with `SUPPORTED_MCP_PROTOCOL_VERSIONS` (`mcp/mod.rs:24`), plus the `initialize` → `notifications/initialized` order and how Orrery reads the `tools/call` result (`content` / `structuredContent` / `isError`).
2. **Tool contract:** check every Nebulous tool name and argument Orrery calls (its lifecycle hooks) against `tools/list`, covering names, types and aliases. Check that Orrery passes `dry_run: false` wherever it means to write, since Nebulous defaults to `true`.
3. **Plan format parity:** compare Orrery's `### STEP` / roadmap parsing with Nebulous's `planning` parser: the ID regex, the table columns, and the Up Next / Completed headings.
4. **Shared palette:** compare the amber values on both sides.
5. **Overlap and standalone use:** list any feature both repos implement. Check that each repo runs without the other (Orrery with no `neb` on PATH, and `neb` without Orrery).
6. **Live check:** if Orrery can spawn `neb mcp`, run that against a throwaway tracker in the scratchpad (never the real `.tracker/`), and record the handshake and one read-only tool call.
7. **Deliverable:** a "Phase 21 audit report" section in this file. Each finding gets file:line evidence from both repos, a side (Nebulous / Orrery / both), and a severity (`blocker for v0.1.0` / should-fix / nice-to-have). The user then picks Phase 22 from it, and `v0.1.0` gets tagged after Phase 22's blockers are fixed.

## Phase 21 audit report — Orrery ↔ Nebulous cooperation (2026-11-18)

Read-only. Nebulous at `94b924f`, Orrery at `d02453d` (rmcp 3.9.0). Live check: Orrery's own `live_mcp` test (`cargo test --locked -p orrery-core --features live-backends --test live_mcp`) run against `neb --root <scratch>/ws mcp` through a wrapper that pins `HOME`/`XDG_*` and the tracker to the scratchpad; the JSON-RPC traffic was tee'd to a log. Then every Nebulous tool was called once with Orrery's exact hook payload. The real `.tracker/` and the real `projects.toml` were not touched. Orrery's code never names Nebulous: it's a generic MCP client, and the two meet only through MCP config.

### Findings

| # | Side | Severity | Finding | Evidence | Suggested fix |
|---|---|---|---|---|---|
| A1 | both | **blocker for v0.1.0** | Orrery hooks and gate tools can't pass arguments. Every call sends only `{"orrery": {project_name, run_id, step_number, step_title, plan_step_sha256, event}}`, and `HookConfig` has just `tool` + `on_failure`. So Orrery can't send `dry_run: false`, an `item_id`, a `status` or a `summary`. Probe result: 16 of 24 Nebulous tools fail on that payload (missing required field); the 5 read tools and `git_status` work; `git_pull`/`git_push`/`git_sync` only ever dry-run. Net: Orrery can read Nebulous and gate on it, but can't drive any Nebulous write except A2's accidental commit. The spec's "version control … through MCP tools (Nebulous …) at lifecycle hooks" isn't achievable today. | Orrery `engine.rs:1316-1325` (hook args), `engine.rs:1411-1420` (gate args), `config.rs:62-66` (`HookConfig`). Nebulous `mcp/mod.rs` `tools/list` `required` arrays; probe output (scratchpad `live/`). | Orrery: add optional static `args` (JSON object) to `HookConfig` and to gate entries, merged with the `orrery` object. That's domain-neutral and fits Orrery's spec. Nebulous (optional, for per-step writes): let tools accept the `orrery` object and resolve the item from `step_title`'s step ID (`plan_import` already makes one TASK per roadmap step). Pick one in Phase 22. |
| A2 | Nebulous | **blocker for v0.1.0** | `git_commit_documented` makes a real commit when `summary` is missing. Its schema says `"required": ["summary"]`, but the args struct has `#[serde(default)] summary: String`, so `""` is accepted and replaced with the default `[TRACKER](sync) sync .tracker metadata state`. The tool has no `dry_run` either. Configured as an Orrery gate or hook, it commits `.tracker/` on every step. That also contradicts the 0.1.0 changelog line "`dry_run` (default true) on tools that write". | `mcp/mod.rs:865` (schema), `mcp/mod.rs:2242` (`summary: String` with `serde(default)`), `git_sync/mod.rs:1753` (fallback message); probe made commit `03eaaa1` in the throwaway repo; `CHANGELOG.md:16`. | Reject an empty `summary` (enforce the schema), and add `dry_run` (default true) like the other git writers. Otherwise, narrow the changelog line. |
| A3 | Orrery | should-fix | Orrery numbers steps by position, so `step_number` isn't the plan's step ID. Nebulous-style headings (`### STEP 13A:`) all parse as "unnumbered", so the 139 steps become 1..139 (13A → 1). Hooks get the wrong number, and only `step_title` carries `13A`. One plain `### STEP 40:` heading would make ingest fail outright ("unnumbered step where step N was expected"). Orrery also has no notion of the Completed table, so a run starts at step 1. | Orrery `plan.rs:67-79` (`parse_title_number`), `plan.rs:143-175`; `orrery ingest plan.md --marker '### STEP '` on a scratch copy listed all 139 from 13A. | Orrery: keep the raw ID (`^[0-9]+[A-Z]*$`) as `step_id` in `PlanStep` and the hook payload, and allow a start-at-step option. |
| A4 | Nebulous | nice-to-have | `plan_import` is the only tool with `#[serde(deny_unknown_fields)]`, so it rejects the `orrery` key ("unknown field `orrery`") where every other tool ignores it. | `mcp/mod.rs:933` | Drop `deny_unknown_fields` there (or document the strictness). |
| A5 | Nebulous | nice-to-have | Protocol: rmcp 3.9.0 asks for `2026-07-28`; Nebulous answers `2025-06-18` and rmcp accepts it. It works today, but Nebulous skips `2025-11-25` (rmcp's `LATEST_WITH_INITIALIZE`), which is the version a future initialize-only client would ask for. | trace `c2s.log` line 1 / `s2c.log` line 1; `mcp/mod.rs:24` | Add `2025-11-25` to `SUPPORTED_MCP_PROTOCOL_VERSIONS` after checking its spec delta. |
| A6 | Nebulous | nice-to-have | `git_fetch` in dry run errors when `origin` doesn't exist, while `git_pull`/`git_push`/`git_sync` dry runs return status. That's a hook-failure trap in a repo with no remote. | probe output: `git_fetch` isError "remote 'origin' does not exist" | Make the `git_fetch` dry run report status like the others. |

### Verified working (no action)
- Handshake order `initialize` → `notifications/initialized` → `tools/list` → `tools/call`; Orrery's live test passes.
- `tools/list` returns all 24 tools in one page (no `nextCursor`); rmcp's `list_all_tools` loop ends cleanly.
- `tools/call` results carry `content`, `structuredContent` and `isError`. Orrery reads `isError` and `content[].text` (`mcp.rs:139-158`).
- Orrery starts stdio servers in the project root (`mcp.rs:784-787`) and requires a bare command on PATH, so `neb mcp` finds `.tracker/` by discovery. Install `neb` on PATH.
- Palette: all 7 constants are identical (`Orrery/crates/orrery-tui/src/theme.rs:5-11` = `Nebulous/crates/nebulous_cli/src/tui/theme.rs:6-25`).
- Standalone: neither repo references the other in code; each builds and runs alone.
- Overlap: both read the same plan file, but different parts (Nebulous `plan_import` reads the roadmap tables; Orrery ingests `### STEP` sections), so there's no conflict. No duplicated feature worth merging.

**Gate for `v0.1.0`:** A1 and A2 fixed, then this live check re-run with a write hook (`dry_run: false`) that actually changes the throwaway tracker. → **cleared 2026-11-18** (see Phase 22 live check).

### Phase 22 live check (2026-11-18)

Nebulous `neb` built at `4da9cee` (40A–40C); Orrery at `71e9b6a` (A1 = Steps 89–92, `merge_tool_args` at `engine.rs:1941`, through `899bdea`). Fresh throwaway tracker in the scratchpad (git repo with a `baseline` commit, a dirty `.tracker/` from a new bug, and an untracked workspace file `notes.txt`), `HOME`/`XDG_*` pinned there, traffic tee'd. Orrery's `live_stdio_call_with_merged_args` (`ORRERY_LIVE_MCP_TOOL=git_commit_documented`), run three times:

| Run | Configured args (Orrery merges in `orrery`) | Result | Repo after |
|---|---|---|---|
| 1 | `{"summary": "orrery step pass"}` | ok; `dry_run: true`, `would: "commit .tracker/ with summary 'orrery step pass'"` | HEAD unchanged (`baseline`) |
| 2 | `{"summary": "orrery step pass", "dry_run": false}` | ok; `dry_run: false`, `commit_sha 8648f80` | new commit `[TRACKER](sync) orrery step pass`, 5 `.tracker/` files only; `notes.txt` still untracked |
| 3 | `{}` (Orrery's bare payload) | isError: "Invalid git_commit_documented args: summary must not be empty" (the live test fails, as intended) | HEAD unchanged |

Handshake as in Phase 21 (rmcp asks `2026-07-28`, Nebulous answers `2025-06-18`). Matches the user's report of Orrery User Step NEB-1. **A1 and A2 are resolved; the `v0.1.0` gate is clear.** Next for the release: `scripts/bump-version.sh --dry-run 0.1.0`, then the real run and the printed commit/tag/push — the user's call.


# Phase 22 — Phase 21 Review Picks: Audit Blockers (written 2026-11-18)

**Phase goal (frozen once approved):** fix the Nebulous side of the Phase 21 audit's blockers and two parked items, then prove Orrery ↔ Nebulous cooperation with a real write. A2: MCP `git_commit_documented` rejects an empty `summary` and defaults to `dry_run: true`. Parked: the Windows watcher self-loop test waits for quiet instead of a fixed 250 ms, and `release.yml` runs `cargo deny check advisories bans` like CI. A1 is fixed in Orrery (static hook/gate `args`), carried there by the user. The phase ends with a live check where an Orrery hook passes `dry_run: false` and changes the throwaway tracker. No tag is cut in Phase 22. Nothing else.

**User picks (2026-11-18):** audit blockers A1 + A2; parked Windows watcher flake and release.yml `cargo deny` mismatch. A1 fix side: Orrery `args` field. Not picked: A3 (Orrery, should-fix), A4–A6 (nice-to-have), jiff 1.0 (stays parked until ≥ 2026-11-29).

| Step | Does | Files |
| :--- | :--- | :--- |
| 40A | `git_commit_documented`: non-empty `summary`, `dry_run` (default true) | `mcp/mod.rs`, `tests/workflow_integration.rs`, `nebulous_cli/tests/cli_integration.rs` |
| 40B | Watcher self-loop test waits for 500 ms of quiet (5 s cap) | `tests/workflow_integration.rs` |
| 40C | `release.yml` deny gate: `check advisories bans` | `.github/workflows/release.yml` |
| [USER] CI | Push and report each OS | — |
| [USER] ORRERY A1 | Apply the Orrery-side A1 change in Orrery (its own plan/workflow) | Orrery repo |
| [CLAUDE] LIVE | Live check: Orrery hook with `args` drives a Nebulous write | — (result goes in this file) |

### STEP 40A: `git_commit_documented` Requires a Summary and Defaults to Dry Run

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 40A: MCP git_commit_documented rejects a missing or blank summary and takes dry_run (default true), so a call with only Orrery's hook payload commits nothing

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/workflow_integration.rs
  - crates/nebulous_cli/tests/cli_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:853-866 (git_commit_documented schema; `"required": ["summary"]` at :865), :2234-2267 (tool_git_commit_documented; `#[serde(default, alias = "message")] summary: String` at :2241-2242), the tool_git_push dry-run branch just above it (shape to copy), :2662 (`fn default_true`)
  - crates/backend_core/src/git_sync/mod.rs:174-190 (`DocumentedCommitOutcome`), :1753 (empty-summary fallback message; storage API keeps allowing "")
  - crates/backend_core/tests/workflow_integration.rs:2289 (`mcp_git_tools_default_to_dry_run`: repo + testuser config setup to copy)
  - crates/nebulous_cli/tests/cli_integration.rs:1223-1234 (MCP request id 10, git_commit_documented), :1431-1441 (its assertions: isError false, documentation_updated true)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Do not change storage `git_commit_documented` or `DocumentedCommitInput` (the storage API and `workflow_integration.rs:1126` keep allowing an empty summary). No other tool's schema or handler changes. No new dependencies.

[PRE_GATE]
grep -q '"required": \["summary"\]' crates/backend_core/src/mcp/mod.rs && ! grep -q 'summary must not be empty' crates/backend_core/src/mcp/mod.rs

[MUTATION_SPEC]
1. mcp/mod.rs schema (:853-866):
   a. Append to the git_commit_documented description string: " Defaults to dry_run: true."
   b. Add a property after "paths":
        "dry_run": { "type": "boolean", "default": true, "description": "When true (default), returns git status and what would be committed without committing. Pass false to commit." }
   `"required": ["summary"]` stays.
2. tool_git_commit_documented Args: add a field
        #[serde(default = "default_true")]
        dry_run: bool,
3. tool_git_commit_documented body, right after `let parsed: Args = ...?;`:
        if parsed.summary.trim().is_empty() {
            return Err(TrackerError::ValidationError(
                "Invalid git_commit_documented args: summary must not be empty".to_string(),
            ));
        }
   Then, after `let storage = self.resolve_active_storage()?;`:
        if parsed.dry_run {
            let status = storage.git_status()?;
            let target = if parsed.tracker_only {
                ".tracker/".to_string()
            } else {
                format!(".tracker/ plus {} listed path(s)", parsed.paths.len())
            };
            return Ok(json!({
                "dry_run": true,
                "status": status,
                "would": format!("commit {target} with summary '{}'", parsed.summary.trim())
            }));
        }
   The real-commit path stays as is, except the result gets `"dry_run": false`:
        let mut value = serde_json::to_value(outcome)
            .map_err(|e| TrackerError::ValidationError(e.to_string()))?;
        value["dry_run"] = json!(false);
        Ok(value)
4. cli_integration.rs request id 10 (git_commit_documented): add `"dry_run": false` to its "arguments" object. Its assertions stay unchanged.
5. workflow_integration.rs: add this test directly after `mcp_git_tools_default_to_dry_run`:
        #[test]
        fn mcp_git_commit_documented_requires_summary_and_defaults_to_dry_run() {
            use backend_core::McpServer;
            use serde_json::json;

            let dir = TempDir::new().unwrap();
            let storage = TrackerStorage::init_standard(dir.path(), "McpCommitDryRun").unwrap();
            let repo = Repository::init(dir.path()).unwrap();
            {
                let mut cfg = repo.config().unwrap();
                cfg.set_str("user.name", "testuser").unwrap();
                cfg.set_str("user.email", "testuser@example.com").unwrap();
            }
            storage
                .git_commit_tracker(Some("chore: init tracker"))
                .unwrap();
            storage
                .create_bug("dirty bug", None, None, Severity::Medium, None)
                .unwrap();
            let head_before = repo.head().unwrap().peel_to_commit().unwrap().id();

            let mut server = McpServer::new(Some(storage.clone()));

            // Orrery's hook payload has no summary: rejected, nothing committed
            let err = server
                .call_tool(
                    "git_commit_documented",
                    json!({"orrery": {"event": "step_pass"}}),
                )
                .unwrap_err();
            assert!(err.to_string().contains("summary must not be empty"));
            assert!(server
                .call_tool("git_commit_documented", json!({"summary": "   "}))
                .is_err());

            // Default is a dry run
            let dry = server
                .call_tool("git_commit_documented", json!({"summary": "record bug"}))
                .unwrap();
            assert_eq!(dry["dry_run"], json!(true));
            assert_eq!(
                repo.head().unwrap().peel_to_commit().unwrap().id(),
                head_before
            );
            assert!(storage.git_status().unwrap().tracker_dirty);

            // dry_run: false commits
            let real = server
                .call_tool(
                    "git_commit_documented",
                    json!({"summary": "record bug", "dry_run": false}),
                )
                .unwrap();
            assert_eq!(real["dry_run"], json!(false));
            assert!(real["commit_sha"].as_str().is_some());
            assert_ne!(
                repo.head().unwrap().peel_to_commit().unwrap().id(),
                head_before
            );
        }
   Run `cargo fmt --all` after adding it.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q '"required": \["summary"\]' crates/backend_core/src/mcp/mod.rs && test "$(grep -c 'summary must not be empty' crates/backend_core/src/mcp/mod.rs)" -eq 1 && grep -q 'without committing. Pass false to commit.' crates/backend_core/src/mcp/mod.rs && cargo test -p backend_core --test workflow_integration mcp_git_commit_documented_requires_summary_and_defaults_to_dry_run 2>&1 | grep -q 'test result: ok. 1 passed'

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 40B: Watcher Self-Loop Test Waits for Quiet

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 40B: test_watcher_detects_external_changes_and_auto_reconciles_indexes proves "no self-trigger loop" by waiting for 500 ms of quiet within 5 s, instead of failing on any event in a fixed 250 ms window

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/tests/workflow_integration.rs:810-814 (the 250 ms assert; Windows CI run 44313520994 failed here at 811:5 on a late event)
  - crates/backend_core/src/watcher/mod.rs:114-148 (50 ms debounce, 300 ms post-reconcile settle: a real loop re-fires within ~350 ms, so 500 ms of quiet still catches it)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Only the final assertion block of this one test changes; the watcher code and every other test stay as they are.

[PRE_GATE]
grep -q 'rx.recv_timeout(Duration::from_millis(250)).is_err(),' crates/backend_core/tests/workflow_integration.rs

[MUTATION_SPEC]
1. Replace lines 810-814 (the comment "// Ensure the watcher did NOT enter a self-trigger loop after reconciliation settled" and the `assert!( rx.recv_timeout(Duration::from_millis(250)).is_err(), "watcher must not loop on its own reconciled index writes" );` block) with:
        // A self-trigger loop never goes quiet; late OS events (seen on Windows CI) do.
        let settle_deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut went_quiet = false;
        while std::time::Instant::now() < settle_deadline {
            if rx.recv_timeout(Duration::from_millis(500)).is_err() {
                went_quiet = true;
                break;
            }
        }
        assert!(
            went_quiet,
            "watcher must not loop on its own reconciled index writes"
        );

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -q 'recv_timeout(Duration::from_millis(250))' crates/backend_core/tests/workflow_integration.rs && grep -q 'went_quiet,' crates/backend_core/tests/workflow_integration.rs && for i in 1 2 3; do cargo test -p backend_core --test workflow_integration test_watcher_detects_external_changes_and_auto_reconciles_indexes 2>&1 | grep -q 'test result: ok. 1 passed' || exit 1; done

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 40C: Release Workflow Runs the Same `cargo deny` Check as CI

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 40C: release.yml's supply-chain gate runs `cargo deny check advisories bans`, matching ci.yml, so a release can't ship a crate CI would reject

[SCOPE_LOCK]
ALLOWED_FILES:
  - .github/workflows/release.yml
READ_ONLY_REFS:
  - .github/workflows/release.yml:49-52 ("Supply-chain advisory gate (cargo deny)", `command: check advisories`)
  - .github/workflows/ci.yml:50-52 (`command: check advisories bans`)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Only the `command:` value on release.yml:52 changes.

[PRE_GATE]
grep -qx '          command: check advisories' .github/workflows/release.yml

[MUTATION_SPEC]
1. release.yml:52: `          command: check advisories` becomes `          command: check advisories bans` (same indentation).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && python3 -c 'import sys,yaml; yaml.safe_load(open(".github/workflows/release.yml"))' && grep -qx '          command: check advisories bans' .github/workflows/release.yml && ! grep -qx '          command: check advisories' .github/workflows/release.yml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### User Step: Push and Report CI Matrix (Phase 22)

**[USER]**. After 40C is COMPLETED and Claude has committed. Push `main`, then report each OS ("passed", or the `panicked at` line plus the assertion lines under it for each failure). Don't push a tag.

```bash
cd /srv/devshare/Nebulous && git push
```

### User Step: Orrery A1 — Static `args` on Hooks and Gates (carried into Orrery)

**[USER]**, in `/srv/devshare/Orrery` under Orrery's own plan and rules (Claude doesn't edit Orrery). Requirement for Orrery's planner:

1. `HookConfig` (`config.rs:62-66`) gets an optional `args` (JSON object, default empty). Gate tools get the same, without breaking existing `gate_tools = ["server:tool", …]` configs (for example a parallel `gate_args` map keyed by tool ref, or entries that accept either a string or `{ tool, args }`).
2. At call time (`engine.rs:1316-1325` hooks, `engine.rs:1411-1420` gates) the call arguments are the configured `args` plus the existing `orrery` object. `orrery` is reserved: config validation rejects an `args` that contains it.
3. CLI: `orrery hook set <event> <tool> [--args JSON]` and the gate equivalent.
4. A pure, `pub` function builds the merged arguments (so it can be unit-tested), plus a `live-backends` test (like `tests/live_mcp.rs`, using `ORRERY_LIVE_MCP_CMD`) that calls one tool with merged args and asserts the result isn't an error.
5. Acceptance: with `on_step_pass` = `<neb server id>:git_commit_documented` and args `{"summary": "orrery step pass", "dry_run": false}`, the hook's call commits `.tracker/`; with `dry_run` left out, it doesn't.

Tell Claude when it has landed in Orrery (commit SHA).

### Claude Step: Live Check — Orrery Hook Drives a Nebulous Write

**[CLAUDE]**, read-only for both repos except this plan file, after Phase 22 CI is green and Orrery A1 has landed. Same setup as the Phase 21 audit (scratchpad tracker, `HOME`/`XDG_*` pinned, `neb` built at HEAD):
1. Run Orrery's A1 live test against `neb mcp` twice: with `{"summary": …, "dry_run": false}` (expect a new commit in the throwaway repo) and without `dry_run` (expect no commit).
2. Repeat the Phase 21 probe with Orrery's bare hook payload: `git_commit_documented` must now return isError "summary must not be empty" and leave HEAD unchanged.
3. Record the result as "Phase 22 live check" under the Phase 21 audit report. A green result clears the `v0.1.0` gate; then `scripts/bump-version.sh 0.1.0` and the tag are the user's call.

---

## Phase 23 — Orrery Uses `neb mcp` as a Registered Server (planned 2026-11-19)

Pending items picked for this phase: **BUG-0003** (41A, added by the user before approval), **BUG-0001** (41B) and **FEAT-0001** (41C). Left pending: BUG-0002 and TASK-0084 (they belong to the Claude Code/Gemini phase), TASK-0085, TASK-0086 and TASK-0088.

**Step items.** The installed `neb` still has BUG-0003: a `--write` import of this file would retitle 12 closed items and create 81 (dry run, 2026-11-19). So right after approval Claude imports a scratch copy that holds only the four STEP rows below, under an empty Completed table: `neb plan import <scratch>/p23.md --write`. That workaround is needed only for this filing. Once 41A is installed, a plain import of this file is safe again; the live check proves it. `[USER]` rows are skipped. `[CLAUDE]` rows become tasks on a plain import (only `[USER]` rows are skipped), which is harmless.

Every spec below was applied and run in a scratch worktree at `520e818` before it was written here. The standard gate and the workspace tests passed (8 tests in the new `mcp_orrery_context`, 13 in `plan_import`). A dry run of this plan with the 41A build showed `CREATE` for the new rows only, no `RETITLE`, and `UNCHANGED 75`.

### STEP 41A: Plan Import Leaves History Alone

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 41A: neb plan import (CLI and MCP) never creates items for Completed rows and never retitles closed or merged items; open items are retitled and closed as before.

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/planning/import.rs
  - crates/backend_core/tests/plan_import.rs
  - crates/nebulous_cli/tests/cli_integration.rs
  - docs/USAGE.md
READ_ONLY_REFS:
  - crates/backend_core/src/planning/import.rs:31-45 (import_plan doc comment), :122-188 (the per-step match in fn run)
  - crates/backend_core/tests/plan_import.rs:11-27 (SAMPLE_PLAN: Up Next 2A, 2B; Completed 1A)
  - crates/nebulous_cli/tests/cli_integration.rs:2576-2634 (test_plan_import_dry_run_write_and_rerun, same plan)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No change to ImportReport, ImportEntry, PLAN_CLOSE_SUMMARY, the CLI output format, or the close path for existing open items (needs_close). No new flags.

[PRE_GATE]
grep -q 'let needs_retitle = existing_title.trim() != trimmed_title;' crates/backend_core/src/planning/import.rs

[MUTATION_SPEC]
1. crates/backend_core/src/planning/import.rs:31-32. Anchor: `/// Upserts task items from parsed roadmap steps by `source_ref`.` followed by `///`. Insert after those two lines (before `/// When `write` is `true`, runs inside`):
/// Completed rows never create items; they only close a matching open item. Closed and merged
/// items are never retitled.
///
2. crates/backend_core/src/planning/import.rs:128-129. Before:
            None => {
                let item_id = if write {
   After:
            None => {
                if step.completed {
                    continue;
                }
                let item_id = if write {
3. crates/backend_core/src/planning/import.rs:139-143. Before:
                    let new_id = detail.manifest.item.id;
                    if step.completed {
                        storage.close_item(&new_id, PLAN_CLOSE_SUMMARY, None, false)?;
                    }
                    Some(new_id)
   After (one line):
                    Some(detail.manifest.item.id)
   Why: a Completed row now never reaches this branch. PLAN_CLOSE_SUMMARY stays in use by the needs_close path.
4. crates/backend_core/src/planning/import.rs:154. Before:
                let needs_retitle = existing_title.trim() != trimmed_title;
   After:
                let needs_retitle =
                    !existing_status.is_completed() && existing_title.trim() != trimmed_title;
5. crates/backend_core/tests/plan_import.rs, fn first_write_run_creates_tasks_with_source_ref. (a) Line 71, before → after:
    assert_eq!(created_steps, vec!["STEP 2A", "STEP 2B", "STEP 1A"]);
    assert_eq!(created_steps, vec!["STEP 2A", "STEP 2B"]);
   (b) Delete lines 94-114, from `    let id_1a = report.created[2]` through the `    );` that closes `Some(PLAN_CLOSE_SUMMARY)` (the whole 1A block), and put these 2 lines in their place, before the function's closing `}`:
    let listed = storage.load_index().expect("should load index").all_items();
    assert_eq!(listed.len(), 2, "a Completed row must not create an item");
6. crates/backend_core/tests/plan_import.rs: in fn second_run_reports_everything_unchanged (line 134), fn manually_closed_item_stays_closed_while_row_is_open (line 257) and fn dry_run_leaves_tracker_byte_identical (line 295, the `dry_steps` assert), each `vec!["STEP 2A", "STEP 2B", "STEP 1A"]` → `vec!["STEP 2A", "STEP 2B"]` (3 places, nothing else on those lines changes).
7. crates/backend_core/tests/plan_import.rs:176, in fn retitled_row_retitles_only. Before → after:
    assert_eq!(unchanged_steps, vec!["STEP 2A", "STEP 1A"]);
    assert_eq!(unchanged_steps, vec!["STEP 2A"]);
8. crates/backend_core/tests/plan_import.rs:230, in fn row_flipped_to_completed_closes. Anchor: `    assert_eq!(detail_2b.manifest.item.status, ItemStatus::Closed);`. Insert after it (before the function's closing `}`):
    assert_eq!(
        detail_2b
            .manifest
            .history
            .previous_fixes
            .last()
            .map(|f| f.summary.as_str()),
        Some(PLAN_CLOSE_SUMMARY)
    );
   Why: this keeps PLAN_CLOSE_SUMMARY covered (and its `use` line 7 in use) after item 5 removes the 1A check.
9. crates/backend_core/tests/plan_import.rs:408 (fn mcp_plan_import_defaults_to_dry_run) and :428 (fn mcp_plan_import_dry_run_false_creates_tasks): `assert_eq!(created.len(), 3);` → `assert_eq!(created.len(), 2);` (2 places).
10. crates/backend_core/tests/plan_import.rs: append at the end of the file (after fn mcp_plan_import_rejects_unknown_arguments), with one blank line before it:
#[test]
fn closed_item_is_not_retitled() {
    let (_dir, storage) = init_test_tracker();
    let steps = parse_roadmap(SAMPLE_PLAN).expect("should parse sample roadmap");
    let first = import_plan(&storage, &steps, None, true).expect("first import");
    let id_2a = first.created[0]
        .item_id
        .clone()
        .expect("should have item_id for 2A");
    storage
        .update_item_metadata(&id_2a, Some("Widget module (reworded)".to_string()), None)
        .expect("should reword 2A");
    storage
        .close_item(&id_2a, "Done", None, false)
        .expect("should close 2A");

    let second = import_plan(&storage, &steps, None, true).expect("second import");
    assert!(second.retitled.is_empty());
    let unchanged_steps: Vec<&str> = second
        .unchanged
        .iter()
        .map(|e| e.step_id.as_str())
        .collect();
    assert_eq!(unchanged_steps, vec!["STEP 2A", "STEP 2B"]);

    let (_, detail_2a) = storage
        .load_item_detail_by_id(&id_2a)
        .expect("should load 2A");
    assert_eq!(detail_2a.manifest.item.title, "Widget module (reworded)");
}
11. crates/nebulous_cli/tests/cli_integration.rs:2620 and :2632, in fn test_plan_import_dry_run_write_and_rerun. Before → after:
    assert_eq!(created.len(), 3);
    assert_eq!(created.len(), 2);
   and
    assert!(rerun_stdout.contains("UNCHANGED 3"));
    assert!(rerun_stdout.contains("UNCHANGED 2"));
12. docs/USAGE.md:180, the `- **Idempotency & field ownership (`source_ref`)**` line. Two in-line changes; the rest of the line stays. (a) `updates `title` on re-import if the plan's `Scope` cell changed, and closes` → `updates `title` on re-import if the plan's `Scope` cell changed (closed and merged items are never retitled), and closes`. (b) `closes an open item when its row is `COMPLETED`. Import never reopens` → `closes an open item when its row is `COMPLETED`. A `COMPLETED` row with no matching item creates nothing. Import never reopens`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q '!existing_status.is_completed() && existing_title.trim() != trimmed_title;' crates/backend_core/src/planning/import.rs && ! grep -q 'let new_id = detail.manifest.item.id;' crates/backend_core/src/planning/import.rs && grep -q '/// Completed rows never create items; they only close a matching open item.' crates/backend_core/src/planning/import.rs && grep -q 'fn closed_item_is_not_retitled' crates/backend_core/tests/plan_import.rs && grep -q 'a Completed row must not create an item' crates/backend_core/tests/plan_import.rs && ! grep -q '"STEP 1A"\]' crates/backend_core/tests/plan_import.rs && grep -q 'UNCHANGED 2' crates/nebulous_cli/tests/cli_integration.rs && grep -qF 'A `COMPLETED` row with no matching item creates nothing.' docs/USAGE.md && grep -qF '(closed and merged items are never retitled)' docs/USAGE.md

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 41B: `plan_import` Accepts the Reserved `orrery` Object

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 41B: MCP plan_import accepts Orrery's reserved `orrery` argument and ignores it; every other unknown argument is still rejected.

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/plan_import.rs
  - docs/USAGE.md
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:932-960 (tool_plan_import, its Args struct with #[serde(deny_unknown_fields)])
  - crates/backend_core/src/mcp/mod.rs:728-742 (plan_import McpToolDefinition, "additionalProperties": false)
  - crates/backend_core/tests/plan_import.rs:374-487 (mcp_tracker_with_plan, mcp_plan_import_rejects_unknown_arguments, closed_item_is_not_retitled from 41A)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Keep #[serde(deny_unknown_fields)] on plan_import's Args. Keep "additionalProperties": false. Don't change or delete mcp_plan_import_rejects_unknown_arguments. No other tool changes.

[PRE_GATE]
grep -q 'fn closed_item_is_not_retitled' crates/backend_core/tests/plan_import.rs && grep -q '#\[serde(deny_unknown_fields)\]' crates/backend_core/src/mcp/mod.rs && ! grep -q 'rename = "orrery"' crates/backend_core/src/mcp/mod.rs

[MUTATION_SPEC]
1. crates/backend_core/src/mcp/mod.rs:940, inside `fn tool_plan_import`'s `struct Args`. Anchor: the line `            dry_run: bool,` directly above `        }` and `        let parsed: Args = serde_json::from_value(args)` / `"Invalid plan_import args: {e}"`. Insert these 3 lines after `dry_run: bool,` (before the closing `}`):
            /// Orrery's reserved per-call context object; accepted and ignored.
            #[serde(default, rename = "orrery")]
            _orrery: Option<Value>,
   Why: Orrery merges an `orrery` object into every hook and gate call; the `_` prefix keeps the never-read field free of a dead_code warning without an #[allow].
2. crates/backend_core/src/mcp/mod.rs:737, the plan_import schema. Before (one line):
                        "dry_run": { "type": "boolean", "default": true, "description": "When true (default), previews changes without modifying .tracker/." }
   After (two lines; note the comma added at the end of the first):
                        "dry_run": { "type": "boolean", "default": true, "description": "When true (default), previews changes without modifying .tracker/." },
                        "orrery": { "type": "object", "description": "Reserved for Orrery's per-call context; accepted and ignored." }
3. crates/backend_core/tests/plan_import.rs: append at the end of the file (after `fn closed_item_is_not_retitled`, the last test, added by 41A), with one blank line before it:
#[test]
fn mcp_plan_import_accepts_orrery_context_object() {
    use backend_core::McpServer;
    use serde_json::json;

    let (_dir, storage) = mcp_tracker_with_plan();
    let mut server = McpServer::new(Some(storage));
    let res = server
        .call_tool(
            "plan_import",
            json!({ "path": "PLAN.md", "orrery": { "step_number": 1, "step_title": "2A: Build widget module" } }),
        )
        .expect("the reserved orrery object must be accepted");
    assert_eq!(res["created"].as_array().map(Vec::len), Some(2));
}
4. docs/USAGE.md:294. Before: `- **Strict validation & response**: unknown arguments are rejected, and the tool returns` … After: `- **Strict validation & response**: unknown arguments are rejected (except the reserved `orrery` context object that Orrery adds to every call, which is accepted and ignored), and the tool returns` … (the rest of the line is unchanged).

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'rename = "orrery"' crates/backend_core/src/mcp/mod.rs && grep -q '_orrery: Option<Value>,' crates/backend_core/src/mcp/mod.rs && grep -q "Reserved for Orrery's per-call context; accepted and ignored." crates/backend_core/src/mcp/mod.rs && grep -q 'fn mcp_plan_import_accepts_orrery_context_object' crates/backend_core/tests/plan_import.rs && grep -q 'fn mcp_plan_import_rejects_unknown_arguments' crates/backend_core/tests/plan_import.rs && grep -qF 'except the reserved `orrery` context object that Orrery adds to every call' docs/USAGE.md

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 41C: Status and Close Tools Resolve the Item From `orrery.step_title`

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 41C: MCP update_item_status and close_item_with_fix make item_id optional; without it they act on the one item whose source_ref is "STEP <ID>", with <ID> parsed from orrery.step_title.

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/mcp_orrery_context.rs (NEW)
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:622-647 (update_item_status / close_item_with_fix McpToolDefinitions)
  - crates/backend_core/src/mcp/mod.rs:1340-1405 (tool_update_item_status, tool_close_item_with_fix)
  - crates/backend_core/src/mcp/mod.rs:2602-2604 (fn non_empty)
  - crates/backend_core/src/planning/import.rs:95-121 (same load_index / all_items / read_item_manifest / source_ref scan)
  - crates/backend_core/src/workflow/lifecycle.rs:49-90 (CreateItemInput::new, with_source_ref)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No change to any other tool, to the result JSON of these two tools, or to the plan_import changes from 41B.

[PRE_GATE]
grep -q 'rename = "orrery"' crates/backend_core/src/mcp/mod.rs && ! grep -q 'fn item_id_from_orrery_step' crates/backend_core/src/mcp/mod.rs

[MUTATION_SPEC]
1. crates/backend_core/src/mcp/mod.rs:628 (update_item_status schema). Before:
                        "item_id": { "type": "string", "description": "Item ID (e.g. BUG-0001)." },
                        "status": { "type": "string", "description": "Target status (open, in_progress, pending, closed, merged)." }
                    },
                    "required": ["item_id", "status"]
   After:
                        "item_id": { "type": "string", "description": "Item ID (e.g. BUG-0001). Optional when the reserved orrery.step_title is present: the item whose source_ref is STEP <ID> is used." },
                        "status": { "type": "string", "description": "Target status (open, in_progress, pending, closed, merged)." }
                    },
                    "required": ["status"]
2. crates/backend_core/src/mcp/mod.rs:640 and :645 (close_item_with_fix schema). Line 640 before → after:
                        "item_id": { "type": "string", "description": "Item ID (e.g. BUG-0001)." },
                        "item_id": { "type": "string", "description": "Item ID (e.g. BUG-0001). Optional when the reserved orrery.step_title is present: the item whose source_ref is STEP <ID> is used." },
   Line 645 before → after:
                    "required": ["item_id", "summary"]
                    "required": ["summary"]
3. crates/backend_core/src/mcp/mod.rs:1342-1356, in `fn tool_update_item_status`. Before:
        struct Args {
            #[serde(alias = "id")]
            item_id: String,
            status: String,
        }

        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid update_item_status args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let status = ItemStatus::from_str(&parsed.status)?;
        let loc = storage.locate_item(parsed.item_id.trim())?;
        let change = storage.update_item_status_reporting(parsed.item_id.trim(), status)?;
   After:
        struct Args {
            #[serde(default, alias = "id")]
            item_id: Option<String>,
            status: String,
        }

        let parsed: Args = serde_json::from_value(args.clone()).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid update_item_status args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let item_id = match non_empty(parsed.item_id.as_deref()) {
            Some(id) => id.to_string(),
            None => item_id_from_orrery_step(&storage, &args)?,
        };
        let status = ItemStatus::from_str(&parsed.status)?;
        let loc = storage.locate_item(&item_id)?;
        let change = storage.update_item_status_reporting(&item_id, status)?;
4. crates/backend_core/src/mcp/mod.rs:1372-1390, in `fn tool_close_item_with_fix`. (a) In its `struct Args`, before → after:
            #[serde(alias = "id")]
            item_id: String,
            #[serde(default, alias = "id")]
            item_id: Option<String>,
   (b) Before:
        let parsed: Args = serde_json::from_value(args).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid close_item_with_fix args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let change = storage.close_item_reporting(
            parsed.item_id.trim(),
   After:
        let parsed: Args = serde_json::from_value(args.clone()).map_err(|e| {
            TrackerError::ValidationError(format!("Invalid close_item_with_fix args: {e}"))
        })?;
        let storage = self.resolve_active_storage()?;
        let item_id = match non_empty(parsed.item_id.as_deref()) {
            Some(id) => id.to_string(),
            None => item_id_from_orrery_step(&storage, &args)?,
        };
        let change = storage.close_item_reporting(
            &item_id,
   The remaining arguments of the close_item_reporting call and everything after it stay unchanged.
5. crates/backend_core/src/mcp/mod.rs:2602-2604. Insert after `fn non_empty` (after its closing `}` and the blank line), this new function:
/// Resolves the one item whose `source_ref` is `STEP <ID>`, with `<ID>` parsed from Orrery's
/// reserved `orrery.step_title` argument (`"3F: Title"` or `"STEP 3F: Title"`).
fn item_id_from_orrery_step(storage: &TrackerStorage, args: &Value) -> Result<String> {
    let title = args
        .get("orrery")
        .and_then(|o| o.get("step_title"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            TrackerError::ValidationError(
                "item_id is required unless orrery.step_title is present".to_string(),
            )
        })?;
    let trimmed = title.trim();
    let step_id = trimmed
        .strip_prefix("STEP ")
        .unwrap_or(trimmed)
        .split_once(':')
        .map(|(id, _)| id.trim())
        .filter(|id| {
            let digits = id.trim_end_matches(|c: char| c.is_ascii_uppercase());
            !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
        })
        .ok_or_else(|| {
            TrackerError::ValidationError(format!(
                "can't parse a step ID from orrery.step_title '{title}'"
            ))
        })?;
    let source_ref = format!("STEP {step_id}");
    let index = storage.load_index()?;
    let mut hits = Vec::new();
    for loc in index.all_items() {
        let manifest = storage.read_item_manifest(&loc.phase_id, &loc.scope_id, &loc.stub.id)?;
        if manifest.item.source_ref.as_deref().map(str::trim) == Some(source_ref.as_str()) {
            hits.push(manifest.item.id);
        }
    }
    match hits.len() {
        1 => Ok(hits.remove(0)),
        0 => Err(TrackerError::ValidationError(format!(
            "no item has source_ref '{source_ref}'"
        ))),
        n => Err(TrackerError::ValidationError(format!(
            "{n} items have source_ref '{source_ref}': {}",
            hits.join(", ")
        ))),
    }
}
   No new `use` lines: Value, Result, TrackerError and TrackerStorage are already imported at the top of mod.rs.
6. crates/backend_core/tests/mcp_orrery_context.rs (NEW), exactly this content:
use backend_core::{CreateItemInput, ItemType, McpServer, TrackerStorage};
use serde_json::{json, Value};
use tempfile::TempDir;

fn tracker_with_step_task(source_ref: &str) -> (TempDir, TrackerStorage, String) {
    let dir = TempDir::new().expect("should create temp dir");
    let storage =
        TrackerStorage::init_standard(dir.path(), "OrbitTest").expect("should init tracker");
    let (_, detail) = storage
        .create_item(
            CreateItemInput::new(ItemType::Task, "Widget step").with_source_ref(source_ref),
        )
        .expect("should create step task");
    let id = detail.manifest.item.id.clone();
    (dir, storage, id)
}

fn orrery(step_title: &str) -> Value {
    json!({ "step_number": 1, "step_title": step_title, "event": "step_start" })
}

#[test]
fn update_item_status_resolves_item_from_orrery_step_title() {
    let (_dir, storage, id) = tracker_with_step_task("STEP 41B");
    let mut server = McpServer::new(Some(storage));
    let res = server
        .call_tool(
            "update_item_status",
            json!({ "status": "in_progress", "orrery": orrery("41B: Widget step") }),
        )
        .expect("step_title must resolve the item");
    assert_eq!(res["item_id"].as_str(), Some(id.as_str()));
    assert_eq!(res["status"].as_str(), Some("in_progress"));
}

#[test]
fn update_item_status_accepts_step_prefix_in_step_title() {
    let (_dir, storage, id) = tracker_with_step_task("STEP 41B");
    let mut server = McpServer::new(Some(storage));
    let res = server
        .call_tool(
            "update_item_status",
            json!({ "status": "in_progress", "orrery": orrery("STEP 41B: Widget step") }),
        )
        .expect("a STEP prefix must be accepted");
    assert_eq!(res["item_id"].as_str(), Some(id.as_str()));
}

#[test]
fn explicit_item_id_wins_over_orrery_step_title() {
    let (_dir, storage, id) = tracker_with_step_task("STEP 41B");
    let mut server = McpServer::new(Some(storage));
    let res = server
        .call_tool(
            "update_item_status",
            json!({ "item_id": id, "status": "pending", "orrery": orrery("99Z: Other step") }),
        )
        .expect("explicit item_id must be used");
    assert_eq!(res["item_id"].as_str(), Some(id.as_str()));
    assert_eq!(res["status"].as_str(), Some("pending"));
}

#[test]
fn close_item_with_fix_resolves_item_from_orrery_step_title() {
    let (_dir, storage, id) = tracker_with_step_task("STEP 41B");
    let mut server = McpServer::new(Some(storage));
    let res = server
        .call_tool(
            "close_item_with_fix",
            json!({ "summary": "step passed", "orrery": orrery("41B: Widget step") }),
        )
        .expect("step_title must resolve the item");
    assert_eq!(res["item_id"].as_str(), Some(id.as_str()));
    assert_eq!(res["status"].as_str(), Some("closed"));
    assert_eq!(res["previous_fixes_count"].as_u64(), Some(1));
}

#[test]
fn step_title_without_a_matching_item_is_an_error() {
    let (_dir, storage, _id) = tracker_with_step_task("STEP 41B");
    let mut server = McpServer::new(Some(storage));
    let err = server
        .call_tool(
            "update_item_status",
            json!({ "status": "in_progress", "orrery": orrery("41C: Next step") }),
        )
        .expect_err("no item for STEP 41C");
    let msg = err.to_string();
    assert!(
        msg.contains("no item has source_ref 'STEP 41C'"),
        "unexpected: {msg}"
    );
}

#[test]
fn unparseable_step_title_is_an_error() {
    let (_dir, storage, _id) = tracker_with_step_task("STEP 41B");
    let mut server = McpServer::new(Some(storage));
    let err = server
        .call_tool(
            "close_item_with_fix",
            json!({ "summary": "step passed", "orrery": orrery("live merged args") }),
        )
        .expect_err("title without a step ID");
    let msg = err.to_string();
    assert!(
        msg.contains("can't parse a step ID from orrery.step_title 'live merged args'"),
        "unexpected: {msg}"
    );
}

#[test]
fn missing_item_id_and_orrery_context_is_an_error() {
    let (_dir, storage, _id) = tracker_with_step_task("STEP 41B");
    let mut server = McpServer::new(Some(storage));
    let err = server
        .call_tool("update_item_status", json!({ "status": "in_progress" }))
        .expect_err("neither item_id nor orrery");
    let msg = err.to_string();
    assert!(
        msg.contains("item_id is required unless orrery.step_title is present"),
        "unexpected: {msg}"
    );
}

#[test]
fn duplicate_source_ref_is_an_error() {
    let (_dir, storage, _id) = tracker_with_step_task("STEP 41B");
    storage
        .create_item(CreateItemInput::new(ItemType::Task, "Duplicate").with_source_ref("STEP 41B"))
        .expect("should create duplicate task");
    let mut server = McpServer::new(Some(storage));
    let err = server
        .call_tool(
            "update_item_status",
            json!({ "status": "in_progress", "orrery": orrery("41B: Widget step") }),
        )
        .expect_err("two items for STEP 41B");
    let msg = err.to_string();
    assert!(
        msg.contains("2 items have source_ref 'STEP 41B'"),
        "unexpected: {msg}"
    );
}

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'fn item_id_from_orrery_step(storage: &TrackerStorage, args: &Value) -> Result<String>' crates/backend_core/src/mcp/mod.rs && grep -q '"required": \["status"\]' crates/backend_core/src/mcp/mod.rs && grep -q '"required": \["summary"\]' crates/backend_core/src/mcp/mod.rs && ! grep -q '"required": \["item_id", "status"\]' crates/backend_core/src/mcp/mod.rs && ! grep -q '"required": \["item_id", "summary"\]' crates/backend_core/src/mcp/mod.rs && grep -q 'None => item_id_from_orrery_step(&storage, &args)?,' crates/backend_core/src/mcp/mod.rs && grep -q 'fn duplicate_source_ref_is_an_error' crates/backend_core/tests/mcp_orrery_context.rs && grep -q 'fn close_item_with_fix_resolves_item_from_orrery_step_title' crates/backend_core/tests/mcp_orrery_context.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 41D: Docs — 24 Tools, `--root` Discovery, Orrery Step Context

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 41D: README says 24 MCP tools; USAGE says --root is optional and documents the Orrery step context for update_item_status / close_item_with_fix.

[SCOPE_LOCK]
ALLOWED_FILES:
  - README.md
  - docs/USAGE.md
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:147-161 (McpServer::discover: --root, else cwd walk-up, else projects.toml active project)
  - crates/backend_core/src/mcp/mod.rs (tests: every_listed_tool_has_a_handler_and_names_are_unique asserts 24 tools)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No other wording changes in either file.

[PRE_GATE]
grep -q 'fn item_id_from_orrery_step' crates/backend_core/src/mcp/mod.rs && grep -q 'exposing 23 tools' README.md

[MUTATION_SPEC]
1. README.md:34. Before → after (only the number changes):
   `MCP server exposing 23 tools across` → `MCP server exposing 24 tools across`
2. docs/USAGE.md:266. Anchor: the line `### Configuration Snippets`. Insert after it one blank line and this paragraph (the existing blank line and `#### Cursor …` heading follow it):
`--root` is optional. Without it, `neb mcp` serves the `.tracker/` found by walking up from its working directory, and only then the active project in `projects.toml`. Clients that start the server in the project root (Orrery does) can leave it out.
3. docs/USAGE.md: append at the end of the file (after the `- **Strict validation & response**` line), with one blank line before it:
### Orrery step context (`update_item_status`, `close_item_with_fix`)

Orrery adds a reserved `orrery` object to every hook and gate call. When `item_id` is left out, both tools parse the step ID from `orrery.step_title` (`"3F: Title"` or `"STEP 3F: Title"`) and act on the one item whose `source_ref` is `STEP <ID>`, which is what `neb plan import` sets. No matching item, more than one, or a title without a step ID is an error. An explicit `item_id` always wins.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'exposing 24 tools' README.md && ! grep -q 'exposing 23 tools' README.md && grep -qF '`--root` is optional. Without it, `neb mcp` serves the `.tracker/` found by walking up' docs/USAGE.md && grep -qF '### Orrery step context (`update_item_status`, `close_item_with_fix`)' docs/USAGE.md && grep -qF 'An explicit `item_id` always wins.' docs/USAGE.md

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### User Step: Reinstall neb (Phase 23)

**[USER]**. After 41D is COMPLETED and Claude has committed. This puts the 41B–41C build on `PATH`, where Orrery runs `neb`.

```bash
cd /srv/devshare/Nebulous && ./scripts/install.sh --user && neb --version
```

### Claude Step: Register `neb` With Orrery

**[CLAUDE]**, after the reinstall. From the repo root:
1. `orrery mcp add-stdio --id neb --command neb --arg mcp`. Orrery rejects absolute args for project servers, and starts stdio servers in the project root, so `neb mcp` serves this repo's `.tracker/` by cwd discovery (`mcp/mod.rs:147-161`). No `neb project switch` is needed. (orrery-setup §5 says one is. That's wrong; it gets fixed in the skill, not here.)
2. `orrery mcp tools neb` must list 24 tools, and `update_item_status` must show `"required": ["status"]`. That proves the 41C build is the one Orrery runs.
3. Make one read-only call through Orrery's client, with a wrapper that `cd`s into this repo and runs `neb mcp`: `ORRERY_LIVE_MCP_TOOL=list_items ORRERY_LIVE_MCP_ARGS='{}'` with the `live_stdio_call_with_merged_args` test. It must not be isError.
4. Check that `git diff .orrery/project.toml` is empty, then commit `.orrery/mcp/servers.toml` as `[TRACKER](orrery) Register neb MCP server`.

### User Step: Push and Report CI Matrix (Phase 23)

**[USER]**. After the register commit. Push `main`, then report each OS ("passed", or the `panicked at` line plus the assertion lines under it for each failure). Don't push a tag.

```bash
cd /srv/devshare/Nebulous && git push
```

### Claude Step: Live Check — Orrery Drives neb Through Its Own Client

**[CLAUDE]**, after CI is green. Read-only for both repos except this plan file. Throwaway tracker in the scratchpad (`neb init`, git repo, `HOME`/`XDG_*` pinned there) with a one-step plan `PLAN.md` (`### STEP 1A: Live check`, an Up Next row for **STEP 1A**), imported with `neb plan import PLAN.md --write`, so `TASK-0001` has `source_ref = "STEP 1A"`. Runs 1–3 use Orrery's `live_stdio_call_with_merged_args` test with a wrapper that `cd`s into the throwaway repo.

| Run | Tool and configured args | Expected |
| :--- | :--- | :--- |
| 0 | Installed `neb`, no Orrery: `neb plan import STEPPED_IMPLEMENTATION_PLAN.md` (dry run) in this repo | No `RETITLE` lines; `CREATE` only for rows with no item yet (41A) |
| 1 | `plan_import` `{"path": "PLAN.md"}` | Not isError (41B); `.tracker/` byte-identical (dry run) |
| 2 | `update_item_status` `{"status": "in_progress"}` | isError `can't parse a step ID from orrery.step_title 'live merged args'`. The test hard-codes that title, so the test panics, and that panic is the pass condition. It proves that neb reads the `orrery` object Orrery sends. |
| 3 | `update_item_status` `{"item_id": "TASK-0001", "status": "pending"}` | Not isError; explicit `item_id` wins |
| 4 | Real hook: `orrery init` in the throwaway repo, `orrery ingest PLAN.md --marker "### STEP "`, `orrery mcp add-stdio --id neb --command neb --arg mcp`, `orrery hook set start neb:update_item_status --args '{"status": "in_progress"}'`, stage 0 = `claude-orch`, then `orrery run --step 1` in the background | `TASK-0001` turns `in_progress` (41C resolves `"1A: Live check"`). Claude kills the run as soon as the status flips; the cost is at most one Haiku orchestrator turn. If Orrery refuses to start before the hook fires, record that as a finding instead. |

Record the result as "Phase 23 live check" below this section. Then the phase ends (Mode 3b).

### Phase 23 live check (2026-11-20)

`neb` installed at `e6104af`; Orrery client from a `git archive` of Orrery `a4b6f7a` (Orrery's checkout had uncommitted edits that didn't compile; not touched). Throwaway repo in the scratchpad, `neb init` with `HOME`/`XDG_*` pinned there, one-step plan imported, so `TASK-0001` has `source_ref = "STEP 1A"`.

| Run | Call | Result |
| :--- | :--- | :--- |
| 0 | `neb plan import STEPPED_IMPLEMENTATION_PLAN.md` (dry run, this repo) | PASS: no `RETITLE`; one `CREATE` (`[CLAUDE] LIVE`, no item yet); `UNCHANGED 79`. Before 41A the same dry run wanted 12 retitles and 81 creates. |
| 1 | `plan_import {"path":"PLAN.md"}` + Orrery's merged `orrery` | PASS: not isError; `.tracker/` byte-identical |
| 2 | `update_item_status {"status":"in_progress"}` + the harness's `step_title` `"live merged args"` | PASS (expected panic): isError `can't parse a step ID from orrery.step_title 'live merged args'` |
| 3 | `update_item_status {"item_id":"TASK-0001","status":"pending"}` | PASS: not isError; TASK-0001 `pending` |
| 4 | Real Orrery run: `on_step_start` = `neb:update_item_status` with args `{"status":"in_progress"}`, step `1A: Live check` | PASS: TASK-0001 `in_progress` ~1 s after `orrery run --step 1`; run killed on the flip. One Haiku orchestrator turn ran ($0.0022 list); its exited container was removed. |

Run 4 setup notes: Orrery won't run with an unassigned stage ("stage 1 must have 1..=15 agents") or with fewer than 2 stages ("stage count must be in 2..=15"), so the throwaway project kept two stages (orchestrator `claude-orch`, validator `claude-val`) and dropped the coder stage. Also seen: the throwaway's `.tracker/.timer.toml` was untracked after the hook started a timer, which confirms pending TASK-0088 (`neb init` doesn't ignore timer files).

### Later phases (outline only, not approved)

- **Phase 24 — Claude Code and Antigravity (`agy`).** Planned 2026-11-20; see "Phase 24" below.
- **Phase 25 — MCP registry publish.** Researched and planned 2026-11-20; see "Phase 25" at the end of this file.

---

## Phase 24 — Claude Code and Antigravity Use `neb mcp` (planned 2026-11-20)

Pending items picked for this phase: **TASK-0084** (42A) and **BUG-0002** (42B). Left pending: TASK-0085, TASK-0086, TASK-0088, TASK-0093, plus the two filed while planning this phase: TASK-0094 (2026-07-28 `server/discover`) and TASK-0095 (the untested Cursor/Codex/Ollama claims).

**What planning found (2026-11-20):**
- Claude Code 2.1.318 sends `server/discover` (protocol `2026-07-28`) first. neb answers `-32601`, and Claude Code then sends `initialize` asking for `2025-11-25`. neb answers `2025-06-18`, which it accepts. It starts the server in the directory `claude` runs in. Probe: a temporary `--mcp-config` with a logging wrapper and one Haiku prompt; no client settings were changed.
- MCP `2025-11-25` (changelog and the tools, transports and lifecycle pages): the only change that applies to a stdio server with tools and resources is JSON Schema 2020-12 as the default `inputSchema` dialect. Every other change is optional or HTTP/auth-only, and `initialize` is unchanged. All 24 schemas use only `type`, `properties`, `required`, `items`, `additionalProperties`, `default` and `description`, which mean the same in 2020-12, and every tool name matches the new naming guidance.
- `git_fetch` has no `dry_run` today. The Phase 21 probe's `dry_run: true` was ignored by serde, so a real fetch ran and failed with no `origin`. `dry_run` defaults to `false` here because fetch never touches the working tree (STEP 14 left it out of the dry-run-default rule) and v0.1.0 callers expect it to fetch.
- `agy` 1.5.12 has one global MCP list (`agy mcp add` has no scope flag) and no per-session MCP flag, so registering it changes your global agy config. That's a `[USER]` step.

Every spec below was applied in a scratch worktree at `e6104af`+`922509b` and passed the standard gate (249 tests).

### STEP 42A: MCP `2025-11-25` Supported and Default

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 42A: initialize supports MCP 2025-11-25 and answers it for unknown or missing versions; a unit test keeps every tool inputSchema an object schema built from 2020-12 keywords.

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/mcp_protocol.rs
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:23-27 (SUPPORTED_MCP_PROTOCOL_VERSIONS, DEFAULT_MCP_PROTOCOL_VERSION)
  - crates/backend_core/src/mcp/mod.rs:2712-2726 (negotiate_protocol_version)
  - crates/backend_core/src/mcp/mod.rs:2751-2788 (mod tests: every_listed_tool_has_a_handler_and_names_are_unique, unknown_tool_is_rejected_by_is_known_tool_and_call_tool)
  - crates/backend_core/tests/mcp_protocol.rs:218-257 (initialize_version helper and the three version tests)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No change to negotiate_protocol_version, to any tool schema, or to the initialize result's other fields.

[PRE_GATE]
grep -q 'pub const DEFAULT_MCP_PROTOCOL_VERSION: &str = "2025-06-18";' crates/backend_core/src/mcp/mod.rs

[MUTATION_SPEC]
1. crates/backend_core/src/mcp/mod.rs:24. Before:
pub const SUPPORTED_MCP_PROTOCOL_VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];
   After (two lines, as rustfmt wraps it):
pub const SUPPORTED_MCP_PROTOCOL_VERSIONS: &[&str] =
    &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];
2. crates/backend_core/src/mcp/mod.rs:27. Before → after:
pub const DEFAULT_MCP_PROTOCOL_VERSION: &str = "2025-06-18";
pub const DEFAULT_MCP_PROTOCOL_VERSION: &str = "2025-11-25";
3. crates/backend_core/src/mcp/mod.rs, inside `mod tests`. Anchor (line 2775-2776): `    #[test]` followed by `    fn unknown_tool_is_rejected_by_is_known_tool_and_call_tool() {`. Insert this test directly before that `#[test]` line, followed by one blank line:
    #[test]
    fn every_input_schema_is_an_object_schema_with_2020_12_keywords() {
        // MCP 2025-11-25 reads inputSchema as JSON Schema 2020-12 when `$schema` is absent.
        const KNOWN: [&str; 7] = [
            "additionalProperties",
            "default",
            "description",
            "items",
            "properties",
            "required",
            "type",
        ];
        fn check(tool: &str, schema: &Value) {
            let Some(obj) = schema.as_object() else {
                panic!("{tool}: schema node is not an object: {schema}");
            };
            for (key, value) in obj {
                assert!(
                    KNOWN.contains(&key.as_str()),
                    "{tool}: unexpected keyword '{key}'"
                );
                match key.as_str() {
                    "properties" => {
                        for prop in value.as_object().expect("properties is an object").values() {
                            check(tool, prop);
                        }
                    }
                    "items" => check(tool, value),
                    "additionalProperties" if value.is_object() => check(tool, value),
                    _ => {}
                }
            }
        }
        for tool in McpServer::tools_list() {
            assert_eq!(tool.input_schema["type"], json!("object"), "{}", tool.name);
            check(&tool.name, &tool.input_schema);
        }
    }
   Why: MCP 2025-11-25 reads inputSchema as JSON Schema 2020-12 when `$schema` is absent; the test fails if a future schema adds a keyword nobody has checked against 2020-12. No new `use` lines: `use super::*;` already brings Value, json! and McpServer into mod tests.
4. crates/backend_core/tests/mcp_protocol.rs:243, in fn initialize_unknown_or_missing_version_gets_newest. Before → after:
        assert_eq!(initialize_version(params), "2025-06-18");
        assert_eq!(initialize_version(params), "2025-11-25");
5. crates/backend_core/tests/mcp_protocol.rs:251, in fn default_protocol_version_is_newest_supported. Before → after:
        ["2025-06-18", "2025-03-26", "2024-11-05"]
        ["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"]

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'pub const DEFAULT_MCP_PROTOCOL_VERSION: &str = "2025-11-25";' crates/backend_core/src/mcp/mod.rs && grep -q '&\["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"\];' crates/backend_core/src/mcp/mod.rs && grep -q 'fn every_input_schema_is_an_object_schema_with_2020_12_keywords' crates/backend_core/src/mcp/mod.rs && grep -q 'assert_eq!(initialize_version(params), "2025-11-25");' crates/backend_core/tests/mcp_protocol.rs && grep -q '\["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"\]' crates/backend_core/tests/mcp_protocol.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 42B: `git_fetch` Dry Run Works Without a Remote

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 42B: MCP git_fetch takes dry_run (default false); a dry run returns git status and what would be fetched without contacting a remote, so it works with no origin.

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/mcp/mod.rs
  - crates/backend_core/tests/workflow_integration.rs
READ_ONLY_REFS:
  - crates/backend_core/src/mcp/mod.rs:823-833 (git_fetch McpToolDefinition; 42A's two-line const moved it down one line from :822-832 on e6104af)
  - crates/backend_core/src/mcp/mod.rs:2156-2171 (tool_git_fetch; :2155-2170 on e6104af)
  - crates/backend_core/src/mcp/mod.rs:2173-2201 (tool_git_pull's dry-run branch: the same status / would shape)
  - crates/backend_core/tests/workflow_integration.rs (fn mcp_git_commit_documented_requires_summary_and_defaults_to_dry_run: same repo setup)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. The non-dry-run git_fetch result stays exactly the serialized status it is today (no new keys). No change to other git tools.

[PRE_GATE]
grep -q 'pub const DEFAULT_MCP_PROTOCOL_VERSION: &str = "2025-11-25";' crates/backend_core/src/mcp/mod.rs && ! grep -q 'fn mcp_git_fetch_dry_run_reports_status_without_a_remote' crates/backend_core/tests/workflow_integration.rs

[MUTATION_SPEC]
1. crates/backend_core/src/mcp/mod.rs, the git_fetch McpToolDefinition (anchor: `name: "git_fetch".to_string(),`). Before:
                description: "Fetches the configured remote branch without modifying the working tree and returns updated ahead/behind commit counts.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "auth_override": { "type": "string", "description": "Optional vault credential account name; must be bound to this project in the local registry. PATs and key file paths are not accepted." }
                    }
                }),
   After:
                description: "Fetches the configured remote branch without modifying the working tree and returns updated ahead/behind commit counts. Pass dry_run: true to get the current status and what would be fetched without contacting the remote.".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "auth_override": { "type": "string", "description": "Optional vault credential account name; must be bound to this project in the local registry. PATs and key file paths are not accepted." },
                        "dry_run": { "type": "boolean", "default": false, "description": "When true, returns git status and what would be fetched without contacting the remote (works without a configured remote). Defaults to false: fetch is non-destructive." }
                    }
                }),
2. crates/backend_core/src/mcp/mod.rs, in `fn tool_git_fetch`. Before:
            #[serde(default)]
            pat: Option<Value>,
        }

        let parsed: Args = serde_json::from_value(args)
            .map_err(|e| TrackerError::ValidationError(format!("Invalid git_fetch args: {e}")))?;
        let auth = mcp_auth_request(parsed.auth_override.as_deref(), parsed.pat.as_ref())?;
        let storage = self.resolve_active_storage()?;
        let status = storage.git_fetch(&auth)?;
   After:
            #[serde(default)]
            pat: Option<Value>,
            #[serde(default)]
            dry_run: bool,
        }

        let parsed: Args = serde_json::from_value(args)
            .map_err(|e| TrackerError::ValidationError(format!("Invalid git_fetch args: {e}")))?;
        let auth = mcp_auth_request(parsed.auth_override.as_deref(), parsed.pat.as_ref())?;
        let storage = self.resolve_active_storage()?;
        if parsed.dry_run {
            let status = storage.git_status()?;
            let branch = status.branch.as_deref().unwrap_or("<none>");
            let remote = status.remote_url.as_deref().unwrap_or("<none>");
            let would = format!("fetch {branch} from {remote}");
            return Ok(json!({
                "dry_run": true,
                "status": status,
                "would": would
            }));
        }
        let status = storage.git_fetch(&auth)?;
   The line after it (`serde_json::to_value(status).map_err(...)`) stays unchanged.
3. crates/backend_core/tests/workflow_integration.rs: append at the end of the file, with one blank line before it:
#[test]
fn mcp_git_fetch_dry_run_reports_status_without_a_remote() {
    use backend_core::McpServer;
    use serde_json::json;

    let dir = TempDir::new().unwrap();
    let storage = TrackerStorage::init_standard(dir.path(), "McpFetchDryRun").unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "testuser").unwrap();
        cfg.set_str("user.email", "testuser@example.com").unwrap();
    }
    storage
        .git_commit_tracker(Some("chore: init tracker"))
        .unwrap();
    assert!(repo.find_remote("origin").is_err());

    let mut server = McpServer::new(Some(storage));
    let dry = server
        .call_tool("git_fetch", json!({"dry_run": true}))
        .expect("dry run must not need a remote");
    assert_eq!(dry["dry_run"], json!(true));
    assert!(dry["status"].is_object());
    let would = dry["would"].as_str().expect("would is a string");
    assert!(would.starts_with("fetch "), "unexpected: {would}");
    assert!(would.ends_with(" from <none>"), "unexpected: {would}");

    assert!(server.call_tool("git_fetch", json!({})).is_err());
}

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -q 'Pass dry_run: true to get the current status and what would be fetched without contacting the remote.' crates/backend_core/src/mcp/mod.rs && grep -q '"dry_run": { "type": "boolean", "default": false, "description": "When true, returns git status and what would be fetched' crates/backend_core/src/mcp/mod.rs && grep -q 'let would = format!("fetch {branch} from {remote}");' crates/backend_core/src/mcp/mod.rs && grep -q 'fn mcp_git_fetch_dry_run_reports_status_without_a_remote' crates/backend_core/tests/workflow_integration.rs

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 42C: USAGE — Claude Code and `agy` Commands, Protocol Versions

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 42C: USAGE Part 3 gains the claude mcp add and agy mcp add commands and a Protocol versions note.

[SCOPE_LOCK]
ALLOWED_FILES:
  - docs/USAGE.md
READ_ONLY_REFS:
  - docs/USAGE.md:266-289 (Configuration Snippets: the --root paragraph, Cursor/Claude Code JSON, Codex TOML)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No other wording changes; the existing snippets stay as they are.

[PRE_GATE]
grep -q 'fn mcp_git_fetch_dry_run_reports_status_without_a_remote' crates/backend_core/tests/workflow_integration.rs && ! grep -qF '#### Antigravity CLI' docs/USAGE.md

[MUTATION_SPEC]
1. docs/USAGE.md:282-287. Anchor: the block that starts with the line `#### OpenAI Codex CLI (`~/.codex/config.toml`)` and ends with the closing ``` of its toml snippet (line 287). Insert after that closing ``` line one blank line and then exactly this text (it ends before the existing blank line and `### `plan_import` MCP Tool` heading):
#### Claude Code CLI (`claude mcp add`)
Claude Code starts the server in the directory you run `claude` from, so `--root` isn't needed:
```bash
# Every project on this machine
claude mcp add --scope user nebulous -- neb mcp

# This repository only, shared through a committed .mcp.json
claude mcp add --scope project nebulous -- neb mcp
```

#### Antigravity CLI (`agy mcp add`)
`agy` keeps one global list of MCP servers. Without `--root`, `neb mcp` serves the `.tracker/` found from its working directory, then the active project in `projects.toml` (`neb project switch`):
```bash
agy mcp add nebulous neb mcp

# Pin one repository instead
agy mcp add nebulous -- neb mcp --root /abs/path/to/repo
```

#### Protocol versions
`initialize` answers with the client's `protocolVersion` when it is `2025-11-25`, `2025-06-18`, `2025-03-26` or `2024-11-05`, and with `2025-11-25` otherwise. The `2026-07-28` `server/discover` request gets `-32601 Method not found`; Claude Code then falls back to `initialize`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -qF '#### Claude Code CLI (`claude mcp add`)' docs/USAGE.md && grep -qF 'claude mcp add --scope user nebulous -- neb mcp' docs/USAGE.md && grep -qF '#### Antigravity CLI (`agy mcp add`)' docs/USAGE.md && grep -qF 'agy mcp add nebulous -- neb mcp --root /abs/path/to/repo' docs/USAGE.md && grep -qF '#### Protocol versions' docs/USAGE.md && grep -qF 'Claude Code then falls back to `initialize`.' docs/USAGE.md

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### User Step: Reinstall neb (Phase 24)

**[USER]**. After 42C is COMPLETED and Claude has committed.

```bash
cd /srv/devshare/Nebulous && ./scripts/install.sh --user && neb --version
```

### User Step: Register neb in Claude Code and agy

**[USER]**. These change your own client configs, so you run them. Claude Code user scope means every project; neb picks the repo from the directory `claude` starts in.

```bash
claude mcp add --scope user nebulous -- neb mcp
```

```bash
agy mcp add nebulous neb mcp
```

```bash
claude mcp list && agy mcp list
```

To undo: `claude mcp remove --scope user nebulous` and `agy mcp remove nebulous`.

### User Step: Push and Report CI Matrix (Phase 24)

**[USER]**. After 42C is committed. Push `main`; Claude reads the run results. Don't push a tag.

```bash
cd /srv/devshare/Nebulous && git push
```

### Claude Step: Live Check — Claude Code and agy Call neb

**[CLAUDE]**, after the reinstall, the registration and green CI. Read-only against this repo's tracker; nothing is written.

| Run | Client and call | Expected |
| :--- | :--- | :--- |
| 1 | Claude Code handshake probe (temporary `--mcp-config` with the logging wrapper, one Haiku prompt) | `initialize` asks `2025-11-25` and neb now answers `2025-11-25` (42A); `tools/list` returns 24 |
| 2 | `claude mcp list` | `nebulous` connected |
| 3 | `claude -p` (Haiku, in this repo, only `mcp__nebulous__list_items` and `mcp__nebulous__git_fetch` allowed): list pending items, then `git_fetch` with `dry_run: true` | The pending IDs match `neb list`; `git_fetch` returns `dry_run: true` and a `would` line (42B); HEAD and `.tracker/` unchanged |
| 4 | `agy mcp list` | `nebulous` listed and enabled |
| 5 | `agy -p` (in this repo): list pending items through the `nebulous` server | The pending IDs match `neb list`. Record which project neb served (cwd or registry fallback), since that tells whether `agy` starts servers in the workspace. |

Cost: one Haiku prompt for run 1, one for run 3, and one agy prompt for run 5. Record the result as "Phase 24 live check" below this section. Then the phase ends (Mode 3b).

### Phase 24 live check (2026-11-20)

`neb` installed at `47ff52f`; registered by the user in Claude Code (user scope) and agy. Read-only: HEAD and a hash of `.tracker/` (timer files excluded) were identical before and after every run.

| Run | Client and call | Result |
| :--- | :--- | :--- |
| 1 | Claude Code 2.1.318 handshake probe (temporary `--mcp-config` + logging wrapper, one Haiku prompt) | PASS: client asks `2025-11-25`, neb answers `2025-11-25` (42A); `tools/list` = 24 |
| 2 | `claude mcp list` | PASS: `nebulous: neb mcp - ✔ Connected` |
| 3 | `claude -p` (Haiku, only `mcp__nebulous__list_items` and `mcp__nebulous__git_fetch` allowed) | PASS: `PENDING: TASK-0085, 0086, 0088, 0093, 0094, 0095` = `neb list`; `git_fetch` dry run `true`, "fetch main from <origin URL>" (42B) |
| 4 | `agy mcp list` | PASS: `nebulous stdio enabled neb mcp` |
| 5 | `agy -p` with session-only `--settings '{"permissions":{"allow":["mcp(nebulous/list_items)"]}}'` | PASS: same 6 pending IDs. The registry's active project is Perigee, so neb found Nebulous by cwd: **agy starts stdio servers in the workspace**. |

Run 5 took three tries. Without a rule, headless agy auto-denies MCP calls; `mcp(nebulous)` doesn't match; `mcp(<server>/<tool>)` does (format from agy's own help text). agy's saved config was unchanged (checked against a copy). Filed as TASK-0100. Cost: two Haiku prompts and three short agy prompts.

---

## Phase 25 research — MCP Registry publish (2026-11-20)

Read-only research by two web-fetch subagents (pages listed below), checked against `release.yml` and the repo. Not a plan yet.

**Sources:** github.com/modelcontextprotocol/registry (README, docs/, `docs/reference/server-json/` spec + draft schema + `official-registry-requirements.md`, `docs/reference/cli/commands.md`); modelcontextprotocol.io/registry (quickstart, package-types, authentication, github-actions); github.com/modelcontextprotocol/mcpb (README, MANIFEST.md, CLI.md, `examples/calculator-rust/manifest.json`).

**Facts:**
- The registry is **in preview**: "breaking changes or data resets may occur". Publishing is done with the `mcp-publisher` CLI; committing a `server.json` alone lists nothing.
- Package types: npm, pypi, nuget, cargo (crates.io), oci (ghcr.io and others), **mcpb**. Each verifies ownership differently: cargo needs a visible `mcp-name:` line in the crate README; oci needs an image label; **mcpb must be a GitHub or GitLab release asset whose URL contains the string `mcp`, and `fileSha256` (lowercase hex of the `.mcpb` file) is required**. Our current `.tar.gz` + `.sha256` don't qualify as-is.
- **Namespace** with GitHub auth: `io.github.<username>/<name>`, so `io.github.TheInfoMoose/nebulous` (repo owner per `git remote`). Case sensitivity: not documented.
- **Auth:** `mcp-publisher login github` (device flow, one-off by hand), or `login github-oidc` in GitHub Actions (`permissions: id-token: write`, no secret). The documented workflow pins `mcp-publisher` (v1.8.1 in the example) and verifies its Sigstore bundle with cosign; it doesn't cover building an `.mcpb` or putting its hash into `server.json`.
- **server.json** (`$schema` `https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json`): required `name`, `description` (≤ 100 chars), `version` (should be semver; ranges rejected); a package needs `registryType`, `identifier` (the asset URL), `transport: {"type": "stdio"}`, and for mcpb `fileSha256`. Arguments go in `packageArguments` as positional/named entries. Whether mcpb packages need them (the bundle's manifest already says how to run) is not documented.
- **Lifecycle:** `mcp-publisher status --status active|deprecated|deleted`; "deleted" only hides a version. No delete or overwrite rule is documented, so treat every published version as permanent.
- **MCPB bundle:** a zip with `manifest.json` (required: `manifest_version`, `name`, `version`, `description`, `author.name`, `server`) and the binary, e.g. `server/neb`. Binary servers use `"server": {"type": "binary", "entry_point": "server/neb", "mcp_config": {"command": "${__dirname}/server/neb", "args": [...]}}` (the official Rust example). `compatibility.platforms` takes OS names only (`darwin`, `win32`, `linux`); CPU architecture isn't covered, and neither is one-bundle-per-OS vs one multi-binary bundle. A `user_config` entry with `"type": "directory"` gives the user a folder picker, usable as `"args": ["mcp", "--root", "${user_config.root}"]`. Tool: `npm install -g @anthropic-ai/mcpb` (`init`, `validate`, `pack`, optional `sign`). `manifest_version`: the spec says current is `0.3`, but one section uses `0.4`.
- **Host working directory** for a bundle (e.g. Claude Desktop): not documented. That's why `--root` from `user_config` matters for mcpb, unlike Claude Code / agy, which start `neb mcp` in the workspace (Phase 24).

**Nebulous today:** `release.yml` builds Linux x86_64 only (`.tar.gz`, `.sha256`); CI tests ubuntu/macOS/Windows. Phases 23 and 24 aren't in any release yet (v0.1.0 = `520d576`). Workspace license MIT.

**Options:**

| Route | What it takes | Trade-off |
| :--- | :--- | :--- |
| **mcpb on GitHub Releases** | `release.yml` also packs a `nebulous-mcp-<ver>-linux-x86_64.mcpb` and hashes it; a committed `server.json` with that URL and hash; publish via OIDC from the workflow | Uses the release pipeline we have; no new public registry account. Needs Node in CI for `mcpb pack` (or a plain `zip` of the same layout). Linux first; macOS/Windows bundles later. |
| cargo (crates.io) | Publish `backend_core` and `nebulous_cli` to crates.io, `mcp-name:` line in the README | Users build from source, including the Linux system libraries `release.yml` installs. Crate names must be free, and it's a second public package to maintain. |
| oci (ghcr.io) | A container image with the `io.modelcontextprotocol.server.name` label | neb needs the user's repo on disk, so users must mount it into the container. That's awkward for a git-backed tracker. |

**Still unverified** (to settle while planning Phase 25, before any spec is written): namespace case sensitivity; whether mcpb packages need `packageArguments`; `manifest_version` 0.3 vs 0.4; how several platform bundles are listed (one `packages` entry each?); whether a plain `zip` passes `mcpb validate`.

**Settled 2026-11-20 (planning Phase 25), from the registry and mcpb source:**
1. **Namespace case:** case-sensitive. The OIDC handler grants `io.github.%s/*` from `repository_owner` as GitHub returns it (`internal/api/handlers/v0/auth/github_oidc.go:293`), `internal/auth/jwt.go` matches with `==` / `HasPrefix`, and the name regex allows `A-Z`. The owner login is `TheInfoMoose`, so the name is exactly `io.github.TheInfoMoose/nebulous`.
2. **`packageArguments`:** not needed. The official mcpb example (`generic-server-json.md`) has only `registryType`, `identifier`, `fileSha256`, `transport`; `registries/mcpb.go` never reads arguments. `Package.version` is optional, so it's left out.
3. **`manifest_version`:** `0.3`. mcpb 2.1.2 accepts 0.1–0.4; 0.4 only adds the `uv` server type; `mcpb init` writes 0.3 (`DEFAULT_MANIFEST_VERSION`), and the Rust example uses 0.3.
4. **Several platforms:** one `packages` entry per bundle, each with its own `identifier` + `fileSha256`. There's no CPU-architecture field anywhere. Not needed while Linux x86_64 is the only bundle.
5. **Packing:** `@anthropic-ai/mcpb@2.1.2` (npm, 2026-10-07) `pack`, not plain `zip`. `validate` reads a manifest or a directory, not a `.mcpb`; `pack` runs it, keeps the exec bit on `server/neb`, and applies the default excludes. Node 22 via `actions/setup-node@v4`.
6. **Registry checks on an mcpb package** (`registries/mcpb.go`): `fileSha256` present; https; host github.com/gitlab.com; a `/owner/repo/releases/download/tag/file` path; `mcp` somewhere in the URL; a HEAD request answering 200 or 3xx. It never downloads the file or checks the hash (clients do), so the publish job checks the hash itself.
7. **Pins:** `mcp-publisher` **v1.8.8** (2026-11-06; v1.8.9 and v1.8.10 are under two weeks old). `sigstore/cosign-installer@6f9f17788090df1f26f669e9d70d6ae9567deba6` (v4.1.2, 2026-05-07), as in the registry's own GitHub Actions doc. `mcp-publisher validate <file>` exists at v1.8.8 (`cmd/publisher/commands/validate.go`).
8. `mcp_config` allows only `${__dirname}`, `${pathSeparator}`, `${/}` and `${user_config.<key>}` (mcpb `validate.ts`), so `--root ${user_config.root}` with a `"type": "directory"`, `"required": true` entry is the way to hand neb its repo.
9. The release binary links only glibc, OpenSSL (`libssl.so.3`, `libcrypto.so.3`) and zlib (`ldd target/release/neb`).

## Phase 25 — Nebulous in the MCP Registry as a Linux `.mcpb` (planned 2026-11-20)

Pending items picked for this phase (user, 2026-11-20): **TASK-0088** (43A), **TASK-0095** and **TASK-0100** (43B). Left pending: TASK-0085, TASK-0086, TASK-0093, TASK-0094.

**Design choices (part of what you approve):**
- `mcpb/manifest.json` and `server.json` are committed **templates** with `version` `0.0.0` and an all-zero `fileSha256`. `release.yml` fills in the version, the asset URL and the hash from the tag with `jq`. Nothing has to be bumped by hand, and `bump-version.sh` / `release-check.sh` don't change.
- The registry publish runs in its own job behind a GitHub **environment** `mcp-registry` with you as required reviewer. A registry version is permanent, so the job waits for your click after Claude has checked the live release asset.
- TASK-0095 is resolved by labelling, not testing: Cursor and Codex snippets stay, marked untested; the Ollama claim goes (it has no config to test). No Cursor, Codex or Ollama is installed here.
- Only templates change in 43C; the jq render is exercised by 43D's POST_GATE against the committed `server.json`.

Specs follow the hybrid-workflow rule: anchors and gate greps were checked against HEAD `d9a20a1`; no step code was applied or built.

### STEP 43A: `neb init` Ignores the Timer Files

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 43A: The `.tracker/.gitignore` that `neb init` writes (and that `with_write_lock` recreates when it's missing) lists `.lock`, `.timer.toml` and `.timer.toml.bak`, so new trackers never commit timer state.

[SCOPE_LOCK]
ALLOWED_FILES:
  - crates/backend_core/src/storage/mod.rs
READ_ONLY_REFS:
  - crates/backend_core/src/storage/mod.rs:22-23 (LOCK_FILE_NAME)
  - crates/backend_core/src/storage/mod.rs:350-354 (init: writes .tracker/.gitignore when absent)
  - crates/backend_core/src/storage/mod.rs:433 (write_tracker_file(&self, path: &Path, contents: &str))
  - crates/backend_core/src/storage/mod.rs:480-483 (timer_toml_path), :892-906 (.timer.toml.bak quarantine)
  - crates/backend_core/src/storage/mod.rs:554-571 (ensure_lock_gitignore)
  - crates/backend_core/src/storage/mod.rs:1663-1704 (mod lock_tests, write_lock_creates_lock_file_and_gitignore_entry)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. The append branch of `ensure_lock_gitignore` (an existing `.gitignore` without `.lock` gets only `.lock` appended) stays exactly as it is. `LOCK_FILE_NAME` keeps its value. No other function changes.

[PRE_GATE]
grep -qF 'storage.write_tracker_file(&gitignore_path, &format!("{LOCK_FILE_NAME}\n"))?;' crates/backend_core/src/storage/mod.rs && ! grep -q 'DEFAULT_TRACKER_GITIGNORE' crates/backend_core/src/storage/mod.rs

[MUTATION_SPEC]
1. storage/mod.rs: directly after the line `pub const LOCK_FILE_NAME: &str = ".lock";` (line 23), add a blank line and:
   /// Contents of the `.tracker/.gitignore` that `init` writes: the lock file and the
   /// timer's runtime state, none of which belong in git.
   pub const DEFAULT_TRACKER_GITIGNORE: &str = ".lock\n.timer.toml\n.timer.toml.bak\n";
2. storage/mod.rs, in `init` (line 353): replace
   storage.write_tracker_file(&gitignore_path, &format!("{LOCK_FILE_NAME}\n"))?;
   with
   storage.write_tracker_file(&gitignore_path, DEFAULT_TRACKER_GITIGNORE)?;
3. storage/mod.rs, in `ensure_lock_gitignore`, the `else` branch (line 569): replace
   self.write_tracker_file(&gitignore_path, &format!("{LOCK_FILE_NAME}\n"))
   with
   self.write_tracker_file(&gitignore_path, DEFAULT_TRACKER_GITIGNORE)
   Why: a missing file is recreated as a fresh generated file. The append branch is left alone so existing trackers in other repos aren't rewritten on every lock.
4. Tests: in `mod lock_tests`, directly after `fn write_lock_creates_lock_file_and_gitignore_entry` ends, add exactly:
    #[test]
    fn init_gitignore_lists_lock_and_timer_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let storage = TrackerStorage::init_standard(dir.path(), "t").expect("init_standard");
        let gitignore = storage.tracker_dir().join(".gitignore");
        let contents = fs::read_to_string(&gitignore).expect("read gitignore");
        assert_eq!(contents, DEFAULT_TRACKER_GITIGNORE);
        let lines: Vec<&str> = contents.lines().collect();
        assert_eq!(lines, vec![LOCK_FILE_NAME, ".timer.toml", ".timer.toml.bak"]);
    }

    #[test]
    fn write_lock_recreates_missing_gitignore_with_timer_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let storage = TrackerStorage::init_standard(dir.path(), "t").expect("init_standard");
        let gitignore = storage.tracker_dir().join(".gitignore");
        fs::remove_file(&gitignore).expect("remove gitignore");
        storage.with_write_lock(|_| Ok(())).expect("with_write_lock");
        let contents = fs::read_to_string(&gitignore).expect("read gitignore");
        assert_eq!(contents, DEFAULT_TRACKER_GITIGNORE);
    }

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -qF 'pub const DEFAULT_TRACKER_GITIGNORE: &str = ".lock\n.timer.toml\n.timer.toml.bak\n";' crates/backend_core/src/storage/mod.rs && grep -q 'fn init_gitignore_lists_lock_and_timer_files' crates/backend_core/src/storage/mod.rs && grep -q 'fn write_lock_recreates_missing_gitignore_with_timer_files' crates/backend_core/src/storage/mod.rs && ! grep -qF 'format!("{LOCK_FILE_NAME}\n")' crates/backend_core/src/storage/mod.rs && test "$(grep -c 'DEFAULT_TRACKER_GITIGNORE' crates/backend_core/src/storage/mod.rs)" -ge 5

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 43B: Docs — Which MCP Clients Are Tested, agy Headless Allow Rule

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 43B: README and USAGE name only the tested MCP clients (Orrery, Claude Code, Antigravity `agy`), mark the Cursor and Codex snippets untested, drop the Ollama claim, and USAGE gives the `mcp(nebulous/<tool>)` allow rule that headless `agy -p` needs.

[SCOPE_LOCK]
ALLOWED_FILES:
  - README.md
  - docs/USAGE.md
READ_ONLY_REFS:
  - README.md:147-160 (### 4. Configure `neb mcp` for AI Coding Agents)
  - docs/USAGE.md:6 (intro bullet for `neb mcp`)
  - docs/USAGE.md:262-310 (Part 3: Configuration Snippets, Antigravity CLI, Protocol versions)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. The JSON and TOML snippets themselves, the Claude Code and `agy mcp add` commands, and every other section stay unchanged.

[PRE_GATE]
grep -qF 'local **Ollama** agent harnesses' README.md && grep -qF 'server for Cursor, Claude Code, Codex, Ollama, and automated self-healing CI/CD pipelines.' docs/USAGE.md && ! grep -qF 'mcp(nebulous/' docs/USAGE.md

[MUTATION_SPEC]
1. README.md:149: replace the whole line
   Run `neb mcp` as a JSON-RPC 2.0 `stdio` server in **Cursor** (`.cursor/mcp.json`), **Claude Code** (`.mcp.json`), **Codex**, or local **Ollama** agent harnesses:
   with
   Run `neb mcp` as a JSON-RPC 2.0 `stdio` server. Tested with **Orrery**, **Claude Code** and **Antigravity** (`agy`); the Cursor and Codex snippets in [`docs/USAGE.md`](docs/USAGE.md) are untested. A client config looks like this:
2. docs/USAGE.md:6: replace
   server for Cursor, Claude Code, Codex, Ollama, and automated self-healing CI/CD pipelines.
   with
   server for Claude Code, Antigravity (`agy`), Orrery, and automated self-healing CI/CD pipelines.
3. docs/USAGE.md, under `### Configuration Snippets`: directly after the paragraph that starts "`--root` is optional." (line 268) add a blank line and this paragraph:
   Tested clients: Orrery, Claude Code and Antigravity (`agy`). The Cursor and Codex snippets follow each client's documented format but haven't been run against `neb mcp`.
4. docs/USAGE.md: change the heading
   #### Cursor (`.cursor/mcp.json`) & Claude Code (`.mcp.json`)
   to
   #### Cursor (`.cursor/mcp.json`, untested) & Claude Code (`.mcp.json`)
5. docs/USAGE.md: change the heading
   #### OpenAI Codex CLI (`~/.codex/config.toml`)
   to
   #### OpenAI Codex CLI (`~/.codex/config.toml`, untested)
6. docs/USAGE.md, `#### Antigravity CLI (`agy mcp add`)`: directly after the closing ``` of its bash block (the block ending with `agy mcp add nebulous -- neb mcp --root /abs/path/to/repo`), add a blank line and:
   Headless runs (`agy -p`) can't ask for permission, so agy 1.5.12 denies every MCP call unless `permissions.allow` names each tool as `mcp(nebulous/<tool>)`. `mcp(nebulous)` alone doesn't match. For one session:
   ```bash
   agy -p 'call nebulous list_items' --settings '{"permissions":{"allow":["mcp(nebulous/list_items)"]}}'
   ```
   The same `permissions.allow` list works in agy's settings file.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && ! grep -qiE '\bollama\b' README.md docs/USAGE.md && grep -qF 'Tested with **Orrery**, **Claude Code** and **Antigravity** (`agy`)' README.md && grep -qF 'Tested clients: Orrery, Claude Code and Antigravity (`agy`).' docs/USAGE.md && grep -qF '#### Cursor (`.cursor/mcp.json`, untested) & Claude Code (`.mcp.json`)' docs/USAGE.md && grep -qF '#### OpenAI Codex CLI (`~/.codex/config.toml`, untested)' docs/USAGE.md && grep -qF "agy -p 'call nebulous list_items' --settings '{\"permissions\":{\"allow\":[\"mcp(nebulous/list_items)\"]}}'" docs/USAGE.md && grep -qF '`mcp(nebulous)` alone doesn'"'"'t match.' docs/USAGE.md

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 43C: MCP Bundle Manifest and `server.json` Templates

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 43C: Two new committed templates: `mcpb/manifest.json` (MCPB 0.3, binary server `server/neb` run as `neb mcp --root ${user_config.root}`, Linux) and `server.json` (registry entry `io.github.TheInfoMoose/nebulous`, one mcpb package). Both hold version `0.0.0` and placeholder values that release.yml fills in (43D).

[SCOPE_LOCK]
ALLOWED_FILES:
  - mcpb/manifest.json (NEW)
  - server.json (NEW)
READ_ONLY_REFS:
  - Cargo.toml:9-14 ([workspace.package]: license = "MIT", authors = ["Nebulous Contributors"])
  - crates/nebulous_cli/src/args.rs (`neb mcp --root <PATH>`)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. No other keys than the ones below; both files use two-space indentation and end with a newline.

[PRE_GATE]
test ! -e mcpb/manifest.json && test ! -e server.json

[MUTATION_SPEC]
1. mcpb/manifest.json (NEW), exactly:
   {
     "manifest_version": "0.3",
     "name": "nebulous",
     "display_name": "Nebulous",
     "version": "0.0.0",
     "description": "Git-backed project and bug tracker for coding agents, served by neb mcp from a repository's .tracker/ folder.",
     "author": {
       "name": "Nebulous Contributors"
     },
     "repository": {
       "type": "git",
       "url": "https://github.com/TheInfoMoose/Nebulous"
     },
     "license": "MIT",
     "server": {
       "type": "binary",
       "entry_point": "server/neb",
       "mcp_config": {
         "command": "${__dirname}/server/neb",
         "args": ["mcp", "--root", "${user_config.root}"]
       }
     },
     "user_config": {
       "root": {
         "type": "directory",
         "title": "Project folder",
         "description": "The repository whose .tracker/ folder neb serves.",
         "required": true
       }
     },
     "compatibility": {
       "platforms": ["linux"]
     }
   }
   Why `version` 0.0.0: release.yml writes the tag's version into the packed copy; the committed file is a template.
2. server.json (NEW, repository root), exactly:
   {
     "$schema": "https://static.modelcontextprotocol.io/schemas/2025-12-11/server.schema.json",
     "name": "io.github.TheInfoMoose/nebulous",
     "title": "Nebulous",
     "description": "Git-backed project and bug tracker for coding agents: items, plans, timers and commits.",
     "version": "0.0.0",
     "repository": {
       "url": "https://github.com/TheInfoMoose/Nebulous",
       "source": "github"
     },
     "packages": [
       {
         "registryType": "mcpb",
         "identifier": "https://github.com/TheInfoMoose/Nebulous/releases/download/v0.0.0/nebulous-mcp-v0.0.0-linux-x86_64.mcpb",
         "fileSha256": "0000000000000000000000000000000000000000000000000000000000000000",
         "transport": {
           "type": "stdio"
         }
       }
     ]
   }
   Why: `name` is case-sensitive and must match the GitHub owner `TheInfoMoose` exactly. `description` is 87 characters (limit 100; the manifest's own description has no limit). release.yml replaces `version`, `identifier` and `fileSha256`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && bash scripts/check-neutral.sh mcpb/manifest.json server.json && jq -e . mcpb/manifest.json >/dev/null && jq -e . server.json >/dev/null && python3 -I -c 'import json; m=json.load(open("mcpb/manifest.json")); s=json.load(open("server.json")); p=s["packages"][0]; assert m["manifest_version"]=="0.3" and m["version"]=="0.0.0" and m["server"]=={"type":"binary","entry_point":"server/neb","mcp_config":{"command":"${__dirname}/server/neb","args":["mcp","--root","${user_config.root}"]}} and m["user_config"]["root"]["type"]=="directory" and m["user_config"]["root"]["required"] is True and m["compatibility"]=={"platforms":["linux"]}; assert s["name"]=="io.github.TheInfoMoose/nebulous" and s["version"]=="0.0.0" and len(s["description"])<=100 and len(s["packages"])==1 and p["registryType"]=="mcpb" and p["fileSha256"]=="0"*64 and p["transport"]=={"type":"stdio"} and "mcp" in p["identifier"]; assert set(s)=={"$schema","name","title","description","version","repository","packages"} and set(p)=={"registryType","identifier","fileSha256","transport"}' && tail -c1 mcpb/manifest.json | od -An -c | grep -q '\\n' && tail -c1 server.json | od -An -c | grep -q '\\n'

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 43D: Release Workflow Packs, Hashes and Smoke-Tests the `.mcpb`

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 43D: The release build job packs `nebulous-mcp-<tag>-linux-x86_64.mcpb` with pinned `@anthropic-ai/mcpb@2.1.2`, renders `dist/server.json` with the tag's version, the asset URL and the bundle's SHA-256, smoke-tests the bundle (unzip, exec bit, version, MCP `initialize`), and the GitHub Release attaches the `.mcpb`.

[SCOPE_LOCK]
ALLOWED_FILES:
  - .github/workflows/release.yml
READ_ONLY_REFS:
  - .github/workflows/release.yml:60-99 (build release binary, package .tar.gz, smoke test, upload-artifact `release-dist`)
  - .github/workflows/release.yml:101-121 (job `publish`, softprops/action-gh-release `files:`)
  - mcpb/manifest.json, server.json (templates from 43C)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Existing steps keep their names, order and commands; only the two new steps are inserted and the two path lists below grow. No new job (43E adds it). Top-level `permissions` stays `contents: read`.

[PRE_GATE]
test -f mcpb/manifest.json && test -f server.json && ! grep -q 'mcpb' .github/workflows/release.yml

[MUTATION_SPEC]
1. release.yml, job `build-linux-x86_64`: directly after the step `- name: Smoke-test archive install (extract → install.sh → neb --help)` (its `run:` block ends with `"${SMOKE_DIR}/prefix/bin/neb" --help`) and before `- name: Upload release archive and checksum`, insert these two steps (6-space indent for `- name`, same as the other steps), with one blank line before each:
      - name: Set up Node.js (for the mcpb CLI)
        uses: actions/setup-node@v4
        with:
          node-version: "22"

      - name: Pack MCP bundle (`.mcpb`) and render `server.json`
        run: |
          set -euo pipefail
          TAG="${GITHUB_REF_NAME}"
          VERSION="${TAG#v}"
          MCPB_NAME="nebulous-mcp-${TAG}-linux-x86_64.mcpb"
          STAGE_DIR="${RUNNER_TEMP}/mcpb-stage"

          mkdir -p "${STAGE_DIR}/server"
          jq --arg v "${VERSION}" '.version = $v' mcpb/manifest.json > "${STAGE_DIR}/manifest.json"
          install -m 0755 target/release/neb "${STAGE_DIR}/server/neb"
          cp LICENSE "${STAGE_DIR}/"

          npx --yes @anthropic-ai/mcpb@2.1.2 pack "${STAGE_DIR}" "dist/${MCPB_NAME}"

          SHA="$(sha256sum "dist/${MCPB_NAME}" | cut -d' ' -f1)"
          URL="https://github.com/${GITHUB_REPOSITORY}/releases/download/${TAG}/${MCPB_NAME}"
          jq --arg v "${VERSION}" --arg url "${URL}" --arg sha "${SHA}" \
            '.version = $v | .packages[0].identifier = $url | .packages[0].fileSha256 = $sha' \
            server.json > dist/server.json

      - name: Smoke-test MCP bundle (unzip → version → initialize)
        run: |
          set -euo pipefail
          TAG="${GITHUB_REF_NAME}"
          VERSION="${TAG#v}"
          MCPB_NAME="nebulous-mcp-${TAG}-linux-x86_64.mcpb"
          SMOKE_DIR="${RUNNER_TEMP}/mcpb-smoke"

          mkdir -p "${SMOKE_DIR}/bundle" "${SMOKE_DIR}/repo"
          unzip -q "dist/${MCPB_NAME}" -d "${SMOKE_DIR}/bundle"
          test -x "${SMOKE_DIR}/bundle/server/neb"
          test "$(jq -r '.version' "${SMOKE_DIR}/bundle/manifest.json")" = "${VERSION}"
          test "$("${SMOKE_DIR}/bundle/server/neb" --version)" = "neb ${VERSION}"
          test "$(jq -r '.version' dist/server.json)" = "${VERSION}"
          test "$(jq -r '.packages[0].fileSha256' dist/server.json)" = "$(sha256sum "dist/${MCPB_NAME}" | cut -d' ' -f1)"
          test "$(jq -r '.packages[0].identifier' dist/server.json)" = "https://github.com/${GITHUB_REPOSITORY}/releases/download/${TAG}/${MCPB_NAME}"

          "${SMOKE_DIR}/bundle/server/neb" init --root "${SMOKE_DIR}/repo" --no-ingest --no-register
          printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"release-smoke","version":"0"}}}' \
            | timeout 30 "${SMOKE_DIR}/bundle/server/neb" mcp --root "${SMOKE_DIR}/repo" > "${SMOKE_DIR}/initialize.json"
          grep -Eq '"protocolVersion": ?"2025-11-25"' "${SMOKE_DIR}/initialize.json"
   Why: the registry never checks the hash, so a wrong `fileSha256` would publish and then fail every client install; the smoke step proves the rendered file matches the packed bundle. The initialize output goes to a file (not `grep -q` in the pipe) so `neb mcp` never gets SIGPIPE under `pipefail`.
2. release.yml, step `Upload release archive and checksum`, its `path: |` list: after the line `dist/*.tar.gz.sha256` add two lines at the same indent:
            dist/*.mcpb
            dist/server.json
   (`dist/RELEASE_NOTES.md` stays last.)
3. release.yml, job `publish`, step `Publish GitHub Release`, its `files: |` list: after the line `dist/*.tar.gz.sha256` add one line at the same indent:
            dist/*.mcpb
   Why: `server.json` isn't attached to the release; it travels to 43E's job through the `release-dist` artifact only.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && python3 -I -c 'import yaml; w=yaml.safe_load(open(".github/workflows/release.yml")); steps=[s.get("name","") for s in w["jobs"]["build-linux-x86_64"]["steps"]]; i=steps.index("Smoke-test archive install (extract → install.sh → neb --help)"); assert steps[i+1:i+5]==["Set up Node.js (for the mcpb CLI)","Pack MCP bundle (`.mcpb`) and render `server.json`","Smoke-test MCP bundle (unzip → version → initialize)","Upload release archive and checksum"], steps; up=w["jobs"]["build-linux-x86_64"]["steps"][i+4]["with"]["path"]; assert "dist/*.mcpb" in up and "dist/server.json" in up; files=w["jobs"]["publish"]["steps"][-1]["with"]["files"]; assert "dist/*.mcpb" in files and "server.json" not in files; assert set(w["jobs"])=={"build-linux-x86_64","publish"}' && grep -qF 'npx --yes @anthropic-ai/mcpb@2.1.2 pack "${STAGE_DIR}" "dist/${MCPB_NAME}"' .github/workflows/release.yml && grep -qF 'MCPB_NAME="nebulous-mcp-${TAG}-linux-x86_64.mcpb"' .github/workflows/release.yml && grep -qF '.version = $v | .packages[0].identifier = $url | .packages[0].fileSha256 = $sha' .github/workflows/release.yml && test "$(jq --arg v 1.2.3 --arg url https://github.com/example/Nebulous/releases/download/v1.2.3/nebulous-mcp-v1.2.3-linux-x86_64.mcpb --arg sha abc '.version = $v | .packages[0].identifier = $url | .packages[0].fileSha256 = $sha' server.json | jq -r '[.version, .packages[0].fileSha256, .name] | join(" ")')" = "1.2.3 abc io.github.TheInfoMoose/nebulous" && test "$(jq --arg v 1.2.3 '.version = $v' mcpb/manifest.json | jq -r '.version')" = "1.2.3"

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 43E: Release Workflow Publishes to the MCP Registry (After Your Approval)

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 43E: A new release job `publish-registry` runs after the GitHub Release, waits in the `mcp-registry` environment for approval, checks the live release asset against `dist/server.json`'s hash, installs cosign-verified `mcp-publisher` v1.8.8, validates `server.json`, logs in with GitHub OIDC and publishes.

[SCOPE_LOCK]
ALLOWED_FILES:
  - .github/workflows/release.yml
READ_ONLY_REFS:
  - .github/workflows/release.yml (job `publish`, the last job; artifact `release-dist` now holds dist/server.json and dist/*.mcpb from 43D)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. Jobs `build-linux-x86_64` and `publish` stay byte-for-byte as 43D left them. Top-level `permissions` stays `contents: read`; `id-token: write` is granted only inside the new job.

[PRE_GATE]
grep -qF 'npx --yes @anthropic-ai/mcpb@2.1.2 pack' .github/workflows/release.yml && ! grep -q 'publish-registry' .github/workflows/release.yml

[MUTATION_SPEC]
1. release.yml: at the end of the file, after the `publish` job's last line (`            dist/*.mcpb`), add one blank line and this job (2-space indent for the job key, as for `publish:`):
  publish-registry:
    name: Publish to MCP Registry
    needs: publish
    runs-on: ubuntu-latest
    environment: mcp-registry
    permissions:
      contents: read
      id-token: write
    env:
      MCP_PUBLISHER_VERSION: v1.8.8
    steps:
      - name: Download rendered server.json
        uses: actions/download-artifact@v4
        with:
          name: release-dist
          path: dist

      - name: Check server.json against the published release asset
        run: |
          set -euo pipefail
          test "$(jq -r '.name' dist/server.json)" = "io.github.TheInfoMoose/nebulous"
          test "$(jq -r '.version' dist/server.json)" = "${GITHUB_REF_NAME#v}"
          URL="$(jq -r '.packages[0].identifier' dist/server.json)"
          SHA="$(jq -r '.packages[0].fileSha256' dist/server.json)"
          test "${SHA}" != "0000000000000000000000000000000000000000000000000000000000000000"
          curl --fail --location --silent --show-error --output "${RUNNER_TEMP}/release-asset.mcpb" "${URL}"
          test "$(sha256sum "${RUNNER_TEMP}/release-asset.mcpb" | cut -d' ' -f1)" = "${SHA}"

      - name: Install cosign
        uses: sigstore/cosign-installer@6f9f17788090df1f26f669e9d70d6ae9567deba6 # v4.1.2

      - name: Download and verify mcp-publisher
        run: |
          set -euo pipefail
          archive="mcp-publisher_linux_amd64.tar.gz"
          base_url="https://github.com/modelcontextprotocol/registry/releases/download/${MCP_PUBLISHER_VERSION}"
          curl --fail --location --silent --show-error --output "$archive" "$base_url/$archive"
          curl --fail --location --silent --show-error --output "$archive.sigstore.json" "$base_url/$archive.sigstore.json"
          cosign verify-blob \
            --bundle "$archive.sigstore.json" \
            --certificate-identity "https://github.com/modelcontextprotocol/registry/.github/workflows/release.yml@refs/tags/${MCP_PUBLISHER_VERSION}" \
            --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
            "$archive"
          tar xzf "$archive" mcp-publisher
          rm -f "$archive" "$archive.sigstore.json"

      - name: Validate server.json
        run: ./mcp-publisher validate dist/server.json

      - name: Authenticate to MCP Registry (GitHub OIDC)
        run: ./mcp-publisher login github-oidc

      - name: Publish server to MCP Registry
        run: ./mcp-publisher publish dist/server.json
   Why: `environment: mcp-registry` makes the job wait for your approval (you add yourself as required reviewer before tagging), because a registry version can't be overwritten. The asset check downloads the public URL, the same thing a client does. The cosign step is the registry's own documented snippet with the OS/arch fixed to `linux_amd64`.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && python3 -I -c 'import yaml; w=yaml.safe_load(open(".github/workflows/release.yml")); assert list(w["jobs"])==["build-linux-x86_64","publish","publish-registry"]; j=w["jobs"]["publish-registry"]; assert j["needs"]=="publish" and j["environment"]=="mcp-registry" and j["permissions"]=={"contents":"read","id-token":"write"} and j["env"]=={"MCP_PUBLISHER_VERSION":"v1.8.8"}; assert [s["name"] for s in j["steps"]]==["Download rendered server.json","Check server.json against the published release asset","Install cosign","Download and verify mcp-publisher","Validate server.json","Authenticate to MCP Registry (GitHub OIDC)","Publish server to MCP Registry"]; assert w["permissions"]=={"contents":"read"}' && grep -qF 'uses: sigstore/cosign-installer@6f9f17788090df1f26f669e9d70d6ae9567deba6 # v4.1.2' .github/workflows/release.yml && grep -qF -- '--certificate-identity "https://github.com/modelcontextprotocol/registry/.github/workflows/release.yml@refs/tags/${MCP_PUBLISHER_VERSION}"' .github/workflows/release.yml && grep -qF 'run: ./mcp-publisher publish dist/server.json' .github/workflows/release.yml && grep -qF 'test "$(sha256sum "${RUNNER_TEMP}/release-asset.mcpb" | cut -d'"' '"' -f1)" = "${SHA}"' .github/workflows/release.yml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### STEP 43F: CHANGELOG, README "Releasing" and USAGE for the Bundle

* **Recommended Command:** `/goal`

```text
[STEP_ID]
STEP 43F: CHANGELOG `## [Unreleased]` lists everything since v0.1.0 (Phases 23–25); README "Releasing" says how the bundle and registry publish work; USAGE has an "MCP bundle (`.mcpb`)" section.

[SCOPE_LOCK]
ALLOWED_FILES:
  - CHANGELOG.md
  - README.md
  - docs/USAGE.md
READ_ONLY_REFS:
  - CHANGELOG.md:1-8 (header, empty `## [Unreleased]`, `## [0.1.0] - 2026-11-18`)
  - README.md:184-190 (## Releasing, steps 1–3)
  - docs/USAGE.md: `#### Protocol versions` (Part 3)
FORBIDDEN_SCOPE:
  - Any file not listed in ALLOWED_FILES. The `## [0.1.0]` section and everything below it in CHANGELOG.md stay byte-for-byte. No version number changes anywhere (the user bumps to 0.2.0).

[PRE_GATE]
grep -q 'publish-registry' .github/workflows/release.yml && grep -qF '#### OpenAI Codex CLI (`~/.codex/config.toml`, untested)' docs/USAGE.md && test "$(awk '/^## \[Unreleased\]$/{f=1;next} f&&/^## \[/{exit} f&&NF{n++} END{print n+0}' CHANGELOG.md)" -eq 0

[MUTATION_SPEC]
1. CHANGELOG.md: between the line `## [Unreleased]` and the line `## [0.1.0] - 2026-11-18`, the content becomes exactly (one blank line after the heading and one before `## [0.1.0]`):
   ### Added
   - MCP bundle: each release attaches `nebulous-mcp-vX.Y.Z-linux-x86_64.mcpb` (Linux x86_64), and the release workflow lists it in the MCP Registry as `io.github.TheInfoMoose/nebulous` once the `mcp-registry` environment approves.
   - MCP: protocol `2025-11-25`, now the default.
   - MCP: `git_fetch` takes `dry_run` (default `false`), which reports status without contacting a remote, so it works in a repo with no `origin`.
   - MCP: `plan_import` accepts the reserved `orrery` object, and `update_item_status` and `close_item_with_fix` find the item from `orrery.step_title`.

   ### Changed
   - `neb plan import` (CLI and MCP) no longer creates items for Completed rows or retitles closed or merged items.
   - `neb init` writes a `.tracker/.gitignore` that also lists `.timer.toml` and `.timer.toml.bak`.
   - Docs: verified `claude mcp add` and `agy mcp add` commands, the allow rule headless `agy -p` needs, and which MCP clients are tested. The Cursor and Codex snippets are marked untested; the Ollama claim is gone.
2. README.md, `## Releasing`: after numbered item 3 (the line starting `3. Before building, the release workflow runs`), add:
   4. The release also packs `nebulous-mcp-vX.Y.Z-linux-x86_64.mcpb` from `mcpb/manifest.json` and renders `server.json` with the tag's version, the bundle's URL and its SHA-256. Both committed files are templates: leave their `version` at `0.0.0`.
   5. The `publish-registry` job then waits for approval in the `mcp-registry` environment. Approve it only after the bundle on the release has been checked. It re-checks the hash, then publishes to the MCP Registry. A registry version can't be overwritten, so a mistake needs a new version.
3. docs/USAGE.md, Part 3: directly before the heading `#### Protocol versions`, add this section followed by a blank line:
   #### MCP bundle (`.mcpb`)
   Each release attaches `nebulous-mcp-vX.Y.Z-linux-x86_64.mcpb` (Linux x86_64 only), listed in the MCP Registry as `io.github.TheInfoMoose/nebulous`. A host that installs MCP bundles asks for **Project folder**, the repository whose `.tracker/` neb serves, and runs `server/neb mcp --root <that folder>`. The binary needs glibc, OpenSSL 3 (`libssl.so.3`) and zlib on the system.

[POST_GATE]
cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo deny check advisories bans && bash scripts/check-neutral.sh && grep -qx '## \[Unreleased\]' CHANGELOG.md && grep -qx '## \[0.1.0\] - 2026-11-18' CHANGELOG.md && test "$(awk '/^## \[Unreleased\]$/{f=1;next} f&&/^## \[/{exit} f&&/^- /{n++} END{print n+0}' CHANGELOG.md)" -eq 7 && grep -qF -- '- MCP bundle: each release attaches `nebulous-mcp-vX.Y.Z-linux-x86_64.mcpb`' CHANGELOG.md && test "$(git show HEAD:CHANGELOG.md | sed -n '/^## \[0.1.0\]/,$p' | sha256sum)" = "$(sed -n '/^## \[0.1.0\]/,$p' CHANGELOG.md | sha256sum)" && grep -qF '5. The `publish-registry` job then waits for approval in the `mcp-registry` environment.' README.md && grep -qF '#### MCP bundle (`.mcpb`)' docs/USAGE.md && test "$(grep -n '#### MCP bundle (`.mcpb`)' docs/USAGE.md | cut -d: -f1)" -lt "$(grep -n '#### Protocol versions' docs/USAGE.md | cut -d: -f1)" && grep -q '^version = "0.1.0"' Cargo.toml

[HALT_INVARIANT]
Halt immediately after POST_GATE exits 0 and report verification status.
```

### User Step: Push and Report CI Matrix (Phase 25)

**[USER]**. After 43F is committed. This also pushes `d9a20a1` (Phase 24 live-check close), which is still local. Don't push a tag.

```bash
cd /srv/devshare/Nebulous && git push
```

### User Step: Create the `mcp-registry` Environment

**[USER]**. Changes repo settings, so you run it. Adds you as the required reviewer, so `publish-registry` waits for your click. Do this **before** tagging: if the environment doesn't exist, GitHub creates it on first use with no protection and the job publishes straight away.

```bash
gh api -X PUT repos/TheInfoMoose/Nebulous/environments/mcp-registry -F 'reviewers[][type]=User' -F "reviewers[][id]=$(gh api user --jq .id)"
```

```bash
gh api repos/TheInfoMoose/Nebulous/environments/mcp-registry --jq '.protection_rules'
```

The second command should list a `required_reviewers` rule with your login. (Same thing in the UI: Settings → Environments → New environment `mcp-registry` → Required reviewers.)

### User Step: Release v0.2.0

**[USER]**. After green CI and the environment exists.

```bash
cd /srv/devshare/Nebulous && scripts/bump-version.sh --dry-run 0.2.0
```

```bash
scripts/bump-version.sh 0.2.0
```

Then run the commit, tag and push commands it prints (`git add Cargo.toml Cargo.lock CHANGELOG.md`, `git commit -m "Release v0.2.0"`, `git tag v0.2.0`, `git push origin main v0.2.0`). Don't approve `publish-registry` yet.

### Claude Step: Check the Bundle Before Approval

**[CLAUDE]**, while `publish-registry` is waiting. Read-only.

| Run | Check | Expected |
| :--- | :--- | :--- |
| 1 | `gh run view` for the v0.2.0 release run | `build-linux-x86_64` and `publish` green; `publish-registry` waiting |
| 2 | `gh release view v0.2.0 --json assets` | `nebulous-mcp-v0.2.0-linux-x86_64.mcpb` next to the `.tar.gz` and `.sha256` |
| 3 | `gh run download` the `release-dist` artifact into the scratchpad; `jq` its `server.json` | name `io.github.TheInfoMoose/nebulous`, version `0.2.0`, identifier = the release asset URL |
| 4 | Download the `.mcpb` from the public URL into its own scratch dir; `sha256sum` | equals `fileSha256` |
| 5 | `unzip -l`, then unzip into a scratch dir; `manifest.json` and `server/neb --version` | `manifest_version` 0.3, version 0.2.0, `neb 0.2.0`, exec bit set |
| 6 | `server/neb mcp --root <scratch tracker>` with one `initialize` and one `tools/list` | answers `2025-11-25`; 24 tools |

All pass → you approve `publish-registry` in the Actions run. Any fail → don't approve; reject the deployment, and the fix ships as 0.2.1.

### Claude Step: Verify the Registry Listing

**[CLAUDE]**, after `publish-registry` is green. Read-only: query the registry's public API for `io.github.TheInfoMoose/nebulous` (the exact `/v0…/servers?search=` path is confirmed at check time) and confirm version `0.2.0`, the `.mcpb` identifier, the hash from run 4 and `status` `active`. Record the result as "Phase 25 live check" below this section. Then the phase ends (Mode 3b).
