pub mod baseline;

use std::path::{Path, PathBuf};

use backend_core::{
    release::{
        config::{load_config, ReleaseConfig, ReleasePaths},
        envelope, ExitCode,
    },
    Result, TrackerError, TrackerStorage,
};
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};

use crate::commands::resolve_storage;

/// Outcome of a `neb release` handler before rendering.
#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseRun {
    pub exit: ExitCode,
    pub status: String,
    pub body: Value,
    pub text: String,
    pub text_to_stderr: bool,
}

impl ReleaseRun {
    pub fn new(
        exit: ExitCode,
        status: impl Into<String>,
        body: Value,
        text: impl Into<String>,
    ) -> Self {
        Self {
            exit,
            status: status.into(),
            body,
            text: text.into(),
            text_to_stderr: false,
        }
    }

    pub fn from_error(e: &TrackerError) -> Self {
        let exit = ExitCode::from_error(e);
        let status = match exit {
            ExitCode::StateConflict => "STATE_CONFLICT",
            _ => "ERROR",
        };
        let msg = e.to_string();
        Self {
            exit,
            status: status.to_string(),
            body: json!({ "error": msg }),
            text: format!("Error: {msg}"),
            text_to_stderr: true,
        }
    }
}

/// Formatted streams and process exit code for a [`ReleaseRun`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rendered {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

fn ensure_trailing_newline(text: &str) -> String {
    if text.is_empty() {
        String::new()
    } else if text.ends_with('\n') {
        text.to_string()
    } else {
        format!("{text}\n")
    }
}

/// Renders a [`ReleaseRun`] into `stdout`, `stderr`, and exit `code`.
pub fn render(command: &str, run: &ReleaseRun, json: bool) -> Rendered {
    let text = ensure_trailing_newline(&run.text);
    let code = run.exit.code();

    if json {
        let env = envelope(command, &run.status, run.exit, run.body.clone());
        let mut stdout = serde_json::to_string_pretty(&env).unwrap_or_default();
        stdout.push('\n');
        Rendered {
            stdout,
            stderr: text,
            code,
        }
    } else if run.text_to_stderr {
        Rendered {
            stdout: String::new(),
            stderr: text,
            code,
        }
    } else {
        Rendered {
            stdout: text,
            stderr: String::new(),
            code,
        }
    }
}

/// Shared execution context for `neb release` handlers.
pub struct ReleaseCtx {
    pub storage: TrackerStorage,
    pub paths: ReleasePaths,
    pub config: ReleaseConfig,
    pub json: bool,
    pub now: DateTime<Utc>,
}

impl ReleaseCtx {
    pub fn open(root: Option<&Path>, json: bool, now: DateTime<Utc>) -> Result<Self> {
        let storage = resolve_storage(root)?;
        let paths = ReleasePaths::new(&storage);
        let config = load_config(&paths)?;
        Ok(Self {
            storage,
            paths,
            config,
            json,
            now,
        })
    }

    pub fn project_root(&self) -> PathBuf {
        self.storage.project_root()
    }

    pub fn timestamp(&self) -> String {
        self.now.to_rfc3339_opts(SecondsFormat::Secs, true)
    }
}

/// Resolves `--item <id|path>` into a loop-state item file path.
///
/// Any value containing `/` or ending in `.toml` is treated as a literal path;
/// otherwise it is validated and resolved as a loop-state item ID.
pub fn resolve_item_path(paths: &ReleasePaths, item: &str) -> Result<PathBuf> {
    if item.contains('/') || item.ends_with(".toml") {
        Ok(PathBuf::from(item))
    } else {
        paths.loop_state_item(item)
    }
}

fn run_or_error(f: impl FnOnce() -> Result<ReleaseRun>) -> ReleaseRun {
    match f() {
        Ok(run) => run,
        Err(ref err) => ReleaseRun::from_error(err),
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn render_json_vs_human_stderr_rule_and_trailing_newline() {
        let run = ReleaseRun::new(
            ExitCode::Ok,
            "PASS",
            json!({ "checked": 3 }),
            "all checks passed",
        );

        let human = render("verify-variance", &run, false);
        assert_eq!(human.code, 0);
        assert_eq!(human.stdout, "all checks passed\n");
        assert_eq!(human.stderr, "");

        let as_json = render("verify-variance", &run, true);
        assert_eq!(as_json.code, 0);
        assert_eq!(as_json.stderr, "all checks passed\n");
        assert!(as_json.stdout.ends_with('\n'));
        let parsed: Value = serde_json::from_str(&as_json.stdout).expect("valid json");
        assert_eq!(parsed["schema"], "neb.release/v1");
        assert_eq!(parsed["command"], "verify-variance");
        assert_eq!(parsed["status"], "PASS");
        assert_eq!(parsed["exit_code"], 0);
        assert_eq!(parsed["checked"], 3);

        // Already-newline-terminated text does not get a double newline.
        let run_nl = ReleaseRun::new(ExitCode::Ok, "PASS", json!({}), "already newline\n");
        let human_nl = render("status", &run_nl, false);
        assert_eq!(human_nl.stdout, "already newline\n");

        // Empty text stays empty.
        let run_empty = ReleaseRun::new(ExitCode::Ok, "PASS", json!({}), "");
        let human_empty = render("status", &run_empty, false);
        assert_eq!(human_empty.stdout, "");
        assert_eq!(human_empty.stderr, "");

        // Error run sends text to stderr in both human and json modes.
        let err = TrackerError::ValidationError("bad input".to_string());
        let err_run = ReleaseRun::from_error(&err);
        let human_err = render("status", &err_run, false);
        assert_eq!(human_err.code, 1);
        assert_eq!(human_err.stdout, "");
        assert_eq!(human_err.stderr, "Error: Validation error: bad input\n");

        let json_err = render("status", &err_run, true);
        assert_eq!(json_err.code, 1);
        assert_eq!(json_err.stderr, "Error: Validation error: bad input\n");
        let parsed_err: Value = serde_json::from_str(&json_err.stdout).expect("valid error json");
        assert_eq!(parsed_err["status"], "ERROR");
        assert_eq!(parsed_err["exit_code"], 1);
        assert_eq!(parsed_err["error"], "Validation error: bad input");
    }

    #[test]
    fn from_error_maps_lock_timeout_and_validation_error() {
        let lock_err = TrackerError::LockTimeout {
            path: PathBuf::from("/tmp/neb-test/.tracker/.lock"),
            waited_ms: 30_000,
        };
        let lock_run = ReleaseRun::from_error(&lock_err);
        assert_eq!(lock_run.exit, ExitCode::StateConflict);
        assert_eq!(lock_run.exit.code(), 7);
        assert_eq!(lock_run.status, "STATE_CONFLICT");
        assert!(lock_run.text_to_stderr);
        assert!(lock_run.text.starts_with("Error: "));
        assert_eq!(lock_run.body["error"], lock_err.to_string());

        let schema_err = TrackerError::UnsupportedSchema {
            path: PathBuf::from("/tmp/neb-test/golden.toml"),
            found: 99,
            supported: 2,
        };
        let schema_run = ReleaseRun::from_error(&schema_err);
        assert_eq!(schema_run.exit, ExitCode::StateConflict);
        assert_eq!(schema_run.status, "STATE_CONFLICT");

        let val_err = TrackerError::ValidationError("invalid flag".to_string());
        let val_run = ReleaseRun::from_error(&val_err);
        assert_eq!(val_run.exit, ExitCode::Error);
        assert_eq!(val_run.exit.code(), 1);
        assert_eq!(val_run.status, "ERROR");
        assert!(val_run.text_to_stderr);
        assert_eq!(val_run.body["error"], val_err.to_string());
    }

    #[test]
    fn resolve_item_path_handles_id_path_and_invalid_id() {
        let tmp = tempdir().expect("tempdir");
        let storage = TrackerStorage::init_standard(tmp.path(), "test-proj").expect("init");
        let paths = ReleasePaths::new(&storage);

        let by_id = resolve_item_path(&paths, "item-001-example").expect("valid id");
        assert_eq!(by_id, paths.loop_state_dir().join("item-001-example.toml"));

        let by_toml = resolve_item_path(&paths, "custom-item.toml").expect("toml suffix");
        assert_eq!(by_toml, PathBuf::from("custom-item.toml"));

        let by_slash =
            resolve_item_path(&paths, "/tmp/neb-test/item-001-example").expect("slash path");
        assert_eq!(by_slash, PathBuf::from("/tmp/neb-test/item-001-example"));

        let bad = resolve_item_path(&paths, "bad id");
        assert!(bad.is_err());
    }

    #[test]
    fn release_ctx_open_and_timestamp() {
        let tmp = tempdir().expect("tempdir");
        let _storage = TrackerStorage::init_standard(tmp.path(), "test-proj").expect("init");
        let fixed_now = Utc.with_ymd_and_hms(2026, 11, 11, 12, 0, 0).unwrap();

        let ctx = ReleaseCtx::open(Some(tmp.path()), true, fixed_now).expect("open ctx");
        assert!(ctx.json);
        assert_eq!(ctx.timestamp(), "2026-11-11T12:00:00Z");
        assert_eq!(
            std::fs::canonicalize(ctx.project_root()).unwrap(),
            std::fs::canonicalize(tmp.path()).unwrap()
        );
    }
}
