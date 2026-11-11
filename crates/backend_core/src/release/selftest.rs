use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::io;
use std::panic::{catch_unwind, AssertUnwindSafe};

use chrono::{DateTime, TimeZone, Utc};
use serde::Serialize;

use crate::release::approvals::{Approval, Approvals};
use crate::release::bisect::{
    next_round, record_round, start_bisect, BisectStatus, NextOutcome, RecordOutcome, RoundInput,
    StartOutcome,
};
use crate::release::candidate::{
    Candidate, ChangedComponent, PhaseRecord, PhaseResult, PhaseState, Phases,
    CANDIDATE_SCHEMA_VERSION,
};
use crate::release::component::is_abi_rename;
use crate::release::config::{Components, Guardrails, DEFAULT_AGENT_ENV_MARKERS};
use crate::release::debver;
use crate::release::gate::{require_user, DenyReason, Terminal};
use crate::release::golden::{Golden, GoldenComponent, Origin, GOLDEN_SCHEMA_VERSION};
use crate::release::holds::{hold_file, ledger_holds, render_hold_file, HoldFile};
use crate::release::inventory::{
    BootInfo, CalamaresModules, InstalledPackage, IsoInventory, IsoMeta,
};
use crate::release::iso::{ExtractedIso, IsoIdentity};
use crate::release::ledger::{Failure, FailureSource, Ledger, Phase, COMPONENT_CLASS};
use crate::release::loopstate::{
    record_attempt, AttemptInput, AttemptStatus, DiffFingerprint, Stage,
};
use crate::release::manifest::Manifest;
use crate::release::promote::{prepare_promote, NotReadyReason, PreparedPromote, PromoteInput};
use crate::release::variance::{evaluate_against_golden, AbiRename, Upgrade};

const DEBVER_CASES: &str = include_str!("testdata/debver_cases.txt");

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CaseResult {
    pub name: &'static str,
    pub passed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelfTestReport {
    pub cases: Vec<CaseResult>,
}

impl SelfTestReport {
    pub fn all_passed(&self) -> bool {
        self.cases.iter().all(|c| c.passed)
    }

    pub fn failed(&self) -> Vec<&CaseResult> {
        self.cases.iter().filter(|c| !c.passed).collect()
    }
}

pub const SELF_TEST_CASES: &[&str] = &[
    "debver-table",
    "variance-upgrade-passes",
    "variance-downgrade-blocks",
    "variance-removal-blocks-unless-approved",
    "abi-rename-kernel",
    "abi-rename-t64",
    "loop-breaker-three-strikes",
    "gate-refuses-no-tty",
    "gate-refuses-agent-marker",
    "hold-file-generation",
    "bisect-halving",
    "bisect-interaction",
    "bisect-cap",
    "promote-preconditions",
];

type CaseFn = fn() -> Result<(), String>;

const CASES: &[(&str, CaseFn)] = &[
    ("debver-table", case_debver_table),
    ("variance-upgrade-passes", case_variance_upgrade_passes),
    ("variance-downgrade-blocks", case_variance_downgrade_blocks),
    (
        "variance-removal-blocks-unless-approved",
        case_variance_removal_blocks_unless_approved,
    ),
    ("abi-rename-kernel", case_abi_rename_kernel),
    ("abi-rename-t64", case_abi_rename_t64),
    (
        "loop-breaker-three-strikes",
        case_loop_breaker_three_strikes,
    ),
    ("gate-refuses-no-tty", case_gate_refuses_no_tty),
    ("gate-refuses-agent-marker", case_gate_refuses_agent_marker),
    ("hold-file-generation", case_hold_file_generation),
    ("bisect-halving", case_bisect_halving),
    ("bisect-interaction", case_bisect_interaction),
    ("bisect-cap", case_bisect_cap),
    ("promote-preconditions", case_promote_preconditions),
];

/// Runs all built-in release self-test cases in order, catching panics per case.
pub fn run_self_test() -> SelfTestReport {
    run_cases(CASES)
}

fn run_cases(cases: &[(&'static str, CaseFn)]) -> SelfTestReport {
    let mut results = Vec::with_capacity(cases.len());
    for &(name, func) in cases {
        let outcome = catch_unwind(AssertUnwindSafe(func));
        let result = match outcome {
            Ok(Ok(())) => CaseResult {
                name,
                passed: true,
                detail: String::new(),
            },
            Ok(Err(detail)) => CaseResult {
                name,
                passed: false,
                detail,
            },
            Err(payload) => {
                let msg = if let Some(s) = payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                CaseResult {
                    name,
                    passed: false,
                    detail: format!("panicked: {msg}"),
                }
            }
        };
        results.push(result);
    }
    SelfTestReport { cases: results }
}

fn ensure(cond: bool, msg: impl FnOnce() -> String) -> Result<(), String> {
    if cond {
        Ok(())
    } else {
        Err(msg())
    }
}

fn fixed_now() -> Result<DateTime<Utc>, String> {
    Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0)
        .single()
        .ok_or_else(|| "invalid fixed self-test timestamp".to_string())
}

fn golden_with(components_list: &[(&str, &str)]) -> Golden {
    let mut components = BTreeMap::new();
    let mut packages = BTreeMap::new();
    for &(id, ver) in components_list {
        packages.insert(id.to_string(), ver.to_string());
        components.insert(
            id.to_string(),
            GoldenComponent {
                version: ver.to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([(id.to_string(), ver.to_string())]),
            },
        );
    }

    Golden {
        schema_version: GOLDEN_SCHEMA_VERSION,
        iso: IsoMeta {
            path: "/tmp/neb-test/golden.iso".to_string(),
            size_bytes: 1024,
            vmlinuz_sha256: "a".repeat(64),
            initrd_sha256: "b".repeat(64),
            extracted_at: "2026-11-10T12:00:00Z".to_string(),
        },
        os_release: BTreeMap::from([("VERSION_ID".to_string(), "2026.11".to_string())]),
        boot: BootInfo { files: Vec::new() },
        calamares_modules: CalamaresModules {
            configs: Vec::new(),
            custom_modules: Vec::new(),
            presence: BTreeMap::new(),
        },
        custom_binaries: BTreeMap::new(),
        packages,
        components,
    }
}

fn candidate_with(changed_list: &[(&str, Option<&str>, &str)]) -> Candidate {
    let changed = changed_list
        .iter()
        .map(|&(id, from, to)| ChangedComponent {
            component: id.to_string(),
            origin: Origin::Debian,
            from_version: from.map(ToOwned::to_owned),
            to_version: to.to_string(),
            kernel_line: false,
        })
        .collect();

    Candidate {
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
            vm_live_boot: PhaseState::Pass,
            vm_install_e2e: PhaseState::Pass,
        },
        changed,
        holds_in_force: Vec::new(),
        history: Vec::new(),
    }
}

fn case_debver_table() -> Result<(), String> {
    let mut count = 0usize;
    for (idx, raw_line) in DEBVER_CASES.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split_whitespace().collect();
        ensure(parts.len() == 3, || {
            format!("line {}: expected 3 fields, got {}", idx + 1, parts.len())
        })?;
        let (a, op, b) = (parts[0], parts[1], parts[2]);
        let expected = match op {
            "lt" => Ordering::Less,
            "eq" => Ordering::Equal,
            "gt" => Ordering::Greater,
            other => return Err(format!("line {}: unknown operator '{other}'", idx + 1)),
        };

        let actual = debver::compare(a, b);
        ensure(actual == expected, || {
            format!(
                "line {}: compare({a:?}, {b:?}) = {actual:?}, expected {expected:?}",
                idx + 1
            )
        })?;

        let rev = debver::compare(b, a);
        let expected_rev = expected.reverse();
        ensure(rev == expected_rev, || {
            format!(
                "line {}: compare({b:?}, {a:?}) = {rev:?}, expected {expected_rev:?}",
                idx + 1
            )
        })?;
        count += 1;
    }

    ensure(count >= 60, || {
        format!("expected at least 60 debver cases, found {count}")
    })
}

fn case_variance_upgrade_passes() -> Result<(), String> {
    let golden = golden_with(&[("example-pkg", "1.0-1")]);
    let live = golden_with(&[("example-pkg", "1.1-1")]);
    let report = evaluate_against_golden(&live, &golden, &Approvals::default())
        .map_err(|e| e.to_string())?;
    ensure(report.is_pass(), || {
        format!("expected upgrade to pass, got {:?}", report.violations)
    })?;
    ensure(
        report.upgrades
            == vec![Upgrade {
                item: "example-pkg".to_string(),
                from_version: "1.0-1".to_string(),
                to_version: "1.1-1".to_string(),
            }],
        || format!("unexpected upgrades: {:?}", report.upgrades),
    )
}

fn case_variance_downgrade_blocks() -> Result<(), String> {
    let golden = golden_with(&[("example-pkg", "1.0-1")]);
    let live = golden_with(&[("example-pkg", "0.9-1")]);
    let report = evaluate_against_golden(&live, &golden, &Approvals::default())
        .map_err(|e| e.to_string())?;
    ensure(report.violations.len() == 1, || {
        format!("expected 1 violation, got {:?}", report.violations)
    })?;
    ensure(
        report.violations[0].kind == "golden_component_downgrade",
        || {
            format!(
                "expected golden_component_downgrade, got {}",
                report.violations[0].kind
            )
        },
    )
}

fn case_variance_removal_blocks_unless_approved() -> Result<(), String> {
    let golden = golden_with(&[("example-pkg", "1.0-1")]);
    let live = golden_with(&[]);
    let report = evaluate_against_golden(&live, &golden, &Approvals::default())
        .map_err(|e| e.to_string())?;
    ensure(report.violations.len() == 1, || {
        format!("expected 1 removal violation, got {:?}", report.violations)
    })?;
    ensure(
        report.violations[0].kind == "golden_component_removed",
        || {
            format!(
                "expected golden_component_removed, got {}",
                report.violations[0].kind
            )
        },
    )?;

    let approvals = Approvals {
        approvals: vec![Approval {
            item_name: "example-pkg".to_string(),
            reason: "self-test approval".to_string(),
            approved: true,
            timestamp: "2026-11-10T12:00:00Z".to_string(),
            version: Some("1.0-1".to_string()),
        }],
    };
    let approved_report =
        evaluate_against_golden(&live, &golden, &approvals).map_err(|e| e.to_string())?;
    ensure(approved_report.is_pass(), || {
        format!(
            "expected approved removal to pass, got {:?}",
            approved_report.violations
        )
    })
}

fn case_abi_rename_kernel() -> Result<(), String> {
    ensure(
        is_abi_rename(
            "linux-image-7.2.15-x64v3-xanmod1",
            "linux-image-7.3.3-x64v3-xanmod1",
        ),
        || "expected kernel image ABI rename to match".to_string(),
    )?;

    let mut golden = golden_with(&[]);
    golden.components.insert(
        "linux-xanmod-x64v3".to_string(),
        GoldenComponent {
            version: "7.2.15-xanmod1-0".to_string(),
            origin: Origin::ThirdParty,
            kernel_line: true,
            source_unknown: false,
            binaries: BTreeMap::from([
                (
                    "linux-headers-7.2.15-x64v3-xanmod1".to_string(),
                    "7.2.15-x64v3-xanmod1-0".to_string(),
                ),
                (
                    "linux-image-7.2.15-x64v3-xanmod1".to_string(),
                    "7.2.15-x64v3-xanmod1-0".to_string(),
                ),
                (
                    "linux-xanmod-x64v3".to_string(),
                    "7.2.15-xanmod1-0".to_string(),
                ),
            ]),
        },
    );

    let mut live = golden_with(&[]);
    live.components.insert(
        "linux-xanmod-x64v3".to_string(),
        GoldenComponent {
            version: "7.3.3-xanmod1-0".to_string(),
            origin: Origin::ThirdParty,
            kernel_line: true,
            source_unknown: false,
            binaries: BTreeMap::from([
                (
                    "linux-headers-7.3.3-x64v3-xanmod1".to_string(),
                    "7.3.3-x64v3-xanmod1-0".to_string(),
                ),
                (
                    "linux-image-7.3.3-x64v3-xanmod1".to_string(),
                    "7.3.3-x64v3-xanmod1-0".to_string(),
                ),
                (
                    "linux-xanmod-x64v3".to_string(),
                    "7.3.3-xanmod1-0".to_string(),
                ),
            ]),
        },
    );

    let report = evaluate_against_golden(&live, &golden, &Approvals::default())
        .map_err(|e| e.to_string())?;
    ensure(report.is_pass(), || {
        format!(
            "expected kernel upgrade to pass, got {:?}",
            report.violations
        )
    })?;
    ensure(report.upgrades.len() == 1, || {
        format!("expected 1 upgrade, got {:?}", report.upgrades)
    })?;
    ensure(report.abi_renames.len() == 2, || {
        format!("expected 2 ABI renames, got {:?}", report.abi_renames)
    })
}

fn case_abi_rename_t64() -> Result<(), String> {
    ensure(is_abi_rename("libexample1", "libexample1t64"), || {
        "expected libexample1 -> libexample1t64 to be an ABI rename".to_string()
    })?;
    ensure(!is_abi_rename("libexample1", "libother1"), || {
        "expected libexample1 -> libother1 not to be an ABI rename".to_string()
    })?;

    let mut golden = golden_with(&[]);
    golden.components.insert(
        "example-lib".to_string(),
        GoldenComponent {
            version: "1.0-1".to_string(),
            origin: Origin::Debian,
            kernel_line: false,
            source_unknown: false,
            binaries: BTreeMap::from([("libexample1".to_string(), "1.0-1".to_string())]),
        },
    );

    let mut live = golden_with(&[]);
    live.components.insert(
        "example-lib".to_string(),
        GoldenComponent {
            version: "1.0-1".to_string(),
            origin: Origin::Debian,
            kernel_line: false,
            source_unknown: false,
            binaries: BTreeMap::from([("libexample1t64".to_string(), "1.0-1".to_string())]),
        },
    );

    let report = evaluate_against_golden(&live, &golden, &Approvals::default())
        .map_err(|e| e.to_string())?;
    ensure(report.is_pass(), || {
        format!("expected t64 rename to pass, got {:?}", report.violations)
    })?;
    ensure(
        report.abi_renames
            == vec![AbiRename {
                component: "example-lib".to_string(),
                from_package: "libexample1".to_string(),
                to_package: "libexample1t64".to_string(),
            }],
        || format!("unexpected abi_renames: {:?}", report.abi_renames),
    )
}

fn case_loop_breaker_three_strikes() -> Result<(), String> {
    let tmp = tempfile::TempDir::new().map_err(|e| e.to_string())?;
    let path = tmp.path().join("item-001-example.toml");
    let guardrails = Guardrails::default();

    let attempt = |ts: &str, diff_fp: &str| AttemptInput {
        stage: Stage::Build,
        status: AttemptStatus::Fail,
        error_summary: "E: Unable to locate package example-pkg".to_string(),
        log_file: None,
        diff: DiffFingerprint {
            fingerprint: diff_fp.to_string(),
            modified_files: vec!["scripts/stages/03-core.sh".to_string()],
        },
        timestamp: ts.to_string(),
    };

    let out1 = record_attempt(
        &path,
        attempt("2026-11-10T12:00:00Z", "diff-1"),
        &guardrails,
    )
    .map_err(|e| e.to_string())?;
    ensure(!out1.loop_locked && out1.retry_count == 1, || {
        format!("after 1st fail expected unlocked retry=1, got {out1:?}")
    })?;

    let out2 = record_attempt(
        &path,
        attempt("2026-11-10T12:05:00Z", "diff-2"),
        &guardrails,
    )
    .map_err(|e| e.to_string())?;
    ensure(!out2.loop_locked && out2.retry_count == 2, || {
        format!("after 2nd fail expected unlocked retry=2, got {out2:?}")
    })?;

    let out3 = record_attempt(
        &path,
        attempt("2026-11-10T12:10:00Z", "diff-3"),
        &guardrails,
    )
    .map_err(|e| e.to_string())?;
    ensure(out3.loop_locked && out3.retry_count == 3, || {
        format!("after 3rd fail expected locked retry=3, got {out3:?}")
    })
}

struct FakeTerm {
    stdin_tty: bool,
    stdout_tty: bool,
    marker: Option<(&'static str, &'static str)>,
    prompted: bool,
}

impl Terminal for FakeTerm {
    fn stdin_is_tty(&self) -> bool {
        self.stdin_tty
    }

    fn stdout_is_tty(&self) -> bool {
        self.stdout_tty
    }

    fn var(&self, key: &str) -> Option<String> {
        self.marker
            .filter(|(k, _)| *k == key)
            .map(|(_, v)| v.to_string())
    }

    fn prompt(&mut self, _message: &str) -> io::Result<String> {
        self.prompted = true;
        Ok("bless\n".to_string())
    }
}

fn default_markers() -> Vec<String> {
    DEFAULT_AGENT_ENV_MARKERS
        .iter()
        .map(|s| (*s).to_string())
        .collect()
}

fn case_gate_refuses_no_tty() -> Result<(), String> {
    let mut term = FakeTerm {
        stdin_tty: false,
        stdout_tty: true,
        marker: None,
        prompted: false,
    };
    let markers = default_markers();
    let res = require_user(&mut term, &markers, "bless golden", "bless");
    match res {
        Err(denied) => {
            ensure(denied.reason == DenyReason::NotTty, || {
                format!("expected DenyReason::NotTty, got {:?}", denied.reason)
            })?;
            ensure(!term.prompted, || {
                "expected prompt never to be called".to_string()
            })
        }
        Ok(()) => Err("expected gate to deny non-tty stdin".to_string()),
    }
}

fn case_gate_refuses_agent_marker() -> Result<(), String> {
    let mut term = FakeTerm {
        stdin_tty: true,
        stdout_tty: true,
        marker: Some(("CLAUDECODE", "1")),
        prompted: false,
    };
    let markers = default_markers();
    let res = require_user(&mut term, &markers, "bless golden", "bless");
    match res {
        Err(denied) => {
            ensure(
                denied.reason == DenyReason::AgentMarker("CLAUDECODE".to_string()),
                || format!("expected AgentMarker(CLAUDECODE), got {:?}", denied.reason),
            )?;
            ensure(!term.prompted, || {
                "expected prompt never to be called".to_string()
            })
        }
        Ok(()) => Err("expected gate to deny CLAUDECODE=1".to_string()),
    }
}

fn case_hold_file_generation() -> Result<(), String> {
    let ledger = Ledger {
        failures: vec![Failure {
            component: "example-pkg".to_string(),
            version: "1.1-1".to_string(),
            phase: Phase::Build,
            class: COMPONENT_CLASS.to_string(),
            classifier_rule: "apt-broken".to_string(),
            log: "/tmp/neb-test/build.log".to_string(),
            recorded_at: "2026-11-10T12:00:00Z".to_string(),
            source: FailureSource::SingleChange,
            members: Vec::new(),
        }],
    };
    let golden = golden_with(&[("example-pkg", "1.0-1")]);
    let holds = ledger_holds(&ledger, &golden, &[]);
    ensure(holds.len() == 1, || {
        format!("expected 1 hold, got {}", holds.len())
    })?;
    ensure(
        holds[0].held_version == "1.0-1" && holds[0].blocked_version == "1.1-1",
        || format!("unexpected hold versions: {:?}", holds[0]),
    )?;

    let hf = hold_file(
        holds.clone(),
        "2026-11-10T12:00:00Z",
        "neb release self-test",
    );
    let rendered = render_hold_file(&hf).map_err(|e| e.to_string())?;
    let parsed: HoldFile = toml::from_str(&rendered).map_err(|e| e.to_string())?;
    ensure(parsed.holds == holds, || {
        format!("round-trip holds mismatch: {:?} != {holds:?}", parsed.holds)
    })?;

    let golden_newer = golden_with(&[("example-pkg", "1.2-1")]);
    let newer_holds = ledger_holds(&ledger, &golden_newer, &[]);
    ensure(newer_holds.is_empty(), || {
        format!("expected hold to clear at 1.2-1, got {newer_holds:?}")
    })
}

fn failing_candidate(changed_list: &[(&str, Option<&str>, &str)]) -> Candidate {
    let mut cand = candidate_with(changed_list);
    cand.phases.vm_live_boot = PhaseState::FailConfirmed;
    cand.history.push(PhaseRecord {
        phase: Phase::VmLiveBoot,
        result: PhaseResult::Fail,
        state: PhaseState::FailConfirmed,
        class: COMPONENT_CLASS.to_string(),
        classifier_rule: "systemd-unit-failed".to_string(),
        logs: vec!["/tmp/neb-test/round-0.log".to_string()],
        iso_sha256: "a".repeat(64),
        overlay_claim: "b".repeat(64),
        squashfs_sha256: "b".repeat(64),
        verdict: String::new(),
        flaked: false,
        recorded_at: "2026-11-10T12:00:00Z".to_string(),
    });
    cand
}

fn case_bisect_halving() -> Result<(), String> {
    let now = fixed_now()?;
    let golden = golden_with(&[
        ("a-pkg", "1.0-1"),
        ("b-pkg", "1.0-1"),
        ("c-pkg", "1.0-1"),
        ("d-pkg", "1.0-1"),
    ]);
    let cand = failing_candidate(&[
        ("a-pkg", Some("1.0-1"), "1.1-1"),
        ("b-pkg", Some("1.0-1"), "1.1-1"),
        ("c-pkg", Some("1.0-1"), "1.1-1"),
        ("d-pkg", Some("1.0-1"), "1.1-1"),
    ]);
    let ledger = Ledger::default();

    let (mut state, cap) = match start_bisect(Some(&cand), None, now) {
        StartOutcome::Started { state, cap, .. } => (*state, cap),
        other => return Err(format!("expected Started, got {other:?}")),
    };

    let max_iter = cap + 1;
    for _ in 0..max_iter {
        let plan =
            match next_round(&mut state, &golden, &ledger, &[], now).map_err(|e| e.to_string())? {
                NextOutcome::Round(plan) => plan,
                other => return Err(format!("expected NextOutcome::Round, got {other:?}")),
            };

        let has_culprit = plan.testing.iter().any(|c| c == "c-pkg");
        let input = if has_culprit {
            RoundInput {
                result: PhaseResult::Fail,
                class: Some(COMPONENT_CLASS.to_string()),
                classifier_rule: "c-pkg-regression".to_string(),
                log: "/tmp/neb-test/vm.log".to_string(),
                now,
            }
        } else {
            RoundInput {
                result: PhaseResult::Pass,
                class: None,
                classifier_rule: String::new(),
                log: "/tmp/neb-test/vm.log".to_string(),
                now,
            }
        };

        let outcome = record_round(&mut state, input).map_err(|e| e.to_string())?;
        if let RecordOutcome::Recorded {
            status: BisectStatus::Culprit,
            failures,
            ..
        } = outcome
        {
            ensure(state.rounds_completed() <= cap, || {
                format!(
                    "rounds_completed {} exceeded cap {cap}",
                    state.rounds_completed()
                )
            })?;
            ensure(
                state.culprits.len() == 1 && state.culprits[0].component == "c-pkg",
                || format!("expected culprit c-pkg, got {:?}", state.culprits),
            )?;
            ensure(
                failures.len() == 1
                    && failures[0].component == "c-pkg"
                    && failures[0].version == "1.1-1"
                    && failures[0].source == FailureSource::Bisect,
                || format!("unexpected culprit failures: {failures:?}"),
            )?;
            return Ok(());
        }
    }

    Err(format!(
        "bisect halving did not terminate within {max_iter} iterations"
    ))
}

fn case_bisect_interaction() -> Result<(), String> {
    let now = fixed_now()?;
    let golden = golden_with(&[("a-pkg", "1.0-1"), ("b-pkg", "1.0-1")]);
    let cand = failing_candidate(&[
        ("a-pkg", Some("1.0-1"), "1.1-1"),
        ("b-pkg", Some("1.0-1"), "1.1-1"),
    ]);
    let ledger = Ledger::default();

    let mut state = match start_bisect(Some(&cand), None, now) {
        StartOutcome::Started { state, .. } => *state,
        other => return Err(format!("expected Started, got {other:?}")),
    };

    // Round 1 passes.
    let _ = next_round(&mut state, &golden, &ledger, &[], now).map_err(|e| e.to_string())?;
    let _ = record_round(
        &mut state,
        RoundInput {
            result: PhaseResult::Pass,
            class: None,
            classifier_rule: String::new(),
            log: "/tmp/neb-test/r1.log".to_string(),
            now,
        },
    )
    .map_err(|e| e.to_string())?;

    // Round 2 passes -> Interaction.
    let _ = next_round(&mut state, &golden, &ledger, &[], now).map_err(|e| e.to_string())?;
    let outcome = record_round(
        &mut state,
        RoundInput {
            result: PhaseResult::Pass,
            class: None,
            classifier_rule: String::new(),
            log: "/tmp/neb-test/r2.log".to_string(),
            now,
        },
    )
    .map_err(|e| e.to_string())?;

    let expected_members = vec!["a-pkg".to_string(), "b-pkg".to_string()];
    match outcome {
        RecordOutcome::Recorded {
            status: BisectStatus::Interaction,
            failures,
            ..
        } => {
            ensure(state.interaction == expected_members, || {
                format!("unexpected interaction members: {:?}", state.interaction)
            })?;
            ensure(
                failures.len() == 2
                    && failures.iter().all(|f| {
                        f.members == expected_members && f.source == FailureSource::Interaction
                    }),
                || format!("unexpected interaction failures: {failures:?}"),
            )
        }
        other => Err(format!("expected Interaction, got {other:?}")),
    }
}

fn case_bisect_cap() -> Result<(), String> {
    let now = fixed_now()?;
    let golden = golden_with(&[("a-pkg", "1.0-1"), ("b-pkg", "1.0-1")]);
    let cand = failing_candidate(&[
        ("a-pkg", Some("1.0-1"), "1.1-1"),
        ("b-pkg", Some("1.0-1"), "1.1-1"),
    ]);
    let ledger = Ledger::default();

    let mut state = match start_bisect(Some(&cand), None, now) {
        StartOutcome::Started { state, .. } => *state,
        other => return Err(format!("expected Started, got {other:?}")),
    };
    state.cap = 1;

    let _ = next_round(&mut state, &golden, &ledger, &[], now).map_err(|e| e.to_string())?;
    let outcome = record_round(
        &mut state,
        RoundInput {
            result: PhaseResult::Pass,
            class: None,
            classifier_rule: String::new(),
            log: "/tmp/neb-test/cap.log".to_string(),
            now,
        },
    )
    .map_err(|e| e.to_string())?;

    ensure(
        matches!(
            outcome,
            RecordOutcome::Recorded {
                status: BisectStatus::CapReached,
                ref failures,
                ..
            } if failures.is_empty()
        ),
        || format!("expected CapReached with empty failures, got {outcome:?}"),
    )?;

    let next = next_round(&mut state, &golden, &ledger, &[], now).map_err(|e| e.to_string())?;
    ensure(next == NextOutcome::CapReached, || {
        format!("expected NextOutcome::CapReached, got {next:?}")
    })
}

fn case_promote_preconditions() -> Result<(), String> {
    let now = fixed_now()?;
    let cand = candidate_with(&[("example-pkg", Some("1.0-1"), "1.1-1")]);

    let extracted = ExtractedIso {
        inventory: IsoInventory {
            iso: IsoMeta {
                path: "/tmp/neb-test/candidate.iso".to_string(),
                size_bytes: 1024,
                vmlinuz_sha256: "a".repeat(64),
                initrd_sha256: "b".repeat(64),
                extracted_at: "2026-11-10T12:00:00Z".to_string(),
            },
            os_release: BTreeMap::from([("VERSION_ID".to_string(), "2026.11".to_string())]),
            packages: BTreeMap::from([(
                "example-pkg".to_string(),
                InstalledPackage {
                    name: "example-pkg".to_string(),
                    version: "1.1-1".to_string(),
                    source: "example-pkg".to_string(),
                    source_version: None,
                    arch: Some("amd64".to_string()),
                    depends: Vec::new(),
                },
            )]),
            custom_binaries: BTreeMap::new(),
            calamares_modules: CalamaresModules {
                configs: Vec::new(),
                custom_modules: Vec::new(),
                presence: BTreeMap::new(),
            },
            boot: BootInfo { files: Vec::new() },
        },
        identity: IsoIdentity {
            squashfs_sha256: "b".repeat(64),
            extra_live_layers: Vec::new(),
        },
    };

    let iso_sha = "a".repeat(64);
    let components = Components::default();
    let manifest = Manifest::default();
    let approvals = Approvals::default();
    let ledger = Ledger::default();

    let base = PromoteInput {
        candidate: Some(&cand),
        extracted: &extracted,
        iso_sha256: &iso_sha,
        components: &components,
        manifest: &manifest,
        golden: None,
        approvals: &approvals,
        ledger: &ledger,
        records: &[],
        active_bisect_candidate: None,
        bisect_rounds: 0,
        cache_deb_names: &[],
        now,
    };

    // 1. NoCandidate
    let no_cand_out = prepare_promote(&PromoteInput {
        candidate: None,
        ..base.clone()
    })
    .map_err(|e| e.to_string())?;
    ensure(
        matches!(
            no_cand_out,
            PreparedPromote::NotReady {
                reason: NotReadyReason::NoCandidate,
                ..
            }
        ),
        || format!("expected NotReady(NoCandidate), got {no_cand_out:?}"),
    )?;

    // 2. PhasesIncomplete
    let mut incomplete_cand = cand.clone();
    incomplete_cand.phases.vm_install_e2e = PhaseState::Pending;
    let incomplete_out = prepare_promote(&PromoteInput {
        candidate: Some(&incomplete_cand),
        ..base.clone()
    })
    .map_err(|e| e.to_string())?;
    ensure(
        matches!(
            incomplete_out,
            PreparedPromote::NotReady {
                reason: NotReadyReason::PhasesIncomplete,
                ..
            }
        ),
        || format!("expected NotReady(PhasesIncomplete), got {incomplete_out:?}"),
    )?;

    // 3. BisectActive (active bisect for another candidate)
    let bisect_out = prepare_promote(&PromoteInput {
        active_bisect_candidate: Some("bbbbbbbbbbbb-20261110T110000Z"),
        ..base.clone()
    })
    .map_err(|e| e.to_string())?;
    ensure(
        matches!(
            bisect_out,
            PreparedPromote::NotReady {
                reason: NotReadyReason::BisectActive,
                ..
            }
        ),
        || format!("expected NotReady(BisectActive), got {bisect_out:?}"),
    )?;

    // 4. IsoMismatch
    let wrong_sha = "b".repeat(64);
    let mismatch_out = prepare_promote(&PromoteInput {
        iso_sha256: &wrong_sha,
        ..base
    })
    .map_err(|e| e.to_string())?;
    ensure(
        matches!(
            mismatch_out,
            PreparedPromote::NotReady {
                reason: NotReadyReason::IsoMismatch,
                ..
            }
        ),
        || format!("expected NotReady(IsoMismatch), got {mismatch_out:?}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_self_test_passes_all_cases_in_declared_order() {
        let report = run_self_test();
        for failure in report.failed() {
            eprintln!("case {} failed: {}", failure.name, failure.detail);
        }
        assert!(report.all_passed());
        assert!(report.failed().is_empty());

        let names: Vec<&str> = report.cases.iter().map(|c| c.name).collect();
        assert_eq!(names.as_slice(), SELF_TEST_CASES);
    }

    #[test]
    fn run_cases_captures_errors_and_panics_without_stopping_early() {
        fn pass_case() -> Result<(), String> {
            Ok(())
        }
        fn err_case() -> Result<(), String> {
            Err("boom".to_string())
        }
        fn panic_case() -> Result<(), String> {
            panic!("kaboom");
        }

        let table: &[(&str, CaseFn)] = &[
            ("case-pass", pass_case),
            ("case-err", err_case),
            ("case-panic", panic_case),
        ];

        let report = run_cases(table);
        assert_eq!(report.cases.len(), 3);
        assert!(!report.all_passed());
        assert_eq!(report.failed().len(), 2);

        assert_eq!(
            report.cases[0],
            CaseResult {
                name: "case-pass",
                passed: true,
                detail: String::new(),
            }
        );
        assert_eq!(
            report.cases[1],
            CaseResult {
                name: "case-err",
                passed: false,
                detail: "boom".to_string(),
            }
        );
        assert_eq!(report.cases[2].name, "case-panic");
        assert!(!report.cases[2].passed);
        assert!(
            report.cases[2].detail.starts_with("panicked:"),
            "unexpected panic detail: {}",
            report.cases[2].detail
        );
    }
}
