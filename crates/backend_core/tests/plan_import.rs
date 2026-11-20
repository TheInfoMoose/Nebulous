use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use backend_core::planning::import::PLAN_CLOSE_SUMMARY;
use backend_core::planning::{import_plan, parse_roadmap, ImportEntry, ImportReport};
use backend_core::{CreateItemInput, ItemStatus, ItemType, Severity, TrackerError, TrackerStorage};

const SAMPLE_PLAN: &str = r#"# Synthetic Plan

## Step Execution Roadmap

### Up Next

| # | Step | Command | Scope | Target Files | Waits on |
| :---: | :--- | :---: | :--- | :--- | :--- |
| 1 | **STEP 2A** | `/goal` | Build widget module | `src/widget.rs` (NEW) | — |
| 2 | **STEP 2B** | `/boost` | Wire app loop | `src/app.rs` | — |

### Completed

| Step ID | Status | Command | Scope | Target Files |
| :--- | :---: | :---: | :--- | :--- |
| **STEP 1A** | **COMPLETED** (`abc1234`) | `/goal` | Bootstrap workspace | `Cargo.toml` |
"#;

fn init_test_tracker() -> (TempDir, TrackerStorage) {
    let dir = TempDir::new().expect("should create temp dir");
    let storage =
        TrackerStorage::init_standard(dir.path(), "OrbitTest").expect("should init tracker");
    (dir, storage)
}

fn snapshot_directory(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(base: &Path, current: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let mut entries: Vec<_> = fs::read_dir(current)
            .expect("should read directory")
            .map(|e| e.expect("should read entry"))
            .collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let rel = path
                    .strip_prefix(base)
                    .expect("should strip prefix")
                    .to_path_buf();
                let bytes = fs::read(&path).expect("should read file bytes");
                out.insert(rel, bytes);
            }
        }
    }

    let mut map = BTreeMap::new();
    walk(root, root, &mut map);
    map
}

#[test]
fn first_write_run_creates_tasks_with_source_ref() {
    let (_dir, storage) = init_test_tracker();
    let steps = parse_roadmap(SAMPLE_PLAN).expect("should parse sample roadmap");

    let report = import_plan(&storage, &steps, None, true).expect("should import plan");

    let created_steps: Vec<&str> = report.created.iter().map(|e| e.step_id.as_str()).collect();
    assert_eq!(created_steps, vec!["STEP 2A", "STEP 2B"]);
    assert!(report.retitled.is_empty());
    assert!(report.closed.is_empty());
    assert!(report.unchanged.is_empty());

    let id_2a = report.created[0]
        .item_id
        .as_deref()
        .expect("write run should populate item_id for 2A");
    assert!(id_2a.starts_with("TASK-"));

    let (_, detail_2a) = storage
        .load_item_detail_by_id(id_2a)
        .expect("should load 2A");
    assert_eq!(detail_2a.manifest.item.item_type, ItemType::Task);
    assert_eq!(detail_2a.manifest.item.title, "Build widget module");
    assert_eq!(
        detail_2a.manifest.item.source_ref.as_deref(),
        Some("STEP 2A")
    );
    assert_eq!(detail_2a.manifest.item.status, ItemStatus::Open);
    assert!(detail_2a.description_markdown.contains("src/widget.rs"));

    let listed = storage.load_index().expect("should load index").all_items();
    assert_eq!(listed.len(), 2, "a Completed row must not create an item");
}

#[test]
fn second_run_reports_everything_unchanged() {
    let (_dir, storage) = init_test_tracker();
    let steps = parse_roadmap(SAMPLE_PLAN).expect("should parse sample roadmap");

    let first = import_plan(&storage, &steps, None, true).expect("first import");
    let second = import_plan(&storage, &steps, None, true).expect("second import");

    assert!(second.created.is_empty());
    assert!(second.retitled.is_empty());
    assert!(second.closed.is_empty());

    let unchanged_steps: Vec<&str> = second
        .unchanged
        .iter()
        .map(|e| e.step_id.as_str())
        .collect();
    assert_eq!(unchanged_steps, vec!["STEP 2A", "STEP 2B"]);
    assert_eq!(second.unchanged, first.created);

    let empty_report = import_plan(&storage, &[], None, true).expect("empty steps import");
    assert_eq!(empty_report, ImportReport::default());
}

#[test]
fn retitled_row_retitles_only() {
    let (_dir, storage) = init_test_tracker();
    let steps = parse_roadmap(SAMPLE_PLAN).expect("should parse sample roadmap");
    let first = import_plan(&storage, &steps, None, true).expect("first import");

    let id_2b = first.created[1]
        .item_id
        .clone()
        .expect("should have item_id for 2B");
    storage
        .update_item_metadata(&id_2b, None, Some(Severity::High))
        .expect("should set severity before re-import");
    let (loc_before, detail_before) = storage
        .load_item_detail_by_id(&id_2b)
        .expect("should load 2B before re-import");

    let edited_plan = SAMPLE_PLAN.replace("Wire app loop", "Wire main application loop");
    let updated_steps = parse_roadmap(&edited_plan).expect("should parse updated roadmap");
    let second = import_plan(&storage, &updated_steps, None, true).expect("second import");

    assert!(second.created.is_empty());
    assert!(second.closed.is_empty());
    assert_eq!(
        second.retitled,
        vec![ImportEntry {
            step_id: "STEP 2B".to_string(),
            item_id: Some(id_2b.clone()),
        }]
    );
    let unchanged_steps: Vec<&str> = second
        .unchanged
        .iter()
        .map(|e| e.step_id.as_str())
        .collect();
    assert_eq!(unchanged_steps, vec!["STEP 2A"]);

    let (loc_after, detail_after) = storage
        .load_item_detail_by_id(&id_2b)
        .expect("should load 2B after re-import");
    assert_eq!(
        detail_after.manifest.item.title,
        "Wire main application loop"
    );
    assert_eq!(
        detail_after.manifest.item.status,
        detail_before.manifest.item.status
    );
    assert_eq!(
        detail_after.manifest.item.severity,
        detail_before.manifest.item.severity
    );
    assert_eq!(loc_after.scope_id, loc_before.scope_id);
    assert_eq!(
        detail_after.description_markdown,
        detail_before.description_markdown
    );
}

#[test]
fn row_flipped_to_completed_closes() {
    let (_dir, storage) = init_test_tracker();
    let steps = parse_roadmap(SAMPLE_PLAN).expect("should parse sample roadmap");
    let first = import_plan(&storage, &steps, None, true).expect("first import");
    let id_2b = first.created[1]
        .item_id
        .clone()
        .expect("should have item_id for 2B");

    let flipped_plan = SAMPLE_PLAN.replace(
        "| 2 | **STEP 2B** | `/boost` | Wire app loop | `src/app.rs` | — |",
        "| 2 | **STEP 2B** | `/boost` | Wire app loop | `src/app.rs` | **COMPLETED** |",
    );
    let updated_steps = parse_roadmap(&flipped_plan).expect("should parse flipped roadmap");
    let second = import_plan(&storage, &updated_steps, None, true).expect("second import");

    assert!(second.created.is_empty());
    assert!(second.retitled.is_empty());
    assert_eq!(
        second.closed,
        vec![ImportEntry {
            step_id: "STEP 2B".to_string(),
            item_id: Some(id_2b.clone()),
        }]
    );

    let (_, detail_2b) = storage
        .load_item_detail_by_id(&id_2b)
        .expect("should load 2B");
    assert_eq!(detail_2b.manifest.item.status, ItemStatus::Closed);
    assert_eq!(
        detail_2b
            .manifest
            .history
            .previous_fixes
            .last()
            .map(|f| f.summary.as_str()),
        Some(PLAN_CLOSE_SUMMARY)
    );
}

#[test]
fn manually_closed_item_stays_closed_while_row_is_open() {
    let (_dir, storage) = init_test_tracker();
    let steps = parse_roadmap(SAMPLE_PLAN).expect("should parse sample roadmap");
    let first = import_plan(&storage, &steps, None, true).expect("first import");
    let id_2a = first.created[0]
        .item_id
        .clone()
        .expect("should have item_id for 2A");

    storage
        .close_item(&id_2a, "Closed manually before plan update", None, false)
        .expect("should close 2A manually");

    let second = import_plan(&storage, &steps, None, true).expect("second import");
    assert!(second.created.is_empty());
    assert!(second.retitled.is_empty());
    assert!(second.closed.is_empty());

    let unchanged_steps: Vec<&str> = second
        .unchanged
        .iter()
        .map(|e| e.step_id.as_str())
        .collect();
    assert_eq!(unchanged_steps, vec!["STEP 2A", "STEP 2B"]);

    let (_, detail_2a) = storage
        .load_item_detail_by_id(&id_2a)
        .expect("should load 2A");
    assert_eq!(detail_2a.manifest.item.status, ItemStatus::Closed);
}

#[test]
fn dry_run_leaves_tracker_byte_identical() {
    let (dir, storage) = init_test_tracker();
    let tracker_dir = dir.path().join(".tracker");
    let before = snapshot_directory(&tracker_dir);

    let steps = parse_roadmap(SAMPLE_PLAN).expect("should parse sample roadmap");
    let missing_phase_err = import_plan(&storage, &steps, Some("unknown_stage"), false)
        .expect_err("unknown phase must fail in dry run");
    assert!(matches!(
        missing_phase_err,
        TrackerError::NotFound {
            kind: "Phase",
            ref id
        } if id == "unknown_stage"
    ));

    let dry_report = import_plan(&storage, &steps, None, false).expect("dry run import");

    let after = snapshot_directory(&tracker_dir);
    assert_eq!(
        before, after,
        "dry run must not create or modify any file under .tracker/"
    );

    let dry_steps: Vec<&str> = dry_report
        .created
        .iter()
        .map(|e| e.step_id.as_str())
        .collect();
    assert_eq!(dry_steps, vec!["STEP 2A", "STEP 2B"]);
    assert!(dry_report.created.iter().all(|e| e.item_id.is_none()));
    assert!(dry_report.retitled.is_empty());
    assert!(dry_report.closed.is_empty());
    assert!(dry_report.unchanged.is_empty());

    let write_report = import_plan(&storage, &steps, None, true).expect("write run import");
    let write_steps: Vec<&str> = write_report
        .created
        .iter()
        .map(|e| e.step_id.as_str())
        .collect();
    assert_eq!(write_steps, dry_steps);
    assert!(write_report.created.iter().all(|e| e.item_id.is_some()));
}

#[test]
fn target_files_under_a_scope_land_in_that_scope() {
    let (_dir, storage) = init_test_tracker();
    storage
        .create_scope(None, "core", "crates/core", vec![], None)
        .expect("should create core scope");
    storage
        .create_scope(None, "cli", "crates/cli", vec![], None)
        .expect("should create cli scope");

    let scoped_plan = r#"# Scoped Plan

## Step Execution Roadmap

### Up Next

| # | Step | Command | Scope | Target Files | Waits on |
| :---: | :--- | :---: | :--- | :--- | :--- |
| 1 | **STEP 3A** | `/goal` | Core changes | `crates/core/src/a.rs`, `crates/core/src/b.rs`, `README.md` | — |
| 2 | **STEP 3B** | `/goal` | Tied changes | `crates/core/src/a.rs`, `crates/cli/src/main.rs` | — |

### Completed

| Step ID | Status | Command | Scope | Target Files |
| :--- | :---: | :---: | :--- | :--- |
"#;

    let steps = parse_roadmap(scoped_plan).expect("should parse scoped roadmap");
    let report = import_plan(&storage, &steps, None, true).expect("should import scoped plan");

    assert_eq!(report.created.len(), 2);
    let item_id = report.created[0]
        .item_id
        .as_deref()
        .expect("should have created item_id");
    let (_, detail) = storage
        .load_item_detail_by_id(item_id)
        .expect("should load created item");
    assert_eq!(detail.manifest.item.scope_ref, "core");

    let tied_id = report.created[1]
        .item_id
        .as_deref()
        .expect("should have created item_id for tied step");
    let (_, tied_detail) = storage
        .load_item_detail_by_id(tied_id)
        .expect("should load tied item");
    assert_eq!(tied_detail.manifest.item.scope_ref, "inbox");

    // Duplicate source_ref across two items triggers ValidationError naming both IDs and the ref.
    let (_, dup_detail) = storage
        .create_item(
            CreateItemInput::new(ItemType::Task, "Manual duplicate source_ref")
                .with_source_ref("STEP 3A"),
        )
        .expect("should create manual duplicate item");
    let dup_err = import_plan(&storage, &steps, None, false)
        .expect_err("duplicate source_ref should fail validation");
    let err_msg = dup_err.to_string();
    assert!(
        err_msg.contains("STEP 3A"),
        "error should name ref: {err_msg}"
    );
    assert!(
        err_msg.contains(item_id),
        "error should name first ID: {err_msg}"
    );
    assert!(
        err_msg.contains(&dup_detail.manifest.item.id),
        "error should name second ID: {err_msg}"
    );
}

fn mcp_tracker_with_plan() -> (TempDir, TrackerStorage) {
    let (dir, storage) = init_test_tracker();
    fs::write(dir.path().join("PLAN.md"), SAMPLE_PLAN).expect("should write PLAN.md");
    (dir, storage)
}

#[test]
fn mcp_plan_import_defaults_to_dry_run() {
    use backend_core::McpServer;
    use serde_json::json;

    let (dir, storage) = mcp_tracker_with_plan();
    let tracker_dir = dir.path().join(".tracker");
    let before = snapshot_directory(&tracker_dir);

    let mut server = McpServer::new(Some(storage));
    let res = server
        .call_tool("plan_import", json!({ "path": "PLAN.md" }))
        .expect("should run plan_import in dry run mode");

    let after = snapshot_directory(&tracker_dir);
    assert_eq!(before, after, "dry run must leave .tracker/ byte-identical");

    let created = res["created"].as_array().expect("created should be array");
    assert_eq!(created.len(), 2);
    assert_eq!(created[0]["step_id"].as_str(), Some("STEP 2A"));
    assert!(created.iter().all(|entry| entry["item_id"].is_null()));
}

#[test]
fn mcp_plan_import_dry_run_false_creates_tasks() {
    use backend_core::McpServer;
    use serde_json::json;

    let (_dir, storage) = mcp_tracker_with_plan();
    let mut server = McpServer::new(Some(storage.clone()));
    let res = server
        .call_tool(
            "plan_import",
            json!({ "path": "PLAN.md", "dry_run": false }),
        )
        .expect("should run plan_import with dry_run false");

    let created = res["created"].as_array().expect("created should be array");
    assert_eq!(created.len(), 2);
    let item_id = created[0]["item_id"]
        .as_str()
        .expect("created[0].item_id should be present");
    assert!(item_id.starts_with("TASK-"));

    let (_, detail) = storage
        .load_item_detail_by_id(item_id)
        .expect("should load created task item");
    assert_eq!(detail.manifest.item.source_ref.as_deref(), Some("STEP 2A"));
}

#[test]
fn mcp_plan_import_rejects_path_outside_project_root() {
    use backend_core::McpServer;
    use serde_json::json;

    let (_dir, storage) = mcp_tracker_with_plan();
    let mut server = McpServer::new(Some(storage));
    let err = server
        .call_tool("plan_import", json!({ "path": "../outside.md" }))
        .expect_err("path outside project root must fail");
    let msg = err.to_string();
    assert!(msg.contains("escapes"), "expected 'escapes' in: {msg}");
}

#[test]
fn mcp_plan_import_rejects_unknown_arguments() {
    use backend_core::McpServer;
    use serde_json::json;

    let (_dir, storage) = mcp_tracker_with_plan();
    let mut server = McpServer::new(Some(storage));
    let err = server
        .call_tool("plan_import", json!({ "path": "PLAN.md", "write": true }))
        .expect_err("unknown argument must fail");
    let msg = err.to_string();
    assert!(msg.contains("write"), "expected 'write' in: {msg}");
}

#[test]
fn closed_item_is_not_retitled() {
    let (_dir, storage) = init_test_tracker();
    let steps = parse_roadmap(SAMPLE_PLAN).expect("should parse sample roadmap");
    let first = import_plan(&storage, &steps, None, true).expect("first import");
    let id_2a = first.created[0]
        .item_id
        .clone()
        .expect("should have item_id for 2A");
    storage
        .update_item_metadata(&id_2a, Some("Widget module (reworded)".to_string()), None)
        .expect("should reword 2A");
    storage
        .close_item(&id_2a, "Done", None, false)
        .expect("should close 2A");

    let second = import_plan(&storage, &steps, None, true).expect("second import");
    assert!(second.retitled.is_empty());
    let unchanged_steps: Vec<&str> = second
        .unchanged
        .iter()
        .map(|e| e.step_id.as_str())
        .collect();
    assert_eq!(unchanged_steps, vec!["STEP 2A", "STEP 2B"]);

    let (_, detail_2a) = storage
        .load_item_detail_by_id(&id_2a)
        .expect("should load 2A");
    assert_eq!(detail_2a.manifest.item.title, "Widget module (reworded)");
}
