## Problem Description
`crates/backend_core/src/mcp/mod.rs` `handle_request` has no `server/discover` arm, so the stateless `2026-07-28` probe that Claude Code 2.1.318 sends first gets `-32601 Method not found`; Claude Code then falls back to `initialize` (2025-11-25), so nothing breaks today. Suggested fix: implement `server/discover` per the 2026-07-28 spec once its MUSTs are reviewed (or keep -32601 deliberately and document it). Found planning Phase 24 (2026-11-20, logging-wrapper probe).

## Reproduction Steps
1. Start `neb mcp` under Claude Code 2.1.318 with a logging wrapper.
2. Observe: the first request is `server/discover` with `_meta.io.modelcontextprotocol/protocolVersion` `2026-07-28`; neb answers `-32601`.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
