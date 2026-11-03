# Nebulous

[![CI](https://github.com/theinfomoose/Nebulous/actions/workflows/ci.yml/badge.svg)](https://github.com/theinfomoose/Nebulous/actions/workflows/ci.yml)
[![Release](https://github.com/theinfomoose/Nebulous/actions/workflows/release.yml/badge.svg)](https://github.com/theinfomoose/Nebulous/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**Nebulous** is a local-first, Git-backed, 4-tier project and bug tracker built in Rust for Linux developers and AI-assisted coding workflows. It pairs a fast companion CLI (`neb`) with a native `libcosmic` desktop GUI (`nebulous`), storing all project state in human-readable TOML manifests and Markdown files inside `.tracker/`.

---

## Key Features

- **4-Tier `.tracker/` TOML + Markdown Hierarchy**:
  - **Tier 1 (`project.toml`)**: Global project metadata, monotonic ID counters (`BUG-XXXX`, `FEAT-XXXX`), phase registry, effort/stagnation thresholds, and Git sync configuration.
  - **Tier 2 (`phases/<phase_id>/phase.toml`)**: Phase lifecycle status, registered domain scopes, and roll-up completion counts.
  - **Tier 3 (`.../domains/<scope_id>/scope.toml`)**: Domain/crate boundary metadata (`target`, `tech_stack`, `description`) and lightweight item stubs (`bugs`, `features`) for instant startup indexing.
  - **Tier 4 (`.../items/<ITEM_ID>/item.toml` + `desc.md`)**: Authoritative item metadata, effort tracking (`logged_time_minutes`, `iteration_count`), historical fix ledger (`recurrence_count`, `previous_fixes`), and rich Markdown specification (`desc.md`).
- **Instant Index-Only Boot & Lazy Detail Loading**:
  - Both `neb list` and the `nebulous` GUI read only Tier 1–3 index manifests on startup, deferring Tier 4 `item.toml` + `desc.md` reads until an item is inspected or selected.
- **Anti-Looping & Regression Prevention for AI Agents**:
  - Tracks `recurrence_count` and a structured `previous_fixes` ledger (`date`, `commit`, `summary`) whenever items are closed and reopened.
  - One-command and one-click **Bounded Agent Prompt** and `@file` reference export (`neb context <ID> [--refs-only] [--copy]`) scopes AI coding assistants to the exact target crate and warns against repeating failed fixes.
- **Effort Valuation & Hotspot Analytics**:
  - Built-in session timer (`.tracker/.timer.toml`), manual minute logging, target-day equivalents (`target_daily_hours`), stagnant item detection (`stagnant_item_threshold_days`), and recurring bug hotspot ranking.
- **Dual Interface (`neb` CLI + `nebulous` COSMIC GUI)**:
  - Live filesystem synchronization via `notify` (`TrackerWatcher`) ensures CLI mutations immediately update the running GUI without manual refreshes.
- **Git & OS Keyring Integration**:
  - Stage, commit, pull, and push `.tracker/` state via `git2`, with automatic index conflict reconciliation and HTTPS Personal Access Token (PAT) storage in the Linux Secret Service keyring (`keyring` crate).

---

## Workspace Crates

| Crate | Binary / Library | Description |
| :--- | :--- | :--- |
| `crates/backend_core` | `backend_core` (lib) | Core 4-tier TOML/Markdown storage, atomic persistence, index reconciliation, workflow lifecycle, timer/metrics engine, agent context generator, `git2` sync, and `notify` watcher. |
| `crates/nebulous_cli` | `neb` (bin) | Fast companion CLI with human-readable and `--json` output modes for terminal and agent workflows. |
| `crates/cosmic_interface` | `nebulous` (bin) | Native Linux desktop GUI built on System76's `libcosmic` (Iced/Smithay) featuring a 4-column Kanban board, lazy-loaded Context Drawer, Metrics & Hotspots dashboard, and Phases/Scopes/Git settings. |

---

## Quick Start

### 1. Build from Source

Ensure you have a recent stable Rust toolchain (`cargo`, `rustc`) and standard system dependencies for `libcosmic` / Wayland / X11 and `libdbus-1-dev` (for Secret Service keyring support) on Linux:

```bash
# Ubuntu / Debian / Pop!_OS build dependencies
sudo apt-get update
sudo apt-get install -y \
  build-essential pkg-config cmake clang \
  libssl-dev libdbus-1-dev \
  libwayland-dev libxkbcommon-dev libx11-dev libxi-dev libgl1-mesa-dev \
  libfontconfig1-dev libfreetype6-dev libexpat1-dev

# Build both `neb` (CLI) and `nebulous` (GUI)
cargo build --workspace --release
```

Compiled binaries are placed in `target/release/neb` and `target/release/nebulous`.

### 2. Initialize a `.tracker/` Workspace

```bash
# Initialize .tracker/ in the current repository
neb init --name "MyProject" --phase-id 01_foundation --phase-title "Foundation & Core"

# Register a domain scope mapped to a crate or directory
neb scope add backend_core --target crates/backend_core --stack rust,serde,toml --desc "Core persistence engine"

# Capture a bug and a feature
neb bug "Fix empty array deserialization" --scope backend_core --severity high
neb feat "Implement bounded agent prompt export" --scope backend_core --severity medium

# List items from the fast top-level index
neb list
```

### 3. Launch the Native COSMIC GUI

```bash
# Launch the desktop GUI against the current workspace
nebulous

# Or run the headless self-test in CI / terminal environments without a display server
nebulous --self-test
```

---

## Documentation

- [**Architecture & 4-Tier Schema Reference (`docs/ARCHITECTURE.md`)**](docs/ARCHITECTURE.md) — Detailed breakdown of the 4-tier `.tracker/` filesystem hierarchy, TOML/Markdown schemas, atomic write guarantees, index reconciliation, and crate architecture.
- [**CLI & GUI Usage Guide (`docs/USAGE.md`)**](docs/USAGE.md) — Complete reference for all `neb` CLI commands/flags, `--json` scripting, AI agent context handoff workflows, and `nebulous` COSMIC GUI navigation.

---

## Verification & Testing

Run the full workspace verification suite:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p cosmic_interface -- --self-test
```

---

## License

Released under the [MIT License](LICENSE).
