## Problem Description
Source: docs/codebase-audit-report.md (AUD-32) — Validation: CONFIRMED.

Citations:
- crates/backend_core/src/models/registry.rs:179-182, 290-294, 581-593
- crates/backend_core/src/storage/mod.rs:473, 1525-1536
- crates/backend_core/src/git_sync/mod.rs:617-619, 1072-1079
- crates/backend_core/src/ingestion/mod.rs:196, 338-340, 732, 922
- .gitignore:8

Details:
1. `registry.rs:293` matches existing entries by raw equality instead of `normalize_query_path_str` and duplicates UNC/trailing-slash normalization with `storage/mod.rs:1525-1536`.
2. `is_temp_atomic_path` (`git_sync/mod.rs:618`) filters any path containing `.tmp.`; `fetch_repo_with_auth` (`1072-1079`) retries with fresh `attempts = 0` on any error.
3. `ingest_project_directory` checks `path.is_file()` (following symlinks), discards `nebulous.component.toml` parse errors (`338-340`), and `promote_component_to_scope` (`732`) calls raw `fs::create_dir_all` instead of `ensure_items_dir`.
4. `.gitignore:8` still lists legacy `/.tracker/.keyring_fallback`.

Suggested Direction:
Share path normalization, match atomic temp suffixes narrowly (`.tmp.<digits>.<digits>`), skip auth retries on invalid-credentials errors, use `entry.file_type()`/`ensure_items_dir`/surfaced parse errors in ingestion, and clean up `.gitignore`.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
