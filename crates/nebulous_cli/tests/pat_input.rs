//! STEP-11: PATs come from stdin (`--pat-stdin`) or `NEB_GIT_PAT`, never from argv.

use std::io::{Cursor, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};

use backend_core::TrackerStorage;
use clap::Parser;
use git2::Repository;
use nebulous_cli::commands::{read_pat_input, GIT_PAT_ENV};
use nebulous_cli::{Cli, Commands, GitCommand};
use tempfile::TempDir;

#[test]
fn pat_argv_value_form_is_rejected() {
    for argv in [
        &["neb", "git", "push", "--pat", "ghp_argv"][..],
        &["neb", "git", "pull", "--pat=ghp_argv"][..],
        &["neb", "git", "fetch", "--pat", "ghp_argv"][..],
        &["neb", "git", "sync", "--pat", "ghp_argv"][..],
        &["neb", "sync", "--pat", "ghp_argv"][..],
    ] {
        assert!(
            Cli::try_parse_from(argv).is_err(),
            "argv PAT must be rejected: {argv:?}"
        );
    }
}

#[test]
fn pat_stdin_flag_parses_without_a_value() {
    let cli = Cli::try_parse_from(["neb", "git", "push", "--pat-stdin"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Commands::Git {
            action: GitCommand::Push { pat_stdin: true }
        })
    ));
    let cli = Cli::try_parse_from(["neb", "sync", "--pat-stdin", "--commit-only"]).unwrap();
    assert!(matches!(
        cli.command,
        Some(Commands::Sync {
            pat_stdin: true,
            ..
        })
    ));
}

#[test]
fn read_pat_input_prefers_stdin_then_env() {
    assert_eq!(GIT_PAT_ENV, "NEB_GIT_PAT");

    let from_stdin = read_pat_input(
        true,
        Some("ghp_env".to_string()),
        Cursor::new("  ghp_stdin  \nignored second line\n"),
    )
    .unwrap();
    assert_eq!(from_stdin.as_deref().map(String::as_str), Some("ghp_stdin"));

    let from_env =
        read_pat_input(false, Some(" ghp_env\n".to_string()), Cursor::new("unused")).unwrap();
    assert_eq!(from_env.as_deref().map(String::as_str), Some("ghp_env"));

    assert!(
        read_pat_input(false, Some("   ".to_string()), Cursor::new(""))
            .unwrap()
            .is_none()
    );
    assert!(read_pat_input(false, None, Cursor::new("ghp_not_read"))
        .unwrap()
        .is_none());

    let err = read_pat_input(true, None, Cursor::new("\n")).expect_err("empty stdin with flag");
    assert!(err.to_string().contains("--pat-stdin"));
}

#[test]
fn neb_git_push_reads_pat_from_stdin() {
    let dir = TempDir::new().unwrap();
    let cfg = dir.path().join("cfg");
    TrackerStorage::init(dir.path(), "PatStdin", "01_core_engine", "Core").unwrap();
    Repository::init(dir.path()).unwrap();

    let mut child = Command::new(PathBuf::from(env!("CARGO_BIN_EXE_neb")))
        .arg("--root")
        .arg(dir.path())
        .args(["git", "push", "--pat-stdin"])
        .env("NEBULOUS_CONFIG_DIR", &cfg)
        .env_remove(GIT_PAT_ENV)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn neb");
    // Empty stdin: the CLI must try to read the token from stdin and refuse.
    child.stdin.take().unwrap().write_all(b"\n").unwrap();
    let out = child.wait_with_output().unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--pat-stdin"),
        "expected stdin PAT error, got: {stderr}"
    );
}
