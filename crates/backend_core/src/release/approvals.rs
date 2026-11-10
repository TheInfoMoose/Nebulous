use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approval {
    pub item_name: String,
    pub reason: String,
    #[serde(default = "default_true")]
    pub approved: bool,
    #[serde(default)]
    pub timestamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Approvals {
    #[serde(default)]
    pub approvals: Vec<Approval>,
}

impl Approvals {
    pub fn covers(&self, item: &str, target_version: Option<&str>) -> bool {
        self.approvals.iter().any(|entry| {
            entry.approved
                && !entry.item_name.trim().is_empty()
                && !entry.reason.trim().is_empty()
                && entry.item_name == item
                && match entry.version.as_deref() {
                    None => true,
                    Some(ver) => target_version == Some(ver),
                }
        })
    }
}

pub fn read_approvals(path: &Path) -> Result<Approvals> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Approvals::default()),
        Err(source) => {
            return Err(TrackerError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    toml::from_str(&text).map_err(|source| TrackerError::TomlDeserialize {
        path: Some(path.to_path_buf()),
        source,
    })
}

pub fn render_approvals(a: &Approvals) -> Result<String> {
    let body = toml::to_string(a)?;
    Ok(format!(
        "# NovOS approved variance exceptions (.tracker/release/approvals.toml)\n\
         # Written by neb release approve-variance. Remove an entry by hand to revoke it.\n\n\
         {body}"
    ))
}

/// Reads `path` (or starts from an empty list if absent), appends `approval`,
/// and atomically writes the updated `approvals.toml`.
///
/// The caller must hold `TrackerStorage::with_write_lock` and must have passed
/// the user-only gate (§5.1).
pub fn append_approval(path: &Path, approval: Approval) -> Result<()> {
    let mut current = read_approvals(path)?;
    current.approvals.push(approval);
    let text = render_approvals(&current)?;

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|source| TrackerError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }

    crate::storage::atomic_write_str(path, &text)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn missing_file_returns_empty_approvals() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("missing/approvals.toml");
        let approvals = read_approvals(&path).expect("read missing approvals");
        assert!(approvals.approvals.is_empty());
        assert!(!approvals.covers("dracut", Some("105-1")));
    }

    #[test]
    fn covers_checks_open_and_version_scoped_approvals_and_rejects_unapproved_or_blank_reason() {
        let approvals = Approvals {
            approvals: vec![
                Approval {
                    item_name: "btrfs-progs".to_string(),
                    reason: "Removed from base image".to_string(),
                    approved: true,
                    timestamp: "2026-11-08T12:00:00Z".to_string(),
                    version: None,
                },
                Approval {
                    item_name: "dracut".to_string(),
                    reason: "Pinned to 105-1 for initrd regression".to_string(),
                    approved: true,
                    timestamp: "2026-11-08T12:05:00Z".to_string(),
                    version: Some("105-1".to_string()),
                },
                Approval {
                    item_name: "disabled-entry".to_string(),
                    reason: "Has a reason but approved is false".to_string(),
                    approved: false,
                    timestamp: "2026-11-08T12:10:00Z".to_string(),
                    version: None,
                },
                Approval {
                    item_name: "blank-reason".to_string(),
                    reason: "   \t  ".to_string(),
                    approved: true,
                    timestamp: "2026-11-08T12:15:00Z".to_string(),
                    version: None,
                },
            ],
        };

        // Open-ended approval covers any target (Some or None).
        assert!(approvals.covers("btrfs-progs", Some("6.6.3-1")));
        assert!(approvals.covers("btrfs-progs", Some("1.0")));
        assert!(approvals.covers("btrfs-progs", None));

        // Version-scoped approval covers only that exact version and never None.
        assert!(approvals.covers("dracut", Some("105-1")));
        assert!(!approvals.covers("dracut", Some("104-1")));
        assert!(!approvals.covers("dracut", None));

        // Unapproved or blank reason never covers.
        assert!(!approvals.covers("disabled-entry", Some("1.0")));
        assert!(!approvals.covers("disabled-entry", None));
        assert!(!approvals.covers("blank-reason", Some("1.0")));
        assert!(!approvals.covers("blank-reason", None));
    }

    #[test]
    fn append_approval_preserves_order_and_default_approved_is_true_for_hand_written_file() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("release/approvals.toml");

        let first = Approval {
            item_name: "btrfs-progs".to_string(),
            reason: "Intentional removal".to_string(),
            approved: true,
            timestamp: "2026-11-08T12:00:00Z".to_string(),
            version: None,
        };
        let second = Approval {
            item_name: "dracut".to_string(),
            reason: "Temporary downgrade".to_string(),
            approved: true,
            timestamp: "2026-11-08T12:01:00Z".to_string(),
            version: Some("105-1".to_string()),
        };

        append_approval(&path, first.clone()).expect("append first");
        append_approval(&path, second.clone()).expect("append second");

        let on_disk = fs::read_to_string(&path).expect("read approvals file");
        assert!(on_disk.starts_with(
            "# NovOS approved variance exceptions (.tracker/release/approvals.toml)\n\
             # Written by neb release approve-variance. Remove an entry by hand to revoke it.\n\n"
        ));

        let loaded = read_approvals(&path).expect("read_approvals");
        assert_eq!(loaded.approvals, vec![first, second]);

        // Hand-written entry without `approved` or `timestamp` defaults `approved` to true.
        let hand_written = "\
[[approvals]]
item_name = \"welcome.conf\"
reason = \"Replaced by custom greeting\"
";
        let hand_path = tmp.path().join("hand.toml");
        fs::write(&hand_path, hand_written).expect("write hand.toml");
        let hand_loaded = read_approvals(&hand_path).expect("read hand.toml");
        assert_eq!(hand_loaded.approvals.len(), 1);
        assert!(hand_loaded.approvals[0].approved);
        assert_eq!(hand_loaded.approvals[0].version, None);
        assert!(hand_loaded.covers("welcome.conf", None));

        // Corrupt TOML returns TomlDeserialize error rather than swallowing it.
        let bad_path = tmp.path().join("bad.toml");
        fs::write(&bad_path, "[[approvals]\nitem_name = ").expect("write bad.toml");
        let err = read_approvals(&bad_path).expect_err("corrupt TOML must fail");
        assert!(matches!(
            err,
            TrackerError::TomlDeserialize { path: Some(_), .. }
        ));
    }
}
