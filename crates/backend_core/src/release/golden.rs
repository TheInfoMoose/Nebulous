use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Result, TrackerError};
use crate::release::inventory::{BootInfo, CalamaresModules, CustomBinary, IsoInventory, IsoMeta};

pub const GOLDEN_SCHEMA_VERSION: u32 = 2;

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Debian,
    ThirdParty,
}

impl Origin {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Debian => "debian",
            Self::ThirdParty => "third_party",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoldenComponent {
    pub version: String,
    pub origin: Origin,
    #[serde(default, skip_serializing_if = "is_false")]
    pub kernel_line: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub source_unknown: bool,
    pub binaries: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Golden {
    pub schema_version: u32,
    pub iso: IsoMeta,
    pub os_release: BTreeMap<String, String>,
    pub boot: BootInfo,
    pub calamares_modules: CalamaresModules,
    pub custom_binaries: BTreeMap<String, CustomBinary>,
    pub packages: BTreeMap<String, String>,
    #[serde(default)]
    pub components: BTreeMap<String, GoldenComponent>,
}

impl Golden {
    pub fn from_inventory(
        inv: &IsoInventory,
        components: BTreeMap<String, GoldenComponent>,
    ) -> Self {
        let packages = inv
            .packages
            .iter()
            .map(|(name, pkg)| (name.clone(), pkg.version.clone()))
            .collect();

        Self {
            schema_version: GOLDEN_SCHEMA_VERSION,
            iso: inv.iso.clone(),
            os_release: inv.os_release.clone(),
            boot: inv.boot.clone(),
            calamares_modules: inv.calamares_modules.clone(),
            custom_binaries: inv.custom_binaries.clone(),
            packages,
            components,
        }
    }
}

pub fn parse_golden(text: &str, path: &Path) -> Result<Golden> {
    let mut table: toml::Table =
        toml::from_str(text).map_err(|source| TrackerError::TomlDeserialize {
            path: Some(path.to_path_buf()),
            source,
        })?;

    let found = match table.get("schema_version") {
        None => {
            table.insert("schema_version".to_string(), toml::Value::Integer(1));
            1u32
        }
        Some(toml::Value::Integer(v)) if *v >= 1 => u32::try_from(*v).unwrap_or(u32::MAX),
        Some(_) => {
            return Err(TrackerError::ValidationError(format!(
                "{}: schema_version must be a positive integer",
                path.display()
            )));
        }
    };

    if found > GOLDEN_SCHEMA_VERSION {
        return Err(TrackerError::UnsupportedSchema {
            path: path.to_path_buf(),
            found,
            supported: GOLDEN_SCHEMA_VERSION,
        });
    }

    Golden::deserialize(toml::Value::Table(table)).map_err(|source| TrackerError::TomlDeserialize {
        path: Some(path.to_path_buf()),
        source,
    })
}

pub fn read_golden(path: &Path) -> Result<Option<Golden>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(TrackerError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    parse_golden(&text, path).map(Some)
}

pub fn render_golden(golden: &Golden) -> Result<String> {
    if !golden.packages.is_empty() && golden.components.is_empty() {
        return Err(TrackerError::ValidationError(
            "golden has packages but no components; convert it with neb release import".to_string(),
        ));
    }

    let mut normalized = golden.clone();
    normalized.schema_version = GOLDEN_SCHEMA_VERSION;
    let body = toml::to_string(&normalized)?;

    Ok(format!(
        "# NovOS golden inventory (.tracker/release/golden.toml), schema v2\n\
         # Written by neb release. Do not edit by hand.\n\n\
         {body}"
    ))
}

/// Renders and atomically writes `golden` to `path` in schema v2 format.
///
/// The caller must hold `TrackerStorage::with_write_lock`. The returned lowercase hex
/// SHA-256 digest of the bytes written is the release record's `golden_sha256` (§4.6).
pub fn write_golden(path: &Path, golden: &Golden) -> Result<String> {
    let text = render_golden(golden)?;

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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::release::rootfs::{inventory_from_root, sha256_file, write_fixture_root};
    use crate::release::ExitCode;

    const V1_FIXTURE: &str = "\
# ==============================================================================
# NovOS Golden OS Snapshot Inventory (.nebulous/golden-snapshot.toml)
# Generated by scripts/nebulous-ctl.py extract-golden
# ==============================================================================

schema_version = 1

[iso]
path = \"/tmp/neb-test/NovOS-sid-amd64.iso\"
size_bytes = 2147483648
vmlinuz_sha256 = \"1111111111111111111111111111111111111111111111111111111111111111\"
initrd_sha256 = \"2222222222222222222222222222222222222222222222222222222222222222\"
extracted_at = \"2026-11-08T12:00:00+00:00\"

[os_release]
HOME_URL = \"https://example.com/novos\"
ID = \"novos\"
NAME = \"NovOS\"

[boot]
files = [\"initrd.img\", \"vmlinuz\"]

[calamares_modules]
configs = [\"fstab.conf\", \"welcome.conf\"]
custom_modules = [\"limine-bootloader\"]

[calamares_modules.presence]
\"fstab.conf\" = true
limine-bootloader = true
\"welcome.conf\" = true

[custom_binaries]
novos-helper = { path = \"/usr/local/bin/novos-helper\", present = true, sha256 = \"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\", size_bytes = 4096, version = \"0.2.0\", is_symlink = false, flags = [\"--pre-packages\"] }

[packages]
base-files = \"14\"
linux-image-amd64 = \"7.3.3-1\"
\"linux-image-7.3.3+deb14-amd64\" = \"7.3.3-1\"
";

    #[test]
    fn parse_v1_golden_snapshot() {
        let path = Path::new("/tmp/neb-test/golden.toml");
        let golden = parse_golden(V1_FIXTURE, path).expect("parse v1 fixture");

        assert_eq!(golden.schema_version, 1);
        assert!(golden.components.is_empty());
        assert_eq!(golden.iso.path, "/tmp/neb-test/NovOS-sid-amd64.iso");
        assert_eq!(golden.iso.size_bytes, 2_147_483_648);
        assert_eq!(
            golden.os_release.get("HOME_URL").map(String::as_str),
            Some("https://example.com/novos")
        );
        assert_eq!(golden.boot.files, vec!["initrd.img", "vmlinuz"]);
        assert_eq!(
            golden.calamares_modules.presence.get("fstab.conf"),
            Some(&true)
        );
        let bin = golden
            .custom_binaries
            .get("novos-helper")
            .expect("novos-helper entry");
        assert_eq!(bin.path, "/usr/local/bin/novos-helper");
        assert!(bin.present);
        assert_eq!(bin.size_bytes, 4096);
        assert_eq!(bin.version, "0.2.0");
        assert!(!bin.is_symlink);
        assert_eq!(bin.flags, vec!["--pre-packages"]);
        assert_eq!(
            golden.packages.get("linux-image-amd64").map(String::as_str),
            Some("7.3.3-1")
        );
        assert_eq!(
            golden
                .packages
                .get("linux-image-7.3.3+deb14-amd64")
                .map(String::as_str),
            Some("7.3.3-1")
        );
    }

    #[test]
    fn parse_v1_without_schema_version_defaults_to_one() {
        let without_schema = V1_FIXTURE.replacen("schema_version = 1\n\n", "", 1);
        assert!(!without_schema.contains("schema_version"));

        let path = Path::new("/tmp/neb-test/golden.toml");
        let golden = parse_golden(&without_schema, path).expect("parse v1 without schema_version");
        assert_eq!(golden.schema_version, 1);
        assert!(golden.components.is_empty());
    }

    #[test]
    fn v2_round_trip_and_deterministic_rendering() {
        let tmp = tempdir().expect("create tempdir");
        write_fixture_root(tmp.path());

        let iso = IsoMeta {
            path: "/tmp/neb-test/NovOS-sid-amd64.iso".to_string(),
            size_bytes: 1024,
            vmlinuz_sha256: "a".repeat(64),
            initrd_sha256: "b".repeat(64),
            extracted_at: "2026-11-08T12:00:00Z".to_string(),
        };
        let inv = inventory_from_root(tmp.path(), iso, tmp.path()).expect("inventory_from_root");

        let mut components = BTreeMap::new();
        components.insert(
            "foo".to_string(),
            GoldenComponent {
                version: "1.2-1".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: false,
                binaries: BTreeMap::from([("libfoo1".to_string(), "1.2-1".to_string())]),
            },
        );
        components.insert(
            "linux-xanmod-x64v3".to_string(),
            GoldenComponent {
                version: "7.3.3-x64v3-xanmod1-0".to_string(),
                origin: Origin::ThirdParty,
                kernel_line: true,
                source_unknown: false,
                binaries: BTreeMap::from([
                    (
                        "linux-image-7.3.3-x64v3-xanmod1".to_string(),
                        "7.3.3-x64v3-xanmod1-0".to_string(),
                    ),
                    (
                        "linux-xanmod-x64v3".to_string(),
                        "7.3.3-x64v3-xanmod1-0".to_string(),
                    ),
                ]),
            },
        );

        let golden = Golden::from_inventory(&inv, components);
        assert_eq!(golden.schema_version, GOLDEN_SCHEMA_VERSION);
        assert_eq!(Origin::Debian.as_str(), "debian");
        assert_eq!(Origin::ThirdParty.as_str(), "third_party");

        let rendered_a = render_golden(&golden).expect("render_golden first");
        let rendered_b = render_golden(&golden).expect("render_golden second");
        assert_eq!(rendered_a, rendered_b);
        assert!(rendered_a.contains("schema_version = 2"));
        assert!(rendered_a.contains("origin = \"third_party\""));
        assert!(!rendered_a.contains("source_unknown"));

        let parsed =
            parse_golden(&rendered_a, Path::new("/tmp/neb-test/golden.toml")).expect("re-parse");
        assert_eq!(parsed, golden);

        let mut with_placeholder = golden;
        with_placeholder
            .components
            .get_mut("foo")
            .expect("foo component")
            .source_unknown = true;
        let rendered_placeholder =
            render_golden(&with_placeholder).expect("render with source_unknown");
        assert!(rendered_placeholder.contains("source_unknown = true"));
        let parsed_placeholder = parse_golden(
            &rendered_placeholder,
            Path::new("/tmp/neb-test/golden.toml"),
        )
        .expect("re-parse placeholder");
        assert_eq!(parsed_placeholder, with_placeholder);
    }

    #[test]
    fn schema_version_errors() {
        let path = Path::new("/tmp/neb-test/golden.toml");

        let v3 = V1_FIXTURE.replacen("schema_version = 1", "schema_version = 3", 1);
        let err = parse_golden(&v3, path).expect_err("schema_version = 3 should fail");
        assert!(matches!(
            err,
            TrackerError::UnsupportedSchema {
                found: 3,
                supported: GOLDEN_SCHEMA_VERSION,
                ..
            }
        ));
        assert_eq!(ExitCode::from_error(&err), ExitCode::StateConflict);

        let non_int = V1_FIXTURE.replacen("schema_version = 1", "schema_version = \"two\"", 1);
        let err_non_int =
            parse_golden(&non_int, path).expect_err("non-integer schema_version should fail");
        assert!(matches!(err_non_int, TrackerError::ValidationError(_)));

        let zero = V1_FIXTURE.replacen("schema_version = 1", "schema_version = 0", 1);
        let err_zero = parse_golden(&zero, path).expect_err("schema_version = 0 should fail");
        assert!(matches!(err_zero, TrackerError::ValidationError(_)));
    }

    #[test]
    fn read_golden_missing_returns_none() {
        let tmp = tempdir().expect("create tempdir");
        let missing = tmp.path().join("does-not-exist/golden.toml");
        assert_eq!(read_golden(&missing).expect("read missing"), None);
    }

    #[test]
    fn render_refuses_packages_without_components_and_write_golden_is_atomic() {
        let path = Path::new("/tmp/neb-test/golden.toml");
        let v1_golden = parse_golden(V1_FIXTURE, path).expect("parse v1");

        let err = render_golden(&v1_golden).expect_err("packages without components must fail");
        match err {
            TrackerError::ValidationError(msg) => {
                assert_eq!(
                    msg,
                    "golden has packages but no components; convert it with neb release import"
                );
            }
            other => panic!("unexpected error: {other:?}"),
        }

        let mut v2_golden = v1_golden;
        v2_golden.components.insert(
            "base-files".to_string(),
            GoldenComponent {
                version: "14".to_string(),
                origin: Origin::Debian,
                kernel_line: false,
                source_unknown: true,
                binaries: BTreeMap::from([("base-files".to_string(), "14".to_string())]),
            },
        );

        let tmp = tempdir().expect("create tempdir");
        let golden_path = tmp.path().join("golden.toml");
        let sha = write_golden(&golden_path, &v2_golden).expect("write_golden");
        let file_sha = sha256_file(&golden_path).expect("sha256_file");
        assert_eq!(sha, file_sha);

        let entries: Vec<_> = fs::read_dir(tmp.path())
            .expect("read_dir")
            .map(|e| {
                e.expect("dir entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(entries, vec!["golden.toml"]);

        let read_back = read_golden(&golden_path)
            .expect("read_golden")
            .expect("Some(golden)");
        let mut expected = v2_golden;
        expected.schema_version = GOLDEN_SCHEMA_VERSION;
        assert_eq!(read_back, expected);
    }
}
