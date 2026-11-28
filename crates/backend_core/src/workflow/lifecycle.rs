use std::fs;
use std::path::Path;

use crate::models::time::{now_timestamp_string, today_date_string};

use crate::agent_context::AgentContextBundle;
use crate::error::{Result, TrackerError};
use crate::git_sync::{DocumentedCommitInput, DocumentedCommitOutcome};
use crate::models::{
    canonical_phase_title, resolve_phase_alias, ActiveTimer, ItemDetail, ItemEffort, ItemHistory,
    ItemManifest, ItemMetadata, ItemStatus, ItemType, PhaseManifest, PhaseMetadata,
    PhaseRegistryEntry, PhaseStatus, PhaseSummary, PreviousFix, ProjectManifest, ScopeItems,
    ScopeManifest, ScopeMetadata, ScopeRegistry, ScopeRegistryEntry, Severity,
};
use crate::storage::reconcile::infer_default_scope_type;
use crate::storage::{validate_identifier, ItemLocation, TrackerStorage};
use crate::workflow::timer::StoppedTimerSession;

/// Result of [`TrackerStorage::start_work_item`], including any timer session that was
/// automatically stopped when switching from a previous item.
#[derive(Debug, Clone)]
pub struct StartedWork {
    pub location: ItemLocation,
    pub detail: ItemDetail,
    pub timer: ActiveTimer,
    pub context: AgentContextBundle,
    pub stopped_previous: Option<StoppedTimerSession>,
}

/// Result of a status update or item close, including any active timer session that was stopped.
#[derive(Debug, Clone)]
pub struct StatusChange {
    pub detail: ItemDetail,
    pub stopped_timer: Option<StoppedTimerSession>,
}

/// Parameters for creating a new tracked bug or feature via [`TrackerStorage::create_item`].
#[derive(Debug, Clone)]
pub struct CreateItemInput {
    pub item_type: ItemType,
    pub title: String,
    pub phase_id: Option<String>,
    pub scope_id: Option<String>,
    pub severity: Severity,
    pub description: Option<String>,
    /// Plan step the item is imported from; written to `item.source_ref` in the same write as the rest of `item.toml`.
    pub source_ref: Option<String>,
}

impl CreateItemInput {
    /// Creates a new `CreateItemInput` with default `Severity::Medium` and automatic
    /// `active_phase` / `default_scope` resolution.
    pub fn new(item_type: ItemType, title: impl Into<String>) -> Self {
        Self {
            item_type,
            title: title.into(),
            phase_id: None,
            scope_id: None,
            severity: Severity::Medium,
            description: None,
            source_ref: None,
        }
    }

    pub fn with_phase(mut self, phase_id: impl Into<String>) -> Self {
        self.phase_id = Some(phase_id.into());
        self
    }

    pub fn with_scope(mut self, scope_id: impl Into<String>) -> Self {
        self.scope_id = Some(scope_id.into());
        self
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn with_source_ref(mut self, source_ref: impl Into<String>) -> Self {
        self.source_ref = Some(source_ref.into());
        self
    }
}

/// Generates a standard `desc.md` Markdown body containing `## Problem Description`,
/// `## Reproduction Steps`, and `## Fix Log & Agent Notes`.
pub fn format_boilerplate_description(
    title: &str,
    custom_description: Option<&str>,
    scope_target: &str,
) -> String {
    let problem_body = match custom_description.map(str::trim).filter(|s| !s.is_empty()) {
        Some(existing) if existing.contains("## Problem Description") => {
            let mut out = existing.to_string();
            if !out.contains("## Reproduction Steps") {
                out.push_str(
                    "\n\n## Reproduction Steps\n1. TODO: Add reproduction or verification steps.\n",
                );
            }
            if !out.contains("## Fix Log & Agent Notes") {
                out.push_str(&format!(
                    "\n\n## Fix Log & Agent Notes\n- Target scope path: `{scope_target}`\n"
                ));
            }
            if !out.ends_with('\n') {
                out.push('\n');
            }
            return out;
        }
        Some(text) => text.to_string(),
        None => format!("Describe the problem or specification for: {title}"),
    };

    format!(
        "## Problem Description\n{problem_body}\n\n## Reproduction Steps\n1. Inspect `{scope_target}`.\n2. Run tests to verify behavior.\n\n## Fix Log & Agent Notes\n- Target scope path: `{scope_target}`\n"
    )
}

impl TrackerStorage {
    /// Creates a new bug or feature item, allocates the next monotonic ID (`BUG-XXXX` or `FEAT-XXXX`),
    /// writes `item.toml` and boilerplate `desc.md` atomically, and updates parent `scope.toml`
    /// and `phase.toml` indexes atomically.
    pub fn create_item(&self, input: CreateItemInput) -> Result<(ItemLocation, ItemDetail)> {
        self.with_write_lock(|_| {
            let title = input.title.trim().to_string();
            if title.is_empty() {
                return Err(TrackerError::ValidationError(
                    "Item title must not be empty".to_string(),
                ));
            }

            let (phase_id, scope_id) =
                self.resolve_phase_and_scope(input.phase_id.as_deref(), input.scope_id.as_deref())?;
            let scope_manifest = self.read_scope(&phase_id, &scope_id)?;
            let item_id = self.allocate_next_id(input.item_type)?;
            let now = now_rfc3339();

            let manifest = ItemManifest {
                item: ItemMetadata {
                    status: ItemStatus::Open,
                    severity: input.severity,
                    source_ref: input.source_ref.clone(),
                    ..ItemMetadata::new(
                        item_id.clone(),
                        input.item_type,
                        title.clone(),
                        scope_id.clone(),
                        now,
                    )
                },
                effort: ItemEffort {
                    logged_time_minutes: 0,
                    iteration_count: 1,
                    last_timer_session: None,
                },
                history: ItemHistory {
                    recurrence_count: 0,
                    previous_fixes: Vec::new(),
                },
            };

            let description_markdown = format_boilerplate_description(
                &title,
                input.description.as_deref(),
                &scope_manifest.scope.target,
            );

            let detail = ItemDetail {
                manifest,
                description_markdown,
            };

            self.create_tracker_dir_new(&self.item_dir(&phase_id, &scope_id, &item_id)?)?;
            self.write_item_detail(&phase_id, &scope_id, &detail)?;
            self.sync_parent_indexes_for_item(&phase_id, &scope_id, &detail.manifest)?;

            let location = ItemLocation {
                phase_id,
                scope_id,
                item_type: input.item_type,
                stub: detail.manifest.to_stub(),
            };

            Ok((location, detail))
        })
    }

    /// Convenience helper for creating a bug (`BUG-XXXX`).
    pub fn create_bug(
        &self,
        title: impl Into<String>,
        phase_id: Option<&str>,
        scope_id: Option<&str>,
        severity: Severity,
        description: Option<&str>,
    ) -> Result<(ItemLocation, ItemDetail)> {
        self.create_item(CreateItemInput {
            phase_id: phase_id.map(str::to_string),
            scope_id: scope_id.map(str::to_string),
            severity,
            description: description.map(str::to_string),
            ..CreateItemInput::new(ItemType::Bug, title)
        })
    }

    /// Convenience helper for creating a feature (`FEAT-XXXX`).
    pub fn create_feature(
        &self,
        title: impl Into<String>,
        phase_id: Option<&str>,
        scope_id: Option<&str>,
        severity: Severity,
        description: Option<&str>,
    ) -> Result<(ItemLocation, ItemDetail)> {
        self.create_item(CreateItemInput {
            phase_id: phase_id.map(str::to_string),
            scope_id: scope_id.map(str::to_string),
            severity,
            description: description.map(str::to_string),
            ..CreateItemInput::new(ItemType::Feature, title)
        })
    }

    /// Resolves the target item ID for `neb start`: an explicit ID, or the top open item from
    /// [`Self::compute_whats_next`] (using `cwd`), or the top recommendation, or the active/inferred item.
    pub fn resolve_start_target(
        &self,
        item_id: Option<&str>,
        cwd: Option<&Path>,
    ) -> Result<String> {
        let resolved_id = match item_id.map(str::trim).filter(|s| !s.is_empty()) {
            Some(explicit) => explicit.to_ascii_uppercase(),
            None => {
                let whats_next = self.compute_whats_next(cwd)?;
                if let Some(open_id) = whats_next.top_open_item_id() {
                    open_id.to_string()
                } else if let Some(any_id) = whats_next.top_item_id() {
                    any_id.to_string()
                } else {
                    self.resolve_active_or_inferred_item_id(None)?
                }
            }
        };
        Ok(resolved_id)
    }

    /// Starts work on `item_id` (or the top recommended `open` / active item when `item_id` is `None`):
    ///
    /// 1. Resolves `item_id` (if `None`, picks the top recommended `open` item from [`Self::compute_whats_next`] or the active item).
    /// 2. If another timer is currently running for a *different* item, automatically stops that previous timer and logs its elapsed minutes (`stop_timer()`) instead of failing with `TimerAlreadyRunning`.
    /// 3. Transitions the target item's status to [`ItemStatus::InProgress`] (`update_item_status`).
    /// 4. Starts (or keeps) the active timer for the target item (`.timer.toml`).
    /// 5. Builds and returns the [`StartedWork`] bundle so the caller can immediately display/copy the prompt and report any stopped timer.
    pub fn start_work_item(
        &self,
        item_id: Option<&str>,
        note: Option<String>,
        cwd: Option<&Path>,
    ) -> Result<StartedWork> {
        self.with_write_lock(|_| {
            let resolved_id = self.resolve_start_target(item_id, cwd)?;

            let target_loc = self.locate_item(&resolved_id)?;
            let canonical_id = target_loc.stub.id;

            let mut stopped_previous = None;
            if let Some(active) = self.read_active_timer()? {
                if active.item_id != canonical_id {
                    stopped_previous = Some(self.stop_timer()?);
                }
            }

            let detail = self.update_item_status(&canonical_id, ItemStatus::InProgress)?;
            let timer = self.switch_or_start_timer(&canonical_id, note)?;
            let location = self.locate_item(&canonical_id)?;
            let context = self.export_agent_context(&canonical_id)?;

            Ok(StartedWork {
                location,
                detail,
                timer,
                context,
                stopped_previous,
            })
        })
    }

    /// Updates an item's status and atomically updates `item.toml`, `scope.toml`, and `phase.toml`.
    ///
    /// - If transitioning from a completed status (`closed` or `merged`) to an active status
    ///   (`open`, `in_progress`, or `pending`), this automatically increments both
    ///   `history.recurrence_count` and `effort.iteration_count`.
    /// - If transitioning to [`ItemStatus::InProgress`] and no timer is running (or a different
    ///   item's timer is running), this automatically starts/switches the session timer onto the item.
    /// - If transitioning to [`ItemStatus::Closed`] or [`ItemStatus::Merged`] while `.timer.toml`
    ///   is timing the item, this automatically stops the timer and adds the elapsed minutes to
    ///   `effort.logged_time_minutes` before writing the closed manifest.
    pub fn update_item_status(&self, item_id: &str, new_status: ItemStatus) -> Result<ItemDetail> {
        self.update_item_status_reporting(item_id, new_status)
            .map(|c| c.detail)
    }

    /// Updates an item's status like [`Self::update_item_status`] and also returns any timer
    /// session stopped during the transition.
    pub fn update_item_status_reporting(
        &self,
        item_id: &str,
        new_status: ItemStatus,
    ) -> Result<StatusChange> {
        self.with_write_lock(|_| {
            let normalized_id = item_id.trim().to_ascii_uppercase();
            let mut stopped_timer = None;
            if new_status == ItemStatus::InProgress {
                if let Some(active) = self.read_active_timer()? {
                    if active.item_id != normalized_id {
                        stopped_timer = Some(self.stop_timer()?);
                    }
                }
            }

            if new_status.is_completed() {
                if let Some(active) = self.read_active_timer()? {
                    // Stop through stop_timer before loading the detail: the inline add skipped the session cap and the no-double-log key.
                    if active.item_id == self.locate_item(&normalized_id)?.stub.id {
                        stopped_timer = Some(self.stop_timer()?);
                    }
                }
            }

            let (loc, mut detail) = self.load_item_detail_by_id(&normalized_id)?;
            let old_status = detail.manifest.item.status;

            if old_status.is_completed() && !new_status.is_completed() {
                detail.manifest.history.recurrence_count =
                    detail.manifest.history.recurrence_count.saturating_add(1);
                detail.manifest.effort.iteration_count =
                    detail.manifest.effort.iteration_count.saturating_add(1);
            } else if !old_status.is_completed() && new_status.is_completed() {
                let today = today_date_string();
                let commit = self.git_head_short_sha();
                detail.manifest.history.previous_fixes.push(PreviousFix {
                    date: today,
                    commit,
                    summary: format!("Status updated to {new_status}"),
                });
            }

            detail.manifest.item.status = new_status;
            detail.manifest.item.updated_at = now_rfc3339();

            self.write_item_manifest(
                &loc.phase_id,
                &loc.scope_id,
                &detail.manifest.item.id,
                &detail.manifest,
            )?;
            self.sync_parent_indexes_for_item(&loc.phase_id, &loc.scope_id, &detail.manifest)?;

            if new_status == ItemStatus::InProgress {
                let _ = self.switch_or_start_timer(&detail.manifest.item.id, None)?;
            }

            Ok(StatusChange {
                detail,
                stopped_timer,
            })
        })
    }

    /// Reopens a `closed` or `merged` item back to `open`, incrementing `recurrence_count` and
    /// `iteration_count`, optionally appending a reopen note to `desc.md`, and atomically updating
    /// `scope.toml` and `phase.toml`.
    pub fn reopen_item(&self, item_id: &str, reason: Option<&str>) -> Result<ItemDetail> {
        self.with_write_lock(|_| {
            let (loc, mut detail) = self.load_item_detail_by_id(item_id)?;
            let old_status = detail.manifest.item.status;

            if !old_status.is_completed() {
                return Err(TrackerError::InvalidTransition {
                    item_id: item_id.to_string(),
                    from: old_status.to_string(),
                    to: ItemStatus::Open.to_string(),
                    reason: "only closed or merged items can be reopened".to_string(),
                });
            }

            detail.manifest.item.status = ItemStatus::Open;
            detail.manifest.history.recurrence_count =
                detail.manifest.history.recurrence_count.saturating_add(1);
            detail.manifest.effort.iteration_count =
                detail.manifest.effort.iteration_count.saturating_add(1);
            detail.manifest.item.updated_at = now_rfc3339();

            if let Some(note) = reason.map(str::trim).filter(|s| !s.is_empty()) {
                let today = today_date_string();
                if !detail.description_markdown.ends_with('\n') {
                    detail.description_markdown.push('\n');
                }
                detail
                    .description_markdown
                    .push_str(&format!("- Reopened ({today}): {note}\n"));
                self.write_item_desc(
                    &loc.phase_id,
                    &loc.scope_id,
                    &detail.manifest.item.id,
                    &detail.description_markdown,
                )?;
            }

            self.write_item_manifest(
                &loc.phase_id,
                &loc.scope_id,
                &detail.manifest.item.id,
                &detail.manifest,
            )?;
            self.sync_parent_indexes_for_item(&loc.phase_id, &loc.scope_id, &detail.manifest)?;

            Ok(detail)
        })
    }

    /// Records a regression on an item (`BUG-XXXX` or `FEAT-XXXX`).
    ///
    /// If the item is currently `closed` or `merged`, it is reopened to `open` via [`Self::reopen_item`].
    /// If the item is already in an active state (`open`, `in_progress`, or `pending`), this increments
    /// `history.recurrence_count` and `effort.iteration_count`, ensures the status is active (`open`
    /// if `pending`), appends the optional regression note to `desc.md`, and atomically syncs
    /// `item.toml`, `scope.toml`, and `phase.toml`.
    pub fn record_regression(&self, item_id: &str, reason: Option<&str>) -> Result<ItemDetail> {
        self.with_write_lock(|_| {
            let (loc, mut detail) = self.load_item_detail_by_id(item_id)?;
            if detail.manifest.item.status.is_completed() {
                return self.reopen_item(item_id, reason);
            }

            if detail.manifest.item.status == ItemStatus::Pending {
                detail.manifest.item.status = ItemStatus::Open;
            }
            detail.manifest.history.recurrence_count =
                detail.manifest.history.recurrence_count.saturating_add(1);
            detail.manifest.effort.iteration_count =
                detail.manifest.effort.iteration_count.saturating_add(1);
            detail.manifest.item.updated_at = now_rfc3339();

            if let Some(note) = reason.map(str::trim).filter(|s| !s.is_empty()) {
                let today = today_date_string();
                if !detail.description_markdown.ends_with('\n') {
                    detail.description_markdown.push('\n');
                }
                detail
                    .description_markdown
                    .push_str(&format!("- Regression recorded ({today}): {note}\n"));
                self.write_item_desc(
                    &loc.phase_id,
                    &loc.scope_id,
                    &detail.manifest.item.id,
                    &detail.description_markdown,
                )?;
            }

            self.write_item_manifest(
                &loc.phase_id,
                &loc.scope_id,
                &detail.manifest.item.id,
                &detail.manifest,
            )?;
            self.sync_parent_indexes_for_item(&loc.phase_id, &loc.scope_id, &detail.manifest)?;

            Ok(detail)
        })
    }

    /// Closes (or merges when `merged == true`) an item, automatically stopping and logging any
    /// active timer running for `item_id`, appending a [`PreviousFix`] entry containing today's date,
    /// the Git `HEAD` short SHA (or `commit_override`, falling back to `"uncommitted"`), and `summary`,
    /// then atomically updating `scope.toml` and `phase.toml`.
    pub fn close_item(
        &self,
        item_id: &str,
        summary: impl Into<String>,
        commit_override: Option<&str>,
        merged: bool,
    ) -> Result<ItemDetail> {
        self.close_item_reporting(item_id, summary, commit_override, merged)
            .map(|c| c.detail)
    }

    /// Closes (or merges when `merged == true`) an item like [`Self::close_item`] and also returns
    /// any timer session stopped during the close.
    pub fn close_item_reporting(
        &self,
        item_id: &str,
        summary: impl Into<String>,
        commit_override: Option<&str>,
        merged: bool,
    ) -> Result<StatusChange> {
        self.with_write_lock(|_| {
            let summary_trimmed = summary.into().trim().to_string();
            if summary_trimmed.is_empty() {
                return Err(TrackerError::ValidationError(
                    "Fix summary must not be empty when closing an item".to_string(),
                ));
            }

            let normalized_id = item_id.trim().to_ascii_uppercase();
            let mut stopped_timer = None;
            if let Some(active) = self.read_active_timer()? {
                if active.item_id == self.locate_item(&normalized_id)?.stub.id {
                    stopped_timer = Some(self.stop_timer()?);
                }
            }

            let (loc, mut detail) = self.load_item_detail_by_id(&normalized_id)?;

            let commit = commit_override
                .map(str::trim)
                .filter(|c| !c.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| self.git_head_short_sha());

            let today = today_date_string();
            detail.manifest.history.previous_fixes.push(PreviousFix {
                date: today,
                commit,
                summary: summary_trimmed,
            });

            detail.manifest.item.status = if merged {
                ItemStatus::Merged
            } else {
                ItemStatus::Closed
            };
            detail.manifest.item.updated_at = now_rfc3339();

            self.write_item_manifest(
                &loc.phase_id,
                &loc.scope_id,
                &detail.manifest.item.id,
                &detail.manifest,
            )?;
            self.sync_parent_indexes_for_item(&loc.phase_id, &loc.scope_id, &detail.manifest)?;

            Ok(StatusChange {
                detail,
                stopped_timer,
            })
        })
    }

    /// Validates, closes (or merges when `merged == true`), and creates a documented Git commit
    /// for `item_id` inside a single [`Self::with_write_lock`] critical section.
    ///
    /// If the post-close manifest adjustment or documented commit fails, restores the pre-close
    /// item detail, indexes, and active timer snapshot. Any rollback error is surfaced alongside
    /// the original commit error instead of being discarded.
    pub fn close_item_and_commit(
        &self,
        item_id: &str,
        summary: impl Into<String>,
        commit_override: Option<&str>,
        merged: bool,
        paths: &[String],
    ) -> Result<(StatusChange, DocumentedCommitOutcome)> {
        self.with_write_lock(|_| {
            let normalized_id = item_id.trim().to_ascii_uppercase();
            let loc = self.locate_item(&normalized_id)?;
            let summary_str = summary.into();

            let commit_input = DocumentedCommitInput {
                item_id: Some(normalized_id.clone()),
                scope_id: Some(loc.scope_id.clone()),
                summary: summary_str.clone(),
                body: None,
                tracker_only: paths.is_empty(),
                paths: paths.to_vec(),
                allow_empty_ok: false,
            };
            self.validate_documented_commit(&commit_input)?;

            let (loc, detail_snapshot) = self.load_item_detail_by_id(&normalized_id)?;
            let timer_snapshot = self.read_active_timer()?;

            let change =
                self.close_item_reporting(&normalized_id, &summary_str, commit_override, merged)?;
            let mut detail = change.detail;

            let commit_result = (|| -> Result<DocumentedCommitOutcome> {
                let auto_append = self.read_project()?.git.commit_policy.auto_append_to_desc;
                if auto_append && !detail.manifest.history.previous_fixes.is_empty() {
                    detail.manifest.history.previous_fixes.pop();
                    self.write_item_manifest(
                        &loc.phase_id,
                        &loc.scope_id,
                        &detail.manifest.item.id,
                        &detail.manifest,
                    )?;
                }
                self.git_commit_documented(commit_input)
            })();

            match commit_result {
                Ok(commit) => {
                    let (_, detail) = self.load_item_detail_by_id(&normalized_id)?;
                    Ok((
                        StatusChange {
                            detail,
                            stopped_timer: change.stopped_timer,
                        },
                        commit,
                    ))
                }
                Err(commit_err) => Err(self.rollback_close_and_commit(
                    &loc,
                    &detail_snapshot,
                    timer_snapshot.as_ref(),
                    commit_err,
                )),
            }
        })
    }

    fn rollback_close_and_commit(
        &self,
        loc: &ItemLocation,
        detail_snapshot: &ItemDetail,
        timer_snapshot: Option<&ActiveTimer>,
        commit_err: TrackerError,
    ) -> TrackerError {
        let mut rollback_errors = Vec::new();

        if let Err(err) = self.write_item_detail(&loc.phase_id, &loc.scope_id, detail_snapshot) {
            rollback_errors.push(format!("restore item detail: {err}"));
        }
        if let Err(err) = self.reconcile_indexes() {
            rollback_errors.push(format!("reconcile indexes: {err}"));
        }
        if let Some(timer) = timer_snapshot {
            if let Err(err) = self.write_active_timer(timer) {
                rollback_errors.push(format!("restore active timer: {err}"));
            }
        }

        if rollback_errors.is_empty() {
            commit_err
        } else {
            TrackerError::GitError(format!(
                "{commit_err}; rollback also failed: {}",
                rollback_errors.join("; ")
            ))
        }
    }

    /// Updates an item's title and/or severity and atomically syncs parent `scope.toml` and `phase.toml`.
    pub fn update_item_metadata(
        &self,
        item_id: &str,
        title: Option<String>,
        severity: Option<Severity>,
    ) -> Result<ItemDetail> {
        self.with_write_lock(|_| {
            let (loc, mut detail) = self.load_item_detail_by_id(item_id)?;

            if let Some(new_title) = title {
                let trimmed = new_title.trim().to_string();
                if trimmed.is_empty() {
                    return Err(TrackerError::ValidationError(
                        "Item title must not be empty".to_string(),
                    ));
                }
                detail.manifest.item.title = trimmed;
            }

            if let Some(new_severity) = severity {
                detail.manifest.item.severity = new_severity;
            }

            detail.manifest.item.updated_at = now_rfc3339();

            self.write_item_manifest(
                &loc.phase_id,
                &loc.scope_id,
                &detail.manifest.item.id,
                &detail.manifest,
            )?;
            self.sync_parent_indexes_for_item(&loc.phase_id, &loc.scope_id, &detail.manifest)?;

            Ok(detail)
        })
    }

    /// Updates an item's Markdown specification (`desc.md`) and bumps `updated_at` in `item.toml`.
    pub fn update_item_description(
        &self,
        item_id: &str,
        description_markdown: impl Into<String>,
    ) -> Result<ItemDetail> {
        self.with_write_lock(|_| {
            let (loc, mut detail) = self.load_item_detail_by_id(item_id)?;
            detail.description_markdown = description_markdown.into();
            detail.manifest.item.updated_at = now_rfc3339();

            self.write_item_detail(&loc.phase_id, &loc.scope_id, &detail)?;
            Ok(detail)
        })
    }

    /// Creates a new phase directory, `phase.toml`, and default `inbox` scope, and registers the
    /// phase in `project.toml`.
    pub fn create_phase(
        &self,
        phase_id: &str,
        title: impl Into<String>,
        set_active: bool,
    ) -> Result<PhaseManifest> {
        self.with_write_lock(|_| {
            let phase_id = resolve_phase_alias(phase_id);
            validate_identifier("Phase", phase_id)?;
            let raw_title = title.into().trim().to_string();
            let title_trimmed = if raw_title.is_empty() {
                if let Some(canon_title) = canonical_phase_title(phase_id) {
                    canon_title.to_string()
                } else {
                    return Err(TrackerError::ValidationError(
                        "Phase title must not be empty".to_string(),
                    ));
                }
            } else {
                raw_title
            };

            let mut project = self.read_project()?;
            if project.phases.registry.iter().any(|p| p.id == phase_id)
                || self.phase_toml_path(phase_id)?.is_file()
            {
                return Err(TrackerError::AlreadyExists {
                    kind: "Phase",
                    id: phase_id.to_string(),
                });
            }

            let today = today_date_string();
            let default_scope = {
                let trimmed = project.project.default_scope.trim();
                if trimmed.is_empty() {
                    "inbox".to_string()
                } else {
                    trimmed.to_string()
                }
            };

            let phase_manifest = PhaseManifest {
                phase: PhaseMetadata {
                    id: phase_id.to_string(),
                    title: title_trimmed.clone(),
                    status: if set_active {
                        PhaseStatus::InProgress
                    } else {
                        PhaseStatus::Planned
                    },
                    start_date: Some(today),
                    end_date: None,
                },
                scopes: ScopeRegistry {
                    manifest: vec![ScopeRegistryEntry {
                        id: default_scope.clone(),
                        scope_type: infer_default_scope_type(&default_scope),
                        path: format!("domains/{default_scope}"),
                    }],
                },
                summary: PhaseSummary::default(),
            };

            let inbox_scope = ScopeManifest {
                scope: ScopeMetadata {
                    id: default_scope.clone(),
                    target: ".tracker".to_string(),
                    tech_stack: Vec::new(),
                    description: Some("Default unscoped triage inbox".to_string()),
                    bound_component: None,
                    tracked_files: Vec::new(),
                },
                items: ScopeItems::default(),
            };

            project.phases.registry.push(PhaseRegistryEntry {
                id: phase_id.to_string(),
                title: title_trimmed,
                path: format!("phases/{phase_id}"),
            });
            if set_active {
                project.project.active_phase = phase_id.to_string();
            }

            self.write_phase(phase_id, &phase_manifest)?;
            self.write_scope(phase_id, &default_scope, &inbox_scope)?;
            self.ensure_items_dir(phase_id, &default_scope)?;
            self.write_project(&project)?;

            Ok(phase_manifest)
        })
    }

    /// Sets `project.active_phase` in `project.toml` after verifying the target phase exists
    /// (resolving canonical aliases such as `pre`, `prod`, `post`).
    pub fn set_active_phase(&self, phase_id: &str) -> Result<ProjectManifest> {
        self.with_write_lock(|_| {
            let resolved = self.ensure_canonical_phase_if_missing(phase_id)?;
            let mut phase_manifest = self.read_phase(&resolved)?;
            if phase_manifest.phase.status == PhaseStatus::Planned {
                phase_manifest.phase.status = PhaseStatus::InProgress;
                self.write_phase(&resolved, &phase_manifest)?;
            }
            let mut project = self.read_project()?;
            project.project.active_phase = resolved;
            self.write_project(&project)?;
            Ok(project)
        })
    }

    /// Creates a new domain scope (`scope.toml` + `items/` directory) inside `phase_id` (defaulting
    /// to `project.active_phase`), registers it in `phase.toml`, and automatically mirrors the
    /// scope across all other registered phases in `project.toml`.
    pub fn create_scope(
        &self,
        phase_id: Option<&str>,
        scope_id: &str,
        target: impl Into<String>,
        tech_stack: Vec<String>,
        description: Option<String>,
    ) -> Result<ScopeManifest> {
        self.with_write_lock(|_| {
            validate_identifier("Scope", scope_id)?;
            let target_trimmed = crate::storage::normalize_relative_path_str(&target.into());
            if target_trimmed.is_empty() {
                return Err(TrackerError::ValidationError(
                    "Scope target path must not be empty".to_string(),
                ));
            }

            let project = self.read_project()?;
            let resolved_phase = match phase_id.map(str::trim).filter(|s| !s.is_empty()) {
                Some(p) => self.ensure_canonical_phase_if_missing(p)?,
                None => resolve_phase_alias(&project.project.active_phase).to_string(),
            };

            let mut phase_manifest = self.read_phase(&resolved_phase)?;
            if self.scope_toml_path(&resolved_phase, scope_id)?.is_file() {
                return Err(TrackerError::AlreadyExists {
                    kind: "Scope",
                    id: scope_id.to_string(),
                });
            }

            let scope_manifest = ScopeManifest {
                scope: ScopeMetadata {
                    id: scope_id.to_string(),
                    target: target_trimmed,
                    tech_stack,
                    description,
                    bound_component: None,
                    tracked_files: Vec::new(),
                },
                items: ScopeItems::default(),
            };

            self.write_scope(&resolved_phase, scope_id, &scope_manifest)?;
            self.ensure_items_dir(&resolved_phase, scope_id)?;

            let scope_type = infer_default_scope_type(scope_id);
            if !phase_manifest
                .scopes
                .manifest
                .iter()
                .any(|s| s.id == scope_id)
            {
                phase_manifest.scopes.manifest.push(ScopeRegistryEntry {
                    id: scope_id.to_string(),
                    scope_type: scope_type.clone(),
                    path: format!("domains/{scope_id}"),
                });
                self.write_phase(&resolved_phase, &phase_manifest)?;
            }

            self.ensure_scope_across_phases(&scope_manifest.scope, Some(&scope_type))?;

            Ok(scope_manifest)
        })
    }

    /// Mirrors a domain scope (`scope.toml` with identical [`ScopeMetadata`] and empty `items/`
    /// directory) across all registered phases in `project.toml` where that scope does not yet exist.
    pub fn ensure_scope_across_phases(
        &self,
        scope_metadata: &ScopeMetadata,
        scope_type_hint: Option<&str>,
    ) -> Result<()> {
        self.with_write_lock(|_| {
            validate_identifier("Scope", &scope_metadata.id)?;
            let project = self.read_project()?;
            let scope_id = &scope_metadata.id;
            let resolved_type = scope_type_hint
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| infer_default_scope_type(scope_id));

            for phase_entry in &project.phases.registry {
                let phase_id = &phase_entry.id;
                let phase_toml = self.phase_toml_path(phase_id)?;
                if !phase_toml.is_file() {
                    continue;
                }
                let mut phase_manifest = self.read_phase(phase_id)?;
                let scope_toml = self.scope_toml_path(phase_id, scope_id)?;

                if !scope_toml.is_file() {
                    let mirrored_manifest = ScopeManifest {
                        scope: scope_metadata.clone(),
                        items: ScopeItems::default(),
                    };
                    self.write_scope(phase_id, scope_id, &mirrored_manifest)?;
                    self.ensure_items_dir(phase_id, scope_id)?;
                } else if scope_metadata.bound_component.is_some() {
                    let mut existing = self.read_scope(phase_id, scope_id)?;
                    existing.scope.target = scope_metadata.target.clone();
                    existing.scope.tech_stack = scope_metadata.tech_stack.clone();
                    existing.scope.bound_component = scope_metadata.bound_component.clone();
                    existing.scope.tracked_files = scope_metadata.tracked_files.clone();
                    if existing.scope.description.is_none() {
                        existing.scope.description = scope_metadata.description.clone();
                    }
                    self.write_scope(phase_id, scope_id, &existing)?;
                }

                if let Some(entry) = phase_manifest
                    .scopes
                    .manifest
                    .iter_mut()
                    .find(|s| s.id == *scope_id)
                {
                    if scope_type_hint.is_some() {
                        entry.scope_type = resolved_type.clone();
                    }
                } else {
                    phase_manifest.scopes.manifest.push(ScopeRegistryEntry {
                        id: scope_id.clone(),
                        scope_type: resolved_type.clone(),
                        path: format!("domains/{scope_id}"),
                    });
                }
                self.write_phase(phase_id, &phase_manifest)?;
            }

            Ok(())
        })
    }

    /// Moves a tracked item (`item.toml` and `desc.md`) from its current phase into `target_phase_id`
    /// (resolving canonical aliases such as `pre`, `prod`, `post`), ensuring the item's scope exists
    /// in the target phase, removing the old item directory, updating `updated_at`, and atomically
    /// reconciling both the source and target phase/scope indexes.
    pub fn move_item_to_phase(
        &self,
        item_id: &str,
        target_phase_id: &str,
    ) -> Result<(ItemLocation, ItemDetail)> {
        self.with_write_lock(|_| {
            let resolved_target_phase = self.ensure_canonical_phase_if_missing(target_phase_id)?;
            let mut target_phase_manifest = self.read_phase(&resolved_target_phase)?;

            let (loc, mut detail) = self.load_item_detail_by_id(item_id.trim())?;
            let canonical_item_id = detail.manifest.item.id.clone();

            if loc.phase_id == resolved_target_phase {
                detail.manifest.item.updated_at = now_rfc3339();
                self.write_item_detail(&loc.phase_id, &loc.scope_id, &detail)?;
                self.sync_parent_indexes_for_item(&loc.phase_id, &loc.scope_id, &detail.manifest)?;
                let updated_loc = ItemLocation {
                    phase_id: loc.phase_id,
                    scope_id: loc.scope_id,
                    item_type: detail.manifest.item.item_type,
                    stub: detail.manifest.to_stub(),
                };
                return Ok((updated_loc, detail));
            }

            let source_scope_manifest = self.read_scope(&loc.phase_id, &loc.scope_id)?;
            let source_phase_manifest = self.read_phase(&loc.phase_id)?;
            let source_scope_type = source_phase_manifest
                .scopes
                .manifest
                .iter()
                .find(|s| s.id == loc.scope_id)
                .map(|s| s.scope_type.clone())
                .unwrap_or_else(|| infer_default_scope_type(&loc.scope_id));

            let target_scope_toml = self.scope_toml_path(&resolved_target_phase, &loc.scope_id)?;
            if !target_scope_toml.is_file() {
                let copied_scope = ScopeManifest {
                    scope: source_scope_manifest.scope.clone(),
                    items: ScopeItems::default(),
                };
                self.write_scope(&resolved_target_phase, &loc.scope_id, &copied_scope)?;
                self.ensure_items_dir(&resolved_target_phase, &loc.scope_id)?;
            }

            if !target_phase_manifest
                .scopes
                .manifest
                .iter()
                .any(|s| s.id == loc.scope_id)
            {
                target_phase_manifest
                    .scopes
                    .manifest
                    .push(ScopeRegistryEntry {
                        id: loc.scope_id.clone(),
                        scope_type: source_scope_type,
                        path: format!("domains/{}", loc.scope_id),
                    });
                self.write_phase(&resolved_target_phase, &target_phase_manifest)?;
            }

            detail.manifest.item.updated_at = now_rfc3339();
            self.write_item_detail(&resolved_target_phase, &loc.scope_id, &detail)?;

            let old_item_dir = self.guard_tracker_path(&self.item_dir(
                &loc.phase_id,
                &loc.scope_id,
                &canonical_item_id,
            )?)?;
            if old_item_dir.exists() {
                fs::remove_dir_all(&old_item_dir).map_err(|source| TrackerError::Io {
                    path: old_item_dir,
                    source,
                })?;
            }

            if let Ok(Some(mut active_timer)) = self.read_active_timer() {
                if active_timer.item_id == canonical_item_id {
                    active_timer.phase_id = Some(resolved_target_phase.clone());
                    let _ = self.write_active_timer(&active_timer);
                }
            }

            self.remove_item_from_parent_indexes(&loc.phase_id, &loc.scope_id, &canonical_item_id)?;
            self.sync_parent_indexes_for_item(
                &resolved_target_phase,
                &loc.scope_id,
                &detail.manifest,
            )?;

            let new_loc = ItemLocation {
                phase_id: resolved_target_phase,
                scope_id: loc.scope_id,
                item_type: detail.manifest.item.item_type,
                stub: detail.manifest.to_stub(),
            };

            Ok((new_loc, detail))
        })
    }

    pub(crate) fn remove_item_from_parent_indexes(
        &self,
        phase_id: &str,
        scope_id: &str,
        item_id: &str,
    ) -> Result<()> {
        let mut scope_manifest = self.read_scope(phase_id, scope_id)?;
        scope_manifest.items.remove_id(item_id);
        self.write_scope(phase_id, scope_id, &scope_manifest)?;
        self.recompute_phase_summary_with_scope(phase_id, scope_id, &scope_manifest)
    }

    pub(crate) fn recompute_phase_summary_with_scope(
        &self,
        phase_id: &str,
        updated_scope_id: &str,
        updated_scope_manifest: &ScopeManifest,
    ) -> Result<()> {
        let mut phase_manifest = self.read_phase(phase_id)?;
        let phase_dir = self.phase_dir(phase_id)?;
        let mut total_items: u32 = 0;
        let mut completed_items: u32 = 0;

        for scope_entry in &phase_manifest.scopes.manifest {
            let s_manifest = if scope_entry.id == updated_scope_id {
                updated_scope_manifest.clone()
            } else {
                let s_dir =
                    self.resolve_scope_path(&phase_dir, &scope_entry.id, Some(&scope_entry.path))?;
                let s_toml = s_dir.join("scope.toml");
                if !s_toml.is_file() {
                    continue;
                }
                self.read_scope_from_path(&s_toml)?
            };

            for (_, stub) in s_manifest.items.iter() {
                total_items += 1;
                if stub.status.is_completed() {
                    completed_items += 1;
                }
            }
        }

        phase_manifest.summary.total_items = total_items;
        phase_manifest.summary.completed_items = completed_items;
        self.write_phase(phase_id, &phase_manifest)?;

        Ok(())
    }

    /// Atomically updates `scope.toml` `[items]` and `phase.toml` `[summary]` after an item
    /// mutation so parent indexes are always consistent without requiring a full tree scan.
    pub(crate) fn sync_parent_indexes_for_item(
        &self,
        phase_id: &str,
        scope_id: &str,
        item_manifest: &ItemManifest,
    ) -> Result<()> {
        let mut scope_manifest = self.read_scope(phase_id, scope_id)?;
        let stub = item_manifest.to_stub();

        scope_manifest.items.remove_id(&stub.id);
        scope_manifest
            .items
            .insert(item_manifest.item.item_type, stub);

        self.write_scope(phase_id, scope_id, &scope_manifest)?;
        self.recompute_phase_summary_with_scope(phase_id, scope_id, &scope_manifest)
    }
}

pub(crate) fn now_rfc3339() -> String {
    now_timestamp_string()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    use git2::{Repository, Signature};
    use tempfile::TempDir;

    use super::*;

    fn init_git_tracker() -> (TempDir, TrackerStorage) {
        let dir = TempDir::new().expect("tempdir");
        let repo = Repository::init(dir.path()).expect("init git repo");
        {
            let mut cfg = repo.config().expect("git config");
            cfg.set_str("user.name", "Test User").expect("user.name");
            cfg.set_str("user.email", "testuser@example.com")
                .expect("user.email");
        }

        let storage = TrackerStorage::init(dir.path(), "LifecycleTest", "01_core", "Core")
            .expect("init tracker");
        storage
            .create_scope(
                Some("01_core"),
                "backend_core",
                "src",
                vec!["rust".to_string()],
                None,
            )
            .expect("create scope");
        storage
            .create_bug(
                "Sample bug for close_item_and_commit",
                Some("01_core"),
                Some("backend_core"),
                Severity::Medium,
                None,
            )
            .expect("create bug");

        {
            let mut idx = repo.index().expect("repo index");
            idx.add_all([".tracker"], git2::IndexAddOption::DEFAULT, None)
                .expect("stage .tracker");
            idx.write().expect("write index");
            let tree = repo
                .find_tree(idx.write_tree().expect("write tree"))
                .expect("find tree");
            let sig = Signature::now("Test User", "testuser@example.com").expect("sig");
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .expect("initial commit");
        }

        (dir, storage)
    }

    #[test]
    fn close_item_and_commit_waits_for_write_lock() {
        let (dir, storage) = init_git_tracker();
        let bg_storage = TrackerStorage::new(dir.path());
        let (tx, rx) = mpsc::channel();

        let holder = thread::spawn(move || {
            bg_storage.with_write_lock(|_| {
                tx.send(()).expect("signal lock acquired");
                thread::sleep(Duration::from_millis(300));
                Ok(())
            })
        });

        rx.recv().expect("background thread should acquire lock");
        let start = Instant::now();
        // Use a policy-rejected summary ("short") so even pre-mutation validation must wait
        // on the outer `with_write_lock` before returning.
        let err = storage
            .close_item_and_commit("BUG-0001", "short", None, false, &[])
            .expect_err("short summary should fail policy validation");
        let elapsed = start.elapsed();

        holder
            .join()
            .expect("lock holder thread should not panic")
            .expect("lock holder should succeed");

        assert!(
            elapsed >= Duration::from_millis(250),
            "expected close_item_and_commit to block on outer with_write_lock, elapsed={elapsed:?}"
        );
        assert!(matches!(err, TrackerError::ValidationError(_)));
    }

    #[test]
    fn close_item_and_commit_surfaces_rollback_failure() {
        let (dir, storage) = init_git_tracker();
        let timer = storage
            .start_timer("BUG-0001", None)
            .expect("start timer on BUG-0001");
        let (loc, detail_snapshot) = storage
            .load_item_detail_by_id("BUG-0001")
            .expect("load BUG-0001 snapshot");

        // Replace `.tracker` with a regular file so restoring item detail, reconciling indexes,
        // and restoring the active timer all fail deterministically (even under root).
        let tracker_dir = dir.path().join(".tracker");
        fs::remove_dir_all(&tracker_dir).expect("remove .tracker dir");
        fs::write(&tracker_dir, "blocked\n").expect("replace .tracker with file");

        let original_err = TrackerError::GitError("simulated commit failure".to_string());
        let combined =
            storage.rollback_close_and_commit(&loc, &detail_snapshot, Some(&timer), original_err);

        match combined {
            TrackerError::GitError(msg) => {
                assert!(
                    msg.contains("simulated commit failure"),
                    "expected original commit failure in message, got: {msg}"
                );
                assert!(
                    msg.contains("rollback"),
                    "expected rollback context in message, got: {msg}"
                );
                assert!(
                    msg.contains("restore item detail"),
                    "expected item detail rollback error in message, got: {msg}"
                );
                assert!(
                    msg.contains("reconcile indexes"),
                    "expected reconcile rollback error in message, got: {msg}"
                );
                assert!(
                    msg.contains("restore active timer"),
                    "expected timer rollback error in message, got: {msg}"
                );
            }
            other => panic!("expected TrackerError::GitError, got: {other:?}"),
        }
    }
}
