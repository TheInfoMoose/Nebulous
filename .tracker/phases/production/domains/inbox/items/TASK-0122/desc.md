## Problem Description
STEP 44C spec gap: STEP 44C requires editing `.orrery/skills/repo-rules.md` (and reading `.orrery/project.toml:1-40`), but inside the Orrery execution container `/workspace/.orrery` is masked as an empty directory (all 11 tracked files under `.orrery/` appear deleted in `git status`), so `.orrery/skills/repo-rules.md` cannot be read or modified from inside the sandboxed container. Location: `.orrery/skills/repo-rules.md`.
Lapse: L4 other
Kind: scope-lapse
Found during: STEP 44C (Neon)

## Reproduction Steps
n/a (spec gap)

## Fix Log & Agent Notes
