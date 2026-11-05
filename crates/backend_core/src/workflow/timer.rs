use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};
use crate::models::{ActiveTimer, ItemDetail};
use crate::storage::TrackerStorage;
use crate::workflow::lifecycle::now_rfc3339;

/// Live status snapshot of an active `.tracker/.timer.toml` session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimerStatus {
    pub timer: ActiveTimer,
    pub elapsed_minutes: u32,
    pub elapsed_seconds: u64,
}

/// Summary returned when stopping an active `.tracker/.timer.toml` session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoppedTimerSession {
    pub timer: ActiveTimer,
    pub elapsed_minutes: u32,
    pub total_logged_minutes: u32,
    pub updated_item: ItemDetail,
}

impl TrackerStorage {
    /// Starts an active session timer for `item_id` at the current UTC timestamp and persists it
    /// to `.tracker/.timer.toml`.
    ///
    /// Fails with [`TrackerError::TimerAlreadyRunning`] if a timer is already active, or
    /// [`TrackerError::NotFound`] if `item_id` does not exist.
    pub fn start_timer(&self, item_id: &str, note: Option<String>) -> Result<ActiveTimer> {
        self.start_timer_at(item_id, Utc::now(), note)
    }

    /// Starts an active session timer for `item_id` at an explicit UTC timestamp `started_at`.
    pub fn start_timer_at(
        &self,
        item_id: &str,
        started_at: DateTime<Utc>,
        note: Option<String>,
    ) -> Result<ActiveTimer> {
        if let Some(existing) = self.read_active_timer()? {
            return Err(TrackerError::TimerAlreadyRunning {
                item_id: existing.item_id,
            });
        }

        let normalized_id = item_id.trim().to_ascii_uppercase();
        let loc = self.locate_item(&normalized_id)?;
        let started_str = started_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        let mut timer = ActiveTimer::new(
            loc.stub.id,
            Some(loc.phase_id),
            Some(loc.scope_id),
            started_str,
        );
        timer.note = note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());

        self.write_active_timer(&timer)?;
        Ok(timer)
    }

    /// Starts a timer for `item_id`, or switches the active timer to `item_id` by automatically
    /// stopping and logging elapsed minutes on any previous item's running timer.
    ///
    /// If a timer is already running for `item_id`, keeps the existing `started_at` timestamp
    /// (updating `note` if a new non-empty note is provided).
    pub fn switch_or_start_timer(
        &self,
        item_id: &str,
        note: Option<String>,
    ) -> Result<ActiveTimer> {
        let normalized_id = item_id.trim().to_ascii_uppercase();
        let loc = self.locate_item(&normalized_id)?;
        let clean_note = note.map(|n| n.trim().to_string()).filter(|n| !n.is_empty());

        if let Some(mut existing) = self.read_active_timer()? {
            if existing.item_id == loc.stub.id {
                let mut changed = false;
                if existing.phase_id.as_deref() != Some(loc.phase_id.as_str()) {
                    existing.phase_id = Some(loc.phase_id);
                    changed = true;
                }
                if existing.scope_id.as_deref() != Some(loc.scope_id.as_str()) {
                    existing.scope_id = Some(loc.scope_id);
                    changed = true;
                }
                if clean_note.is_some() && existing.note != clean_note {
                    existing.note = clean_note;
                    changed = true;
                }
                if changed {
                    self.write_active_timer(&existing)?;
                }
                return Ok(existing);
            }
            let _ = self.stop_timer()?;
        }

        self.start_timer(&loc.stub.id, clean_note)
    }

    /// Stops the currently running timer in `.tracker/.timer.toml`, adds the elapsed minutes to
    /// the target item's `effort.logged_time_minutes`, and removes `.tracker/.timer.toml`.
    pub fn stop_timer(&self) -> Result<StoppedTimerSession> {
        self.stop_timer_at(Utc::now())
    }

    /// Stops the currently running timer at an explicit UTC timestamp `stopped_at`.
    pub fn stop_timer_at(&self, stopped_at: DateTime<Utc>) -> Result<StoppedTimerSession> {
        let timer = self
            .read_active_timer()?
            .ok_or(TrackerError::NoActiveTimer)?;

        let elapsed_minutes = timer.elapsed_minutes_at(stopped_at)?;
        let (loc, mut detail) = self.load_item_detail_by_id(&timer.item_id)?;

        detail.manifest.effort.logged_time_minutes = detail
            .manifest
            .effort
            .logged_time_minutes
            .saturating_add(elapsed_minutes);
        detail.manifest.item.updated_at = stopped_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();

        self.write_item_manifest(
            &loc.phase_id,
            &loc.scope_id,
            &detail.manifest.item.id,
            &detail.manifest,
        )?;
        self.clear_active_timer()?;

        let total_logged_minutes = detail.manifest.effort.logged_time_minutes;
        Ok(StoppedTimerSession {
            timer,
            elapsed_minutes,
            total_logged_minutes,
            updated_item: detail,
        })
    }

    /// Returns the status and elapsed time of the active timer in `.tracker/.timer.toml`, if any.
    pub fn timer_status(&self) -> Result<Option<TimerStatus>> {
        self.timer_status_at(Utc::now())
    }

    /// Returns the status and elapsed time of the active timer evaluated at `now`.
    pub fn timer_status_at(&self, now: DateTime<Utc>) -> Result<Option<TimerStatus>> {
        let Some(timer) = self.read_active_timer()? else {
            return Ok(None);
        };
        let start = timer.parsed_started_at()?;
        let elapsed_seconds = (now - start).num_seconds().max(0) as u64;
        let elapsed_minutes = (elapsed_seconds / 60) as u32;

        Ok(Some(TimerStatus {
            timer,
            elapsed_minutes,
            elapsed_seconds,
        }))
    }

    /// Manually logs `minutes` of effort onto `item_id` (`effort.logged_time_minutes`).
    pub fn log_effort(&self, item_id: &str, minutes: u32) -> Result<ItemDetail> {
        let (loc, mut detail) = self.load_item_detail_by_id(item_id)?;
        detail.manifest.effort.logged_time_minutes = detail
            .manifest
            .effort
            .logged_time_minutes
            .saturating_add(minutes);
        detail.manifest.item.updated_at = now_rfc3339();

        self.write_item_manifest(
            &loc.phase_id,
            &loc.scope_id,
            &detail.manifest.item.id,
            &detail.manifest,
        )?;
        Ok(detail)
    }
}
