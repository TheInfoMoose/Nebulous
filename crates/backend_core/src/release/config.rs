use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};
use crate::storage::{is_valid_identifier, TrackerStorage};

pub const DEFAULT_VARIANCE_POLICY: &str = "asymmetric_forward_only";
pub const DEFAULT_AGENT_ENV_MARKERS: &[&str] = &[
    "CLAUDECODE",
    "CLAUDE_CODE_ENTRYPOINT",
    "ANTIGRAVITY_AGENT",
    "NEB_AGENT",
    "CI",
];
pub const DEFAULT_BUILD_HARNESS: &[&str] = &[
    "docker",
    "compose",
    "-f",
    "docker/docker-compose.ci.yml",
    "run",
    "--rm",
    "novos-builder",
];
pub const DEFAULT_QEMU_HARNESS: &[&str] = &["python3", "-I", "scripts/test-docker-qemu.py"];
pub const DEFAULT_VERDICT_PATH: &str = "logs/qemu-artifacts/last-test-verdict.json";
pub const DEFAULT_KERNEL_LINES: &[&str] = &["linux-xanmod-x64v3", "linux-image-amd64"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePaths {
    root: PathBuf,
}

impl ReleasePaths {
    pub fn new(storage: &TrackerStorage) -> Self {
        Self {
            root: storage.tracker_dir().join("release"),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> PathBuf {
        self.root.join("manifest.toml")
    }

    pub fn golden(&self) -> PathBuf {
        self.root.join("golden.toml")
    }

    pub fn approvals(&self) -> PathBuf {
        self.root.join("approvals.toml")
    }

    pub fn releases_dir(&self) -> PathBuf {
        self.root.join("releases")
    }

    pub fn candidate(&self) -> PathBuf {
        self.root.join("candidate.toml")
    }

    pub fn failures(&self) -> PathBuf {
        self.root.join("failures.toml")
    }

    pub fn bisect(&self) -> PathBuf {
        self.root.join("bisect.toml")
    }

    pub fn loop_state_dir(&self) -> PathBuf {
        self.root.join("loop-state")
    }

    pub fn config(&self) -> PathBuf {
        self.root.join("config.toml")
    }

    pub fn loop_state_item(&self, id: &str) -> Result<PathBuf> {
        if !is_valid_identifier(id) {
            return Err(TrackerError::ValidationError(format!(
                "Invalid loop-state item identifier '{id}': must match ^[A-Za-z0-9][A-Za-z0-9_-]{{0,63}}$ \
                 and must not be a reserved Windows device name"
            )));
        }
        Ok(self.loop_state_dir().join(format!("{id}.toml")))
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ReleaseConfig {
    pub guardrails: Guardrails,
    pub harness: Harness,
    pub components: Components,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Guardrails {
    pub variance_policy: String,
    pub allow_package_removal: bool,
    pub allow_version_downgrade: bool,
    pub allow_custom_tool_disable: bool,
    pub loop_repeat_warn_threshold: u32,
    pub loop_max_retries_hard_stop: u32,
    pub require_fresh_stage_1_to_6_build: bool,
    pub agent_env_markers: Vec<String>,
}

impl Default for Guardrails {
    fn default() -> Self {
        Self {
            variance_policy: DEFAULT_VARIANCE_POLICY.to_string(),
            allow_package_removal: false,
            allow_version_downgrade: false,
            allow_custom_tool_disable: false,
            loop_repeat_warn_threshold: 2,
            loop_max_retries_hard_stop: 3,
            require_fresh_stage_1_to_6_build: true,
            agent_env_markers: DEFAULT_AGENT_ENV_MARKERS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Harness {
    pub build: Vec<String>,
    pub qemu: Vec<String>,
    pub verdict: String,
}

impl Default for Harness {
    fn default() -> Self {
        Self {
            build: DEFAULT_BUILD_HARNESS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            qemu: DEFAULT_QEMU_HARNESS
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            verdict: DEFAULT_VERDICT_PATH.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Components {
    pub kernel_lines: Vec<String>,
    pub third_party: Vec<String>,
}

impl Default for Components {
    fn default() -> Self {
        Self {
            kernel_lines: DEFAULT_KERNEL_LINES
                .iter()
                .map(|s| (*s).to_string())
                .collect(),
            third_party: Vec::new(),
        }
    }
}

pub fn load_config(paths: &ReleasePaths) -> Result<ReleaseConfig> {
    let config_path = paths.config();
    if !config_path.exists() {
        return Ok(ReleaseConfig::default());
    }

    let raw = fs::read_to_string(&config_path).map_err(|source| TrackerError::Io {
        path: config_path.clone(),
        source,
    })?;
    let cfg: ReleaseConfig =
        toml::from_str(&raw).map_err(|source| TrackerError::TomlDeserialize {
            path: Some(config_path.clone()),
            source,
        })?;

    validate_config(&cfg, &config_path)?;
    Ok(cfg)
}

fn validate_config(cfg: &ReleaseConfig, path: &Path) -> Result<()> {
    let relaxed = |key: &str| {
        TrackerError::ValidationError(format!(
            "release guardrail {key} is relaxed in {}; the forward-only policy cannot be changed",
            path.display()
        ))
    };

    if cfg.guardrails.variance_policy != DEFAULT_VARIANCE_POLICY {
        return Err(relaxed("variance_policy"));
    }
    if cfg.guardrails.allow_package_removal {
        return Err(relaxed("allow_package_removal"));
    }
    if cfg.guardrails.allow_version_downgrade {
        return Err(relaxed("allow_version_downgrade"));
    }
    if cfg.guardrails.allow_custom_tool_disable {
        return Err(relaxed("allow_custom_tool_disable"));
    }
    if cfg.guardrails.agent_env_markers.is_empty() {
        return Err(TrackerError::ValidationError(format!(
            "release guardrail agent_env_markers is empty in {}; the user-only gate cannot be configured away",
            path.display()
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;
    use crate::release::ExitCode;

    const CONTRACT_EXAMPLE_CONFIG: &str = r#"[guardrails]
variance_policy = "asymmetric_forward_only"
allow_package_removal = false
allow_version_downgrade = false
allow_custom_tool_disable = false
loop_repeat_warn_threshold = 2
loop_max_retries_hard_stop = 3
require_fresh_stage_1_to_6_build = true
agent_env_markers = ["CLAUDECODE", "CLAUDE_CODE_ENTRYPOINT", "ANTIGRAVITY_AGENT", "NEB_AGENT", "CI"]

[harness]
build = ["docker", "compose", "-f", "docker/docker-compose.ci.yml", "run", "--rm", "novos-builder"]
qemu = ["python3", "-I", "scripts/test-docker-qemu.py"]
verdict = "logs/qemu-artifacts/last-test-verdict.json"

[components]
kernel_lines = ["linux-xanmod-x64v3", "linux-image-amd64"]
third_party = []
"#;

    fn test_paths() -> (TempDir, ReleasePaths) {
        let tmp = TempDir::new().expect("tempdir");
        let storage = TrackerStorage::init_standard(tmp.path(), "test-proj").expect("init");
        let paths = ReleasePaths::new(&storage);
        (tmp, paths)
    }

    #[test]
    fn release_paths_match_contract_locations_and_validate_loop_state_id() {
        let (_tmp, paths) = test_paths();
        let root = paths.root().to_path_buf();

        assert_eq!(paths.manifest(), root.join("manifest.toml"));
        assert_eq!(paths.golden(), root.join("golden.toml"));
        assert_eq!(paths.approvals(), root.join("approvals.toml"));
        assert_eq!(paths.releases_dir(), root.join("releases"));
        assert_eq!(paths.candidate(), root.join("candidate.toml"));
        assert_eq!(paths.failures(), root.join("failures.toml"));
        assert_eq!(paths.bisect(), root.join("bisect.toml"));
        assert_eq!(paths.loop_state_dir(), root.join("loop-state"));
        assert_eq!(paths.config(), root.join("config.toml"));

        assert_eq!(
            paths.loop_state_item("BUG-0001").expect("valid id"),
            root.join("loop-state").join("BUG-0001.toml")
        );

        let err = paths
            .loop_state_item("../x")
            .expect_err("parent traversal must fail");
        assert!(matches!(err, TrackerError::ValidationError(_)));
    }

    #[test]
    fn missing_config_returns_defaults() {
        let (_tmp, paths) = test_paths();
        let cfg = load_config(&paths).expect("load missing config");
        assert_eq!(cfg, ReleaseConfig::default());
    }

    #[test]
    fn contract_example_parses_equal_to_default() {
        let (_tmp, paths) = test_paths();
        fs::create_dir_all(paths.root()).expect("create release dir");
        fs::write(paths.config(), CONTRACT_EXAMPLE_CONFIG).expect("write config");

        let cfg = load_config(&paths).expect("load example config");
        assert_eq!(cfg, ReleaseConfig::default());
    }

    #[test]
    fn relaxed_forward_only_keys_error_with_key_name() {
        let cases = [
            (
                r#"[guardrails]
variance_policy = "permissive"
"#,
                "variance_policy",
            ),
            (
                r#"[guardrails]
allow_package_removal = true
"#,
                "allow_package_removal",
            ),
            (
                r#"[guardrails]
allow_version_downgrade = true
"#,
                "allow_version_downgrade",
            ),
            (
                r#"[guardrails]
allow_custom_tool_disable = true
"#,
                "allow_custom_tool_disable",
            ),
        ];

        for (toml_text, expected_key) in cases {
            let (_tmp, paths) = test_paths();
            fs::create_dir_all(paths.root()).expect("create release dir");
            fs::write(paths.config(), toml_text).expect("write config");

            let err = load_config(&paths).expect_err("relaxed guardrail must fail");
            assert_eq!(ExitCode::from_error(&err), ExitCode::Error);
            match err {
                TrackerError::ValidationError(msg) => {
                    assert!(
                        msg.contains(&format!("release guardrail {expected_key} is relaxed in")),
                        "unexpected message for {expected_key}: {msg}"
                    );
                    assert!(msg.contains("the forward-only policy cannot be changed"));
                }
                other => panic!("expected ValidationError, got {other:?}"),
            }
        }
    }

    #[test]
    fn empty_agent_env_markers_errors() {
        let (_tmp, paths) = test_paths();
        fs::create_dir_all(paths.root()).expect("create release dir");
        fs::write(
            paths.config(),
            r#"[guardrails]
agent_env_markers = []
"#,
        )
        .expect("write config");

        let err = load_config(&paths).expect_err("empty agent_env_markers must fail");
        assert_eq!(ExitCode::from_error(&err), ExitCode::Error);
        match err {
            TrackerError::ValidationError(msg) => {
                assert!(msg.contains("release guardrail agent_env_markers is empty in"));
                assert!(msg.contains("the user-only gate cannot be configured away"));
            }
            other => panic!("expected ValidationError, got {other:?}"),
        }
    }

    #[test]
    fn partial_guardrails_table_retains_five_default_agent_markers() {
        let (_tmp, paths) = test_paths();
        fs::create_dir_all(paths.root()).expect("create release dir");
        fs::write(
            paths.config(),
            r#"[guardrails]
loop_max_retries_hard_stop = 5
"#,
        )
        .expect("write config");

        let cfg = load_config(&paths).expect("load partial config");
        assert_eq!(cfg.guardrails.loop_max_retries_hard_stop, 5);
        assert_eq!(
            cfg.guardrails.agent_env_markers,
            vec![
                "CLAUDECODE",
                "CLAUDE_CODE_ENTRYPOINT",
                "ANTIGRAVITY_AGENT",
                "NEB_AGENT",
                "CI",
            ]
        );
    }
}
