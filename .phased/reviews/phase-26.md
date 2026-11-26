# Phase 26 review

Range: `3b928924561147a399ba535e2883198f3d3856c0..3bce38f1a40d2c2867613de4af3375b47de551c7` (`3b928924..3bce38f1`)

## Independent pass

**Findings:** None (0 findings).

### What was traced
- `Cargo.lock`: Verified only transitive patch bumps (`form_urlencoded` `1.2.2 -> 1.2.3`, `url` `2.5.8 -> 2.5.9`); no `Cargo.toml` changes or unexpected dependency additions.
- `crates/backend_core/tests/storage_integration.rs` (`1402-1481`): Verified replacement of process-global `std::env::set_var`/`remove_var("NEBULOUS_CONFIG_DIR")` and top-level global registry functions with explicit-path helpers (`register_project_in`, `load_global_registry_from`, `save_global_registry_to`, `set_active_project_in`, `remove_project_in`) against `crates/backend_core/src/models/registry.rs` (`377-479`). Confirmed all assertions are preserved, zero `set_var`/`remove_var` siblings remain under `crates/`, and `NEBULOUS_CONFIG_DIR` remains covered via isolated subprocess `Command::env` tests in `crates/nebulous_cli/tests/`.
- `scripts/phased-step.sh` (`216`, `246`, `269`) and `scripts/validate-step.sh` (`78`):
  - Verified `ALLOWED_FILES` `sed` regex strips optional backticks around paths while preserving plain paths, glob patterns, and trailing comments.
  - Verified `code_stamp` `{ grep -v '^[^ ]*  \.tracker/' || true; }` prevents `pipefail` exits when `fingerprint` is empty or contains only `.tracker/` entries.
  - Verified `tracker_items` replacement of GNU `find -printf '%h\n'` with POSIX `find ... | sed 's#/item\.toml$##; s#.*/##'` produces identical item IDs.
  - Verified `bash -n` passes on both scripts and audited all remaining `grep` calls in both scripts for `set -euo pipefail` hazards.
- `docs/USAGE.md` (`162-171`): Verified updated `neb git commit` and `neb sync` examples match the current `clap` definitions in `crates/nebulous_cli/src/args.rs` (`301-323`, `690-717`) and confirmed no stale `--all` references remain in user docs.
- Sensitive data check: No real hosts, IPs, emails, tokens, client names, or internal paths found in any changed code or usage doc file.

### What was not checked
- Did not execute `cargo test --workspace`, `cargo clippy`, or `cargo deny` during the blind pass.
- Did not run `scripts/phased-step.sh` or `scripts/validate-step.sh` end-to-end against a live scratch repository or on BSD/macOS `find`.
- `shellcheck` is not installed in this environment, so static shell lint beyond `bash -n` was not run.

<!-- SEALED: Independent pass locked -->

## Contract pass

### Step-by-step contract verification
| Step | Task | Outcome | Contract & Must-Still-Work Assessment |
| :--- | :--- | :--- | :--- |
| **STEP 44A** | `TASK-0117` (closed) | Committed (`ee1f2a6`) | **Met.** `Cargo.lock` updated `url` `2.5.8 -> 2.5.9` (+ transitive `form_urlencoded` `1.2.2 -> 1.2.3`); `deny.toml` and `Cargo.toml` untouched. Satisfies `BUG-0008`. |
| **STEP 44B** | `TASK-0118` (closed) | Committed (`97a5781`) | **Met.** `scripts/phased-step.sh` `code_stamp()` handles empty/`.tracker/`-only trees under `set -euo pipefail`, `tracker_items()` replaces GNU `find -printf` with portable `find \| sed`, and both `scripts/phased-step.sh` and `scripts/validate-step.sh` strip optional backticks in `ALLOWED_FILES`. Satisfies `BUG-0016`. |
| **STEP 44C** | `TASK-0119` (pending) | Deferred (`92e8c93`) | **Unmet (deferred).** Orrery masks `/workspace/.orrery` as an empty directory inside the execution container, making `.orrery/skills/repo-rules.md` unreadable/uneditable by Neon (`TASK-0122`). `TASK-0115` remains open. |
| **STEP 44D** | `TASK-0120` (closed) | Deferred (`5300229`), restaged (`5d0b313`), committed (`c20aa19`) | **Met (after restage).** `docs/USAGE.md` removes `--all` and uses `--path` / `.tracker/`-only examples. Satisfies `BUG-0017`. Restage scoped `check-neutral.sh` in `POST_GATE` to `docs/USAGE.md` because repo-wide `bash scripts/check-neutral.sh` fails on 3 pre-existing tracked files (`STEPPED_IMPLEMENTATION_PLAN.md`: 19 hits, `docs/codebase-audit-report.md`: 1 hit, `TASK-0115/desc.md`: 1 hit; tracked in `TASK-0123`). |
| **STEP 44E** | `TASK-0121` (closed) | Committed (`2db073f`) | **Met.** Replaced `std::env::set_var`/`remove_var("NEBULOUS_CONFIG_DIR")` with `*_in`/`*_from`/`*_to` registry helpers in `crates/backend_core/tests/storage_integration.rs`; all assertions preserved. Completes the test-hazard slice of `TASK-0116` (item remains open for remaining slices). |

### Unrequested changes, deviations, amendments, and origin mapping
- **Unrequested changes / validator deviations:** None. Every committed step stayed strictly within `ALLOWED_FILES`.
- **Amendments:** None (`.phased/amendments/` is empty).
- **Independent findings origin:** N/A (0 independent findings).
- **New tracker items filed:** None needed; both operational findings discovered during the phase are already filed as pending items (`TASK-0122` and `TASK-0123`).

## Handoff

- **Verdict:** PASS WITH DEFERRAL (4/5 steps completed cleanly, 0 code defects; 1 deferred step `STEP 44C`; 1 repo-wide CI neutrality blocker `TASK-0123`).
- **Pending items by priority:**
  - `high`: `TASK-0122` (`.orrery/` masked in Orrery container — blocks `STEP 44C`), `TASK-0123` (repo-wide `scripts/check-neutral.sh` fails on 3 tracked files: `STEPPED_IMPLEMENTATION_PLAN.md`, `docs/codebase-audit-report.md`, `TASK-0115/desc.md` — will fail Ubuntu CI).
  - `medium`: `TASK-0119` (deferred `STEP 44C`: `.orrery/skills/repo-rules.md` + `.gitignore`).
  - `low` (pre-existing parked): `TASK-0085`, `TASK-0086`, `TASK-0093`, `TASK-0094`.
- **Deferred steps to re-split:**
  - `STEP 44C` (`TASK-0119`): Split `.gitignore` into a normal step and move `.orrery/skills/repo-rules.md` to a `[USER]` / host-side step since Orrery masks `/workspace/.orrery`.
- **Advances items ready to close (pending user approval):**
  - `BUG-0008` (`STEP 44A`, `ee1f2a6`), `BUG-0016` (`STEP 44B`, `97a5781`), `BUG-0017` (`STEP 44D`, `c20aa19`).
  - Keep open: `TASK-0115` (`STEP 44C` deferred), `TASK-0116` (only `env::set_var` slice done).
- **User decisions needed:**
  1. Approve closing `BUG-0008`, `BUG-0016`, `BUG-0017`.
  2. Decide how to resolve `TASK-0123` before next push (exclude plan/audit/tracker prose in `check-neutral.sh` vs scrub the 21 hits) and whether to handle `.orrery/skills/repo-rules.md` (`TASK-0122`) host-side.
