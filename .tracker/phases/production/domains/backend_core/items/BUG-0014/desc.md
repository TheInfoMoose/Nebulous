## Problem Description
Source: docs/codebase-audit-report.md (AUD-07) — Validation: CONFIRMED.

Citations:
- crates/backend_core/src/git_sync/mod.rs:801-814, 1247, 1291-1326, 1840-1914, 1934-1953
- crates/backend_core/tests/git_safety.rs:1-426

Details:
1. `pull_repo_with_auth` (`1293`) suppresses `reconcile_indexes()` failures via `if storage.reconcile_indexes().is_ok()`, leaving stale/conflicted index manifests in the merge commit.
2. `git_sync_with_report` (`1943-1945`) pushes to remote without checking `!pull_outcome.duplicate_ids.is_empty()`.
3. `git_commit_documented` (`1840-1914`) advances `HEAD` before `write_item_detail`, `sync_parent_indexes_for_item`, and `initial_commit.amend`; mid-step failures leave `HEAD` moved, dirty `.tracker/` files, and `.git/index` out of sync. On success, `recorded_fix_sha` is the pre-amend SHA orphaned by `amend()`. (Note: a Git commit cannot contain its own post-amend SHA in its tree, so avoiding an orphaned SHA requires either a separate tracker commit or not embedding the self-SHA.)
4. `sync_user_index_paths` (`801-814`) writes zeroed stat metadata (`ctime`/`mtime`/`ino`/`uid`/`gid`), leaving updated entries stat-dirty to `git` CLI.

Suggested Direction:
Propagate `reconcile_indexes()` errors on merge resolution; abort `git_sync` before `git_push` when `duplicate_ids` is non-empty; handle `git_commit_documented` rollback/index-resync and orphaned SHA semantics cleanly; refresh real file stat metadata when syncing index entries.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
