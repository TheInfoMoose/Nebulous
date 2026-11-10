//! `neb release`: replaces NovOS scripts/nebulous-ctl.py and scripts/mcp_nebulous_ci.py. Spec: /srv/devshare/NovOS/docs/NEB_RELEASE_CONTRACT.md.

pub mod component;
pub mod config;
pub mod debver;
pub mod gate;
pub mod golden;
pub mod inventory;
pub mod iso;
pub mod rootfs;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::TrackerError;

pub const ENVELOPE_SCHEMA: &str = "neb.release/v1";

pub const STATUS_PASS: &str = "PASS";
pub const HARD_STOP_VARIANCE_LOCK: &str = "HARD_STOP_VARIANCE_LOCK";
pub const HARD_STOP_LOOP_LOCK: &str = "HARD_STOP_LOOP_LOCK";
pub const HARD_STOP_PERMISSION_DENIED: &str = "HARD_STOP_PERMISSION_DENIED";

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitCode {
    Ok = 0,
    Error = 1,
    VarianceLock = 2,
    LoopLock = 3,
    PermissionDenied = 4,
    NotReady = 5,
    BisectStop = 6,
    StateConflict = 7,
}

impl ExitCode {
    pub fn code(self) -> i32 {
        self as i32
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Ok => "EXIT_OK",
            Self::Error => "EXIT_ERROR",
            Self::VarianceLock => "EXIT_VARIANCE_LOCK",
            Self::LoopLock => "EXIT_LOOP_LOCK",
            Self::PermissionDenied => "EXIT_PERMISSION_DENIED",
            Self::NotReady => "EXIT_NOT_READY",
            Self::BisectStop => "EXIT_BISECT_STOP",
            Self::StateConflict => "EXIT_STATE_CONFLICT",
        }
    }

    pub fn from_error(e: &TrackerError) -> Self {
        match e {
            TrackerError::LockTimeout { .. } | TrackerError::UnsupportedSchema { .. } => {
                Self::StateConflict
            }
            _ => Self::Error,
        }
    }
}

pub fn envelope(command: &str, status: &str, exit: ExitCode, body: Value) -> Value {
    let mut out = Map::new();
    out.insert(
        "schema".to_string(),
        Value::String(ENVELOPE_SCHEMA.to_string()),
    );
    out.insert("command".to_string(), Value::String(command.to_string()));
    out.insert("status".to_string(), Value::String(status.to_string()));
    out.insert("exit_code".to_string(), Value::from(exit.code()));

    match body {
        Value::Object(map) => {
            for (k, v) in map {
                if !matches!(k.as_str(), "schema" | "command" | "status" | "exit_code") {
                    out.insert(k, v);
                }
            }
        }
        Value::Null => {}
        other => {
            out.insert("data".to_string(), other);
        }
    }

    Value::Object(out)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Violation {
    #[serde(rename = "type")]
    pub kind: String,
    pub item: String,
    pub detail: String,
}

impl Violation {
    pub fn new(
        kind: impl Into<String>,
        item: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            kind: kind.into(),
            item: item.into(),
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Outcome {
    pub exit: ExitCode,
    pub status: String,
    pub body: Value,
}

impl Outcome {
    pub fn envelope(&self, command: &str) -> Value {
        envelope(command, &self.status, self.exit, self.body.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::json;

    use super::*;

    #[test]
    fn exit_codes_and_names_match_contract() {
        let cases = [
            (ExitCode::Ok, 0, "EXIT_OK"),
            (ExitCode::Error, 1, "EXIT_ERROR"),
            (ExitCode::VarianceLock, 2, "EXIT_VARIANCE_LOCK"),
            (ExitCode::LoopLock, 3, "EXIT_LOOP_LOCK"),
            (ExitCode::PermissionDenied, 4, "EXIT_PERMISSION_DENIED"),
            (ExitCode::NotReady, 5, "EXIT_NOT_READY"),
            (ExitCode::BisectStop, 6, "EXIT_BISECT_STOP"),
            (ExitCode::StateConflict, 7, "EXIT_STATE_CONFLICT"),
        ];

        for (variant, expected_code, expected_name) in cases {
            assert_eq!(variant.code(), expected_code);
            assert_eq!(variant.name(), expected_name);
        }
    }

    #[test]
    fn status_constants_match_contract() {
        assert_eq!(STATUS_PASS, "PASS");
        assert_eq!(HARD_STOP_VARIANCE_LOCK, "HARD_STOP_VARIANCE_LOCK");
        assert_eq!(HARD_STOP_LOOP_LOCK, "HARD_STOP_LOOP_LOCK");
        assert_eq!(HARD_STOP_PERMISSION_DENIED, "HARD_STOP_PERMISSION_DENIED");
    }

    #[test]
    fn envelope_merges_object_body_and_envelope_keys_win_on_clash() {
        let body = json!({
            "checked": 12,
            "violations": [],
            "schema": "wrong/v0",
            "command": "wrong-cmd",
            "status": "WRONG",
            "exit_code": 99
        });

        let env = envelope("verify-variance", STATUS_PASS, ExitCode::Ok, body);
        assert_eq!(
            env,
            json!({
                "schema": "neb.release/v1",
                "command": "verify-variance",
                "status": "PASS",
                "exit_code": 0,
                "checked": 12,
                "violations": []
            })
        );
    }

    #[test]
    fn envelope_handles_null_and_non_object_bodies() {
        let null_env = envelope("status", STATUS_PASS, ExitCode::Ok, Value::Null);
        assert_eq!(
            null_env,
            json!({
                "schema": "neb.release/v1",
                "command": "status",
                "status": "PASS",
                "exit_code": 0
            })
        );

        let array_env = Outcome {
            exit: ExitCode::VarianceLock,
            status: HARD_STOP_VARIANCE_LOCK.to_string(),
            body: json!(["item-a", "item-b"]),
        }
        .envelope("verify-variance");
        assert_eq!(
            array_env,
            json!({
                "schema": "neb.release/v1",
                "command": "verify-variance",
                "status": "HARD_STOP_VARIANCE_LOCK",
                "exit_code": 2,
                "data": ["item-a", "item-b"]
            })
        );
    }

    #[test]
    fn violation_serializes_kind_as_type() {
        let v = Violation::new(
            "package_removed",
            "linux-image-xanmod",
            "missing from ISO inventory",
        );
        let serialized = serde_json::to_value(&v).expect("serialize Violation");
        assert_eq!(
            serialized,
            json!({
                "type": "package_removed",
                "item": "linux-image-xanmod",
                "detail": "missing from ISO inventory"
            })
        );

        let round_trip: Violation =
            serde_json::from_value(serialized).expect("deserialize Violation");
        assert_eq!(round_trip, v);
    }

    #[test]
    fn from_error_maps_lock_timeout_and_unsupported_schema_to_state_conflict() {
        let lock_err = TrackerError::LockTimeout {
            path: PathBuf::from(".tracker/.lock"),
            waited_ms: 5000,
        };
        assert_eq!(ExitCode::from_error(&lock_err), ExitCode::StateConflict);
        assert_eq!(ExitCode::from_error(&lock_err).code(), 7);

        let schema_err = TrackerError::UnsupportedSchema {
            path: PathBuf::from(".tracker/project.toml"),
            found: 99,
            supported: 1,
        };
        assert_eq!(ExitCode::from_error(&schema_err), ExitCode::StateConflict);

        let other_err = TrackerError::ValidationError("invalid arg".to_string());
        assert_eq!(ExitCode::from_error(&other_err), ExitCode::Error);
    }
}
