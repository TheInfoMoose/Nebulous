## Step Execution Roadmap

**Workflow:** phased

**Base gate:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`

**Forbidden paths:** `none`

**Planning files:** `none`

**How to read this:** work goes strictly top to bottom through **Up Next**, one step at a time. The **#** column is the only order. Step IDs are kept because commits and notes cite them. A step whose "Waits on" names an Open Question or a **[USER]** action doesn't start until that's resolved.

**Who runs what (phased workflow):**
- **Claude Opus, fresh session each:** `/project-plan` (direction), `/phase-plan` (contract steps, no pre-written code) and `/phase-review` (code first, contracts second).
- **Orrery TUI:** Gemini Hydrogen orchestrates, Gemini Neon implements, Gemini Hydrogen validates. Hooks gate, commit and defer. Nothing failing is committed.
- **Claude Sonnet, fresh session:** `/phase-issue` fixes and restages a stopped or deferred step, with the user's approval.
- **Neon** never commits, never edits this file or the forbidden paths, and files bugs and scope lapses as **pending** neb items.
- **Repeated failures** or a scope lapse defer the step. A step whose PRE_GATE then fails is deferred without running. Too many deferrals (`max(2, ceil(steps/3))`) stop the phase.

**Phase goal (Phase <N>):** <1–3 lines. Frozen until the phase review. Every Up Next step serves it. Keep the `(Phase <N>)` tag so `phased-step.sh new-phase` writes `phase = "<N>"` into `.orrery/phase.toml`.>

**Side findings, bugs and scope lapses** become **pending neb items**, reviewed once at phase end. Every Up Next row has a neb task (`source_ref` = step ID); commits start with `[<TASK-ID>](<scope>)`.

### Up Next

| # | Step | Command | Scope | Target Files | Waits on |
| :---: | :--- | :---: | :--- | :--- | :--- |

### Completed

| Step ID | Status | Command | Scope | Target Files |
| :--- | :---: | :---: | :--- | :--- |

### Future phases

Outlines only (goal + step titles). `/phase-plan` writes the step blocks just before each phase starts.

- **Phase <N+1>:** <goal>
  - <step title>

### Open Questions

_None yet._

### Parked

Tracked in neb: `neb list --status pending`. Reviewed at phase end; only picked items become steps.
