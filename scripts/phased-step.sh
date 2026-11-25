#!/usr/bin/env bash
# ==============================================================================
# phased-step.sh — step gate for the phased workflow (Hydrogen orchestrates,
# Neon codes, Claude reviews per phase). Generic copy shipped with the
# `phased-workflow` Claude skill. A fork of hybrid-workflow's validate-step.sh
# (same step-block parser and snapshot); that script is left untouched.
#
#   phased-step.sh pregate   <ID>                 run the step's PRE_GATE (exit = its exit)
#   phased-step.sh start     <ID>                 snapshot before Neon starts
#   phased-step.sh check     <ID> [--no-gate]     verify scope, tracker, amendment, POST_GATE
#   phased-step.sh commit    <ID>                 commit a step whose last check passed
#   phased-step.sh defer     <ID> --reason "<r>"  save attempt, restore tree, park the task
#   phased-step.sh status                         phase log + early-stop verdict (exit 3 = stop)
#   phased-step.sh new-phase                      rotate the phase log, write .orrery/phase.toml (Claude, after approval)
#
# Options for every mode: --plan <path> (default $PHASED_PLAN, else
# STEPPED_IMPLEMENTATION_PLAN.md). The plan header must say "**Workflow:** phased"
# and carry "**Base gate:** `...`" and "**Forbidden paths:** `...`" lines.
#
# `check` fails (exit 1) when, compared with the `start` snapshot:
#   - HEAD moved, or the plan file changed;
#   - a path changed outside ALLOWED_FILES (+ Cargo.lock if listed, + amendment
#     extra_files, + .phased/amendments/<ID>.toml), or hit a forbidden glob;
#   - .tracker/ changed other than new items with status "pending" and index
#     files (project.toml, phase.toml, scope.toml, .timer.toml);
#   - the amendment breaks a limit (references/amendments.md) or changed after
#     its first use;
#   - POST_GATE (or the amendment's post_gate) exits non-zero.
# On PASS it stamps the tree; `commit` refuses unless the tree still matches.
#
# `defer` restores only the step's own paths (ALLOWED_FILES, Cargo.lock if
# listed, the amendment's extra_files and the amendment file). Anything else
# that changed since `start` is listed as "not restored" and left as it is.
#
# `new-phase` also writes .orrery/phase.toml (when .orrery/ exists) from the
# plan's "### Up Next" rows, in table order: `**STEP <ID>**` rows become step
# entries (with the open neb task whose source_ref is "STEP <ID>" as `item`),
# `**[USER] <ID>**` rows become user entries titled from the Scope column. No
# rows removes the file, so Orrery falls back to the whole plan.
# State lives in .git/phased/ (never tracked). Exit 2 = usage/setup error.
# ==============================================================================
set -euo pipefail

usage() {
    sed -n '8,16p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' >&2
    exit 2
}

[[ $# -ge 1 ]] || usage
MODE="$1"
shift
STEP_ID=""
case "$MODE" in
    pregate | start | check | commit | defer)
        [[ $# -ge 1 ]] || usage
        STEP_ID="$1"
        shift
        [[ "$STEP_ID" =~ ^[0-9]+[A-Z]*$ ]] || { echo "STEP_ID must look like 13A, 25 or 7AB" >&2; exit 2; }
        ;;
    status | new-phase) ;;
    *) usage ;;
esac
RUN_GATE=1
REASON=""
PLAN="${PHASED_PLAN:-STEPPED_IMPLEMENTATION_PLAN.md}"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-gate) RUN_GATE=0 ;;
        --plan)
            [[ $# -ge 2 ]] || usage
            PLAN="$2"
            shift
            ;;
        --reason)
            [[ $# -ge 2 ]] || usage
            REASON="$2"
            shift
            ;;
        *) usage ;;
    esac
    shift
done

ROOT="$(git rev-parse --show-toplevel 2>/dev/null)" ||
    { echo "not inside a git repo (run from the target repo)" >&2; exit 2; }
cd "$ROOT"
[[ -f "$PLAN" ]] || { echo "missing plan file $PLAN (repo root: $ROOT)" >&2; exit 2; }
grep -qE '^\*\*Workflow:\*\*[[:space:]]+phased\b' "$PLAN" ||
    { echo "$PLAN has no '**Workflow:** phased' header line; this repo isn't on the phased workflow" >&2; exit 2; }

STATE="$(git rev-parse --git-dir)/phased"
LOG="${STATE}/log"
CURRENT="${STATE}/current"
mkdir -p "$STATE"

header_value() { # first `...` on the "**<name>:**" header line
    sed -nE "s/^\*\*$1:\*\*[[:space:]]+\`([^\`]*)\`.*/\1/p" "$PLAN" | head -n1
}
BASE_GATE="$(header_value 'Base gate')"
FORBIDDEN_RAW="$(header_value 'Forbidden paths')"
[[ "$FORBIDDEN_RAW" == "none" || "$FORBIDDEN_RAW" == \<* ]] && FORBIDDEN_RAW=""
[[ -n "${PHASED_FORBIDDEN:-}" ]] && FORBIDDEN_RAW="${FORBIDDEN_RAW:+${FORBIDDEN_RAW}:}${PHASED_FORBIDDEN}"
IFS=':' read -r -a FORBIDDEN <<<"$FORBIDDEN_RAW"

log_line() { printf '%s\t%s\t%s\t%s\t%s\n' "$(date -u +%FT%TZ)" "$1" "$2" "$3" "$4" >>"$LOG"; }

# --- status / new-phase (no step) -------------------------------------------------
if [[ "$MODE" == "status" ]]; then
    STEPS="$(grep -cE '^\|[[:space:]]*[0-9]+[[:space:]]*\|[[:space:]]*\*\*STEP [0-9]+[A-Z]*\*\*' "$PLAN" || true)"
    DEFERRED=0
    [[ -f "$LOG" ]] && DEFERRED="$(awk -F'\t' '$3 == "deferred"' "$LOG" | wc -l)"
    LIMIT=$(((STEPS + 2) / 3))
    ((LIMIT < 2)) && LIMIT=2
    STOP=0
    ((DEFERRED > LIMIT)) && STOP=1
    echo "Phase log (${LOG}):"
    if [[ -s "$LOG" ]]; then column -t -s $'\t' "$LOG" | sed 's/^/  /'; else echo "  (empty)"; fi
    [[ -f "$CURRENT" ]] && echo "In flight: STEP $(cat "$CURRENT")"
    echo "DEFERRED=${DEFERRED} STEPS=${STEPS} LIMIT=${LIMIT} STOP=${STOP}"
    [[ "$STOP" -eq 1 ]] && { echo "Early stop: deferrals exceed the limit; hand the phase to Claude review."; exit 3; }
    exit 0
fi
if [[ "$MODE" == "new-phase" ]]; then
    [[ -f "$CURRENT" ]] && { echo "STEP $(cat "$CURRENT") is still in flight; commit or defer it first" >&2; exit 2; }
    if [[ -s "$LOG" ]]; then
        mv "$LOG" "${LOG}.$(date -u +%Y%m%dT%H%M%SZ)"
        echo "Phase log rotated."
    else
        echo "Phase log already empty."
    fi
    [[ -d .orrery ]] || exit 0
    python3 -I - "$PLAN" <<'PY'
import pathlib, re, sys, tomllib

plan = pathlib.Path(sys.argv[1]).read_text()
out = pathlib.Path(".orrery/phase.toml")

lines, in_up, rows = plan.splitlines(), False, []
for line in lines:
    if line.startswith("### "):
        if in_up:
            break
        in_up = line.strip() == "### Up Next"
        continue
    if not in_up or not line.startswith("|"):
        continue
    cells = [c.strip() for c in line.strip().strip("|").split("|")]
    if len(cells) < 2 or not re.fullmatch(r"[0-9]+", cells[0]):
        continue
    m = re.fullmatch(r"\*\*STEP ([0-9]+[A-Z]*)\*\*", cells[1])
    if m:
        rows.append(("step", m.group(1), ""))
        continue
    m = re.fullmatch(r"\*\*\[USER\] ([A-Za-z0-9_-]+)\*\*", cells[1])
    if m:
        title = cells[3].replace("`", "").replace("**", "") if len(cells) > 3 else ""
        rows.append(("user", m.group(1), title))

if not rows:
    if out.exists():
        out.unlink()
        print("Up Next is empty: removed .orrery/phase.toml (Orrery runs the whole plan).")
    else:
        print("Up Next is empty: no .orrery/phase.toml written.")
    sys.exit(0)

tasks = {}
tracker = pathlib.Path(".tracker")
if tracker.is_dir():
    for p in sorted(tracker.rglob("item.toml")):
        try:
            item = tomllib.loads(p.read_text()).get("item", {})
        except Exception:
            continue
        ref = item.get("source_ref", "")
        if ref.startswith("STEP ") and item.get("status") != "closed":
            tasks.setdefault(ref[5:], item.get("id", ""))

def q(v):
    return '"' + v.replace("\\", "\\\\").replace('"', '\\"') + '"'

m = re.search(r"^\*\*Phase goal \(Phase ([A-Za-z0-9]+)\b", plan, re.M)
body = ["# Written by phased-step.sh new-phase from the plan's Up Next table.",
        "# Regenerated every phase; edits here are lost."]
if m:
    body.append(f"phase = {q(m.group(1))}")
for kind, ident, title in rows:
    body += ["", "[[queue]]", f"{kind} = {q(ident)}"]
    if kind == "step" and tasks.get(ident):
        body.append(f"item = {q(tasks[ident])}")
    if kind == "user" and title:
        body.append(f"title = {q(title)}")
out.write_text("\n".join(body) + "\n")
print(f"Wrote .orrery/phase.toml: {len(rows)} queue entr{'y' if len(rows) == 1 else 'ies'}"
      f" ({', '.join(i for _, i, _ in rows)}). Commit it with the plan.")
PY
    exit 0
fi

SNAP="${STATE}/steps/${STEP_ID}"
AMEND=".phased/amendments/${STEP_ID}.toml"

# --- Step-block extraction (same rules as validate-step.sh) ------------------------
BLOCK="$(awk -v id="### STEP ${STEP_ID}:" '
    index($0, id) == 1 { f = 1 }
    f { print }
    f && /^\[HALT_INVARIANT\]/ { exit }
' "$PLAN")"
[[ -n "$BLOCK" ]] || { echo "no '### STEP ${STEP_ID}:' block in $PLAN" >&2; exit 2; }
TITLE="$(head -n1 <<<"$BLOCK" | sed -E "s/^### STEP ${STEP_ID}:[[:space:]]*//")"
ALLOWED_SECTION="$(awk '
    /^ALLOWED_FILES:/ { f = 1; next }
    f && /^(READ_ONLY_REFS|FORBIDDEN_SCOPE):|^\[/ { exit }
    f { print }
' <<<"$BLOCK")"
mapfile -t ALLOWED < <(sed -nE 's/^[[:space:]]*-[[:space:]]+`*([^[:space:]`]+)`*.*/\1/p' <<<"$ALLOWED_SECTION")
[[ ${#ALLOWED[@]} -gt 0 ]] || { echo "STEP ${STEP_ID} has no ALLOWED_FILES entries" >&2; exit 2; }
grep -q 'Cargo\.lock' <<<"$ALLOWED_SECTION" && ALLOWED+=("Cargo.lock")
PRE_GATE="$(awk '/^\[PRE_GATE\]/ { getline; print; exit }' <<<"$BLOCK")"
POST_GATE="$(awk '/^\[POST_GATE\]/ { getline; print; exit }' <<<"$BLOCK")"

# --- helpers -----------------------------------------------------------------------
fingerprint() { # "<sha256|DELETED>  <path>" per dirty path
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
matches_any() { # <path> <glob>...
    local p="$1" g
    shift
    for g in "$@"; do
        [[ -n "$g" ]] || continue
        # shellcheck disable=SC2053  # $g is intentionally a glob pattern
        [[ "$p" == $g ]] && return 0
    done
    return 1
}
code_stamp() { # tree fingerprint minus .tracker/ (Hydrogen may file findings after check)
    fingerprint | { grep -v '^[^ ]*  \.tracker/' || true; } | sha256sum | cut -d' ' -f1
}
changed_since_snapshot() {
    comm -3 <(sort -u "${SNAP}/tree") <(fingerprint) | sed -E 's/^\t//; s/^[^ ]+  //' | sort -u
}
# Prints "<TASK-ID>\t<scope>" for the item whose source_ref is "STEP <ID>".
find_task() {
    [[ -d .tracker ]] || return 0
    python3 -I - "$STEP_ID" <<'PY'
import pathlib, sys, tomllib
want = f"STEP {sys.argv[1]}"
for p in sorted(pathlib.Path(".tracker").rglob("item.toml")):
    try:
        item = tomllib.loads(p.read_text()).get("item", {})
    except Exception:
        continue
    if item.get("source_ref") == want and item.get("status") != "closed":
        print(f'{item.get("id", "")}\t{item.get("scope_ref", "")}')
        break
PY
}
tracker_items() { # item IDs that exist now
    [[ -d .tracker ]] || return 0
    find .tracker -path '*/items/*/item.toml' | sed 's#/item\.toml$##; s#.*/##' | sort -u
}

# Validates the amendment; prints POST_GATE\t…, EXTRA\t<path>\t<kind>, ERR\t… lines.
read_amendment() {
    python3 -I - "$AMEND" "$STEP_ID" "$POST_GATE" "$BASE_GATE" "$PLAN" "$FORBIDDEN_RAW" <<'PY'
import fnmatch, re, sys, tomllib
path, step, orig_gate, base_gate, plan, forbidden = sys.argv[1:7]
forbidden = [g for g in forbidden.split(":") if g]
def err(m): print(f"ERR\t{m}")
try:
    a = tomllib.loads(open(path).read())
except Exception as e:
    err(f"{path} is not valid TOML: {e}"); sys.exit(0)
if a.get("step") != step: err(f"step = {a.get('step')!r}, expected {step!r}")
if a.get("lapse") not in ("L1", "L2", "L3"): err("lapse must be L1, L2 or L3 (L4 lapses are deferred, not amended)")
if not str(a.get("reason", "")).strip(): err("reason is empty")
known = {"step", "lapse", "reason", "post_gate", "extra_files", "anchor_fix"}
for k in a:
    if k not in known: err(f"unknown key {k!r}")

GREP = re.compile(r"""(!\s*)?\bgrep\s+(?:-\S+\s+)*(['"])(.*?)\2""")
def norm(p): return re.sub(r"\\b|\\<|\\>|^\^|\$$", "", p)
def greps(line): return {(bool(m.group(1)), norm(m.group(3))) for m in GREP.finditer(line)}

gate = a.get("post_gate")
if gate is not None:
    gate = str(gate)
    if "\n" in gate: err("post_gate must be one line")
    if not base_gate: err("plan header has no **Base gate:** line; post_gate can't be checked")
    elif base_gate not in gate: err("post_gate must contain the plan's Base gate verbatim")
    for bad in ("|| true", "||true", "|| :", "; true", "set +e", "2>/dev/null ||"):
        if bad in gate: err(f"post_gate may not contain {bad!r}")
    missing = greps(orig_gate) - greps(gate)
    for neg, pat in sorted(missing):
        err(f"post_gate drops the spec's {'! ' if neg else ''}grep for {pat!r}")
    if gate.count("grep") < orig_gate.count("grep"): err("post_gate has fewer greps than the spec's POST_GATE")
    print(f"POST_GATE\t{gate}")

extras = a.get("extra_files", [])
if not isinstance(extras, list): err("extra_files must be an array of tables"); extras = []
if len(extras) > 2: err(f"extra_files has {len(extras)} entries; the limit is 2")
for e in extras[:2]:
    p, kind = str(e.get("path", "")), str(e.get("kind", ""))
    if not p or any(c in p for c in "*?[{"): err(f"extra_files path {p!r} must be one literal path"); continue
    if kind not in ("mod", "use", "reexport", "callsite"): err(f"{p}: kind must be mod, use, reexport or callsite"); continue
    if p == plan or p.startswith((".tracker/", ".phased/", ".git/")): err(f"{p}: not allowed as an extra file"); continue
    if any(fnmatch.fnmatchcase(p, g) for g in forbidden): err(f"{p}: matches a forbidden path"); continue
    print(f"EXTRA\t{p}\t{kind}")
af = a.get("anchor_fix", [])
if not isinstance(af, list) or not all(isinstance(x, str) for x in af): err("anchor_fix must be an array of strings")
PY
}

# --- pregate -----------------------------------------------------------------------
if [[ "$MODE" == "pregate" ]]; then
    [[ -n "$PRE_GATE" ]] || { echo "STEP ${STEP_ID} has no PRE_GATE line" >&2; exit 2; }
    if bash -c "$PRE_GATE"; then
        echo "STEP ${STEP_ID}: PRE_GATE exit 0"
        exit 0
    else
        rc=$?
        echo "STEP ${STEP_ID}: PRE_GATE exit ${rc}"
        exit 1
    fi
fi

# --- start -------------------------------------------------------------------------
if [[ "$MODE" == "start" ]]; then
    if [[ -f "$CURRENT" && "$(cat "$CURRENT")" != "$STEP_ID" ]]; then
        echo "STEP $(cat "$CURRENT") is still in flight; commit or defer it first" >&2
        exit 2
    fi
    if [[ -f "${SNAP}/head" && "$(cat "${SNAP}/head")" == "$(git rev-parse HEAD)" ]]; then
        echo "STEP ${STEP_ID}: keeping the existing snapshot at HEAD $(cut -c1-12 "${SNAP}/head") (resume/retry)."
        echo "$STEP_ID" >"$CURRENT"
        exit 0
    fi
    DIRTY_ALLOWED=()
    while IFS= read -r line; do
        p="${line#*  }"
        matches_any "$p" "${ALLOWED[@]}" && DIRTY_ALLOWED+=("$p")
    done < <(fingerprint)
    if [[ ${#DIRTY_ALLOWED[@]} -gt 0 ]]; then
        echo "STEP ${STEP_ID}: these ALLOWED_FILES are already dirty; commit or restore them first so defer can restore the step safely:" >&2
        printf '  %s\n' "${DIRTY_ALLOWED[@]}" >&2
        exit 2
    fi
    [[ -e "$AMEND" ]] && { echo "STEP ${STEP_ID}: ${AMEND} already exists; remove it or let Claude ratify it first" >&2; exit 2; }
    rm -rf "$SNAP"
    mkdir -p "$SNAP"
    git rev-parse HEAD >"${SNAP}/head"
    sha256sum -- "$PLAN" | cut -d' ' -f1 >"${SNAP}/plan"
    fingerprint >"${SNAP}/tree"
    tracker_items >"${SNAP}/items"
    echo "$STEP_ID" >"$CURRENT"
    echo "STEP ${STEP_ID}: snapshot taken (HEAD $(cut -c1-12 "${SNAP}/head"), $(wc -l <"${SNAP}/tree") pre-existing dirty paths)."
    echo "Allowed files:"
    printf '  %s\n' "${ALLOWED[@]}"
    exit 0
fi

# --- defer without a snapshot (PRE_GATE skip) ---------------------------------------
TASK_LINE="$(find_task)"
TASK_ID="${TASK_LINE%%$'\t'*}"
TASK_SCOPE="${TASK_LINE#*$'\t'}"
[[ -z "$TASK_LINE" ]] && TASK_SCOPE=""
prefix() { if [[ -n "$TASK_ID" ]]; then printf '[%s](%s) ' "$TASK_ID" "$TASK_SCOPE"; fi; }

write_defer_note_and_park() { # <patch-path or -> <restored list> <manual list> [<other list>]
    mkdir -p .phased/deferred
    {
        echo "# STEP ${STEP_ID} deferred"
        echo
        echo "- Title: ${TITLE}"
        echo "- Task: ${TASK_ID:-none}"
        echo "- Date: $(date -u +%FT%TZ)"
        echo "- Reason: ${REASON}"
        echo "- Attempt patch (local only): ${1}"
        [[ -n "$2" ]] && echo "- Restored: ${2}"
        [[ -n "$3" ]] && echo "- NOT restored (dirty before start; needs a human): ${3}"
        [[ -n "${4:-}" ]] && echo "- Left as they are (changed outside the step's files, not in the patch): ${4}"
    } >".phased/deferred/${STEP_ID}.md"
    if [[ -n "$TASK_ID" ]]; then
        neb status "$TASK_ID" pending >/dev/null
    fi
    git add -A -- ".phased/deferred/${STEP_ID}.md"
    [[ -d .tracker ]] && git add -A -- .tracker
    git commit -q -m "$(prefix)tracker: defer STEP ${STEP_ID}" -m "Reason: ${REASON}"
    log_line "$STEP_ID" deferred "$(git rev-parse --short HEAD)" "$REASON"
    rm -f "$CURRENT"
}

if [[ "$MODE" == "defer" ]]; then
    [[ -n "$REASON" ]] || { echo "defer needs --reason \"<why>\"" >&2; exit 2; }
    git diff --cached --quiet || { echo "the index has staged changes; defer only commits its own files" >&2; exit 2; }
    if [[ ! -d "$SNAP" ]]; then
        write_defer_note_and_park "- (step never started)" "" ""
        echo "STEP ${STEP_ID}: deferred without running (${REASON})."
        exit 0
    fi
    mkdir -p "${STATE}/deferred"
    PATCH="${STATE}/deferred/${STEP_ID}.patch"
    : >"$PATCH"
    RESTORED=()
    MANUAL=()
    OTHER=()
    OWNED=("${ALLOWED[@]}" "$AMEND")
    if [[ -f "$AMEND" ]]; then
        while IFS=$'\t' read -r kind a _; do
            [[ "$kind" == EXTRA ]] && OWNED+=("$a")
        done < <(read_amendment || true)
    fi
    mapfile -t CHANGED < <(changed_since_snapshot)
    for p in "${CHANGED[@]}"; do
        [[ -n "$p" ]] || continue
        [[ "$p" == .tracker/* || "$p" == "$PLAN" ]] && continue
        if ! matches_any "$p" "${OWNED[@]}"; then
            OTHER+=("$p")
            continue
        fi
        if grep -qF "  ${p}" "${SNAP}/tree"; then
            MANUAL+=("$p")
            continue
        fi
        if git cat-file -e "HEAD:${p}" 2>/dev/null; then
            git diff --binary HEAD -- "$p" >>"$PATCH"
            git restore --source=HEAD --staged --worktree -- "$p"
        else
            git diff --no-index --binary /dev/null "$p" >>"$PATCH" || true
            rm -f -- "$p"
        fi
        RESTORED+=("$p")
    done
    write_defer_note_and_park "$PATCH" "${RESTORED[*]:-}" "${MANUAL[*]:-}" "${OTHER[*]:-}"
    echo "STEP ${STEP_ID}: deferred. Attempt saved to ${PATCH}; restored ${#RESTORED[@]} path(s)."
    if [[ ${#OTHER[@]} -gt 0 ]]; then
        echo "Changed outside the step's files; left as they are (not in the patch):"
        printf '  %s\n' "${OTHER[@]}"
    fi
    if [[ ${#MANUAL[@]} -gt 0 ]]; then
        echo "These were dirty before start and changed again; not restored, tell the user:"
        printf '  %s\n' "${MANUAL[@]}"
        exit 1
    fi
    exit 0
fi

# --- check / commit need the snapshot ----------------------------------------------
[[ -d "$SNAP" ]] || { echo "no snapshot for STEP ${STEP_ID}; run: $0 start ${STEP_ID}" >&2; exit 2; }

EFFECTIVE_GATE="$POST_GATE"
EXTRA_PATHS=()
AMENDED=0
FAIL=0
fail() { echo "FAIL: $*"; FAIL=1; }

if [[ -f "$AMEND" ]]; then
    AMENDED=1
    AHASH="$(sha256sum -- "$AMEND" | cut -d' ' -f1)"
    if [[ -f "${SNAP}/amend_hash" && "$(cat "${SNAP}/amend_hash")" != "$AHASH" ]]; then
        fail "${AMEND} changed after its first use; one amendment per step"
    fi
    [[ -f "${SNAP}/amend_hash" ]] || echo "$AHASH" >"${SNAP}/amend_hash"
    while IFS=$'\t' read -r kind a b; do
        case "$kind" in
            ERR) fail "amendment: ${a}" ;;
            POST_GATE) EFFECTIVE_GATE="$a" ;;
            EXTRA) EXTRA_PATHS+=("$a") ;;
        esac
    done < <(read_amendment)
fi

# 1. No commits. 2. Plan untouched.
[[ "$(git rev-parse HEAD)" == "$(cat "${SNAP}/head")" ]] ||
    fail "HEAD moved since start ($(cut -c1-12 "${SNAP}/head") -> $(git rev-parse --short=12 HEAD)); nothing may commit mid-step"
[[ "$(sha256sum -- "$PLAN" | cut -d' ' -f1)" == "$(cat "${SNAP}/plan")" ]] ||
    fail "$PLAN changed since start; only Claude edits the plan, at phase review"

# 3. Scope + tracker guard.
mapfile -t CHANGED < <(changed_since_snapshot)
INDEX_FILES=(project.toml phase.toml scope.toml .timer.toml)
OUT_OF_SCOPE=()
HIT_FORBIDDEN=()
BAD_TRACKER=()
STEP_PATHS=()
for p in "${CHANGED[@]}"; do
    [[ -n "$p" ]] || continue
    [[ "$p" == "$PLAN" ]] && continue
    if [[ "$p" == .tracker/* ]]; then
        if [[ "$p" =~ /items/([^/]+)/ ]]; then
            id="${BASH_REMATCH[1]}"
            if grep -qxF "$id" "${SNAP}/items"; then
                BAD_TRACKER+=("$p (existing item ${id} changed)")
            elif [[ -f "$p" && "$p" == */item.toml ]] &&
                ! grep -qE '^status = "pending"' "$p"; then
                BAD_TRACKER+=("$p (new item ${id} isn't pending)")
            fi
        elif ! matches_any "$(basename "$p")" "${INDEX_FILES[@]}"; then
            BAD_TRACKER+=("$p (not an item or index file)")
        fi
        continue
    fi
    matches_any "$p" "${FORBIDDEN[@]}" && HIT_FORBIDDEN+=("$p")
    if matches_any "$p" "${ALLOWED[@]}" "${EXTRA_PATHS[@]}" "$AMEND"; then
        STEP_PATHS+=("$p")
    else
        OUT_OF_SCOPE+=("$p")
    fi
done
if [[ ${#OUT_OF_SCOPE[@]} -gt 0 ]]; then
    fail "changed outside ALLOWED_FILES (scope lapse? see the amendment limits):"
    printf '        %s\n' "${OUT_OF_SCOPE[@]}"
fi
if [[ ${#HIT_FORBIDDEN[@]} -gt 0 ]]; then
    fail "changed a forbidden path:"
    printf '        %s\n' "${HIT_FORBIDDEN[@]}"
fi
if [[ ${#BAD_TRACKER[@]} -gt 0 ]]; then
    fail ".tracker/ may only gain new pending items:"
    printf '        %s\n' "${BAD_TRACKER[@]}"
fi
# 3b. Amendment extra files: existing files, at most 3 changed lines each.
for p in "${EXTRA_PATHS[@]}"; do
    if ! git cat-file -e "HEAD:${p}" 2>/dev/null; then
        fail "amendment extra file ${p} must already exist (new files need a re-split)"
        continue
    fi
    read -r add del _ < <(git diff --numstat HEAD -- "$p" || true) || true
    [[ "${add:-0}" == "-" ]] && { fail "amendment extra file ${p} is binary"; continue; }
    if (( ${add:-0} + ${del:-0} > 3 )); then
        fail "amendment extra file ${p} changed $(( ${add:-0} + ${del:-0} )) lines; the limit is 3"
    fi
done

echo "Changed since start (${#CHANGED[@]}):"
[[ ${#CHANGED[@]} -gt 0 && -n "${CHANGED[0]}" ]] && printf '  %s\n' "${CHANGED[@]}"

if [[ "$MODE" == "commit" ]]; then
    [[ -f "${SNAP}/pass" ]] || { echo "STEP ${STEP_ID}: no passing check recorded; run check first" >&2; exit 1; }
    [[ "$(code_stamp)" == "$(cat "${SNAP}/pass")" ]] ||
        { echo "STEP ${STEP_ID}: the tree changed since the passing check; run check again" >&2; exit 1; }
    [[ "$FAIL" -eq 0 ]] || { echo "STEP ${STEP_ID}: scope check fails now; not committing" >&2; exit 1; }
    git diff --cached --quiet || { echo "the index has staged changes; commit stages only the step's files" >&2; exit 2; }
    if [[ ${#STEP_PATHS[@]} -eq 0 ]]; then
        # BUG-0021: a rerun after a pass (e.g. orrery reset) has nothing new to commit.
        # Success when the step's commit already exists; a real no-op step still fails.
        PREV="$(git log -n1 --format=%h -F --grep="STEP ${STEP_ID}: " HEAD)"
        if [[ -n "$PREV" ]]; then
            echo "STEP ${STEP_ID}: already committed ${PREV}; nothing new to commit."
            log_line "$STEP_ID" committed-earlier "$PREV" "rerun, no new changes"
            rm -f "$CURRENT"
            exit 0
        fi
        echo "STEP ${STEP_ID}: no step files changed; nothing to commit" >&2
        exit 1
    fi
    git add -A -- "${STEP_PATHS[@]}"
    BODY="Phased workflow: committed by Hydrogen after phased-step.sh check PASS."
    [[ "$AMENDED" -eq 1 ]] && BODY+=$'\n'"Amended: ${AMEND} (pending Claude phase review)."
    git commit -q -m "$(prefix)STEP ${STEP_ID}: ${TITLE}" -m "$BODY"
    SHA="$(git rev-parse --short HEAD)"
    if [[ -n "$TASK_ID" ]]; then
        neb close "$TASK_ID" --commit "$SHA" -m "STEP ${STEP_ID}: ${TITLE} (phased check PASS)" >/dev/null
        git add -A -- .tracker
        git diff --cached --quiet || git commit -q -m "$(prefix)tracker: close STEP ${STEP_ID}"
    elif [[ -d .tracker ]]; then
        git add -A -- .tracker
        git diff --cached --quiet || git commit -q -m "STEP ${STEP_ID}: tracker (pending items filed during the step)"
    fi
    OUTCOME=committed
    [[ "$AMENDED" -eq 1 ]] && OUTCOME=committed+amended
    log_line "$STEP_ID" "$OUTCOME" "$SHA" "${TASK_ID:-no task}"
    rm -f "$CURRENT"
    echo "STEP ${STEP_ID}: committed ${SHA}${TASK_ID:+, closed ${TASK_ID}}."
    exit 0
fi

# 4. POST_GATE (check mode).
rm -f "${SNAP}/pass"
if [[ "$RUN_GATE" -eq 1 ]]; then
    if [[ -z "$EFFECTIVE_GATE" ]]; then
        fail "STEP ${STEP_ID} has no POST_GATE line"
    else
        GLOG="${SNAP}/post_gate.log"
        [[ "$AMENDED" -eq 1 && "$EFFECTIVE_GATE" != "$POST_GATE" ]] && echo "Using the amendment's post_gate."
        echo "Running POST_GATE (log: ${GLOG})"
        if bash -c "$EFFECTIVE_GATE" >"$GLOG" 2>&1; then
            echo "POST_GATE: exit 0"
        else
            rc=$?
            fail "POST_GATE exited ${rc}; last 30 lines:"
            tail -n 30 "$GLOG" | sed 's/^/        /'
        fi
    fi
else
    echo "POST_GATE: skipped (--no-gate); commit stays locked"
fi

if [[ "$FAIL" -eq 0 && "$RUN_GATE" -eq 1 ]]; then
    code_stamp >"${SNAP}/pass"
    echo "STEP ${STEP_ID}: PASS"
elif [[ "$FAIL" -eq 0 ]]; then
    echo "STEP ${STEP_ID}: PASS (scope only)"
else
    echo "STEP ${STEP_ID}: FAIL"
fi
exit "$FAIL"
