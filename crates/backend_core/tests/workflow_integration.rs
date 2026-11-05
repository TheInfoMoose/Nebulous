use std::fs;
use std::path::Path;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use git2::{Repository, Signature};
use tempfile::TempDir;

use backend_core::{
    build_auth_callbacks, build_pat_callbacks, CommitPolicyConfig, CreateItemInput,
    DocumentedCommitInput, ItemDetail, ItemEffort, ItemHistory, ItemManifest, ItemMetadata,
    ItemStatus, ItemType, NextActionKind, PreviousFix, Severity, TrackerError, TrackerStorage,
};

fn init_temp_tracker() -> (TempDir, TrackerStorage) {
    let dir = TempDir::new().expect("should create tempdir");
    let storage = TrackerStorage::init(
        dir.path(),
        "NebulousTestProject",
        "01_core_engine",
        "Core Engine & Persistence",
    )
    .expect("should init TrackerStorage");

    storage
        .create_scope(
            Some("01_core_engine"),
            "backend_core",
            "crates/backend_core",
            vec!["rust".to_string(), "serde".to_string(), "git2".to_string()],
            Some("Core Rust persistence and workflow crate".to_string()),
        )
        .expect("should create backend_core scope");

    (dir, storage)
}

#[test]
fn test_item_creation_boilerplate_and_atomic_parent_index_updates() {
    let (_dir, storage) = init_temp_tracker();

    // 1. Create bug with omitted scope -> falls back to default_scope ("inbox")
    let (loc_inbox, detail_inbox) = storage
        .create_bug(
            "Unscoped crash on startup",
            None,
            None,
            Severity::High,
            None,
        )
        .expect("should create bug in default inbox scope");

    assert_eq!(loc_inbox.phase_id, "01_core_engine");
    assert_eq!(loc_inbox.scope_id, "inbox");
    assert_eq!(detail_inbox.manifest.item.id, "BUG-0001");
    assert_eq!(detail_inbox.manifest.item.status, ItemStatus::Open);
    assert_eq!(detail_inbox.manifest.item.severity, Severity::High);
    assert_eq!(detail_inbox.manifest.effort.logged_time_minutes, 0);
    assert_eq!(detail_inbox.manifest.effort.iteration_count, 1);
    assert_eq!(detail_inbox.manifest.history.recurrence_count, 0);
    assert!(detail_inbox
        .description_markdown
        .contains("## Problem Description"));
    assert!(detail_inbox
        .description_markdown
        .contains("## Reproduction Steps"));
    assert!(detail_inbox
        .description_markdown
        .contains("## Fix Log & Agent Notes"));

    // 2. Create feature in explicit `backend_core` scope with custom description
    let (loc_feat, detail_feat) = storage
        .create_feature(
            "Bounded agent context exporter",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::Critical,
            Some(
                "Synthesize @file refs and bounded prompt from scope.toml, item.toml, and desc.md.",
            ),
        )
        .expect("should create feature in backend_core");

    assert_eq!(loc_feat.scope_id, "backend_core");
    assert_eq!(detail_feat.manifest.item.id, "FEAT-0001");
    assert!(detail_feat
        .description_markdown
        .contains("Synthesize @file refs and bounded prompt"));
    assert!(detail_feat
        .description_markdown
        .contains("crates/backend_core"));

    // 3. Verify parent `scope.toml` and `phase.toml` were updated atomically
    let inbox_scope = storage.read_scope("01_core_engine", "inbox").unwrap();
    assert_eq!(inbox_scope.items.bugs.len(), 1);
    assert_eq!(inbox_scope.items.bugs[0].id, "BUG-0001");

    let backend_scope = storage
        .read_scope("01_core_engine", "backend_core")
        .unwrap();
    assert_eq!(backend_scope.items.features.len(), 1);
    assert_eq!(backend_scope.items.features[0].id, "FEAT-0001");

    let phase = storage.read_phase("01_core_engine").unwrap();
    assert_eq!(phase.summary.total_items, 2);
    assert_eq!(phase.summary.completed_items, 0);

    // 4. Empty title must fail validation
    assert!(matches!(
        storage.create_bug("   ", None, None, Severity::Low, None),
        Err(TrackerError::ValidationError(_))
    ));
}

#[test]
fn test_close_captures_git_head_or_uncommitted_and_reopen_increments_recurrence() {
    let (dir, storage) = init_temp_tracker();

    let (_, bug) = storage
        .create_bug(
            "Empty array deserialization failure",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::High,
            Some("Parser fails when bugs = []"),
        )
        .unwrap();
    assert_eq!(bug.manifest.item.id, "BUG-0001");

    // Non-completed transitions (`open` -> `in_progress` -> `pending`) must NOT increment recurrence_count
    let in_prog = storage
        .update_item_status("BUG-0001", ItemStatus::InProgress)
        .unwrap();
    assert_eq!(in_prog.manifest.item.status, ItemStatus::InProgress);
    assert_eq!(in_prog.manifest.history.recurrence_count, 0);
    assert_eq!(in_prog.manifest.effort.iteration_count, 1);

    // Reopening an item that is NOT closed/merged must return InvalidTransition
    assert!(matches!(
        storage.reopen_item("BUG-0001", Some("Still active")),
        Err(TrackerError::InvalidTransition { .. })
    ));

    // Close #1: Before initializing a Git repository, commit should fallback to `"uncommitted"`
    let closed_1 = storage
        .close_item(
            "BUG-0001",
            "Added #[serde(default)] to ScopeItems.bugs",
            None,
            false,
        )
        .expect("should close BUG-0001 with uncommitted fallback");
    assert_eq!(closed_1.manifest.item.status, ItemStatus::Closed);
    assert_eq!(closed_1.manifest.history.previous_fixes.len(), 1);
    assert_eq!(
        closed_1.manifest.history.previous_fixes[0].commit,
        "uncommitted"
    );
    assert_eq!(
        closed_1.manifest.history.previous_fixes[0].summary,
        "Added #[serde(default)] to ScopeItems.bugs"
    );

    // Verify parent `phase.toml` and `scope.toml` reflect completion atomically
    let phase_after_close1 = storage.read_phase("01_core_engine").unwrap();
    assert_eq!(phase_after_close1.summary.total_items, 1);
    assert_eq!(phase_after_close1.summary.completed_items, 1);
    let scope_after_close1 = storage
        .read_scope("01_core_engine", "backend_core")
        .unwrap();
    assert_eq!(scope_after_close1.items.bugs[0].status, ItemStatus::Closed);

    // Reopen #1 via `reopen_item`: must increment `recurrence_count` (0 -> 1) and `iteration_count` (1 -> 2)
    let reopened_1 = storage
        .reopen_item(
            "BUG-0001",
            Some("Still fails when features = [] is omitted"),
        )
        .expect("should reopen BUG-0001");
    assert_eq!(reopened_1.manifest.item.status, ItemStatus::Open);
    assert_eq!(reopened_1.manifest.history.recurrence_count, 1);
    assert_eq!(reopened_1.manifest.effort.iteration_count, 2);
    assert!(reopened_1
        .description_markdown
        .contains("Still fails when features = [] is omitted"));

    let phase_after_reopen1 = storage.read_phase("01_core_engine").unwrap();
    assert_eq!(phase_after_reopen1.summary.completed_items, 0);

    // Initialize a real `git2` repository in `dir.path()` and create an initial commit
    let _repo = Repository::init(dir.path()).expect("should init git2 repo");
    let commit_sha = storage
        .git_commit_tracker(Some("chore: initial tracker snapshot"))
        .expect("git_commit_tracker should succeed")
        .expect("should produce a commit SHA");
    assert!(commit_sha.len() >= 7);
    assert_eq!(storage.git_head_short_sha(), commit_sha);

    // Close #2 (as Merged) inside the Git repo: must auto-capture `commit_sha` from `git2` HEAD
    let closed_2 = storage
        .close_item(
            "BUG-0001",
            "Added #[serde(default)] on both bugs and features",
            None,
            true,
        )
        .expect("should merge BUG-0001 with git2 HEAD short SHA");
    assert_eq!(closed_2.manifest.item.status, ItemStatus::Merged);
    assert_eq!(closed_2.manifest.history.previous_fixes.len(), 2);
    assert_eq!(
        closed_2.manifest.history.previous_fixes[1].commit,
        commit_sha
    );
    assert_eq!(
        storage
            .read_phase("01_core_engine")
            .unwrap()
            .summary
            .completed_items,
        1
    );

    // Reopen #2 via `update_item_status(..., ItemStatus::InProgress)`: must ALSO increment
    // `recurrence_count` (1 -> 2) and `iteration_count` (2 -> 3)
    let reopened_2 = storage
        .update_item_status("BUG-0001", ItemStatus::InProgress)
        .expect("update_item_status from merged to in_progress should trigger reopen counters");
    assert_eq!(reopened_2.manifest.item.status, ItemStatus::InProgress);
    assert_eq!(reopened_2.manifest.history.recurrence_count, 2);
    assert_eq!(reopened_2.manifest.effort.iteration_count, 3);
    assert_eq!(
        storage
            .read_phase("01_core_engine")
            .unwrap()
            .summary
            .completed_items,
        0
    );

    // Empty summary on close_item must fail validation
    assert!(matches!(
        storage.close_item("BUG-0001", "   ", None, false),
        Err(TrackerError::ValidationError(_))
    ));

    // Closing via `update_item_status(..., ItemStatus::Closed)` must also append a `PreviousFix` entry
    let closed_via_status = storage
        .update_item_status("BUG-0001", ItemStatus::Closed)
        .expect("closing via update_item_status should append PreviousFix");
    assert_eq!(closed_via_status.manifest.item.status, ItemStatus::Closed);
    assert_eq!(closed_via_status.manifest.history.previous_fixes.len(), 3);
    assert_eq!(
        closed_via_status.manifest.history.previous_fixes[2].commit,
        commit_sha
    );
}

#[test]
fn test_active_timer_and_manual_effort_logging() {
    let (_dir, storage) = init_temp_tracker();

    storage
        .create_bug(
            "Timer test item",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::Medium,
            None,
        )
        .unwrap();

    // No timer initially
    assert!(storage.timer_status().unwrap().is_none());
    assert!(matches!(
        storage.stop_timer(),
        Err(TrackerError::NoActiveTimer)
    ));

    // Start timer at t0
    let t0 = Utc.with_ymd_and_hms(2026, 11, 2, 14, 0, 0).unwrap();
    let active = storage
        .start_timer_at("BUG-0001", t0, Some("Investigating parser".to_string()))
        .expect("should start timer");
    assert_eq!(active.item_id, "BUG-0001");
    assert_eq!(active.note.as_deref(), Some("Investigating parser"));
    assert!(storage.timer_toml_path().is_file());

    // Starting another timer while one is active must return TimerAlreadyRunning
    assert!(matches!(
        storage.start_timer_at("BUG-0001", t0, None),
        Err(TrackerError::TimerAlreadyRunning { ref item_id }) if item_id == "BUG-0001"
    ));

    // Check status at t0 + 35 minutes 20 seconds
    let t1 = Utc.with_ymd_and_hms(2026, 11, 2, 14, 35, 20).unwrap();
    let status = storage
        .timer_status_at(t1)
        .unwrap()
        .expect("timer should be active");
    assert_eq!(status.elapsed_minutes, 35);
    assert_eq!(status.elapsed_seconds, 35 * 60 + 20);

    // Stop timer at t0 + 50 minutes
    let t2 = Utc.with_ymd_and_hms(2026, 11, 2, 14, 50, 0).unwrap();
    let stopped = storage
        .stop_timer_at(t2)
        .expect("should stop timer and accumulate minutes");
    assert_eq!(stopped.elapsed_minutes, 50);
    assert_eq!(stopped.total_logged_minutes, 50);
    assert!(!storage.timer_toml_path().exists());

    // Add manual effort (25 minutes) -> total should be 75 minutes
    let updated = storage
        .log_effort("BUG-0001", 25)
        .expect("should log manual effort");
    assert_eq!(updated.manifest.effort.logged_time_minutes, 75);
}

#[test]
fn test_metrics_engine_valuation_stagnant_items_and_recurring_hotspots() {
    let (_dir, storage) = init_temp_tracker();

    // Configure target_daily_hours = 6.0 and stagnant_item_threshold_days = 4
    let now = Utc.with_ymd_and_hms(2026, 11, 10, 12, 0, 0).unwrap();

    // Item 1: Open, updated 6 days ago (STAGNANT, 120 mins)
    let bug1 = ItemDetail {
        manifest: ItemManifest {
            item: ItemMetadata {
                id: "BUG-0001".to_string(),
                item_type: ItemType::Bug,
                title: "Stagnant high-severity bug".to_string(),
                status: ItemStatus::Open,
                severity: Severity::High,
                scope_ref: "backend_core".to_string(),
                created_at: "2026-11-01T12:00:00Z".to_string(),
                updated_at: "2026-11-04T12:00:00Z".to_string(), // 6 days before `now`
            },
            effort: ItemEffort {
                logged_time_minutes: 120,
                iteration_count: 3,
            },
            history: ItemHistory {
                recurrence_count: 2,
                previous_fixes: vec![
                    PreviousFix {
                        date: "2026-11-02".to_string(),
                        commit: "111aaaa".to_string(),
                        summary: "First incomplete attempt".to_string(),
                    },
                    PreviousFix {
                        date: "2026-11-03".to_string(),
                        commit: "222bbbb".to_string(),
                        summary: "Second attempt".to_string(),
                    },
                ],
            },
        },
        description_markdown: "## Problem Description\nBug 1".to_string(),
    };

    // Item 2: InProgress in inbox, updated 4 days ago (exactly at threshold = 4 days -> STAGNANT, 60 mins)
    let feat1 = ItemDetail {
        manifest: ItemManifest {
            item: ItemMetadata {
                id: "FEAT-0001".to_string(),
                item_type: ItemType::Feature,
                title: "Threshold boundary feature".to_string(),
                status: ItemStatus::InProgress,
                severity: Severity::Medium,
                scope_ref: "inbox".to_string(),
                created_at: "2026-11-01T12:00:00Z".to_string(),
                updated_at: "2026-11-06T12:00:00Z".to_string(), // 4 days before `now`
            },
            effort: ItemEffort {
                logged_time_minutes: 60,
                iteration_count: 1,
            },
            history: ItemHistory::default(),
        },
        description_markdown: "## Problem Description\nFeat 1".to_string(),
    };

    // Item 3: Closed, updated 9 days ago (must NOT be flagged stagnant because it is closed! 180 mins)
    let bug2 = ItemDetail {
        manifest: ItemManifest {
            item: ItemMetadata {
                id: "BUG-0002".to_string(),
                item_type: ItemType::Bug,
                title: "Old closed bug".to_string(),
                status: ItemStatus::Closed,
                severity: Severity::Critical,
                scope_ref: "backend_core".to_string(),
                created_at: "2026-11-01T00:00:00Z".to_string(),
                updated_at: "2026-11-01T12:00:00Z".to_string(), // 9 days ago, but closed
            },
            effort: ItemEffort {
                logged_time_minutes: 180,
                iteration_count: 1,
            },
            history: ItemHistory {
                recurrence_count: 0,
                previous_fixes: vec![PreviousFix {
                    date: "2026-11-01".to_string(),
                    commit: "333cccc".to_string(),
                    summary: "Clean fix".to_string(),
                }],
            },
        },
        description_markdown: "## Problem Description\nBug 2".to_string(),
    };

    storage
        .write_item_detail("01_core_engine", "backend_core", &bug1)
        .unwrap();
    storage
        .write_item_detail("01_core_engine", "inbox", &feat1)
        .unwrap();
    storage
        .write_item_detail("01_core_engine", "backend_core", &bug2)
        .unwrap();
    storage.reconcile_indexes().unwrap();

    let metrics = storage
        .compute_metrics_at(now)
        .expect("should compute metrics at `now`");

    // Total logged minutes = 120 + 60 + 180 = 360 mins = 6.0 hours = 1.0 target day (at 6.0h/day)
    assert_eq!(metrics.total_logged_minutes, 360);
    assert!((metrics.total_logged_hours - 6.0).abs() < 1e-6);
    assert!((metrics.target_days_equivalent - 1.0).abs() < 1e-6);
    assert_eq!(metrics.total_items, 3);
    assert_eq!(metrics.completed_items, 1);

    // Stagnant items: BUG-0001 (6 days) and FEAT-0001 (4 days); BUG-0002 is excluded because Closed
    assert_eq!(metrics.stagnant_items.len(), 2);
    assert_eq!(metrics.stagnant_items[0].item_id, "BUG-0001");
    assert_eq!(metrics.stagnant_items[0].days_since_update, 6);
    assert_eq!(metrics.stagnant_items[1].item_id, "FEAT-0001");
    assert_eq!(metrics.stagnant_items[1].days_since_update, 4);

    // Hotspots: BUG-0001 has recurrence_count = 2 (ranked #1), BUG-0002 has recurrence_count = 0, fixes = 1 (#2)
    assert_eq!(metrics.item_hotspots.len(), 2);
    assert_eq!(metrics.item_hotspots[0].item_id, "BUG-0001");
    assert_eq!(metrics.item_hotspots[0].recurrence_count, 2);
    assert_eq!(metrics.item_hotspots[0].previous_fixes_count, 2);
    assert_eq!(metrics.item_hotspots[1].item_id, "BUG-0002");

    // Scope hotspots: backend_core has 2 total recurrences and 3 total previous fixes
    assert_eq!(metrics.scope_hotspots.len(), 1);
    assert_eq!(metrics.scope_hotspots[0].scope_id, "backend_core");
    assert_eq!(metrics.scope_hotspots[0].total_recurrences, 2);
    assert_eq!(metrics.scope_hotspots[0].total_previous_fixes, 3);
    assert_eq!(metrics.scope_hotspots[0].recurring_items_count, 1);
}

#[test]
fn test_agent_context_exporter_refs_and_bounded_prompt() {
    let (_dir, storage) = init_temp_tracker();

    storage
        .create_bug(
            "Deserializer panics on empty array",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::High,
            Some("When `bugs = []` is present in `scope.toml`, parsing fails."),
        )
        .unwrap();

    storage
        .close_item(
            "BUG-0001",
            "Tried custom visitor on ScopeItems",
            Some("a4f89c1"),
            false,
        )
        .unwrap();
    storage
        .reopen_item(
            "BUG-0001",
            Some("Still fails when features = [] is omitted"),
        )
        .unwrap();

    let bundle = storage
        .export_agent_context("BUG-0001")
        .expect("should export agent context bundle");

    assert_eq!(
        bundle.file_refs,
        "@.tracker/phases/01_core_engine/domains/backend_core/scope.toml @.tracker/phases/01_core_engine/domains/backend_core/items/BUG-0001/item.toml @.tracker/phases/01_core_engine/domains/backend_core/items/BUG-0001/desc.md"
    );
    assert_eq!(
        storage.export_file_refs("BUG-0001").unwrap(),
        bundle.file_refs
    );

    let prompt = &bundle.bounded_prompt;
    // 1. Contains scope boundaries & tech stack
    assert!(prompt.contains("crates/backend_core"));
    assert!(prompt.contains("rust, serde, git2"));
    assert!(prompt.contains("Restrict all code modifications strictly to `crates/backend_core`"));
    // 2. Contains recurrence warning and previous fix entry
    assert!(prompt.contains("REGRESSION & PREVIOUS FIX WARNING (`recurrence_count = 1`)"));
    assert!(prompt.contains("a4f89c1"));
    assert!(prompt.contains("Tried custom visitor on ScopeItems"));
    // 3. Contains desc.md specification and reopen notes
    assert!(prompt.contains("When `bugs = []` is present in `scope.toml`, parsing fails."));
    assert!(prompt.contains("Still fails when features = [] is omitted"));
}

#[test]
fn test_git_sync_status_commit_push_pull_and_keyring() {
    let (dir, storage) = init_temp_tracker();

    // Initialize local working Git repository
    let repo = Repository::init(dir.path()).expect("should init local git repo");
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test Developer").unwrap();
        cfg.set_str("user.email", "dev@nebulous.local").unwrap();
    }

    // Create an item so `.tracker/` has uncommitted files
    storage
        .create_bug(
            "Git sync verification bug",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::Medium,
            None,
        )
        .unwrap();

    // Check status before commit
    let status_before = storage.git_status().expect("git_status should succeed");
    assert!(status_before.tracker_dirty);
    assert!(!status_before.dirty_tracker_files.is_empty());

    // Stage and commit `.tracker/`
    let first_sha = storage
        .git_commit_tracker(Some("chore(tracker): add BUG-0001"))
        .expect("should commit .tracker/")
        .expect("should return commit SHA");
    assert!(first_sha.len() >= 7);

    // Status should now be clean and second commit should be a no-op (`Ok(None)`)
    let status_after = storage.git_status().unwrap();
    assert!(!status_after.tracker_dirty);
    assert_eq!(
        status_after.head_short_sha.as_deref(),
        Some(first_sha.as_str())
    );
    assert_eq!(storage.git_commit_tracker(None).unwrap(), None);

    // Set up a bare remote repository and push/pull between two workspaces
    let bare_dir = TempDir::new().unwrap();
    let _bare_repo = Repository::init_bare(bare_dir.path()).expect("should init bare remote repo");

    let current_branch = repo
        .head()
        .unwrap()
        .shorthand()
        .unwrap_or("main")
        .to_string();
    repo.remote("origin", bare_dir.path().to_str().unwrap())
        .unwrap();

    let mut project = storage.read_project().unwrap();
    project.git.remote = "origin".to_string();
    project.git.branch = current_branch.clone();
    storage.write_project(&project).unwrap();
    storage
        .git_commit_tracker(Some("chore(tracker): configure remote branch"))
        .unwrap();

    // Push from workspace 1 to bare remote
    storage
        .git_push(Some("test_pat_token_123"))
        .expect("should push branch to bare remote");

    // Clone bare remote into workspace 2
    let clone_dir = TempDir::new().unwrap();
    let cloned_repo =
        Repository::clone(bare_dir.path().to_str().unwrap(), clone_dir.path()).unwrap();

    // In workspace 1, create a second item (`FEAT-0001`), commit, and push
    storage
        .create_feature(
            "Remote synced feature",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::High,
            None,
        )
        .unwrap();
    storage
        .git_commit_tracker(Some("chore(tracker): add FEAT-0001"))
        .unwrap();
    storage.git_push(None).unwrap();

    // Ensure workspace 2 is on the same branch name and pull via `TrackerStorage::git_pull`
    {
        let head_commit = cloned_repo.head().unwrap().peel_to_commit().unwrap();
        let _ = cloned_repo.branch(&current_branch, &head_commit, true);
        cloned_repo
            .set_head(&format!("refs/heads/{current_branch}"))
            .unwrap();
    }
    let cloned_storage = TrackerStorage::new(clone_dir.path());
    let pulled = cloned_storage
        .git_pull(None)
        .expect("should pull new commit from remote and reconcile indexes");
    assert!(pulled);

    // Verify `FEAT-0001` is now loaded and indexed in workspace 2
    let (loc, detail) = cloned_storage
        .load_item_detail_by_id("FEAT-0001")
        .expect("workspace 2 should have FEAT-0001 after git_pull");
    assert_eq!(loc.scope_id, "backend_core");
    assert_eq!(detail.manifest.item.title, "Remote synced feature");

    // Verify Keyring PAT lifecycle (`store_pat`, `get_pat`, `delete_pat`) and `build_pat_callbacks`
    storage
        .store_pat("ghp_secret_pat_token_abc123")
        .expect("should store PAT");
    assert_eq!(
        storage.get_pat().expect("should retrieve stored PAT"),
        "ghp_secret_pat_token_abc123"
    );
    storage.delete_pat().expect("should delete PAT");
    assert!(storage.get_pat().is_err());

    let _callbacks = build_pat_callbacks(Some("ghp_secret_pat_token_abc123"));

    // Also test normal (non-fast-forward) merge on pull when workspace 1 and workspace 2 diverge cleanly
    storage
        .create_bug(
            "Divergent bug from workspace 1",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::Low,
            None,
        )
        .unwrap();
    storage
        .git_commit_tracker(Some("chore: divergent commit in workspace 1"))
        .unwrap();
    storage.git_push(None).unwrap();

    // Commit an unrelated file in workspace 2 (re-reading the on-disk index first) so pull requires a normal merge commit
    let readme_path = clone_dir.path().join("README.md");
    fs::write(&readme_path, "# Cloned Workspace\n").unwrap();
    {
        let mut idx = cloned_repo.index().unwrap();
        idx.read(true).unwrap();
        idx.add_path(Path::new("README.md")).unwrap();
        idx.write().unwrap();
        let tree_id = idx.write_tree().unwrap();
        let tree = cloned_repo.find_tree(tree_id).unwrap();
        let parent = cloned_repo.head().unwrap().peel_to_commit().unwrap();
        let sig = Signature::now("Clone User", "clone@nebulous.local").unwrap();
        cloned_repo
            .commit(
                Some("HEAD"),
                &sig,
                &sig,
                "docs: add README",
                &tree,
                &[&parent],
            )
            .unwrap();
    }

    let merged_pull = cloned_storage
        .git_pull(None)
        .expect("should perform clean normal merge on divergent branch");
    assert!(merged_pull);
    assert!(cloned_storage.load_item_detail_by_id("BUG-0002").is_ok());

    // Also verify divergent `.tracker/` item additions on both workspaces merge and auto-reconcile derived indexes cleanly!
    cloned_storage
        .git_push(None)
        .expect("should push merged commit back to remote");
    storage
        .git_pull(None)
        .expect("workspace 1 should pull merge commit");

    // Workspace 1 creates BUG-0003 and pushes
    storage
        .create_bug(
            "Concurrent bug in workspace 1",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::Medium,
            None,
        )
        .unwrap();
    storage
        .git_commit_tracker(Some("chore: add BUG-0003 in workspace 1"))
        .unwrap();
    storage.git_push(None).unwrap();

    // Workspace 2 creates FEAT-0002 (diverging on project.toml, phase.toml, scope.toml) and pulls
    cloned_storage
        .create_feature(
            "Concurrent feature in workspace 2",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::High,
            None,
        )
        .unwrap();
    cloned_storage
        .git_commit_tracker(Some("chore: add FEAT-0002 in workspace 2"))
        .unwrap();

    let concurrent_tracker_pull = cloned_storage
        .git_pull(None)
        .expect("divergent tracker item additions should auto-reconcile derived index conflicts");
    assert!(concurrent_tracker_pull);
    assert!(cloned_storage.load_item_detail_by_id("BUG-0003").is_ok());
    assert!(cloned_storage.load_item_detail_by_id("FEAT-0002").is_ok());
    assert!(!cloned_storage.git_status().unwrap().tracker_dirty);
}

#[test]
fn test_watcher_detects_external_changes_and_auto_reconciles_indexes() {
    let (_dir, storage) = init_temp_tracker();

    let (_watcher, rx) = storage
        .watch()
        .expect("should start notify watcher on .tracker/");

    // Simulate an external tool or git checkout writing a new item directory `items/BUG-0007`
    // directly on disk without updating `scope.toml` or `phase.toml`.
    let external_bug = ItemDetail {
        manifest: ItemManifest {
            item: ItemMetadata {
                id: "BUG-0007".to_string(),
                item_type: ItemType::Bug,
                title: "Externally created bug".to_string(),
                status: ItemStatus::Closed,
                severity: Severity::High,
                scope_ref: "backend_core".to_string(),
                created_at: "2026-11-02T10:00:00Z".to_string(),
                updated_at: "2026-11-02T11:00:00Z".to_string(),
            },
            effort: ItemEffort {
                logged_time_minutes: 15,
                iteration_count: 1,
            },
            history: ItemHistory::default(),
        },
        description_markdown: "## Problem Description\nCreated outside CLI/GUI.\n".to_string(),
    };

    storage
        .write_item_detail("01_core_engine", "backend_core", &external_bug)
        .expect("should write external item");

    // Wait for watcher event and automatic reconciliation
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let mut reconciled_event = None;
    while std::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if let Ok(ev) = rx.recv_timeout(remaining) {
            assert!(!ev.changed_paths.is_empty());
            if ev
                .reconcile_report
                .as_ref()
                .map(|r| r.total_items >= 1)
                .unwrap_or(false)
            {
                reconciled_event = Some(ev);
                break;
            }
        }
    }
    let event = reconciled_event
        .expect("should receive TrackerWatchEvent with reconciled item within 5 seconds");
    assert!(!event.changed_paths.is_empty());
    assert!(event.reconcile_report.is_some());

    // Drain any trailing events briefly and verify `scope.toml` and `phase.toml` were reconciled
    while rx.recv_timeout(Duration::from_millis(100)).is_ok() {}

    let scope = storage
        .read_scope("01_core_engine", "backend_core")
        .unwrap();
    assert_eq!(scope.items.bugs.len(), 1);
    assert_eq!(scope.items.bugs[0].id, "BUG-0007");
    assert_eq!(scope.items.bugs[0].status, ItemStatus::Closed);

    let phase = storage.read_phase("01_core_engine").unwrap();
    assert_eq!(phase.summary.total_items, 1);
    assert_eq!(phase.summary.completed_items, 1);

    // Ensure the watcher did NOT enter a self-trigger loop after reconciliation settled
    assert!(
        rx.recv_timeout(Duration::from_millis(250)).is_err(),
        "watcher must not loop on its own reconciled index writes"
    );
}

#[test]
fn test_phase_and_scope_management_and_metadata_updates() {
    let (_dir, storage) = init_temp_tracker();

    // Create a new phase and set it active
    let phase2 = storage
        .create_phase("02_ui_polish", "UI Polish Phase", true)
        .expect("should create and activate 02_ui_polish");
    assert_eq!(phase2.phase.id, "02_ui_polish");
    assert_eq!(
        storage.read_project().unwrap().project.active_phase,
        "02_ui_polish"
    );

    // Duplicate phase creation must fail with AlreadyExists
    assert!(matches!(
        storage.create_phase("02_ui_polish", "Duplicate", false),
        Err(TrackerError::AlreadyExists { kind: "Phase", .. })
    ));

    // Create scope in active phase (`02_ui_polish`)
    let cosmic_scope = storage
        .create_scope(
            None,
            "cosmic_interface",
            "crates/cosmic_interface",
            vec!["rust".to_string(), "libcosmic".to_string()],
            None,
        )
        .expect("should create cosmic_interface scope in active phase");
    assert_eq!(cosmic_scope.scope.id, "cosmic_interface");

    // Duplicate scope creation must fail with AlreadyExists
    assert!(matches!(
        storage.create_scope(
            Some("02_ui_polish"),
            "cosmic_interface",
            "crates/cosmic_interface",
            vec![],
            None
        ),
        Err(TrackerError::AlreadyExists { kind: "Scope", .. })
    ));

    // Create item using builder in active phase's cosmic_interface scope
    let (_, item) = storage
        .create_item(
            CreateItemInput::new(ItemType::Feature, "Kanban card layout")
                .with_scope("cosmic_interface")
                .with_severity(Severity::Medium),
        )
        .unwrap();
    assert_eq!(item.manifest.item.id, "FEAT-0001");

    // Update metadata (title + severity) and description
    let updated_meta = storage
        .update_item_metadata(
            "FEAT-0001",
            Some("4-Column Kanban Card Board".to_string()),
            Some(Severity::High),
        )
        .unwrap();
    assert_eq!(
        updated_meta.manifest.item.title,
        "4-Column Kanban Card Board"
    );
    assert_eq!(updated_meta.manifest.item.severity, Severity::High);

    let updated_scope = storage
        .read_scope("02_ui_polish", "cosmic_interface")
        .unwrap();
    assert_eq!(
        updated_scope.items.features[0].title,
        "4-Column Kanban Card Board"
    );
    assert_eq!(updated_scope.items.features[0].severity, Severity::High);

    let updated_desc = storage
        .update_item_description("FEAT-0001", "## Problem Description\nUpdated spec.\n")
        .unwrap();
    assert_eq!(
        updated_desc.description_markdown,
        "## Problem Description\nUpdated spec.\n"
    );

    // Switch active phase back to `01_core_engine`
    let proj = storage.set_active_phase("01_core_engine").unwrap();
    assert_eq!(proj.project.active_phase, "01_core_engine");
}

#[test]
fn test_git_fetch_ahead_behind_and_workspace_dirty_tracking() {
    let (dir, storage) = init_temp_tracker();
    let repo = Repository::init(dir.path()).expect("should init local git repo");
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test Developer").unwrap();
        cfg.set_str("user.email", "dev@nebulous.local").unwrap();
    }

    let bare_dir = TempDir::new().unwrap();
    let _bare_repo = Repository::init_bare(bare_dir.path()).expect("should init bare remote");
    let bare_url = bare_dir.path().to_str().unwrap().to_string();
    repo.remote("origin", &bare_url).unwrap();

    // Commit initial .tracker state
    let initial_sha = storage
        .git_commit_tracker(Some("chore(tracker): initial commit"))
        .unwrap()
        .unwrap();
    let current_branch = repo
        .head()
        .unwrap()
        .shorthand()
        .unwrap_or("main")
        .to_string();

    let mut project = storage.read_project().unwrap();
    project.git.remote = "origin".to_string();
    project.git.branch = current_branch.clone();
    storage.write_project(&project).unwrap();
    storage
        .git_commit_tracker(Some("chore(tracker): set remote and branch"))
        .unwrap();

    // Push initial state -> ahead = 0, behind = 0
    storage.git_push(None).expect("should push to bare remote");
    let st_synced = storage.git_status().unwrap();
    assert_eq!(st_synced.remote_url.as_deref(), Some(bare_url.as_str()));
    assert_eq!(st_synced.ahead_count, 0);
    assert_eq!(st_synced.behind_count, 0);
    assert!(!st_synced.tracker_dirty);
    assert!(!st_synced.workspace_dirty);
    assert!(st_synced.dirty_workspace_files.is_empty());

    // Clone bare remote into workspace 2
    let clone_dir = TempDir::new().unwrap();
    let cloned_repo = Repository::clone(&bare_url, clone_dir.path()).unwrap();
    {
        let mut cfg = cloned_repo.config().unwrap();
        cfg.set_str("user.name", "Remote Developer").unwrap();
        cfg.set_str("user.email", "remote@nebulous.local").unwrap();
        let head_commit = cloned_repo.head().unwrap().peel_to_commit().unwrap();
        let _ = cloned_repo.branch(&current_branch, &head_commit, true);
        cloned_repo
            .set_head(&format!("refs/heads/{current_branch}"))
            .unwrap();
    }
    let cloned_storage = TrackerStorage::new(clone_dir.path());

    // Workspace 2 creates an item, commits, and pushes to remote
    cloned_storage
        .create_bug(
            "Upstream bug reported in workspace 2",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::High,
            None,
        )
        .unwrap();
    cloned_storage
        .git_commit_tracker(Some("chore(tracker): add BUG-0001 from workspace 2"))
        .unwrap();
    cloned_storage.git_push(None).unwrap();

    // Workspace 1 modifies a workspace file (uncommitted) and runs `git_fetch`
    let src_dir = dir.path().join("crates/backend_core/src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), "pub fn placeholder() {}\n").unwrap();

    let head_before_fetch = storage.git_head_short_sha();
    let fetch_status = storage
        .git_fetch(None)
        .expect("git_fetch should succeed without modifying working tree");

    // Verify `git_fetch` updated `behind_count` to 1, left `ahead_count` at 0, preserved `HEAD`,
    // and reported the dirty workspace file without touching the working tree
    assert_eq!(fetch_status.ahead_count, 0);
    assert_eq!(fetch_status.behind_count, 1);
    assert_eq!(storage.git_head_short_sha(), head_before_fetch);
    assert_ne!(head_before_fetch, initial_sha);
    assert!(fetch_status.workspace_dirty);
    assert_eq!(
        fetch_status.dirty_workspace_files,
        vec!["crates/backend_core/src/lib.rs".to_string()]
    );
    assert!(
        storage.load_item_detail_by_id("BUG-0001").is_err(),
        "git_fetch must not merge remote changes into working tree"
    );

    // Clean up uncommitted workspace file, create a local tracker commit in workspace 1 so it both
    // diverges ahead (1) and remains behind (1)
    fs::remove_file(src_dir.join("lib.rs")).unwrap();
    storage
        .create_feature(
            "Local feature in workspace 1",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::Medium,
            None,
        )
        .unwrap();
    storage
        .git_commit_tracker(Some("chore(tracker): local FEAT-0001"))
        .unwrap();

    let diverged_status = storage.git_status().unwrap();
    assert_eq!(diverged_status.ahead_count, 1);
    assert_eq!(diverged_status.behind_count, 1);

    // Pull and push to bring ahead_count and behind_count back to 0
    assert!(storage.git_pull(None).unwrap());
    storage.git_push(None).unwrap();
    let final_status = storage.git_status().unwrap();
    assert_eq!(final_status.ahead_count, 0);
    assert_eq!(final_status.behind_count, 0);

    // Verify `git_pull` rejects overwriting an uncommitted dirty workspace file when remote modifies the same file,
    // without advancing local HEAD first.
    cloned_storage.git_pull(None).unwrap();
    let remote_src = clone_dir.path().join("crates/backend_core/src");
    fs::create_dir_all(&remote_src).unwrap();
    fs::write(
        remote_src.join("conflict.rs"),
        "pub fn remote_version() {}\n",
    )
    .unwrap();
    cloned_storage
        .git_commit_documented(DocumentedCommitInput::for_item(
            "BUG-0001",
            "Add conflict.rs on remote workspace",
        ))
        .unwrap();
    cloned_storage.git_push(None).unwrap();

    // Create conflicting uncommitted file in workspace 1
    fs::write(
        src_dir.join("conflict.rs"),
        "pub fn local_dirty_version() {}\n",
    )
    .unwrap();
    let head_before_failed_pull = storage.git_head_short_sha();
    let pull_err = storage
        .git_pull(None)
        .expect_err("git_pull must reject overwriting dirty workspace file");
    assert!(matches!(pull_err, TrackerError::GitError(ref msg) if msg.contains("conflict.rs")));
    assert_eq!(
        storage.git_head_short_sha(),
        head_before_failed_pull,
        "failed git_pull must not advance local HEAD"
    );
    assert_eq!(
        fs::read_to_string(src_dir.join("conflict.rs")).unwrap(),
        "pub fn local_dirty_version() {}\n",
        "failed git_pull must preserve local dirty file contents"
    );

    // Remove conflicting dirty file -> git_pull now succeeds cleanly
    fs::remove_file(src_dir.join("conflict.rs")).unwrap();
    assert!(storage.git_pull(None).unwrap());
    assert_eq!(
        fs::read_to_string(src_dir.join("conflict.rs")).unwrap(),
        "pub fn remote_version() {}\n"
    );
}

#[test]
fn test_git_commit_documented_policy_enforcement_and_atomic_doc_updates() {
    let (dir, storage) = init_temp_tracker();
    let repo = Repository::init(dir.path()).expect("should init local git repo");
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Policy Tester").unwrap();
        cfg.set_str("user.email", "policy@nebulous.local").unwrap();
    }

    // Verify default CommitPolicyConfig values in project.toml
    let project = storage.read_project().unwrap();
    assert_eq!(
        project.git.commit_policy,
        CommitPolicyConfig {
            require_item_id: true,
            require_scope: true,
            require_summary: true,
            min_summary_length: 12,
            auto_append_to_desc: true,
            ssh_key_account: None,
        }
    );

    // 1. Create BUG-0001 in backend_core scope
    storage
        .create_bug(
            "Parser rejects empty array in scope manifest",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::High,
            Some("Ensure empty arrays deserialize cleanly."),
        )
        .unwrap();

    // 2. Pure `.tracker/` metadata commit with no item_id should auto-generate `[TRACKER](sync) ...`
    let tracker_commit = storage
        .git_commit_documented(DocumentedCommitInput::tracker_only(""))
        .expect("pure .tracker/ commit without item_id should succeed");
    assert_eq!(
        tracker_commit.message,
        "[TRACKER](sync) sync .tracker metadata state"
    );
    assert!(!tracker_commit.documentation_updated);
    assert!(!storage.git_status().unwrap().tracker_dirty);

    // 3. Committing when neither workspace nor .tracker/ has changes must fail
    assert!(matches!(
        storage.git_commit_documented(DocumentedCommitInput::for_item(
            "BUG-0001",
            "Attempt empty commit with no changes"
        )),
        Err(TrackerError::ValidationError(_))
    ));

    // 4. Modify a workspace code file in `crates/backend_core/src/parser.rs`
    let code_dir = dir.path().join("crates/backend_core/src");
    fs::create_dir_all(&code_dir).unwrap();
    let code_file = code_dir.join("parser.rs");
    fs::write(&code_file, "pub fn parse_scope() -> bool { true }\n").unwrap();

    let status_with_code = storage.git_status().unwrap();
    assert!(status_with_code.workspace_dirty);
    assert_eq!(
        status_with_code.dirty_workspace_files,
        vec!["crates/backend_core/src/parser.rs".to_string()]
    );

    // 5. Policy Violation A: Missing item_id on workspace code commit when `require_item_id = true`
    let missing_item_err = storage
        .git_commit_documented(DocumentedCommitInput::new(
            "Implement scope parser fix without item ID",
        ))
        .expect_err("must reject undocumented code commit without item_id");
    assert!(matches!(
        missing_item_err,
        TrackerError::ValidationError(ref msg) if msg.contains("item_id")
    ));

    // 6. Policy Violation B: Non-existent item_id
    let unknown_item_err = storage
        .git_commit_documented(DocumentedCommitInput::for_item(
            "BUG-9999",
            "Fix scope parser with non-existent item",
        ))
        .expect_err("must reject unknown item_id");
    assert!(matches!(
        unknown_item_err,
        TrackerError::NotFound { kind: "Item", .. }
    ));

    // 7. Policy Violation C: Summary shorter than `min_summary_length` (12 chars)
    let short_summary_err = storage
        .git_commit_documented(DocumentedCommitInput::for_item("BUG-0001", "Too short"))
        .expect_err("must reject summary shorter than min_summary_length");
    assert!(matches!(
        short_summary_err,
        TrackerError::ValidationError(ref msg) if msg.contains("at least 12 characters")
    ));

    // 8. Policy Violation D: Unsafe relative path (`..`)
    let unsafe_path_err = storage
        .git_commit_documented(
            DocumentedCommitInput::for_item(
                "BUG-0001",
                "Valid summary length for path traversal test",
            )
            .with_paths(["../outside_workspace.rs"]),
        )
        .expect_err("must reject path traversal");
    assert!(matches!(
        unsafe_path_err,
        TrackerError::ValidationError(ref msg) if msg.contains("..")
    ));

    // 9. Valid documented code commit linked to BUG-0001 with body notes
    let outcome = storage
        .git_commit_documented(
            DocumentedCommitInput::for_item(
                "BUG-0001",
                "Add serde default fallback to ScopeItems arrays",
            )
            .with_body("Verified with empty bugs and features arrays in unit tests."),
        )
        .expect("valid documented code commit should succeed");

    assert_eq!(outcome.item_id.as_deref(), Some("BUG-0001"));
    assert_eq!(outcome.scope_id.as_deref(), Some("backend_core"));
    assert!(outcome.documentation_updated);
    assert_eq!(
        outcome.message,
        "[BUG-0001](backend_core) Add serde default fallback to ScopeItems arrays\n\nVerified with empty bugs and features arrays in unit tests."
    );
    assert!(outcome
        .changed_files
        .contains(&"crates/backend_core/src/parser.rs".to_string()));
    assert!(outcome.changed_files.contains(
        &".tracker/phases/01_core_engine/domains/backend_core/items/BUG-0001/item.toml".to_string()
    ));
    assert!(outcome.changed_files.contains(
        &".tracker/phases/01_core_engine/domains/backend_core/items/BUG-0001/desc.md".to_string()
    ));

    // Verify working tree and `.tracker/` are completely clean after atomic re-stage & amend
    let post_commit_status = storage.git_status().unwrap();
    assert!(!post_commit_status.tracker_dirty);
    assert!(!post_commit_status.workspace_dirty);
    assert_eq!(
        post_commit_status.head_short_sha.as_deref(),
        Some(outcome.short_sha())
    );

    // Verify `item.toml` (`history.previous_fixes`) and `desc.md` (`## Fix Log & Agent Notes`) were updated
    let (_, updated_bug) = storage.load_item_detail_by_id("BUG-0001").unwrap();
    assert_eq!(updated_bug.manifest.history.previous_fixes.len(), 1);
    let fix_entry = &updated_bug.manifest.history.previous_fixes[0];
    assert_eq!(fix_entry.commit, outcome.recorded_fix_sha);
    assert!(fix_entry
        .summary
        .contains("Add serde default fallback to ScopeItems arrays"));
    assert!(fix_entry
        .summary
        .contains("crates/backend_core/src/parser.rs"));

    assert!(updated_bug
        .description_markdown
        .contains("## Fix Log & Agent Notes"));
    assert!(updated_bug
        .description_markdown
        .contains(&outcome.recorded_fix_sha));
    assert!(updated_bug
        .description_markdown
        .contains("Add serde default fallback to ScopeItems arrays"));
    assert!(updated_bug
        .description_markdown
        .contains("crates/backend_core/src/parser.rs"));
    assert!(updated_bug
        .description_markdown
        .contains("Verified with empty bugs and features arrays in unit tests."));

    // Verify Git HEAD commit message in repository matches formatted message
    let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
    assert_eq!(
        head_commit.message().unwrap().trim(),
        outcome.message.trim()
    );

    // 10. Non-existent path in `with_paths` must fail with ValidationError
    let nonexistent_err = storage
        .git_commit_documented(
            DocumentedCommitInput::for_item("BUG-0001", "Attempt commit with nonexistent path")
                .with_paths(["crates/backend_core/src/does_not_exist.rs"]),
        )
        .expect_err("must reject nonexistent explicit path");
    assert!(matches!(
        nonexistent_err,
        TrackerError::ValidationError(ref msg) if msg.contains("does_not_exist.rs")
    ));

    // 11. Selective `paths` staging: modify two files (`a.rs` and `b.rs`), commit only `a.rs`
    let file_a = code_dir.join("a.rs");
    let file_b = code_dir.join("b.rs");
    fs::write(&file_a, "pub fn a() {}\n").unwrap();
    fs::write(&file_b, "pub fn b() {}\n").unwrap();

    let selective_outcome = storage
        .git_commit_documented(
            DocumentedCommitInput::for_item(
                "BUG-0001",
                "Stage only module a while leaving b dirty",
            )
            .with_paths(["crates/backend_core/src/a.rs"]),
        )
        .expect("selective path commit should succeed");
    assert!(selective_outcome
        .changed_files
        .contains(&"crates/backend_core/src/a.rs".to_string()));
    assert!(!selective_outcome
        .changed_files
        .contains(&"crates/backend_core/src/b.rs".to_string()));

    let status_after_selective = storage.git_status().unwrap();
    assert!(!status_after_selective.tracker_dirty);
    assert!(status_after_selective.workspace_dirty);
    assert_eq!(
        status_after_selective.dirty_workspace_files,
        vec!["crates/backend_core/src/b.rs".to_string()]
    );

    // 12. Selective deletion via `with_paths`: delete `a.rs` on disk while `b.rs` is also dirty,
    // and stage ONLY the deletion of `a.rs` linked to BUG-0001
    fs::remove_file(&file_a).unwrap();
    let selective_delete_outcome = storage
        .git_commit_documented(
            DocumentedCommitInput::for_item("BUG-0001", "Remove obsolete module a selectively")
                .with_paths(["crates/backend_core/src/a.rs"]),
        )
        .expect("selective deletion commit should succeed even when file is removed from disk");
    assert!(selective_delete_outcome
        .changed_files
        .contains(&"crates/backend_core/src/a.rs".to_string()));
    assert!(!selective_delete_outcome
        .changed_files
        .contains(&"crates/backend_core/src/b.rs".to_string()));
    assert_eq!(
        storage.git_status().unwrap().dirty_workspace_files,
        vec!["crates/backend_core/src/b.rs".to_string()]
    );

    // 13. Custom policy: `require_item_id = false`, `require_scope = true`, `auto_append_to_desc = false`
    let mut proj = storage.read_project().unwrap();
    proj.git.commit_policy.require_item_id = false;
    proj.git.commit_policy.require_scope = true;
    proj.git.commit_policy.auto_append_to_desc = false;
    storage.write_project(&proj).unwrap();

    // Missing scope_id when require_scope = true and item_id is None must fail
    assert!(matches!(
        storage.git_commit_documented(DocumentedCommitInput::new(
            "Workspace commit missing required scope_id"
        )),
        Err(TrackerError::ValidationError(ref msg)) if msg.contains("scope_id")
    ));

    let workspace_scoped_outcome = storage
        .git_commit_documented(
            DocumentedCommitInput::new("Add module b in backend_core with custom policy")
                .with_scope("backend_core"),
        )
        .expect(
            "workspace commit with explicit scope should succeed when require_item_id is false",
        );
    assert_eq!(
        workspace_scoped_outcome.message,
        "[WORKSPACE](backend_core) Add module b in backend_core with custom policy"
    );
    assert!(!workspace_scoped_outcome.documentation_updated);
    assert!(workspace_scoped_outcome
        .changed_files
        .contains(&"crates/backend_core/src/b.rs".to_string()));
    assert!(!storage.git_status().unwrap().workspace_dirty);
}

#[test]
fn test_git_auth_resolution_ssh_keypair_and_pat() {
    let (dir, storage) = init_temp_tracker();
    let vault_dir = dir.path().join("isolated_vault");
    std::env::set_var("NEBULOUS_CONFIG_DIR", &vault_dir);

    // Generate an Ed25519 SSH keypair for a custom account and configure `ssh_key_account` in `[git.commit_policy]`
    let manager = backend_core::KeyPairManager::vault_only(&vault_dir)
        .expect("should init isolated KeyPairManager");
    let generated = manager
        .generate_ed25519_keypair("nebulous/ssh-deploy", Some("deploy@nebulous.local"))
        .expect("should generate Ed25519 SSH keypair");

    let mut project = storage.read_project().unwrap();
    project.git.commit_policy.ssh_key_account = Some("nebulous/ssh-deploy".to_string());
    storage.write_project(&project).unwrap();

    storage
        .store_pat("ghp_auth_resolution_pat_token_456")
        .expect("should store PAT");

    let resolved = storage
        .resolve_git_auth_with_manager(Some(&manager), None)
        .expect("should resolve git auth from SecureStorage & KeyPairManager");
    assert!(resolved.has_pat());
    assert_eq!(
        resolved.pat.as_deref(),
        Some("ghp_auth_resolution_pat_token_456")
    );
    assert!(resolved.has_ssh_credentials());
    assert_eq!(
        resolved
            .ssh_keypair
            .as_ref()
            .map(|k| k.fingerprint_sha256.as_str()),
        Some(generated.fingerprint_sha256.as_str())
    );

    let _callbacks = build_auth_callbacks(resolved);

    // Clean up stored credentials
    let _ = manager.delete_ssh_keypair("nebulous/ssh-deploy");
    let _ = storage.delete_pat();
    std::env::remove_var("NEBULOUS_CONFIG_DIR");
}

#[test]
fn test_cross_stage_scope_mirroring_and_move_item_to_phase() {
    let dir = TempDir::new().expect("should create tempdir");
    let storage = TrackerStorage::init_standard(dir.path(), "StageFlowProject")
        .expect("should init_standard 3-stage project");

    // 1. Create `backend_core` scope in `pre` (`pre_production`) and verify automatic mirroring
    // across `pre_production`, `production`, and `post_production`.
    let created_scope = storage
        .create_scope(
            Some("pre"),
            "backend_core",
            "crates/backend_core",
            vec!["rust".to_string(), "serde".to_string()],
            Some("Core Rust backend".to_string()),
        )
        .expect("should create and mirror backend_core scope");
    assert_eq!(created_scope.scope.id, "backend_core");

    for stage in ["pre_production", "production", "post_production"] {
        let scope_in_stage = storage
            .read_scope(stage, "backend_core")
            .expect("backend_core must be mirrored in all 3 canonical stages");
        assert_eq!(scope_in_stage.scope.target, "crates/backend_core");
        assert_eq!(
            scope_in_stage.scope.tech_stack,
            vec!["rust".to_string(), "serde".to_string()]
        );
        assert_eq!(
            scope_in_stage.scope.description.as_deref(),
            Some("Core Rust backend")
        );
        assert!(storage.items_dir(stage, "backend_core").unwrap().is_dir());
    }

    // 2. Create `FEAT-0001` in `pre_production` (`pre`) under `backend_core`
    let (loc_pre, detail_pre) = storage
        .create_feature(
            "3-Stage Canonical Phasing & Item Movement",
            Some("pre"),
            Some("backend_core"),
            Severity::High,
            Some("Verify item movement across pre_production -> production -> post_production."),
        )
        .expect("should create FEAT-0001 in pre_production");
    assert_eq!(loc_pre.phase_id, "pre_production");
    assert_eq!(loc_pre.scope_id, "backend_core");
    assert_eq!(detail_pre.manifest.item.id, "FEAT-0001");

    // Start an active timer on FEAT-0001 so we also verify timer phase_id updates on move
    let t0 = Utc.with_ymd_and_hms(2026, 11, 4, 18, 0, 0).unwrap();
    storage
        .start_timer_at(
            "FEAT-0001",
            t0,
            Some("Implementing stage movement".to_string()),
        )
        .expect("should start timer on FEAT-0001");

    // 3. Move `FEAT-0001` from `pre_production` -> `production` using shorthand `"prod"`
    let (loc_prod, detail_prod) = storage
        .move_item_to_phase("FEAT-0001", "prod")
        .expect("should move FEAT-0001 to production");
    assert_eq!(loc_prod.phase_id, "production");
    assert_eq!(loc_prod.scope_id, "backend_core");
    assert_eq!(detail_prod.manifest.item.id, "FEAT-0001");
    assert!(detail_prod
        .description_markdown
        .contains("Verify item movement across pre_production"));

    // Old item directory in `pre_production` must be removed, and `production` item directory must exist
    assert!(!storage
        .item_dir("pre_production", "backend_core", "FEAT-0001")
        .unwrap()
        .exists());
    assert!(storage
        .item_toml_path("production", "backend_core", "FEAT-0001")
        .unwrap()
        .is_file());
    assert!(storage
        .item_desc_path("production", "backend_core", "FEAT-0001")
        .unwrap()
        .is_file());

    // Source and target `scope.toml` and `phase.toml` summaries must be atomically updated
    assert!(storage
        .read_scope("pre_production", "backend_core")
        .unwrap()
        .items
        .features
        .is_empty());
    assert_eq!(
        storage
            .read_phase("pre_production")
            .unwrap()
            .summary
            .total_items,
        0
    );
    assert_eq!(
        storage
            .read_scope("production", "backend_core")
            .unwrap()
            .items
            .features
            .len(),
        1
    );
    assert_eq!(
        storage
            .read_phase("production")
            .unwrap()
            .summary
            .total_items,
        1
    );

    // Active timer `.timer.toml` must point to `"production"`
    assert_eq!(
        storage
            .read_active_timer()
            .unwrap()
            .unwrap()
            .phase_id
            .as_deref(),
        Some("production")
    );

    // 4. Move `FEAT-0001` from `production` -> `post_production` using numeric alias `"3"`
    let (loc_post, _) = storage
        .move_item_to_phase("FEAT-0001", "3")
        .expect("should move FEAT-0001 to post_production");
    assert_eq!(loc_post.phase_id, "post_production");
    assert_eq!(
        storage
            .read_phase("production")
            .unwrap()
            .summary
            .total_items,
        0
    );
    assert_eq!(
        storage
            .read_phase("post_production")
            .unwrap()
            .summary
            .total_items,
        1
    );

    // Switch active phase using shorthand alias `"post"`
    let proj_after = storage
        .set_active_phase("post")
        .expect("should switch active phase via alias");
    assert_eq!(proj_after.project.active_phase, "post_production");
    assert_eq!(
        storage.read_phase("post").unwrap().phase.status,
        backend_core::PhaseStatus::InProgress
    );
}

#[test]
fn test_infer_scope_from_context_all_priority_levels() {
    let dir = TempDir::new().expect("should create tempdir");
    let storage = TrackerStorage::init_standard(dir.path(), "ScopeInferenceProject")
        .expect("should init_standard");

    // Priority 5: Only `inbox` exists -> falls back to `default_scope` ("inbox")
    assert_eq!(
        storage.infer_scope_from_context(None).unwrap(),
        "inbox",
        "when only inbox exists, must fall back to inbox"
    );

    // Priority 4: Exactly one non-inbox scope (`backend_core`) exists -> returns `backend_core`
    storage
        .create_scope(
            Some("pre"),
            "backend_core",
            "crates/backend_core",
            vec!["rust".to_string()],
            None,
        )
        .unwrap();
    assert_eq!(
        storage.infer_scope_from_context(None).unwrap(),
        "backend_core",
        "when exactly one non-inbox scope exists, must return that scope"
    );

    // Add a second and third scope (`nebulous_cli` and nested `backend_core_sub`) so multiple non-inbox scopes exist
    storage
        .create_scope(
            Some("pre"),
            "nebulous_cli",
            "crates/nebulous_cli",
            vec!["rust".to_string()],
            None,
        )
        .unwrap();
    storage
        .create_scope(
            Some("pre"),
            "backend_workflow",
            "crates/backend_core/src/workflow",
            vec!["rust".to_string()],
            None,
        )
        .unwrap();

    // With >1 non-inbox scopes, no cwd match, no git dirty files, and no timer -> falls back to `inbox`
    assert_eq!(
        storage.infer_scope_from_context(Some(dir.path())).unwrap(),
        "inbox"
    );

    // Priority 3: Active timer with a `scope_id` -> returns timer's `scope_id`
    storage
        .create_bug(
            "CLI arg parser bug",
            Some("pre"),
            Some("nebulous_cli"),
            Severity::Medium,
            None,
        )
        .unwrap();
    storage.start_timer("BUG-0001", None).unwrap();
    assert_eq!(
        storage.infer_scope_from_context(None).unwrap(),
        "nebulous_cli",
        "active timer scope must be inferred when cwd and git dirty files do not match"
    );

    // Priority 2: Dirty/modified files in Git (`most modified files wins`) overrides active timer
    let _repo = Repository::init(dir.path()).expect("should init git repo");
    let backend_src = dir.path().join("crates/backend_core/src");
    let cli_src = dir.path().join("crates/nebulous_cli/src");
    fs::create_dir_all(&backend_src).unwrap();
    fs::create_dir_all(&cli_src).unwrap();
    fs::write(backend_src.join("a.rs"), "pub fn a() {}\n").unwrap();
    fs::write(backend_src.join("b.rs"), "pub fn b() {}\n").unwrap();
    fs::write(cli_src.join("main.rs"), "fn main() {}\n").unwrap();

    assert_eq!(
        storage.infer_scope_from_context(None).unwrap(),
        "backend_core",
        "scope with most modified files in Git (2 vs 1) must win over timer"
    );

    // Priority 1: `cwd` inside a scope's `target` directory (longest prefix match) overrides Git dirty files
    let cli_cwd = dir.path().join("crates/nebulous_cli/src");
    assert_eq!(
        storage.infer_scope_from_context(Some(&cli_cwd)).unwrap(),
        "nebulous_cli",
        "cwd inside crates/nebulous_cli/src must infer nebulous_cli"
    );

    let nested_workflow_cwd = dir.path().join("crates/backend_core/src/workflow");
    assert_eq!(
        storage
            .infer_scope_from_context(Some(&nested_workflow_cwd))
            .unwrap(),
        "backend_workflow",
        "longest target prefix match (crates/backend_core/src/workflow vs crates/backend_core) must win"
    );
}

#[test]
fn test_resolve_active_or_inferred_item_id_and_whats_next_ranking() {
    let dir = TempDir::new().expect("should create tempdir");
    let storage = TrackerStorage::init_standard(dir.path(), "WhatsNextProject")
        .expect("should init_standard");

    // Create a Cargo.toml on disk while only `inbox` scope exists -> triggers UnscopedWorkspace warning
    fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    let initial_report = storage.compute_whats_next(None).unwrap();
    assert_eq!(initial_report.active_phase, "pre_production");
    assert_eq!(initial_report.inferred_scope, "inbox");
    assert!(initial_report
        .recommendations
        .iter()
        .any(|r| r.kind == NextActionKind::UnscopedWorkspace));

    // Promote scopes `backend_core` and `nebulous_cli`
    storage
        .create_scope(
            Some("pre"),
            "backend_core",
            "crates/backend_core",
            vec!["rust".to_string()],
            None,
        )
        .unwrap();
    storage
        .create_scope(
            Some("pre"),
            "nebulous_cli",
            "crates/nebulous_cli",
            vec!["rust".to_string()],
            None,
        )
        .unwrap();

    // Create items to test ranking:
    // - BUG-0001: Open in `nebulous_cli`, Medium severity, default placeholder description (missing spec)
    storage
        .create_bug(
            "CLI table alignment glitch",
            Some("pre"),
            Some("nebulous_cli"),
            Severity::Medium,
            None,
        )
        .unwrap();

    // - FEAT-0001: Open in `backend_core`, High severity
    storage
        .create_feature(
            "Assistive engine ranking",
            Some("pre"),
            Some("backend_core"),
            Severity::High,
            Some("Complete specification provided."),
        )
        .unwrap();

    // - BUG-0002: Open in `backend_core`, High severity (should beat FEAT-0001 on `item_type == Bug`)
    storage
        .create_bug(
            "Lockfile contention in backend_core",
            Some("pre"),
            Some("backend_core"),
            Severity::High,
            None, // leaves default placeholder in desc.md!
        )
        .unwrap();

    // - BUG-0003: Recurring bug (`recurrence_count = 1`), Medium severity in `nebulous_cli`
    storage
        .create_bug(
            "Recurring regression in CLI",
            Some("pre"),
            Some("nebulous_cli"),
            Severity::Medium,
            Some("Detailed regression notes."),
        )
        .unwrap();
    storage
        .record_regression("BUG-0003", Some("Re-triggered on empty args"))
        .unwrap();

    // - FEAT-0002: Stagnant item in `nebulous_cli` (updated 10 days before `eval_now`)
    let (_, mut stagnant_feat) = storage
        .create_feature(
            "Stagnant CLI subcommand",
            Some("pre"),
            Some("nebulous_cli"),
            Severity::Low,
            Some("Detailed spec."),
        )
        .unwrap();
    let eval_now = Utc::now();
    stagnant_feat.manifest.item.updated_at =
        (eval_now - chrono::Duration::days(10)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    storage
        .write_item_detail("pre_production", "nebulous_cli", &stagnant_feat)
        .unwrap();
    storage.reconcile_indexes().unwrap();

    let backend_cwd = dir.path().join("crates/backend_core");
    let report = storage
        .compute_whats_next_at(Some(&backend_cwd), eval_now)
        .expect("should compute whats_next");

    assert_eq!(report.inferred_scope, "backend_core");
    // Expected order when no item is active:
    // #1: BUG-0003 (RecurringBug, recurrence_count = 1)
    // #2: FEAT-0002 (StagnantItem, 9 days since update >= threshold 4)
    // #3: BUG-0002 (PrioritizedOpenItem in inferred_scope `backend_core`, High, Bug)
    // #4: FEAT-0001 (PrioritizedOpenItem in inferred_scope `backend_core`, High, Feature)
    // #5: BUG-0001 (PrioritizedOpenItem in other scope `nebulous_cli`, Medium, Bug)
    let ranked_ids: Vec<&str> = report
        .recommendations
        .iter()
        .filter_map(|r| r.item_id.as_deref())
        .collect();
    assert_eq!(
        ranked_ids,
        vec!["BUG-0003", "FEAT-0002", "BUG-0002", "FEAT-0001", "BUG-0001"]
    );
    assert_eq!(report.recommendations[0].kind, NextActionKind::RecurringBug);
    assert_eq!(report.recommendations[1].kind, NextActionKind::StagnantItem);
    assert_eq!(
        report.recommendations[2].kind,
        NextActionKind::PrioritizedOpenItem
    );

    // Verify `resolve_active_or_inferred_item_id`:
    // 1. Explicit ID normalizes case
    assert_eq!(
        storage
            .resolve_active_or_inferred_item_id(Some("  bug-0002 "))
            .unwrap(),
        "BUG-0002"
    );
    // 2. When no timer or in_progress item exists, returns ValidationError suggesting top recommended ID ("BUG-0003")
    let err = storage
        .resolve_active_or_inferred_item_id(None)
        .expect_err("should fail when no timer or in_progress item exists");
    assert!(matches!(
        err,
        TrackerError::ValidationError(ref msg) if msg.contains("BUG-0003")
    ));

    // Close BUG-0003 and FEAT-0002 so BUG-0002 (which has default placeholder `desc.md`) is the top recommended item
    storage
        .close_item("BUG-0003", "Fixed recurring bug", None, false)
        .unwrap();
    storage
        .close_item("FEAT-0002", "Completed stagnant feature", None, false)
        .unwrap();

    let report_missing_spec = storage
        .compute_whats_next_at(Some(&backend_cwd), eval_now)
        .unwrap();
    assert_eq!(report_missing_spec.top_item_id(), Some("BUG-0002"));
    assert!(
        !report_missing_spec.missing_fields.is_empty(),
        "top recommended item with default desc.md placeholder must report missing_fields"
    );
    assert!(report_missing_spec
        .missing_fields
        .contains(&"problem_description".to_string()));
    assert!(!report_missing_spec.recommendations[0]
        .missing_fields
        .is_empty());
}

#[test]
fn test_start_work_item_auto_timer_switch_close_stop_and_stage_advancement_hint() {
    let dir = TempDir::new().expect("should create tempdir");
    let storage =
        TrackerStorage::init_standard(dir.path(), "AutoFlowProject").expect("should init_standard");

    storage
        .create_scope(
            Some("pre"),
            "backend_core",
            "crates/backend_core",
            vec!["rust".to_string()],
            None,
        )
        .unwrap();

    storage
        .create_bug(
            "Critical startup bug",
            Some("pre"),
            Some("backend_core"),
            Severity::Critical,
            Some("Fix startup crash."),
        )
        .unwrap();
    storage
        .create_feature(
            "Secondary feature",
            Some("pre"),
            Some("backend_core"),
            Severity::Medium,
            Some("Implement secondary feature."),
        )
        .unwrap();

    // 1. Call `start_work_item(None, ...)` -> automatically picks top recommended open item (`BUG-0001`),
    // transitions status to `InProgress`, starts `.timer.toml`, and returns `AgentContextBundle`.
    let (loc1, detail1, timer1, bundle1) = storage
        .start_work_item(None, Some("Starting critical bug fix".to_string()))
        .expect("start_work_item(None) should pick BUG-0001");
    assert_eq!(loc1.stub.id, "BUG-0001");
    assert_eq!(detail1.manifest.item.status, ItemStatus::InProgress);
    assert_eq!(timer1.item_id, "BUG-0001");
    assert_eq!(timer1.note.as_deref(), Some("Starting critical bug fix"));
    assert!(bundle1.bounded_prompt.contains("BUG-0001"));

    // `resolve_active_or_inferred_item_id(None)` now resolves `BUG-0001` via active timer
    assert_eq!(
        storage.resolve_active_or_inferred_item_id(None).unwrap(),
        "BUG-0001"
    );

    // Simulate 30 minutes elapsed on `BUG-0001`'s timer by backdating `started_at` in `.timer.toml`
    let mut backdated_timer = storage.read_active_timer().unwrap().unwrap();
    let thirty_mins_ago = Utc::now() - chrono::Duration::minutes(30);
    backdated_timer.started_at = thirty_mins_ago.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    storage.write_active_timer(&backdated_timer).unwrap();

    // 2. Call `start_work_item(Some("FEAT-0001"), ...)` while `BUG-0001` timer is running:
    // Must automatically stop `BUG-0001`'s timer, log ~30 minutes onto `BUG-0001`, transition
    // `FEAT-0001` to `InProgress`, and start `.timer.toml` on `FEAT-0001`!
    let (loc2, detail2, timer2, bundle2) = storage
        .start_work_item(Some("feat-0001"), Some("Switching to feature".to_string()))
        .expect("start_work_item on FEAT-0001 should auto-switch timer without error");
    assert_eq!(loc2.stub.id, "FEAT-0001");
    assert_eq!(detail2.manifest.item.status, ItemStatus::InProgress);
    assert_eq!(timer2.item_id, "FEAT-0001");
    assert_eq!(timer2.note.as_deref(), Some("Switching to feature"));
    assert!(bundle2.bounded_prompt.contains("FEAT-0001"));

    let (_, bug1_after_switch) = storage.load_item_detail_by_id("BUG-0001").unwrap();
    assert!(
        bug1_after_switch.manifest.effort.logged_time_minutes >= 29,
        "previous item BUG-0001 must have elapsed minutes logged when timer switches"
    );

    // Simulate 15 minutes elapsed on `FEAT-0001`'s timer
    let mut feat_timer = storage.read_active_timer().unwrap().unwrap();
    assert_eq!(feat_timer.item_id, "FEAT-0001");
    let fifteen_mins_ago = Utc::now() - chrono::Duration::minutes(15);
    feat_timer.started_at = fifteen_mins_ago.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    storage.write_active_timer(&feat_timer).unwrap();

    // 3. Closing `FEAT-0001` while `.timer.toml` is timing it must automatically stop the timer
    // and add the 15 elapsed minutes to `effort.logged_time_minutes` before writing the closed manifest!
    let closed_feat = storage
        .close_item("FEAT-0001", "Completed secondary feature", None, true)
        .expect("close_item should auto-stop active timer and log elapsed minutes");
    assert_eq!(closed_feat.manifest.item.status, ItemStatus::Merged);
    assert!(
        closed_feat.manifest.effort.logged_time_minutes >= 14,
        "closing timed item must accumulate elapsed timer minutes"
    );
    assert!(
        storage.read_active_timer().unwrap().is_none(),
        ".timer.toml must be cleared when timed item is closed"
    );

    // Now only `BUG-0001` remains `InProgress` (with no active timer):
    // `resolve_active_or_inferred_item_id(None)` must resolve `BUG-0001` via single `in_progress` item!
    assert_eq!(
        storage.resolve_active_or_inferred_item_id(None).unwrap(),
        "BUG-0001"
    );

    // Close `BUG-0001` as well so all items in `pre_production` (2/2) are completed
    storage
        .close_item("BUG-0001", "Resolved critical startup bug", None, false)
        .unwrap();

    // 4. Verify `compute_whats_next` emits Stage Advancement Hint (`pre_production` -> `production`)
    let completed_stage_report = storage.compute_whats_next(None).unwrap();
    assert!(
        completed_stage_report.stage_advancement_hint.is_some(),
        "when all items in active_phase are completed, stage_advancement_hint must be present"
    );
    assert!(completed_stage_report
        .stage_advancement_hint
        .as_deref()
        .unwrap()
        .contains("production"));
    assert!(completed_stage_report
        .recommendations
        .iter()
        .any(|r| r.kind == NextActionKind::AdvanceStage
            && r.phase_id.as_deref() == Some("production")));
}
