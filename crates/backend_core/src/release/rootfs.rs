use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::{Result, TrackerError};
use crate::release::inventory::CustomBinary;

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
}
