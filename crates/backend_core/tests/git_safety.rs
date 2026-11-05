//! STEP-10 git safety regressions: scoped staging, isolated index, HEAD-branch pull/push,
//! safe checkout, and nested project roots.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use git2::{Repository, Signature, Status};
use tempfile::TempDir;

use backend_core::{DocumentedCommitInput, GitAuthRequest, Severity, TrackerError, TrackerStorage};

fn init_tracker(root: &Path) -> TrackerStorage {
    let storage = TrackerStorage::init(root, "GitSafety", "01_core_engine", "Core Engine")
        .expect("should init tracker");
    storage
        .create_scope(
            Some("01_core_engine"),
            "backend_core",
            "crates/backend_core",
            vec!["rust".to_string()],
            None,
        )
        .expect("should create scope");
    storage
}

fn init_repo(path: &Path) -> Repository {
    let repo = Repository::init(path).expect("should init repo");
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Safety Tester").unwrap();
        cfg.set_str("user.email", "safety@nebulous.local").unwrap();
    }
    repo
}

/// Commits the given repo-relative paths with plain git2 (simulating the user's own `git commit`).
fn user_commit(repo: &Repository, paths: &[&str], message: &str) {
    let mut idx = repo.index().unwrap();
    idx.read(true).unwrap();
    for p in paths {
        idx.add_path(Path::new(p)).unwrap();
    }
    idx.write().unwrap();
    let tree = repo.find_tree(idx.write_tree().unwrap()).unwrap();
    let sig = Signature::now("Safety Tester", "safety@nebulous.local").unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .unwrap();
}

fn head_branch(repo: &Repository) -> String {
    repo.head().unwrap().shorthand().unwrap().to_string()
}

fn all_refs(repo: &Repository) -> BTreeMap<String, String> {
    repo.references()
        .unwrap()
        .filter_map(|r| r.ok())
        .filter_map(|r| {
            let name = r.name().ok()?.to_string();
            let target = r
                .target()
                .map(|o| o.to_string())
                .or_else(|| r.symbolic_target().ok().flatten().map(str::to_string))?;
            Some((name, target))
        })
        .collect()
}

struct RemotePair {
    _ws1_dir: TempDir,
    _bare_dir: TempDir,
    ws2_dir: TempDir,
    storage1: TrackerStorage,
    storage2: TrackerStorage,
    repo2: Repository,
    branch: String,
}

/// Workspace 1 with a pushed `.tracker/` containing BUG-0001, and workspace 2 cloned from the
/// bare remote and checked out on the same (configured) branch.
fn setup_remote_pair() -> RemotePair {
    let ws1_dir = TempDir::new().unwrap();
    let storage1 = init_tracker(ws1_dir.path());
    let repo1 = init_repo(ws1_dir.path());
    storage1
        .create_bug(
            "Shared bug",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::Medium,
            None,
        )
        .unwrap();
    storage1
        .git_commit_tracker(Some("chore: initial tracker"))
        .unwrap()
        .expect("initial tracker commit");
    let branch = head_branch(&repo1);

    let bare_dir = TempDir::new().unwrap();
    Repository::init_bare(bare_dir.path()).unwrap();
    repo1
        .remote("origin", bare_dir.path().to_str().unwrap())
        .unwrap();
    let mut project = storage1.read_project().unwrap();
    project.git.remote = "origin".to_string();
    project.git.branch = branch.clone();
    storage1.write_project(&project).unwrap();
    storage1
        .git_commit_tracker(Some("chore: configure remote"))
        .unwrap();
    storage1
        .git_push(&GitAuthRequest::default())
        .expect("initial push");

    let ws2_dir = TempDir::new().unwrap();
    let repo2 = Repository::clone(bare_dir.path().to_str().unwrap(), ws2_dir.path()).unwrap();
    {
        let mut cfg = repo2.config().unwrap();
        cfg.set_str("user.name", "Clone Tester").unwrap();
        cfg.set_str("user.email", "clone@nebulous.local").unwrap();
        let head_commit = repo2.head().unwrap().peel_to_commit().unwrap();
        let _ = repo2.branch(&branch, &head_commit, true);
        repo2.set_head(&format!("refs/heads/{branch}")).unwrap();
    }
    let storage2 = TrackerStorage::new(ws2_dir.path());

    RemotePair {
        _ws1_dir: ws1_dir,
        _bare_dir: bare_dir,
        ws2_dir,
        storage1,
        storage2,
        repo2,
        branch,
    }
}

fn bug_desc_rel() -> &'static str {
    ".tracker/phases/01_core_engine/domains/backend_core/items/BUG-0001/desc.md"
}

#[test]
fn pre_staged_hunk_survives_git_commit_documented() {
    let dir = TempDir::new().unwrap();
    let storage = init_tracker(dir.path());
    let repo = init_repo(dir.path());

    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/lib.rs"), "pub fn a() {}\n").unwrap();
    user_commit(&repo, &["src/lib.rs"], "initial lib");
    storage.git_commit_tracker(Some("chore: tracker")).unwrap();

    // User stages a hunk in src/lib.rs, then keeps editing the worktree.
    fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn a() {}\npub fn staged() {}\n",
    )
    .unwrap();
    {
        let mut idx = repo.index().unwrap();
        idx.read(true).unwrap();
        idx.add_path(Path::new("src/lib.rs")).unwrap();
        idx.write().unwrap();
    }
    fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn a() {}\npub fn staged() {}\npub fn unstaged() {}\n",
    )
    .unwrap();
    let staged_id = {
        let mut idx = repo.index().unwrap();
        idx.read(true).unwrap();
        idx.get_path(Path::new("src/lib.rs"), 0).unwrap().id
    };
    let head_lib_id = repo
        .head()
        .unwrap()
        .peel_to_tree()
        .unwrap()
        .get_path(Path::new("src/lib.rs"))
        .unwrap()
        .id();

    storage
        .create_bug(
            "Tracker change",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::Low,
            None,
        )
        .unwrap();
    let outcome = storage
        .git_commit_documented(DocumentedCommitInput::tracker_only("record new bug"))
        .expect("tracker-only commit should succeed");
    assert!(outcome
        .changed_files
        .iter()
        .all(|p| p.starts_with(".tracker/")));

    // The commit did not include the staged hunk...
    let head_tree = repo.head().unwrap().peel_to_tree().unwrap();
    assert_eq!(
        head_tree.get_path(Path::new("src/lib.rs")).unwrap().id(),
        head_lib_id
    );
    // ...and the user's staged entry is byte-identical.
    let mut idx = repo.index().unwrap();
    idx.read(true).unwrap();
    assert_eq!(
        idx.get_path(Path::new("src/lib.rs"), 0).unwrap().id,
        staged_id
    );
    let st = repo.status_file(Path::new("src/lib.rs")).unwrap();
    assert!(st.contains(Status::INDEX_MODIFIED) && st.contains(Status::WT_MODIFIED));
    // Committed `.tracker/` paths are refreshed in the index (no phantom staged reversions).
    assert!(!storage.git_status().unwrap().tracker_dirty);
}

#[test]
fn workspace_commits_require_explicit_non_glob_paths_and_skip_untracked() {
    let dir = TempDir::new().unwrap();
    let storage = init_tracker(dir.path());
    let repo = init_repo(dir.path());
    storage
        .create_bug(
            "Bug",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::Low,
            None,
        )
        .unwrap();
    storage.git_commit_tracker(None).unwrap();

    fs::create_dir_all(dir.path().join("src")).unwrap();
    fs::write(dir.path().join("src/fix.rs"), "pub fn fix() {}\n").unwrap();
    fs::write(dir.path().join("secret.env"), "TOKEN=hunter2\n").unwrap();

    // Default input is tracker-only.
    assert!(DocumentedCommitInput::default().tracker_only);
    let parsed: DocumentedCommitInput =
        serde_json::from_str(r#"{"summary":"x"}"#).expect("deserializes");
    assert!(parsed.tracker_only);

    let glob_err = storage
        .git_commit_documented(
            DocumentedCommitInput::for_item("BUG-0001", "Glob paths must be rejected")
                .with_paths(["src/*.rs"]),
        )
        .expect_err("glob path rejected");
    assert!(matches!(glob_err, TrackerError::ValidationError(ref m) if m.contains("glob")));

    let abs_err = storage
        .git_commit_documented(
            DocumentedCommitInput::for_item("BUG-0001", "Absolute paths must be rejected")
                .with_paths(["/etc/passwd"]),
        )
        .expect_err("absolute path rejected");
    assert!(matches!(abs_err, TrackerError::ValidationError(_)));

    let no_paths_err = storage
        .git_commit_documented(
            DocumentedCommitInput::for_item("BUG-0001", "Workspace commit without paths")
                .with_tracker_only(false),
        )
        .expect_err("workspace commit without paths rejected");
    assert!(matches!(no_paths_err, TrackerError::ValidationError(ref m) if m.contains("paths")));

    // Listing a directory stages tracked changes only; listing the file stages it.
    let outcome = storage
        .git_commit_documented(
            DocumentedCommitInput::for_item("BUG-0001", "Add explicit fix module")
                .with_paths(["src/fix.rs"]),
        )
        .expect("explicit path commit");
    assert!(outcome.changed_files.contains(&"src/fix.rs".to_string()));
    let head_tree = repo.head().unwrap().peel_to_tree().unwrap();
    assert!(head_tree.get_path(Path::new("secret.env")).is_err());
    assert_eq!(
        storage.git_status().unwrap().dirty_workspace_files,
        vec!["secret.env".to_string()]
    );
}

#[test]
fn pull_and_push_on_non_configured_branch_error_and_leave_refs_unchanged() {
    let pair = setup_remote_pair();

    // Remote advances so a pull would have something to fast-forward.
    pair.storage1
        .create_bug(
            "Upstream bug",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::Low,
            None,
        )
        .unwrap();
    pair.storage1.git_commit_tracker(None).unwrap();
    pair.storage1.git_push(&GitAuthRequest::default()).unwrap();

    // Workspace 2 switches to a feature branch.
    let head_commit = pair.repo2.head().unwrap().peel_to_commit().unwrap();
    pair.repo2.branch("feature", &head_commit, false).unwrap();
    pair.repo2.set_head("refs/heads/feature").unwrap();

    let refs_before = all_refs(&pair.repo2);
    let fetch_head = pair.repo2.path().join("FETCH_HEAD");
    let fetch_head_before = fs::read(&fetch_head).ok();

    let err = pair
        .storage2
        .git_pull(&GitAuthRequest::default())
        .expect_err("pull on non-configured branch must fail");
    assert!(
        matches!(err, TrackerError::GitError(ref m) if m.contains("feature") && m.contains(&pair.branch)),
        "error should name both branches: {err:?}"
    );
    let push_err = pair
        .storage2
        .git_push(&GitAuthRequest::default())
        .expect_err("push on non-configured branch must fail");
    assert!(matches!(push_err, TrackerError::GitError(ref m) if m.contains("feature")));

    assert_eq!(all_refs(&pair.repo2), refs_before);
    assert_eq!(fs::read(&fetch_head).ok(), fetch_head_before);
    assert_eq!(head_branch(&pair.repo2), "feature");
    assert!(pair.storage2.load_item_detail_by_id("BUG-0002").is_err());
}

#[test]
fn dirty_tracker_edit_survives_merge_attempt() {
    let pair = setup_remote_pair();
    let desc_rel = bug_desc_rel();

    // Remote edits BUG-0001's desc.md.
    let ws1_desc = pair.storage1.project_root().join(desc_rel);
    let mut remote_text = fs::read_to_string(&ws1_desc).unwrap();
    remote_text.push_str("\nRemote note.\n");
    fs::write(&ws1_desc, &remote_text).unwrap();
    pair.storage1
        .git_commit_tracker(Some("docs: remote note"))
        .unwrap()
        .expect("remote commit");
    pair.storage1.git_push(&GitAuthRequest::default()).unwrap();

    // Workspace 2 diverges with an unrelated commit (forces the normal-merge path)...
    fs::write(pair.ws2_dir.path().join("README.md"), "# ws2\n").unwrap();
    user_commit(&pair.repo2, &["README.md"], "docs: readme");
    // ...and has an uncommitted edit to the same tracker file.
    let ws2_desc = pair.ws2_dir.path().join(desc_rel);
    let local_text = format!(
        "{}\nLocal uncommitted note.\n",
        fs::read_to_string(&ws2_desc).unwrap()
    );
    fs::write(&ws2_desc, &local_text).unwrap();

    let head_before = pair.repo2.head().unwrap().target().unwrap();
    let err = pair
        .storage2
        .git_pull(&GitAuthRequest::default())
        .expect_err("pull must refuse to overwrite dirty .tracker edit");
    assert!(matches!(err, TrackerError::GitError(ref m) if m.contains("desc.md")));
    assert_eq!(fs::read_to_string(&ws2_desc).unwrap(), local_text);
    assert_eq!(pair.repo2.head().unwrap().target().unwrap(), head_before);
}

#[test]
fn nested_project_root_commit_stages_tracker() {
    let repo_dir = TempDir::new().unwrap();
    let repo = init_repo(repo_dir.path());
    let nested = repo_dir.path().join("apps/service");
    fs::create_dir_all(&nested).unwrap();
    let storage = init_tracker(&nested);
    storage
        .create_bug(
            "Nested bug",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::Low,
            None,
        )
        .unwrap();

    let sha = storage
        .git_commit_tracker(Some("chore: nested tracker"))
        .unwrap();
    assert!(sha.is_some(), "nested .tracker must be committed");
    let head_tree = repo.head().unwrap().peel_to_tree().unwrap();
    assert!(head_tree
        .get_path(Path::new("apps/service/.tracker/project.toml"))
        .is_ok());
    assert!(!storage.git_status().unwrap().tracker_dirty);

    // Documented commit with a project-relative workspace path lands at the nested repo path.
    fs::create_dir_all(nested.join("src")).unwrap();
    fs::write(nested.join("src/main.rs"), "fn main() {}\n").unwrap();
    let outcome = storage
        .git_commit_documented(
            DocumentedCommitInput::for_item("BUG-0001", "Add nested service entrypoint")
                .with_paths(["src/main.rs"]),
        )
        .expect("nested documented commit");
    assert!(outcome.documentation_updated);
    let head_tree = repo.head().unwrap().peel_to_tree().unwrap();
    assert!(head_tree
        .get_path(Path::new("apps/service/src/main.rs"))
        .is_ok());
    assert!(head_tree
        .get_path(Path::new(
            "apps/service/.tracker/phases/01_core_engine/domains/backend_core/items/BUG-0001/item.toml"
        ))
        .is_ok());
    let status = storage.git_status().unwrap();
    assert!(
        !status.tracker_dirty && !status.workspace_dirty,
        "{status:?}"
    );
}
