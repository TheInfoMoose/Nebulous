## Problem Description
Source: docs/codebase-audit-report.md (AUD-09) — Validation: CONFIRMED (minor precision note).

Citations:
- scripts/phased-step.sh:42, 216, 219-220, 245-247, 269, 326, 548-549, 553-556, 596, 609
- scripts/validate-step.sh:78

Details:
1. Under `set -euo pipefail` (`42`), `code_stamp()` (`246`) runs `fingerprint | grep -v '^[^ ]*  \.tracker/' | sha256sum | cut -d' ' -f1`. When `fingerprint` emits zero non-`.tracker/` lines (e.g., `.tracker/`-only steps or no-op reruns), `grep -v` exits `1`, aborting `check` at line `609` before printing `PASS` (note: shell redirection still creates `${SNAP}/pass`, but `check` fails so normal flow never reaches `commit` / `BUG-0021` rerun handling).
2. `tracker_items()` (`269`) uses GNU-only `find -printf '%h
'`, failing on macOS/BSD.
3. `ALLOWED` extraction (`phased-step.sh:216`, `validate-step.sh:78`) preserves literal Markdown backticks, and `PRE_GATE`/`POST_GATE` extraction (`219-220`) reads only one line after the tag.

Suggested Direction:
Guard `grep -v` with `{ grep -v '^[^ ]*  \.tracker/' || true; }` (or `awk`), replace `find -printf` with portable `dirname` / Python `pathlib`, and strip Markdown backticks in `ALLOWED` extraction.

## Reproduction Steps
1. Inspect `scripts`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `scripts`
