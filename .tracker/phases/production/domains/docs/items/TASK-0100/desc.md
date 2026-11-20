## Problem Description
`docs/USAGE.md` "Antigravity CLI (`agy mcp add`)" (STEP 42C) gives only the `agy mcp add` commands. In headless mode (`agy -p`), agy 1.5.12 auto-denies every MCP call ("a tool required the \"mcp\" permission that headless mode cannot prompt for") unless `permissions.allow` has a `mcp(<server>/<tool>)` rule; `mcp(nebulous)` alone does not match. Suggested fix: add one sentence and an example, e.g. `agy -p ... --settings '{"permissions":{"allow":["mcp(nebulous/list_items)"]}}'` (session-only) or the same key in agy's settings.json. Found in the Phase 24 live check (2026-11-20).

## Reproduction Steps
1. `agy mcp add nebulous neb mcp`.
2. `agy -p 'call nebulous list_items'` in a repo: no output, permission auto-denied.
3. Add `--settings '{"permissions":{"allow":["mcp(nebulous/list_items)"]}}'`: the call succeeds.

## Fix Log & Agent Notes
- Target scope path: `docs`
