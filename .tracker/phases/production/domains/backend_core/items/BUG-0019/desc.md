## Problem Description
Source: docs/codebase-audit-report.md (AUD-14) — Validation: CONFIRMED.

Citations:
- crates/backend_core/src/git_sync/mod.rs:591-605, 980-1006, 1067-1077, 1117-1118, 1155-1156

Details:
`url_allows_pat` checks only whether the callback URL starts with `https://` without pinning the host to the configured remote URL, and `FetchOptions`/`PushOptions` leave `libgit2`'s default `GIT_REMOTE_REDIRECT_INITIAL` enabled. A malicious or compromised remote returning a cross-host HTTPS redirect on the initial request can capture the user's Git PAT.

Suggested Direction:
Pin `pat_for_url` to the configured remote host and/or configure `follow_redirects` to disallow cross-host redirects.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
