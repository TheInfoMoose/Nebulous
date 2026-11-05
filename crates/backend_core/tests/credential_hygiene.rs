//! STEP-11 credential hygiene: no key files on disk, HTTPS-only PATs, redacted remote URLs,
//! local-only account binding, and MCP refusal of raw PATs.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use backend_core::{
    fetch_repo_with_auth, git_credential, nebulous_config_dir, purge_legacy_ssh_cache,
    redact_url_userinfo, GitAuthMaterial, KeyPairManager, McpServer, SecureStorage, TrackerStorage,
    LEGACY_SSH_CACHE_DIR_NAME,
};
use git2::{CredentialType, Repository};
use serde_json::json;
use tempfile::TempDir;

const SECRET_PAT: &str = "ghp_step11_never_send_me";

fn init_tracker_repo(root: &Path, remote_url: &str) -> TrackerStorage {
    let storage = TrackerStorage::init(root, "CredHygiene", "01_core_engine", "Core Engine")
        .expect("should init tracker");
    let repo = Repository::init(root).expect("should init repo");
    repo.remote("origin", remote_url)
        .expect("should add remote");
    storage
}

#[test]
fn redact_url_userinfo_strips_credentials() {
    let cases = [
        (
            "https://user:ghp_secret@github.com/org/repo.git",
            "https://github.com/org/repo.git",
        ),
        (
            "https://ghp_secret@github.com/org/repo.git",
            "https://github.com/org/repo.git",
        ),
        (
            "http://u:p@127.0.0.1:8080/r?x=1",
            "http://127.0.0.1:8080/r?x=1",
        ),
        ("ssh://git@host:22/org/repo", "ssh://host:22/org/repo"),
        (
            "https://github.com/org/repo.git",
            "https://github.com/org/repo.git",
        ),
        ("git@github.com:org/repo.git", "git@github.com:org/repo.git"),
        ("/srv/git/bare.git", "/srv/git/bare.git"),
        // `@` after the authority (in the path) is not userinfo.
        ("https://host/org/a@b", "https://host/org/a@b"),
    ];
    for (input, expected) in cases {
        assert_eq!(redact_url_userinfo(input), expected, "input: {input}");
    }
}

#[test]
fn git_status_and_mcp_never_return_url_userinfo() {
    let dir = TempDir::new().unwrap();
    let storage = init_tracker_repo(
        dir.path(),
        &format!("https://x-access-token:{SECRET_PAT}@github.com/org/repo.git"),
    );

    let status = storage.git_status().expect("git_status");
    assert_eq!(
        status.remote_url.as_deref(),
        Some("https://github.com/org/repo.git")
    );
    let serialized = serde_json::to_string(&status).unwrap();
    assert!(!serialized.contains(SECRET_PAT));
    assert!(!serialized.contains("x-access-token"));
    assert!(!serialized.contains('@'));

    let mut server = McpServer::new(Some(TrackerStorage::new(dir.path())));
    let mcp_status = server
        .call_tool("git_status", json!({}))
        .expect("MCP git_status");
    let mcp_serialized = mcp_status.to_string();
    assert!(!mcp_serialized.contains(SECRET_PAT));
    assert!(!mcp_serialized.contains("x-access-token"));
    assert_eq!(
        mcp_status["remote_url"].as_str(),
        Some("https://github.com/org/repo.git")
    );
}

#[test]
fn mcp_git_tools_refuse_raw_pats_and_unbound_accounts() {
    let dir = TempDir::new().unwrap();
    let _storage = init_tracker_repo(dir.path(), "https://github.com/org/repo.git");
    let mut server = McpServer::new(Some(TrackerStorage::new(dir.path())));

    for tool in ["git_fetch", "git_pull", "git_push", "git_sync"] {
        let err = server
            .call_tool(tool, json!({ "pat": SECRET_PAT }))
            .expect_err("raw PAT over MCP must be refused");
        assert!(
            err.to_string().contains("not accepted over MCP"),
            "{tool}: {err}"
        );
    }

    // A key file path as auth_override is rejected before any network or vault access.
    let key_path = dir.path().join("id_ed25519");
    std::fs::write(&key_path, "not really a key").unwrap();
    let err = server
        .call_tool(
            "git_fetch",
            json!({ "auth_override": key_path.to_str().unwrap() }),
        )
        .expect_err("file-path auth_override must be rejected");
    assert!(err.to_string().contains("auth_override"), "{err}");
}

#[test]
fn pat_is_offered_only_to_https_urls() {
    let auth = GitAuthMaterial {
        pat: Some(SECRET_PAT.to_string()),
        ..GitAuthMaterial::default()
    };
    let user_pass = CredentialType::USER_PASS_PLAINTEXT;

    assert!(git_credential(&auth, "https://github.com/org/repo.git", None, user_pass).is_ok());
    assert!(git_credential(&auth, "HTTPS://github.com/org/repo.git", None, user_pass).is_ok());
    for url in [
        "http://github.com/org/repo.git",
        "git://github.com/org/repo.git",
        "ssh://git@github.com/org/repo.git",
        "file:///srv/repo.git",
    ] {
        let err = git_credential(&auth, url, None, user_pass)
            .err()
            .unwrap_or_else(|| panic!("PAT must not be offered to {url}"));
        assert!(err.message().contains("https://"), "{url}: {err}");
    }
}

/// Minimal HTTP server that answers every request with `401 Basic` and records request heads.
fn spawn_basic_auth_challenge_server() -> (String, Arc<Mutex<Vec<String>>>, Arc<AtomicBool>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
    listener.set_nonblocking(true).unwrap();
    let port = listener.local_addr().unwrap().port();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let stop = Arc::new(AtomicBool::new(false));
    let (req_t, stop_t) = (Arc::clone(&requests), Arc::clone(&stop));

    thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(20);
        while !stop_t.load(Ordering::SeqCst) && Instant::now() < deadline {
            let Ok((mut stream, _)) = listener.accept() else {
                thread::sleep(Duration::from_millis(10));
                continue;
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut head = Vec::new();
            let mut buf = [0u8; 1024];
            while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => head.extend_from_slice(&buf[..n]),
                }
            }
            req_t
                .lock()
                .unwrap()
                .push(String::from_utf8_lossy(&head).into_owned());
            let _ = stream.write_all(
                b"HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"neb\"\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            );
        }
    });

    (
        format!("http://127.0.0.1:{port}/org/repo.git"),
        requests,
        stop,
    )
}

#[test]
fn http_remote_never_receives_pat() {
    let (url, requests, stop) = spawn_basic_auth_challenge_server();
    let dir = TempDir::new().unwrap();
    let repo = Repository::init(dir.path()).unwrap();
    repo.remote("origin", &url).unwrap();

    let auth = GitAuthMaterial {
        pat: Some(SECRET_PAT.to_string()),
        ..GitAuthMaterial::default()
    };
    let result = fetch_repo_with_auth(dir.path(), "origin", "main", auth);
    stop.store(true, Ordering::SeqCst);
    assert!(result.is_err(), "fetch against a 401 server must fail");

    let seen = requests.lock().unwrap().clone();
    assert!(!seen.is_empty(), "the test server must have been contacted");
    for req in &seen {
        assert!(
            !req.to_ascii_lowercase().contains("authorization:"),
            "http:// remote received credentials: {req}"
        );
        assert!(!req.contains(SECRET_PAT));
    }
}

#[test]
fn vault_ssh_key_is_used_from_memory_and_ssh_cache_is_never_created() {
    let dir = TempDir::new().unwrap();
    let manager = KeyPairManager::vault_only(dir.path().join("vault")).unwrap();
    let record = manager
        .generate_ed25519_keypair("cred/hygiene", None)
        .unwrap();

    let global_cache = nebulous_config_dir().join(LEGACY_SSH_CACHE_DIR_NAME);
    let global_cache_existed = global_cache.exists();

    let auth = GitAuthMaterial {
        ssh_keypair: Some(record),
        ..GitAuthMaterial::default()
    };
    for allowed in [
        CredentialType::SSH_KEY,
        CredentialType::SSH_MEMORY,
        CredentialType::SSH_KEY | CredentialType::SSH_MEMORY,
    ] {
        assert!(
            git_credential(
                &auth,
                "ssh://git@example.com/org/repo",
                Some("git"),
                allowed
            )
            .is_ok(),
            "vault key must be usable from memory for {allowed:?}"
        );
    }

    assert!(!dir
        .path()
        .join("vault")
        .join(LEGACY_SSH_CACHE_DIR_NAME)
        .exists());
    if !global_cache_existed {
        assert!(
            !global_cache.exists(),
            "credential callback must never create {}",
            global_cache.display()
        );
    }
}

#[test]
fn legacy_ssh_cache_is_purged_when_the_vault_opens() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("cfg");
    let cache = cfg.join(LEGACY_SSH_CACHE_DIR_NAME);
    std::fs::create_dir_all(&cache).unwrap();
    std::fs::write(
        cache.join("nebulous_origin"),
        "-----BEGIN OPENSSH PRIVATE KEY-----",
    )
    .unwrap();

    let _vault = SecureStorage::vault_only(&cfg).expect("open vault");
    assert!(!cache.exists(), "ssh_cache must be purged on startup");

    // Idempotent when absent.
    assert!(!purge_legacy_ssh_cache(&cfg).unwrap());
}
