pub mod agent_context;
pub mod error;
pub mod git_sync;
pub mod models;
pub mod storage;
pub mod watcher;
pub mod workflow;

pub use agent_context::*;
pub use error::{Result, TrackerError};
pub use git_sync::*;
pub use models::*;
pub use storage::*;
pub use watcher::*;
pub use workflow::{
    format_boilerplate_description, CreateItemInput, PhaseEffortValuation, ProjectMetricsReport,
    RecurringItemHotspot, ScopeEffortValuation, ScopeHotspot, StagnantItemEntry,
    StoppedTimerSession, TimerStatus,
};

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::TimeZone;

    use crate::error::TrackerError;
    use crate::models::{
        ActiveTimer, GitSyncMode, ItemManifest, ItemStatus, ItemType, PhaseManifest, PhaseStatus,
        ProjectManifest, ScopeManifest, Severity,
    };

    #[test]
    fn test_project_manifest_round_trip_and_unquoted_datetime() {
        let raw = r#"
[project]
name = "Nebulous"
version = "0.1.0"
active_phase = "01_core_engine"
default_scope = "inbox"
created_at = 2026-11-01T21:00:00Z

[counters]
bug = 1
feature = 4

[phases]
registry = [
    { id = "01_core_engine", title = "Core Engine & Parsing", path = "phases/01_core_engine" },
    { id = "02_cloud_sync",  title = "Cloud & Remote Sync",    path = "phases/02_cloud_sync" }
]

[metrics]
target_daily_hours = 6.0
stagnant_item_threshold_days = 4

[git]
sync_mode = "manual"
sync_interval_minutes = 30
remote = "origin"
branch = "main"
"#;

        let manifest =
            ProjectManifest::from_toml_str(raw).expect("should deserialize project.toml");
        assert_eq!(manifest.project.name, "Nebulous");
        assert_eq!(manifest.project.active_phase, "01_core_engine");
        assert_eq!(manifest.project.default_scope, "inbox");
        assert_eq!(manifest.project.created_at, "2026-11-01T21:00:00Z");
        assert!(manifest.project.parsed_created_at().is_ok());
        assert_eq!(manifest.counters.bug, 1);
        assert_eq!(manifest.counters.feature, 4);
        assert_eq!(manifest.phases.registry.len(), 2);
        assert_eq!(manifest.metrics.target_daily_hours, 6.0);
        assert_eq!(manifest.metrics.stagnant_item_threshold_days, 4);
        assert_eq!(manifest.git.sync_mode, GitSyncMode::Manual);
        assert_eq!(manifest.git.remote, "origin");

        let serialized = manifest
            .to_toml_string()
            .expect("should serialize project.toml");
        let reparsed =
            ProjectManifest::from_toml_str(&serialized).expect("should round-trip project.toml");
        assert_eq!(manifest, reparsed);
    }

    #[test]
    fn test_project_manifest_minimal_defaults() {
        let minimal = r#"
[project]
name = "MinimalProject"
active_phase = "01_init"
created_at = "2026-11-01T21:00:00Z"
"#;
        let manifest =
            ProjectManifest::from_toml_str(minimal).expect("should deserialize minimal project");
        assert_eq!(manifest.project.version, "0.1.0");
        assert_eq!(manifest.project.default_scope, "inbox");
        assert_eq!(manifest.counters.bug, 0);
        assert_eq!(manifest.counters.feature, 0);
        assert!(manifest.phases.registry.is_empty());
        assert_eq!(manifest.metrics.target_daily_hours, 6.0);
        assert_eq!(manifest.metrics.stagnant_item_threshold_days, 4);
        assert_eq!(manifest.git.sync_mode, GitSyncMode::Manual);
        assert_eq!(manifest.git.sync_interval_minutes, 30);
    }

    #[test]
    fn test_phase_manifest_round_trip_and_optional_dates() {
        let raw = r#"
[phase]
id = "01_core_engine"
title = "Core Engine & Parsing"
status = "in_progress"
start_date = 2026-11-01

[scopes]
manifest = [
    { id = "backend_core",     type = "rust_crate", path = "domains/backend_core" },
    { id = "nebulous_cli",     type = "cli_crate",  path = "domains/nebulous_cli" },
    { id = "cosmic_interface", type = "ui_crate",   path = "domains/cosmic_interface" }
]

[summary]
total_items = 8
completed_items = 2
"#;

        let manifest = PhaseManifest::from_toml_str(raw).expect("should deserialize phase.toml");
        assert_eq!(manifest.phase.id, "01_core_engine");
        assert_eq!(manifest.phase.status, PhaseStatus::InProgress);
        assert_eq!(manifest.phase.start_date.as_deref(), Some("2026-11-01"));
        assert!(manifest.phase.parsed_start_date().unwrap().is_some());
        assert_eq!(manifest.scopes.manifest.len(), 3);
        assert_eq!(manifest.scopes.manifest[2].id, "cosmic_interface");
        assert_eq!(manifest.scopes.manifest[2].scope_type, "ui_crate");
        assert_eq!(manifest.summary.total_items, 8);
        assert_eq!(manifest.summary.completed_items, 2);

        let serialized = manifest
            .to_toml_string()
            .expect("should serialize phase.toml");
        let reparsed =
            PhaseManifest::from_toml_str(&serialized).expect("should round-trip phase.toml");
        assert_eq!(manifest, reparsed);
    }

    #[test]
    fn test_scope_manifest_populated_and_empty_arrays() {
        let populated = r#"
[scope]
id = "backend_core"
target = "crates/backend_core"
tech_stack = ["rust", "serde", "tokio"]

[items]
bugs = [
    { id = "BUG-0001", title = "TOML deserialization fails on empty array", status = "open", severity = "high" }
]
features = [
    { id = "FEAT-0004", title = "Parallel file-tree walker", status = "in_progress", severity = "medium" }
]
"#;
        let scope =
            ScopeManifest::from_toml_str(populated).expect("should parse populated scope.toml");
        assert_eq!(scope.scope.id, "backend_core");
        assert_eq!(scope.total_items(), 2);
        assert_eq!(scope.items.bugs[0].id, "BUG-0001");
        assert_eq!(scope.items.bugs[0].status, ItemStatus::Open);
        assert_eq!(scope.items.bugs[0].severity, Severity::High);
        assert_eq!(scope.items.features[0].id, "FEAT-0004");
        assert_eq!(scope.items.features[0].status, ItemStatus::InProgress);

        // Edge case 1: Explicit empty arrays `bugs = []` and `features = []`
        let explicit_empty = r#"
[scope]
id = "inbox"
target = ".tracker"
tech_stack = []

[items]
bugs = []
features = []
"#;
        let empty_scope = ScopeManifest::from_toml_str(explicit_empty)
            .expect("should parse scope.toml with explicit empty arrays");
        assert!(empty_scope.items.bugs.is_empty());
        assert!(empty_scope.items.features.is_empty());
        assert_eq!(empty_scope.total_items(), 0);

        // Edge case 2: Omitted `features` or omitted `[items]` section entirely
        let omitted_items = r#"
[scope]
id = "cosmic_interface"
target = "crates/cosmic_interface"
"#;
        let omitted_scope = ScopeManifest::from_toml_str(omitted_items)
            .expect("should parse scope.toml with omitted [items] section");
        assert!(omitted_scope.scope.tech_stack.is_empty());
        assert!(omitted_scope.items.bugs.is_empty());
        assert!(omitted_scope.items.features.is_empty());

        let round_trip = empty_scope.to_toml_string().unwrap();
        let reparsed = ScopeManifest::from_toml_str(&round_trip).unwrap();
        assert_eq!(empty_scope, reparsed);
    }

    #[test]
    fn test_item_manifest_round_trip_and_empty_previous_fixes() {
        let raw = r#"
[item]
id = "BUG-0001"
type = "bug"
title = "TOML deserialization fails on empty array"
status = "open"
severity = "high"
scope_ref = "backend_core"
created_at = 2026-11-01T21:15:00Z
updated_at = 2026-11-01T21:40:00Z

[effort]
logged_time_minutes = 45
iteration_count = 2

[history]
recurrence_count = 1
previous_fixes = [
    { date = 2026-11-01, commit = "a4f89c1", summary = "Added default trait fallback" }
]
"#;

        let item = ItemManifest::from_toml_str(raw).expect("should deserialize item.toml");
        assert_eq!(item.item.id, "BUG-0001");
        assert_eq!(item.item.item_type, ItemType::Bug);
        assert_eq!(item.item.status, ItemStatus::Open);
        assert_eq!(item.item.severity, Severity::High);
        assert_eq!(item.effort.logged_time_minutes, 45);
        assert_eq!(item.effort.iteration_count, 2);
        assert_eq!(item.history.recurrence_count, 1);
        assert_eq!(item.history.previous_fixes.len(), 1);
        assert_eq!(item.history.previous_fixes[0].commit, "a4f89c1");
        assert!(item.history.previous_fixes[0].parsed_date().is_ok());

        let stub = item.to_stub();
        assert_eq!(stub.id, "BUG-0001");
        assert_eq!(stub.status, ItemStatus::Open);
        assert_eq!(stub.severity, Severity::High);

        let serialized = item.to_toml_string().expect("should serialize item.toml");
        let reparsed =
            ItemManifest::from_toml_str(&serialized).expect("should round-trip item.toml");
        assert_eq!(item, reparsed);

        // Edge case: `previous_fixes = []` or omitted `[history]`
        let empty_fixes = r#"
[item]
id = "FEAT-0001"
type = "feature"
title = "Implement 4-tier TOML models"
status = "in_progress"
severity = "medium"
scope_ref = "backend_core"
created_at = "2026-11-01T21:15:00Z"
updated_at = "2026-11-01T21:40:00Z"

[history]
recurrence_count = 0
previous_fixes = []
"#;
        let feat = ItemManifest::from_toml_str(empty_fixes)
            .expect("should parse item.toml with empty previous_fixes");
        assert_eq!(feat.item.item_type, ItemType::Feature);
        assert!(feat.history.previous_fixes.is_empty());
        assert_eq!(feat.effort.logged_time_minutes, 0);
    }

    #[test]
    fn test_enum_parsing_and_severity_ordering() {
        // ItemStatus parsing
        assert_eq!(ItemStatus::from_str("open").unwrap(), ItemStatus::Open);
        assert_eq!(
            ItemStatus::from_str("IN_PROGRESS").unwrap(),
            ItemStatus::InProgress
        );
        assert_eq!(
            ItemStatus::from_str("in-progress").unwrap(),
            ItemStatus::InProgress
        );
        assert_eq!(
            ItemStatus::from_str("pending").unwrap(),
            ItemStatus::Pending
        );
        assert_eq!(ItemStatus::from_str("closed").unwrap(), ItemStatus::Closed);
        assert_eq!(ItemStatus::from_str("merged").unwrap(), ItemStatus::Merged);
        assert!(!ItemStatus::Open.is_completed());
        assert!(!ItemStatus::InProgress.is_completed());
        assert!(ItemStatus::Closed.is_completed());
        assert!(ItemStatus::Merged.is_completed());
        assert!(matches!(
            ItemStatus::from_str("invalid_status"),
            Err(TrackerError::InvalidEnumValue {
                enum_name: "ItemStatus",
                ..
            })
        ));

        // Severity parsing and ordering
        assert_eq!(Severity::from_str("low").unwrap(), Severity::Low);
        assert_eq!(Severity::from_str("med").unwrap(), Severity::Medium);
        assert_eq!(Severity::from_str("HIGH").unwrap(), Severity::High);
        assert_eq!(Severity::from_str("crit").unwrap(), Severity::Critical);
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::High < Severity::Critical);
        assert!(matches!(
            Severity::from_str("blocker"),
            Err(TrackerError::InvalidEnumValue {
                enum_name: "Severity",
                ..
            })
        ));

        // ItemType parsing and ID formatting
        assert_eq!(ItemType::from_str("bug").unwrap(), ItemType::Bug);
        assert_eq!(ItemType::from_str("feat").unwrap(), ItemType::Feature);
        assert_eq!(ItemType::from_str("feature").unwrap(), ItemType::Feature);
        assert_eq!(ItemType::Bug.format_id(1), "BUG-0001");
        assert_eq!(ItemType::Feature.format_id(42), "FEAT-0042");

        // PhaseStatus & GitSyncMode parsing
        assert_eq!(
            PhaseStatus::from_str("in-progress").unwrap(),
            PhaseStatus::InProgress
        );
        assert_eq!(
            GitSyncMode::from_str("scheduled").unwrap(),
            GitSyncMode::Scheduled
        );
    }

    #[test]
    fn test_active_timer_serialization_and_elapsed_calculation() {
        let timer = ActiveTimer::new(
            "BUG-0001",
            Some("01_core_engine".to_string()),
            Some("backend_core".to_string()),
            "2026-11-01T21:00:00Z",
        );
        let raw = timer.to_toml_string().expect("should serialize timer");
        let parsed = ActiveTimer::from_toml_str(&raw).expect("should deserialize timer");
        assert_eq!(timer, parsed);

        let later = chrono::Utc
            .with_ymd_and_hms(2026, 11, 1, 21, 45, 30)
            .unwrap();
        assert_eq!(parsed.elapsed_minutes_at(later).unwrap(), 45);

        // If `now` is earlier than `started_at`, elapsed minutes clamps to 0
        let earlier = chrono::Utc
            .with_ymd_and_hms(2026, 11, 1, 20, 30, 0)
            .unwrap();
        assert_eq!(parsed.elapsed_minutes_at(earlier).unwrap(), 0);
    }

    #[test]
    fn test_local_datetime_item_stub_defaults_and_malformed_inputs() {
        use crate::models::{parse_date, parse_timestamp_utc};

        // Local TOML datetimes without 'Z' offset
        let dt = parse_timestamp_utc("2026-11-01T21:00:00").expect("should parse local datetime");
        assert_eq!(
            dt,
            chrono::Utc.with_ymd_and_hms(2026, 11, 1, 21, 0, 0).unwrap()
        );
        let d = parse_date("2026-11-01T21:00:00").expect("should extract date from local datetime");
        assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2026, 11, 1).unwrap());

        // Invalid datetime string
        assert!(matches!(
            parse_timestamp_utc("not-a-timestamp"),
            Err(TrackerError::InvalidDateTime { .. })
        ));
        assert!(matches!(
            parse_date("not-a-date"),
            Err(TrackerError::InvalidDateTime { .. })
        ));

        // ItemStub with omitted status and severity uses defaults (open, medium)
        let scope_with_minimal_stub = r#"
[scope]
id = "backend_core"
target = "crates/backend_core"

[items]
bugs = [
    { id = "BUG-0009", title = "Minimal stub defaults" }
]
"#;
        let scope = ScopeManifest::from_toml_str(scope_with_minimal_stub)
            .expect("should parse ItemStub with default status and severity");
        assert_eq!(scope.items.bugs[0].status, ItemStatus::Open);
        assert_eq!(scope.items.bugs[0].severity, Severity::Medium);

        // Malformed TOML: non-string/non-datetime in created_at
        let bad_date_type = r#"
[project]
name = "BadDate"
active_phase = "01_init"
created_at = 12345
"#;
        assert!(matches!(
            ProjectManifest::from_toml_str(bad_date_type),
            Err(TrackerError::TomlDeserialize { .. })
        ));

        // Malformed TOML: negative integer in u32 counter
        let negative_counter = r#"
[project]
name = "BadCounter"
active_phase = "01_init"
created_at = "2026-11-01T21:00:00Z"

[counters]
bug = -1
"#;
        assert!(matches!(
            ProjectManifest::from_toml_str(negative_counter),
            Err(TrackerError::TomlDeserialize { .. })
        ));
    }
}
