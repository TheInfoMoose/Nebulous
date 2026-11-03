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

    /// Stage and commit `.tracker/` changes and synchronize with the configured Git remote.
    Sync {
        /// Commit message for `.tracker/` changes.
        #[arg(short, long)]
        message: Option<String>,

        /// Optional Personal Access Token override for HTTPS push/pull.
        #[arg(long)]
        pat: Option<String>,

        /// Stage and commit `.tracker/` locally without pushing or pulling the remote.
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
