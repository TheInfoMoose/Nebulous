use std::fs;
use std::path::{Path, PathBuf};

use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use ssh_key::{
    private::{Ed25519Keypair, KeypairData},
    HashAlg, LineEnding, PrivateKey,
};
use zeroize::Zeroizing;

use crate::crypto::{ensure_secure_dir, set_secure_file_permissions, SecureStorage};
use crate::error::{Result, TrackerError};
use crate::storage::{atomic_write_bytes, TrackerStorage};

const SSH_PRIVATE_KEY_PREFIX: &str = "ssh_priv:";
const SSH_PUBLIC_META_PREFIX: &str = "ssh_pub:";

/// Full OpenSSH keypair record including the zeroized private key and public key metadata.
#[derive(Clone)]
pub struct SshKeyPairRecord {
    pub account: String,
    pub algorithm: String,
    pub public_key_openssh: String,
    pub private_key_openssh: Zeroizing<String>,
    pub fingerprint_sha256: String,
    pub comment: Option<String>,
}

impl std::fmt::Debug for SshKeyPairRecord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshKeyPairRecord")
            .field("account", &self.account)
            .field("algorithm", &self.algorithm)
            .field("public_key_openssh", &self.public_key_openssh)
            .field("private_key_openssh", &"<REDACTED>")
            .field("fingerprint_sha256", &self.fingerprint_sha256)
            .field("comment", &self.comment)
            .finish()
    }
}

impl SshKeyPairRecord {
    /// Extracts the public metadata portion of this keypair record without the private key.
    pub fn public_info(&self) -> SshPublicKeyInfo {
        SshPublicKeyInfo {
            account: self.account.clone(),
            algorithm: self.algorithm.clone(),
            public_key_openssh: self.public_key_openssh.clone(),
            fingerprint_sha256: self.fingerprint_sha256.clone(),
            comment: self.comment.clone(),
        }
    }
}

/// Safe, serializable metadata describing a stored SSH public key without exposing private key material.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SshPublicKeyInfo {
    pub account: String,
    pub algorithm: String,
    pub public_key_openssh: String,
    pub fingerprint_sha256: String,
    pub comment: Option<String>,
}

/// High-level manager for Ed25519/OpenSSH keypairs and HTTPS Personal Access Tokens (PATs)
/// backed by [`SecureStorage`] (`ST_Rust_Age` `simply-transfer-crypto` architecture).
#[derive(Debug, Clone)]
pub struct KeyPairManager {
    storage: SecureStorage,
}

impl KeyPairManager {
    /// Creates a `KeyPairManager` backed by [`SecureStorage::new()`].
    pub fn new() -> Result<Self> {
        Ok(Self {
            storage: SecureStorage::new()?,
        })
    }

    /// Creates a `KeyPairManager` backed by a custom [`SecureStorage`] instance.
    pub fn with_storage(storage: SecureStorage) -> Self {
        Self { storage }
    }

    /// Creates a vault-only `KeyPairManager` rooted at `config_dir` (bypassing OS keyring).
    pub fn vault_only(config_dir: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            storage: SecureStorage::vault_only(config_dir)?,
        })
    }

    /// Returns a reference to the underlying [`SecureStorage`].
    pub fn storage(&self) -> &SecureStorage {
        &self.storage
    }

    /// Generates a new Ed25519 OpenSSH keypair for `account` (e.g. project name or `project/remote`),
    /// stores it in [`SecureStorage`], and returns the [`SshKeyPairRecord`].
    pub fn generate_ed25519_keypair(
        &self,
        account: &str,
        comment: Option<&str>,
    ) -> Result<SshKeyPairRecord> {
        let normalized_account = normalize_account_id(account)?;
        let effective_comment = comment
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .unwrap_or(&normalized_account);

        let mut seed = Zeroizing::new([0u8; 32]);
        OsRng.fill_bytes(&mut *seed);

        let ed25519_kp = Ed25519Keypair::from_seed(&seed);

        let private_key = PrivateKey::new(KeypairData::Ed25519(ed25519_kp), effective_comment)
            .map_err(|e| {
                TrackerError::KeyringError(format!("Failed to build Ed25519 SSH private key: {e}"))
            })?;

        self.persist_parsed_private_key(&normalized_account, &private_key)
    }

    /// Imports an existing OpenSSH private key ( optionally passphrase-protected) for `account`
    /// into [`SecureStorage`].
    pub fn import_ssh_keypair(
        &self,
        account: &str,
        private_key_openssh: &str,
        passphrase: Option<&str>,
    ) -> Result<SshKeyPairRecord> {
        let normalized_account = normalize_account_id(account)?;
        let trimmed = private_key_openssh.trim();
        if trimmed.is_empty() {
            return Err(TrackerError::ValidationError(
                "OpenSSH private key PEM must not be empty".to_string(),
            ));
        }

        let outer = PrivateKey::from_openssh(trimmed).map_err(|e| {
            TrackerError::KeyringError(format!("Invalid OpenSSH private key format: {e}"))
        })?;
        let outer_comment = outer.comment().to_string().trim().to_string();

        let mut parsed = if outer.is_encrypted() {
            let pass = passphrase.filter(|p| !p.is_empty()).ok_or_else(|| {
                TrackerError::ValidationError(
                    "OpenSSH private key is encrypted; a passphrase is required to import it"
                        .to_string(),
                )
            })?;
            russh::keys::decode_secret_key(trimmed, Some(pass)).map_err(|e| {
                TrackerError::KeyringError(format!(
                    "Failed to decode or decrypt OpenSSH private key: {e}"
                ))
            })?
        } else {
            outer
        };

        if parsed.is_encrypted() {
            return Err(TrackerError::ValidationError(
                "OpenSSH private key is encrypted; a passphrase is required to import it"
                    .to_string(),
            ));
        }

        if parsed.comment().to_string().trim().is_empty() && !outer_comment.is_empty() {
            parsed.set_comment(outer_comment.as_str());
        }

        self.persist_parsed_private_key(&normalized_account, &parsed)
    }

    /// Reads an OpenSSH private key file from disk and imports it into [`SecureStorage`] under `account`.
    pub fn import_ssh_key_file(
        &self,
        account: &str,
        private_key_path: impl AsRef<Path>,
        passphrase: Option<&str>,
    ) -> Result<SshKeyPairRecord> {
        let path = private_key_path.as_ref();
        let raw = Zeroizing::new(fs::read_to_string(path).map_err(|source| TrackerError::Io {
            path: path.to_path_buf(),
            source,
        })?);
        self.import_ssh_keypair(account, &raw, passphrase)
    }

    /// Retrieves the stored [`SshKeyPairRecord`] for `account` from [`SecureStorage`].
    pub fn get_ssh_keypair(&self, account: &str) -> Result<SshKeyPairRecord> {
        let normalized_account = normalize_account_id(account)?;
        let priv_key_slot = format!("{SSH_PRIVATE_KEY_PREFIX}{normalized_account}");
        let private_key_openssh = self.storage.get_secret(&priv_key_slot)?;

        let parsed = PrivateKey::from_openssh(private_key_openssh.as_str()).map_err(|e| {
            TrackerError::KeyringError(format!(
                "Stored SSH key for '{normalized_account}' is invalid: {e}"
            ))
        })?;

        build_record_from_private_key(normalized_account, &parsed, private_key_openssh)
    }

    /// Retrieves only the public key metadata ([`SshPublicKeyInfo`]) for `account`.
    pub fn get_ssh_public_key_info(&self, account: &str) -> Result<SshPublicKeyInfo> {
        let normalized_account = normalize_account_id(account)?;
        let pub_meta_slot = format!("{SSH_PUBLIC_META_PREFIX}{normalized_account}");
        if let Ok(serialized_meta) = self.storage.get_secret(&pub_meta_slot) {
            if let Ok(info) = serde_json::from_str::<SshPublicKeyInfo>(&serialized_meta) {
                return Ok(info);
            }
        }
        Ok(self.get_ssh_keypair(&normalized_account)?.public_info())
    }

    /// Deletes the stored SSH keypair (both private key and public metadata) for `account`.
    pub fn delete_ssh_keypair(&self, account: &str) -> Result<()> {
        let normalized_account = normalize_account_id(account)?;
        let priv_key_slot = format!("{SSH_PRIVATE_KEY_PREFIX}{normalized_account}");
        let pub_meta_slot = format!("{SSH_PUBLIC_META_PREFIX}{normalized_account}");

        let _ = self.storage.delete_secret(&pub_meta_slot);
        self.storage.delete_secret(&priv_key_slot)
    }

    /// Returns `true` if an SSH keypair is stored for `account`.
    pub fn has_ssh_keypair(&self, account: &str) -> bool {
        if let Ok(normalized_account) = normalize_account_id(account) {
            let priv_key_slot = format!("{SSH_PRIVATE_KEY_PREFIX}{normalized_account}");
            self.storage.has_secret(&priv_key_slot)
        } else {
            false
        }
    }

    /// Materializes a stored SSH keypair to `<target_dir>/<file_stem>` (`0600` on Unix) and
    /// `<target_dir>/<file_stem>.pub` for tools (such as `libgit2` or external `ssh`) that require
    /// filesystem paths.
    pub fn export_keypair_to_dir(
        &self,
        account: &str,
        target_dir: impl AsRef<Path>,
        file_stem: &str,
    ) -> Result<(PathBuf, PathBuf)> {
        let record = self.get_ssh_keypair(account)?;
        let dir = target_dir.as_ref();
        ensure_secure_dir(dir)?;

        let stem = file_stem.trim();
        if stem.is_empty() || stem.contains('/') || stem.contains('\\') || stem.contains("..") {
            return Err(TrackerError::ValidationError(format!(
                "Invalid SSH key file stem '{file_stem}'"
            )));
        }

        let priv_path = dir.join(stem);
        let pub_path = dir.join(format!("{stem}.pub"));

        atomic_write_bytes(&priv_path, record.private_key_openssh.as_bytes())?;
        set_secure_file_permissions(&priv_path)?;

        let mut pub_line = record.public_key_openssh.clone();
        if !pub_line.ends_with('\n') {
            pub_line.push('\n');
        }
        atomic_write_bytes(&pub_path, pub_line.as_bytes())?;

        Ok((priv_path, pub_path))
    }

    /// Stores an HTTPS Personal Access Token (PAT) for `project_or_remote` in [`SecureStorage`].
    pub fn store_pat(&self, project_or_remote: &str, token: &str) -> Result<()> {
        let account = normalize_account_id(project_or_remote)?;
        let trimmed_token = token.trim();
        if trimmed_token.is_empty() {
            return Err(TrackerError::ValidationError(
                "Personal Access Token (PAT) must not be empty".to_string(),
            ));
        }
        self.storage.store_secret(&account, trimmed_token)
    }

    /// Retrieves an HTTPS Personal Access Token (PAT) for `project_or_remote` wrapped in [`Zeroizing`].
    pub fn get_pat(&self, project_or_remote: &str) -> Result<Zeroizing<String>> {
        let account = normalize_account_id(project_or_remote)?;
        self.storage.get_secret(&account)
    }

    /// Retrieves an HTTPS Personal Access Token (PAT) for `project_or_remote` as a `String`.
    pub fn get_pat_string(&self, project_or_remote: &str) -> Result<String> {
        let pat = self.get_pat(project_or_remote)?;
        Ok((*pat).clone())
    }

    /// Deletes a stored HTTPS Personal Access Token (PAT) for `project_or_remote`.
    pub fn delete_pat(&self, project_or_remote: &str) -> Result<()> {
        let account = normalize_account_id(project_or_remote)?;
        self.storage.delete_secret(&account)
    }

    /// Returns `true` if a PAT is stored for `project_or_remote`.
    pub fn has_pat(&self, project_or_remote: &str) -> bool {
        if let Ok(account) = normalize_account_id(project_or_remote) {
            self.storage.has_secret(&account)
        } else {
            false
        }
    }

    fn persist_parsed_private_key(
        &self,
        normalized_account: &str,
        private_key: &PrivateKey,
    ) -> Result<SshKeyPairRecord> {
        let openssh_priv = private_key.to_openssh(LineEnding::LF).map_err(|e| {
            TrackerError::KeyringError(format!("Failed to serialize OpenSSH private key: {e}"))
        })?;

        // Validate that russh can also decode the OpenSSH key cleanly
        let _ = russh::keys::decode_secret_key(openssh_priv.as_str(), None).map_err(|e| {
            TrackerError::KeyringError(format!("russh validation of OpenSSH key failed: {e}"))
        })?;

        let record = build_record_from_private_key(
            normalized_account.to_string(),
            private_key,
            openssh_priv,
        )?;

        let priv_key_slot = format!("{SSH_PRIVATE_KEY_PREFIX}{normalized_account}");
        let pub_meta_slot = format!("{SSH_PUBLIC_META_PREFIX}{normalized_account}");

        self.storage
            .store_secret(&priv_key_slot, record.private_key_openssh.as_str())?;

        if let Ok(meta_json) = serde_json::to_string(&record.public_info()) {
            let _ = self.storage.store_secret(&pub_meta_slot, &meta_json);
        }

        Ok(record)
    }
}

fn build_record_from_private_key(
    account: String,
    private_key: &PrivateKey,
    private_key_openssh: Zeroizing<String>,
) -> Result<SshKeyPairRecord> {
    let pub_key = private_key.public_key();
    let public_key_openssh = pub_key.to_openssh().map_err(|e| {
        TrackerError::KeyringError(format!("Failed to encode OpenSSH public key: {e}"))
    })?;
    let fingerprint_sha256 = pub_key.fingerprint(HashAlg::Sha256).to_string();
    let algorithm = private_key.algorithm().as_str().to_string();
    let comment_raw = private_key.comment().to_string();
    let comment_str = comment_raw.trim();
    let comment = if comment_str.is_empty() {
        None
    } else {
        Some(comment_str.to_string())
    };

    Ok(SshKeyPairRecord {
        account,
        algorithm,
        public_key_openssh,
        private_key_openssh,
        fingerprint_sha256,
        comment,
    })
}

fn normalize_account_id(account: &str) -> Result<String> {
    let trimmed = account.trim();
    if trimmed.is_empty() {
        return Err(TrackerError::ValidationError(
            "Account or project identifier must not be empty".to_string(),
        ));
    }
    if trimmed.starts_with(SSH_PRIVATE_KEY_PREFIX) || trimmed.starts_with(SSH_PUBLIC_META_PREFIX) {
        return Err(TrackerError::ValidationError(format!(
            "Account identifier '{trimmed}' must not start with reserved prefix"
        )));
    }
    Ok(trimmed.to_string())
}

impl TrackerStorage {
    /// Returns a [`SecureStorage`] instance rooted at [`crate::crypto::nebulous_config_dir()`].
    pub fn secure_storage(&self) -> Result<SecureStorage> {
        SecureStorage::new()
    }

    /// Returns a [`KeyPairManager`] instance rooted at [`crate::crypto::nebulous_config_dir()`].
    pub fn keypair_manager(&self) -> Result<KeyPairManager> {
        KeyPairManager::new()
    }

    /// Generates an Ed25519 SSH keypair for this project and stores it in [`SecureStorage`].
    pub fn generate_ssh_keypair(&self, comment: Option<&str>) -> Result<SshKeyPairRecord> {
        let project = self.read_project()?;
        let manager = self.keypair_manager()?;
        manager.generate_ed25519_keypair(&project.project.name, comment)
    }

    /// Imports an OpenSSH private key for this project into [`SecureStorage`].
    pub fn import_ssh_keypair(
        &self,
        private_key_openssh: &str,
        passphrase: Option<&str>,
    ) -> Result<SshKeyPairRecord> {
        let project = self.read_project()?;
        let manager = self.keypair_manager()?;
        manager.import_ssh_keypair(&project.project.name, private_key_openssh, passphrase)
    }

    /// Retrieves the stored [`SshKeyPairRecord`] for this project from [`SecureStorage`].
    pub fn get_ssh_keypair(&self) -> Result<SshKeyPairRecord> {
        let project = self.read_project()?;
        let manager = self.keypair_manager()?;
        manager.get_ssh_keypair(&project.project.name)
    }

    /// Retrieves the stored [`SshPublicKeyInfo`] for this project without exposing private key material.
    pub fn get_ssh_public_key_info(&self) -> Result<SshPublicKeyInfo> {
        let project = self.read_project()?;
        let manager = self.keypair_manager()?;
        manager.get_ssh_public_key_info(&project.project.name)
    }

    /// Deletes the stored SSH keypair for this project from [`SecureStorage`].
    pub fn delete_ssh_keypair(&self) -> Result<()> {
        let project = self.read_project()?;
        let manager = self.keypair_manager()?;
        manager.delete_ssh_keypair(&project.project.name)
    }
}
