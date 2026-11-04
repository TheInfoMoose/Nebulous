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
    let isolated_config = TempDir::new().expect("should create isolated NEBULOUS_CONFIG_DIR");
    let mut cmd = Command::new(neb_bin());
    cmd.args(args);
    cmd.env("NEBULOUS_CONFIG_DIR", isolated_config.path());
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

fn run_neb_with_env(args: &[&str], current_dir: Option<&Path>, config_dir: &Path) -> Output {
    let mut cmd = Command::new(neb_bin());
    cmd.args(args);
    cmd.env("NEBULOUS_CONFIG_DIR", config_dir);
    if let Some(dir) = current_dir {
        cmd.current_dir(dir);
    }
    cmd.output().expect("should execute neb binary")
}

fn run_neb_json_with_env(args: &[&str], current_dir: Option<&Path>, config_dir: &Path) -> Value {
    let mut full_args = vec!["--json"];
    full_args.extend_from_slice(args);
    let output = run_neb_with_env(&full_args, current_dir, config_dir);
    if !output.status.success() {
        panic!(
            "neb {:?} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
            full_args,
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let stdout = String::from_utf8(output.stdout).expect("stdout should be valid UTF-8");
    serde_json::from_str(&stdout).unwrap_or_else(|err| {
        panic!(
            "Failed to parse JSON output from neb {:?}: {err}\nRaw stdout:\n{stdout}",
            full_args
        )
    })
}

#[test]
fn test_project_registry_ingest_component_pin_rollback_and_git_commit() {
    let temp = TempDir::new().expect("should create temp dir");
    let cfg_dir = temp.path().join("nebulous_config");
    let proj_alpha = temp.path().join("novos_alpha");
    let proj_beta = temp.path().join("neb_beta");
    fs::create_dir_all(&proj_alpha).unwrap();
    fs::create_dir_all(&proj_beta).unwrap();

    // Initialize Git repository in proj_alpha
    let repo = Repository::init(&proj_alpha).expect("should init git repo");
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "NovOS Bot").unwrap();
        cfg.set_str("user.email", "bot@novos.dev").unwrap();
        let sig = Signature::now("NovOS Bot", "bot@novos.dev").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();
    }

    // Create sample multi-ecosystem directories in proj_alpha
    fs::create_dir_all(proj_alpha.join("crates/core_lib/src")).unwrap();
    fs::write(
        proj_alpha.join("crates/core_lib/Cargo.toml"),
        "[package]\nname = \"core_lib\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        proj_alpha.join("crates/core_lib/src/lib.rs"),
        "pub fn boot() -> bool { true }\n",
    )
    .unwrap();

    fs::create_dir_all(proj_alpha.join("iso")).unwrap();
    fs::write(
        proj_alpha.join("iso/build_iso.sh"),
        "#!/usr/bin/env bash\necho 'Building NovOS ISO'\n",
    )
    .unwrap();

    let alpha_str = proj_alpha.to_str().unwrap();
    let beta_str = proj_beta.to_str().unwrap();

    // 1. `neb project add | list | switch | remove`
    let add_alpha = run_neb_json_with_env(
        &[
            "project",
            "add",
            alpha_str,
            "--name",
            "NovOS Alpha",
            "--tag",
            "distro,sid,xanmod",
        ],
        None,
        &cfg_dir,
    );
    assert_eq!(add_alpha["action"], "add");
    assert_eq!(add_alpha["id"], "novos_alpha");
    assert_eq!(add_alpha["name"], "NovOS Alpha");
    assert_eq!(add_alpha["active_project_id"], "novos_alpha");

    let add_beta = run_neb_json_with_env(
        &["project", "add", beta_str, "--name", "Neb Beta"],
        None,
        &cfg_dir,
    );
    assert_eq!(add_beta["id"], "neb_beta");
    assert_eq!(add_beta["active_project_id"], "neb_beta");

    let list_projects = run_neb_json_with_env(&["project", "list"], None, &cfg_dir);
    assert_eq!(list_projects["total"], 2);
    assert_eq!(list_projects["active_project_id"], "neb_beta");

    let switch_alpha = run_neb_json_with_env(&["project", "switch", "NovOS Alpha"], None, &cfg_dir);
    assert_eq!(switch_alpha["action"], "switch");
    assert_eq!(switch_alpha["active_project_id"], "novos_alpha");

    let remove_beta = run_neb_json_with_env(&["project", "remove", "neb_beta"], None, &cfg_dir);
    assert_eq!(remove_beta["action"], "remove");
    assert_eq!(remove_beta["id"], "neb_beta");
    assert_eq!(remove_beta["active_project_id"], "novos_alpha");

    // 2. `neb ingest --write-manifests --promote-all`
    let ingest_json = run_neb_json_with_env(
        &[
            "--root",
            alpha_str,
            "ingest",
            "--write-manifests",
            "--promote-all",
        ],
        None,
        &cfg_dir,
    );
    assert_eq!(ingest_json["total_components"], 2);
    assert_eq!(ingest_json["promoted_scopes"].as_array().unwrap().len(), 2);
    assert!(proj_alpha
        .join("crates/core_lib/nebulous.component.toml")
        .is_file());
    assert!(proj_alpha.join("iso/nebulous.component.toml").is_file());

    // 3. `neb component list | show | init | pin | rollback`
    let comp_list =
        run_neb_json_with_env(&["--root", alpha_str, "component", "list"], None, &cfg_dir);
    assert_eq!(comp_list["total"], 2);

    let comp_init = run_neb_json_with_env(
        &[
            "--root",
            alpha_str,
            "component",
            "init",
            "iso",
            "--id",
            "iso",
            "--name",
            "NovOS Live ISO",
            "--type",
            "os_image",
            "--flag",
            "--xanmod-lto,--zstd",
        ],
        None,
        &cfg_dir,
    );
    assert_eq!(comp_init["id"], "iso");
    assert_eq!(comp_init["component_type"], "os_image");

    // Pin stable release with dependency versions
    let comp_pin = run_neb_json_with_env(
        &[
            "--root",
            alpha_str,
            "component",
            "pin",
            "iso",
            "--commit",
            "good123",
            "--script",
            "iso/build_iso.sh",
            "--dep",
            "linux-xanmod=6.12.4-1,systemd=256.8-1",
        ],
        None,
        &cfg_dir,
    );
    assert_eq!(comp_pin["action"], "pin");
    assert_eq!(comp_pin["last_known_good"]["commit_sha"], "good123");
    assert_eq!(
        comp_pin["last_known_good"]["dependency_pins"]["linux-xanmod"],
        "6.12.4-1"
    );

    let comp_show = run_neb_json_with_env(
        &["--root", alpha_str, "component", "show", "iso"],
        None,
        &cfg_dir,
    );
    assert_eq!(comp_show["pinned_release"]["commit_sha"], "good123");

    // Create an active bug on `iso` scope and simulate a broken rolling update, then rollback
    let bug_json = run_neb_json_with_env(
        &[
            "--root",
            alpha_str,
            "bug",
            "Boot failure after XanMod 6.13-rc1 bump",
            "-s",
            "iso",
            "--severity",
            "critical",
        ],
        None,
        &cfg_dir,
    );
    assert_eq!(bug_json["id"], "BUG-0001");

    // Verify `neb context --copy` gracefully falls back on headless CI without failing
    let ctx_copy = run_neb_json_with_env(
        &["--root", alpha_str, "context", "BUG-0001", "--copy"],
        None,
        &cfg_dir,
    );
    assert_eq!(ctx_copy["item_id"], "BUG-0001");
    assert!(ctx_copy["output"]
        .as_str()
        .unwrap()
        .contains("Bounded Agent Task: BUG-0001"));

    // Mutate dependency version on disk, then run `neb component rollback iso -i BUG-0001`
    let storage = TrackerStorage::new(&proj_alpha);
    storage
        .record_component_rolling_update(
            "iso",
            backend_core::RollingUpdateEntry {
                dependency_name: "linux-xanmod".to_string(),
                kind: "kernel_xanmod".to_string(),
                previous_version: Some("6.12.4-1".to_string()),
                new_version: "6.13.0-rc1".to_string(),
                tracked_item_id: Some("BUG-0001".to_string()),
                status: "failed".to_string(),
                ..Default::default()
            },
        )
        .unwrap();

    let rollback_json = run_neb_json_with_env(
        &[
            "--root",
            alpha_str,
            "component",
            "rollback",
            "iso",
            "-i",
            "BUG-0001",
            "-r",
            "QEMU boot panicked in initramfs with 6.13.0-rc1",
        ],
        None,
        &cfg_dir,
    );
    assert_eq!(rollback_json["action"], "rollback");
    assert_eq!(rollback_json["restored"], true);
    assert_eq!(rollback_json["regression_recorded"], true);
    assert_eq!(rollback_json["item_id"], "BUG-0001");
    assert_eq!(rollback_json["recurrence_count"], 1);

    let restored_iso = storage.read_component_manifest("iso").unwrap();
    let xanmod_dep = restored_iso
        .dependencies
        .iter()
        .find(|d| d.name == "linux-xanmod")
        .unwrap();
    assert_eq!(xanmod_dep.current_version, "6.12.4-1");

    // 4. `neb git status` and `neb git commit -i <ITEM_ID> -m <SUMMARY> --all`
    let git_status_before =
        run_neb_json_with_env(&["--root", alpha_str, "git", "status"], None, &cfg_dir);
    assert_eq!(git_status_before["workspace_dirty"], true);
    assert_eq!(git_status_before["tracker_dirty"], true);

    // Policy check: attempting to commit workspace code without `-i <ITEM_ID>` fails when require_item_id is true
    let missing_item_commit = run_neb_with_env(
        &[
            "--root",
            alpha_str,
            "git",
            "commit",
            "-m",
            "Undocumented workspace commit attempt",
            "--all",
        ],
        None,
        &cfg_dir,
    );
    assert!(
        !missing_item_commit.status.success(),
        "undocumented workspace commit must be rejected by commit_policy"
    );

    // Documented commit with `-i BUG-0001 -m <SUMMARY> --all` succeeds and updates item.toml + desc.md atomically
    let commit_json = run_neb_json_with_env(
        &[
            "--root",
            alpha_str,
            "git",
            "commit",
            "-i",
            "BUG-0001",
            "-m",
            "Restore XanMod 6.12.4-1 pin and stabilize ISO build script",
            "--all",
        ],
        None,
        &cfg_dir,
    );
    assert!(commit_json["commit_sha"].as_str().is_some());
    assert_eq!(commit_json["item_id"], "BUG-0001");
    assert_eq!(commit_json["scope_id"], "iso");
    assert_eq!(commit_json["documentation_updated"], true);
    assert!(commit_json["message"]
        .as_str()
        .unwrap()
        .starts_with("[BUG-0001](iso) Restore XanMod 6.12.4-1 pin"));

    let git_status_after =
        run_neb_json_with_env(&["--root", alpha_str, "git", "status"], None, &cfg_dir);
    assert_eq!(git_status_after["workspace_dirty"], false);
    assert_eq!(git_status_after["tracker_dirty"], false);

    let (_, detail_after) = storage.load_item_detail_by_id("BUG-0001").unwrap();
    assert_eq!(detail_after.manifest.history.previous_fixes.len(), 1);
    assert!(detail_after
        .description_markdown
        .contains("Restore XanMod 6.12.4-1 pin and stabilize ISO build script"));
}

#[test]
fn test_mcp_stdio_jsonrpc_session_end_to_end() {
    use std::io::Write;
    use std::process::Stdio;

    let temp = TempDir::new().expect("should create temp dir for MCP test");
    let cfg_dir = temp.path().join("mcp_config");
    let root = temp.path().join("novos_workspace");
    fs::create_dir_all(&root).unwrap();

    // Initialize Git repository + files for ingestion
    let repo = Repository::init(&root).expect("should init git repo");
    {
        let mut cfg = repo.config().unwrap();
        cfg.set_str("user.name", "MCP Agent").unwrap();
        cfg.set_str("user.email", "agent@nebulous.dev").unwrap();
        let sig = Signature::now("MCP Agent", "agent@nebulous.dev").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();
    }

    fs::create_dir_all(root.join("crates/kernel_mgr/src")).unwrap();
    fs::write(
        root.join("crates/kernel_mgr/Cargo.toml"),
        "[package]\nname = \"kernel_mgr\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(
        root.join("crates/kernel_mgr/src/lib.rs"),
        "pub fn verify_boot() -> bool { true }\n",
    )
    .unwrap();

    fs::create_dir_all(root.join("iso")).unwrap();
    fs::write(
        root.join("iso/build.sh"),
        "#!/usr/bin/env bash\necho 'building iso'\n",
    )
    .unwrap();

    let storage = TrackerStorage::init(&root, "NovOS", "01_rolling_core", "Rolling Release Core")
        .expect("should init NovOS tracker");
    let _ = backend_core::register_project_in(
        &cfg_dir,
        storage.project_root(),
        Some("NovOS"),
        vec!["distro".to_string()],
        true,
    )
    .unwrap();

    // Build a sequence of JSON-RPC 2.0 requests exercising the full MCP lifecycle and all 4 tool suites
    let requests = vec![
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "test-agent", "version": "1.0.0" }
            }
        }),
        // Notification (no id) -> must not emit a response line
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "ingest_project_directory",
                "arguments": {
                    "write_manifests": true,
                    "promote_all": true
                }
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "list_crate_boundaries",
                "arguments": {}
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "pin_stable_release",
                "arguments": {
                    "component_id": "iso",
                    "commit_sha": "stable01",
                    "build_script_path": "iso/build.sh",
                    "build_flags": ["--xanmod-signed", "--squashfs-zstd"],
                    "dependency_pins": {
                        "linux-xanmod": "6.12.4-1",
                        "live-build": "20261101"
                    }
                }
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "record_rolling_update",
                "arguments": {
                    "component_id": "iso",
                    "dependency_name": "linux-xanmod",
                    "kind": "kernel_xanmod",
                    "previous_version": "6.12.4-1",
                    "new_version": "6.13.0-rc2",
                    "upstream_source": "https://deb.xanmod.org",
                    "build_script_path": "iso/build.sh",
                    "summary": "Bump XanMod kernel to 6.13.0-rc2"
                }
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "get_active_task",
                "arguments": {}
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": {
                "name": "record_verification_run",
                "arguments": {
                    "component_id": "iso",
                    "passed": false,
                    "exit_code": 134,
                    "command": "qemu-system-x86_64 -cdrom novos.iso --nographic",
                    "summary": "Kernel panic mounting overlayfs in live boot",
                    "logs_excerpt": "VFS: Cannot open root device overlay"
                }
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {
                "name": "rollback_to_pinned_release",
                "arguments": {
                    "component_id": "iso",
                    "restore_manifest": true,
                    "reason": "Auto-rollback triggered after QEMU exit code 134"
                }
            }
        }),
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": {
                "name": "git_commit_documented",
                "arguments": {
                    "item_id": "FEAT-0001",
                    "summary": "Rollback linux-xanmod to 6.12.4-1 after QEMU boot failure"
                }
            }
        }),
        // Edge case: unknown tool returns JSON-RPC error -32602
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": {
                "name": "nonexistent_tool",
                "arguments": {}
            }
        }),
    ];

    let mut stdin_payload = String::new();
    for req in &requests {
        stdin_payload.push_str(&serde_json::to_string(req).unwrap());
        stdin_payload.push('\n');
    }

    let mut child = Command::new(neb_bin())
        .args(["--root", root.to_str().unwrap(), "mcp"])
        .env("NEBULOUS_CONFIG_DIR", &cfg_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("should spawn neb mcp subprocess");

    {
        let child_stdin = child.stdin.as_mut().expect("stdin should be piped");
        child_stdin
            .write_all(stdin_payload.as_bytes())
            .expect("should write JSON-RPC lines to neb mcp stdin");
    }

    let output = child
        .wait_with_output()
        .expect("neb mcp should exit cleanly when stdin closes");
    assert!(
        output.status.success(),
        "neb mcp failed:\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout_str = String::from_utf8(output.stdout).expect("valid UTF-8 stdout");
    let responses: Vec<Value> = stdout_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| serde_json::from_str::<Value>(line).expect("each line must be valid JSON-RPC"))
        .collect();

    // 11 requests with `id` (the notification had no `id` and emitted no response)
    assert_eq!(responses.len(), 11);

    // 1. initialize
    assert_eq!(responses[0]["id"], 1);
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "nebulous-mcp");

    // 2. tools/list
    assert_eq!(responses[1]["id"], 2);
    let tool_names: Vec<&str> = responses[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    for expected_tool in [
        "get_active_task",
        "list_items",
        "get_item_context",
        "list_crate_boundaries",
        "create_item",
        "update_item_status",
        "close_item_with_fix",
        "record_regression",
        "list_projects",
        "switch_project",
        "ingest_project_directory",
        "upsert_component_manifest",
        "promote_component_to_scope",
        "record_rolling_update",
        "record_verification_run",
        "pin_stable_release",
        "rollback_to_pinned_release",
        "git_status",
        "git_fetch",
        "git_pull",
        "git_push",
        "git_commit_documented",
        "git_sync",
    ] {
        assert!(
            tool_names.contains(&expected_tool),
            "tools/list missing expected tool '{expected_tool}'"
        );
    }

    // 3. ingest_project_directory
    assert_eq!(responses[2]["id"], 3);
    assert_eq!(responses[2]["result"]["isError"], false);
    assert_eq!(responses[2]["result"]["total_components"], 2);

    // 4. list_crate_boundaries
    assert_eq!(responses[3]["id"], 4);
    let boundaries = responses[3]["result"]["boundaries"].as_array().unwrap();
    assert!(boundaries.iter().any(|b| b["scope_id"] == "iso"));
    assert!(boundaries.iter().any(|b| b["scope_id"] == "kernel_mgr"));

    // 5. pin_stable_release
    assert_eq!(responses[4]["id"], 5);
    assert_eq!(
        responses[4]["result"]["last_known_good"]["commit_sha"],
        "stable01"
    );
    assert_eq!(
        responses[4]["result"]["last_known_good"]["dependency_pins"]["linux-xanmod"],
        "6.12.4-1"
    );

    // 6. record_rolling_update -> auto-created tracking item FEAT-0001
    assert_eq!(responses[5]["id"], 6);
    assert_eq!(responses[5]["result"]["tracked_item_id"], "FEAT-0001");
    assert_eq!(
        responses[5]["result"]["rolling_update"]["new_version"],
        "6.13.0-rc2"
    );

    // 7. get_active_task -> returns FEAT-0001 with Bounded Agent Prompt
    assert_eq!(responses[6]["id"], 7);
    assert_eq!(responses[6]["result"]["active"], true);
    assert_eq!(responses[6]["result"]["item_id"], "FEAT-0001");
    assert_eq!(responses[6]["result"]["scope_id"], "iso");
    assert!(responses[6]["result"]["bounded_prompt"]
        .as_str()
        .unwrap()
        .contains("Bounded Agent Task: FEAT-0001"));

    // 8. record_verification_run -> records failure on iso and FEAT-0001
    assert_eq!(responses[7]["id"], 8);
    assert_eq!(responses[7]["result"]["passed"], false);
    assert_eq!(responses[7]["result"]["exit_code"], 134);
    assert_eq!(responses[7]["result"]["tracked_item_id"], "FEAT-0001");

    // 9. rollback_to_pinned_release -> restores 6.12.4-1 and records regression on FEAT-0001
    assert_eq!(responses[8]["id"], 9);
    assert_eq!(responses[8]["result"]["restored"], true);
    assert_eq!(responses[8]["result"]["regression_recorded"], true);
    assert_eq!(responses[8]["result"]["item_id"], "FEAT-0001");
    assert_eq!(responses[8]["result"]["recurrence_count"], 1);
    assert_eq!(
        responses[8]["result"]["last_known_good"]["commit_sha"],
        "stable01"
    );

    // 10. git_commit_documented -> commits workspace + .tracker/ and updates FEAT-0001
    assert_eq!(responses[9]["id"], 10);
    assert_eq!(responses[9]["result"]["isError"], false);
    assert_eq!(responses[9]["result"]["item_id"], "FEAT-0001");
    assert_eq!(responses[9]["result"]["documentation_updated"], true);

    // 11. unknown tool -> JSON-RPC error -32602
    assert_eq!(responses[10]["id"], 11);
    assert_eq!(responses[10]["error"]["code"], -32602);
}

#[test]
fn test_mcp_remaining_tools_resources_content_length_and_error_paths() {
    use backend_core::McpServer;
    use std::io::Cursor;

    let temp = TempDir::new().expect("should create temp dir");
    let cfg_dir = temp.path().join("mcp_cfg");
    let proj_one = temp.path().join("proj_one");
    let proj_two = temp.path().join("proj_two");
    fs::create_dir_all(&proj_one).unwrap();
    fs::create_dir_all(&proj_two).unwrap();

    let s1 = TrackerStorage::init(&proj_one, "ProjOne", "01_core", "Core Phase").unwrap();
    let s2 = TrackerStorage::init(&proj_two, "ProjTwo", "01_core", "Core Phase").unwrap();
    backend_core::register_project_in(&cfg_dir, s1.project_root(), Some("ProjOne"), vec![], true)
        .unwrap();
    backend_core::register_project_in(&cfg_dir, s2.project_root(), Some("ProjTwo"), vec![], false)
        .unwrap();

    let mut server = McpServer::from_storage(s1.clone()).with_registry_path(&cfg_dir);

    // 1. ping
    let ping_resp: Value = serde_json::from_str(
        &server
            .handle_raw_message(r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(ping_resp["id"], 1);
    assert!(ping_resp["result"].is_object());

    // 2. list_projects & switch_project
    let lp_val = server
        .call_tool("list_projects", serde_json::json!({}))
        .unwrap();
    assert_eq!(lp_val["total"], 2);
    assert_eq!(lp_val["active_project_id"], "projone");

    let sw_val = server
        .call_tool(
            "switch_project",
            serde_json::json!({ "id_or_name": "ProjTwo" }),
        )
        .unwrap();
    assert_eq!(sw_val["active_project_id"], "projtwo");

    // Switch back to ProjOne
    server
        .call_tool(
            "switch_project",
            serde_json::json!({ "id_or_name": "projone" }),
        )
        .unwrap();

    // 3. upsert_component_manifest & promote_component_to_scope
    fs::create_dir_all(proj_one.join("scripts")).unwrap();
    fs::write(proj_one.join("scripts/deploy.sh"), "#!/bin/sh\necho ok\n").unwrap();

    let upsert_val = server
        .call_tool(
            "upsert_component_manifest",
            serde_json::json!({
                "id": "deploy_scripts",
                "name": "Deploy Scripts",
                "relative_path": "scripts",
                "component_type": "build_scripts",
                "tech_stack": ["shell"]
            }),
        )
        .unwrap();
    assert_eq!(upsert_val["component"]["id"], "deploy_scripts");

    let promote_val = server
        .call_tool(
            "promote_component_to_scope",
            serde_json::json!({
                "component_id": "deploy_scripts"
            }),
        )
        .unwrap();
    assert_eq!(promote_val["scope_id"], "deploy_scripts");
    assert_eq!(promote_val["bound_component"], "deploy_scripts");

    // 4. create_item, list_items, get_item_context, update_item_status, close_item_with_fix, record_regression
    let created = server
        .call_tool(
            "create_item",
            serde_json::json!({
                "item_type": "bug",
                "title": "Deploy script fails on spaces in path",
                "scope_id": "deploy_scripts",
                "severity": "high"
            }),
        )
        .unwrap();
    assert_eq!(created["item_id"], "BUG-0001");

    let listed = server
        .call_tool(
            "list_items",
            serde_json::json!({
                "scope_id": "deploy_scripts",
                "status": "open"
            }),
        )
        .unwrap();
    assert_eq!(listed["total"], 1);

    let ctx = server
        .call_tool(
            "get_item_context",
            serde_json::json!({
                "item_id": "BUG-0001"
            }),
        )
        .unwrap();
    assert_eq!(ctx["item_id"], "BUG-0001");
    assert_eq!(ctx["target_path"], "scripts");

    let updated = server
        .call_tool(
            "update_item_status",
            serde_json::json!({
                "item_id": "BUG-0001",
                "status": "in_progress"
            }),
        )
        .unwrap();
    assert_eq!(updated["status"], "in_progress");

    let closed = server
        .call_tool(
            "close_item_with_fix",
            serde_json::json!({
                "item_id": "BUG-0001",
                "summary": "Quoted path variables in deploy.sh",
                "commit": "fix1234"
            }),
        )
        .unwrap();
    assert_eq!(closed["status"], "closed");
    assert_eq!(closed["previous_fixes_count"], 1);

    let reg = server
        .call_tool(
            "record_regression",
            serde_json::json!({
                "item_id": "BUG-0001",
                "reason": "Still fails when symlink target has spaces"
            }),
        )
        .unwrap();
    assert_eq!(reg["status"], "open");
    assert_eq!(reg["recurrence_count"], 1);

    // 5. resources/list & resources/read
    let res_list: Value = serde_json::from_str(
        &server
            .handle_raw_message(r#"{"jsonrpc":"2.0","id":2,"method":"resources/list"}"#)
            .unwrap(),
    )
    .unwrap();
    let uris: Vec<&str> = res_list["result"]["resources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["uri"].as_str().unwrap())
        .collect();
    assert!(uris.contains(&"nebulous://project"));
    assert!(uris.contains(&"nebulous://items/BUG-0001"));

    let res_read: Value = serde_json::from_str(
        &server
            .handle_raw_message(
                r#"{"jsonrpc":"2.0","id":3,"method":"resources/read","params":{"uri":"nebulous://items/BUG-0001"}}"#,
            )
            .unwrap(),
    )
    .unwrap();
    assert!(res_read["result"]["contents"][0]["text"]
        .as_str()
        .unwrap()
        .contains("BUG-0001"));

    // 6. Error handling: malformed JSON (-32700), unknown method (-32601), rollback without pin (isError: true)
    let parse_err: Value =
        serde_json::from_str(&server.handle_raw_message("{not valid json").unwrap()).unwrap();
    assert_eq!(parse_err["error"]["code"], -32700);

    let method_err: Value = serde_json::from_str(
        &server
            .handle_raw_message(r#"{"jsonrpc":"2.0","id":4,"method":"unknown/method"}"#)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(method_err["error"]["code"], -32601);

    let unpinned_rollback: Value = serde_json::from_str(
        &server
            .handle_raw_message(
                r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"rollback_to_pinned_release","arguments":{"component_id":"deploy_scripts"}}}"#,
            )
            .unwrap(),
    )
    .unwrap();
    assert_eq!(unpinned_rollback["result"]["isError"], true);

    // 7. record_rolling_update on a brand-new dependency without previous_version keeps previous_version = null
    let rolling_new_dep = server
        .call_tool(
            "record_rolling_update",
            serde_json::json!({
                "component_id": "deploy_scripts",
                "dependency_name": "rsync",
                "new_version": "3.3.0-1",
                "create_tracking_item": false
            }),
        )
        .unwrap();
    assert!(rolling_new_dep["rolling_update"]["previous_version"].is_null());
    assert_eq!(rolling_new_dep["rolling_update"]["new_version"], "3.3.0-1");

    // 8. Content-Length framed stdio session AND newline-delimited malformed line recovery
    let body = r#"{"jsonrpc":"2.0","id":99,"method":"ping"}"#;
    let framed_input = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    let mut out_buf = Vec::new();
    server
        .serve_io(Cursor::new(framed_input.into_bytes()), &mut out_buf)
        .unwrap();
    let framed_out = String::from_utf8(out_buf).unwrap();
    assert!(framed_out.starts_with("Content-Length: "));
    assert!(framed_out.contains(r#""id":99"#));

    let mixed_lines = "{not valid json\n{\"jsonrpc\":\"2.0\",\"id\":100,\"method\":\"ping\"}\n";
    let mut line_out_buf = Vec::new();
    server
        .serve_io(Cursor::new(mixed_lines.as_bytes()), &mut line_out_buf)
        .unwrap();
    let line_responses: Vec<Value> = String::from_utf8(line_out_buf)
        .unwrap()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(line_responses.len(), 2);
    assert_eq!(line_responses[0]["error"]["code"], -32700);
    assert_eq!(line_responses[1]["id"], 100);
}

#[test]
fn test_cli_active_directory_and_declared_directory_isolation() {
    use backend_core::McpServer;

    let temp = TempDir::new().expect("should create temp dir");
    let cfg_dir = temp.path().join("nebulous_cfg");
    let proj_dir = temp.path().join("active_proj");
    let untracked_dir = temp.path().join("untracked_folder");
    fs::create_dir_all(&proj_dir).unwrap();
    fs::create_dir_all(&untracked_dir).unwrap();

    // Initialize proj_dir and register it as active in projects.toml
    let proj_str = proj_dir.to_str().unwrap();
    run_neb_json_with_env(
        &["project", "add", proj_str, "--name", "ActiveProj"],
        None,
        &cfg_dir,
    );

    // Create two distinct component subdirectories in proj_dir: `crates/core_lib` and `iso`
    let core_lib_dir = proj_dir.join("crates/core_lib");
    fs::create_dir_all(core_lib_dir.join("src")).unwrap();
    fs::write(
        core_lib_dir.join("Cargo.toml"),
        "[package]\nname = \"core_lib\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    fs::write(core_lib_dir.join("src/lib.rs"), "pub fn init() {}\n").unwrap();

    let iso_dir = proj_dir.join("iso");
    fs::create_dir_all(&iso_dir).unwrap();
    fs::write(iso_dir.join("build.sh"), "#!/usr/bin/env bash\n").unwrap();

    // 1. Calling `neb list` or `neb ingest` in an uninitialized directory outside any `.tracker/`
    // MUST fail cleanly with TrackerRootNotFound and MUST NOT fall back to `projects.toml`!
    let list_untracked = run_neb_with_env(&["list"], Some(&untracked_dir), &cfg_dir);
    assert!(
        !list_untracked.status.success(),
        "neb list in untracked directory must not fall back to global registry"
    );
    let ingest_untracked = run_neb_with_env(&["ingest"], Some(&untracked_dir), &cfg_dir);
    assert!(
        !ingest_untracked.status.success(),
        "neb ingest in untracked directory must not fall back to global registry"
    );

    // 2. Calling `neb ingest` from a project subdirectory (`crates/core_lib`) without arguments
    // MUST isolate scanning strictly to the active directory (`crates/core_lib`) and NOT scan `iso/`!
    let ingest_from_subdir = run_neb_json_with_env(&["ingest"], Some(&core_lib_dir), &cfg_dir);
    assert_eq!(ingest_from_subdir["total_components"], 1);
    assert_eq!(ingest_from_subdir["components"][0]["id"], "core_lib");

    let storage = TrackerStorage::new(&proj_dir);
    let proj_after_subdir = storage.read_project().unwrap();
    assert_eq!(proj_after_subdir.components.len(), 1);
    assert!(proj_after_subdir.components.find("core_lib").is_some());
    assert!(
        proj_after_subdir.components.find("iso").is_none(),
        "sibling iso/ directory must not be scanned when neb ingest is called inside crates/core_lib"
    );

    // 3. Calling `neb ingest <DECLARED_DIR>` (even from an untracked directory or another subdir)
    // with an explicit path to `iso` isolates scanning strictly to `iso`!
    let iso_abs_str = iso_dir.to_str().unwrap();
    let ingest_declared_abs =
        run_neb_json_with_env(&["ingest", iso_abs_str], Some(&untracked_dir), &cfg_dir);
    assert_eq!(ingest_declared_abs["total_components"], 1);
    assert_eq!(ingest_declared_abs["components"][0]["id"], "iso");

    // Calling `neb ingest --path ../iso` (relative declared path from `crates/` directory)
    let crates_dir = proj_dir.join("crates");
    let ingest_declared_rel =
        run_neb_json_with_env(&["ingest", "--path", "../iso"], Some(&crates_dir), &cfg_dir);
    assert_eq!(ingest_declared_rel["total_components"], 1);
    assert_eq!(ingest_declared_rel["components"][0]["id"], "iso");

    // Calling `neb --root <PROJ> ingest <OUTSIDE_DIR>` must fail validation
    let outside_str = untracked_dir.to_str().unwrap();
    let ingest_outside_root = run_neb_with_env(
        &["--root", proj_str, "ingest", outside_str],
        Some(&proj_dir),
        &cfg_dir,
    );
    assert!(!ingest_outside_root.status.success());

    // Calling `neb --root . list` from a project subdirectory (`crates/core_lib`) resolves `.`
    // against CWD and walks up ancestor directories to find `.tracker/`
    let list_from_dot_root =
        run_neb_with_env(&["--root", ".", "list"], Some(&core_lib_dir), &cfg_dir);
    assert!(
        list_from_dot_root.status.success(),
        "neb --root . list from a project subdirectory should discover ancestor .tracker"
    );

    // 4. MCP `ingest_project_directory` with an absolute subdirectory path inside the active project
    // cleanly strips `project_root` and isolates scanning to that subdirectory
    let mut mcp = McpServer::from_storage(storage).with_registry_path(&cfg_dir);
    let mcp_ingest = mcp
        .call_tool(
            "ingest_project_directory",
            serde_json::json!({
                "subpath": core_lib_dir.to_string_lossy()
            }),
        )
        .expect("MCP ingest_project_directory should accept absolute path inside project_root");
    assert_eq!(mcp_ingest["total_components"], 1);
    assert_eq!(mcp_ingest["components"][0]["id"], "core_lib");

    // 5. MCP and CLI must reject an absolute path outside the active project or a symlink inside
    // the project that resolves outside the active project root
    assert!(mcp
        .call_tool(
            "ingest_project_directory",
            serde_json::json!({
                "subpath": untracked_dir.to_string_lossy()
            }),
        )
        .is_err());

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let escape_link = proj_dir.join("escape_link");
        symlink(&untracked_dir, &escape_link).unwrap();

        let cli_escape = run_neb_with_env(
            &["ingest", escape_link.to_str().unwrap()],
            Some(&proj_dir),
            &cfg_dir,
        );
        assert!(
            !cli_escape.status.success(),
            "neb ingest on a symlink resolving outside project_root must fail"
        );

        assert!(mcp
            .call_tool(
                "ingest_project_directory",
                serde_json::json!({
                    "subpath": escape_link.to_string_lossy()
                }),
            )
            .is_err());
    }
}
