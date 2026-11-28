## Problem Description
crates/backend_core/src/workflow/lifecycle.rs:604-624 — If git_commit_documented advances HEAD (lines 2060/2084) and then fails in sync_user_index_paths (line 2090), close_item_and_commit runs rollback_close_and_commit and restores .tracker/ on disk to open while HEAD already contains the closing commit(s).

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
