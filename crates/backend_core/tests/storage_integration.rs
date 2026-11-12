use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use backend_core::{
    atomic_write_str, parse_item_id_number, ActiveTimer, ItemDetail, ItemEffort, ItemHistory,
    ItemManifest, ItemMetadata, ItemStatus, ItemStub, ItemType, PhaseManifest, PhaseMetadata,
    PhaseStatus, PhaseSummary, PreviousFix, ScopeItems, ScopeManifest, ScopeMetadata,
    ScopeRegistry, Severity, TrackerError, TrackerStorage,
};

static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(1);

struct TempTrackerDir {
    path: PathBuf,
}

impl TempTrackerDir {
    fn new(prefix: &str) -> Self {
        let seq = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let pid = std::process::id();
        let dir_name = format!("nebulous_test_{prefix}_{pid}_{seq}");
        let path = std::env::temp_dir().join(dir_name);
        if path.exists() {
            let _ = fs::remove_dir_all(&path);
        }
        fs::create_dir_all(&path).expect("should create temp test directory");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempTrackerDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn seed_dogfood_tracker(root: &Path) -> TrackerStorage {
    let storage = TrackerStorage::init(
        root,
        "Nebulous",
        "01_core_engine",
        "Core Engine & Persistence",
    )
    .expect("should initialize dogfood fixture tracker");

    for (scope_id, target) in [
        ("backend_core", "crates/backend_core"),
        ("nebulous_cli", "crates/nebulous_cli"),
        ("cosmic_interface", "crates/cosmic_interface"),
    ] {
        storage
            .create_scope(
                Some("01_core_engine"),
                scope_id,
                target,
                vec!["rust".to_string()],
                Some(format!("{scope_id} domain scope")),
            )
            .expect("should create scope in 01_core_engine");
    }

    storage
        .create_phase("02_git_and_polish", "Git Sync & Polish", false)
        .expect("should create phase 02_git_and_polish");

    for (scope_id, target) in [
        ("backend_core", "crates/backend_core"),
        ("nebulous_cli", "crates/nebulous_cli"),
        ("cosmic_interface", "crates/cosmic_interface"),
    ] {
        storage
            .create_scope(
                Some("02_git_and_polish"),
                scope_id,
                target,
                vec!["rust".to_string()],
                Some(format!("{scope_id} domain scope")),
            )
            .expect("should create scope in 02_git_and_polish");
    }

    storage
        .create_bug(
            "Empty array fails deserialization in scope.toml",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::High,
            Some("Occurs when bugs = [] or features = [] is omitted."),
        )
        .unwrap();
    storage
        .close_item(
            "BUG-0001",
            "Added #[serde(default)] on ScopeItems vectors",
            Some("1234abc"),
            false,
        )
        .unwrap();

    storage
        .create_feature(
            "4-tier .tracker/ TOML + Markdown storage engine",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::High,
            Some("Implement atomic writes and lazy loading of desc.md."),
        )
        .unwrap();
    storage
        .close_item(
            "FEAT-0001",
            "Implemented TrackerStorage and atomic persistence",
            Some("2345bcd"),
            false,
        )
        .unwrap();

    storage
        .create_feature(
            "Workflow state machine & recurrence ledger",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::High,
            Some("Track recurrence_count and previous_fixes on close/reopen."),
        )
        .unwrap();

    storage
        .create_feature(
            "Metrics & valuation engine",
            Some("01_core_engine"),
            Some("backend_core"),
            Severity::Medium,
            Some("Compute target-days valuation, stagnation, and hotspot rankings."),
        )
        .unwrap();

    storage
        .create_feature(
            "Companion CLI (`neb`) with 16 subcommands",
            Some("01_core_engine"),
            Some("nebulous_cli"),
            Severity::Medium,
            Some("Provide human-readable and --json output modes."),
        )
        .unwrap();
    storage
        .update_item_status("FEAT-0004", ItemStatus::Pending)
        .unwrap();

    storage
        .create_feature(
            "Native libcosmic desktop GUI (`nebulous`)",
            Some("01_core_engine"),
            Some("cosmic_interface"),
            Severity::High,
            Some("4-column Kanban board, context drawer, and metrics view."),
        )
        .unwrap();

    storage
}

fn sample_item_detail(
    id: &str,
    item_type: ItemType,
    title: &str,
    status: ItemStatus,
    severity: Severity,
    scope_ref: &str,
    desc: &str,
) -> ItemDetail {
    ItemDetail {
        manifest: ItemManifest {
            item: ItemMetadata {
                id: id.to_string(),
                item_type,
                title: title.to_string(),
                status,
                severity,
                scope_ref: scope_ref.to_string(),
                created_at: "2026-11-02T10:00:00Z".to_string(),
                updated_at: "2026-11-02T12:00:00Z".to_string(),
            },
            effort: ItemEffort {
                logged_time_minutes: 25,
                iteration_count: 1,
            },
            history: ItemHistory {
                recurrence_count: 0,
                previous_fixes: vec![PreviousFix {
                    date: "2026-11-02".to_string(),
                    commit: "1234abc".to_string(),
                    summary: "Initial implementation".to_string(),
                }],
            },
        },
        description_markdown: desc.to_string(),
    }
}

#[test]
fn test_atomic_write_leaves_no_temp_files_and_overwrites_cleanly() {
    let temp = TempTrackerDir::new("atomic_write");
    let target = temp.path().join("nested").join("config.toml");

    atomic_write_str(&target, "first = 1\n").expect("initial atomic write should succeed");
    assert_eq!(fs::read_to_string(&target).unwrap(), "first = 1\n");

    atomic_write_str(&target, "second = 2\n").expect("overwrite atomic write should succeed");
    assert_eq!(fs::read_to_string(&target).unwrap(), "second = 2\n");

    // Ensure no `.tmp.` sibling files remain
    let entries: Vec<String> = fs::read_dir(target.parent().unwrap())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(entries, vec!["config.toml".to_string()]);
}

#[test]
fn test_lazy_loading_does_not_read_desc_md_during_boot_index_load() {
    let temp = TempTrackerDir::new("lazy_load");
    let storage = TrackerStorage::init(
        temp.path(),
        "LazyTest",
        "01_core_engine",
        "Core Engine Phase",
    )
    .expect("should initialize tracker");

    // Create a backend_core scope
    let scope = ScopeManifest {
        scope: ScopeMetadata {
            id: "backend_core".to_string(),
            target: "crates/backend_core".to_string(),
            tech_stack: vec!["rust".to_string()],
            description: None,
            bound_component: None,
            tracked_files: Vec::new(),
        },
        items: ScopeItems {
            bugs: vec![ItemStub {
                id: "BUG-0001".to_string(),
                title: "Lazy bug".to_string(),
                status: ItemStatus::Open,
                severity: Severity::High,
            }],
            features: vec![],
        },
    };
    storage
        .write_scope("01_core_engine", "backend_core", &scope)
        .unwrap();

    let mut phase = storage.read_phase("01_core_engine").unwrap();
    phase
        .scopes
        .manifest
        .push(backend_core::ScopeRegistryEntry {
            id: "backend_core".to_string(),
            scope_type: "rust_crate".to_string(),
            path: "domains/backend_core".to_string(),
        });
    phase.summary.total_items = 1;
    storage.write_phase("01_core_engine", &phase).unwrap();

    // Write `item.toml`, but intentionally DO NOT create `desc.md` yet!
    let detail = sample_item_detail(
        "BUG-0001",
        ItemType::Bug,
        "Lazy bug",
        ItemStatus::Open,
        Severity::High,
        "backend_core",
        "## Lazy Markdown Body\nLoaded only on demand.",
    );
    storage
        .write_item_manifest(
            "01_core_engine",
            "backend_core",
            "BUG-0001",
            &detail.manifest,
        )
        .unwrap();

    // `load_index()` MUST succeed without touching `desc.md`
    let index = storage
        .load_index()
        .expect("load_index should not read desc.md");
    assert_eq!(index.phases.len(), 1);
    let found = index
        .find_item("BUG-0001")
        .expect("should find BUG-0001 in index");
    assert_eq!(found.scope_id, "backend_core");

    // Calling `load_item_detail()` now fails because `desc.md` is missing
    assert!(matches!(
        storage.load_item_detail("01_core_engine", "backend_core", "BUG-0001"),
        Err(TrackerError::Io { .. })
    ));

    // Write `desc.md` and verify `load_item_detail()` and `load_item_detail_by_id()` succeed
    storage
        .write_item_desc(
            "01_core_engine",
            "backend_core",
            "BUG-0001",
            &detail.description_markdown,
        )
        .unwrap();

    let loaded = storage
        .load_item_detail("01_core_engine", "backend_core", "BUG-0001")
        .expect("should lazily load item.toml + desc.md");
    assert_eq!(loaded, detail);

    let (loc, loaded_by_id) = storage
        .load_item_detail_by_id("BUG-0001")
        .expect("should locate and lazily load by ID");
    assert_eq!(loc.phase_id, "01_core_engine");
    assert_eq!(loc.scope_id, "backend_core");
    assert_eq!(loaded_by_id, detail);
}

#[test]
fn test_monotonic_id_allocation_and_directory_scan_safety_check() {
    let temp = TempTrackerDir::new("id_alloc");
    let storage =
        TrackerStorage::init(temp.path(), "IdTest", "01_core_engine", "Core Engine").unwrap();

    assert_eq!(parse_item_id_number("BUG-0001"), Some((ItemType::Bug, 1)));
    assert_eq!(
        parse_item_id_number("feat-0042"),
        Some((ItemType::Feature, 42))
    );
    assert_eq!(parse_item_id_number("INVALID-0001"), None);
    assert_eq!(parse_item_id_number("BUG-xyz"), None);

    // Sequential allocation from 0
    assert_eq!(storage.allocate_next_id(ItemType::Bug).unwrap(), "BUG-0001");
    assert_eq!(storage.allocate_next_id(ItemType::Bug).unwrap(), "BUG-0002");
    assert_eq!(
        storage.allocate_next_id(ItemType::Feature).unwrap(),
        "FEAT-0001"
    );

    // Simulate external/manual creation of `items/BUG-0009` and `items/FEAT-0012/item.toml`
    // while `project.toml` still has `bug = 2` and `feature = 1`.
    let manual_bug_dir = storage
        .item_dir("01_core_engine", "inbox", "BUG-0009")
        .unwrap();
    fs::create_dir_all(&manual_bug_dir).unwrap();

    let feat_12 = sample_item_detail(
        "FEAT-0012",
        ItemType::Feature,
        "Manual feature on disk",
        ItemStatus::Open,
        Severity::Medium,
        "inbox",
        "Manual desc",
    );
    storage
        .write_item_detail("01_core_engine", "inbox", &feat_12)
        .unwrap();

    // Next allocated IDs must jump past the on-disk items
    assert_eq!(storage.allocate_next_id(ItemType::Bug).unwrap(), "BUG-0010");
    assert_eq!(
        storage.allocate_next_id(ItemType::Feature).unwrap(),
        "FEAT-0013"
    );

    let project = storage.read_project().unwrap();
    assert_eq!(project.counters.bug, 10);
    assert_eq!(project.counters.feature, 13);
}

#[test]
fn test_scope_resolution_fallback_to_default_scope_and_inbox_bootstrap() {
    let temp = TempTrackerDir::new("scope_fallback");
    let storage =
        TrackerStorage::init(temp.path(), "FallbackTest", "01_core_engine", "Phase 1").unwrap();

    // Omitting both phase and scope falls back to active_phase ("01_core_engine") and default_scope ("inbox")
    let (phase, scope) = storage.resolve_phase_and_scope(None, None).unwrap();
    assert_eq!(phase, "01_core_engine");
    assert_eq!(scope, "inbox");

    // Empty strings also fall back to active_phase and default_scope
    let (phase2, scope2) = storage
        .resolve_phase_and_scope(Some("  "), Some(""))
        .unwrap();
    assert_eq!(phase2, "01_core_engine");
    assert_eq!(scope2, "inbox");

    // Delete `domains/inbox/scope.toml` and remove it from `phase.toml` to verify auto-bootstrapping
    fs::remove_dir_all(storage.scope_dir("01_core_engine", "inbox").unwrap()).unwrap();
    let mut phase_manifest = storage.read_phase("01_core_engine").unwrap();
    phase_manifest.scopes.manifest.clear();
    storage
        .write_phase("01_core_engine", &phase_manifest)
        .unwrap();

    let (phase3, scope3) = storage.resolve_phase_and_scope(None, None).unwrap();
    assert_eq!(phase3, "01_core_engine");
    assert_eq!(scope3, "inbox");
    assert!(storage
        .scope_toml_path("01_core_engine", "inbox")
        .unwrap()
        .is_file());

    // Requesting a non-existent non-default scope returns NotFound
    assert!(matches!(
        storage.resolve_phase_and_scope(None, Some("unknown_crate")),
        Err(TrackerError::NotFound { kind: "Scope", .. })
    ));

    // Path traversal identifiers are rejected
    assert!(matches!(
        storage.resolve_phase_and_scope(Some("../escape"), None),
        Err(TrackerError::ValidationError(_))
    ));
    assert!(matches!(
        storage.resolve_phase_and_scope(None, Some("bad/scope")),
        Err(TrackerError::ValidationError(_))
    ));
}

#[test]
fn test_reconcile_indexes_rebuilds_from_authoritative_item_toml() {
    let temp = TempTrackerDir::new("reconcile");
    let storage =
        TrackerStorage::init(temp.path(), "ReconcileTest", "01_core_engine", "Phase 1").unwrap();

    // Create an unregistered scope directory `domains/backend_core` on disk
    let backend_scope = ScopeManifest {
        scope: ScopeMetadata {
            id: "backend_core".to_string(),
            target: "crates/backend_core".to_string(),
            tech_stack: vec!["rust".to_string()],
            description: None,
            bound_component: None,
            tracked_files: Vec::new(),
        },
        // Intentionally wrong/stale stub list in scope.toml
        items: ScopeItems {
            bugs: vec![ItemStub {
                id: "BUG-9999".to_string(),
                title: "Ghost bug that does not exist".to_string(),
                status: ItemStatus::Open,
                severity: Severity::Low,
            }],
            features: vec![],
        },
    };
    storage
        .write_scope("01_core_engine", "backend_core", &backend_scope)
        .unwrap();

    // Write 3 authoritative items under `backend_core` and 1 under `inbox`
    let bug_2 = sample_item_detail(
        "BUG-0002",
        ItemType::Bug,
        "Second bug (open)",
        ItemStatus::Open,
        Severity::High,
        "backend_core",
        "Bug 2 body",
    );
    let bug_1 = sample_item_detail(
        "BUG-0001",
        ItemType::Bug,
        "First bug (closed)",
        ItemStatus::Closed,
        Severity::Critical,
        "backend_core",
        "Bug 1 body",
    );
    let feat_3 = sample_item_detail(
        "FEAT-0003",
        ItemType::Feature,
        "Merged feature",
        ItemStatus::Merged,
        Severity::Medium,
        "backend_core",
        "Feat 3 body",
    );
    let inbox_bug = sample_item_detail(
        "BUG-0005",
        ItemType::Bug,
        "Inbox triage item",
        ItemStatus::InProgress,
        Severity::Low,
        "inbox",
        "Inbox body",
    );

    storage
        .write_item_detail("01_core_engine", "backend_core", &bug_2)
        .unwrap();
    storage
        .write_item_detail("01_core_engine", "backend_core", &bug_1)
        .unwrap();
    storage
        .write_item_detail("01_core_engine", "backend_core", &feat_3)
        .unwrap();
    storage
        .write_item_detail("01_core_engine", "inbox", &inbox_bug)
        .unwrap();

    // Also create an unregistered phase `02_extra_phase` on disk
    let extra_phase = PhaseManifest {
        phase: PhaseMetadata {
            id: "02_extra_phase".to_string(),
            title: "Extra Discovered Phase".to_string(),
            status: PhaseStatus::Planned,
            start_date: None,
            end_date: None,
        },
        scopes: ScopeRegistry::default(),
        summary: PhaseSummary {
            total_items: 500,
            completed_items: 400,
        },
    };
    storage.write_phase("02_extra_phase", &extra_phase).unwrap();

    // Run reconciliation
    let report = storage
        .reconcile_indexes()
        .expect("reconcile_indexes should succeed");
    assert_eq!(report.phases_reconciled, 2);
    assert_eq!(report.scopes_reconciled, 2);
    assert_eq!(report.total_items, 4);
    assert_eq!(report.completed_items, 2); // BUG-0001 (Closed) + FEAT-0003 (Merged)
    assert_eq!(report.counters.bug, 5);
    assert_eq!(report.counters.feature, 3);

    // Verify `scope.toml` in `backend_core` has sorted BUG-0001, BUG-0002 and FEAT-0003, and no ghost BUG-9999
    let reconciled_backend = storage
        .read_scope("01_core_engine", "backend_core")
        .unwrap();
    assert_eq!(reconciled_backend.items.bugs.len(), 2);
    assert_eq!(reconciled_backend.items.bugs[0].id, "BUG-0001");
    assert_eq!(reconciled_backend.items.bugs[0].status, ItemStatus::Closed);
    assert_eq!(reconciled_backend.items.bugs[1].id, "BUG-0002");
    assert_eq!(reconciled_backend.items.bugs[1].status, ItemStatus::Open);
    assert_eq!(reconciled_backend.items.features.len(), 1);
    assert_eq!(reconciled_backend.items.features[0].id, "FEAT-0003");
    assert_eq!(
        reconciled_backend.items.features[0].status,
        ItemStatus::Merged
    );

    // Verify `phase.toml` summary totals
    let reconciled_phase1 = storage.read_phase("01_core_engine").unwrap();
    assert_eq!(reconciled_phase1.summary.total_items, 4);
    assert_eq!(reconciled_phase1.summary.completed_items, 2);

    let reconciled_phase2 = storage.read_phase("02_extra_phase").unwrap();
    assert_eq!(reconciled_phase2.summary.total_items, 0);
    assert_eq!(reconciled_phase2.summary.completed_items, 0);
}

#[test]
fn test_active_timer_storage_lifecycle() {
    let temp = TempTrackerDir::new("timer");
    let storage =
        TrackerStorage::init(temp.path(), "TimerTest", "01_core_engine", "Phase 1").unwrap();

    assert!(storage.read_active_timer().unwrap().is_none());

    let timer = ActiveTimer::new(
        "BUG-0001",
        Some("01_core_engine".to_string()),
        Some("backend_core".to_string()),
        "2026-11-02T15:00:00Z",
    );
    storage.write_active_timer(&timer).unwrap();
    assert_eq!(storage.read_active_timer().unwrap(), Some(timer));

    storage.clear_active_timer().unwrap();
    assert!(storage.read_active_timer().unwrap().is_none());
}

#[test]
fn test_live_dogfood_tracker_loads_and_reconciles_cleanly() {
    let temp_root = TempTrackerDir::new("dogfood_fixture");
    let _seeded = seed_dogfood_tracker(temp_root.path());
    let storage = TrackerStorage::discover(temp_root.path())
        .expect("should discover seeded .tracker/ fixture");

    let index_before = storage
        .load_index()
        .expect("should load seeded .tracker/ top-level index");

    assert_eq!(index_before.project.project.name, "Nebulous");
    assert_eq!(index_before.project.project.active_phase, "01_core_engine");
    assert_eq!(index_before.project.project.default_scope, "inbox");
    assert_eq!(index_before.phases.len(), 2);

    let phase1 = index_before
        .phase("01_core_engine")
        .expect("should contain phase 01_core_engine");
    assert!(phase1.scope("backend_core").is_some());
    assert!(phase1.scope("nebulous_cli").is_some());
    assert!(phase1.scope("cosmic_interface").is_some());
    assert!(phase1.scope("inbox").is_some());

    let all_items = index_before.all_items();
    assert!(
        all_items.len() >= 6,
        "expected at least 6 dogfood items in .tracker/"
    );

    // Lazily load every indexed item and validate its manifest + markdown description
    for loc in &all_items {
        let detail = storage
            .load_item_detail(&loc.phase_id, &loc.scope_id, &loc.stub.id)
            .expect("every indexed dogfood item must have a valid item.toml and desc.md");
        assert_eq!(detail.manifest.item.id, loc.stub.id);
        assert_eq!(detail.manifest.item.title, loc.stub.title);
        assert_eq!(detail.manifest.item.status, loc.stub.status);
        assert_eq!(detail.manifest.item.severity, loc.stub.severity);
        assert!(
            detail
                .description_markdown
                .contains("## Problem Description"),
            "desc.md for {} should contain ## Problem Description",
            loc.stub.id
        );
    }

    // Copy seeded `.tracker/` into an isolated temp directory, reconcile, and verify zero drift
    let temp_copy = TempTrackerDir::new("dogfood_copy");
    let copied_tracker_dir = temp_copy.path().join(".tracker");
    copy_dir_recursive(storage.tracker_dir(), &copied_tracker_dir)
        .expect("should copy seeded .tracker/ to temp dir");

    let copied_storage = TrackerStorage::new(temp_copy.path());
    let report = copied_storage
        .reconcile_indexes()
        .expect("reconciling seeded .tracker/ should succeed");
    assert_eq!(report.phases_reconciled, 2);
    assert_eq!(report.scopes_reconciled, 8);
    assert_eq!(report.total_items, 6);
    assert_eq!(report.completed_items, 2);

    let index_after = copied_storage.load_index().unwrap();
    assert_eq!(index_before, index_after);
}

#[test]
fn test_custom_relative_paths_and_reconcile_edge_cases() {
    let temp = TempTrackerDir::new("custom_paths_and_edge_cases");
    let storage = TrackerStorage::init(
        temp.path(),
        "CustomPathTest",
        "01_core_engine",
        "Core Engine",
    )
    .unwrap();

    // 1. Verify resolve_phase_and_scope registers inbox in phase.toml even when domains/inbox/scope.toml
    // already exists on disk (only removed from phase.toml manifest).
    let mut phase_manifest = storage.read_phase("01_core_engine").unwrap();
    phase_manifest.scopes.manifest.clear();
    storage
        .write_phase("01_core_engine", &phase_manifest)
        .unwrap();
    assert!(storage
        .scope_toml_path("01_core_engine", "inbox")
        .unwrap()
        .is_file());

    let (resolved_phase, resolved_scope) = storage.resolve_phase_and_scope(None, None).unwrap();
    assert_eq!(resolved_phase, "01_core_engine");
    assert_eq!(resolved_scope, "inbox");
    assert_eq!(
        storage
            .read_phase("01_core_engine")
            .unwrap()
            .scopes
            .manifest
            .len(),
        1
    );

    // 2. Register a custom phase path and custom scope path in project.toml / phase.toml
    let mut project = storage.read_project().unwrap();
    project
        .phases
        .registry
        .push(backend_core::PhaseRegistryEntry {
            id: "02_custom".to_string(),
            title: "Custom Phase".to_string(),
            path: "phases/custom_phase_dir".to_string(),
        });
    // Also add a stale phase entry whose directory does not exist on disk
    project
        .phases
        .registry
        .push(backend_core::PhaseRegistryEntry {
            id: "99_deleted_phase".to_string(),
            title: "Deleted Phase".to_string(),
            path: "phases/99_deleted_phase".to_string(),
        });
    project.project.active_phase = "99_deleted_phase".to_string();
    storage.write_project(&project).unwrap();

    let custom_phase = PhaseManifest {
        phase: PhaseMetadata {
            id: "02_custom".to_string(),
            title: "Custom Phase".to_string(),
            status: PhaseStatus::InProgress,
            start_date: None,
            end_date: None,
        },
        scopes: ScopeRegistry {
            manifest: vec![
                backend_core::ScopeRegistryEntry {
                    id: "custom_scope".to_string(),
                    scope_type: "rust_crate".to_string(),
                    path: "custom_domains/custom_scope_dir".to_string(),
                },
                backend_core::ScopeRegistryEntry {
                    id: "deleted_scope".to_string(),
                    scope_type: "rust_crate".to_string(),
                    path: "domains/deleted_scope".to_string(),
                },
            ],
        },
        summary: PhaseSummary::default(),
    };
    storage.write_phase("02_custom", &custom_phase).unwrap();

    let custom_scope = ScopeManifest {
        scope: ScopeMetadata {
            id: "custom_scope".to_string(),
            target: "crates/custom".to_string(),
            tech_stack: vec!["rust".to_string()],
            description: None,
            bound_component: None,
            tracked_files: Vec::new(),
        },
        items: ScopeItems::default(),
    };
    storage
        .write_scope("02_custom", "custom_scope", &custom_scope)
        .unwrap();

    // Write an item into custom_scope (via custom phase & scope paths)
    let feat_7 = sample_item_detail(
        "FEAT-0007",
        ItemType::Feature,
        "Canonical feature in custom scope",
        ItemStatus::Closed,
        Severity::High,
        "custom_scope",
        "Custom scope markdown",
    );
    storage
        .write_item_detail("02_custom", "custom_scope", &feat_7)
        .unwrap();

    // Create a duplicate subdirectory `items/FEAT-0007_dup/item.toml` with the same ID `FEAT-0007`
    // and a partial directory `items/BUG-0042` without an `item.toml`
    let dup_dir = storage
        .items_dir("02_custom", "custom_scope")
        .unwrap()
        .join("FEAT-0007_dup");
    fs::create_dir_all(&dup_dir).unwrap();
    let mut dup_manifest = feat_7.manifest.clone();
    dup_manifest.item.title = "Non-canonical duplicate title".to_string();
    dup_manifest.item.status = ItemStatus::Open;
    atomic_write_str(
        &dup_dir.join("item.toml"),
        &dup_manifest.to_toml_string().unwrap(),
    )
    .unwrap();

    let partial_bug_dir = storage
        .items_dir("02_custom", "custom_scope")
        .unwrap()
        .join("BUG-0042");
    fs::create_dir_all(&partial_bug_dir).unwrap();

    // Reconcile indexes: should prune 99_deleted_phase and deleted_scope, fallback active_phase,
    // deduplicate FEAT-0007 preferring canonical directory, and advance BUG counter to 42.
    let report = storage.reconcile_indexes().unwrap();
    assert_eq!(report.phases_reconciled, 2);
    assert_eq!(report.scopes_reconciled, 2);
    assert_eq!(report.total_items, 1);
    assert_eq!(report.completed_items, 1);
    assert_eq!(report.counters.bug, 42);
    assert_eq!(report.counters.feature, 7);

    // load_index() and load_item_detail_by_id() must succeed cleanly after pruning stale entries
    let index = storage
        .load_index()
        .expect("load_index should succeed after pruning deleted phases/scopes");
    assert_eq!(index.project.project.active_phase, "01_core_engine");
    assert_eq!(index.phases.len(), 2);

    let (loc, detail) = storage
        .load_item_detail_by_id("FEAT-0007")
        .expect("should locate and lazily load FEAT-0007 from custom phase/scope path");
    assert_eq!(loc.phase_id, "02_custom");
    assert_eq!(loc.scope_id, "custom_scope");
    assert_eq!(
        detail.manifest.item.title,
        "Canonical feature in custom scope"
    );
    assert_eq!(detail.manifest.item.status, ItemStatus::Closed);

    // Next allocated bug ID must be BUG-0043
    assert_eq!(storage.allocate_next_id(ItemType::Bug).unwrap(), "BUG-0043");
}

#[test]
fn test_global_project_registry_lifecycle_and_edge_cases() {
    use backend_core::{
        load_global_registry_from, register_project_in, remove_project_in, save_global_registry_to,
        set_active_project_in, GlobalProjectRegistry,
    };

    let temp = TempTrackerDir::new("global_registry");
    let cfg_dir = temp.path().join("config");
    let proj_a_dir = temp.path().join("Nebulous");
    let proj_b_dir = temp.path().join("Orbit");
    let proj_c_dir = temp.path().join("Orbit_Fork");

    fs::create_dir_all(&proj_a_dir).unwrap();
    fs::create_dir_all(&proj_b_dir).unwrap();
    fs::create_dir_all(&proj_c_dir).unwrap();

    TrackerStorage::init(&proj_a_dir, "Nebulous", "01_core", "Core Phase").unwrap();
    let orbit_storage =
        TrackerStorage::init(&proj_b_dir, "Orbit", "01_sid_base", "Sid Base Phase").unwrap();
    let mut orbit_proj = orbit_storage.read_project().unwrap();
    orbit_proj.git.branch = "rolling-sid".to_string();
    orbit_storage.write_project(&orbit_proj).unwrap();

    // 1. Loading before projects.toml exists returns an empty default registry
    let initial = load_global_registry_from(&cfg_dir).expect("should load empty default registry");
    assert!(initial.projects.is_empty());
    assert!(initial.active_project_id.is_none());

    // 2. Register first project (Nebulous): automatically becomes active
    let reg_a = register_project_in(
        &cfg_dir,
        &proj_a_dir,
        None,
        vec![
            "rust".to_string(),
            "tracker".to_string(),
            "rust".to_string(),
        ],
        false,
    )
    .expect("should register Nebulous");
    assert_eq!(reg_a.id, "nebulous");
    assert_eq!(reg_a.name, "Nebulous");
    assert_eq!(reg_a.default_branch, "main");
    assert_eq!(reg_a.active_phase.as_deref(), Some("01_core"));
    assert_eq!(reg_a.tags, vec!["rust".to_string(), "tracker".to_string()]);
    assert!(reg_a.parsed_last_opened_at().is_ok());

    let loaded_after_a = load_global_registry_from(&cfg_dir).unwrap();
    assert_eq!(
        loaded_after_a.active_project_id.as_deref(),
        Some("nebulous")
    );
    assert_eq!(loaded_after_a.projects.len(), 1);

    // 3. Register second project (Orbit) with set_active = true
    let reg_b = register_project_in(
        &cfg_dir,
        &proj_b_dir,
        None,
        vec!["distro".to_string(), "rolling_base".to_string()],
        true,
    )
    .expect("should register Orbit");
    assert_eq!(reg_b.id, "orbit");
    assert_eq!(reg_b.default_branch, "rolling-sid");
    assert_eq!(reg_b.active_phase.as_deref(), Some("01_sid_base"));

    // 4. Register third project with conflicting name "Orbit" at a different path -> gets id "orbit_2"
    let reg_c = register_project_in(
        &cfg_dir,
        &proj_c_dir,
        Some("Orbit"),
        vec!["experimental".to_string()],
        false,
    )
    .expect("should register second Orbit project with disambiguated id");
    assert_eq!(reg_c.id, "orbit_2");

    // 5. Re-registering proj_a_dir updates existing entry in place without creating duplicates
    let reg_a_updated = register_project_in(
        &cfg_dir,
        &proj_a_dir,
        Some("Nebulous Core"),
        vec!["dogfood".to_string()],
        false,
    )
    .unwrap();
    assert_eq!(reg_a_updated.id, "nebulous");
    assert_eq!(reg_a_updated.name, "Nebulous Core");
    assert_eq!(reg_a_updated.tags, vec!["dogfood".to_string()]);

    let reg_state = load_global_registry_from(&cfg_dir).unwrap();
    assert_eq!(reg_state.projects.len(), 3);
    assert_eq!(reg_state.active_project_id.as_deref(), Some("orbit"));

    // 6. Switch active project by name or ID
    let switched = set_active_project_in(&cfg_dir, "Nebulous Core").unwrap();
    assert_eq!(switched.id, "nebulous");
    let reg_state = load_global_registry_from(&cfg_dir).unwrap();
    assert_eq!(reg_state.active_project_id.as_deref(), Some("nebulous"));

    // 7. Remove active project ("nebulous") -> active_project_id falls back to first remaining ("orbit")
    let removed = remove_project_in(&cfg_dir, "nebulous").unwrap();
    assert_eq!(removed.id, "nebulous");
    let reg_state = load_global_registry_from(&cfg_dir).unwrap();
    assert_eq!(reg_state.projects.len(), 2);
    assert_eq!(reg_state.active_project_id.as_deref(), Some("orbit"));

    // 8. Round-trip TOML serialization of GlobalProjectRegistry
    let raw_toml = reg_state.to_toml_string().unwrap();
    let reparsed = GlobalProjectRegistry::from_toml_str(&raw_toml).unwrap();
    assert_eq!(reg_state, reparsed);
    save_global_registry_to(&cfg_dir, &reparsed).unwrap();

    // 9. Error cases: non-existent directory, non-existent project ID
    assert!(matches!(
        register_project_in(
            &cfg_dir,
            temp.path().join("does_not_exist"),
            None,
            vec![],
            false
        ),
        Err(TrackerError::NotFound {
            kind: "ProjectDirectory",
            ..
        })
    ));
    assert!(matches!(
        set_active_project_in(&cfg_dir, "missing_project"),
        Err(TrackerError::NotFound {
            kind: "Project",
            ..
        })
    ));
    assert!(matches!(
        remove_project_in(&cfg_dir, "missing_project"),
        Err(TrackerError::NotFound {
            kind: "Project",
            ..
        })
    ));
}

#[test]
fn test_two_stage_directory_ingestion_gitignore_filtering_and_scope_promotion() {
    use backend_core::{IngestionOptions, COMPONENT_MANIFEST_FILE_NAME};

    let temp = TempTrackerDir::new("ingestion_two_stage");
    let root = temp.path();

    let storage = TrackerStorage::init(
        root,
        "PolyglotWorkspace",
        "01_foundation",
        "Foundation Phase",
    )
    .unwrap();

    // Write `.gitignore` (note: no `.git` directory created in temp dir, testing `require_git(false)`)
    fs::write(
        root.join(".gitignore"),
        "ignored_secret.txt\n*.tmp.log\nignored_dir/\ncrates/core_lib/generated/\n",
    )
    .unwrap();

    // Write files that MUST be ignored by `.gitignore` or default skip list (`target/`, `node_modules/`, `.venv/`, etc.)
    fs::write(root.join("ignored_secret.txt"), "top secret").unwrap();
    fs::write(root.join("debug.tmp.log"), "log data").unwrap();
    fs::create_dir_all(root.join("ignored_dir")).unwrap();
    fs::write(root.join("ignored_dir/skip_me.rs"), "fn skip() {}").unwrap();
    for skipped_vendor in [
        "target/debug",
        "node_modules/pkg",
        "dist/bundle",
        "build/out",
        "__pycache__",
        ".venv/lib",
    ] {
        let dir = root.join(skipped_vendor);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("artifact.txt"), "skip").unwrap();
    }

    // 1. Rust library crate (`crates/core_lib`) with intentionally non-parseable Rust syntax to
    // prove ingestion captures file lists without AST parsing.
    let core_lib_dir = root.join("crates/core_lib/src");
    fs::create_dir_all(&core_lib_dir).unwrap();
    fs::write(
        root.join("crates/core_lib/Cargo.toml"),
        "[package]\nname = \"core_lib\"\n",
    )
    .unwrap();
    fs::write(
        core_lib_dir.join("lib.rs"),
        "@@@ NOT_VALID_AST_SYNTAX {{{ <<<",
    )
    .unwrap();
    fs::create_dir_all(root.join("crates/core_lib/generated")).unwrap();
    fs::write(
        root.join("crates/core_lib/generated/ignored_gen.rs"),
        "// ignored by .gitignore",
    )
    .unwrap();

    // 2. Rust CLI crate (`crates/neb_cli`)
    let cli_dir = root.join("crates/neb_cli/src");
    fs::create_dir_all(&cli_dir).unwrap();
    fs::write(
        root.join("crates/neb_cli/Cargo.toml"),
        "[package]\nname = \"neb_cli\"\n",
    )
    .unwrap();
    fs::write(cli_dir.join("main.rs"), "fn main() {}").unwrap();

    // 3. Python package (`services/agent_worker`)
    let py_dir = root.join("services/agent_worker");
    fs::create_dir_all(&py_dir).unwrap();
    fs::write(
        py_dir.join("pyproject.toml"),
        "[project]\nname = \"agent_worker\"\n",
    )
    .unwrap();
    fs::write(py_dir.join("worker.py"), "def run(): pass\n").unwrap();

    // 4. Un-manifested top-level functional directories: `scripts/`, `iso/`, `config/`
    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::write(root.join("scripts/build-iso.sh"), "#!/usr/bin/env bash\n").unwrap();
    fs::write(root.join("scripts/verify-virt.sh"), "#!/usr/bin/env bash\n").unwrap();

    fs::create_dir_all(root.join("iso")).unwrap();
    fs::write(
        root.join("iso/packages.sid.list"),
        "systemd\nlinux-image-custom\n",
    )
    .unwrap();

    fs::create_dir_all(root.join("config")).unwrap();
    fs::write(
        root.join("config/apt.conf"),
        "APT::Default-Release \"sid\";\n",
    )
    .unwrap();

    // Run Stage 1 ingestion with `write_manifests = true`
    let report = storage
        .ingest_project_directory(IngestionOptions::new().with_write_manifests(true))
        .expect("ingest_project_directory should succeed");

    // Expected components: agent_worker, config, core_lib, iso, neb_cli, scripts
    let comp_ids: Vec<String> = report.components.iter().map(|c| c.id.clone()).collect();
    assert_eq!(
        comp_ids,
        vec![
            "agent_worker".to_string(),
            "config".to_string(),
            "core_lib".to_string(),
            "iso".to_string(),
            "neb_cli".to_string(),
            "scripts".to_string(),
        ]
    );
    assert_eq!(report.manifests_written.len(), 6);

    // Verify `.gitignore` and skipped directories were strictly excluded
    for comp in &report.components {
        for f in &comp.files {
            assert!(!f.contains("ignored_secret"));
            assert!(!f.contains(".tmp.log"));
            assert!(!f.contains("ignored_dir"));
            assert!(!f.contains("ignored_gen.rs"));
            assert!(!f.contains("target/"));
            assert!(!f.contains("node_modules/"));
            assert!(!f.contains(".venv/"));
            assert!(!f.contains(".tracker/"));
        }
    }

    // Verify component types and relative file lists in `project.toml`
    let project = storage.read_project().unwrap();
    assert_eq!(project.components.len(), 6);

    let core_comp = project.components.find("core_lib").unwrap();
    assert_eq!(core_comp.component_type, "rust_crate");
    assert_eq!(core_comp.relative_path, "crates/core_lib");
    assert_eq!(
        core_comp.files,
        vec![
            "crates/core_lib/Cargo.toml".to_string(),
            "crates/core_lib/src/lib.rs".to_string(),
        ]
    );

    let cli_comp = project.components.find("neb_cli").unwrap();
    assert_eq!(cli_comp.component_type, "cli_crate");

    let py_comp = project.components.find("agent_worker").unwrap();
    assert_eq!(py_comp.component_type, "python_pkg");
    assert!(py_comp.tech_stack.contains(&"python".to_string()));

    let scripts_comp = project.components.find("scripts").unwrap();
    assert_eq!(scripts_comp.component_type, "build_scripts");
    assert_eq!(
        scripts_comp.files,
        vec![
            "scripts/build-iso.sh".to_string(),
            "scripts/verify-virt.sh".to_string(),
        ]
    );
    assert!(root
        .join("scripts")
        .join(COMPONENT_MANIFEST_FILE_NAME)
        .is_file());

    let iso_comp = project.components.find("iso").unwrap();
    assert_eq!(iso_comp.component_type, "os_image");
    assert_eq!(iso_comp.files, vec!["iso/packages.sid.list".to_string()]);

    // Stage 2: Promote `scripts` component to a phase domain scope with a selected file subset
    let promoted_scripts = storage
        .promote_component_to_scope(
            Some("01_foundation"),
            "scripts",
            Some("release_scripts"),
            Some(vec!["scripts/build-iso.sh".to_string()]),
        )
        .expect("should promote scripts component to scope");
    assert_eq!(promoted_scripts.scope.id, "release_scripts");
    assert_eq!(promoted_scripts.scope.target, "scripts");
    assert_eq!(
        promoted_scripts.scope.bound_component.as_deref(),
        Some("scripts")
    );
    assert_eq!(
        promoted_scripts.scope.tracked_files,
        vec!["scripts/build-iso.sh".to_string()]
    );

    // Promote `core_lib` with default all files and default scope ID
    let promoted_core = storage
        .promote_component_to_scope(None, "core_lib", None, None)
        .expect("should promote core_lib to active phase scope");
    assert_eq!(promoted_core.scope.id, "core_lib");
    assert_eq!(promoted_core.scope.target, "crates/core_lib");
    assert_eq!(
        promoted_core.scope.bound_component.as_deref(),
        Some("core_lib")
    );
    assert_eq!(
        promoted_core.scope.tracked_files,
        vec![
            "crates/core_lib/Cargo.toml".to_string(),
            "crates/core_lib/src/lib.rs".to_string(),
        ]
    );

    let phase_manifest = storage.read_phase("01_foundation").unwrap();
    assert!(phase_manifest
        .scopes
        .manifest
        .iter()
        .any(|s| s.id == "release_scripts" && s.scope_type == "build_scripts"));
    assert!(phase_manifest
        .scopes
        .manifest
        .iter()
        .any(|s| s.id == "core_lib" && s.scope_type == "rust_crate"));
}

#[test]
fn test_orbit_component_manifest_pinning_rolling_updates_and_rollback() {
    use backend_core::{
        ComponentDependency, ComponentManifest, IngestionOptions, RollingUpdateEntry,
        VerificationRecord,
    };

    let temp = TempTrackerDir::new("orbit_pinning");
    let root = temp.path();

    let storage = TrackerStorage::init(
        root,
        "Orbit",
        "01_rolling_sid",
        "Orbit Base & Custom Rolling Release",
    )
    .unwrap();

    fs::create_dir_all(root.join("iso")).unwrap();
    fs::write(root.join("iso/live-build.sh"), "#!/usr/bin/env bash\n").unwrap();
    fs::write(
        root.join("iso/packages.list"),
        "linux-image-custom\nsystemd\n",
    )
    .unwrap();

    // 1. Author a `nebulous.component.toml` for Orbit ISO image tracking a rolling base + custom kernel
    let mut orbit_iso = ComponentManifest::new("orbit_iso", "Orbit Live ISO", "iso", "os_image");
    orbit_iso.tech_stack = vec![
        "rolling_base".to_string(),
        "custom".to_string(),
        "shell".to_string(),
    ];
    orbit_iso.tags = vec!["rolling".to_string(), "bootable".to_string()];
    orbit_iso.flags = vec!["--arch=amd64".to_string(), "--comp=zstd".to_string()];
    orbit_iso.files = vec![
        "iso/live-build.sh".to_string(),
        "iso/packages.list".to_string(),
    ];
    orbit_iso.dependencies = vec![
        ComponentDependency::new(
            "linux-image-6.11-custom1",
            "kernel_custom",
            ">=6.11.0",
            "6.11.4-custom1",
            "https://packages.example.com/kernel",
        ),
        ComponentDependency::new(
            "systemd",
            "apt_sid",
            ">=256",
            "256.7-1",
            "http://packages.example.com/base rolling",
        ),
    ];

    let manifest_path = storage
        .write_component_manifest(&orbit_iso)
        .expect("should write nebulous.component.toml and sync project.toml");
    assert!(manifest_path.is_file());

    // 2. Pin known-good release (`last_known_good`) after passing VM verification
    let initial_verification = VerificationRecord {
        passed: true,
        exit_code: 0,
        command: "./scripts/verify-virt.sh --iso build/orbit.iso".to_string(),
        summary: "VM UEFI boot + systemd multi-user target reached in 4.2s".to_string(),
        logs_excerpt: Some("Orbit login: root (automatic login)".to_string()),
        verified_at: Some("2026-11-03T12:00:00Z".to_string()),
    };

    let pinned_manifest = storage
        .pin_component_release(
            "orbit_iso",
            Some("a1b2c3d"),
            Some("iso/live-build.sh".to_string()),
            vec!["--arch=amd64".to_string(), "--comp=zstd".to_string()],
            Some(initial_verification),
        )
        .expect("should pin stable release");

    let pin = pinned_manifest
        .pinned_release
        .as_ref()
        .expect("pinned_release should be populated");
    assert_eq!(pin.commit_sha, "a1b2c3d");
    assert_eq!(pin.build_script_path.as_deref(), Some("iso/live-build.sh"));
    assert_eq!(
        pin.dependency_pins
            .get("linux-image-6.11-custom1")
            .map(String::as_str),
        Some("6.11.4-custom1")
    );
    assert_eq!(
        pin.dependency_pins.get("systemd").map(String::as_str),
        Some("256.7-1")
    );
    assert!(pin.last_verification_run.as_ref().unwrap().passed);

    // 3. Simulate an upstream rolling update bumping custom kernel to 6.11.8-custom1
    let updated_manifest = storage
        .record_component_rolling_update(
            "orbit_iso",
            RollingUpdateEntry {
                dependency_name: "linux-image-6.11-custom1".to_string(),
                kind: "kernel_custom".to_string(),
                previous_version: None,
                new_version: "6.11.8-custom1".to_string(),
                upstream_source: Some("https://packages.example.com/kernel".to_string()),
                build_script_path: Some("iso/live-build.sh".to_string()),
                tracked_item_id: Some("BUG-0001".to_string()),
                status: "pending".to_string(),
                recorded_at: Some("2026-11-03T13:00:00Z".to_string()),
                summary: Some("Bump custom kernel to 6.11.8-custom1".to_string()),
                verification_run: None,
            },
        )
        .expect("should record rolling update");
    assert_eq!(
        updated_manifest.dependencies[0].current_version,
        "6.11.8-custom1"
    );
    assert_eq!(
        updated_manifest.rolling_updates[0]
            .previous_version
            .as_deref(),
        Some("6.11.4-custom1")
    );

    // 4. Record a failed VM verification run for the rolling update
    let after_fail = storage
        .record_component_verification(
            "orbit_iso",
            VerificationRecord {
                passed: false,
                exit_code: 1,
                command: "./scripts/verify-virt.sh --iso build/orbit.iso".to_string(),
                summary: "Initramfs dracut module missing for 6.11.8-custom1".to_string(),
                logs_excerpt: Some(
                    "Kernel panic - not syncing: VFS: Unable to mount root fs".to_string(),
                ),
                verified_at: Some("2026-11-03T13:05:00Z".to_string()),
            },
        )
        .expect("should record failed verification");
    assert_eq!(after_fail.rolling_updates[0].status, "failed");
    // Pinned release's last passing verification run must remain intact
    assert!(
        after_fail
            .pinned_release
            .as_ref()
            .unwrap()
            .last_verification_run
            .as_ref()
            .unwrap()
            .passed
    );

    // 5. Roll back to `last_known_good` pin
    let rolled_back = storage
        .rollback_component_to_pinned("orbit_iso")
        .expect("should roll back component to pinned_release");
    assert_eq!(
        rolled_back.dependencies[0].current_version,
        "6.11.4-custom1"
    );
    assert_eq!(rolled_back.rolling_updates[0].status, "rolled_back");

    // 6. Re-running `ingest_project_directory` preserves the existing `nebulous.component.toml`
    // pins, dependencies, and rolling updates while updating discovered files!
    fs::write(root.join("iso/grub.cfg"), "set timeout=5\n").unwrap();
    let reingest = storage
        .ingest_project_directory(IngestionOptions::new())
        .expect("re-ingestion should preserve nebulous.component.toml metadata");
    let reingested_iso = reingest
        .components
        .iter()
        .find(|c| c.id == "orbit_iso")
        .expect("should find orbit_iso component");
    assert_eq!(reingested_iso.dependencies.len(), 2);
    assert!(reingested_iso.pinned_release.is_some());
    assert_eq!(reingested_iso.rolling_updates.len(), 1);
    assert!(reingested_iso.files.contains(&"iso/grub.cfg".to_string()));

    // 7. Reject path traversal in write_component_manifest and promote_component_to_scope
    let mut bad_comp = ComponentManifest::new("bad_comp", "Bad", "../escape", "custom");
    assert!(storage.write_component_manifest(&bad_comp).is_err());

    bad_comp.relative_path = "iso".to_string();
    bad_comp.files = vec!["../outside.sh".to_string()];
    assert!(storage.write_component_manifest(&bad_comp).is_err());

    assert!(storage
        .promote_component_to_scope(
            None,
            "orbit_iso",
            None,
            Some(vec!["../escape_file.sh".to_string()]),
        )
        .is_err());
}

#[test]
fn test_ingestion_subpath_duplicate_leaf_ids_and_promote_all() {
    use backend_core::{
        load_global_registry, register_project, remove_project, save_global_registry,
        set_active_project, IngestionOptions,
    };

    let temp = TempTrackerDir::new("subpath_and_dup_ids");
    let root = temp.path().join("workspace");
    fs::create_dir_all(&root).unwrap();

    let storage = TrackerStorage::init(&root, "DupLeafWorkspace", "01_init", "Init Phase").unwrap();

    // Two directories with the same leaf name `utils`: `crates/utils` and `packages/utils`
    fs::create_dir_all(root.join("crates/utils/src")).unwrap();
    fs::write(
        root.join("crates/utils/Cargo.toml"),
        "[package]\nname = \"utils\"\n",
    )
    .unwrap();
    fs::write(root.join("crates/utils/src/lib.rs"), "pub fn a() {}").unwrap();

    fs::create_dir_all(root.join("packages/utils")).unwrap();
    fs::write(
        root.join("packages/utils/package.json"),
        "{\"name\": \"utils\"}\n",
    )
    .unwrap();
    fs::write(root.join("packages/utils/index.ts"), "export const b = 1;").unwrap();

    // First scan only `crates/utils` via `subpath`, then scan `packages/utils` via `subpath` with `promote_all`
    let rep1 = storage
        .ingest_project_directory(IngestionOptions::new().with_subpath("crates/utils"))
        .unwrap();
    assert_eq!(rep1.components.len(), 1);
    assert_eq!(rep1.components[0].id, "utils");

    let rep2 = storage
        .ingest_project_directory(
            IngestionOptions::new()
                .with_subpath("packages/utils")
                .with_promote_all(true),
        )
        .unwrap();
    assert_eq!(rep2.components.len(), 1);
    assert_eq!(rep2.components[0].id, "packages_utils");
    assert_eq!(rep2.promoted_scopes, vec!["packages_utils".to_string()]);

    // Both `utils` and `packages_utils` must coexist in `project.toml`
    let project = storage.read_project().unwrap();
    assert_eq!(project.components.len(), 2);
    assert!(project.components.find("utils").is_some());
    assert!(project.components.find("packages_utils").is_some());

    // Also exercise the top-level global registry functions via NEBULOUS_CONFIG_DIR
    let cfg_dir = temp.path().join("env_global_cfg");
    std::env::set_var("NEBULOUS_CONFIG_DIR", &cfg_dir);
    let reg_entry = register_project(
        &root,
        Some("DupLeafWorkspace"),
        vec!["polyglot".into()],
        true,
    )
    .unwrap();
    assert_eq!(reg_entry.id, "dupleafworkspace");
    let loaded_reg = load_global_registry().unwrap();
    assert_eq!(
        loaded_reg.active_project_id.as_deref(),
        Some("dupleafworkspace")
    );
    save_global_registry(&loaded_reg).unwrap();
    let switched = set_active_project("dupleafworkspace").unwrap();
    assert_eq!(switched.id, "dupleafworkspace");
    let removed = remove_project("dupleafworkspace").unwrap();
    assert_eq!(removed.id, "dupleafworkspace");
    assert!(load_global_registry().unwrap().projects.is_empty());
    std::env::remove_var("NEBULOUS_CONFIG_DIR");
}

#[test]
fn test_component_manifest_table_header_merges_top_level_sections() {
    use backend_core::ComponentManifest;

    let raw = r#"
[component]
id = "orbit_kernel"
name = "Orbit Custom Kernel"
path = "kernel"
type = "kernel_module"
tech_stack = ["c_cpp", "kbuild"]
tags = ["custom", "sid"]
flags = ["LLVM=1"]
files = ["kernel/Kconfig", "kernel/Makefile"]

[[dependencies]]
name = "linux-source-6.11-custom1"
kind = "kernel_custom"
version_constraint = ">=6.11.0"
current_version = "6.11.4-custom1"
upstream_source = "https://packages.example.com/kernel"

[last_known_good]
commit_sha = "deadbeef"
pinned_at = "2026-11-03T12:00:00Z"
build_script_path = "kernel/build.sh"
build_flags = ["LLVM=1", "-j16"]

[last_known_good.dependency_pins]
linux-source-6.11-custom1 = "6.11.4-custom1"

[[rolling_updates]]
name = "linux-source-6.11-custom1"
kind = "kernel_custom"
from_version = "6.11.4-custom1"
to_version = "6.11.8-custom1"
status = "passed"
"#;

    let parsed = ComponentManifest::from_toml_str(raw)
        .expect("should parse [component]-wrapped manifest with top-level sections");
    assert_eq!(parsed.id, "orbit_kernel");
    assert_eq!(parsed.relative_path, "kernel");
    assert_eq!(parsed.component_type, "kernel_module");
    assert_eq!(parsed.dependencies.len(), 1);
    assert_eq!(parsed.dependencies[0].name, "linux-source-6.11-custom1");
    let pin = parsed
        .pinned_release
        .as_ref()
        .expect("pinned_release should be populated");
    assert_eq!(pin.commit_sha, "deadbeef");
    assert_eq!(
        pin.dependency_pins
            .get("linux-source-6.11-custom1")
            .map(String::as_str),
        Some("6.11.4-custom1")
    );
    assert_eq!(parsed.rolling_updates.len(), 1);
    assert_eq!(parsed.rolling_updates[0].new_version, "6.11.8-custom1");

    // Verify that when `[component]` is present and invalid inside a nested section,
    // `from_toml_str` returns the `WrappedComponent` error rather than `missing field id`.
    let invalid_wrapped = r#"
[component]
id = "orbit_kernel"
name = "Orbit Custom Kernel"

[last_known_good.dependency_pins]
bad_pin = ["not", "a", "string"]
"#;
    let err = ComponentManifest::from_toml_str(invalid_wrapped).unwrap_err();
    assert!(
        err.to_string().contains("bad_pin"),
        "expected WrappedComponent error mentioning bad_pin, got: {err}"
    );
}

#[test]
fn test_write_component_manifest_auto_populates_files_and_reingest_prunes_deleted_dirs() {
    use backend_core::{ComponentManifest, IngestionOptions};

    let temp = TempTrackerDir::new("auto_files_and_prune");
    let root = temp.path();

    let storage = TrackerStorage::init(root, "Orbit", "01_init", "Init Phase").unwrap();
    fs::write(root.join(".gitignore"), "*.bak\n").unwrap();

    fs::create_dir_all(root.join("scripts")).unwrap();
    fs::write(root.join("scripts/build.sh"), "#!/bin/sh\n").unwrap();
    fs::write(root.join("scripts/old.bak"), "ignored\n").unwrap();

    // 1. Writing a ComponentManifest with empty `files` auto-discovers files in `scripts/`
    let manifest = ComponentManifest::new("scripts", "Build Scripts", "scripts", "build_scripts");
    assert!(manifest.files.is_empty());
    storage.write_component_manifest(&manifest).unwrap();

    let loaded = storage.read_component_manifest("scripts").unwrap();
    assert_eq!(loaded.files, vec!["scripts/build.sh".to_string()]);

    // 2. Add a temporary un-manifested directory `temp_tools/`, ingest, then delete `temp_tools/`
    fs::create_dir_all(root.join("temp_tools")).unwrap();
    fs::write(root.join("temp_tools/run.py"), "print(1)\n").unwrap();

    let rep1 = storage
        .ingest_project_directory(IngestionOptions::new())
        .unwrap();
    assert_eq!(rep1.components.len(), 2);
    assert!(storage
        .read_project()
        .unwrap()
        .components
        .find("temp_tools")
        .is_some());

    // Delete `temp_tools/` and re-ingest full project -> `temp_tools` is pruned from `project.toml`
    fs::remove_dir_all(root.join("temp_tools")).unwrap();
    let rep2 = storage
        .ingest_project_directory(IngestionOptions::new())
        .unwrap();
    assert_eq!(rep2.components.len(), 1);
    assert!(storage
        .read_project()
        .unwrap()
        .components
        .find("temp_tools")
        .is_none());
}

#[test]
fn test_nested_monorepo_unmanifested_parent_with_manifested_child_and_root_manifest() {
    use backend_core::IngestionOptions;

    let temp = TempTrackerDir::new("nested_monorepo_hybrid");
    let root = temp.path();

    let storage = TrackerStorage::init(root, "HybridRepo", "01_init", "Init Phase").unwrap();

    // Root package with `Cargo.toml` + `src/lib.rs`, plus a nested crate `crates/sub_crate/Cargo.toml`
    // AND an un-manifested `scripts/` directory that also contains a nested manifested `scripts/helper_tool/Cargo.toml`.
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"hybrid_root\"\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub fn root_fn() {}").unwrap();

    fs::create_dir_all(root.join("crates/sub_crate/src")).unwrap();
    fs::write(
        root.join("crates/sub_crate/Cargo.toml"),
        "[package]\nname = \"sub_crate\"\n",
    )
    .unwrap();
    fs::write(root.join("crates/sub_crate/src/lib.rs"), "pub fn sub() {}").unwrap();

    fs::create_dir_all(root.join("scripts/helper_tool/src")).unwrap();
    fs::write(root.join("scripts/top_level.sh"), "#!/bin/sh\n").unwrap();
    fs::write(
        root.join("scripts/helper_tool/Cargo.toml"),
        "[package]\nname = \"helper_tool\"\n",
    )
    .unwrap();
    fs::write(root.join("scripts/helper_tool/src/main.rs"), "fn main() {}").unwrap();

    let report = storage
        .ingest_project_directory(IngestionOptions::new())
        .expect("should ingest hybrid repo");

    let proj = storage.read_project().unwrap();
    // Expect 4 components:
    // - `.` (hybridrepo) with `Cargo.toml` and `src/lib.rs`
    // - `crates/sub_crate` (sub_crate)
    // - `scripts` with `scripts/top_level.sh`
    // - `scripts/helper_tool` (helper_tool)
    assert_eq!(report.components.len(), 4);

    let root_comp = proj
        .components
        .find(".")
        .expect("should find root component");
    assert_eq!(
        root_comp.files,
        vec!["Cargo.toml".to_string(), "src/lib.rs".to_string()]
    );

    let scripts_comp = proj
        .components
        .find("scripts")
        .expect("should find top-level scripts component");
    assert_eq!(scripts_comp.files, vec!["scripts/top_level.sh".to_string()]);

    let helper_comp = proj
        .components
        .find("helper_tool")
        .expect("should find nested helper_tool component");
    assert_eq!(
        helper_comp.files,
        vec![
            "scripts/helper_tool/Cargo.toml".to_string(),
            "scripts/helper_tool/src/main.rs".to_string(),
        ]
    );
}

#[test]
fn test_container_sibling_dirs_id_stability_subpath_and_hand_edited_manifests() {
    use backend_core::{
        register_project_in, set_active_project_in, ComponentManifest, IngestionOptions,
        COMPONENT_MANIFEST_FILE_NAME,
    };

    let temp = TempTrackerDir::new("edge_cases_step2");
    let root = temp.path().join("monorepo");
    fs::create_dir_all(&root).unwrap();

    let storage = TrackerStorage::init(&root, "EdgeMonorepo", "01_init", "Init Phase").unwrap();

    // 1. Ingest `packages/utils` first so it receives `id = "utils"`.
    fs::create_dir_all(root.join("packages/utils")).unwrap();
    fs::write(
        root.join("packages/utils/package.json"),
        "{\"name\": \"utils\"}\n",
    )
    .unwrap();
    fs::write(
        root.join("packages/utils/index.ts"),
        "export const v = 1;\n",
    )
    .unwrap();

    let first_rep = storage
        .ingest_project_directory(IngestionOptions::new().with_subpath("packages/utils"))
        .unwrap();
    assert_eq!(first_rep.components.len(), 1);
    assert_eq!(first_rep.components[0].id, "utils");
    assert_eq!(first_rep.components[0].relative_path, "packages/utils");

    // 2. Now add `crates/utils` (which sorts BEFORE `packages/utils` alphabetically) AND
    // an un-manifested sibling directory `packages/live_hooks/01-setup.sh` inside the `packages/` container.
    fs::create_dir_all(root.join("crates/utils/src")).unwrap();
    fs::write(
        root.join("crates/utils/Cargo.toml"),
        "[package]\nname = \"utils\"\n",
    )
    .unwrap();
    fs::write(root.join("crates/utils/src/lib.rs"), "pub fn c() {}\n").unwrap();

    fs::create_dir_all(root.join("packages/live_hooks")).unwrap();
    fs::write(
        root.join("packages/live_hooks/01-setup.sh"),
        "#!/usr/bin/env bash\n",
    )
    .unwrap();

    // Full workspace ingestion:
    // - `packages/utils` MUST keep its existing `id = "utils"` (not stolen by `crates/utils`!)
    // - `crates/utils` MUST receive disambiguated `id = "crates_utils"`
    // - `packages/live_hooks` MUST be discovered as its own component at `packages/live_hooks`
    let full_rep = storage
        .ingest_project_directory(IngestionOptions::new())
        .unwrap();
    assert_eq!(full_rep.components.len(), 3);

    let proj = storage.read_project().unwrap();
    let pkg_utils = proj
        .components
        .find("packages/utils")
        .expect("should find packages/utils");
    assert_eq!(
        pkg_utils.id, "utils",
        "existing component id must not be stolen by alphabetically earlier new directory"
    );

    let crate_utils = proj
        .components
        .find("crates/utils")
        .expect("should find crates/utils");
    assert_eq!(crate_utils.id, "crates_utils");

    let live_hooks = proj
        .components
        .find("packages/live_hooks")
        .expect("should discover un-manifested sibling inside container directory");
    assert_eq!(live_hooks.id, "live_hooks");
    assert_eq!(
        live_hooks.files,
        vec!["packages/live_hooks/01-setup.sh".to_string()]
    );

    // 3. Subpath ingestion on a nested un-manifested directory (`scripts/virt`)
    fs::create_dir_all(root.join("scripts/virt")).unwrap();
    fs::write(
        root.join("scripts/virt/boot_check.sh"),
        "#!/usr/bin/env bash\n",
    )
    .unwrap();

    let sub_rep = storage
        .ingest_project_directory(IngestionOptions::new().with_subpath("scripts/virt"))
        .unwrap();
    assert_eq!(sub_rep.components.len(), 1);
    assert_eq!(sub_rep.components[0].relative_path, "scripts/virt");
    assert_eq!(sub_rep.components[0].id, "virt");

    // 4. Hand-edited `iso/nebulous.component.toml` omitting `id` and `path`, with unquoted TOML
    // datetimes and `[last_known_good]` / `dependency_versions` / `verification_status` aliases
    fs::create_dir_all(root.join("iso")).unwrap();
    fs::write(root.join("iso/build.sh"), "#!/bin/sh\n").unwrap();
    let hand_edited_toml = r#"
[component]
name = "Orbit Live ISO"
type = "os_image"

[[dependencies]]
package = "linux-image-custom"
channel = "kernel_custom"
constraint = ">=6.11"
version = "6.11.4-custom1"
source = "https://packages.example.com/kernel"

[last_known_good]
commit_sha = "abc1234"
pinned_at = 2026-11-03T14:30:00Z
build_flags = ["--uefi"]

[last_known_good.dependency_versions]
linux-image-custom = "6.11.4-custom1"

[last_known_good.verification_status]
passed = true
exit_code = 0
command = "virt-system-x86_64"
summary = "Booted cleanly"
verified_at = 2026-11-03T14:29:00Z

[[rolling_updates]]
package = "linux-image-custom"
kind = "kernel_custom"
from_version = "6.11.2-custom1"
to_version = "6.11.4-custom1"
status = "passed"
recorded_at = 2026-11-03T14:25:00Z
"#;
    fs::write(
        root.join("iso").join(COMPONENT_MANIFEST_FILE_NAME),
        hand_edited_toml,
    )
    .unwrap();

    // Reading directly by relative directory path (`"iso"`) before ingestion must populate `id = "orbit_live_iso"`
    // and `relative_path = "iso"` (not `"."`) and parse unquoted TOML timestamps!
    let direct_iso = storage
        .read_component_manifest("iso")
        .expect("should read hand-edited iso/nebulous.component.toml");
    assert_eq!(direct_iso.id, "orbit_live_iso");
    assert_eq!(direct_iso.relative_path, "iso");
    assert_eq!(direct_iso.files, vec!["iso/build.sh".to_string()]);
    let pin = direct_iso.pinned_release.as_ref().unwrap();
    assert!(pin.parsed_pinned_at().is_ok());
    assert!(pin
        .last_verification_run
        .as_ref()
        .unwrap()
        .parsed_verified_at()
        .unwrap()
        .is_some());
    assert!(direct_iso.rolling_updates[0]
        .parsed_recorded_at()
        .unwrap()
        .is_some());

    // Reject completely empty TOML without id or name
    assert!(ComponentManifest::from_toml_str("tech_stack = [\"rust\"]\n").is_err());

    // 5. GlobalProjectRegistry lookup priority: exact ID over display name, trailing slash path,
    // and rejecting regular files in `register_project_in`
    let cfg_dir = temp.path().join("reg_cfg");
    let file_path = temp.path().join("not_a_dir.txt");
    fs::write(&file_path, "hello").unwrap();
    assert!(register_project_in(&cfg_dir, &file_path, None, vec![], false).is_err());

    let dir_1 = temp.path().join("proj_one");
    let dir_2 = temp.path().join("proj_two");
    fs::create_dir_all(&dir_1).unwrap();
    fs::create_dir_all(&dir_2).unwrap();

    // Register dir_1 with name "alpha" (id = "alpha"), then rename its display name to "beta"
    register_project_in(&cfg_dir, &dir_1, Some("alpha"), vec![], false).unwrap();
    register_project_in(&cfg_dir, &dir_1, Some("beta"), vec![], false).unwrap();
    // Register dir_2 with name "beta" -> gets id = "beta"
    let reg_2 = register_project_in(&cfg_dir, &dir_2, Some("beta"), vec![], false).unwrap();
    assert_eq!(reg_2.id, "beta");

    // Searching for "beta" must prioritize Project 2's exact ID `id == "beta"` over Project 1's `name == "beta"`!
    let active = set_active_project_in(&cfg_dir, "beta").unwrap();
    assert_eq!(active.id, "beta");

    // Switching by path with trailing slash must also resolve dir_1 (`id == "alpha"`)
    let dir_1_with_slash = format!("{}/", dir_1.display());
    let active_by_path = set_active_project_in(&cfg_dir, &dir_1_with_slash).unwrap();
    assert_eq!(active_by_path.id, "alpha");
}

#[test]
fn test_safe_project_root_validation_poisoned_registry_pruning_and_hidden_dir_skip() {
    use backend_core::{
        is_unsafe_broad_directory, load_global_registry_from, register_project_in,
        validate_safe_project_root, GlobalProjectRegistry, IngestionOptions,
    };

    // 1. Verify filesystem root, system directories, /tmp root, and $HOME are rejected
    assert!(is_unsafe_broad_directory(Path::new("/")));
    assert!(is_unsafe_broad_directory(Path::new("/.tracker")));
    assert!(is_unsafe_broad_directory(Path::new("/etc")));
    assert!(is_unsafe_broad_directory(Path::new("/usr")));
    assert!(is_unsafe_broad_directory(Path::new("/home")));
    assert!(is_unsafe_broad_directory(Path::new("/tmp")));
    assert!(is_unsafe_broad_directory(&std::env::temp_dir()));
    assert!(validate_safe_project_root(Path::new("/")).is_err());
    assert!(validate_safe_project_root(Path::new("/tmp")).is_err());
    assert!(TrackerStorage::init(Path::new("/"), "Root", "01_init", "Init").is_err());
    assert!(TrackerStorage::init(Path::new("/tmp"), "TmpRoot", "01_init", "Init").is_err());

    if let Some(home_os) = std::env::var_os("HOME") {
        let home_path = PathBuf::from(home_os);
        if !home_path.starts_with(std::env::temp_dir()) {
            assert!(is_unsafe_broad_directory(&home_path));
            assert!(is_unsafe_broad_directory(&home_path.join(".tracker")));
            assert!(validate_safe_project_root(&home_path).is_err());
            assert!(TrackerStorage::init(&home_path, "HomeBad", "01_init", "Init").is_err());
        }
    }

    // 2. Verify GlobalProjectRegistry refuses to register unsafe broad roots and prunes legacy
    // poisoned entries (`root_path = "/home"` or `$HOME` or `/`) on load while falling back `active_project_id`
    let temp = TempTrackerDir::new("safe_root_and_pruning");
    let cfg_dir = temp.path().join("cfg");
    fs::create_dir_all(&cfg_dir).unwrap();
    let valid_proj = temp.path().join("valid_workspace");
    fs::create_dir_all(&valid_proj).unwrap();
    let storage =
        TrackerStorage::init(&valid_proj, "ValidWorkspace", "01_init", "Init Phase").unwrap();

    assert!(register_project_in(&cfg_dir, Path::new("/"), None, vec![], true).is_err());
    assert!(register_project_in(&cfg_dir, Path::new("/tmp"), None, vec![], true).is_err());

    let poisoned_toml = format!(
        r#"active_project_id = "poisoned_home"

[[projects]]
id = "poisoned_home"
name = "theinfomoose"
root_path = "/home"
tracker_path = "/home/.tracker"
default_branch = "main"
last_opened_at = "2026-11-04T00:00:00Z"

[[projects]]
id = "valid_workspace"
name = "ValidWorkspace"
root_path = "{}"
tracker_path = "{}"
default_branch = "main"
last_opened_at = "2026-11-04T00:00:00Z"
"#,
        valid_proj.display(),
        valid_proj.join(".tracker").display()
    );
    fs::write(cfg_dir.join("projects.toml"), &poisoned_toml).unwrap();

    let loaded_reg = load_global_registry_from(&cfg_dir).expect("should load and prune registry");
    assert_eq!(loaded_reg.projects.len(), 1);
    assert_eq!(loaded_reg.projects[0].id, "valid_workspace");
    assert_eq!(
        loaded_reg.active_project_id.as_deref(),
        Some("valid_workspace")
    );
    let persisted_raw = fs::read_to_string(cfg_dir.join("projects.toml")).unwrap();
    assert!(
        !persisted_raw.contains("poisoned_home"),
        "poisoned unsafe entry must be pruned on disk as well as in memory"
    );

    let parsed_reg = GlobalProjectRegistry::from_toml_str(&poisoned_toml).unwrap();
    assert_eq!(parsed_reg.projects.len(), 1);
    assert_eq!(
        parsed_reg.active_project_id.as_deref(),
        Some("valid_workspace")
    );

    // 3. Verify `ingest_project_directory` refuses to scan an unsafe broad root (e.g. `TrackerStorage::new("/tmp")`)
    // and skips hidden dot-directories (`.cargo`, `.cache`, `.local`, `.config`) inside a valid project
    let unsafe_storage = TrackerStorage::new(Path::new("/tmp"));
    assert!(unsafe_storage
        .ingest_project_directory(IngestionOptions::new())
        .is_err());

    for hidden_dir in [".cargo/bin", ".cache/pip", ".local/share", ".config/nvim"] {
        let p = valid_proj.join(hidden_dir);
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("Cargo.toml"), "[package]\nname = \"hidden_pkg\"\n").unwrap();
        fs::write(p.join("secret.rs"), "pub fn secret() {}\n").unwrap();
    }
    fs::create_dir_all(valid_proj.join("crates/app/src")).unwrap();
    fs::write(
        valid_proj.join("crates/app/Cargo.toml"),
        "[package]\nname = \"app\"\n",
    )
    .unwrap();
    fs::write(valid_proj.join("crates/app/src/main.rs"), "fn main() {}\n").unwrap();

    let report = storage
        .ingest_project_directory(IngestionOptions::new())
        .expect("should ingest valid_proj while skipping hidden dot-directories");
    assert_eq!(report.components.len(), 1);
    assert_eq!(report.components[0].id, "app");
    assert_eq!(
        report.components[0].files,
        vec![
            "crates/app/Cargo.toml".to_string(),
            "crates/app/src/main.rs".to_string(),
        ]
    );

    // 4. Verify symlinked project roots work cleanly with follow_links(false), while symlinks inside
    // a project pointing outside the project root are rejected when passed as `subpath`.
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let outside_dir = temp.path().join("outside_secret");
        fs::create_dir_all(&outside_dir).unwrap();
        fs::write(
            outside_dir.join("Cargo.toml"),
            "[package]\nname = \"outside_pkg\"\n",
        )
        .unwrap();
        fs::write(outside_dir.join("lib.rs"), "pub fn outside() {}\n").unwrap();

        let symlink_in_proj = valid_proj.join("external_link");
        symlink(&outside_dir, &symlink_in_proj).unwrap();

        // Full project scan skips the symlink (`follow_links(false)`)
        let full_report = storage
            .ingest_project_directory(IngestionOptions::new())
            .expect("full scan should skip symlink");
        assert_eq!(full_report.components.len(), 1);
        assert_eq!(full_report.components[0].id, "app");

        // Targeted subpath scan of a symlink pointing outside `valid_proj` must be rejected
        assert!(storage
            .ingest_project_directory(IngestionOptions::new().with_subpath("external_link"))
            .is_err());

        // Opening the project through a symlinked project root must still index `app`
        let symlink_proj_root = temp.path().join("symlink_to_valid_proj");
        symlink(&valid_proj, &symlink_proj_root).unwrap();
        let symlink_storage = TrackerStorage::new(&symlink_proj_root);
        let symlink_report = symlink_storage
            .ingest_project_directory(IngestionOptions::new())
            .expect("symlinked project root should resolve and index components");
        assert_eq!(symlink_report.components.len(), 1);
        assert_eq!(symlink_report.components[0].id, "app");
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let dest_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), dest_path)?;
        }
    }
    Ok(())
}

#[test]
fn test_canonical_phase_aliases_and_three_stage_scaffolding() {
    use backend_core::{
        canonical_phase_title, resolve_phase_alias, ComponentManifest, CANONICAL_PHASES,
    };

    // 1. Verify canonical phase constants and alias resolution (`pre`/`prod`/`post`/`1`/`2`/`3`/custom)
    assert_eq!(CANONICAL_PHASES.len(), 3);
    for alias in [
        "pre",
        "PRE",
        "pre-prod",
        "pre_prod",
        "pre-production",
        "pre_production",
        "1",
    ] {
        assert_eq!(resolve_phase_alias(alias), "pre_production");
        assert_eq!(canonical_phase_title(alias), Some("Pre-Production"));
    }
    for alias in ["prod", "PROD", "production", "active", "2"] {
        assert_eq!(resolve_phase_alias(alias), "production");
        assert_eq!(canonical_phase_title(alias), Some("Production"));
    }
    for alias in [
        "post",
        "POST",
        "post-prod",
        "post_prod",
        "post-production",
        "post_production",
        "3",
    ] {
        assert_eq!(resolve_phase_alias(alias), "post_production");
        assert_eq!(canonical_phase_title(alias), Some("Post-Production"));
    }
    assert_eq!(resolve_phase_alias("01_custom_stage"), "01_custom_stage");
    assert_eq!(canonical_phase_title("01_custom_stage"), None);

    // 2. Verify `TrackerStorage::init_standard` scaffolds all 3 canonical stages and `inbox` scopes
    let temp = TempTrackerDir::new("three_stage_init_standard");
    let storage = TrackerStorage::init_standard(temp.path(), "CanonicalProject")
        .expect("init_standard should scaffold 3 canonical stages");

    let index = storage.load_index().expect("should load 3-stage index");
    assert_eq!(index.project.project.active_phase, "pre_production");
    assert_eq!(index.phases.len(), 3);
    assert_eq!(index.phases[0].phase_id, "pre_production");
    assert_eq!(
        index.phases[0].manifest.phase.status,
        PhaseStatus::InProgress
    );
    assert_eq!(index.phases[1].phase_id, "production");
    assert_eq!(index.phases[1].manifest.phase.status, PhaseStatus::Planned);
    assert_eq!(index.phases[2].phase_id, "post_production");
    assert_eq!(index.phases[2].manifest.phase.status, PhaseStatus::Planned);

    // Alias lookups on `TrackerIndex` and `TrackerStorage`
    assert!(index.phase("pre").is_some());
    assert!(index.phase("prod").is_some());
    assert!(index.phase("post").is_some());
    assert!(index.scope("1", "inbox").is_some());
    assert!(index.scope("2", "inbox").is_some());
    assert!(index.scope("3", "inbox").is_some());
    assert_eq!(
        storage.read_phase("pre").unwrap().phase.id,
        "pre_production"
    );
    assert_eq!(storage.read_phase("prod").unwrap().phase.id, "production");
    assert_eq!(
        storage.read_phase("post").unwrap().phase.id,
        "post_production"
    );

    // 3. Verify `promote_component_to_scope` automatically mirrors the promoted scope across all 3 stages
    let comp_dir = temp.path().join("crates/engine_core/src");
    fs::create_dir_all(&comp_dir).unwrap();
    fs::write(
        temp.path().join("crates/engine_core/Cargo.toml"),
        "[package]\nname = \"engine_core\"\n",
    )
    .unwrap();
    fs::write(comp_dir.join("lib.rs"), "pub fn run() {}\n").unwrap();

    let comp = ComponentManifest::new(
        "engine_core",
        "Engine Core",
        "crates/engine_core",
        "rust_crate",
    );
    storage.write_component_manifest(&comp).unwrap();
    let promoted = storage
        .promote_component_to_scope(Some("pre"), "engine_core", None, None)
        .expect("should promote engine_core and mirror across all 3 canonical stages");
    assert_eq!(promoted.scope.id, "engine_core");

    for stage_alias in ["pre", "prod", "post"] {
        let mirrored_scope = storage
            .read_scope(stage_alias, "engine_core")
            .expect("promoted scope must exist in every canonical stage");
        assert_eq!(mirrored_scope.scope.target, "crates/engine_core");
        assert_eq!(
            mirrored_scope.scope.bound_component.as_deref(),
            Some("engine_core")
        );
        assert_eq!(
            mirrored_scope.scope.tracked_files,
            vec![
                "crates/engine_core/Cargo.toml".to_string(),
                "crates/engine_core/src/lib.rs".to_string(),
            ]
        );
        assert!(storage
            .items_dir(stage_alias, "engine_core")
            .unwrap()
            .is_dir());
    }

    // 4. Verify `TrackerStorage::init` with shorthand alias `"prod"` scaffolds all 3 stages with `production` active
    let temp_prod = TempTrackerDir::new("three_stage_init_alias");
    let prod_storage =
        TrackerStorage::init(temp_prod.path(), "ProdInitProject", "prod", "Production").unwrap();
    let prod_idx = prod_storage.load_index().unwrap();
    assert_eq!(prod_idx.project.project.active_phase, "production");
    assert_eq!(prod_idx.phases.len(), 3);
    assert_eq!(
        prod_storage.read_phase("prod").unwrap().phase.status,
        PhaseStatus::InProgress
    );
    assert_eq!(
        prod_storage.read_phase("pre").unwrap().phase.status,
        PhaseStatus::Planned
    );
}

#[test]
fn concurrent_creators_get_unique_ids() {
    use std::collections::HashSet;
    use std::thread;

    let temp = TempTrackerDir::new("concurrent_creators");
    let root = temp.path().to_path_buf();
    let storage = TrackerStorage::init_standard(&root, "ConcurrentTest").unwrap();

    let root_a = root.clone();
    let handle_a = thread::spawn(move || {
        let s = TrackerStorage::new(&root_a);
        let mut ids = Vec::with_capacity(50);
        for i in 0..50 {
            let (_, detail) = s
                .create_bug(format!("a{i}"), None, None, Severity::Medium, None)
                .expect("thread A should create bug");
            ids.push(detail.manifest.item.id);
        }
        ids
    });

    let root_b = root.clone();
    let handle_b = thread::spawn(move || {
        let s = TrackerStorage::new(&root_b);
        let mut ids = Vec::with_capacity(50);
        for i in 0..50 {
            let (_, detail) = s
                .create_bug(format!("b{i}"), None, None, Severity::Medium, None)
                .expect("thread B should create bug");
            ids.push(detail.manifest.item.id);
        }
        ids
    });

    let ids_a = handle_a.join().expect("thread A should not panic");
    let ids_b = handle_b.join().expect("thread B should not panic");

    let mut distinct_ids: HashSet<String> = HashSet::with_capacity(100);
    distinct_ids.extend(ids_a);
    distinct_ids.extend(ids_b);
    assert_eq!(distinct_ids.len(), 100);

    let active_phase = storage.read_project().unwrap().project.active_phase;
    let items_dir = storage.items_dir(&active_phase, "inbox").unwrap();
    let dir_count = fs::read_dir(&items_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .count();
    assert_eq!(dir_count, 100);
}

#[test]
fn versioned_reads_and_writes_preserve_unknown_keys_and_layout() {
    let temp = TempTrackerDir::new("versioned_compat");
    let storage = TrackerStorage::init_standard(temp.path(), "CompatTest").unwrap();

    let (loc, detail) = storage
        .create_bug("Initial bug", None, None, Severity::Medium, None)
        .unwrap();
    let item_path = storage
        .item_toml_path(&loc.phase_id, &loc.scope_id, &detail.manifest.item.id)
        .unwrap();

    // 1. Freshly created bug's item.toml has no schema_version line, and layout matches model serializer
    let item_text = fs::read_to_string(&item_path).unwrap();
    assert!(
        !item_text.contains("schema_version"),
        "v1 bug item.toml must not contain schema_version: {item_text}"
    );
    assert_eq!(
        item_text,
        storage
            .read_item_manifest(&loc.phase_id, &loc.scope_id, &detail.manifest.item.id)
            .unwrap()
            .to_toml_string()
            .unwrap()
    );

    let project_path = storage.project_toml_path();
    let project_text = fs::read_to_string(&project_path).unwrap();
    assert_eq!(
        project_text,
        storage.read_project().unwrap().to_toml_string().unwrap()
    );

    // 2. Hand-add `future_flag = true` at top level and `future_field = "x"` under `[item]` to item.toml
    let injected_item_toml = format!(
        "future_flag = true\n{}",
        item_text.replacen("[item]\n", "[item]\nfuture_field = \"x\"\n", 1)
    );
    fs::write(&item_path, &injected_item_toml).unwrap();

    storage
        .update_item_status(&detail.manifest.item.id, ItemStatus::InProgress)
        .unwrap();

    let updated_item_text = fs::read_to_string(&item_path).unwrap();
    assert!(
        updated_item_text.contains("future_flag = true"),
        "top-level unknown key must survive update_item_status: {updated_item_text}"
    );
    assert!(
        updated_item_text.contains("future_field = \"x\""),
        "nested [item] unknown key must survive update_item_status: {updated_item_text}"
    );
    let parsed_updated: toml::Table = toml::from_str(&updated_item_text).unwrap();
    assert_eq!(
        parsed_updated
            .get("future_flag")
            .and_then(toml::Value::as_bool),
        Some(true)
    );
    assert_eq!(
        parsed_updated
            .get("item")
            .and_then(toml::Value::as_table)
            .and_then(|t| t.get("future_field"))
            .and_then(toml::Value::as_str),
        Some("x")
    );

    // 3. Unknown top-level key in project.toml survives set_active_phase
    let injected_project_toml = format!("future_project_key = \"keep_me\"\n{project_text}");
    fs::write(&project_path, &injected_project_toml).unwrap();

    storage.set_active_phase("production").unwrap();
    let updated_project_text = fs::read_to_string(&project_path).unwrap();
    assert!(
        updated_project_text.contains("future_project_key = \"keep_me\""),
        "unknown top-level key in project.toml must survive set_active_phase: {updated_project_text}"
    );

    // 4. Unknown keys in phase.toml and scope.toml survive writes, and explicit schema_version = 1 is retained
    let phase_path = storage.phase_toml_path(&loc.phase_id).unwrap();
    let phase_text = fs::read_to_string(&phase_path).unwrap();
    fs::write(
        &phase_path,
        format!("schema_version = 1\nfuture_phase_key = 42\n{phase_text}"),
    )
    .unwrap();
    let phase_manifest = storage.read_phase(&loc.phase_id).unwrap();
    storage.write_phase(&loc.phase_id, &phase_manifest).unwrap();
    let updated_phase_text = fs::read_to_string(&phase_path).unwrap();
    assert!(
        updated_phase_text.contains("schema_version = 1"),
        "explicit schema_version = 1 must be retained when old_had_key is true: {updated_phase_text}"
    );
    assert!(
        updated_phase_text.contains("future_phase_key = 42"),
        "unknown key in phase.toml must survive write_phase: {updated_phase_text}"
    );

    let scope_path = storage
        .scope_toml_path(&loc.phase_id, &loc.scope_id)
        .unwrap();
    let scope_text = fs::read_to_string(&scope_path).unwrap();
    fs::write(
        &scope_path,
        format!("future_scope_key = \"sc\"\n{scope_text}"),
    )
    .unwrap();
    let scope_manifest = storage.read_scope(&loc.phase_id, &loc.scope_id).unwrap();
    storage
        .write_scope(&loc.phase_id, &loc.scope_id, &scope_manifest)
        .unwrap();
    let updated_scope_text = fs::read_to_string(&scope_path).unwrap();
    assert!(
        updated_scope_text.contains("future_scope_key = \"sc\""),
        "unknown key in scope.toml must survive write_scope: {updated_scope_text}"
    );

    // 5. An item.toml with schema_version = 9 makes read_item_manifest and write_item_manifest return UnsupportedSchema
    let v9_item_toml = format!("schema_version = 9\n{updated_item_text}");
    fs::write(&item_path, &v9_item_toml).unwrap();

    let read_err = storage
        .read_item_manifest(&loc.phase_id, &loc.scope_id, &detail.manifest.item.id)
        .expect_err("reading schema_version = 9 must fail");
    match read_err {
        TrackerError::UnsupportedSchema {
            path,
            found,
            supported,
        } => {
            assert_eq!(path, item_path);
            assert_eq!(found, 9);
            assert_eq!(supported, 1);
        }
        other => panic!("expected UnsupportedSchema on read, got {other:?}"),
    }

    let write_err = storage
        .write_item_manifest(
            &loc.phase_id,
            &loc.scope_id,
            &detail.manifest.item.id,
            &detail.manifest,
        )
        .expect_err("overwriting schema_version = 9 must fail");
    match write_err {
        TrackerError::UnsupportedSchema {
            path,
            found,
            supported,
        } => {
            assert_eq!(path, item_path);
            assert_eq!(found, 9);
            assert_eq!(supported, 1);
        }
        other => panic!("expected UnsupportedSchema on write, got {other:?}"),
    }

    assert_eq!(
        fs::read_to_string(&item_path).unwrap(),
        v9_item_toml,
        "item.toml with unsupported schema must remain byte-identical after rejected write"
    );
}

#[test]
fn versioned_writes_do_not_resurrect_cleared_fields_or_legacy_aliases() {
    let temp = TempTrackerDir::new("versioned_clear_and_alias");
    let storage = TrackerStorage::init_standard(temp.path(), "ClearAndAliasTest").unwrap();

    // 1. Clearing an optional/skipped field on scope.toml is not undone
    let scope_path = storage.scope_toml_path("pre_production", "inbox").unwrap();
    let mut scope = storage.read_scope("pre_production", "inbox").unwrap();
    scope.scope.bound_component = Some("x".to_string());
    storage
        .write_scope("pre_production", "inbox", &scope)
        .unwrap();
    assert_eq!(
        storage
            .read_scope("pre_production", "inbox")
            .unwrap()
            .scope
            .bound_component
            .as_deref(),
        Some("x")
    );

    scope.scope.bound_component = None;
    storage
        .write_scope("pre_production", "inbox", &scope)
        .unwrap();
    let scope_text = fs::read_to_string(&scope_path).unwrap();
    assert!(
        !scope_text.contains("bound_component"),
        "cleared bound_component must not remain in scope.toml: {scope_text}"
    );
    assert_eq!(
        storage
            .read_scope("pre_production", "inbox")
            .unwrap()
            .scope
            .bound_component,
        None
    );

    // 2. Legacy alias keys (e.g. `[components] manifest = [...]` or `[counters] feat = ...`) are not kept alongside canonical keys
    let project_path = storage.project_toml_path();
    let project_text = fs::read_to_string(&project_path).unwrap();
    let legacy_project_toml = format!(
        "{}\n[[components.manifest]]\nid = \"core\"\nname = \"Core\"\nrelative_path = \"crates/core\"\ncomponent_type = \"rust_crate\"\n",
        project_text.replace("feature = 0", "feat = 3")
    );
    fs::write(&project_path, &legacy_project_toml).unwrap();

    let loaded_project = storage.read_project().unwrap();
    assert_eq!(loaded_project.counters.feature, 3);
    assert_eq!(loaded_project.components.len(), 1);

    storage.write_project(&loaded_project).unwrap();
    let rewritten_project_text = fs::read_to_string(&project_path).unwrap();
    assert!(
        !rewritten_project_text.contains("feat ="),
        "legacy alias `feat` must not be resurrected alongside `feature`: {rewritten_project_text}"
    );
    assert!(
        !rewritten_project_text.contains("components.manifest"),
        "legacy alias `components.manifest` must not be resurrected alongside `components.registry`: {rewritten_project_text}"
    );
    let reloaded_project = storage.read_project().unwrap();
    assert_eq!(reloaded_project.counters.feature, 3);
    assert_eq!(reloaded_project.components.len(), 1);
}

#[test]
fn reconcile_skips_and_reports_unsupported_item_schema() {
    let temp = TempTrackerDir::new("reconcile_skip_unsupported_schema");
    let storage = TrackerStorage::init_standard(temp.path(), "ReconcileSkipUnsupported").unwrap();

    let (loc1, bug1) = storage
        .create_bug("First bug", None, None, Severity::Medium, None)
        .unwrap();
    let (loc2, bug2) = storage
        .create_bug("Second bug", None, None, Severity::Medium, None)
        .unwrap();

    let bug2_path = storage
        .item_toml_path(&loc2.phase_id, &loc2.scope_id, &bug2.manifest.item.id)
        .unwrap();
    let bug2_original = fs::read_to_string(&bug2_path).unwrap();
    let bug2_v9 = format!("schema_version = 9\n{bug2_original}");
    fs::write(&bug2_path, &bug2_v9).unwrap();

    let report = storage
        .reconcile_indexes()
        .expect("reconcile_indexes should skip unsupported item schema");

    let expected_rel = format!(
        "phases/{}/domains/{}/items/{}/item.toml",
        loc2.phase_id, loc2.scope_id, bug2.manifest.item.id
    );
    assert_eq!(report.skipped_unsupported, vec![expected_rel]);
    assert_eq!(report.total_items, 1);

    let scope = storage.read_scope(&loc1.phase_id, &loc1.scope_id).unwrap();
    assert_eq!(scope.items.bugs.len(), 1);
    assert_eq!(scope.items.bugs[0].id, bug1.manifest.item.id);

    let phase = storage.read_phase(&loc1.phase_id).unwrap();
    assert_eq!(phase.summary.total_items, 1);

    assert!(report.counters.bug >= 2);
    assert!(storage.read_project().unwrap().counters.bug >= 2);

    assert_eq!(
        fs::read_to_string(&bug2_path).unwrap(),
        bug2_v9,
        "skipped item.toml must remain byte-identical after reconcile_indexes"
    );
}
