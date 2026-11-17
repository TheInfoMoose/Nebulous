## Problem Description
Imported from roadmap step `STEP 25R`: Import from .nebulous: plan_import (validate sources, golden v1 → v2 placeholders, config.toml from [guardrails], verbatim manifest/approvals/items, Conflict if any destination exists) + commit_import (new files only, remove on failure); config.rs gains pub validate_config + render_config (Gemini; validate-step PASS)
- Step ID: `STEP 25R`
- Command: `/boost`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/release/import.rs`
  - `crates/backend_core/src/release/mod.rs`
  - `crates/backend_core/src/release/config.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
