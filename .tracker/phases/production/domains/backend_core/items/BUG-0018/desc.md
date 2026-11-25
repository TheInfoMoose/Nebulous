## Problem Description
Source: docs/codebase-audit-report.md (AUD-12) — Validation: PARTIALLY CONFIRMED (core defect confirmed; note desc.md/record_verification_run is already fenced).

Citations:
- crates/backend_core/src/agent_context/mod.rs:16-20, 148, 153-170, 181-183, 209-219

Details:
`item.title` (`157`) only passes through `strip_control`, which preserves `
` and `	` (`18`) and passes Unicode bidi format chars (`Cf`, since `char::is_control()` only covers `Cc`). `scope.scope.id` (`167`), `scope.scope.target` (`168, 182`), `tech_stack` (`148`), `fix.date` (`216`), and `fix.commit` (`217`) are interpolated raw into Markdown headings, prose, blockquotes, and inline code spans. (Note: `detail.description_markdown`—where `record_verification_run` appends—is already wrapped in adaptive `fenced(...)` at line `232`.)

Suggested Direction:
Add a `single_line_safe()` sanitizer collapsing ``/`
`/`	` to spaces, stripping `Cc` and bidi `Cf` characters, and escaping/stripping backticks for all single-line interpolated fields.

## Reproduction Steps
1. Inspect `crates/backend_core`.
2. Run tests to verify behavior.

## Fix Log & Agent Notes
- Target scope path: `crates/backend_core`
