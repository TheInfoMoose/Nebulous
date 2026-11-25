## Problem Description
Source: docs/codebase-audit-report.md (AUD-19) — Validation: CONFIRMED.

Citations:
- crates/nebulous_cli/src/clipboard.rs:20
- crates/nebulous_cli/src/args.rs:167-175
- crates/nebulous_cli/src/tui/state.rs:1204, 1235

Details:
`copy_text` invokes `arboard::Clipboard::new().and_then(|mut cb| cb.set_text(...))` synchronously on the main thread with no timeout before falling back to OSC 52. Because `neb start` defaults `--copy=true`, a stalled X11 socket (e.g., stale SSH X11 forwarding) freezes `neb start`, `neb context --copy`, and TUI `s`/`c` indefinitely.

Suggested Direction:
Wrap `arboard` calls in a bounded worker-thread timeout or fall back to OSC 52 when the clipboard backend does not respond within a short deadline.

## Reproduction Steps
1. Inspect `crates/nebulous_cli`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/nebulous_cli`
