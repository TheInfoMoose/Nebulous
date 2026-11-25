## Problem Description
`git_commit_documented` fails with `ValidationError("No changes to commit in workspace or .tracker/")` when the tree matches the parent (`crates/backend_core/src/git_sync/mod.rs:1823` and `:1832`). Orrery's `on_step_pass` hook (`neb:git_commit_documented`, `on_failure = Halt`) then halts a step that passed, whenever a rerun has nothing new to commit (Orrery BUG-0021, found in Orrery User Step LIVE-3, 2026-11-20).
Suggested fix: an optional argument (e.g. `allow_empty_ok: bool`, default false) that returns success with "nothing to commit" instead of the error. Callers that want strictness keep today's behavior.
Decision (Orrery Phase R planning, 2026-11-20): the fix lives in the hook tools, not in Orrery; the phased workflow's `phased-step.sh commit` already treats an already-committed step as success.

## Reproduction Steps
1. Orrery project with `orrery hook set pass neb:git_commit_documented --args '{"summary":"orrery step pass","dry_run":false}'`; `orrery run` passes step 1 and commits.
2. `orrery reset --step 1 && orrery run` → StepPassed, then the hook fails with "No changes to commit"; Orrery stops at AwaitingOperator, exit 3.

## Fix Log & Agent Notes
