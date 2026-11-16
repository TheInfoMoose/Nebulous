#!/usr/bin/env bash
# ==============================================================================
# validate-step.sh — post-hoc check of one Argon step executed by another agent.
# Generic copy shipped with the `hybrid-workflow` Claude skill.
#
#   validate-step.sh start <STEP_ID> [--plan <path>]             snapshot before hand-off
#   validate-step.sh check <STEP_ID> [--plan <path>] [--no-gate] verify afterwards
#
# Run it from anywhere inside the target repo (or from the repo's scripts/ copy).
# The plan file is <path> relative to the repo root; default comes from
# $HYBRID_PLAN, else STEPPED_IMPLEMENTATION_PLAN.md.
#
# `check` fails (exit 1) when, compared with the `start` snapshot:
#   - HEAD moved (the executor committed),
#   - the plan file changed (hashed, so it works tracked or gitignored),
#   - any file changed, appeared or vanished outside the step's ALLOWED_FILES,
#   - any changed path matches a $HYBRID_FORBIDDEN glob (colon-separated),
#     even if ALLOWED_FILES lists it,
#   - the step's POST_GATE exits non-zero.
# Files already dirty at `start` count only if their content changed afterwards.
# Snapshots live in .git/validate-step/ (never tracked). Exit 2 = usage/setup error.
#
# ALLOWED_FILES entries: the first token of each "  - <path>" line. Shell glob
# patterns (*) are honoured; brace lists are not. If the ALLOWED_FILES section
# mentions Cargo.lock, Cargo.lock is allowed too.
# ==============================================================================
set -euo pipefail

usage() {
    echo "usage: $0 start|check <STEP_ID> [--plan <path>] [--no-gate]" >&2
    exit 2
}

[[ $# -ge 2 ]] || usage
MODE="$1"
STEP_ID="$2"
shift 2
RUN_GATE=1
PLAN="${HYBRID_PLAN:-STEPPED_IMPLEMENTATION_PLAN.md}"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-gate) RUN_GATE=0 ;;
        --plan)
            [[ $# -ge 2 ]] || usage
            PLAN="$2"
            shift
            ;;
        *) usage ;;
    esac
    shift
done
[[ "$MODE" == "start" || "$MODE" == "check" ]] || usage
[[ "$STEP_ID" =~ ^[0-9]+[A-Z]*$ ]] || { echo "STEP_ID must look like 13A, 25 or 7AB" >&2; exit 2; }

# Prefer the repo we are standing in; fall back to the repo holding this script.
ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" ||
    ROOT="$(git -C "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)" rev-parse --show-toplevel 2>/dev/null)" ||
    { echo "not inside a git repo (run from the target repo)" >&2; exit 2; }
cd "$ROOT"
[[ -f "$PLAN" ]] || { echo "missing plan file $PLAN (repo root: $ROOT)" >&2; exit 2; }
IFS=':' read -r -a FORBIDDEN <<<"${HYBRID_FORBIDDEN:-}"
STATE_DIR="$(git rev-parse --git-dir)/validate-step"
SNAP="${STATE_DIR}/${STEP_ID}"

# --- Argon block extraction ---------------------------------------------------
BLOCK="$(awk -v id="### STEP ${STEP_ID}:" '
    index($0, id) == 1 { f = 1 }
    f { print }
    f && /^\[HALT_INVARIANT\]/ { exit }
' "$PLAN")"
[[ -n "$BLOCK" ]] || { echo "no '### STEP ${STEP_ID}:' block in $PLAN" >&2; exit 2; }

ALLOWED_SECTION="$(awk '
    /^ALLOWED_FILES:/ { f = 1; next }
    f && /^(READ_ONLY_REFS|FORBIDDEN_SCOPE):|^\[/ { exit }
    f { print }
' <<<"$BLOCK")"
mapfile -t ALLOWED < <(sed -nE 's/^[[:space:]]*-[[:space:]]+([^[:space:]]+).*/\1/p' <<<"$ALLOWED_SECTION")
[[ ${#ALLOWED[@]} -gt 0 ]] || { echo "STEP ${STEP_ID} has no ALLOWED_FILES entries" >&2; exit 2; }
grep -q 'Cargo\.lock' <<<"$ALLOWED_SECTION" && ALLOWED+=("Cargo.lock")
POST_GATE="$(awk '/^\[POST_GATE\]/ { getline; print; exit }' <<<"$BLOCK")"

# --- Working-tree fingerprint: "<sha256|DELETED>  <path>" per dirty path -------
fingerprint() {
    git -c core.quotePath=false status --porcelain=v1 --no-renames --untracked-files=all |
        awk 'length($0) > 3 { print substr($0, 4) }' |
        while IFS= read -r p; do
            [[ -n "$p" ]] || continue
            if [[ -f "$p" ]]; then
                printf '%s  %s\n' "$(sha256sum -- "$p" | cut -d' ' -f1)" "$p"
            else
                printf 'DELETED  %s\n' "$p"
            fi
        done | sort -u
}

if [[ "$MODE" == "start" ]]; then
    mkdir -p "$SNAP"
    git rev-parse HEAD >"${SNAP}/head"
    sha256sum -- "$PLAN" | cut -d' ' -f1 >"${SNAP}/plan"
    fingerprint >"${SNAP}/tree"
    echo "STEP ${STEP_ID}: snapshot taken (HEAD $(cut -c1-12 "${SNAP}/head"), $(wc -l <"${SNAP}/tree") pre-existing dirty paths)."
    echo "Allowed files:"
    printf '  %s\n' "${ALLOWED[@]}"
    exit 0
fi

[[ -d "$SNAP" ]] || { echo "no snapshot for STEP ${STEP_ID}; run: $0 start ${STEP_ID}" >&2; exit 2; }

FAIL=0
fail() { echo "FAIL: $*"; FAIL=1; }

# 1. No commits.
if [[ "$(git rev-parse HEAD)" != "$(cat "${SNAP}/head")" ]]; then
    fail "HEAD moved since start ($(cut -c1-12 "${SNAP}/head") -> $(git rev-parse --short=12 HEAD)); the executor must not commit"
fi

# 2. Plan file untouched.
if [[ "$(sha256sum -- "$PLAN" | cut -d' ' -f1)" != "$(cat "${SNAP}/plan")" ]]; then
    fail "$PLAN changed since start; the executor must not edit it"
fi

# 3. Scope: paths whose fingerprint differs from the snapshot.
is_allowed() {
    local p="$1" a
    for a in "${ALLOWED[@]}"; do
        # shellcheck disable=SC2053  # $a is intentionally a glob pattern
        [[ "$p" == $a ]] && return 0
    done
    return 1
}
mapfile -t CHANGED < <(
    comm -3 <(sort -u "${SNAP}/tree") <(fingerprint) |
        sed -E 's/^\t//; s/^[^ ]+  //' | sort -u
)
is_forbidden() {
    local p="$1" f
    for f in "${FORBIDDEN[@]}"; do
        [[ -n "$f" ]] || continue
        # shellcheck disable=SC2053  # $f is intentionally a glob pattern
        [[ "$p" == $f ]] && return 0
    done
    return 1
}
OUT_OF_SCOPE=()
HIT_FORBIDDEN=()
for p in "${CHANGED[@]}"; do
    [[ -n "$p" ]] || continue
    [[ "$p" == "$PLAN" ]] && continue
    is_forbidden "$p" && HIT_FORBIDDEN+=("$p")
    is_allowed "$p" || OUT_OF_SCOPE+=("$p")
done
if [[ ${#OUT_OF_SCOPE[@]} -gt 0 ]]; then
    fail "changed outside ALLOWED_FILES:"
    printf '        %s\n' "${OUT_OF_SCOPE[@]}"
fi
if [[ ${#HIT_FORBIDDEN[@]} -gt 0 ]]; then
    fail "changed a HYBRID_FORBIDDEN path:"
    printf '        %s\n' "${HIT_FORBIDDEN[@]}"
fi
echo "Changed since start (${#CHANGED[@]}):"
[[ ${#CHANGED[@]} -gt 0 && -n "${CHANGED[0]}" ]] && printf '  %s\n' "${CHANGED[@]}"

# 4. POST_GATE.
if [[ "$RUN_GATE" -eq 1 ]]; then
    if [[ -z "$POST_GATE" ]]; then
        fail "STEP ${STEP_ID} has no POST_GATE line"
    else
        LOG="${SNAP}/post_gate.log"
        echo "Running POST_GATE (log: ${LOG})"
        if bash -c "$POST_GATE" >"$LOG" 2>&1; then
            echo "POST_GATE: exit 0"
        else
            rc=$?
            fail "POST_GATE exited ${rc}; last 30 lines:"
            tail -n 30 "$LOG" | sed 's/^/        /'
        fi
    fi
else
    echo "POST_GATE: skipped (--no-gate)"
fi

if [[ "$FAIL" -eq 0 ]]; then
    echo "STEP ${STEP_ID}: PASS"
else
    echo "STEP ${STEP_ID}: FAIL"
fi
exit "$FAIL"
