use std::path::PathBuf;

use backend_core::{ItemStatus, ItemType, Severity};
use clap::{Parser, Subcommand};

/// Companion CLI (`neb`) for the Nebulous project and bug tracker.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "neb",
    version,
    about = "Companion CLI (`neb`) for the Nebulous 4-tier project and bug tracker",
    propagate_version = true
)]
pub struct Cli {
    /// Path to the workspace root or `.tracker/` directory (defaults to discovering `.tracker/` in current or ancestor directories).
    #[arg(long, global = true, value_name = "PATH")]
    pub root: Option<PathBuf>,

    /// Output results in machine-readable JSON format.
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    /// Initialize a new `.tracker/` hierarchy in the target directory.
    Init {
        /// Project name (defaults to directory name or "Nebulous").
        #[arg(short, long)]
        name: Option<String>,

        /// Initial phase identifier.
        #[arg(long, default_value = "01_core_engine")]
        phase_id: String,

        /// Initial phase human-readable title.
        #[arg(long, default_value = "Core Engine & Initial Scope")]
        phase_title: String,
    },

    /// Quick-capture a bug item (`BUG-XXXX`).
    Bug {
        /// Short summary title of the bug.
        title: String,

        /// Target domain scope ID (falls back to `default_scope` / `inbox` if omitted).
        #[arg(short, long)]
        scope: Option<String>,

        /// Target phase ID (falls back to `active_phase` if omitted).
        #[arg(short, long)]
        phase: Option<String>,

        /// Bug severity (`low`, `medium`, `high`, `critical`).
        #[arg(long, default_value = "medium")]
        severity: Severity,

        /// Problem description and reproduction notes for `desc.md`.
        #[arg(short, long = "desc", visible_alias = "description")]
        desc: Option<String>,
    },

    /// Quick-capture a feature item (`FEAT-XXXX`).
    #[command(visible_alias = "feature")]
    Feat {
        /// Short summary title of the feature.
        title: String,

        /// Target domain scope ID (falls back to `default_scope` / `inbox` if omitted).
        #[arg(short, long)]
        scope: Option<String>,

        /// Target phase ID (falls back to `active_phase` if omitted).
        #[arg(short, long)]
        phase: Option<String>,

        /// Feature priority/severity (`low`, `medium`, `high`, `critical`).
        #[arg(long, default_value = "medium")]
        severity: Severity,

        /// Feature specification and notes for `desc.md`.
        #[arg(short, long = "desc", visible_alias = "description")]
        desc: Option<String>,
    },

    /// Fast index-backed listing of tracked items with optional filters.
    List {
        /// Filter by domain scope ID.
        #[arg(short, long)]
        scope: Option<String>,

        /// Filter by phase ID.
        #[arg(short, long)]
        phase: Option<String>,

        /// Filter by item status (`open`, `in_progress`, `pending`, `closed`, `merged`).
        #[arg(long)]
        status: Option<ItemStatus>,

        /// Filter by item type (`bug`, `feat` / `feature`).
        #[arg(long = "type", visible_alias = "kind")]
        item_type: Option<ItemType>,

        /// Filter to stagnant non-completed items exceeding `stagnant_item_threshold_days`.
        #[arg(long)]
        stagnant: bool,

        /// Filter to recurring items (`recurrence_count > 0`).
        #[arg(long)]
        recurring: bool,
    },

    /// Display full item metadata, effort, fix history ledger, and `desc.md`.
    Show {
        /// Item ID (e.g. `BUG-0001` or `FEAT-0001`).
        id: String,
    },

    /// Update an item's status (`open`, `in_progress`, `pending`, `closed`, `merged`).
    Status {
        /// Item ID (e.g. `BUG-0001`).
        id: String,

        /// New status (`open`, `in_progress`, `pending`, `closed`, `merged`).
        status: ItemStatus,
    },

    /// Close or merge an item and record the fix in `previous_fixes`.
    Close {
        /// Item ID (e.g. `BUG-0001`).
        id: String,

        /// Summary of the fix recorded in `previous_fixes`.
        #[arg(
            short = 'm',
            long = "summary",
            visible_alias = "message",
            default_value = "Closed via neb CLI"
        )]
        summary: String,

        /// Explicit commit short SHA (defaults to auto-detected Git `HEAD` SHA or `"uncommitted"`).
        #[arg(long)]
        commit: Option<String>,

        /// Mark the item as `merged` instead of `closed`.
        #[arg(long)]
        merged: bool,
    },

    /// Reopen a `closed` or `merged` item, incrementing `recurrence_count` and `iteration_count`.
    Reopen {
        /// Item ID (e.g. `BUG-0001`).
        id: String,

        /// Optional note explaining why the item was reopened (appended to `desc.md`).
        #[arg(short = 'r', long = "reason", visible_alias = "note")]
        reason: Option<String>,
    },

    /// Start, stop, or inspect the active session timer (`.tracker/.timer.toml`).
    Timer {
        #[command(subcommand)]
        action: Option<TimerCommand>,
    },

    /// Manually log effort minutes onto an item (`effort.logged_time_minutes`).
    Log {
        /// Item ID (e.g. `BUG-0001`).
        id: String,

        /// Minutes to log onto the item.
        #[arg(short = 'm', long = "minutes")]
        minutes: Option<u32>,

        /// Positional minutes fallback when `-m/--minutes` is omitted.
        #[arg(value_name = "MINUTES")]
        positional_minutes: Option<u32>,
    },

    /// Output (and optionally copy to clipboard) `@file` references or a Bounded Agent Prompt.
    Context {
        /// Item ID (e.g. `BUG-0001`).
        id: String,

        /// Output only compact `@file` reference tags (`@.tracker/.../scope.toml ...`).
        #[arg(long)]
        refs_only: bool,

        /// Copy the exported context string to the system clipboard via `arboard`.
        #[arg(long)]
        copy: bool,
    },

    /// Print project time valuation, stagnant items, and recurring bug hotspots.
    Metrics,

    /// Stage and commit `.tracker/` (or documented workspace) changes and synchronize with the configured Git remote.
    Sync {
        /// Commit message for `.tracker/` or workspace changes.
        #[arg(short, long)]
        message: Option<String>,

        /// Optional linked item ID (`BUG-XXXX` or `FEAT-XXXX`) when committing workspace changes.
        #[arg(short = 'i', long = "item")]
        item: Option<String>,

        /// Stage all workspace + `.tracker/` changes instead of `.tracker/` only.
        #[arg(short = 'a', long = "all")]
        all: bool,

        /// Optional Personal Access Token override for HTTPS push/pull.
        #[arg(long)]
        pat: Option<String>,

        /// Stage and commit locally without pushing or pulling the remote.
        #[arg(long)]
        commit_only: bool,
    },

    /// Reconcile `.tracker/` indexes (`project.toml`, `phase.toml`, `scope.toml`) from authoritative `item.toml` files.
    #[command(name = "sync-index")]
    SyncIndex,

    /// Manage project phases (`list`, `add`, `use`).
    Phase {
        #[command(subcommand)]
        action: PhaseCommand,
    },

    /// Manage domain scopes (`list`, `add`).
    Scope {
        #[command(subcommand)]
        action: ScopeCommand,
    },

    /// Manage the global multi-project registry (`list`, `add`, `switch`, `remove`).
    Project {
        #[command(subcommand)]
        action: ProjectCommand,
    },

    /// Scan and ingest project directory components into `.tracker/project.toml` (`[components]`),
    /// isolating to the active working directory or the declared directory.
    Ingest {
        /// Optional target directory or subdirectory path to restrict scanning (defaults to current working directory).
        #[arg(long = "path", short = 'p', value_name = "SUBDIR")]
        path: Option<String>,

        /// Positional directory or subdirectory fallback when `--path` is omitted.
        #[arg(value_name = "SUBDIR")]
        positional_path: Option<String>,

        /// Write or update `nebulous.component.toml` files in each discovered component directory.
        #[arg(long)]
        write_manifests: bool,

        /// Promote all discovered components into domain scopes in the target/active phase.
        #[arg(long)]
        promote_all: bool,

        /// Target phase ID when `--promote-all` is enabled (defaults to `project.active_phase`).
        #[arg(long)]
        phase: Option<String>,
    },

    /// Manage ingested components, `nebulous.component.toml` manifests, and rolling-release pins (`list`, `show`, `init`, `pin`, `rollback`, `promote`).
    Component {
        #[command(subcommand)]
        action: ComponentCommand,
    },

    /// Execute explicit Git workflow operations (`status`, `fetch`, `pull`, `push`, `commit`, `sync`).
    Git {
        #[command(subcommand)]
        action: GitCommand,
    },

    /// Start the JSON-RPC 2.0 `stdio` Model Context Protocol (MCP) server for AI coding agents and CI/CD pipelines.
    Mcp,
}

#[derive(Debug, Clone, Subcommand)]
pub enum TimerCommand {
    /// Start an active session timer for `<ID>`.
    Start {
        /// Item ID to time (e.g. `BUG-0001`).
        id: String,

        /// Optional note attached to the timer session.
        #[arg(short, long)]
        note: Option<String>,
    },

    /// Stop the active session timer and add elapsed minutes to the item.
    Stop {
        /// Optional item ID assertion (verifies the stopped timer matches `<ID>` if supplied).
        id: Option<String>,
    },

    /// Show the currently running session timer and elapsed duration.
    Status {
        /// Optional item ID filter.
        id: Option<String>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum PhaseCommand {
    /// List all registered project phases.
    List,

    /// Create and register a new project phase.
    Add {
        /// Phase identifier (e.g. `02_git_and_polish`).
        id: String,

        /// Human-readable phase title.
        #[arg(short, long)]
        title: Option<String>,

        /// Positional phase title fallback when `-t/--title` is omitted.
        #[arg(value_name = "TITLE")]
        positional_title: Option<String>,

        /// Set the newly created phase as `project.active_phase`.
        #[arg(long = "use", visible_alias = "active")]
        set_active: bool,
    },

    /// Switch `project.active_phase` to `<ID>`.
    Use {
        /// Target phase identifier.
        id: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ScopeCommand {
    /// List domain scopes within a phase.
    List {
        /// Phase ID (defaults to `project.active_phase`).
        #[arg(short, long)]
        phase: Option<String>,
    },

    /// Create and register a new domain scope within a phase.
    Add {
        /// Scope identifier (e.g. `backend_core`).
        id: String,

        /// Target crate or directory path (e.g. `crates/backend_core`).
        #[arg(short, long)]
        target: Option<String>,

        /// Positional target path fallback when `-t/--target` is omitted.
        #[arg(value_name = "TARGET")]
        positional_target: Option<String>,

        /// Phase ID to create the scope under (defaults to `project.active_phase`).
        #[arg(short, long)]
        phase: Option<String>,

        /// Comma-separated or repeated technology stack tags (e.g. `--stack rust,serde`).
        #[arg(long = "stack", visible_alias = "tech-stack", value_delimiter = ',')]
        tech_stack: Vec<String>,

        /// Optional description of the domain scope's responsibility boundary.
        #[arg(short, long = "desc", visible_alias = "description")]
        desc: Option<String>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ProjectCommand {
    /// List all projects registered in the global Nebulous project registry (`projects.toml`).
    List,

    /// Register a project directory in the global Nebulous project registry (`projects.toml`).
    Add {
        /// Path to the project workspace root or `.tracker/` directory.
        path: PathBuf,

        /// Optional display name override for the project.
        #[arg(short, long)]
        name: Option<String>,

        /// Optional comma-separated or repeated tags for the project.
        #[arg(long = "tag", visible_alias = "tags", value_delimiter = ',')]
        tags: Vec<String>,

        /// Set this project as the active project in `projects.toml`.
        #[arg(long = "use", visible_alias = "active", default_value_t = true)]
        set_active: bool,
    },

    /// Switch the active project in the global Nebulous registry by ID, name, or path.
    #[command(visible_alias = "use")]
    Switch {
        /// Project ID, name, or workspace path.
        id_or_name: String,
    },

    /// Remove a project from the global Nebulous registry by ID, name, or path.
    #[command(visible_alias = "rm")]
    Remove {
        /// Project ID, name, or workspace path.
        id_or_name: String,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum ComponentCommand {
    /// List all ingested components registered in `.tracker/project.toml` (`[components]`).
    List,

    /// Display full details, file list, dependencies, and pinned release configuration for a component.
    Show {
        /// Component ID or relative directory path.
        id: String,
    },

    /// Initialize or update a `nebulous.component.toml` manifest in `<DIR>` and register it in `project.toml`.
    Init {
        /// Workspace-relative directory path (e.g. `iso`, `crates/backend_core`, or `.`).
        #[arg(default_value = ".")]
        dir: String,

        /// Explicit component identifier override.
        #[arg(long)]
        id: Option<String>,

        /// Human-readable component name override.
        #[arg(short, long)]
        name: Option<String>,

        /// Component classification type (`rust_crate`, `cli_crate`, `ui_crate`, `python_pkg`, `node_pkg`, `c_cpp`, `os_image`, `kernel_module`, `build_scripts`, `custom`).
        #[arg(long = "type", visible_alias = "kind")]
        component_type: Option<String>,

        /// Comma-separated or repeated tech stack tags (e.g. `--stack rust,tokio`).
        #[arg(long = "stack", visible_alias = "tech-stack", value_delimiter = ',')]
        tech_stack: Vec<String>,

        /// Comma-separated or repeated component tags.
        #[arg(long = "tag", visible_alias = "tags", value_delimiter = ',')]
        tags: Vec<String>,

        /// Comma-separated or repeated build flags.
        #[arg(
            long = "flag",
            visible_alias = "flags",
            value_delimiter = ',',
            allow_hyphen_values = true
        )]
        flags: Vec<String>,

        /// Optional component description.
        #[arg(short, long = "desc", visible_alias = "description")]
        desc: Option<String>,
    },

    /// Snapshot the component's current Git commit SHA, dependency versions, and build flags as `last_known_good` (`pinned_release`).
    Pin {
        /// Component ID or relative directory path.
        id: String,

        /// Explicit commit SHA override (defaults to current Git `HEAD` short SHA).
        #[arg(long)]
        commit: Option<String>,

        /// Optional target build script relative path (e.g. `scripts/build_iso.sh`).
        #[arg(long = "script", visible_alias = "build-script")]
        build_script: Option<String>,

        /// Comma-separated or repeated build flags to snapshot.
        #[arg(
            long = "flag",
            visible_alias = "flags",
            value_delimiter = ',',
            allow_hyphen_values = true
        )]
        flags: Vec<String>,

        /// Optional `NAME=VERSION` dependency version pairs to record and pin.
        #[arg(long = "dep", visible_alias = "dependency", value_delimiter = ',')]
        deps: Vec<String>,
    },

    /// Roll back a component's dependencies and build flags to its `pinned_release` (`last_known_good`) snapshot.
    Rollback {
        /// Component ID or relative directory path.
        id: String,

        /// Optional active item ID to record a regression on during rollback.
        #[arg(short = 'i', long = "item")]
        item: Option<String>,

        /// Optional rollback reason appended to the item's `desc.md`.
        #[arg(short = 'r', long = "reason", visible_alias = "note")]
        reason: Option<String>,

        /// Return the pinned configuration without overwriting `nebulous.component.toml`.
        #[arg(long)]
        no_restore: bool,
    },

    /// Promote an ingested component into a Tier 3 domain scope (`scope.toml`) in a phase.
    Promote {
        /// Component ID or relative directory path.
        id: String,

        /// Target phase ID (defaults to `project.active_phase`).
        #[arg(short, long)]
        phase: Option<String>,

        /// Custom scope ID override (defaults to the component ID).
        #[arg(short, long)]
        scope: Option<String>,

        /// Optional comma-separated subset of component files to track.
        #[arg(long = "files", value_delimiter = ',')]
        files: Vec<String>,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum GitCommand {
    /// Inspect repository branch, remote URL, `ahead`/`behind` counts, and dirty `.tracker/` or workspace files.
    Status,

    /// Fetch the configured remote branch without modifying the working tree.
    Fetch {
        /// Optional Personal Access Token or SSH key override.
        #[arg(long)]
        pat: Option<String>,
    },

    /// Fetch and merge/fast-forward from the configured remote and reconcile `.tracker/` indexes.
    Pull {
        /// Optional Personal Access Token or SSH key override.
        #[arg(long)]
        pat: Option<String>,
    },

    /// Push the configured branch to the configured remote.
    Push {
        /// Optional Personal Access Token or SSH key override.
        #[arg(long)]
        pat: Option<String>,
    },

    /// Create a policy-validated, self-documenting Git commit linked to a tracked item (`[<ITEM_ID>](<scope>) <summary>`).
    Commit {
        /// Linked work item ID (`BUG-XXXX` or `FEAT-XXXX`).
        #[arg(short = 'i', long = "item")]
        item: Option<String>,

        /// Concise commit summary message.
        #[arg(
            short = 'm',
            long = "summary",
            visible_alias = "message",
            default_value = ""
        )]
        summary: String,

        /// Optional domain scope ID override (resolved from `--item` when omitted).
        #[arg(short = 's', long = "scope")]
        scope: Option<String>,

        /// Optional extended commit body or agent notes.
        #[arg(short = 'b', long = "body", visible_alias = "notes")]
        body: Option<String>,

        /// Stage all modified/untracked workspace files alongside `.tracker/`.
        #[arg(short = 'a', long = "all")]
        all: bool,

        /// Stage only `.tracker/` metadata files without workspace code changes.
        #[arg(long)]
        tracker_only: bool,

        /// Optional explicit workspace-relative file paths to stage alongside `.tracker/`.
        #[arg(long = "path", visible_alias = "paths", value_delimiter = ',')]
        paths: Vec<String>,
    },

    /// Stage and commit `.tracker/` changes and synchronize with the configured Git remote.
    Sync {
        /// Commit message for `.tracker/` changes.
        #[arg(short, long)]
        message: Option<String>,

        /// Optional Personal Access Token override.
        #[arg(long)]
        pat: Option<String>,

        /// Commit `.tracker/` locally without pushing or pulling.
        #[arg(long)]
        commit_only: bool,
    },
}
