## Problem Description
Source: docs/codebase-audit-report.md (AUD-11) — Validation: PARTIALLY CONFIRMED (OS/ISO leftovers confirmed; Orrery/plan_import are intentional CHANGELOG features).

Citations:
- CHANGELOG.md:14, 29
- crates/backend_core/src/models/component.rs:19-31, 180-613; models/project.rs:24-25
- crates/backend_core/src/mcp/mod.rs:702, 745-812, 754, 1718-2149, 2758
- crates/backend_core/src/ingestion/mod.rs:763-869
- crates/backend_core/src/watcher/mod.rs:33 (`libcosmic`); crates/backend_core/src/error.rs:90 (`neb`)
- crates/nebulous_cli/src/args.rs:567, 593, 602
- README.md:7, 34; docs/USAGE.md:6; scripts/{phased-step.sh,neon-dispatch,validate-step.sh}

Details:
`CHANGELOG.md:29` states the release engine and `neb release` were removed, yet OS/ISO rolling-release artifacts remain active across `component.rs` (`OsImage`, `KernelModule`), `mcp/mod.rs` (Suite 3 rolling-update/pinning/rollback tools, `"orbit_image"`, `"apt_sid"`, `"kernel_custom"`), `ingestion/mod.rs`, `args.rs` (`scripts/build_iso.sh`), `README.md:7, 34` (`rolling-release OS builders`), and `watcher/mod.rs:33` (`libcosmic`). Note: Orrery context (`mcp/mod.rs:2635-2679`) and `plan_import` are intentional features announced in `CHANGELOG.md:14`, and `docs/USAGE.md:6` mentions Orrery/CI rather than OS builders.

Suggested Direction:
Remove or feature-gate the leftover OS/ISO release-engineering subsystem, remove `libcosmic`/`orbit_image`/`apt_sid` leftovers, decouple `error.rs:90` from the `neb` binary name, and separate repo-dev agent scripts from shipped `scripts/`.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
