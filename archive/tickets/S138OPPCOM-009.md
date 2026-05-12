# S138OPPCOM-009: Observer Section 3a Opportunities rendering

**Status**: COMPLETED
**Priority**: LOW
**Effort**: Small
**Engine Changes**: Trace/tooling only — carries derived `Opportunity` records on `AgentDecisionTrace` and renders them from the observer binary; no authoritative world-state mutation
**Deps**: archive/tickets/S138OPPCOM-001.md (Opportunity and OpportunityCompilerLoad types), archive/tickets/S138OPPCOM-006.md (compile pass produces per-tick opportunities and load counters)

## Problem

S138's debuggability surface (FND-29) requires that compiled opportunities be inspectable from the observer binary. The existing observer Section 3 at `crates/worldwake-cli/src/bin/observer.rs:684` is a markdown table of decision events; S137 also extends this section with `EventTag::RepairApplied` rendering. To preserve the existing format and coexist with S137, this ticket refactors Section 3 into two sibling sub-sections — **3a Opportunities** (new, this ticket) and **3b Decision History** (rename of existing rendering) — rather than modifying the existing table structure.

## Assumption Reassessment (2026-05-11)

1. Existing focused/unit coverage: `crates/worldwake-cli/src/bin/observer.rs` contained the old Section 3 (Decision History) rendering using a markdown table `| Tick | Agent | Event | Payload Summary |`; the implemented heading is now `## Section 3b — Decision History`.
2. Spec/doc reference: `archive/specs/S138-opportunity-compiler.md` deliverable section "Observer Section 3" — the spec example shows a non-tabular format for opportunities.
3. Shared abstraction boundary: the observer reads from `DecisionTraceSink` (defined in `worldwake-ai`). Live reassessment showed `archive/tickets/S138OPPCOM-006.md` recorded `OpportunityCompilerLoad` but did not retain the actual `Opportunity` records for report consumers, so this ticket owns the narrow derived trace-carrier addition: `AgentDecisionTrace.compiled_opportunities: Vec<Opportunity>`.
4. Tooling/trace classification: the observer is a binary that consumes public APIs from `worldwake-ai` (decision trace) without writing to any system. The new trace field carries derived per-tick read-model data only; it is not authoritative state and does not affect save/load format.
5. Coordination with S137: S137 also lands rendering for `EventTag::RepairApplied` in this section. Both specs extend Section 3; this ticket creates the sub-section split (3a / 3b) and S137's tickets land their rendering inside 3b (or a sibling 3c). The split is non-blocking for S137.

## Architecture Check

1. Sub-section split preserves the existing markdown table verbatim — S137's `RepairApplied` rendering inside Section 3b stays where it was, no rework forced on S137.
2. New 3a uses the spec's plain-text format ("Tick 412 — Agent A: bread@bakery: salience 720 — effects: ...") which is structurally different from the table; co-locating them under a single Section 3 numbered slot is cleaner than introducing a new top-level Section 12 or similar.
3. Read-only consumer pattern (FND-29 debuggability): the observer never mutates engine state; opportunity rendering reads `DecisionTraceSink` records.
4. The renderer iterates `DecisionTraceSink.traces` by `(tick, agent)` order (BTreeMap-stable) and groups opportunities per-agent per-tick, rendering top-K by salience where K defaults to a reasonable value (8) absent an explicit observer-args flag.
5. The retained `compiled_opportunities` trace vector is a diagnostic copy of the read-phase `Opportunity` list, not a second canonical opportunity store; the read phase remains the owner of candidate-generation input.

## Verification Layers

1. Empty trace input produces an empty Section 3a (no header noise) — focused unit test on the renderer
2. Trace with 3 opportunities at tick 412 for agent A renders the expected plain-text format — focused unit test (golden string match)
3. Existing Section 3b decision-history table rows are unchanged from pre-S138, with only the section heading renamed — regression test on the section renderer and observer fixture
4. Section 3a precedes Section 3b in the rendered output — focused unit test on section ordering

## What to Change

### 1. Refactor existing Section 3 → Section 3b

Modify `crates/worldwake-cli/src/bin/observer.rs:684`:

Rename the existing section header from "Section 3 — Decision History" (or current text) to "Section 3b — Decision History". The body of the section is unchanged.

### 2. Add Section 3a Opportunities renderer

Insert immediately before Section 3b a new sub-section "Section 3a — Opportunities". Renderer iterates per-agent per-tick:

```text
Section 3a — Opportunities

Tick 412 — Agent A:
  bread@bakery: salience 720 — effects: CommodityTransfer; commodity: Bread; legal: BelievedOwned(baker); exposure: Public
  bread@bakery: salience 540 — effects: CommodityTransfer; commodity: Bread; legal: BelievedOwned(baker); exposure: PublicWithCriminalRisk
  altar@hut: salience 380 — effects: CommodityTransfer; commodity: Bread; legal: SociallyOpenToRequest; exposure: PublicWithShameRisk
```

Render up to K=8 opportunities per agent per tick (top by `Opportunity.salience`). Skip ticks with no opportunities recorded.

### 3. Helper functions in the observer binary

Add local helpers in the same file:
- `render_opportunity_compiler_section(traces: &DecisionTraceSink, agents: &[(EntityId, String)], world: &World) -> String` — produces the Section 3a body with stable agent/world labels
- `format_opportunity_line(opp: &Opportunity, world: &World) -> String` — renders a single opportunity in the spec's plain-text format

These helpers stay local to `observer.rs` (tooling-only; no library export needed).

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — Section 3 header refactor + new Section 3a renderer + helper functions)
- `crates/worldwake-ai/src/decision_trace.rs` and `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — carry `compiled_opportunities` on `AgentDecisionTrace`)
- Trace/test constructor fallout in `crates/worldwake-ai/src/survival_forensics.rs`, `crates/worldwake-ai/tests/golden_harness/*.rs`, and `crates/worldwake-visualizer/src/trace_buffers.rs`
- `crates/worldwake-cli/tests/observer_decision_history.rs` and its fixture (modify — Section 3b header regression)

## Out of Scope

- Authoritative engine-state changes — observer is read-only and the new trace field is derived diagnostic data only
- S137's `EventTag::RepairApplied` rendering — lands in S137's tickets within Section 3b
- New observer-args flags (e.g., `--top-k-opportunities`) — could be added in a future tooling pass; not required for S138

## Acceptance Criteria

### Tests That Must Pass

1. New test: `render_opportunity_compiler_section` over a trace with 0 opportunities returns an empty string (no section header noise)
2. New test: `render_opportunity_compiler_section` over a trace with 3 opportunities for agent A at tick 412 produces the spec's example format
3. New test: opportunities are rendered in `salience`-descending order (top-K = 8)
4. New/updated tests: Section 3b decision-history rows are unchanged except for the intentional `Section 3b` header rename
5. Existing observer binary builds: `cargo build -p worldwake-cli --bin observer`
6. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Observer never mutates engine state — `DecisionTraceSink` is read-only at the section-renderer surface
2. Section 3a precedes Section 3b in the rendered output
3. Sub-sections share Section 3's numbered slot; no new top-level section number is introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (inline `#[cfg(test)]`) — focused Section 3a renderer, line-format, ordering, and Section 3b regression tests
2. `crates/worldwake-cli/tests/observer_decision_history.rs` fixture — updated to anchor the renamed `Section 3b` decision-history output

### Commands

1. `cargo test -p worldwake-cli --bin observer render_opportunity_compiler_section`
2. `cargo test -p worldwake-cli --bin observer tests::format_opportunity_line_renders_plain_text_summary -- --exact`
3. `cargo test -p worldwake-cli --bin observer tests::render_decision_history_section_covers_all_variants -- --exact`
4. `cargo test -p worldwake-cli`
5. `cargo test -p worldwake-ai`
6. `cargo test -p worldwake-visualizer`
7. `cargo build -p worldwake-cli --bin observer`
8. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-05-11.

- Added derived `compiled_opportunities` carriage to `AgentDecisionTrace`, populated from the S138 read phase, so observer/report consumers can inspect actual per-agent per-tick `Opportunity` records rather than only `OpportunityCompilerLoad` counters.
- Added observer Section 3a rendering from the decision trace sink, grouped by `(tick, agent)`, sorted by descending salience, capped at top 8 opportunities per agent-tick, and omitted entirely when no opportunities exist.
- Renamed existing decision-history output to Section 3b without changing the table rows, and updated the observer decision-history fixture to the new heading.
- Updated all-target trace constructor fallout in AI golden harness helpers and the visualizer trace-buffer tests.
- Truth-synced `archive/specs/S138-opportunity-compiler.md` so the spec records `AgentDecisionTrace.compiled_opportunities` as derived diagnostic trace data.

## Deviations

- The live branch did not already retain `Opportunity` records on `DecisionTraceSink`; S138OPPCOM-006 retained only `OpportunityCompilerLoad`. This ticket absorbed the narrow trace-carrier addition because Section 3a could not render the requested opportunity details from aggregate counters alone.
- The helper signatures include `agents` and `world` arguments so the observer can render stable names for agents, places, owners, and risk sources.

## Verification Result

- Passed `cargo test -p worldwake-cli --bin observer render_opportunity_compiler_section`
- Passed `cargo test -p worldwake-cli --bin observer tests::format_opportunity_line_renders_plain_text_summary -- --exact`
- Passed `cargo test -p worldwake-cli --bin observer tests::render_decision_history_section_covers_all_variants -- --exact`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo test -p worldwake-visualizer`
- Passed `cargo build -p worldwake-cli --bin observer`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `git diff --check`
- Passed `python3 .codex/skills/implement-ticket/scripts/check_closeout.py tickets/S138OPPCOM-009.md`
