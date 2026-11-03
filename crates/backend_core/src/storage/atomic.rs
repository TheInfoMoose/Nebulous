use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::{Result, TrackerError};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Atomically writes `contents` to `target_path` by writing to a sibling temporary file,
/// syncing to disk, and renaming over `target_path`.
///
/// This guarantees that concurrent readers (CLI, GUI, or Git) never observe a truncated
/// or partially written TOML/Markdown file.
pub fn atomic_write_bytes(target_path: &Path, contents: &[u8]) -> Result<()> {
    let parent = target_path.parent().ok_or_else(|| TrackerError::Io {
        path: target_path.to_path_buf(),
        source: std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
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

    let write_result = (|| -> std::io::Result<()> {
        if let Ok(metadata) = fs::metadata(target_path) {
            let _ = temp_file.set_permissions(metadata.permissions());
        }
        temp_file.write_all(contents)?;
        temp_file.flush()?;
        temp_file.sync_all()?;
        Ok(())
    })();

    // Ensure the file descriptor is closed before renaming.
    drop(temp_file);

    if let Err(source) = write_result {
        let _ = fs::remove_file(&temp_path);
        return Err(TrackerError::Io {
            path: target_path.to_path_buf(),
            source,
        });
    }

    if let Err(source) = fs::rename(&temp_path, target_path) {
        let _ = fs::remove_file(&temp_path);
        return Err(TrackerError::Io {
            path: target_path.to_path_buf(),
            source,
        });
    }

    Ok(())
}

/// Atomically writes a UTF-8 string to `target_path`.
pub fn atomic_write_str(target_path: &Path, contents: &str) -> Result<()> {
    atomic_write_bytes(target_path, contents.as_bytes())
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
