## Problem Description
Source: docs/codebase-audit-report.md (AUD-01) — Validation: CONFIRMED.

Citations:
- Cargo.lock:3280-3282 (url 2.5.8 via git2 0.21.0)
- crates/backend_core/Cargo.toml:14; crates/nebulous_cli/Cargo.toml:29
- deny.toml:2, 8-13; CHANGELOG.md:34
- .github/workflows/ci.yml:48-52; .github/workflows/release.yml:49-52

Details:
`cargo deny --offline check advisories bans licenses sources` fails (exit 1) with `RUSTSEC-2026-0592` on `url 2.5.8` (stack overflow in `Url::origin()` on nested `blob:` URLs, fixed in `>= 2.5.9`). Although `Url::origin()` is unreachable through `git2 0.21.0` and `backend_core`, `deny.toml` enforces `ignore = []`, blocking both CI and release pipelines.

Suggested Direction:
Run `cargo update -p url` to advance `Cargo.lock` to `url >= 2.5.9` and re-verify `cargo deny check advisories bans licenses sources`.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
