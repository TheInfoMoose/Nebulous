## Problem Description
Source: docs/codebase-audit-report.md (AUD-28) — Validation: CONFIRMED.

Citations:
- crates/nebulous_cli/src/commands/mod.rs:662, 759, 1286, 2273
- crates/nebulous_cli/src/args.rs:200-203, 301-323, 419-446, 523-524, 720-733
- crates/nebulous_cli/tests/cli_integration.rs:2459-2467

Details:
`commands/mod.rs` holds 28 subcommand handlers in 3,146 lines with three `#[allow(clippy::too_many_arguments)]` suppressions. `Commands::Sync` accepts `--item`/`--path` while `GitCommand::Sync` omits both; `Commands::Stage` overlaps `PhaseCommand`; `ProjectCommand::Add` defines hidden `--use`/`--active` (`use_compat`) whose value is bound to `_` and ignored (`--no-use --use` silently leaves `--no-use` active).

Suggested Direction:
Split `commands/mod.rs` into domain submodules, reconcile `sync` / `git sync` and `stage` / `phase`, and either wire `use_compat` via clap `overrides_with("no_use")` or remove it.

## Reproduction Steps
1. Inspect `crates/nebulous_cli`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/nebulous_cli`
