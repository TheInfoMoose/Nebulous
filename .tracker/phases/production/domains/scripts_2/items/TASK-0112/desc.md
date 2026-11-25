## Problem Description
Source: docs/codebase-audit-report.md (AUD-16) — Validation: CONFIRMED.

Citations:
- .github/workflows/ci.yml:13-66; .github/workflows/release.yml:16-122; deny.toml:1, 21-42
- scripts/bump-version.sh:79-115; scripts/install.sh:25-29, 107, 114, 129-136, 148-153, 160, 167
- mcpb/manifest.json:5; server.json:6, 14-15 (tracked by TASK-0103..TASK-0108)

Details:
1. `ci.yml` and `release.yml` use mutable action tags (`@v4`, `@v2`, `@stable`), `ci.yml` lacks `permissions: contents: read` and `--locked`, and both workflows run `cargo deny check advisories bans` without `licenses sources`.
2. `release.yml:78` runs `cp -r docs "${STAGING_DIR}/"`, bundling internal reports into release tarballs.
3. `bump-version.sh` mutates `Cargo.toml`/`Cargo.lock`/`CHANGELOG.md` before running `release-check.sh`.
4. `install.sh` breaks `--prefix /` (blank `PREFIX` on macOS/BSD, `BINDIR="//bin"` failing `PATH` check on Linux) and overrides install source with `NEBULOUS_BIN_DIR` even when `SKIP_BUILD=0` runs `cargo build`.
5. Note: `mcpb/manifest.json` and `server.json` are part of active Phase 25 work (`TASK-0103`..`TASK-0108`) and should be wired into `bump-version.sh`/`release.yml` there.

Suggested Direction:
SHA-pin workflow actions, enforce `licenses sources` in `cargo deny`, add `permissions` and `--locked` to `ci.yml`, package only user-facing docs in `release.yml`, run pre-checks before mutating in `bump-version.sh`, and fix `install.sh` `--prefix /` and `NEBULOUS_BIN_DIR` handling.

## Reproduction Steps
1. Inspect `scripts`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `scripts`
