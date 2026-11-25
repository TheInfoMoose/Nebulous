## Problem Description
Source: docs/codebase-audit-report.md (AUD-10) — Validation: CONFIRMED.

Citations:
- docs/USAGE.md:166, 169
- crates/nebulous_cli/src/args.rs:301-323, 687-717
- crates/nebulous_cli/src/lib.rs:280; crates/nebulous_cli/tests/cli_integration.rs:1005-1011

Details:
Both copy-paste examples in `docs/USAGE.md` (`166, 169`) pass `--all` to `neb git commit` and `neb sync`. `--all` was intentionally removed and replaced with `--path <FILES>` (or `.tracker/`-only defaults), and tests explicitly assert `--all` fails to parse.

Suggested Direction:
Update `docs/USAGE.md:165-170` to document `--path <FILES>` and `.tracker/`-only default commit behavior.

## Reproduction Steps
1. Inspect `docs`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `docs`
