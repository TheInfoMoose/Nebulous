use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use cosmic::app::Settings;
use cosmic::iced::Size;
use cosmic_interface::{run_headless_self_test, AppFlags, NebulousApp};

/// Native `libcosmic` desktop GUI (`nebulous`) for the Nebulous 4-tier project and bug tracker.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "nebulous",
    version,
    about = "Native libcosmic desktop GUI (`nebulous`) for the Nebulous project and bug tracker"
)]
pub struct GuiArgs {
    /// Path to the workspace root or `.tracker/` directory (defaults to current working directory).
    #[arg(short, long, visible_alias = "root", value_name = "PATH")]
    pub workspace: Option<PathBuf>,

    /// Run headless self-test (initializes app state against `.tracker/`, exercises card selection,
    /// filtering, and view rendering in memory, and exits `0` without requiring a display server).
    #[arg(long)]
    pub self_test: bool,
}

fn main() -> ExitCode {
    let args = GuiArgs::parse();

    if args.self_test {
        return match run_headless_self_test(args.workspace.as_deref()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("nebulous --self-test failed: {err}");
                ExitCode::FAILURE
            }
        };
    }

    let settings = Settings::default()
        .size(Size::new(1360.0, 860.0))
        .client_decorations(true);

    let flags = AppFlags {
        workspace_path: args.workspace,
    };

    if let Err(err) = cosmic::app::run::<NebulousApp>(settings, flags) {
        eprintln!("nebulous GUI exited with error: {err}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
