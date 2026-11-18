# Changelog

All notable changes to Nebulous are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions: `0.0.x` = alpha, `0.1.x` = beta, `1.0.0` = first stable release.

## [Unreleased]

## [0.1.0] - 2026-11-18

First beta (0.1.0).

### Added
- Task items (`TASK-XXXX`): `neb task` and the TUI `t` key.
- `neb plan import` and the MCP `plan_import` tool import roadmap tables from a stepped plan file (dry run by default).
- Opt-in hash item IDs (`id_scheme = "hash"`).
- Schema versions on all four tracker tiers. Unknown keys are preserved, and items written by a newer version are skipped and reported.
- Timer: per-session cap, idempotent stop, and quarantine of a corrupt or orphaned timer file.
- MCP: `dry_run` (default true) on tools that write or use the network, and protocol version negotiation.
- TUI: live reload, wall-clock timer, and clipboard copy with Wayland and OSC 52 fallback.

### Changed
- Every tracker write takes a cross-process lock on `.tracker/.lock`, and vault writes take their own file lock.
- MCP `tools/call` results carry `content`, `structuredContent` and `isError`, and stdio framing recovers from bad frames.
- Dates and times use jiff instead of chrono. Stored formats are unchanged.
- When no item can be inferred, the error suggests `neb next` instead of naming a recommendation.
- CI runs on Linux, macOS and Windows.

### Removed
- The release engine and `neb release` (moved to a separate project).

### Security
- System directories can't be project roots on Linux, macOS (including `/private/...`) or Windows.
- Vault files get user-only permissions on Windows (ACL), as on Unix.
- `cargo deny` passes with zero open RustSec advisories.

Releases v0.0.1–v0.0.4 predate this file; see their GitHub Release notes.
