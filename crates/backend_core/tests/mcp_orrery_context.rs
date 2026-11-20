use backend_core::{CreateItemInput, ItemType, McpServer, TrackerStorage};
use serde_json::{json, Value};
use tempfile::TempDir;

fn tracker_with_step_task(source_ref: &str) -> (TempDir, TrackerStorage, String) {
    let dir = TempDir::new().expect("should create temp dir");
    let storage =
        TrackerStorage::init_standard(dir.path(), "OrbitTest").expect("should init tracker");
    let (_, detail) = storage
        .create_item(
            CreateItemInput::new(ItemType::Task, "Widget step").with_source_ref(source_ref),
        )
        .expect("should create step task");
    let id = detail.manifest.item.id.clone();
    (dir, storage, id)
}

fn orrery(step_title: &str) -> Value {
    json!({ "step_number": 1, "step_title": step_title, "event": "step_start" })
}

#[test]
fn update_item_status_resolves_item_from_orrery_step_title() {
    let (_dir, storage, id) = tracker_with_step_task("STEP 41B");
    let mut server = McpServer::new(Some(storage));
    let res = server
        .call_tool(
            "update_item_status",
            json!({ "status": "in_progress", "orrery": orrery("41B: Widget step") }),
        )
        .expect("step_title must resolve the item");
    assert_eq!(res["item_id"].as_str(), Some(id.as_str()));
    assert_eq!(res["status"].as_str(), Some("in_progress"));
}

#[test]
fn update_item_status_accepts_step_prefix_in_step_title() {
    let (_dir, storage, id) = tracker_with_step_task("STEP 41B");
    let mut server = McpServer::new(Some(storage));
    let res = server
        .call_tool(
            "update_item_status",
            json!({ "status": "in_progress", "orrery": orrery("STEP 41B: Widget step") }),
        )
        .expect("a STEP prefix must be accepted");
    assert_eq!(res["item_id"].as_str(), Some(id.as_str()));
}

#[test]
fn explicit_item_id_wins_over_orrery_step_title() {
    let (_dir, storage, id) = tracker_with_step_task("STEP 41B");
    let mut server = McpServer::new(Some(storage));
    let res = server
        .call_tool(
            "update_item_status",
            json!({ "item_id": id, "status": "pending", "orrery": orrery("99Z: Other step") }),
        )
        .expect("explicit item_id must be used");
    assert_eq!(res["item_id"].as_str(), Some(id.as_str()));
    assert_eq!(res["status"].as_str(), Some("pending"));
}

#[test]
fn close_item_with_fix_resolves_item_from_orrery_step_title() {
    let (_dir, storage, id) = tracker_with_step_task("STEP 41B");
    let mut server = McpServer::new(Some(storage));
    let res = server
        .call_tool(
            "close_item_with_fix",
            json!({ "summary": "step passed", "orrery": orrery("41B: Widget step") }),
        )
        .expect("step_title must resolve the item");
    assert_eq!(res["item_id"].as_str(), Some(id.as_str()));
    assert_eq!(res["status"].as_str(), Some("closed"));
    assert_eq!(res["previous_fixes_count"].as_u64(), Some(1));
}

#[test]
fn step_title_without_a_matching_item_is_an_error() {
    let (_dir, storage, _id) = tracker_with_step_task("STEP 41B");
    let mut server = McpServer::new(Some(storage));
    let err = server
        .call_tool(
            "update_item_status",
            json!({ "status": "in_progress", "orrery": orrery("41C: Next step") }),
        )
        .expect_err("no item for STEP 41C");
    let msg = err.to_string();
    assert!(
        msg.contains("no item has source_ref 'STEP 41C'"),
        "unexpected: {msg}"
    );
}

#[test]
fn unparseable_step_title_is_an_error() {
    let (_dir, storage, _id) = tracker_with_step_task("STEP 41B");
    let mut server = McpServer::new(Some(storage));
    let err = server
        .call_tool(
            "close_item_with_fix",
            json!({ "summary": "step passed", "orrery": orrery("live merged args") }),
        )
        .expect_err("title without a step ID");
    let msg = err.to_string();
    assert!(
        msg.contains("can't parse a step ID from orrery.step_title 'live merged args'"),
        "unexpected: {msg}"
    );
}

#[test]
fn missing_item_id_and_orrery_context_is_an_error() {
    let (_dir, storage, _id) = tracker_with_step_task("STEP 41B");
    let mut server = McpServer::new(Some(storage));
    let err = server
        .call_tool("update_item_status", json!({ "status": "in_progress" }))
        .expect_err("neither item_id nor orrery");
    let msg = err.to_string();
    assert!(
        msg.contains("item_id is required unless orrery.step_title is present"),
        "unexpected: {msg}"
    );
}

#[test]
fn duplicate_source_ref_is_an_error() {
    let (_dir, storage, _id) = tracker_with_step_task("STEP 41B");
    storage
        .create_item(CreateItemInput::new(ItemType::Task, "Duplicate").with_source_ref("STEP 41B"))
        .expect("should create duplicate task");
    let mut server = McpServer::new(Some(storage));
    let err = server
        .call_tool(
            "update_item_status",
            json!({ "status": "in_progress", "orrery": orrery("41B: Widget step") }),
        )
        .expect_err("two items for STEP 41B");
    let msg = err.to_string();
    assert!(
        msg.contains("2 items have source_ref 'STEP 41B'"),
        "unexpected: {msg}"
    );
}
