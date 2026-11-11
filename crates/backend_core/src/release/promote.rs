use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use chrono::{DateTime, SecondsFormat, Utc};
use sha2::{Digest, Sha256};

use crate::error::{Result, TrackerError};
use crate::release::approvals::Approvals;
use crate::release::candidate::{Candidate, HoldInForce, PhaseState};
use crate::release::component::golden_from_inventory;
use crate::release::config::Components;
use crate::release::golden::{render_golden, write_golden, Golden, Origin};
use crate::release::holds::ledger_holds;
use crate::release::iso::ExtractedIso;
use crate::release::ledger::{Ledger, Phase};
use crate::release::manifest::Manifest;
use crate::release::records::{
    record_file_name, write_record, RecordComponent, ReleaseRecord, RELEASE_RECORD_SCHEMA_VERSION,
};
use crate::release::variance::{evaluate_variance, VarianceReport};
use crate::release::Violation;

pub const PROMOTED_BY: &str = "neb release promote";

#[derive(Debug, Clone)]
pub struct PromoteInput<'a> {
    pub candidate: Option<&'a Candidate>,
    pub extracted: &'a ExtractedIso,
    pub iso_sha256: &'a str,
    pub components: &'a Components,
    pub manifest: &'a Manifest,
    pub golden: Option<&'a Golden>,
    pub approvals: &'a Approvals,
    pub ledger: &'a Ledger,
    pub records: &'a [(PathBuf, ReleaseRecord)],
    pub active_bisect_candidate: Option<&'a str>,
    pub bisect_rounds: u32,
    pub cache_deb_names: &'a [String],
    pub now: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotReadyReason {
    NoCandidate,
    IsoMismatch,
    PhasesIncomplete,
    BisectActive,
}

impl NotReadyReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoCandidate => "no_candidate",
            Self::IsoMismatch => "iso_mismatch",
            Self::PhasesIncomplete => "phases_incomplete",
            Self::BisectActive => "bisect_active",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PromotePlan {
    pub golden: Golden,
    pub golden_text: String,
    pub golden_sha256: String,
    pub record: ReleaseRecord,
    pub record_file_name: String,
    pub cleared_holds: Vec<HoldInForce>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreparedPromote {
    Ready(Box<PromotePlan>),
    NotReady {
        reason: NotReadyReason,
        detail: String,
    },
    VarianceLock {
        violations: Vec<Violation>,
        report: VarianceReport,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Promoted {
    pub record_path: PathBuf,
    pub record_sha256: String,
    pub golden_sha256: String,
    pub cleared_holds: Vec<HoldInForce>,
}

fn strip_leading_epoch(v: &str) -> &str {
    if let Some((epoch_str, rest)) = v.split_once(':') {
        if !epoch_str.is_empty() && epoch_str.bytes().all(|b| b.is_ascii_digit()) {
            return rest;
        }
    }
    v
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

/// Maps third-party components in `golden` to matching `.deb` paths in `artifact_cache`.
pub fn third_party_cache_from_listing(
    golden: &Golden,
    artifact_cache: &str,
    deb_names: &[String],
) -> BTreeMap<String, Vec<String>> {
    let cache_dir = artifact_cache.trim_end_matches('/');
    if cache_dir.is_empty() {
        return BTreeMap::new();
    }

    let mut out = BTreeMap::new();
    for (comp_id, comp) in &golden.components {
        if comp.origin != Origin::ThirdParty {
            continue;
        }
        let mut matches = Vec::new();
        for (binary, version) in &comp.binaries {
            let stripped = strip_leading_epoch(version);
            let prefix_stripped = format!("{binary}_{stripped}_");
            let escaped = version.replace(':', "%3a");
            let prefix_escaped = format!("{binary}_{escaped}_");

            for deb in deb_names {
                if deb.ends_with(".deb")
                    && (deb.starts_with(&prefix_stripped) || deb.starts_with(&prefix_escaped))
                {
                    matches.push(format!("{cache_dir}/{deb}"));
                }
            }
        }
        if !matches.is_empty() {
            matches.sort();
            matches.dedup();
            out.insert(comp_id.clone(), matches);
        }
    }

    out
}

/// Validates all promote preconditions and builds the new golden snapshot and release record
/// in memory without writing to disk.
pub fn prepare_promote(input: &PromoteInput) -> Result<PreparedPromote> {
    if !input.extracted.identity.is_production() {
        let layers = if input.extracted.identity.extra_live_layers.is_empty() {
            "none".to_string()
        } else {
            input.extracted.identity.extra_live_layers.join(", ")
        };
        return Err(TrackerError::ValidationError(format!(
            "{} is not a production ISO (extra live layers: {}; squashfs sha256: '{}'); promote takes the production ISO",
            input.extracted.inventory.iso.path,
            layers,
            input.extracted.identity.squashfs_sha256
        )));
    }

    let Some(c) = input.candidate else {
        return Ok(PreparedPromote::NotReady {
            reason: NotReadyReason::NoCandidate,
            detail: "no active candidate (.tracker/release/candidate.toml does not exist)"
                .to_string(),
        });
    };

    if input.iso_sha256 != c.iso_sha256 {
        return Ok(PreparedPromote::NotReady {
            reason: NotReadyReason::IsoMismatch,
            detail: format!(
                "ISO sha256 '{}' does not match candidate '{}'",
                input.iso_sha256, c.iso_sha256
            ),
        });
    }

    if input.extracted.identity.squashfs_sha256 != c.squashfs_sha256 {
        return Ok(PreparedPromote::NotReady {
            reason: NotReadyReason::IsoMismatch,
            detail: format!(
                "production squashfs sha256 '{}' does not match candidate '{}'",
                input.extracted.identity.squashfs_sha256, c.squashfs_sha256
            ),
        });
    }

    for phase in Phase::ALL {
        let state = c.phases.get(phase);
        if state != PhaseState::Pass {
            return Ok(PreparedPromote::NotReady {
                reason: NotReadyReason::PhasesIncomplete,
                detail: format!(
                    "phase '{}' is '{}'; promote needs all three phases to pass",
                    phase.as_str(),
                    state.as_str()
                ),
            });
        }
    }

    if let Some(bisect_id) = input.active_bisect_candidate {
        if bisect_id != c.candidate_id {
            return Ok(PreparedPromote::NotReady {
                reason: NotReadyReason::BisectActive,
                detail: format!(
                    "active bisect is for candidate '{bisect_id}', not '{}'",
                    c.candidate_id
                ),
            });
        }
    }

    let new_golden = golden_from_inventory(&input.extracted.inventory, input.components);
    let report = evaluate_variance(&new_golden, input.manifest, input.golden, input.approvals)?;
    if !report.violations.is_empty() {
        return Ok(PreparedPromote::VarianceLock {
            violations: report.violations.clone(),
            report,
        });
    }

    let version = new_golden
        .os_release
        .get("VERSION_ID")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            TrackerError::ValidationError(
                "ISO os-release has no VERSION_ID; cannot name the release record".to_string(),
            )
        })?
        .to_string();

    let record_file_name = record_file_name(&version, &input.now)?;
    let golden_text = render_golden(&new_golden)?;
    let golden_sha256 = sha256_hex(golden_text.as_bytes());

    let phases = Phase::ALL
        .into_iter()
        .map(|p| (p.as_str().to_string(), c.phases.get(p).as_str().to_string()))
        .collect();

    let components = new_golden
        .components
        .iter()
        .map(|(id, gcomp)| {
            (
                id.clone(),
                RecordComponent {
                    version: gcomp.version.clone(),
                    origin: gcomp.origin,
                    binaries: gcomp.binaries.clone(),
                },
            )
        })
        .collect();

    let third_party_cache =
        third_party_cache_from_listing(&new_golden, &c.artifact_cache, input.cache_deb_names);

    let candidate_table = match toml::Value::try_from(c)? {
        toml::Value::Table(table) => table,
        _ => {
            return Err(TrackerError::ValidationError(
                "candidate did not serialize to a TOML table".to_string(),
            ));
        }
    };

    let record = ReleaseRecord {
        schema_version: RELEASE_RECORD_SCHEMA_VERSION,
        version,
        promoted_at: input.now.to_rfc3339_opts(SecondsFormat::Secs, true),
        promoted_by: PROMOTED_BY.to_string(),
        reason: String::new(),
        snapshot: c.snapshot.clone(),
        iso_sha256: c.iso_sha256.clone(),
        golden_sha256: golden_sha256.clone(),
        hold_file_sha256: c.hold_file_sha256.clone(),
        candidate_id: c.candidate_id.clone(),
        bisect_rounds: input.bisect_rounds,
        phases,
        components,
        third_party_cache,
        candidate: Some(candidate_table),
    };

    let remaining_holds: BTreeSet<String> = ledger_holds(input.ledger, &new_golden, input.records)
        .into_iter()
        .map(|h| h.component)
        .collect();

    let cleared_holds = c
        .holds_in_force
        .iter()
        .filter(|h| !remaining_holds.contains(&h.component))
        .cloned()
        .collect();

    Ok(PreparedPromote::Ready(Box::new(PromotePlan {
        golden: new_golden,
        golden_text,
        golden_sha256,
        record,
        record_file_name,
        cleared_holds,
    })))
}

fn rollback_after_golden(
    old_golden_text: Option<&str>,
    golden_path: &Path,
    record_path: &Path,
    original: TrackerError,
) -> TrackerError {
    let golden_res: Result<()> = match old_golden_text {
        Some(old) => crate::storage::atomic_write_str(golden_path, old),
        None => fs::remove_file(golden_path).map_err(|source| TrackerError::Io {
            path: golden_path.to_path_buf(),
            source,
        }),
    };
    let record_res: Result<()> = fs::remove_file(record_path).map_err(|source| TrackerError::Io {
        path: record_path.to_path_buf(),
        source,
    });

    if let Err(rollback_err) = golden_res.and(record_res) {
        TrackerError::ValidationError(format!(
            "promote failed: {original}; rollback also failed: {rollback_err}; check {} and {}",
            golden_path.display(),
            record_path.display()
        ))
    } else {
        original
    }
}

/// Commits a prepared promote plan across the release record, `golden.toml`, and `candidate.toml`
/// with compensating rollback on failure.
///
/// The caller must hold `TrackerStorage::with_write_lock` across [`prepare_promote`] and this
/// call. An active bisect state file is not touched.
pub fn commit_promote(
    plan: &PromotePlan,
    golden_path: &Path,
    releases_dir: &Path,
    candidate_path: &Path,
) -> Result<Promoted> {
    let old_golden_text = match fs::read_to_string(golden_path) {
        Ok(text) => Some(text),
        Err(err) if err.kind() == ErrorKind::NotFound => None,
        Err(source) => {
            return Err(TrackerError::Io {
                path: golden_path.to_path_buf(),
                source,
            });
        }
    };

    let record_path = releases_dir.join(&plan.record_file_name);
    let record_sha256 = write_record(&record_path, &plan.record)?;

    let written_golden_sha = match write_golden(golden_path, &plan.golden) {
        Ok(sha) => sha,
        Err(original) => {
            if let Err(rm_err) = fs::remove_file(&record_path) {
                let rollback_err = TrackerError::Io {
                    path: record_path.clone(),
                    source: rm_err,
                };
                return Err(TrackerError::ValidationError(format!(
                    "promote failed: {original}; rollback also failed: {rollback_err}; check {} and {}",
                    golden_path.display(),
                    record_path.display()
                )));
            }
            return Err(original);
        }
    };

    if written_golden_sha != plan.golden_sha256 {
        let original = TrackerError::ValidationError(
            "golden.toml render changed between prepare and commit".to_string(),
        );
        return Err(rollback_after_golden(
            old_golden_text.as_deref(),
            golden_path,
            &record_path,
            original,
        ));
    }

    if let Err(source) = fs::remove_file(candidate_path) {
        let original = TrackerError::Io {
            path: candidate_path.to_path_buf(),
            source,
        };
        return Err(rollback_after_golden(
            old_golden_text.as_deref(),
            golden_path,
            &record_path,
            original,
        ));
    }

    Ok(Promoted {
        record_path,
        record_sha256,
        golden_sha256: plan.golden_sha256.clone(),
        cleared_holds: plan.cleared_holds.clone(),
    })
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tempfile::tempdir;

    use super::*;
    use crate::release::approvals::Approval;
    use crate::release::candidate::{
        write_candidate, ChangedComponent, Phases, CANDIDATE_SCHEMA_VERSION,
    };
    use crate::release::golden::{read_golden, GoldenComponent, GOLDEN_SCHEMA_VERSION};
    use crate::release::holds::HoldReason;
    use crate::release::inventory::{
        BootInfo, CalamaresModules, InstalledPackage, IsoInventory, IsoMeta,
    };
    use crate::release::iso::IsoIdentity;
    use crate::release::ledger::{Failure, FailureSource, COMPONENT_CLASS};
    use crate::release::records::read_record;
    use crate::release::rootfs::sha256_file;

    fn sample_extracted_iso(version_id: Option<&str>, pkg_ver: &str) -> ExtractedIso {
        let mut os_release = BTreeMap::new();
        os_release.insert("ID".to_string(), "debian".to_string());
        if let Some(vid) = version_id {
            os_release.insert("VERSION_ID".to_string(), vid.to_string());
        }

        let mut packages = BTreeMap::new();
        packages.insert(
            "example-pkg".to_string(),
            InstalledPackage {
                name: "example-pkg".to_string(),
                version: pkg_ver.to_string(),
                source: "example-pkg".to_string(),
                source_version: None,
                arch: Some("amd64".to_string()),
                depends: Vec::new(),
            },
        );
        packages.insert(
            "example-kernel-tool".to_string(),
            InstalledPackage {
                name: "example-kernel-tool".to_string(),
                version: "1:2.0-1".to_string(),
                source: "example-kernel-tool".to_string(),
                source_version: None,
                arch: Some("amd64".to_string()),
                depends: Vec::new(),
            },
        );

        ExtractedIso {
            inventory: IsoInventory {
                iso: IsoMeta {
                    path: "/tmp/neb-test/novos.iso".to_string(),
                    size_bytes: 4096,
                    vmlinuz_sha256: "1".repeat(64),
                    initrd_sha256: "2".repeat(64),
                    extracted_at: "2026-11-10T12:00:00Z".to_string(),
                },
                os_release,
                packages,
                custom_binaries: BTreeMap::new(),
                calamares_modules: CalamaresModules {
                    configs: Vec::new(),
                    custom_modules: Vec::new(),
                    presence: BTreeMap::new(),
                },
                boot: BootInfo { files: Vec::new() },
            },
            identity: IsoIdentity {
                squashfs_sha256: "d".repeat(64),
                extra_live_layers: Vec::new(),
            },
        }
    }

    fn sample_components() -> Components {
        Components {
            kernel_lines: Vec::new(),
            third_party: vec!["example-kernel-tool".to_string()],
        }
    }

    fn sample_candidate() -> Candidate {
        Candidate {
            schema_version: CANDIDATE_SCHEMA_VERSION,
            candidate_id: "aaaaaaaaaaaa-20261110T120000Z".to_string(),
            created_at: "2026-11-10T12:00:00Z".to_string(),
            iso_path: "/tmp/neb-test/novos.iso".to_string(),
            iso_sha256: "a".repeat(64),
            squashfs_sha256: "d".repeat(64),
            snapshot: "20261101T000000Z".to_string(),
            artifact_cache: "artifact-cache".to_string(),
            hold_file: ".tracker/release/holds.toml".to_string(),
            hold_file_sha256: "c".repeat(64),
            repo_head: "b".repeat(40),
            unchanged_count: 1,
            phases: Phases {
                build: PhaseState::Pass,
                vm_live_boot: PhaseState::Pass,
                vm_install_e2e: PhaseState::Pass,
            },
            changed: vec![ChangedComponent {
                component: "example-pkg".to_string(),
                origin: Origin::Debian,
                from_version: Some("1.0-1".to_string()),
                to_version: "1.2-1".to_string(),
                kernel_line: false,
            }],
            holds_in_force: Vec::new(),
            history: Vec::new(),
        }
    }

    fn empty_golden() -> Golden {
        Golden {
            schema_version: GOLDEN_SCHEMA_VERSION,
            iso: IsoMeta {
                path: "/tmp/neb-test/old.iso".to_string(),
                size_bytes: 2048,
                vmlinuz_sha256: "1".repeat(64),
                initrd_sha256: "2".repeat(64),
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
            components: BTreeMap::new(),
        }
    }

    fn golden_comp(version: &str, origin: Origin, bins: &[(&str, &str)]) -> GoldenComponent {
        GoldenComponent {
            version: version.to_string(),
            origin,
            kernel_line: false,
            source_unknown: false,
            binaries: bins
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        }
    }

    #[test]
    fn third_party_cache_matches_epoch_stripped_and_escaped_deb_names() {
        let mut g = empty_golden();
        g.components.insert(
            "example-tool".to_string(),
            golden_comp("1:2.0-1", Origin::ThirdParty, &[("tool", "1:2.0-1")]),
        );
        g.components.insert(
            "example-pkg".to_string(),
            golden_comp("1:2.0-1", Origin::Debian, &[("tool", "1:2.0-1")]),
        );
        g.components.insert(
            "unmatched-tp".to_string(),
            golden_comp("3.0-1", Origin::ThirdParty, &[("other", "3.0-1")]),
        );

        let deb_names = vec![
            "tool_2.0-1_amd64.deb".to_string(),
            "tool_1%3a2.0-1_amd64.deb".to_string(),
            "tool_2.0-1_amd64.deb".to_string(), // duplicate to verify dedup
            "tool_2.0-10_amd64.deb".to_string(),
            "toolbox_2.0-1_amd64.deb".to_string(),
            "tool_2.0-1_amd64.txt".to_string(),
        ];

        let map = third_party_cache_from_listing(&g, "artifact-cache/", &deb_names);
        assert_eq!(map.len(), 1);
        assert_eq!(
            map.get("example-tool"),
            Some(&vec![
                "artifact-cache/tool_1%3a2.0-1_amd64.deb".to_string(),
                "artifact-cache/tool_2.0-1_amd64.deb".to_string(),
            ])
        );
        assert!(!map.contains_key("example-pkg"));
        assert!(!map.contains_key("unmatched-tp"));

        let empty_map = third_party_cache_from_listing(&g, "", &deb_names);
        assert!(empty_map.is_empty());

        let slash_only = third_party_cache_from_listing(&g, "///", &deb_names);
        assert!(slash_only.is_empty());
    }

    #[test]
    fn prepare_promote_happy_path_builds_golden_and_record() {
        let extracted = sample_extracted_iso(Some("2026.11"), "1.2-1");
        let candidate = sample_candidate();
        let components = sample_components();
        let manifest = Manifest::default();
        let approvals = Approvals::default();
        let ledger = Ledger::default();
        let records = Vec::new();
        let deb_names = vec!["example-kernel-tool_2.0-1_amd64.deb".to_string()];
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 14, 30, 0).unwrap();
        let iso_sha = "a".repeat(64);

        let input = PromoteInput {
            candidate: Some(&candidate),
            extracted: &extracted,
            iso_sha256: &iso_sha,
            components: &components,
            manifest: &manifest,
            golden: None,
            approvals: &approvals,
            ledger: &ledger,
            records: &records,
            active_bisect_candidate: None,
            bisect_rounds: 2,
            cache_deb_names: &deb_names,
            now,
        };

        let prepared = prepare_promote(&input).expect("prepare_promote");
        let PreparedPromote::Ready(plan) = prepared else {
            panic!("expected Ready, got {prepared:?}");
        };

        assert_eq!(plan.record.version, "2026.11");
        assert_eq!(plan.record_file_name, "2026.11-20261110T143000Z.toml");
        assert_eq!(plan.record.promoted_by, PROMOTED_BY);
        assert_eq!(plan.record.promoted_at, "2026-11-10T14:30:00Z");
        assert_eq!(plan.record.bisect_rounds, 2);
        assert_eq!(
            plan.record.phases.get("build").map(String::as_str),
            Some("pass")
        );
        assert_eq!(
            plan.record.phases.get("vm-live-boot").map(String::as_str),
            Some("pass")
        );
        assert_eq!(
            plan.record.phases.get("vm-install-e2e").map(String::as_str),
            Some("pass")
        );

        let expected_golden_text = render_golden(&plan.golden).expect("render_golden");
        assert_eq!(plan.golden_text, expected_golden_text);
        assert_eq!(
            plan.golden_sha256,
            sha256_hex(expected_golden_text.as_bytes())
        );
        assert_eq!(plan.record.golden_sha256, plan.golden_sha256);

        let archived_cand = plan.record.candidate.as_ref().expect("archived candidate");
        assert_eq!(
            archived_cand
                .get("squashfs_sha256")
                .and_then(toml::Value::as_str),
            Some(candidate.squashfs_sha256.as_str())
        );

        let pkg_comp = plan.record.components.get("example-pkg").expect("pkg");
        assert_eq!(pkg_comp.version, "1.2-1");
        assert_eq!(pkg_comp.origin, Origin::Debian);

        let tp_comp = plan
            .record
            .components
            .get("example-kernel-tool")
            .expect("tp");
        assert_eq!(tp_comp.version, "1:2.0-1");
        assert_eq!(tp_comp.origin, Origin::ThirdParty);
        assert_eq!(
            plan.record.third_party_cache.get("example-kernel-tool"),
            Some(&vec![
                "artifact-cache/example-kernel-tool_2.0-1_amd64.deb".to_string()
            ])
        );
    }

    #[test]
    fn prepare_promote_refuses_test_overlay_or_empty_squashfs_sha() {
        let mut extracted_overlay = sample_extracted_iso(Some("2026.11"), "1.2-1");
        extracted_overlay.identity.extra_live_layers =
            vec!["/live/zz-novos-test.squashfs".to_string()];
        let candidate = sample_candidate();
        let components = sample_components();
        let manifest = Manifest::default();
        let approvals = Approvals::default();
        let ledger = Ledger::default();
        let records = Vec::new();
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 14, 30, 0).unwrap();
        let iso_sha = "a".repeat(64);

        let input = PromoteInput {
            candidate: Some(&candidate),
            extracted: &extracted_overlay,
            iso_sha256: &iso_sha,
            components: &components,
            manifest: &manifest,
            golden: None,
            approvals: &approvals,
            ledger: &ledger,
            records: &records,
            active_bisect_candidate: None,
            bisect_rounds: 0,
            cache_deb_names: &[],
            now,
        };

        let err = prepare_promote(&input).expect_err("test ISO must be rejected");
        assert!(err.to_string().contains("/live/zz-novos-test.squashfs"));

        let mut extracted_empty_sha = sample_extracted_iso(Some("2026.11"), "1.2-1");
        extracted_empty_sha.identity.squashfs_sha256.clear();
        let input_empty_sha = PromoteInput {
            extracted: &extracted_empty_sha,
            ..input
        };
        let err_empty = prepare_promote(&input_empty_sha).expect_err("empty squashfs sha rejected");
        assert!(err_empty.to_string().contains("not a production ISO"));
    }

    #[test]
    fn prepare_promote_precondition_checks_return_not_ready() {
        let extracted = sample_extracted_iso(Some("2026.11"), "1.2-1");
        let candidate = sample_candidate();
        let components = sample_components();
        let manifest = Manifest::default();
        let approvals = Approvals::default();
        let ledger = Ledger::default();
        let records = Vec::new();
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 14, 30, 0).unwrap();
        let iso_sha = "a".repeat(64);

        let base_input = PromoteInput {
            candidate: None,
            extracted: &extracted,
            iso_sha256: &iso_sha,
            components: &components,
            manifest: &manifest,
            golden: None,
            approvals: &approvals,
            ledger: &ledger,
            records: &records,
            active_bisect_candidate: None,
            bisect_rounds: 0,
            cache_deb_names: &[],
            now,
        };

        // (b) No candidate
        match prepare_promote(&base_input).expect("no candidate") {
            PreparedPromote::NotReady { reason, .. } => {
                assert_eq!(reason, NotReadyReason::NoCandidate);
                assert_eq!(reason.as_str(), "no_candidate");
            }
            other => panic!("expected NotReady(NoCandidate), got {other:?}"),
        }

        // (c) ISO sha mismatch
        let wrong_sha = "f".repeat(64);
        let iso_mismatch_input = PromoteInput {
            candidate: Some(&candidate),
            iso_sha256: &wrong_sha,
            ..base_input.clone()
        };
        match prepare_promote(&iso_mismatch_input).expect("iso mismatch") {
            PreparedPromote::NotReady { reason, detail } => {
                assert_eq!(reason, NotReadyReason::IsoMismatch);
                assert_eq!(reason.as_str(), "iso_mismatch");
                assert!(detail.contains(&wrong_sha));
                assert!(detail.contains(&candidate.iso_sha256));
            }
            other => panic!("expected NotReady(IsoMismatch), got {other:?}"),
        }

        // (c) squashfs sha mismatch
        let mut wrong_sq_cand = candidate.clone();
        wrong_sq_cand.squashfs_sha256 = "e".repeat(64);
        let sq_mismatch_input = PromoteInput {
            candidate: Some(&wrong_sq_cand),
            ..base_input.clone()
        };
        match prepare_promote(&sq_mismatch_input).expect("squashfs mismatch") {
            PreparedPromote::NotReady { reason, detail } => {
                assert_eq!(reason, NotReadyReason::IsoMismatch);
                assert!(detail.contains(&wrong_sq_cand.squashfs_sha256));
            }
            other => panic!("expected NotReady(IsoMismatch), got {other:?}"),
        }

        // (d) vm-install-e2e Pending
        let mut incomplete_cand = candidate.clone();
        incomplete_cand.phases.vm_install_e2e = PhaseState::Pending;
        let incomplete_input = PromoteInput {
            candidate: Some(&incomplete_cand),
            ..base_input.clone()
        };
        match prepare_promote(&incomplete_input).expect("incomplete") {
            PreparedPromote::NotReady { reason, detail } => {
                assert_eq!(reason, NotReadyReason::PhasesIncomplete);
                assert_eq!(reason.as_str(), "phases_incomplete");
                assert!(detail.contains("vm-install-e2e"));
                assert!(detail.contains("pending"));
            }
            other => panic!("expected NotReady(PhasesIncomplete), got {other:?}"),
        }

        // (e) active bisect for another candidate -> NotReady(BisectActive)
        let bisect_other_input = PromoteInput {
            candidate: Some(&candidate),
            active_bisect_candidate: Some("other-cand-id"),
            ..base_input.clone()
        };
        match prepare_promote(&bisect_other_input).expect("bisect other") {
            PreparedPromote::NotReady { reason, detail } => {
                assert_eq!(reason, NotReadyReason::BisectActive);
                assert_eq!(reason.as_str(), "bisect_active");
                assert!(detail.contains("other-cand-id"));
            }
            other => panic!("expected NotReady(BisectActive), got {other:?}"),
        }

        // (e) active bisect for THIS candidate -> Ready
        let bisect_same_input = PromoteInput {
            candidate: Some(&candidate),
            active_bisect_candidate: Some(&candidate.candidate_id),
            ..base_input
        };
        assert!(matches!(
            prepare_promote(&bisect_same_input).expect("bisect same"),
            PreparedPromote::Ready(_)
        ));
    }

    #[test]
    fn prepare_promote_variance_lock_and_missing_version_id() {
        let extracted = sample_extracted_iso(Some("2026.11"), "1.2-1");
        let candidate = sample_candidate();
        let components = sample_components();
        let manifest = Manifest::default();
        let approvals = Approvals::default();
        let ledger = Ledger::default();
        let records = Vec::new();
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 14, 30, 0).unwrap();
        let iso_sha = "a".repeat(64);

        let mut old_golden = empty_golden();
        old_golden.components.insert(
            "removed-pkg".to_string(),
            golden_comp("1.0-1", Origin::Debian, &[("removed-pkg", "1.0-1")]),
        );

        let input_var = PromoteInput {
            candidate: Some(&candidate),
            extracted: &extracted,
            iso_sha256: &iso_sha,
            components: &components,
            manifest: &manifest,
            golden: Some(&old_golden),
            approvals: &approvals,
            ledger: &ledger,
            records: &records,
            active_bisect_candidate: None,
            bisect_rounds: 0,
            cache_deb_names: &[],
            now,
        };

        match prepare_promote(&input_var).expect("variance lock") {
            PreparedPromote::VarianceLock { violations, .. } => {
                assert!(violations.iter().any(|v| v.item == "removed-pkg"));
            }
            other => panic!("expected VarianceLock, got {other:?}"),
        }

        // Approved removal passes variance, then check missing VERSION_ID
        let approvals_ok = Approvals {
            approvals: vec![Approval {
                item_name: "removed-pkg".to_string(),
                version: None,
                reason: "retired".to_string(),
                approved: true,
                timestamp: "2026-11-10T00:00:00Z".to_string(),
            }],
        };
        let extracted_no_ver = sample_extracted_iso(None, "1.2-1");
        let input_no_ver = PromoteInput {
            extracted: &extracted_no_ver,
            approvals: &approvals_ok,
            ..input_var
        };
        let err = prepare_promote(&input_no_ver).expect_err("missing VERSION_ID");
        assert!(err.to_string().contains("VERSION_ID"));
    }

    #[test]
    fn prepare_promote_computes_cleared_holds() {
        let components = sample_components();
        let manifest = Manifest::default();
        let approvals = Approvals::default();
        let records = Vec::new();
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 14, 30, 0).unwrap();
        let iso_sha = "a".repeat(64);

        let ledger = Ledger {
            failures: vec![Failure {
                component: "example-pkg".to_string(),
                version: "1.1-1".to_string(),
                phase: Phase::VmLiveBoot,
                class: COMPONENT_CLASS.to_string(),
                classifier_rule: "rule-1".to_string(),
                log: "boot.log".to_string(),
                recorded_at: "2026-11-09T00:00:00Z".to_string(),
                source: FailureSource::SingleChange,
                members: Vec::new(),
            }],
        };

        let mut candidate = sample_candidate();
        let hold_entry = HoldInForce {
            component: "example-pkg".to_string(),
            held_version: "1.0-1".to_string(),
            reason: HoldReason::Failure,
        };
        candidate.holds_in_force = vec![hold_entry.clone()];

        // Inventory carries 1.2-1 (newer than blocked 1.1-1) -> hold is cleared
        let extracted_newer = sample_extracted_iso(Some("2026.11"), "1.2-1");
        let input_cleared = PromoteInput {
            candidate: Some(&candidate),
            extracted: &extracted_newer,
            iso_sha256: &iso_sha,
            components: &components,
            manifest: &manifest,
            golden: None,
            approvals: &approvals,
            ledger: &ledger,
            records: &records,
            active_bisect_candidate: None,
            bisect_rounds: 0,
            cache_deb_names: &[],
            now,
        };
        let PreparedPromote::Ready(plan_cleared) =
            prepare_promote(&input_cleared).expect("prepare cleared")
        else {
            panic!("expected Ready");
        };
        assert_eq!(plan_cleared.cleared_holds, vec![hold_entry]);

        // Inventory still carries 1.0-1 (not newer than blocked 1.1-1) -> hold stays active
        let extracted_held = sample_extracted_iso(Some("2026.11"), "1.0-1");
        let input_kept = PromoteInput {
            extracted: &extracted_held,
            ..input_cleared
        };
        let PreparedPromote::Ready(plan_kept) = prepare_promote(&input_kept).expect("prepare kept")
        else {
            panic!("expected Ready");
        };
        assert!(plan_kept.cleared_holds.is_empty());
    }

    #[test]
    fn commit_promote_happy_path_writes_record_and_golden_and_removes_candidate() {
        let dir = tempdir().expect("tempdir");
        let golden_path = dir.path().join("golden.toml");
        let releases_dir = dir.path().join("releases");
        let candidate_path = dir.path().join("candidate.toml");

        let candidate = sample_candidate();
        write_candidate(&candidate_path, &candidate).expect("write candidate");

        let extracted = sample_extracted_iso(Some("2026.11"), "1.2-1");
        let components = sample_components();
        let manifest = Manifest::default();
        let approvals = Approvals::default();
        let ledger = Ledger::default();
        let records = Vec::new();
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 14, 30, 0).unwrap();
        let iso_sha = "a".repeat(64);

        let input = PromoteInput {
            candidate: Some(&candidate),
            extracted: &extracted,
            iso_sha256: &iso_sha,
            components: &components,
            manifest: &manifest,
            golden: None,
            approvals: &approvals,
            ledger: &ledger,
            records: &records,
            active_bisect_candidate: None,
            bisect_rounds: 0,
            cache_deb_names: &[],
            now,
        };

        let PreparedPromote::Ready(plan) = prepare_promote(&input).expect("prepare") else {
            panic!("expected Ready");
        };

        let promoted =
            commit_promote(&plan, &golden_path, &releases_dir, &candidate_path).expect("commit");

        assert!(promoted.record_path.is_file());
        assert!(!candidate_path.exists());

        let disk_record = read_record(&promoted.record_path).expect("read_record");
        assert_eq!(disk_record, plan.record);
        assert_eq!(
            promoted.record_sha256,
            sha256_file(&promoted.record_path).expect("sha record")
        );

        let disk_golden = read_golden(&golden_path)
            .expect("read_golden")
            .expect("golden exists");
        assert_eq!(disk_golden, plan.golden);
        assert_eq!(
            promoted.golden_sha256,
            sha256_file(&golden_path).expect("sha golden")
        );
        assert_eq!(promoted.golden_sha256, plan.golden_sha256);
    }

    #[test]
    fn commit_promote_rolls_back_on_failures() {
        let extracted = sample_extracted_iso(Some("2026.11"), "1.2-1");
        let candidate = sample_candidate();
        let components = sample_components();
        let manifest = Manifest::default();
        let approvals = Approvals::default();
        let ledger = Ledger::default();
        let records = Vec::new();
        let now = Utc.with_ymd_and_hms(2026, 11, 10, 14, 30, 0).unwrap();
        let iso_sha = "a".repeat(64);

        let input = PromoteInput {
            candidate: Some(&candidate),
            extracted: &extracted,
            iso_sha256: &iso_sha,
            components: &components,
            manifest: &manifest,
            golden: None,
            approvals: &approvals,
            ledger: &ledger,
            records: &records,
            active_bisect_candidate: None,
            bisect_rounds: 0,
            cache_deb_names: &[],
            now,
        };
        let PreparedPromote::Ready(plan) = prepare_promote(&input).expect("prepare") else {
            panic!("expected Ready");
        };

        // (1) golden_path is an existing directory -> Err, record file does not exist, candidate unchanged
        {
            let dir = tempdir().expect("tempdir");
            let golden_path = dir.path().join("golden.toml");
            let releases_dir = dir.path().join("releases");
            let candidate_path = dir.path().join("candidate.toml");

            fs::create_dir_all(&golden_path).expect("create dir at golden_path");
            write_candidate(&candidate_path, &candidate).expect("write candidate");
            let cand_before = fs::read_to_string(&candidate_path).expect("read cand");

            let res = commit_promote(&plan, &golden_path, &releases_dir, &candidate_path);
            assert!(res.is_err());
            assert!(!releases_dir.join(&plan.record_file_name).exists());
            assert_eq!(
                fs::read_to_string(&candidate_path).expect("read cand after"),
                cand_before
            );
        }

        // (1b) write_golden fails after write_record succeeded -> record file is rolled back
        #[cfg(unix)]
        {
            let dir = tempdir().expect("tempdir");
            let dangling_parent = dir.path().join("dangling_dir");
            std::os::unix::fs::symlink("missing_target", &dangling_parent).expect("symlink");
            let golden_path = dangling_parent.join("golden.toml");
            let releases_dir = dir.path().join("releases");
            let candidate_path = dir.path().join("candidate.toml");

            write_candidate(&candidate_path, &candidate).expect("write candidate");
            let cand_before = fs::read_to_string(&candidate_path).expect("read cand");

            let res = commit_promote(&plan, &golden_path, &releases_dir, &candidate_path);
            assert!(res.is_err());
            assert!(!releases_dir.join(&plan.record_file_name).exists());
            assert_eq!(
                fs::read_to_string(&candidate_path).expect("read cand after"),
                cand_before
            );
        }

        // (2) candidate_path is a directory (remove_file fails) with an old golden present ->
        // Err, golden.toml is byte-identical to the old one, record file does not exist
        {
            let dir = tempdir().expect("tempdir");
            let golden_path = dir.path().join("golden.toml");
            let releases_dir = dir.path().join("releases");
            let candidate_path = dir.path().join("candidate.toml");

            let mut old_g = empty_golden();
            old_g.components.insert(
                "example-pkg".to_string(),
                golden_comp("0.9-1", Origin::Debian, &[("example-pkg", "0.9-1")]),
            );
            old_g
                .packages
                .insert("example-pkg".to_string(), "0.9-1".to_string());
            write_golden(&golden_path, &old_g).expect("write old golden");
            let old_golden_bytes = fs::read_to_string(&golden_path).expect("read old golden");

            fs::create_dir_all(&candidate_path).expect("candidate_path as dir");

            let res = commit_promote(&plan, &golden_path, &releases_dir, &candidate_path);
            assert!(res.is_err());
            assert_eq!(
                fs::read_to_string(&golden_path).expect("golden restored"),
                old_golden_bytes
            );
            assert!(!releases_dir.join(&plan.record_file_name).exists());
        }

        // (3) with no old golden and the same candidate failure -> golden.toml does not exist afterwards
        {
            let dir = tempdir().expect("tempdir");
            let golden_path = dir.path().join("golden.toml");
            let releases_dir = dir.path().join("releases");
            let candidate_path = dir.path().join("candidate.toml");

            fs::create_dir_all(&candidate_path).expect("candidate_path as dir");

            let res = commit_promote(&plan, &golden_path, &releases_dir, &candidate_path);
            assert!(res.is_err());
            assert!(!golden_path.exists());
            assert!(!releases_dir.join(&plan.record_file_name).exists());
        }

        // (4) a record file of the same name already exists -> AlreadyExists and golden/candidate untouched
        {
            let dir = tempdir().expect("tempdir");
            let golden_path = dir.path().join("golden.toml");
            let releases_dir = dir.path().join("releases");
            let candidate_path = dir.path().join("candidate.toml");

            write_record(&releases_dir.join(&plan.record_file_name), &plan.record)
                .expect("pre-write record");
            write_candidate(&candidate_path, &candidate).expect("write candidate");
            let cand_before = fs::read_to_string(&candidate_path).expect("read cand");

            let err = commit_promote(&plan, &golden_path, &releases_dir, &candidate_path)
                .expect_err("duplicate record must fail");
            assert!(matches!(err, TrackerError::AlreadyExists { .. }));
            assert!(!golden_path.exists());
            assert_eq!(
                fs::read_to_string(&candidate_path).expect("read cand after"),
                cand_before
            );
        }

        // (5) plan.golden_sha256 mismatch after write_golden -> restores old golden, removes record, keeps candidate
        {
            let dir = tempdir().expect("tempdir");
            let golden_path = dir.path().join("golden.toml");
            let releases_dir = dir.path().join("releases");
            let candidate_path = dir.path().join("candidate.toml");

            let mut old_g = empty_golden();
            old_g.components.insert(
                "example-pkg".to_string(),
                golden_comp("0.9-1", Origin::Debian, &[("example-pkg", "0.9-1")]),
            );
            old_g
                .packages
                .insert("example-pkg".to_string(), "0.9-1".to_string());
            write_golden(&golden_path, &old_g).expect("write old golden");
            let old_golden_bytes = fs::read_to_string(&golden_path).expect("read old golden");

            write_candidate(&candidate_path, &candidate).expect("write candidate");
            let cand_before = fs::read_to_string(&candidate_path).expect("read cand");

            let mut tampered_plan = (*plan).clone();
            tampered_plan.golden_sha256 = "0".repeat(64);

            let err = commit_promote(&tampered_plan, &golden_path, &releases_dir, &candidate_path)
                .expect_err("tampered golden_sha256 must fail and roll back");
            assert!(err
                .to_string()
                .contains("golden.toml render changed between prepare and commit"));
            assert_eq!(
                fs::read_to_string(&golden_path).expect("golden restored"),
                old_golden_bytes
            );
            assert!(!releases_dir.join(&plan.record_file_name).exists());
            assert_eq!(
                fs::read_to_string(&candidate_path).expect("read cand after"),
                cand_before
            );
        }

        // (6) rollback_after_golden surfaces both original and rollback errors when a rollback step fails
        {
            let dir = tempdir().expect("tempdir");
            let golden_path = dir.path().join("missing_golden.toml");
            let record_path = dir.path().join("missing_record.toml");
            let orig = TrackerError::ValidationError("primary failure".to_string());
            let combined = rollback_after_golden(None, &golden_path, &record_path, orig);
            let msg = combined.to_string();
            assert!(msg.contains("promote failed: Validation error: primary failure"));
            assert!(msg.contains("rollback also failed:"));
            assert!(msg.contains(&golden_path.display().to_string()));
            assert!(msg.contains(&record_path.display().to_string()));
        }
    }
}
