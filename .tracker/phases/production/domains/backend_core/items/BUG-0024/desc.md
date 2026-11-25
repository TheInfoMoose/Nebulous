## Problem Description
Source: docs/codebase-audit-report.md (AUD-21) — Validation: CONFIRMED.

Citations:
- crates/backend_core/src/storage/reconcile.rs:64, 87, 115, 153, 279-290
- crates/backend_core/src/workflow/metrics.rs:132-136, 147, 223-227, 237-238, 277-281
- crates/backend_core/src/workflow/assist.rs:387-399

Details:
`reconcile_indexes` skips corrupt/newer-schema `item.toml` files but propagates `?` on `read_phase_from_path` and `read_scope_from_path`. Similarly, `compute_metrics_at` aborts globally on a single unreadable `item.toml` or bad `updated_at` (unlike `compute_whats_next_at`), sets `daily_pace_ratio` identical to `target_days_equivalent`, and can emit `f64::INFINITY` (JSON `null`) on tiny `target_daily_hours`.

Suggested Direction:
Extend skip-and-report handling to `phase.toml`/`scope.toml` in `reconcile_indexes` and to unreadable/bad-date items in `compute_metrics_at`; guard `f64::is_finite()` on pace metrics and fix/deprecate `daily_pace_ratio`.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
