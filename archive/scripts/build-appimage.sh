#!/usr/bin/env bash
set -euo pipefail

# Nebulous Portable AppImage & AppDir Builder
# Packages both `nebulous` (COSMIC GUI) and `neb` (CLI + MCP server) into a portable AppDir.
# Builds `Nebulous-x86_64.AppImage` when `appimagetool` is installed on the host, and always
# produces a ready-to-run portable `AppDir` + `.tar.gz` bundle.

CALLER_PWD="$(pwd)"
SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}" 2>/dev/null || realpath "${BASH_SOURCE[0]}" 2>/dev/null || echo "${BASH_SOURCE[0]}")"
SCRIPT_DIR="$(cd "$(dirname "${SCRIPT_PATH}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
APP_ID="com.nebulous.Tracker"
ARCH="${ARCH:-x86_64}"

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

SKIP_BUILD=0
OUTPUT_DIR="${REPO_ROOT}/target/appimage"

usage() {
  cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Package Nebulous (nebulous GUI + neb CLI/MCP) into an AppDir and AppImage.

Options:
  --skip-build          Reuse existing target/release/{nebulous,neb} binaries
  --output-dir <PATH>   Directory for AppDir and AppImage artifacts (default: target/appimage)
  -h, --help            Show this help message
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build)
      SKIP_BUILD=1
      shift
      ;;
    --output-dir)
      if [[ $# -lt 2 || -z "${2:-}" || "${2:-}" == -* ]]; then
        echo "Error: --output-dir requires a non-empty directory argument" >&2
        exit 1
      fi
      OUTPUT_DIR="$2"
      shift 2
      ;;
    --output-dir=*)
      OUTPUT_DIR="${1#*=}"
      if [[ -z "${OUTPUT_DIR}" ]]; then
        echo "Error: --output-dir requires a non-empty directory argument" >&2
        exit 1
      fi
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

OUTPUT_DIR="$(abs_path "${OUTPUT_DIR}")"
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

if [[ ! -x "${NEBULOUS_BIN}" || ! -x "${NEB_BIN}" ]]; then
  echo "Error: Missing binaries in ${BIN_SOURCE_DIR}/. Run without --skip-build first." >&2
  exit 1
fi

APPDIR="${OUTPUT_DIR}/Nebulous.AppDir"
rm -rf "${APPDIR}"
mkdir -p \
  "${APPDIR}/usr/bin" \
  "${APPDIR}/usr/share/applications" \
  "${APPDIR}/usr/share/icons/hicolor/scalable/apps" \
  "${APPDIR}/usr/share/metainfo"

install -m 0755 "${NEBULOUS_BIN}" "${APPDIR}/usr/bin/nebulous"
install -m 0755 "${NEB_BIN}" "${APPDIR}/usr/bin/neb"
install -m 0644 "${REPO_ROOT}/packaging/${APP_ID}.desktop" "${APPDIR}/usr/share/applications/${APP_ID}.desktop"
install -m 0644 "${REPO_ROOT}/packaging/${APP_ID}.desktop" "${APPDIR}/${APP_ID}.desktop"
install -m 0644 "${REPO_ROOT}/packaging/icons/${APP_ID}.svg" "${APPDIR}/usr/share/icons/hicolor/scalable/apps/${APP_ID}.svg"
install -m 0644 "${REPO_ROOT}/packaging/icons/${APP_ID}.svg" "${APPDIR}/${APP_ID}.svg"
ln -sf "${APP_ID}.svg" "${APPDIR}/.DirIcon"
install -m 0644 "${REPO_ROOT}/packaging/${APP_ID}.metainfo.xml" "${APPDIR}/usr/share/metainfo/${APP_ID}.metainfo.xml"

cat > "${APPDIR}/AppRun" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

INVOKED_AS="$(basename "${ARGV0:-$0}")"
SCRIPT_PATH="$(readlink -f "${BASH_SOURCE[0]}" 2>/dev/null || realpath "${BASH_SOURCE[0]}" 2>/dev/null || echo "${BASH_SOURCE[0]}")"
HERE="${APPDIR:-$(cd "$(dirname "${SCRIPT_PATH}")" && pwd)}"
export PATH="${HERE}/usr/bin:${PATH}"
export XDG_DATA_DIRS="${HERE}/usr/share:${XDG_DATA_DIRS:-/usr/local/share:/usr/share}"

# Dispatch to `neb` CLI when invoked via a `neb` symlink, `--cli` flag, or `neb` subcommand (`mcp`, etc.)
if [[ "${INVOKED_AS}" == "neb" || "${INVOKED_AS}" == "neb-"* || "${INVOKED_AS}" == *.neb ]]; then
  exec "${HERE}/usr/bin/neb" "$@"
fi

if [[ "${1:-}" == "--cli" ]]; then
  shift
  exec "${HERE}/usr/bin/neb" "$@"
fi

# Inspect arguments (allowing global `neb` flags like `--json` or `--root <PATH>` before subcommands)
ARGS=("$@")
IDX=0
while [[ ${IDX} -lt ${#ARGS[@]} ]]; do
  ARG="${ARGS[${IDX}]}"
  case "${ARG}" in
    --json)
      exec "${HERE}/usr/bin/neb" "$@"
      ;;
    --root|--workspace|-w)
      IDX=$((IDX + 2))
      continue
      ;;
    --root=*|--workspace=*)
      IDX=$((IDX + 1))
      continue
      ;;
    mcp|init|bug|feat|feature|list|show|status|close|reopen|timer|log|context|metrics|sync|sync-index|phase|scope|project|ingest|component|git)
      exec "${HERE}/usr/bin/neb" "$@"
      ;;
    *)
      break
      ;;
  esac
done

# Support launching GUI by passing a project directory path directly (`AppRun /path/to/repo`)
if [[ $# -ge 1 && "${1:-}" != -* && -d "${1}" ]]; then
  DIR_ARG="$1"
  shift
  exec "${HERE}/usr/bin/nebulous" --workspace "${DIR_ARG}" "$@"
fi

exec "${HERE}/usr/bin/nebulous" "$@"
EOF

chmod 0755 "${APPDIR}/AppRun"

TARBALL="${OUTPUT_DIR}/Nebulous-${ARCH}-AppDir.tar.gz"
tar -C "${OUTPUT_DIR}" -czf "${TARBALL}" "Nebulous.AppDir"
echo "Created portable AppDir bundle: ${TARBALL}"

APPIMAGE_OUT="${OUTPUT_DIR}/Nebulous-${ARCH}.AppImage"
if command -v appimagetool >/dev/null 2>&1; then
  echo "Building ${APPIMAGE_OUT} with appimagetool..."
  ARCH="${ARCH}" APPIMAGE_EXTRACT_AND_RUN="${APPIMAGE_EXTRACT_AND_RUN:-1}" appimagetool "${APPDIR}" "${APPIMAGE_OUT}"
  chmod 0755 "${APPIMAGE_OUT}"
  echo "Built AppImage: ${APPIMAGE_OUT}"
else
  echo "Note: 'appimagetool' not found in PATH; portable AppDir and tarball are ready at:"
  echo "  AppDir  : ${APPDIR} (run '${APPDIR}/AppRun' or '${APPDIR}/AppRun --cli ...')"
  echo "  Archive : ${TARBALL}"
fi
