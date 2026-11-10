pub mod agent_context;
pub mod crypto;
pub mod error;
pub mod git_sync;
pub mod ingestion;
pub mod mcp;
pub mod models;
pub mod release;
pub mod storage;
pub mod watcher;
pub mod workflow;

pub use agent_context::*;
pub use crypto::*;
pub use error::{Result, TrackerError};
pub use git_sync::*;
pub use ingestion::*;
pub use mcp::*;
pub use models::*;
pub use storage::*;
pub use watcher::*;
pub use workflow::{
    format_boilerplate_description, CreateItemInput, NextActionKind, NextRecommendation,
    PhaseEffortValuation, PredictiveDefaults, ProjectMetricsReport, RecurringItemHotspot,
    ScopeEffortValuation, ScopeHotspot, StagnantItemEntry, StoppedTimerSession, TimerStatus,
    WhatsNextReport,
};

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use chrono::TimeZone;

    use crate::error::TrackerError;
    use crate::models::{
        ActiveTimer, GitSyncMode, ItemManifest, ItemStatus, ItemType, PhaseManifest, PhaseStatus,
        ProjectManifest, ScopeManifest, Severity,
    };

    #[test]
    fn test_project_manifest_round_trip_and_unquoted_datetime() {
        let raw = r#"
[project]
name = "Nebulous"
version = "0.1.0"
active_phase = "01_core_engine"
default_scope = "inbox"
created_at = 2026-11-01T21:00:00Z

[counters]
bug = 1
feature = 4

[phases]
registry = [
    { id = "01_core_engine", title = "Core Engine & Parsing", path = "phases/01_core_engine" },
    { id = "02_cloud_sync",  title = "Cloud & Remote Sync",    path = "phases/02_cloud_sync" }
]

[metrics]
target_daily_hours = 6.0
stagnant_item_threshold_days = 4

[git]
sync_mode = "manual"
sync_interval_minutes = 30
remote = "origin"
branch = "main"
"#;

        let manifest =
            ProjectManifest::from_toml_str(raw).expect("should deserialize project.toml");
        assert_eq!(manifest.project.name, "Nebulous");
        assert_eq!(manifest.project.active_phase, "01_core_engine");
        assert_eq!(manifest.project.default_scope, "inbox");
        assert_eq!(manifest.project.created_at, "2026-11-01T21:00:00Z");
        assert!(manifest.project.parsed_created_at().is_ok());
        assert_eq!(manifest.counters.bug, 1);
        assert_eq!(manifest.counters.feature, 4);
        assert_eq!(manifest.phases.registry.len(), 2);
        assert_eq!(manifest.metrics.target_daily_hours, 6.0);
        assert_eq!(manifest.metrics.stagnant_item_threshold_days, 4);
        assert_eq!(manifest.git.sync_mode, GitSyncMode::Manual);
        assert_eq!(manifest.git.remote, "origin");

        let serialized = manifest
            .to_toml_string()
            .expect("should serialize project.toml");
        let reparsed =
            ProjectManifest::from_toml_str(&serialized).expect("should round-trip project.toml");
        assert_eq!(manifest, reparsed);
    }

    #[test]
    fn test_project_manifest_minimal_defaults() {
        let minimal = r#"
[project]
name = "MinimalProject"
active_phase = "01_init"
created_at = "2026-11-01T21:00:00Z"
"#;
        let manifest =
            ProjectManifest::from_toml_str(minimal).expect("should deserialize minimal project");
        assert_eq!(manifest.project.version, "0.1.0");
        assert_eq!(manifest.project.default_scope, "inbox");
        assert_eq!(manifest.counters.bug, 0);
        assert_eq!(manifest.counters.feature, 0);
        assert!(manifest.phases.registry.is_empty());
        assert_eq!(manifest.metrics.target_daily_hours, 6.0);
        assert_eq!(manifest.metrics.stagnant_item_threshold_days, 4);
        assert_eq!(manifest.git.sync_mode, GitSyncMode::Manual);
        assert_eq!(manifest.git.sync_interval_minutes, 30);
    }

    #[test]
    fn test_phase_manifest_round_trip_and_optional_dates() {
        let raw = r#"
[phase]
id = "01_core_engine"
title = "Core Engine & Parsing"
status = "in_progress"
start_date = 2026-11-01

[scopes]
manifest = [
    { id = "backend_core",     type = "rust_crate", path = "domains/backend_core" },
    { id = "nebulous_cli",     type = "cli_crate",  path = "domains/nebulous_cli" },
    { id = "cosmic_interface", type = "ui_crate",   path = "domains/cosmic_interface" }
]

[summary]
total_items = 8
completed_items = 2
"#;

        let manifest = PhaseManifest::from_toml_str(raw).expect("should deserialize phase.toml");
        assert_eq!(manifest.phase.id, "01_core_engine");
        assert_eq!(manifest.phase.status, PhaseStatus::InProgress);
        assert_eq!(manifest.phase.start_date.as_deref(), Some("2026-11-01"));
        assert!(manifest.phase.parsed_start_date().unwrap().is_some());
        assert_eq!(manifest.scopes.manifest.len(), 3);
        assert_eq!(manifest.scopes.manifest[2].id, "cosmic_interface");
        assert_eq!(manifest.scopes.manifest[2].scope_type, "ui_crate");
        assert_eq!(manifest.summary.total_items, 8);
        assert_eq!(manifest.summary.completed_items, 2);

        let serialized = manifest
            .to_toml_string()
            .expect("should serialize phase.toml");
        let reparsed =
            PhaseManifest::from_toml_str(&serialized).expect("should round-trip phase.toml");
        assert_eq!(manifest, reparsed);
    }

    #[test]
    fn test_scope_manifest_populated_and_empty_arrays() {
        let populated = r#"
[scope]
id = "backend_core"
target = "crates/backend_core"
tech_stack = ["rust", "serde", "tokio"]

[items]
bugs = [
    { id = "BUG-0001", title = "TOML deserialization fails on empty array", status = "open", severity = "high" }
]
features = [
    { id = "FEAT-0004", title = "Parallel file-tree walker", status = "in_progress", severity = "medium" }
]
"#;
        let scope =
            ScopeManifest::from_toml_str(populated).expect("should parse populated scope.toml");
        assert_eq!(scope.scope.id, "backend_core");
        assert_eq!(scope.total_items(), 2);
        assert_eq!(scope.items.bugs[0].id, "BUG-0001");
        assert_eq!(scope.items.bugs[0].status, ItemStatus::Open);
        assert_eq!(scope.items.bugs[0].severity, Severity::High);
        assert_eq!(scope.items.features[0].id, "FEAT-0004");
        assert_eq!(scope.items.features[0].status, ItemStatus::InProgress);

        // Edge case 1: Explicit empty arrays `bugs = []` and `features = []`
        let explicit_empty = r#"
[scope]
id = "inbox"
target = ".tracker"
tech_stack = []

[items]
bugs = []
features = []
"#;
        let empty_scope = ScopeManifest::from_toml_str(explicit_empty)
            .expect("should parse scope.toml with explicit empty arrays");
        assert!(empty_scope.items.bugs.is_empty());
        assert!(empty_scope.items.features.is_empty());
        assert_eq!(empty_scope.total_items(), 0);

        // Edge case 2: Omitted `features` or omitted `[items]` section entirely
        let omitted_items = r#"
[scope]
id = "cosmic_interface"
target = "crates/cosmic_interface"
"#;
        let omitted_scope = ScopeManifest::from_toml_str(omitted_items)
            .expect("should parse scope.toml with omitted [items] section");
        assert!(omitted_scope.scope.tech_stack.is_empty());
        assert!(omitted_scope.items.bugs.is_empty());
        assert!(omitted_scope.items.features.is_empty());

        let round_trip = empty_scope.to_toml_string().unwrap();
        let reparsed = ScopeManifest::from_toml_str(&round_trip).unwrap();
        assert_eq!(empty_scope, reparsed);
    }

    #[test]
    fn test_item_manifest_round_trip_and_empty_previous_fixes() {
        let raw = r#"
[item]
id = "BUG-0001"
type = "bug"
title = "TOML deserialization fails on empty array"
status = "open"
severity = "high"
scope_ref = "backend_core"
created_at = 2026-11-01T21:15:00Z
updated_at = 2026-11-01T21:40:00Z

[effort]
logged_time_minutes = 45
iteration_count = 2

[history]
recurrence_count = 1
previous_fixes = [
    { date = 2026-11-01, commit = "a4f89c1", summary = "Added default trait fallback" }
]
"#;

        let item = ItemManifest::from_toml_str(raw).expect("should deserialize item.toml");
        assert_eq!(item.item.id, "BUG-0001");
        assert_eq!(item.item.item_type, ItemType::Bug);
        assert_eq!(item.item.status, ItemStatus::Open);
        assert_eq!(item.item.severity, Severity::High);
        assert_eq!(item.effort.logged_time_minutes, 45);
        assert_eq!(item.effort.iteration_count, 2);
        assert_eq!(item.history.recurrence_count, 1);
        assert_eq!(item.history.previous_fixes.len(), 1);
        assert_eq!(item.history.previous_fixes[0].commit, "a4f89c1");
        assert!(item.history.previous_fixes[0].parsed_date().is_ok());

        let stub = item.to_stub();
        assert_eq!(stub.id, "BUG-0001");
        assert_eq!(stub.status, ItemStatus::Open);
        assert_eq!(stub.severity, Severity::High);

        let serialized = item.to_toml_string().expect("should serialize item.toml");
        let reparsed =
            ItemManifest::from_toml_str(&serialized).expect("should round-trip item.toml");
        assert_eq!(item, reparsed);

        // Edge case: `previous_fixes = []` or omitted `[history]`
        let empty_fixes = r#"
[item]
id = "FEAT-0001"
type = "feature"
title = "Implement 4-tier TOML models"
status = "in_progress"
severity = "medium"
scope_ref = "backend_core"
created_at = "2026-11-01T21:15:00Z"
updated_at = "2026-11-01T21:40:00Z"

[history]
recurrence_count = 0
previous_fixes = []
"#;
        let feat = ItemManifest::from_toml_str(empty_fixes)
            .expect("should parse item.toml with empty previous_fixes");
        assert_eq!(feat.item.item_type, ItemType::Feature);
        assert!(feat.history.previous_fixes.is_empty());
        assert_eq!(feat.effort.logged_time_minutes, 0);
    }

    #[test]
    fn test_enum_parsing_and_severity_ordering() {
        // ItemStatus parsing
        assert_eq!(ItemStatus::from_str("open").unwrap(), ItemStatus::Open);
        assert_eq!(
            ItemStatus::from_str("IN_PROGRESS").unwrap(),
            ItemStatus::InProgress
        );
        assert_eq!(
            ItemStatus::from_str("in-progress").unwrap(),
            ItemStatus::InProgress
        );
        assert_eq!(
            ItemStatus::from_str("pending").unwrap(),
            ItemStatus::Pending
        );
        assert_eq!(ItemStatus::from_str("closed").unwrap(), ItemStatus::Closed);
        assert_eq!(ItemStatus::from_str("merged").unwrap(), ItemStatus::Merged);
        assert!(!ItemStatus::Open.is_completed());
        assert!(!ItemStatus::InProgress.is_completed());
        assert!(ItemStatus::Closed.is_completed());
        assert!(ItemStatus::Merged.is_completed());
        assert!(matches!(
            ItemStatus::from_str("invalid_status"),
            Err(TrackerError::InvalidEnumValue {
                enum_name: "ItemStatus",
                ..
            })
        ));

        // Severity parsing and ordering
        assert_eq!(Severity::from_str("low").unwrap(), Severity::Low);
        assert_eq!(Severity::from_str("med").unwrap(), Severity::Medium);
        assert_eq!(Severity::from_str("HIGH").unwrap(), Severity::High);
        assert_eq!(Severity::from_str("crit").unwrap(), Severity::Critical);
        assert!(Severity::Low < Severity::Medium);
        assert!(Severity::Medium < Severity::High);
        assert!(Severity::High < Severity::Critical);
        assert!(matches!(
            Severity::from_str("blocker"),
            Err(TrackerError::InvalidEnumValue {
                enum_name: "Severity",
                ..
            })
        ));

        // ItemType parsing and ID formatting
        assert_eq!(ItemType::from_str("bug").unwrap(), ItemType::Bug);
        assert_eq!(ItemType::from_str("feat").unwrap(), ItemType::Feature);
        assert_eq!(ItemType::from_str("feature").unwrap(), ItemType::Feature);
        assert_eq!(ItemType::Bug.format_id(1), "BUG-0001");
        assert_eq!(ItemType::Feature.format_id(42), "FEAT-0042");

        // PhaseStatus & GitSyncMode parsing
        assert_eq!(
            PhaseStatus::from_str("in-progress").unwrap(),
            PhaseStatus::InProgress
        );
        assert_eq!(
            GitSyncMode::from_str("scheduled").unwrap(),
            GitSyncMode::Scheduled
        );
    }

    #[test]
    fn test_active_timer_serialization_and_elapsed_calculation() {
        let timer = ActiveTimer::new(
            "BUG-0001",
            Some("01_core_engine".to_string()),
            Some("backend_core".to_string()),
            "2026-11-01T21:00:00Z",
        );
        let raw = timer.to_toml_string().expect("should serialize timer");
        let parsed = ActiveTimer::from_toml_str(&raw).expect("should deserialize timer");
        assert_eq!(timer, parsed);

        let later = chrono::Utc
            .with_ymd_and_hms(2026, 11, 1, 21, 45, 30)
            .unwrap();
        assert_eq!(parsed.elapsed_minutes_at(later).unwrap(), 45);

        // If `now` is earlier than `started_at`, elapsed minutes clamps to 0
        let earlier = chrono::Utc
            .with_ymd_and_hms(2026, 11, 1, 20, 30, 0)
            .unwrap();
        assert_eq!(parsed.elapsed_minutes_at(earlier).unwrap(), 0);
    }

    #[test]
    fn test_local_datetime_item_stub_defaults_and_malformed_inputs() {
        use crate::models::{parse_date, parse_timestamp_utc};

        // Local TOML datetimes without 'Z' offset
        let dt = parse_timestamp_utc("2026-11-01T21:00:00").expect("should parse local datetime");
        assert_eq!(
            dt,
            chrono::Utc.with_ymd_and_hms(2026, 11, 1, 21, 0, 0).unwrap()
        );
        let d = parse_date("2026-11-01T21:00:00").expect("should extract date from local datetime");
        assert_eq!(d, chrono::NaiveDate::from_ymd_opt(2026, 11, 1).unwrap());

        // Invalid datetime string
        assert!(matches!(
            parse_timestamp_utc("not-a-timestamp"),
            Err(TrackerError::InvalidDateTime { .. })
        ));
        assert!(matches!(
            parse_date("not-a-date"),
            Err(TrackerError::InvalidDateTime { .. })
        ));

        // ItemStub with omitted status and severity uses defaults (open, medium)
        let scope_with_minimal_stub = r#"
[scope]
id = "backend_core"
target = "crates/backend_core"

[items]
bugs = [
    { id = "BUG-0009", title = "Minimal stub defaults" }
]
"#;
        let scope = ScopeManifest::from_toml_str(scope_with_minimal_stub)
            .expect("should parse ItemStub with default status and severity");
        assert_eq!(scope.items.bugs[0].status, ItemStatus::Open);
        assert_eq!(scope.items.bugs[0].severity, Severity::Medium);

        // Malformed TOML: non-string/non-datetime in created_at
        let bad_date_type = r#"
[project]
name = "BadDate"
active_phase = "01_init"
created_at = 12345
"#;
        assert!(matches!(
            ProjectManifest::from_toml_str(bad_date_type),
            Err(TrackerError::TomlDeserialize { .. })
        ));

        // Malformed TOML: negative integer in u32 counter
        let negative_counter = r#"
[project]
name = "BadCounter"
active_phase = "01_init"
created_at = "2026-11-01T21:00:00Z"

[counters]
bug = -1
"#;
        assert!(matches!(
            ProjectManifest::from_toml_str(negative_counter),
            Err(TrackerError::TomlDeserialize { .. })
        ));
    }

    #[test]
    fn test_aes256_gcm_round_trip_and_tamper_detection() {
        use crate::crypto::{decrypt_aes256_gcm, derive_key_sha256, encrypt_aes256_gcm};

        let key = derive_key_sha256(b"test-master-secret", b"unit-test-context");
        let plaintext = b"ghp_super_secret_personal_access_token_1234567890";

        let ciphertext = encrypt_aes256_gcm(&key, plaintext).expect("encryption should succeed");
        assert_ne!(ciphertext.as_slice(), plaintext);

        let decrypted =
            decrypt_aes256_gcm(&key, &ciphertext).expect("decryption should round-trip");
        assert_eq!(decrypted.as_slice(), plaintext);

        // Wrong key must fail authentication
        let wrong_key = derive_key_sha256(b"wrong-master-secret", b"unit-test-context");
        assert!(decrypt_aes256_gcm(&wrong_key, &ciphertext).is_err());

        // Tampered ciphertext byte must fail GCM authentication tag check
        let mut tampered = ciphertext.clone();
        let last_idx = tampered.len() - 1;
        tampered[last_idx] ^= 0x55;
        assert!(decrypt_aes256_gcm(&key, &tampered).is_err());

        // Truncated payload must be rejected
        assert!(decrypt_aes256_gcm(&key, &ciphertext[..8]).is_err());
    }

    #[test]
    fn test_secure_storage_vault_fallback_and_permissions() {
        use crate::crypto::SecureStorage;

        let temp_dir = tempfile::tempdir().expect("should create tempdir");
        let config_dir = temp_dir.path().join("nebulous_cfg");

        let storage =
            SecureStorage::vault_only(&config_dir).expect("should create vault_only SecureStorage");
        assert!(!storage.uses_keyring());

        // Store multiple secrets and verify persistence across distinct instances
        storage
            .store_secret("project:nebulous:pat", "ghp_vault_token_abc987")
            .expect("should store PAT in encrypted vault");
        storage
            .store_secret("project:novos:pat", "glpat_novos_rolling_token")
            .expect("should store second PAT in encrypted vault");

        assert!(storage.vault_path().is_file());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let dir_mode = std::fs::metadata(&config_dir).unwrap().permissions().mode() & 0o777;
            let file_mode = std::fs::metadata(storage.vault_path())
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(
                dir_mode, 0o700,
                "vault directory must have 0700 permissions"
            );
            assert_eq!(
                file_mode, 0o600,
                "vault.enc file must have 0600 permissions"
            );
        }

        // Reopen vault from disk using a fresh SecureStorage instance
        let reopened =
            SecureStorage::vault_only(&config_dir).expect("should reopen encrypted vault");
        assert_eq!(
            reopened
                .get_secret_string("project:nebulous:pat")
                .expect("should decrypt stored PAT"),
            "ghp_vault_token_abc987"
        );
        assert_eq!(
            reopened
                .get_secret_string("project:novos:pat")
                .expect("should decrypt second PAT"),
            "glpat_novos_rolling_token"
        );
        assert_eq!(
            reopened.list_vault_keys().unwrap(),
            vec![
                "project:nebulous:pat".to_string(),
                "project:novos:pat".to_string()
            ]
        );

        // Delete secret and verify removal
        reopened
            .delete_secret("project:nebulous:pat")
            .expect("should delete secret");
        assert!(!reopened.has_secret("project:nebulous:pat"));
        assert!(reopened.get_secret("project:nebulous:pat").is_err());
        assert!(reopened.has_secret("project:novos:pat"));

        // Empty key or empty secret must fail validation
        assert!(reopened.store_secret("   ", "secret").is_err());
        assert!(reopened.store_secret("valid_key", "").is_err());

        // Regression test 1: Opening an existing vault with a wrong master key and calling
        // store_secret MUST fail with an authentication error and MUST NOT clobber existing entries.
        let wrong_key_storage = SecureStorage::with_options(
            "nebulous-test",
            &config_dir,
            false,
            Some([0xAA; crate::crypto::AES256_KEY_LEN]),
        )
        .expect("should construct instance with explicit wrong key");
        assert!(
            wrong_key_storage
                .store_secret("project:attacker:pat", "malicious_overwrite")
                .is_err(),
            "store_secret with wrong master key must fail rather than overwrite vault.enc"
        );
        assert_eq!(
            reopened
                .get_secret_string("project:novos:pat")
                .expect("existing secret must remain intact after failed write"),
            "glpat_novos_rolling_token"
        );

        // Regression test 2: Truncated/short vault.key (< 32 bytes) must return an error
        // rather than silently generating a new random key over vault.key.
        let bad_key_dir = temp_dir.path().join("bad_vault_key_cfg");
        std::fs::create_dir_all(&bad_key_dir).unwrap();
        let short_seed_b64 = base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            b"too-short-16byte",
        );
        std::fs::write(bad_key_dir.join("vault.key"), &short_seed_b64).unwrap();
        assert!(
            SecureStorage::vault_only(&bad_key_dir).is_err(),
            "vault.key shorter than 32 bytes must be rejected"
        );
        assert_eq!(
            std::fs::read_to_string(bad_key_dir.join("vault.key")).unwrap(),
            short_seed_b64,
            "corrupted vault.key must not be overwritten"
        );

        // Regression test 3: Concurrent multi-threaded vault initialization and writes
        let concurrent_dir = std::sync::Arc::new(temp_dir.path().join("concurrent_vault"));
        let mut handles = Vec::new();
        for idx in 0..8 {
            let dir_clone = std::sync::Arc::clone(&concurrent_dir);
            handles.push(std::thread::spawn(move || {
                let store = SecureStorage::vault_only(&*dir_clone)
                    .expect("concurrent vault_only init should succeed");
                store
                    .store_secret(&format!("thread_key_{idx}"), &format!("secret_val_{idx}"))
                    .expect("concurrent store_secret should succeed");
            }));
        }
        for h in handles {
            h.join().expect("worker thread should not panic");
        }
        let final_store = SecureStorage::vault_only(&*concurrent_dir).unwrap();
        for idx in 0..8 {
            assert_eq!(
                final_store
                    .get_secret_string(&format!("thread_key_{idx}"))
                    .unwrap(),
                format!("secret_val_{idx}")
            );
        }
    }

    #[test]
    fn test_keypair_manager_ed25519_generation_import_export_and_pat() {
        use crate::crypto::KeyPairManager;

        let temp_dir = tempfile::tempdir().expect("should create tempdir");
        let manager = KeyPairManager::vault_only(temp_dir.path().join("vault"))
            .expect("should init KeyPairManager");

        // 1. Generate Ed25519 SSH keypair
        let generated = manager
            .generate_ed25519_keypair("nebulous/origin", Some("bot@nebulous.dev"))
            .expect("should generate Ed25519 keypair");
        assert_eq!(generated.account, "nebulous/origin");
        assert_eq!(generated.algorithm, "ssh-ed25519");
        assert!(generated.public_key_openssh.starts_with("ssh-ed25519 "));
        assert!(generated
            .private_key_openssh
            .contains("BEGIN OPENSSH PRIVATE KEY"));
        assert!(generated.fingerprint_sha256.starts_with("SHA256:"));
        assert_eq!(generated.comment.as_deref(), Some("bot@nebulous.dev"));

        // 2. Retrieve keypair and public info from vault
        assert!(manager.has_ssh_keypair("nebulous/origin"));
        let retrieved = manager
            .get_ssh_keypair("nebulous/origin")
            .expect("should retrieve stored Ed25519 keypair");
        assert_eq!(retrieved.public_key_openssh, generated.public_key_openssh);
        assert_eq!(retrieved.fingerprint_sha256, generated.fingerprint_sha256);

        let pub_info = manager
            .get_ssh_public_key_info("nebulous/origin")
            .expect("should retrieve public key info");
        assert_eq!(pub_info.fingerprint_sha256, generated.fingerprint_sha256);
        assert_eq!(pub_info.public_key_openssh, generated.public_key_openssh);

        // 3. Write the OpenSSH private key to a user-owned file and import it under another
        //    account (keys are never exported by Nebulous itself; this simulates `~/.ssh`).
        let priv_path = temp_dir.path().join("id_ed25519_nebulous");
        std::fs::write(&priv_path, generated.private_key_openssh.as_bytes())
            .expect("should write test private key file");

        let imported = manager
            .import_ssh_key_file("novos/upstream", &priv_path, None)
            .expect("should import exported OpenSSH private key file");
        assert_eq!(imported.fingerprint_sha256, generated.fingerprint_sha256);
        assert_eq!(imported.public_key_openssh, generated.public_key_openssh);

        // 4. Passphrase-encrypted OpenSSH private key import with non-ASCII comment
        let encrypted_pem = "-----BEGIN OPENSSH PRIVATE KEY-----\n\
b3BlbnNzaC1rZXktdjEAAAAACmFlczI1Ni1jdHIAAAAGYmNyeXB0AAAAGAAAABCJWvj7D9\n\
9lXeI3N3dASzPYAAAAIAAAAAEAAAAzAAAAC3NzaC1lZDI1NTE5AAAAIJlS08rT0LhI49eT\n\
LGG4KE/IeixN0+O1IUGFB0W+mcO8AAAAoGAbFSJv1o4sJOl6TAHSzLySOmi+khl03KQF4K\n\
UrASJtGLQUSJ1uknaMXhhvzocnntKGdtnwQqupPxsAvlJVx+YQvKtvnNodzJQWr2AK3eQe\n\
GCGSdmIZNBxsPeFEeDqGk9hWjbpb+NmVQL4zGidvTgVqt4K+b5nqyTHpbhcd3o4uCpVVsl\n\
sIrutI2NuqcarHHe39XSQPcgOvzeKMOaWd1FA=\n\
-----END OPENSSH PRIVATE KEY-----\n";

        // Importing encrypted key without passphrase or with wrong passphrase must fail
        assert!(manager
            .import_ssh_keypair("encrypted/no_pass", encrypted_pem, None)
            .is_err());
        assert!(manager
            .import_ssh_keypair("encrypted/empty_pass", encrypted_pem, Some(""))
            .is_err());
        assert!(manager
            .import_ssh_keypair(
                "encrypted/wrong_pass",
                encrypted_pem,
                Some("wrong-passphrase")
            )
            .is_err());

        // Importing encrypted key with correct passphrase must succeed and preserve non-ASCII comment
        let decrypted_record = manager
            .import_ssh_keypair(
                "encrypted/ok",
                encrypted_pem,
                Some("correct-horse-battery-staple"),
            )
            .expect("should decrypt and import passphrase-protected OpenSSH key");
        assert_eq!(
            decrypted_record.public_key_openssh,
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIJlS08rT0LhI49eTLGG4KE/IeixN0+O1IUGFB0W+mcO8 key-ñoño-🚀@nebulous.dev"
        );
        assert_eq!(
            decrypted_record.comment.as_deref(),
            Some("key-ñoño-🚀@nebulous.dev")
        );

        // 5. Delete SSH keypair
        manager
            .delete_ssh_keypair("nebulous/origin")
            .expect("should delete SSH keypair");
        assert!(!manager.has_ssh_keypair("nebulous/origin"));
        assert!(manager.get_ssh_keypair("nebulous/origin").is_err());

        // 6. HTTPS PAT round-trip via KeyPairManager & reserved prefix protection
        assert!(manager
            .store_pat("ssh_priv:nebulous/origin", "ghp_bad")
            .is_err());
        assert!(manager
            .store_pat("ssh_pub:nebulous/origin", "ghp_bad")
            .is_err());
        manager
            .store_pat("NebulousRepo", "  ghp_trimmed_pat_token_999  ")
            .expect("should store PAT");
        assert!(manager.has_pat("NebulousRepo"));
        assert_eq!(
            manager.get_pat_string("NebulousRepo").unwrap(),
            "ghp_trimmed_pat_token_999"
        );
        manager
            .delete_pat("NebulousRepo")
            .expect("should delete PAT");
        assert!(!manager.has_pat("NebulousRepo"));
    }

    #[test]
    fn test_cross_platform_config_dir_resolution_and_path_normalization() {
        use std::path::PathBuf;

        use crate::crypto::resolve_nebulous_config_dir_from;
        use crate::storage::{
            atomic_write_str, normalize_relative_path, normalize_relative_path_list,
            normalize_relative_path_str, TrackerStorage,
        };

        // 1. NEBULOUS_CONFIG_DIR override takes highest precedence on all OS targets
        assert_eq!(
            resolve_nebulous_config_dir_from(
                Some("/tmp/custom_nebulous_cfg"),
                Some("/home/user/.xdg"),
                Some("C:\\Users\\user\\AppData\\Roaming"),
                Some("/home/user"),
                "linux"
            ),
            PathBuf::from("/tmp/custom_nebulous_cfg")
        );

        // 2. XDG_CONFIG_HOME/nebulous
        assert_eq!(
            resolve_nebulous_config_dir_from(
                None,
                Some("/home/user/.xdg_config"),
                None,
                Some("/home/user"),
                "linux"
            ),
            PathBuf::from("/home/user/.xdg_config").join("nebulous")
        );

        // 3. Linux default (~/.config/nebulous)
        assert_eq!(
            resolve_nebulous_config_dir_from(None, None, None, Some("/home/alice"), "linux"),
            PathBuf::from("/home/alice")
                .join(".config")
                .join("nebulous")
        );

        // 4. macOS default (~/Library/Application Support/nebulous)
        assert_eq!(
            resolve_nebulous_config_dir_from(None, None, None, Some("/Users/alice"), "macos"),
            PathBuf::from("/Users/alice")
                .join("Library")
                .join("Application Support")
                .join("nebulous")
        );

        // 5. Windows default (%APPDATA%\nebulous)
        assert_eq!(
            resolve_nebulous_config_dir_from(
                None,
                None,
                Some("C:\\Users\\alice\\AppData\\Roaming"),
                Some("C:\\Users\\alice"),
                "windows"
            ),
            PathBuf::from("C:\\Users\\alice\\AppData\\Roaming").join("nebulous")
        );

        // 6. Relative path normalization across platforms
        assert_eq!(
            normalize_relative_path_str("phases\\01_core_engine\\"),
            "phases/01_core_engine"
        );
        assert_eq!(
            normalize_relative_path_str(".\\domains\\backend_core\\"),
            "domains/backend_core"
        );
        assert_eq!(normalize_relative_path_str(".\\"), ".");
        assert_eq!(normalize_relative_path_str("///"), "");
        assert_eq!(
            normalize_relative_path(std::path::Path::new("crates\\backend_core\\src\\lib.rs")),
            "crates/backend_core/src/lib.rs"
        );
        assert_eq!(
            normalize_relative_path_list([
                "crates\\backend_core\\Cargo.toml",
                "./crates/backend_core/Cargo.toml",
                "./crates/nebulous_cli\\src\\main.rs",
                "   ",
            ]),
            vec![
                "crates/backend_core/Cargo.toml".to_string(),
                "crates/nebulous_cli/src/main.rs".to_string(),
            ]
        );

        // 7. Verify TrackerStorage normalizes backslash paths on write & read
        let temp_dir = tempfile::tempdir().unwrap();
        let storage =
            TrackerStorage::init(temp_dir.path(), "NormTest", "01_core_engine", "Core Engine")
                .unwrap();

        let mut project = storage.read_project().unwrap();
        project.phases.registry[0].path = "phases\\01_core_engine".to_string();
        storage.write_project(&project).unwrap();
        let reread_project = storage.read_project().unwrap();
        assert_eq!(
            reread_project.phases.registry[0].path,
            "phases/01_core_engine"
        );

        let scope = storage
            .create_scope(
                Some("01_core_engine"),
                "backend_core",
                "crates\\backend_core\\",
                vec!["rust".to_string()],
                None,
            )
            .unwrap();
        assert_eq!(scope.scope.target, "crates/backend_core");

        // 8. Verify atomic_write_str replaces a read-only target and preserves its read-only
        //    permission (it must never widen the original mode).
        let readonly_target = temp_dir.path().join("readonly_test.toml");
        atomic_write_str(&readonly_target, "v = 1\n").unwrap();
        let mut perms = std::fs::metadata(&readonly_target).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&readonly_target, perms).unwrap();

        atomic_write_str(&readonly_target, "v = 2\n")
            .expect("atomic_write_str should succeed over read-only target file");
        assert_eq!(
            std::fs::read_to_string(&readonly_target).unwrap(),
            "v = 2\n"
        );
        let mut after = std::fs::metadata(&readonly_target).unwrap().permissions();
        assert!(
            after.readonly(),
            "atomic_write_str must preserve the target's read-only permission"
        );
        #[allow(clippy::permissions_set_readonly_false)]
        after.set_readonly(false);
        std::fs::set_permissions(&readonly_target, after).unwrap();
    }
}
