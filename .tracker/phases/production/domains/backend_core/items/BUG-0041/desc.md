## Problem Description
crates/backend_core/src/git_sync/mod.rs:2058-2078 — When auto_append_to_desc is enabled, git_commit_documented moves the branch ref with repo.reference(symbolic, doc_commit_oid, true, ...) (force=true) without verifying HEAD still equals parent_commit, risking silent drop of concurrent commits. Also doc_result failure (lines 2070-2077) discards rollback errors via let _ =.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
