use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::release::config::Components;
use crate::release::debver;
use crate::release::golden::{Golden, GoldenComponent, Origin};
use crate::release::inventory::{InstalledPackage, IsoInventory};

pub fn abi_key(name: &str) -> String {
    let base = name.strip_suffix("t64").unwrap_or(name);
    let mut out = String::with_capacity(base.len());
    let mut in_digits = false;

    for ch in base.chars() {
        if ch.is_ascii_digit() {
            if !in_digits {
                out.push('#');
                in_digits = true;
            }
        } else {
            in_digits = false;
            out.push(ch);
        }
    }

    out
}

pub fn is_abi_rename(old: &str, new: &str) -> bool {
    old != new && abi_key(old) == abi_key(new)
}

fn resolve_origin(id: &str, binaries: &BTreeMap<String, String>, cfg: &Components) -> Origin {
    if cfg
        .third_party
        .iter()
        .any(|tp| tp == id || binaries.contains_key(tp.as_str()))
    {
        Origin::ThirdParty
    } else {
        Origin::Debian
    }
}

pub fn group_components(
    packages: &BTreeMap<String, InstalledPackage>,
    cfg: &Components,
) -> BTreeMap<String, GoldenComponent> {
    let mut components = BTreeMap::new();
    let mut claimed: BTreeSet<&str> = BTreeSet::new();

    for meta_name in &cfg.kernel_lines {
        if claimed.contains(meta_name.as_str()) {
            continue;
        }
        let Some(meta_pkg) = packages.get(meta_name.as_str()) else {
            continue;
        };

        claimed.insert(meta_pkg.name.as_str());
        let mut binaries = BTreeMap::new();
        binaries.insert(meta_pkg.name.clone(), meta_pkg.version.clone());

        for dep in &meta_pkg.depends {
            if claimed.contains(dep.as_str()) {
                continue;
            }
            if !dep.starts_with("linux-image-") && !dep.starts_with("linux-headers-") {
                continue;
            }
            if cfg.kernel_lines.iter().any(|kl| kl == dep) {
                continue;
            }
            if let Some(dep_pkg) = packages.get(dep.as_str()) {
                claimed.insert(dep_pkg.name.as_str());
                binaries.insert(dep_pkg.name.clone(), dep_pkg.version.clone());
            }
        }

        let origin = resolve_origin(&meta_pkg.name, &binaries, cfg);
        components.insert(
            meta_pkg.name.clone(),
            GoldenComponent {
                version: meta_pkg.version.clone(),
                origin,
                kernel_line: true,
                source_unknown: false,
                binaries,
            },
        );
    }

    for (name, pkg) in packages {
        if claimed.contains(name.as_str()) {
            continue;
        }

        let comp_id = if pkg.source.is_empty() {
            &pkg.name
        } else {
            &pkg.source
        };
        let candidate_ver = pkg.source_version.as_deref().unwrap_or(&pkg.version);

        if let Some(existing) = components.get_mut(comp_id.as_str()) {
            if debver::compare(candidate_ver, &existing.version) == Ordering::Greater {
                existing.version = candidate_ver.to_string();
            }
            existing.binaries.insert(name.clone(), pkg.version.clone());
        } else {
            let mut binaries = BTreeMap::new();
            binaries.insert(name.clone(), pkg.version.clone());
            components.insert(
                comp_id.clone(),
                GoldenComponent {
                    version: candidate_ver.to_string(),
                    origin: Origin::Debian,
                    kernel_line: false,
                    source_unknown: false,
                    binaries,
                },
            );
        }
    }

    for (id, comp) in &mut components {
        if !comp.kernel_line {
            comp.origin = resolve_origin(id, &comp.binaries, cfg);
        }
    }

    components
}

pub fn golden_from_inventory(inv: &IsoInventory, cfg: &Components) -> Golden {
    Golden::from_inventory(inv, group_components(&inv.packages, cfg))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComponentMatch<'a> {
    pub id: &'a str,
    pub component: &'a GoldenComponent,
    pub version: &'a str,
}

pub fn find_match<'a>(
    golden_id: &str,
    golden: &GoldenComponent,
    new: &'a BTreeMap<String, GoldenComponent>,
) -> Option<ComponentMatch<'a>> {
    if golden.source_unknown {
        for (new_id, new_comp) in new {
            if let Some(bin_ver) = new_comp.binaries.get(golden_id) {
                return Some(ComponentMatch {
                    id: new_id.as_str(),
                    component: new_comp,
                    version: bin_ver.as_str(),
                });
            }
        }
        None
    } else {
        let (new_id, new_comp) = new.get_key_value(golden_id)?;
        Some(ComponentMatch {
            id: new_id.as_str(),
            component: new_comp,
            version: new_comp.version.as_str(),
        })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinaryChanges {
    pub abi_renames: Vec<(String, String)>,
    pub dropped: Vec<String>,
}

pub fn binary_changes(old: &GoldenComponent, new: &GoldenComponent) -> BinaryChanges {
    let vanished: Vec<&String> = old
        .binaries
        .keys()
        .filter(|name| !new.binaries.contains_key(*name))
        .collect();
    let appeared: Vec<&String> = new
        .binaries
        .keys()
        .filter(|name| !old.binaries.contains_key(*name))
        .collect();

    let mut used = vec![false; appeared.len()];
    let mut abi_renames = Vec::new();
    let mut dropped = Vec::new();

    for old_name in vanished {
        if let Some((idx, new_name)) = appeared
            .iter()
            .enumerate()
            .find(|(idx, new_name)| !used[*idx] && is_abi_rename(old_name, new_name))
        {
            used[idx] = true;
            abi_renames.push((old_name.clone(), (*new_name).clone()));
        } else {
            dropped.push(old_name.clone());
        }
    }

    BinaryChanges {
        abi_renames,
        dropped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pkg(
        name: &str,
        version: &str,
        source: &str,
        source_version: Option<&str>,
        depends: &[&str],
    ) -> InstalledPackage {
        InstalledPackage {
            name: name.to_string(),
            version: version.to_string(),
            source: source.to_string(),
            source_version: source_version.map(ToString::to_string),
            arch: Some("amd64".to_string()),
            depends: depends.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn abi_rename_detection() {
        assert!(is_abi_rename(
            "linux-image-7.2.15-x64v3-xanmod1",
            "linux-image-7.3.3-x64v3-xanmod1"
        ));
        assert!(is_abi_rename("libfoo1", "libfoo2"));
        assert!(is_abi_rename("libpcap0.8", "libpcap0.8t64"));

        assert!(!is_abi_rename("libfoo1", "libbar1"));
        assert!(!is_abi_rename("libfoo1", "libfoo1"));
        assert!(!is_abi_rename(
            "linux-image-7.3.3-x64v3-xanmod1",
            "linux-image-7.3.3-x64v3-xanmod1"
        ));
    }

    #[test]
    fn xanmod_kernel_line_grouping_and_third_party_origin() {
        let mut packages = BTreeMap::new();
        packages.insert(
            "linux-xanmod-x64v3".to_string(),
            pkg(
                "linux-xanmod-x64v3",
                "7.3.3-xanmod1-0",
                "linux-xanmod-x64v3",
                None,
                &[
                    "linux-headers-7.3.3-x64v3-xanmod1",
                    "linux-image-7.3.3-x64v3-xanmod1",
                ],
            ),
        );
        packages.insert(
            "linux-image-7.3.3-x64v3-xanmod1".to_string(),
            pkg(
                "linux-image-7.3.3-x64v3-xanmod1",
                "7.3.3-x64v3-xanmod1-0~20261107.g2ef8115",
                "linux-7.3.3-x64v3-xanmod1",
                None,
                &[],
            ),
        );
        packages.insert(
            "linux-headers-7.3.3-x64v3-xanmod1".to_string(),
            pkg(
                "linux-headers-7.3.3-x64v3-xanmod1",
                "7.3.3-x64v3-xanmod1-0~20261107.g2ef8115",
                "linux-7.3.3-x64v3-xanmod1",
                None,
                &[],
            ),
        );

        let cfg = Components {
            kernel_lines: vec!["linux-xanmod-x64v3".to_string()],
            third_party: vec!["linux-xanmod-x64v3".to_string()],
        };

        let grouped = group_components(&packages, &cfg);
        assert_eq!(grouped.len(), 1);

        let line = grouped
            .get("linux-xanmod-x64v3")
            .expect("xanmod kernel line component");
        assert_eq!(line.version, "7.3.3-xanmod1-0");
        assert!(line.kernel_line);
        assert!(!line.source_unknown);
        assert_eq!(line.origin, Origin::ThirdParty);
        assert_eq!(line.binaries.len(), 3);
        assert_eq!(
            line.binaries.get("linux-xanmod-x64v3").map(String::as_str),
            Some("7.3.3-xanmod1-0")
        );
        assert_eq!(
            line.binaries
                .get("linux-image-7.3.3-x64v3-xanmod1")
                .map(String::as_str),
            Some("7.3.3-x64v3-xanmod1-0~20261107.g2ef8115")
        );
        assert_eq!(
            line.binaries
                .get("linux-headers-7.3.3-x64v3-xanmod1")
                .map(String::as_str),
            Some("7.3.3-x64v3-xanmod1-0~20261107.g2ef8115")
        );
    }

    #[test]
    fn debian_kernel_line_leaves_unclaimed_headers_in_source_component() {
        let mut packages = BTreeMap::new();
        packages.insert(
            "linux-image-amd64".to_string(),
            pkg(
                "linux-image-amd64",
                "7.3.3-1",
                "linux-signed-amd64",
                None,
                &["linux-image-7.3.3+deb14-amd64"],
            ),
        );
        packages.insert(
            "linux-image-7.3.3+deb14-amd64".to_string(),
            pkg(
                "linux-image-7.3.3+deb14-amd64",
                "7.3.3-1",
                "linux-signed-amd64",
                None,
                &[],
            ),
        );
        packages.insert(
            "linux-headers-amd64".to_string(),
            pkg("linux-headers-amd64", "7.3.3-1", "linux", None, &[]),
        );

        let cfg = Components::default();
        let grouped = group_components(&packages, &cfg);
        assert_eq!(grouped.len(), 2);

        let kline = grouped
            .get("linux-image-amd64")
            .expect("debian kernel line");
        assert!(kline.kernel_line);
        assert_eq!(kline.version, "7.3.3-1");
        assert_eq!(kline.origin, Origin::Debian);
        assert_eq!(
            kline.binaries,
            BTreeMap::from([
                (
                    "linux-image-7.3.3+deb14-amd64".to_string(),
                    "7.3.3-1".to_string()
                ),
                ("linux-image-amd64".to_string(), "7.3.3-1".to_string()),
            ])
        );

        let linux = grouped.get("linux").expect("linux component");
        assert!(!linux.kernel_line);
        assert_eq!(linux.version, "7.3.3-1");
        assert_eq!(
            linux.binaries,
            BTreeMap::from([("linux-headers-amd64".to_string(), "7.3.3-1".to_string())])
        );
    }

    #[test]
    fn source_grouping_prefers_source_version_and_picks_highest_candidate() {
        let mut packages = BTreeMap::new();
        packages.insert(
            "foo".to_string(),
            pkg("foo", "1.2-1", "foo", Some("1.2-1"), &[]),
        );
        packages.insert(
            "libfoo1".to_string(),
            pkg("libfoo1", "1.2-1+b1", "foo", Some("1.2-1"), &[]),
        );
        packages.insert(
            "bar-common".to_string(),
            pkg("bar-common", "2.0-1", "bar", None, &[]),
        );
        packages.insert(
            "bar-bin".to_string(),
            pkg("bar-bin", "2.0-2", "bar", None, &[]),
        );

        let grouped = group_components(&packages, &Components::default());
        let foo = grouped.get("foo").expect("foo component");
        assert_eq!(foo.version, "1.2-1");
        assert_eq!(
            foo.binaries,
            BTreeMap::from([
                ("foo".to_string(), "1.2-1".to_string()),
                ("libfoo1".to_string(), "1.2-1+b1".to_string()),
            ])
        );

        let bar = grouped.get("bar").expect("bar component");
        assert_eq!(bar.version, "2.0-2");
        assert_eq!(
            bar.binaries,
            BTreeMap::from([
                ("bar-bin".to_string(), "2.0-2".to_string()),
                ("bar-common".to_string(), "2.0-1".to_string()),
            ])
        );
    }

    #[test]
    fn find_match_handles_placeholders_and_normal_components() {
        let mut new_components = BTreeMap::new();
        new_components.insert(
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

        let placeholder = GoldenComponent {
            version: "1.10.5-2".to_string(),
            origin: Origin::Debian,
            kernel_line: false,
            source_unknown: true,
            binaries: BTreeMap::from([("libpcap0.8".to_string(), "1.10.5-2".to_string())]),
        };

        let matched =
            find_match("libpcap0.8", &placeholder, &new_components).expect("placeholder match");
        assert_eq!(matched.id, "libpcap");
        assert_eq!(matched.version, "1.10.5-2+b1");
        assert_eq!(matched.component, &new_components["libpcap"]);

        let gone_placeholder = GoldenComponent {
            version: "1.0-1".to_string(),
            origin: Origin::Debian,
            kernel_line: false,
            source_unknown: true,
            binaries: BTreeMap::from([("libgone0".to_string(), "1.0-1".to_string())]),
        };
        assert_eq!(
            find_match("libgone0", &gone_placeholder, &new_components),
            None
        );

        let normal_golden = GoldenComponent {
            version: "1.10.5-1".to_string(),
            origin: Origin::Debian,
            kernel_line: false,
            source_unknown: false,
            binaries: BTreeMap::from([("libpcap0.8".to_string(), "1.10.5-1".to_string())]),
        };
        let normal_match =
            find_match("libpcap", &normal_golden, &new_components).expect("normal id match");
        assert_eq!(normal_match.id, "libpcap");
        assert_eq!(normal_match.version, "1.10.5-2");
    }

    #[test]
    fn binary_changes_classifies_abi_renames_and_dropped_binaries() {
        let old_xanmod = GoldenComponent {
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
        };
        let new_xanmod = GoldenComponent {
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
        };

        let xanmod_changes = binary_changes(&old_xanmod, &new_xanmod);
        assert_eq!(
            xanmod_changes.abi_renames,
            vec![
                (
                    "linux-headers-7.2.15-x64v3-xanmod1".to_string(),
                    "linux-headers-7.3.3-x64v3-xanmod1".to_string()
                ),
                (
                    "linux-image-7.2.15-x64v3-xanmod1".to_string(),
                    "linux-image-7.3.3-x64v3-xanmod1".to_string()
                ),
            ]
        );
        assert!(xanmod_changes.dropped.is_empty());

        let old_pcap = GoldenComponent {
            version: "1.10.4-4".to_string(),
            origin: Origin::Debian,
            kernel_line: false,
            source_unknown: false,
            binaries: BTreeMap::from([
                ("libfoo-doc".to_string(), "1.10.4-4".to_string()),
                ("libpcap0.8".to_string(), "1.10.4-4".to_string()),
            ]),
        };
        let new_pcap = GoldenComponent {
            version: "1.10.5-2".to_string(),
            origin: Origin::Debian,
            kernel_line: false,
            source_unknown: false,
            binaries: BTreeMap::from([
                ("libpcap-extra".to_string(), "1.10.5-2".to_string()),
                ("libpcap0.8t64".to_string(), "1.10.5-2".to_string()),
            ]),
        };

        let pcap_changes = binary_changes(&old_pcap, &new_pcap);
        assert_eq!(
            pcap_changes.abi_renames,
            vec![("libpcap0.8".to_string(), "libpcap0.8t64".to_string())]
        );
        assert_eq!(pcap_changes.dropped, vec!["libfoo-doc".to_string()]);
    }
}
