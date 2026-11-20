use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::error::{Result, TrackerError};
use crate::models::{resolve_phase_alias, ItemStatus, ItemType};
use crate::planning::PlanStep;
use crate::storage::{normalize_relative_path_str, ScopeIndex, TrackerStorage};
use crate::workflow::assist::match_file_to_best_scope;
use crate::workflow::CreateItemInput;

/// Summary recorded when an imported plan step is closed because its roadmap row is completed.
pub const PLAN_CLOSE_SUMMARY: &str = "Completed per plan roadmap";

/// Pairing of a roadmap step identifier and its corresponding tracker item identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImportEntry {
    pub step_id: String,
    pub item_id: Option<String>,
}

/// Summary of actions taken (or planned during a dry run) by [`import_plan`].
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ImportReport {
    pub created: Vec<ImportEntry>,
    pub retitled: Vec<ImportEntry>,
    pub closed: Vec<ImportEntry>,
    pub unchanged: Vec<ImportEntry>,
}

/// Upserts task items from parsed roadmap steps by `source_ref`.
///
/// Completed rows never create items; they only close a matching open item. Closed and merged
/// items are never retitled.
///
/// When `write` is `true`, runs inside [`TrackerStorage::with_write_lock`]. When `write` is
/// `false` (dry run), acquires no lock and invokes no write path.
pub fn import_plan(
    storage: &TrackerStorage,
    steps: &[PlanStep],
    phase_id: Option<&str>,
    write: bool,
) -> Result<ImportReport> {
    if write {
        storage.with_write_lock(|s| run(s, steps, phase_id, true))
    } else {
        run(storage, steps, phase_id, false)
    }
}

fn run(
    storage: &TrackerStorage,
    steps: &[PlanStep],
    phase_id: Option<&str>,
    write: bool,
) -> Result<ImportReport> {
    let mut seen_step_ids = HashSet::new();
    for step in steps {
        let step_id = step.step_id.trim();
        if step_id.is_empty() {
            return Err(TrackerError::ValidationError(
                "Plan step_id must not be empty".to_string(),
            ));
        }
        if step.title.trim().is_empty() {
            return Err(TrackerError::ValidationError(format!(
                "Plan step '{step_id}' title must not be empty"
            )));
        }
        if !seen_step_ids.insert(step_id.to_string()) {
            return Err(TrackerError::ValidationError(format!(
                "Duplicate step_id '{step_id}' in plan steps"
            )));
        }
    }

    let index = storage.load_index()?;

    let phase = match phase_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => resolve_phase_alias(raw).to_string(),
        None => resolve_phase_alias(&index.project.project.active_phase).to_string(),
    };

    let phase_index = index.phase(&phase).ok_or_else(|| TrackerError::NotFound {
        kind: "Phase",
        id: phase.clone(),
    })?;

    let default_scope = {
        let trimmed = index.project.project.default_scope.trim();
        if trimmed.is_empty() {
            "inbox".to_string()
        } else {
            trimmed.to_string()
        }
    };

    let mut by_source_ref: HashMap<String, (String, String, ItemStatus)> = HashMap::new();
    for loc in index.all_items() {
        let manifest = storage.read_item_manifest(&loc.phase_id, &loc.scope_id, &loc.stub.id)?;
        if let Some(source_ref) = manifest
            .item
            .source_ref
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            if let Some((existing_id, _, _)) = by_source_ref.insert(
                source_ref.to_string(),
                (
                    manifest.item.id.clone(),
                    manifest.item.title.clone(),
                    manifest.item.status,
                ),
            ) {
                return Err(TrackerError::ValidationError(format!(
                    "Duplicate source_ref '{source_ref}' on items {existing_id} and {}",
                    manifest.item.id
                )));
            }
        }
    }

    let mut report = ImportReport::default();

    for step in steps {
        let step_id = step.step_id.trim();
        let trimmed_title = step.title.trim();

        match by_source_ref.get(step_id) {
            None => {
                if step.completed {
                    continue;
                }
                let item_id = if write {
                    let scope = pick_scope(&phase_index.scopes, &default_scope, &step.target_files);
                    let desc = format_step_description(step);
                    let (_, detail) = storage.create_item(
                        CreateItemInput::new(ItemType::Task, trimmed_title)
                            .with_phase(&phase)
                            .with_scope(scope)
                            .with_description(desc)
                            .with_source_ref(step_id),
                    )?;
                    Some(detail.manifest.item.id)
                } else {
                    None
                };

                report.created.push(ImportEntry {
                    step_id: step.step_id.clone(),
                    item_id,
                });
            }
            Some((existing_id, existing_title, existing_status)) => {
                let needs_retitle =
                    !existing_status.is_completed() && existing_title.trim() != trimmed_title;
                let needs_close = step.completed && !existing_status.is_completed();

                if needs_retitle {
                    if write {
                        storage.update_item_metadata(
                            existing_id,
                            Some(trimmed_title.to_string()),
                            None,
                        )?;
                    }
                    report.retitled.push(ImportEntry {
                        step_id: step.step_id.clone(),
                        item_id: Some(existing_id.clone()),
                    });
                }

                if needs_close {
                    if write {
                        storage.close_item(existing_id, PLAN_CLOSE_SUMMARY, None, false)?;
                    }
                    report.closed.push(ImportEntry {
                        step_id: step.step_id.clone(),
                        item_id: Some(existing_id.clone()),
                    });
                }

                if !needs_retitle && !needs_close {
                    report.unchanged.push(ImportEntry {
                        step_id: step.step_id.clone(),
                        item_id: Some(existing_id.clone()),
                    });
                }
            }
        }
    }

    Ok(report)
}

fn pick_scope(scopes: &[ScopeIndex], default_scope: &str, target_files: &[String]) -> String {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for raw_file in target_files {
        let norm = normalize_relative_path_str(raw_file);
        if norm.is_empty() {
            continue;
        }
        if let Some((scope_id, _)) = match_file_to_best_scope(scopes, default_scope, &norm) {
            *counts.entry(scope_id).or_insert(0) += 1;
        }
    }

    let mut best_scope: Option<&str> = None;
    let mut best_count: usize = 0;
    let mut tied = false;

    for (scope_id, count) in &counts {
        if *count > best_count {
            best_count = *count;
            best_scope = Some(scope_id.as_str());
            tied = false;
        } else if *count == best_count {
            tied = true;
        }
    }

    match best_scope {
        Some(winner) if !tied && best_count > 0 => winner.to_string(),
        _ => default_scope.to_string(),
    }
}

fn format_step_description(step: &PlanStep) -> String {
    let command = step.command.as_deref().unwrap_or("—");
    let order = match step.order {
        Some(n) => n.to_string(),
        None => "completed".to_string(),
    };

    let mut lines = vec![
        format!(
            "Imported from roadmap step `{}`: {}",
            step.step_id,
            step.title.trim()
        ),
        format!("- Step ID: `{}`", step.step_id),
        format!("- Command: `{command}`"),
        format!("- Roadmap order: {order}"),
    ];

    if step.target_files.is_empty() {
        lines.push("- Target files: —".to_string());
    } else {
        lines.push("- Target files:".to_string());
        for file in &step.target_files {
            lines.push(format!("  - `{file}`"));
        }
    }

    lines.join("\n")
}
