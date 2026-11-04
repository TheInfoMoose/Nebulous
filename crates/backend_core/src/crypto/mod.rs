pub mod keys;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

pub use keys::{KeyPairManager, SshKeyPairRecord, SshPublicKeyInfo};

use crate::error::{Result, TrackerError};
use crate::storage::atomic_write_bytes;

/// Default OS keyring service identifier for Nebulous secrets.
pub const DEFAULT_KEYRING_SERVICE: &str = "nebulous-tracker";

/// Default encrypted vault filename inside [`nebulous_config_dir`].
pub const VAULT_FILE_NAME: &str = "vault.enc";

/// Default master key seed filename inside [`nebulous_config_dir`].
pub const VAULT_KEY_FILE_NAME: &str = "vault.key";

/// AES-256 key size in bytes (256 bits).
pub const AES256_KEY_LEN: usize = 32;

/// AES-256-GCM standard nonce size in bytes (96 bits).
pub const GCM_NONCE_LEN: usize = 12;

/// AES-256-GCM authentication tag size in bytes (128 bits).
pub const GCM_TAG_LEN: usize = 16;

/// Magic header prefix for the Nebulous encrypted vault file (`NEBV` + version byte `1`).
const VAULT_HEADER_MAGIC: &[u8; 5] = b"NEBV\x01";

/// Domain-separation context used when deriving a 256-bit AES-GCM key via SHA-256.
const VAULT_KDF_CONTEXT: &[u8] = b"nebulous-simply-transfer-crypto-aes256gcm-v1";

static VAULT_FILE_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn vault_lock() -> &'static Mutex<()> {
    VAULT_FILE_LOCK.get_or_init(|| Mutex::new(()))
}

/// Resolves the cross-platform Nebulous configuration directory.
///
/// Precedence order:
/// 1. `NEBULOUS_CONFIG_DIR` environment variable (used for isolated testing and custom setups).
/// 2. `$XDG_CONFIG_HOME/nebulous` when `XDG_CONFIG_HOME` is set and non-empty.
/// 3. Platform-native config directory:
///    - **Windows**: `%APPDATA%\nebulous` (or `%USERPROFILE%\AppData\Roaming\nebulous`)
///    - **macOS**: `~/Library/Application Support/nebulous`
///    - **Linux / FreeBSD / Unix**: `~/.config/nebulous`
pub fn nebulous_config_dir() -> PathBuf {
    let nebulous_override = std::env::var("NEBULOUS_CONFIG_DIR").ok();
    let xdg_config_home = std::env::var("XDG_CONFIG_HOME").ok();
    let appdata = std::env::var("APPDATA").ok();
    let home = std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok());

    resolve_nebulous_config_dir_from(
        nebulous_override.as_deref(),
        xdg_config_home.as_deref(),
        appdata.as_deref(),
        home.as_deref(),
        std::env::consts::OS,
    )
}

/// Deterministic helper for resolving the Nebulous config directory across platforms (`linux`, `macos`, `windows`).
pub fn resolve_nebulous_config_dir_from(
    nebulous_override: Option<&str>,
    xdg_config_home: Option<&str>,
    appdata: Option<&str>,
    home: Option<&str>,
    target_os: &str,
) -> PathBuf {
    if let Some(custom) = nebulous_override.map(str::trim).filter(|s| !s.is_empty()) {
        return PathBuf::from(custom);
    }

    if let Some(xdg) = xdg_config_home.map(str::trim).filter(|s| !s.is_empty()) {
        return PathBuf::from(xdg).join("nebulous");
    }

    match target_os {
        "windows" => {
            if let Some(app) = appdata.map(str::trim).filter(|s| !s.is_empty()) {
                return PathBuf::from(app).join("nebulous");
            }
            if let Some(h) = home.map(str::trim).filter(|s| !s.is_empty()) {
                return PathBuf::from(h)
                    .join("AppData")
                    .join("Roaming")
                    .join("nebulous");
            }
        }
        "macos" => {
            if let Some(h) = home.map(str::trim).filter(|s| !s.is_empty()) {
                return PathBuf::from(h)
                    .join("Library")
                    .join("Application Support")
                    .join("nebulous");
            }
        }
        _ => {
            if let Some(h) = home.map(str::trim).filter(|s| !s.is_empty()) {
                return PathBuf::from(h).join(".config").join("nebulous");
            }
        }
    }

    std::env::temp_dir().join("nebulous")
}

/// Derives a 256-bit key from `secret` and `context` using SHA-256, wrapped in [`Zeroizing`].
pub fn derive_key_sha256(secret: &[u8], context: &[u8]) -> Zeroizing<[u8; AES256_KEY_LEN]> {
    let mut hasher = Sha256::new();
    hasher.update(context);
    hasher.update([0x00]);
    hasher.update(secret);
    let digest = hasher.finalize();

    let mut key = Zeroizing::new([0u8; AES256_KEY_LEN]);
    key.copy_from_slice(&digest);
    key
}

/// Encrypts `plaintext` with AES-256-GCM using a fresh 96-bit random nonce from [`OsRng`].
///
/// Output format: `VAULT_HEADER_MAGIC (5B) || nonce (12B) || ciphertext_with_16B_gcm_tag`.
pub fn encrypt_aes256_gcm(key: &[u8; AES256_KEY_LEN], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| TrackerError::KeyringError(format!("Invalid AES-256-GCM key: {e}")))?;

    let mut nonce_bytes = [0u8; GCM_NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| TrackerError::KeyringError(format!("AES-256-GCM encryption failed: {e}")))?;

    let mut out = Vec::with_capacity(VAULT_HEADER_MAGIC.len() + GCM_NONCE_LEN + ciphertext.len());
    out.extend_from_slice(VAULT_HEADER_MAGIC);
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// Decrypts an AES-256-GCM payload produced by [`encrypt_aes256_gcm`], returning a [`Zeroizing`] buffer.
pub fn decrypt_aes256_gcm(
    key: &[u8; AES256_KEY_LEN],
    payload: &[u8],
) -> Result<Zeroizing<Vec<u8>>> {
    let body = if payload.starts_with(VAULT_HEADER_MAGIC) {
        &payload[VAULT_HEADER_MAGIC.len()..]
    } else if payload.starts_with(b"NEBV") {
        return Err(TrackerError::KeyringError(
            "Unsupported encrypted vault format version".to_string(),
        ));
    } else {
        payload
    };

    if body.len() < GCM_NONCE_LEN + GCM_TAG_LEN {
        return Err(TrackerError::KeyringError(
            "Encrypted vault payload is truncated or invalid".to_string(),
        ));
    }

    let (nonce_bytes, ciphertext) = body.split_at(GCM_NONCE_LEN);
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| TrackerError::KeyringError(format!("Invalid AES-256-GCM key: {e}")))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|_| {
        TrackerError::KeyringError(
            "AES-256-GCM authentication failed (wrong key or tampered vault)".to_string(),
        )
    })?;

    Ok(Zeroizing::new(plaintext))
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct VaultStorePayload {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    entries: BTreeMap<String, String>,
}

impl Drop for VaultStorePayload {
    fn drop(&mut self) {
        for val in self.entries.values_mut() {
            val.zeroize();
        }
    }
}

/// Cross-platform credential store (`ST_Rust_Age` `simply-transfer-crypto` architecture) combining
/// native OS keyring storage (`linux-native`, `apple-native`, `windows-native`) with an
/// authenticated AES-256-GCM encrypted file vault fallback (`vault.enc`).
#[derive(Clone)]
pub struct SecureStorage {
    service_name: String,
    config_dir: PathBuf,
    vault_path: PathBuf,
    master_key: Zeroizing<[u8; AES256_KEY_LEN]>,
    use_keyring: bool,
}

impl std::fmt::Debug for SecureStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecureStorage")
            .field("service_name", &self.service_name)
            .field("config_dir", &self.config_dir)
            .field("vault_path", &self.vault_path)
            .field("use_keyring", &self.use_keyring)
            .field("master_key", &"<REDACTED>")
            .finish()
    }
}

impl SecureStorage {
    /// Creates a `SecureStorage` instance rooted at [`nebulous_config_dir()`].
    ///
    /// Uses the OS keyring as primary storage (unless `NEBULOUS_DISABLE_KEYRING=1` is set)
    /// and maintains an AES-256-GCM encrypted vault fallback at `<config_dir>/vault.enc`.
    pub fn new() -> Result<Self> {
        Self::with_config_dir(nebulous_config_dir())
    }

    /// Creates a `SecureStorage` instance using `config_dir` for `vault.enc` and `vault.key`.
    pub fn with_config_dir(config_dir: impl AsRef<Path>) -> Result<Self> {
        let disable_keyring = std::env::var("NEBULOUS_DISABLE_KEYRING")
            .map(|v| matches!(v.trim(), "1" | "true" | "TRUE" | "yes"))
            .unwrap_or(false);
        Self::with_options(
            DEFAULT_KEYRING_SERVICE,
            config_dir.as_ref(),
            !disable_keyring,
            None,
        )
    }

    /// Creates a vault-only `SecureStorage` instance (bypassing the OS keyring) rooted at `config_dir`.
    ///
    /// Useful for deterministic unit tests, headless CI runners, and containerized pipelines.
    pub fn vault_only(config_dir: impl AsRef<Path>) -> Result<Self> {
        Self::with_options(DEFAULT_KEYRING_SERVICE, config_dir.as_ref(), false, None)
    }

    /// Creates a `SecureStorage` instance with an explicit 32-byte master key and custom options.
    pub fn with_options(
        service_name: impl Into<String>,
        config_dir: impl AsRef<Path>,
        use_keyring: bool,
        explicit_master_key: Option<[u8; AES256_KEY_LEN]>,
    ) -> Result<Self> {
        let config_dir = config_dir.as_ref().to_path_buf();
        ensure_secure_dir(&config_dir)?;

        let vault_path = config_dir.join(VAULT_FILE_NAME);
        let key_path = config_dir.join(VAULT_KEY_FILE_NAME);
        let service_name = service_name.into();

        let master_key = match explicit_master_key {
            Some(mut raw) => {
                let derived = derive_key_sha256(&raw, VAULT_KDF_CONTEXT);
                raw.zeroize();
                derived
            }
            None => load_or_create_vault_master_key(&key_path)?,
        };

        Ok(Self {
            service_name,
            config_dir,
            vault_path,
            master_key,
            use_keyring,
        })
    }

    /// Returns the path to the encrypted vault file (`vault.enc`).
    pub fn vault_path(&self) -> &Path {
        &self.vault_path
    }

    /// Returns the configuration directory backing this storage instance.
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// Returns whether OS keyring primary queries are enabled on this instance.
    pub fn uses_keyring(&self) -> bool {
        self.use_keyring
    }

    /// Stores a secret under `account_key` in the OS keyring (when enabled) and persists it in
    /// the authenticated AES-256-GCM encrypted vault fallback (`vault.enc`).
    pub fn store_secret(&self, account_key: &str, secret: &str) -> Result<()> {
        let normalized_key = normalize_secret_key(account_key)?;
        if secret.is_empty() {
            return Err(TrackerError::ValidationError(
                "Secret value must not be empty".to_string(),
            ));
        }

        // Always persist into the encrypted vault so headless/fallback sessions stay synchronized.
        self.write_vault_entry(&normalized_key, secret)?;

        if self.use_keyring {
            if let Ok(entry) = keyring::Entry::new(&self.service_name, &normalized_key) {
                match entry.set_password(secret) {
                    Ok(()) => {}
                    Err(
                        keyring::Error::PlatformFailure(_)
                        | keyring::Error::NoStorageAccess(_)
                        | keyring::Error::NoEntry,
                    ) => {
                        // Gracefully fall back to the AES-256-GCM encrypted vault.
                    }
                    Err(_) => {
                        // Vault write already succeeded; do not fail if keyring backend rejects.
                    }
                }
            }
        }

        Ok(())
    }

    /// Retrieves a secret for `account_key` wrapped in [`Zeroizing`], checking the OS keyring
    /// first (when enabled) and falling back to the AES-256-GCM encrypted vault (`vault.enc`).
    pub fn get_secret(&self, account_key: &str) -> Result<Zeroizing<String>> {
        let normalized_key = normalize_secret_key(account_key)?;

        if self.use_keyring {
            if let Ok(entry) = keyring::Entry::new(&self.service_name, &normalized_key) {
                if let Ok(secret) = entry.get_password() {
                    return Ok(Zeroizing::new(secret));
                }
            }
        }

        if let Some(secret) = self.read_vault_entry(&normalized_key)? {
            return Ok(secret);
        }

        Err(TrackerError::KeyringError(format!(
            "Credential '{normalized_key}' not found in OS keyring or encrypted vault"
        )))
    }

    /// Convenience wrapper around [`SecureStorage::get_secret`] returning a plain `String`.
    pub fn get_secret_string(&self, account_key: &str) -> Result<String> {
        let secret = self.get_secret(account_key)?;
        Ok((*secret).clone())
    }

    /// Deletes a secret for `account_key` from both the OS keyring and the encrypted vault.
    pub fn delete_secret(&self, account_key: &str) -> Result<()> {
        let normalized_key = normalize_secret_key(account_key)?;
        let removed_from_vault = self.remove_vault_entry(&normalized_key)?;

        let mut removed_from_keyring = false;
        if self.use_keyring {
            if let Ok(entry) = keyring::Entry::new(&self.service_name, &normalized_key) {
                if entry.delete_credential().is_ok() {
                    removed_from_keyring = true;
                }
            }
        }

        if removed_from_vault || removed_from_keyring {
            Ok(())
        } else {
            Err(TrackerError::KeyringError(format!(
                "Credential '{normalized_key}' not found"
            )))
        }
    }

    /// Returns `true` if `account_key` exists in either the OS keyring or the encrypted vault.
    pub fn has_secret(&self, account_key: &str) -> bool {
        self.get_secret(account_key).is_ok()
    }

    /// Lists all account keys currently stored in the encrypted vault.
    pub fn list_vault_keys(&self) -> Result<Vec<String>> {
        let _guard = vault_lock()
            .lock()
            .map_err(|e| TrackerError::KeyringError(e.to_string()))?;
        let payload = self.load_vault_unlocked()?;
        Ok(payload.entries.keys().cloned().collect())
    }

    fn write_vault_entry(&self, key: &str, secret: &str) -> Result<()> {
        let _guard = vault_lock()
            .lock()
            .map_err(|e| TrackerError::KeyringError(e.to_string()))?;
        let mut payload = self.load_vault_unlocked()?;
        payload.version = 1;
        payload.entries.insert(key.to_string(), secret.to_string());
        self.save_vault_unlocked(&payload)
    }

    fn read_vault_entry(&self, key: &str) -> Result<Option<Zeroizing<String>>> {
        let _guard = vault_lock()
            .lock()
            .map_err(|e| TrackerError::KeyringError(e.to_string()))?;
        let payload = self.load_vault_unlocked()?;
        Ok(payload.entries.get(key).cloned().map(Zeroizing::new))
    }

    fn remove_vault_entry(&self, key: &str) -> Result<bool> {
        let _guard = vault_lock()
            .lock()
            .map_err(|e| TrackerError::KeyringError(e.to_string()))?;
        if !self.vault_path.is_file() {
            return Ok(false);
        }
        let mut payload = self.load_vault_unlocked()?;
        let existed = payload
            .entries
            .remove(key)
            .map(|mut v| v.zeroize())
            .is_some();
        if existed {
            self.save_vault_unlocked(&payload)?;
        }
        Ok(existed)
    }

    fn load_vault_unlocked(&self) -> Result<VaultStorePayload> {
        if !self.vault_path.is_file() {
            return Ok(VaultStorePayload {
                version: 1,
                entries: BTreeMap::new(),
            });
        }

        let encrypted = fs::read(&self.vault_path).map_err(|source| TrackerError::Io {
            path: self.vault_path.clone(),
            source,
        })?;
        if encrypted.is_empty() {
            return Ok(VaultStorePayload {
                version: 1,
                entries: BTreeMap::new(),
            });
        }

        let decrypted = decrypt_aes256_gcm(&self.master_key, &encrypted)?;
        serde_json::from_slice(&decrypted).map_err(|e| {
            TrackerError::KeyringError(format!("Failed to deserialize decrypted vault JSON: {e}"))
        })
    }

    fn save_vault_unlocked(&self, payload: &VaultStorePayload) -> Result<()> {
        ensure_secure_dir(&self.config_dir)?;
        let serialized = Zeroizing::new(serde_json::to_vec(payload).map_err(|e| {
            TrackerError::KeyringError(format!("Failed to serialize vault JSON: {e}"))
        })?);
        let encrypted = encrypt_aes256_gcm(&self.master_key, &serialized)?;
        atomic_write_bytes(&self.vault_path, &encrypted)?;
        set_secure_file_permissions(&self.vault_path)?;
        Ok(())
    }
}

fn normalize_secret_key(key: &str) -> Result<String> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err(TrackerError::ValidationError(
            "Credential account key must not be empty".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

fn load_or_create_vault_master_key(key_path: &Path) -> Result<Zeroizing<[u8; AES256_KEY_LEN]>> {
    if let Ok(env_key) = std::env::var("NEBULOUS_VAULT_MASTER_KEY") {
        let trimmed = env_key.trim();
        if !trimmed.is_empty() {
            let derived = derive_key_sha256(trimmed.as_bytes(), VAULT_KDF_CONTEXT);
            return Ok(derived);
        }
    }

    let _guard = vault_lock()
        .lock()
        .map_err(|e| TrackerError::KeyringError(e.to_string()))?;

    if key_path.is_file() {
        let raw =
            Zeroizing::new(
                fs::read_to_string(key_path).map_err(|source| TrackerError::Io {
                    path: key_path.to_path_buf(),
                    source,
                })?,
            );
        let decoded = Zeroizing::new(
            BASE64_STANDARD
                .decode(raw.trim())
                .map_err(|e| TrackerError::KeyringError(format!("Corrupted vault.key: {e}")))?,
        );
        if decoded.len() < AES256_KEY_LEN {
            return Err(TrackerError::KeyringError(format!(
                "Corrupted vault.key: decoded seed is {} bytes, expected at least {AES256_KEY_LEN}",
                decoded.len()
            )));
        }
        return Ok(derive_key_sha256(&decoded, VAULT_KDF_CONTEXT));
    }

    let mut random_seed = Zeroizing::new([0u8; AES256_KEY_LEN]);
    OsRng.fill_bytes(&mut *random_seed);
    let encoded = Zeroizing::new(BASE64_STANDARD.encode(*random_seed));

    if let Some(parent) = key_path.parent() {
        ensure_secure_dir(parent)?;
    }
    atomic_write_bytes(key_path, encoded.as_bytes())?;
    set_secure_file_permissions(key_path)?;

    Ok(derive_key_sha256(&*random_seed, VAULT_KDF_CONTEXT))
}

pub(crate) fn ensure_secure_dir(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).map_err(|source| TrackerError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o700);
        let _ = fs::set_permissions(dir, perms);
    }
    Ok(())
}

pub(crate) fn set_secure_file_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o600);
        fs::set_permissions(path, perms).map_err(|source| TrackerError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}
