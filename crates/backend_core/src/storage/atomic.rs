use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Result, TrackerError};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Atomically writes `contents` to `target_path` by writing to a sibling temporary file,
/// syncing it to disk, and renaming it over `target_path`.
///
/// - The original file's permissions (Unix mode bits / Windows read-only attribute) are
///   preserved exactly; they are never widened.
/// - On Unix the parent directory is fsynced after the rename so the new directory entry is
///   durable.
/// - There is no non-atomic fallback: if the rename fails, the target is left untouched and
///   the error is returned. On Windows only, a read-only attribute or a transient handle lock
///   is worked around by clearing the attribute, retrying, and as a last resort replacing the
///   target via remove-then-rename (Windows cannot rename over a read-only file).
pub fn atomic_write_bytes(target_path: &Path, contents: &[u8]) -> Result<()> {
    let io_err = |source: std::io::Error| TrackerError::Io {
        path: target_path.to_path_buf(),
        source,
    };

    let parent = target_path.parent().ok_or_else(|| {
        io_err(std::io::Error::new(
            ErrorKind::InvalidInput,
            "target path has no parent directory",
        ))
    })?;
    // `Path::new("file").parent()` is `Some("")`; treat that as the current directory.
    let parent: &Path = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

    fs::create_dir_all(parent).map_err(|source| TrackerError::Io {
        path: parent.to_path_buf(),
        source,
    })?;

    let file_name = target_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("tracker_file");

    let pid = std::process::id();
    let (temp_path, mut temp_file) = create_unique_temp_file(parent, file_name, pid)?;

    // Capture the existing regular file's permissions verbatim (never follow a symlink here).
    let original_permissions = fs::symlink_metadata(target_path)
        .ok()
        .filter(|m| m.is_file())
        .map(|m| m.permissions());

    let write_result = (|| -> std::io::Result<()> {
        temp_file.write_all(contents)?;
        temp_file.flush()?;
        // Apply the original mode via the open descriptor (fchmod), so even a read-only
        // original mode does not block the write above.
        if let Some(perms) = original_permissions.clone() {
            temp_file.set_permissions(perms)?;
        }
        temp_file.sync_all()?;
        Ok(())
    })();

    // Ensure the file descriptor is closed before renaming (critical on Windows).
    drop(temp_file);

    if let Err(source) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(io_err(source));
    }

    if let Err(source) = replace_target(&temp_path, target_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(io_err(source));
    }

    #[cfg(unix)]
    sync_dir(parent).map_err(|source| TrackerError::Io {
        path: parent.to_path_buf(),
        source,
    })?;

    Ok(())
}

/// Atomically writes a UTF-8 string to `target_path`.
pub fn atomic_write_str(target_path: &Path, contents: &str) -> Result<()> {
    atomic_write_bytes(target_path, contents.as_bytes())
}

/// Unix: `rename(2)` atomically replaces the target regardless of the target's own mode.
#[cfg(not(windows))]
fn replace_target(temp_path: &Path, target_path: &Path) -> std::io::Result<()> {
    fs::rename(temp_path, target_path)
}

/// Windows: `MoveFileEx(REPLACE_EXISTING)` fails on read-only targets and on targets held open
/// by indexers/watchers, so clear the read-only attribute (the preserved attribute was already
/// applied to the temp file) and retry briefly before falling back to remove-then-rename.
#[cfg(windows)]
fn replace_target(temp_path: &Path, target_path: &Path) -> std::io::Result<()> {
    use std::thread;
    use std::time::Duration;

    let first = match fs::rename(temp_path, target_path) {
        Ok(()) => return Ok(()),
        Err(err)
            if err.kind() == ErrorKind::PermissionDenied
                || err.kind() == ErrorKind::AlreadyExists =>
        {
            err
        }
        Err(err) => return Err(err),
    };

    if let Ok(meta) = fs::symlink_metadata(target_path) {
        let mut perms = meta.permissions();
        if perms.readonly() {
            // On Windows this only clears FILE_ATTRIBUTE_READONLY; it does not widen ACLs.
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            let _ = fs::set_permissions(target_path, perms);
        }
    }

    for attempt in 0..3u64 {
        if attempt > 0 {
            thread::sleep(Duration::from_millis(5 * attempt));
        }
        if fs::rename(temp_path, target_path).is_ok() {
            return Ok(());
        }
    }

    if fs::remove_file(target_path).is_ok() {
        return fs::rename(temp_path, target_path);
    }
    Err(first)
}

/// Fsyncs a directory so a completed rename inside it survives power loss.
#[cfg(unix)]
fn sync_dir(dir: &Path) -> std::io::Result<()> {
    fs::File::open(dir)?.sync_all()
}

fn create_unique_temp_file(
    parent: &Path,
    file_name: &str,
    pid: u32,
) -> Result<(PathBuf, fs::File)> {
    for _ in 0..1024 {
        let seq = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let temp_name = format!(".{file_name}.tmp.{pid}.{seq}");
        let temp_path = parent.join(temp_name);

        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp_path)
        {
            Ok(file) => return Ok((temp_path, file)),
            Err(err) if err.kind() == ErrorKind::AlreadyExists => continue,
            Err(source) => {
                return Err(TrackerError::Io {
                    path: temp_path,
                    source,
                });
            }
        }
    }

    Err(TrackerError::Io {
        path: parent.to_path_buf(),
        source: std::io::Error::new(
            ErrorKind::AlreadyExists,
            "failed to allocate unique temporary file after 1024 attempts",
        ),
    })
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nebulous_atomic_{tag}_{}_{}",
            std::process::id(),
            TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn mode(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o7777
    }

    #[test]
    fn preserves_original_mode_across_atomic_write() {
        let dir = temp_dir("mode");
        let target = dir.join("item.toml");
        for original in [0o600, 0o640, 0o444, 0o400] {
            fs::write(&target, b"old").unwrap();
            fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
            fs::set_permissions(&target, fs::Permissions::from_mode(original)).unwrap();

            atomic_write_str(&target, "new").unwrap();

            assert_eq!(fs::read_to_string(&target).unwrap(), "new");
            assert_eq!(
                mode(&target),
                original,
                "mode {original:o} must be preserved exactly (never widened)"
            );
            fs::set_permissions(&target, fs::Permissions::from_mode(0o600)).unwrap();
            fs::remove_file(&target).unwrap();
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn failed_rename_leaves_target_untouched_and_no_temp_files() {
        let dir = temp_dir("norename");
        // A non-empty directory at the target path makes rename(2) fail; there must be no
        // truncate-and-write or delete-then-rename fallback.
        let target = dir.join("blocked");
        fs::create_dir_all(target.join("child")).unwrap();

        assert!(atomic_write_str(&target, "x").is_err());
        assert!(target.join("child").is_dir());
        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .collect();
        assert!(leftovers.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
