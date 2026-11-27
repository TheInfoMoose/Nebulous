pub mod keys;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

pub use keys::{KeyPairManager, SshKeyPairRecord, SshPublicKeyInfo};
pub use zeroize::Zeroizing;

use crate::error::{Result, TrackerError};
use crate::storage::atomic_write_bytes_with_mode;

/// Default OS keyring service identifier for Nebulous secrets.
pub const DEFAULT_KEYRING_SERVICE: &str = "nebulous-tracker";

/// Default encrypted vault filename inside [`nebulous_config_dir`].
pub const VAULT_FILE_NAME: &str = "vault.enc";

/// Default master key seed filename inside [`nebulous_config_dir`].
pub const VAULT_KEY_FILE_NAME: &str = "vault.key";

/// Advisory lock file name inside [`nebulous_config_dir`].
const VAULT_LOCK_FILE_NAME: &str = "vault.lock";

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

/// Acquires an exclusive advisory file lock on `<config_dir>/vault.lock` while running `f`.
///
/// The vault is a read-modify-write of one encrypted file; an in-process Mutex doesn't stop a
/// second `neb` process from losing a vault entry or racing `vault.key` creation.
fn with_vault_lock<T>(config_dir: &Path, f: impl FnOnce() -> Result<T>) -> Result<T> {
    ensure_secure_dir(config_dir)?;
    let lock_path = config_dir.join(VAULT_LOCK_FILE_NAME);
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .map_err(|source| TrackerError::Io {
            path: lock_path.clone(),
            source,
        })?;

    let mut rw_lock = fd_lock::RwLock::new(file);
    let start = Instant::now();
    let _fd_guard = loop {
        match rw_lock.try_write() {
            Ok(guard) => break guard,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                let elapsed = start.elapsed();
                if elapsed >= crate::storage::DEFAULT_LOCK_TIMEOUT {
                    return Err(TrackerError::LockTimeout {
                        path: lock_path,
                        waited_ms: elapsed.as_millis() as u64,
                    });
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(source) => {
                return Err(TrackerError::Io {
                    path: lock_path,
                    source,
                });
            }
        }
    };

    f()
}

/// Name of the legacy plaintext SSH key export directory (`<config_dir>/ssh_cache`).
///
/// Older releases wrote decrypted private keys here for `libgit2`. Keys are now handed to
/// `libgit2` from memory only; any leftover directory is removed by [`purge_legacy_ssh_cache`].
pub const LEGACY_SSH_CACHE_DIR_NAME: &str = "ssh_cache";

/// Removes the legacy plaintext `<config_dir>/ssh_cache` key export directory if present.
///
/// Returns `true` if a directory (or stray file) was removed. Symlinks are unlinked, never followed.
pub fn purge_legacy_ssh_cache(config_dir: impl AsRef<Path>) -> Result<bool> {
    let cache = config_dir.as_ref().join(LEGACY_SSH_CACHE_DIR_NAME);
    let Ok(meta) = fs::symlink_metadata(&cache) else {
        return Ok(false);
    };
    let result = if meta.is_dir() {
        fs::remove_dir_all(&cache)
    } else {
        fs::remove_file(&cache)
    };
    result.map_err(|source| TrackerError::Io {
        path: cache.clone(),
        source,
    })?;
    Ok(true)
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

/// Resolves the cross-platform Nebulous configuration directory, returning an error if no
/// config or home environment variable is set.
///
/// Follows the same precedence as [`nebulous_config_dir`] without falling back to a shared
/// temporary directory.
pub fn try_nebulous_config_dir() -> Result<PathBuf> {
    let nebulous_override = std::env::var("NEBULOUS_CONFIG_DIR").ok();
    let xdg_config_home = std::env::var("XDG_CONFIG_HOME").ok();
    let appdata = std::env::var("APPDATA").ok();
    let home = std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok());

    resolve_nebulous_config_dir_checked(
        nebulous_override.as_deref(),
        xdg_config_home.as_deref(),
        appdata.as_deref(),
        home.as_deref(),
        std::env::consts::OS,
    )
}

/// Deterministic checked helper for resolving the Nebulous config directory across platforms
/// (`linux`, `macos`, `windows`), returning `Err(TrackerError::ValidationError(_))` when all
/// config/home variables are unset or empty.
pub fn resolve_nebulous_config_dir_checked(
    nebulous_override: Option<&str>,
    xdg_config_home: Option<&str>,
    appdata: Option<&str>,
    home: Option<&str>,
    target_os: &str,
) -> Result<PathBuf> {
    if let Some(custom) = nebulous_override.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(PathBuf::from(custom));
    }

    if let Some(xdg) = xdg_config_home.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(PathBuf::from(xdg).join("nebulous"));
    }

    match target_os {
        "windows" => {
            if let Some(app) = appdata.map(str::trim).filter(|s| !s.is_empty()) {
                return Ok(PathBuf::from(app).join("nebulous"));
            }
            if let Some(h) = home.map(str::trim).filter(|s| !s.is_empty()) {
                return Ok(PathBuf::from(h)
                    .join("AppData")
                    .join("Roaming")
                    .join("nebulous"));
            }
        }
        "macos" => {
            if let Some(h) = home.map(str::trim).filter(|s| !s.is_empty()) {
                return Ok(PathBuf::from(h)
                    .join("Library")
                    .join("Application Support")
                    .join("nebulous"));
            }
        }
        _ => {
            if let Some(h) = home.map(str::trim).filter(|s| !s.is_empty()) {
                return Ok(PathBuf::from(h).join(".config").join("nebulous"));
            }
        }
    }

    Err(TrackerError::ValidationError(
        "Unable to determine Nebulous config directory: set NEBULOUS_CONFIG_DIR, XDG_CONFIG_HOME, APPDATA, HOME, or USERPROFILE".to_string(),
    ))
}

/// Deterministic helper for resolving the Nebulous config directory across platforms (`linux`, `macos`, `windows`).
pub fn resolve_nebulous_config_dir_from(
    nebulous_override: Option<&str>,
    xdg_config_home: Option<&str>,
    appdata: Option<&str>,
    home: Option<&str>,
    target_os: &str,
) -> PathBuf {
    resolve_nebulous_config_dir_checked(
        nebulous_override,
        xdg_config_home,
        appdata,
        home,
        target_os,
    )
    .unwrap_or_else(|_| std::env::temp_dir().join("nebulous"))
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
    /// Creates a `SecureStorage` instance rooted at [`try_nebulous_config_dir()`].
    ///
    /// Uses the OS keyring as primary storage (unless `NEBULOUS_DISABLE_KEYRING=1` is set)
    /// and maintains an AES-256-GCM encrypted vault fallback at `<config_dir>/vault.enc`.
    pub fn new() -> Result<Self> {
        Self::with_config_dir(try_nebulous_config_dir()?)
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
        // Startup hygiene: never leave plaintext key exports from older releases on disk.
        purge_legacy_ssh_cache(&config_dir)?;

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
        with_vault_lock(&self.config_dir, || {
            let payload = self.load_vault_unlocked()?;
            Ok(payload.entries.keys().cloned().collect())
        })
    }

    fn write_vault_entry(&self, key: &str, secret: &str) -> Result<()> {
        with_vault_lock(&self.config_dir, || {
            let mut payload = self.load_vault_unlocked()?;
            payload.version = 1;
            payload.entries.insert(key.to_string(), secret.to_string());
            self.save_vault_unlocked(&payload)
        })
    }

    fn read_vault_entry(&self, key: &str) -> Result<Option<Zeroizing<String>>> {
        with_vault_lock(&self.config_dir, || {
            let payload = self.load_vault_unlocked()?;
            Ok(payload.entries.get(key).cloned().map(Zeroizing::new))
        })
    }

    fn remove_vault_entry(&self, key: &str) -> Result<bool> {
        with_vault_lock(&self.config_dir, || {
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
        })
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
        atomic_write_bytes_with_mode(&self.vault_path, &encrypted, Some(0o600))?;
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

    let parent = key_path.parent().ok_or_else(|| {
        TrackerError::ValidationError("vault.key path has no parent directory".to_string())
    })?;

    // Two processes creating vault.key at once would each write a different key, and one
    // process's vault becomes undecryptable.
    with_vault_lock(parent, || {
        if key_path.is_file() {
            let raw = Zeroizing::new(fs::read_to_string(key_path).map_err(|source| {
                TrackerError::Io {
                    path: key_path.to_path_buf(),
                    source,
                }
            })?);
            let decoded =
                Zeroizing::new(BASE64_STANDARD.decode(raw.trim()).map_err(|e| {
                    TrackerError::KeyringError(format!("Corrupted vault.key: {e}"))
                })?);
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
        atomic_write_bytes_with_mode(key_path, encoded.as_bytes(), Some(0o600))?;
        set_secure_file_permissions(key_path)?;

        Ok(derive_key_sha256(&*random_seed, VAULT_KDF_CONTEXT))
    })
}

/// Resolves the absolute path to `icacls.exe` under `%SystemRoot%\System32` so PATH hijacking
/// cannot intercept ACL hardening on Windows.
#[cfg(windows)]
fn icacls_exe_path() -> PathBuf {
    let system_root = std::env::var("SystemRoot")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| r"C:\Windows".to_string());
    PathBuf::from(system_root)
        .join("System32")
        .join("icacls.exe")
}

/// Removes inherited ACEs on `path` and grants full control only to the current Windows user.
///
/// Windows does not honour Unix permission modes; without this, vault files and directories carry
/// whatever ACLs their directory has, including a `NEBULOUS_CONFIG_DIR` outside the profile.
#[cfg(windows)]
fn restrict_to_current_user(path: &Path, is_dir: bool) -> Result<()> {
    let user = std::env::var("USERNAME").map_err(|_| {
        TrackerError::KeyringError(format!(
            "USERNAME is not set; cannot restrict ACL on {}",
            path.display()
        ))
    })?;
    let principal = match std::env::var("USERDOMAIN") {
        Ok(domain) if !domain.trim().is_empty() => {
            let domain = domain.trim();
            format!("{domain}\\{user}")
        }
        _ => user,
    };
    let grant = if is_dir {
        format!("{principal}:(OI)(CI)F")
    } else {
        format!("{principal}:F")
    };
    let output = std::process::Command::new(icacls_exe_path())
        .arg(path)
        .arg("/inheritance:r")
        .arg("/grant:r")
        .arg(&grant)
        .output()
        .map_err(|source| TrackerError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if !output.status.success() {
        return Err(TrackerError::KeyringError(format!(
            "icacls failed on {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(())
}

/// Creates `dir` and sets `0700` on Unix; on Windows, inherited ACEs are removed and full control
/// is granted to the current user only.
pub(crate) fn ensure_secure_dir(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir).map_err(|source| TrackerError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(0o700);
        fs::set_permissions(dir, perms).map_err(|source| TrackerError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
    }
    #[cfg(windows)]
    {
        restrict_to_current_user(dir, true)?;
    }
    Ok(())
}

/// On Unix, sets file permissions to `0600`. On Windows, inherited ACEs are removed and full
/// control is granted to the current user only.
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
    #[cfg(windows)]
    {
        restrict_to_current_user(path, false)?;
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn vault_write_waits_for_vault_file_lock() {
        let dir = tempfile::tempdir().unwrap();
        let storage = SecureStorage::vault_only(dir.path()).unwrap();
        let path = dir.path().to_path_buf();
        let (tx, rx) = mpsc::channel();

        let handle = std::thread::spawn(move || {
            with_vault_lock(&path, || {
                tx.send(()).unwrap();
                std::thread::sleep(Duration::from_millis(400));
                Ok(())
            })
            .unwrap();
        });

        rx.recv().unwrap();
        let start = Instant::now();
        storage.store_secret("testuser", "s3cret").unwrap();
        let elapsed = start.elapsed();

        assert!(
            elapsed >= Duration::from_millis(300),
            "expected store_secret to wait on vault.lock, elapsed={elapsed:?}"
        );
        assert_eq!(storage.get_secret_string("testuser").unwrap(), "s3cret");
        assert!(dir.path().join("vault.lock").is_file());
        handle.join().unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn windows_vault_file_acl_is_user_only() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("vault.enc");
        fs::write(&file, b"secret").unwrap();
        set_secure_file_permissions(&file).unwrap();

        let output = std::process::Command::new("icacls")
            .arg(&file)
            .output()
            .unwrap();
        assert!(output.status.success());
        let stdout = String::from_utf8_lossy(&output.stdout);
        let user = std::env::var("USERNAME").unwrap();
        assert!(
            stdout.contains(&user),
            "expected icacls output to contain USERNAME ({user}), got: {stdout}"
        );
        assert!(
            !stdout.contains("(I)"),
            "expected icacls output to have no inherited ACE marker (I), got: {stdout}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn vault_files_created_with_0600_mode() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let storage = SecureStorage::vault_only(dir.path()).unwrap();
        storage.store_secret("testuser", "s3cret").unwrap();

        let dir_mode = fs::metadata(dir.path()).unwrap().permissions().mode() & 0o7777;
        let key_mode = fs::metadata(dir.path().join(VAULT_KEY_FILE_NAME))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777;
        let enc_mode = fs::metadata(dir.path().join(VAULT_FILE_NAME))
            .unwrap()
            .permissions()
            .mode()
            & 0o7777;

        assert_eq!(dir_mode, 0o700);
        assert_eq!(key_mode, 0o600);
        assert_eq!(enc_mode, 0o600);
    }

    #[cfg(unix)]
    #[test]
    fn ensure_secure_dir_propagates_permission_error() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let loose_dir = dir.path().join("loose");
        fs::create_dir_all(&loose_dir).unwrap();
        fs::set_permissions(&loose_dir, fs::Permissions::from_mode(0o755)).unwrap();

        ensure_secure_dir(&loose_dir).unwrap();
        let tightened = fs::metadata(&loose_dir).unwrap().permissions().mode() & 0o7777;
        assert_eq!(tightened, 0o700);

        let blocker = dir.path().join("regular_file");
        fs::write(&blocker, b"not a directory").unwrap();
        assert!(ensure_secure_dir(&blocker).is_err());
        assert!(ensure_secure_dir(&blocker.join("child")).is_err());
    }

    #[test]
    fn try_nebulous_config_dir_rejects_missing_env_vars() {
        let err = resolve_nebulous_config_dir_checked(None, None, None, None, "linux").unwrap_err();
        assert!(matches!(err, TrackerError::ValidationError(_)));

        let ok =
            resolve_nebulous_config_dir_checked(Some("/tmp/custom"), None, None, None, "linux")
                .unwrap();
        assert_eq!(ok, PathBuf::from("/tmp/custom"));
    }
}
