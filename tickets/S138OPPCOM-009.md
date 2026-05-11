# S138OPPCOM-009: Observer Section 3a Opportunities rendering

**Status**: PENDING
**Priority**: LOW
**Effort**: Small
**Engine Changes**: None (tooling-only; observer binary in `worldwake-cli`)
**Deps**: archive/tickets/S138OPPCOM-001.md (CandidateSource, OpportunityCompilerLoad types), archive/tickets/S138OPPCOM-006.md (Opportunity records emitted on the decision-trace sink)

## Problem

S138's debuggability surface (FND-29) requires that compiled opportunities be inspectable from the observer binary. The existing observer Section 3 at `crates/worldwake-cli/src/bin/observer.rs:684` is a markdown table of decision events; S137 also extends this section with `EventTag::RepairApplied` rendering. To preserve the existing format and coexist with S137, this ticket refactors Section 3 into two sibling sub-sections — **3a Opportunities** (new, this ticket) and **3b Decision History** (rename of existing rendering) — rather than modifying the existing table structure.

## Assumption Reassessment (2026-05-11)

1. Existing focused/unit coverage: `crates/worldwake-cli/src/bin/observer.rs:684` contains the Section 3 (Decision History) rendering using a markdown table `| Tick | Agent | Event | Payload Summary |`. Sections in the binary are tagged textually (e.g., "## Section 3 — Decision History"); confirm the literal heading style during implementation.
2. Spec/doc reference: `specs/S138-opportunity-compiler.md` deliverable section "Observer Section 3" — the spec example shows a non-tabular format for opportunities.
3. Shared abstraction boundary: the observer reads from `DecisionTraceSink` (defined in `worldwake-ai`); archive/tickets/S138OPPCOM-006.md records per-tick opportunity data on the sink. This ticket only consumes the sink — no engine state mutation.
4. Tooling-only classification: per `references/worldwake-validation-patterns.md` "Read-Only Tooling Consumer", the observer is a binary that consumes public APIs from `worldwake-ai` (decision trace) without writing to any system.
5. Coordination with S137: S137 also lands rendering for `EventTag::RepairApplied` in this section. Both specs extend Section 3; this ticket creates the sub-section split (3a / 3b) and S137's tickets land their rendering inside 3b (or a sibling 3c). The split is non-blocking for S137.

## Architecture Check

1. Sub-section split preserves the existing markdown table verbatim — S137's `RepairApplied` rendering inside Section 3b stays where it was, no rework forced on S137.
2. New 3a uses the spec's plain-text format ("Tick 412 — Agent A: bread@bakery: salience 720 — effects: ...") which is structurally different from the table; co-locating them under a single Section 3 numbered slot is cleaner than introducing a new top-level Section 12 or similar.
3. Read-only consumer pattern (FND-29 debuggability): the observer never mutates engine state; opportunity rendering reads `DecisionTraceSink` records.
4. The renderer iterates `DecisionTraceSink.traces` by `(tick, agent)` order (BTreeMap-stable) and groups opportunities per-agent per-tick, rendering top-K by salience where K defaults to a reasonable value (8) absent an explicit observer-args flag.

## Verification Layers

1. Empty trace input produces an empty Section 3a (no header noise) — focused unit test on the renderer
2. Trace with 3 opportunities at tick 412 for agent A renders the expected plain-text format — focused unit test (golden string match)
3. Existing Section 3b output (decision-history table) is byte-identical to pre-S138 — regression test on the section renderer
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
- `render_opportunity_compiler_section(traces: &DecisionTraceSink) -> String` — produces the Section 3a body
- `format_opportunity_line(opp: &Opportunity) -> String` — renders a single opportunity in the spec's plain-text format

These helpers stay local to `observer.rs` (tooling-only; no library export needed).

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — Section 3 header refactor + new Section 3a renderer + helper functions)

## Out of Scope

- Engine-state changes — observer is read-only
- S137's `EventTag::RepairApplied` rendering — lands in S137's tickets within Section 3b
- New observer-args flags (e.g., `--top-k-opportunities`) — could be added in a future tooling pass; not required for S138

## Acceptance Criteria

### Tests That Must Pass

1. New test: `render_opportunity_compiler_section` over a trace with 0 opportunities returns an empty string (no section header noise)
2. New test: `render_opportunity_compiler_section` over a trace with 3 opportunities for agent A at tick 412 produces the spec's example format
3. New test: opportunities are rendered in `salience`-descending order (top-K = 8)
4. New test: Section 3b output is unchanged from pre-S138 for the same trace input
5. Existing observer binary builds: `cargo build -p worldwake-cli --bin observer`
6. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Observer never mutates engine state — `DecisionTraceSink` is read-only at the section-renderer surface
2. Section 3a precedes Section 3b in the rendered output
3. Sub-sections share Section 3's numbered slot; no new top-level section number is introduced

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (inline `#[cfg(test)]`, or sibling `observer_section_3a_tests.rs`) — 4 new tests per Acceptance Criteria

### Commands

1. `cargo test -p worldwake-cli observer`
2. `cargo build -p worldwake-cli --bin observer`
3. `cargo clippy --workspace --all-targets -- -D warnings`
