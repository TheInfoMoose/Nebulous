#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

usage() {
  echo "Usage: bump-version.sh [--dry-run] X.Y.Z" >&2
  exit 2
}

DRY_RUN=0
if [[ $# -ge 1 && "$1" == "--dry-run" ]]; then
  DRY_RUN=1
  shift
fi

[[ $# -eq 1 ]] || usage
NEW="$1"

if [[ ! "$NEW" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  usage
fi

CURRENT="$(awk '
  /^\[workspace\.package\]$/ { in_pkg = 1; next }
  /^\[/ { in_pkg = 0 }
  in_pkg && /^version[[:space:]]*=[[:space:]]*"/ {
    sub(/^version[[:space:]]*=[[:space:]]*"/, "")
    sub(/".*$/, "")
    print
    exit
  }
' Cargo.toml)"

if [[ "$NEW" != "$CURRENT" ]]; then
  FIRST_SORTED="$(printf '%s\n%s\n' "$CURRENT" "$NEW" | sort -V | head -n 1)"
  if [[ "$FIRST_SORTED" != "$CURRENT" ]]; then
    echo "bump-version: ${NEW} is lower than the current ${CURRENT}" >&2
    exit 1
  fi
fi

UNRELEASED_COUNT="$(grep -cx '## \[Unreleased\]' CHANGELOG.md || true)"
if [[ "$UNRELEASED_COUNT" -ne 1 ]]; then
  echo "bump-version: CHANGELOG.md needs a non-empty ## [Unreleased] section" >&2
  exit 1
fi

HAS_UNRELEASED_BODY="$(awk '
  /^## \[Unreleased\]$/ { in_unrel = 1; next }
  in_unrel && /^## \[/ { exit }
  in_unrel && $0 !~ /^[[:space:]]*$/ { found = 1; exit }
  END { print (found ? 1 : 0) }
' CHANGELOG.md)"

if [[ "$HAS_UNRELEASED_BODY" -ne 1 ]]; then
  echo "bump-version: CHANGELOG.md needs a non-empty ## [Unreleased] section" >&2
  exit 1
fi

ESCAPED_NEW="${NEW//./\\.}"
if grep -Eq "^## \[${ESCAPED_NEW}\]" CHANGELOG.md; then
  echo "bump-version: CHANGELOG.md already has a ${NEW} section" >&2
  exit 1
fi

DATE="$(date -u +%Y-%m-%d)"

if [[ "$DRY_RUN" -eq 1 ]]; then
  if [[ "$NEW" == "$CURRENT" ]]; then
    echo "Cargo.toml: already ${NEW}"
  else
    echo "Cargo.toml: ${CURRENT} -> ${NEW}"
  fi
  echo "CHANGELOG.md: ## [Unreleased] -> ## [${NEW}] - ${DATE}"
  exit 0
fi

if [[ "$NEW" != "$CURRENT" ]]; then
  TMP_CARGO="$(mktemp)"
  awk -v cur="$CURRENT" -v new="$NEW" '
    /^\[workspace\.package\]$/ { in_pkg = 1; print; next }
    /^\[/ { in_pkg = 0 }
    in_pkg && $0 == "version = \"" cur "\"" {
      print "version = \"" new "\""
      next
    }
    /^backend_core[[:space:]]*=.*path[[:space:]]*=[[:space:]]*"crates\/backend_core"/ {
      sub("version = \"" cur "\"", "version = \"" new "\"")
      print
      next
    }
    { print }
  ' Cargo.toml > "$TMP_CARGO"
  mv "$TMP_CARGO" Cargo.toml

  cargo update --workspace
fi

TMP_CHANGELOG="$(mktemp)"
awk -v new="$NEW" -v dt="$DATE" '
  /^## \[Unreleased\]$/ {
    print "## [Unreleased]"
    print ""
    print "## [" new "] - " dt
    next
  }
  { print }
' CHANGELOG.md > "$TMP_CHANGELOG"
mv "$TMP_CHANGELOG" CHANGELOG.md

if ! bash scripts/release-check.sh "v${NEW}" > /dev/null; then
  echo "bump-version: release-check failed; fix the files above before tagging" >&2
  exit 1
fi

cat <<EOF
Next (run these yourself):
  git add Cargo.toml Cargo.lock CHANGELOG.md
  git commit -m "Release v${NEW}"
  git tag v${NEW}
  git push origin main v${NEW}
EOF
