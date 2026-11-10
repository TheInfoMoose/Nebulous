use std::fmt;
use std::io::{self, IsTerminal, Write};

use crate::release::{ExitCode, HARD_STOP_PERMISSION_DENIED};

pub trait Terminal {
    fn stdin_is_tty(&self) -> bool;
    fn stdout_is_tty(&self) -> bool;
    fn var(&self, key: &str) -> Option<String>;
    fn prompt(&mut self, message: &str) -> io::Result<String>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RealTerminal;

impl Terminal for RealTerminal {
    fn stdin_is_tty(&self) -> bool {
        io::stdin().is_terminal()
    }

    fn stdout_is_tty(&self) -> bool {
        io::stdout().is_terminal()
    }

    /// Uses `var_os` with lossy UTF-8 conversion so a non-UTF-8 environment
    /// value still counts as set (whereas `env::var` returns `Err(NotUnicode)`,
    /// which `.ok()` would discard as `None` and skip the agent-marker check).
    fn var(&self, key: &str) -> Option<String> {
        std::env::var_os(key).map(|v| v.to_string_lossy().into_owned())
    }

    fn prompt(&mut self, message: &str) -> io::Result<String> {
        eprint!("{message}");
        io::stderr().flush()?;
        let mut line = String::new();
        io::stdin().read_line(&mut line)?;
        Ok(line)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DenyReason {
    NotTty,
    AgentMarker(String),
    NotConfirmed,
}

impl fmt::Display for DenyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotTty => write!(f, "stdin and stdout must both be interactive TTYs"),
            Self::AgentMarker(name) => write!(f, "agent environment marker {name} is set"),
            Self::NotConfirmed => write!(f, "user confirmation word did not match"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateDenied {
    pub reason: DenyReason,
}

impl GateDenied {
    pub fn exit(&self) -> ExitCode {
        ExitCode::PermissionDenied
    }
}

impl fmt::Display for GateDenied {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{HARD_STOP_PERMISSION_DENIED}: {}", self.reason)
    }
}

impl std::error::Error for GateDenied {}

pub fn require_user(
    term: &mut dyn Terminal,
    markers: &[String],
    action: &str,
    confirm_word: &str,
) -> Result<(), GateDenied> {
    if !term.stdin_is_tty() || !term.stdout_is_tty() {
        return Err(GateDenied {
            reason: DenyReason::NotTty,
        });
    }

    for marker in markers {
        if let Some(value) = term.var(marker) {
            if !value.is_empty() {
                return Err(GateDenied {
                    reason: DenyReason::AgentMarker(marker.clone()),
                });
            }
        }
    }

    let prompt_message = format!("Type '{confirm_word}' to {action}: ");
    let response = term.prompt(&prompt_message).map_err(|_| GateDenied {
        reason: DenyReason::NotConfirmed,
    })?;

    if response.trim() != confirm_word {
        return Err(GateDenied {
            reason: DenyReason::NotConfirmed,
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::release::config::DEFAULT_AGENT_ENV_MARKERS;

    struct FakeTerminal {
        stdin_tty: bool,
        stdout_tty: bool,
        env: HashMap<String, String>,
        reply: Option<io::Result<String>>,
        prompts: Vec<String>,
    }

    impl FakeTerminal {
        fn interactive(reply: &str) -> Self {
            Self {
                stdin_tty: true,
                stdout_tty: true,
                env: HashMap::new(),
                reply: Some(Ok(reply.to_string())),
                prompts: Vec::new(),
            }
        }
    }

    impl Terminal for FakeTerminal {
        fn stdin_is_tty(&self) -> bool {
            self.stdin_tty
        }

        fn stdout_is_tty(&self) -> bool {
            self.stdout_tty
        }

        fn var(&self, key: &str) -> Option<String> {
            self.env.get(key).cloned()
        }

        fn prompt(&mut self, message: &str) -> io::Result<String> {
            self.prompts.push(message.to_string());
            self.reply.take().unwrap_or_else(|| Ok(String::new()))
        }
    }

    fn default_markers() -> Vec<String> {
        DEFAULT_AGENT_ENV_MARKERS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    }

    #[test]
    fn no_tty_denies_with_not_tty_and_never_prompts() {
        let markers = default_markers();

        let mut no_stdin = FakeTerminal::interactive("bless\n");
        no_stdin.stdin_tty = false;
        let err = require_user(&mut no_stdin, &markers, "bless golden snapshot", "bless")
            .expect_err("non-tty stdin must be denied");
        assert_eq!(err.reason, DenyReason::NotTty);
        assert_eq!(err.exit(), ExitCode::PermissionDenied);
        assert!(no_stdin.prompts.is_empty());

        let mut no_stdout = FakeTerminal::interactive("bless\n");
        no_stdout.stdout_tty = false;
        let err = require_user(&mut no_stdout, &markers, "bless golden snapshot", "bless")
            .expect_err("non-tty stdout must be denied");
        assert_eq!(err.reason, DenyReason::NotTty);
        assert!(no_stdout.prompts.is_empty());
    }

    #[test]
    fn claudecode_marker_denies_with_agent_marker_and_never_prompts() {
        let markers = default_markers();
        let mut term = FakeTerminal::interactive("bless\n");
        term.env.insert("CLAUDECODE".to_string(), "1".to_string());

        let err = require_user(&mut term, &markers, "bless golden snapshot", "bless")
            .expect_err("CLAUDECODE=1 must be denied");
        assert_eq!(
            err.reason,
            DenyReason::AgentMarker("CLAUDECODE".to_string())
        );
        assert_eq!(err.exit(), ExitCode::PermissionDenied);
        assert!(err.to_string().starts_with("HARD_STOP_PERMISSION_DENIED: "));
        assert!(err.to_string().contains("CLAUDECODE"));
        assert!(term.prompts.is_empty());
    }

    #[test]
    fn empty_marker_value_does_not_count() {
        let markers = default_markers();
        let mut term = FakeTerminal::interactive("unlock\n");
        term.env.insert("CLAUDECODE".to_string(), String::new());
        term.env.insert("CI".to_string(), String::new());

        require_user(&mut term, &markers, "unlock loop state", "unlock")
            .expect("empty marker values must be ignored");
        assert_eq!(term.prompts, vec!["Type 'unlock' to unlock loop state: "]);
    }

    #[test]
    fn wrong_word_denies_with_not_confirmed_and_right_word_succeeds() {
        let markers = default_markers();

        let mut wrong = FakeTerminal::interactive("yes\n");
        let err = require_user(&mut wrong, &markers, "bless golden snapshot", "bless")
            .expect_err("wrong confirmation word must be denied");
        assert_eq!(err.reason, DenyReason::NotConfirmed);
        assert_eq!(err.exit(), ExitCode::PermissionDenied);
        assert_eq!(
            wrong.prompts,
            vec!["Type 'bless' to bless golden snapshot: "]
        );

        let mut case_mismatch = FakeTerminal::interactive("BLESS\n");
        let err = require_user(
            &mut case_mismatch,
            &markers,
            "bless golden snapshot",
            "bless",
        )
        .expect_err("case-mismatched confirmation word must be denied");
        assert_eq!(err.reason, DenyReason::NotConfirmed);

        let mut right = FakeTerminal::interactive("  bless\r\n");
        require_user(&mut right, &markers, "bless golden snapshot", "bless")
            .expect("exact trimmed confirmation word must pass");
        assert_eq!(
            right.prompts,
            vec!["Type 'bless' to bless golden snapshot: "]
        );
    }

    #[test]
    fn custom_marker_list_is_honoured() {
        let custom_markers = vec!["CUSTOM_BOT_ENV".to_string(), "SECOND_MARKER".to_string()];

        let mut term_blocked = FakeTerminal::interactive("abort\n");
        term_blocked
            .env
            .insert("SECOND_MARKER".to_string(), "true".to_string());
        let err = require_user(&mut term_blocked, &custom_markers, "abort bisect", "abort")
            .expect_err("custom marker set in env must be denied");
        assert_eq!(
            err.reason,
            DenyReason::AgentMarker("SECOND_MARKER".to_string())
        );
        assert!(term_blocked.prompts.is_empty());

        let mut term_allowed = FakeTerminal::interactive("abort\n");
        term_allowed
            .env
            .insert("CLAUDECODE".to_string(), "1".to_string());
        require_user(&mut term_allowed, &custom_markers, "abort bisect", "abort")
            .expect("markers not in custom list must not block");
    }

    #[test]
    fn checks_stop_at_first_failure_and_handle_prompt_io_error() {
        let markers = default_markers();

        // Non-TTY + agent marker set -> stops at NotTty first.
        let mut not_tty_and_marker = FakeTerminal::interactive("wrong\n");
        not_tty_and_marker.stdin_tty = false;
        not_tty_and_marker
            .env
            .insert("CLAUDECODE".to_string(), "1".to_string());
        let err = require_user(
            &mut not_tty_and_marker,
            &markers,
            "bless golden snapshot",
            "bless",
        )
        .expect_err("must fail with NotTty before checking markers");
        assert_eq!(err.reason, DenyReason::NotTty);
        assert!(err.to_string().starts_with("HARD_STOP_PERMISSION_DENIED: "));

        // Prompt I/O error -> NotConfirmed.
        let mut io_err_term = FakeTerminal {
            stdin_tty: true,
            stdout_tty: true,
            env: HashMap::new(),
            reply: Some(Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "broken pipe",
            ))),
            prompts: Vec::new(),
        };
        let err = require_user(&mut io_err_term, &markers, "import state", "import")
            .expect_err("prompt I/O error must be denied");
        assert_eq!(err.reason, DenyReason::NotConfirmed);
        assert!(err.to_string().starts_with("HARD_STOP_PERMISSION_DENIED: "));
    }
}
