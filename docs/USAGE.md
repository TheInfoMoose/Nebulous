# Nebulous Usage Guide (`neb` CLI, Retro Amber TUI & `neb mcp` Server)

Nebulous provides three synchronized terminal and agent interfaces operating on the same `.tracker/` directory and global project registry:
- **`neb` CLI**: Ergonomic zero-ID command-line interface with predictive scope/item inference, 3-stage lifecycle management, and `--json` output mode.
- **`neb` / `neb tui` (`neb ui`)**: Interactive retro Amber-on-Black CRT phosphor terminal dashboard (`ratatui` + `crossterm`) with an aspect-locked ASCII Nebula + `NEBULOUS` header and responsive scaling locks.
- **`neb mcp`**: Built-in JSON-RPC 2.0 `stdio` Model Context Protocol server for Cursor, Claude Code, Codex, Ollama, and automated self-healing CI/CD pipelines.

---

## Part 1: Companion CLI (`neb`)

### Global Flags

Every `neb` command accepts these global options:
- `--root <PATH>`: Path to the workspace root or `.tracker/` directory. If omitted, `neb` walks up ancestor directories from the current working directory to discover `.tracker/project.toml`.
- `--json`: Emit machine-readable JSON output instead of human-formatted tables/text.

---

### 1. One-Command Bootstrap & 3-Stage Lifecycle (`neb init`, `neb stage`, `neb move`)

By default, `neb init` scaffolds all 3 canonical stages (`pre_production`, `production`, `post_production`), scans and promotes discovered workspace components into domain scopes across all 3 stages, and registers the project in `projects.toml`:

```bash
# One-command initialization (scaffolds 3 canonical stages, ingests components, and registers project)
neb init

# Optional flags to skip component ingestion or global registry registration
neb init --name "Nebulous" --no-ingest --no-register

# List all stages and completion counts
neb stage

# Switch the active stage using canonical aliases (pre | prod | post | 1 | 2 | 3)
neb stage prod

# Move the active item (or an explicit item ID) to another stage
neb move prod
neb move FEAT-0001 post
```

---

### 2. Predictive Capture & Zero-ID Daily Flow (`neb bug`, `neb feat`, `neb task`, `neb next`, `neb start`, `neb done`)

When `--scope` is omitted, `neb bug` and `neb feat` automatically infer the target domain scope from:
1. Your current working directory (`cwd` inside a scope's `target` directory),
2. Modified/dirty files in Git (`git status`),
3. The currently running `.timer.toml` session, or
4. A single non-`inbox` scope in the active stage (falling back to `inbox`).

When to use each item type:
- **Bug (`neb bug`)**: something broken.
- **Feature (`neb feat`)**: new capability for the user.
- **Task (`neb task`, alias `neb chore`)**: any other planned change.

```bash
# Capture a bug or feature (scope is inferred automatically when -s/--scope is omitted)
neb bug "TOML parser panics on unquoted local datetime" --severity high
neb feat "Add horizontal scroll window for compact modal fields" --severity medium
neb task "Extract roadmap parser into planning module"

# Compute and display the ranked "What's Next" queue, inferred scope, active timer, and stage advancement hints
neb next

# Start working on the top recommended item (or an explicit <ID>):
# automatically stops any prior timer, marks in_progress, starts timer, and copies bounded agent context
neb start
neb start BUG-0001

# Inspect full item details or export Bounded Agent Context for the active item without typing its ID
neb show
neb context
neb context --refs-only --copy

# Close the active item (automatically stops its timer and logs elapsed minutes) and create a documented Git commit
neb done -m "Added custom serde deserializer for unquoted TOML datetimes" -c
```

Timer sessions are stored in `.tracker/.timer.toml`: `metrics.max_session_minutes` in `project.toml` (default `480`, `0` = off) caps one session, and a corrupt `.timer.toml` is moved to `.timer.toml.bak`.

---

### 3. Global Multi-Project Registry (`neb project`)

Nebulous tracks registered workspaces in `<config_dir>/nebulous/projects.toml` (`~/.config/nebulous/projects.toml` on Linux, `~/Library/Application Support/nebulous/projects.toml` on macOS, `%APPDATA%\nebulous\projects.toml` on Windows; overridable via `NEBULOUS_CONFIG_DIR`).

```bash
# List all registered projects and see which project is active (*)
neb project list

# Register a project directory (initializes .tracker/ if needed and sets it active)
neb project add /path/to/repo --name "Orbit" --tags os,rolling,custom-kernel

# Switch the active project by ID, name, or path
neb project switch "Orbit"

# Remove a project from the global registry (does not delete its .tracker/ directory)
neb project remove "Orbit"
```

---

### 4. Two-Stage Directory Ingestion & Component Pinning (`neb ingest` & `neb component`)

#### Stage 1: Scan & Ingest Project Directory (`neb ingest`)
Walks the active working directory (or declared `<SUBDIR>`) respecting `.gitignore` (skipping `target/`, `node_modules/`, `.git/`, `.tracker/`, `dist/`, `build/`, `__pycache__/`, `.venv/`), detects ecosystem manifests (`Cargo.toml`, `package.json`, `pyproject.toml`, `CMakeLists.txt`, `Makefile`, `Dockerfile`, `nebulous.component.toml`), and records relative file lists (`files = [...]`) into `.tracker/project.toml` `[components]`.

```bash
# Scan the current working directory and register discovered components
neb ingest

# Scan a specific subdirectory, write nebulous.component.toml files, and promote all to scopes across stages
neb ingest --path crates --write-manifests --promote-all
```

#### Stage 2: Manage `nebulous.component.toml` & Rolling-Release Pins (`neb component`)

```bash
# List all ingested components in the current project
neb component list

# Inspect a component's files, dependencies, and pinned_release snapshot
neb component show iso

# Initialize or update a nebulous.component.toml in an un-manifested directory
neb component init iso \
  --id orbit_image \
  --name "Orbit Live Image" \
  --type os_image \
  --stack rolling-base,custom-kernel,image-builder \
  --tags boot,iso \
  --flags "--arch=amd64,--flavour=rt"

# Pin the current known-good Git commit, build script, flags, and package versions
neb component pin orbit_image \
  --script scripts/build_iso.sh \
  --flags "--arch=amd64,--flavour=rt" \
  --dep linux-image-rt=6.11.6-rt1,systemd=256.7-1

# Roll back to the last_known_good pin when an upstream rolling update breaks the build
neb component rollback orbit_image \
  --item BUG-0001 \
  --reason "Kernel 6.12-rc1 broke initramfs ZFS module compile in the VM boot check"

# Promote an ingested component into a Tier 3 Domain Scope (automatically mirrored across stages)
neb component promote orbit_image --scope orbit_image
```

---

### 5.Git Operations & Enforced Commit Documentation Policy (`neb git` & `neb sync`)

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

# Full one-command sync (stage/commit + pull + reconcile + push)
neb sync -i BUG-0001 -m "Fix unquoted TOML datetime deserialization" --all
```

---

### 6. Plan Import (`neb plan import`)

`neb plan import <PATH>` reads the `## Step Execution Roadmap` section of a stepped Markdown plan file and synchronizes its steps as `task` items:
- **Roadmap tables**: reads `### Up Next` (columns `# | Step | Command | Scope | Target Files | Waits on`) followed by `### Completed` (columns `Step ID | Status | Command | Scope | Target Files`). Rows whose `Step` cell starts with `[USER]` are skipped; an `Up Next` row whose `Waits on` cell starts with `COMPLETED` counts as completed.
- **Dry run by default**: running `neb plan import <PATH>` without `--write` previews all planned actions without modifying `.tracker/`. Pass `--write` to apply changes, and `-p` / `--phase <ID>` to choose the stage where newly created tasks land (defaults to `active_phase`).
- **Output format**: prints all `CREATE <ID|(new)> ← <STEP>` lines first, then all `RETITLE <ID> ← <STEP>` lines, then all `CLOSE <ID> ← <STEP>` lines (each group in roadmap order), followed by `UNCHANGED <n>` and (when not writing) `dry run: nothing written; re-run with --write`. Pass `--json` to emit the structured `ImportReport`.
- **Idempotency & field ownership (`source_ref`)**: each imported task stores the plan step ID in `item.source_ref` (for example `source_ref = "STEP 2A"`), so re-running `neb plan import` matches existing items and reports `UNCHANGED` when nothing changed. The plan sets `title` (from `Scope`), `type` (`task`), and `scope` at creation, updates `title` on re-import if the plan's `Scope` cell changed (closed and merged items are never retitled), and closes an open item when its row is `COMPLETED`. A `COMPLETED` row with no matching item creates nothing. Import never reopens a closed item and never changes stage, scope, severity, effort, or `desc.md`.
- **Scope selection**: for new tasks, the target scope is the scope in the import stage matching the most `Target Files` entries (by longest `target` path prefix or exact `tracked_files` match); a tie or zero matches falls back to `default_scope` (`inbox`).

```bash
# Dry-run preview against a stepped plan file (nothing written)
neb plan import STEPPED_IMPLEMENTATION_PLAN.md

# Apply the import into the production stage
neb plan import STEPPED_IMPLEMENTATION_PLAN.md --phase prod --write
```

---

### 7. Item IDs

By default, Nebulous allocates sequential item IDs (`BUG-0001`, `FEAT-0001`, `TASK-0001`) using monotonic per-type counters in `.tracker/project.toml`.

To opt into collision-resistant random hash IDs across branches, set `id_scheme = "hash"` under `[project]` in `.tracker/project.toml` (when the key is absent, `"sequential"` is used):

```toml
[project]
id_scheme = "hash"
```

- **Format**: `<PREFIX>-<6 uppercase hex digits>` (for example `BUG-7F3A2C`, `FEAT-4B19E0`, `TASK-9C04D2`), always containing at least one hex letter (`A`–`F`) so hash IDs are unambiguous from sequential IDs.
- **Immutability**: switching `id_scheme` affects newly allocated items only; existing item IDs never change.
- **Duplicate-ID warnings**: `neb sync-index` (as well as `neb sync` and `neb git pull`) warns if the same item ID exists in more than one item directory across stages or scopes.

---

## Part 2: Retro Amber-on-Black TUI (`neb` / `neb tui`)

Running bare `neb` or `neb tui` (`neb ui`) in an interactive terminal launches the retro CRT Amber-on-Black dashboard:

```bash
neb
# or
neb tui
```

### Visual Architecture & Scaling Locks

1. **P3 Phosphor Amber-on-Black Palette**:
   - Pure CRT black background (`#000000`) painted across every cell of the terminal buffer so host terminal themes never bleed through.
   - Primary Amber (`#FFB000`), High-Intensity Gold (`#FFD866`), Muted Amber Dust (`#996300`), Deep Starfield Amber (`#5C3A00`), and High-Contrast Alert Orange (`#FF5500`).
2. **Centered ASCII Nebula + `NEBULOUS` Header**:
   - **Full Banner (`5×76`, when terminal is `>= 78×26`)**: Multi-line block-letter `N E B U L O U S` glyphs framed by symmetric swirling cosmic dust (`* . · ░▒▓ ( ✦`, `✦ ) ▓▒░ · . *`).
   - **Compact Banner (`3×60`, when terminal is `< 78` cols or `< 26` rows)**: Compact 3-line cosmic swirl framing `╔═╗ N E B U L O U S ╔═╗`.
   - Rendered with horizontal centering (`Alignment::Center`) and wrapping strictly disabled so the ASCII artwork never clips or deforms.
3. **Responsive Workspace Breakpoint Locks**:
   - **Wide Mode (`>= 110` cols)**: 3-pane horizontal layout (Left Scopes Menu `26` cols, Center Stage Items Table `Min(44)` cols with locked `ID`/`Sev`/`Status`/`Title` columns, Right Inspector & Agent Context `Min(38)` cols).
   - **Medium Mode (`64..109` cols)**: 2-pane top row (Left Scopes Menu `22` cols + Center Items Table `Min(40)` cols) paired with a scrollable bottom Inspector Drawer (`Min(7)` rows).
   - **Minimum Terminal Lock Guard (`< 64×20`)**: Displays a centered scaling lock screen with the compact Nebula logo, current vs. required dimensions (`Current: WxH | Minimum Locked: 64x20`), and active focus without clipping or panicking.
4. **Modal Field Accessibility & `[REQUIRED FIELD LOCK]`**:
   - `New Bug` (`b`) and `New Feature` (`f`) modals pre-fill `Scope` (via `infer_scope_from_context`), `Stage` (active stage), and `Severity` (`medium`), giving every required field (`[1] Title *REQUIRED*`, `[2] Scope *REQUIRED*`, `[3] Stage *REQUIRED*`, `[4] Severity *REQUIRED*`, `[5] Description`) a locked row height and horizontal cursor-window scrolling.
   - Pressing `Enter` when a required field (`Title` or `Summary`) is empty triggers an inline `AMBER_ALERT` `[REQUIRED FIELD LOCK]` indicator and keeps focus locked on the missing field.

### Keyboard Shortcuts

| Key | Action |
| :--- | :--- |
| `1` / `2` / `3` | Change the viewed stage (`Pre-Production` / `Production` / `Post-Production`) only (`neb stage` changes the active stage) |
| `Tab` / `Shift+Tab` or `h` / `l` | Cycle focus across panes (`Scopes` <-> `Items` <-> `Details`) or modal fields |
| `j` / `k` or `↑` / `↓` | Move selection up/down in active list/table, or scroll Details inspector |
| `PgUp` / `PgDn` or `J` / `K` | Scroll the Inspector / `desc.md` / Fix History pane |
| `b` / `f` | Open predictive `New Bug` / `New Feature` modal |
| `t` | Open predictive `New Task` modal |
| `s` | Start work on selected/recommended item (marks `in_progress`, starts timer, copies prompt) |
| `d` | Open `Done / Close` modal pre-filled with `"Resolved <ID>: <title>"` and Git `HEAD` SHA |
| `m` | Open 1-key Stage Move modal (`1` Pre-Production, `2` Production, `3` Post-Production) |
| `c` | Copy Bounded Agent Prompt for selected/active item to clipboard |
| `i` | Ingest workspace directory and promote discovered components to scopes across all stages |
| `n` | Jump directly to the top-ranked `"What's Next"` item |
| `r` | Reload `.tracker/` index and items |
| `/` | Filter stage items by ID, title, or scope (`Enter`/`Esc` to exit filter input) |
| `q` | Quit the TUI (inside a modal it types text) |
| `Esc` | Close the modal or quit the TUI |

The list refreshes automatically on external `.tracker/` changes.

---

## Part 3: Model Context Protocol Server (`neb mcp`)

`neb mcp` runs a standard JSON-RPC 2.0 MCP server over `stdin`/`stdout` (supporting both newline-delimited JSON-RPC and `Content-Length` headers).

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

### `plan_import` MCP Tool

The `plan_import` MCP tool imports roadmap steps from a stepped plan file using the same engine as `neb plan import`:
- **Arguments**:
  - `path` (required string): path to the plan Markdown file, relative to the project root (paths escaping the project root are rejected).
  - `phase` (optional string): target stage for newly created tasks (defaults to `active_phase`).
  - `dry_run` (optional boolean, default `true`): when `true`, previews changes without modifying `.tracker/`; set `dry_run: false` to write changes.
- **Strict validation & response**: unknown arguments are rejected, and the tool returns the same JSON `ImportReport` (`created`, `retitled`, `closed`, `unchanged`) as `neb plan import --json`.
