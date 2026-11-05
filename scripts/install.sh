#!/usr/bin/env bash
set -euo pipefail

# Nebulous CLI Installer (`neb`)
# Supports:
#   ./scripts/install.sh [--user] [--system] [--prefix <DIR>] [--skip-build] [--uninstall]

CALLER_PWD="$(pwd)"
SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}" 2>/dev/null || realpath "${BASH_SOURCE[0]}" 2>/dev/null || echo "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "${SCRIPT_PATH}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

abs_path() {
  local target="$1"
  if command -v realpath >/dev/null 2>&1; then
    realpath -m "${target}"
  elif [[ "${target}" == /* ]]; then
    echo "${target%/}"
  else
    echo "${CALLER_PWD}/${target%/}"
  fi
}

MODE="user"
CUSTOM_PREFIX=""
ACTION="install"
SKIP_BUILD=0

usage() {
  cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Install or uninstall the Nebulous CLI (neb).

Options:
  --user            Install to user prefix (\${HOME}/.local) [default]
  --system          Install to system prefix (/usr/local)
  --prefix <PATH>   Install to an explicit custom prefix path
  --skip-build      Use existing target/release/neb binary without running cargo build
  --uninstall       Remove installed neb binary from target prefix
  -h, --help        Show this help message
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --user)
      MODE="user"
      shift
      ;;
    --system)
      MODE="system"
      shift
      ;;
    --prefix)
      if [[ $# -lt 2 || -z "${2:-}" || "${2:-}" == -* ]]; then
        echo "Error: --prefix requires a non-empty directory argument" >&2
        exit 1
      fi
      CUSTOM_PREFIX="$2"
      shift 2
      ;;
    --prefix=*)
      CUSTOM_PREFIX="${1#*=}"
      if [[ -z "${CUSTOM_PREFIX}" ]]; then
        echo "Error: --prefix requires a non-empty directory argument" >&2
        exit 1
      fi
      shift
      ;;
    --skip-build)
      SKIP_BUILD=1
      shift
      ;;
    --uninstall|uninstall)
      ACTION="uninstall"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Error: Unknown option '$1'" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -n "${CUSTOM_PREFIX}" ]]; then
  PREFIX="$(abs_path "${CUSTOM_PREFIX}")"
elif [[ "${MODE}" == "system" ]]; then
  PREFIX="/usr/local"
else
  PREFIX="$(abs_path "${HOME}/.local")"
fi

BINDIR="${PREFIX}/bin"

if [[ "${ACTION}" == "uninstall" ]]; then
  echo "Uninstalling Nebulous CLI from ${PREFIX}..."
  rm -f "${BINDIR}/neb"
  echo "Uninstalled Nebulous CLI from ${PREFIX}."
  exit 0
fi

if [[ -n "${NEBULOUS_BIN_DIR:-}" ]]; then
  BIN_SOURCE_DIR="$(abs_path "${NEBULOUS_BIN_DIR}")"
else
  BIN_SOURCE_DIR="${REPO_ROOT}/target/release"
fi

cd "${REPO_ROOT}"

if [[ "${SKIP_BUILD}" -eq 0 ]]; then
  echo "Building Nebulous CLI release binary (neb)..."
  cargo build --release -p nebulous_cli
fi

NEB_BIN="${BIN_SOURCE_DIR}/neb"

if [[ ! -x "${NEB_BIN}" ]]; then
  echo "Error: Release binary not found at ${NEB_BIN}. Run without --skip-build first." >&2
  exit 1
fi

echo "Installing Nebulous CLI to ${PREFIX}..."
install -d "${BINDIR}"
install -m 0755 "${NEB_BIN}" "${BINDIR}/neb"

echo "Successfully installed:"
echo "  CLI Binary : ${BINDIR}/neb"

if [[ ":${PATH}:" != *":${BINDIR}:"* ]]; then
  echo ""
  echo "Note: ${BINDIR} is not currently in your PATH."
  echo "Add the following line to your shell profile (~/.bashrc or ~/.zshrc):"
  echo "  export PATH=\"${BINDIR}:\$PATH\""
fi
