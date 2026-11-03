use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::models::{ItemStatus, ItemType, PreviousFix, Severity};
use crate::storage::TrackerStorage;

/// Comprehensive project analytics report combining time valuation, stagnant item detection,
/// and recurring bug hotspot rankings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectMetricsReport {
    pub target_daily_hours: f64,
    pub stagnant_threshold_days: u32,
    pub total_logged_minutes: u32,
    pub total_logged_hours: f64,
    pub target_days_equivalent: f64,
    pub total_items: u32,
    pub completed_items: u32,
    pub phases: Vec<PhaseEffortValuation>,
    pub stagnant_items: Vec<StagnantItemEntry>,
    pub item_hotspots: Vec<RecurringItemHotspot>,
    pub scope_hotspots: Vec<ScopeHotspot>,
}

/// Time valuation and completion metrics for a single project phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PhaseEffortValuation {
    pub phase_id: String,
    pub phase_title: String,
    pub total_items: u32,
    pub completed_items: u32,
    pub logged_minutes: u32,
    pub logged_hours: f64,
    pub target_daily_hours: f64,
    pub target_days_equivalent: f64,
    pub daily_pace_ratio: f64,
    pub scopes: Vec<ScopeEffortValuation>,
}

/// Time valuation and completion metrics for a domain scope within a phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScopeEffortValuation {
    pub phase_id: String,
    pub scope_id: String,
    pub target: String,
    pub total_items: u32,
    pub completed_items: u32,
    pub logged_minutes: u32,
    pub logged_hours: f64,
}

/// Alert entry for a non-completed item whose `updated_at` exceeds `stagnant_item_threshold_days`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagnantItemEntry {
    pub item_id: String,
    pub title: String,
    pub item_type: ItemType,
    pub status: ItemStatus,
    pub severity: Severity,
    pub phase_id: String,
    pub scope_id: String,
    pub updated_at: String,
    pub days_since_update: i64,
    pub logged_time_minutes: u32,
}

/// Item-level hotspot entry ranked by `recurrence_count` and historical fix count.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecurringItemHotspot {
    pub item_id: String,
    pub title: String,
    pub item_type: ItemType,
    pub status: ItemStatus,
    pub severity: Severity,
    pub phase_id: String,
    pub scope_id: String,
    pub recurrence_count: u32,
    pub iteration_count: u32,
    pub previous_fixes_count: usize,
    pub logged_time_minutes: u32,
    pub previous_fixes: Vec<PreviousFix>,
}

/// Domain scope hotspot entry aggregating recurrences and fix history across items in that scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScopeHotspot {
    pub scope_id: String,
    pub target: String,
    pub total_recurrences: u32,
    pub total_previous_fixes: usize,
    pub recurring_items_count: usize,
    pub total_logged_minutes: u32,
}

impl TrackerStorage {
    /// Computes the full project metrics report (phase/scope time valuation vs `target_daily_hours`,
    /// stagnant items, and recurring bug hotspots) at the current UTC timestamp.
    pub fn compute_metrics(&self) -> Result<ProjectMetricsReport> {
        self.compute_metrics_at(Utc::now())
    }

    /// Computes the full project metrics report evaluated against `now`.
    pub fn compute_metrics_at(&self, now: DateTime<Utc>) -> Result<ProjectMetricsReport> {
        let index = self.load_index()?;
        let target_daily_hours = index.project.metrics.target_daily_hours;
        let stagnant_threshold_days = index.project.metrics.stagnant_item_threshold_days;

        let mut total_logged_minutes: u32 = 0;
        let mut total_items: u32 = 0;
        let mut completed_items: u32 = 0;
        let mut phases_valuation = Vec::with_capacity(index.phases.len());
        let mut stagnant_items = Vec::new();
        let mut item_hotspots = Vec::new();
        let mut scope_agg: BTreeMap<String, ScopeHotspot> = BTreeMap::new();

        for phase_idx in &index.phases {
            let mut phase_logged_minutes: u32 = 0;
            let mut phase_total_items: u32 = 0;
            let mut phase_completed_items: u32 = 0;
            let mut scopes_valuation = Vec::with_capacity(phase_idx.scopes.len());

            for scope_idx in &phase_idx.scopes {
                let mut scope_logged_minutes: u32 = 0;
                let mut scope_total_items: u32 = 0;
                let mut scope_completed_items: u32 = 0;

                let all_stubs = scope_idx
                    .manifest
                    .items
                    .bugs
                    .iter()
                    .chain(scope_idx.manifest.items.features.iter());

                for stub in all_stubs {
                    let manifest = self.read_item_manifest(
                        &phase_idx.phase_id,
                        &scope_idx.scope_id,
                        &stub.id,
                    )?;

                    scope_total_items += 1;
                    if manifest.item.status.is_completed() {
                        scope_completed_items += 1;
                    }
                    scope_logged_minutes =
                        scope_logged_minutes.saturating_add(manifest.effort.logged_time_minutes);

                    // Check stagnation on non-completed items
                    if !manifest.item.status.is_completed() {
                        let updated_dt = manifest.item.parsed_updated_at()?;
                        let days_since = (now - updated_dt).num_days();
                        if days_since >= i64::from(stagnant_threshold_days) {
                            stagnant_items.push(StagnantItemEntry {
                                item_id: manifest.item.id.clone(),
                                title: manifest.item.title.clone(),
                                item_type: manifest.item.item_type,
                                status: manifest.item.status,
                                severity: manifest.item.severity,
                                phase_id: phase_idx.phase_id.clone(),
                                scope_id: scope_idx.scope_id.clone(),
                                updated_at: manifest.item.updated_at.clone(),
                                days_since_update: days_since,
                                logged_time_minutes: manifest.effort.logged_time_minutes,
                            });
                        }
                    }

                    // Check recurrence & fix history hotspots
                    let fixes_len = manifest.history.previous_fixes.len();
                    if manifest.history.recurrence_count > 0 || fixes_len > 0 {
                        item_hotspots.push(RecurringItemHotspot {
                            item_id: manifest.item.id.clone(),
                            title: manifest.item.title.clone(),
                            item_type: manifest.item.item_type,
                            status: manifest.item.status,
                            severity: manifest.item.severity,
                            phase_id: phase_idx.phase_id.clone(),
                            scope_id: scope_idx.scope_id.clone(),
                            recurrence_count: manifest.history.recurrence_count,
                            iteration_count: manifest.effort.iteration_count,
                            previous_fixes_count: fixes_len,
                            logged_time_minutes: manifest.effort.logged_time_minutes,
                            previous_fixes: manifest.history.previous_fixes.clone(),
                        });
                    }

                    let entry = scope_agg
                        .entry(scope_idx.scope_id.clone())
                        .or_insert_with(|| ScopeHotspot {
                            scope_id: scope_idx.scope_id.clone(),
                            target: scope_idx.manifest.scope.target.clone(),
                            total_recurrences: 0,
                            total_previous_fixes: 0,
                            recurring_items_count: 0,
                            total_logged_minutes: 0,
                        });
                    entry.total_recurrences = entry
                        .total_recurrences
                        .saturating_add(manifest.history.recurrence_count);
                    entry.total_previous_fixes += fixes_len;
                    if manifest.history.recurrence_count > 0 {
                        entry.recurring_items_count += 1;
                    }
                    entry.total_logged_minutes = entry
                        .total_logged_minutes
                        .saturating_add(manifest.effort.logged_time_minutes);
                }

                let scope_logged_hours = f64::from(scope_logged_minutes) / 60.0;
                scopes_valuation.push(ScopeEffortValuation {
                    phase_id: phase_idx.phase_id.clone(),
                    scope_id: scope_idx.scope_id.clone(),
                    target: scope_idx.manifest.scope.target.clone(),
                    total_items: scope_total_items,
                    completed_items: scope_completed_items,
                    logged_minutes: scope_logged_minutes,
                    logged_hours: scope_logged_hours,
                });

                phase_total_items += scope_total_items;
                phase_completed_items += scope_completed_items;
                phase_logged_minutes = phase_logged_minutes.saturating_add(scope_logged_minutes);
            }

            let phase_logged_hours = f64::from(phase_logged_minutes) / 60.0;
            let phase_days_equiv = if target_daily_hours > 0.0 {
                phase_logged_hours / target_daily_hours
            } else {
                0.0
            };

            phases_valuation.push(PhaseEffortValuation {
                phase_id: phase_idx.phase_id.clone(),
                phase_title: phase_idx.manifest.phase.title.clone(),
                total_items: phase_total_items,
                completed_items: phase_completed_items,
                logged_minutes: phase_logged_minutes,
                logged_hours: phase_logged_hours,
                target_daily_hours,
                target_days_equivalent: phase_days_equiv,
                daily_pace_ratio: phase_days_equiv,
                scopes: scopes_valuation,
            });

            total_items += phase_total_items;
            completed_items += phase_completed_items;
            total_logged_minutes = total_logged_minutes.saturating_add(phase_logged_minutes);
        }

        // Sort stagnant items by days_since_update DESC, severity DESC, item_id ASC
        stagnant_items.sort_by(|a, b| {
            b.days_since_update
                .cmp(&a.days_since_update)
                .then_with(|| b.severity.cmp(&a.severity))
                .then_with(|| a.item_id.cmp(&b.item_id))
        });

        // Sort item hotspots by recurrence_count DESC, previous_fixes_count DESC, logged_time_minutes DESC, item_id ASC
        item_hotspots.sort_by(|a, b| {
            b.recurrence_count
                .cmp(&a.recurrence_count)
                .then_with(|| b.previous_fixes_count.cmp(&a.previous_fixes_count))
                .then_with(|| b.logged_time_minutes.cmp(&a.logged_time_minutes))
                .then_with(|| a.item_id.cmp(&b.item_id))
        });

        let mut scope_hotspots: Vec<ScopeHotspot> = scope_agg
            .into_values()
            .filter(|s| s.total_recurrences > 0 || s.total_previous_fixes > 0)
            .collect();
        scope_hotspots.sort_by(|a, b| {
            b.total_recurrences
                .cmp(&a.total_recurrences)
                .then_with(|| b.total_previous_fixes.cmp(&a.total_previous_fixes))
                .then_with(|| b.total_logged_minutes.cmp(&a.total_logged_minutes))
                .then_with(|| a.scope_id.cmp(&b.scope_id))
        });

        let total_logged_hours = f64::from(total_logged_minutes) / 60.0;
        let target_days_equivalent = if target_daily_hours > 0.0 {
            total_logged_hours / target_daily_hours
        } else {
            0.0
        };

        Ok(ProjectMetricsReport {
            target_daily_hours,
            stagnant_threshold_days,
            total_logged_minutes,
            total_logged_hours,
            target_days_equivalent,
            total_items,
            completed_items,
            phases: phases_valuation,
            stagnant_items,
            item_hotspots,
            scope_hotspots,
        })
    }

    /// Returns all non-closed items that have not been updated within `stagnant_item_threshold_days`.
    pub fn find_stagnant_items(&self) -> Result<Vec<StagnantItemEntry>> {
        Ok(self.compute_metrics()?.stagnant_items)
    }

    /// Returns all non-closed items that have not been updated within `stagnant_item_threshold_days` at `now`.
    pub fn find_stagnant_items_at(&self, now: DateTime<Utc>) -> Result<Vec<StagnantItemEntry>> {
        Ok(self.compute_metrics_at(now)?.stagnant_items)
    }

    /// Returns recurring item hotspots and domain scope hotspots ranked by `recurrence_count` and fix history.
    pub fn find_recurring_hotspots(
        &self,
    ) -> Result<(Vec<RecurringItemHotspot>, Vec<ScopeHotspot>)> {
        let report = self.compute_metrics()?;
        Ok((report.item_hotspots, report.scope_hotspots))
    }
}
