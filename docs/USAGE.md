# Nebulous Usage Guide (`neb` CLI & `nebulous` COSMIC GUI)

Nebulous provides two synchronized interfaces that operate on the same `.tracker/` directory:
- **`neb`**: Fast companion CLI for terminal workflows, scripts, and AI coding agents.
- **`nebulous`**: Native `libcosmic` desktop GUI for visual Kanban management, live timers, markdown editing, and hotspot analytics.

---

## Part 1: Companion CLI (`neb`)

### Global Flags

Every `neb` command accepts these global options:
- `--root <PATH>`: Path to the workspace root or `.tracker/` directory. If omitted, `neb` walks up ancestor directories from the current working directory to discover `.tracker/project.toml`.
- `--json`: Emit machine-readable JSON output instead of human-formatted tables/text.

---

### 1. Workspace Initialization & Index Reconciliation

#### `neb init`
Initialize a new `.tracker/` hierarchy with an initial phase and default `inbox` scope.

```bash
neb init [--name <NAME>] [--phase-id <PHASE_ID>] [--phase-title <TITLE>]
```

Examples:
```bash
neb init
neb init --name "Nebulous" --phase-id 01_core_engine --phase-title "Core Engine & Initial Scope"
```

#### `neb sync-index`
Reconcile all derived index manifests (`project.toml`, `phase.toml`, `scope.toml`) from the authoritative `items/*/item.toml` files on disk. Useful after manual file edits or external scripts.

```bash
neb sync-index
neb --json sync-index
```

---

### 2. Managing Phases & Domain Scopes

#### `neb phase`
List, create, or switch the active project phase (`project.active_phase`).

```bash
# List all registered phases and their completion summaries
neb phase list

# Create a new phase and optionally set it as active (--use / --active)
neb phase add 02_git_and_polish --title "Git Sync & Polish" --use

# Switch the active phase
neb phase use 02_git_and_polish
```

#### `neb scope`
List or register domain scopes within a phase. Each scope maps to a specific crate or directory boundary (`target`) and technology stack (`tech_stack`).

```bash
# List scopes in the active phase (or specify --phase <PHASE_ID>)
neb scope list
neb scope list --phase 01_core_engine

# Register a new domain scope
neb scope add backend_core \
  --target crates/backend_core \
  --stack rust,serde,toml \
  --desc "Core 4-tier persistence and workflow engine"
```

---

### 3. Capturing & Inspecting Items

#### `neb bug` & `neb feat` (alias: `neb feature`)
Quick-capture a new bug (`BUG-XXXX`) or feature (`FEAT-XXXX`). When `--scope` is omitted, items are routed to `project.default_scope` (`inbox` by default). When `--phase` is omitted, items are created in `project.active_phase`.

```bash
# Capture a high-severity bug in backend_core
neb bug "TOML parser panics on unquoted local datetime" \
  --scope backend_core \
  --severity high \
  --desc "Occurs when parsing legacy config files without a Z suffix."

# Capture a feature in cosmic_interface
neb feat "Add keyboard shortcut for Quick-Add bar" \
  --scope cosmic_interface \
  --severity medium
```

Allowed `--severity` values: `low`, `medium` (default), `high`, `critical`.

#### `neb list`
Fast, index-backed listing of tracked items with optional filters. Reads only Tier 1–3 index files unless `--stagnant` or `--recurring` filters are requested.

```bash
# List all items across the workspace
neb list

# Filter by scope, phase, status, or item type
neb list --scope backend_core --status open
neb list --phase 01_core_engine --type bug

# Filter to stagnant items (exceeding stagnant_item_threshold_days) or recurring bugs
neb list --stagnant
neb list --recurring

# Machine-readable JSON array output
neb --json list --scope backend_core
```

#### `neb show`
Lazily loads and prints the full `item.toml` metadata, effort stats, historical `previous_fixes` ledger, and `desc.md` Markdown body for an item.

```bash
neb show BUG-0001
neb --json show BUG-0001
```

---

### 4. Item Lifecycle, Closing & Reopening

#### `neb status`
Update an item's status (`open`, `in_progress`, `pending`, `closed`, `merged`).

```bash
neb status BUG-0001 in_progress
neb status FEAT-0001 pending
```

#### `neb close`
Close (or mark as `merged` with `--merged`) an item and append a structured entry to `history.previous_fixes` containing today's date, the Git `HEAD` short SHA (or `--commit <SHA>`), and the fix summary.

```bash
# Close with a fix summary (auto-detects Git HEAD short SHA)
neb close BUG-0001 -m "Added custom serde deserializer for unquoted TOML datetimes"

# Mark a feature as merged with an explicit commit SHA
neb close FEAT-0001 -m "Implemented bounded prompt generator" --commit a4f89c1 --merged
```

#### `neb reopen`
Reopen a `closed` or `merged` item back to `open`. Automatically increments `history.recurrence_count` and `effort.iteration_count`, and optionally appends a dated reopen note to `desc.md`.

```bash
neb reopen BUG-0001 --reason "Edge case still triggers when time offset is negative"
```

---

### 5. Effort Tracking & Session Timer

#### `neb timer`
Manage the persistent session timer stored at `.tracker/.timer.toml`.

```bash
# Start timing an item (auto-stops and logs any previously running timer)
neb timer start BUG-0001 --note "Debugging datetime parser"

# Check active timer status and elapsed minutes
neb timer status
neb timer

# Stop the active timer and add elapsed minutes to the item's logged_time_minutes
neb timer stop
```

#### `neb log`
Manually add effort minutes to an item's `effort.logged_time_minutes`.

```bash
neb log BUG-0001 -m 30
# Or positional minutes:
neb log BUG-0001 45
```

---

### 6. AI Agent Context Export (`neb context`)

Generates context handoff payloads for AI coding assistants:

```bash
# Print the full Bounded Agent Prompt (domain boundary + previous fixes warning + desc.md)
neb context BUG-0001

# Print only compact @file references (@.tracker/.../scope.toml @.tracker/.../item.toml @.tracker/.../desc.md)
neb context BUG-0001 --refs-only

# Copy the Bounded Agent Prompt (or @file refs) directly to the system clipboard
neb context BUG-0001 --copy
neb context BUG-0001 --refs-only --copy
```

---

### 7. Project Metrics & Git Synchronization

#### `neb metrics`
Compute and display project time valuation (`target_daily_hours` equivalent), per-phase and per-scope effort breakdowns, stagnant items, and recurring bug hotspots.

```bash
neb metrics
neb --json metrics
```

#### `neb sync`
Stage and commit `.tracker/` changes and synchronize (pull + index reconciliation + push) with the configured Git remote (`project.toml` `[git]`).

```bash
# Commit .tracker/ changes locally without pushing/pulling remote
neb sync --commit-only -m "chore(tracker): triage phase 01 bugs"

# Full sync (commit + pull + reconcile + push) using OS keyring PAT or explicit --pat
neb sync -m "chore(tracker): update sprint progress"
neb sync --pat "$GITHUB_PAT"
```

---

## Part 2: Native COSMIC GUI (`nebulous`)

### Launching the GUI

```bash
# Open the workspace in the current directory (initializes .tracker/ if none exists)
nebulous

# Open a specific workspace path
nebulous --workspace /path/to/project

# Run the headless verification self-test (no display server required)
nebulous --self-test
```

### GUI Layout & Workflows

1. **Top Header Bar**:
   - **Phase Selector Dropdown**: Switch between project phases (`01_core_engine`, etc.) and view live phase completion ratios (`completed / total`) and logged effort hours.
   - **Quick-Add Bar (`+ Bug` / `+ Feature`)**: Capture a new bug or feature inline with severity selection without leaving the board.
   - **Filter Pills**: Narrow visible Kanban cards by `All`, `Bugs`, `Features`, `High/Crit`, `Stagnant`, or `Recurring`, with live badge counts.
   - **Active Session Timer Pill**: Displays the currently timed item ID and elapsed minutes, with a one-click `Stop` button.
   - **Git Status & Sync Controls**: Shows current `HEAD` short SHA, dirty `.tracker/` status, `Commit .tracker`, `Git Sync`, and `Reconcile`.

2. **Left Navigation Sidebar**:
   - Switch between the four primary views:
     - **Kanban Board**: 4-column board (`OPEN`, `IN PROGRESS`, `PENDING`, `CLOSED / MERGED`).
     - **Metrics & Hotspots**: Visual dashboard for effort valuation, target-day equivalents, stagnant item alerts, and recurring bug hotspots.
     - **Phases & Scopes**: Create and inspect project phases and domain scopes (`target`, `tech_stack`, `description`), or click `Open on Board` to filter directly to a scope.
     - **Settings & Git**: Configure `target_daily_hours`, `stagnant_item_threshold_days`, `default_scope`, Git sync mode (`manual` vs. `scheduled`), remote/branch, and store/delete HTTPS Personal Access Tokens (PATs) in the OS keyring.
   - **Domain Scope Filter List**: Click `All Scopes` or any individual scope in the active phase to filter the Kanban board.

3. **Central 4-Column Kanban Board**:
   - Displays cards with item ID, type badge (`BUG` / `FEAT`), severity badge (`LOW` / `MED` / `HIGH` / `CRIT`), domain scope tag, logged effort minutes, stagnant days warning, and recurrence badge (`recurrence_count > 0`).
   - Provides quick status transition buttons (`Start`, `Pause`, `Reopen`), timer start/stop toggle, and card selection.

4. **Right-Hand Context Drawer (Lazy-Loaded)**:
   - Opens automatically when a card is clicked, lazily loading `item.toml` and `desc.md` from disk.
   - **Title & Severity Editing**: Rename the item or change its severity with atomic index synchronization.
   - **Timer & Effort Logging**: Start/stop the session timer or log `+15m`, `+30m`, `+60m`, or custom minutes.
   - **Close with Fix Summary / Reopen**: Enter a fix summary (and optional commit SHA) to close or merge an item into `previous_fixes`, or reopen a completed item with a reason note.
   - **AI Agent Context Export**: One-click **Copy `@file` Refs** and **Copy Bounded Agent Prompt** buttons copy context payloads directly to the system clipboard.
   - **Native `desc.md` Editor**: Edit the item's Markdown specification in-place with template insertion buttons, dirty-state tracking (`Save desc.md`), or open the file in `$VISUAL` / `$EDITOR` / `xdg-open`.
