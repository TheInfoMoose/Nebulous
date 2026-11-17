## Problem Description
Imported from roadmap step `STEP 26E`: ReconcileReport.duplicate_ids (all phases/scopes, parsed manifests only); neb sync-index warns in text + JSON (Gemini; validate-step PASS)
- Step ID: `STEP 26E`
- Command: `/goal`
- Roadmap order: completed
- Target files:
  - `crates/backend_core/src/storage/reconcile.rs`
  - `crates/nebulous_cli/src/commands/mod.rs`
  - `crates/backend_core/tests/storage_integration.rs`

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
