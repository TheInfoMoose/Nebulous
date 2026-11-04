use std::cmp::Ordering;
use std::fs;

use chrono::Utc;

use crate::error::{Result, TrackerError};
use crate::models::{
    ItemDetail, ItemEffort, ItemHistory, ItemManifest, ItemMetadata, ItemStatus, ItemStub,
    ItemType, PhaseManifest, PhaseMetadata, PhaseRegistryEntry, PhaseStatus, PhaseSummary,
    PreviousFix, ProjectManifest, ScopeItems, ScopeManifest, ScopeMetadata, ScopeRegistry,
    ScopeRegistryEntry, Severity,
};
use crate::storage::id::parse_item_id_number;
use crate::storage::reconcile::infer_default_scope_type;
use crate::storage::{validate_identifier, ItemLocation, TrackerStorage};

/// Parameters for creating a new tracked bug or feature via [`TrackerStorage::create_item`].
#[derive(Debug, Clone)]
pub struct CreateItemInput {
    pub item_type: ItemType,
    pub title: String,
    pub phase_id: Option<String>,
    pub scope_id: Option<String>,
    pub severity: Severity,
    pub description: Option<String>,
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
                id: item_id.clone(),
                item_type: input.item_type,
                title: title.clone(),
                status: ItemStatus::Open,
                severity: input.severity,
                scope_ref: scope_id.clone(),
                created_at: now.clone(),
                updated_at: now,
            },
            effort: ItemEffort {
                logged_time_minutes: 0,
                iteration_count: 1,
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

        self.write_item_detail(&phase_id, &scope_id, &detail)?;
        self.sync_parent_indexes_for_item(&phase_id, &scope_id, &detail.manifest)?;

        let location = ItemLocation {
            phase_id,
            scope_id,
            item_type: input.item_type,
            stub: detail.manifest.to_stub(),
        };

        Ok((location, detail))
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
            item_type: ItemType::Bug,
            title: title.into(),
            phase_id: phase_id.map(str::to_string),
            scope_id: scope_id.map(str::to_string),
            severity,
            description: description.map(str::to_string),
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
            item_type: ItemType::Feature,
            title: title.into(),
            phase_id: phase_id.map(str::to_string),
            scope_id: scope_id.map(str::to_string),
            severity,
            description: description.map(str::to_string),
        })
    }

    /// Updates an item's status and atomically updates `item.toml`, `scope.toml`, and `phase.toml`.
    ///
    /// If transitioning from a completed status (`closed` or `merged`) to an active status
    /// (`open`, `in_progress`, or `pending`), this automatically increments both
    /// `history.recurrence_count` and `effort.iteration_count`.
    pub fn update_item_status(&self, item_id: &str, new_status: ItemStatus) -> Result<ItemDetail> {
        let (loc, mut detail) = self.load_item_detail_by_id(item_id)?;
        let old_status = detail.manifest.item.status;

        if old_status.is_completed() && !new_status.is_completed() {
            detail.manifest.history.recurrence_count =
                detail.manifest.history.recurrence_count.saturating_add(1);
            detail.manifest.effort.iteration_count =
                detail.manifest.effort.iteration_count.saturating_add(1);
        } else if !old_status.is_completed() && new_status.is_completed() {
            let today = Utc::now().format("%Y-%m-%d").to_string();
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

        Ok(detail)
    }

    /// Reopens a `closed` or `merged` item back to `open`, incrementing `recurrence_count` and
    /// `iteration_count`, optionally appending a reopen note to `desc.md`, and atomically updating
    /// `scope.toml` and `phase.toml`.
    pub fn reopen_item(&self, item_id: &str, reason: Option<&str>) -> Result<ItemDetail> {
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
            let today = Utc::now().format("%Y-%m-%d").to_string();
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
    }

    /// Records a regression on an item (`BUG-XXXX` or `FEAT-XXXX`).
    ///
    /// If the item is currently `closed` or `merged`, it is reopened to `open` via [`Self::reopen_item`].
    /// If the item is already in an active state (`open`, `in_progress`, or `pending`), this increments
    /// `history.recurrence_count` and `effort.iteration_count`, ensures the status is active (`open`
    /// if `pending`), appends the optional regression note to `desc.md`, and atomically syncs
    /// `item.toml`, `scope.toml`, and `phase.toml`.
    pub fn record_regression(&self, item_id: &str, reason: Option<&str>) -> Result<ItemDetail> {
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
            let today = Utc::now().format("%Y-%m-%d").to_string();
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
    }

    /// Closes (or merges when `merged == true`) an item, appending a [`PreviousFix`] entry
    /// containing today's date, the Git `HEAD` short SHA (or `commit_override`, falling back to
    /// `"uncommitted"`), and `summary`, then atomically updating `scope.toml` and `phase.toml`.
    pub fn close_item(
        &self,
        item_id: &str,
        summary: impl Into<String>,
        commit_override: Option<&str>,
        merged: bool,
    ) -> Result<ItemDetail> {
        let summary_trimmed = summary.into().trim().to_string();
        if summary_trimmed.is_empty() {
            return Err(TrackerError::ValidationError(
                "Fix summary must not be empty when closing an item".to_string(),
            ));
        }

        let (loc, mut detail) = self.load_item_detail_by_id(item_id)?;
        let commit = commit_override
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| self.git_head_short_sha());

        let today = Utc::now().format("%Y-%m-%d").to_string();
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

        Ok(detail)
    }

    /// Updates an item's title and/or severity and atomically syncs parent `scope.toml` and `phase.toml`.
    pub fn update_item_metadata(
        &self,
        item_id: &str,
        title: Option<String>,
        severity: Option<Severity>,
    ) -> Result<ItemDetail> {
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
    }

    /// Updates an item's Markdown specification (`desc.md`) and bumps `updated_at` in `item.toml`.
    pub fn update_item_description(
        &self,
        item_id: &str,
        description_markdown: impl Into<String>,
    ) -> Result<ItemDetail> {
        let (loc, mut detail) = self.load_item_detail_by_id(item_id)?;
        detail.description_markdown = description_markdown.into();
        detail.manifest.item.updated_at = now_rfc3339();

        self.write_item_detail(&loc.phase_id, &loc.scope_id, &detail)?;
        Ok(detail)
    }

    /// Creates a new phase directory, `phase.toml`, and default `inbox` scope, and registers the
    /// phase in `project.toml`.
    pub fn create_phase(
        &self,
        phase_id: &str,
        title: impl Into<String>,
        set_active: bool,
    ) -> Result<PhaseManifest> {
        validate_identifier("Phase", phase_id)?;
        let title_trimmed = title.into().trim().to_string();
        if title_trimmed.is_empty() {
            return Err(TrackerError::ValidationError(
                "Phase title must not be empty".to_string(),
            ));
        }

        let mut project = self.read_project()?;
        if project.phases.registry.iter().any(|p| p.id == phase_id)
            || self.phase_toml_path(phase_id)?.is_file()
        {
            return Err(TrackerError::AlreadyExists {
                kind: "Phase",
                id: phase_id.to_string(),
            });
        }

        let today = Utc::now().format("%Y-%m-%d").to_string();
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
        fs::create_dir_all(self.items_dir(phase_id, &default_scope)?).map_err(|source| {
            TrackerError::Io {
                path: self.tracker_dir().to_path_buf(),
                source,
            }
        })?;
        self.write_project(&project)?;

        Ok(phase_manifest)
    }

    /// Sets `project.active_phase` in `project.toml` after verifying the target phase exists.
    pub fn set_active_phase(&self, phase_id: &str) -> Result<ProjectManifest> {
        validate_identifier("Phase", phase_id)?;
        let _ = self.read_phase(phase_id)?;
        let mut project = self.read_project()?;
        project.project.active_phase = phase_id.to_string();
        self.write_project(&project)?;
        Ok(project)
    }

    /// Creates a new domain scope (`scope.toml` + `items/` directory) inside `phase_id` (defaulting
    /// to `project.active_phase`) and registers it in `phase.toml`.
    pub fn create_scope(
        &self,
        phase_id: Option<&str>,
        scope_id: &str,
        target: impl Into<String>,
        tech_stack: Vec<String>,
        description: Option<String>,
    ) -> Result<ScopeManifest> {
        validate_identifier("Scope", scope_id)?;
        let target_trimmed = crate::storage::normalize_relative_path_str(&target.into());
        if target_trimmed.is_empty() {
            return Err(TrackerError::ValidationError(
                "Scope target path must not be empty".to_string(),
            ));
        }


        let project = self.read_project()?;
        let resolved_phase = match phase_id.map(str::trim).filter(|s| !s.is_empty()) {
            Some(p) => {
                validate_identifier("Phase", p)?;
                p.to_string()
            }
            None => project.project.active_phase.clone(),
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
        fs::create_dir_all(self.items_dir(&resolved_phase, scope_id)?).map_err(|source| {
            TrackerError::Io {
                path: self.tracker_dir().to_path_buf(),
                source,
            }
        })?;

        if !phase_manifest
            .scopes
            .manifest
            .iter()
            .any(|s| s.id == scope_id)
        {
            phase_manifest.scopes.manifest.push(ScopeRegistryEntry {
                id: scope_id.to_string(),
                scope_type: infer_default_scope_type(scope_id),
                path: format!("domains/{scope_id}"),
            });
            self.write_phase(&resolved_phase, &phase_manifest)?;
        }

        Ok(scope_manifest)
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

        scope_manifest
            .items
            .bugs
            .retain(|existing| existing.id != stub.id);
        scope_manifest
            .items
            .features
            .retain(|existing| existing.id != stub.id);

        match item_manifest.item.item_type {
            ItemType::Bug => {
                scope_manifest.items.bugs.push(stub);
                scope_manifest.items.bugs.sort_by(compare_stubs);
            }
            ItemType::Feature => {
                scope_manifest.items.features.push(stub);
                scope_manifest.items.features.sort_by(compare_stubs);
            }
        }

        self.write_scope(phase_id, scope_id, &scope_manifest)?;

        // Recompute `phase.toml` `[summary]` (`total_items` and `completed_items`) across all scopes in `phase_id`
        let mut phase_manifest = self.read_phase(phase_id)?;
        let phase_dir = self.phase_dir(phase_id)?;
        let mut total_items: u32 = 0;
        let mut completed_items: u32 = 0;

        for scope_entry in &phase_manifest.scopes.manifest {
            let s_manifest = if scope_entry.id == scope_id {
                scope_manifest.clone()
            } else {
                let s_dir =
                    self.resolve_scope_path(&phase_dir, &scope_entry.id, Some(&scope_entry.path))?;
                let s_toml = s_dir.join("scope.toml");
                if !s_toml.is_file() {
                    continue;
                }
                self.read_scope_from_path(&s_toml)?
            };

            for bug in &s_manifest.items.bugs {
                total_items += 1;
                if bug.status.is_completed() {
                    completed_items += 1;
                }
            }
            for feat in &s_manifest.items.features {
                total_items += 1;
                if feat.status.is_completed() {
                    completed_items += 1;
                }
            }
        }

        phase_manifest.summary.total_items = total_items;
        phase_manifest.summary.completed_items = completed_items;
        self.write_phase(phase_id, &phase_manifest)?;

        Ok(())
    }
}

pub(crate) fn now_rfc3339() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn compare_stubs(a: &ItemStub, b: &ItemStub) -> Ordering {
    match (parse_item_id_number(&a.id), parse_item_id_number(&b.id)) {
        (Some((type_a, num_a)), Some((type_b, num_b))) if type_a == type_b => {
            num_a.cmp(&num_b).then_with(|| a.id.cmp(&b.id))
        }
        _ => a.id.cmp(&b.id),
    }
}
