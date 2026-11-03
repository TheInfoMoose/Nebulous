use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use backend_core::{
    AgentContextBundle, CreateItemInput, ItemStatus, ItemType, PhaseStatus, PreviousFix,
    ProjectCounters, Result, Severity, TrackerError, TrackerStorage,
};
use serde::Serialize;

use crate::args::{Cli, Commands, PhaseCommand, ScopeCommand, TimerCommand};

/// Executes a parsed [`Cli`] command and prints the resulting output to `stdout`.
pub fn execute(cli: &Cli) -> Result<()> {
    let output = execute_to_string(cli)?;
    if output.ends_with('\n') {
        print!("{output}");
    } else {
        println!("{output}");
    }
    Ok(())
}

/// Executes a parsed [`Cli`] command and returns the formatted output string (human or JSON).
pub fn execute_to_string(cli: &Cli) -> Result<String> {
    match &cli.command {
        Commands::Init {
            name,
            phase_id,
            phase_title,
        } => handle_init(
            cli.root.as_deref(),
            name.as_deref(),
            phase_id,
            phase_title,
            cli.json,
        ),
        Commands::Bug {
            title,
            scope,
            phase,
            severity,
            desc,
        } => {
            let storage = resolve_storage(cli.root.as_deref())?;
            handle_create_item(
                &storage,
                ItemType::Bug,
                title,
                phase.as_deref(),
                scope.as_deref(),
                *severity,
                desc.as_deref(),
                cli.json,
            )
        }
        Commands::Feat {
            title,
            scope,
            phase,
            severity,
            desc,
        } => {
            let storage = resolve_storage(cli.root.as_deref())?;
            handle_create_item(
                &storage,
                ItemType::Feature,
                title,
                phase.as_deref(),
                scope.as_deref(),
                *severity,
                desc.as_deref(),
                cli.json,
            )
        }
        Commands::List {
            scope,
            phase,
            status,
            item_type,
            stagnant,
            recurring,
        } => {
            let storage = resolve_storage(cli.root.as_deref())?;
            handle_list(
                &storage,
                phase.as_deref(),
                scope.as_deref(),
                *status,
                *item_type,
                *stagnant,
                *recurring,
                cli.json,
            )
        }
        Commands::Show { id } => {
            let storage = resolve_storage(cli.root.as_deref())?;
            handle_show(&storage, id, cli.json)
        }
        Commands::Status { id, status } => {
            let storage = resolve_storage(cli.root.as_deref())?;
            handle_status(&storage, id, *status, cli.json)
        }
        Commands::Close {
            id,
            summary,
            commit,
            merged,
        } => {
            let storage = resolve_storage(cli.root.as_deref())?;
            handle_close(&storage, id, summary, commit.as_deref(), *merged, cli.json)
        }
        Commands::Reopen { id, reason } => {
            let storage = resolve_storage(cli.root.as_deref())?;
            handle_reopen(&storage, id, reason.as_deref(), cli.json)
        }
        Commands::Timer { action } => {
            let storage = resolve_storage(cli.root.as_deref())?;
            let resolved_action = action.clone().unwrap_or(TimerCommand::Status { id: None });
            handle_timer(&storage, &resolved_action, cli.json)
        }
        Commands::Log {
            id,
            minutes,
            positional_minutes,
        } => {
            let storage = resolve_storage(cli.root.as_deref())?;
            let mins = minutes.or(*positional_minutes).ok_or_else(|| {
                TrackerError::ValidationError(
                    "Minutes must be provided via -m/--minutes <MINS> or positional <MINUTES>"
                        .to_string(),
                )
            })?;
            handle_log(&storage, id, mins, cli.json)
        }
        Commands::Context {
            id,
            refs_only,
            copy,
        } => {
            let storage = resolve_storage(cli.root.as_deref())?;
            handle_context(&storage, id, *refs_only, *copy, cli.json)
        }
        Commands::Metrics => {
            let storage = resolve_storage(cli.root.as_deref())?;
            handle_metrics(&storage, cli.json)
        }
        Commands::Sync {
            message,
            pat,
            commit_only,
        } => {
            let storage = resolve_storage(cli.root.as_deref())?;
            handle_sync(
                &storage,
                message.as_deref(),
                pat.as_deref(),
                *commit_only,
                cli.json,
            )
        }
        Commands::SyncIndex => {
            let storage = resolve_storage(cli.root.as_deref())?;
            handle_sync_index(&storage, cli.json)
        }
        Commands::Phase { action } => {
            let storage = resolve_storage(cli.root.as_deref())?;
            handle_phase(&storage, action, cli.json)
        }
        Commands::Scope { action } => {
            let storage = resolve_storage(cli.root.as_deref())?;
            handle_scope(&storage, action, cli.json)
        }
    }
}

/// Discovers `.tracker/` from `--root <PATH>` if provided, or by walking up from the current working directory.
pub fn resolve_storage(root: Option<&Path>) -> Result<TrackerStorage> {
    match root {
        Some(explicit_root) => TrackerStorage::discover(explicit_root),
        None => {
            let cwd = std::env::current_dir().map_err(|source| TrackerError::Io {
                path: PathBuf::from("."),
                source,
            })?;
            TrackerStorage::discover(cwd)
        }
    }
}

fn infer_project_name(target_root: &Path) -> String {
    let canonical = target_root
        .canonicalize()
        .unwrap_or_else(|_| target_root.to_path_buf());
    let candidate = if canonical.file_name().and_then(|s| s.to_str()) == Some(".tracker") {
        canonical.parent().unwrap_or(&canonical)
    } else {
        &canonical
    };
    candidate
        .file_name()
        .and_then(|n| n.to_str())
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "." && *s != "/")
        .unwrap_or("Nebulous")
        .to_string()
}

fn to_json_pretty<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string_pretty(value)
        .map_err(|err| TrackerError::ValidationError(format!("JSON serialization error: {err}")))
}

#[derive(Debug, Serialize)]
struct InitOutputJson {
    status: &'static str,
    project_name: String,
    tracker_dir: String,
    active_phase: String,
    default_scope: String,
}

fn handle_init(
    root: Option<&Path>,
    name: Option<&str>,
    phase_id: &str,
    phase_title: &str,
    json: bool,
) -> Result<String> {
    let target_root = match root {
        Some(p) => p.to_path_buf(),
        None => std::env::current_dir().map_err(|source| TrackerError::Io {
            path: PathBuf::from("."),
            source,
        })?,
    };

    let project_name = name
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| infer_project_name(&target_root));

    let storage = TrackerStorage::init(&target_root, &project_name, phase_id, phase_title)?;
    let project = storage.read_project()?;
    let tracker_dir_str = storage.tracker_dir().to_string_lossy().to_string();

    if json {
        return to_json_pretty(&InitOutputJson {
            status: "initialized",
            project_name: project.project.name,
            tracker_dir: tracker_dir_str,
            active_phase: project.project.active_phase,
            default_scope: project.project.default_scope,
        });
    }

    Ok(format!(
        "Initialized Nebulous tracker '{}' at {} (active phase: {}, default scope: {})",
        project.project.name,
        tracker_dir_str,
        project.project.active_phase,
        project.project.default_scope
    ))
}

#[derive(Debug, Serialize)]
struct CreatedItemOutputJson {
    id: String,
    #[serde(rename = "type")]
    item_type: ItemType,
    title: String,
    status: ItemStatus,
    severity: Severity,
    phase_id: String,
    scope_id: String,
    item_toml: String,
    desc_md: String,
}

#[allow(clippy::too_many_arguments)]
fn handle_create_item(
    storage: &TrackerStorage,
    item_type: ItemType,
    title: &str,
    phase: Option<&str>,
    scope: Option<&str>,
    severity: Severity,
    desc: Option<&str>,
    json: bool,
) -> Result<String> {
    let input = CreateItemInput {
        item_type,
        title: title.to_string(),
        phase_id: phase.map(str::to_string),
        scope_id: scope.map(str::to_string),
        severity,
        description: desc.map(str::to_string),
    };

    let (loc, detail) = storage.create_item(input)?;
    let item_toml = storage
        .item_toml_path(&loc.phase_id, &loc.scope_id, &loc.stub.id)?
        .to_string_lossy()
        .to_string();
    let desc_md = storage
        .item_desc_path(&loc.phase_id, &loc.scope_id, &loc.stub.id)?
        .to_string_lossy()
        .to_string();

    if json {
        return to_json_pretty(&CreatedItemOutputJson {
            id: detail.manifest.item.id,
            item_type: detail.manifest.item.item_type,
            title: detail.manifest.item.title,
            status: detail.manifest.item.status,
            severity: detail.manifest.item.severity,
            phase_id: loc.phase_id,
            scope_id: loc.scope_id,
            item_toml,
            desc_md,
        });
    }

    Ok(format!(
        "Created {} {} [{}] in {}/{}: {}",
        detail.manifest.item.item_type,
        detail.manifest.item.id,
        detail.manifest.item.severity,
        loc.phase_id,
        loc.scope_id,
        detail.manifest.item.title
    ))
}

#[derive(Debug, Serialize)]
struct ListItemEntryJson {
    id: String,
    #[serde(rename = "type")]
    item_type: ItemType,
    title: String,
    status: ItemStatus,
    severity: Severity,
    phase_id: String,
    scope_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    days_since_update: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recurrence_count: Option<u32>,
}

#[derive(Debug, Serialize)]
struct ListOutputJson {
    project_name: String,
    active_phase: String,
    total: usize,
    items: Vec<ListItemEntryJson>,
}

#[allow(clippy::too_many_arguments)]
fn handle_list(
    storage: &TrackerStorage,
    phase_filter: Option<&str>,
    scope_filter: Option<&str>,
    status_filter: Option<ItemStatus>,
    type_filter: Option<ItemType>,
    stagnant_only: bool,
    recurring_only: bool,
    json: bool,
) -> Result<String> {
    let index = storage.load_index()?;

    let stagnant_map: HashMap<String, i64> = if stagnant_only {
        storage
            .find_stagnant_items()?
            .into_iter()
            .map(|entry| (entry.item_id, entry.days_since_update))
            .collect()
    } else {
        HashMap::new()
    };

    let recurring_map: HashMap<String, u32> = if recurring_only {
        storage
            .find_recurring_hotspots()?
            .0
            .into_iter()
            .filter(|entry| entry.recurrence_count > 0)
            .map(|entry| (entry.item_id, entry.recurrence_count))
            .collect()
    } else {
        HashMap::new()
    };

    let mut entries = Vec::new();
    for loc in index.all_items() {
        if let Some(p) = phase_filter {
            if loc.phase_id != p {
                continue;
            }
        }
        if let Some(s) = scope_filter {
            if loc.scope_id != s {
                continue;
            }
        }
        if let Some(st) = status_filter {
            if loc.stub.status != st {
                continue;
            }
        }
        if let Some(kind) = type_filter {
            if loc.item_type != kind {
                continue;
            }
        }
        if stagnant_only && !stagnant_map.contains_key(&loc.stub.id) {
            continue;
        }
        if recurring_only && !recurring_map.contains_key(&loc.stub.id) {
            continue;
        }

        entries.push(ListItemEntryJson {
            days_since_update: stagnant_map.get(&loc.stub.id).copied(),
            recurrence_count: recurring_map.get(&loc.stub.id).copied(),
            id: loc.stub.id,
            item_type: loc.item_type,
            title: loc.stub.title,
            status: loc.stub.status,
            severity: loc.stub.severity,
            phase_id: loc.phase_id,
            scope_id: loc.scope_id,
        });
    }

    if json {
        return to_json_pretty(&ListOutputJson {
            project_name: index.project.project.name,
            active_phase: index.project.project.active_phase,
            total: entries.len(),
            items: entries,
        });
    }

    if entries.is_empty() {
        return Ok("No matching items found.".to_string());
    }

    let mut out = String::new();
    let _ = writeln!(
        out,
        "{:<10} {:<8} {:<12} {:<9} {:<16} {:<18} TITLE",
        "ID", "TYPE", "STATUS", "SEVERITY", "PHASE", "SCOPE"
    );
    let _ = writeln!(out, "{}", "-".repeat(96));

    for item in &entries {
        let mut badge_suffix = String::new();
        if let Some(days) = item.days_since_update {
            let _ = write!(badge_suffix, " [stagnant: {days}d]");
        }
        if let Some(rec) = item.recurrence_count {
            let _ = write!(badge_suffix, " [reopened: {rec}x]");
        }
        let _ = writeln!(
            out,
            "{:<10} {:<8} {:<12} {:<9} {:<16} {:<18} {}{}",
            item.id,
            item.item_type.to_string(),
            item.status.to_string(),
            item.severity.to_string(),
            item.phase_id,
            item.scope_id,
            item.title,
            badge_suffix
        );
    }
    let _ = write!(out, "\nTotal: {} item(s)", entries.len());
    Ok(out)
}

#[derive(Debug, Serialize)]
struct ShowOutputJson {
    phase_id: String,
    scope_id: String,
    item: backend_core::ItemMetadata,
    effort: backend_core::ItemEffort,
    history: backend_core::ItemHistory,
    description_markdown: String,
}

fn handle_show(storage: &TrackerStorage, id: &str, json: bool) -> Result<String> {
    let (loc, detail) = storage.load_item_detail_by_id(id)?;

    if json {
        return to_json_pretty(&ShowOutputJson {
            phase_id: loc.phase_id,
            scope_id: loc.scope_id,
            item: detail.manifest.item,
            effort: detail.manifest.effort,
            history: detail.manifest.history,
            description_markdown: detail.description_markdown,
        });
    }

    let item = &detail.manifest.item;
    let effort = &detail.manifest.effort;
    let history = &detail.manifest.history;

    let mut out = String::new();
    let _ = writeln!(out, "{} — {}", item.id, item.title);
    let _ = writeln!(out, "{}", "=".repeat(72));
    let _ = writeln!(out, "Type:             {}", item.item_type);
    let _ = writeln!(out, "Status:           {}", item.status);
    let _ = writeln!(out, "Severity:         {}", item.severity);
    let _ = writeln!(out, "Phase:            {}", loc.phase_id);
    let _ = writeln!(out, "Scope:            {}", loc.scope_id);
    let _ = writeln!(out, "Created At:       {}", item.created_at);
    let _ = writeln!(out, "Updated At:       {}", item.updated_at);
    let _ = writeln!(
        out,
        "Effort Logged:    {}m ({} iteration(s))",
        effort.logged_time_minutes, effort.iteration_count
    );
    let _ = writeln!(out, "Recurrence Count: {}", history.recurrence_count);

    if !history.previous_fixes.is_empty() {
        let _ = writeln!(
            out,
            "\nPrevious Fixes Ledger ({}):",
            history.previous_fixes.len()
        );
        for (idx, fix) in history.previous_fixes.iter().enumerate() {
            let _ = writeln!(
                out,
                "  {}. [{}] commit {} — {}",
                idx + 1,
                fix.date,
                fix.commit,
                fix.summary
            );
        }
    }

    let _ = writeln!(out, "\n--- Specification (desc.md) ---");
    let _ = write!(out, "{}", detail.description_markdown.trim_end());
    Ok(out)
}

#[derive(Debug, Serialize)]
struct StatusUpdateOutputJson {
    id: String,
    status: ItemStatus,
    phase_id: String,
    scope_id: String,
    recurrence_count: u32,
    iteration_count: u32,
    updated_at: String,
}

fn handle_status(
    storage: &TrackerStorage,
    id: &str,
    status: ItemStatus,
    json: bool,
) -> Result<String> {
    let loc = storage.locate_item(id)?;
    let detail = storage.update_item_status(id, status)?;

    if json {
        return to_json_pretty(&StatusUpdateOutputJson {
            id: detail.manifest.item.id,
            status: detail.manifest.item.status,
            phase_id: loc.phase_id,
            scope_id: loc.scope_id,
            recurrence_count: detail.manifest.history.recurrence_count,
            iteration_count: detail.manifest.effort.iteration_count,
            updated_at: detail.manifest.item.updated_at,
        });
    }

    Ok(format!(
        "Updated {} status -> {} (recurrence_count: {}, iteration_count: {})",
        detail.manifest.item.id,
        detail.manifest.item.status,
        detail.manifest.history.recurrence_count,
        detail.manifest.effort.iteration_count
    ))
}

#[derive(Debug, Serialize)]
struct CloseOutputJson {
    id: String,
    status: ItemStatus,
    fix: Option<PreviousFix>,
    previous_fixes_count: usize,
    updated_at: String,
}

fn handle_close(
    storage: &TrackerStorage,
    id: &str,
    summary: &str,
    commit: Option<&str>,
    merged: bool,
    json: bool,
) -> Result<String> {
    let detail = storage.close_item(id, summary, commit, merged)?;
    let latest_fix = detail.manifest.history.previous_fixes.last().cloned();
    let fixes_count = detail.manifest.history.previous_fixes.len();

    if json {
        return to_json_pretty(&CloseOutputJson {
            id: detail.manifest.item.id,
            status: detail.manifest.item.status,
            fix: latest_fix,
            previous_fixes_count: fixes_count,
            updated_at: detail.manifest.item.updated_at,
        });
    }

    let commit_str = latest_fix
        .as_ref()
        .map(|f| f.commit.as_str())
        .unwrap_or("uncommitted");
    let summary_str = latest_fix
        .as_ref()
        .map(|f| f.summary.as_str())
        .unwrap_or(summary);

    Ok(format!(
        "Closed {} as {} (commit: {}, summary: \"{}\")",
        detail.manifest.item.id, detail.manifest.item.status, commit_str, summary_str
    ))
}

#[derive(Debug, Serialize)]
struct ReopenOutputJson {
    id: String,
    status: ItemStatus,
    recurrence_count: u32,
    iteration_count: u32,
    reason: Option<String>,
    updated_at: String,
}

fn handle_reopen(
    storage: &TrackerStorage,
    id: &str,
    reason: Option<&str>,
    json: bool,
) -> Result<String> {
    let detail = storage.reopen_item(id, reason)?;

    if json {
        return to_json_pretty(&ReopenOutputJson {
            id: detail.manifest.item.id,
            status: detail.manifest.item.status,
            recurrence_count: detail.manifest.history.recurrence_count,
            iteration_count: detail.manifest.effort.iteration_count,
            reason: reason.map(str::to_string),
            updated_at: detail.manifest.item.updated_at,
        });
    }

    Ok(format!(
        "Reopened {} -> {} (recurrence_count: {}, iteration_count: {})",
        detail.manifest.item.id,
        detail.manifest.item.status,
        detail.manifest.history.recurrence_count,
        detail.manifest.effort.iteration_count
    ))
}

#[derive(Debug, Serialize)]
struct TimerActionOutputJson {
    action: &'static str,
    active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    item_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    phase_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scope_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elapsed_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elapsed_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_logged_minutes: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

fn handle_timer(storage: &TrackerStorage, action: &TimerCommand, json: bool) -> Result<String> {
    match action {
        TimerCommand::Start { id, note } => {
            let timer = storage.start_timer(id, note.clone())?;
            if json {
                return to_json_pretty(&TimerActionOutputJson {
                    action: "start",
                    active: true,
                    item_id: Some(timer.item_id),
                    phase_id: timer.phase_id,
                    scope_id: timer.scope_id,
                    started_at: Some(timer.started_at),
                    elapsed_minutes: Some(0),
                    elapsed_seconds: Some(0),
                    total_logged_minutes: None,
                    note: timer.note,
                });
            }
            Ok(format!(
                "Started timer for {} at {}",
                timer.item_id, timer.started_at
            ))
        }
        TimerCommand::Stop { id } => {
            if let Some(expected_id) = id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                if let Some(active) = storage.read_active_timer()? {
                    if active.item_id != expected_id {
                        return Err(TrackerError::ValidationError(format!(
                            "Active timer is running for '{}', not '{}'",
                            active.item_id, expected_id
                        )));
                    }
                }
            }
            let session = storage.stop_timer()?;
            if json {
                return to_json_pretty(&TimerActionOutputJson {
                    action: "stop",
                    active: false,
                    item_id: Some(session.timer.item_id),
                    phase_id: session.timer.phase_id,
                    scope_id: session.timer.scope_id,
                    started_at: Some(session.timer.started_at),
                    elapsed_minutes: Some(session.elapsed_minutes),
                    elapsed_seconds: None,
                    total_logged_minutes: Some(session.total_logged_minutes),
                    note: session.timer.note,
                });
            }
            Ok(format!(
                "Stopped timer for {}: +{}m (total logged: {}m)",
                session.timer.item_id, session.elapsed_minutes, session.total_logged_minutes
            ))
        }
        TimerCommand::Status { id } => {
            let status_opt = storage.timer_status()?;
            let filtered = match (
                status_opt,
                id.as_deref().map(str::trim).filter(|s| !s.is_empty()),
            ) {
                (Some(st), Some(expected)) if st.timer.item_id != expected => None,
                (other, _) => other,
            };

            match filtered {
                Some(st) => {
                    if json {
                        return to_json_pretty(&TimerActionOutputJson {
                            action: "status",
                            active: true,
                            item_id: Some(st.timer.item_id),
                            phase_id: st.timer.phase_id,
                            scope_id: st.timer.scope_id,
                            started_at: Some(st.timer.started_at),
                            elapsed_minutes: Some(st.elapsed_minutes),
                            elapsed_seconds: Some(st.elapsed_seconds),
                            total_logged_minutes: None,
                            note: st.timer.note,
                        });
                    }
                    Ok(format!(
                        "Active timer: {} ({}m / {}s elapsed since {})",
                        st.timer.item_id,
                        st.elapsed_minutes,
                        st.elapsed_seconds,
                        st.timer.started_at
                    ))
                }
                None => {
                    if json {
                        return to_json_pretty(&TimerActionOutputJson {
                            action: "status",
                            active: false,
                            item_id: None,
                            phase_id: None,
                            scope_id: None,
                            started_at: None,
                            elapsed_minutes: None,
                            elapsed_seconds: None,
                            total_logged_minutes: None,
                            note: None,
                        });
                    }
                    Ok("No active timer.".to_string())
                }
            }
        }
    }
}

#[derive(Debug, Serialize)]
struct LogEffortOutputJson {
    id: String,
    added_minutes: u32,
    logged_time_minutes: u32,
    iteration_count: u32,
    updated_at: String,
}

fn handle_log(storage: &TrackerStorage, id: &str, minutes: u32, json: bool) -> Result<String> {
    let detail = storage.log_effort(id, minutes)?;

    if json {
        return to_json_pretty(&LogEffortOutputJson {
            id: detail.manifest.item.id,
            added_minutes: minutes,
            logged_time_minutes: detail.manifest.effort.logged_time_minutes,
            iteration_count: detail.manifest.effort.iteration_count,
            updated_at: detail.manifest.item.updated_at,
        });
    }

    Ok(format!(
        "Logged +{}m on {} (total logged: {}m)",
        minutes, detail.manifest.item.id, detail.manifest.effort.logged_time_minutes
    ))
}

#[derive(Debug, Serialize)]
struct ContextOutputJson {
    #[serde(flatten)]
    bundle: AgentContextBundle,
    refs_only: bool,
    output: String,
    copied_to_clipboard: bool,
}

fn handle_context(
    storage: &TrackerStorage,
    id: &str,
    refs_only: bool,
    copy: bool,
    json: bool,
) -> Result<String> {
    let bundle = storage.export_agent_context(id)?;
    let selected_output = if refs_only {
        bundle.file_refs.clone()
    } else {
        bundle.bounded_prompt.clone()
    };

    let copied_to_clipboard = if copy {
        match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(selected_output.clone())) {
            Ok(()) => true,
            Err(err) => {
                eprintln!("Warning: unable to copy context to system clipboard: {err}");
                false
            }
        }
    } else {
        false
    };

    if json {
        return to_json_pretty(&ContextOutputJson {
            bundle,
            refs_only,
            output: selected_output,
            copied_to_clipboard,
        });
    }

    Ok(selected_output)
}

fn handle_metrics(storage: &TrackerStorage, json: bool) -> Result<String> {
    let report = storage.compute_metrics()?;

    if json {
        return to_json_pretty(&report);
    }

    let mut out = String::new();
    let _ = writeln!(out, "=== Nebulous Project Metrics & Valuation ===");
    let _ = writeln!(
        out,
        "Items Completed:      {}/{}",
        report.completed_items, report.total_items
    );
    let _ = writeln!(
        out,
        "Total Effort Logged:  {:.2}h ({}m)",
        report.total_logged_hours, report.total_logged_minutes
    );
    let _ = writeln!(
        out,
        "Target Daily Pace:    {:.2}h/day ({:.2} target days equivalent)",
        report.target_daily_hours, report.target_days_equivalent
    );
    let _ = writeln!(
        out,
        "Stagnant Threshold:   {} day(s)",
        report.stagnant_threshold_days
    );

    let _ = writeln!(out, "\n--- Phase & Scope Time Valuation ---");
    for phase in &report.phases {
        let _ = writeln!(
            out,
            "Phase {} ({}): {}/{} done | {:.2}h ({}m) logged | {:.2} target days",
            phase.phase_id,
            phase.phase_title,
            phase.completed_items,
            phase.total_items,
            phase.logged_hours,
            phase.logged_minutes,
            phase.target_days_equivalent
        );
        for scope in &phase.scopes {
            let _ = writeln!(
                out,
                "  - {:<18} ({:<24}): {}/{} done | {:.2}h ({}m)",
                scope.scope_id,
                scope.target,
                scope.completed_items,
                scope.total_items,
                scope.logged_hours,
                scope.logged_minutes
            );
        }
    }

    let _ = writeln!(
        out,
        "\n--- Stagnant Items ({}) ---",
        report.stagnant_items.len()
    );
    if report.stagnant_items.is_empty() {
        let _ = writeln!(out, "  None.");
    } else {
        for item in &report.stagnant_items {
            let _ = writeln!(
                out,
                "  - {} [{}] ({}/{}) — {}d since update ({}) — {}",
                item.item_id,
                item.severity,
                item.phase_id,
                item.scope_id,
                item.days_since_update,
                item.status,
                item.title
            );
        }
    }

    let _ = writeln!(
        out,
        "\n--- Recurring Bug & Fix Hotspots ({} item(s), {} scope(s)) ---",
        report.item_hotspots.len(),
        report.scope_hotspots.len()
    );
    if report.item_hotspots.is_empty() {
        let _ = write!(out, "  None.");
    } else {
        for hotspot in &report.item_hotspots {
            let _ = writeln!(
                out,
                "  - {} ({}/{}) — recurrences: {}, fixes: {}, iterations: {}, effort: {}m — {}",
                hotspot.item_id,
                hotspot.phase_id,
                hotspot.scope_id,
                hotspot.recurrence_count,
                hotspot.previous_fixes_count,
                hotspot.iteration_count,
                hotspot.logged_time_minutes,
                hotspot.title
            );
        }
    }

    Ok(out.trim_end().to_string())
}

#[derive(Debug, Serialize)]
struct SyncOutputJson {
    commit_only: bool,
    committed_or_head_sha: Option<String>,
    tracker_dirty_after: bool,
}

fn handle_sync(
    storage: &TrackerStorage,
    message: Option<&str>,
    pat: Option<&str>,
    commit_only: bool,
    json: bool,
) -> Result<String> {
    let sha = if commit_only {
        storage.git_commit_tracker(message)?
    } else {
        storage.git_sync(message, pat)?
    };
    let tracker_dirty_after = storage
        .git_status()
        .map(|s| s.tracker_dirty)
        .unwrap_or(false);

    if json {
        return to_json_pretty(&SyncOutputJson {
            commit_only,
            committed_or_head_sha: sha,
            tracker_dirty_after,
        });
    }

    match sha {
        Some(commit_sha) => Ok(format!("Synchronized .tracker/ state (HEAD: {commit_sha})")),
        None => Ok("No .tracker/ changes to commit.".to_string()),
    }
}

#[derive(Debug, Serialize)]
struct SyncIndexOutputJson {
    phases_reconciled: usize,
    scopes_reconciled: usize,
    total_items: u32,
    completed_items: u32,
    counters: ProjectCounters,
}

fn handle_sync_index(storage: &TrackerStorage, json: bool) -> Result<String> {
    let report = storage.reconcile_indexes()?;

    if json {
        return to_json_pretty(&SyncIndexOutputJson {
            phases_reconciled: report.phases_reconciled,
            scopes_reconciled: report.scopes_reconciled,
            total_items: report.total_items,
            completed_items: report.completed_items,
            counters: report.counters,
        });
    }

    Ok(format!(
        "Reconciled .tracker/ indexes: {} phase(s), {} scope(s), {} item(s) ({} completed), counters: bug={}, feature={}",
        report.phases_reconciled,
        report.scopes_reconciled,
        report.total_items,
        report.completed_items,
        report.counters.bug,
        report.counters.feature
    ))
}

#[derive(Debug, Serialize)]
struct PhaseListEntryJson {
    id: String,
    title: String,
    active: bool,
    status: PhaseStatus,
    scopes_count: usize,
    total_items: u32,
    completed_items: u32,
}

#[derive(Debug, Serialize)]
struct PhaseListOutputJson {
    active_phase: String,
    phases: Vec<PhaseListEntryJson>,
}

#[derive(Debug, Serialize)]
struct PhaseMutationOutputJson {
    action: &'static str,
    phase_id: String,
    title: String,
    active_phase: String,
}

fn handle_phase(storage: &TrackerStorage, action: &PhaseCommand, json: bool) -> Result<String> {
    match action {
        PhaseCommand::List => {
            let index = storage.load_index()?;
            let active_id = index.project.project.active_phase.clone();
            let phases: Vec<PhaseListEntryJson> = index
                .phases
                .iter()
                .map(|p| PhaseListEntryJson {
                    id: p.phase_id.clone(),
                    title: p.manifest.phase.title.clone(),
                    active: p.phase_id == active_id,
                    status: p.manifest.phase.status,
                    scopes_count: p.scopes.len(),
                    total_items: p.manifest.summary.total_items,
                    completed_items: p.manifest.summary.completed_items,
                })
                .collect();

            if json {
                return to_json_pretty(&PhaseListOutputJson {
                    active_phase: active_id,
                    phases,
                });
            }

            let mut out = String::new();
            let _ = writeln!(
                out,
                "   {:<18} {:<12} {:<8} {:<10} TITLE",
                "PHASE ID", "STATUS", "SCOPES", "PROGRESS"
            );
            let _ = writeln!(out, "{}", "-".repeat(78));
            for p in &phases {
                let marker = if p.active { "*" } else { " " };
                let progress = format!("{}/{}", p.completed_items, p.total_items);
                let _ = writeln!(
                    out,
                    " {} {:<18} {:<12} {:<8} {:<10} {}",
                    marker,
                    p.id,
                    p.status.to_string(),
                    p.scopes_count,
                    progress,
                    p.title
                );
            }
            Ok(out.trim_end().to_string())
        }
        PhaseCommand::Add {
            id,
            title,
            positional_title,
            set_active,
        } => {
            let resolved_title = title
                .clone()
                .or_else(|| positional_title.clone())
                .unwrap_or_else(|| id.clone());
            let phase_manifest = storage.create_phase(id, &resolved_title, *set_active)?;
            let project = storage.read_project()?;

            if json {
                return to_json_pretty(&PhaseMutationOutputJson {
                    action: "add",
                    phase_id: phase_manifest.phase.id,
                    title: phase_manifest.phase.title,
                    active_phase: project.project.active_phase,
                });
            }

            Ok(format!(
                "Created phase '{}' ({}) [active_phase: {}]",
                phase_manifest.phase.id, phase_manifest.phase.title, project.project.active_phase
            ))
        }
        PhaseCommand::Use { id } => {
            let project = storage.set_active_phase(id)?;
            let phase = storage.read_phase(id)?;

            if json {
                return to_json_pretty(&PhaseMutationOutputJson {
                    action: "use",
                    phase_id: phase.phase.id,
                    title: phase.phase.title,
                    active_phase: project.project.active_phase,
                });
            }

            Ok(format!(
                "Switched active phase to '{}' ({})",
                project.project.active_phase, phase.phase.title
            ))
        }
    }
}

#[derive(Debug, Serialize)]
struct ScopeListEntryJson {
    id: String,
    target: String,
    tech_stack: Vec<String>,
    bugs_count: usize,
    features_count: usize,
    total_items: usize,
}

#[derive(Debug, Serialize)]
struct ScopeListOutputJson {
    phase_id: String,
    scopes: Vec<ScopeListEntryJson>,
}

#[derive(Debug, Serialize)]
struct ScopeAddOutputJson {
    phase_id: String,
    id: String,
    target: String,
    tech_stack: Vec<String>,
    description: Option<String>,
}

fn handle_scope(storage: &TrackerStorage, action: &ScopeCommand, json: bool) -> Result<String> {
    match action {
        ScopeCommand::List { phase } => {
            let index = storage.load_index()?;
            let phase_id = phase
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(&index.project.project.active_phase);

            let phase_idx = index
                .phase(phase_id)
                .ok_or_else(|| TrackerError::NotFound {
                    kind: "Phase",
                    id: phase_id.to_string(),
                })?;

            let scopes: Vec<ScopeListEntryJson> = phase_idx
                .scopes
                .iter()
                .map(|s| ScopeListEntryJson {
                    id: s.scope_id.clone(),
                    target: s.manifest.scope.target.clone(),
                    tech_stack: s.manifest.scope.tech_stack.clone(),
                    bugs_count: s.manifest.items.bugs.len(),
                    features_count: s.manifest.items.features.len(),
                    total_items: s.manifest.total_items(),
                })
                .collect();

            if json {
                return to_json_pretty(&ScopeListOutputJson {
                    phase_id: phase_id.to_string(),
                    scopes,
                });
            }

            let mut out = String::new();
            let _ = writeln!(out, "Phase: {phase_id}");
            let _ = writeln!(
                out,
                "{:<18} {:<26} {:<6} {:<6} TECH STACK",
                "SCOPE ID", "TARGET PATH", "BUGS", "FEATS"
            );
            let _ = writeln!(out, "{}", "-".repeat(78));
            for s in &scopes {
                let stack = if s.tech_stack.is_empty() {
                    "-".to_string()
                } else {
                    s.tech_stack.join(", ")
                };
                let _ = writeln!(
                    out,
                    "{:<18} {:<26} {:<6} {:<6} {}",
                    s.id, s.target, s.bugs_count, s.features_count, stack
                );
            }
            Ok(out.trim_end().to_string())
        }
        ScopeCommand::Add {
            id,
            target,
            positional_target,
            phase,
            tech_stack,
            desc,
        } => {
            let project = storage.read_project()?;
            let resolved_phase = phase
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(&project.project.active_phase)
                .to_string();

            let resolved_target = target
                .clone()
                .or_else(|| positional_target.clone())
                .unwrap_or_else(|| format!("crates/{id}"));

            let cleaned_stack: Vec<String> = tech_stack
                .iter()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            let scope_manifest = storage.create_scope(
                Some(&resolved_phase),
                id,
                &resolved_target,
                cleaned_stack,
                desc.clone(),
            )?;

            if json {
                return to_json_pretty(&ScopeAddOutputJson {
                    phase_id: resolved_phase,
                    id: scope_manifest.scope.id,
                    target: scope_manifest.scope.target,
                    tech_stack: scope_manifest.scope.tech_stack,
                    description: scope_manifest.scope.description,
                });
            }

            Ok(format!(
                "Created scope '{}' (target: '{}') in phase '{}'",
                scope_manifest.scope.id, scope_manifest.scope.target, resolved_phase
            ))
        }
    }
}
