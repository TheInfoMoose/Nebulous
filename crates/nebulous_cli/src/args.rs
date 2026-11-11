use std::path::PathBuf;

use backend_core::{ItemStatus, ItemType, Severity};
use clap::{Parser, Subcommand, ValueEnum};

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
    pub command: Option<Commands>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    /// Launch the interactive terminal dashboard (or display the active status + "What's Next" summary).
    #[command(visible_alias = "ui")]
    Tui,

    /// Initialize a new `.tracker/` hierarchy in the target directory (scaffolds all 3 canonical stages, ingests components, and registers project by default).
    Init {
        /// Project name (defaults to directory name or "Nebulous").
        #[arg(short, long)]
        name: Option<String>,

        /// Initial phase identifier.
        #[arg(long, default_value = "pre_production")]
        phase_id: String,

        /// Initial phase human-readable title.
        #[arg(long, default_value = "Pre-Production")]
        phase_title: String,

        /// Skip automatic component discovery and scope promotion across stages.
        #[arg(long)]
        no_ingest: bool,

        /// Skip automatic registration in the global `projects.toml` registry.
        #[arg(long)]
        no_register: bool,
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

    /// Display full item metadata, effort, fix history ledger, and `desc.md` (defaults to active or top "What's Next" item if `<ID>` is omitted).
    Show {
        /// Optional item ID (e.g. `BUG-0001` or `FEAT-0001`).
        id: Option<String>,
    },

    /// Start working on `<ID>` (or the top-ranked "What's Next" item), transition to `in_progress`, start the timer, and export bounded agent context.
    Start {
        /// Optional item ID (e.g. `BUG-0001`); resolves to top "What's Next" item if omitted.
        id: Option<String>,

        /// Optional note attached to the timer session.
        #[arg(short, long)]
        note: Option<String>,

        /// Copy the bounded agent context to the system clipboard (defaults to `true`).
        #[arg(
            long,
            default_value_t = true,
            default_missing_value = "true",
            num_args = 0..=1,
            require_equals = true,
            action = clap::ArgAction::Set
        )]
        copy: bool,
    },

    /// Compute and display the ranked "What's Next" queue, active timer, inferred scope, missing spec hints, and stage advancement recommendation.
    Next,

    /// Move an item across canonical stages (`pre` / `prod` / `post`). If `<ID>` is omitted, moves the active or top inferred item.
    Move {
        /// Item ID (or target stage when only one positional argument is supplied, e.g. `neb move prod`).
        id: Option<String>,

        /// Target stage (`pre`, `prod`, `post`, or explicit phase ID) when `<ID>` is also provided.
        #[arg(default_value = "")]
        stage: String,

        /// Explicit target stage override (`--to prod`).
        #[arg(short = 't', long = "to", value_name = "STAGE")]
        to: Option<String>,
    },

    /// Quick alias to switch the active stage (`neb stage prod`) or list stages (`neb stage`).
    Stage {
        /// Optional stage alias (`pre`, `prod`, `post`) or phase ID to switch to.
        stage: Option<String>,
    },

    /// Update an item's status (`open`, `in_progress`, `pending`, `closed`, `merged`).
    Status {
        /// Item ID (e.g. `BUG-0001`).
        id: String,

        /// New status (`open`, `in_progress`, `pending`, `closed`, `merged`).
        status: ItemStatus,
    },

    /// Close or merge an item (defaults to active or top inferred item if `<ID>` is omitted) and record the fix in `previous_fixes`.
    #[command(visible_alias = "done")]
    Close {
        /// Optional item ID (e.g. `BUG-0001`); resolves via active timer or `in_progress` item when omitted.
        id: Option<String>,

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

        /// Automatically create a documented Git commit (`[<ID>](<scope>) <summary>`) in the same step.
        /// Commits `.tracker/` only unless workspace files are listed with `--path`.
        #[arg(short = 'c', long = "commit-changes")]
        commit_changes: bool,

        /// Explicit project-relative workspace files/directories to include in the `-c` commit.
        /// Untracked files are committed only when listed by exact path; globs are rejected.
        #[arg(
            long = "path",
            visible_alias = "paths",
            value_delimiter = ',',
            requires = "commit_changes"
        )]
        paths: Vec<String>,
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

    /// Output (and optionally copy to clipboard) `@file` references or a Bounded Agent Prompt (defaults to active item when `<ID>` is omitted).
    Context {
        /// Optional item ID (e.g. `BUG-0001`); resolves via active timer or `in_progress` item when omitted.
        id: Option<String>,

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

        /// Explicit project-relative workspace files/directories to commit alongside `.tracker/`.
        /// Untracked files are committed only when listed by exact path; globs are rejected.
        #[arg(long = "path", visible_alias = "paths", value_delimiter = ',')]
        paths: Vec<String>,

        /// Read an HTTPS Personal Access Token from stdin (first line). Alternatively set
        /// `NEB_GIT_PAT`. PATs are never accepted on the command line and only sent to `https://` remotes.
        #[arg(long = "pat-stdin")]
        pat_stdin: bool,

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

        /// Register without making it the active project.
        #[arg(long = "no-use")]
        no_use: bool,

        #[arg(long = "use", alias = "active", hide = true)]
        use_compat: bool,
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
        /// Read an HTTPS Personal Access Token from stdin (first line). Alternatively set
        /// `NEB_GIT_PAT`. PATs are never accepted on the command line and only sent to `https://` remotes.
        #[arg(long = "pat-stdin")]
        pat_stdin: bool,
    },

    /// Fetch and merge/fast-forward from the configured remote and reconcile `.tracker/` indexes.
    Pull {
        /// Read an HTTPS Personal Access Token from stdin (first line). Alternatively set
        /// `NEB_GIT_PAT`. PATs are never accepted on the command line and only sent to `https://` remotes.
        #[arg(long = "pat-stdin")]
        pat_stdin: bool,
    },

    /// Push the configured branch to the configured remote.
    Push {
        /// Read an HTTPS Personal Access Token from stdin (first line). Alternatively set
        /// `NEB_GIT_PAT`. PATs are never accepted on the command line and only sent to `https://` remotes.
        #[arg(long = "pat-stdin")]
        pat_stdin: bool,
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

        /// Stage only `.tracker/` metadata files (the default when no `--path` is given).
        #[arg(long, conflicts_with = "paths")]
        tracker_only: bool,

        /// Explicit project-relative workspace files/directories to stage alongside `.tracker/`.
        /// Untracked files are staged only when listed by exact path; globs are rejected.
        #[arg(long = "path", visible_alias = "paths", value_delimiter = ',')]
        paths: Vec<String>,
    },

    /// Stage and commit `.tracker/` changes and synchronize with the configured Git remote.
    Sync {
        /// Commit message for `.tracker/` changes.
        #[arg(short, long)]
        message: Option<String>,

        /// Read an HTTPS Personal Access Token from stdin (first line). Alternatively set
        /// `NEB_GIT_PAT`. PATs are never accepted on the command line and only sent to `https://` remotes.
        #[arg(long = "pat-stdin")]
        pat_stdin: bool,

        /// Commit `.tracker/` locally without pushing or pulling.
        #[arg(long)]
        commit_only: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum StageArg {
    Build,
    LiveBoot,
    InstallE2e,
}

impl From<StageArg> for backend_core::release::loopstate::Stage {
    fn from(value: StageArg) -> Self {
        match value {
            StageArg::Build => Self::Build,
            StageArg::LiveBoot => Self::LiveBoot,
            StageArg::InstallE2e => Self::InstallE2e,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PhaseArg {
    Build,
    VmLiveBoot,
    VmInstallE2e,
}

impl From<PhaseArg> for backend_core::release::ledger::Phase {
    fn from(value: PhaseArg) -> Self {
        match value {
            PhaseArg::Build => Self::Build,
            PhaseArg::VmLiveBoot => Self::VmLiveBoot,
            PhaseArg::VmInstallE2e => Self::VmInstallE2e,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ResultArg {
    Pass,
    Fail,
}

impl From<ResultArg> for backend_core::release::candidate::PhaseResult {
    fn from(value: ResultArg) -> Self {
        match value {
            ResultArg::Pass => Self::Pass,
            ResultArg::Fail => Self::Fail,
        }
    }
}

impl From<ResultArg> for backend_core::release::loopstate::AttemptStatus {
    fn from(value: ResultArg) -> Self {
        match value {
            ResultArg::Pass => Self::Pass,
            ResultArg::Fail => Self::Fail,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum HoldFormat {
    HoldFile,
}

#[derive(Debug, Clone, Subcommand)]
pub enum ReleaseCommand {
    /// Extract a schema v2 golden inventory from an ISO without modifying `.tracker/release/`.
    ExtractGolden {
        /// Path to the ISO image to inspect.
        #[arg(long)]
        iso: PathBuf,

        /// Output path for the rendered golden TOML (required unless `--json` is passed).
        #[arg(long)]
        output: Option<PathBuf>,
    },

    /// Verify repository invariants and optional ISO forward-only variance against manifest and golden.
    VerifyVariance {
        /// Path to the ISO image to verify against manifest and golden.
        #[arg(
            long,
            required_unless_present = "repo_only",
            conflicts_with = "repo_only"
        )]
        iso: Option<PathBuf>,

        /// Verify repository invariants only without inspecting an ISO.
        #[arg(long)]
        repo_only: bool,

        /// Override path to `manifest.toml`.
        #[arg(long)]
        manifest: Option<PathBuf>,

        /// Override path to `golden.toml`.
        #[arg(long)]
        golden: Option<PathBuf>,

        /// Override path to `approvals.toml`.
        #[arg(long)]
        approvals: Option<PathBuf>,
    },

    /// Record a build or test attempt on a loop-state item and evaluate 3-strike loop guardrails.
    RecordAttempt {
        /// Loop-state item ID or path to its TOML file.
        #[arg(long)]
        item: String,

        /// Pipeline stage that ran (`build`, `live-boot`, `install-e2e`).
        #[arg(long)]
        stage: StageArg,

        /// Outcome of the attempt (`pass` or `fail`).
        #[arg(long)]
        status: ResultArg,

        /// Short summary of the failure error when status is `fail`.
        #[arg(long, default_value = "")]
        error_summary: String,

        /// Optional log file whose ANSI-stripped tail contributes to the error fingerprint.
        #[arg(long)]
        log_file: Option<PathBuf>,
    },

    /// Check whether a loop-state item is locked and whether repository invariants pass.
    CheckGuardrail {
        /// Loop-state item ID or path to its TOML file.
        #[arg(long)]
        item: String,
    },

    /// Bless an ISO inventory as the new golden baseline and write a release record. User-only: needs an interactive terminal.
    BlessGolden {
        /// Path to the production ISO image to bless.
        #[arg(long)]
        iso: PathBuf,

        /// Reason recorded in the release record for blessing this baseline.
        #[arg(long)]
        reason: String,
    },

    /// Approve a specific component, binary, or module removal or downgrade in `approvals.toml`. User-only: needs an interactive terminal.
    ApproveVariance {
        /// Component ID, custom binary path, or Calamares module name to approve.
        #[arg(long)]
        item_name: String,

        /// Reason for approving the removal or downgrade.
        #[arg(long)]
        reason: String,

        /// Optional target version to scope the approval to.
        #[arg(long)]
        version: Option<String>,
    },

    /// Reset a loop-locked item's retry count and unlock it for further attempts. User-only: needs an interactive terminal.
    UnlockLoop {
        /// Loop-state item ID or path to its TOML file.
        #[arg(long)]
        item: String,
    },

    /// Emit a hold file pinning components to the versions recorded in a release record.
    PinsFromRelease {
        /// Release record path or `latest`.
        #[arg(long, default_value = "latest")]
        release: String,

        /// Comma-separated component IDs to pin (defaults to all components in the record).
        #[arg(long, value_delimiter = ',')]
        components: Vec<String>,

        /// Output format (`hold-file`).
        #[arg(long, value_enum, default_value_t = HoldFormat::HoldFile)]
        format: HoldFormat,
    },

    /// Run the built-in offline release self-test suite.
    SelfTest,

    /// Register a new release candidate from a production ISO after verifying forward-only variance.
    Candidate {
        /// Path to the production ISO image.
        #[arg(long)]
        iso: PathBuf,

        /// Debian snapshot timestamp used by the build.
        #[arg(long)]
        snapshot: Option<String>,

        /// Optional third-party `.deb` artifact cache directory used by the build.
        #[arg(long)]
        artifact_cache: Option<PathBuf>,

        /// Optional hold file used by the build.
        #[arg(long)]
        hold_file: Option<PathBuf>,
    },

    /// Record a pass or fail result for a candidate phase (`build`, `vm-live-boot`, `vm-install-e2e`).
    RecordPhase {
        /// Candidate phase (`build`, `vm-live-boot`, `vm-install-e2e`).
        phase: PhaseArg,

        /// Phase outcome (`pass` or `fail`).
        result: ResultArg,

        /// Log file path(s) produced by this phase run.
        #[arg(long = "log", required = true)]
        logs: Vec<PathBuf>,

        /// Path to the ISO image tested in this phase.
        #[arg(long)]
        iso: PathBuf,

        /// Failure classification from `classify-failure.py` (required when result is `fail`).
        #[arg(long, required_if_eq("result", "fail"))]
        class: Option<String>,

        /// Classifier rule identifier (required when result is `fail`).
        #[arg(long, required_if_eq("result", "fail"))]
        classifier_rule: Option<String>,

        /// Optional path to `last-test-verdict.json` for VM phases.
        #[arg(long)]
        verdict: Option<PathBuf>,
    },

    /// Record a confirmed component failure in `.tracker/release/failures.toml`.
    RecordFailure {
        /// Component identifier that failed.
        #[arg(long)]
        component: String,

        /// Component version that failed.
        #[arg(long)]
        version: String,

        /// Phase in which the failure occurred.
        #[arg(long)]
        phase: PhaseArg,

        /// Failure classification (`component`).
        #[arg(long)]
        class: String,

        /// Classifier rule identifier.
        #[arg(long)]
        classifier_rule: String,

        /// Log path for the failure.
        #[arg(long)]
        log: String,

        /// Optional comma-separated member component IDs for interaction failures.
        #[arg(long, value_delimiter = ',')]
        members: Vec<String>,
    },

    /// List active component holds or render a TOML hold file for `build-novos.sh`.
    Holds {
        /// Hold every component in `golden.toml` at its last-known-good version.
        #[arg(long)]
        all_last_good: bool,

        /// Optional output format (`hold-file`).
        #[arg(long, value_enum)]
        format: Option<HoldFormat>,
    },

    /// Promote a passing release candidate to `golden.toml` and write a release record.
    Promote {
        /// Path to the production ISO image matching the active candidate.
        #[arg(long)]
        iso: PathBuf,
    },

    /// Drive deterministic component bisection across a confirmed multi-component failure.
    Bisect {
        #[command(subcommand)]
        action: BisectCommand,
    },

    /// Display a read-only summary of golden, candidate, bisect, holds, failures, and last release.
    Status,

    /// Import legacy `.nebulous/` release state into `.tracker/release/`. User-only: needs an interactive terminal.
    Import {
        /// Source `.nebulous` directory path.
        #[arg(long)]
        from: PathBuf,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum BisectCommand {
    /// Start a new bisect session from the active candidate's confirmed failure and changed set.
    Start,

    /// Plan the next bisect round and emit its hold file.
    Next,

    /// Record the outcome of the active bisect round and advance the suspect queue.
    Record {
        /// Round outcome (`pass` or `fail`).
        result: ResultArg,

        /// Failure classification (required when result is `fail`).
        #[arg(long, required_if_eq("result", "fail"))]
        class: Option<String>,

        /// Classifier rule identifier (required when result is `fail`).
        #[arg(long, required_if_eq("result", "fail"))]
        classifier_rule: Option<String>,

        /// Optional log path for the round.
        #[arg(long)]
        log: Option<String>,
    },

    /// Display the current bisect state without modifying `.tracker/release/bisect.toml`.
    Status,

    /// Remove `.tracker/release/bisect.toml` to abort an active or finished bisect. User-only: needs an interactive terminal.
    Abort,
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use backend_core::release::{
        candidate::PhaseResult,
        ledger::Phase,
        loopstate::{AttemptStatus, Stage},
    };
    use clap::{CommandFactory, Parser, ValueEnum};

    use super::*;

    #[derive(Debug, Parser)]
    #[command(name = "release")]
    struct ReleaseCli {
        #[command(subcommand)]
        cmd: ReleaseCommand,
    }

    #[test]
    fn release_cli_debug_assert_and_subcommand_names() {
        ReleaseCli::command().debug_assert();

        let cmd = ReleaseCli::command();
        let actual: BTreeSet<String> = cmd
            .get_subcommands()
            .map(|sub| sub.get_name().to_string())
            .collect();
        let expected: BTreeSet<String> = [
            "extract-golden",
            "verify-variance",
            "record-attempt",
            "check-guardrail",
            "bless-golden",
            "approve-variance",
            "unlock-loop",
            "pins-from-release",
            "self-test",
            "candidate",
            "record-phase",
            "record-failure",
            "holds",
            "promote",
            "bisect",
            "status",
            "import",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert_eq!(actual.len(), 17);
        assert_eq!(actual, expected);
    }

    #[test]
    fn parse_all_seventeen_release_subcommands() {
        // 1. extract-golden
        let parsed = ReleaseCli::try_parse_from([
            "release",
            "extract-golden",
            "--iso",
            "/tmp/neb-test/live.iso",
            "--output",
            "/tmp/neb-test/golden.toml",
        ])
        .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::ExtractGolden {
                ref iso,
                output: Some(ref out),
            } if iso == Path::new("/tmp/neb-test/live.iso")
                && out == Path::new("/tmp/neb-test/golden.toml")
        ));

        // 2. verify-variance (--repo-only and --iso)
        let parsed =
            ReleaseCli::try_parse_from(["release", "verify-variance", "--repo-only"]).unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::VerifyVariance {
                iso: None,
                repo_only: true,
                manifest: None,
                golden: None,
                approvals: None,
            }
        ));

        let parsed = ReleaseCli::try_parse_from([
            "release",
            "verify-variance",
            "--iso",
            "/tmp/neb-test/live.iso",
            "--manifest",
            "/tmp/neb-test/manifest.toml",
            "--golden",
            "/tmp/neb-test/golden.toml",
            "--approvals",
            "/tmp/neb-test/approvals.toml",
        ])
        .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::VerifyVariance {
                iso: Some(ref iso),
                repo_only: false,
                manifest: Some(_),
                golden: Some(_),
                approvals: Some(_),
            } if iso == Path::new("/tmp/neb-test/live.iso")
        ));

        // 3. record-attempt
        let parsed = ReleaseCli::try_parse_from([
            "release",
            "record-attempt",
            "--item",
            "item-001-example",
            "--stage",
            "live-boot",
            "--status",
            "fail",
        ])
        .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::RecordAttempt {
                ref item,
                stage: StageArg::LiveBoot,
                status: ResultArg::Fail,
                ref error_summary,
                log_file: None,
            } if item == "item-001-example" && error_summary.is_empty()
        ));

        // 4. check-guardrail
        let parsed = ReleaseCli::try_parse_from([
            "release",
            "check-guardrail",
            "--item",
            "item-001-example",
        ])
        .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::CheckGuardrail { ref item } if item == "item-001-example"
        ));

        // 5. bless-golden
        let parsed = ReleaseCli::try_parse_from([
            "release",
            "bless-golden",
            "--iso",
            "/tmp/neb-test/live.iso",
            "--reason",
            "Initial baseline",
        ])
        .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::BlessGolden {
                ref iso,
                ref reason,
            } if iso == Path::new("/tmp/neb-test/live.iso") && reason == "Initial baseline"
        ));

        // 6. approve-variance
        let parsed = ReleaseCli::try_parse_from([
            "release",
            "approve-variance",
            "--item-name",
            "example-pkg",
            "--reason",
            "Retired package",
            "--version",
            "1.0-1",
        ])
        .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::ApproveVariance {
                ref item_name,
                ref reason,
                version: Some(ref ver),
            } if item_name == "example-pkg" && reason == "Retired package" && ver == "1.0-1"
        ));

        // 7. unlock-loop
        let parsed =
            ReleaseCli::try_parse_from(["release", "unlock-loop", "--item", "item-001-example"])
                .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::UnlockLoop { ref item } if item == "item-001-example"
        ));

        // 8. pins-from-release (defaults and with --components)
        let parsed = ReleaseCli::try_parse_from(["release", "pins-from-release"]).unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::PinsFromRelease {
                ref release,
                ref components,
                format: HoldFormat::HoldFile,
            } if release == "latest" && components.is_empty()
        ));

        let parsed = ReleaseCli::try_parse_from([
            "release",
            "pins-from-release",
            "--components",
            "a-pkg,b-pkg",
        ])
        .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::PinsFromRelease {
                ref components,
                format: HoldFormat::HoldFile,
                ..
            } if components == &["a-pkg", "b-pkg"]
        ));

        // 9. self-test
        let parsed = ReleaseCli::try_parse_from(["release", "self-test"]).unwrap();
        assert!(matches!(parsed.cmd, ReleaseCommand::SelfTest));

        // 10. candidate
        let parsed = ReleaseCli::try_parse_from([
            "release",
            "candidate",
            "--iso",
            "/tmp/neb-test/live.iso",
            "--snapshot",
            "20261110T120000Z",
            "--artifact-cache",
            "/tmp/neb-test/cache",
            "--hold-file",
            "/tmp/neb-test/holds.toml",
        ])
        .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::Candidate {
                ref iso,
                snapshot: Some(ref snap),
                artifact_cache: Some(_),
                hold_file: Some(_),
            } if iso == Path::new("/tmp/neb-test/live.iso") && snap == "20261110T120000Z"
        ));

        // 11. record-phase
        let parsed = ReleaseCli::try_parse_from([
            "release",
            "record-phase",
            "vm-live-boot",
            "fail",
            "--log",
            "/tmp/neb-test/a.log",
            "--log",
            "/tmp/neb-test/b.log",
            "--iso",
            "/tmp/neb-test/c.iso",
            "--class",
            "component",
            "--classifier-rule",
            "example-rule",
        ])
        .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::RecordPhase {
                phase: PhaseArg::VmLiveBoot,
                result: ResultArg::Fail,
                ref logs,
                ref iso,
                class: Some(ref class),
                classifier_rule: Some(ref rule),
                verdict: None,
            } if logs.len() == 2
                && iso == Path::new("/tmp/neb-test/c.iso")
                && class == "component"
                && rule == "example-rule"
        ));

        // 12. record-failure
        let parsed = ReleaseCli::try_parse_from([
            "release",
            "record-failure",
            "--component",
            "example-pkg",
            "--version",
            "1.1-1",
            "--phase",
            "build",
            "--class",
            "component",
            "--classifier-rule",
            "example-rule",
            "--log",
            "/tmp/neb-test/build.log",
            "--members",
            "a-pkg,b-pkg",
        ])
        .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::RecordFailure {
                ref component,
                ref version,
                phase: PhaseArg::Build,
                ref class,
                ref classifier_rule,
                ref log,
                ref members,
            } if component == "example-pkg"
                && version == "1.1-1"
                && class == "component"
                && classifier_rule == "example-rule"
                && log == "/tmp/neb-test/build.log"
                && members == &["a-pkg", "b-pkg"]
        ));

        // 13. holds
        let parsed = ReleaseCli::try_parse_from([
            "release",
            "holds",
            "--all-last-good",
            "--format",
            "hold-file",
        ])
        .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::Holds {
                all_last_good: true,
                format: Some(HoldFormat::HoldFile),
            }
        ));

        // 14. promote
        let parsed =
            ReleaseCli::try_parse_from(["release", "promote", "--iso", "/tmp/neb-test/live.iso"])
                .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::Promote { ref iso } if iso == Path::new("/tmp/neb-test/live.iso")
        ));

        // 15. bisect (start, next, record pass, record fail, status, abort)
        let parsed = ReleaseCli::try_parse_from(["release", "bisect", "start"]).unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::Bisect {
                action: BisectCommand::Start
            }
        ));

        let parsed = ReleaseCli::try_parse_from(["release", "bisect", "next"]).unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::Bisect {
                action: BisectCommand::Next
            }
        ));

        let parsed = ReleaseCli::try_parse_from(["release", "bisect", "record", "pass"]).unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::Bisect {
                action: BisectCommand::Record {
                    result: ResultArg::Pass,
                    class: None,
                    classifier_rule: None,
                    log: None,
                }
            }
        ));

        let parsed = ReleaseCli::try_parse_from([
            "release",
            "bisect",
            "record",
            "fail",
            "--class",
            "component",
            "--classifier-rule",
            "example-rule",
            "--log",
            "/tmp/neb-test/round.log",
        ])
        .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::Bisect {
                action: BisectCommand::Record {
                    result: ResultArg::Fail,
                    class: Some(ref c),
                    classifier_rule: Some(ref r),
                    log: Some(ref l),
                }
            } if c == "component" && r == "example-rule" && l == "/tmp/neb-test/round.log"
        ));

        let parsed = ReleaseCli::try_parse_from(["release", "bisect", "status"]).unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::Bisect {
                action: BisectCommand::Status
            }
        ));

        let parsed = ReleaseCli::try_parse_from(["release", "bisect", "abort"]).unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::Bisect {
                action: BisectCommand::Abort
            }
        ));

        // 16. status
        let parsed = ReleaseCli::try_parse_from(["release", "status"]).unwrap();
        assert!(matches!(parsed.cmd, ReleaseCommand::Status));

        // 17. import
        let parsed =
            ReleaseCli::try_parse_from(["release", "import", "--from", "/tmp/neb-test/.nebulous"])
                .unwrap();
        assert!(matches!(
            parsed.cmd,
            ReleaseCommand::Import { ref from } if from == Path::new("/tmp/neb-test/.nebulous")
        ));
    }

    #[test]
    fn release_cli_refusals() {
        // extract-golden without --iso
        assert!(ReleaseCli::try_parse_from(["release", "extract-golden"]).is_err());

        // verify-variance with neither --iso nor --repo-only
        assert!(ReleaseCli::try_parse_from(["release", "verify-variance"]).is_err());

        // verify-variance with both --iso and --repo-only
        assert!(ReleaseCli::try_parse_from([
            "release",
            "verify-variance",
            "--iso",
            "/tmp/neb-test/a.iso",
            "--repo-only",
        ])
        .is_err());

        // record-phase fail without --class (and without --classifier-rule)
        assert!(ReleaseCli::try_parse_from([
            "release",
            "record-phase",
            "build",
            "fail",
            "--log",
            "/tmp/neb-test/x.log",
            "--iso",
            "/tmp/neb-test/y.iso",
        ])
        .is_err());

        // bisect record fail without --class
        assert!(ReleaseCli::try_parse_from(["release", "bisect", "record", "fail"]).is_err());

        // record-attempt with invalid --stage
        assert!(ReleaseCli::try_parse_from([
            "release",
            "record-attempt",
            "--item",
            "item-001-example",
            "--stage",
            "boot",
            "--status",
            "fail",
        ])
        .is_err());

        // record-phase with no --log
        assert!(ReleaseCli::try_parse_from([
            "release",
            "record-phase",
            "build",
            "pass",
            "--iso",
            "/tmp/neb-test/y.iso",
        ])
        .is_err());

        // import without --from
        assert!(ReleaseCli::try_parse_from(["release", "import"]).is_err());
    }

    #[test]
    fn value_enum_from_mappings_match_clap_names() {
        for &val in StageArg::value_variants() {
            let clap_name = val.to_possible_value().unwrap().get_name().to_string();
            let backend_val = Stage::from(val);
            assert_eq!(backend_val.as_str(), clap_name);
        }

        for &val in PhaseArg::value_variants() {
            let clap_name = val.to_possible_value().unwrap().get_name().to_string();
            let backend_val = Phase::from(val);
            assert_eq!(backend_val.as_str(), clap_name);
        }

        for &val in ResultArg::value_variants() {
            let clap_name = val.to_possible_value().unwrap().get_name().to_string();
            let phase_res = PhaseResult::from(val);
            let attempt_status = AttemptStatus::from(val);
            assert_eq!(phase_res.as_str(), clap_name);
            assert_eq!(attempt_status.as_str(), clap_name);
        }

        let hold_variants = HoldFormat::value_variants();
        assert_eq!(hold_variants.len(), 1);
        assert_eq!(
            hold_variants[0].to_possible_value().unwrap().get_name(),
            "hold-file"
        );
    }
}
