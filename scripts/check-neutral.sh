#!/usr/bin/env bash
set -euo pipefail

# Neutrality ratchet for Nebulous: enforces that tracked files do not reference
# NovOS or distro-specific tokens (novos, debian, dpkg, squashfs, xorriso,
# calamares, xanmod, qemu).
# The allowlist only shrinks; when you remove hits, lower or delete the entry in the same change.

cd "$(git rev-parse --show-toplevel)"

PATTERN='novos|debian|dpkg|squashfs|xorriso|calamares|xanmod|qemu'

declare -A ALLOWLIST=()
declare -A SEEN_TRACKED=()
total_allowlisted_lines=0

while read -r count path; do
  [[ -z "${count:-}" ]] && continue
  ALLOWLIST["$path"]="$count"
  total_allowlisted_lines=$((total_allowlisted_lines + count))
done <<'EOF'
EOF

files=()
no_args=0
if [[ $# -eq 0 ]]; then
  no_args=1
  while IFS= read -r f; do
    [[ -z "$f" ]] && continue
    [[ -e "$f" ]] || continue
    files+=("$f")
    SEEN_TRACKED["$f"]=1
  done < <(git ls-files | grep -vE '^archive/|^Cargo\.lock$|^scripts/check-neutral\.sh$')
else
  files=("$@")
fi

failed=0

for file in "${files[@]}"; do
  if [[ ! -f "$file" ]]; then echo "${file}: no such file" >&2; failed=1; continue; fi
  n="$(grep -ciIE "$PATTERN" "$file" || true)"
  if [[ -v ALLOWLIST["$file"] ]]; then
    m="${ALLOWLIST["$file"]}"
    if (( n > m )); then
      echo "${file}: ${n} lines, allowlist says ${m}: remove the new ones" >&2
      grep -niIE "$PATTERN" "$file" >&2 || true
      failed=1
    elif (( n < m )); then
      if (( n == 0 )); then
        echo "${file}: 0 lines, allowlist says ${m}: delete the entry" >&2
      else
        echo "${file}: ${n} lines, allowlist says ${m}: lower the entry to ${n}" >&2
      fi
      failed=1
    fi
  else
    if (( n > 0 )); then
      echo "${file}: ${n} line(s) name a distro (not allowlisted)" >&2
      grep -niIE "$PATTERN" "$file" >&2 || true
      failed=1
    fi
  fi
done

if (( no_args == 1 )); then
  for path in "${!ALLOWLIST[@]}"; do
    if [[ ! -v SEEN_TRACKED["$path"] ]]; then
      echo "${path}: allowlisted but not tracked: delete the entry" >&2
      failed=1
    fi
  done
fi

if (( failed != 0 )); then
  exit 1
fi

echo "check-neutral: OK (${#ALLOWLIST[@]} allowlisted file(s), ${total_allowlisted_lines} line(s) left to scrub)"
