use std::process::ExitCode;

use clap::Parser;
use nebulous_cli::{execute, Cli};
use serde_json::json;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let json_mode = cli.json;

    if let Err(err) = execute(&cli) {
        if json_mode {
            let payload = json!({
                "status": "error",
                "error": err.to_string(),
            });
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&payload)
                    .unwrap_or_else(|_| format!("{{\"error\":\"{err}\"}}"))
            );
        } else {
            eprintln!("Error: {err}");
        }
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
