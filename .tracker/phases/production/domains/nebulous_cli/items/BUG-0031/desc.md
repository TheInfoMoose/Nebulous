## Problem Description
Source: docs/codebase-audit-report.md (AUD-31) — Validation: CONFIRMED (minor nuance on env_pat zeroization path).

Citations:
- crates/nebulous_cli/src/commands/mod.rs:985-1006, 1458-1580, 1485, 1527, 1775, 1785-1808, 2617-2621
- crates/nebulous_cli/src/main.rs:17-21
- crates/nebulous_cli/Cargo.toml:18

Details:
1. `read_pat_input` leaves `env_pat` un-zeroized when `pat_stdin == true` and allocates a trimmed copy when `pat_stdin == false`.
2. `warn_if_capped` and `note_quarantined_timer` print prose to `stderr` under `--json`, and `note_quarantined_timer` is omitted from `TimerCommand::Start`/`Stop`.
3. `TimerCommand::Stop` (`1485`) and `TimerCommand::Status` (`1527`) compare `item_id` case-sensitively.
4. `ComponentCommand::Init --type` (`2617-2621`) silently accepts arbitrary invalid types.
5. `nebulous_cli/Cargo.toml:18` enables `arboard`'s `wayland-data-control` unconditionally on non-Linux targets; `main.rs:20` formats unescaped `{err}` in its JSON fallback.

Suggested Direction:
Zeroize `env_pat` in place, normalize `item_id` case in `TimerCommand::Stop`/`Status`, reject invalid `ComponentType` values, gate `wayland-data-control` under Linux target dependencies, and structure/suppress warnings under `--json`.

## Reproduction Steps
1. Inspect `crates/nebulous_cli`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/nebulous_cli`
