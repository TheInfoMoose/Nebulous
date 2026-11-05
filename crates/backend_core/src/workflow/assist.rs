use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, Utc};
use git2::{Repository, Status, StatusOptions};
use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};
use crate::ingestion::ECOSYSTEM_MANIFEST_FILES;
use crate::models::{resolve_phase_alias, ItemStatus, ItemType, Severity, CANONICAL_PHASES};
use crate::storage::id::parse_item_id_number;
use crate::storage::{
    normalize_relative_path, normalize_relative_path_str, PhaseIndex, ScopeIndex, TrackerStorage,
};
use crate::workflow::TimerStatus;

/// Predictive context and inferred default field values for zero-friction CLI/TUI workflows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PredictiveDefaults {
    pub active_phase: String,
    pub inferred_scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recommended_item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_timer: Option<TimerStatus>,
    pub head_short_sha: String,
}

/// Category of an actionable recommendation in [`WhatsNextReport`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextActionKind {
    /// Workspace has crates/components on disk not yet promoted to domain scopes (only `inbox` exists).
    UnscopedWorkspace,
    /// An item is currently `in_progress` or actively being timed in `.timer.toml`.
    ActiveFocus,
    /// Open item with `recurrence_count > 0` (regression / recurring bug hotspot).
    RecurringBug,
    /// Non-completed item whose `updated_at` exceeds `stagnant_item_threshold_days`.
    StagnantItem,
    /// Highest-priority `open` item in the active stage (prioritizing `inferred_scope`, severity, and bugs).
    PrioritizedOpenItem,
    /// All items in `active_phase` are completed (`closed` or `merged`), suggesting stage advancement.
    AdvanceStage,
}

impl fmt::Display for NextActionKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnscopedWorkspace => write!(f, "unscoped_workspace"),
            Self::ActiveFocus => write!(f, "active_focus"),
            Self::RecurringBug => write!(f, "recurring_bug"),
            Self::StagnantItem => write!(f, "stagnant_item"),
            Self::PrioritizedOpenItem => write!(f, "prioritized_open_item"),
            Self::AdvanceStage => write!(f, "advance_stage"),
        }
    }
}

/// Single ranked recommendation entry in a [`WhatsNextReport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NextRecommendation {
    pub rank: usize,
    pub kind: NextActionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_type: Option<ItemType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub severity: Option<Severity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<ItemStatus>,
    #[serde(default)]
    pub recurrence_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub days_since_update: Option<i64>,
    pub summary: String,
    pub suggested_command: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_fields: Vec<String>,
}

/// Comprehensive "What's Next" report combining inferred scope, active phase/timer state,
/// ranked recommendations, stage advancement guidance, and missing specification hints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WhatsNextReport {
    pub active_phase: String,
    pub inferred_scope: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_timer: Option<TimerStatus>,
    pub recommendations: Vec<NextRecommendation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stage_advancement_hint: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub missing_fields: Vec<String>,
}

impl WhatsNextReport {
    /// Returns the ID of the top-ranked recommended work item, if any.
    pub fn top_item_id(&self) -> Option<&str> {
        self.recommendations
            .iter()
            .find_map(|r| r.item_id.as_deref())
    }

    /// Returns the ID of the top-ranked `open` work item, if any.
    pub fn top_open_item_id(&self) -> Option<&str> {
        self.recommendations
            .iter()
            .find(|r| r.status == Some(ItemStatus::Open))
            .and_then(|r| r.item_id.as_deref())
    }
}

impl TrackerStorage {
    /// Infers the most relevant domain `scope_id` for the current developer/agent context:
    ///
    /// 1. If `cwd` is inside a scope's `target` directory (longest prefix match among scopes in the active phase), returns that `scope_id`.
    /// 2. Otherwise inspects dirty/modified files in Git (`git2::Repository::open(project_root)`) and matches modified file paths against registered scope `target` paths (most modified files wins).
    /// 3. Otherwise if an active timer is running (`.timer.toml`) with a `scope_id`, returns that `scope_id`.
    /// 4. Otherwise if the active phase has exactly one non-`inbox` scope, returns that scope.
    /// 5. Falls back to `project.default_scope` (`"inbox"`).
    pub fn infer_scope_from_context(&self, cwd: Option<&Path>) -> Result<String> {
        let index = self.load_index()?;
        let default_scope = {
            let trimmed = index.project.project.default_scope.trim();
            if trimmed.is_empty() {
                "inbox".to_string()
            } else {
                trimmed.to_string()
            }
        };

        let active_phase_id = resolve_phase_alias(&index.project.project.active_phase);
        let active_phase = index
            .phase(active_phase_id)
            .or_else(|| index.active_phase())
            .or_else(|| index.phases.first());

        let Some(phase_idx) = active_phase else {
            return Ok(default_scope);
        };

        let project_root = self.project_root();

        // 1. If `cwd` is inside a scope's `target` directory (longest prefix match among scopes in the active phase)
        if let Some(cwd_path) = cwd {
            if let Some(rel_cwd) = relative_path_within_project(&project_root, cwd_path) {
                if let Some(matched_scope) = match_scope_by_longest_target_prefix(
                    &phase_idx.scopes,
                    &default_scope,
                    &rel_cwd,
                ) {
                    return Ok(matched_scope);
                }
            }
        }

        // 2. Otherwise inspect dirty/modified files in Git and match against registered scope `target` paths (most modified files wins)
        if let Some(git_matched_scope) =
            infer_scope_from_git_dirty_files(&project_root, &phase_idx.scopes, &default_scope)
        {
            return Ok(git_matched_scope);
        }

        // 3. Otherwise if an active timer is running (`.timer.toml`) with a `scope_id`, return that `scope_id`
        if let Some(timer) = self.read_active_timer()? {
            if let Some(scope_id) = timer
                .scope_id
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                if phase_idx.scope(scope_id).is_some() {
                    return Ok(scope_id.to_string());
                }
            }
            if let Ok(loc) = self.locate_item(&timer.item_id) {
                if !loc.scope_id.trim().is_empty() {
                    return Ok(loc.scope_id);
                }
            }
        }

        // 4. Otherwise if the active phase has exactly one non-`inbox` scope, return that scope
        let non_inbox_scopes: Vec<&ScopeIndex> = phase_idx
            .scopes
            .iter()
            .filter(|s| s.scope_id != "inbox" && s.scope_id != default_scope)
            .collect();
        if non_inbox_scopes.len() == 1 {
            return Ok(non_inbox_scopes[0].scope_id.clone());
        }

        // 5. Fall back to `project.default_scope` (`"inbox"`)
        Ok(default_scope)
    }

    /// Resolves an explicit or inferred work item ID for zero-ID commands (`neb show`, `neb done`, `neb context`):
    ///
    /// 1. If `explicit_id` is `Some(id)` and non-empty, normalizes case (`id.trim().to_ascii_uppercase()`) and returns it.
    /// 2. If `.timer.toml` has an active timer, returns `timer.item_id`.
    /// 3. If there is a single `ItemStatus::InProgress` item in the active phase (or across the project if none in active phase), returns its ID.
    /// 4. Otherwise returns a descriptive [`TrackerError::ValidationError`] explaining that no active timer or single `in_progress` item was found and suggesting the top recommended item ID.
    pub fn resolve_active_or_inferred_item_id(&self, explicit_id: Option<&str>) -> Result<String> {
        if let Some(raw) = explicit_id.map(str::trim).filter(|s| !s.is_empty()) {
            return Ok(raw.to_ascii_uppercase());
        }

        if let Some(timer) = self.read_active_timer()? {
            let trimmed = timer.item_id.trim();
            if !trimmed.is_empty() {
                return Ok(trimmed.to_ascii_uppercase());
            }
        }

        let index = self.load_index()?;
        let active_phase_id = resolve_phase_alias(&index.project.project.active_phase);
        if let Some(active_phase) = index
            .phase(active_phase_id)
            .or_else(|| index.active_phase())
        {
            let in_progress_in_active: Vec<String> = active_phase
                .scopes
                .iter()
                .flat_map(|s| {
                    s.manifest
                        .items
                        .bugs
                        .iter()
                        .chain(s.manifest.items.features.iter())
                })
                .filter(|stub| stub.status == ItemStatus::InProgress)
                .map(|stub| stub.id.clone())
                .collect();

            if in_progress_in_active.len() == 1 {
                return Ok(in_progress_in_active[0].clone());
            }
            if in_progress_in_active.len() > 1 {
                let suggested = self
                    .compute_whats_next(None)
                    .ok()
                    .and_then(|r| r.top_item_id().map(str::to_string))
                    .unwrap_or_else(|| in_progress_in_active[0].clone());
                return Err(TrackerError::ValidationError(format!(
                    "No active timer found and multiple in_progress items exist in '{}' ({}). Specify an explicit item ID (top recommended: {suggested}).",
                    active_phase.phase_id,
                    in_progress_in_active.join(", ")
                )));
            }
        }

        let all_in_progress: Vec<String> = index
            .all_items()
            .into_iter()
            .filter(|loc| loc.stub.status == ItemStatus::InProgress)
            .map(|loc| loc.stub.id)
            .collect();

        if all_in_progress.len() == 1 {
            return Ok(all_in_progress[0].clone());
        }

        let suggestion_suffix = match self
            .compute_whats_next(None)
            .ok()
            .and_then(|r| r.top_item_id().map(str::to_string))
        {
            Some(top_id) => format!(
                " Top recommended item: {top_id} (run `neb start {top_id}` or pass `{top_id}` explicitly)."
            ),
            None => " No open work items are currently available.".to_string(),
        };

        if all_in_progress.len() > 1 {
            Err(TrackerError::ValidationError(format!(
                "No active timer or single in_progress item found (multiple in_progress items: {}).{}",
                all_in_progress.join(", "),
                suggestion_suffix
            )))
        } else {
            Err(TrackerError::ValidationError(format!(
                "No active timer or single in_progress item found.{}",
                suggestion_suffix
            )))
        }
    }

    /// Computes predictive defaults (`inferred_scope`, `active_phase`, `active_item_id`, `recommended_item_id`, `active_timer`, and `head_short_sha`).
    pub fn predict_defaults(&self, cwd: Option<&Path>) -> Result<PredictiveDefaults> {
        let whats_next = self.compute_whats_next(cwd)?;
        let active_item_id = self.resolve_active_or_inferred_item_id(None).ok();
        let recommended_item_id = whats_next.top_item_id().map(str::to_string);
        let head_short_sha = self.git_head_short_sha();

        Ok(PredictiveDefaults {
            active_phase: whats_next.active_phase,
            inferred_scope: whats_next.inferred_scope,
            active_item_id,
            recommended_item_id,
            active_timer: whats_next.active_timer,
            head_short_sha,
        })
    }

    /// Evaluates current project state and produces a ranked [`WhatsNextReport`] of actionable recommendations.
    pub fn compute_whats_next(&self, cwd: Option<&Path>) -> Result<WhatsNextReport> {
        self.compute_whats_next_at(cwd, Utc::now())
    }

    /// Evaluates current project state at `now` and produces a ranked [`WhatsNextReport`].
    pub fn compute_whats_next_at(
        &self,
        cwd: Option<&Path>,
        now: DateTime<Utc>,
    ) -> Result<WhatsNextReport> {
        let index = self.load_index()?;
        let default_scope = {
            let trimmed = index.project.project.default_scope.trim();
            if trimmed.is_empty() {
                "inbox".to_string()
            } else {
                trimmed.to_string()
            }
        };
        let active_phase_id = resolve_phase_alias(&index.project.project.active_phase).to_string();
        let inferred_scope = self.infer_scope_from_context(cwd)?;
        let active_timer = self.timer_status_at(now)?;
        let stagnant_threshold_days = index.project.metrics.stagnant_item_threshold_days;

        let mut recommendations: Vec<NextRecommendation> = Vec::new();
        let mut seen_items: BTreeSet<String> = BTreeSet::new();

        let active_phase_opt = index
            .phase(&active_phase_id)
            .or_else(|| index.active_phase())
            .or_else(|| index.phases.first());

        // 1. Unscoped Workspace Warning: if only `inbox` exists in active phase and workspace has crates/components on disk
        if let Some(phase_idx) = active_phase_opt {
            let only_inbox_exists = phase_idx
                .scopes
                .iter()
                .all(|s| s.scope_id == "inbox" || s.scope_id == default_scope);
            if only_inbox_exists
                && (!index.project.components.registry.is_empty()
                    || workspace_has_unscoped_components_on_disk(&self.project_root()))
            {
                recommendations.push(NextRecommendation {
                    rank: recommendations.len() + 1,
                    kind: NextActionKind::UnscopedWorkspace,
                    item_id: None,
                    title: None,
                    item_type: None,
                    phase_id: Some(phase_idx.phase_id.clone()),
                    scope_id: Some(default_scope.clone()),
                    severity: None,
                    status: None,
                    recurrence_count: 0,
                    days_since_update: None,
                    summary: "Workspace has crates/components on disk not yet promoted to domain scopes (only 'inbox' exists). Run `neb init` or `neb ingest --promote-all`.".to_string(),
                    suggested_command: "neb ingest --promote-all".to_string(),
                    missing_fields: Vec::new(),
                });
            }
        }

        // Load full manifests for non-completed items across the project
        #[derive(Clone)]
        struct CandidateItem {
            phase_id: String,
            scope_id: String,
            id: String,
            title: String,
            item_type: ItemType,
            status: ItemStatus,
            severity: Severity,
            recurrence_count: u32,
            days_since_update: i64,
        }

        let mut candidates = Vec::new();
        for phase_idx in &index.phases {
            for scope_idx in &phase_idx.scopes {
                let all_stubs = scope_idx
                    .manifest
                    .items
                    .bugs
                    .iter()
                    .chain(scope_idx.manifest.items.features.iter());
                for stub in all_stubs {
                    if stub.status.is_completed() {
                        continue;
                    }
                    let Ok(manifest) =
                        self.read_item_manifest(&phase_idx.phase_id, &scope_idx.scope_id, &stub.id)
                    else {
                        continue;
                    };
                    if manifest.item.status.is_completed() {
                        continue;
                    }
                    let days_since_update = manifest
                        .item
                        .parsed_updated_at()
                        .map(|dt| (now - dt).num_days())
                        .unwrap_or(0);

                    candidates.push(CandidateItem {
                        phase_id: phase_idx.phase_id.clone(),
                        scope_id: scope_idx.scope_id.clone(),
                        id: manifest.item.id,
                        title: manifest.item.title,
                        item_type: manifest.item.item_type,
                        status: manifest.item.status,
                        severity: manifest.item.severity,
                        recurrence_count: manifest.history.recurrence_count,
                        days_since_update,
                    });
                }
            }
        }

        // 2. Active Timer / In-Progress Focus
        if let Some(ref timer_st) = active_timer {
            let timed_id = &timer_st.timer.item_id;
            if let Some(c) = candidates.iter().find(|c| &c.id == timed_id) {
                seen_items.insert(c.id.clone());
                recommendations.push(NextRecommendation {
                    rank: recommendations.len() + 1,
                    kind: NextActionKind::ActiveFocus,
                    item_id: Some(c.id.clone()),
                    title: Some(c.title.clone()),
                    item_type: Some(c.item_type),
                    phase_id: Some(c.phase_id.clone()),
                    scope_id: Some(c.scope_id.clone()),
                    severity: Some(c.severity),
                    status: Some(c.status),
                    recurrence_count: c.recurrence_count,
                    days_since_update: Some(c.days_since_update),
                    summary: format!(
                        "Active timer running on {} ({}m elapsed): {} — run `neb done` when finished or `neb context --copy`.",
                        c.id, timer_st.elapsed_minutes, c.title
                    ),
                    suggested_command: format!("neb done {} -m \"Resolved {}\"", c.id, c.id),
                    missing_fields: Vec::new(),
                });
            }
        }

        let mut in_progress_items: Vec<&CandidateItem> = candidates
            .iter()
            .filter(|c| c.status == ItemStatus::InProgress && !seen_items.contains(&c.id))
            .collect();
        in_progress_items.sort_by(|a, b| {
            let a_in_active = a.phase_id == active_phase_id;
            let b_in_active = b.phase_id == active_phase_id;
            b_in_active
                .cmp(&a_in_active)
                .then_with(|| (b.scope_id == inferred_scope).cmp(&(a.scope_id == inferred_scope)))
                .then_with(|| b.severity.cmp(&a.severity))
                .then_with(|| compare_item_ids(&a.id, &b.id))
        });

        for c in in_progress_items {
            seen_items.insert(c.id.clone());
            recommendations.push(NextRecommendation {
                rank: recommendations.len() + 1,
                kind: NextActionKind::ActiveFocus,
                item_id: Some(c.id.clone()),
                title: Some(c.title.clone()),
                item_type: Some(c.item_type),
                phase_id: Some(c.phase_id.clone()),
                scope_id: Some(c.scope_id.clone()),
                severity: Some(c.severity),
                status: Some(c.status),
                recurrence_count: c.recurrence_count,
                days_since_update: Some(c.days_since_update),
                summary: format!(
                    "In-progress focus in '{}': {} ({}) — run `neb done` when finished or `neb context --copy`.",
                    c.scope_id, c.title, c.id
                ),
                suggested_command: format!("neb done {} -m \"Resolved {}\"", c.id, c.id),
                missing_fields: Vec::new(),
            });
        }

        // 3. Recurring Critical/High Bugs (open items with recurrence_count > 0, ranked by highest severity, then highest recurrence)
        let mut recurring_open: Vec<&CandidateItem> = candidates
            .iter()
            .filter(|c| {
                c.status == ItemStatus::Open
                    && c.recurrence_count > 0
                    && !seen_items.contains(&c.id)
            })
            .collect();
        recurring_open.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| b.recurrence_count.cmp(&a.recurrence_count))
                .then_with(|| {
                    (a.item_type == ItemType::Bug)
                        .cmp(&(b.item_type == ItemType::Bug))
                        .reverse()
                })
                .then_with(|| compare_item_ids(&a.id, &b.id))
        });

        for c in recurring_open {
            seen_items.insert(c.id.clone());
            recommendations.push(NextRecommendation {
                rank: recommendations.len() + 1,
                kind: NextActionKind::RecurringBug,
                item_id: Some(c.id.clone()),
                title: Some(c.title.clone()),
                item_type: Some(c.item_type),
                phase_id: Some(c.phase_id.clone()),
                scope_id: Some(c.scope_id.clone()),
                severity: Some(c.severity),
                status: Some(c.status),
                recurrence_count: c.recurrence_count,
                days_since_update: Some(c.days_since_update),
                summary: format!(
                    "Recurring {} ({}, recurrence_count = {}): {}",
                    c.item_type, c.severity, c.recurrence_count, c.title
                ),
                suggested_command: format!("neb start {}", c.id),
                missing_fields: Vec::new(),
            });
        }

        // 4. Stagnant Items: open/in-progress/pending items exceeding `stagnant_item_threshold_days`
        let mut stagnant: Vec<&CandidateItem> = candidates
            .iter()
            .filter(|c| {
                c.days_since_update >= i64::from(stagnant_threshold_days)
                    && !seen_items.contains(&c.id)
            })
            .collect();
        stagnant.sort_by(|a, b| {
            b.days_since_update
                .cmp(&a.days_since_update)
                .then_with(|| b.severity.cmp(&a.severity))
                .then_with(|| compare_item_ids(&a.id, &b.id))
        });

        for c in stagnant {
            seen_items.insert(c.id.clone());
            recommendations.push(NextRecommendation {
                rank: recommendations.len() + 1,
                kind: NextActionKind::StagnantItem,
                item_id: Some(c.id.clone()),
                title: Some(c.title.clone()),
                item_type: Some(c.item_type),
                phase_id: Some(c.phase_id.clone()),
                scope_id: Some(c.scope_id.clone()),
                severity: Some(c.severity),
                status: Some(c.status),
                recurrence_count: c.recurrence_count,
                days_since_update: Some(c.days_since_update),
                summary: format!(
                    "Stagnant item ({} days since update, {}): {}",
                    c.days_since_update, c.severity, c.title
                ),
                suggested_command: format!("neb start {}", c.id),
                missing_fields: Vec::new(),
            });
        }

        // 5. Highest-Severity Open Items in Active Stage (prioritizing inferred scope):
        // Rank remaining `open` items in `active_phase` by `(scope == inferred_scope, severity DESC, item_type == Bug, id ASC)`
        let mut active_stage_open: Vec<&CandidateItem> = candidates
            .iter()
            .filter(|c| {
                c.phase_id == active_phase_id
                    && c.status == ItemStatus::Open
                    && !seen_items.contains(&c.id)
            })
            .collect();
        active_stage_open.sort_by(|a, b| {
            let a_in_scope = a.scope_id == inferred_scope;
            let b_in_scope = b.scope_id == inferred_scope;
            b_in_scope
                .cmp(&a_in_scope)
                .then_with(|| b.severity.cmp(&a.severity))
                .then_with(|| (b.item_type == ItemType::Bug).cmp(&(a.item_type == ItemType::Bug)))
                .then_with(|| compare_item_ids(&a.id, &b.id))
        });

        for c in active_stage_open {
            seen_items.insert(c.id.clone());
            recommendations.push(NextRecommendation {
                rank: recommendations.len() + 1,
                kind: NextActionKind::PrioritizedOpenItem,
                item_id: Some(c.id.clone()),
                title: Some(c.title.clone()),
                item_type: Some(c.item_type),
                phase_id: Some(c.phase_id.clone()),
                scope_id: Some(c.scope_id.clone()),
                severity: Some(c.severity),
                status: Some(c.status),
                recurrence_count: c.recurrence_count,
                days_since_update: Some(c.days_since_update),
                summary: format!(
                    "Open {} in '{}' [{}]: {}",
                    c.item_type, c.scope_id, c.severity, c.title
                ),
                suggested_command: format!("neb start {}", c.id),
                missing_fields: Vec::new(),
            });
        }

        // 6. Stage Advancement Hint: if `active_phase` has `total_items > 0` and all items in `active_phase` are completed (`closed` or `merged`)
        let mut stage_advancement_hint = None;
        if let Some(phase_idx) = active_phase_opt {
            let (total_in_stage, completed_in_stage) = count_phase_items(phase_idx);
            if total_in_stage > 0 && total_in_stage == completed_in_stage {
                let next_stage = suggest_next_stage(
                    &phase_idx.phase_id,
                    &index
                        .project
                        .phases
                        .registry
                        .iter()
                        .map(|p| p.id.as_str())
                        .collect::<Vec<_>>(),
                );
                let hint_msg = match next_stage.as_deref() {
                    Some(next_id) => format!(
                        "All {total_in_stage} item(s) in stage '{}' are completed. Advance to '{}' (`neb stage {next_id}`) or move ready items forward.",
                        phase_idx.phase_id, next_id
                    ),
                    None => format!(
                        "All {total_in_stage} item(s) in final stage '{}' are completed.",
                        phase_idx.phase_id
                    ),
                };
                let cmd = match next_stage.as_deref() {
                    Some(next_id) => format!("neb stage {next_id}"),
                    None => "neb metrics".to_string(),
                };
                stage_advancement_hint = Some(hint_msg.clone());
                recommendations.push(NextRecommendation {
                    rank: recommendations.len() + 1,
                    kind: NextActionKind::AdvanceStage,
                    item_id: None,
                    title: None,
                    item_type: None,
                    phase_id: next_stage.or_else(|| Some(phase_idx.phase_id.clone())),
                    scope_id: None,
                    severity: None,
                    status: None,
                    recurrence_count: 0,
                    days_since_update: None,
                    summary: hint_msg,
                    suggested_command: cmd,
                    missing_fields: Vec::new(),
                });
            }
        }

        // 7. Missing Spec Guidance: For the top recommended item, inspect `desc.md`;
        // if it still contains the default placeholder ("Describe the problem or specification for:"
        // or "TODO: Add reproduction"), include a field-completion hint (`missing_fields: Vec<String>`).
        let mut top_missing_fields = Vec::new();
        if let Some(top_rec) = recommendations.iter_mut().find(|r| r.item_id.is_some()) {
            if let (Some(phase_id), Some(scope_id), Some(item_id)) = (
                top_rec.phase_id.as_deref(),
                top_rec.scope_id.as_deref(),
                top_rec.item_id.as_deref(),
            ) {
                if let Ok(desc) = self.read_item_desc(phase_id, scope_id, item_id) {
                    top_missing_fields = detect_missing_spec_fields(&desc);
                    if !top_missing_fields.is_empty() {
                        top_rec.missing_fields = top_missing_fields.clone();
                        top_rec.summary.push_str(&format!(
                            " [Missing spec fields: {} — fill reproduction steps or acceptance criteria in desc.md]",
                            top_missing_fields.join(", ")
                        ));
                    }
                }
            }
        }

        for (idx, rec) in recommendations.iter_mut().enumerate() {
            rec.rank = idx + 1;
        }

        Ok(WhatsNextReport {
            active_phase: active_phase_id,
            inferred_scope,
            active_timer,
            recommendations,
            stage_advancement_hint,
            missing_fields: top_missing_fields,
        })
    }
}

fn detect_missing_spec_fields(desc: &str) -> Vec<String> {
    let mut missing = Vec::new();
    if desc.contains("Describe the problem or specification for:") {
        missing.push("problem_description".to_string());
        missing.push("acceptance_criteria".to_string());
    }
    if desc.contains("TODO: Add reproduction") {
        missing.push("reproduction_steps".to_string());
    }
    missing
}

fn count_phase_items(phase_idx: &PhaseIndex) -> (u32, u32) {
    let mut total = 0u32;
    let mut completed = 0u32;
    for scope in &phase_idx.scopes {
        for bug in &scope.manifest.items.bugs {
            total += 1;
            if bug.status.is_completed() {
                completed += 1;
            }
        }
        for feat in &scope.manifest.items.features {
            total += 1;
            if feat.status.is_completed() {
                completed += 1;
            }
        }
    }
    if total == 0 && phase_idx.manifest.summary.total_items > 0 {
        return (
            phase_idx.manifest.summary.total_items,
            phase_idx.manifest.summary.completed_items,
        );
    }
    (total, completed)
}

fn suggest_next_stage(current_phase_id: &str, registered_phases: &[&str]) -> Option<String> {
    let resolved = resolve_phase_alias(current_phase_id);
    match resolved {
        "pre_production" => Some("production".to_string()),
        "production" => Some("post_production".to_string()),
        "post_production" => None,
        _ => {
            if let Some(pos) = registered_phases
                .iter()
                .position(|p| *p == current_phase_id)
            {
                if pos + 1 < registered_phases.len() {
                    return Some(registered_phases[pos + 1].to_string());
                }
            }
            CANONICAL_PHASES
                .iter()
                .find(|(id, _)| *id == "production")
                .map(|(id, _)| (*id).to_string())
        }
    }
}

fn compare_item_ids(a: &str, b: &str) -> Ordering {
    match (parse_item_id_number(a), parse_item_id_number(b)) {
        (Some((type_a, num_a)), Some((type_b, num_b))) if type_a == type_b => {
            num_a.cmp(&num_b).then_with(|| a.cmp(b))
        }
        _ => a.cmp(b),
    }
}

fn relative_path_within_project(project_root: &Path, cwd: &Path) -> Option<String> {
    if cwd.is_relative() {
        let norm = normalize_relative_path(cwd);
        if !norm.starts_with("..") {
            return Some(if norm.is_empty() {
                ".".to_string()
            } else {
                norm
            });
        }
    }

    let lex_root = lexical_clean(project_root);
    let lex_cwd = lexical_clean(cwd);
    if let Ok(stripped) = lex_cwd.strip_prefix(&lex_root) {
        let norm = normalize_relative_path(stripped);
        return Some(if norm.is_empty() {
            ".".to_string()
        } else {
            norm
        });
    }

    let canon_root = fs::canonicalize(project_root).ok()?;
    let canon_cwd = canonicalize_with_nonexistent_tail(cwd)?;
    let stripped = canon_cwd.strip_prefix(&canon_root).ok()?;
    let norm = normalize_relative_path(stripped);
    Some(if norm.is_empty() {
        ".".to_string()
    } else {
        norm
    })
}

fn canonicalize_with_nonexistent_tail(path: &Path) -> Option<PathBuf> {
    let mut current = path;
    let mut tail_components = Vec::new();
    while !current.exists() {
        let file_name = current.file_name()?;
        tail_components.push(file_name.to_os_string());
        current = current.parent()?;
    }
    let mut base = fs::canonicalize(current).ok()?;
    for comp in tail_components.into_iter().rev() {
        base.push(comp);
    }
    Some(base)
}

fn lexical_clean(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn match_scope_by_longest_target_prefix(
    scopes: &[ScopeIndex],
    default_scope: &str,
    rel_path: &str,
) -> Option<String> {
    let norm_rel = normalize_relative_path_str(rel_path);
    if norm_rel.is_empty() {
        return None;
    }

    let mut best_match: Option<(usize, &str)> = None;
    for scope_idx in scopes {
        if scope_idx.scope_id == "inbox" || scope_idx.scope_id == default_scope {
            continue;
        }
        let target = normalize_relative_path_str(&scope_idx.manifest.scope.target);
        if target.is_empty() || target == ".tracker" {
            continue;
        }

        let matched_len = if target == "." {
            if norm_rel == "." {
                Some(1)
            } else {
                None
            }
        } else if norm_rel == target || norm_rel.starts_with(&format!("{target}/")) {
            Some(target.len())
        } else {
            None
        };

        if let Some(len) = matched_len {
            match best_match {
                None => best_match = Some((len, &scope_idx.scope_id)),
                Some((best_len, best_id)) => {
                    if len > best_len || (len == best_len && scope_idx.scope_id.as_str() < best_id)
                    {
                        best_match = Some((len, &scope_idx.scope_id));
                    }
                }
            }
        }
    }

    best_match.map(|(_, id)| id.to_string())
}

fn infer_scope_from_git_dirty_files(
    project_root: &Path,
    scopes: &[ScopeIndex],
    default_scope: &str,
) -> Option<String> {
    let repo = Repository::open(project_root)
        .or_else(|_| Repository::discover(project_root))
        .ok()?;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);
    let statuses = repo.statuses(Some(&mut opts)).ok()?;

    let mut counts: BTreeMap<String, (usize, usize)> = BTreeMap::new();

    for entry in statuses.iter() {
        let st = entry.status();
        if st == Status::CURRENT || st.contains(Status::IGNORED) {
            continue;
        }
        let Some(raw_path) = entry.path() else {
            continue;
        };
        let norm = normalize_relative_path_str(raw_path);
        if norm.is_empty()
            || norm == ".tracker"
            || norm.starts_with(".tracker/")
            || norm.contains(".tmp.")
        {
            continue;
        }

        if let Some((scope_id, target_len)) = match_file_to_best_scope(scopes, default_scope, &norm)
        {
            let entry_count = counts.entry(scope_id).or_insert((0, target_len));
            entry_count.0 += 1;
            entry_count.1 = entry_count.1.max(target_len);
        }
    }

    counts
        .into_iter()
        .max_by(|(id_a, (count_a, len_a)), (id_b, (count_b, len_b))| {
            count_a
                .cmp(count_b)
                .then_with(|| len_a.cmp(len_b))
                .then_with(|| id_b.cmp(id_a))
        })
        .and_then(|(scope_id, (count, _))| if count > 0 { Some(scope_id) } else { None })
}

fn match_file_to_best_scope(
    scopes: &[ScopeIndex],
    default_scope: &str,
    rel_file: &str,
) -> Option<(String, usize)> {
    let mut best: Option<(usize, &str)> = None;
    for scope_idx in scopes {
        if scope_idx.scope_id == "inbox" || scope_idx.scope_id == default_scope {
            continue;
        }
        let target = normalize_relative_path_str(&scope_idx.manifest.scope.target);
        if target.is_empty() || target == ".tracker" {
            continue;
        }

        let explicit_file_match = scope_idx
            .manifest
            .scope
            .tracked_files
            .iter()
            .any(|tf| normalize_relative_path_str(tf) == rel_file);

        let matched_len = if explicit_file_match {
            Some(rel_file.len() + 1)
        } else if target == "." {
            Some(0)
        } else if rel_file == target || rel_file.starts_with(&format!("{target}/")) {
            Some(target.len())
        } else {
            None
        };

        if let Some(len) = matched_len {
            match best {
                None => best = Some((len, &scope_idx.scope_id)),
                Some((best_len, best_id)) => {
                    if len > best_len || (len == best_len && scope_idx.scope_id.as_str() < best_id)
                    {
                        best = Some((len, &scope_idx.scope_id));
                    }
                }
            }
        }
    }
    best.map(|(len, id)| (id.to_string(), len))
}

fn workspace_has_unscoped_components_on_disk(project_root: &Path) -> bool {
    for marker in ECOSYSTEM_MANIFEST_FILES {
        if project_root.join(marker).is_file() {
            return true;
        }
    }
    for common_dir in ["crates", "src", "packages", "apps", "services", "scripts"] {
        if project_root.join(common_dir).is_dir() {
            return true;
        }
    }
    false
}
