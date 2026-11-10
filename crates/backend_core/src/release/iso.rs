use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};
use crate::release::inventory::{IsoInventory, IsoMeta};
use crate::release::rootfs::{self, COSMIC_EPOCH_COMPONENTS};

pub const PRODUCTION_SQUASHFS_ISO_PATH: &str = "/live/filesystem.squashfs";
pub const TEST_OVERLAY_LAYER_ISO_PATH: &str = "/live/zz-novos-test.squashfs";
pub const TEST_OVERLAY_MANIFEST_REL: &str = "usr/share/novos-test/overlay.json";

const UNSQUASHFS_FIXED_TARGETS: [&str; 9] = [
    "var/lib/dpkg/status",
    "etc/os-release",
    "usr/lib/os-release",
    "usr/local/bin",
    "usr/local/sbin",
    "etc/calamares",
    "usr/lib/calamares/modules",
    "usr/share/doc/live-boot",
    "usr/bin/live-boot",
];

/// Identity facts of a built Live ISO.
///
/// The production squashfs is byte-identical in a test ISO; `extra_live_layers`
/// (sorted ISO paths) is what tells them apart.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IsoIdentity {
    pub squashfs_sha256: String,
    pub extra_live_layers: Vec<String>,
}

impl IsoIdentity {
    pub fn is_production(&self) -> bool {
        self.extra_live_layers.is_empty() && !self.squashfs_sha256.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractedIso {
    pub inventory: IsoInventory,
    pub identity: IsoIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverlayManifest {
    pub schema: u32,
    #[serde(default)]
    pub production_iso: String,
    pub production_filesystem_squashfs_sha256: String,
}

fn stderr_tail(stderr: &[u8]) -> String {
    let stderr_text = String::from_utf8_lossy(stderr);
    let tail_lines: Vec<&str> = stderr_text.lines().rev().take(20).collect();
    tail_lines
        .into_iter()
        .rev()
        .collect::<Vec<&str>>()
        .join("\n")
        .trim()
        .to_string()
}

pub fn parse_live_squashfs_listing(stdout: &str) -> Vec<String> {
    let mut layers: Vec<String> = stdout
        .lines()
        .filter_map(|raw_line| {
            let line = raw_line.trim();
            if line.is_empty() {
                return None;
            }
            let unquoted = line
                .strip_prefix('\'')
                .and_then(|s| s.strip_suffix('\''))
                .unwrap_or(line);
            if unquoted.starts_with("/live/")
                && unquoted.ends_with(".squashfs")
                && unquoted != PRODUCTION_SQUASHFS_ISO_PATH
            {
                Some(unquoted.to_string())
            } else {
                None
            }
        })
        .collect();
    layers.sort();
    layers.dedup();
    layers
}

fn list_extra_live_layers(iso_abs: &Path) -> Result<Vec<String>> {
    let output = Command::new("xorriso")
        .arg("-indev")
        .arg(iso_abs)
        .arg("-find")
        .arg("/live")
        .arg("-name")
        .arg("*.squashfs")
        .arg("-type")
        .arg("f")
        .output()
        .map_err(|source| {
            if source.kind() == ErrorKind::NotFound {
                TrackerError::ValidationError(
                    "xorriso is not installed (needed to read the ISO)".to_string(),
                )
            } else {
                TrackerError::Io {
                    path: iso_abs.to_path_buf(),
                    source,
                }
            }
        })?;

    if !output.status.success() {
        return Err(TrackerError::ValidationError(format!(
            "xorriso could not list /live in {}: {}",
            iso_abs.display(),
            stderr_tail(&output.stderr)
        )));
    }

    let stdout_text = String::from_utf8_lossy(&output.stdout);
    Ok(parse_live_squashfs_listing(&stdout_text))
}

/// Extract both the [`IsoInventory`] and [`IsoIdentity`] from a built Live ISO.
///
/// The release flow calls this; a test ISO's inventory is identical to its production ISO's,
/// so callers that must not take a test ISO check `identity.is_production()`.
pub fn extract_iso(iso: &Path, project_root: &Path) -> Result<ExtractedIso> {
    if !iso.is_file() {
        return Err(TrackerError::ValidationError(format!(
            "ISO file not found: {}",
            iso.display()
        )));
    }

    let iso_abs = fs::canonicalize(iso).map_err(|source| TrackerError::Io {
        path: iso.to_path_buf(),
        source,
    })?;
    let size_bytes = fs::metadata(&iso_abs)
        .map_err(|source| TrackerError::Io {
            path: iso_abs.clone(),
            source,
        })?
        .len();

    let extra_live_layers = list_extra_live_layers(&iso_abs)?;

    let tmp_dir = tempfile::Builder::new()
        .prefix("neb-iso-")
        .tempdir()
        .map_err(|source| TrackerError::Io {
            path: std::env::temp_dir(),
            source,
        })?;
    let tmp_path = tmp_dir.path();
    let squashfs_path = tmp_path.join("filesystem.squashfs");
    let vmlinuz_path = tmp_path.join("vmlinuz");
    let initrd_path = tmp_path.join("initrd.img");
    let extract_dir = tmp_path.join("extract");

    let xorriso_output = Command::new("xorriso")
        .arg("-osirrox")
        .arg("on")
        .arg("-indev")
        .arg(&iso_abs)
        .arg("-extract")
        .arg(PRODUCTION_SQUASHFS_ISO_PATH)
        .arg(&squashfs_path)
        .arg("-extract")
        .arg("/live/vmlinuz")
        .arg(&vmlinuz_path)
        .arg("-extract")
        .arg("/live/initrd.img")
        .arg(&initrd_path)
        .output()
        .map_err(|source| {
            if source.kind() == ErrorKind::NotFound {
                TrackerError::ValidationError(
                    "xorriso is not installed (needed to read the ISO)".to_string(),
                )
            } else {
                TrackerError::Io {
                    path: iso_abs.clone(),
                    source,
                }
            }
        })?;

    if !xorriso_output.status.success() {
        return Err(TrackerError::ValidationError(format!(
            "xorriso failed on {}: {}",
            iso.display(),
            stderr_tail(&xorriso_output.stderr)
        )));
    }

    let squashfs_sha256 = rootfs::sha256_file(&squashfs_path)?;

    let vmlinuz_sha256 = if vmlinuz_path.is_file() {
        rootfs::sha256_file(&vmlinuz_path)?
    } else {
        String::new()
    };
    let initrd_sha256 = if initrd_path.is_file() {
        rootfs::sha256_file(&initrd_path)?
    } else {
        String::new()
    };

    let mut unsquash_cmd = Command::new("unsquashfs");
    unsquash_cmd.arg("-d").arg(&extract_dir).arg(&squashfs_path);
    for target in UNSQUASHFS_FIXED_TARGETS {
        unsquash_cmd.arg(target);
    }
    for comp in COSMIC_EPOCH_COMPONENTS {
        unsquash_cmd.arg(format!("usr/bin/{comp}"));
    }
    unsquash_cmd.arg("boot");

    let _unsquash_output = unsquash_cmd.output().map_err(|source| {
        if source.kind() == ErrorKind::NotFound {
            TrackerError::ValidationError(
                "unsquashfs is not installed (needed to read the ISO)".to_string(),
            )
        } else {
            TrackerError::Io {
                path: squashfs_path.clone(),
                source,
            }
        }
    })?;

    let meta = IsoMeta {
        path: iso_abs.to_string_lossy().into_owned(),
        size_bytes,
        vmlinuz_sha256,
        initrd_sha256,
        extracted_at: Utc::now().to_rfc3339_opts(SecondsFormat::Micros, false),
    };

    let inventory = rootfs::inventory_from_root(&extract_dir, meta, project_root)?;
    Ok(ExtractedIso {
        inventory,
        identity: IsoIdentity {
            squashfs_sha256,
            extra_live_layers,
        },
    })
}

/// Extract an [`IsoInventory`] from a built Live ISO using unprivileged `xorriso` and
/// `unsquashfs` inside a temporary directory.
///
/// Runs unprivileged with no loop or filesystem mounts; the temporary directory is deleted
/// automatically when dropped (both on success and on error), per contract §5.3.
pub fn extract_iso_inventory(iso: &Path, project_root: &Path) -> Result<IsoInventory> {
    extract_iso(iso, project_root).map(|e| e.inventory)
}

/// Read `/usr/share/novos-test/overlay.json` from `/live/zz-novos-test.squashfs` if present.
///
/// This is the host-side tie between a test ISO and its production ISO (NovOS STEP 15F),
/// used by record-phase for VM phases.
pub fn read_test_overlay_manifest(iso: &Path) -> Result<Option<OverlayManifest>> {
    if !iso.is_file() {
        return Err(TrackerError::ValidationError(format!(
            "ISO file not found: {}",
            iso.display()
        )));
    }

    let iso_abs = fs::canonicalize(iso).map_err(|source| TrackerError::Io {
        path: iso.to_path_buf(),
        source,
    })?;

    let extra_layers = list_extra_live_layers(&iso_abs)?;
    if !extra_layers
        .iter()
        .any(|l| l == TEST_OVERLAY_LAYER_ISO_PATH)
    {
        return Ok(None);
    }

    let tmp_dir = tempfile::Builder::new()
        .prefix("neb-iso-")
        .tempdir()
        .map_err(|source| TrackerError::Io {
            path: std::env::temp_dir(),
            source,
        })?;
    let tmp_path = tmp_dir.path();
    let layer_squashfs = tmp_path.join("layer.squashfs");
    let extract_dir = tmp_path.join("extract");

    let xorriso_output = Command::new("xorriso")
        .arg("-osirrox")
        .arg("on")
        .arg("-indev")
        .arg(&iso_abs)
        .arg("-extract")
        .arg(TEST_OVERLAY_LAYER_ISO_PATH)
        .arg(&layer_squashfs)
        .output()
        .map_err(|source| {
            if source.kind() == ErrorKind::NotFound {
                TrackerError::ValidationError(
                    "xorriso is not installed (needed to read the ISO)".to_string(),
                )
            } else {
                TrackerError::Io {
                    path: iso_abs.clone(),
                    source,
                }
            }
        })?;

    if !xorriso_output.status.success() {
        return Err(TrackerError::ValidationError(format!(
            "xorriso failed on {}: {}",
            iso.display(),
            stderr_tail(&xorriso_output.stderr)
        )));
    }

    let _unsquash_output = Command::new("unsquashfs")
        .arg("-d")
        .arg(&extract_dir)
        .arg(&layer_squashfs)
        .arg(TEST_OVERLAY_MANIFEST_REL)
        .output()
        .map_err(|source| {
            if source.kind() == ErrorKind::NotFound {
                TrackerError::ValidationError(
                    "unsquashfs is not installed (needed to read the ISO)".to_string(),
                )
            } else {
                TrackerError::Io {
                    path: layer_squashfs.clone(),
                    source,
                }
            }
        })?;

    let manifest_path = extract_dir.join(TEST_OVERLAY_MANIFEST_REL);
    if !manifest_path.is_file() {
        return Err(TrackerError::ValidationError(format!(
            "{} has {} but no /{}",
            iso.display(),
            TEST_OVERLAY_LAYER_ISO_PATH,
            TEST_OVERLAY_MANIFEST_REL
        )));
    }

    let raw = fs::read_to_string(&manifest_path).map_err(|source| TrackerError::Io {
        path: manifest_path,
        source,
    })?;

    let manifest: OverlayManifest = serde_json::from_str(&raw).map_err(|err| {
        TrackerError::ValidationError(format!("{}: invalid overlay.json: {err}", iso.display()))
    })?;

    if manifest
        .production_filesystem_squashfs_sha256
        .trim()
        .is_empty()
    {
        return Err(TrackerError::ValidationError(format!(
            "{}: overlay.json is missing production_filesystem_squashfs_sha256",
            iso.display()
        )));
    }

    Ok(Some(manifest))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::ffi::OsString;
    use std::path::PathBuf;

    use tempfile::tempdir;

    use super::*;

    fn list_neb_iso_temp_entries() -> HashSet<OsString> {
        let mut entries = HashSet::new();
        if let Ok(read_dir) = fs::read_dir(std::env::temp_dir()) {
            for entry in read_dir.flatten() {
                let name = entry.file_name();
                if name.to_string_lossy().starts_with("neb-iso-") {
                    entries.insert(name);
                }
            }
        }
        entries
    }

    #[test]
    fn parse_live_squashfs_listing_filters_sorts_and_dedups() {
        let two_line = "'/live/filesystem.squashfs'\n'/live/zz-novos-test.squashfs'\n";
        assert_eq!(
            parse_live_squashfs_listing(two_line),
            vec!["/live/zz-novos-test.squashfs".to_string()]
        );

        let prod_only = "  '/live/filesystem.squashfs'  \n";
        assert!(parse_live_squashfs_listing(prod_only).is_empty());

        let mixed = "\n\
            /live/b-extra.squashfs\n\
            '/live/a-extra.squashfs'\n\
            '/live/filesystem.squashfs'\n\
            '/live/initrd.img'\n\
            '/boot/other.squashfs'\n\
            /live/a-extra.squashfs\n\
            \n";
        assert_eq!(
            parse_live_squashfs_listing(mixed),
            vec![
                "/live/a-extra.squashfs".to_string(),
                "/live/b-extra.squashfs".to_string(),
            ]
        );
    }

    #[test]
    fn iso_identity_is_production_checks_layers_and_sha() {
        let prod = IsoIdentity {
            squashfs_sha256: "a".repeat(64),
            extra_live_layers: Vec::new(),
        };
        assert!(prod.is_production());

        let with_layer = IsoIdentity {
            squashfs_sha256: "a".repeat(64),
            extra_live_layers: vec![TEST_OVERLAY_LAYER_ISO_PATH.to_string()],
        };
        assert!(!with_layer.is_production());

        let empty_sha = IsoIdentity {
            squashfs_sha256: String::new(),
            extra_live_layers: Vec::new(),
        };
        assert!(!empty_sha.is_production());
    }

    #[test]
    fn read_test_overlay_manifest_missing_iso_returns_validation_error() {
        let tmp = tempdir().expect("create tempdir");
        let missing = tmp.path().join("nonexistent.iso");

        let err = read_test_overlay_manifest(&missing)
            .expect_err("missing ISO should fail with ValidationError");
        match err {
            TrackerError::ValidationError(msg) => {
                assert!(
                    msg.contains("ISO file not found"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected ValidationError, got: {other:?}"),
        }
    }

    #[test]
    fn missing_iso_path_returns_validation_error() {
        let tmp = tempdir().expect("create tempdir");
        let missing = tmp.path().join("nonexistent.iso");

        let err = extract_iso_inventory(&missing, tmp.path())
            .expect_err("missing ISO should fail with ValidationError");
        match err {
            TrackerError::ValidationError(msg) => {
                assert!(
                    msg.contains("ISO file not found"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected ValidationError, got: {other:?}"),
        }

        // Passing a directory instead of a regular file also fails with "ISO file not found"
        let dir_err = extract_iso_inventory(tmp.path(), tmp.path())
            .expect_err("directory path should fail with ValidationError");
        match dir_err {
            TrackerError::ValidationError(msg) => {
                assert!(
                    msg.contains("ISO file not found"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected ValidationError, got: {other:?}"),
        }
    }

    #[test]
    fn zero_bytes_iso_fails_and_cleans_up_temp_dir() {
        let tmp = tempdir().expect("create tempdir");
        let bad_iso = tmp.path().join("zero.iso");
        fs::write(&bad_iso, vec![0u8; 4096]).expect("write 4 KiB zero ISO");

        let before = list_neb_iso_temp_entries();
        let err = extract_iso_inventory(&bad_iso, tmp.path())
            .expect_err("zero-filled ISO should fail extraction");
        let after = list_neb_iso_temp_entries();

        match err {
            TrackerError::ValidationError(_) => {}
            other => panic!("expected ValidationError, got: {other:?}"),
        }

        let leaked: Vec<_> = after.difference(&before).collect();
        assert!(
            leaked.is_empty(),
            "neb-iso-* tempdir was not cleaned up: {leaked:?}"
        );
    }

    #[test]
    #[ignore = "needs xorriso and squashfs-tools; run: cargo test -p backend_core iso_round_trip -- --ignored"]
    fn iso_round_trip() {
        let tmp = tempdir().expect("create tempdir");
        let fixture_root = tmp.path().join("root");
        fs::create_dir_all(&fixture_root).expect("create fixture root");
        rootfs::write_fixture_root(&fixture_root);

        let iso_dir = tmp.path().join("isodir");
        let live_dir = iso_dir.join("live");
        fs::create_dir_all(&live_dir).expect("create isodir/live");

        let squashfs_out = live_dir.join("filesystem.squashfs");
        let mksquashfs_status = Command::new("mksquashfs")
            .arg(&fixture_root)
            .arg(&squashfs_out)
            .arg("-noappend")
            .arg("-quiet")
            .status()
            .expect("spawn mksquashfs");
        assert!(mksquashfs_status.success(), "mksquashfs failed");

        let vmlinuz_in = live_dir.join("vmlinuz");
        let initrd_in = live_dir.join("initrd.img");
        fs::write(&vmlinuz_in, b"kernel").expect("write vmlinuz");
        fs::write(&initrd_in, b"initrd").expect("write initrd.img");

        let iso_path = tmp.path().join("t.iso");
        let xorriso_status = Command::new("xorriso")
            .arg("-as")
            .arg("mkisofs")
            .arg("-quiet")
            .arg("-o")
            .arg(&iso_path)
            .arg(&iso_dir)
            .status()
            .expect("spawn xorriso -as mkisofs");
        assert!(xorriso_status.success(), "xorriso -as mkisofs failed");

        let extracted =
            extract_iso_inventory(&iso_path, &fixture_root).expect("extract_iso_inventory");
        let direct =
            rootfs::inventory_from_root(&fixture_root, extracted.iso.clone(), &fixture_root)
                .expect("direct inventory_from_root");

        assert_eq!(extracted.packages, direct.packages);
        assert_eq!(extracted.os_release, direct.os_release);
        assert_eq!(extracted.boot, direct.boot);
        assert_eq!(extracted.calamares_modules, direct.calamares_modules);
        assert_eq!(extracted.custom_binaries, direct.custom_binaries);

        let expected_kernel_sha = rootfs::sha256_file(&vmlinuz_in).expect("sha256 vmlinuz");
        let expected_initrd_sha = rootfs::sha256_file(&initrd_in).expect("sha256 initrd");
        assert_eq!(extracted.iso.vmlinuz_sha256, expected_kernel_sha);
        assert_eq!(extracted.iso.initrd_sha256, expected_initrd_sha);
        assert_eq!(
            extracted.iso.size_bytes,
            fs::metadata(&iso_path).expect("iso metadata").len()
        );
        assert!(PathBuf::from(&extracted.iso.path).is_absolute());
        assert!(
            extracted.iso.extracted_at.ends_with("+00:00"),
            "expected +00:00 UTC offset, got: {}",
            extracted.iso.extracted_at
        );
    }

    #[test]
    #[ignore = "needs xorriso and squashfs-tools; run: cargo test -p backend_core iso_identity_round_trip -- --ignored"]
    fn iso_identity_round_trip() {
        let tmp = tempdir().expect("create tempdir");
        let fixture_root = tmp.path().join("root");
        fs::create_dir_all(&fixture_root).expect("create fixture root");
        rootfs::write_fixture_root(&fixture_root);

        let iso_dir = tmp.path().join("isodir");
        let live_dir = iso_dir.join("live");
        fs::create_dir_all(&live_dir).expect("create isodir/live");

        let squashfs_out = live_dir.join("filesystem.squashfs");
        let mksquashfs_status = Command::new("mksquashfs")
            .arg(&fixture_root)
            .arg(&squashfs_out)
            .arg("-noappend")
            .arg("-quiet")
            .status()
            .expect("spawn mksquashfs");
        assert!(mksquashfs_status.success(), "mksquashfs failed");

        let expected_squashfs_sha =
            rootfs::sha256_file(&squashfs_out).expect("sha256 filesystem.squashfs");

        let vmlinuz_in = live_dir.join("vmlinuz");
        let initrd_in = live_dir.join("initrd.img");
        fs::write(&vmlinuz_in, b"kernel").expect("write vmlinuz");
        fs::write(&initrd_in, b"initrd").expect("write initrd.img");

        let prod_iso_path = tmp.path().join("prod.iso");
        let xorriso_status = Command::new("xorriso")
            .arg("-as")
            .arg("mkisofs")
            .arg("-quiet")
            .arg("-o")
            .arg(&prod_iso_path)
            .arg(&iso_dir)
            .status()
            .expect("spawn xorriso -as mkisofs");
        assert!(xorriso_status.success(), "xorriso -as mkisofs failed");

        let prod_extracted =
            extract_iso(&prod_iso_path, &fixture_root).expect("extract_iso on prod ISO");
        assert!(prod_extracted.identity.is_production());
        assert!(prod_extracted.identity.extra_live_layers.is_empty());
        assert_eq!(
            prod_extracted.identity.squashfs_sha256,
            expected_squashfs_sha
        );
        assert_eq!(
            read_test_overlay_manifest(&prod_iso_path).expect("read_test_overlay_manifest prod"),
            None
        );

        let overlay_root = tmp.path().join("overlay_root");
        let manifest_path = overlay_root.join(TEST_OVERLAY_MANIFEST_REL);
        fs::create_dir_all(manifest_path.parent().expect("manifest parent"))
            .expect("create overlay manifest dir");
        let manifest_json = format!(
            r#"{{"schema":1,"production_iso":"novos.iso","production_filesystem_squashfs_sha256":"{expected_squashfs_sha}","packages":[]}}"#
        );
        fs::write(&manifest_path, manifest_json).expect("write overlay.json");

        let overlay_squashfs_out = live_dir.join("zz-novos-test.squashfs");
        let overlay_mksquashfs_status = Command::new("mksquashfs")
            .arg(&overlay_root)
            .arg(&overlay_squashfs_out)
            .arg("-noappend")
            .arg("-quiet")
            .status()
            .expect("spawn mksquashfs for overlay");
        assert!(
            overlay_mksquashfs_status.success(),
            "overlay mksquashfs failed"
        );

        let test_iso_path = tmp.path().join("test.iso");
        let test_xorriso_status = Command::new("xorriso")
            .arg("-as")
            .arg("mkisofs")
            .arg("-quiet")
            .arg("-o")
            .arg(&test_iso_path)
            .arg(&iso_dir)
            .status()
            .expect("spawn xorriso -as mkisofs for test ISO");
        assert!(
            test_xorriso_status.success(),
            "test ISO xorriso -as mkisofs failed"
        );

        let test_extracted =
            extract_iso(&test_iso_path, &fixture_root).expect("extract_iso on test ISO");
        assert!(!test_extracted.identity.is_production());
        assert_eq!(
            test_extracted.identity.extra_live_layers,
            vec![TEST_OVERLAY_LAYER_ISO_PATH.to_string()]
        );
        assert_eq!(
            test_extracted.identity.squashfs_sha256,
            expected_squashfs_sha
        );
        assert_eq!(
            test_extracted.inventory.packages,
            prod_extracted.inventory.packages
        );

        let overlay_manifest = read_test_overlay_manifest(&test_iso_path)
            .expect("read_test_overlay_manifest test")
            .expect("expected Some(OverlayManifest)");
        assert_eq!(overlay_manifest.schema, 1);
        assert_eq!(overlay_manifest.production_iso, "novos.iso");
        assert_eq!(
            overlay_manifest.production_filesystem_squashfs_sha256,
            expected_squashfs_sha
        );

        // Error case 1: empty production_filesystem_squashfs_sha256
        fs::write(
            &manifest_path,
            r#"{"schema":1,"production_iso":"novos.iso","production_filesystem_squashfs_sha256":""}"#,
        )
        .expect("write empty sha overlay.json");
        assert!(Command::new("mksquashfs")
            .arg(&overlay_root)
            .arg(&overlay_squashfs_out)
            .arg("-noappend")
            .arg("-quiet")
            .status()
            .expect("mksquashfs empty sha")
            .success());
        let empty_sha_iso = tmp.path().join("empty_sha.iso");
        assert!(Command::new("xorriso")
            .arg("-as")
            .arg("mkisofs")
            .arg("-quiet")
            .arg("-o")
            .arg(&empty_sha_iso)
            .arg(&iso_dir)
            .status()
            .expect("mkisofs empty sha")
            .success());
        match read_test_overlay_manifest(&empty_sha_iso).expect_err("empty sha must fail") {
            TrackerError::ValidationError(msg) => {
                assert!(
                    msg.contains("production_filesystem_squashfs_sha256"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected ValidationError, got: {other:?}"),
        }

        // Error case 2: invalid JSON in overlay.json
        fs::write(&manifest_path, b"not-json").expect("write invalid overlay.json");
        assert!(Command::new("mksquashfs")
            .arg(&overlay_root)
            .arg(&overlay_squashfs_out)
            .arg("-noappend")
            .arg("-quiet")
            .status()
            .expect("mksquashfs invalid json")
            .success());
        let invalid_json_iso = tmp.path().join("invalid_json.iso");
        assert!(Command::new("xorriso")
            .arg("-as")
            .arg("mkisofs")
            .arg("-quiet")
            .arg("-o")
            .arg(&invalid_json_iso)
            .arg(&iso_dir)
            .status()
            .expect("mkisofs invalid json")
            .success());
        match read_test_overlay_manifest(&invalid_json_iso).expect_err("invalid json must fail") {
            TrackerError::ValidationError(msg) => {
                assert!(
                    msg.contains("invalid overlay.json:"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected ValidationError, got: {other:?}"),
        }

        // Error case 3: layer exists but overlay.json is missing
        fs::remove_file(&manifest_path).expect("remove overlay.json");
        assert!(Command::new("mksquashfs")
            .arg(&overlay_root)
            .arg(&overlay_squashfs_out)
            .arg("-noappend")
            .arg("-quiet")
            .status()
            .expect("mksquashfs missing overlay.json")
            .success());
        let missing_manifest_iso = tmp.path().join("missing_manifest.iso");
        assert!(Command::new("xorriso")
            .arg("-as")
            .arg("mkisofs")
            .arg("-quiet")
            .arg("-o")
            .arg(&missing_manifest_iso)
            .arg(&iso_dir)
            .status()
            .expect("mkisofs missing overlay.json")
            .success());
        match read_test_overlay_manifest(&missing_manifest_iso)
            .expect_err("missing overlay.json must fail")
        {
            TrackerError::ValidationError(msg) => {
                assert!(
                    msg.contains("has /live/zz-novos-test.squashfs but no /usr/share/novos-test/overlay.json"),
                    "unexpected message: {msg}"
                );
            }
            other => panic!("expected ValidationError, got: {other:?}"),
        }
    }
}
