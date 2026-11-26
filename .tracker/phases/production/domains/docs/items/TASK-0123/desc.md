## Problem Description
STEP 44D spec gap: `[POST_GATE]` runs repo-wide `bash scripts/check-neutral.sh` (no file arguments), which fails exit 1 on pre-existing tracked files outside `ALLOWED_FILES`: `.tracker/phases/production/domains/inbox/items/TASK-0115/desc.md` (1 hit), `STEPPED_IMPLEMENTATION_PLAN.md` (19 hits), and `docs/codebase-audit-report.md` (1 hit). `bash scripts/check-neutral.sh docs/USAGE.md` exits 0. Location: `STEPPED_IMPLEMENTATION_PLAN.md` (STEP 44D `[POST_GATE]`) / `scripts/check-neutral.sh`.
Lapse: L1 marker grep (repo-wide check-neutral.sh gate fails on pre-existing files outside ALLOWED_FILES)
Kind: scope-lapse
Found during: STEP 44D (Neon)

## Reproduction Steps
n/a (spec gap)

## Fix Log & Agent Notes
