use std::fmt::Write;

use serde::Serialize;

use crate::error::{Result, TrackerError};
use crate::release::bisect::{read_bisect, Culprit};
use crate::release::candidate::{read_candidate, Phases};
use crate::release::config::ReleasePaths;
use crate::release::golden::read_golden;
use crate::release::holds::{ledger_holds, Hold};
use crate::release::ledger::read_ledger;
use crate::release::records::list_records;
use crate::release::rootfs::sha256_file;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusGolden {
    pub version: Option<String>,
    pub sha256: String,
    pub schema_version: u32,
    pub components_count: usize,
    pub packages_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusCandidate {
    pub candidate_id: String,
    pub created_at: String,
    pub iso_sha256: String,
    pub phases: Phases,
    pub changed_count: usize,
    pub holds_in_force_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusBisect {
    pub candidate_id: String,
    pub status: String,
    pub active: bool,
    pub failing_phase: String,
    pub round: u32,
    pub cap: u32,
    pub rounds_completed: u32,
    pub culprits: Vec<Culprit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StatusRelease {
    pub file: String,
    pub version: String,
    pub promoted_at: String,
    pub candidate_id: String,
    pub iso_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReleaseStatus {
    pub golden: Option<StatusGolden>,
    pub candidate: Option<StatusCandidate>,
    pub bisect: Option<StatusBisect>,
    pub holds: Vec<Hold>,
    pub failures_count: usize,
    pub last_release: Option<StatusRelease>,
}

/// Reads the release status summary from `.tracker/release/` without taking a write lock.
pub fn read_status(paths: &ReleasePaths) -> Result<ReleaseStatus> {
    let golden_path = paths.golden();
    let golden_doc = read_golden(&golden_path)?;
    let golden = match &golden_doc {
        Some(g) => {
            let sha256 = sha256_file(&golden_path)?;
            let version = g
                .os_release
                .get("VERSION_ID")
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            Some(StatusGolden {
                version,
                sha256,
                schema_version: g.schema_version,
                components_count: g.components.len(),
                packages_count: g.packages.len(),
            })
        }
        None => None,
    };

    let candidate = read_candidate(&paths.candidate())?.map(|c| StatusCandidate {
        candidate_id: c.candidate_id,
        created_at: c.created_at,
        iso_sha256: c.iso_sha256,
        phases: c.phases,
        changed_count: c.changed.len(),
        holds_in_force_count: c.holds_in_force.len(),
    });

    let bisect = read_bisect(&paths.bisect())?.map(|b| {
        let active = b.is_active();
        let rounds_completed = b.rounds_completed();
        StatusBisect {
            candidate_id: b.candidate_id,
            status: b.status.as_str().to_string(),
            active,
            failing_phase: b.failing_phase.as_str().to_string(),
            round: b.round,
            cap: b.cap,
            rounds_completed,
            culprits: b.culprits,
        }
    });

    let ledger = read_ledger(&paths.failures())?;
    let failures_count = ledger.failures.len();

    let records = list_records(&paths.releases_dir())?;
    let last_release = records.last().map(|(path, rec)| StatusRelease {
        file: path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string(),
        version: rec.version.clone(),
        promoted_at: rec.promoted_at.clone(),
        candidate_id: rec.candidate_id.clone(),
        iso_sha256: rec.iso_sha256.clone(),
    });

    let holds = match &golden_doc {
        Some(g) => ledger_holds(&ledger, g, &records),
        None => Vec::new(),
    };

    Ok(ReleaseStatus {
        golden,
        candidate,
        bisect,
        holds,
        failures_count,
        last_release,
    })
}

/// Converts a [`ReleaseStatus`] to a JSON value for `--json` output and MCP responses.
pub fn status_json(s: &ReleaseStatus) -> Result<serde_json::Value> {
    serde_json::to_value(s)
        .map_err(|e| TrackerError::ValidationError(format!("status serialization failed: {e}")))
}

/// Renders a human-readable multi-line text summary of `s` with a trailing newline.
pub fn render_status_text(s: &ReleaseStatus) -> String {
    let mut out = String::new();

    match &s.golden {
        Some(g) => {
            let ver = g.version.as_deref().unwrap_or("(no VERSION_ID)");
            let short_sha: String = g.sha256.chars().take(12).collect();
            let _ = write!(
                out,
                "golden: {ver} sha256 {short_sha} schema v{}, {} components",
                g.schema_version, g.components_count
            );
            if g.schema_version < 2 {
                out.push_str("  (schema v1: run neb release import)");
            }
            out.push('\n');
        }
        None => {
            out.push_str("golden: none\n");
        }
    }

    match &s.candidate {
        Some(c) => {
            let _ = writeln!(
                out,
                "candidate: {} build={} vm-live-boot={} vm-install-e2e={}, {} changed",
                c.candidate_id,
                c.phases.build.as_str(),
                c.phases.vm_live_boot.as_str(),
                c.phases.vm_install_e2e.as_str(),
                c.changed_count
            );
        }
        None => {
            out.push_str("candidate: none\n");
        }
    }

    match &s.bisect {
        Some(b) => {
            let _ = write!(
                out,
                "bisect: {} round {}/{} on {}",
                b.status, b.round, b.cap, b.failing_phase
            );
            if !b.active {
                out.push_str(" (inactive)");
            }
            out.push('\n');
        }
        None => {
            out.push_str("bisect: none\n");
        }
    }

    let _ = writeln!(out, "holds: {}", s.holds.len());
    for h in &s.holds {
        let _ = writeln!(
            out,
            "  {} held at {} (blocked {}, {})",
            h.component,
            h.held_version,
            h.blocked_version,
            h.reason.as_str()
        );
    }

    let _ = writeln!(out, "failures: {}", s.failures_count);

    match &s.last_release {
        Some(r) => {
            let _ = writeln!(
                out,
                "last release: {} promoted {} ({})",
                r.version, r.promoted_at, r.file
            );
        }
        None => {
            out.push_str("last release: none\n");
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use tempfile::tempdir;

    use super::*;
    use crate::release::bisect::{
        write_bisect, BisectState, BisectStatus, Culprit, Round, RoundResult, BISECT_SCHEMA_VERSION,
    };
    use crate::release::candidate::{
        write_candidate, Candidate, ChangedComponent, HoldInForce, PhaseState,
        CANDIDATE_SCHEMA_VERSION,
    };
    use crate::release::golden::{
        write_golden, Golden, GoldenComponent, Origin, GOLDEN_SCHEMA_VERSION,
    };
    use crate::release::holds::HoldReason;
    use crate::release::inventory::{BootInfo, CalamaresModules, IsoMeta};
    use crate::release::ledger::{record_failure, Failure, FailureSource, Phase, COMPONENT_CLASS};
    use crate::release::records::{
        write_record, RecordComponent, ReleaseRecord, RELEASE_RECORD_SCHEMA_VERSION,
    };
    use crate::storage::TrackerStorage;

    fn sample_golden(version_id: Option<&str>) -> Golden {
        let mut os_release = BTreeMap::new();
        if let Some(v) = version_id {
            os_release.insert("VERSION_ID".to_string(), v.to_string());
        }
        let mut components = BTreeMap::new();
        components.insert(
            "example-pkg".to_string(),
            GoldenComponent {
                version: "1.0-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("example-pkg".to_string(), "1.0-1".to_string())]),
            },
        );
        Golden {
            schema_version: GOLDEN_SCHEMA_VERSION,
            iso: IsoMeta {
                path: "/tmp/neb-test/golden.iso".to_string(),
                size_bytes: 1024,
                vmlinuz_sha256: "a".repeat(64),
                initrd_sha256: "b".repeat(64),
                extracted_at: "2026-11-10T12:00:00Z".to_string(),
            },
            os_release,
            boot: BootInfo { files: Vec::new() },
            calamares_modules: CalamaresModules {
                configs: Vec::new(),
                custom_modules: Vec::new(),
                presence: BTreeMap::new(),
            },
            custom_binaries: BTreeMap::new(),
            packages: BTreeMap::from([("example-pkg".to_string(), "1.0-1".to_string())]),
            components,
        }
    }

    #[test]
    fn empty_tree_returns_all_none_and_six_default_lines() {
        let tmp = tempdir().expect("tempdir");
        let storage = TrackerStorage::init_standard(tmp.path(), "test-proj").expect("init");
        let paths = ReleasePaths::new(&storage);

        let status = read_status(&paths).expect("read_status");
        assert_eq!(
            status,
            ReleaseStatus {
                golden: None,
                candidate: None,
                bisect: None,
                holds: Vec::new(),
                failures_count: 0,
                last_release: None,
            }
        );

        let text = render_status_text(&status);
        assert_eq!(
            text,
            "golden: none\n\
             candidate: none\n\
             bisect: none\n\
             holds: 0\n\
             failures: 0\n\
             last release: none\n"
        );
    }

    #[test]
    fn populated_tree_populates_all_fields_json_keys_and_text() {
        let tmp = tempdir().expect("tempdir");
        let storage = TrackerStorage::init_standard(tmp.path(), "test-proj").expect("init");
        let paths = ReleasePaths::new(&storage);

        let golden = sample_golden(Some("2026.11"));
        let golden_sha = write_golden(&paths.golden(), &golden).expect("write golden");

        let candidate = Candidate {
            schema_version: CANDIDATE_SCHEMA_VERSION,
            candidate_id: "aaaaaaaaaaaa-20261110T120000Z".to_string(),
            created_at: "2026-11-10T12:00:00Z".to_string(),
            iso_path: "/tmp/neb-test/candidate.iso".to_string(),
            iso_sha256: "a".repeat(64),
            squashfs_sha256: "b".repeat(64),
            snapshot: "20261110T000000Z".to_string(),
            artifact_cache: String::new(),
            hold_file: String::new(),
            hold_file_sha256: String::new(),
            repo_head: "a".repeat(40),
            unchanged_count: 1,
            phases: Phases {
                build: PhaseState::Pass,
                vm_live_boot: PhaseState::FailConfirmed,
                vm_install_e2e: PhaseState::Pending,
            },
            changed: vec![ChangedComponent {
                component: "example-pkg".to_string(),
                origin: Origin::Debian,
                from_version: Some("1.0-1".to_string()),
                to_version: "1.1-1".to_string(),
                kernel_line: false,
            }],
            holds_in_force: vec![HoldInForce {
                component: "example-pkg".to_string(),
                held_version: "1.0-1".to_string(),
                reason: HoldReason::Failure,
            }],
            history: Vec::new(),
        };
        write_candidate(&paths.candidate(), &candidate).expect("write candidate");

        let bisect = BisectState {
            schema_version: BISECT_SCHEMA_VERSION,
            candidate_id: "aaaaaaaaaaaa-20261110T120000Z".to_string(),
            failing_phase: Phase::VmLiveBoot,
            cap: 4,
            round: 2,
            status: BisectStatus::Culprit,
            started_at: "2026-11-10T12:10:00Z".to_string(),
            always_applied: Vec::new(),
            interaction: Vec::new(),
            culprits: vec![Culprit {
                component: "example-pkg".to_string(),
                version: "1.1-1".to_string(),
                round: 2,
            }],
            queue: Vec::new(),
            rounds: vec![
                Round {
                    round: 1,
                    started_at: "2026-11-10T12:11:00Z".to_string(),
                    result: RoundResult::Pass,
                    hold_file_sha256: "a".repeat(64),
                    classifier_rule: String::new(),
                    log: String::new(),
                    recorded_at: "2026-11-10T12:12:00Z".to_string(),
                    testing: vec!["other-pkg".to_string()],
                    held: vec!["example-pkg".to_string()],
                },
                Round {
                    round: 2,
                    started_at: "2026-11-10T12:13:00Z".to_string(),
                    result: RoundResult::Fail,
                    hold_file_sha256: "b".repeat(64),
                    classifier_rule: "unit-failed".to_string(),
                    log: "/tmp/neb-test/r2.log".to_string(),
                    recorded_at: "2026-11-10T12:14:00Z".to_string(),
                    testing: vec!["example-pkg".to_string()],
                    held: vec!["other-pkg".to_string()],
                },
            ],
            versions: BTreeMap::from([("example-pkg".to_string(), "1.1-1".to_string())]),
        };
        write_bisect(&paths.bisect(), &bisect).expect("write bisect");

        record_failure(
            &paths.failures(),
            Failure {
                component: "example-pkg".to_string(),
                version: "1.1-1".to_string(),
                phase: Phase::VmLiveBoot,
                class: COMPONENT_CLASS.to_string(),
                classifier_rule: "unit-failed".to_string(),
                log: "/tmp/neb-test/r2.log".to_string(),
                recorded_at: "2026-11-10T12:14:00Z".to_string(),
                source: FailureSource::Bisect,
                members: Vec::new(),
            },
        )
        .expect("record failure");

        let rec = ReleaseRecord {
            schema_version: RELEASE_RECORD_SCHEMA_VERSION,
            version: "2026.11".to_string(),
            promoted_at: "2026-11-09T10:00:00Z".to_string(),
            promoted_by: "neb release promote".to_string(),
            reason: String::new(),
            snapshot: "20261109T000000Z".to_string(),
            iso_sha256: "a".repeat(64),
            golden_sha256: golden_sha.clone(),
            hold_file_sha256: String::new(),
            candidate_id: "000000000000-20261109T100000Z".to_string(),
            bisect_rounds: 0,
            phases: BTreeMap::new(),
            components: BTreeMap::from([(
                "example-pkg".to_string(),
                RecordComponent {
                    version: "1.0-1".to_string(),
                    origin: Origin::Debian,
                    binaries: BTreeMap::from([("example-pkg".to_string(), "1.0-1".to_string())]),
                },
            )]),
            third_party_cache: BTreeMap::new(),
            candidate: None,
        };
        let rec_file = "2026.11-20261109T100000Z.toml";
        write_record(&paths.releases_dir().join(rec_file), &rec).expect("write record");

        let status = read_status(&paths).expect("read_status");
        let g = status.golden.as_ref().expect("golden present");
        assert_eq!(g.version.as_deref(), Some("2026.11"));
        assert_eq!(g.sha256, sha256_file(&paths.golden()).expect("sha256_file"));
        assert_eq!(g.sha256, golden_sha);
        assert_eq!(g.schema_version, 2);
        assert_eq!(g.components_count, 1);
        assert_eq!(g.packages_count, 1);

        let c = status.candidate.as_ref().expect("candidate present");
        assert_eq!(c.candidate_id, "aaaaaaaaaaaa-20261110T120000Z");
        assert_eq!(c.changed_count, 1);
        assert_eq!(c.holds_in_force_count, 1);

        let b = status.bisect.as_ref().expect("bisect present");
        assert_eq!(b.status, "culprit");
        assert!(!b.active);
        assert_eq!(b.failing_phase, "vm-live-boot");
        assert_eq!(b.round, 2);
        assert_eq!(b.cap, 4);
        assert_eq!(b.rounds_completed, 2);
        assert_eq!(b.culprits.len(), 1);

        assert_eq!(status.holds.len(), 1);
        assert_eq!(status.holds[0].component, "example-pkg");
        assert_eq!(status.holds[0].held_version, "1.0-1");
        assert_eq!(status.holds[0].blocked_version, "1.1-1");
        assert_eq!(status.holds[0].snapshot, "20261109T000000Z");
        assert_eq!(status.failures_count, 1);

        let lr = status.last_release.as_ref().expect("last release present");
        assert_eq!(lr.file, rec_file);
        assert_eq!(lr.version, "2026.11");
        assert_eq!(lr.promoted_at, "2026-11-09T10:00:00Z");

        let json_val = status_json(&status).expect("status_json");
        let obj = json_val.as_object().expect("json object");
        let mut keys: Vec<&str> = obj.keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            vec![
                "bisect",
                "candidate",
                "failures_count",
                "golden",
                "holds",
                "last_release"
            ]
        );

        let text = render_status_text(&status);
        assert!(text.contains(&format!(
            "golden: 2026.11 sha256 {} schema v2, 1 components",
            &golden_sha[..12]
        )));
        assert!(text.contains(
            "candidate: aaaaaaaaaaaa-20261110T120000Z build=pass vm-live-boot=fail_confirmed vm-install-e2e=pending, 1 changed"
        ));
        assert!(text.contains("bisect: culprit round 2/4 on vm-live-boot (inactive)"));
        assert!(text.contains("holds: 1\n  example-pkg held at 1.0-1 (blocked 1.1-1, failure)"));
        assert!(text.contains("failures: 1"));
        assert!(text.contains(
            "last release: 2026.11 promoted 2026-11-09T10:00:00Z (2026.11-20261109T100000Z.toml)"
        ));
    }

    #[test]
    fn v1_golden_shows_import_hint_and_empty_holds_and_corrupt_candidate_errors() {
        let tmp = tempdir().expect("tempdir");
        let storage = TrackerStorage::init_standard(tmp.path(), "test-proj").expect("init");
        let paths = ReleasePaths::new(&storage);

        fs::create_dir_all(paths.root()).expect("mkdir release");
        let v1_toml = r#"
[iso]
path = "/tmp/neb-test/v1.iso"
size_bytes = 512
vmlinuz_sha256 = "aaaa"
initrd_sha256 = "bbbb"
extracted_at = "2026-11-01T00:00:00Z"

[os_release]
ID = "debian"

[boot]
files = []

[calamares_modules]
configs = []
custom_modules = []

[calamares_modules.presence]

[custom_binaries]

[packages]
example-pkg = "1.0-1"
"#;
        fs::write(paths.golden(), v1_toml).expect("write v1 golden");

        record_failure(
            &paths.failures(),
            Failure {
                component: "example-pkg".to_string(),
                version: "1.1-1".to_string(),
                phase: Phase::Build,
                class: COMPONENT_CLASS.to_string(),
                classifier_rule: "rule".to_string(),
                log: "/tmp/neb-test/build.log".to_string(),
                recorded_at: "2026-11-10T12:00:00Z".to_string(),
                source: FailureSource::SingleChange,
                members: Vec::new(),
            },
        )
        .expect("record failure");

        let status = read_status(&paths).expect("read_status v1");
        let g = status.golden.as_ref().expect("golden v1");
        assert_eq!(g.schema_version, 1);
        assert_eq!(g.version, None);
        assert_eq!(g.components_count, 0);
        assert_eq!(g.packages_count, 1);
        assert!(status.holds.is_empty());

        let text = render_status_text(&status);
        assert!(text.contains("(no VERSION_ID)"));
        assert!(text.contains("run neb release import"));

        // Whitespace-only VERSION_ID normalizes to None and active bisect omits " (inactive)".
        let ws_golden = sample_golden(Some("   "));
        write_golden(&paths.golden(), &ws_golden).expect("write whitespace VERSION_ID golden");
        let active_bisect = BisectState {
            schema_version: BISECT_SCHEMA_VERSION,
            candidate_id: "aaaaaaaaaaaa-20261110T120000Z".to_string(),
            failing_phase: Phase::VmLiveBoot,
            cap: 4,
            round: 1,
            status: BisectStatus::Active,
            started_at: "2026-11-10T12:10:00Z".to_string(),
            always_applied: Vec::new(),
            interaction: Vec::new(),
            culprits: Vec::new(),
            queue: Vec::new(),
            rounds: Vec::new(),
            versions: BTreeMap::new(),
        };
        write_bisect(&paths.bisect(), &active_bisect).expect("write active bisect");

        let active_status = read_status(&paths).expect("read_status with active bisect");
        assert_eq!(active_status.golden.as_ref().unwrap().version, None);
        assert!(active_status.bisect.as_ref().unwrap().active);
        let active_text = render_status_text(&active_status);
        assert!(active_text.contains("golden: (no VERSION_ID)"));
        assert!(active_text.contains("bisect: active round 1/4 on vm-live-boot\n"));
        assert!(!active_text.contains("(inactive)"));

        // Corrupt candidate.toml -> Err.
        fs::write(paths.candidate(), "not = [valid toml").expect("write corrupt candidate");
        assert!(read_status(&paths).is_err());
    }
}
