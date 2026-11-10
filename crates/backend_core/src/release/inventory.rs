use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub source: String,
    pub source_version: Option<String>,
    pub arch: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsoMeta {
    pub path: String,
    pub size_bytes: u64,
    pub vmlinuz_sha256: String,
    pub initrd_sha256: String,
    pub extracted_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootInfo {
    pub files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CalamaresModules {
    pub configs: Vec<String>,
    pub custom_modules: Vec<String>,
    pub presence: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomBinary {
    pub path: String,
    pub present: bool,
    pub sha256: String,
    pub size_bytes: u64,
    pub version: String,
    pub is_symlink: bool,
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsoInventory {
    pub iso: IsoMeta,
    pub os_release: BTreeMap<String, String>,
    pub boot: BootInfo,
    pub calamares_modules: CalamaresModules,
    pub custom_binaries: BTreeMap<String, CustomBinary>,
    pub packages: BTreeMap<String, InstalledPackage>,
}

fn parse_source_field(raw: &str, package_name: &str) -> (String, Option<String>) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return (package_name.to_string(), None);
    }

    if let Some(before_close) = trimmed.strip_suffix(')') {
        if let Some((src_name, ver_part)) = before_close.split_once('(') {
            let src = src_name.trim();
            let ver = ver_part.trim();
            let source = if src.is_empty() {
                package_name.to_string()
            } else {
                src.to_string()
            };
            let source_version = if ver.is_empty() {
                None
            } else {
                Some(ver.to_string())
            };
            return (source, source_version);
        }
    }

    (trimmed.to_string(), None)
}

pub fn parse_depends(raw: &str) -> Vec<String> {
    let mut names = Vec::new();

    for part in raw.split(',') {
        for alt in part.split('|') {
            let trimmed = alt.trim();
            let token = trimmed
                .split(|c: char| c.is_whitespace() || c == '(')
                .next()
                .unwrap_or("");
            let name = token.split_once(':').map_or(token, |(pkg, _)| pkg);
            if !name.is_empty() {
                names.push(name.to_string());
            }
        }
    }

    names.sort();
    names.dedup();
    names
}

fn flush_stanza(
    fields: &mut Vec<(String, String)>,
    packages: &mut BTreeMap<String, InstalledPackage>,
) {
    if fields.is_empty() {
        return;
    }

    let mut pkg_name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut status: Option<String> = None;
    let mut source_raw: Option<String> = None;
    let mut arch: Option<String> = None;
    let mut depends_raw: Option<String> = None;

    for (key, val) in fields.drain(..) {
        let trimmed_val = val.trim();
        if key.eq_ignore_ascii_case("Package") {
            if !trimmed_val.is_empty() {
                pkg_name = Some(trimmed_val.to_string());
            }
        } else if key.eq_ignore_ascii_case("Version") {
            if !trimmed_val.is_empty() {
                version = Some(trimmed_val.to_string());
            }
        } else if key.eq_ignore_ascii_case("Status") {
            status = Some(trimmed_val.to_string());
        } else if key.eq_ignore_ascii_case("Source") {
            if !trimmed_val.is_empty() {
                source_raw = Some(trimmed_val.to_string());
            }
        } else if key.eq_ignore_ascii_case("Architecture") {
            if !trimmed_val.is_empty() {
                arch = Some(trimmed_val.to_string());
            }
        } else if key.eq_ignore_ascii_case("Depends") && !trimmed_val.is_empty() {
            depends_raw = Some(trimmed_val.to_string());
        }
    }

    if let (Some(name), Some(ver), Some(stat)) = (pkg_name, version, status) {
        if stat.split_whitespace().any(|word| word == "installed") {
            let (source, source_version) = match source_raw {
                Some(ref raw) => parse_source_field(raw, &name),
                None => (name.clone(), None),
            };
            let depends = depends_raw
                .as_deref()
                .map(parse_depends)
                .unwrap_or_default();
            packages.insert(
                name.clone(),
                InstalledPackage {
                    name,
                    version: ver,
                    source,
                    source_version,
                    arch,
                    depends,
                },
            );
        }
    }
}

pub fn parse_dpkg_status(text: &str) -> BTreeMap<String, InstalledPackage> {
    let mut packages = BTreeMap::new();
    let mut fields: Vec<(String, String)> = Vec::new();

    for line in text.lines() {
        if line.trim().is_empty() {
            flush_stanza(&mut fields, &mut packages);
            continue;
        }

        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some((_, last_val)) = fields.last_mut() {
                last_val.push('\n');
                last_val.push_str(line);
            }
            continue;
        }

        if let Some((key, val)) = line.split_once(':') {
            fields.push((key.trim().to_string(), val.trim().to_string()));
        }
    }

    flush_stanza(&mut fields, &mut packages);
    packages
}

pub fn parse_os_release(text: &str) -> BTreeMap<String, String> {
    let mut result = BTreeMap::new();

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        if k.is_empty() {
            continue;
        }
        let v = v.trim();
        let unquoted = if v.len() >= 2
            && ((v.starts_with('"') && v.ends_with('"'))
                || (v.starts_with('\'') && v.ends_with('\'')))
        {
            &v[1..v.len() - 1]
        } else {
            v
        };
        result.insert(k.to_string(), unquoted.to_string());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_dpkg_status_four_stanza_fixture_and_edge_cases() {
        let fixture = "\
Package: old-removed
Status: deinstall ok config-files
Architecture: amd64
Version: 0.9-1

Package: libfoo1
Status: install ok installed
Architecture: amd64
Source: foo (1.2-1)
Version: 1.2-1+b1
Description: foo runtime library

Package: bar-tools
Status: hold ok installed
Architecture: all
Source: bar
Description: bar utilities
 A multi-line description that continues here
 .
 Version: 9.9.9-fake-in-description
 Status: deinstall ok config-files
Version: 2.0-3

Package: plain-pkg
Status: install ok installed
Architecture: amd64
Version: 3.4.5-1
";

        let pkgs = parse_dpkg_status(fixture);
        assert_eq!(pkgs.len(), 3);
        assert!(!pkgs.contains_key("old-removed"));

        assert_eq!(
            pkgs.get("libfoo1"),
            Some(&InstalledPackage {
                name: "libfoo1".to_string(),
                version: "1.2-1+b1".to_string(),
                source: "foo".to_string(),
                source_version: Some("1.2-1".to_string()),
                arch: Some("amd64".to_string()),
                depends: Vec::new(),
            })
        );

        assert_eq!(
            pkgs.get("bar-tools"),
            Some(&InstalledPackage {
                name: "bar-tools".to_string(),
                version: "2.0-3".to_string(),
                source: "bar".to_string(),
                source_version: None,
                arch: Some("all".to_string()),
                depends: Vec::new(),
            })
        );

        assert_eq!(
            pkgs.get("plain-pkg"),
            Some(&InstalledPackage {
                name: "plain-pkg".to_string(),
                version: "3.4.5-1".to_string(),
                source: "plain-pkg".to_string(),
                source_version: None,
                arch: Some("amd64".to_string()),
                depends: Vec::new(),
            })
        );
    }

    #[test]
    fn parse_dpkg_status_drops_half_installed_and_replaces_duplicate_package() {
        let text = "\
Package: alpha
Status: install ok half-installed
Version: 1.0-1

Package: dup-pkg
Status: install ok installed
Version: 1.0-1

Package: dup-pkg
Status: install ok installed
Source: dup-src (2.0-1)
Version: 2.0-1+b2
";

        let pkgs = parse_dpkg_status(text);
        assert_eq!(pkgs.len(), 1);
        assert!(!pkgs.contains_key("alpha"));
        assert_eq!(
            pkgs.get("dup-pkg"),
            Some(&InstalledPackage {
                name: "dup-pkg".to_string(),
                version: "2.0-1+b2".to_string(),
                source: "dup-src".to_string(),
                source_version: Some("2.0-1".to_string()),
                arch: None,
                depends: Vec::new(),
            })
        );
    }

    #[test]
    fn parse_depends_extracts_sorted_deduped_package_names() {
        let raw = "linux-image-7.3.3-x64v3-xanmod1 (= 7.3.3-x64v3-xanmod1-0~20261107.g2ef8115), linux-headers-7.3.3-x64v3-xanmod1 | foo, libc6:any (>= 2.36)";
        assert_eq!(
            parse_depends(raw),
            vec![
                "foo".to_string(),
                "libc6".to_string(),
                "linux-headers-7.3.3-x64v3-xanmod1".to_string(),
                "linux-image-7.3.3-x64v3-xanmod1".to_string(),
            ]
        );
        assert!(parse_depends("").is_empty());
    }

    #[test]
    fn parse_dpkg_status_captures_multiline_depends_and_omits_empty_in_json() {
        let text = "\
Package: linux-xanmod-x64v3
Status: install ok installed
Architecture: amd64
Version: 7.3.3-xanmod1-0
Depends: linux-image-7.3.3-x64v3-xanmod1 (= 7.3.3-x64v3-xanmod1-0~20261107.g2ef8115),
 linux-headers-7.3.3-x64v3-xanmod1 (= 7.3.3-x64v3-xanmod1-0~20261107.g2ef8115)

Package: standalone-pkg
Status: install ok installed
Architecture: amd64
Version: 1.0-1
";

        let pkgs = parse_dpkg_status(text);
        let kernel = pkgs.get("linux-xanmod-x64v3").expect("kernel meta present");
        assert_eq!(
            kernel.depends,
            vec![
                "linux-headers-7.3.3-x64v3-xanmod1".to_string(),
                "linux-image-7.3.3-x64v3-xanmod1".to_string(),
            ]
        );

        let standalone = pkgs.get("standalone-pkg").expect("standalone-pkg present");
        assert!(standalone.depends.is_empty());

        let json_val = serde_json::to_value(standalone).expect("serialize standalone-pkg");
        assert!(
            json_val.get("depends").is_none(),
            "empty depends should be skipped in JSON: {json_val}"
        );
    }

    #[test]
    fn parse_os_release_strips_quotes_and_skips_comments_and_blanks() {
        let text = "\
# Comment line
NAME=\"NovOS\"
PRETTY_NAME='NovOS 2026.10 (Nova)'

ID=novos
ANSI_COLOR=\"38;2;255;107;26\"
EMPTY_QUOTED=\"\"
";

        let parsed = parse_os_release(text);
        assert_eq!(parsed.get("NAME").map(String::as_str), Some("NovOS"));
        assert_eq!(
            parsed.get("PRETTY_NAME").map(String::as_str),
            Some("NovOS 2026.10 (Nova)")
        );
        assert_eq!(parsed.get("ID").map(String::as_str), Some("novos"));
        assert_eq!(
            parsed.get("ANSI_COLOR").map(String::as_str),
            Some("38;2;255;107;26")
        );
        assert_eq!(parsed.get("EMPTY_QUOTED").map(String::as_str), Some(""));
        assert_eq!(parsed.len(), 5);
    }
}
