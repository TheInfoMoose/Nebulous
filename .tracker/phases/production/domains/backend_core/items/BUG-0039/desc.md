## Problem Description
crates/backend_core/src/git_sync/mod.rs:1409-1455 — pull_repo_with_auth writes the merged tree to the workdir via repo.checkout_tree before calling storage.reconcile_indexes()?. If reconcile_indexes fails, HEAD stays at local_commit but the workdir remains mutated with remote files and unreconciled indexes, breaking subsequent pulls until manually cleaned. Roll back workdir checkout on reconcile failure and assert workdir cleanliness in pull_propagates_merge_reconcile_error_without_advancing_head.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
