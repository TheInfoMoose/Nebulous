# Nebulous

[![CI](https://github.com/theinfomoose/Nebulous/actions/workflows/ci.yml/badge.svg)](https://github.com/theinfomoose/Nebulous/actions/workflows/ci.yml)
[![Release](https://github.com/theinfomoose/Nebulous/actions/workflows/release.yml/badge.svg)](https://github.com/theinfomoose/Nebulous/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**Nebulous** is a CLI-first, local-first, Git-backed 4-tier project, bug, and release-engineering tracker built in Rust for developers, rolling-release OS builders (such as **NovOS**), and AI-assisted coding workflows. It combines an ergonomic zero-ID command-line interface (`neb`), a retro Amber-on-Black CRT phosphor terminal dashboard (`neb` / `neb tui`), and a built-in Model Context Protocol server (`neb mcp`), storing all project state in human-readable TOML manifests and Markdown files inside `.tracker/`.

---

## Key Features

- **Retro Amber-on-Black Phosphor TUI (`neb` / `neb tui`)**:
  - Classic P3 monochrome CRT Amber-on-Black palette (`#000000` obsidian background, `#FFB000` primary amber, `#FFD866` bright gold highlights, `#996300` muted dust/borders, `#FF5500` high-contrast alert orange).
  - Aspect-locked centered ASCII art Nebula cloud + `NEBULOUS` centerpiece header (`5×76` full block-letter banner on wide terminals, `3×60` compact banner on smaller terminals) with wrapping strictly disabled so the header never clips or deforms.
  - Responsive scaling locks (`>= 110` cols 3-pane layout, `64..109` cols 2-pane + bottom inspector drawer, `< 64×20` non-deforming minimum viewport scaling lock guard) and modal `[REQUIRED FIELD LOCK]` validation with horizontal cursor-window scrolling.
- **Canonical 3-Stage Lifecycle & Automatic Cross-Stage Scope Mirroring**:
  - One-command `neb init` scaffolds all 3 canonical stages out of the box:
    1. `pre_production` (`pre` / `1` — Planning & Design)
    2. `production` (`prod` / `2` — Active Implementation)
    3. `post_production` (`post` / `3` — Verification, Polish & Release)
  - Domain scopes (crates, packages, OS build modules) are automatically mirrored across all registered stages and items move cleanly between stages via `neb move <stage>` or the TUI `[m]` modal.
- **Predictive Assist ("What's Next") & Zero-ID Workflow Shortcuts**:
  - Automatically infers the target domain `scope` from your active working directory, modified Git files, or active timer when `--scope` is omitted.
  - Zero-ID commands (`neb start`, `neb done`, `neb show`, `neb context`, `neb move`) automatically target the active timer, sole `in_progress` item, or top-ranked `"What's Next"` recommendation (`neb next`).
- **4-Tier `.tracker/` TOML + Markdown Hierarchy**:
  - **Tier 1 (`project.toml`)**: Global project metadata, monotonic ID counters (`BUG-XXXX`, `FEAT-XXXX`), stage registry, ingested `[components]` catalog, effort/stagnation thresholds, and Git sync + `[git.commit_policy]` configuration.
  - **Tier 2 (`phases/<stage_id>/phase.toml`)**: Stage lifecycle status, registered domain scopes, and roll-up completion counts.
  - **Tier 3 (`.../domains/<scope_id>/scope.toml`)**: Domain/crate boundary metadata (`target`, `tech_stack`, `description`, `bound_component`, `tracked_files`) and lightweight item stubs (`bugs`, `features`) for instant startup indexing.
  - **Tier 4 (`.../items/<ITEM_ID>/item.toml` + `desc.md`)**: Authoritative item metadata, effort tracking (`logged_time_minutes`, `iteration_count`), historical fix ledger (`recurrence_count`, `previous_fixes`), and rich Markdown specification (`desc.md`).
- **Two-Stage Directory Ingestion & Rolling-Release Pinning (`nebulous.component.toml`)**:
  - Walks the project tree respecting `.gitignore`, auto-detects ecosystem markers (`Cargo.toml`, `package.json`, `pyproject.toml`, `CMakeLists.txt`, `Makefile`, `Dockerfile`, or `nebulous.component.toml`), snapshots `last_known_good` (`pinned_release`) configurations, and promotes components directly into domain scopes across all stages.
- **Model Context Protocol Server (`neb mcp`) & Enforced Git Commit Policy**:
  - Built-in JSON-RPC 2.0 `stdio` MCP server exposing 23 tools across Task & Boundary Management, Project & Component Ingestion, NovOS Self-Healing CI/CD, and Policy-Validated Git Operations.
  - Enforced `[git.commit_policy]` formats commits as `[<ITEM_ID>](<scope>) <summary>` and automatically records the commit SHA, date, changed files, and summary into `item.toml` and `desc.md`.

---

## Workspace Crates

| Crate | Binary / Library | Description |
| :--- | :--- | :--- |
| [`crates/backend_core`](crates/backend_core) | `backend_core` (lib) | Core 4-tier TOML/Markdown storage, 3-stage lifecycle & cross-stage scope mirroring, predictive `"What's Next"` engine, `.gitignore`-aware directory ingestion, `nebulous.component.toml` pinning, timer/metrics engine, agent context generator, `SecureStorage` + SSH `KeyPairManager`, `git2` sync, MCP server (`mcp`), and `notify` watcher. |
| [`crates/nebulous_cli`](crates/nebulous_cli) | `neb` (bin) | Companion CLI, retro Amber-on-Black `ratatui` TUI (`neb` / `neb tui`), and `neb mcp` stdio server with human-readable and `--json` output modes. |

---

## Installation

### Option 0: Prebuilt Release Archive (Linux x86_64)

Download `nebulous-<tag>-x86_64-unknown-linux-gnu.tar.gz` and its `.sha256` from the GitHub Releases page, then:

```bash
sha256sum -c nebulous-<tag>-x86_64-unknown-linux-gnu.tar.gz.sha256
tar xzf nebulous-<tag>-x86_64-unknown-linux-gnu.tar.gz
cd nebulous-<tag>-x86_64-unknown-linux-gnu

# Installs the bundled prebuilt `neb` to ~/.local/bin (no Rust toolchain needed)
./install.sh --user
```

### Option 1: Turnkey Installer from a Checkout (`scripts/install.sh`)

Builds and installs the `neb` CLI/TUI binary in release mode:

```bash
# Install to ~/.local/bin (default --user mode)
./scripts/install.sh --user

# Or install system-wide to /usr/local/bin
sudo ./scripts/install.sh --system

# Uninstall
./scripts/install.sh --uninstall
```

### Option 2: Cross-Platform Build from Source (Linux, macOS, Windows)

```bash
cargo build --workspace --release
```

---

## Quick Start

### 1. One-Command Bootstrap & Predictive Capture

`neb` works on the `.tracker/` workspace found in the current directory or its ancestors. Run
`neb init` inside your project first. Launched outside any workspace, the interactive dashboard
(`neb` / `neb tui`) opens the active project registered in `projects.toml`
(`neb project list` / `neb project switch`); every other command requires being inside a
workspace or passing `--root <PATH>`.

```bash
# Scaffolds pre_production, production, post_production, ingests crates into scopes across all 3 stages, and registers the project
neb init

# Capture a bug or feature (scope is inferred automatically from your CWD or dirty Git files!)
neb bug "Fix empty array deserialization" --severity high
neb feat "Implement bounded agent prompt export" --severity medium

# Inspect the ranked "What's Next" queue
neb next
```

### 2. Zero-ID Flow & Stage Transitions

```bash
# Start work on the top recommended item (marks in_progress, starts timer, copies bounded agent prompt)
neb start

# Move the active item from pre_production to production and switch active stage
neb move prod
neb stage prod

# Close the active item and create a policy-validated Git commit [<ID>](<scope>) <summary> in one command
neb done -m "Handle empty array deserialization in TOML parser" -c
```

### 3. Launch the Retro Amber-on-Black TUI

```bash
# Launch the interactive Amber-on-Black terminal dashboard
neb
# or explicitly:
neb tui
```

**TUI Hotkeys**:
- `1` / `2` / `3`: Switch active stage (`Pre-Production` / `Production` / `Post-Production`)
- `Tab` / `Shift+Tab` or `h` / `l`: Cycle pane focus (`Scopes` <-> `Items` <-> `Details`) or modal fields
- `j` / `k` or `↑` / `↓`: Navigate scopes/items or scroll inspector details (`PgUp` / `PgDn` / `J` / `K`)
- `b` / `f`: Open predictive `New Bug` / `New Feature` modal (pre-filled Scope, Stage, and Severity)
- `s`: Start work on selected/recommended item (starts timer + copies bounded agent prompt)
- `d`: Open `Done / Close` modal pre-filled with `"Resolved <ID>: <title>"` and Git `HEAD` SHA
- `m`: Open 1-key stage move modal (`1` Pre-Production, `2` Production, `3` Post-Production)
- `c`: Copy bounded agent context | `i`: Ingest & promote workspace components | `n`: Jump to `"What's Next"` item | `/`: Filter items | `q` / `Esc`: Quit / Cancel

### 4. Configure `neb mcp` for AI Coding Agents

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

---

## Documentation

- [**Architecture & 4-Tier Schema Reference (`docs/ARCHITECTURE.md`)**](docs/ARCHITECTURE.md) — Detailed breakdown of the 4-tier `.tracker/` hierarchy, 3-stage lifecycle, predictive assist engine, retro Amber-on-Black TUI scaling locks, `nebulous.component.toml` rolling-release pinning schema, `[git.commit_policy]`, and `neb mcp` architecture.
- [**CLI, TUI & MCP Usage Guide (`docs/USAGE.md`)**](docs/USAGE.md) — Complete reference for all `neb` CLI subcommands, zero-ID shortcuts, Amber-on-Black TUI workflows, and MCP server configurations.

---

## Verification & Testing

Run the full workspace verification suite:

```bash
bash -n scripts/install.sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

---

## License

Released under the [MIT License](LICENSE).
