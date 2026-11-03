use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use backend_core::{ItemStatus, Severity, TrackerStorage};
use git2::{Repository, Signature};
use serde_json::Value;
use tempfile::TempDir;

fn neb_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_neb"))
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

fn run_neb(args: &[&str], current_dir: Option<&Path>) -> Output {
    let mut cmd = Command::new(neb_bin());
    cmd.args(args);
    if let Some(dir) = current_dir {
        cmd.current_dir(dir);
    }
    cmd.output().expect("should execute neb binary")
}

fn run_neb_ok(args: &[&str], current_dir: Option<&Path>) -> String {
    let output = run_neb(args, current_dir);
    if !output.status.success() {
        panic!(
            "neb {:?} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            args,
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    String::from_utf8(output.stdout).expect("stdout should be valid UTF-8")
}

fn run_neb_json(args: &[&str], current_dir: Option<&Path>) -> Value {
    let mut full_args = vec!["--json"];
    full_args.extend_from_slice(args);
    let stdout = run_neb_ok(&full_args, current_dir);
    serde_json::from_str(&stdout).unwrap_or_else(|err| {
        panic!(
            "Failed to parse JSON output from neb {:?}: {err}\nRaw stdout:\n{stdout}",
            full_args
        )
    })
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
fn test_temp_dir_full_cli_lifecycle_and_json_mode() {
    let temp = TempDir::new().expect("should create temp dir");
    let root_str = temp.path().to_str().unwrap();

    // 1. `neb init`
    let init_json = run_neb_json(
        &[
            "--root",
            root_str,
            "init",
            "--name",
            "CliTestProject",
            "--phase-id",
            "01_foundation",
            "--phase-title",
            "Foundation Phase",
        ],
        None,
    );
    assert_eq!(init_json["status"], "initialized");
    assert_eq!(init_json["project_name"], "CliTestProject");
    assert_eq!(init_json["active_phase"], "01_foundation");
    assert_eq!(init_json["default_scope"], "inbox");
    assert!(temp.path().join(".tracker/project.toml").is_file());

    // 2. `neb scope add` & `neb scope list`
    let scope_add_json = run_neb_json(
        &[
            "--root",
            root_str,
            "scope",
            "add",
            "backend_core",
            "--target",
            "crates/backend_core",
            "--stack",
            "rust,serde,toml",
            "--desc",
            "Core backend storage crate",
        ],
        None,
    );
    assert_eq!(scope_add_json["id"], "backend_core");
    assert_eq!(scope_add_json["target"], "crates/backend_core");

    let scope_list_json = run_neb_json(&["--root", root_str, "scope", "list"], None);
    assert_eq!(scope_list_json["phase_id"], "01_foundation");
    let scopes_arr = scope_list_json["scopes"].as_array().unwrap();
    assert_eq!(scopes_arr.len(), 2); // inbox + backend_core

    // 3. `neb bug` (explicit scope and default inbox fallback)
    let bug1_json = run_neb_json(
        &[
            "--root",
            root_str,
            "bug",
            "Empty array fails deserialization",
            "-s",
            "backend_core",
            "--severity",
            "high",
            "-d",
            "Occurs when scope.toml has bugs = []",
        ],
        None,
    );
    assert_eq!(bug1_json["id"], "BUG-0001");
    assert_eq!(bug1_json["type"], "bug");
    assert_eq!(bug1_json["severity"], "high");
    assert_eq!(bug1_json["scope_id"], "backend_core");

    // Bug without `-s` falls back to `inbox`
    let bug2_json = run_neb_json(&["--root", root_str, "bug", "Unscoped triage issue"], None);
    assert_eq!(bug2_json["id"], "BUG-0002");
    assert_eq!(bug2_json["scope_id"], "inbox");
    assert_eq!(bug2_json["severity"], "medium");

    // 4. `neb feat`
    let feat1_json = run_neb_json(
        &[
            "--root",
            root_str,
            "feat",
            "Implement companion CLI subcommands",
            "-s",
            "backend_core",
            "--severity",
            "critical",
            "-d",
            "Support all 16 subcommands with clap derive",
        ],
        None,
    );
    assert_eq!(feat1_json["id"], "FEAT-0001");
    assert_eq!(feat1_json["type"], "feature");
    assert_eq!(feat1_json["severity"], "critical");
    assert_eq!(feat1_json["scope_id"], "backend_core");

    // 5. `neb list` (and ancestor directory discovery without `--root`)
    let nested_dir = temp.path().join("crates/backend_core/src");
    fs::create_dir_all(&nested_dir).unwrap();
    let list_from_nested = run_neb_json(&["list"], Some(&nested_dir));
    assert_eq!(list_from_nested["total"], 3);

    let list_bugs_only = run_neb_json(
        &[
            "--root",
            root_str,
            "list",
            "--type",
            "bug",
            "-s",
            "backend_core",
        ],
        None,
    );
    assert_eq!(list_bugs_only["total"], 1);
    assert_eq!(list_bugs_only["items"][0]["id"], "BUG-0001");

    // 6. `neb timer <start|status|stop>` and `neb log`
    let timer_idle = run_neb_json(&["--root", root_str, "timer", "status"], None);
    assert_eq!(timer_idle["active"], false);

    let timer_start = run_neb_json(
        &[
            "--root",
            root_str,
            "timer",
            "start",
            "BUG-0001",
            "--note",
            "Debugging serde default",
        ],
        None,
    );
    assert_eq!(timer_start["action"], "start");
    assert_eq!(timer_start["active"], true);
    assert_eq!(timer_start["item_id"], "BUG-0001");

    let timer_running = run_neb_json(&["--root", root_str, "timer", "status"], None);
    assert_eq!(timer_running["active"], true);
    assert_eq!(timer_running["item_id"], "BUG-0001");

    let timer_stop = run_neb_json(&["--root", root_str, "timer", "stop", "BUG-0001"], None);
    assert_eq!(timer_stop["action"], "stop");
    assert_eq!(timer_stop["active"], false);
    assert_eq!(timer_stop["item_id"], "BUG-0001");

    let log_json = run_neb_json(&["--root", root_str, "log", "BUG-0001", "-m", "45"], None);
    assert_eq!(log_json["id"], "BUG-0001");
    assert_eq!(log_json["added_minutes"], 45);
    assert_eq!(log_json["logged_time_minutes"], 45);

    // 7. `neb status`, `neb close`, `neb reopen`
    let status_json = run_neb_json(
        &["--root", root_str, "status", "BUG-0001", "in_progress"],
        None,
    );
    assert_eq!(status_json["status"], "in_progress");
    assert_eq!(status_json["recurrence_count"], 0);

    let close_json = run_neb_json(
        &[
            "--root",
            root_str,
            "close",
            "BUG-0001",
            "-m",
            "Added #[serde(default)] on bugs and features vecs",
            "--commit",
            "a4f89c1",
        ],
        None,
    );
    assert_eq!(close_json["status"], "closed");
    assert_eq!(close_json["previous_fixes_count"], 1);
    assert_eq!(close_json["fix"]["commit"], "a4f89c1");
    assert_eq!(
        close_json["fix"]["summary"],
        "Added #[serde(default)] on bugs and features vecs"
    );

    let reopen_json = run_neb_json(
        &[
            "--root",
            root_str,
            "reopen",
            "BUG-0001",
            "-r",
            "Still fails when [items] section is omitted entirely",
        ],
        None,
    );
    assert_eq!(reopen_json["status"], "open");
    assert_eq!(reopen_json["recurrence_count"], 1);
    assert_eq!(reopen_json["iteration_count"], 2);

    // Verify `neb list --recurring` returns BUG-0001
    let recurring_list = run_neb_json(&["--root", root_str, "list", "--recurring"], None);
    assert_eq!(recurring_list["total"], 1);
    assert_eq!(recurring_list["items"][0]["id"], "BUG-0001");
    assert_eq!(recurring_list["items"][0]["recurrence_count"], 1);

    // 8. `neb show`
    let show_json = run_neb_json(&["--root", root_str, "show", "BUG-0001"], None);
    assert_eq!(show_json["item"]["id"], "BUG-0001");
    assert_eq!(show_json["effort"]["logged_time_minutes"], 45);
    assert_eq!(show_json["history"]["recurrence_count"], 1);
    assert!(show_json["description_markdown"]
        .as_str()
        .unwrap()
        .contains("Still fails when [items] section is omitted entirely"));

    // 9. `neb context` (both full prompt and `--refs-only`)
    let refs_out = run_neb_ok(
        &["--root", root_str, "context", "BUG-0001", "--refs-only"],
        None,
    );
    assert!(refs_out.contains("@.tracker/phases/01_foundation/domains/backend_core/scope.toml"));
    assert!(refs_out
        .contains("@.tracker/phases/01_foundation/domains/backend_core/items/BUG-0001/item.toml"));
    assert!(refs_out
        .contains("@.tracker/phases/01_foundation/domains/backend_core/items/BUG-0001/desc.md"));

    let prompt_out = run_neb_ok(&["--root", root_str, "context", "BUG-0001"], None);
    assert!(prompt_out.contains("# Bounded Agent Task: BUG-0001"));
    assert!(prompt_out.contains("crates/backend_core"));
    assert!(prompt_out.contains("REGRESSION & PREVIOUS FIX WARNING (`recurrence_count = 1`)"));
    assert!(prompt_out.contains("a4f89c1"));

    // 10. `neb metrics`
    let metrics_json = run_neb_json(&["--root", root_str, "metrics"], None);
    assert_eq!(metrics_json["total_items"], 3);
    assert_eq!(metrics_json["completed_items"], 0);
    assert_eq!(metrics_json["total_logged_minutes"], 45);
    assert_eq!(metrics_json["item_hotspots"][0]["item_id"], "BUG-0001");
    assert_eq!(metrics_json["item_hotspots"][0]["recurrence_count"], 1);

    // 11. `neb phase <add|use|list>`
    let phase_add_json = run_neb_json(
        &[
            "--root",
            root_str,
            "phase",
            "add",
            "02_polish",
            "--title",
            "Polish & Release",
        ],
        None,
    );
    assert_eq!(phase_add_json["phase_id"], "02_polish");
    assert_eq!(phase_add_json["active_phase"], "01_foundation");

    let phase_use_json = run_neb_json(&["--root", root_str, "phase", "use", "02_polish"], None);
    assert_eq!(phase_use_json["active_phase"], "02_polish");

    let phase_list_json = run_neb_json(&["--root", root_str, "phase", "list"], None);
    assert_eq!(phase_list_json["active_phase"], "02_polish");
    assert_eq!(phase_list_json["phases"].as_array().unwrap().len(), 2);

    // 12. `neb sync-index` repairs manually corrupted `scope.toml`
    let storage = TrackerStorage::new(temp.path());
    let mut scope_manifest = storage.read_scope("01_foundation", "backend_core").unwrap();
    scope_manifest.items.bugs.clear();
    scope_manifest.items.features.clear();
    storage
        .write_scope("01_foundation", "backend_core", &scope_manifest)
        .unwrap();

    let sync_idx_json = run_neb_json(&["--root", root_str, "sync-index"], None);
    assert_eq!(sync_idx_json["phases_reconciled"], 2);
    assert_eq!(sync_idx_json["total_items"], 3);

    let restored_scope = storage.read_scope("01_foundation", "backend_core").unwrap();
    assert_eq!(restored_scope.items.bugs.len(), 1);
    assert_eq!(restored_scope.items.features.len(), 1);
}

#[test]
fn test_live_repository_tracker_directory_and_cloned_mutations() {
    let fixture = TempDir::new().expect("should create temp dogfood fixture dir");
    let root = fixture.path();
    let root_str = root.to_str().unwrap();

    let live_storage = seed_dogfood_tracker(root);
    let index_before = live_storage
        .load_index()
        .expect("seeded .tracker/ index should load cleanly");

    // 1. Verify read-only commands directly against the seeded `.tracker/` fixture directory
    let list_human = run_neb_ok(&["--root", root_str, "list"], None);
    assert!(list_human
        .contains("BUG-0001   bug      closed       high      01_core_engine   backend_core"));
    assert!(list_human
        .contains("FEAT-0001  feature  closed       high      01_core_engine   backend_core"));
    assert!(list_human
        .contains("FEAT-0004  feature  pending      medium    01_core_engine   nebulous_cli"));

    let list_json = run_neb_json(&["--root", root_str, "list"], None);
    assert_eq!(list_json["project_name"], "Nebulous");
    assert!(list_json["total"].as_u64().unwrap() >= 6);

    let show_out = run_neb_ok(&["--root", root_str, "show", "BUG-0001"], None);
    assert!(show_out.contains("BUG-0001"));
    assert!(show_out.contains("backend_core"));

    let context_refs = run_neb_ok(
        &["--root", root_str, "context", "FEAT-0004", "--refs-only"],
        None,
    );
    assert!(
        context_refs.contains("@.tracker/phases/01_core_engine/domains/nebulous_cli/scope.toml")
    );
    assert!(context_refs.contains(
        "@.tracker/phases/01_core_engine/domains/nebulous_cli/items/FEAT-0004/item.toml"
    ));

    let context_prompt = run_neb_ok(&["--root", root_str, "context", "BUG-0001"], None);
    assert!(context_prompt.contains("# Bounded Agent Task: BUG-0001"));
    assert!(context_prompt.contains("crates/backend_core"));

    let metrics_human = run_neb_ok(&["--root", root_str, "metrics"], None);
    assert!(metrics_human.contains("Nebulous Project Metrics & Valuation"));
    assert!(metrics_human.contains("01_core_engine"));

    let metrics_json = run_neb_json(&["--root", root_str, "metrics"], None);
    assert!(metrics_json["total_items"].as_u64().unwrap() >= 6);
    assert!(metrics_json["completed_items"].as_u64().unwrap() >= 2);

    // Verify `neb sync-index` against seeded `.tracker/` preserves the exact index state
    let sync_live_json = run_neb_json(&["--root", root_str, "sync-index"], None);
    assert_eq!(sync_live_json["phases_reconciled"], 2);
    assert_eq!(sync_live_json["scopes_reconciled"], 8);
    let index_after = live_storage.load_index().unwrap();
    assert_eq!(index_before, index_after);

    // 2. Copy the seeded `.tracker/` directory to another temporary workspace and exercise mutating
    // commands (`neb bug`, `neb feat`, `neb timer`, `neb close`, `neb reopen`, `neb sync-index`)
    let temp_live = TempDir::new().expect("should create temp copy dir");
    copy_dir_recursive(&root.join(".tracker"), &temp_live.path().join(".tracker"))
        .expect("should copy seeded .tracker/ into temp dir");
    let copy_root_str = temp_live.path().to_str().unwrap();

    let new_bug = run_neb_json(
        &[
            "--root",
            copy_root_str,
            "bug",
            "CLI table alignment edge case",
            "-s",
            "nebulous_cli",
            "--severity",
            "low",
        ],
        None,
    );
    assert_eq!(new_bug["id"], "BUG-0002");
    assert_eq!(new_bug["scope_id"], "nebulous_cli");

    let new_feat = run_neb_json(
        &[
            "--root",
            copy_root_str,
            "feat",
            "JSON output flag for all neb subcommands",
            "-s",
            "nebulous_cli",
            "--severity",
            "high",
        ],
        None,
    );
    assert_eq!(new_feat["id"], "FEAT-0006");

    run_neb_ok(
        &["--root", copy_root_str, "timer", "start", "FEAT-0004"],
        None,
    );
    run_neb_ok(&["--root", copy_root_str, "timer", "stop"], None);

    let closed_feat4 = run_neb_json(
        &[
            "--root",
            copy_root_str,
            "close",
            "FEAT-0004",
            "-m",
            "Implemented all 16 neb subcommands in crates/nebulous_cli",
            "--merged",
        ],
        None,
    );
    assert_eq!(closed_feat4["status"], "merged");

    let reopened_feat4 = run_neb_json(
        &[
            "--root",
            copy_root_str,
            "reopen",
            "FEAT-0004",
            "-r",
            "Add extra integration test assertions",
        ],
        None,
    );
    assert_eq!(reopened_feat4["status"], "open");
    assert_eq!(reopened_feat4["recurrence_count"], 1);

    let reclosed_feat4 = run_neb_json(
        &[
            "--root",
            copy_root_str,
            "close",
            "FEAT-0004",
            "-m",
            "Completed integration test coverage",
        ],
        None,
    );
    assert_eq!(reclosed_feat4["status"], string_status(ItemStatus::Closed));
    assert_eq!(reclosed_feat4["previous_fixes_count"], 2);

    let reconcile_copy = run_neb_json(&["--root", copy_root_str, "sync-index"], None);
    assert_eq!(reconcile_copy["total_items"], 8);
    assert_eq!(reconcile_copy["counters"]["bug"], 2);
    assert_eq!(reconcile_copy["counters"]["feature"], 6);
}

fn string_status(st: ItemStatus) -> String {
    st.to_string()
}

#[test]
fn test_git_sync_and_error_edge_cases() {
    let temp = TempDir::new().unwrap();
    let root_str = temp.path().to_str().unwrap();

    // Error case 1: Running `neb list` in a directory without `.tracker/`
    let missing_out = run_neb(&["--root", root_str, "list"], None);
    assert!(!missing_out.status.success());

    // Initialize git repo + `.tracker/`
    let repo = Repository::init(temp.path()).expect("should init git repo");
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "Test User").unwrap();
        cfg.set_str("user.email", "test@example.com").unwrap();
        let sig = Signature::now("Test User", "test@example.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();
    }

    run_neb_ok(&["--root", root_str, "init", "--name", "GitTest"], None);
    run_neb_ok(
        &["--root", root_str, "bug", "Bug to commit via neb sync"],
        None,
    );

    // Error case 2: Empty title rejected
    let empty_title = run_neb(&["--root", root_str, "bug", "   "], None);
    assert!(!empty_title.status.success());

    // Error case 3: Reopening an already-open item fails
    let reopen_open = run_neb(&["--root", root_str, "reopen", "BUG-0001"], None);
    assert!(!reopen_open.status.success());

    // Error case 4: Closing with an empty summary fails
    let empty_close = run_neb(
        &["--root", root_str, "close", "BUG-0001", "-m", "   "],
        None,
    );
    assert!(!empty_close.status.success());

    // Error case 5: Stopping timer when none is active fails
    let stop_inactive = run_neb(&["--root", root_str, "timer", "stop"], None);
    assert!(!stop_inactive.status.success());

    // Error case 6: Starting a second timer while one is active fails
    run_neb_ok(&["--root", root_str, "timer", "start", "BUG-0001"], None);
    let double_start = run_neb(&["--root", root_str, "timer", "start", "BUG-0001"], None);
    assert!(!double_start.status.success());

    // Error case 7: Stopping timer with mismatched item ID fails
    let mismatch_stop = run_neb(&["--root", root_str, "timer", "stop", "BUG-9999"], None);
    assert!(!mismatch_stop.status.success());
    run_neb_ok(&["--root", root_str, "timer", "stop", "BUG-0001"], None);

    // `neb sync` commits dirty `.tracker/` files into the Git repository
    let sync_json = run_neb_json(
        &[
            "--root",
            root_str,
            "sync",
            "-m",
            "chore(tracker): snapshot BUG-0001",
        ],
        None,
    );
    assert!(sync_json["committed_or_head_sha"].as_str().is_some());
    assert_eq!(sync_json["tracker_dirty_after"], false);
}
