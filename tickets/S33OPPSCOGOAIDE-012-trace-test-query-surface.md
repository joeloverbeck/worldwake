# S33OPPSCOGOAIDE-012: Add stable decision-trace query/build helpers for tests

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: None
**Deps**: S33OPPSCOGOAIDE-009, archive/tickets/S33OPPSCOGOAIDE-011-opportunity-scoped-stage1-trace.md, specs/S33-opportunity-scoped-goal-identity.md

## Problem

The decision-trace architecture is now correctly opportunity-scoped, but many focused and golden tests still inspect raw trace structs field-by-field. That couples tests to internal trace storage choices such as `goal` vs `opportunity`, and it turns architectural cleanups into broad mechanical test edits.

This is a debugging-contract problem, not a planner-behavior problem. Worldwake wants explainable emergence and durable debugging surfaces. Tests that must know every internal trace struct field are not a durable debugging surface.

## Assumption Reassessment (2026-03-28)

1. The shared abstraction boundary under audit is the public decision-trace read surface used by tests: `DecisionTraceSink`, `AgentDecisionTrace`, `PlanningPipelineTrace`, `CandidateTrace`, `CandidateEvidenceTrace`, `RankedGoalSummary`, and `SelectionTrace` in `crates/worldwake-ai/src/decision_trace.rs`, plus the golden and focused tests that currently reach into those structs directly.
2. Reassessment after `archive/tickets/S33OPPSCOGOAIDE-011-opportunity-scoped-stage1-trace.md` shows the current architecture is correct but the proof surface is brittle: many tests in `crates/worldwake-ai/tests/` and `crates/worldwake-ai/src/agent_tick/tests.rs` had to be updated mechanically from `goal`/`kind` reads to `opportunity.goal_key` / `goal_key.kind`.
3. This is a coverage-surface issue, not a missing behavior issue. Existing focused and golden coverage already proves the relevant AI/runtime behavior; the gap is that the proof surface is overly coupled to raw trace representation.
4. The intended verification layer is still focused decision-trace coverage first, with golden coverage only for representative call sites that demonstrate the new helper/query surface is sufficient for real scenario assertions.
5. The live desire-level query helpers already exist and are architecturally appropriate examples of the right direction: `DecisionTraceSink::goal_status_at()` and `DecisionTraceSink::goal_history_for()` derive stable answers from trace internals rather than forcing callers to traverse raw storage themselves.
6. The same fact currently has two practical consumption paths in tests:
   - raw field traversal over trace structs
   - higher-level helper queries such as `goal_status_at()`
   After this ticket, the canonical path for tests should be explicit helper/query/build APIs. Raw struct traversal should remain available only where the helper layer does not yet expose the necessary fact.
7. This ticket must not reintroduce goal-only aliasing or flatten opportunity identity back into desire identity. Any helper added here must preserve `OpportunityKey` as the canonical concrete-candidate identity and derive desire-level answers intentionally.
8. Adjacent contradiction exposed by reassessment: if helper APIs are added as lossy wrappers that expose only `GoalKey`, they would reopen the exact aliasing problem S33 just removed. That would be a required consequence to avoid in-scope, not a follow-up.
9. This is not a stale-request, contested-affordance, or authoritative-start ticket. Additional runtime-layer mapping beyond trace query ergonomics is not applicable.
10. Mismatch + correction: the recommended architecture note from the completed ticket was not “add compatibility wrappers for old fields.” The clean follow-up is a stable helper/query surface that keeps canonical identities and hides storage details, consistent with `docs/FOUNDATIONS.md` Principle 26.

## Architecture Check

1. A stable query/build surface for tests is cleaner than asking tests to know every field layout inside `CandidateTrace`, `RankedGoalSummary`, or `SelectionTrace`. That aligns with `docs/FOUNDATIONS.md` Principle 27: debuggability is a product feature, not an accident of internal struct shape.
2. The clean approach is additive helper/query/build APIs over the canonical trace model, not compatibility aliases on the live structs. Tests should ask explicit questions like “was this opportunity generated?”, “which ranked opportunity was selected?”, or “give me the evidence trace for this opportunity/goal family” rather than reaching into storage fields.
3. This also aligns with Principle 25: helpers are derived read surfaces over canonical trace state, never parallel truth. The canonical stored trace remains opportunity-scoped.
4. No backwards-compatibility field aliases or duplicate goal-only mirrors should be introduced. Broken tests should migrate to the helper surface rather than keeping deprecated raw-field assumptions alive.

## Verification Layers

1. Stable opportunity-scoped trace lookup for generated/ranked/evidence/selected facts -> focused decision-trace helper tests.
2. Desire-level helper APIs still derive correct answers from canonical opportunity-scoped state -> focused decision-trace helper tests.
3. Golden tests can assert representative scenario facts through helper/query APIs without direct raw-field traversal -> representative golden trace assertions on one or two existing scenarios.
4. Additional authoritative/action-trace mapping is not applicable because this ticket changes only the trace query/build proof surface, not runtime or authoritative behavior.
5. If a remaining golden assertion still genuinely needs raw struct access after helper additions, the ticket should name that gap explicitly instead of hiding it behind partial wrappers.

## What to Change

### 1. Add stable trace query helpers

- Add helper/query methods on `DecisionTraceSink`, `PlanningPipelineTrace`, or dedicated trace helper types for common test assertions:
  - generated opportunity lookup
  - ranked opportunity lookup
  - selected opportunity lookup
  - evidence lookup by `OpportunityKey` and, where justified, by desire-level predicate
  - ranking comparison lookup without exposing raw field paths
- Keep helper signatures explicit about whether they operate at desire level or opportunity level.

### 2. Add lightweight trace builders/helpers for focused tests

- Introduce minimal builder/test-support helpers where repeated manual trace struct construction is currently noisy or shape-coupled.
- Prefer small local builders in `decision_trace.rs` test support or a dedicated trace-test helper module over sprawling fixture objects.
- The builder surface must preserve canonical opportunity identity and should not synthesize fake goal-only convenience paths.

### 3. Migrate representative focused and golden tests

- Update the most coupled tests to use the new helper/query/build APIs instead of direct field traversal.
- Prioritize representative tests that currently prove:
  - ranked comparison assertions
  - selected opportunity assertions
  - candidate evidence assertions
  - generated-candidate presence assertions
- Do not attempt a repo-wide style rewrite beyond the places needed to prove the new surface is sufficient.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — helper/query/build surface)
- `crates/worldwake-ai/src/lib.rs` (modify — public re-exports if helper types/functions are public)
- `crates/worldwake-ai/src/agent_tick/tests.rs` (modify — migrate representative focused assertions)
- `crates/worldwake-ai/tests/golden_*.rs` (modify — migrate representative golden assertions that currently depend on raw field layout)

## Out of Scope

- Planner/runtime behavior changes
- Authoritative action/system changes
- Opportunity identity refactors
- Save/load changes
- Reintroducing goal-only compatibility aliases
- Repo-wide test-style normalization unrelated to decision traces

## Acceptance Criteria

### Tests That Must Pass

1. Focused decision-trace tests can assert generated/ranked/evidence/selected facts through stable helper/query APIs without direct raw-field traversal for the covered scenarios.
2. The helper surface preserves the distinction between desire-level queries and opportunity-level queries.
3. Representative golden tests use the helper/query surface instead of raw struct layout for their trace assertions.
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo clippy --workspace`
6. Existing suite: `cargo test --workspace`

### Invariants

1. `OpportunityKey` remains the canonical identity for concrete candidate-stage trace facts.
2. Helper/query APIs are derived views over trace state, not duplicate stored truth.
3. No backward-compatibility aliases are added to keep obsolete raw trace field assumptions alive.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — add focused helper/query tests for generated, ranked, selected, and evidence lookup.
   Rationale: proves the new helper surface is strong enough to replace raw storage traversal for the core trace questions.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — migrate one or more representative focused assertions to the helper surface.
   Rationale: proves the helpers work in realistic traced agent-tick scenarios, not just synthetic unit fixtures.
3. `crates/worldwake-ai/tests/golden_*.rs` — migrate representative golden assertions that currently inspect `generated`, ranked summaries, or evidence traces by raw field layout.
   Rationale: proves the helper surface is durable enough for end-to-end trace assertions and reduces future refactor churn.

### Commands

1. `cargo test -p worldwake-ai decision_trace::tests`
2. `cargo test -p worldwake-ai agent_tick::tests`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace`
5. `cargo test --workspace`
