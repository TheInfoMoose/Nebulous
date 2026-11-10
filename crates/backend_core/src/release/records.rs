use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Result, TrackerError};
use crate::release::golden::Origin;

pub const RELEASE_RECORD_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordComponent {
    pub version: String,
    pub origin: Origin,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub binaries: BTreeMap<String, String>,
}

/// A promoted or blessed release record stored under `.tracker/release/releases/<version>-<UTC>.toml`.
///
/// Scalar fields are ordered before tables so `toml::to_string` serializes without placing a
/// plain value after a table section. `candidate` stores the archived `candidate.toml` written
/// by STEP 25P (`promote`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseRecord {
    pub schema_version: u32,
    pub version: String,
    pub promoted_at: String,
    pub promoted_by: String,
    #[serde(default)]
    pub reason: String,
    pub snapshot: String,
    pub iso_sha256: String,
    pub golden_sha256: String,
    #[serde(default)]
    pub hold_file_sha256: String,
    pub candidate_id: String,
    #[serde(default)]
    pub bisect_rounds: u32,
    #[serde(default)]
    pub phases: BTreeMap<String, String>,
    #[serde(default)]
    pub components: BTreeMap<String, RecordComponent>,
    #[serde(default)]
    pub third_party_cache: BTreeMap<String, Vec<String>>,
    /// The archived `candidate.toml`, written by STEP 25P.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate: Option<toml::Table>,
}

fn is_valid_version_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '~' | '-')
}

pub fn record_file_name(version: &str, promoted_at: &DateTime<Utc>) -> Result<String> {
    if version.is_empty() || !version.chars().all(is_valid_version_char) {
        return Err(TrackerError::ValidationError(format!(
            "invalid release version '{version}': must be non-empty and contain only [A-Za-z0-9._+~-]"
        )));
    }
    let ts = promoted_at.format("%Y%m%dT%H%M%SZ");
    Ok(format!("{version}-{ts}.toml"))
}

pub fn parse_record(text: &str, path: &Path) -> Result<ReleaseRecord> {
    let table: toml::Table =
        toml::from_str(text).map_err(|source| TrackerError::TomlDeserialize {
            path: Some(path.to_path_buf()),
            source,
        })?;

    let found = match table.get("schema_version") {
        None => {
            return Err(TrackerError::ValidationError(format!(
                "{}: missing required schema_version",
                path.display()
            )));
        }
        Some(toml::Value::Integer(v)) if *v >= 1 => u32::try_from(*v).unwrap_or(u32::MAX),
        Some(_) => {
            return Err(TrackerError::ValidationError(format!(
                "{}: schema_version must be a positive integer",
                path.display()
            )));
        }
    };

    if found > RELEASE_RECORD_SCHEMA_VERSION {
        return Err(TrackerError::UnsupportedSchema {
            path: path.to_path_buf(),
            found,
            supported: RELEASE_RECORD_SCHEMA_VERSION,
        });
    }

    ReleaseRecord::deserialize(toml::Value::Table(table)).map_err(|source| {
        TrackerError::TomlDeserialize {
            path: Some(path.to_path_buf()),
            source,
        }
    })
}

pub fn read_record(path: &Path) -> Result<ReleaseRecord> {
    let text = fs::read_to_string(path).map_err(|source| TrackerError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse_record(&text, path)
}

pub fn render_record(r: &ReleaseRecord) -> Result<String> {
    let mut normalized = r.clone();
    normalized.schema_version = RELEASE_RECORD_SCHEMA_VERSION;
    let body = toml::to_string(&normalized)?;
    Ok(format!(
        "# NovOS release record (written by {})\n\n{body}",
        r.promoted_by
    ))
}

/// Renders and atomically writes `r` to `path`, refusing with [`TrackerError::AlreadyExists`]
/// if `path` already exists.
///
/// The caller must hold `TrackerStorage::with_write_lock`. Returns the lowercase hex SHA-256
/// digest of the written text.
pub fn write_record(path: &Path, r: &ReleaseRecord) -> Result<String> {
    if path.exists() {
        let id = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        return Err(TrackerError::AlreadyExists {
            kind: "release record",
            id,
        });
    }

    let text = render_record(r)?;

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|source| TrackerError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }

    crate::storage::atomic_write_str(path, &text)?;

    let digest = Sha256::digest(text.as_bytes());
    let mut hex = String::with_capacity(64);
    for byte in digest {
        let _ = write!(&mut hex, "{byte:02x}");
    }
    Ok(hex)
}

pub fn list_records(dir: &Path) -> Result<Vec<(PathBuf, ReleaseRecord)>> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(source) => {
            return Err(TrackerError::Io {
                path: dir.to_path_buf(),
                source,
            });
        }
    };

    let mut records = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|source| TrackerError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("toml") {
            continue;
        }
        let file_type = entry.file_type().map_err(|source| TrackerError::Io {
            path: path.clone(),
            source,
        })?;
        if !file_type.is_file() {
            continue;
        }
        let rec = read_record(&path)?;
        records.push((path, rec));
    }

    records.sort_by(|(pa, ra), (pb, rb)| {
        ra.promoted_at
            .cmp(&rb.promoted_at)
            .then_with(|| pa.file_name().cmp(&pb.file_name()))
    });

    Ok(records)
}

pub fn latest_record(dir: &Path) -> Result<Option<(PathBuf, ReleaseRecord)>> {
    Ok(list_records(dir)?.pop())
}

pub fn last_carrying<'a>(
    records: &'a [(PathBuf, ReleaseRecord)],
    component: &str,
    version: &str,
) -> Option<&'a ReleaseRecord> {
    records.iter().rev().find_map(|(_, rec)| {
        rec.components
            .get(component)
            .filter(|c| c.version == version)
            .map(|_| rec)
    })
}

pub fn snapshot_for(
    records: &[(PathBuf, ReleaseRecord)],
    component: &str,
    version: &str,
) -> String {
    last_carrying(records, component, version)
        .map(|r| r.snapshot.clone())
        .unwrap_or_default()
}

pub fn cache_paths_for(
    records: &[(PathBuf, ReleaseRecord)],
    component: &str,
    version: &str,
) -> Vec<String> {
    last_carrying(records, component, version)
        .and_then(|r| r.third_party_cache.get(component))
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tempfile::tempdir;

    use super::*;
    use crate::release::rootfs::sha256_file;
    use crate::release::ExitCode;

    fn sample_record(version: &str, promoted_at: &str, snapshot: &str) -> ReleaseRecord {
        let mut phases = BTreeMap::new();
        phases.insert("build".to_string(), "pass".to_string());
        phases.insert("vm-live-boot".to_string(), "pass".to_string());
        phases.insert("vm-install-e2e".to_string(), "pass".to_string());

        let mut pkg_bins = BTreeMap::new();
        pkg_bins.insert("example-pkg".to_string(), "1.0-1".to_string());
        pkg_bins.insert("libexample1".to_string(), "1.0-1".to_string());

        let mut components = BTreeMap::new();
        components.insert(
            "example-pkg".to_string(),
            RecordComponent {
                version: "1.0-1".to_string(),
                origin: Origin::Debian,
                binaries: pkg_bins,
            },
        );
        components.insert(
            "example-kernel".to_string(),
            RecordComponent {
                version: "7.3.3-1".to_string(),
                origin: Origin::ThirdParty,
                binaries: BTreeMap::new(),
            },
        );

        let mut third_party_cache = BTreeMap::new();
        third_party_cache.insert(
            "example-kernel".to_string(),
            vec!["artifact-cache/example-kernel_7.3.3-1_amd64.deb".to_string()],
        );

        ReleaseRecord {
            schema_version: RELEASE_RECORD_SCHEMA_VERSION,
            version: version.to_string(),
            promoted_at: promoted_at.to_string(),
            promoted_by: "neb release promote".to_string(),
            reason: String::new(),
            snapshot: snapshot.to_string(),
            iso_sha256: "1111111111111111111111111111111111111111111111111111111111111111"
                .to_string(),
            golden_sha256: "2222222222222222222222222222222222222222222222222222222222222222"
                .to_string(),
            hold_file_sha256: String::new(),
            candidate_id: "111111111111-20261109T120000Z".to_string(),
            bisect_rounds: 0,
            phases,
            components,
            third_party_cache,
            candidate: None,
        }
    }

    #[test]
    fn record_file_name_formats_and_validates_version() {
        let dt = Utc.with_ymd_and_hms(2026, 11, 9, 12, 0, 0).unwrap();
        let name = record_file_name("2026.10", &dt).expect("valid version");
        assert_eq!(name, "2026.10-20261109T120000Z.toml");

        let plus_tilde = record_file_name("2026.10+1~rc1_test-2", &dt).expect("valid chars");
        assert_eq!(plus_tilde, "2026.10+1~rc1_test-2-20261109T120000Z.toml");

        for bad in ["", "../x", "2026/10", "2026 10"] {
            let err = record_file_name(bad, &dt).expect_err("invalid version must fail");
            assert!(matches!(err, TrackerError::ValidationError(_)));
        }
    }

    #[test]
    fn write_and_read_record_roundtrip_sha256_already_exists_and_schema_version() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("releases/2026.10-20261109T120000Z.toml");
        let rec = sample_record("2026.10", "2026-11-09T12:00:00Z", "20261101T000000Z");

        let sha = write_record(&path, &rec).expect("write_record");
        assert_eq!(sha.len(), 64);
        assert_eq!(sha, sha256_file(&path).expect("sha256_file"));

        let loaded = read_record(&path).expect("read_record");
        assert_eq!(loaded, rec);

        // Second write to the same path must fail with AlreadyExists.
        let dup_err = write_record(&path, &rec).expect_err("duplicate write must fail");
        match dup_err {
            TrackerError::AlreadyExists { kind, id } => {
                assert_eq!(kind, "release record");
                assert_eq!(id, "2026.10-20261109T120000Z.toml");
            }
            other => panic!("expected AlreadyExists, got {other:?}"),
        }

        // Unsupported schema_version = 2 -> UnsupportedSchema (exit 7).
        let raw = fs::read_to_string(&path).expect("read raw");
        let v2_raw = raw.replacen("schema_version = 1", "schema_version = 2", 1);
        let v2_err = parse_record(&v2_raw, &path).expect_err("v2 must fail");
        assert!(matches!(
            v2_err,
            TrackerError::UnsupportedSchema {
                found: 2,
                supported: 1,
                ..
            }
        ));
        assert_eq!(ExitCode::from_error(&v2_err), ExitCode::StateConflict);

        // Missing schema_version -> ValidationError.
        let no_ver = raw.replacen("schema_version = 1\n", "", 1);
        assert!(matches!(
            parse_record(&no_ver, &path).expect_err("missing schema_version"),
            TrackerError::ValidationError(_)
        ));
    }

    #[test]
    fn list_records_sorts_by_promoted_at_ignores_non_toml_and_errors_on_corrupt_toml() {
        let tmp = tempdir().expect("tempdir");
        let dir = tmp.path().join("releases");

        // Missing directory returns empty.
        assert!(list_records(&dir).expect("missing dir").is_empty());
        assert!(latest_record(&dir).expect("missing dir latest").is_none());

        let r1 = sample_record("2026.10", "2026-11-01T10:00:00Z", "20261101T000000Z");
        let r2 = sample_record("2026.10", "2026-11-05T10:00:00Z", "20261105T000000Z");
        let r3 = sample_record("2026.10", "2026-11-09T10:00:00Z", "20261109T000000Z");

        // Write out of chronological order.
        write_record(&dir.join("2026.10-20261105T100000Z.toml"), &r2).expect("write r2");
        write_record(&dir.join("2026.10-20261109T100000Z.toml"), &r3).expect("write r3");
        write_record(&dir.join("2026.10-20261101T100000Z.toml"), &r1).expect("write r1");

        // Non-toml file is ignored.
        fs::write(dir.join("README.txt"), "ignore me").expect("write txt");

        let listed = list_records(&dir).expect("list_records");
        let listed_records: Vec<ReleaseRecord> = listed.into_iter().map(|(_, r)| r).collect();
        assert_eq!(listed_records, vec![r1, r2, r3.clone()]);

        let (_, latest) = latest_record(&dir)
            .expect("latest_record")
            .expect("some latest");
        assert_eq!(latest, r3);

        // Corrupt .toml file causes list_records to return an error.
        fs::write(dir.join("corrupt.toml"), "not valid = [toml").expect("write corrupt");
        assert!(list_records(&dir).is_err());
    }

    #[test]
    fn last_carrying_snapshot_for_and_cache_paths_for() {
        let mut r_a = sample_record("2026.10", "2026-11-01T10:00:00Z", "20261101T000000Z");
        let mut r_b = sample_record("2026.10", "2026-11-05T10:00:00Z", "20261105T000000Z");
        let mut r_c = sample_record("2026.10", "2026-11-09T10:00:00Z", "20261109T000000Z");

        r_a.components.get_mut("example-pkg").unwrap().version = "1.0-1".to_string();
        r_b.components.get_mut("example-pkg").unwrap().version = "1.0-1".to_string();
        r_c.components.get_mut("example-pkg").unwrap().version = "1.1-1".to_string();

        let records = vec![
            (PathBuf::from("a.toml"), r_a),
            (PathBuf::from("b.toml"), r_b),
            (PathBuf::from("c.toml"), r_c),
        ];

        assert_eq!(
            snapshot_for(&records, "example-pkg", "1.0-1"),
            "20261105T000000Z"
        );
        assert_eq!(
            snapshot_for(&records, "example-pkg", "1.1-1"),
            "20261109T000000Z"
        );
        assert_eq!(snapshot_for(&records, "example-pkg", "9.9-9"), "");
        assert_eq!(snapshot_for(&records, "missing-pkg", "1.0-1"), "");

        assert_eq!(
            cache_paths_for(&records, "example-kernel", "7.3.3-1"),
            vec!["artifact-cache/example-kernel_7.3.3-1_amd64.deb".to_string()]
        );
        assert!(cache_paths_for(&records, "example-kernel", "0.0.1").is_empty());
    }
}
