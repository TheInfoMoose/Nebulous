#!/usr/bin/env bash
set -euo pipefail

# Nebulous Turnkey Installer (`nebulous` COSMIC GUI + `neb` CLI + Desktop Integration)
# Supports:
#   ./scripts/install.sh [--user] [--system] [--prefix <DIR>] [--skip-build] [--uninstall]

CALLER_PWD="$(pwd)"
SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}" 2>/dev/null || realpath "${BASH_SOURCE[0]}" 2>/dev/null || echo "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "${SCRIPT_PATH}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
APP_ID="com.nebulous.Tracker"

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

Install or uninstall Nebulous (nebulous GUI, neb CLI, icon, and .desktop launcher).

Options:
  --user            Install to user prefix (\${HOME}/.local) [default]
  --system          Install to system prefix (/usr/local)
  --prefix <PATH>   Install to an explicit custom prefix path
  --skip-build      Use existing target/release/{nebulous,neb} binaries without running cargo build
  --uninstall       Remove installed binaries, icon, metainfo, and .desktop entry from target prefix
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
DATADIR="${PREFIX}/share"
APPDIR="${DATADIR}/applications"
ICONDIR="${DATADIR}/icons/hicolor/scalable/apps"
METAICODIR="${DATADIR}/metainfo"

refresh_desktop_caches() {
  if command -v update-desktop-database >/dev/null 2>&1 && [[ -d "${APPDIR}" ]]; then
    update-desktop-database "${APPDIR}" >/dev/null 2>&1 || true
  fi
  if command -v gtk-update-icon-cache >/dev/null 2>&1 && [[ -d "${DATADIR}/icons/hicolor" ]]; then
    gtk-update-icon-cache -f -t "${DATADIR}/icons/hicolor" >/dev/null 2>&1 || true
  fi
}

if [[ "${ACTION}" == "uninstall" ]]; then
  echo "Uninstalling Nebulous from ${PREFIX}..."
  rm -f "${BINDIR}/nebulous"
  rm -f "${BINDIR}/neb"
  rm -f "${APPDIR}/${APP_ID}.desktop"
  rm -f "${ICONDIR}/${APP_ID}.svg"
  rm -f "${METAICODIR}/${APP_ID}.metainfo.xml"
  refresh_desktop_caches
  echo "Uninstalled Nebulous from ${PREFIX}."
  exit 0
fi

if [[ -n "${NEBULOUS_BIN_DIR:-}" ]]; then
  BIN_SOURCE_DIR="$(abs_path "${NEBULOUS_BIN_DIR}")"
else
  BIN_SOURCE_DIR="${REPO_ROOT}/target/release"
fi

cd "${REPO_ROOT}"

if [[ "${SKIP_BUILD}" -eq 0 ]]; then
  echo "Building Nebulous release binaries (nebulous + neb)..."
  cargo build --workspace --release
fi

NEBULOUS_BIN="${BIN_SOURCE_DIR}/nebulous"
NEB_BIN="${BIN_SOURCE_DIR}/neb"

if [[ ! -x "${NEBULOUS_BIN}" ]]; then
  echo "Error: Release binary not found at ${NEBULOUS_BIN}. Run without --skip-build first." >&2
  exit 1
fi
if [[ ! -x "${NEB_BIN}" ]]; then
  echo "Error: Release binary not found at ${NEB_BIN}. Run without --skip-build first." >&2
  exit 1
fi

echo "Installing Nebulous to ${PREFIX}..."
install -d "${BINDIR}" "${APPDIR}" "${ICONDIR}" "${METAICODIR}"
install -m 0755 "${NEBULOUS_BIN}" "${BINDIR}/nebulous"
install -m 0755 "${NEB_BIN}" "${BINDIR}/neb"
install -m 0644 "${REPO_ROOT}/packaging/${APP_ID}.desktop" "${APPDIR}/${APP_ID}.desktop"
install -m 0644 "${REPO_ROOT}/packaging/icons/${APP_ID}.svg" "${ICONDIR}/${APP_ID}.svg"
install -m 0644 "${REPO_ROOT}/packaging/${APP_ID}.metainfo.xml" "${METAICODIR}/${APP_ID}.metainfo.xml"

refresh_desktop_caches

echo "Successfully installed:"
echo "  GUI Binary : ${BINDIR}/nebulous"
echo "  CLI Binary : ${BINDIR}/neb"
echo "  Desktop    : ${APPDIR}/${APP_ID}.desktop"
echo "  Icon       : ${ICONDIR}/${APP_ID}.svg"
echo "  Metainfo   : ${METAICODIR}/${APP_ID}.metainfo.xml"

if [[ ":${PATH}:" != *":${BINDIR}:"* ]]; then
  echo ""
  echo "Note: ${BINDIR} is not currently in your PATH."
  echo "Add the following line to your shell profile (~/.bashrc or ~/.zshrc):"
  echo "  export PATH=\"${BINDIR}:\$PATH\""
fi
