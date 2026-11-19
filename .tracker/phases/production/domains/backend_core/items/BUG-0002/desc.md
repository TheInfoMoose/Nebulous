## Problem Description
Audit A6 (Phase 21): MCP `git_fetch` in dry run errors ("remote 'origin' does not exist") when the repo has no `origin`, while `git_pull`/`git_push`/`git_sync` dry runs return status — a hook-failure trap in a repo with no remote — make the `git_fetch` dry run report status like the others.
Migrated from the plan file's Parked list (2026-11-18).

## Reproduction Steps
1. In a repo with no `origin` remote, call MCP `git_fetch` with `dry_run: true`.
2. Observe: isError "remote 'origin' does not exist".

## Fix Log & Agent Notes
