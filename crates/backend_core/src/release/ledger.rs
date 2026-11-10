use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};

pub const COMPONENT_CLASS: &str = "component";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    Build,
    VmLiveBoot,
    VmInstallE2e,
}

impl Phase {
    pub const ALL: [Phase; 3] = [Phase::Build, Phase::VmLiveBoot, Phase::VmInstallE2e];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::VmLiveBoot => "vm-live-boot",
            Self::VmInstallE2e => "vm-install-e2e",
        }
    }

    pub fn next(self) -> Option<Phase> {
        match self {
            Self::Build => Some(Self::VmLiveBoot),
            Self::VmLiveBoot => Some(Self::VmInstallE2e),
            Self::VmInstallE2e => None,
        }
    }
}

impl FromStr for Phase {
    type Err = TrackerError;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim() {
            "build" => Ok(Self::Build),
            "vm-live-boot" => Ok(Self::VmLiveBoot),
            "vm-install-e2e" => Ok(Self::VmInstallE2e),
            other => Err(TrackerError::ValidationError(format!(
                "invalid phase '{other}': expected one of build, vm-live-boot, vm-install-e2e"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FailureSource {
    Bisect,
    SingleChange,
    Interaction,
}

impl FailureSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Bisect => "bisect",
            Self::SingleChange => "single-change",
            Self::Interaction => "interaction",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Failure {
    pub component: String,
    pub version: String,
    pub phase: Phase,
    pub class: String,
    pub classifier_rule: String,
    pub log: String,
    pub recorded_at: String,
    pub source: FailureSource,
    #[serde(default)]
    pub members: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ledger {
    #[serde(default, rename = "failure")]
    pub failures: Vec<Failure>,
}

impl Ledger {
    pub fn has_exact(&self, component: &str, version: &str) -> bool {
        self.failures
            .iter()
            .any(|f| f.component == component && f.version == version)
    }

    pub fn for_component<'a>(
        &'a self,
        component: &'a str,
    ) -> impl Iterator<Item = &'a Failure> + 'a {
        self.failures
            .iter()
            .filter(move |f| f.component == component)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordFailureOutcome {
    Recorded(Failure),
    RejectedClass { class: String },
}

pub fn read_ledger(path: &Path) -> Result<Ledger> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(Ledger::default()),
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

pub fn render_ledger(l: &Ledger) -> Result<String> {
    let body = toml::to_string(l)?;
    Ok(format!(
        "# NovOS failure ledger (.tracker/release/failures.toml)\n\
         # Written by neb release record-failure and neb release bisect. Delete an entry by hand to release its hold.\n\n\
         {body}"
    ))
}

/// Validates and appends a component failure to `failures.toml`.
///
/// If `entry.class` is not [`COMPONENT_CLASS`] (`"component"`), returns
/// [`RecordFailureOutcome::RejectedClass`] without creating or modifying the ledger (§4.9:
/// `infra` and `unknown` failures never reach the ledger).
///
/// The caller must hold `TrackerStorage::with_write_lock` and supply `entry.recorded_at`.
pub fn record_failure(path: &Path, entry: Failure) -> Result<RecordFailureOutcome> {
    if entry.class != COMPONENT_CLASS {
        return Ok(RecordFailureOutcome::RejectedClass { class: entry.class });
    }

    if entry.component.trim().is_empty() {
        return Err(TrackerError::ValidationError(
            "failure entry 'component' must not be blank".to_string(),
        ));
    }
    if entry.version.trim().is_empty() {
        return Err(TrackerError::ValidationError(
            "failure entry 'version' must not be blank".to_string(),
        ));
    }
    if entry.classifier_rule.trim().is_empty() {
        return Err(TrackerError::ValidationError(
            "failure entry 'classifier_rule' must not be blank".to_string(),
        ));
    }

    let mut ledger = read_ledger(path)?;
    ledger.failures.push(entry.clone());
    let text = render_ledger(&ledger)?;

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|source| TrackerError::Io {
                path: parent.to_path_buf(),
                source,
            })?;
        }
    }

    crate::storage::atomic_write_str(path, &text)?;
    Ok(RecordFailureOutcome::Recorded(entry))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::*;

    fn sample_failure(component: &str, version: &str, phase: Phase, class: &str) -> Failure {
        Failure {
            component: component.to_string(),
            version: version.to_string(),
            phase,
            class: class.to_string(),
            classifier_rule: "apt-breaks".to_string(),
            log: "logs/release-candidate-20261108T164600Z/round-0/build.log".to_string(),
            recorded_at: "2026-11-08T16:46:00Z".to_string(),
            source: FailureSource::Bisect,
            members: Vec::new(),
        }
    }

    #[test]
    fn phase_serde_all_next_and_parse() {
        assert_eq!(
            Phase::ALL,
            [Phase::Build, Phase::VmLiveBoot, Phase::VmInstallE2e]
        );
        assert_eq!(Phase::Build.as_str(), "build");
        assert_eq!(Phase::VmLiveBoot.as_str(), "vm-live-boot");
        assert_eq!(Phase::VmInstallE2e.as_str(), "vm-install-e2e");

        assert_eq!(Phase::Build.next(), Some(Phase::VmLiveBoot));
        assert_eq!(Phase::VmLiveBoot.next(), Some(Phase::VmInstallE2e));
        assert_eq!(Phase::VmInstallE2e.next(), None);

        assert!(Phase::Build < Phase::VmLiveBoot);
        assert!(Phase::VmLiveBoot < Phase::VmInstallE2e);

        assert_eq!("build".parse::<Phase>().expect("parse build"), Phase::Build);
        assert_eq!(
            "vm-live-boot".parse::<Phase>().expect("parse vm-live-boot"),
            Phase::VmLiveBoot
        );
        assert_eq!(
            "vm-install-e2e"
                .parse::<Phase>()
                .expect("parse vm-install-e2e"),
            Phase::VmInstallE2e
        );

        let err = "live-boot"
            .parse::<Phase>()
            .expect_err("live-boot must fail");
        match err {
            TrackerError::ValidationError(msg) => {
                assert!(msg.contains("build"));
                assert!(msg.contains("vm-live-boot"));
                assert!(msg.contains("vm-install-e2e"));
            }
            other => panic!("expected ValidationError, got {other:?}"),
        }

        let json = serde_json::to_string(&Phase::ALL).expect("serialize Phase::ALL");
        assert_eq!(json, r#"["build","vm-live-boot","vm-install-e2e"]"#);
        let round: [Phase; 3] = serde_json::from_str(&json).expect("deserialize Phase::ALL");
        assert_eq!(round, Phase::ALL);
    }

    #[test]
    fn record_failure_rejects_infra_and_unknown_without_creating_file() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("release/failures.toml");

        for class in ["infra", "unknown"] {
            let entry = sample_failure("example-pkg", "2.5.5-1+1", Phase::Build, class);
            let outcome = record_failure(&path, entry).expect("record_failure call");
            assert_eq!(
                outcome,
                RecordFailureOutcome::RejectedClass {
                    class: class.to_string()
                }
            );
            assert!(!path.exists(), "file must not be created for class={class}");
        }

        let ledger = read_ledger(&path).expect("read missing ledger");
        assert!(ledger.failures.is_empty());
    }

    #[test]
    fn record_failure_appends_component_entries_validates_fields_and_roundtrips_members() {
        let tmp = tempdir().expect("tempdir");
        let path = tmp.path().join("release/failures.toml");

        let mut blank_ver = sample_failure("example-pkg", "   ", Phase::Build, "component");
        let err = record_failure(&path, blank_ver.clone()).expect_err("blank version must fail");
        match err {
            TrackerError::ValidationError(msg) => assert!(msg.contains("version")),
            other => panic!("expected ValidationError, got {other:?}"),
        }
        blank_ver.version = "2.5.5-1+1".to_string();
        blank_ver.component = "".to_string();
        let err = record_failure(&path, blank_ver.clone()).expect_err("blank component must fail");
        match err {
            TrackerError::ValidationError(msg) => assert!(msg.contains("component")),
            other => panic!("expected ValidationError, got {other:?}"),
        }
        blank_ver.component = "example-pkg".to_string();
        blank_ver.classifier_rule = "\t".to_string();
        let err = record_failure(&path, blank_ver).expect_err("blank classifier_rule must fail");
        match err {
            TrackerError::ValidationError(msg) => assert!(msg.contains("classifier_rule")),
            other => panic!("expected ValidationError, got {other:?}"),
        }
        assert!(!path.exists());

        let first = sample_failure("example-pkg", "2.5.5-1+1", Phase::Build, "component");
        let mut second = sample_failure("example-pkg", "2.5.5-1+1", Phase::VmLiveBoot, "component");
        second.source = FailureSource::SingleChange;

        let mut third = sample_failure("example-net", "1.4.0-2", Phase::VmInstallE2e, "component");
        third.source = FailureSource::Interaction;
        third.members = vec!["example-net".to_string(), "example-pkg".to_string()];

        assert_eq!(
            record_failure(&path, first.clone()).expect("record first"),
            RecordFailureOutcome::Recorded(first.clone())
        );
        assert_eq!(
            record_failure(&path, second.clone()).expect("record second"),
            RecordFailureOutcome::Recorded(second.clone())
        );
        assert_eq!(
            record_failure(&path, third.clone()).expect("record third"),
            RecordFailureOutcome::Recorded(third.clone())
        );

        let raw = fs::read_to_string(&path).expect("read rendered file");
        assert!(raw.starts_with(
            "# NovOS failure ledger (.tracker/release/failures.toml)\n\
             # Written by neb release record-failure and neb release bisect. Delete an entry by hand to release its hold.\n\n"
        ));
        assert!(raw.contains("[[failure]]"));
        assert!(raw.contains("source = \"single-change\""));
        assert!(raw.contains("source = \"interaction\""));

        let loaded = read_ledger(&path).expect("read_ledger");
        assert_eq!(loaded.failures, vec![first.clone(), second.clone(), third]);

        assert!(loaded.has_exact("example-pkg", "2.5.5-1+1"));
        assert!(!loaded.has_exact("example-pkg", "2.5.2-1+2"));
        assert!(!loaded.has_exact("other-pkg", "2.5.5-1+1"));

        let pkg_entries: Vec<&Failure> = loaded.for_component("example-pkg").collect();
        assert_eq!(pkg_entries, vec![&first, &second]);
    }
}
