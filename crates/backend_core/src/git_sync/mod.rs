use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use git2::{
    build::CheckoutBuilder, Cred, CredentialType, FetchOptions, IndexAddOption, PushOptions,
    RemoteCallbacks, Repository, Signature, Status, StatusOptions,
};
use serde::{Deserialize, Serialize};

use crate::error::{Result, TrackerError};
use crate::storage::TrackerStorage;

/// OS keyring service identifier for Nebulous Personal Access Tokens (PATs).
pub const KEYRING_SERVICE: &str = "nebulous-tracker";

static FALLBACK_PAT_STORE: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn fallback_store() -> &'static Mutex<HashMap<String, String>> {
    FALLBACK_PAT_STORE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Summary of the Git repository state relevant to `.tracker/`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRepoStatus {
    pub head_short_sha: Option<String>,
    pub branch: Option<String>,
    pub tracker_dirty: bool,
    pub dirty_tracker_files: Vec<String>,
}

/// Stores a Personal Access Token (PAT) for `project_name` in the OS keyring.
pub fn store_pat_in_keyring(project_name: &str, token: &str) -> Result<()> {
    let account = normalize_keyring_account(project_name)?;
    let trimmed_token = token.trim();
    if trimmed_token.is_empty() {
        return Err(TrackerError::ValidationError(
            "Personal Access Token (PAT) must not be empty".to_string(),
        ));
    }

    let entry = keyring::Entry::new(KEYRING_SERVICE, &account)?;
    match entry.set_password(trimmed_token) {
        Ok(()) => {
            if let Ok(mut guard) = fallback_store().lock() {
                guard.insert(account, trimmed_token.to_string());
            }
            Ok(())
        }
        Err(
            keyring::Error::PlatformFailure(_)
            | keyring::Error::NoStorageAccess(_)
            | keyring::Error::NoEntry,
        ) => {
            let mut guard = fallback_store()
                .lock()
                .map_err(|e| TrackerError::KeyringError(e.to_string()))?;
            guard.insert(account, trimmed_token.to_string());
            Ok(())
        }
        Err(err) => Err(TrackerError::KeyringError(err.to_string())),
    }
}

/// Retrieves a Personal Access Token (PAT) for `project_name` from the OS keyring.
pub fn get_pat_from_keyring(project_name: &str) -> Result<String> {
    let account = normalize_keyring_account(project_name)?;
    let entry = keyring::Entry::new(KEYRING_SERVICE, &account)?;
    match entry.get_password() {
        Ok(password) => Ok(password),
        Err(keyring_err) => {
            if let Ok(guard) = fallback_store().lock() {
                if let Some(cached) = guard.get(&account) {
                    return Ok(cached.clone());
                }
            }
            Err(TrackerError::KeyringError(keyring_err.to_string()))
        }
    }
}

/// Deletes a stored Personal Access Token (PAT) for `project_name` from the OS keyring.
pub fn delete_pat_from_keyring(project_name: &str) -> Result<()> {
    let account = normalize_keyring_account(project_name)?;
    let had_fallback = fallback_store()
        .lock()
        .ok()
        .and_then(|mut guard| guard.remove(&account))
        .is_some();

    let entry = keyring::Entry::new(KEYRING_SERVICE, &account)?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(_) if had_fallback => Ok(()),
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
    if let Ok(buf) = commit.as_object().short_id() {
        if let Some(s) = buf.as_str() {
            return Ok(s.to_string());
        }
    }
    let full = commit.id().to_string();
    Ok(full.chars().take(7).collect())
}

/// Inspects the Git status of the repository containing `repo_path`, focusing on `.tracker/` changes.
pub fn repo_status(repo_path: impl AsRef<Path>) -> Result<GitRepoStatus> {
    let repo = Repository::discover(repo_path.as_ref())?;
    let head_short_sha = head_short_sha_from_repo(&repo).ok();
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(str::to_string));

    let mut opts = StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);

    let statuses = repo.statuses(Some(&mut opts))?;
    let mut dirty_tracker_files = Vec::new();

    for entry in statuses.iter() {
        let st = entry.status();
        if st == Status::CURRENT || st.contains(Status::IGNORED) {
            continue;
        }
        let Some(path) = entry.path() else {
            continue;
        };
        if is_tracker_relative_path(path) && !is_temp_atomic_path(path) {
            dirty_tracker_files.push(path.to_string());
        }
    }

    dirty_tracker_files.sort();
    dirty_tracker_files.dedup();

    Ok(GitRepoStatus {
        head_short_sha,
        branch,
        tracker_dirty: !dirty_tracker_files.is_empty(),
        dirty_tracker_files,
    })
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

/// Stages all modified, added, and deleted files under `.tracker/` and creates a Git commit.
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
    let mut index = repo.index()?;

    let mut filter_cb = |path: &Path, _matched_spec: &[u8]| -> i32 {
        if path.to_string_lossy().contains(".tmp.") {
            1
        } else {
            0
        }
    };
    index.add_all([".tracker"], IndexAddOption::DEFAULT, Some(&mut filter_cb))?;
    index.update_all([".tracker"], None)?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
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

    let commit_obj = repo.find_commit(commit_oid)?;
    let short_sha = commit_obj
        .as_object()
        .short_id()
        .ok()
        .and_then(|b| b.as_str().map(str::to_string))
        .unwrap_or_else(|| commit_oid.to_string().chars().take(7).collect());

    Ok(Some(short_sha))
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

/// Builds `git2::RemoteCallbacks` configured to supply a Personal Access Token (PAT) over HTTPS.
pub fn build_pat_callbacks<'a>(pat: Option<&'a str>) -> RemoteCallbacks<'a> {
    let mut callbacks = RemoteCallbacks::new();
    if let Some(token) = pat.map(str::trim).filter(|t| !t.is_empty()) {
        callbacks.credentials(move |_url, username_from_url, allowed_types| {
            if allowed_types.contains(CredentialType::USER_PASS_PLAINTEXT) {
                let user = username_from_url.unwrap_or("x-access-token");
                return Cred::userpass_plaintext(user, token);
            }
            if allowed_types.contains(CredentialType::USERNAME) {
                return Cred::username(username_from_url.unwrap_or("x-access-token"));
            }
            Cred::default()
        });
    }
    callbacks
}

/// Pushes `branch` to `remote_name` using optional HTTPS PAT authentication.
pub fn push_repo(
    repo_path: impl AsRef<Path>,
    remote_name: &str,
    branch: &str,
    pat: Option<&str>,
) -> Result<()> {
    let repo = Repository::discover(repo_path.as_ref())?;
    let mut remote = repo.find_remote(remote_name)?;
    let mut push_opts = PushOptions::new();
    push_opts.remote_callbacks(build_pat_callbacks(pat));

    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    remote.push(&[&refspec], Some(&mut push_opts))?;
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
    let repo = Repository::discover(repo_path.as_ref())?;
    let mut remote = repo.find_remote(remote_name)?;
    let mut fetch_opts = FetchOptions::new();
    fetch_opts.remote_callbacks(build_pat_callbacks(pat));

    remote.fetch(&[branch], Some(&mut fetch_opts), None)?;

    let fetch_head = match repo.find_reference("FETCH_HEAD") {
        Ok(r) => r,
        Err(_) => return Ok(false),
    };
    let fetch_commit = repo.reference_to_annotated_commit(&fetch_head)?;
    let (analysis, _) = repo.merge_analysis(&[&fetch_commit])?;

    if analysis.is_up_to_date() {
        return Ok(false);
    }

    let refname = format!("refs/heads/{branch}");
    if analysis.is_fast_forward() {
        match repo.find_reference(&refname) {
            Ok(mut r) => {
                r.set_target(fetch_commit.id(), "Fast-Forward pull")?;
            }
            Err(_) => {
                repo.reference(&refname, fetch_commit.id(), true, "Fast-Forward pull")?;
            }
        }
        repo.set_head(&refname)?;
        repo.checkout_head(Some(CheckoutBuilder::default().force()))?;
        return Ok(true);
    }

    if analysis.is_normal() {
        let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
        let local_commit = repo.find_commit(head_commit.id())?;
        let remote_commit = repo.find_commit(fetch_commit.id())?;

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

                if is_derived_tracker_index_path(&path_str) {
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
        let sig = resolve_signature(&repo)?;
        let msg = format!("Merge remote-tracking branch '{remote_name}/{branch}'");
        let merge_commit_oid = repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &msg,
            &tree,
            &[&local_commit, &remote_commit],
        )?;
        repo.checkout_head(Some(CheckoutBuilder::default().force()))?;

        if resolved_derived_index_conflict {
            if let Ok(storage) = TrackerStorage::discover(repo_path.as_ref()) {
                if storage.reconcile_indexes().is_ok() {
                    let mut repo_idx = repo.index()?;
                    let _ = repo_idx.read(true);
                    repo_idx.add_all([".tracker"], IndexAddOption::DEFAULT, None)?;
                    repo_idx.update_all([".tracker"], None)?;
                    repo_idx.write()?;
                    let reconciled_tree_id = repo_idx.write_tree()?;
                    let reconciled_tree = repo.find_tree(reconciled_tree_id)?;
                    let merge_commit = repo.find_commit(merge_commit_oid)?;
                    let _ = merge_commit.amend(
                        Some("HEAD"),
                        None,
                        None,
                        None,
                        None,
                        Some(&reconciled_tree),
                    );
                }
            }
        }

        return Ok(true);
    }

    Ok(false)
}

impl TrackerStorage {
    /// Returns the current Git `HEAD` short SHA for the workspace or `"uncommitted"` if unavailable.
    pub fn git_head_short_sha(&self) -> String {
        head_short_sha_or_uncommitted(self.project_root())
    }

    /// Returns the Git status for the repository containing `.tracker/`.
    pub fn git_status(&self) -> Result<GitRepoStatus> {
        repo_status(self.project_root())
    }

    /// Stages all `.tracker/` changes and creates a commit with `message`.
    pub fn git_commit_tracker(&self, message: Option<&str>) -> Result<Option<String>> {
        stage_and_commit_tracker(self.project_root(), message)
    }

    /// Stores a Personal Access Token (PAT) in the OS keyring for this project.
    pub fn store_pat(&self, token: &str) -> Result<()> {
        let project = self.read_project()?;
        store_pat_in_keyring(&project.project.name, token)
    }

    /// Retrieves the Personal Access Token (PAT) from the OS keyring for this project.
    pub fn get_pat(&self) -> Result<String> {
        let project = self.read_project()?;
        get_pat_from_keyring(&project.project.name)
    }

    /// Deletes the Personal Access Token (PAT) from the OS keyring for this project.
    pub fn delete_pat(&self) -> Result<()> {
        let project = self.read_project()?;
        delete_pat_from_keyring(&project.project.name)
    }

    /// Pushes the configured branch to the configured remote in `project.toml` `[git]`.
    ///
    /// Uses `pat_override` if provided, otherwise checks the OS keyring for a stored PAT.
    pub fn git_push(&self, pat_override: Option<&str>) -> Result<()> {
        let project = self.read_project()?;
        let stored_pat = if pat_override.is_none() {
            get_pat_from_keyring(&project.project.name).ok()
        } else {
            None
        };
        let effective_pat = pat_override.or(stored_pat.as_deref());
        push_repo(
            self.project_root(),
            &project.git.remote,
            &project.git.branch,
            effective_pat,
        )
    }

    /// Pulls the configured branch from the configured remote in `project.toml` `[git]` and
    /// reconciles `.tracker/` indexes if new commits were pulled.
    pub fn git_pull(&self, pat_override: Option<&str>) -> Result<bool> {
        let project = self.read_project()?;
        let stored_pat = if pat_override.is_none() {
            get_pat_from_keyring(&project.project.name).ok()
        } else {
            None
        };
        let effective_pat = pat_override.or(stored_pat.as_deref());
        let updated = pull_repo(
            self.project_root(),
            &project.git.remote,
            &project.git.branch,
            effective_pat,
        )?;
        if updated {
            let _ = self.reconcile_indexes()?;
        }
        Ok(updated)
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
