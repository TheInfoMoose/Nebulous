## Problem Description
Imported from roadmap step `STEP 46D`: Pin libgit2 PAT credentials and disable cross-host redirects (RemoteRedirect::None) to the configured HTTPS remote host (BUG-0019 / AUD-14)
- Step ID: `STEP 46D`
- Command: `/boost`
- Roadmap order: 4
- Target files:
  - `crates/backend_core/src/git_sync/mod.rs`
  - `crates/backend_core/tests/credential_hygiene.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
