use std::fs;
use std::path::Path;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use git2::{Repository, Signature};
use tempfile::TempDir;

use backend_core::{
    build_pat_callbacks, CreateItemInput, ItemDetail, ItemEffort, ItemHistory, ItemManifest,
    ItemMetadata, ItemStatus, ItemType, PreviousFix, Severity, TrackerError, TrackerStorage,
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
    let event = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("should receive TrackerWatchEvent within 5 seconds");
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
