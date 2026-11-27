## Problem Description
Phase 28 review finding (new defect, origin: plan, STEP 46D). In crates/backend_core/src/git_sync/mod.rs:1153-1158, push_repo_with_auth passes remote.url() to build_auth_callbacks_for_remote instead of remote.pushurl().or_else(|| remote.url()). Remotes that configure a distinct HTTPS pushurl fail push authentication with a PAT host-mismatch error.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
