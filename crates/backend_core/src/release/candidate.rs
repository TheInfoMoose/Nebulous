use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};
use crate::release::approvals::Approvals;
use crate::release::component::find_match;
use crate::release::debver;
use crate::release::golden::{Golden, Origin};
use crate::release::holds::{ledger_holds, HoldReason};
use crate::release::ledger::{Ledger, Phase, COMPONENT_CLASS};
use crate::release::manifest::Manifest;
use crate::release::records::ReleaseRecord;
use crate::release::variance::{evaluate_variance, VarianceReport};
use crate::release::Violation;

pub const CANDIDATE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseState {
    #[default]
    Pending,
    Pass,
    FailUnconfirmed,
    FailConfirmed,
}

impl PhaseState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Pass => "pass",
            Self::FailUnconfirmed => "fail_unconfirmed",
            Self::FailConfirmed => "fail_confirmed",
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Phases {
    pub build: PhaseState,
    #[serde(rename = "vm-live-boot")]
    pub vm_live_boot: PhaseState,
    #[serde(rename = "vm-install-e2e")]
    pub vm_install_e2e: PhaseState,
}

impl Phases {
    pub fn get(&self, p: Phase) -> PhaseState {
        match p {
            Phase::Build => self.build,
            Phase::VmLiveBoot => self.vm_live_boot,
            Phase::VmInstallE2e => self.vm_install_e2e,
        }
    }

    pub fn set(&mut self, p: Phase, s: PhaseState) {
        match p {
            Phase::Build => self.build = s,
            Phase::VmLiveBoot => self.vm_live_boot = s,
            Phase::VmInstallE2e => self.vm_install_e2e = s,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedComponent {
    pub component: String,
    pub origin: Origin,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_version: Option<String>,
    pub to_version: String,
    #[serde(default)]
    pub kernel_line: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldInForce {
    pub component: String,
    pub held_version: String,
    pub reason: HoldReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PhaseResult {
    Pass,
    Fail,
}

impl PhaseResult {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseRecord {
    pub phase: Phase,
    pub result: PhaseResult,
    pub state: PhaseState,
    #[serde(default)]
    pub class: String,
    #[serde(default)]
    pub classifier_rule: String,
    #[serde(default)]
    pub logs: Vec<String>,
    #[serde(default)]
    pub iso_sha256: String,
    #[serde(default)]
    pub verdict: String,
    #[serde(default)]
    pub flaked: bool,
    pub recorded_at: String,
}

/// Active release candidate state stored at `.tracker/release/candidate.toml`.
///
/// Scalar fields precede table fields so `toml::to_string` never emits a scalar after a table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Candidate {
    pub schema_version: u32,
    pub candidate_id: String,
    pub created_at: String,
    pub iso_path: String,
    pub iso_sha256: String,
    pub snapshot: String,
    #[serde(default)]
    pub artifact_cache: String,
    #[serde(default)]
    pub hold_file: String,
    #[serde(default)]
    pub hold_file_sha256: String,
    pub repo_head: String,
    pub unchanged_count: usize,
    #[serde(default)]
    pub phases: Phases,
    #[serde(default)]
    pub changed: Vec<ChangedComponent>,
    #[serde(default)]
    pub holds_in_force: Vec<HoldInForce>,
    #[serde(default)]
    pub history: Vec<PhaseRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateInput<'a> {
    pub live: &'a Golden,
    pub iso_path: &'a str,
    pub iso_sha256: &'a str,
    pub snapshot: &'a str,
    pub artifact_cache: &'a str,
    pub hold_file: &'a str,
    pub hold_file_sha256: &'a str,
    pub repo_head: &'a str,
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhaseInput {
    pub phase: Phase,
    pub result: PhaseResult,
    pub logs: Vec<String>,
    pub iso_sha256: Option<String>,
    pub hold_file_sha256: String,
    pub snapshot: String,
    pub repo_head: String,
    pub class: Option<String>,
    pub classifier_rule: Option<String>,
    pub verdict: Option<String>,
    pub recorded_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CandidateOutcome {
    Created {
        candidate: Box<Candidate>,
        report: VarianceReport,
    },
    VarianceLock {
        violations: Vec<Violation>,
        report: VarianceReport,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefusalReason {
    NoCandidate,
    OutOfOrder,
    IsoMismatch,
    InputsMismatch,
    PhaseFailed,
}

impl RefusalReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoCandidate => "no_candidate",
            Self::OutOfOrder => "out_of_order",
            Self::IsoMismatch => "iso_mismatch",
            Self::InputsMismatch => "inputs_mismatch",
            Self::PhaseFailed => "phase_failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseOutcome {
    pub status: String,
    pub phase: Phase,
    pub candidate_id: String,
    pub phases: Phases,
    pub next: String,
    pub changed_count: usize,
    pub rerun_required: bool,
    pub flaked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordPhaseResult {
    Recorded(PhaseOutcome),
    RejectedClass {
        class: String,
    },
    Refused {
        reason: RefusalReason,
        detail: String,
    },
}

pub fn candidate_id(iso_sha256: &str, now: &DateTime<Utc>) -> String {
    let prefix: String = iso_sha256.chars().take(12).collect();
    format!("{prefix}-{}", now.format("%Y%m%dT%H%M%SZ"))
}

/// Purely evaluates a live inventory (`input.live`) against `manifest`, `golden`, `approvals`,
/// and `ledger` to construct a [`Candidate`] or return [`CandidateOutcome::VarianceLock`].
pub fn build_candidate(
    input: &CandidateInput,
    manifest: &Manifest,
    golden: Option<&Golden>,
    approvals: &Approvals,
    ledger: &Ledger,
    records: &[(PathBuf, ReleaseRecord)],
) -> Result<CandidateOutcome> {
    let report = evaluate_variance(input.live, manifest, golden, approvals)?;

    // Map each matched live component ID to its golden version (keeping the first match in golden ID order).
    let mut golden_versions: BTreeMap<&str, &str> = BTreeMap::new();
    if let Some(g) = golden {
        for (gid, gcomp) in &g.components {
            if let Some(m) = find_match(gid, gcomp, &input.live.components) {
                golden_versions
                    .entry(m.id)
                    .or_insert(gcomp.version.as_str());
            }
        }
    }

    let mut changed = Vec::new();
    let mut unchanged_count = 0usize;

    for (live_id, live_comp) in &input.live.components {
        match golden_versions.get(live_id.as_str()) {
            None => {
                changed.push(ChangedComponent {
                    component: live_id.clone(),
                    origin: live_comp.origin,
                    from_version: None,
                    to_version: live_comp.version.clone(),
                    kernel_line: live_comp.kernel_line,
                });
            }
            Some(&from_ver) => match debver::compare(&live_comp.version, from_ver) {
                Ordering::Greater => {
                    changed.push(ChangedComponent {
                        component: live_id.clone(),
                        origin: live_comp.origin,
                        from_version: Some(from_ver.to_string()),
                        to_version: live_comp.version.clone(),
                        kernel_line: live_comp.kernel_line,
                    });
                }
                Ordering::Equal => {
                    unchanged_count += 1;
                }
                Ordering::Less => {
                    // Downgrade: already in report.violations unless approved away.
                }
            },
        }
    }

    let mut violations = report.violations.clone();
    for ch in &changed {
        if ledger.has_exact(&ch.component, &ch.to_version) {
            violations.push(Violation::new(
                "held_version_present",
                ch.component.clone(),
                format!(
                    "Component '{}' {} is held by the failure ledger but is present in the ISO; the hold file was not applied",
                    ch.component, ch.to_version
                ),
            ));
        }
    }

    if !violations.is_empty() {
        return Ok(CandidateOutcome::VarianceLock { violations, report });
    }

    let holds_in_force = match golden {
        Some(g) => ledger_holds(ledger, g, records)
            .into_iter()
            .map(|h| HoldInForce {
                component: h.component,
                held_version: h.held_version,
                reason: h.reason,
            })
            .collect(),
        None => Vec::new(),
    };

    let candidate = Candidate {
        schema_version: CANDIDATE_SCHEMA_VERSION,
        candidate_id: candidate_id(input.iso_sha256, &input.now),
        created_at: input.now.to_rfc3339_opts(SecondsFormat::Secs, true),
        iso_path: input.iso_path.to_string(),
        iso_sha256: input.iso_sha256.to_string(),
        snapshot: input.snapshot.to_string(),
        artifact_cache: input.artifact_cache.to_string(),
        hold_file: input.hold_file.to_string(),
        hold_file_sha256: input.hold_file_sha256.to_string(),
        repo_head: input.repo_head.to_string(),
        unchanged_count,
        phases: Phases::default(),
        changed,
        holds_in_force,
        history: Vec::new(),
    };

    Ok(CandidateOutcome::Created {
        candidate: Box::new(candidate),
        report,
    })
}

/// Reads `DEBIAN_SNAPSHOT` from `<repo_root>/config/build.env`.
pub fn read_build_env_snapshot(repo_root: &Path) -> Result<String> {
    let path = repo_root.join("config/build.env");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => {
            return Err(TrackerError::ValidationError(
                "DEBIAN_SNAPSHOT is not set in config/build.env; pass --snapshot".to_string(),
            ));
        }
        Err(source) => {
            return Err(TrackerError::Io { path, source });
        }
    };

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let rest = trimmed
            .strip_prefix("export ")
            .map(str::trim_start)
            .unwrap_or(trimmed);
        if let Some(raw_val) = rest.strip_prefix("DEBIAN_SNAPSHOT=") {
            let val = raw_val.trim();
            let unquoted = if (val.starts_with('"') && val.ends_with('"') && val.len() >= 2)
                || (val.starts_with('\'') && val.ends_with('\'') && val.len() >= 2)
            {
                &val[1..val.len() - 1]
            } else {
                val
            };
            let clean = unquoted.trim();
            if clean.is_empty() {
                return Err(TrackerError::ValidationError(
                    "DEBIAN_SNAPSHOT is not set in config/build.env; pass --snapshot".to_string(),
                ));
            }
            return Ok(clean.to_string());
        }
    }

    Err(TrackerError::ValidationError(
        "DEBIAN_SNAPSHOT is not set in config/build.env; pass --snapshot".to_string(),
    ))
}

pub fn read_candidate(path: &Path) -> Result<Option<Candidate>> {
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
            if found > CANDIDATE_SCHEMA_VERSION {
                return Err(TrackerError::UnsupportedSchema {
                    path: path.to_path_buf(),
                    found,
                    supported: CANDIDATE_SCHEMA_VERSION,
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

    let candidate = Candidate::deserialize(toml::Value::Table(table)).map_err(|source| {
        TrackerError::TomlDeserialize {
            path: Some(path.to_path_buf()),
            source,
        }
    })?;

    Ok(Some(candidate))
}

pub fn render_candidate(c: &Candidate) -> Result<String> {
    let body = toml::to_string(c)?;
    Ok(format!(
        "# NovOS release candidate (.tracker/release/candidate.toml)\n\
         # Written by neb release candidate and neb release record-phase.\n\n\
         {body}"
    ))
}

/// Renders and atomically writes `c` to `path`, replacing any previous candidate (§4.2).
///
/// The caller must hold `TrackerStorage::with_write_lock`. Any active bisect (`bisect.toml`)
/// is not touched.
pub fn write_candidate(path: &Path, c: &Candidate) -> Result<()> {
    let text = render_candidate(c)?;

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

/// Applies a phase result to `c` in memory after checking failure class, phase ordering,
/// terminal failure state, input identity, and the flake guard (§4.5).
pub fn apply_phase(c: &mut Candidate, input: PhaseInput) -> Result<RecordPhaseResult> {
    // (a) Fail class & classifier_rule validation.
    let (record_class, record_rule) = match input.result {
        PhaseResult::Fail => {
            let cls = match input
                .class
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                Some(cls) => cls.to_string(),
                None => {
                    return Err(TrackerError::ValidationError(
                        "record-phase fail requires --class".to_string(),
                    ));
                }
            };
            if cls != COMPONENT_CLASS {
                return Ok(RecordPhaseResult::RejectedClass { class: cls });
            }
            let rule = match input
                .classifier_rule
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
            {
                Some(r) => r.to_string(),
                None => {
                    return Err(TrackerError::ValidationError(
                        "record-phase fail requires a non-blank --classifier-rule".to_string(),
                    ));
                }
            };
            (cls, rule)
        }
        PhaseResult::Pass => (
            input.class.unwrap_or_default(),
            input.classifier_rule.unwrap_or_default(),
        ),
    };

    // (b) Order: every phase before input.phase must be Pass; every phase after must be Pending.
    for p in Phase::ALL {
        if p < input.phase {
            let st = c.phases.get(p);
            if st != PhaseState::Pass {
                return Ok(RecordPhaseResult::Refused {
                    reason: RefusalReason::OutOfOrder,
                    detail: format!(
                        "cannot record phase '{}': prerequisite phase '{}' is '{}' (expected 'pass')",
                        input.phase.as_str(),
                        p.as_str(),
                        st.as_str()
                    ),
                });
            }
        } else if p > input.phase {
            let st = c.phases.get(p);
            if st != PhaseState::Pending {
                return Ok(RecordPhaseResult::Refused {
                    reason: RefusalReason::OutOfOrder,
                    detail: format!(
                        "cannot record phase '{}': later phase '{}' is already '{}' (expected 'pending')",
                        input.phase.as_str(),
                        p.as_str(),
                        st.as_str()
                    ),
                });
            }
        }
    }

    // (c) Current state is FailConfirmed -> Refused(PhaseFailed).
    let current_state = c.phases.get(input.phase);
    if current_state == PhaseState::FailConfirmed {
        return Ok(RecordPhaseResult::Refused {
            reason: RefusalReason::PhaseFailed,
            detail: format!(
                "phase '{}' is already 'fail_confirmed' for candidate '{}'",
                input.phase.as_str(),
                c.candidate_id
            ),
        });
    }

    // (d) Identity checks.
    if input.hold_file_sha256 != c.hold_file_sha256 {
        return Ok(RecordPhaseResult::Refused {
            reason: RefusalReason::InputsMismatch,
            detail: format!(
                "hold_file_sha256 mismatch: input '{}' != candidate '{}'",
                input.hold_file_sha256, c.hold_file_sha256
            ),
        });
    }
    if input.snapshot != c.snapshot {
        return Ok(RecordPhaseResult::Refused {
            reason: RefusalReason::InputsMismatch,
            detail: format!(
                "snapshot mismatch: input '{}' != candidate '{}'",
                input.snapshot, c.snapshot
            ),
        });
    }
    if input.repo_head != c.repo_head {
        return Ok(RecordPhaseResult::Refused {
            reason: RefusalReason::InputsMismatch,
            detail: format!(
                "repo_head mismatch: input '{}' != candidate '{}'",
                input.repo_head, c.repo_head
            ),
        });
    }

    match input.phase {
        Phase::VmLiveBoot | Phase::VmInstallE2e => match input.iso_sha256.as_deref() {
            Some(sha) if sha == c.iso_sha256 => {}
            Some(sha) => {
                return Ok(RecordPhaseResult::Refused {
                    reason: RefusalReason::IsoMismatch,
                    detail: format!(
                        "iso_sha256 mismatch for phase '{}': input '{sha}' != candidate '{}'",
                        input.phase.as_str(),
                        c.iso_sha256
                    ),
                });
            }
            None => {
                return Ok(RecordPhaseResult::Refused {
                    reason: RefusalReason::IsoMismatch,
                    detail: format!(
                        "iso_sha256 is required for VM phase '{}'",
                        input.phase.as_str()
                    ),
                });
            }
        },
        Phase::Build => {
            if input.result == PhaseResult::Pass {
                if let Some(sha) = input.iso_sha256.as_deref() {
                    if sha != c.iso_sha256 {
                        return Ok(RecordPhaseResult::Refused {
                            reason: RefusalReason::IsoMismatch,
                            detail: format!(
                                "iso_sha256 mismatch for phase 'build': input '{sha}' != candidate '{}'",
                                c.iso_sha256
                            ),
                        });
                    }
                }
            }
        }
    }

    // (e) State transition & flake guard.
    let (new_state, status, next, rerun_required, flaked) = match input.result {
        PhaseResult::Pass => {
            let flaked = current_state == PhaseState::FailUnconfirmed;
            let next = match input.phase.next() {
                Some(next_phase) => next_phase.as_str().to_string(),
                None => "promote".to_string(),
            };
            (PhaseState::Pass, "PASS".to_string(), next, false, flaked)
        }
        PhaseResult::Fail => match current_state {
            PhaseState::Pending | PhaseState::Pass => (
                PhaseState::FailUnconfirmed,
                "FAIL_UNCONFIRMED".to_string(),
                "rerun".to_string(),
                true,
                false,
            ),
            PhaseState::FailUnconfirmed => {
                let next = match c.changed.len() {
                    0 => "stop".to_string(),
                    1 => "record-failure".to_string(),
                    _ => "bisect".to_string(),
                };
                (
                    PhaseState::FailConfirmed,
                    "FAIL_CONFIRMED".to_string(),
                    next,
                    false,
                    false,
                )
            }
            PhaseState::FailConfirmed => unreachable!("checked in step (c)"),
        },
    };

    c.phases.set(input.phase, new_state);

    // (f) Append PhaseRecord and return Recorded.
    c.history.push(PhaseRecord {
        phase: input.phase,
        result: input.result,
        state: new_state,
        class: record_class,
        classifier_rule: record_rule,
        logs: input.logs,
        iso_sha256: input.iso_sha256.unwrap_or_default(),
        verdict: input.verdict.unwrap_or_default(),
        flaked,
        recorded_at: input.recorded_at,
    });

    Ok(RecordPhaseResult::Recorded(PhaseOutcome {
        status,
        phase: input.phase,
        candidate_id: c.candidate_id.clone(),
        phases: c.phases.clone(),
        next,
        changed_count: c.changed.len(),
        rerun_required,
        flaked,
    }))
}

/// Loads `candidate.toml` from `path`, applies `input`, and writes the updated candidate
/// only when [`RecordPhaseResult::Recorded`] is returned.
///
/// The caller must hold `TrackerStorage::with_write_lock`.
pub fn record_phase(path: &Path, input: PhaseInput) -> Result<RecordPhaseResult> {
    let Some(mut candidate) = read_candidate(path)? else {
        return Ok(RecordPhaseResult::Refused {
            reason: RefusalReason::NoCandidate,
            detail: "no active candidate (.tracker/release/candidate.toml does not exist)"
                .to_string(),
        });
    };

    let res = apply_phase(&mut candidate, input)?;
    if matches!(res, RecordPhaseResult::Recorded(_)) {
        write_candidate(path, &candidate)?;
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tempfile::tempdir;

    use super::*;
    use crate::release::approvals::Approval;
    use crate::release::golden::{GoldenComponent, GOLDEN_SCHEMA_VERSION};
    use crate::release::inventory::{BootInfo, CalamaresModules, IsoMeta};
    use crate::release::ledger::{Failure, FailureSource};
    use crate::release::records::{RecordComponent, RELEASE_RECORD_SCHEMA_VERSION};

    fn empty_golden() -> Golden {
        Golden {
            schema_version: GOLDEN_SCHEMA_VERSION,
            iso: IsoMeta {
                path: "build/test.iso".to_string(),
                size_bytes: 1024,
                vmlinuz_sha256: "a".repeat(64),
                initrd_sha256: "b".repeat(64),
                extracted_at: "2026-11-09T00:00:00Z".to_string(),
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
            components: BTreeMap::new(),
        }
    }

    fn comp(version: &str, origin: Origin, kernel_line: bool) -> GoldenComponent {
        let mut binaries = BTreeMap::new();
        binaries.insert("bin".to_string(), version.to_string());
        GoldenComponent {
            version: version.to_string(),
            origin,
            kernel_line,
            source_unknown: false,
            binaries,
        }
    }

    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 10, 12, 30, 45).unwrap()
    }

    const SAMPLE_HOLD_SHA: &str =
        "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    fn sample_input<'a>(live: &'a Golden, sha: &'a str) -> CandidateInput<'a> {
        CandidateInput {
            live,
            iso_path: "build/novos.iso",
            iso_sha256: sha,
            snapshot: "20261101T000000Z",
            artifact_cache: "artifact-cache",
            hold_file: "build/holds.toml",
            hold_file_sha256: SAMPLE_HOLD_SHA,
            repo_head: "deadbeef1234567890",
            now: fixed_now(),
        }
    }

    fn pass_phase_input(phase: Phase, iso_sha: Option<String>, hold_sha: &str) -> PhaseInput {
        PhaseInput {
            phase,
            result: PhaseResult::Pass,
            logs: vec![format!("logs/{}.log", phase.as_str())],
            iso_sha256: iso_sha,
            hold_file_sha256: hold_sha.to_string(),
            snapshot: "20261101T000000Z".to_string(),
            repo_head: "deadbeef1234567890".to_string(),
            class: None,
            classifier_rule: None,
            verdict: None,
            recorded_at: "2026-11-10T12:35:00Z".to_string(),
        }
    }

    fn fail_phase_input(
        phase: Phase,
        iso_sha: Option<String>,
        hold_sha: &str,
        class: Option<&str>,
        rule: Option<&str>,
    ) -> PhaseInput {
        PhaseInput {
            phase,
            result: PhaseResult::Fail,
            logs: vec![format!("logs/{}-fail.log", phase.as_str())],
            iso_sha256: iso_sha,
            hold_file_sha256: hold_sha.to_string(),
            snapshot: "20261101T000000Z".to_string(),
            repo_head: "deadbeef1234567890".to_string(),
            class: class.map(String::from),
            classifier_rule: rule.map(String::from),
            verdict: Some("verdict.json".to_string()),
            recorded_at: "2026-11-10T12:36:00Z".to_string(),
        }
    }

    #[test]
    fn candidate_id_and_created_at_format() {
        let now = fixed_now();
        let sha = "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789";
        assert_eq!(candidate_id(sha, &now), "abcdef012345-20261110T123045Z");
        assert_eq!(candidate_id("short", &now), "short-20261110T123045Z");

        let live = empty_golden();
        let input = CandidateInput {
            live: &live,
            iso_path: "build/test.iso",
            iso_sha256: sha,
            snapshot: "20261101T000000Z",
            artifact_cache: "",
            hold_file: "",
            hold_file_sha256: "",
            repo_head: "11112222",
            now,
        };
        let out = build_candidate(
            &input,
            &Manifest::default(),
            None,
            &Approvals::default(),
            &Ledger::default(),
            &[],
        )
        .expect("build_candidate");
        match out {
            CandidateOutcome::Created { candidate, .. } => {
                assert_eq!(candidate.candidate_id, "abcdef012345-20261110T123045Z");
                assert_eq!(candidate.created_at, "2026-11-10T12:30:45Z");
                assert!(candidate.created_at.ends_with('Z'));
            }
            other => panic!("expected Created, got {other:?}"),
        }
    }

    #[test]
    fn build_candidate_classifies_upgraded_new_equal_and_approved_downgrade() {
        let mut golden = empty_golden();
        golden.components.insert(
            "example-upgraded".to_string(),
            comp("1.0-1", Origin::Debian, false),
        );
        golden.components.insert(
            "example-equal".to_string(),
            comp("2.0-1", Origin::Debian, false),
        );
        golden.components.insert(
            "example-downgraded".to_string(),
            comp("3.0-1", Origin::Debian, false),
        );

        let mut live = empty_golden();
        live.components.insert(
            "example-upgraded".to_string(),
            comp("1.1-1", Origin::Debian, false),
        );
        live.components.insert(
            "example-new-kernel".to_string(),
            comp("7.3.3-1", Origin::ThirdParty, true),
        );
        live.components.insert(
            "example-equal".to_string(),
            comp("2.0-1", Origin::Debian, false),
        );
        live.components.insert(
            "example-downgraded".to_string(),
            comp("2.5-1", Origin::Debian, false),
        );

        let approvals = Approvals {
            approvals: vec![Approval {
                item_name: "example-downgraded".to_string(),
                reason: "approved pin".to_string(),
                approved: true,
                timestamp: "2026-11-10T00:00:00Z".to_string(),
                version: Some("2.5-1".to_string()),
            }],
        };

        let sha = "a".repeat(64);
        let input = sample_input(&live, &sha);
        let out = build_candidate(
            &input,
            &Manifest::default(),
            Some(&golden),
            &approvals,
            &Ledger::default(),
            &[],
        )
        .expect("build_candidate");

        let CandidateOutcome::Created { candidate, .. } = out else {
            panic!("expected Created, got {out:?}");
        };

        assert_eq!(candidate.unchanged_count, 1);
        assert_eq!(candidate.phases, Phases::default());
        assert_eq!(
            candidate.changed,
            vec![
                ChangedComponent {
                    component: "example-new-kernel".to_string(),
                    origin: Origin::ThirdParty,
                    from_version: None,
                    to_version: "7.3.3-1".to_string(),
                    kernel_line: true,
                },
                ChangedComponent {
                    component: "example-upgraded".to_string(),
                    origin: Origin::Debian,
                    from_version: Some("1.0-1".to_string()),
                    to_version: "1.1-1".to_string(),
                    kernel_line: false,
                },
            ]
        );
    }

    #[test]
    fn build_candidate_maps_imported_placeholder_to_live_component() {
        let mut golden = empty_golden();
        let mut placeholder_bins = BTreeMap::new();
        placeholder_bins.insert("libexample0".to_string(), "1.0-1".to_string());
        golden.components.insert(
            "libexample0".to_string(),
            GoldenComponent {
                version: "1.0-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: true,
                binaries: placeholder_bins.clone(),
            },
        );

        // Same version in live under source component id `example-lib` -> unchanged, not new.
        let mut live_equal = empty_golden();
        live_equal.components.insert(
            "example-lib".to_string(),
            GoldenComponent {
                version: "1.0-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: placeholder_bins,
            },
        );

        let sha = "a".repeat(64);
        let input_eq = sample_input(&live_equal, &sha);
        let out_eq = build_candidate(
            &input_eq,
            &Manifest::default(),
            Some(&golden),
            &Approvals::default(),
            &Ledger::default(),
            &[],
        )
        .expect("build_candidate equal placeholder");
        match out_eq {
            CandidateOutcome::Created { candidate, .. } => {
                assert!(candidate.changed.is_empty());
                assert_eq!(candidate.unchanged_count, 1);
            }
            other => panic!("expected Created, got {other:?}"),
        }

        // Upgraded version in live under `example-lib` -> changed with from_version = Some("1.0-1").
        let mut live_up = empty_golden();
        let mut up_bins = BTreeMap::new();
        up_bins.insert("libexample0".to_string(), "1.2-1".to_string());
        live_up.components.insert(
            "example-lib".to_string(),
            GoldenComponent {
                version: "1.2-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: up_bins,
            },
        );
        let input_up = sample_input(&live_up, &sha);
        let out_up = build_candidate(
            &input_up,
            &Manifest::default(),
            Some(&golden),
            &Approvals::default(),
            &Ledger::default(),
            &[],
        )
        .expect("build_candidate upgraded placeholder");
        match out_up {
            CandidateOutcome::Created { candidate, .. } => {
                assert_eq!(candidate.unchanged_count, 0);
                assert_eq!(
                    candidate.changed,
                    vec![ChangedComponent {
                        component: "example-lib".to_string(),
                        origin: Origin::Debian,
                        from_version: Some("1.0-1".to_string()),
                        to_version: "1.2-1".to_string(),
                        kernel_line: false,
                    }]
                );
            }
            other => panic!("expected Created, got {other:?}"),
        }

        // Multiple imported placeholders mapping to the same multi-binary live component ->
        // keeps the first match in golden ID order (`libexample-bin` at 1.0-0 before `libexample0` at 1.0-1).
        let mut bin_placeholder = BTreeMap::new();
        bin_placeholder.insert("libexample-bin".to_string(), "1.0-0".to_string());
        golden.components.insert(
            "libexample-bin".to_string(),
            GoldenComponent {
                version: "1.0-0".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: true,
                binaries: bin_placeholder,
            },
        );
        let mut multi_bins = BTreeMap::new();
        multi_bins.insert("libexample-bin".to_string(), "1.2-1".to_string());
        multi_bins.insert("libexample0".to_string(), "1.2-1".to_string());
        let mut live_multi = empty_golden();
        live_multi.components.insert(
            "example-lib".to_string(),
            GoldenComponent {
                version: "1.2-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: multi_bins,
            },
        );
        let input_multi = sample_input(&live_multi, &sha);
        let out_multi = build_candidate(
            &input_multi,
            &Manifest::default(),
            Some(&golden),
            &Approvals::default(),
            &Ledger::default(),
            &[],
        )
        .expect("build_candidate multiple placeholders");
        match out_multi {
            CandidateOutcome::Created { candidate, .. } => {
                assert_eq!(candidate.unchanged_count, 0);
                assert_eq!(
                    candidate.changed,
                    vec![ChangedComponent {
                        component: "example-lib".to_string(),
                        origin: Origin::Debian,
                        from_version: Some("1.0-0".to_string()),
                        to_version: "1.2-1".to_string(),
                        kernel_line: false,
                    }]
                );
            }
            other => panic!("expected Created, got {other:?}"),
        }
    }

    #[test]
    fn build_candidate_enforces_held_version_present_variance_lock_and_populates_holds_in_force() {
        let mut golden = empty_golden();
        golden.components.insert(
            "example-pkg".to_string(),
            comp("1.0-1", Origin::Debian, false),
        );
        golden.components.insert(
            "example-held".to_string(),
            comp("2.0-1", Origin::Debian, false),
        );

        let ledger = Ledger {
            failures: vec![Failure {
                component: "example-held".to_string(),
                version: "2.1-1".to_string(),
                phase: Phase::Build,
                class: "component".to_string(),
                classifier_rule: "apt-breaks".to_string(),
                log: "logs/build.log".to_string(),
                recorded_at: "2026-11-09T00:00:00Z".to_string(),
                source: FailureSource::Bisect,
                members: Vec::new(),
            }],
        };

        // 1. Live ISO carries held version 2.1-1 -> VarianceLock with held_version_present.
        let mut live_bad = empty_golden();
        live_bad.components.insert(
            "example-pkg".to_string(),
            comp("1.1-1", Origin::Debian, false),
        );
        live_bad.components.insert(
            "example-held".to_string(),
            comp("2.1-1", Origin::Debian, false),
        );

        let sha = "a".repeat(64);
        let input_bad = sample_input(&live_bad, &sha);
        let out_bad = build_candidate(
            &input_bad,
            &Manifest::default(),
            Some(&golden),
            &Approvals::default(),
            &ledger,
            &[],
        )
        .expect("build_candidate held");
        match out_bad {
            CandidateOutcome::VarianceLock { violations, .. } => {
                assert_eq!(violations.len(), 1);
                assert_eq!(violations[0].kind, "held_version_present");
                assert_eq!(violations[0].item, "example-held");
                assert_eq!(
                    violations[0].detail,
                    "Component 'example-held' 2.1-1 is held by the failure ledger but is present in the ISO; the hold file was not applied"
                );
            }
            other => panic!("expected VarianceLock, got {other:?}"),
        }

        // 2. Unapproved removal -> VarianceLock with golden_component_removed.
        let mut live_removed = empty_golden();
        live_removed.components.insert(
            "example-held".to_string(),
            comp("2.0-1", Origin::Debian, false),
        );
        let input_rem = sample_input(&live_removed, &sha);
        let out_rem = build_candidate(
            &input_rem,
            &Manifest::default(),
            Some(&golden),
            &Approvals::default(),
            &ledger,
            &[],
        )
        .expect("build_candidate removed");
        match out_rem {
            CandidateOutcome::VarianceLock { violations, .. } => {
                assert_eq!(violations.len(), 1);
                assert_eq!(violations[0].kind, "golden_component_removed");
                assert_eq!(violations[0].item, "example-pkg");
            }
            other => panic!("expected VarianceLock, got {other:?}"),
        }

        // 3. Hold respected (example-held stays at 2.0-1) -> Created with holds_in_force populated.
        let mut live_ok = empty_golden();
        live_ok.components.insert(
            "example-pkg".to_string(),
            comp("1.1-1", Origin::Debian, false),
        );
        live_ok.components.insert(
            "example-held".to_string(),
            comp("2.0-1", Origin::Debian, false),
        );
        let mut rec_comps = BTreeMap::new();
        rec_comps.insert(
            "example-held".to_string(),
            RecordComponent {
                version: "2.0-1".to_string(),
                origin: Origin::Debian,
                binaries: BTreeMap::new(),
            },
        );
        let records = vec![(
            PathBuf::from("2026.10-20261108T000000Z.toml"),
            ReleaseRecord {
                schema_version: RELEASE_RECORD_SCHEMA_VERSION,
                version: "2026.10".to_string(),
                promoted_at: "2026-11-08T00:00:00Z".to_string(),
                promoted_by: "neb release promote".to_string(),
                reason: String::new(),
                snapshot: "20261101T000000Z".to_string(),
                iso_sha256: "f".repeat(64),
                golden_sha256: "e".repeat(64),
                hold_file_sha256: String::new(),
                candidate_id: "cand-1".to_string(),
                bisect_rounds: 0,
                phases: BTreeMap::new(),
                components: rec_comps,
                third_party_cache: BTreeMap::new(),
                candidate: None,
            },
        )];
        let input_ok = sample_input(&live_ok, &sha);
        let out_ok = build_candidate(
            &input_ok,
            &Manifest::default(),
            Some(&golden),
            &Approvals::default(),
            &ledger,
            &records,
        )
        .expect("build_candidate ok");
        match out_ok {
            CandidateOutcome::Created { candidate, .. } => {
                assert_eq!(
                    candidate.holds_in_force,
                    vec![HoldInForce {
                        component: "example-held".to_string(),
                        held_version: "2.0-1".to_string(),
                        reason: HoldReason::Failure,
                    }]
                );
            }
            other => panic!("expected Created, got {other:?}"),
        }
    }

    #[test]
    fn build_candidate_with_no_golden_marks_every_component_new() {
        let mut live = empty_golden();
        live.components
            .insert("pkg-a".to_string(), comp("1.0-1", Origin::Debian, false));
        live.components.insert(
            "pkg-b".to_string(),
            comp("2.0-1", Origin::ThirdParty, false),
        );

        let sha = "a".repeat(64);
        let input = sample_input(&live, &sha);
        let out = build_candidate(
            &input,
            &Manifest::default(),
            None,
            &Approvals::default(),
            &Ledger::default(),
            &[],
        )
        .expect("build_candidate no golden");

        match out {
            CandidateOutcome::Created { candidate, .. } => {
                assert_eq!(candidate.unchanged_count, 0);
                assert!(candidate.holds_in_force.is_empty());
                assert_eq!(candidate.changed.len(), 2);
                assert!(candidate.changed.iter().all(|c| c.from_version.is_none()));
            }
            other => panic!("expected Created, got {other:?}"),
        }
    }

    #[test]
    fn read_build_env_snapshot_parses_export_and_plain_lines_and_errors_when_missing() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();

        // Missing file.
        let err = read_build_env_snapshot(root).expect_err("missing build.env");
        assert_eq!(
            err.to_string(),
            "Validation error: DEBIAN_SNAPSHOT is not set in config/build.env; pass --snapshot"
        );

        let cfg_dir = root.join("config");
        fs::create_dir_all(&cfg_dir).expect("create config dir");
        let env_path = cfg_dir.join("build.env");

        // Double quotes with export.
        fs::write(
            &env_path,
            "# comment\nexport NOVOS_PIN_SNAPSHOT=\"20261107T205437Z\"\nexport DEBIAN_SNAPSHOT=\"20261101T000000Z\"\n",
        )
        .expect("write build.env");
        assert_eq!(
            read_build_env_snapshot(root).expect("read snapshot"),
            "20261101T000000Z"
        );

        // Single quotes without export.
        fs::write(&env_path, "DEBIAN_SNAPSHOT='20261102T120000Z'\n").expect("write single-quoted");
        assert_eq!(
            read_build_env_snapshot(root).expect("read single-quoted"),
            "20261102T120000Z"
        );

        // Missing key.
        fs::write(
            &env_path,
            "export NOVOS_PIN_SNAPSHOT=\"20261107T205437Z\"\n",
        )
        .expect("write without key");
        let err = read_build_env_snapshot(root).expect_err("missing key");
        assert_eq!(
            err.to_string(),
            "Validation error: DEBIAN_SNAPSHOT is not set in config/build.env; pass --snapshot"
        );

        // Empty value.
        fs::write(&env_path, "export DEBIAN_SNAPSHOT=\"\"\n").expect("write empty value");
        let err = read_build_env_snapshot(root).expect_err("empty value");
        assert_eq!(
            err.to_string(),
            "Validation error: DEBIAN_SNAPSHOT is not set in config/build.env; pass --snapshot"
        );
    }

    #[test]
    fn write_and_read_candidate_round_trip_and_unsupported_schema() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("release/candidate.toml");

        assert_eq!(read_candidate(&path).expect("missing candidate"), None);

        let mut live = empty_golden();
        live.components.insert(
            "example-pkg".to_string(),
            comp("1.2-1", Origin::Debian, false),
        );
        let sha = "a".repeat(64);
        let input = sample_input(&live, &sha);
        let CandidateOutcome::Created { mut candidate, .. } = build_candidate(
            &input,
            &Manifest::default(),
            None,
            &Approvals::default(),
            &Ledger::default(),
            &[],
        )
        .expect("build_candidate") else {
            panic!("expected Created");
        };

        candidate.holds_in_force.push(HoldInForce {
            component: "example-lib".to_string(),
            held_version: "1.0-1".to_string(),
            reason: HoldReason::Failure,
        });
        let hold_sha = candidate.hold_file_sha256.clone();
        apply_phase(
            &mut candidate,
            pass_phase_input(Phase::Build, Some(sha.clone()), &hold_sha),
        )
        .expect("apply build pass");

        write_candidate(&path, &candidate).expect("write_candidate");
        let raw = fs::read_to_string(&path).expect("read raw");
        assert!(raw.starts_with(
            "# NovOS release candidate (.tracker/release/candidate.toml)\n\
             # Written by neb release candidate and neb release record-phase.\n\n"
        ));

        let loaded = read_candidate(&path)
            .expect("read_candidate")
            .expect("Some(candidate)");
        assert_eq!(loaded, *candidate);

        // Schema version 2 -> UnsupportedSchema.
        let mut v2 = (*candidate).clone();
        v2.schema_version = 2;
        write_candidate(&path, &v2).expect("write v2");
        let err = read_candidate(&path).expect_err("schema v2 must error");
        match err {
            TrackerError::UnsupportedSchema {
                found: 2,
                supported: 1,
                ..
            } => {}
            other => panic!("expected UnsupportedSchema, got {other:?}"),
        }

        // Non-positive schema_version -> ValidationError.
        fs::write(&path, "schema_version = 0\n").expect("write schema 0");
        assert!(matches!(
            read_candidate(&path).expect_err("schema 0"),
            TrackerError::ValidationError(_)
        ));

        // Corrupt TOML -> TomlDeserialize.
        fs::write(&path, "not = [valid toml\n").expect("write corrupt toml");
        assert!(matches!(
            read_candidate(&path).expect_err("corrupt toml"),
            TrackerError::TomlDeserialize { path: Some(_), .. }
        ));
    }

    fn make_candidate_with_changed(changed_count: usize) -> (Candidate, String, String) {
        let mut live = empty_golden();
        for i in 0..changed_count {
            live.components.insert(
                format!("example-pkg-{i}"),
                comp("1.0-1", Origin::Debian, false),
            );
        }
        let sha = "a".repeat(64);
        let input = sample_input(&live, &sha);
        let CandidateOutcome::Created { candidate, .. } = build_candidate(
            &input,
            &Manifest::default(),
            None,
            &Approvals::default(),
            &Ledger::default(),
            &[],
        )
        .expect("build_candidate") else {
            panic!("expected Created");
        };
        let hold_sha = candidate.hold_file_sha256.clone();
        (*candidate, sha, hold_sha)
    }

    #[test]
    fn flake_guard_confirmed_failure_next_routing_and_phase_failed_refusal() {
        for (n_changed, expected_next) in [(2, "bisect"), (1, "record-failure"), (0, "stop")] {
            let (mut cand, sha, hold_sha) = make_candidate_with_changed(n_changed);

            // Build pass -> next vm-live-boot.
            let res = apply_phase(
                &mut cand,
                pass_phase_input(Phase::Build, Some(sha.clone()), &hold_sha),
            )
            .expect("build pass");
            assert_eq!(
                res,
                RecordPhaseResult::Recorded(PhaseOutcome {
                    status: "PASS".to_string(),
                    phase: Phase::Build,
                    candidate_id: cand.candidate_id.clone(),
                    phases: Phases {
                        build: PhaseState::Pass,
                        vm_live_boot: PhaseState::Pending,
                        vm_install_e2e: PhaseState::Pending,
                    },
                    next: "vm-live-boot".to_string(),
                    changed_count: n_changed,
                    rerun_required: false,
                    flaked: false,
                })
            );

            // First vm-live-boot fail -> FAIL_UNCONFIRMED, rerun_required = true, next = rerun.
            let res = apply_phase(
                &mut cand,
                fail_phase_input(
                    Phase::VmLiveBoot,
                    Some(sha.clone()),
                    &hold_sha,
                    Some("component"),
                    Some("qemu-boot-timeout"),
                ),
            )
            .expect("first fail");
            assert_eq!(
                res,
                RecordPhaseResult::Recorded(PhaseOutcome {
                    status: "FAIL_UNCONFIRMED".to_string(),
                    phase: Phase::VmLiveBoot,
                    candidate_id: cand.candidate_id.clone(),
                    phases: Phases {
                        build: PhaseState::Pass,
                        vm_live_boot: PhaseState::FailUnconfirmed,
                        vm_install_e2e: PhaseState::Pending,
                    },
                    next: "rerun".to_string(),
                    changed_count: n_changed,
                    rerun_required: true,
                    flaked: false,
                })
            );

            // Second vm-live-boot fail -> FAIL_CONFIRMED, next depends on changed_count.
            let res = apply_phase(
                &mut cand,
                fail_phase_input(
                    Phase::VmLiveBoot,
                    Some(sha.clone()),
                    &hold_sha,
                    Some("component"),
                    Some("qemu-boot-timeout"),
                ),
            )
            .expect("second fail");
            assert_eq!(
                res,
                RecordPhaseResult::Recorded(PhaseOutcome {
                    status: "FAIL_CONFIRMED".to_string(),
                    phase: Phase::VmLiveBoot,
                    candidate_id: cand.candidate_id.clone(),
                    phases: Phases {
                        build: PhaseState::Pass,
                        vm_live_boot: PhaseState::FailConfirmed,
                        vm_install_e2e: PhaseState::Pending,
                    },
                    next: expected_next.to_string(),
                    changed_count: n_changed,
                    rerun_required: false,
                    flaked: false,
                })
            );

            // Third record on that phase -> Refused(PhaseFailed).
            let res = apply_phase(
                &mut cand,
                pass_phase_input(Phase::VmLiveBoot, Some(sha.clone()), &hold_sha),
            )
            .expect("third record");
            match res {
                RecordPhaseResult::Refused {
                    reason: RefusalReason::PhaseFailed,
                    ..
                } => {}
                other => panic!("expected Refused(PhaseFailed), got {other:?}"),
            }
        }
    }

    #[test]
    fn unconfirmed_fail_then_pass_marks_flaked_true_and_keeps_both_history_records() {
        let (mut cand, sha, hold_sha) = make_candidate_with_changed(1);

        apply_phase(
            &mut cand,
            fail_phase_input(
                Phase::Build,
                None,
                &hold_sha,
                Some("component"),
                Some("stage3-fail"),
            ),
        )
        .expect("build fail 1");

        let res = apply_phase(
            &mut cand,
            pass_phase_input(Phase::Build, Some(sha.clone()), &hold_sha),
        )
        .expect("build pass after flake");
        match res {
            RecordPhaseResult::Recorded(outcome) => {
                assert_eq!(outcome.status, "PASS");
                assert!(outcome.flaked);
                assert!(!outcome.rerun_required);
                assert_eq!(outcome.next, "vm-live-boot");
            }
            other => panic!("expected Recorded, got {other:?}"),
        }

        assert_eq!(cand.history.len(), 2);
        assert_eq!(cand.history[0].state, PhaseState::FailUnconfirmed);
        assert!(!cand.history[0].flaked);
        assert_eq!(cand.history[1].state, PhaseState::Pass);
        assert!(cand.history[1].flaked);

        // Complete remaining phases to verify next == "promote" on vm-install-e2e.
        apply_phase(
            &mut cand,
            pass_phase_input(Phase::VmLiveBoot, Some(sha.clone()), &hold_sha),
        )
        .expect("vm-live-boot pass");
        let final_res = apply_phase(
            &mut cand,
            pass_phase_input(Phase::VmInstallE2e, Some(sha), &hold_sha),
        )
        .expect("vm-install-e2e pass");
        match final_res {
            RecordPhaseResult::Recorded(outcome) => {
                assert_eq!(outcome.next, "promote");
                assert!(!outcome.flaked);
            }
            other => panic!("expected Recorded, got {other:?}"),
        }
    }

    #[test]
    fn out_of_order_refusals_before_prerequisite_or_after_later_phase() {
        let (mut cand, sha, hold_sha) = make_candidate_with_changed(1);

        // vm-install-e2e before vm-live-boot passed -> Refused(OutOfOrder).
        let res = apply_phase(
            &mut cand,
            pass_phase_input(Phase::VmInstallE2e, Some(sha.clone()), &hold_sha),
        )
        .expect("out of order");
        match res {
            RecordPhaseResult::Refused {
                reason: RefusalReason::OutOfOrder,
                detail,
            } => {
                assert!(detail.contains("build"));
                assert!(detail.contains("pending"));
            }
            other => panic!("expected OutOfOrder, got {other:?}"),
        }

        // Advance build -> Pass, vm-live-boot -> FailUnconfirmed.
        apply_phase(
            &mut cand,
            pass_phase_input(Phase::Build, Some(sha.clone()), &hold_sha),
        )
        .expect("build pass");
        apply_phase(
            &mut cand,
            fail_phase_input(
                Phase::VmLiveBoot,
                Some(sha.clone()),
                &hold_sha,
                Some("component"),
                Some("boot-fail"),
            ),
        )
        .expect("vm-live-boot fail");

        // Re-recording build after vm-live-boot has a result -> Refused(OutOfOrder).
        let res = apply_phase(
            &mut cand,
            pass_phase_input(Phase::Build, Some(sha), &hold_sha),
        )
        .expect("re-record build");
        match res {
            RecordPhaseResult::Refused {
                reason: RefusalReason::OutOfOrder,
                detail,
            } => {
                assert!(detail.contains("vm-live-boot"));
                assert!(detail.contains("fail_unconfirmed"));
            }
            other => panic!("expected OutOfOrder, got {other:?}"),
        }
    }

    #[test]
    fn identity_and_iso_mismatch_checks() {
        let (mut cand, sha, hold_sha) = make_candidate_with_changed(1);
        let wrong_sha = "b".repeat(64);

        // Build fail with a different iso sha -> accepted.
        let res = apply_phase(
            &mut cand,
            fail_phase_input(
                Phase::Build,
                Some(wrong_sha.clone()),
                &hold_sha,
                Some("component"),
                Some("build-err"),
            ),
        )
        .expect("build fail different iso sha");
        assert!(matches!(res, RecordPhaseResult::Recorded(_)));

        // Build pass with a different iso sha -> IsoMismatch.
        let res = apply_phase(
            &mut cand,
            pass_phase_input(Phase::Build, Some(wrong_sha.clone()), &hold_sha),
        )
        .expect("build pass different iso sha");
        assert!(matches!(
            res,
            RecordPhaseResult::Refused {
                reason: RefusalReason::IsoMismatch,
                ..
            }
        ));

        // Build pass with None iso sha -> accepted.
        let res = apply_phase(&mut cand, pass_phase_input(Phase::Build, None, &hold_sha))
            .expect("build pass None iso sha");
        assert!(matches!(res, RecordPhaseResult::Recorded(_)));

        // VM phase with missing iso sha -> IsoMismatch.
        let res = apply_phase(
            &mut cand,
            pass_phase_input(Phase::VmLiveBoot, None, &hold_sha),
        )
        .expect("vm phase missing iso sha");
        assert!(matches!(
            res,
            RecordPhaseResult::Refused {
                reason: RefusalReason::IsoMismatch,
                ..
            }
        ));

        // VM phase with different iso sha -> IsoMismatch.
        let res = apply_phase(
            &mut cand,
            pass_phase_input(Phase::VmLiveBoot, Some(wrong_sha), &hold_sha),
        )
        .expect("vm phase wrong iso sha");
        assert!(matches!(
            res,
            RecordPhaseResult::Refused {
                reason: RefusalReason::IsoMismatch,
                ..
            }
        ));

        // Different hold_file_sha256 -> InputsMismatch.
        let res = apply_phase(
            &mut cand,
            pass_phase_input(Phase::VmLiveBoot, Some(sha.clone()), "wrong-hold-sha"),
        )
        .expect("wrong hold_file_sha256");
        match res {
            RecordPhaseResult::Refused {
                reason: RefusalReason::InputsMismatch,
                detail,
            } => assert!(detail.contains("hold_file_sha256")),
            other => panic!("expected InputsMismatch, got {other:?}"),
        }

        // Different snapshot -> InputsMismatch.
        let mut bad_snap = pass_phase_input(Phase::VmLiveBoot, Some(sha.clone()), &hold_sha);
        bad_snap.snapshot = "20261001T000000Z".to_string();
        match apply_phase(&mut cand, bad_snap).expect("wrong snapshot") {
            RecordPhaseResult::Refused {
                reason: RefusalReason::InputsMismatch,
                detail,
            } => assert!(detail.contains("snapshot")),
            other => panic!("expected InputsMismatch, got {other:?}"),
        }

        // Different repo_head -> InputsMismatch.
        let mut bad_head = pass_phase_input(Phase::VmLiveBoot, Some(sha), &hold_sha);
        bad_head.repo_head = "00001111".to_string();
        match apply_phase(&mut cand, bad_head).expect("wrong repo_head") {
            RecordPhaseResult::Refused {
                reason: RefusalReason::InputsMismatch,
                detail,
            } => assert!(detail.contains("repo_head")),
            other => panic!("expected InputsMismatch, got {other:?}"),
        }
    }

    #[test]
    fn apply_phase_rejects_infra_class_and_errors_on_missing_class_or_rule() {
        let (mut cand, sha, hold_sha) = make_candidate_with_changed(1);
        let original = cand.clone();

        // class = "infra" -> RejectedClass, candidate unchanged.
        let res = apply_phase(
            &mut cand,
            fail_phase_input(
                Phase::Build,
                Some(sha.clone()),
                &hold_sha,
                Some("infra"),
                Some("network-timeout"),
            ),
        )
        .expect("infra fail");
        assert_eq!(
            res,
            RecordPhaseResult::RejectedClass {
                class: "infra".to_string()
            }
        );
        assert_eq!(cand, original);

        // Fail with no class -> ValidationError.
        let err = apply_phase(
            &mut cand,
            fail_phase_input(
                Phase::Build,
                Some(sha.clone()),
                &hold_sha,
                None,
                Some("rule"),
            ),
        )
        .expect_err("missing class must error");
        assert!(matches!(err, TrackerError::ValidationError(_)));

        // Fail with blank classifier_rule -> ValidationError.
        let err = apply_phase(
            &mut cand,
            fail_phase_input(
                Phase::Build,
                Some(sha),
                &hold_sha,
                Some("component"),
                Some("   "),
            ),
        )
        .expect_err("blank rule must error");
        assert!(matches!(err, TrackerError::ValidationError(_)));
    }

    #[test]
    fn record_phase_handles_missing_file_and_leaves_file_identical_on_refusal() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("release/candidate.toml");

        let (cand, sha, hold_sha) = make_candidate_with_changed(1);

        // No candidate file -> Refused(NoCandidate) and no file created.
        let res = record_phase(
            &path,
            pass_phase_input(Phase::Build, Some(sha.clone()), &hold_sha),
        )
        .expect("record_phase missing");
        assert!(matches!(
            res,
            RecordPhaseResult::Refused {
                reason: RefusalReason::NoCandidate,
                ..
            }
        ));
        assert!(!path.exists());

        // Write candidate, then trigger a Refused result -> file stays byte-identical.
        write_candidate(&path, &cand).expect("write_candidate");
        let bytes_before = fs::read(&path).expect("read before");

        let refused = record_phase(
            &path,
            pass_phase_input(Phase::VmLiveBoot, Some(sha.clone()), &hold_sha),
        )
        .expect("record_phase out of order");
        assert!(matches!(
            refused,
            RecordPhaseResult::Refused {
                reason: RefusalReason::OutOfOrder,
                ..
            }
        ));
        let bytes_after = fs::read(&path).expect("read after");
        assert_eq!(bytes_before, bytes_after);

        // Now record a valid phase -> file updates.
        let recorded = record_phase(&path, pass_phase_input(Phase::Build, Some(sha), &hold_sha))
            .expect("record_phase build pass");
        assert!(matches!(recorded, RecordPhaseResult::Recorded(_)));
        let loaded = read_candidate(&path).expect("read").expect("Some");
        assert_eq!(loaded.phases.build, PhaseState::Pass);
    }
}
