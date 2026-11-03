use std::fmt::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::models::{ItemDetail, ScopeManifest};
use crate::storage::TrackerStorage;

/// Synthesized context handoff payload for an external AI coding agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentContextBundle {
    pub item_id: String,
    pub phase_id: String,
    pub scope_id: String,
    pub scope_rel_path: String,
    pub item_rel_path: String,
    pub desc_rel_path: String,
    pub file_refs: String,
    pub bounded_prompt: String,
}

impl TrackerStorage {
    /// Generates an [`AgentContextBundle`] containing both compact `@file` reference tags and
    /// a synthesized Bounded Agent Prompt for `item_id`.
    pub fn export_agent_context(&self, item_id: &str) -> Result<AgentContextBundle> {
        let (loc, detail) = self.load_item_detail_by_id(item_id)?;
        let scope_manifest = self.read_scope(&loc.phase_id, &loc.scope_id)?;

        let tracker_dir = self.tracker_dir();
        let scope_abs = self.scope_toml_path(&loc.phase_id, &loc.scope_id)?;
        let item_abs = self.item_toml_path(&loc.phase_id, &loc.scope_id, &loc.stub.id)?;
        let desc_abs = self.item_desc_path(&loc.phase_id, &loc.scope_id, &loc.stub.id)?;

        let scope_rel_path = to_tracker_relative_slash_path(
            tracker_dir,
            &scope_abs,
            &format!(
                ".tracker/phases/{}/domains/{}/scope.toml",
                loc.phase_id, loc.scope_id
            ),
        );
        let item_rel_path = to_tracker_relative_slash_path(
            tracker_dir,
            &item_abs,
            &format!(
                ".tracker/phases/{}/domains/{}/items/{}/item.toml",
                loc.phase_id, loc.scope_id, loc.stub.id
            ),
        );
        let desc_rel_path = to_tracker_relative_slash_path(
            tracker_dir,
            &desc_abs,
            &format!(
                ".tracker/phases/{}/domains/{}/items/{}/desc.md",
                loc.phase_id, loc.scope_id, loc.stub.id
            ),
        );

        let file_refs = format!("@{scope_rel_path} @{item_rel_path} @{desc_rel_path}");
        let bounded_prompt = synthesize_bounded_prompt(
            &loc.phase_id,
            &scope_manifest,
            &detail,
            &scope_rel_path,
            &item_rel_path,
            &desc_rel_path,
        );

        Ok(AgentContextBundle {
            item_id: loc.stub.id,
            phase_id: loc.phase_id,
            scope_id: loc.scope_id,
            scope_rel_path,
            item_rel_path,
            desc_rel_path,
            file_refs,
            bounded_prompt,
        })
    }

    /// Returns only the compact `@file` reference string for `item_id`:
    /// `@.tracker/.../scope.toml @.tracker/.../item.toml @.tracker/.../desc.md`.
    pub fn export_file_refs(&self, item_id: &str) -> Result<String> {
        Ok(self.export_agent_context(item_id)?.file_refs)
    }

    /// Returns the synthesized Bounded Agent Prompt for `item_id`.
    pub fn export_bounded_prompt(&self, item_id: &str) -> Result<String> {
        Ok(self.export_agent_context(item_id)?.bounded_prompt)
    }
}

/// Synthesizes a self-contained Bounded Agent Prompt from `scope.toml`, `item.toml`, and `desc.md`.
pub fn synthesize_bounded_prompt(
    phase_id: &str,
    scope: &ScopeManifest,
    detail: &ItemDetail,
    scope_rel_path: &str,
    item_rel_path: &str,
    desc_rel_path: &str,
) -> String {
    let item = &detail.manifest.item;
    let effort = &detail.manifest.effort;
    let history = &detail.manifest.history;

    let tech_stack_str = if scope.scope.tech_stack.is_empty() {
        "none specified".to_string()
    } else {
        scope.scope.tech_stack.join(", ")
    };

    let mut out = String::with_capacity(1024);

    let _ = writeln!(out, "# Bounded Agent Task: {} — {}", item.id, item.title);
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "**Context Files:** `@{scope_rel_path}` `@{item_rel_path}` `@{desc_rel_path}`"
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "## 1. Strict Domain Scope & Crate Boundary");
    let _ = writeln!(out, "- **Phase:** `{phase_id}`");
    let _ = writeln!(out, "- **Scope ID:** `{}`", scope.scope.id);
    let _ = writeln!(out, "- **Target Path:** `{}`", scope.scope.target);
    let _ = writeln!(out, "- **Tech Stack:** {tech_stack_str}");
    if let Some(desc) = scope
        .scope
        .description
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        let _ = writeln!(out, "- **Scope Description:** {}", desc.trim());
    }
    let _ = writeln!(
        out,
        "- **Boundary Constraint:** Restrict all code modifications strictly to `{}`. Do NOT modify unrelated crates or UI/CLI layers outside this scope.",
        scope.scope.target
    );
    let _ = writeln!(out);

    let _ = writeln!(out, "## 2. Item Metadata & Historical Fix Ledger");
    let _ = writeln!(out, "- **Item ID:** `{}`", item.id);
    let _ = writeln!(out, "- **Type:** `{}`", item.item_type);
    let _ = writeln!(out, "- **Status:** `{}`", item.status);
    let _ = writeln!(out, "- **Severity:** `{}`", item.severity);
    let _ = writeln!(
        out,
        "- **Effort Logged:** {} minutes ({} iteration(s))",
        effort.logged_time_minutes, effort.iteration_count
    );
    let _ = writeln!(out, "- **Recurrence Count:** {}", history.recurrence_count);

    if history.recurrence_count > 0 || !history.previous_fixes.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "> [!WARNING] **REGRESSION & PREVIOUS FIX WARNING (`recurrence_count = {}`)**",
            history.recurrence_count
        );
        let _ = writeln!(
            out,
            "> This item has previous fix attempts recorded in `previous_fixes`. Review them carefully and DO NOT repeat failed or incomplete patches:"
        );
        for (idx, fix) in history.previous_fixes.iter().enumerate() {
            let _ = writeln!(
                out,
                "> {}. **[{}]** (commit `{}`): {}",
                idx + 1,
                fix.date,
                fix.commit,
                fix.summary
            );
        }
    } else {
        let _ = writeln!(out, "- **Previous Fixes:** None recorded.");
    }

    let _ = writeln!(out);
    let _ = writeln!(out, "## 3. Task Specification (`desc.md`)");
    let _ = writeln!(out);
    let _ = writeln!(out, "{}", detail.description_markdown.trim());

    out
}

fn to_tracker_relative_slash_path(tracker_dir: &Path, full_path: &Path, fallback: &str) -> String {
    if let Ok(stripped) = full_path.strip_prefix(tracker_dir) {
        let s = stripped.to_string_lossy().replace('\\', "/");
        let trimmed = s.trim_start_matches('/');
        if !trimmed.is_empty() {
            return format!(".tracker/{trimmed}");
        }
    }
    fallback.to_string()
}
