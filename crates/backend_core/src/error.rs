use std::path::PathBuf;
use thiserror::Error;

/// Result alias for `backend_core` operations.
pub type Result<T> = std::result::Result<T, TrackerError>;

/// Strongly-typed error enum for all Nebulous `.tracker/` persistence, model, workflow, and sync operations.
#[derive(Debug, Error)]
pub enum TrackerError {
    #[error("I/O error at '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("I/O error: {0}")]
    RawIo(#[from] std::io::Error),

    #[error("TOML deserialization error{}: {source}", path.as_ref().map(|p| format!(" in '{}'", p.display())).unwrap_or_default())]
    TomlDeserialize {
        path: Option<PathBuf>,
        // Boxed so TrackerError stays under clippy's 128-byte large-error limit on Windows (where PathBuf is larger).
        #[source]
        source: Box<toml::de::Error>,
    },

    #[error("TOML serialization error: {0}")]
    TomlSerialize(#[from] toml::ser::Error),

    #[error("Invalid value '{value}' for {enum_name} (expected one of: {expected})")]
    InvalidEnumValue {
        enum_name: &'static str,
        value: String,
        expected: &'static str,
    },

    #[error("Invalid date/timestamp '{value}': {reason}")]
    InvalidDateTime { value: String, reason: String },

    #[error("{kind} '{id}' not found")]
    NotFound { kind: &'static str, id: String },

    #[error("{kind} '{id}' already exists")]
    AlreadyExists { kind: &'static str, id: String },

    #[error("Invalid status transition for '{item_id}' from '{from}' to '{to}': {reason}")]
    InvalidTransition {
        item_id: String,
        from: String,
        to: String,
        reason: String,
    },

    #[error("Timer is already running for item '{item_id}'")]
    TimerAlreadyRunning { item_id: String },

    #[error("No active timer is currently running")]
    NoActiveTimer,

    #[error(
        "Could not locate '.tracker' directory starting from '{0}'. \
         Run 'neb init' to create one, or pass --root <PATH>."
    )]
    TrackerRootNotFound(PathBuf),

    #[error(
        "No '.tracker' workspace found starting from '{0}' and no active project is registered \
         in projects.toml. Run 'neb init' here, 'neb project add <PATH>', \
         or 'neb --root <PATH>'."
    )]
    NoWorkspaceForTui(PathBuf),

    #[error("Git synchronization error: {0}")]
    GitError(String),

    #[error("Keyring credential error: {0}")]
    KeyringError(String),

    #[error("Filesystem watcher error: {0}")]
    WatcherError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("timed out after {waited_ms} ms waiting for tracker lock {path}")]
    LockTimeout { path: PathBuf, waited_ms: u64 },

    #[error(
        "{path} has schema_version {found}; this neb supports up to {supported}. Upgrade neb."
    )]
    UnsupportedSchema {
        path: PathBuf,
        found: u32,
        supported: u32,
    },
}

impl From<toml::de::Error> for TrackerError {
    fn from(source: toml::de::Error) -> Self {
        Self::TomlDeserialize {
            path: None,
            source: Box::new(source),
        }
    }
}

impl From<git2::Error> for TrackerError {
    fn from(source: git2::Error) -> Self {
        Self::GitError(source.to_string())
    }
}

impl From<keyring::Error> for TrackerError {
    fn from(source: keyring::Error) -> Self {
        Self::KeyringError(source.to_string())
    }
}

impl From<notify::Error> for TrackerError {
    fn from(source: notify::Error) -> Self {
        Self::WatcherError(source.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toml_deserialize_error_keeps_message() {
        let err: TrackerError = toml::from_str::<toml::Value>("a = [").unwrap_err().into();
        assert!(matches!(
            err,
            TrackerError::TomlDeserialize { path: None, .. }
        ));
        assert!(err.to_string().starts_with("TOML deserialization error: "));
    }
}
