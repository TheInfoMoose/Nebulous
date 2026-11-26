**Repo:** Nebulous, a CLI/TUI-first, local-first, Git-backed project, bug, and task tracker (`neb`). Crates: `crates/backend_core` (models, storage, workflow, mcp, git_sync, planning, crypto, watcher) and `crates/nebulous_cli` (CLI, TUI, `neb mcp`). Docs: `docs/USAGE.md`, `docs/ARCHITECTURE.md`.

**Source of truth:** the step's step block in `STEPPED_IMPLEMENTATION_PLAN.md`. The plan header's "Who runs what" section applies.

**Rules:**
- Gates (run on the host by the operator): `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, plus `scripts/check-neutral.sh`.
- **Neutrality:** tracked files must not mention distro- or OS-project-specific tokens (`scripts/check-neutral.sh` has the pattern). Nebulous is a generic tracker.
- Never edit files outside this repo, and never edit the plan file.
- `.tracker/` is this repo's own live tracker. Never edit it by hand; Neon files side findings and bugs as pending `neb` items.
- Test data uses placeholders only (`testuser`, `example.com`, `192.0.2.0/24`).
