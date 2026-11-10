use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};
use crate::release::debver;
use crate::release::golden::{Golden, GoldenComponent, Origin};
use crate::release::ledger::Ledger;
use crate::release::records::{cache_paths_for, snapshot_for, ReleaseRecord};

pub const HOLD_FILE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HoldReason {
    Failure,
    Bisect,
    LastGood,
    Pin,
}

impl HoldReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Failure => "failure",
            Self::Bisect => "bisect",
            Self::LastGood => "last-good",
            Self::Pin => "pin",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CachePath {
    One(String),
    Many(Vec<String>),
}

impl CachePath {
    pub fn from_paths(mut paths: Vec<String>) -> Self {
        match paths.len() {
            0 => Self::One(String::new()),
            1 => Self::One(paths.pop().unwrap_or_default()),
            _ => Self::Many(paths),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hold {
    pub component: String,
    pub held_version: String,
    pub origin: Origin,
    pub packages: BTreeMap<String, String>,
    pub snapshot: String,
    pub cache_path: CachePath,
    pub reason: HoldReason,
    pub blocked_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HoldFile {
    pub schema_version: u32,
    pub generated_at: String,
    pub generated_by: String,
    #[serde(default, rename = "hold")]
    pub holds: Vec<Hold>,
}

fn make_hold(
    id: &str,
    version: &str,
    origin: Origin,
    packages: BTreeMap<String, String>,
    reason: HoldReason,
    blocked_version: &str,
    records: &[(PathBuf, ReleaseRecord)],
) -> Hold {
    let (snapshot, cache_path) = match origin {
        Origin::Debian => (
            snapshot_for(records, id, version),
            CachePath::One(String::new()),
        ),
        Origin::ThirdParty => (
            String::new(),
            CachePath::from_paths(cache_paths_for(records, id, version)),
        ),
    };

    Hold {
        component: id.to_string(),
        held_version: version.to_string(),
        origin,
        packages,
        snapshot,
        cache_path,
        reason,
        blocked_version: blocked_version.to_string(),
    }
}

/// Computes active failure holds by matching `ledger` entries against `golden`.
///
/// One hold is emitted per component (with `blocked_version` set to the highest active ledger
/// version for that component). A ledger entry is active while `golden` has the component and
/// `golden`'s version is not newer than the entry's version.
pub fn ledger_holds(
    ledger: &Ledger,
    golden: &Golden,
    records: &[(PathBuf, ReleaseRecord)],
) -> Vec<Hold> {
    let mut active: BTreeMap<&str, (&GoldenComponent, &str)> = BTreeMap::new();

    for entry in &ledger.failures {
        let Some(gcomp) = golden.components.get(&entry.component) else {
            continue;
        };
        if debver::compare(&gcomp.version, &entry.version) == Ordering::Greater {
            continue;
        }
        match active.get_mut(entry.component.as_str()) {
            Some((_, highest_blocked)) => {
                if debver::compare(&entry.version, highest_blocked) == Ordering::Greater {
                    *highest_blocked = entry.version.as_str();
                }
            }
            None => {
                active.insert(entry.component.as_str(), (gcomp, entry.version.as_str()));
            }
        }
    }

    active
        .into_iter()
        .map(|(id, (gcomp, blocked_version))| {
            make_hold(
                id,
                &gcomp.version,
                gcomp.origin,
                gcomp.binaries.clone(),
                HoldReason::Failure,
                blocked_version,
                records,
            )
        })
        .collect()
}

/// Emits one `last-good` hold per component in `golden`, in component ID order.
pub fn all_last_good(golden: &Golden, records: &[(PathBuf, ReleaseRecord)]) -> Vec<Hold> {
    golden
        .components
        .iter()
        .map(|(id, gcomp)| {
            make_hold(
                id,
                &gcomp.version,
                gcomp.origin,
                gcomp.binaries.clone(),
                HoldReason::LastGood,
                "",
                records,
            )
        })
        .collect()
}

/// Emits `pin` holds for the named `components` (or all components in `record` when `None`).
///
/// Returns [`TrackerError::ValidationError`] naming every requested component that is absent
/// from `record.components`.
pub fn pins_from_release(
    record: &ReleaseRecord,
    components: Option<&[String]>,
) -> Result<Vec<Hold>> {
    let ids: Vec<&str> = match components {
        Some(names) => {
            let unique: BTreeSet<&str> = names.iter().map(String::as_str).collect();
            let missing: Vec<&str> = unique
                .iter()
                .copied()
                .filter(|id| !record.components.contains_key(*id))
                .collect();
            if !missing.is_empty() {
                return Err(TrackerError::ValidationError(format!(
                    "component(s) not found in release record '{}': {}",
                    record.version,
                    missing.join(", ")
                )));
            }
            unique.into_iter().collect()
        }
        None => record.components.keys().map(String::as_str).collect(),
    };

    let holds = ids
        .into_iter()
        .map(|id| {
            let rcomp = &record.components[id];
            let (snapshot, cache_path) = match rcomp.origin {
                Origin::Debian => (record.snapshot.clone(), CachePath::One(String::new())),
                Origin::ThirdParty => {
                    let paths = record
                        .third_party_cache
                        .get(id)
                        .cloned()
                        .unwrap_or_default();
                    (String::new(), CachePath::from_paths(paths))
                }
            };
            Hold {
                component: id.to_string(),
                held_version: rcomp.version.clone(),
                origin: rcomp.origin,
                packages: rcomp.binaries.clone(),
                snapshot,
                cache_path,
                reason: HoldReason::Pin,
                blocked_version: String::new(),
            }
        })
        .collect();

    Ok(holds)
}

pub fn hold_file(holds: Vec<Hold>, generated_at: &str, generated_by: &str) -> HoldFile {
    HoldFile {
        schema_version: HOLD_FILE_SCHEMA_VERSION,
        generated_at: generated_at.to_string(),
        generated_by: generated_by.to_string(),
        holds,
    }
}

pub fn render_hold_file(h: &HoldFile) -> Result<String> {
    let mut normalized = h.clone();
    normalized.schema_version = HOLD_FILE_SCHEMA_VERSION;
    Ok(toml::to_string(&normalized)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release::golden::GOLDEN_SCHEMA_VERSION;
    use crate::release::inventory::{BootInfo, CalamaresModules, IsoMeta};
    use crate::release::ledger::{Failure, FailureSource, Phase};
    use crate::release::records::{RecordComponent, RELEASE_RECORD_SCHEMA_VERSION};

    fn empty_golden() -> Golden {
        Golden {
            schema_version: GOLDEN_SCHEMA_VERSION,
            iso: IsoMeta {
                path: "/tmp/neb-test/test.iso".to_string(),
                size_bytes: 1024,
                vmlinuz_sha256: "a".repeat(64),
                initrd_sha256: "b".repeat(64),
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

    fn failure_entry(component: &str, version: &str, phase: Phase) -> Failure {
        Failure {
            component: component.to_string(),
            version: version.to_string(),
            phase,
            class: "component".to_string(),
            classifier_rule: "apt-breaks".to_string(),
            log: "logs/round-0/build.log".to_string(),
            recorded_at: "2026-11-08T16:46:00Z".to_string(),
            source: FailureSource::Bisect,
            members: Vec::new(),
        }
    }

    fn make_record(
        version: &str,
        promoted_at: &str,
        snapshot: &str,
        components: BTreeMap<String, RecordComponent>,
        third_party_cache: BTreeMap<String, Vec<String>>,
    ) -> ReleaseRecord {
        ReleaseRecord {
            schema_version: RELEASE_RECORD_SCHEMA_VERSION,
            version: version.to_string(),
            promoted_at: promoted_at.to_string(),
            promoted_by: "neb release promote".to_string(),
            reason: String::new(),
            snapshot: snapshot.to_string(),
            iso_sha256: "1".repeat(64),
            golden_sha256: "2".repeat(64),
            hold_file_sha256: String::new(),
            candidate_id: "111111111111-20261108T120000Z".to_string(),
            bisect_rounds: 0,
            phases: BTreeMap::new(),
            components,
            third_party_cache,
            candidate: None,
        }
    }

    #[test]
    fn ledger_holds_active_newest_snapshot_and_golden_comparison() {
        let mut golden = empty_golden();
        golden.components.insert(
            "example-pkg".to_string(),
            golden_comp(
                "2.5.2-1+2",
                Origin::Debian,
                &[("example-pkg", "2.5.2-1+2"), ("libexample1", "2.5.2-1+2")],
            ),
        );

        let ledger = Ledger {
            failures: vec![failure_entry("example-pkg", "2.5.5-1+1", Phase::Build)],
        };

        let mut old_comps = BTreeMap::new();
        old_comps.insert(
            "example-pkg".to_string(),
            RecordComponent {
                version: "2.5.2-1+2".to_string(),
                origin: Origin::Debian,
                binaries: BTreeMap::from([("example-pkg".to_string(), "2.5.2-1+2".to_string())]),
            },
        );
        let rec1 = make_record(
            "2026.10",
            "2026-11-01T10:00:00Z",
            "20261101T000000Z",
            old_comps.clone(),
            BTreeMap::new(),
        );
        let rec2 = make_record(
            "2026.11",
            "2026-11-07T10:00:00Z",
            "20261107T205437Z",
            old_comps,
            BTreeMap::new(),
        );
        let records = vec![
            (PathBuf::from("r1.toml"), rec1),
            (PathBuf::from("r2.toml"), rec2),
        ];

        let holds = ledger_holds(&ledger, &golden, &records);
        assert_eq!(
            holds,
            vec![Hold {
                component: "example-pkg".to_string(),
                held_version: "2.5.2-1+2".to_string(),
                origin: Origin::Debian,
                packages: BTreeMap::from([
                    ("example-pkg".to_string(), "2.5.2-1+2".to_string()),
                    ("libexample1".to_string(), "2.5.2-1+2".to_string()),
                ]),
                snapshot: "20261107T205437Z".to_string(),
                cache_path: CachePath::One(String::new()),
                reason: HoldReason::Failure,
                blocked_version: "2.5.5-1+1".to_string(),
            }]
        );

        // Golden equal to the ledger entry -> hold is still active.
        golden.components.get_mut("example-pkg").unwrap().version = "2.5.5-1+1".to_string();
        let equal_holds = ledger_holds(&ledger, &golden, &records);
        assert_eq!(equal_holds.len(), 1);
        assert_eq!(equal_holds[0].held_version, "2.5.5-1+1");
        assert_eq!(equal_holds[0].blocked_version, "2.5.5-1+1");

        // Golden newer than the ledger entry -> hold drops out.
        golden.components.get_mut("example-pkg").unwrap().version = "2.5.6-1".to_string();
        let newer_holds = ledger_holds(&ledger, &golden, &records);
        assert!(newer_holds.is_empty());
    }

    #[test]
    fn ledger_holds_deduplicates_component_to_highest_blocked_and_skips_absent_from_golden() {
        let mut golden = empty_golden();
        golden.components.insert(
            "example-pkg".to_string(),
            golden_comp("2.5.2-1+2", Origin::Debian, &[("example-pkg", "2.5.2-1+2")]),
        );

        let ledger = Ledger {
            failures: vec![
                failure_entry("example-pkg", "2.5.5-1", Phase::Build),
                failure_entry("example-pkg", "2.5.6-1", Phase::Build),
                failure_entry("example-pkg", "2.5.5-1", Phase::VmLiveBoot),
                failure_entry("absent-pkg", "1.0-1", Phase::Build),
            ],
        };

        let holds = ledger_holds(&ledger, &golden, &[]);
        assert_eq!(holds.len(), 1);
        assert_eq!(holds[0].component, "example-pkg");
        assert_eq!(holds[0].held_version, "2.5.2-1+2");
        assert_eq!(holds[0].blocked_version, "2.5.6-1");
        assert_eq!(holds[0].snapshot, "");
    }

    #[test]
    fn cache_path_shapes_for_third_party_and_all_last_good() {
        let mut golden = empty_golden();
        golden.components.insert(
            "debian-pkg".to_string(),
            golden_comp("1.0-1", Origin::Debian, &[("debian-pkg", "1.0-1")]),
        );
        golden.components.insert(
            "kernel-many".to_string(),
            golden_comp("7.3.3-1", Origin::ThirdParty, &[("kernel-many", "7.3.3-1")]),
        );
        golden.components.insert(
            "tool-none".to_string(),
            golden_comp("0.1.0", Origin::ThirdParty, &[("tool-none", "0.1.0")]),
        );
        golden.components.insert(
            "tool-one".to_string(),
            golden_comp("0.2.0", Origin::ThirdParty, &[("tool-one", "0.2.0")]),
        );

        let mut r_comps = BTreeMap::new();
        for (id, g) in &golden.components {
            r_comps.insert(
                id.clone(),
                RecordComponent {
                    version: g.version.clone(),
                    origin: g.origin,
                    binaries: g.binaries.clone(),
                },
            );
        }

        let mut tp_cache = BTreeMap::new();
        tp_cache.insert(
            "kernel-many".to_string(),
            vec![
                "artifact-cache/linux-image.deb".to_string(),
                "artifact-cache/linux-headers.deb".to_string(),
            ],
        );
        tp_cache.insert(
            "tool-one".to_string(),
            vec!["artifact-cache/tool-one.deb".to_string()],
        );

        let rec = make_record(
            "2026.10",
            "2026-11-08T12:00:00Z",
            "20261108T000000Z",
            r_comps,
            tp_cache,
        );
        let records = vec![(PathBuf::from("rec.toml"), rec)];

        let holds = all_last_good(&golden, &records);
        assert_eq!(holds.len(), 4);
        assert_eq!(holds[0].component, "debian-pkg");
        assert_eq!(holds[0].reason, HoldReason::LastGood);
        assert_eq!(holds[0].blocked_version, "");
        assert_eq!(holds[0].snapshot, "20261108T000000Z");
        assert_eq!(holds[0].cache_path, CachePath::One(String::new()));

        assert_eq!(holds[1].component, "kernel-many");
        assert_eq!(holds[1].snapshot, "");
        assert_eq!(
            holds[1].cache_path,
            CachePath::Many(vec![
                "artifact-cache/linux-image.deb".to_string(),
                "artifact-cache/linux-headers.deb".to_string(),
            ])
        );

        assert_eq!(holds[2].component, "tool-none");
        assert_eq!(holds[2].cache_path, CachePath::One(String::new()));

        assert_eq!(holds[3].component, "tool-one");
        assert_eq!(
            holds[3].cache_path,
            CachePath::One("artifact-cache/tool-one.deb".to_string())
        );

        let hf = hold_file(holds, "2026-11-09T12:00:00Z", "neb release holds");
        let rendered = render_hold_file(&hf).expect("render_hold_file");
        assert!(rendered.contains("schema_version = 1"));
        assert!(rendered.contains("[[hold]]"));
        assert!(rendered.contains("reason = \"last-good\""));
        assert!(rendered.contains("cache_path = \"artifact-cache/tool-one.deb\""));
        assert!(rendered.contains("cache_path = \"\""));
        assert!(rendered.contains("artifact-cache/linux-image.deb"));

        let parsed: HoldFile = toml::from_str(&rendered).expect("parse rendered HoldFile");
        assert_eq!(parsed, hf);
    }

    #[test]
    fn pins_from_release_all_subset_dedup_and_missing_error() {
        let mut comps = BTreeMap::new();
        comps.insert(
            "example-pkg".to_string(),
            RecordComponent {
                version: "1.0-1".to_string(),
                origin: Origin::Debian,
                binaries: BTreeMap::from([("example-pkg".to_string(), "1.0-1".to_string())]),
            },
        );
        comps.insert(
            "example-kernel".to_string(),
            RecordComponent {
                version: "7.3.3-1".to_string(),
                origin: Origin::ThirdParty,
                binaries: BTreeMap::from([("example-kernel".to_string(), "7.3.3-1".to_string())]),
            },
        );

        let mut tp_cache = BTreeMap::new();
        tp_cache.insert(
            "example-kernel".to_string(),
            vec!["artifact-cache/example-kernel.deb".to_string()],
        );

        let rec = make_record(
            "2026.10",
            "2026-11-08T12:00:00Z",
            "20261108T000000Z",
            comps,
            tp_cache,
        );

        // None -> all components in component ID order.
        let all = pins_from_release(&rec, None).expect("pins_from_release all");
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].component, "example-kernel");
        assert_eq!(all[0].reason, HoldReason::Pin);
        assert_eq!(all[0].blocked_version, "");
        assert_eq!(all[0].snapshot, "");
        assert_eq!(
            all[0].cache_path,
            CachePath::One("artifact-cache/example-kernel.deb".to_string())
        );

        assert_eq!(all[1].component, "example-pkg");
        assert_eq!(all[1].reason, HoldReason::Pin);
        assert_eq!(all[1].snapshot, "20261108T000000Z");
        assert_eq!(all[1].cache_path, CachePath::One(String::new()));

        // Named subset with duplicate and reverse input order -> sorted, one hold per unique component.
        let subset_names = vec![
            "example-pkg".to_string(),
            "example-kernel".to_string(),
            "example-pkg".to_string(),
        ];
        let subset =
            pins_from_release(&rec, Some(&subset_names)).expect("pins_from_release subset");
        assert_eq!(subset.len(), 2);
        assert_eq!(subset[0].component, "example-kernel");
        assert_eq!(subset[1].component, "example-pkg");

        // Single named component subset.
        let single_name = vec!["example-pkg".to_string()];
        let single = pins_from_release(&rec, Some(&single_name)).expect("pins_from_release single");
        assert_eq!(single.len(), 1);
        assert_eq!(single[0].component, "example-pkg");

        // Empty HoldFile round-trip.
        let empty_hf = hold_file(Vec::new(), "2026-11-09T12:00:00Z", "neb release holds");
        let empty_rendered = render_hold_file(&empty_hf).expect("render empty HoldFile");
        assert!(!empty_rendered.starts_with('#'));
        let empty_parsed: HoldFile = toml::from_str(&empty_rendered).expect("parse empty HoldFile");
        assert_eq!(empty_parsed, empty_hf);

        // Missing component -> ValidationError naming the missing component(s).
        let bad_names = vec![
            "example-pkg".to_string(),
            "missing-one".to_string(),
            "missing-two".to_string(),
        ];
        let err = pins_from_release(&rec, Some(&bad_names)).expect_err("missing component");
        match err {
            TrackerError::ValidationError(msg) => {
                assert!(msg.contains("missing-one"));
                assert!(msg.contains("missing-two"));
            }
            other => panic!("expected ValidationError, got {other:?}"),
        }

        assert_eq!(HoldReason::Failure.as_str(), "failure");
        assert_eq!(HoldReason::Bisect.as_str(), "bisect");
        assert_eq!(HoldReason::LastGood.as_str(), "last-good");
        assert_eq!(HoldReason::Pin.as_str(), "pin");
    }
}
