## Problem Description
Source: docs/codebase-audit-report.md (AUD-23) — Validation: CONFIRMED.

Citations:
- crates/backend_core/src/planning/mod.rs:52-77, 256-315
- crates/backend_core/src/planning/import.rs:98-122

Details:
1. `parse_roadmap` parses any `|`-prefixed line inside `### Up Next`/`### Completed` without tracking fenced code blocks.
2. `split_table_row` mishandles escaped backslashes before pipes (`\|` at row end or mid-cell).
3. `parse_target_files` splits strictly on `", "`.
4. `import_plan` builds `by_source_ref` globally across all phases, so reusing `STEP 1` in a second phase hijacks/retitles Phase 1's item or fails with `Duplicate source_ref`.

Suggested Direction:
Track code-fence state in `parse_roadmap`, fix escaped-backslash handling in `split_table_row`, relax `parse_target_files` comma splitting, and scope `by_source_ref` in `import_plan` to the target phase.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
