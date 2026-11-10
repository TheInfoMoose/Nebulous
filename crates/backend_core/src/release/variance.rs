use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};
use crate::release::approvals::Approvals;
use crate::release::component::{binary_changes, find_match};
use crate::release::debver;
use crate::release::golden::Golden;
use crate::release::inventory::CustomBinary;
use crate::release::Violation;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Upgrade {
    pub item: String,
    pub from_version: String,
    pub to_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiRename {
    pub component: String,
    pub from_package: String,
    pub to_package: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VarianceReport {
    pub violations: Vec<Violation>,
    pub upgrades: Vec<Upgrade>,
    pub abi_renames: Vec<AbiRename>,
    pub checked_packages_count: usize,
    pub checked_binaries_count: usize,
    pub checked_components_count: usize,
}

impl VarianceReport {
    pub fn is_pass(&self) -> bool {
        self.violations.is_empty()
    }
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

pub fn evaluate_against_golden(
    live: &Golden,
    golden: &Golden,
    approvals: &Approvals,
) -> Result<VarianceReport> {
    if golden.components.is_empty() && !golden.packages.is_empty() {
        return Err(TrackerError::ValidationError(
            "golden.toml is schema v1 (no components); run neb release import".to_string(),
        ));
    }

    let mut violations = Vec::new();
    let mut upgrades = Vec::new();
    let mut abi_renames = Vec::new();

    // 1. Components, in golden id order (BTreeMap).
    for (id, gcomp) in &golden.components {
        match find_match(id, gcomp, &live.components) {
            None => {
                if !approvals.covers(id, Some(&gcomp.version)) {
                    violations.push(Violation::new(
                        "golden_component_removed",
                        id.clone(),
                        format!(
                            "Component '{id}' (golden={}) was removed from live OS",
                            gcomp.version
                        ),
                    ));
                }
            }
            Some(m) => {
                match debver::compare(m.version, &gcomp.version) {
                    Ordering::Less => {
                        if !approvals.covers(id, Some(m.version)) {
                            violations.push(Violation::new(
                                "golden_component_downgrade",
                                id.clone(),
                                format!(
                                    "Component '{id}' downgraded: live={} < golden={}",
                                    m.version, gcomp.version
                                ),
                            ));
                        }
                    }
                    Ordering::Greater => {
                        upgrades.push(Upgrade {
                            item: id.clone(),
                            from_version: gcomp.version.clone(),
                            to_version: m.version.to_string(),
                        });
                    }
                    Ordering::Equal => {}
                }

                if !gcomp.source_unknown {
                    let changes = binary_changes(gcomp, m.component);
                    for (from_package, to_package) in changes.abi_renames {
                        abi_renames.push(AbiRename {
                            component: id.clone(),
                            from_package,
                            to_package,
                        });
                    }
                    for name in changes.dropped {
                        let golden_bin_ver = gcomp.binaries.get(&name).map(String::as_str);
                        if !approvals.covers(&name, golden_bin_ver) {
                            violations.push(Violation::new(
                                "golden_binary_package_dropped",
                                name.clone(),
                                format!(
                                    "Binary package '{name}' of component '{id}' vanished and is not an ABI rename"
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }

    // 2. Custom binaries, in golden key order (BTreeMap).
    let mut live_bins: BTreeMap<&str, &CustomBinary> = BTreeMap::new();
    for (key, entry) in &live.custom_binaries {
        live_bins.insert(key.as_str(), entry);
        live_bins.insert(basename(key), entry);
        if !entry.path.is_empty() {
            live_bins.insert(entry.path.as_str(), entry);
        }
    }

    for (bin_key, golden_meta) in &golden.custom_binaries {
        let bname = basename(bin_key);
        let full_path = if golden_meta.path.is_empty() {
            bin_key.as_str()
        } else {
            golden_meta.path.as_str()
        };
        let is_approved = |target_ver: Option<&str>| {
            approvals.covers(bin_key, target_ver)
                || approvals.covers(bname, target_ver)
                || approvals.covers(full_path, target_ver)
        };

        let live_entry = live_bins
            .get(bin_key.as_str())
            .or_else(|| live_bins.get(bname))
            .or_else(|| live_bins.get(full_path))
            .copied();

        match live_entry {
            None => {
                if !is_approved(Some(&golden_meta.version)) {
                    violations.push(Violation::new(
                        "golden_binary_removed",
                        bin_key.clone(),
                        format!(
                            "Custom binary '{bin_key}' present in golden snapshot is missing from live OS"
                        ),
                    ));
                }
            }
            Some(entry) if !entry.present => {
                if !is_approved(Some(&golden_meta.version)) {
                    violations.push(Violation::new(
                        "golden_binary_removed",
                        bin_key.clone(),
                        format!(
                            "Custom binary '{bin_key}' present in golden snapshot is missing from live OS"
                        ),
                    ));
                }
            }
            Some(entry) => match debver::compare(&entry.version, &golden_meta.version) {
                Ordering::Less => {
                    if !is_approved(Some(&entry.version)) {
                        violations.push(Violation::new(
                            "golden_binary_downgrade",
                            bin_key.clone(),
                            format!(
                                "Custom binary '{bin_key}' downgraded: live={} < golden={}",
                                entry.version, golden_meta.version
                            ),
                        ));
                    }
                }
                Ordering::Greater => {
                    upgrades.push(Upgrade {
                        item: bin_key.clone(),
                        from_version: golden_meta.version.clone(),
                        to_version: entry.version.clone(),
                    });
                }
                Ordering::Equal => {}
            },
        }
    }

    // 3. Calamares modules, in golden presence key order (BTreeMap).
    for (mod_name, &was_present) in &golden.calamares_modules.presence {
        let is_present = live
            .calamares_modules
            .presence
            .get(mod_name)
            .copied()
            .unwrap_or(false);
        if was_present && !is_present && !approvals.covers(mod_name, None) {
            violations.push(Violation::new(
                "golden_calamares_removed",
                mod_name.clone(),
                format!("Calamares module '{mod_name}' present in golden snapshot is missing"),
            ));
        }
    }

    Ok(VarianceReport {
        violations,
        upgrades,
        abi_renames,
        checked_packages_count: live.packages.len(),
        checked_binaries_count: live.custom_binaries.len(),
        checked_components_count: live.components.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::release::approvals::Approval;
    use crate::release::golden::{GoldenComponent, Origin, GOLDEN_SCHEMA_VERSION};
    use crate::release::inventory::{BootInfo, CalamaresModules, IsoMeta};

    fn empty_golden() -> Golden {
        Golden {
            schema_version: GOLDEN_SCHEMA_VERSION,
            iso: IsoMeta {
                path: "/tmp/neb-test/NovOS-sid-amd64.iso".to_string(),
                size_bytes: 1024,
                vmlinuz_sha256: String::new(),
                initrd_sha256: String::new(),
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

    fn custom_bin(path: &str, version: &str, present: bool) -> CustomBinary {
        CustomBinary {
            path: path.to_string(),
            present,
            sha256: "a".repeat(64),
            size_bytes: 4096,
            version: version.to_string(),
            is_symlink: false,
            flags: Vec::new(),
        }
    }

    fn approval(item_name: &str, version: Option<&str>) -> Approval {
        Approval {
            item_name: item_name.to_string(),
            reason: "Approved in unit test".to_string(),
            approved: true,
            timestamp: "2026-11-08T12:00:00Z".to_string(),
            version: version.map(ToString::to_string),
        }
    }

    #[test]
    fn xanmod_kernel_line_upgrade_passes_with_upgrade_and_abi_renames() {
        let mut golden = empty_golden();
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

        let mut live = empty_golden();
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
                        "7.3.3-x64v3-xanmod1-0~20261107.g2ef8115".to_string(),
                    ),
                    (
                        "linux-image-7.3.3-x64v3-xanmod1".to_string(),
                        "7.3.3-x64v3-xanmod1-0~20261107.g2ef8115".to_string(),
                    ),
                    (
                        "linux-xanmod-x64v3".to_string(),
                        "7.3.3-xanmod1-0".to_string(),
                    ),
                ]),
            },
        );

        let report = evaluate_against_golden(&live, &golden, &Approvals::default())
            .expect("evaluate_against_golden");
        assert!(report.is_pass());
        assert!(report.violations.is_empty());
        assert_eq!(
            report.upgrades,
            vec![Upgrade {
                item: "linux-xanmod-x64v3".to_string(),
                from_version: "7.2.15-xanmod1-0".to_string(),
                to_version: "7.3.3-xanmod1-0".to_string(),
            }]
        );
        assert_eq!(
            report.abi_renames,
            vec![
                AbiRename {
                    component: "linux-xanmod-x64v3".to_string(),
                    from_package: "linux-headers-7.2.15-x64v3-xanmod1".to_string(),
                    to_package: "linux-headers-7.3.3-x64v3-xanmod1".to_string(),
                },
                AbiRename {
                    component: "linux-xanmod-x64v3".to_string(),
                    from_package: "linux-image-7.2.15-x64v3-xanmod1".to_string(),
                    to_package: "linux-image-7.3.3-x64v3-xanmod1".to_string(),
                },
            ]
        );
    }

    #[test]
    fn component_downgrade_and_version_scoped_approvals() {
        let mut golden = empty_golden();
        golden.components.insert(
            "dracut".to_string(),
            GoldenComponent {
                version: "110-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("dracut".to_string(), "110-1".to_string())]),
            },
        );

        let mut live = empty_golden();
        live.components.insert(
            "dracut".to_string(),
            GoldenComponent {
                version: "105-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("dracut".to_string(), "105-1".to_string())]),
            },
        );

        // Unapproved -> violation.
        let unapproved =
            evaluate_against_golden(&live, &golden, &Approvals::default()).expect("unapproved");
        assert!(!unapproved.is_pass());
        assert_eq!(
            unapproved.violations,
            vec![Violation::new(
                "golden_component_downgrade",
                "dracut",
                "Component 'dracut' downgraded: live=105-1 < golden=110-1",
            )]
        );

        // Open approval -> PASS.
        let open_approvals = Approvals {
            approvals: vec![approval("dracut", None)],
        };
        let open_report =
            evaluate_against_golden(&live, &golden, &open_approvals).expect("open approval");
        assert!(open_report.is_pass());

        // Approval for matching live version 105-1 -> PASS.
        let matching_approvals = Approvals {
            approvals: vec![approval("dracut", Some("105-1"))],
        };
        let matching_report =
            evaluate_against_golden(&live, &golden, &matching_approvals).expect("matching version");
        assert!(matching_report.is_pass());

        // Approval for wrong version 104-1 -> still violation.
        let wrong_ver_approvals = Approvals {
            approvals: vec![approval("dracut", Some("104-1"))],
        };
        let wrong_report =
            evaluate_against_golden(&live, &golden, &wrong_ver_approvals).expect("wrong version");
        assert!(!wrong_report.is_pass());
        assert_eq!(wrong_report.violations.len(), 1);
        assert_eq!(
            wrong_report.violations[0].kind,
            "golden_component_downgrade"
        );
    }

    #[test]
    fn component_removal_and_approval() {
        let mut golden = empty_golden();
        golden.components.insert(
            "btrfs-progs".to_string(),
            GoldenComponent {
                version: "6.6.3-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("btrfs-progs".to_string(), "6.6.3-1".to_string())]),
            },
        );
        let live = empty_golden();

        let report =
            evaluate_against_golden(&live, &golden, &Approvals::default()).expect("unapproved");
        assert!(!report.is_pass());
        assert_eq!(
            report.violations,
            vec![Violation::new(
                "golden_component_removed",
                "btrfs-progs",
                "Component 'btrfs-progs' (golden=6.6.3-1) was removed from live OS",
            )]
        );

        let approved = Approvals {
            approvals: vec![approval("btrfs-progs", Some("6.6.3-1"))],
        };
        let pass_report = evaluate_against_golden(&live, &golden, &approved).expect("approved");
        assert!(pass_report.is_pass());
    }

    #[test]
    fn dropped_binary_package_vs_t64_abi_rename() {
        let mut golden = empty_golden();
        golden.components.insert(
            "foo".to_string(),
            GoldenComponent {
                version: "1.0-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([
                    ("libfoo-doc".to_string(), "1.0-1".to_string()),
                    ("libfoo1".to_string(), "1.0-1".to_string()),
                ]),
            },
        );
        golden.components.insert(
            "libpcap".to_string(),
            GoldenComponent {
                version: "1.10.4-4".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("libpcap0.8".to_string(), "1.10.4-4".to_string())]),
            },
        );

        let mut live = empty_golden();
        live.components.insert(
            "foo".to_string(),
            GoldenComponent {
                version: "1.0-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("libfoo1".to_string(), "1.0-1".to_string())]),
            },
        );
        live.components.insert(
            "libpcap".to_string(),
            GoldenComponent {
                version: "1.10.4-4".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("libpcap0.8t64".to_string(), "1.10.4-4".to_string())]),
            },
        );

        let report =
            evaluate_against_golden(&live, &golden, &Approvals::default()).expect("evaluate");
        assert!(!report.is_pass());
        assert_eq!(
            report.violations,
            vec![Violation::new(
                "golden_binary_package_dropped",
                "libfoo-doc",
                "Binary package 'libfoo-doc' of component 'foo' vanished and is not an ABI rename",
            )]
        );
        assert_eq!(
            report.abi_renames,
            vec![AbiRename {
                component: "libpcap".to_string(),
                from_package: "libpcap0.8".to_string(),
                to_package: "libpcap0.8t64".to_string(),
            }]
        );

        // Approving the dropped binary package clears the violation.
        let approved = Approvals {
            approvals: vec![approval("libfoo-doc", Some("1.0-1"))],
        };
        let pass_report = evaluate_against_golden(&live, &golden, &approved).expect("approved");
        assert!(pass_report.is_pass());
    }

    #[test]
    fn imported_placeholder_upgrade_and_downgrade() {
        let mut golden = empty_golden();
        golden.components.insert(
            "libpcap0.8".to_string(),
            GoldenComponent {
                version: "1.10.5-2".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: true,
                binaries: BTreeMap::from([("libpcap0.8".to_string(), "1.10.5-2".to_string())]),
            },
        );

        let mut live_higher = empty_golden();
        live_higher.components.insert(
            "libpcap".to_string(),
            GoldenComponent {
                version: "1.10.5-2".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([
                    ("libpcap-dev".to_string(), "1.10.5-2".to_string()),
                    ("libpcap0.8".to_string(), "1.10.5-2+b1".to_string()),
                ]),
            },
        );

        let up_report = evaluate_against_golden(&live_higher, &golden, &Approvals::default())
            .expect("placeholder higher");
        assert!(up_report.is_pass());
        assert_eq!(
            up_report.upgrades,
            vec![Upgrade {
                item: "libpcap0.8".to_string(),
                from_version: "1.10.5-2".to_string(),
                to_version: "1.10.5-2+b1".to_string(),
            }]
        );
        assert!(up_report.abi_renames.is_empty());

        let mut live_lower = empty_golden();
        live_lower.components.insert(
            "libpcap".to_string(),
            GoldenComponent {
                version: "1.10.4-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("libpcap0.8".to_string(), "1.10.4-1".to_string())]),
            },
        );

        let down_report = evaluate_against_golden(&live_lower, &golden, &Approvals::default())
            .expect("placeholder lower");
        assert!(!down_report.is_pass());
        assert_eq!(
            down_report.violations,
            vec![Violation::new(
                "golden_component_downgrade",
                "libpcap0.8",
                "Component 'libpcap0.8' downgraded: live=1.10.4-1 < golden=1.10.5-2",
            )]
        );
    }

    #[test]
    fn custom_binaries_missing_not_present_downgrade_upgrade_and_basename_approval() {
        let mut golden = empty_golden();
        golden.custom_binaries.insert(
            "bin-missing".to_string(),
            custom_bin("/usr/local/bin/bin-missing", "1.0.0", true),
        );
        golden.custom_binaries.insert(
            "bin-disabled".to_string(),
            custom_bin("/usr/local/bin/bin-disabled", "1.0.0", true),
        );
        golden.custom_binaries.insert(
            "bin-downgraded".to_string(),
            custom_bin("/usr/local/bin/bin-downgraded", "2.0.0", true),
        );
        golden.custom_binaries.insert(
            "bin-upgraded".to_string(),
            custom_bin("/usr/local/bin/bin-upgraded", "1.0.0", true),
        );
        golden.custom_binaries.insert(
            "/usr/local/bin/full-path-tool".to_string(),
            custom_bin("/usr/local/bin/full-path-tool", "1.5.0", true),
        );

        let mut live = empty_golden();
        live.custom_binaries.insert(
            "bin-disabled".to_string(),
            custom_bin("/usr/local/bin/bin-disabled", "1.0.0", false),
        );
        live.custom_binaries.insert(
            "bin-downgraded".to_string(),
            custom_bin("/usr/local/bin/bin-downgraded", "1.5.0", true),
        );
        live.custom_binaries.insert(
            "bin-upgraded".to_string(),
            custom_bin("/usr/local/bin/bin-upgraded", "1.2.0", true),
        );

        // Approve `full-path-tool` by basename only.
        let approvals = Approvals {
            approvals: vec![approval("full-path-tool", Some("1.5.0"))],
        };

        let report = evaluate_against_golden(&live, &golden, &approvals).expect("custom binaries");
        assert!(!report.is_pass());
        assert_eq!(
            report.violations,
            vec![
                Violation::new(
                    "golden_binary_removed",
                    "bin-disabled",
                    "Custom binary 'bin-disabled' present in golden snapshot is missing from live OS",
                ),
                Violation::new(
                    "golden_binary_downgrade",
                    "bin-downgraded",
                    "Custom binary 'bin-downgraded' downgraded: live=1.5.0 < golden=2.0.0",
                ),
                Violation::new(
                    "golden_binary_removed",
                    "bin-missing",
                    "Custom binary 'bin-missing' present in golden snapshot is missing from live OS",
                ),
            ]
        );
        assert_eq!(
            report.upgrades,
            vec![Upgrade {
                item: "bin-upgraded".to_string(),
                from_version: "1.0.0".to_string(),
                to_version: "1.2.0".to_string(),
            }]
        );
    }

    #[test]
    fn custom_binary_lookup_matches_across_key_basename_and_path() {
        let mut golden = empty_golden();
        golden.custom_binaries.insert(
            "/usr/local/bin/tool-by-basename".to_string(),
            custom_bin("/usr/local/bin/tool-by-basename", "1.0.0", true),
        );
        golden.custom_binaries.insert(
            "tool-by-path".to_string(),
            custom_bin("/usr/local/sbin/tool-by-path", "1.0.0", true),
        );

        let mut live = empty_golden();
        live.custom_binaries.insert(
            "tool-by-basename".to_string(),
            custom_bin("/usr/local/bin/tool-by-basename", "1.1.0", true),
        );
        live.custom_binaries.insert(
            "renamed-key".to_string(),
            custom_bin("/usr/local/sbin/tool-by-path", "1.0.0", true),
        );

        let report = evaluate_against_golden(&live, &golden, &Approvals::default())
            .expect("cross lookup custom binaries");
        assert!(report.is_pass());
        assert_eq!(
            report.upgrades,
            vec![Upgrade {
                item: "/usr/local/bin/tool-by-basename".to_string(),
                from_version: "1.0.0".to_string(),
                to_version: "1.1.0".to_string(),
            }]
        );
    }

    #[test]
    fn calamares_module_removed_and_approved() {
        let mut golden = empty_golden();
        golden
            .calamares_modules
            .presence
            .insert("fstab.conf".to_string(), true);
        golden
            .calamares_modules
            .presence
            .insert("welcome.conf".to_string(), true);

        let mut live = empty_golden();
        live.calamares_modules
            .presence
            .insert("welcome.conf".to_string(), true);

        let report =
            evaluate_against_golden(&live, &golden, &Approvals::default()).expect("calamares");
        assert!(!report.is_pass());
        assert_eq!(
            report.violations,
            vec![Violation::new(
                "golden_calamares_removed",
                "fstab.conf",
                "Calamares module 'fstab.conf' present in golden snapshot is missing",
            )]
        );

        // Versioned approval does not cover a Calamares module (`target_version` is None).
        let versioned = Approvals {
            approvals: vec![approval("fstab.conf", Some("1.0"))],
        };
        let still_fails = evaluate_against_golden(&live, &golden, &versioned).expect("versioned");
        assert!(!still_fails.is_pass());

        // Open approval covers it.
        let open = Approvals {
            approvals: vec![approval("fstab.conf", None)],
        };
        let passes = evaluate_against_golden(&live, &golden, &open).expect("open");
        assert!(passes.is_pass());
    }

    #[test]
    fn v1_golden_without_components_returns_validation_error() {
        let mut golden = empty_golden();
        golden.schema_version = 1;
        golden
            .packages
            .insert("base-files".to_string(), "14".to_string());

        let live = empty_golden();
        let err = evaluate_against_golden(&live, &golden, &Approvals::default())
            .expect_err("v1 golden with packages and no components must fail");
        match err {
            TrackerError::ValidationError(msg) => {
                assert_eq!(
                    msg,
                    "golden.toml is schema v1 (no components); run neb release import"
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn report_counts_equal_live_map_lengths_and_output_is_deterministic() {
        let mut golden = empty_golden();
        golden.components.insert(
            "base-files".to_string(),
            GoldenComponent {
                version: "14".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("base-files".to_string(), "14".to_string())]),
            },
        );

        let mut live = empty_golden();
        live.packages
            .insert("base-files".to_string(), "14".to_string());
        live.packages
            .insert("libc6".to_string(), "2.40-1".to_string());
        live.packages
            .insert("libc-bin".to_string(), "2.40-1".to_string());
        live.custom_binaries.insert(
            "tool-a".to_string(),
            custom_bin("/usr/local/bin/tool-a", "0.1.0", true),
        );
        live.custom_binaries.insert(
            "tool-b".to_string(),
            custom_bin("/usr/local/bin/tool-b", "0.2.0", true),
        );
        live.components.insert(
            "base-files".to_string(),
            GoldenComponent {
                version: "14".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("base-files".to_string(), "14".to_string())]),
            },
        );
        live.components.insert(
            "glibc".to_string(),
            GoldenComponent {
                version: "2.40-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([
                    ("libc-bin".to_string(), "2.40-1".to_string()),
                    ("libc6".to_string(), "2.40-1".to_string()),
                ]),
            },
        );

        let report_a =
            evaluate_against_golden(&live, &golden, &Approvals::default()).expect("report a");
        let report_b =
            evaluate_against_golden(&live, &golden, &Approvals::default()).expect("report b");
        assert_eq!(report_a, report_b);
        assert!(report_a.is_pass());
        assert_eq!(report_a.checked_packages_count, 3);
        assert_eq!(report_a.checked_binaries_count, 2);
        assert_eq!(report_a.checked_components_count, 2);
    }
}
