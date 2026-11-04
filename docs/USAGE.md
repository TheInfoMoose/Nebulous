# Nebulous Usage Guide (`neb` CLI, `neb mcp` Server & `nebulous` COSMIC GUI)

Nebulous provides three synchronized interfaces that operate on the same `.tracker/` directory and global project registry:
- **`neb`**: Fast companion CLI for terminal workflows, release scripts, and CI pipelines.
- **`neb mcp`**: Built-in JSON-RPC 2.0 `stdio` Model Context Protocol server for Cursor, Claude Code, Codex, Ollama, and automated self-healing CI/CD pipelines.
- **`nebulous`**: Native `libcosmic` desktop GUI for multi-project switching, visual Kanban management, directory ingestion, component pinning, and documented Git commits.

---

## Part 1: Companion CLI (`neb`)

### Global Flags

Every `neb` command accepts these global options:
- `--root <PATH>`: Path to the workspace root or `.tracker/` directory. If omitted, `neb` walks up ancestor directories from the current working directory to discover `.tracker/project.toml`, or falls back to the active project in `projects.toml`.
- `--json`: Emit machine-readable JSON output instead of human-formatted tables/text.

---

### 1. Global Multi-Project Registry (`neb project`)

 Nebulous tracks registered workspaces in `<config_dir>/nebulous/projects.toml` (`~/.config/nebulous/projects.toml` on Linux, `~/Library/Application Support/nebulous/projects.toml` on macOS, `%APPDATA%\nebulous\projects.toml` on Windows; overridable via `NEBULOUS_CONFIG_DIR`).

```bash
# List all registered projects and see which project is active (*)
neb project list

# Register a project directory (initializes .tracker/ if needed and sets it active)
neb project add /path/to/repo --name "NovOS" --tags os,debian-sid,xanmod

# Switch the active project by ID, name, or path
neb project switch "NovOS"

# Remove a project from the global registry (does not delete its .tracker/ directory)
neb project remove "NovOS"
```

---

### 2. Two-Stage Directory Ingestion & Component Pinning (`neb ingest` & `neb component`)

#### Stage 1: Scan & Ingest Project Directory (`neb ingest`)
Walks the project directory respecting `.gitignore` (skipping `target/`, `node_modules/`, `.git/`, `.tracker/`, `dist/`, `build/`, `__pycache__/`, `.venv/`), detects ecosystem manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, `CMakeLists.txt`, `Makefile`, `Dockerfile`, `nebulous.component.toml`), and records relative file lists (`files = [...]`) into `.tracker/project.toml` `[components]`.

```bash
# Scan the repository and register discovered components in .tracker/project.toml
neb ingest

# Scan a specific subdirectory, write nebulous.component.toml files, and promote all to scopes
neb ingest --path crates --write-manifests --promote-all
```

#### Stage 2: Manage `nebulous.component.toml` & Rolling-Release Pins (`neb component`)

Ideal for both multi-crate repositories and rolling-release OS builds (such as **NovOS** tracking Debian Sid packages and XanMod kernels):

```bash
# List all ingested components in the current project
neb component list

# Inspect a component's files, dependencies, and pinned_release snapshot
neb component show iso

# Initialize or update a nebulous.component.toml in an un-manifested directory
neb component init iso \
  --id novos_iso \
  --name "NovOS Live ISO" \
  --type os_image \
  --stack debian-sid,xanmod,live-build \
  --tags boot,iso \
  --flags "--arch=amd64,--flavour=xanmod"

# Pin the current known-good Git commit, build script, flags, and package versions
neb component pin novos_iso \
  --script scripts/build_iso.sh \
  --flags "--arch=amd64,--flavour=xanmod" \
  --dep linux-xanmod-x64v3=6.11.6-x64v3-xanmod1,systemd=256.7-1

# Roll back to the last_known_good pin when an upstream Sid/XanMod update breaks the build
neb component rollback novos_iso \
  --item BUG-0001 \
  --reason "Kernel 6.12-rc1 broke initramfs ZFS module compile in QEMU check"

# Promote an ingested component into a Tier 3 Phase Domain Scope (scope.toml)
neb component promote novos_iso --phase 01_core_engine --scope novos_iso
```

---

### 3. Workspace Initialization, Phases & Scopes

```bash
# Initialize .tracker/ in a directory
neb init --name "Nebulous" --phase-id 01_core_engine --phase-title "Core Engine & Initial Scope"

# Reconcile derived Tier 1-3 indexes from authoritative Tier 4 item.toml files
neb sync-index

# Manage phases
neb phase list
neb phase add 02_git_and_polish --title "Git Sync & Polish" --use
neb phase use 02_git_and_polish

# Manage domain scopes
neb scope list
neb scope add backend_core --target crates/backend_core --stack rust,serde,toml --desc "Core engine"
```

---

### 4. Capturing, Inspecting & Transitioning Items

```bash
# Capture a bug or feature
neb bug "TOML parser panics on unquoted local datetime" --scope backend_core --severity high
neb feat "Add keyboard shortcut for Quick-Add bar" --scope cosmic_interface --severity medium

# Fast index-backed listing with filters
neb list
neb list --scope backend_core --status open
neb list --stagnant
neb list --recurring

# Inspect full item.toml + desc.md details
neb show BUG-0001

# Transition status, close with fix summary, or reopen (incrementing recurrence_count)
neb status BUG-0001 in_progress
neb close BUG-0001 -m "Added custom serde deserializer for unquoted TOML datetimes"
neb reopen BUG-0001 --reason "Edge case still triggers when time offset is negative"
```

---

### 5. Effort Tracking & AI Agent Context Export

```bash
# Session timer (.tracker/.timer.toml)
neb timer start BUG-0001 --note "Debugging datetime parser"
neb timer status
neb timer stop

# Manual minute logging
neb log BUG-0001 -m 30

# Export Bounded Agent Prompt or compact @file references
neb context BUG-0001
neb context BUG-0001 --refs-only
neb context BUG-0001 --copy

# Compute project effort valuation, stagnant items, and recurring bug hotspots
neb metrics
```

---

### 6. Git Operations & Enforced Commit Documentation Policy (`neb git` & `neb sync`)

Nebulous enforces `[git.commit_policy]` on workspace commits, formatting commit messages as `[<ITEM_ID>](<scope>) <summary>` and automatically appending the commit SHA, date, changed files, and summary to the item's `item.toml` (`history.previous_fixes`) and `desc.md` (`## Fix Log & Agent Notes`).

```bash
# Inspect branch, remote URL, ahead/behind counts, and dirty .tracker/ or workspace files
neb git status

# Explicit remote operations (uses SecureStorage PAT / SSH Ed25519 key / SSH agent)
neb git fetch
neb git pull
neb git push

# Create a policy-validated, self-documenting commit linked to BUG-0001
neb git commit -i BUG-0001 -m "Fix unquoted TOML datetime deserialization" --all

# Commit specific workspace paths alongside .tracker/
neb git commit -i FEAT-0001 -m "Add MCP stdio server loop" --paths crates/backend_core/src/mcp/mod.rs

# Commit only .tracker/ metadata changes (auto-generates [TRACKER](sync) summary if -m is omitted)
neb git commit --tracker-only -m "Triage phase 01 backlog"

# Full one-command sync (stage/commit + pull + reconcile + push)
neb sync -i BUG-0001 -m "Fix unquoted TOML datetime deserialization" --all
```

---

## Part 2: Model Context Protocol Server (`neb mcp`)

`neb mcp` runs a standard JSON-RPC 2.0 MCP server over `stdin`/`stdout` (supporting both newline-delimited JSON-RPC and `Content-Length` headers).

### Exposed MCP Tools (21 Tools)

1. **Task & Boundary Tools**:
   - `get_active_task`: Returns the currently timed or `in_progress` item with its full bounded context.
   - `list_items`: Lists tracked items with optional `phase_id`, `scope_id`, `status`, `item_type`, `stagnant_only`, and `recurring_only` filters.
   - `get_item_context`: Returns the full `ItemDetail`, `@file` references, and Bounded Agent Prompt for an item.
   - `list_crate_boundaries`: Returns all domain scopes in a phase along with their `target_path`, `tech_stack`, `bound_component`, `tracked_files`, and `pinned_release`.
   - `create_item`: Creates a new `BUG-XXXX` or `FEAT-XXXX` item.
   - `update_item_status`: Transitions an item's status.
   - `close_item_with_fix`: Closes or merges an item and records the fix in `previous_fixes`.
   - `record_regression`: Reopens a closed item (or logs a failed attempt), incrementing `recurrence_count` and appending failure details to `desc.md`.
2. **Project & Ingestion Tools**:
   - `list_projects`: Lists all projects in the global `projects.toml` registry.
   - `switch_project`: Switches the active project (or registers a new project directory).
   - `ingest_project_directory`: Runs the `.gitignore`-aware directory scanner and populates `[components]`.
   - `upsert_component_manifest`: Creates or updates a `nebulous.component.toml` manifest (dependencies, flags, tags).
   - `promote_component_to_scope`: Promotes an ingested component into a Tier 3 Phase Domain Scope (`scope.toml`).
3. **NovOS Release Engineering & Self-Healing CI Tools**:
   - `record_rolling_update`: Logs an upstream rolling package delta (e.g. Debian Sid or XanMod kernel bump) onto a component and creates/updates a tracking work item.
   - `record_verification_run`: Records QEMU/container test execution results (`passed`, `exit_code`, `command`, `summary`, `logs_excerpt`) onto the component and linked item.
   - `pin_stable_release`: Snapshots the current Git commit SHA, dependency versions, and build flags into `nebulous.component.toml` and `project.toml` as `last_known_good`.
   - `rollback_to_pinned_release`: Restores `nebulous.component.toml` to the `last_known_good` pin and records a regression on the linked item when verification fails.
4. **Git & Documentation Tools**:
   - `git_status`, `git_fetch`, `git_pull`, `git_push`, `git_commit_documented`, `git_sync`.

### Configuration Snippets

#### Cursor (`.cursor/mcp.json`) & Claude Code (`.mcp.json`)
```json
{
  "mcpServers": {
    "nebulous": {
      "command": "neb",
      "args": ["mcp", "--root", "/abs/path/to/workspace"]
    }
  }
}
```

#### OpenAI Codex CLI (`~/.codex/config.toml`)
```toml
[mcp_servers.nebulous]
command = "neb"
args = ["mcp", "--root", "/abs/path/to/workspace"]
```

#### Local Ollama / Open-Source Agent Bridge
Point any MCP-compatible Ollama client (such as `mcp-cli`, `ollmcp`, or custom Python/Rust harnesses) at `neb mcp`:
```json
{
  "servers": {
    "nebulous": {
      "transport": "stdio",
      "command": "neb",
      "args": ["mcp"]
    }
  }
}
```

#### NovOS Self-Healing QEMU CI Pipeline Example
```bash
# 1. Ingest ISO build component and record upstream Sid/XanMod package update via MCP or CLI
neb ingest --write-manifests
neb --json component pin novos_iso --script scripts/build_iso.sh --dep linux-xanmod=6.11.6

# 2. If a subsequent rolling update fails QEMU boot verification, automatically roll back:
if ! ./scripts/run-qemu-smoke-test.sh; then
  neb --json component rollback novos_iso \
    --item BUG-0001 \
    --reason "QEMU smoke test failed after rolling Sid update; restored last_known_good pin"
fi
```

---

## Part 3: Native COSMIC GUI (`nebulous`)

### Launching the GUI

```bash
# Open the workspace in the current directory (or active project in projects.toml)
nebulous

# Open a specific workspace path
nebulous --workspace /path/to/project

# Run the headless verification self-test (no display server required)
nebulous --self-test
```

### GUI Views & Workflows

1. **Top Header Bar**:
   - **Project Switcher Dropdown & `+ Project` Button**: Switch between registered projects in `projects.toml` or register/open a new repository directory directly from the header bar.
   - **Phase Selector Dropdown**: Switch between project phases and view live phase completion ratios and logged effort hours.
   - **Quick-Add Bar (`+ Bug` / `+ Feature`)**: Capture a new bug or feature inline with severity selection.
   - **Filter Pills & Active Session Timer Pill**: Filter cards (`All`, `Bugs`, `Features`, `High/Crit`, `Stagnant`, `Recurring`) and control the live timer.
   - **Git Status & Sync Bar**: Displays `HEAD` short SHA, ahead/behind counts (`↑N ↓M`), dirty file counts, `Fetch`, `Commit .tracker`, `Git Sync`, and `Reconcile`.
2. **Kanban Board View**:
   - 4-column board (`OPEN`, `IN PROGRESS`, `PENDING`, `CLOSED / MERGED`) with lazy-loaded right-hand **Context Drawer** for editing `desc.md`, logging effort, closing/reopening items, and copying Bounded Agent Prompts.
3. **Metrics & Hotspots View**:
   - Visualizes effort valuation (`target_daily_hours` equivalents), phase/scope effort breakdowns, stagnant items, and recurring bug hotspots.
4. **Dedicated `Phases & Scopes` View**:
   - **Project Directory Ingestion & Component Catalog**: Click **Scan Project Directory** (`neb ingest`) to discover components, inspect file counts and tech stacks, write `nebulous.component.toml`, or click **Promote to Scope** with 1 click.
   - **Component Manifest & Rolling-Release Pin Builder**: Edit component metadata, dependencies (`name=version`), build script path, and build flags, with 1-click **Pin Known-Good Release** and **Rollback to Pin** actions.
   - **Phase & Domain Scope Managers**: Create/activate phases and inspect or create domain scopes (`bound_component`, `tracked_files`).
5. **Dedicated `Git & Settings` View**:
   - **Multi-Project Registry Manager**: Add, switch, or remove projects in `projects.toml`.
   - **Git Operations & Documented Commit Builder**: Run `Fetch`, `Pull`, `Push`, `Sync All`, or build a policy-validated commit linked to an Item ID (`Stage .tracker/ only` vs. `Stage Workspace + .tracker/`).
   - **Commit Policy & Remote Settings**: Toggle `require_item_id`, `require_scope`, `auto_append_to_desc`, `min_summary_length`, remote/branch, and sync mode.
   - **Cross-Platform `SecureStorage` Vault**: Store/delete HTTPS PATs and generate or import SSH Ed25519 keys backed by OS Keyring + AES-256-GCM `vault.enc`.

---

## Part 4: Linux Packaging & Installation

- **User or System Install**: `./scripts/install.sh --user` (installs to `~/.local`) or `sudo ./scripts/install.sh --system` (installs to `/usr/local`). Remove with `./scripts/install.sh --uninstall`.
- **Portable AppImage / AppDir**: `./scripts/build-appimage.sh` creates `target/appimage/Nebulous.AppDir` and `target/appimage/Nebulous-x86_64-AppDir.tar.gz` (plus `Nebulous-x86_64.AppImage` if `appimagetool` is installed).
- **Flatpak**: `flatpak-builder --user --install --force-clean build-flatpak packaging/flatpak/com.nebulous.Tracker.yml`.
