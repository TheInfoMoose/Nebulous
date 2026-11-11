use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Result, TrackerError};
use crate::release::candidate::{Candidate, PhaseResult, PhaseState};
use crate::release::golden::Golden;
use crate::release::holds::{
    all_last_good, hold_file, ledger_holds, render_hold_file, Hold, HoldReason,
};
use crate::release::ledger::{Failure, FailureSource, Ledger, Phase, COMPONENT_CLASS};
use crate::release::records::ReleaseRecord;
use crate::release::ExitCode;

pub const BISECT_SCHEMA_VERSION: u32 = 1;
pub const BISECT_GENERATED_BY: &str = "neb release bisect";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BisectStatus {
    Active,
    Culprit,
    Interaction,
    CapReached,
}

impl BisectStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Culprit => "culprit",
            Self::Interaction => "interaction",
            Self::CapReached => "cap_reached",
        }
    }

    pub fn status_code(self) -> &'static str {
        match self {
            Self::Active => "BISECT_ROUND_RECORDED",
            Self::Culprit => "BISECT_CULPRIT",
            Self::Interaction => "BISECT_INTERACTION",
            Self::CapReached => "BISECT_CAP_REACHED",
        }
    }

    pub fn exit_code(self) -> ExitCode {
        match self {
            Self::Active | Self::Culprit => ExitCode::Ok,
            Self::Interaction | Self::CapReached => ExitCode::BisectStop,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoundResult {
    Pending,
    Pass,
    Fail,
}

impl RoundResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Pass => "pass",
            Self::Fail => "fail",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Round {
    pub round: u32,
    pub started_at: String,
    pub result: RoundResult,
    pub hold_file_sha256: String,
    #[serde(default)]
    pub classifier_rule: String,
    #[serde(default)]
    pub log: String,
    #[serde(default)]
    pub recorded_at: String,
    pub testing: Vec<String>,
    pub held: Vec<String>,
}

/// A suspect component set in the bisection queue.
///
/// `a_round` is the round that tested the first half and passed; `None` means the first half is next.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Suspect {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a_round: Option<u32>,
    pub classifier_rule: String,
    pub log: String,
    pub members: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Culprit {
    pub component: String,
    pub version: String,
    pub round: u32,
}

/// Persistent bisection state stored at `.tracker/release/bisect.toml`.
///
/// Scalar and plain-array fields precede table and array-of-table fields for TOML serialization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BisectState {
    pub schema_version: u32,
    pub candidate_id: String,
    pub failing_phase: Phase,
    pub cap: u32,
    pub round: u32,
    pub status: BisectStatus,
    pub started_at: String,
    pub always_applied: Vec<String>,
    #[serde(default)]
    pub interaction: Vec<String>,
    #[serde(default)]
    pub culprits: Vec<Culprit>,
    #[serde(default)]
    pub queue: Vec<Suspect>,
    #[serde(default)]
    pub rounds: Vec<Round>,
    pub versions: BTreeMap<String, String>,
}

impl BisectState {
    pub fn is_active(&self) -> bool {
        self.status == BisectStatus::Active
    }

    pub fn rounds_completed(&self) -> u32 {
        let count = self
            .rounds
            .iter()
            .filter(|r| r.result != RoundResult::Pending)
            .count();
        u32::try_from(count).unwrap_or(u32::MAX)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BisectNotReady {
    NoCandidate,
    NoConfirmedFailure,
    TooFewComponents,
    NotActive,
    NoRoundInProgress,
}

impl BisectNotReady {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoCandidate => "no_candidate",
            Self::NoConfirmedFailure => "no_confirmed_failure",
            Self::TooFewComponents => "too_few_components",
            Self::NotActive => "not_active",
            Self::NoRoundInProgress => "no_round_in_progress",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartOutcome {
    Started {
        state: Box<BisectState>,
        n: usize,
        cap: u32,
    },
    NotReady {
        reason: BisectNotReady,
        detail: String,
    },
    Conflict {
        detail: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundPlan {
    pub round: u32,
    pub testing: Vec<String>,
    pub held: Vec<String>,
    pub hold_file_text: String,
    pub hold_file_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextOutcome {
    Round(RoundPlan),
    NotReady {
        reason: BisectNotReady,
        detail: String,
    },
    CapReached,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoundInput {
    pub result: PhaseResult,
    pub class: Option<String>,
    pub classifier_rule: String,
    pub log: String,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordOutcome {
    Recorded {
        status: BisectStatus,
        next: &'static str,
        failures: Vec<Failure>,
    },
    RejectedClass {
        class: String,
    },
    NotReady {
        reason: BisectNotReady,
        detail: String,
    },
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

/// Computes the maximum number of bisect rounds: `2 * ceil(log2(n)) + 2` (or `2` when `n < 2`).
pub fn bisect_cap(n: usize) -> u32 {
    if n < 2 {
        return 2;
    }
    let ceil_log2 = (n - 1).ilog2() + 1;
    2 * ceil_log2 + 2
}

/// Splits `members` into `A` (first `ceil(len / 2)` elements) and `B` (the rest).
pub fn split(members: &[String]) -> (Vec<String>, Vec<String>) {
    let mid = members.len().div_ceil(2);
    (members[..mid].to_vec(), members[mid..].to_vec())
}

/// Returns the `candidate_id` of an active bisect, or `None` if absent or finished.
pub fn active_bisect_candidate(state: Option<&BisectState>) -> Option<&str> {
    state
        .filter(|s| s.is_active())
        .map(|s| s.candidate_id.as_str())
}

/// Purely validates `candidate` and `existing` to initialize a new [`BisectState`].
pub fn start_bisect(
    candidate: Option<&Candidate>,
    existing: Option<&BisectState>,
    now: DateTime<Utc>,
) -> StartOutcome {
    let Some(c) = candidate else {
        return StartOutcome::NotReady {
            reason: BisectNotReady::NoCandidate,
            detail: "no active candidate (.tracker/release/candidate.toml does not exist)"
                .to_string(),
        };
    };

    if let Some(ex) = existing.filter(|s| s.is_active()) {
        return StartOutcome::Conflict {
            detail: format!(
                "a bisect is already active for candidate '{}' at round {}; finish it or run neb release bisect abort",
                ex.candidate_id, ex.round
            ),
        };
    }

    let Some(failing_phase) = Phase::ALL
        .into_iter()
        .find(|&p| c.phases.get(p) == PhaseState::FailConfirmed)
    else {
        return StartOutcome::NotReady {
            reason: BisectNotReady::NoConfirmedFailure,
            detail: format!(
                "no confirmed phase failure on candidate '{}': build={}, vm-live-boot={}, vm-install-e2e={}",
                c.candidate_id,
                c.phases.build.as_str(),
                c.phases.vm_live_boot.as_str(),
                c.phases.vm_install_e2e.as_str()
            ),
        };
    };

    let (classifier_rule, log) = c
        .history
        .iter()
        .rev()
        .find(|r| r.phase == failing_phase && r.state == PhaseState::FailConfirmed)
        .map(|r| {
            (
                r.classifier_rule.clone(),
                r.logs.first().cloned().unwrap_or_default(),
            )
        })
        .unwrap_or_default();

    let mut versions = BTreeMap::new();
    let mut always_applied = Vec::new();
    for ch in &c.changed {
        if ch.from_version.is_some() {
            versions.insert(ch.component.clone(), ch.to_version.clone());
        } else {
            always_applied.push(ch.component.clone());
        }
    }
    always_applied.sort();
    always_applied.dedup();

    let bisectable: Vec<String> = versions.keys().cloned().collect();
    let n = bisectable.len();
    if n <= 1 {
        let mut detail = format!("{n} bisectable changed component(s); bisect needs at least 2");
        if !always_applied.is_empty() {
            let _ = write!(
                detail,
                "; new components cannot be held at golden: {}",
                always_applied.join(", ")
            );
        }
        return StartOutcome::NotReady {
            reason: BisectNotReady::TooFewComponents,
            detail,
        };
    }

    let cap = bisect_cap(n);
    let state = BisectState {
        schema_version: BISECT_SCHEMA_VERSION,
        candidate_id: c.candidate_id.clone(),
        failing_phase,
        cap,
        round: 0,
        status: BisectStatus::Active,
        started_at: now.to_rfc3339_opts(SecondsFormat::Secs, true),
        always_applied,
        interaction: Vec::new(),
        culprits: Vec::new(),
        queue: vec![Suspect {
            a_round: None,
            classifier_rule,
            log,
            members: bisectable,
        }],
        rounds: Vec::new(),
        versions,
    };

    StartOutcome::Started {
        state: Box::new(state),
        n,
        cap,
    }
}

/// Plans (or re-plans in place when the last round is still `Pending`) the next bisection round
/// and renders its hold file.
pub fn next_round(
    state: &mut BisectState,
    golden: &Golden,
    ledger: &Ledger,
    records: &[(PathBuf, ReleaseRecord)],
    now: DateTime<Utc>,
) -> Result<NextOutcome> {
    if state.status == BisectStatus::CapReached {
        return Ok(NextOutcome::CapReached);
    }
    if !state.is_active() {
        return Ok(NextOutcome::NotReady {
            reason: BisectNotReady::NotActive,
            detail: format!(
                "bisect is not active (status is '{}')",
                state.status.as_str()
            ),
        });
    }
    if state.rounds_completed() >= state.cap {
        state.status = BisectStatus::CapReached;
        return Ok(NextOutcome::CapReached);
    }

    let Some(suspect) = state.queue.first() else {
        return Err(TrackerError::ValidationError(
            "bisect: active bisect has an empty suspect queue".to_string(),
        ));
    };

    let (a, b) = split(&suspect.members);
    let testing = if suspect.a_round.is_none() { a } else { b };
    let testing_set: BTreeSet<&str> = testing.iter().map(String::as_str).collect();
    let held: Vec<String> = state
        .versions
        .keys()
        .filter(|id| !testing_set.contains(id.as_str()))
        .cloned()
        .collect();
    let held_set: BTreeSet<&str> = held.iter().map(String::as_str).collect();

    for id in &held {
        if !golden.components.contains_key(id) {
            return Err(TrackerError::ValidationError(format!(
                "bisect: held component '{id}' is not in golden.toml"
            )));
        }
    }

    let (round_num, round_started_at, replan_in_place) = match state.rounds.last() {
        Some(last) if last.result == RoundResult::Pending => {
            (last.round, last.started_at.clone(), true)
        }
        _ => (
            u32::try_from(state.rounds.len()).unwrap_or(0) + 1,
            now.to_rfc3339_opts(SecondsFormat::Secs, true),
            false,
        ),
    };

    let mut holds: Vec<Hold> = ledger_holds(ledger, golden, records)
        .into_iter()
        .filter(|h| !held_set.contains(h.component.as_str()))
        .collect();

    for mut h in all_last_good(golden, records) {
        if held_set.contains(h.component.as_str()) {
            h.reason = HoldReason::Bisect;
            holds.push(h);
        }
    }
    holds.sort_by(|x, y| x.component.cmp(&y.component));

    let hf = hold_file(holds, &round_started_at, BISECT_GENERATED_BY);
    let hold_file_text = render_hold_file(&hf)?;
    let hold_file_sha256 = sha256_hex(hold_file_text.as_bytes());

    if replan_in_place {
        if let Some(last) = state.rounds.last_mut() {
            last.testing = testing.clone();
            last.held = held.clone();
            last.hold_file_sha256 = hold_file_sha256.clone();
        }
    } else {
        state.rounds.push(Round {
            round: round_num,
            started_at: round_started_at,
            result: RoundResult::Pending,
            hold_file_sha256: hold_file_sha256.clone(),
            classifier_rule: String::new(),
            log: String::new(),
            recorded_at: String::new(),
            testing: testing.clone(),
            held: held.clone(),
        });
    }
    state.round = round_num;

    Ok(NextOutcome::Round(RoundPlan {
        round: round_num,
        testing,
        held,
        hold_file_text,
        hold_file_sha256,
    }))
}

fn narrow_suspect(
    state: &mut BisectState,
    half: Vec<String>,
    round_num: u32,
    classifier_rule: String,
    log: String,
    recorded_at: String,
) -> Vec<Failure> {
    if half.len() == 1 {
        let component = half.into_iter().next().unwrap_or_default();
        let version = state.versions.get(&component).cloned().unwrap_or_default();
        state.culprits.push(Culprit {
            component: component.clone(),
            version: version.clone(),
            round: round_num,
        });
        let failure = Failure {
            component,
            version,
            phase: state.failing_phase,
            class: COMPONENT_CLASS.to_string(),
            classifier_rule,
            log,
            recorded_at,
            source: FailureSource::Bisect,
            members: Vec::new(),
        };
        state.queue.remove(0);
        if state.queue.is_empty() {
            state.status = BisectStatus::Culprit;
        }
        vec![failure]
    } else {
        state.queue[0] = Suspect {
            a_round: None,
            classifier_rule,
            log,
            members: half,
        };
        Vec::new()
    }
}

/// Records the outcome of the in-progress bisect round and advances the halving state machine.
pub fn record_round(state: &mut BisectState, input: RoundInput) -> Result<RecordOutcome> {
    if !state.is_active() {
        return Ok(RecordOutcome::NotReady {
            reason: BisectNotReady::NotActive,
            detail: format!(
                "bisect is not active (status is '{}')",
                state.status.as_str()
            ),
        });
    }

    let is_pending = state
        .rounds
        .last()
        .is_some_and(|r| r.result == RoundResult::Pending);
    if !is_pending {
        return Ok(RecordOutcome::NotReady {
            reason: BisectNotReady::NoRoundInProgress,
            detail: "no bisect round in progress; run neb release bisect next".to_string(),
        });
    }

    if input.result == PhaseResult::Fail {
        let cls = input.class.as_deref().unwrap_or("");
        if cls != COMPONENT_CLASS {
            return Ok(RecordOutcome::RejectedClass {
                class: cls.to_string(),
            });
        }
        if input.classifier_rule.trim().is_empty() {
            return Err(TrackerError::ValidationError(
                "bisect record fail requires a non-blank classifier_rule".to_string(),
            ));
        }
    }

    if state.queue.is_empty() {
        return Err(TrackerError::ValidationError(
            "bisect: active bisect has an empty suspect queue".to_string(),
        ));
    }

    let recorded_at = input.now.to_rfc3339_opts(SecondsFormat::Secs, true);
    let round_num = {
        let last = state
            .rounds
            .last_mut()
            .expect("verified pending round exists");
        last.result = match input.result {
            PhaseResult::Pass => RoundResult::Pass,
            PhaseResult::Fail => RoundResult::Fail,
        };
        last.classifier_rule = input.classifier_rule.clone();
        last.log = input.log.clone();
        last.recorded_at = recorded_at.clone();
        last.round
    };

    let s = state.queue[0].clone();
    let (a, b) = split(&s.members);

    let failures = match (s.a_round, input.result) {
        (None, PhaseResult::Fail) => narrow_suspect(
            state,
            a,
            round_num,
            input.classifier_rule,
            input.log,
            recorded_at,
        ),
        (None, PhaseResult::Pass) => {
            state.queue[0].a_round = Some(round_num);
            Vec::new()
        }
        (Some(_), PhaseResult::Fail) => narrow_suspect(
            state,
            b,
            round_num,
            input.classifier_rule,
            input.log,
            recorded_at,
        ),
        (Some(_), PhaseResult::Pass) => {
            state.interaction = s.members.clone();
            let interaction_failures: Vec<Failure> = s
                .members
                .iter()
                .map(|member| Failure {
                    component: member.clone(),
                    version: state.versions.get(member).cloned().unwrap_or_default(),
                    phase: state.failing_phase,
                    class: COMPONENT_CLASS.to_string(),
                    classifier_rule: s.classifier_rule.clone(),
                    log: s.log.clone(),
                    recorded_at: recorded_at.clone(),
                    source: FailureSource::Interaction,
                    members: s.members.clone(),
                })
                .collect();
            state.queue.clear();
            state.status = BisectStatus::Interaction;
            interaction_failures
        }
    };

    if state.status == BisectStatus::Active && state.rounds_completed() >= state.cap {
        state.status = BisectStatus::CapReached;
    }

    let next = match state.status {
        BisectStatus::Active => "next",
        BisectStatus::Culprit => "rerun-without-culprit",
        BisectStatus::Interaction | BisectStatus::CapReached => "stop",
    };

    Ok(RecordOutcome::Recorded {
        status: state.status,
        next,
        failures,
    })
}

pub fn read_bisect(path: &Path) -> Result<Option<BisectState>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(TrackerError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    let table: toml::Table =
        toml::from_str(&text).map_err(|source| TrackerError::TomlDeserialize {
            path: Some(path.to_path_buf()),
            source,
        })?;

    match table.get("schema_version") {
        Some(toml::Value::Integer(v)) if *v >= 1 => {
            let found = u32::try_from(*v).unwrap_or(u32::MAX);
            if found > BISECT_SCHEMA_VERSION {
                return Err(TrackerError::UnsupportedSchema {
                    path: path.to_path_buf(),
                    found,
                    supported: BISECT_SCHEMA_VERSION,
                });
            }
        }
        Some(_) => {
            return Err(TrackerError::ValidationError(format!(
                "{}: schema_version must be a positive integer",
                path.display()
            )));
        }
        None => {}
    }

    let state = BisectState::deserialize(toml::Value::Table(table)).map_err(|source| {
        TrackerError::TomlDeserialize {
            path: Some(path.to_path_buf()),
            source,
        }
    })?;

    Ok(Some(state))
}

pub fn render_bisect(state: &BisectState) -> Result<String> {
    let body = toml::to_string(state)?;
    Ok(format!(
        "# NovOS bisect state (.tracker/release/bisect.toml)\n\
         # Written by neb release bisect.\n\n\
         {body}"
    ))
}

/// Renders and atomically writes `state` to `path`.
///
/// The caller must hold `TrackerStorage::with_write_lock` and write any ledger entries returned
/// by [`record_round`] before calling this function.
pub fn write_bisect(path: &Path, state: &BisectState) -> Result<()> {
    let text = render_bisect(state)?;

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|source| TrackerError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }

    crate::storage::atomic_write_str(path, &text)
}

/// Removes `bisect.toml` if present, returning `true` when removed and `false` when absent.
///
/// The user-only confirmation gate (§5.1) for `neb release bisect abort` is enforced by the CLI handler.
pub fn remove_bisect(path: &Path) -> Result<bool> {
    match fs::remove_file(path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(false),
        Err(source) => Err(TrackerError::Io {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tempfile::tempdir;

    use super::*;
    use crate::release::candidate::{
        ChangedComponent, PhaseRecord, Phases, CANDIDATE_SCHEMA_VERSION,
    };
    use crate::release::golden::{GoldenComponent, Origin, GOLDEN_SCHEMA_VERSION};
    use crate::release::holds::HoldFile;
    use crate::release::inventory::{BootInfo, CalamaresModules, IsoMeta};

    fn sample_golden() -> Golden {
        let mut components = BTreeMap::new();
        for (id, ver) in [
            ("a-pkg", "1.0-1"),
            ("b-pkg", "1.0-1"),
            ("ppp", "2.5.2-1+2"),
            ("z-pkg", "1.0-1"),
            ("other-pkg", "3.0-1"),
        ] {
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
                initrd_sha256: "a".repeat(64),
                extracted_at: "2026-11-08T12:00:00Z".to_string(),
            },
            os_release: BTreeMap::new(),
            boot: BootInfo { files: Vec::new() },
            calamares_modules: CalamaresModules {
                configs: Vec::new(),
                custom_modules: Vec::new(),
                presence: BTreeMap::new(),
            },
            custom_binaries: BTreeMap::new(),
            packages: BTreeMap::new(),
            components,
        }
    }

    fn changed_comp(id: &str, from: Option<&str>, to: &str) -> ChangedComponent {
        ChangedComponent {
            component: id.to_string(),
            origin: Origin::Debian,
            from_version: from.map(ToOwned::to_owned),
            to_version: to.to_string(),
            kernel_line: false,
        }
    }

    fn sample_candidate() -> Candidate {
        Candidate {
            schema_version: CANDIDATE_SCHEMA_VERSION,
            candidate_id: "aaaaaaaaaaaa-20261110T120000Z".to_string(),
            created_at: "2026-11-10T12:00:00Z".to_string(),
            iso_path: "/tmp/neb-test/candidate.iso".to_string(),
            iso_sha256: "a".repeat(64),
            squashfs_sha256: "a".repeat(64),
            snapshot: "20261107T205437Z".to_string(),
            artifact_cache: String::new(),
            hold_file: String::new(),
            hold_file_sha256: String::new(),
            repo_head: "a".repeat(40),
            unchanged_count: 10,
            phases: Phases {
                build: PhaseState::Pass,
                vm_live_boot: PhaseState::FailConfirmed,
                vm_install_e2e: PhaseState::Pending,
            },
            // Intentionally out of alphabetical order + includes a new component (from_version: None).
            changed: vec![
                changed_comp("z-pkg", Some("1.0-1"), "1.1-1"),
                changed_comp("new-pkg", None, "0.1-1"),
                changed_comp("ppp", Some("2.5.2-1+2"), "2.5.5-1+1"),
                changed_comp("a-pkg", Some("1.0-1"), "1.1-1"),
                changed_comp("b-pkg", Some("1.0-1"), "1.1-1"),
            ],
            holds_in_force: Vec::new(),
            history: vec![
                PhaseRecord {
                    phase: Phase::VmLiveBoot,
                    result: PhaseResult::Fail,
                    state: PhaseState::FailUnconfirmed,
                    class: COMPONENT_CLASS.to_string(),
                    classifier_rule: "early-rule".to_string(),
                    logs: vec!["logs/round-0/try-1.log".to_string()],
                    iso_sha256: "a".repeat(64),
                    overlay_claim: "a".repeat(64),
                    squashfs_sha256: "a".repeat(64),
                    verdict: String::new(),
                    flaked: false,
                    recorded_at: "2026-11-10T12:05:00Z".to_string(),
                },
                PhaseRecord {
                    phase: Phase::VmLiveBoot,
                    result: PhaseResult::Fail,
                    state: PhaseState::FailConfirmed,
                    class: COMPONENT_CLASS.to_string(),
                    classifier_rule: "systemd-unit-failed".to_string(),
                    logs: vec![
                        "logs/round-0/vm-live-boot.log".to_string(),
                        "logs/round-0/extra.log".to_string(),
                    ],
                    iso_sha256: "a".repeat(64),
                    overlay_claim: "a".repeat(64),
                    squashfs_sha256: "a".repeat(64),
                    verdict: String::new(),
                    flaked: false,
                    recorded_at: "2026-11-10T12:10:00Z".to_string(),
                },
            ],
        }
    }

    #[test]
    fn bisect_cap_and_split_tables() {
        assert_eq!(bisect_cap(0), 2);
        assert_eq!(bisect_cap(1), 2);
        assert_eq!(bisect_cap(2), 4);
        assert_eq!(bisect_cap(3), 6);
        assert_eq!(bisect_cap(4), 6);
        assert_eq!(bisect_cap(5), 8);
        assert_eq!(bisect_cap(8), 8);
        assert_eq!(bisect_cap(9), 10);

        let three = vec!["a-pkg".to_string(), "b-pkg".to_string(), "ppp".to_string()];
        assert_eq!(
            split(&three),
            (
                vec!["a-pkg".to_string(), "b-pkg".to_string()],
                vec!["ppp".to_string()]
            )
        );

        let four = vec![
            "a-pkg".to_string(),
            "b-pkg".to_string(),
            "ppp".to_string(),
            "z-pkg".to_string(),
        ];
        assert_eq!(
            split(&four),
            (
                vec!["a-pkg".to_string(), "b-pkg".to_string()],
                vec!["ppp".to_string(), "z-pkg".to_string()]
            )
        );
    }

    #[test]
    fn start_bisect_preconditions_and_happy_path() {
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 12, 15, 0).unwrap();

        // No candidate -> NoCandidate.
        match start_bisect(None, None, now) {
            StartOutcome::NotReady { reason, .. } => {
                assert_eq!(reason, BisectNotReady::NoCandidate);
                assert_eq!(reason.as_str(), "no_candidate");
            }
            other => panic!("expected NoCandidate, got {other:?}"),
        }

        let cand = sample_candidate();
        let StartedState { mut state } = {
            match start_bisect(Some(&cand), None, now) {
                StartOutcome::Started { state, n, cap } => {
                    assert_eq!(n, 4);
                    assert_eq!(cap, 6);
                    StartedState { state: *state }
                }
                other => panic!("expected Started, got {other:?}"),
            }
        };

        assert_eq!(state.schema_version, BISECT_SCHEMA_VERSION);
        assert_eq!(state.candidate_id, cand.candidate_id);
        assert_eq!(state.failing_phase, Phase::VmLiveBoot);
        assert_eq!(state.cap, 6);
        assert_eq!(state.round, 0);
        assert_eq!(state.status, BisectStatus::Active);
        assert_eq!(state.always_applied, vec!["new-pkg".to_string()]);
        assert_eq!(state.queue.len(), 1);
        assert_eq!(
            state.queue[0],
            Suspect {
                a_round: None,
                classifier_rule: "systemd-unit-failed".to_string(),
                log: "logs/round-0/vm-live-boot.log".to_string(),
                members: vec![
                    "a-pkg".to_string(),
                    "b-pkg".to_string(),
                    "ppp".to_string(),
                    "z-pkg".to_string(),
                ],
            }
        );
        assert_eq!(
            state.versions.get("ppp").map(String::as_str),
            Some("2.5.5-1+1")
        );
        assert!(!state.versions.contains_key("new-pkg"));

        // Active existing state -> Conflict naming candidate_id and round.
        state.round = 2;
        match start_bisect(Some(&cand), Some(&state), now) {
            StartOutcome::Conflict { detail } => {
                assert!(detail.contains(&cand.candidate_id));
                assert!(detail.contains("round 2"));
            }
            other => panic!("expected Conflict, got {other:?}"),
        }

        // CapReached existing state -> replaced (Started).
        state.status = BisectStatus::CapReached;
        match start_bisect(Some(&cand), Some(&state), now) {
            StartOutcome::Started { state: fresh, .. } => {
                assert_eq!(fresh.status, BisectStatus::Active);
                assert_eq!(fresh.round, 0);
            }
            other => panic!("expected Started replacing CapReached, got {other:?}"),
        }

        // No FailConfirmed phase -> NoConfirmedFailure naming every phase's state.
        let mut unconfirmed = cand.clone();
        unconfirmed.phases.vm_live_boot = PhaseState::FailUnconfirmed;
        match start_bisect(Some(&unconfirmed), None, now) {
            StartOutcome::NotReady { reason, detail } => {
                assert_eq!(reason, BisectNotReady::NoConfirmedFailure);
                assert_eq!(reason.as_str(), "no_confirmed_failure");
                assert!(detail.contains("pass"));
                assert!(detail.contains("fail_unconfirmed"));
                assert!(detail.contains("pending"));
            }
            other => panic!("expected NoConfirmedFailure, got {other:?}"),
        }

        // One bisectable + new-pkg -> TooFewComponents naming new-pkg.
        let mut one_bisectable = cand;
        one_bisectable.changed = vec![
            changed_comp("a-pkg", Some("1.0-1"), "1.1-1"),
            changed_comp("new-pkg", None, "0.1-1"),
        ];
        match start_bisect(Some(&one_bisectable), None, now) {
            StartOutcome::NotReady { reason, detail } => {
                assert_eq!(reason, BisectNotReady::TooFewComponents);
                assert_eq!(reason.as_str(), "too_few_components");
                assert!(detail.contains("1 bisectable"));
                assert!(detail.contains("new-pkg"));
            }
            other => panic!("expected TooFewComponents, got {other:?}"),
        }
    }

    struct StartedState {
        state: BisectState,
    }

    #[test]
    fn next_round_holds_replanning_and_missing_golden_validation() {
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 12, 15, 0).unwrap();
        let cand = sample_candidate();
        let StartOutcome::Started { mut state, .. } = start_bisect(Some(&cand), None, now) else {
            panic!("expected Started");
        };

        let golden = sample_golden();
        // Ledger has a hold on `other-pkg` (unrelated to bisect) and a hold on `ppp` (in `held`).
        let ledger = Ledger {
            failures: vec![
                Failure {
                    component: "other-pkg".to_string(),
                    version: "3.1-1".to_string(),
                    phase: Phase::Build,
                    class: COMPONENT_CLASS.to_string(),
                    classifier_rule: "apt-breaks".to_string(),
                    log: "logs/old.log".to_string(),
                    recorded_at: "2026-11-09T12:00:00Z".to_string(),
                    source: FailureSource::SingleChange,
                    members: Vec::new(),
                },
                Failure {
                    component: "ppp".to_string(),
                    version: "2.5.4-1".to_string(),
                    phase: Phase::Build,
                    class: COMPONENT_CLASS.to_string(),
                    classifier_rule: "apt-breaks".to_string(),
                    log: "logs/old-ppp.log".to_string(),
                    recorded_at: "2026-11-09T12:00:00Z".to_string(),
                    source: FailureSource::Bisect,
                    members: Vec::new(),
                },
            ],
        };

        let round1_time = Utc.with_ymd_and_hms(2026, 11, 10, 12, 20, 0).unwrap();
        let NextOutcome::Round(plan1) =
            next_round(&mut state, &golden, &ledger, &[], round1_time).expect("next_round 1")
        else {
            panic!("expected Round");
        };

        assert_eq!(plan1.round, 1);
        assert_eq!(
            plan1.testing,
            vec!["a-pkg".to_string(), "b-pkg".to_string()]
        );
        assert_eq!(plan1.held, vec!["ppp".to_string(), "z-pkg".to_string()]);
        assert_eq!(state.round, 1);
        assert_eq!(state.rounds.len(), 1);
        assert_eq!(state.rounds[0].result, RoundResult::Pending);

        let parsed_hf: HoldFile =
            toml::from_str(&plan1.hold_file_text).expect("parse rendered hold file");
        assert_eq!(parsed_hf.generated_by, BISECT_GENERATED_BY);
        assert_eq!(parsed_hf.holds.len(), 3);
        // Sorted by component: other-pkg (failure), ppp (bisect), z-pkg (bisect).
        assert_eq!(parsed_hf.holds[0].component, "other-pkg");
        assert_eq!(parsed_hf.holds[0].reason, HoldReason::Failure);
        assert_eq!(parsed_hf.holds[0].blocked_version, "3.1-1");

        assert_eq!(parsed_hf.holds[1].component, "ppp");
        assert_eq!(parsed_hf.holds[1].reason, HoldReason::Bisect);
        assert_eq!(parsed_hf.holds[1].held_version, "2.5.2-1+2");
        assert_eq!(parsed_hf.holds[1].blocked_version, "");

        assert_eq!(parsed_hf.holds[2].component, "z-pkg");
        assert_eq!(parsed_hf.holds[2].reason, HoldReason::Bisect);
        assert_eq!(parsed_hf.holds[2].held_version, "1.0-1");

        // Second call while pending re-plans round 1 in place with the same started_at and sha.
        let later_time = Utc.with_ymd_and_hms(2026, 11, 10, 12, 25, 0).unwrap();
        let NextOutcome::Round(plan1_again) =
            next_round(&mut state, &golden, &ledger, &[], later_time).expect("re-plan round 1")
        else {
            panic!("expected Round on re-plan");
        };
        assert_eq!(plan1_again, plan1);
        assert_eq!(state.rounds.len(), 1);

        // Held component missing from golden -> ValidationError.
        let mut incomplete_golden = golden.clone();
        incomplete_golden.components.remove("ppp");
        let err = next_round(&mut state, &incomplete_golden, &ledger, &[], later_time)
            .expect_err("missing golden component must error");
        match err {
            TrackerError::ValidationError(msg) => assert!(msg.contains("ppp")),
            other => panic!("expected ValidationError, got {other:?}"),
        }

        // Finished state -> NotActive.
        state.status = BisectStatus::Culprit;
        match next_round(&mut state, &golden, &ledger, &[], later_time).expect("not active") {
            NextOutcome::NotReady { reason, .. } => {
                assert_eq!(reason, BisectNotReady::NotActive);
                assert_eq!(reason.as_str(), "not_active");
            }
            other => panic!("expected NotActive, got {other:?}"),
        }
    }

    #[test]
    fn record_round_preconditions_and_ppp_culprit_first_and_second_half() {
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 12, 15, 0).unwrap();
        let cand = sample_candidate();
        let golden = sample_golden();
        let ledger = Ledger::default();

        let StartOutcome::Started { mut state, .. } = start_bisect(Some(&cand), None, now) else {
            panic!("expected Started");
        };

        // No round in progress -> NoRoundInProgress.
        match record_round(
            &mut state,
            RoundInput {
                result: PhaseResult::Pass,
                class: None,
                classifier_rule: String::new(),
                log: String::new(),
                now,
            },
        )
        .expect("no round")
        {
            RecordOutcome::NotReady { reason, .. } => {
                assert_eq!(reason, BisectNotReady::NoRoundInProgress);
                assert_eq!(reason.as_str(), "no_round_in_progress");
            }
            other => panic!("expected NoRoundInProgress, got {other:?}"),
        }

        // Plan round 1: tests [a-pkg, b-pkg].
        next_round(&mut state, &golden, &ledger, &[], now).expect("round 1");

        // Fail with class `infra` -> RejectedClass and round 1 stays Pending.
        let rejected = record_round(
            &mut state,
            RoundInput {
                result: PhaseResult::Fail,
                class: Some("infra".to_string()),
                classifier_rule: "apt-dns".to_string(),
                log: "logs/round-1/build.log".to_string(),
                now,
            },
        )
        .expect("rejected class");
        assert_eq!(
            rejected,
            RecordOutcome::RejectedClass {
                class: "infra".to_string()
            }
        );
        assert_eq!(state.rounds[0].result, RoundResult::Pending);
        assert_eq!(RoundResult::Pending.as_str(), "pending");
        assert_eq!(RoundResult::Pass.as_str(), "pass");
        assert_eq!(RoundResult::Fail.as_str(), "fail");
        assert_eq!(BisectStatus::Active.as_str(), "active");
        assert_eq!(BisectStatus::Culprit.as_str(), "culprit");
        assert_eq!(BisectStatus::Interaction.as_str(), "interaction");
        assert_eq!(BisectStatus::CapReached.as_str(), "cap_reached");

        // Fail with class `None` -> RejectedClass { class: "" } and round 1 stays Pending.
        let rejected_none = record_round(
            &mut state,
            RoundInput {
                result: PhaseResult::Fail,
                class: None,
                classifier_rule: "apt-dns".to_string(),
                log: "logs/round-1/build.log".to_string(),
                now,
            },
        )
        .expect("rejected None class");
        assert_eq!(
            rejected_none,
            RecordOutcome::RejectedClass {
                class: String::new()
            }
        );
        assert_eq!(state.rounds[0].result, RoundResult::Pending);

        // Blank classifier_rule on a component fail -> ValidationError, round 1 stays Pending.
        let blank_err = record_round(
            &mut state,
            RoundInput {
                result: PhaseResult::Fail,
                class: Some(COMPONENT_CLASS.to_string()),
                classifier_rule: "   ".to_string(),
                log: "logs/round-1/build.log".to_string(),
                now,
            },
        )
        .expect_err("blank rule must fail");
        assert!(matches!(blank_err, TrackerError::ValidationError(_)));
        assert_eq!(state.rounds[0].result, RoundResult::Pending);

        // Round 1 [a-pkg, b-pkg] pass -> Active, next "next".
        let r1_out = record_round(
            &mut state,
            RoundInput {
                result: PhaseResult::Pass,
                class: None,
                classifier_rule: String::new(),
                log: "logs/round-1/vm.log".to_string(),
                now,
            },
        )
        .expect("record round 1 pass");
        assert_eq!(
            r1_out,
            RecordOutcome::Recorded {
                status: BisectStatus::Active,
                next: "next",
                failures: Vec::new(),
            }
        );
        assert_eq!(state.queue[0].a_round, Some(1));

        // Round 2 tests [ppp, z-pkg] and fails.
        let NextOutcome::Round(plan2) =
            next_round(&mut state, &golden, &ledger, &[], now).expect("round 2")
        else {
            panic!("expected round 2");
        };
        assert_eq!(plan2.round, 2);
        assert_eq!(plan2.testing, vec!["ppp".to_string(), "z-pkg".to_string()]);
        assert_eq!(plan2.held, vec!["a-pkg".to_string(), "b-pkg".to_string()]);

        let r2_out = record_round(
            &mut state,
            RoundInput {
                result: PhaseResult::Fail,
                class: Some(COMPONENT_CLASS.to_string()),
                classifier_rule: "ppp-break".to_string(),
                log: "logs/round-2/vm.log".to_string(),
                now,
            },
        )
        .expect("record round 2 fail");
        assert_eq!(
            r2_out,
            RecordOutcome::Recorded {
                status: BisectStatus::Active,
                next: "next",
                failures: Vec::new(),
            }
        );
        assert_eq!(state.queue[0].a_round, None);
        assert_eq!(
            state.queue[0].members,
            vec!["ppp".to_string(), "z-pkg".to_string()]
        );

        // Fork state before round 3 to test both first-half culprit (`ppp`) and second-half culprit (`z-pkg`).
        let mut state_second_half = state.clone();

        // Branch 1: Round 3 tests [ppp] and fails -> Culprit `ppp`.
        let NextOutcome::Round(plan3) =
            next_round(&mut state, &golden, &ledger, &[], now).expect("round 3")
        else {
            panic!("expected round 3");
        };
        assert_eq!(plan3.round, 3);
        assert_eq!(plan3.testing, vec!["ppp".to_string()]);
        assert_eq!(
            plan3.held,
            vec![
                "a-pkg".to_string(),
                "b-pkg".to_string(),
                "z-pkg".to_string()
            ]
        );

        let r3_time = Utc.with_ymd_and_hms(2026, 11, 10, 13, 0, 0).unwrap();
        let r3_out = record_round(
            &mut state,
            RoundInput {
                result: PhaseResult::Fail,
                class: Some(COMPONENT_CLASS.to_string()),
                classifier_rule: "ppp-confirmed-rule".to_string(),
                log: "logs/round-3/vm.log".to_string(),
                now: r3_time,
            },
        )
        .expect("record round 3 fail");

        assert_eq!(
            r3_out,
            RecordOutcome::Recorded {
                status: BisectStatus::Culprit,
                next: "rerun-without-culprit",
                failures: vec![Failure {
                    component: "ppp".to_string(),
                    version: "2.5.5-1+1".to_string(),
                    phase: Phase::VmLiveBoot,
                    class: COMPONENT_CLASS.to_string(),
                    classifier_rule: "ppp-confirmed-rule".to_string(),
                    log: "logs/round-3/vm.log".to_string(),
                    recorded_at: "2026-11-10T13:00:00Z".to_string(),
                    source: FailureSource::Bisect,
                    members: Vec::new(),
                }],
            }
        );
        assert_eq!(state.status.status_code(), "BISECT_CULPRIT");
        assert_eq!(state.status.exit_code(), ExitCode::Ok);
        assert_eq!(
            state.culprits,
            vec![Culprit {
                component: "ppp".to_string(),
                version: "2.5.5-1+1".to_string(),
                round: 3,
            }]
        );
        assert!(state.queue.is_empty());

        // Branch 2: Round 3 [ppp] passes -> Round 4 tests [z-pkg] and fails -> Culprit `z-pkg`.
        next_round(&mut state_second_half, &golden, &ledger, &[], now).expect("round 3 b2");
        let r3_pass = record_round(
            &mut state_second_half,
            RoundInput {
                result: PhaseResult::Pass,
                class: None,
                classifier_rule: String::new(),
                log: "logs/round-3/pass.log".to_string(),
                now,
            },
        )
        .expect("round 3 pass b2");
        assert_eq!(
            r3_pass,
            RecordOutcome::Recorded {
                status: BisectStatus::Active,
                next: "next",
                failures: Vec::new(),
            }
        );

        let NextOutcome::Round(plan4) =
            next_round(&mut state_second_half, &golden, &ledger, &[], now).expect("round 4 b2")
        else {
            panic!("expected round 4");
        };
        assert_eq!(plan4.round, 4);
        assert_eq!(plan4.testing, vec!["z-pkg".to_string()]);
        assert_eq!(
            plan4.held,
            vec!["a-pkg".to_string(), "b-pkg".to_string(), "ppp".to_string()]
        );

        let r4_out = record_round(
            &mut state_second_half,
            RoundInput {
                result: PhaseResult::Fail,
                class: Some(COMPONENT_CLASS.to_string()),
                classifier_rule: "z-pkg-rule".to_string(),
                log: "logs/round-4/vm.log".to_string(),
                now: r3_time,
            },
        )
        .expect("round 4 fail b2");
        assert_eq!(
            r4_out,
            RecordOutcome::Recorded {
                status: BisectStatus::Culprit,
                next: "rerun-without-culprit",
                failures: vec![Failure {
                    component: "z-pkg".to_string(),
                    version: "1.1-1".to_string(),
                    phase: Phase::VmLiveBoot,
                    class: COMPONENT_CLASS.to_string(),
                    classifier_rule: "z-pkg-rule".to_string(),
                    log: "logs/round-4/vm.log".to_string(),
                    recorded_at: "2026-11-10T13:00:00Z".to_string(),
                    source: FailureSource::Bisect,
                    members: Vec::new(),
                }],
            }
        );
        assert_eq!(
            state_second_half.culprits,
            vec![Culprit {
                component: "z-pkg".to_string(),
                version: "1.1-1".to_string(),
                round: 4,
            }]
        );
    }

    #[test]
    fn interaction_and_cap_reached_and_active_bisect_candidate() {
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 14, 0, 0).unwrap();
        let golden = sample_golden();
        let ledger = Ledger::default();

        let mut cand = sample_candidate();
        cand.changed = vec![
            changed_comp("a-pkg", Some("1.0-1"), "1.1-1"),
            changed_comp("b-pkg", Some("1.0-1"), "1.2-1"),
        ];

        let StartOutcome::Started { mut state, .. } = start_bisect(Some(&cand), None, now) else {
            panic!("expected Started");
        };
        assert_eq!(
            active_bisect_candidate(Some(&state)),
            Some(cand.candidate_id.as_str())
        );
        assert_eq!(active_bisect_candidate(None), None);

        // Round 1 [a-pkg] passes.
        let NextOutcome::Round(r1) =
            next_round(&mut state, &golden, &ledger, &[], now).expect("r1")
        else {
            panic!("expected Round 1");
        };
        assert_eq!(r1.testing, vec!["a-pkg".to_string()]);
        record_round(
            &mut state,
            RoundInput {
                result: PhaseResult::Pass,
                class: None,
                classifier_rule: String::new(),
                log: "logs/r1.log".to_string(),
                now,
            },
        )
        .expect("r1 pass");

        // Round 2 [b-pkg] passes -> Interaction!
        let NextOutcome::Round(r2) =
            next_round(&mut state, &golden, &ledger, &[], now).expect("r2")
        else {
            panic!("expected Round 2");
        };
        assert_eq!(r2.testing, vec!["b-pkg".to_string()]);

        let rec_time = Utc.with_ymd_and_hms(2026, 11, 10, 14, 10, 0).unwrap();
        let out = record_round(
            &mut state,
            RoundInput {
                result: PhaseResult::Pass,
                class: None,
                classifier_rule: String::new(),
                log: "logs/r2.log".to_string(),
                now: rec_time,
            },
        )
        .expect("r2 pass -> interaction");

        let members = vec!["a-pkg".to_string(), "b-pkg".to_string()];
        assert_eq!(
            out,
            RecordOutcome::Recorded {
                status: BisectStatus::Interaction,
                next: "stop",
                failures: vec![
                    Failure {
                        component: "a-pkg".to_string(),
                        version: "1.1-1".to_string(),
                        phase: Phase::VmLiveBoot,
                        class: COMPONENT_CLASS.to_string(),
                        classifier_rule: "systemd-unit-failed".to_string(),
                        log: "logs/round-0/vm-live-boot.log".to_string(),
                        recorded_at: "2026-11-10T14:10:00Z".to_string(),
                        source: FailureSource::Interaction,
                        members: members.clone(),
                    },
                    Failure {
                        component: "b-pkg".to_string(),
                        version: "1.2-1".to_string(),
                        phase: Phase::VmLiveBoot,
                        class: COMPONENT_CLASS.to_string(),
                        classifier_rule: "systemd-unit-failed".to_string(),
                        log: "logs/round-0/vm-live-boot.log".to_string(),
                        recorded_at: "2026-11-10T14:10:00Z".to_string(),
                        source: FailureSource::Interaction,
                        members: members.clone(),
                    },
                ],
            }
        );
        assert_eq!(state.status.exit_code(), ExitCode::BisectStop);
        assert_eq!(state.status.status_code(), "BISECT_INTERACTION");
        assert_eq!(state.interaction, members);
        assert_eq!(active_bisect_candidate(Some(&state)), None);

        // Cap test: state literal with cap = 1 -> after one passing round on A, CapReached with no failures.
        let StartOutcome::Started {
            state: mut cap_state,
            ..
        } = start_bisect(Some(&cand), None, now)
        else {
            panic!("expected Started");
        };
        cap_state.cap = 1;
        next_round(&mut cap_state, &golden, &ledger, &[], now).expect("cap r1");
        let cap_out = record_round(
            &mut cap_state,
            RoundInput {
                result: PhaseResult::Pass,
                class: None,
                classifier_rule: String::new(),
                log: "logs/cap-r1.log".to_string(),
                now,
            },
        )
        .expect("cap r1 pass");
        assert_eq!(
            cap_out,
            RecordOutcome::Recorded {
                status: BisectStatus::CapReached,
                next: "stop",
                failures: Vec::new(),
            }
        );
        assert_eq!(BisectStatus::Active.status_code(), "BISECT_ROUND_RECORDED");
        assert_eq!(BisectStatus::Active.exit_code(), ExitCode::Ok);
        assert_eq!(cap_state.status.exit_code(), ExitCode::BisectStop);
        assert_eq!(cap_state.status.status_code(), "BISECT_CAP_REACHED");
        assert_eq!(active_bisect_candidate(Some(&cap_state)), None);
        assert_eq!(
            next_round(&mut cap_state, &golden, &ledger, &[], now).expect("next after cap"),
            NextOutcome::CapReached
        );

        // Active state whose completed rounds already meet cap transitions to CapReached in next_round.
        let mut active_at_cap = cap_state.clone();
        active_at_cap.status = BisectStatus::Active;
        assert_eq!(
            next_round(&mut active_at_cap, &golden, &ledger, &[], now)
                .expect("active at cap -> CapReached"),
            NextOutcome::CapReached
        );
        assert_eq!(active_at_cap.status, BisectStatus::CapReached);

        // record_round on a non-active state returns NotReady(NotActive).
        let mut culprit_state = cap_state;
        culprit_state.status = BisectStatus::Culprit;
        assert_eq!(active_bisect_candidate(Some(&culprit_state)), None);
        match record_round(
            &mut culprit_state,
            RoundInput {
                result: PhaseResult::Pass,
                class: None,
                classifier_rule: String::new(),
                log: String::new(),
                now,
            },
        )
        .expect("record on non-active")
        {
            RecordOutcome::NotReady { reason, detail } => {
                assert_eq!(reason, BisectNotReady::NotActive);
                assert!(detail.contains("culprit"));
            }
            other => panic!("expected NotActive, got {other:?}"),
        }

        // Multi-suspect queue: isolating a culprit from queue[0] while queue[1] remains keeps status Active.
        let StartOutcome::Started {
            state: mut multi_state,
            ..
        } = start_bisect(Some(&cand), None, now)
        else {
            panic!("expected Started");
        };
        multi_state.queue.push(Suspect {
            a_round: None,
            classifier_rule: "second-suspect-rule".to_string(),
            log: "logs/round-0/second.log".to_string(),
            members: vec!["ppp".to_string(), "z-pkg".to_string()],
        });
        multi_state
            .versions
            .insert("ppp".to_string(), "2.5.5-1+1".to_string());
        multi_state
            .versions
            .insert("z-pkg".to_string(), "1.1-1".to_string());
        next_round(&mut multi_state, &golden, &ledger, &[], now).expect("multi r1");
        let multi_out = record_round(
            &mut multi_state,
            RoundInput {
                result: PhaseResult::Fail,
                class: Some(COMPONENT_CLASS.to_string()),
                classifier_rule: "a-pkg-culprit".to_string(),
                log: "logs/multi-r1.log".to_string(),
                now,
            },
        )
        .expect("multi r1 fail");
        assert_eq!(
            multi_out,
            RecordOutcome::Recorded {
                status: BisectStatus::Active,
                next: "next",
                failures: vec![Failure {
                    component: "a-pkg".to_string(),
                    version: "1.1-1".to_string(),
                    phase: Phase::VmLiveBoot,
                    class: COMPONENT_CLASS.to_string(),
                    classifier_rule: "a-pkg-culprit".to_string(),
                    log: "logs/multi-r1.log".to_string(),
                    recorded_at: "2026-11-10T14:00:00Z".to_string(),
                    source: FailureSource::Bisect,
                    members: Vec::new(),
                }],
            }
        );
        assert_eq!(multi_state.culprits.len(), 1);
        assert_eq!(multi_state.queue.len(), 1);
        assert_eq!(
            multi_state.queue[0].members,
            vec!["ppp".to_string(), "z-pkg".to_string()]
        );
    }

    #[test]
    fn bisect_io_roundtrip_schema_checks_and_remove() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("release/bisect.toml");

        assert_eq!(read_bisect(&path).expect("missing file"), None);
        assert!(!remove_bisect(&path).expect("remove missing"));

        let now = Utc.with_ymd_and_hms(2026, 11, 10, 15, 0, 0).unwrap();
        let cand = sample_candidate();
        let golden = sample_golden();
        let ledger = Ledger::default();

        let StartOutcome::Started { mut state, .. } = start_bisect(Some(&cand), None, now) else {
            panic!("expected Started");
        };
        next_round(&mut state, &golden, &ledger, &[], now).expect("round 1");
        record_round(
            &mut state,
            RoundInput {
                result: PhaseResult::Pass,
                class: None,
                classifier_rule: String::new(),
                log: "logs/r1.log".to_string(),
                now,
            },
        )
        .expect("record round 1");

        // Populate culprits as well so every array-of-tables and `[versions]` serializes together.
        state.culprits.push(Culprit {
            component: "ppp".to_string(),
            version: "2.5.5-1+1".to_string(),
            round: 1,
        });

        write_bisect(&path, &state).expect("write_bisect");
        let raw = fs::read_to_string(&path).expect("read raw bisect.toml");
        assert!(raw.starts_with(
            "# NovOS bisect state (.tracker/release/bisect.toml)\n\
             # Written by neb release bisect.\n\n"
        ));

        let loaded = read_bisect(&path)
            .expect("read_bisect")
            .expect("Some(BisectState)");
        assert_eq!(loaded, *state);

        assert!(remove_bisect(&path).expect("remove existing"));
        assert!(!path.exists());
        assert!(!remove_bisect(&path).expect("remove after removal"));

        // schema_version = 99 -> UnsupportedSchema.
        let future_toml = raw.replacen("schema_version = 1", "schema_version = 99", 1);
        fs::write(&path, future_toml).expect("write future schema");
        match read_bisect(&path).expect_err("future schema must fail") {
            TrackerError::UnsupportedSchema {
                found, supported, ..
            } => {
                assert_eq!(found, 99);
                assert_eq!(supported, BISECT_SCHEMA_VERSION);
            }
            other => panic!("expected UnsupportedSchema, got {other:?}"),
        }

        // schema_version = 0 -> ValidationError.
        let zero_toml = raw.replacen("schema_version = 1", "schema_version = 0", 1);
        fs::write(&path, zero_toml).expect("write zero schema");
        assert!(matches!(
            read_bisect(&path).expect_err("zero schema must fail"),
            TrackerError::ValidationError(_)
        ));
    }
}
