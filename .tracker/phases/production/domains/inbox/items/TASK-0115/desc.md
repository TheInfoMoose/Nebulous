## Problem Description
Source: docs/codebase-audit-report.md (AUD-30) — Validation: CONFIRMED.

Citations:
- STEPPED_IMPLEMENTATION_PLAN.md:1-6, 11, 17, 23-31, 240-315, 3249-5040, 10043
- .orrery/project.toml:2, 9, 24; .orrery/skills/repo-rules.md:1-3; .phased/plan-header.md:3; .gitignore:19

Details:
`STEPPED_IMPLEMENTATION_PLAN.md` is 994 KB (10,386 lines) with a stale Phase 4 title, ~1,790 lines of withdrawn Phase 9 `neb release` specs, and external `/srv/devshare/NovOS` paths. Tracked `.orrery/*` configures `hybrid-workflow` while untracked `.phased/plan-header.md` and `scripts/phased-step.sh` configure `phased-workflow`. `.orrery/skills/repo-rules.md` references `/workspace/` and `"release tracker"`. `.gitignore:19` ignores `STEPPED_IMPLEMENTATION_PLAN.md` while `.orrery/project.toml:2` treats it as canonical, and `STEP 43C` `PRE_GATE` (`10043`) fails because `mcpb/manifest.json` and `server.json` are already untracked on disk (see `TASK-0103`).

Suggested Direction:
Archive completed/withdrawn phases out of `STEPPED_IMPLEMENTATION_PLAN.md`, align `.orrery/`, `.phased/`, and `.gitignore` on a single workflow, and clean up stale references in `.orrery/skills/repo-rules.md`.

## Reproduction Steps
1. Inspect `.tracker`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `.tracker`
