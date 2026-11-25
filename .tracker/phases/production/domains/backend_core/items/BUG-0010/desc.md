## Problem Description
Source: docs/codebase-audit-report.md (AUD-03) — Validation: CONFIRMED.

Citations:
- crates/backend_core/src/storage/mod.rs:239-244, 419-439, 517-523, 1281-1384
- crates/backend_core/tests/path_containment.rs:141-277

Details:
1. `TrackerStorage::discover` probes `candidate.join("project.toml").is_file()` (following symlinks) and accepts a symlinked `.tracker` directory. `resolve_contained` canonicalizes both `base` and `joined`, so a symlinked `.tracker` root passes `starts_with(&canon_base)`. When `rel` is empty (`has_normal == false`), `reject_final_symlink` is skipped.
2. `resolve_contained` returns the lexical `joined` path and callers open files/dirs without `O_NOFOLLOW` or directory-fd pinning (TOCTOU window).
3. `tests/path_containment.rs` does not test a symlinked `.tracker` root.

Suggested Direction:
Reject symlinked `.tracker` roots via `fs::symlink_metadata(&self.tracker_dir)` in `discover` and `guard_tracker_path`, use `O_NOFOLLOW` / dir-fd handles on Unix for sensitive tracker I/O, and add regression tests.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
