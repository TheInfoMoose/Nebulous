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
