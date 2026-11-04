use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use crate::error::{Result, TrackerError};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Atomically writes `contents` to `target_path` by writing to a sibling temporary file,
/// syncing to disk, and renaming over `target_path`.
///
/// Includes a cross-platform Windows-safe fallback if `fs::rename` encounters
/// `PermissionDenied` or `AlreadyExists` due to read-only flags or transient file locks.
pub fn atomic_write_bytes(target_path: &Path, contents: &[u8]) -> Result<()> {
    let parent = target_path.parent().ok_or_else(|| TrackerError::Io {
        path: target_path.to_path_buf(),
        source: std::io::Error::new(
            ErrorKind::InvalidInput,
            "target path has no parent directory",
        ),
    })?;

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

    let target_permissions = fs::metadata(target_path).ok().map(|m| {
        let mut perms = m.permissions();
        if perms.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
        }
        perms
    });

    let write_result = (|| -> std::io::Result<()> {
        if let Some(ref perms) = target_permissions {
            let _ = temp_file.set_permissions(perms.clone());
        }
        temp_file.write_all(contents)?;
        temp_file.flush()?;
        temp_file.sync_all()?;
        Ok(())
    })();

    // Ensure the file descriptor is closed before renaming (critical on Windows).
    drop(temp_file);

    if let Err(source) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(TrackerError::Io {
            path: target_path.to_path_buf(),
            source,
        });
    }

    if let Err(source) = replace_target_with_fallback(&temp_path, target_path, contents) {
        let _ = fs::remove_file(&temp_path);
        return Err(TrackerError::Io {
            path: target_path.to_path_buf(),
            source,
        });
    }

    if let Some(perms) = target_permissions {
        let _ = fs::set_permissions(target_path, perms);
    }

    Ok(())
}

/// Atomically writes a UTF-8 string to `target_path`.
pub fn atomic_write_str(target_path: &Path, contents: &str) -> Result<()> {
    atomic_write_bytes(target_path, contents.as_bytes())
}

fn replace_target_with_fallback(
    temp_path: &Path,
    target_path: &Path,
    contents: &[u8],
) -> std::io::Result<()> {
    match fs::rename(temp_path, target_path) {
        Ok(()) => Ok(()),
        Err(err)
            if err.kind() == ErrorKind::PermissionDenied
                || err.kind() == ErrorKind::AlreadyExists =>
        {
            replace_target_after_rename_error(temp_path, target_path, contents)
        }
        Err(err) => Err(err),
    }
}

pub(crate) fn replace_target_after_rename_error(
    temp_path: &Path,
    target_path: &Path,
    contents: &[u8],
) -> std::io::Result<()> {
    // Clear read-only attribute on target_path if set (common cause on Windows)
    if let Ok(meta) = fs::metadata(target_path) {
        let mut perms = meta.permissions();
        if perms.readonly() {
            #[allow(clippy::permissions_set_readonly_false)]
            perms.set_readonly(false);
            let _ = fs::set_permissions(target_path, perms);
        }
    }

    // Retry rename with short backoff in case of transient indexer/watcher handle lock
    for attempt in 0..3 {
        if attempt > 0 {
            thread::sleep(Duration::from_millis(5 * attempt));
        }
        if fs::rename(temp_path, target_path).is_ok() {
            return Ok(());
        }
    }

    // Fallback 1: remove target_path and rename temp_path into place
    if fs::remove_file(target_path).is_ok() && fs::rename(temp_path, target_path).is_ok() {
        return Ok(());
    }

    // Fallback 2: direct in-place overwrite + sync when target cannot be unlinked/renamed
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(target_path)?;
    file.write_all(contents)?;
    file.flush()?;
    file.sync_all()?;
    drop(file);
    let _ = fs::remove_file(temp_path);
    Ok(())
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
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
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
            std::io::ErrorKind::AlreadyExists,
            "failed to allocate unique temporary file after 1024 attempts",
        ),
    })
}
