pub mod args;
pub mod commands;
pub mod tui;

pub use args::{
    Cli, Commands, ComponentCommand, GitCommand, PhaseCommand, ProjectCommand, ScopeCommand,
    TimerCommand,
};
pub use commands::{execute, execute_to_string, resolve_storage};
pub use tui::{render_to_buffer, render_to_string, TuiApp};

#[cfg(test)]
mod tests {
    use backend_core::{ItemStatus, ItemType, Severity};
    use clap::{CommandFactory, Parser};

    use super::*;

    #[test]
    fn test_cli_clap_command_factory_debug_assert() {
        Cli::command().debug_assert();
    }

    #[test]
    fn test_parse_all_sixteen_subcommands() {
        // 0. bare `neb` and `neb tui` / `neb ui`
        let cli = Cli::try_parse_from(["neb"]).unwrap();
        assert!(cli.command.is_none());

        let cli = Cli::try_parse_from(["neb", "tui"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Tui)));

        let cli = Cli::try_parse_from(["neb", "ui"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Tui)));

        // 1. init
        let cli = Cli::try_parse_from(["neb", "init", "--name", "Demo"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Init {
                no_ingest: false,
                no_register: false,
                ..
            })
        ));

        // 2. bug
        let cli = Cli::try_parse_from([
            "neb",
            "--json",
            "bug",
            "Parser crash",
            "-s",
            "backend_core",
            "-p",
            "01_core_engine",
            "--severity",
            "high",
            "-d",
            "Repro steps",
        ])
        .unwrap();
        assert!(cli.json);
        assert!(matches!(
            cli.command,
            Some(Commands::Bug {
                severity: Severity::High,
                ..
            })
        ));

        // 3. feat
        let cli = Cli::try_parse_from(["neb", "feat", "New Kanban filter"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Feat { .. })));

        // 4. list
        let cli = Cli::try_parse_from([
            "neb",
            "list",
            "-s",
            "backend_core",
            "--status",
            "open",
            "--type",
            "bug",
            "--stagnant",
            "--recurring",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::List {
                status: Some(ItemStatus::Open),
                item_type: Some(ItemType::Bug),
                stagnant: true,
                recurring: true,
                ..
            })
        ));

        // 5. show (explicit and zero-ID)
        let cli = Cli::try_parse_from(["neb", "show", "BUG-0001"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Show { id: Some(_) })));
        let cli = Cli::try_parse_from(["neb", "show"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Show { id: None })));

        // 6. status
        let cli = Cli::try_parse_from(["neb", "status", "BUG-0001", "in_progress"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Status {
                status: ItemStatus::InProgress,
                ..
            })
        ));

        // 7. close & done (explicit and zero-ID, with -c)
        let cli = Cli::try_parse_from([
            "neb",
            "close",
            "BUG-0001",
            "-m",
            "Fixed parser",
            "--commit",
            "abc1234",
            "--merged",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Close { merged: true, .. })
        ));
        let cli = Cli::try_parse_from(["neb", "done", "-m", "Implemented feature", "-c"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Close {
                id: None,
                commit_changes: true,
                ..
            })
        ));

        // 8. reopen
        let cli = Cli::try_parse_from(["neb", "reopen", "BUG-0001", "-r", "Regression detected"])
            .unwrap();
        assert!(matches!(cli.command, Some(Commands::Reopen { .. })));

        // 9. timer
        let cli = Cli::try_parse_from(["neb", "timer", "start", "BUG-0001"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Timer {
                action: Some(TimerCommand::Start { .. })
            })
        ));

        // 10. log
        let cli = Cli::try_parse_from(["neb", "log", "BUG-0001", "-m", "30"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Log {
                minutes: Some(30),
                ..
            })
        ));

        // 11. context (explicit and zero-ID)
        let cli =
            Cli::try_parse_from(["neb", "context", "BUG-0001", "--refs-only", "--copy"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Context {
                refs_only: true,
                copy: true,
                ..
            })
        ));
        let cli = Cli::try_parse_from(["neb", "context"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Context { id: None, .. })
        ));

        // 12. metrics
        let cli = Cli::try_parse_from(["neb", "metrics"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Metrics)));

        // 13. sync
        let cli = Cli::try_parse_from(["neb", "sync", "--message", "Sync tracker"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Sync { .. })));

        // 14. sync-index
        let cli = Cli::try_parse_from(["neb", "sync-index"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::SyncIndex)));

        // 15. phase
        let cli = Cli::try_parse_from(["neb", "phase", "use", "01_core_engine"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Phase {
                action: PhaseCommand::Use { .. }
            })
        ));

        // 16. scope
        let cli = Cli::try_parse_from([
            "neb",
            "scope",
            "add",
            "nebulous_cli",
            "-t",
            "crates/nebulous_cli",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Scope {
                action: ScopeCommand::Add { .. }
            })
        ));

        // 17. project
        let cli = Cli::try_parse_from(["neb", "project", "add", "/tmp/orbit", "--name", "Orbit"])
            .unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Project {
                action: ProjectCommand::Add { .. }
            })
        ));

        // 18. ingest
        let cli = Cli::try_parse_from([
            "neb",
            "ingest",
            "--path",
            "crates",
            "--write-manifests",
            "--promote-all",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Ingest {
                write_manifests: true,
                promote_all: true,
                ..
            })
        ));

        // 19. component
        let cli = Cli::try_parse_from(["neb", "component", "pin", "orbit_iso"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Component {
                action: ComponentCommand::Pin { .. }
            })
        ));

        // 20. git
        let cli = Cli::try_parse_from([
            "neb",
            "git",
            "commit",
            "-i",
            "BUG-0001",
            "-m",
            "Fix deserialization",
            "--path",
            "src/a.rs,src/b.rs",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Git {
                action: GitCommand::Commit { ref paths, .. }
            }) if paths.len() == 2
        ));
        assert!(Cli::try_parse_from(["neb", "git", "commit", "-m", "x", "--all"]).is_err());

        // 21. mcp
        let cli = Cli::try_parse_from(["neb", "mcp"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Mcp)));

        // 22. start, next, move, stage
        let cli = Cli::try_parse_from(["neb", "start"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Start {
                id: None,
                copy: true,
                ..
            })
        ));

        let cli = Cli::try_parse_from(["neb", "next"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Next)));

        let cli = Cli::try_parse_from(["neb", "move", "prod"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Move { .. })));

        let cli = Cli::try_parse_from(["neb", "move", "FEAT-0001", "prod"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Move { .. })));

        let cli = Cli::try_parse_from(["neb", "stage", "prod"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Commands::Stage { stage: Some(_) })
        ));
    }
}
