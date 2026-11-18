#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "Usage: release-check.sh [--root DIR] <tag>" >&2
  exit 2
}

ROOT=""
if [[ $# -ge 1 && "$1" == "--root" ]]; then
  [[ $# -ge 2 ]] || usage
  ROOT="$2"
  shift 2
fi

[[ $# -eq 1 ]] || usage
TAG="$1"

if [[ ! "$TAG" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
  usage
fi

if [[ -z "$ROOT" ]]; then
  ROOT="$(git rev-parse --show-toplevel)"
fi

VERSION="${TAG#v}"

CARGO_VERSION="$(awk '
  /^\[workspace\.package\]$/ { in_pkg = 1; next }
  /^\[/ { in_pkg = 0 }
  in_pkg && /^version[[:space:]]*=[[:space:]]*"/ {
    sub(/^version[[:space:]]*=[[:space:]]*"/, "")
    sub(/".*$/, "")
    print
    exit
  }
' "$ROOT/Cargo.toml")"

if [[ "$CARGO_VERSION" != "$VERSION" ]]; then
  echo "release-check: tag ${TAG} does not match Cargo.toml version ${CARGO_VERSION}" >&2
  exit 1
fi

ESCAPED_VERSION="${VERSION//./\\.}"

if ! grep -Eq "^## \[${ESCAPED_VERSION}\] - [0-9]{4}-[0-9]{2}-[0-9]{2}$" "$ROOT/CHANGELOG.md"; then
  echo "release-check: CHANGELOG.md has no dated '## [${VERSION}] - YYYY-MM-DD' section" >&2
  exit 1
fi

BODY="$(awk -v ver="$VERSION" '
  BEGIN {
    esc = ver
    gsub(/\./, "\\.", esc)
    re = "^## \\[" esc "\\] - [0-9]{4}-[0-9]{2}-[0-9]{2}$"
    in_sec = 0
    count = 0
  }
  in_sec && /^## \[/ {
    exit
  }
  in_sec {
    lines[++count] = $0
    next
  }
  $0 ~ re {
    in_sec = 1
  }
  END {
    start = 1
    while (start <= count && lines[start] ~ /^[[:space:]]*$/) {
      start++
    }
    end = count
    while (end >= start && lines[end] ~ /^[[:space:]]*$/) {
      end--
    }
    for (i = start; i <= end; i++) {
      print lines[i]
    }
  }
' "$ROOT/CHANGELOG.md")"

if [[ -z "$BODY" ]]; then
  echo "release-check: CHANGELOG.md section for ${VERSION} is empty" >&2
  exit 1
fi

printf '%s\n' "$BODY"
