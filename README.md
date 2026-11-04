# Nebulous

[![CI](https://github.com/theinfomoose/Nebulous/actions/workflows/ci.yml/badge.svg)](https://github.com/theinfomoose/Nebulous/actions/workflows/ci.yml)
[![Release](https://github.com/theinfomoose/Nebulous/actions/workflows/release.yml/badge.svg)](https://github.com/theinfomoose/Nebulous/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**Nebulous** is a local-first, Git-backed, 4-tier project, bug, and release-engineering tracker built in Rust for developers, rolling-release OS builders (such as **NovOS**), and AI-assisted coding workflows. It pairs a fast companion CLI (`neb`) and built-in Model Context Protocol server (`neb mcp`) with a native `libcosmic` desktop GUI (`nebulous`), storing all project state in human-readable TOML manifests and Markdown files inside `.tracker/`.

---

## Key Features

- **4-Tier `.tracker/` TOML + Markdown Hierarchy**:
  - **Tier 1 (`project.toml`)**: Global project metadata, monotonic ID counters (`BUG-XXXX`, `FEAT-XXXX`), phase registry, ingested `[components]` catalog, effort/stagnation thresholds, and Git sync + `[git.commit_policy]` configuration.
  - **Tier 2 (`phases/<phase_id>/phase.toml`)**: Phase lifecycle status, registered domain scopes, and roll-up completion counts.
  - **Tier 3 (`.../domains/<scope_id>/scope.toml`)**: Domain/crate boundary metadata (`target`, `tech_stack`, `description`, `bound_component`, `tracked_files`) and lightweight item stubs (`bugs`, `features`) for instant startup indexing.
  - **Tier 4 (`.../items/<ITEM_ID>/item.toml` + `desc.md`)**: Authoritative item metadata, effort tracking (`logged_time_minutes`, `iteration_count`), historical fix ledger (`recurrence_count`, `previous_fixes`), and rich Markdown specification (`desc.md`).
- **Global Multi-Project Registry (`projects.toml`)**:
  - Manage and switch between multiple repositories across your machine (`neb project list | add | switch | remove` or the GUI Project Switcher dropdown) while keeping each repository's `.tracker/` self-contained.
- **Two-Stage Directory Ingestion & Rolling-Release Pinning (`nebulous.component.toml`)**:
  - Stage 1 (`neb ingest`): Walks the project tree respecting `.gitignore`, auto-detects ecosystem markers (`Cargo.toml`, `package.json`, `pyproject.toml`, `CMakeLists.txt`, `Makefile`, `Dockerfile`, or `nebulous.component.toml`), and records relative file paths (`files = [...]`) without slow AST parsing.
  - Stage 2 (`neb component init | pin | rollback | promote`): Generates `nebulous.component.toml` manifests for un-manifested directories or OS components (such as **NovOS** tracking Debian Sid packages and XanMod kernels), snapshots `last_known_good` (`pinned_release`) configurations, and promotes components directly into Phase Domain Scopes.
- **Model Context Protocol Server (`neb mcp`) & Anti-Looping Guardrails**:
  - Built-in JSON-RPC 2.0 `stdio` MCP server exposing 21 tools across Task & Boundary Management, Project & Component Ingestion, NovOS Self-Healing CI/CD (`record_rolling_update`, `record_verification_run`, `pin_stable_release`, `rollback_to_pinned_release`), and Policy-Validated Git Operations.
  - One-command **Bounded Agent Prompt** and `@file` reference export (`neb context <ID> [--refs-only] [--copy]`) scopes AI coding assistants to the exact target crate and warns against repeating failed fixes.
- **Full Git Sync, Enforced Commit Documentation Policy & Cross-Platform Vault**:
  - Explicit `fetch`, `pull`, `push`, `commit`, and `sync` workflows via `git2`.
  - Enforced `[git.commit_policy]` formats commits as `[<ITEM_ID>](<scope>) <summary>` and automatically records the commit SHA, date, changed files, and summary into `item.toml` (`history.previous_fixes`) and `desc.md` (`## Fix Log & Agent Notes`).
  - Cross-platform `SecureStorage` stores HTTPS PATs and SSH Ed25519 keys in the native OS keyring (`linux-native`, `apple-native`, `windows-native`) with an authenticated **AES-256-GCM encrypted vault fallback** (`vault.enc`) for headless CI runners and containers.

---

## Workspace Crates

| Crate | Binary / Library | Description |
| :--- | :--- | :--- |
| [`crates/backend_core`](file:///home/theinfomoose/Documents/Projects/Nebulous/crates/backend_core) | `backend_core` (lib) | Core 4-tier TOML/Markdown storage, atomic persistence, multi-project registry, `.gitignore`-aware directory ingestion, `nebulous.component.toml` pinning, workflow lifecycle, timer/metrics engine, agent context generator, `SecureStorage` + SSH `KeyPairManager`, `git2` sync, MCP server (`mcp`), and `notify` watcher. |
| [`crates/nebulous_cli`](file:///home/theinfomoose/Documents/Projects/Nebulous/crates/nebulous_cli) | `neb` (bin) | Fast companion CLI and `neb mcp` stdio server with human-readable and `--json` output modes for terminal, CI, and AI agent workflows. |
| [`crates/cosmic_interface`](file:///home/theinfomoose/Documents/Projects/Nebulous/crates/cosmic_interface) | `nebulous` (bin) | Native desktop GUI built on System76's `libcosmic` featuring a Project Switcher, 4-column Kanban board, lazy-loaded Context Drawer, Metrics & Hotspots dashboard, dedicated **Phases & Scopes** (ingestion + component pinning) view, and dedicated **Git & Settings** view. |

---

## Installation & Packaging

### Option 1: Turnkey Linux Installer (`scripts/install.sh`)

Builds `nebulous` and `neb` in release mode and installs the binaries, scalable SVG icon, AppStream metadata, and `.desktop` launcher:

```bash
# Install to ~/.local/bin and ~/.local/share (default --user mode)
./scripts/install.sh --user

# Or install system-wide to /usr/local
sudo ./scripts/install.sh --system

# Uninstall
./scripts/install.sh --uninstall
```

### Option 2: Portable AppImage & AppDir Bundle (`scripts/build-appimage.sh`)

Packages both `nebulous` (GUI) and `neb` (CLI + MCP server) into a self-contained `Nebulous.AppDir` (and builds `Nebulous-x86_64.AppImage` when `appimagetool` is in `PATH`):

```bash
./scripts/build-appimage.sh

# Launch GUI via AppImage / AppRun
./target/appimage/Nebulous.AppDir/AppRun

# Invoke the bundled CLI or MCP server via --cli or mcp
./target/appimage/Nebulous.AppDir/AppRun --cli list
./target/appimage/Nebulous.AppDir/AppRun mcp
```

### Option 3: Flatpak Package (`packaging/flatpak/com.nebulous.Tracker.yml`)

The Flatpak manifest is pre-configured with `--filesystem=host`, `--share=network`, `--socket=ssh-auth`, and `--talk-name=org.freedesktop.secrets` so Git remotes, SSH agents, OS keyring credentials, and host workspaces work seamlessly inside the sandbox:

```bash
flatpak-builder --user --install --force-clean build-flatpak packaging/flatpak/com.nebulous.Tracker.yml
flatpak run com.nebulous.Tracker
```

### Option 4: Cross-Platform Build from Source (Linux, macOS, Windows)

```bash
# Ubuntu / Debian / Pop!_OS build dependencies
sudo apt-get update
sudo apt-get install -y \
  build-essential pkg-config cmake clang \
  libssl-dev libdbus-1-dev \
  libwayland-dev libxkbcommon-dev libx11-dev libxi-dev libgl1-mesa-dev \
  libfontconfig1-dev libfreetype6-dev libexpat1-dev

# Build both `neb` (CLI + MCP server) and `nebulous` (GUI)
cargo build --workspace --release
```

On **macOS** (`apple-native` Keychain) and **Windows** (`windows-native` Credential Manager), `backend_core` and `neb` compile natively with full path normalization (`/`) and Windows-safe atomic file replacement:

```bash
cargo build -p nebulous_cli --release
```

---

## Quick Start

### 1. Initialize, Register & Ingest a Project

```bash
# Initialize .tracker/ in the current repository
neb init --name "MyProject" --phase-id 01_foundation --phase-title "Foundation & Core"

# Register in the global multi-project registry (~/.config/nebulous/projects.toml)
neb project add . --name "MyProject" --tags rust,linux

# Scan the project directory, generate nebulous.component.toml manifests, and promote to scopes
neb ingest --write-manifests --promote-all

# Capture a bug and a feature
neb bug "Fix empty array deserialization" --scope backend_core --severity high
neb feat "Implement bounded agent prompt export" --scope backend_core --severity medium

# List items from the fast top-level index
neb list
```

### 2. Policy-Validated Documented Git Commits

```bash
# Check repository status, ahead/behind counts, and dirty .tracker/ or workspace files
neb git status

# Create a self-documenting commit linked to BUG-0001 (automatically updates item.toml & desc.md)
neb git commit -i BUG-0001 -m "Handle empty array deserialization in TOML parser" --all

# Fetch, pull (with automatic .tracker/ index reconciliation), and push
neb git fetch
neb git pull
neb git push
```

### 3. Configure `neb mcp` for AI Coding Agents

Run `neb mcp` as a JSON-RPC 2.0 `stdio` server in **Cursor** (`.cursor/mcp.json`), **Claude Code** (`.mcp.json`), **Codex**, or local **Ollama** agent harnesses:

```json
{
  "mcpServers": {
    "nebulous": {
      "command": "neb",
      "args": ["mcp", "--root", "/abs/path/to/your/repo"]
    }
  }
}
```

### 4. Launch the Native COSMIC GUI

```bash
# Launch the desktop GUI against the current or active registered project
nebulous

# Or run the headless self-test in CI / terminal environments without a display server
nebulous --self-test
```

---

## Documentation

- [**Architecture & 4-Tier Schema Reference (`docs/ARCHITECTURE.md`)**](docs/ARCHITECTURE.md) — Detailed breakdown of the 4-tier `.tracker/` hierarchy, `projects.toml` registry, `nebulous.component.toml` rolling-release pinning schema, `[git.commit_policy]`, `SecureStorage` vault, and `neb mcp` architecture.
- [**CLI, MCP & GUI Usage Guide (`docs/USAGE.md`)**](docs/USAGE.md) — Complete reference for all `neb` CLI subcommands, MCP server configurations (Cursor, Claude Code, Codex, Ollama, and NovOS QEMU CI pipelines), packaging scripts, and `nebulous` COSMIC GUI views.

---

## Verification & Testing

Run the full workspace verification suite:

```bash
bash -n scripts/install.sh scripts/build-appimage.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p cosmic_interface -- --self-test
```

---

## License

Released under the [MIT License](LICENSE).
