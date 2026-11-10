use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Component, Path, PathBuf};

use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};

use crate::error::{Result, TrackerError};
use crate::release::inventory::{
    parse_dpkg_status, parse_os_release, BootInfo, CalamaresModules, CustomBinary,
    InstalledPackage, IsoInventory, IsoMeta,
};

pub const COSMIC_EPOCH_COMPONENTS: [&str; 8] = [
    "cosmic-comp",
    "cosmic-session",
    "cosmic-greeter",
    "cosmic-panel",
    "cosmic-applets",
    "cosmic-files",
    "cosmic-term",
    "cosmic-settings",
];

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut prefix: Option<std::ffi::OsString> = None;
    let mut has_root = false;
    let mut stack: Vec<std::ffi::OsString> = Vec::new();

    for comp in path.components() {
        match comp {
            Component::Prefix(p) => {
                prefix = Some(p.as_os_str().to_os_string());
            }
            Component::RootDir => {
                has_root = true;
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if stack
                    .last()
                    .is_some_and(|last| last != std::ffi::OsStr::new(".."))
                {
                    stack.pop();
                } else if !has_root {
                    stack.push(std::ffi::OsString::from(".."));
                }
            }
            Component::Normal(part) => {
                stack.push(part.to_os_string());
            }
        }
    }

    let mut out = PathBuf::new();
    if let Some(p) = prefix {
        out.push(p);
    }
    if has_root {
        out.push(Component::RootDir.as_os_str());
    }
    for part in stack {
        out.push(part);
    }
    if out.as_os_str().is_empty() {
        out.push(".");
    }
    out
}

/// Resolve multi-hop relative and absolute symlinks strictly inside `root`.
/// Never follows a symlink to a host path outside `root`.
pub fn resolve_in_root(root: &Path, entry: &Path) -> PathBuf {
    let root_norm = normalize_lexically(root);
    let mut current = normalize_lexically(entry);
    let mut visited = HashSet::new();

    while fs::symlink_metadata(&current)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
    {
        if !visited.insert(current.clone()) {
            break;
        }
        let Ok(target) = fs::read_link(&current) else {
            break;
        };
        let candidate = if target.is_absolute() {
            let rel: PathBuf = target
                .components()
                .filter(|c| !matches!(c, Component::RootDir | Component::Prefix(_)))
                .collect();
            normalize_lexically(&root_norm.join(rel))
        } else {
            let parent = current.parent().unwrap_or_else(|| Path::new(""));
            normalize_lexically(&parent.join(target))
        };

        if candidate.starts_with(&root_norm) {
            current = candidate;
        } else if let Some(name) = candidate.file_name() {
            current = root_norm.join(name);
        } else {
            current = root_norm.clone();
        }
    }

    current
}

/// Compute the lowercase hex SHA-256 digest of `path` using streaming 64 KiB reads.
pub fn sha256_file(path: &Path) -> Result<String> {
    let mut file = File::open(path).map_err(|source| TrackerError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];

    loop {
        match file.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => hasher.update(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(source) => {
                return Err(TrackerError::Io {
                    path: path.to_path_buf(),
                    source,
                });
            }
        }
    }

    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    Ok(hex)
}

/// Read `<project_root>/Cargo.toml` and each literal `workspace.members` member manifest to
/// build a `package.name -> package.version` lookup map.
///
/// Supports both string versions and `version = { workspace = true }` (resolved from
/// `[workspace.package].version`). Never errors; missing or unparsable manifests are skipped.
pub fn workspace_crate_versions(project_root: &Path) -> BTreeMap<String, String> {
    let mut versions = BTreeMap::new();
    let root_cargo = project_root.join("Cargo.toml");
    let Ok(root_text) = fs::read_to_string(&root_cargo) else {
        return versions;
    };
    let Ok(root_val) = toml::from_str::<toml::Value>(&root_text) else {
        return versions;
    };
    let Some(ws_table) = root_val.get("workspace").and_then(toml::Value::as_table) else {
        return versions;
    };

    let ws_pkg_version = ws_table
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|pkg| pkg.get("version"))
        .and_then(toml::Value::as_str)
        .filter(|s| !s.is_empty());

    let Some(members) = ws_table.get("members").and_then(toml::Value::as_array) else {
        return versions;
    };

    for member_entry in members {
        let Some(member_rel) = member_entry.as_str() else {
            continue;
        };
        let member_cargo = project_root.join(member_rel).join("Cargo.toml");
        let Ok(member_text) = fs::read_to_string(&member_cargo) else {
            continue;
        };
        let Ok(member_val) = toml::from_str::<toml::Value>(&member_text) else {
            continue;
        };
        let Some(pkg) = member_val.get("package").and_then(toml::Value::as_table) else {
            continue;
        };
        let Some(name) = pkg
            .get("name")
            .and_then(toml::Value::as_str)
            .filter(|s| !s.is_empty())
        else {
            continue;
        };

        let resolved_ver = match pkg.get("version") {
            Some(toml::Value::String(ver)) if !ver.is_empty() => Some(ver.clone()),
            Some(toml::Value::Table(tbl))
                if tbl.get("workspace").and_then(toml::Value::as_bool) == Some(true) =>
            {
                ws_pkg_version.map(ToOwned::to_owned)
            }
            _ => None,
        };

        if let Some(version) = resolved_ver {
            versions.insert(name.to_string(), version);
        }
    }

    versions
}

/// Scan `usr/local/bin` and `usr/local/sbin` under `root` for custom binaries.
///
/// Note: the version regex uses `(?-u)` so `\b` is ASCII-only, whereas Python's `latin1`
/// text decoding treats non-ASCII letters (`0xC0..=0xFF` etc.) as word characters; the
/// difference only matters for non-ASCII bytes immediately adjacent to the binary name or
/// version string.
pub fn scan_custom_binaries(
    root: &Path,
    crate_versions: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, CustomBinary>> {
    let mut custom_binaries = BTreeMap::new();

    for rel_dir in ["usr/local/bin", "usr/local/sbin"] {
        let dir_path = root.join(rel_dir);
        if !dir_path.is_dir() {
            continue;
        }

        let read_dir = fs::read_dir(&dir_path).map_err(|source| TrackerError::Io {
            path: dir_path.clone(),
            source,
        })?;
        let mut entries: Vec<(String, PathBuf)> = Vec::new();
        for entry_res in read_dir {
            let entry = entry_res.map_err(|source| TrackerError::Io {
                path: dir_path.clone(),
                source,
            })?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let raw_path = dir_path.join(entry.file_name());
            entries.push((name, raw_path));
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        for (entry_name, raw_entry) in entries {
            let abs_in_os = format!("/{rel_dir}/{entry_name}");
            let is_symlink = fs::symlink_metadata(&raw_entry)
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false);
            let resolved = resolve_in_root(root, &raw_entry);
            if !resolved.is_file() {
                continue;
            }

            let digest = sha256_file(&resolved)?;
            let size_bytes = fs::metadata(&resolved)
                .map_err(|source| TrackerError::Io {
                    path: resolved.clone(),
                    source,
                })?
                .len();

            let resolved_name = resolved
                .file_name()
                .map(|n| n.to_string_lossy().into_owned());
            let mut detected_ver = crate_versions
                .get(&entry_name)
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    resolved_name
                        .as_deref()
                        .and_then(|rn| crate_versions.get(rn))
                        .filter(|s| !s.is_empty())
                })
                .cloned()
                .unwrap_or_else(|| "0.1.0".to_string());

            let mut flags = Vec::new();
            if let Ok(raw_bytes) = fs::read(&resolved) {
                if raw_bytes
                    .windows(b"--pre-packages".len())
                    .any(|w| w == b"--pre-packages")
                {
                    flags.push("--pre-packages".to_string());
                }

                let pattern = format!(
                    r"(?-u)\b{}\s+v?(\d+\.\d+\.\d+(?:[-+~][0-9A-Za-z._-]+)?)\b",
                    regex::escape(&entry_name)
                );
                if let Ok(re) = regex::bytes::Regex::new(&pattern) {
                    if let Some(caps) = re.captures(&raw_bytes) {
                        if let Some(m) = caps.get(1) {
                            detected_ver = String::from_utf8_lossy(m.as_bytes()).into_owned();
                        }
                    }
                }
            }

            custom_binaries.insert(
                entry_name,
                CustomBinary {
                    path: abs_in_os,
                    present: true,
                    sha256: digest,
                    size_bytes,
                    version: detected_ver,
                    is_symlink,
                    flags,
                },
            );
        }
    }

    Ok(custom_binaries)
}

fn is_file_or_symlink(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .map(|m| m.file_type().is_file() || m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Build an [`IsoInventory`] from an already-extracted ISO root directory.
///
/// Synthetic COSMIC component entries (`COSMIC_EPOCH_COMPONENTS`) record
/// `source = "cosmic-epoch-monorepo"` and `source_version = Some(V)` because the monorepo
/// package ships those binaries; under contract §4.1, component identity and version come
/// from `Source`, grouping all monorepo-provided binaries into the `cosmic-epoch-monorepo`
/// component.
pub fn inventory_from_root(root: &Path, iso: IsoMeta, project_root: &Path) -> Result<IsoInventory> {
    let status_file = root.join("var/lib/dpkg/status");
    if !status_file.is_file() {
        return Err(TrackerError::ValidationError(format!(
            "var/lib/dpkg/status is missing from the extracted ISO root {}",
            root.display()
        )));
    }

    let mut os_release_entry = resolve_in_root(root, &root.join("etc/os-release"));
    if !os_release_entry.is_file() {
        os_release_entry = root.join("usr/lib/os-release");
    }
    let os_release = if os_release_entry.is_file() {
        let raw = fs::read(&os_release_entry).map_err(|source| TrackerError::Io {
            path: os_release_entry,
            source,
        })?;
        parse_os_release(&String::from_utf8_lossy(&raw))
    } else {
        BTreeMap::new()
    };

    let status_raw = fs::read(&status_file).map_err(|source| TrackerError::Io {
        path: status_file,
        source,
    })?;
    let mut packages = parse_dpkg_status(&String::from_utf8_lossy(&status_raw));

    if !packages.contains_key("live-boot") {
        let lb_changelog = root.join("usr/share/doc/live-boot/changelog.gz");
        let lb_bin = root.join("usr/bin/live-boot");
        if lb_changelog.is_file() && is_file_or_symlink(&lb_bin) {
            let gz_file = File::open(&lb_changelog).map_err(|source| TrackerError::Io {
                path: lb_changelog.clone(),
                source,
            })?;
            let mut reader = BufReader::new(GzDecoder::new(gz_file));
            let mut first_line_bytes = Vec::new();
            reader
                .read_until(b'\n', &mut first_line_bytes)
                .map_err(|source| TrackerError::Io {
                    path: lb_changelog,
                    source,
                })?;
            let first_line_cow = String::from_utf8_lossy(&first_line_bytes);
            let first_line = first_line_cow.trim();
            if let Some((_, after_open)) = first_line.split_once('(') {
                if let Some((inside, _)) = after_open.split_once(')') {
                    let version = inside.trim();
                    if !version.is_empty() {
                        packages.insert(
                            "live-boot".to_string(),
                            InstalledPackage {
                                name: "live-boot".to_string(),
                                version: version.to_string(),
                                source: "live-boot".to_string(),
                                source_version: None,
                                arch: None,
                                depends: Vec::new(),
                            },
                        );
                    }
                }
            }
        }
    }

    // Source is the monorepo because `cosmic-epoch-monorepo` ships these binaries; setting
    // `source = "cosmic-epoch-monorepo"` makes them members of the monorepo component under §4.1.
    if let Some(epoch_ver) = packages
        .get("cosmic-epoch-monorepo")
        .map(|pkg| pkg.version.clone())
    {
        for comp in COSMIC_EPOCH_COMPONENTS {
            if !packages.contains_key(comp) {
                let comp_bin = root.join("usr/bin").join(comp);
                if is_file_or_symlink(&comp_bin) {
                    packages.insert(
                        comp.to_string(),
                        InstalledPackage {
                            name: comp.to_string(),
                            version: epoch_ver.clone(),
                            source: "cosmic-epoch-monorepo".to_string(),
                            source_version: Some(epoch_ver.clone()),
                            arch: None,
                            depends: Vec::new(),
                        },
                    );
                }
            }
        }
    }

    let custom_binaries = scan_custom_binaries(root, &workspace_crate_versions(project_root))?;

    let mut calamares_configs = Vec::new();
    let mut calamares_presence = BTreeMap::new();
    let cal_mod_dir = root.join("etc/calamares/modules");
    if cal_mod_dir.is_dir() {
        let read_dir = fs::read_dir(&cal_mod_dir).map_err(|source| TrackerError::Io {
            path: cal_mod_dir.clone(),
            source,
        })?;
        for entry_res in read_dir {
            let entry = entry_res.map_err(|source| TrackerError::Io {
                path: cal_mod_dir.clone(),
                source,
            })?;
            let name = entry.file_name().to_string_lossy().into_owned();
            calamares_configs.push(name);
        }
        calamares_configs.sort();
        for name in &calamares_configs {
            calamares_presence.insert(name.clone(), true);
        }
    }

    let mut custom_calamares_modules = Vec::new();
    let usr_cal_dir = root.join("usr/lib/calamares/modules");
    if usr_cal_dir.is_dir() {
        let read_dir = fs::read_dir(&usr_cal_dir).map_err(|source| TrackerError::Io {
            path: usr_cal_dir.clone(),
            source,
        })?;
        for entry_res in read_dir {
            let entry = entry_res.map_err(|source| TrackerError::Io {
                path: usr_cal_dir.clone(),
                source,
            })?;
            if entry.path().is_dir() {
                let name = entry.file_name().to_string_lossy().into_owned();
                custom_calamares_modules.push(name);
            }
        }
        custom_calamares_modules.sort();
        for name in &custom_calamares_modules {
            calamares_presence.insert(name.clone(), true);
        }
    }

    let mut boot_files = Vec::new();
    let boot_dir = root.join("boot");
    if boot_dir.is_dir() {
        let read_dir = fs::read_dir(&boot_dir).map_err(|source| TrackerError::Io {
            path: boot_dir.clone(),
            source,
        })?;
        for entry_res in read_dir {
            let entry = entry_res.map_err(|source| TrackerError::Io {
                path: boot_dir.clone(),
                source,
            })?;
            if is_file_or_symlink(&entry.path()) {
                let name = entry.file_name().to_string_lossy().into_owned();
                boot_files.push(name);
            }
        }
        boot_files.sort();
    }

    Ok(IsoInventory {
        iso,
        os_release,
        boot: BootInfo { files: boot_files },
        calamares_modules: CalamaresModules {
            configs: calamares_configs,
            custom_modules: custom_calamares_modules,
            presence: calamares_presence,
        },
        custom_binaries,
        packages,
    })
}

#[cfg(test)]
pub(crate) fn write_fixture_root(dir: &Path) {
    use std::io::Write;
    use std::os::unix::fs::symlink;

    use flate2::write::GzEncoder;
    use flate2::Compression;

    let dpkg_dir = dir.join("var/lib/dpkg");
    fs::create_dir_all(&dpkg_dir).expect("create var/lib/dpkg");
    fs::write(
        dpkg_dir.join("status"),
        "\
Package: cosmic-epoch-monorepo
Status: install ok installed
Architecture: amd64
Version: 1.0.0~alpha7-1

Package: libfoo1
Status: install ok installed
Architecture: amd64
Source: foo (1.2-1)
Version: 1.2-1

Package: base-files
Status: install ok installed
Architecture: amd64
Version: 14
",
    )
    .expect("write dpkg status");

    let usr_bin = dir.join("usr/bin");
    fs::create_dir_all(&usr_bin).expect("create usr/bin");
    fs::write(usr_bin.join("cosmic-comp"), b"ELF cosmic-comp").expect("write cosmic-comp");
    fs::write(usr_bin.join("cosmic-term"), b"ELF cosmic-term").expect("write cosmic-term");
    symlink("cosmic-comp", usr_bin.join("cosmic-panel")).expect("symlink cosmic-panel");
    fs::write(usr_bin.join("live-boot"), b"#!/bin/sh\n").expect("write live-boot");

    let lb_doc_dir = dir.join("usr/share/doc/live-boot");
    fs::create_dir_all(&lb_doc_dir).expect("create live-boot doc dir");
    let gz_file = File::create(lb_doc_dir.join("changelog.gz")).expect("create changelog.gz");
    let mut encoder = GzEncoder::new(gz_file, Compression::default());
    encoder
        .write_all(b"live-boot (1:20250815) unstable; urgency=medium\n\n  * Initial release.\n")
        .expect("write changelog.gz body");
    encoder.finish().expect("finish changelog.gz");

    let usr_lib = dir.join("usr/lib");
    fs::create_dir_all(&usr_lib).expect("create usr/lib");
    fs::write(usr_lib.join("os-release"), "NAME=\"NovOS\"\nID=novos\n")
        .expect("write usr/lib/os-release");
    let etc_dir = dir.join("etc");
    fs::create_dir_all(&etc_dir).expect("create etc");
    symlink("../usr/lib/os-release", etc_dir.join("os-release")).expect("symlink etc/os-release");

    let usr_local_bin = dir.join("usr/local/bin");
    fs::create_dir_all(&usr_local_bin).expect("create usr/local/bin");
    fs::write(usr_local_bin.join("tool-a"), b"tool-a v1.2.3").expect("write tool-a");

    let cal_etc = dir.join("etc/calamares/modules");
    fs::create_dir_all(&cal_etc).expect("create etc/calamares/modules");
    fs::write(cal_etc.join("welcome.conf"), b"---\n").expect("write welcome.conf");
    fs::write(cal_etc.join("users.conf"), b"---\n").expect("write users.conf");

    let cal_usr = dir.join("usr/lib/calamares/modules/limine-bootloader");
    fs::create_dir_all(&cal_usr).expect("create limine-bootloader dir");

    let boot_dir = dir.join("boot");
    fs::create_dir_all(&boot_dir).expect("create boot");
    fs::write(
        boot_dir.join("vmlinuz-7.3.3-x64v3-xanmod1"),
        b"kernel-image",
    )
    .expect("write vmlinuz-7.3.3-x64v3-xanmod1");
    symlink("vmlinuz-7.3.3-x64v3-xanmod1", boot_dir.join("vmlinuz")).expect("symlink vmlinuz");
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::symlink;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn resolve_in_root_confines_absolute_relative_escape_and_cycles() {
        let tmp = tempdir().expect("create tempdir");
        let root = tmp.path();
        let bin_dir = root.join("usr/local/bin");
        fs::create_dir_all(&bin_dir).expect("create usr/local/bin");

        let tool_a = bin_dir.join("tool-a");
        fs::write(&tool_a, b"binary-a").expect("write tool-a");

        // Absolute symlink inside root -> resolves relative to root
        let abs_link = bin_dir.join("link");
        symlink("/usr/local/bin/tool-a", &abs_link).expect("create abs symlink");
        assert_eq!(resolve_in_root(root, &abs_link), tool_a);

        // Relative symlink that climbs out of root -> confined to root/passwd
        let escape_link = bin_dir.join("escape");
        symlink("../../../../../etc/passwd", &escape_link).expect("create escaping symlink");
        assert_eq!(resolve_in_root(root, &escape_link), root.join("passwd"));

        // Two-link cycle terminates without hanging
        let cycle_a = bin_dir.join("cycle-a");
        let cycle_b = bin_dir.join("cycle-b");
        symlink("cycle-b", &cycle_a).expect("create cycle-a");
        symlink("cycle-a", &cycle_b).expect("create cycle-b");
        let resolved_cycle = resolve_in_root(root, &cycle_a);
        assert!(resolved_cycle == cycle_a || resolved_cycle == cycle_b);
    }

    #[test]
    fn scan_custom_binaries_extracts_versions_flags_symlinks_and_precedence() {
        let tmp = tempdir().expect("create tempdir");
        let root = tmp.path();
        let bin_dir = root.join("usr/local/bin");
        let sbin_dir = root.join("usr/local/sbin");
        fs::create_dir_all(&bin_dir).expect("create bin");
        fs::create_dir_all(&sbin_dir).expect("create sbin");

        let tool_a_bytes = b"ELF\0header tool-a v1.2.3-rc1 --pre-packages\0footer";
        let tool_a_path = bin_dir.join("tool-a");
        fs::write(&tool_a_path, tool_a_bytes).expect("write tool-a");
        let tool_a_sha = sha256_file(&tool_a_path).expect("hash tool-a");

        let tool_b_path = bin_dir.join("tool-b");
        fs::write(&tool_b_path, b"ELF\0no version string here").expect("write tool-b");

        let tool_c_path = bin_dir.join("tool-c");
        fs::write(&tool_c_path, b"ELF\0plain binary").expect("write tool-c");

        let link_path = bin_dir.join("link");
        symlink("/usr/local/bin/tool-a", &link_path).expect("create link to tool-a");

        // Escaping symlink in sbin whose confined target (root/passwd) does not exist
        let escape_sbin = sbin_dir.join("escape-sbin");
        symlink("../../../../../etc/passwd", &escape_sbin).expect("create escaping sbin link");

        // Entry in sbin with the same name as a bin entry replaces the bin entry
        let dup_bin = bin_dir.join("shared-tool");
        fs::write(&dup_bin, b"shared-tool 1.0.0").expect("write bin shared-tool");
        let dup_sbin = sbin_dir.join("shared-tool");
        fs::write(&dup_sbin, b"shared-tool v2.5.0 --pre-packages").expect("write sbin shared-tool");

        let mut crate_versions = BTreeMap::new();
        crate_versions.insert("tool-b".to_string(), "0.4.0".to_string());

        let scanned = scan_custom_binaries(root, &crate_versions).expect("scan custom binaries");

        let a = scanned.get("tool-a").expect("tool-a present");
        assert_eq!(a.path, "/usr/local/bin/tool-a");
        assert!(a.present);
        assert_eq!(a.sha256, tool_a_sha);
        assert_eq!(a.size_bytes, tool_a_bytes.len() as u64);
        assert_eq!(a.version, "1.2.3-rc1");
        assert!(!a.is_symlink);
        assert_eq!(a.flags, vec!["--pre-packages".to_string()]);

        let b = scanned.get("tool-b").expect("tool-b present");
        assert_eq!(b.version, "0.4.0");
        assert!(!b.is_symlink);
        assert!(b.flags.is_empty());

        let c = scanned.get("tool-c").expect("tool-c present");
        assert_eq!(c.version, "0.1.0");
        assert!(!c.is_symlink);
        assert!(c.flags.is_empty());

        let link = scanned.get("link").expect("link present");
        assert_eq!(link.path, "/usr/local/bin/link");
        assert!(link.is_symlink);
        assert_eq!(link.sha256, tool_a_sha);
        assert_eq!(link.version, "0.1.0");

        assert!(!scanned.contains_key("escape-sbin"));

        let shared = scanned.get("shared-tool").expect("shared-tool present");
        assert_eq!(shared.path, "/usr/local/sbin/shared-tool");
        assert_eq!(shared.version, "2.5.0");
        assert_eq!(shared.flags, vec!["--pre-packages".to_string()]);
    }

    #[test]
    fn sha256_file_matches_known_abc_digest() {
        let tmp = tempdir().expect("create tempdir");
        let file_path = tmp.path().join("abc.txt");
        fs::write(&file_path, b"abc").expect("write abc");

        let digest = sha256_file(&file_path).expect("sha256_file");
        assert_eq!(
            digest,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn workspace_crate_versions_handles_direct_inherited_missing_member_and_missing_root() {
        let empty_tmp = tempdir().expect("create empty tempdir");
        assert!(workspace_crate_versions(empty_tmp.path()).is_empty());

        let ws_tmp = tempdir().expect("create workspace tempdir");
        let ws_root = ws_tmp.path();
        fs::write(
            ws_root.join("Cargo.toml"),
            r#"
[workspace]
members = ["crates/direct-ver", "crates/inherited-ver", "crates/missing-member"]

[workspace.package]
version = "2.0.0"
"#,
        )
        .expect("write root Cargo.toml");

        let direct_dir = ws_root.join("crates/direct-ver");
        fs::create_dir_all(&direct_dir).expect("create direct-ver");
        fs::write(
            direct_dir.join("Cargo.toml"),
            r#"
[package]
name = "direct-tool"
version = "0.7.3"
"#,
        )
        .expect("write direct Cargo.toml");

        let inherited_dir = ws_root.join("crates/inherited-ver");
        fs::create_dir_all(&inherited_dir).expect("create inherited-ver");
        fs::write(
            inherited_dir.join("Cargo.toml"),
            r#"
[package]
name = "inherited-tool"
version.workspace = true
"#,
        )
        .expect("write inherited Cargo.toml");

        let versions = workspace_crate_versions(ws_root);
        assert_eq!(versions.len(), 2);
        assert_eq!(
            versions.get("direct-tool").map(String::as_str),
            Some("0.7.3")
        );
        assert_eq!(
            versions.get("inherited-tool").map(String::as_str),
            Some("2.0.0")
        );
    }

    fn dummy_iso_meta() -> IsoMeta {
        IsoMeta {
            path: "/tmp/novos.iso".to_string(),
            size_bytes: 1024,
            vmlinuz_sha256: String::new(),
            initrd_sha256: String::new(),
            extracted_at: "2026-11-10T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn inventory_from_root_builds_full_inventory_from_fixture() {
        let tmp = tempdir().expect("create tempdir");
        let root = tmp.path();
        write_fixture_root(root);

        let inv = inventory_from_root(root, dummy_iso_meta(), root).expect("inventory_from_root");

        // 3 dpkg packages + live-boot + 3 COSMIC components (comp, term, panel) = 7 packages
        assert_eq!(inv.packages.len(), 7);
        assert_eq!(
            inv.packages
                .get("cosmic-epoch-monorepo")
                .map(|p| p.version.as_str()),
            Some("1.0.0~alpha7-1")
        );
        assert_eq!(
            inv.packages.get("libfoo1"),
            Some(&InstalledPackage {
                name: "libfoo1".to_string(),
                version: "1.2-1".to_string(),
                source: "foo".to_string(),
                source_version: Some("1.2-1".to_string()),
                arch: Some("amd64".to_string()),
                depends: Vec::new(),
            })
        );
        assert_eq!(
            inv.packages.get("base-files").map(|p| p.version.as_str()),
            Some("14")
        );
        assert_eq!(
            inv.packages.get("live-boot"),
            Some(&InstalledPackage {
                name: "live-boot".to_string(),
                version: "1:20250815".to_string(),
                source: "live-boot".to_string(),
                source_version: None,
                arch: None,
                depends: Vec::new(),
            })
        );
        for comp in ["cosmic-comp", "cosmic-term", "cosmic-panel"] {
            assert_eq!(
                inv.packages.get(comp),
                Some(&InstalledPackage {
                    name: comp.to_string(),
                    version: "1.0.0~alpha7-1".to_string(),
                    source: "cosmic-epoch-monorepo".to_string(),
                    source_version: Some("1.0.0~alpha7-1".to_string()),
                    arch: None,
                    depends: Vec::new(),
                })
            );
        }
        assert!(!inv.packages.contains_key("cosmic-files"));

        assert_eq!(
            inv.os_release.get("NAME").map(String::as_str),
            Some("NovOS")
        );
        assert_eq!(inv.os_release.get("ID").map(String::as_str), Some("novos"));

        assert_eq!(
            inv.calamares_modules.configs,
            vec!["users.conf".to_string(), "welcome.conf".to_string()]
        );
        assert_eq!(
            inv.calamares_modules.custom_modules,
            vec!["limine-bootloader".to_string()]
        );
        assert_eq!(inv.calamares_modules.presence.len(), 3);
        assert_eq!(
            inv.calamares_modules.presence.get("users.conf"),
            Some(&true)
        );
        assert_eq!(
            inv.calamares_modules.presence.get("welcome.conf"),
            Some(&true)
        );
        assert_eq!(
            inv.calamares_modules.presence.get("limine-bootloader"),
            Some(&true)
        );

        assert_eq!(
            inv.boot.files,
            vec![
                "vmlinuz".to_string(),
                "vmlinuz-7.3.3-x64v3-xanmod1".to_string()
            ]
        );
        assert_eq!(
            inv.custom_binaries
                .get("tool-a")
                .map(|b| b.version.as_str()),
            Some("1.2.3")
        );
    }

    #[test]
    fn inventory_from_root_preserves_dpkg_registered_live_boot() {
        let tmp = tempdir().expect("create tempdir");
        let root = tmp.path();
        write_fixture_root(root);

        let status_path = root.join("var/lib/dpkg/status");
        let mut status_text = fs::read_to_string(&status_path).expect("read status");
        status_text.push_str(
            "\nPackage: live-boot\nStatus: install ok installed\nArchitecture: all\nVersion: 1:20240101\n",
        );
        fs::write(&status_path, status_text).expect("update status");

        let inv = inventory_from_root(root, dummy_iso_meta(), root).expect("inventory_from_root");
        let lb = inv.packages.get("live-boot").expect("live-boot present");
        assert_eq!(lb.version, "1:20240101");
        assert_eq!(lb.arch.as_deref(), Some("all"));
    }

    #[test]
    fn inventory_from_root_errors_when_dpkg_status_is_missing() {
        let tmp = tempdir().expect("create tempdir");
        let root = tmp.path();
        let err = inventory_from_root(root, dummy_iso_meta(), root)
            .expect_err("missing dpkg status should error");
        match err {
            TrackerError::ValidationError(msg) => {
                assert!(
                    msg.contains("var/lib/dpkg/status is missing from the extracted ISO root"),
                    "unexpected validation error message: {msg}"
                );
            }
            other => panic!("expected ValidationError, got: {other:?}"),
        }
    }
}
