pub mod args;
pub mod commands;

pub use args::{
    Cli, Commands, ComponentCommand, GitCommand, PhaseCommand, ProjectCommand, ScopeCommand,
    TimerCommand,
};
pub use commands::{execute, execute_to_string, resolve_storage};

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
        // 1. init
        let cli = Cli::try_parse_from(["neb", "init", "--name", "Demo"]).unwrap();
        assert!(matches!(cli.command, Commands::Init { .. }));

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
            Commands::Bug {
                severity: Severity::High,
                ..
            }
        ));

        // 3. feat
        let cli = Cli::try_parse_from(["neb", "feat", "New Kanban filter"]).unwrap();
        assert!(matches!(cli.command, Commands::Feat { .. }));

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
            Commands::List {
                status: Some(ItemStatus::Open),
                item_type: Some(ItemType::Bug),
                stagnant: true,
                recurring: true,
                ..
            }
        ));

        // 5. show
        let cli = Cli::try_parse_from(["neb", "show", "BUG-0001"]).unwrap();
        assert!(matches!(cli.command, Commands::Show { .. }));

        // 6. status
        let cli = Cli::try_parse_from(["neb", "status", "BUG-0001", "in_progress"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Status {
                status: ItemStatus::InProgress,
                ..
            }
        ));

        // 7. close
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
        assert!(matches!(cli.command, Commands::Close { merged: true, .. }));

        // 8. reopen
        let cli = Cli::try_parse_from(["neb", "reopen", "BUG-0001", "-r", "Regression detected"])
            .unwrap();
        assert!(matches!(cli.command, Commands::Reopen { .. }));

        // 9. timer
        let cli = Cli::try_parse_from(["neb", "timer", "start", "BUG-0001"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Timer {
                action: Some(TimerCommand::Start { .. })
            }
        ));

        // 10. log
        let cli = Cli::try_parse_from(["neb", "log", "BUG-0001", "-m", "30"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Log {
                minutes: Some(30),
                ..
            }
        ));

        // 11. context
        let cli =
            Cli::try_parse_from(["neb", "context", "BUG-0001", "--refs-only", "--copy"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Context {
                refs_only: true,
                copy: true,
                ..
            }
        ));

        // 12. metrics
        let cli = Cli::try_parse_from(["neb", "metrics"]).unwrap();
        assert!(matches!(cli.command, Commands::Metrics));

        // 13. sync
        let cli = Cli::try_parse_from(["neb", "sync", "--message", "Sync tracker"]).unwrap();
        assert!(matches!(cli.command, Commands::Sync { .. }));

        // 14. sync-index
        let cli = Cli::try_parse_from(["neb", "sync-index"]).unwrap();
        assert!(matches!(cli.command, Commands::SyncIndex));

        // 15. phase
        let cli = Cli::try_parse_from(["neb", "phase", "use", "01_core_engine"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Phase {
                action: PhaseCommand::Use { .. }
            }
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
            Commands::Scope {
                action: ScopeCommand::Add { .. }
            }
        ));

        // 17. project
        let cli = Cli::try_parse_from(["neb", "project", "add", "/tmp/novos", "--name", "NovOS"])
            .unwrap();
        assert!(matches!(
            cli.command,
            Commands::Project {
                action: ProjectCommand::Add { .. }
            }
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
            Commands::Ingest {
                write_manifests: true,
                promote_all: true,
                ..
            }
        ));

        // 19. component
        let cli = Cli::try_parse_from(["neb", "component", "pin", "novos_iso"]).unwrap();
        assert!(matches!(
            cli.command,
            Commands::Component {
                action: ComponentCommand::Pin { .. }
            }
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
            "--all",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Commands::Git {
                action: GitCommand::Commit { all: true, .. }
            }
        ));

        // 21. mcp
        let cli = Cli::try_parse_from(["neb", "mcp"]).unwrap();
        assert!(matches!(cli.command, Commands::Mcp));
    }
}
