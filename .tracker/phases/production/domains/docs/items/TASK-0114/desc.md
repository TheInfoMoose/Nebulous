## Problem Description
Source: docs/codebase-audit-report.md (AUD-29) — Validation: CONFIRMED.

Citations:
- README.md:27, 29, 138, 141, 145
- docs/USAGE.md:10-208, 234, 241, 246
- docs/ARCHITECTURE.md:13-22, 49-79, 94-118
- CHANGELOG.md:6, 18
- crates/nebulous_cli/src/args.rs:28-391, 138; tui/state.rs:484-495; tui/render.rs:530-555

Details:
1. `README.md:138` says TUI `1/2/3` switches active stage, contradicting `USAGE.md:241` and `tui/state.rs:484-495` (viewed stage only).
2. `README.md` omits `t`/`r`; `USAGE.md:234` omits `New Task (t)`; `tui/render.rs:530-555` omits `[r] Reload` in footer legend.
3. `USAGE.md` omits 8+ core commands (`list`, `status`, `reopen`, `timer`, `log`, `metrics`, `phase`, `scope`, `sync-index`), omits `neb task` from scope inference, and `args.rs:138` omits `task`/`chore` in `List --type` help.
4. `ARCHITECTURE.md` omits `planning`, `clipboard`, `storage::compat`, `TASK-XXXX`, `id_scheme = "hash"`, `.tracker/.lock`, `.tracker/.gitignore`, `.timer.toml.bak`; `README.md` omits `TASK-XXXX` and `hash` IDs; `CHANGELOG.md` `[Unreleased]` is empty despite post-0.1.0 commits and contradicts `git_fetch` `dry_run`.

Suggested Direction:
Reconcile `README.md`, `docs/USAGE.md`, `docs/ARCHITECTURE.md`, `args.rs` help text, TUI footer legend, and `CHANGELOG.md` against current code.

## Reproduction Steps
1. Inspect `docs`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `docs`
