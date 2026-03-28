# S33OPPSCOGOAIDE-013: Canonicalize `SelectionTrace` on `selected_opportunity`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `worldwake-ai` decision-trace data model and writer/read helpers
**Deps**: archive/tickets/completed/S33OPPSCOGOAIDE-012-trace-test-query-surface.md, specs/S33-opportunity-scoped-goal-identity.md

## Problem

`SelectionTrace` currently stores the selected branch twice: once as `selected: Option<GoalKey>` and again as `selected_opportunity: Option<OpportunityKey>`. For an opportunity-scoped trace architecture, that is one fact traveling through two lawful stored paths inside the same trace record.

That duplication is the remaining architectural seam in the decision-trace model. It keeps a goal-only alias alive next to the canonical opportunity identity, which increases drift risk and makes future trace cleanups harder to reason about.

## Assumption Reassessment (2026-03-28)

1. The exact shared abstraction boundary under audit is the plan-selection portion of the decision-trace model in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs): `SelectionTrace`, `PlanningPipelineTrace::selected_ranked_summary()`, `AgentDecisionTrace::goal_history_entry()`, `goal_status_in_planning()`, and the formatting helpers that currently read both `selected` and `selected_opportunity`.
2. Current code confirms the duplication exists in stored trace state, not just in helper APIs: `SelectionTrace` has both `pub selected: Option<GoalKey>` and `pub selected_opportunity: Option<OpportunityKey>` in [`crates/worldwake-ai/src/decision_trace.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_trace.rs).
3. Current writer sites populate both fields from the same selected plan path in [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs): snapshot continuation writes `selected = active_goal_key` and `selected_opportunity = runtime.current_plan.as_ref().map(|plan| plan.opportunity)`, while fresh search writes `selected = Some(selected_goal)` and `selected_opportunity = Some(selected_plan.opportunity)`.
4. The same fact currently has two stored transport paths:
   - concrete selected branch as `SelectionTrace.selected_opportunity`
   - desire-level selected goal as `SelectionTrace.selected`
   After this ticket, the canonical stored path should be `selected_opportunity`; desire-level answers should be derived from that canonical path via helper methods.
5. This ticket is planner-trace-driven but not behavior-driven. The live `GoalKind` families that already exercise selection reads include `TreatWounds`, `RestockCommodity`, `InvestigateViolation`, and `ReduceDanger`, but the invariant here is selection identity shape, not any one goal family’s ranking arithmetic.
6. This is a single-layer AI/debugging-contract refactor. No authoritative action validation, scheduler start, or world-state mutation behavior is in scope.
7. Existing focused coverage already proves selected-opportunity semantics are meaningful and stable: `decision_trace::tests::selected_ranked_summary_uses_selected_opportunity_for_same_goal_siblings`, `agent_tick::tests::trace_snapshot_continuation_records_selected_plan_provenance`, and `agent_tick::tests::trace_planning_outcome_for_hungry_agent` all exercise the selected-plan trace surface in the current binary layout (`cargo test -p worldwake-ai -- --list` verified on 2026-03-28).
8. The architectural mismatch is real: S33 made `OpportunityKey` the canonical concrete candidate identity, but `SelectionTrace.selected` still stores a parallel goal-only summary as primary data instead of as a derived view. Scope correction for this ticket is to remove that duplicate stored path, not to preserve it with aliases.
9. Adjacent contradiction exposed by reassessment: downstream tests and helper consumers still read `selection.selected` directly in golden files. That migration is a required consequence of this cleanup, but it is large enough to keep as a separate follow-up ticket rather than broadening this ticket’s production-shape change.
10. No ranking-sensitive symmetry claims are required here. The change is storage canonicalization only; ranking order, motive arithmetic, and branch selection rules stay unchanged.

## Architecture Check

1. Canonicalizing on `selected_opportunity` is cleaner because it preserves one concrete identity source for the selected branch and derives desire-level answers from it. That matches `docs/FOUNDATIONS.md` Principle 3’s bias toward the concrete thing over an abstract summary and Principle 25’s rule that derived summaries are caches, never truth.
2. Removing the duplicate stored field is cleaner than keeping both fields “in sync” with writer discipline. Synchronization rules are a maintenance burden; one canonical field plus derived helpers is simpler and more robust.
3. No backwards-compatibility alias or deprecated mirror should be introduced. If `selection.selected` disappears, consumers must migrate to derived helpers rather than keeping a shadow field alive.

## Verification Layers

1. `SelectionTrace` stores one canonical concrete branch identity and still answers desire-level selection queries correctly -> focused `decision_trace` tests.
2. Snapshot continuation and fresh search still record the correct selected branch after the shape change -> focused `agent_tick` runtime trace tests.
3. Human-readable summaries still report the selected goal/opportunity consistently after deriving goal-level views from the canonical opportunity field -> focused `decision_trace` summary tests.
4. Additional action-trace or authoritative-world verification is not applicable because this ticket changes only the AI decision-trace data model and derived read surface.

## What to Change

### 1. Remove redundant stored goal selection

- Remove `selected: Option<GoalKey>` from `SelectionTrace`.
- Add derived helpers such as `selected_goal()` / `selected_goal_is()` that read from `selected_opportunity.map(|op| op.goal_key)` instead of stored duplicate state.
- Keep helper naming explicit about whether a query is desire-level or opportunity-level.

### 2. Update trace writers and internal readers

- Update `agent_tick/planning.rs` and any other `SelectionTrace` constructors to populate only `selected_opportunity`.
- Update `goal_status_in_planning()`, `goal_history_entry()`, formatting helpers, and internal tests in `decision_trace.rs` to use the derived desire-level helper rather than direct field access.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — canonical selection identity and derived helpers)
- `crates/worldwake-ai/src/agent_tick/planning.rs` (modify — trace writer updates)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — default `SelectionTrace` construction if needed)

## Out of Scope

- Broad golden-test migration away from direct `selection.selected` reads
- Candidate-generation helper expansion unrelated to selection identity
- Planner behavior, ranking logic, or opportunity identity semantics
- Save/load changes

## Acceptance Criteria

### Tests That Must Pass

1. `SelectionTrace` no longer stores a duplicate goal-only selected field.
2. Desire-level helpers still report selected-goal status correctly by deriving from `selected_opportunity`.
3. Snapshot continuation and fresh-search traces still report the same selected branch as before the refactor.
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `OpportunityKey` remains the canonical stored identity for the selected concrete branch.
2. Desire-level selected-goal answers are derived views over canonical trace state, not parallel stored truth.
3. No compatibility shim or alias field is added to preserve the old duplicate path.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/decision_trace.rs` — update focused tests around selected-rank summary, goal-status derivation, and summary formatting.
   Rationale: proves desire-level read helpers remain correct after removing duplicate stored state.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — update focused runtime trace assertions that cover fresh-search and snapshot-continuation selection recording.
   Rationale: proves trace writers still emit the right canonical selected opportunity in live planning flows.

### Commands

1. `cargo test -p worldwake-ai decision_trace::tests::selected_ranked_summary_uses_selected_opportunity_for_same_goal_siblings`
2. `cargo test -p worldwake-ai decision_trace::tests::goal_status_distinguishes_omitted_suppressed_zero_motive_ranked_and_selected`
3. `cargo test -p worldwake-ai agent_tick::tests::trace_snapshot_continuation_records_selected_plan_provenance`
4. `cargo test -p worldwake-ai agent_tick::tests::trace_planning_outcome_for_hungry_agent`
5. `cargo test -p worldwake-ai`
