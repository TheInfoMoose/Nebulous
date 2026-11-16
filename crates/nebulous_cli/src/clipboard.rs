use std::io::{IsTerminal, Write};

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

/// Clipboard mechanism that succeeded when copying text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyMethod {
    System,
    Osc52,
}

/// Formats `text` as an OSC 52 clipboard escape sequence.
pub fn osc52_sequence(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", STANDARD.encode(text.as_bytes()))
}

/// Copies `text` via `arboard`, falling back to OSC 52 on `stderr` when `stderr` is a terminal.
pub fn copy_text(text: &str) -> Result<CopyMethod, String> {
    match arboard::Clipboard::new().and_then(|mut cb| cb.set_text(text.to_owned())) {
        Ok(()) => Ok(CopyMethod::System),
        Err(arboard_err) => {
            let mut stderr = std::io::stderr();
            if stderr.is_terminal() {
                let seq = osc52_sequence(text);
                if stderr
                    .write_all(seq.as_bytes())
                    .and_then(|()| stderr.flush())
                    .is_ok()
                {
                    return Ok(CopyMethod::Osc52);
                }
            }
            Err(arboard_err.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc52_sequence() {
        assert_eq!(osc52_sequence("hi"), "\x1b]52;c;aGk=\x07");
    }
}
