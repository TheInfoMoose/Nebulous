## Problem Description
Imported from roadmap step `STEP 47A`: Fix git_credential_for_remote SSH fallback on non-HTTPS remotes and pin push_repo_with_auth to remote.pushurl() (BUG-0035, BUG-0036; closes BUG-0019)
- Step ID: `STEP 47A`
- Command: `/goal`
- Roadmap order: 1
- Target files:
  - `crates/backend_core/src/git_sync/mod.rs`
  - `crates/backend_core/tests/credential_hygiene.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
