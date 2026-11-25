## Problem Description
Source: docs/codebase-audit-report.md (AUD-22) — Validation: CONFIRMED.

Citations:
- crates/backend_core/src/storage/compat.rs:76-94, 148-165
- crates/backend_core/src/storage/mod.rs:682-717, 751, 797, 832, 861

Details:
1. `compat::unknown_keys` misclassifies explicit default fields suppressed by `#[serde(default, skip_serializing_if = "...")]` as unknown and re-injects them forever at `1 + K` full TOML roundtrips.
2. `read_versioned` allows schema v2 across all 4 tiers while `write_project`/`write_phase`/`write_scope` hardcode `version = 1` (and `write_item_manifest` passes `1` for Bug/Feature), silently downgrading v2 manifests on write.
3. `compat::migrate` is an uncalled no-op stub.

Suggested Direction:
Track known field names or `#[serde(flatten)]` extra maps instead of `skip_serializing_if`-sensitive probing, define per-tier `MAX_*_SCHEMA_VERSION` constants, and remove or mark `compat::migrate` until migrations exist.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
