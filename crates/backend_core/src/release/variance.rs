use std::cmp::Ordering;
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};
use crate::release::approvals::Approvals;
use crate::release::component::{binary_changes, find_match};
use crate::release::debver;
use crate::release::golden::Golden;
use crate::release::inventory::CustomBinary;
use crate::release::manifest::Manifest;
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

fn index_custom_binaries(bins: &BTreeMap<String, CustomBinary>) -> BTreeMap<&str, &CustomBinary> {
    let mut indexed = BTreeMap::new();
    for (key, entry) in bins {
        indexed.insert(key.as_str(), entry);
        indexed.insert(basename(key), entry);
        if !entry.path.is_empty() {
            indexed.insert(entry.path.as_str(), entry);
        }
    }
    indexed
}

/// Evaluates `live` against `manifest` using the forward-only variance rule.
///
/// Whereas the Python implementation skipped an entire item on any approval by
/// name, `neb` scopes a versioned approval to the downgrade target version (and
/// open-ended approvals without a `version` cover missing items, flags, and
/// downgrades alike).
pub fn evaluate_against_manifest(
    live: &Golden,
    manifest: &Manifest,
    approvals: &Approvals,
) -> Vec<Violation> {
    let mut violations = Vec::new();

    // 0. Check os-release metadata when required in manifest and present in live.
    if manifest.os.required && !live.os_release.is_empty() && !approvals.covers("os_release", None)
    {
        if let (Some(expected_id), Some(live_id)) = (
            manifest.os.distro_id.as_deref().filter(|s| !s.is_empty()),
            live.os_release
                .get("ID")
                .map(String::as_str)
                .filter(|s| !s.is_empty()),
        ) {
            if live_id != expected_id {
                violations.push(Violation::new(
                    "manifest_os_id_mismatch",
                    "os_release.ID",
                    format!("/etc/os-release ID '{live_id}' != '{expected_id}'"),
                ));
            }
        }

        if let (Some(min_os_ver), Some(live_os_ver)) = (
            manifest
                .os
                .distro_version
                .as_deref()
                .filter(|s| !s.is_empty()),
            live.os_release
                .get("VERSION_ID")
                .map(String::as_str)
                .filter(|s| !s.is_empty()),
        ) {
            if debver::compare(live_os_ver, min_os_ver) == Ordering::Less {
                violations.push(Violation::new(
                    "manifest_os_version_downgrade",
                    "os_release.VERSION_ID",
                    format!("/etc/os-release VERSION_ID {live_os_ver} < {min_os_ver}"),
                ));
            }
        }
    }

    // 1. Check against manifest.toml packages (core, then cosmic, BTreeMap order).
    for group in [&manifest.packages.core, &manifest.packages.cosmic] {
        for (pkg_name, spec) in group {
            if !spec.required {
                continue;
            }
            match live.packages.get(pkg_name) {
                None => {
                    if !approvals.covers(pkg_name, None) {
                        violations.push(Violation::new(
                            "manifest_package_missing",
                            pkg_name.clone(),
                            format!(
                                "Required manifest package '{pkg_name}' is missing from live OS"
                            ),
                        ));
                    }
                }
                Some(live_ver) => {
                    if debver::compare(live_ver, &spec.min_version) == Ordering::Less
                        && !approvals.covers(pkg_name, Some(live_ver))
                    {
                        violations.push(Violation::new(
                            "manifest_package_downgrade",
                            pkg_name.clone(),
                            format!(
                                "Package '{pkg_name}' version {live_ver} < manifest min_version {}",
                                spec.min_version
                            ),
                        ));
                    }
                }
            }
        }
    }

    // 2. Check against manifest.toml custom_binaries (BTreeMap order).
    let live_bins = index_custom_binaries(&live.custom_binaries);
    for (bin_path, spec) in &manifest.custom_binaries {
        if !spec.required {
            continue;
        }
        let bname = basename(bin_path);
        let ok = |target_ver: Option<&str>| {
            approvals.covers(bin_path, target_ver) || approvals.covers(bname, target_ver)
        };

        let live_entry = live_bins
            .get(bin_path.as_str())
            .or_else(|| live_bins.get(bname))
            .copied();

        match live_entry {
            None => {
                if !ok(None) {
                    violations.push(Violation::new(
                        "manifest_binary_missing",
                        bin_path.clone(),
                        format!("Required custom binary '{bin_path}' is missing from live OS"),
                    ));
                }
            }
            Some(entry) if !entry.present => {
                if !ok(None) {
                    violations.push(Violation::new(
                        "manifest_binary_missing",
                        bin_path.clone(),
                        format!("Required custom binary '{bin_path}' is missing from live OS"),
                    ));
                }
            }
            Some(entry) => {
                if debver::compare(&entry.version, &spec.min_version) == Ordering::Less
                    && !ok(Some(&entry.version))
                {
                    violations.push(Violation::new(
                        "manifest_binary_downgrade",
                        bin_path.clone(),
                        format!(
                            "Custom binary '{bin_path}' version {} < manifest min_version {}",
                            entry.version, spec.min_version
                        ),
                    ));
                }
                for req_flag in &spec.required_flags {
                    if !entry.flags.contains(req_flag) && !ok(None) {
                        violations.push(Violation::new(
                            "manifest_binary_flag_missing",
                            bin_path.clone(),
                            format!(
                                "Custom binary '{bin_path}' missing required flag '{req_flag}'"
                            ),
                        ));
                    }
                }
            }
        }
    }

    // 3. Check against manifest.toml calamares_modules.
    if manifest.calamares_modules.required {
        for cfg in &manifest.calamares_modules.required_configs {
            let is_present = live
                .calamares_modules
                .presence
                .get(cfg)
                .copied()
                .unwrap_or(false);
            if !is_present && !approvals.covers(cfg, None) {
                violations.push(Violation::new(
                    "manifest_calamares_missing",
                    cfg.clone(),
                    format!("Required Calamares module config '{cfg}' missing from live OS"),
                ));
            }
        }
        for cmod in &manifest.calamares_modules.custom_modules {
            let is_present = live
                .calamares_modules
                .presence
                .get(cmod)
                .copied()
                .unwrap_or(false);
            if !is_present && !approvals.covers(cmod, None) {
                violations.push(Violation::new(
                    "manifest_calamares_custom_missing",
                    cmod.clone(),
                    format!("Required Calamares custom module '{cmod}' missing from live OS"),
                ));
            }
        }
    }

    violations
}

pub fn evaluate_variance(
    live: &Golden,
    manifest: &Manifest,
    golden: Option<&Golden>,
    approvals: &Approvals,
) -> Result<VarianceReport> {
    let mut violations = evaluate_against_manifest(live, manifest, approvals);
    let mut upgrades = Vec::new();
    let mut abi_renames = Vec::new();

    if let Some(g) = golden {
        let golden_report = evaluate_against_golden(live, g, approvals)?;
        violations.extend(golden_report.violations);
        upgrades = golden_report.upgrades;
        abi_renames = golden_report.abi_renames;
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
    let live_bins = index_custom_binaries(&live.custom_binaries);

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
    use crate::release::manifest::{
        BinarySpec, CalamaresSpec, ManifestPackages, OsSpec, PackageSpec, MANIFEST_SCHEMA_VERSION,
    };

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

    fn sample_manifest() -> Manifest {
        Manifest {
            schema_version: MANIFEST_SCHEMA_VERSION,
            os: OsSpec {
                required: true,
                distro_id: Some("exampleos".to_string()),
                distro_version: Some("13".to_string()),
            },
            packages: ManifestPackages {
                core: BTreeMap::from([
                    (
                        "dracut".to_string(),
                        PackageSpec {
                            required: true,
                            min_version: "105-1".to_string(),
                        },
                    ),
                    (
                        "systemd-sysv".to_string(),
                        PackageSpec {
                            required: true,
                            min_version: "255".to_string(),
                        },
                    ),
                ]),
                cosmic: BTreeMap::from([(
                    "cosmic-comp".to_string(),
                    PackageSpec {
                        required: true,
                        min_version: "0.1.0".to_string(),
                    },
                )]),
            },
            custom_binaries: BTreeMap::from([(
                "/usr/local/bin/testtool".to_string(),
                BinarySpec {
                    required: true,
                    min_version: "0.2.0".to_string(),
                    required_flags: vec!["--pre-packages".to_string(), "--verify".to_string()],
                },
            )]),
            calamares_modules: CalamaresSpec {
                required: true,
                required_configs: vec!["fstab.conf".to_string()],
                custom_modules: vec!["limine-bootloader".to_string()],
            },
        }
    }

    fn matching_live_for_manifest() -> Golden {
        let mut live = empty_golden();
        live.os_release
            .insert("ID".to_string(), "exampleos".to_string());
        live.os_release
            .insert("VERSION_ID".to_string(), "13".to_string());
        live.packages
            .insert("dracut".to_string(), "106-1".to_string());
        live.packages
            .insert("systemd-sysv".to_string(), "256".to_string());
        live.packages
            .insert("cosmic-comp".to_string(), "0.1.0".to_string());

        let mut bin = custom_bin("/usr/local/bin/testtool", "0.2.0", true);
        bin.flags = vec!["--pre-packages".to_string(), "--verify".to_string()];
        live.custom_binaries
            .insert("/usr/local/bin/testtool".to_string(), bin);

        live.calamares_modules
            .presence
            .insert("fstab.conf".to_string(), true);
        live.calamares_modules
            .presence
            .insert("limine-bootloader".to_string(), true);
        live
    }

    #[test]
    fn manifest_clean_live_inventory_produces_no_violations() {
        let manifest = sample_manifest();
        let live = matching_live_for_manifest();
        let violations = evaluate_against_manifest(&live, &manifest, &Approvals::default());
        assert!(violations.is_empty());
    }

    #[test]
    fn manifest_reports_each_violation_kind_with_exact_details() {
        let manifest = sample_manifest();

        // 1. manifest_os_id_mismatch
        let mut live = matching_live_for_manifest();
        live.os_release
            .insert("ID".to_string(), "wrongos".to_string());
        assert_eq!(
            evaluate_against_manifest(&live, &manifest, &Approvals::default()),
            vec![Violation::new(
                "manifest_os_id_mismatch",
                "os_release.ID",
                "/etc/os-release ID 'wrongos' != 'exampleos'",
            )]
        );

        // 2. manifest_os_version_downgrade
        let mut live = matching_live_for_manifest();
        live.os_release
            .insert("VERSION_ID".to_string(), "12".to_string());
        assert_eq!(
            evaluate_against_manifest(&live, &manifest, &Approvals::default()),
            vec![Violation::new(
                "manifest_os_version_downgrade",
                "os_release.VERSION_ID",
                "/etc/os-release VERSION_ID 12 < 13",
            )]
        );

        // 3. manifest_package_missing (core and cosmic)
        let mut live = matching_live_for_manifest();
        live.packages.remove("dracut");
        live.packages.remove("cosmic-comp");
        assert_eq!(
            evaluate_against_manifest(&live, &manifest, &Approvals::default()),
            vec![
                Violation::new(
                    "manifest_package_missing",
                    "dracut",
                    "Required manifest package 'dracut' is missing from live OS",
                ),
                Violation::new(
                    "manifest_package_missing",
                    "cosmic-comp",
                    "Required manifest package 'cosmic-comp' is missing from live OS",
                ),
            ]
        );

        // 4. manifest_package_downgrade
        let mut live = matching_live_for_manifest();
        live.packages
            .insert("dracut".to_string(), "104-1".to_string());
        assert_eq!(
            evaluate_against_manifest(&live, &manifest, &Approvals::default()),
            vec![Violation::new(
                "manifest_package_downgrade",
                "dracut",
                "Package 'dracut' version 104-1 < manifest min_version 105-1",
            )]
        );

        // 5. manifest_binary_missing (absent from map)
        let mut live = matching_live_for_manifest();
        live.custom_binaries.clear();
        assert_eq!(
            evaluate_against_manifest(&live, &manifest, &Approvals::default()),
            vec![Violation::new(
                "manifest_binary_missing",
                "/usr/local/bin/testtool",
                "Required custom binary '/usr/local/bin/testtool' is missing from live OS",
            )]
        );

        // 6. manifest_binary_missing (present = false)
        let mut live = matching_live_for_manifest();
        live.custom_binaries
            .get_mut("/usr/local/bin/testtool")
            .expect("testtool")
            .present = false;
        assert_eq!(
            evaluate_against_manifest(&live, &manifest, &Approvals::default()),
            vec![Violation::new(
                "manifest_binary_missing",
                "/usr/local/bin/testtool",
                "Required custom binary '/usr/local/bin/testtool' is missing from live OS",
            )]
        );

        // 7. manifest_binary_downgrade
        let mut live = matching_live_for_manifest();
        live.custom_binaries
            .get_mut("/usr/local/bin/testtool")
            .expect("testtool")
            .version = "0.1.5".to_string();
        assert_eq!(
            evaluate_against_manifest(&live, &manifest, &Approvals::default()),
            vec![Violation::new(
                "manifest_binary_downgrade",
                "/usr/local/bin/testtool",
                "Custom binary '/usr/local/bin/testtool' version 0.1.5 < manifest min_version 0.2.0",
            )]
        );

        // 8. manifest_binary_flag_missing
        let mut live = matching_live_for_manifest();
        live.custom_binaries
            .get_mut("/usr/local/bin/testtool")
            .expect("testtool")
            .flags = vec!["--pre-packages".to_string()];
        assert_eq!(
            evaluate_against_manifest(&live, &manifest, &Approvals::default()),
            vec![Violation::new(
                "manifest_binary_flag_missing",
                "/usr/local/bin/testtool",
                "Custom binary '/usr/local/bin/testtool' missing required flag '--verify'",
            )]
        );

        // 9. manifest_calamares_missing
        let mut live = matching_live_for_manifest();
        live.calamares_modules.presence.remove("fstab.conf");
        assert_eq!(
            evaluate_against_manifest(&live, &manifest, &Approvals::default()),
            vec![Violation::new(
                "manifest_calamares_missing",
                "fstab.conf",
                "Required Calamares module config 'fstab.conf' missing from live OS",
            )]
        );

        // 10. manifest_calamares_custom_missing
        let mut live = matching_live_for_manifest();
        live.calamares_modules.presence.remove("limine-bootloader");
        assert_eq!(
            evaluate_against_manifest(&live, &manifest, &Approvals::default()),
            vec![Violation::new(
                "manifest_calamares_custom_missing",
                "limine-bootloader",
                "Required Calamares custom module 'limine-bootloader' missing from live OS",
            )]
        );
    }

    #[test]
    fn manifest_optional_sections_and_specs_are_skipped_when_not_required() {
        let mut manifest = sample_manifest();
        manifest.os.required = false;
        manifest.calamares_modules.required = false;
        manifest.packages.core.insert(
            "optional-pkg".to_string(),
            PackageSpec {
                required: false,
                min_version: "99.0".to_string(),
            },
        );
        manifest.custom_binaries.insert(
            "/usr/local/bin/optional-bin".to_string(),
            BinarySpec {
                required: false,
                min_version: "9.9.9".to_string(),
                required_flags: vec!["--flag".to_string()],
            },
        );

        let mut live = matching_live_for_manifest();
        live.os_release
            .insert("ID".to_string(), "otheros".to_string());
        live.os_release
            .insert("VERSION_ID".to_string(), "1".to_string());
        live.calamares_modules.presence.clear();

        let violations = evaluate_against_manifest(&live, &manifest, &Approvals::default());
        assert!(violations.is_empty());
    }

    #[test]
    fn manifest_basename_and_version_scoped_approvals() {
        let manifest = sample_manifest();

        // Missing binary keyed by full path is covered by an open approval on its basename.
        let mut live_missing_bin = matching_live_for_manifest();
        live_missing_bin.custom_binaries.clear();
        let basename_approvals = Approvals {
            approvals: vec![approval("testtool", None)],
        };
        assert!(
            evaluate_against_manifest(&live_missing_bin, &manifest, &basename_approvals).is_empty()
        );

        // Version-scoped approval `version = "254"` covers systemd-sysv downgrade to 254, not 253.
        let ver_approvals = Approvals {
            approvals: vec![approval("systemd-sysv", Some("254"))],
        };
        let mut live_254 = matching_live_for_manifest();
        live_254
            .packages
            .insert("systemd-sysv".to_string(), "254".to_string());
        assert!(evaluate_against_manifest(&live_254, &manifest, &ver_approvals).is_empty());

        let mut live_253 = matching_live_for_manifest();
        live_253
            .packages
            .insert("systemd-sysv".to_string(), "253".to_string());
        assert_eq!(
            evaluate_against_manifest(&live_253, &manifest, &ver_approvals),
            vec![Violation::new(
                "manifest_package_downgrade",
                "systemd-sysv",
                "Package 'systemd-sysv' version 253 < manifest min_version 255",
            )]
        );
    }

    #[test]
    fn evaluate_variance_combines_manifest_and_optional_golden() {
        let manifest = sample_manifest();
        let mut live = matching_live_for_manifest();
        // Trigger a manifest violation: remove cosmic-comp.
        live.packages.remove("cosmic-comp");
        live.components.insert(
            "dracut".to_string(),
            GoldenComponent {
                version: "106-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("dracut".to_string(), "106-1".to_string())]),
            },
        );

        // With golden = None: upgrades and abi_renames are empty, manifest violation is reported.
        let without_golden = evaluate_variance(&live, &manifest, None, &Approvals::default())
            .expect("evaluate_variance without golden");
        assert!(!without_golden.is_pass());
        assert!(without_golden.upgrades.is_empty());
        assert!(without_golden.abi_renames.is_empty());
        assert_eq!(
            without_golden.violations,
            vec![Violation::new(
                "manifest_package_missing",
                "cosmic-comp",
                "Required manifest package 'cosmic-comp' is missing from live OS",
            )]
        );
        assert_eq!(without_golden.checked_packages_count, live.packages.len());
        assert_eq!(
            without_golden.checked_binaries_count,
            live.custom_binaries.len()
        );
        assert_eq!(
            without_golden.checked_components_count,
            live.components.len()
        );

        // With golden = Some: manifest violations precede golden violations, and golden upgrades are kept.
        let mut golden = empty_golden();
        golden.components.insert(
            "dracut".to_string(),
            GoldenComponent {
                version: "105-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("dracut".to_string(), "105-1".to_string())]),
            },
        );
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

        let with_golden = evaluate_variance(&live, &manifest, Some(&golden), &Approvals::default())
            .expect("evaluate_variance with golden");
        assert!(!with_golden.is_pass());
        assert_eq!(
            with_golden.violations,
            vec![
                Violation::new(
                    "manifest_package_missing",
                    "cosmic-comp",
                    "Required manifest package 'cosmic-comp' is missing from live OS",
                ),
                Violation::new(
                    "golden_component_removed",
                    "btrfs-progs",
                    "Component 'btrfs-progs' (golden=6.6.3-1) was removed from live OS",
                ),
            ]
        );
        assert_eq!(
            with_golden.upgrades,
            vec![Upgrade {
                item: "dracut".to_string(),
                from_version: "105-1".to_string(),
                to_version: "106-1".to_string(),
            }]
        );
    }
}
