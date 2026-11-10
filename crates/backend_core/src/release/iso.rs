use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::process::Command;

use chrono::{SecondsFormat, Utc};

use crate::error::{Result, TrackerError};
use crate::release::inventory::{IsoInventory, IsoMeta};
use crate::release::rootfs::{self, COSMIC_EPOCH_COMPONENTS};

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

/// Extract an [`IsoInventory`] from a built Live ISO using unprivileged `xorriso` and
/// `unsquashfs` inside a temporary directory.
///
/// Runs unprivileged with no loop or filesystem mounts; the temporary directory is deleted
/// automatically when dropped (both on success and on error), per contract §5.3.
pub fn extract_iso_inventory(iso: &Path, project_root: &Path) -> Result<IsoInventory> {
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
        .arg("/live/filesystem.squashfs")
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
        let stderr_text = String::from_utf8_lossy(&xorriso_output.stderr);
        let tail_lines: Vec<&str> = stderr_text.lines().rev().take(20).collect();
        let stderr_tail = tail_lines
            .into_iter()
            .rev()
            .collect::<Vec<&str>>()
            .join("\n");
        return Err(TrackerError::ValidationError(format!(
            "xorriso failed on {}: {}",
            iso.display(),
            stderr_tail.trim()
        )));
    }

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

    rootfs::inventory_from_root(&extract_dir, meta, project_root)
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
}
