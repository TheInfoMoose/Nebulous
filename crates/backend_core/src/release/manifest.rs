use std::collections::BTreeMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};

pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

fn default_true() -> bool {
    true
}

fn default_pkg_min_version() -> String {
    "0".to_string()
}

fn default_bin_min_version() -> String {
    "0.1.0".to_string()
}

fn default_schema_version() -> u32 {
    MANIFEST_SCHEMA_VERSION
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct OsSpec {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub distro_id: Option<String>,
    #[serde(default)]
    pub distro_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageSpec {
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default = "default_pkg_min_version")]
    pub min_version: String,
}

impl Default for PackageSpec {
    fn default() -> Self {
        Self {
            required: true,
            min_version: default_pkg_min_version(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ManifestPackages {
    #[serde(default)]
    pub core: BTreeMap<String, PackageSpec>,
    #[serde(default)]
    pub cosmic: BTreeMap<String, PackageSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinarySpec {
    #[serde(default = "default_true")]
    pub required: bool,
    #[serde(default = "default_bin_min_version")]
    pub min_version: String,
    #[serde(default)]
    pub required_flags: Vec<String>,
}

impl Default for BinarySpec {
    fn default() -> Self {
        Self {
            required: true,
            min_version: default_bin_min_version(),
            required_flags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CalamaresSpec {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub required_configs: Vec<String>,
    #[serde(default)]
    pub custom_modules: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default)]
    pub os: OsSpec,
    #[serde(default)]
    pub packages: ManifestPackages,
    #[serde(default)]
    pub custom_binaries: BTreeMap<String, BinarySpec>,
    #[serde(default)]
    pub calamares_modules: CalamaresSpec,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            schema_version: MANIFEST_SCHEMA_VERSION,
            os: OsSpec::default(),
            packages: ManifestPackages::default(),
            custom_binaries: BTreeMap::new(),
            calamares_modules: CalamaresSpec::default(),
        }
    }
}

pub fn parse_manifest(text: &str, path: &Path) -> Result<Manifest> {
    let mut table: toml::Table =
        toml::from_str(text).map_err(|source| TrackerError::TomlDeserialize {
            path: Some(path.to_path_buf()),
            source,
        })?;

    let found = match table.get("schema_version") {
        None => {
            table.insert(
                "schema_version".to_string(),
                toml::Value::Integer(i64::from(MANIFEST_SCHEMA_VERSION)),
            );
            MANIFEST_SCHEMA_VERSION
        }
        Some(toml::Value::Integer(v)) if *v >= 1 => u32::try_from(*v).unwrap_or(u32::MAX),
        Some(_) => {
            return Err(TrackerError::ValidationError(format!(
                "{}: schema_version must be a positive integer",
                path.display()
            )));
        }
    };

    if found > MANIFEST_SCHEMA_VERSION {
        return Err(TrackerError::UnsupportedSchema {
            path: path.to_path_buf(),
            found,
            supported: MANIFEST_SCHEMA_VERSION,
        });
    }

    Manifest::deserialize(toml::Value::Table(table)).map_err(|source| {
        TrackerError::TomlDeserialize {
            path: Some(path.to_path_buf()),
            source,
        }
    })
}

pub fn read_manifest(path: &Path) -> Result<Option<Manifest>> {
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

    parse_manifest(&text, path).map(Some)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;
    use crate::release::ExitCode;

    const SAMPLE_MANIFEST: &str = "\
# Sample release manifest (.tracker/release/manifest.toml)
schema_version = 1

[os]
required = true
distro_id = \"exampleos\"
distro_version = \"13\"

[kernels]
default_line = \"linux-example-x64v3\"
fallback_line = \"linux-image-amd64\"

[packages.core]
dracut = { required = true, min_version = \"105-1\" }
example-pkg = {}
optional-pkg = { required = false }

[packages.cosmic]
cosmic-comp = { min_version = \"0.1.0\" }

[custom_binaries.\"/usr/local/bin/testtool\"]
required = true
min_version = \"0.2.0\"
required_flags = [\"--pre-packages\", \"--verify\"]

[custom_binaries.\"/usr/local/bin/defaulttool\"]

[calamares_modules]
required = true
required_configs = [\"fstab.conf\", \"welcome.conf\"]
custom_modules = [\"limine-bootloader\"]
";

    #[test]
    fn parse_real_shaped_manifest_and_ignore_unknown_tables() {
        let path = Path::new("/tmp/neb-test/manifest.toml");
        let manifest = parse_manifest(SAMPLE_MANIFEST, path).expect("parse sample manifest");

        assert_eq!(manifest.schema_version, 1);
        assert!(manifest.os.required);
        assert_eq!(manifest.os.distro_id.as_deref(), Some("exampleos"));
        assert_eq!(manifest.os.distro_version.as_deref(), Some("13"));

        let dracut = manifest.packages.core.get("dracut").expect("dracut");
        assert!(dracut.required);
        assert_eq!(dracut.min_version, "105-1");

        let bare = manifest
            .packages
            .core
            .get("example-pkg")
            .expect("example-pkg");
        assert!(bare.required);
        assert_eq!(bare.min_version, "0");
        assert_eq!(bare, &PackageSpec::default());

        let optional = manifest
            .packages
            .core
            .get("optional-pkg")
            .expect("optional-pkg");
        assert!(!optional.required);
        assert_eq!(optional.min_version, "0");

        let cosmic_comp = manifest
            .packages
            .cosmic
            .get("cosmic-comp")
            .expect("cosmic-comp");
        assert!(cosmic_comp.required);
        assert_eq!(cosmic_comp.min_version, "0.1.0");

        let testtool = manifest
            .custom_binaries
            .get("/usr/local/bin/testtool")
            .expect("testtool");
        assert!(testtool.required);
        assert_eq!(testtool.min_version, "0.2.0");
        assert_eq!(testtool.required_flags, vec!["--pre-packages", "--verify"]);

        let defaulttool = manifest
            .custom_binaries
            .get("/usr/local/bin/defaulttool")
            .expect("defaulttool");
        assert!(defaulttool.required);
        assert_eq!(defaulttool.min_version, "0.1.0");
        assert!(defaulttool.required_flags.is_empty());
        assert_eq!(defaulttool, &BinarySpec::default());

        assert!(manifest.calamares_modules.required);
        assert_eq!(
            manifest.calamares_modules.required_configs,
            vec!["fstab.conf", "welcome.conf"]
        );
        assert_eq!(
            manifest.calamares_modules.custom_modules,
            vec!["limine-bootloader"]
        );
    }

    #[test]
    fn parse_manifest_defaults_missing_schema_version_and_rejects_invalid_or_unsupported() {
        let path = Path::new("/tmp/neb-test/manifest.toml");
        let without_schema = SAMPLE_MANIFEST.replacen("schema_version = 1\n\n", "", 1);
        let parsed = parse_manifest(&without_schema, path).expect("missing schema_version -> 1");
        assert_eq!(parsed.schema_version, 1);

        let empty = parse_manifest("", path).expect("empty manifest");
        assert_eq!(empty, Manifest::default());
        assert!(!empty.os.required);
        assert!(!empty.calamares_modules.required);

        let v2 = SAMPLE_MANIFEST.replacen("schema_version = 1", "schema_version = 2", 1);
        let err = parse_manifest(&v2, path).expect_err("schema_version = 2 must fail");
        assert!(matches!(
            err,
            TrackerError::UnsupportedSchema {
                found: 2,
                supported: MANIFEST_SCHEMA_VERSION,
                ..
            }
        ));
        assert_eq!(ExitCode::from_error(&err), ExitCode::StateConflict);

        let zero = SAMPLE_MANIFEST.replacen("schema_version = 1", "schema_version = 0", 1);
        let err_zero = parse_manifest(&zero, path).expect_err("schema_version = 0 must fail");
        assert!(matches!(err_zero, TrackerError::ValidationError(_)));

        let bad_type =
            SAMPLE_MANIFEST.replacen("schema_version = 1", "schema_version = \"one\"", 1);
        let err_type =
            parse_manifest(&bad_type, path).expect_err("string schema_version must fail");
        assert!(matches!(err_type, TrackerError::ValidationError(_)));
    }

    #[test]
    fn read_manifest_missing_returns_none_and_existing_parses() {
        let tmp = tempdir().expect("tempdir");
        let missing = tmp.path().join("missing/manifest.toml");
        assert_eq!(read_manifest(&missing).expect("read missing"), None);

        let present = tmp.path().join("manifest.toml");
        fs::write(&present, SAMPLE_MANIFEST).expect("write manifest.toml");
        let loaded = read_manifest(&present)
            .expect("read_manifest")
            .expect("Some(manifest)");
        assert_eq!(loaded.schema_version, 1);
        assert!(loaded.packages.core.contains_key("dracut"));
    }
}
