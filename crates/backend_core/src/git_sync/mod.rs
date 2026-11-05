use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use chrono::Utc;
use git2::{
    build::CheckoutBuilder, BranchType, Cred, CredentialType, FetchOptions, Index, IndexAddOption,
    IndexEntry, IndexTime, Oid, PushOptions, RemoteCallbacks, Repository, Signature, Status,
    StatusOptions, Tree,
};
use serde::{Deserialize, Serialize};

use crate::crypto::{nebulous_config_dir, KeyPairManager, SecureStorage, SshKeyPairRecord};
use crate::error::{Result, TrackerError};
pub use crate::models::project::CommitPolicyConfig;
use crate::models::PreviousFix;
use crate::storage::{normalize_relative_path_str, TrackerStorage};
use crate::workflow::lifecycle::now_rfc3339;

/// OS keyring service identifier for Nebulous Personal Access Tokens (PATs).
pub const KEYRING_SERVICE: &str = "nebulous-tracker";

static FALLBACK_PAT_STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn fallback_store() -> &'static Mutex<HashMap<String, String>> {
    FALLBACK_PAT_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Summary of the Git repository state relevant to the workspace and `.tracker/`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRepoStatus {
    pub head_short_sha: Option<String>,
    pub branch: Option<String>,
    pub remote_url: Option<String>,
    pub ahead_count: usize,
    pub behind_count: usize,
    pub tracker_dirty: bool,
    pub workspace_dirty: bool,
    pub dirty_tracker_files: Vec<String>,
    pub dirty_workspace_files: Vec<String>,
}

/// Input parameters for [`TrackerStorage::git_commit_documented`].
///
/// Commits are `.tracker/`-only by default. Workspace files are committed only when
/// `tracker_only == false` **and** each file or directory is listed explicitly in `paths`;
/// untracked files are staged only when their exact path is listed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentedCommitInput {
    /// Linked work item ID (`BUG-XXXX` or `FEAT-XXXX`).
    /// Required for workspace code commits when `commit_policy.require_item_id` is `true`.
    #[serde(default)]
    pub item_id: Option<String>,
    /// Optional domain scope ID override. When omitted and `item_id` is set, resolved from the item's scope.
    #[serde(default)]
    pub scope_id: Option<String>,
    /// Concise summary of the commit.
    #[serde(default)]
    pub summary: String,
    /// Optional extended commit body or agent notes.
    #[serde(default)]
    pub body: Option<String>,
    /// When `true` (the default), stages only `.tracker/` files. When `false`, stages `.tracker/`
    /// plus the explicitly listed workspace `paths` (which must be non-empty).
    #[serde(default = "default_tracker_only")]
    pub tracker_only: bool,
    /// Explicit project-root-relative file or directory paths to stage alongside `.tracker/`.
    /// Glob metacharacters (`* ? [ ] { }`), absolute paths, and `..` segments are rejected.
    /// Directories stage only changes to already-tracked files.
    #[serde(default)]
    pub paths: Vec<String>,
}

fn default_tracker_only() -> bool {
    true
}

impl Default for DocumentedCommitInput {
    fn default() -> Self {
        Self {
            item_id: None,
            scope_id: None,
            summary: String::new(),
            body: None,
            tracker_only: true,
            paths: Vec::new(),
        }
    }
}

impl DocumentedCommitInput {
    /// Creates a new `.tracker/`-only `DocumentedCommitInput` with `summary`.
    /// Use [`Self::with_paths`] to include explicit workspace files.
    pub fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            ..Self::default()
        }
    }

    /// Creates a new `.tracker/`-only `DocumentedCommitInput` linked to `item_id` with `summary`.
    /// Use [`Self::with_paths`] to include explicit workspace files.
    pub fn for_item(item_id: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            item_id: Some(item_id.into()),
            summary: summary.into(),
            ..Self::default()
        }
    }

    /// Creates a new `.tracker/`-only `DocumentedCommitInput`.
    pub fn tracker_only(summary: impl Into<String>) -> Self {
        Self::new(summary)
    }

    pub fn with_item_id(mut self, item_id: impl Into<String>) -> Self {
        self.item_id = Some(item_id.into());
        self
    }

    pub fn with_scope(mut self, scope_id: impl Into<String>) -> Self {
        self.scope_id = Some(scope_id.into());
        self
    }

    pub fn with_body(mut self, body: impl Into<String>) -> Self {
        self.body = Some(body.into());
        self
    }

    pub fn with_tracker_only(mut self, tracker_only: bool) -> Self {
        self.tracker_only = tracker_only;
        self
    }

    pub fn with_include_workspace(mut self, include_workspace: bool) -> Self {
        self.tracker_only = !include_workspace;
        self
    }

    /// Sets explicit workspace paths to stage; a non-empty list switches off `tracker_only`.
    pub fn with_paths<I, S>(mut self, paths: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.paths = paths.into_iter().map(Into::into).collect();
        if !self.paths.is_empty() {
            self.tracker_only = false;
        }
        self
    }
}

/// Outcome returned by [`TrackerStorage::git_commit_documented`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentedCommitOutcome {
    /// Final `HEAD` short SHA after committing (and amending `.tracker/` documentation if applicable).
    pub commit_sha: String,
    /// Short SHA recorded in `item.toml` (`history.previous_fixes`) and `desc.md`.
    pub recorded_fix_sha: String,
    /// Full formatted commit message.
    pub message: String,
    /// Linked item ID (`BUG-XXXX` / `FEAT-XXXX`), if any.
    pub item_id: Option<String>,
    /// Resolved scope ID, if any.
    pub scope_id: Option<String>,
    /// Normalized workspace-relative paths changed in this commit.
    pub changed_files: Vec<String>,
    /// `true` if `item.toml` and `desc.md` were automatically updated and staged into the commit.
    pub documentation_updated: bool,
}

impl DocumentedCommitOutcome {
    /// Returns the final `HEAD` short commit SHA.
    pub fn short_sha(&self) -> &str {
        &self.commit_sha
    }
}

/// Type alias for [`DocumentedCommitOutcome`].
pub type DocumentedCommitReport = DocumentedCommitOutcome;

/// Type alias for [`DocumentedCommitOutcome`].
pub type DocumentedCommitResult = DocumentedCommitOutcome;

/// Resolved Git authentication material combining HTTPS PAT and SSH credentials
/// (`SecureStorage` / `KeyPairManager`, `~/.ssh/id_ed25519`, or `SSH_AUTH_SOCK`).
#[derive(Debug, Clone, Default)]
pub struct GitAuthMaterial {
    pub pat: Option<String>,
    pub ssh_keypair: Option<SshKeyPairRecord>,
    pub ssh_key_file: Option<(PathBuf, Option<PathBuf>)>,
    pub ssh_auth_sock: Option<String>,
}

impl GitAuthMaterial {
    /// Returns `true` if an HTTPS PAT is available.
    pub fn has_pat(&self) -> bool {
        self.pat.as_deref().is_some_and(|t| !t.trim().is_empty())
    }

    /// Returns `true` if any SSH credential source (vault keypair, `~/.ssh` key file, or `SSH_AUTH_SOCK`) is available.
    pub fn has_ssh_credentials(&self) -> bool {
        self.ssh_keypair.is_some() || self.ssh_key_file.is_some() || self.ssh_auth_sock.is_some()
    }
}

/// Stores a Personal Access Token (PAT) for `project_name` in [`SecureStorage`] (OS keyring + AES-256-GCM vault).
pub fn store_pat_in_keyring(project_name: &str, token: &str) -> Result<()> {
    let account = normalize_keyring_account(project_name)?;
    let trimmed_token = token.trim();
    if trimmed_token.is_empty() {
        return Err(TrackerError::ValidationError(
            "Personal Access Token (PAT) must not be empty".to_string(),
        ));
    }

    let mut stored_in_vault = false;
    if let Ok(sec) = SecureStorage::new() {
        if sec.store_secret(&account, trimmed_token).is_ok() {
            stored_in_vault = true;
        }
    }

    if let Ok(mut guard) = fallback_store().lock() {
        guard.insert(account.clone(), trimmed_token.to_string());
    }

    let entry = keyring::Entry::new(KEYRING_SERVICE, &account)?;
    match entry.set_password(trimmed_token) {
        Ok(()) => Ok(()),
        Err(
            keyring::Error::PlatformFailure(_)
            | keyring::Error::NoStorageAccess(_)
            | keyring::Error::NoEntry,
        ) => Ok(()),
        Err(_) if stored_in_vault => Ok(()),
        Err(err) => Err(TrackerError::KeyringError(err.to_string())),
    }
}

/// Retrieves a Personal Access Token (PAT) for `project_name` from the OS keyring or encrypted vault.
pub fn get_pat_from_keyring(project_name: &str) -> Result<String> {
    let account = normalize_keyring_account(project_name)?;
    let entry = keyring::Entry::new(KEYRING_SERVICE, &account)?;
    if let Ok(password) = entry.get_password() {
        return Ok(password);
    }

    if let Ok(sec) = SecureStorage::new() {
        if let Ok(secret) = sec.get_secret_string(&account) {
            return Ok(secret);
        }
    }

    if let Ok(guard) = fallback_store().lock() {
        if let Some(cached) = guard.get(&account) {
            return Ok(cached.clone());
        }
    }

    Err(TrackerError::KeyringError(format!(
        "Personal Access Token (PAT) for '{account}' not found in OS keyring or encrypted vault"
    )))
}

/// Deletes a stored Personal Access Token (PAT) for `project_name` from the OS keyring and encrypted vault.
pub fn delete_pat_from_keyring(project_name: &str) -> Result<()> {
    let account = normalize_keyring_account(project_name)?;
    let had_fallback = fallback_store()
        .lock()
        .ok()
        .and_then(|mut guard| guard.remove(&account))
        .is_some();

    let had_vault = SecureStorage::new()
        .ok()
        .map(|sec| sec.delete_secret(&account).is_ok())
        .unwrap_or(false);

    let entry = keyring::Entry::new(KEYRING_SERVICE, &account)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(_) if had_fallback || had_vault => Ok(()),
        Err(err) => Err(TrackerError::KeyringError(err.to_string())),
    }
}

fn normalize_keyring_account(project_name: &str) -> Result<String> {
    let trimmed = project_name.trim();
    if trimmed.is_empty() {
        return Err(TrackerError::ValidationError(
            "Project name for keyring account must not be empty".to_string(),
        ));
    }
    Ok(trimmed.to_string())
}

/// Resolves the current `HEAD` commit short SHA for the Git repository containing `repo_path`.
pub fn head_short_sha(repo_path: impl AsRef<Path>) -> Result<String> {
    let repo = Repository::discover(repo_path.as_ref())?;
    head_short_sha_from_repo(&repo)
}

/// Resolves the current `HEAD` commit short SHA or returns `"uncommitted"` if the directory is
/// not inside a Git repository or has no commits yet.
pub fn head_short_sha_or_uncommitted(repo_path: impl AsRef<Path>) -> String {
    head_short_sha(repo_path).unwrap_or_else(|_| "uncommitted".to_string())
}

fn head_short_sha_from_repo(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    let commit = head.peel_to_commit()?;
    Ok(short_sha_for_oid(repo, commit.id()))
}

fn short_sha_for_oid(repo: &Repository, oid: Oid) -> String {
    if let Ok(commit) = repo.find_commit(oid) {
        if let Ok(buf) = commit.as_object().short_id() {
            if let Ok(s) = buf.as_str() {
                return s.to_string();
            }
        }
    }
    oid.to_string().chars().take(7).collect()
}

/// Inspects the Git status of the repository containing `repo_path`, computing dirty `.tracker/`
/// files, dirty workspace code files, remote URL, and `ahead_count` / `behind_count`.
pub fn repo_status(repo_path: impl AsRef<Path>) -> Result<GitRepoStatus> {
    let path = repo_path.as_ref();
    let (configured_remote, configured_branch) = TrackerStorage::discover(path)
        .ok()
        .and_then(|s| s.read_project().ok())
        .map(|p| (Some(p.git.remote), Some(p.git.branch)))
        .unwrap_or((None, None));

    repo_status_with_hints(
        path,
        configured_remote.as_deref(),
        configured_branch.as_deref(),
    )
}

fn repo_status_with_hints(
    repo_path: &Path,
    remote_hint: Option<&str>,
    branch_hint: Option<&str>,
) -> Result<GitRepoStatus> {
    let repo = Repository::discover(repo_path)?;
    let head_short_sha = head_short_sha_from_repo(&repo).ok();
    let head_branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().ok().map(str::to_string));
    let branch = head_branch
        .clone()
        .or_else(|| branch_hint.map(str::to_string));

    let remote_name = remote_hint
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("origin");

    let remote_url = repo
        .find_remote(remote_name)
        .or_else(|_| repo.find_remote("origin"))
        .ok()
        .and_then(|r| r.url().ok().map(str::to_string));

    let (ahead_count, behind_count) =
        compute_ahead_behind(&repo, remote_name, branch_hint, head_branch.as_deref());

    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);

    let statuses = repo.statuses(Some(&mut opts))?;
    let mut dirty_tracker_files = Vec::new();
    let mut dirty_workspace_files = Vec::new();

    for entry in statuses.iter() {
        let st = entry.status();
        if st == Status::CURRENT || st.contains(Status::IGNORED) {
            continue;
        }
        let Ok(raw_path) = entry.path() else {
            continue;
        };
        let normalized = normalize_relative_path_str(raw_path);
        if normalized.is_empty() || is_temp_atomic_path(&normalized) {
            continue;
        }
        if is_tracker_relative_path(&normalized) {
            dirty_tracker_files.push(normalized);
        } else {
            dirty_workspace_files.push(normalized);
        }
    }

    dirty_tracker_files.sort();
    dirty_tracker_files.dedup();
    dirty_workspace_files.sort();
    dirty_workspace_files.dedup();

    Ok(GitRepoStatus {
        head_short_sha,
        branch,
        remote_url,
        ahead_count,
        behind_count,
        tracker_dirty: !dirty_tracker_files.is_empty(),
        workspace_dirty: !dirty_workspace_files.is_empty(),
        dirty_tracker_files,
        dirty_workspace_files,
    })
}

fn compute_ahead_behind(
    repo: &Repository,
    remote_name: &str,
    configured_branch: Option<&str>,
    head_branch: Option<&str>,
) -> (usize, usize) {
    let mut candidate_branches = Vec::new();
    if let Some(b) = head_branch
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "HEAD")
    {
        candidate_branches.push(b);
    }
    if let Some(b) = configured_branch.map(str::trim).filter(|s| !s.is_empty()) {
        if !candidate_branches.contains(&b) {
            candidate_branches.push(b);
        }
    }

    let local_oid = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .map(|c| c.id())
        .or_else(|| {
            for b in &candidate_branches {
                if let Ok(r) = repo.find_reference(&format!("refs/heads/{b}")) {
                    if let Ok(c) = r.peel_to_commit() {
                        return Some(c.id());
                    }
                }
            }
            None
        });

    let Some(local_oid) = local_oid else {
        return (0, 0);
    };

    let mut upstream_oid = None;
    for b in &candidate_branches {
        let remote_ref = format!("refs/remotes/{remote_name}/{b}");
        if let Ok(r) = repo.find_reference(&remote_ref) {
            if let Ok(c) = r.peel_to_commit() {
                upstream_oid = Some(c.id());
                break;
            }
        }
    }

    if upstream_oid.is_none() {
        for b in &candidate_branches {
            if let Ok(local_branch) = repo.find_branch(b, BranchType::Local) {
                if let Ok(upstream) = local_branch.upstream() {
                    if let Ok(c) = upstream.get().peel_to_commit() {
                        upstream_oid = Some(c.id());
                        break;
                    }
                }
            }
        }
    }

    if upstream_oid.is_none() {
        if let Ok(fetch_head) = repo.find_reference("FETCH_HEAD") {
            if let Ok(c) = fetch_head.peel_to_commit() {
                upstream_oid = Some(c.id());
            }
        }
    }

    let Some(upstream_oid) = upstream_oid else {
        return (0, 0);
    };

    repo.graph_ahead_behind(local_oid, upstream_oid)
        .unwrap_or((0, 0))
}

fn is_tracker_relative_path(path: &str) -> bool {
    path == ".tracker"
        || path.starts_with(".tracker/")
        || path.starts_with(".tracker\\")
        || path.contains("/.tracker/")
}

fn is_temp_atomic_path(path: &str) -> bool {
    path.contains(".tmp.")
}

fn is_derived_tracker_index_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    normalized == ".tracker/project.toml"
        || (normalized.starts_with(".tracker/phases/")
            && (normalized.ends_with("/phase.toml") || normalized.ends_with("/scope.toml")))
}

const GLOB_METACHARS: [char; 6] = ['*', '?', '[', ']', '{', '}'];

fn contains_glob_metachar(path: &str) -> bool {
    path.contains(GLOB_METACHARS)
}

fn temp_atomic_filter(path: &Path, _matched_spec: &[u8]) -> i32 {
    if path.to_string_lossy().contains(".tmp.") {
        1
    } else {
        0
    }
}

/// Returns the repository working directory, rejecting bare repositories.
fn repo_workdir(repo: &Repository) -> Result<PathBuf> {
    repo.workdir().map(Path::to_path_buf).ok_or_else(|| {
        TrackerError::GitError("Repository has no working directory (bare repository)".to_string())
    })
}

/// Computes the repo-workdir-relative prefix of `project_root` (`""` when the project root is the
/// workdir itself), so pathspecs work for project roots nested inside a repository.
fn project_prefix(repo: &Repository, project_root: &Path) -> Result<String> {
    let workdir = repo_workdir(repo)?.canonicalize()?;
    let root = project_root.canonicalize()?;
    let rel = root.strip_prefix(&workdir).map_err(|_| {
        TrackerError::GitError(format!(
            "Project root '{}' is outside repository workdir '{}'",
            root.display(),
            workdir.display()
        ))
    })?;
    let prefix = normalize_relative_path_str(&rel.to_string_lossy());
    let prefix = if prefix == "." { String::new() } else { prefix };
    if contains_glob_metachar(&prefix) {
        return Err(TrackerError::GitError(format!(
            "Project root path '{prefix}' contains glob metacharacters and cannot be used as a pathspec"
        )));
    }
    Ok(prefix)
}

fn join_repo_path(prefix: &str, rel: &str) -> String {
    if prefix.is_empty() {
        rel.to_string()
    } else {
        format!("{prefix}/{rel}")
    }
}

/// Strips the project prefix from a repo-relative path, returning a project-relative path.
fn strip_repo_prefix<'a>(prefix: &str, repo_rel: &'a str) -> Option<&'a str> {
    if prefix.is_empty() {
        return Some(repo_rel);
    }
    repo_rel
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_prefix('/'))
}

/// Opens a dedicated repository handle whose index is a private in-memory [`Index`] seeded from
/// `parent_tree`. The user's on-disk staging area (`.git/index`) is never read or written: the
/// private index has no backing file, so it cannot be persisted by accident.
fn open_private_staging(
    workdir: &Path,
    parent_tree: Option<&Tree<'_>>,
) -> Result<(Repository, Index)> {
    let stage_repo = Repository::open(workdir)?;
    let mut index = Index::new()?;
    if let Some(tree) = parent_tree {
        index.read_tree(tree)?;
    }
    stage_repo.set_index(&mut index)?;
    Ok((stage_repo, index))
}

/// Stages every add/modify/delete under `<prefix>/.tracker` into the private `index`.
fn stage_tracker_dir(index: &mut Index, prefix: &str) -> Result<()> {
    let spec = join_repo_path(prefix, ".tracker");
    index.update_all([spec.as_str()], None)?;
    let mut filter_cb = temp_atomic_filter;
    index.add_all(
        [spec.as_str()],
        IndexAddOption::DEFAULT,
        Some(&mut filter_cb),
    )?;
    Ok(())
}

/// Stages an explicit workspace path (repo-relative) into the private `index`.
///
/// Files are added (even if untracked, because they were listed explicitly) or removed when
/// deleted. Directories only stage modifications/deletions of already-tracked files.
fn stage_explicit_path(index: &mut Index, workdir: &Path, repo_rel: &str) -> Result<()> {
    index.update_all([repo_rel], None)?;
    let abs = workdir.join(repo_rel);
    if let Ok(meta) = std::fs::symlink_metadata(&abs) {
        if meta.is_file() || meta.file_type().is_symlink() {
            index.add_path(Path::new(repo_rel))?;
        }
    }
    Ok(())
}

/// Lists repo-relative paths with any change (index, worktree, or untracked) under `spec`.
fn changed_paths_under(repo: &Repository, spec: &str) -> Result<Vec<String>> {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false)
        .pathspec(spec);
    let statuses = repo.statuses(Some(&mut opts))?;
    let mut out = Vec::new();
    for entry in statuses.iter() {
        if entry.status() == Status::CURRENT || entry.status().contains(Status::IGNORED) {
            continue;
        }
        if let Ok(p) = entry.path() {
            let norm = normalize_relative_path_str(p);
            if !norm.is_empty() && !is_temp_atomic_path(&norm) {
                out.push(norm);
            }
        }
    }
    Ok(out)
}

fn changed_files_between(
    repo: &Repository,
    old_tree: Option<&Tree<'_>>,
    new_tree: &Tree<'_>,
) -> Result<Vec<String>> {
    let diff = repo.diff_tree_to_tree(old_tree, Some(new_tree), None)?;
    let mut changed = Vec::new();
    for delta in diff.deltas() {
        for p in [delta.new_file().path(), delta.old_file().path()]
            .into_iter()
            .flatten()
        {
            let norm = normalize_relative_path_str(&p.to_string_lossy());
            if !norm.is_empty() && !is_temp_atomic_path(&norm) {
                changed.push(norm);
            }
        }
    }
    changed.sort();
    changed.dedup();
    Ok(changed)
}

/// After a commit built from a private index, updates only the committed `paths` in the user's
/// on-disk index to match `tree` (`git commit --only` semantics). Every other index entry —
/// including pre-staged hunks in unrelated files — is left byte-identical. Without this the
/// user's index would still hold the pre-commit blobs and a later `git commit` would revert
/// Nebulous' commit.
fn sync_user_index_paths(repo: &Repository, tree: &Tree<'_>, paths: &[String]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let mut index = repo.index()?;
    index.read(true)?;
    for p in paths {
        let path = Path::new(p);
        match tree.get_path(path) {
            Ok(entry) => {
                let blob_size = repo
                    .find_blob(entry.id())
                    .map(|b| b.size() as u32)
                    .unwrap_or(0);
                for stage in 1..=3 {
                    let _ = index.remove(path, stage);
                }
                index.add(&IndexEntry {
                    ctime: IndexTime::new(0, 0),
                    mtime: IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: entry.filemode() as u32,
                    uid: 0,
                    gid: 0,
                    file_size: blob_size,
                    id: entry.id(),
                    flags: 0,
                    flags_extended: 0,
                    path: p.as_bytes().to_vec(),
                })?;
            }
            Err(_) => {
                for stage in 0..=3 {
                    let _ = index.remove(path, stage);
                }
            }
        }
    }
    index.write()?;
    Ok(())
}

/// Resolves the branch `HEAD` points at and verifies it matches the configured `[git] branch`.
///
/// Errors when `HEAD` is detached or on a different branch, so pull/push never move or publish a
/// branch the user is not on.
fn checked_head_branch(repo: &Repository, configured_branch: &str) -> Result<String> {
    let head = repo.find_reference("HEAD")?;
    let target = head.symbolic_target()?.map(str::to_string).ok_or_else(|| {
        TrackerError::GitError(
            "HEAD is detached; check out the configured branch before pull/push".to_string(),
        )
    })?;
    let branch = target
        .strip_prefix("refs/heads/")
        .ok_or_else(|| TrackerError::GitError(format!("HEAD points at non-branch ref '{target}'")))?
        .to_string();
    let configured = configured_branch.trim();
    if !configured.is_empty() && configured != branch {
        return Err(TrackerError::GitError(format!(
            "HEAD is on branch '{branch}' but [git] branch is configured as '{configured}'; \
             check out '{configured}' or update project.toml before pull/push"
        )));
    }
    Ok(branch)
}

/// Stages all modified, added, and deleted files under `.tracker/` and creates a Git commit.
///
/// The commit tree is built from a private in-memory index; only the committed `.tracker/`
/// paths are refreshed in the user's index afterwards.
///
/// Returns `Ok(Some(short_sha))` if a new commit was created, or `Ok(None)` if `.tracker/` had
/// no changes to commit.
pub fn stage_and_commit_tracker(
    repo_path: impl AsRef<Path>,
    message: Option<&str>,
) -> Result<Option<String>> {
    let status = repo_status(&repo_path)?;
    if !status.tracker_dirty {
        return Ok(None);
    }

    let repo = Repository::discover(repo_path.as_ref())?;
    let workdir = repo_workdir(&repo)?;
    let prefix = project_prefix(&repo, repo_path.as_ref())?;
    let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parent_tree = parent_commit.as_ref().and_then(|p| p.tree().ok());

    let (_stage_repo, mut index) = open_private_staging(&workdir, parent_tree.as_ref())?;
    stage_tracker_dir(&mut index, &prefix)?;

    let tree_id = index.write_tree_to(&repo)?;
    let tree = repo.find_tree(tree_id)?;

    if let Some(ref parent) = parent_commit {
        if parent.tree_id() == tree_id {
            return Ok(None);
        }
    }

    let sig = resolve_signature(&repo)?;
    let commit_msg = message
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .unwrap_or("chore(tracker): sync .tracker state");

    let commit_oid = match parent_commit {
        Some(ref parent) => repo.commit(Some("HEAD"), &sig, &sig, commit_msg, &tree, &[parent])?,
        None => repo.commit(Some("HEAD"), &sig, &sig, commit_msg, &tree, &[])?,
    };

    let committed = changed_files_between(&repo, parent_tree.as_ref(), &tree)?;
    sync_user_index_paths(&repo, &tree, &committed)?;

    Ok(Some(short_sha_for_oid(&repo, commit_oid)))
}

fn resolve_signature(repo: &Repository) -> Result<Signature<'static>> {
    if let Ok(sig) = repo.signature() {
        let name = sig.name().unwrap_or("Nebulous Tracker").to_string();
        let email = sig.email().unwrap_or("nebulous@localhost").to_string();
        if let Ok(owned) = Signature::now(&name, &email) {
            return Ok(owned);
        }
    }
    Ok(Signature::now("Nebulous Tracker", "nebulous@localhost")?)
}

fn resolve_effective_branch(repo: &Repository, configured_branch: &str) -> String {
    let trimmed = configured_branch.trim();
    if !trimmed.is_empty()
        && repo
            .find_reference(&format!("refs/heads/{trimmed}"))
            .is_ok()
    {
        return trimmed.to_string();
    }
    if let Ok(head) = repo.head() {
        if let Ok(shorthand) = head.shorthand() {
            if !shorthand.is_empty() && shorthand != "HEAD" {
                return shorthand.to_string();
            }
        }
    }
    if trimmed.is_empty() {
        "main".to_string()
    } else {
        trimmed.to_string()
    }
}

fn discover_local_ssh_key_files() -> Option<(PathBuf, Option<PathBuf>)> {
    let home = std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())?;
    let ssh_dir = PathBuf::from(home).join(".ssh");
    for stem in ["id_ed25519", "id_rsa"] {
        let priv_path = ssh_dir.join(stem);
        if priv_path.is_file() {
            let pub_candidate = ssh_dir.join(format!("{stem}.pub"));
            let pub_path = if pub_candidate.is_file() {
                Some(pub_candidate)
            } else {
                None
            };
            return Some((priv_path, pub_path));
        }
    }
    None
}

/// Builds `git2::RemoteCallbacks` configured to supply a Personal Access Token (PAT) over HTTPS
/// or fallback SSH credentials when requested by the remote transport.
pub fn build_pat_callbacks<'a>(pat: Option<&'a str>) -> RemoteCallbacks<'a> {
    let auth = GitAuthMaterial {
        pat: pat
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string),
        ssh_keypair: None,
        ssh_key_file: discover_local_ssh_key_files(),
        ssh_auth_sock: std::env::var("SSH_AUTH_SOCK")
            .ok()
            .filter(|s| !s.trim().is_empty()),
    };
    build_auth_callbacks(auth)
}

/// Builds `git2::RemoteCallbacks` capable of authenticating via HTTPS PAT or SSH credentials
/// (`SecureStorage` / `KeyPairManager` OpenSSH keypair, `~/.ssh/id_ed25519`, or `SSH_AUTH_SOCK`).
pub fn build_auth_callbacks<'a>(auth: GitAuthMaterial) -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();
    let mut attempts: u8 = 0;

    callbacks.credentials(move |_url, username_from_url, allowed_types| {
        attempts = attempts.saturating_add(1);
        if attempts > 5 {
            return Err(git2::Error::from_str(
                "Maximum Git authentication attempts exceeded",
            ));
        }

        if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
            if let Some(ref token) = auth.pat {
                let user = username_from_url.unwrap_or("x-access-token");
                return Cred::userpass_plaintext(user, token);
            }
        }

        if allowed_types.intersects(
            CredentialType::SSH_KEY | CredentialType::SSH_MEMORY | CredentialType::SSH_CUSTOM,
        ) {
            let ssh_user = username_from_url.unwrap_or("git");

            if let Some(ref record) = auth.ssh_keypair {
                if allowed_types.contains(CredentialType::SSH_MEMORY)
                    || allowed_types.contains(CredentialType::SSH_KEY)
                {
                    if let Ok(cred) = Cred::ssh_key_from_memory(
                        ssh_user,
                        Some(&record.public_key_openssh),
                        record.private_key_openssh.as_str(),
                        None,
                    ) {
                        return Ok(cred);
                    }
                }

                if allowed_types.contains(CredentialType::SSH_KEY) {
                    if let Ok(manager) = KeyPairManager::new() {
                        let cache_dir = nebulous_config_dir().join("ssh_cache");
                        let safe_stem: String = record
                            .account
                            .chars()
                            .map(|c| {
                                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                                    c
                                } else {
                                    '_'
                                }
                            })
                            .collect();
                        if let Ok((priv_path, pub_path)) =
                            manager.export_keypair_to_dir(&record.account, &cache_dir, &safe_stem)
                        {
                            if let Ok(cred) =
                                Cred::ssh_key(ssh_user, Some(&pub_path), &priv_path, None)
                            {
                                return Ok(cred);
                            }
                        }
                    }
                }
            }

            if let Some((ref priv_path, ref pub_path)) = auth.ssh_key_file {
                if let Ok(cred) = Cred::ssh_key(ssh_user, pub_path.as_deref(), priv_path, None) {
                    return Ok(cred);
                }
            }

            if auth.ssh_auth_sock.is_some() {
                if let Ok(cred) = Cred::ssh_key_from_agent(ssh_user) {
                    return Ok(cred);
                }
            }
        }

        if allowed_types.contains(CredentialType::USERNAME) {
            return Cred::username(username_from_url.unwrap_or("git"));
        }

        Cred::default()
    });

    callbacks
}

/// Fetches `branch` from `remote_name` without modifying the working tree or `HEAD`,
/// updating `refs/remotes/<remote_name>/<branch>` so `ahead_count` and `behind_count` are current.
pub fn fetch_repo(
    repo_path: impl AsRef<Path>,
    remote_name: &str,
    branch: &str,
    pat: Option<&str>,
) -> Result<GitRepoStatus> {
    let auth = GitAuthMaterial {
        pat: pat
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string),
        ssh_keypair: None,
        ssh_key_file: discover_local_ssh_key_files(),
        ssh_auth_sock: std::env::var("SSH_AUTH_SOCK")
            .ok()
            .filter(|s| !s.trim().is_empty()),
    };
    fetch_repo_with_auth(repo_path, remote_name, branch, auth)
}

/// Fetches `branch` from `remote_name` using [`GitAuthMaterial`] without modifying the working tree.
pub fn fetch_repo_with_auth(
    repo_path: impl AsRef<Path>,
    remote_name: &str,
    branch: &str,
    auth: GitAuthMaterial,
) -> Result<GitRepoStatus> {
    let path = repo_path.as_ref();
    let repo = Repository::discover(path)?;
    let effective_branch = resolve_effective_branch(&repo, branch);
    let mut remote = repo.find_remote(remote_name)?;
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(build_auth_callbacks(auth.clone()));

    let tracking_refspec =
        format!("+refs/heads/{effective_branch}:refs/remotes/{remote_name}/{effective_branch}");
    if remote
        .fetch(&[&tracking_refspec], Some(&mut fetch_opts), None)
        .is_err()
    {
        let mut retry_opts = FetchOptions::new();
        retry_opts.remote_callbacks(build_auth_callbacks(auth));
        remote.fetch(&[&effective_branch], Some(&mut retry_opts), None)?;
    }

    if let Ok(fetch_head) = repo.find_reference("FETCH_HEAD") {
        if let Ok(fetch_commit) = repo.reference_to_annotated_commit(&fetch_head) {
            let remote_ref = format!("refs/remotes/{remote_name}/{effective_branch}");
            let _ = repo.reference(
                &remote_ref,
                fetch_commit.id(),
                true,
                "fetch update remote tracking ref",
            );
            if branch.trim() != effective_branch && !branch.trim().is_empty() {
                let configured_remote_ref = format!("refs/remotes/{remote_name}/{}", branch.trim());
                let _ = repo.reference(
                    &configured_remote_ref,
                    fetch_commit.id(),
                    true,
                    "fetch update configured remote tracking ref",
                );
            }
        }
    }

    repo_status_with_hints(path, Some(remote_name), Some(&effective_branch))
}

/// Pushes `branch` to `remote_name` using optional HTTPS PAT authentication.
pub fn push_repo(
    repo_path: impl AsRef<Path>,
    remote_name: &str,
    branch: &str,
    pat: Option<&str>,
) -> Result<()> {
    let auth = GitAuthMaterial {
        pat: pat
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string),
        ssh_keypair: None,
        ssh_key_file: discover_local_ssh_key_files(),
        ssh_auth_sock: std::env::var("SSH_AUTH_SOCK")
            .ok()
            .filter(|s| !s.trim().is_empty()),
    };
    push_repo_with_auth(repo_path, remote_name, branch, auth)
}

/// Pushes HEAD's branch to `remote_name` using [`GitAuthMaterial`] (HTTPS PAT or SSH credentials).
///
/// Errors (without pushing) when HEAD is detached or not on the configured `branch`.
pub fn push_repo_with_auth(
    repo_path: impl AsRef<Path>,
    remote_name: &str,
    branch: &str,
    auth: GitAuthMaterial,
) -> Result<()> {
    let repo = Repository::discover(repo_path.as_ref())?;
    let head_branch = checked_head_branch(&repo, branch)?;
    let mut remote = repo.find_remote(remote_name)?;
    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(build_auth_callbacks(auth));

    let refspec = format!("refs/heads/{head_branch}:refs/heads/{head_branch}");
    remote.push(&[&refspec], Some(&mut push_opts))?;

    // Update the local remote-tracking ref so ahead/behind counts immediately reflect the push
    if let Ok(local_ref) = repo.find_reference(&format!("refs/heads/{head_branch}")) {
        if let Ok(commit) = local_ref.peel_to_commit() {
            let remote_ref = format!("refs/remotes/{remote_name}/{head_branch}");
            let _ = repo.reference(
                &remote_ref,
                commit.id(),
                true,
                "push update remote tracking ref",
            );
        }
    }

    Ok(())
}

/// Fetches and merges (fast-forward or clean merge) `branch` from `remote_name` using optional
/// HTTPS PAT authentication.
///
/// Returns `Ok(true)` if new commits were integrated into the local branch, or `Ok(false)` if
/// already up to date.
pub fn pull_repo(
    repo_path: impl AsRef<Path>,
    remote_name: &str,
    branch: &str,
    pat: Option<&str>,
) -> Result<bool> {
    let auth = GitAuthMaterial {
        pat: pat
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string),
        ssh_keypair: None,
        ssh_key_file: discover_local_ssh_key_files(),
        ssh_auth_sock: std::env::var("SSH_AUTH_SOCK")
            .ok()
            .filter(|s| !s.trim().is_empty()),
    };
    pull_repo_with_auth(repo_path, remote_name, branch, auth)
}

/// Fetches and merges (fast-forward or clean merge with automatic `.tracker/` derived index
/// conflict resolution) HEAD's branch from `remote_name` using [`GitAuthMaterial`].
///
/// Errors before fetching when HEAD is detached or not on the configured `branch`. Checkouts are
/// safe (never forced): local modifications that would be overwritten abort the pull.
pub fn pull_repo_with_auth(
    repo_path: impl AsRef<Path>,
    remote_name: &str,
    branch: &str,
    auth: GitAuthMaterial,
) -> Result<bool> {
    let repo = Repository::discover(repo_path.as_ref())?;
    // Refuse before touching any ref when HEAD is not on the configured branch.
    let head_branch = checked_head_branch(&repo, branch)?;
    let prefix = project_prefix(&repo, repo_path.as_ref())?;
    let mut remote = repo.find_remote(remote_name)?;
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(build_auth_callbacks(auth));

    remote.fetch(&[&head_branch], Some(&mut fetch_opts), None)?;

    let fetch_head = match repo.find_reference("FETCH_HEAD") {
        Ok(r) => r,
        Err(_) => return Ok(false),
    };
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let remote_tracking_ref = format!("refs/remotes/{remote_name}/{head_branch}");
    let _ = repo.reference(
        &remote_tracking_ref,
        fetch_commit.id(),
        true,
        "pull update remote tracking ref",
    );

    let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.is_up_to_date() {
        return Ok(false);
    }

    // Before modifying HEAD, ensure no dirty workspace or `.tracker/` files collide with files
    // changed on the remote. Fail closed when the status check itself errors.
    let local_commit_opt = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let local_tree_opt = local_commit_opt.as_ref().and_then(|c| c.tree().ok());
    let remote_commit = repo.find_commit(fetch_commit.id())?;
    let remote_tree = remote_commit.tree()?;
    let status = repo_status_with_hints(repo_path.as_ref(), Some(remote_name), Some(&head_branch))
        .map_err(|e| {
            TrackerError::GitError(format!(
                "Refusing to pull: could not verify working tree is clean ({e})"
            ))
        })?;
    let dirty: Vec<&String> = status
        .dirty_workspace_files
        .iter()
        .chain(status.dirty_tracker_files.iter())
        .collect();
    if !dirty.is_empty() {
        let remote_changed = changed_files_between(&repo, local_tree_opt.as_ref(), &remote_tree)?;
        for norm in remote_changed {
            if dirty.iter().any(|d| **d == norm) {
                return Err(TrackerError::GitError(format!(
                    "Uncommitted local changes in '{norm}' would be overwritten by pull"
                )));
            }
        }
    }

    let refname = format!("refs/heads/{head_branch}");
    if analysis.is_fast_forward() {
        repo.checkout_tree(
            remote_tree.as_object(),
            Some(CheckoutBuilder::default().safe()),
        )?;
        // HEAD is symbolic to `refname` (verified above), so only that ref moves.
        match repo.find_reference(&refname) {
            Ok(mut r) => {
                r.set_target(fetch_commit.id(), "Fast-Forward pull")?;
            }
            Err(_) => {
                repo.reference(&refname, fetch_commit.id(), false, "Fast-Forward pull")?;
            }
        }
        return Ok(true);
    }

    if analysis.is_normal() {
        let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
        let local_commit = repo.find_commit(head_commit.id())?;

        let mut index = repo.merge_commits(&local_commit, &remote_commit, None)?;
        let mut resolved_derived_index_conflict = false;
        if index.has_conflicts() {
            let mut resolutions = Vec::new();
            for conflict_res in index.conflicts()? {
                let conflict = conflict_res?;
                let candidate = conflict
                    .our
                    .as_ref()
                    .or(conflict.their.as_ref())
                    .or(conflict.ancestor.as_ref());
                let path_str = candidate
                    .and_then(|e| std::str::from_utf8(&e.path).ok())
                    .unwrap_or_default()
                    .to_string();

                if strip_repo_prefix(&prefix, &path_str).is_some_and(is_derived_tracker_index_path)
                {
                    if let Some(mut chosen) = conflict.our.or(conflict.their) {
                        chosen.flags = 0;
                        chosen.flags_extended = 0;
                        resolutions.push((path_str, chosen));
                        continue;
                    }
                }
                return Err(TrackerError::GitError(
                    "Merge conflicts detected while pulling remote branch".to_string(),
                ));
            }

            for (path_str, chosen) in resolutions {
                let p = Path::new(&path_str);
                let _ = index.remove(p, 1);
                let _ = index.remove(p, 2);
                let _ = index.remove(p, 3);
                index.add(&chosen)?;
                resolved_derived_index_conflict = true;
            }
        }

        let tree_id = index.write_tree_to(&repo)?;
        let tree = repo.find_tree(tree_id)?;
        // Safe checkout: libgit2 detects conflicts with local modifications before writing any
        // file, so on conflict nothing is written and HEAD is left untouched.
        repo.checkout_tree(tree.as_object(), Some(CheckoutBuilder::default().safe()))
            .map_err(|e| {
                TrackerError::GitError(format!(
                    "Pull aborted: merge result conflicts with local changes ({})",
                    e.message()
                ))
            })?;
        let sig = resolve_signature(&repo)?;
        let msg = format!("Merge remote-tracking branch '{remote_name}/{head_branch}'");
        let merge_commit_oid = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &msg,
            &tree,
            &[&local_commit, &remote_commit],
        )?;

        if resolved_derived_index_conflict {
            if let Ok(storage) = TrackerStorage::discover(repo_path.as_ref()) {
                if storage.reconcile_indexes().is_ok() {
                    // Stage only the reconciled derived indexes into a private index seeded from
                    // the merge tree; unrelated local `.tracker/` edits stay uncommitted.
                    let workdir = repo_workdir(&repo)?;
                    let tracker_spec = join_repo_path(&prefix, ".tracker");
                    let derived: Vec<String> = changed_paths_under(&repo, &tracker_spec)?
                        .into_iter()
                        .filter(|p| {
                            strip_repo_prefix(&prefix, p).is_some_and(is_derived_tracker_index_path)
                        })
                        .collect();
                    let (_stage_repo, mut private_idx) =
                        open_private_staging(&workdir, Some(&tree))?;
                    for p in &derived {
                        stage_explicit_path(&mut private_idx, &workdir, p)?;
                    }
                    let reconciled_tree_id = private_idx.write_tree_to(&repo)?;
                    if reconciled_tree_id != tree_id {
                        let reconciled_tree = repo.find_tree(reconciled_tree_id)?;
                        let merge_commit = repo.find_commit(merge_commit_oid)?;
                        merge_commit.amend(
                            Some("HEAD"),
                            None,
                            None,
                            None,
                            None,
                            Some(&reconciled_tree),
                        )?;
                        let amended = changed_files_between(&repo, Some(&tree), &reconciled_tree)?;
                        sync_user_index_paths(&repo, &reconciled_tree, &amended)?;
                    }
                }
            }
        }

        return Ok(true);
    }

    Ok(false)
}

fn append_documented_fix_to_markdown(
    existing_markdown: &str,
    commit_sha: &str,
    date: &str,
    scope_id: &str,
    summary: &str,
    body: Option<&str>,
    changed_files: &[String],
) -> String {
    let mut out = existing_markdown.to_string();
    if !out.contains("## Fix Log & Agent Notes") {
        if !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("\n## Fix Log & Agent Notes\n");
    } else if !out.ends_with('\n') {
        out.push('\n');
    }

    let files_formatted = if changed_files.is_empty() {
        ".tracker".to_string()
    } else {
        changed_files.join(", ")
    };

    out.push_str(&format!(
        "- **Commit `{commit_sha}`** ({date}) [`{scope_id}`]: {summary}\n  - Changed files: `{files_formatted}`\n"
    ));

    if let Some(notes) = body.map(str::trim).filter(|b| !b.is_empty()) {
        for line in notes.lines() {
            out.push_str(&format!("  - Note: {}\n", line.trim()));
        }
    }

    out
}

/// Validated, ready-to-commit form of a [`DocumentedCommitInput`].
struct PreparedCommit {
    auto_append_to_desc: bool,
    /// Repo-relative prefix of the project root (`""` when it is the workdir).
    prefix: String,
    /// Normalized project-relative workspace paths (never `.tracker/`).
    workspace_paths: Vec<String>,
    item_id: Option<String>,
    loaded_item: Option<(crate::storage::ItemLocation, crate::models::ItemDetail)>,
    resolved_scope: Option<String>,
    effective_summary: String,
    commit_message: String,
}

impl TrackerStorage {
    /// Returns the current Git `HEAD` short SHA for the workspace or `"uncommitted"` if unavailable.
    pub fn git_head_short_sha(&self) -> String {
        head_short_sha_or_uncommitted(self.project_root())
    }

    /// Returns the Git status for the repository containing `.tracker/`, including
    /// `dirty_workspace_files`, `dirty_tracker_files`, `ahead_count`, `behind_count`, and `remote_url`.
    pub fn git_status(&self) -> Result<GitRepoStatus> {
        let root = self.project_root();
        if let Ok(project) = self.read_project() {
            repo_status_with_hints(&root, Some(&project.git.remote), Some(&project.git.branch))
        } else {
            repo_status(&root)
        }
    }

    /// Stages all `.tracker/` changes and creates a commit with `message`.
    pub fn git_commit_tracker(&self, message: Option<&str>) -> Result<Option<String>> {
        stage_and_commit_tracker(self.project_root(), message)
    }

    /// Resolves authentication material (HTTPS PAT and SSH credentials from `SecureStorage` /
    /// `KeyPairManager`, `~/.ssh/id_ed25519`, or `SSH_AUTH_SOCK`) for this project.
    pub fn resolve_git_auth(&self, auth_override: Option<&str>) -> Result<GitAuthMaterial> {
        let default_manager = self.keypair_manager().ok();
        self.resolve_git_auth_with_manager(default_manager.as_ref(), auth_override)
    }

    /// Resolves authentication material using an explicit [`KeyPairManager`] (when provided)
    /// alongside keyring/fallback PAT storage, `~/.ssh/id_ed25519`, and `SSH_AUTH_SOCK`.
    pub fn resolve_git_auth_with_manager(
        &self,
        manager: Option<&KeyPairManager>,
        auth_override: Option<&str>,
    ) -> Result<GitAuthMaterial> {
        let project = self.read_project()?;

        let mut pat = None;
        let mut ssh_keypair = None;
        let mut ssh_key_file = discover_local_ssh_key_files();

        if let Some(override_str) = auth_override.map(str::trim).filter(|s| !s.is_empty()) {
            let candidate_path = Path::new(override_str);
            if candidate_path.is_file() {
                let pub_candidate = PathBuf::from(format!("{override_str}.pub"));
                let pub_path = if pub_candidate.is_file() {
                    Some(pub_candidate)
                } else {
                    None
                };
                ssh_key_file = Some((candidate_path.to_path_buf(), pub_path));
                if let Some(mgr) = manager {
                    let _ = mgr
                        .import_ssh_key_file(&project.project.name, candidate_path, None)
                        .map(|rec| {
                            ssh_keypair = Some(rec);
                        });
                }
            } else if let Some(mgr) = manager {
                if let Ok(rec) = mgr.get_ssh_keypair(override_str) {
                    ssh_keypair = Some(rec);
                } else {
                    pat = Some(override_str.to_string());
                }
            } else {
                pat = Some(override_str.to_string());
            }
        }

        if pat.is_none() {
            pat = get_pat_from_keyring(&project.project.name)
                .ok()
                .or_else(|| {
                    manager.and_then(|mgr| {
                        mgr.get_pat_string(&format!(
                            "{}/{}",
                            project.project.name, project.git.remote
                        ))
                        .ok()
                        .or_else(|| mgr.get_pat_string(&project.project.name).ok())
                    })
                });
        }

        if ssh_keypair.is_none() {
            if let Some(mgr) = manager {
                let mut candidate_accounts = Vec::new();
                if let Some(ref acc) = project.git.commit_policy.ssh_key_account {
                    if !acc.trim().is_empty() {
                        candidate_accounts.push(acc.trim().to_string());
                    }
                }
                if let Some(ref acc) = project.git.ssh_key_account {
                    if !acc.trim().is_empty()
                        && !candidate_accounts.contains(&acc.trim().to_string())
                    {
                        candidate_accounts.push(acc.trim().to_string());
                    }
                }
                candidate_accounts.push(format!("{}/{}", project.project.name, project.git.remote));
                candidate_accounts.push(project.project.name.clone());

                for acc in candidate_accounts {
                    if let Ok(rec) = mgr.get_ssh_keypair(&acc) {
                        ssh_keypair = Some(rec);
                        break;
                    }
                }
            }
        }

        let ssh_auth_sock = std::env::var("SSH_AUTH_SOCK")
            .ok()
            .filter(|s| !s.trim().is_empty());

        Ok(GitAuthMaterial {
            pat,
            ssh_keypair,
            ssh_key_file,
            ssh_auth_sock,
        })
    }

    /// Stores a Personal Access Token (PAT) in the OS keyring and encrypted vault for this project.
    pub fn store_pat(&self, token: &str) -> Result<()> {
        let project = self.read_project()?;
        store_pat_in_keyring(&project.project.name, token)
    }

    /// Retrieves the Personal Access Token (PAT) from the OS keyring or encrypted vault for this project.
    pub fn get_pat(&self) -> Result<String> {
        let project = self.read_project()?;
        get_pat_from_keyring(&project.project.name)
    }

    /// Deletes the Personal Access Token (PAT) from the OS keyring and encrypted vault for this project.
    pub fn delete_pat(&self) -> Result<()> {
        let project = self.read_project()?;
        delete_pat_from_keyring(&project.project.name)
    }

    /// Fetches the configured remote and branch without modifying the working tree, returning
    /// the updated [`GitRepoStatus`] with `ahead_count` and `behind_count`.
    pub fn git_fetch(&self, auth_override: Option<&str>) -> Result<GitRepoStatus> {
        let project = self.read_project()?;
        let auth = self.resolve_git_auth(auth_override)?;
        fetch_repo_with_auth(
            self.project_root(),
            &project.git.remote,
            &project.git.branch,
            auth,
        )
    }

    /// Pushes the configured branch to the configured remote in `project.toml` `[git]`.
    ///
    /// Uses `auth_override` if provided, otherwise resolves HTTPS PAT or SSH credentials from
    /// [`SecureStorage`] / [`KeyPairManager`], `~/.ssh/id_ed25519`, or `SSH_AUTH_SOCK`.
    pub fn git_push(&self, auth_override: Option<&str>) -> Result<()> {
        let project = self.read_project()?;
        let auth = self.resolve_git_auth(auth_override)?;
        push_repo_with_auth(
            self.project_root(),
            &project.git.remote,
            &project.git.branch,
            auth,
        )
    }

    /// Pulls the configured branch from the configured remote in `project.toml` `[git]` and
    /// reconciles `.tracker/` indexes if new commits were pulled.
    pub fn git_pull(&self, auth_override: Option<&str>) -> Result<bool> {
        let project = self.read_project()?;
        let auth = self.resolve_git_auth(auth_override)?;
        let updated = pull_repo_with_auth(
            self.project_root(),
            &project.git.remote,
            &project.git.branch,
            auth,
        )?;
        if updated {
            let _ = self.reconcile_indexes()?;
        }
        Ok(updated)
    }

    /// Validates `input` against path rules and `[git.commit_policy]` without staging or
    /// committing anything. [`Self::git_commit_documented`] runs the same checks; callers that
    /// mutate tracker state before committing (e.g. `neb done -c`) call this first.
    pub fn validate_documented_commit(&self, input: &DocumentedCommitInput) -> Result<()> {
        self.prepare_documented_commit(input).map(|_| ())
    }

    fn prepare_documented_commit(&self, input: &DocumentedCommitInput) -> Result<PreparedCommit> {
        let project = self.read_project()?;
        let policy = &project.git.commit_policy;
        let root = self.project_root();
        let repo = Repository::discover(&root)?;
        let workdir = repo_workdir(&repo)?;
        let canonical_workdir = workdir.canonicalize()?;
        let prefix = project_prefix(&repo, &root)?;
        let head_tree = repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok())
            .and_then(|c| c.tree().ok());

        let mut workspace_paths: Vec<String> = Vec::with_capacity(input.paths.len());
        for raw_p in &input.paths {
            let trimmed = raw_p.trim();
            if trimmed.starts_with('/')
                || trimmed.starts_with('\\')
                || Path::new(trimmed).is_absolute()
                || trimmed.chars().nth(1) == Some(':')
            {
                return Err(TrackerError::ValidationError(format!(
                    "Unsafe path '{raw_p}' (absolute paths are not allowed; use project-relative paths)"
                )));
            }
            if contains_glob_metachar(trimmed) {
                return Err(TrackerError::ValidationError(format!(
                    "Unsafe path '{raw_p}' (glob metacharacters * ? [ ] {{ }} are not allowed; list explicit paths)"
                )));
            }
            let norm = normalize_relative_path_str(trimmed);
            if norm.is_empty() || norm == "." {
                return Err(TrackerError::ValidationError(format!(
                    "Unsafe path '{raw_p}' (the whole project root cannot be staged; list explicit paths)"
                )));
            }
            if norm.split('/').any(|seg| seg == "..") {
                return Err(TrackerError::ValidationError(format!(
                    "Unsafe relative path '{raw_p}' (parent traversal '..' is not allowed)"
                )));
            }
            if norm.split('/').any(|seg| seg == ".git") {
                return Err(TrackerError::ValidationError(format!(
                    "Unsafe path '{raw_p}' (.git internals cannot be staged)"
                )));
            }
            if is_tracker_relative_path(&norm) {
                // `.tracker/` is always staged; listing it is a no-op.
                continue;
            }

            let on_disk = root.join(&norm);
            let exists_on_disk = std::fs::symlink_metadata(&on_disk).is_ok();
            if exists_on_disk {
                let resolved_parent = on_disk
                    .parent()
                    .map(Path::canonicalize)
                    .transpose()?
                    .unwrap_or_else(|| canonical_workdir.clone());
                if !resolved_parent.starts_with(&canonical_workdir) {
                    return Err(TrackerError::ValidationError(format!(
                        "Unsafe path '{raw_p}' resolves outside the repository workdir"
                    )));
                }
            }
            let repo_rel = join_repo_path(&prefix, &norm);
            let exists_in_head = head_tree
                .as_ref()
                .is_some_and(|t| t.get_path(Path::new(&repo_rel)).is_ok());
            if !exists_on_disk && !exists_in_head {
                return Err(TrackerError::ValidationError(format!(
                    "Path '{norm}' does not exist in workspace or HEAD"
                )));
            }
            if !workspace_paths.contains(&norm) {
                workspace_paths.push(norm);
            }
        }

        if input.tracker_only && !workspace_paths.is_empty() {
            return Err(TrackerError::ValidationError(
                "Workspace paths were provided but tracker_only is true; set tracker_only=false to commit workspace files".to_string(),
            ));
        }
        if !input.tracker_only && workspace_paths.is_empty() {
            return Err(TrackerError::ValidationError(
                "Workspace commits require explicit `paths` (files or directories relative to the project root); use tracker_only=true to commit only .tracker/".to_string(),
            ));
        }

        let normalized_item_id = input
            .item_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let normalized_scope_override = input
            .scope_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let trimmed_summary = input.summary.trim().to_string();

        let is_pure_tracker_commit = normalized_item_id.is_none() && input.tracker_only;

        let loaded_item = if let Some(ref item_id) = normalized_item_id {
            Some(self.load_item_detail_by_id(item_id)?)
        } else {
            None
        };

        let resolved_scope = normalized_scope_override
            .or_else(|| loaded_item.as_ref().map(|(loc, _)| loc.scope_id.clone()));

        if !is_pure_tracker_commit {
            if policy.require_item_id && normalized_item_id.is_none() {
                return Err(TrackerError::ValidationError(
                    "Commit policy requires an item_id (e.g. BUG-0001 or FEAT-0001) when committing workspace code changes".to_string(),
                ));
            }

            if policy.require_scope && resolved_scope.is_none() {
                return Err(TrackerError::ValidationError(
                    "Commit policy requires a domain scope_id for code commits".to_string(),
                ));
            }

            if policy.require_summary && trimmed_summary.is_empty() {
                return Err(TrackerError::ValidationError(
                    "Commit policy requires a non-empty commit summary".to_string(),
                ));
            }

            if policy.min_summary_length > 0
                && (policy.require_summary || !trimmed_summary.is_empty())
                && trimmed_summary.chars().count() < policy.min_summary_length
            {
                return Err(TrackerError::ValidationError(format!(
                    "Commit summary must be at least {} characters (got {})",
                    policy.min_summary_length,
                    trimmed_summary.chars().count()
                )));
            }
        }

        let effective_summary = if trimmed_summary.is_empty() {
            "sync .tracker metadata state".to_string()
        } else {
            trimmed_summary
        };

        let header = match (&normalized_item_id, &resolved_scope) {
            (Some(item_id), Some(scope_id)) => {
                format!("[{item_id}]({scope_id}) {effective_summary}")
            }
            (Some(item_id), None) => format!("[{item_id}] {effective_summary}"),
            (None, _) if is_pure_tracker_commit => {
                let scope_tag = resolved_scope.as_deref().unwrap_or("sync");
                format!("[TRACKER]({scope_tag}) {effective_summary}")
            }
            (None, Some(scope_id)) => format!("[WORKSPACE]({scope_id}) {effective_summary}"),
            (None, None) => format!("[WORKSPACE] {effective_summary}"),
        };

        let commit_message = match input
            .body
            .as_deref()
            .map(str::trim)
            .filter(|b| !b.is_empty())
        {
            Some(body) => format!("{header}\n\n{body}"),
            None => header,
        };

        Ok(PreparedCommit {
            auto_append_to_desc: policy.auto_append_to_desc,
            prefix,
            workspace_paths,
            item_id: normalized_item_id,
            loaded_item,
            resolved_scope,
            effective_summary,
            commit_message,
        })
    }

    /// Stages `.tracker/` only (the default) OR `.tracker/` + explicitly listed workspace paths,
    /// validates `[git.commit_policy]`, formats a structured commit message, and (when an `item_id`
    /// is linked and `auto_append_to_desc` is enabled) automatically records the commit SHA, date,
    /// changed file list, and summary in `item.toml` (`history.previous_fixes`) and `desc.md`
    /// (`## Fix Log & Agent Notes`), re-staging `.tracker/` so the documentation and code commit
    /// remain atomic.
    ///
    /// The commit is built from a private in-memory index seeded from `HEAD`; the user's staging
    /// area is never used as a source, and afterwards only the committed paths are refreshed in it.
    pub fn git_commit_documented(
        &self,
        input: DocumentedCommitInput,
    ) -> Result<DocumentedCommitOutcome> {
        let prepared = self.prepare_documented_commit(&input)?;
        let root = self.project_root();
        let repo = Repository::discover(&root)?;
        let workdir = repo_workdir(&repo)?;
        let prefix = prepared.prefix.as_str();
        let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parent_tree = parent_commit.as_ref().and_then(|p| p.tree().ok());

        let (_stage_repo, mut index) = open_private_staging(&workdir, parent_tree.as_ref())?;
        stage_tracker_dir(&mut index, prefix)?;
        for p in &prepared.workspace_paths {
            stage_explicit_path(&mut index, &workdir, &join_repo_path(prefix, p))?;
        }

        let tree_id = index.write_tree_to(&repo)?;
        let tree = repo.find_tree(tree_id)?;

        if let Some(ref parent) = parent_commit {
            if parent.tree_id() == tree_id {
                return Err(TrackerError::ValidationError(
                    "No changes to commit in workspace or .tracker/".to_string(),
                ));
            }
        }

        let mut changed_files = changed_files_between(&repo, parent_tree.as_ref(), &tree)?;
        if changed_files.is_empty() {
            return Err(TrackerError::ValidationError(
                "No changes to commit in workspace or .tracker/".to_string(),
            ));
        }

        let sig = resolve_signature(&repo)?;
        let commit_message = prepared.commit_message;
        let initial_commit_oid = match parent_commit {
            Some(ref parent) => {
                repo.commit(Some("HEAD"), &sig, &sig, &commit_message, &tree, &[parent])?
            }
            None => repo.commit(Some("HEAD"), &sig, &sig, &commit_message, &tree, &[])?,
        };

        let recorded_fix_sha = short_sha_for_oid(&repo, initial_commit_oid);
        let mut final_commit_sha = recorded_fix_sha.clone();
        let mut final_tree = tree;
        let mut documentation_updated = false;
        let effective_summary = prepared.effective_summary;
        let resolved_scope = prepared.resolved_scope;

        if let Some((loc, mut detail)) = prepared.loaded_item {
            if prepared.auto_append_to_desc {
                let today = Utc::now().format("%Y-%m-%d").to_string();
                let workspace_or_changed: Vec<String> = changed_files
                    .iter()
                    .filter(|p| !is_tracker_relative_path(p))
                    .cloned()
                    .collect();
                let files_for_log = if workspace_or_changed.is_empty() {
                    changed_files.clone()
                } else {
                    workspace_or_changed
                };

                let fix_summary_with_files =
                    format!("{effective_summary} [files: {}]", files_for_log.join(", "));
                detail.manifest.history.previous_fixes.push(PreviousFix {
                    date: today.clone(),
                    commit: recorded_fix_sha.clone(),
                    summary: fix_summary_with_files,
                });
                detail.manifest.item.updated_at = now_rfc3339();

                let scope_label = resolved_scope.as_deref().unwrap_or(&loc.scope_id);
                detail.description_markdown = append_documented_fix_to_markdown(
                    &detail.description_markdown,
                    &recorded_fix_sha,
                    &today,
                    scope_label,
                    &effective_summary,
                    input.body.as_deref(),
                    &files_for_log,
                );

                self.write_item_detail(&loc.phase_id, &loc.scope_id, &detail)?;
                self.sync_parent_indexes_for_item(&loc.phase_id, &loc.scope_id, &detail.manifest)?;

                // Re-stage `.tracker/` into the private index and amend so documentation and
                // code changes remain atomic in a single commit.
                stage_tracker_dir(&mut index, prefix)?;
                let amended_tree_id = index.write_tree_to(&repo)?;
                let amended_tree = repo.find_tree(amended_tree_id)?;
                let initial_commit = repo.find_commit(initial_commit_oid)?;
                let amended_oid = initial_commit.amend(
                    Some("HEAD"),
                    None,
                    None,
                    None,
                    None,
                    Some(&amended_tree),
                )?;
                final_commit_sha = short_sha_for_oid(&repo, amended_oid);
                documentation_updated = true;

                changed_files = changed_files_between(&repo, parent_tree.as_ref(), &amended_tree)?;
                final_tree = amended_tree;
            }
        }

        // `git commit --only` semantics: refresh just the committed paths in the user's index.
        sync_user_index_paths(&repo, &final_tree, &changed_files)?;

        Ok(DocumentedCommitOutcome {
            commit_sha: final_commit_sha,
            recorded_fix_sha,
            message: commit_message,
            item_id: prepared.item_id,
            scope_id: resolved_scope,
            changed_files,
            documentation_updated,
        })
    }

    /// Stages and commits any dirty `.tracker/` changes, then pulls and pushes if the configured
    /// remote exists in the repository.
    pub fn git_sync(
        &self,
        message: Option<&str>,
        pat_override: Option<&str>,
    ) -> Result<Option<String>> {
        let committed_sha = self.git_commit_tracker(message)?;
        let project = self.read_project()?;
        let repo = Repository::discover(self.project_root())?;
        if repo.find_remote(&project.git.remote).is_ok() {
            let _ = self.git_pull(pat_override)?;
            self.git_push(pat_override)?;
        }
        Ok(committed_sha.or_else(|| head_short_sha_from_repo(&repo).ok()))
    }
}
