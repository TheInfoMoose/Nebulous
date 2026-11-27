## Problem Description
Imported from roadmap step `STEP 46E`: Sanitize all single-line untrusted fields and Unicode bidi format chars in agent_context::synthesize_bounded_prompt (BUG-0018 / AUD-12)
- Step ID: `STEP 46E`
- Command: `/goal`
- Roadmap order: 5
- Target files:
  - `crates/backend_core/src/agent_context/mod.rs`
  - `crates/backend_core/tests/workflow_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
