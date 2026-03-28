# S33OPPSCOGOAIDE-006: Carry OpportunityKey on PlannedPlan

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — `PlannedPlan` serialized shape changes
**Deps**: `specs/S33-opportunity-scoped-goal-identity.md`

## Problem

The live S33 architecture already separates desire identity from opportunity identity during candidate generation, blocker matching, exhaustion, and ranked planning admission. The remaining seam is the search result boundary: `GroundedGoal` enters `search_plan()` with `GoalKey + OpportunityAnchor`, but `PlannedPlan` still leaves search with only `goal: GoalKey`.

That means the selected runtime plan sheds the canonical concrete opportunity that was actually searched. The exact shared abstraction boundary under audit is:

- opportunity-scoped searched candidate identity entering `crates/worldwake-ai/src/search/mod.rs::search_plan`
- persisted/runtime `crates/worldwake-ai/src/planner_ops.rs::PlannedPlan` identity leaving search and flowing through selection, retention, save/load, and diagnostics

## Assumption Reassessment (2026-03-28)

1. `OpportunityAnchor` and `OpportunityKey` already exist in [`crates/worldwake-core/src/goal.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/goal.rs), so this ticket must not claim ownership of introducing opportunity-scoped identity types.
2. `GroundedGoal` already carries `anchor` plus isolated `evidence_entities` / `evidence_places` in [`crates/worldwake-ai/src/goal_model.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/goal_model.rs). Candidate generation already emits separate opportunities, and focused tests already cover isolated evidence behavior.
3. `AgentDecisionRuntime.exhaustion_cache` is already keyed by `OpportunityKey` in [`crates/worldwake-ai/src/decision_runtime.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/decision_runtime.rs). The original ticket text claiming this is deferred architecture is stale and removed from scope.
4. `search_plan()` currently constructs `PlannedPlan::new(goal.key, ...)` in [`crates/worldwake-ai/src/search/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/mod.rs), which proves the live gap: the searched opportunity still collapses to bare `GoalKey` at plan construction.
5. `PlannedPlan` currently has no `opportunity` field in [`crates/worldwake-ai/src/planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs). This is still the missing contract.
6. `IntentionFrame` remains intentionally desire-scoped in [`crates/worldwake-core/src/intention_frame.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-core/src/intention_frame.rs). This ticket must preserve that boundary.
7. The originally cited files are partially stale: `search` now lives under [`crates/worldwake-ai/src/search/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/mod.rs), not `src/search.rs`.
8. Save/load impact is no longer a separate deferred concern. `AgentDecisionRuntime.current_plan` is already serialized, and existing runtime serialization tests in [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) prove this ticket necessarily changes persisted runtime bytes in-scope.
9. Existing real focused tests already relevant to this ticket include:
   - `agent_tick::planning::tests::same_goal_ranked_opportunities_are_attempted_in_order`
   - `agent_tick::planning::tests::traced_planning_records_same_goal_opportunity_attempt_order`
   - `candidate_generation::tests::acquire_multi_source_emits_distinct_place_anchors_and_isolated_evidence`
   These confirm the opportunity-scoped architecture upstream of `PlannedPlan` is already live.

## Architecture Check

1. Adding `opportunity: OpportunityKey` directly to `PlannedPlan` is cleaner than reconstructing opportunity from search traces, runtime side state, or current-step targets later. Those would create duplicate lawful transport paths for the same fact and violate the repository’s no-alias / no-shim rule.
2. Keeping both `goal` and `opportunity` is architecturally correct. `goal` remains the desire-scoped identity used by `IntentionFrame`, frame continuity, and higher-level plan switching. `opportunity` is the concrete tactic/source identity. They are related but not interchangeable.
3. The current architecture is otherwise sound. The beneficial change here is not a broader refactor; it is sealing the one remaining identity leak at the search-result boundary so runtime state stays aligned with the already-correct upstream candidate model.
4. Future cleanup worth noting, but out of scope here: if plan selection ever needs to compare multiple found sibling opportunities for the same `GoalKey`, the selection input boundary should become `OpportunityKey`-keyed instead of `(GoalKey, Option<PlannedPlan>)`. The current early-stop search behavior keeps that contradiction dormant, so this ticket should not broaden into that redesign.

## Verification Layers

1. searched opportunity survives into `PlannedPlan` -> focused search/unit coverage on `PlanSearchResult::Found(plan)` and direct `plan.opportunity` assertions
2. runtime persistence preserves plan opportunity identity -> focused runtime serialization coverage in `agent_tick` save/load tests
3. desire continuity remains `GoalKey`-scoped rather than opportunity-scoped -> focused `agent_tick` / plan-selection coverage that `IntentionFrame` behavior still keys off `goal`
4. same-goal sibling opportunity planning behavior remains intact -> existing focused `agent_tick::planning` tests plus full `worldwake-ai` suite

## What to Change

### 1. Add `opportunity: OpportunityKey` to `PlannedPlan`

In [`crates/worldwake-ai/src/planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs):

```rust
pub struct PlannedPlan {
    pub goal: GoalKey,
    pub opportunity: OpportunityKey,
    pub steps: Vec<PlannedStep>,
    pub total_estimated_ticks: u32,
    pub terminal_kind: PlanTerminalKind,
}
```

Update `PlannedPlan::new(...)` so callers must pass the concrete `OpportunityKey`.

### 2. Populate it only from the searched `GroundedGoal`

In [`crates/worldwake-ai/src/search/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/mod.rs), construct the `OpportunityKey` from the concrete searched `GroundedGoal` and pass it into every `PlannedPlan::new(...)` call. Do not reconstruct it later from step targets or runtime context.

### 3. Preserve it through runtime consumers and tests

Update all `PlannedPlan` constructors, clones, equality expectations, serialization assertions, and helper fixtures so the new field survives unchanged through:

- search results
- plan selection helpers
- runtime save/load
- plan retention / replacement summaries where `PlannedPlan` is cloned or compared

## Files to Touch

- [`crates/worldwake-ai/src/planner_ops.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/planner_ops.rs) — add `PlannedPlan.opportunity` and update constructor
- [`crates/worldwake-ai/src/search/mod.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/mod.rs) — populate `PlannedPlan.opportunity` from the searched `GroundedGoal`
- [`crates/worldwake-ai/src/plan_selection.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/plan_selection.rs) — adjust fixtures / helpers if needed for new constructor shape
- [`crates/worldwake-ai/src/agent_tick/planning.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/planning.rs) — preserve new field through helper tests / conversions if needed
- [`crates/worldwake-ai/src/search/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/search/tests.rs) — focused searched-opportunity coverage
- [`crates/worldwake-ai/src/agent_tick/tests.rs`](/home/joeloverbeck/projects/worldwake/crates/worldwake-ai/src/agent_tick/tests.rs) — runtime serialization and/or frame-continuity coverage updates

## Out of Scope

- introducing `OpportunityAnchor` / `OpportunityKey` types
- candidate generation emission granularity
- blocker escalation architecture
- opportunity-scoped exhaustion cache
- widening `IntentionFrame` to opportunity scope
- reworking selection inputs to compare multiple found sibling opportunities
- unrelated planner trace redesign

## Acceptance Criteria

1. Every `PlanSearchResult::Found(plan)` carries the concrete `OpportunityKey` of the searched `GroundedGoal`.
2. `PlannedPlan.goal == PlannedPlan.opportunity.goal_key` always.
3. `IntentionFrame` continuity remains keyed to `GoalKey`, not `OpportunityKey`.
4. Runtime save/load preserves `PlannedPlan.opportunity`.
5. `cargo test -p worldwake-ai`, `cargo test --workspace`, and `cargo clippy --workspace` pass.

## Tests

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs` — add a focused test proving a found plan carries the searched `OpportunityKey`.
   Rationale: this is the direct missing contract and the strongest proof surface for the bug.
2. `crates/worldwake-ai/src/agent_tick/tests.rs` — update runtime serialization coverage so a saved/restored `current_plan` retains `opportunity`.
   Rationale: `PlannedPlan` is persisted inside runtime state, so this ticket changes the serialized contract in-scope.
3. `crates/worldwake-ai/src/plan_selection.rs` and/or `crates/worldwake-ai/src/agent_tick/planning.rs` test helpers — update constructor fixtures or add a focused assertion that same-goal continuity remains keyed to `goal`.
   Rationale: guards against accidentally broadening commitment identity from desire scope to opportunity scope.

### Commands

1. `cargo test -p worldwake-ai search::tests::planned_plan_carries_searched_opportunity_key`
2. `cargo test -p worldwake-ai agent_tick::tests::save_runtime_state_serializes_persisted_driver_state`
3. `cargo test -p worldwake-ai`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

- Completion date: 2026-03-28
- What actually changed:
  - `PlannedPlan` now carries `opportunity: OpportunityKey` in addition to `goal: GoalKey`.
  - `search_plan()` now binds the searched `GroundedGoal` opportunity directly into every found `PlannedPlan`.
  - Runtime serialization coverage now proves `current_plan.opportunity` survives save/load.
  - Focused constructor and search tests now prove the new identity contract.
  - `PlanSearchResult::Found` was boxed as a required consequence of the larger `PlannedPlan` payload so `cargo clippy --workspace` remains clean without suppressing `large_enum_variant`.
- Deviations from original plan:
  - The original ticket assumed most opportunity-scoped architecture was still pending and that save/load impact was deferred. Reassessment showed both assumptions were stale, so the ticket was narrowed to the live `PlannedPlan` boundary and updated before implementation.
  - The final implementation also included the `Box<PlannedPlan>` change on `PlanSearchResult::Found`, which was not in the original draft but was required to preserve the repo’s lint contract after enlarging `PlannedPlan`.
- Verification results:
  - `cargo test -p worldwake-ai search::tests::planned_plan_carries_searched_opportunity_key`
  - `cargo test -p worldwake-ai planner_ops::tests::planned_plan_new_preserves_concrete_opportunity_identity`
  - `cargo test -p worldwake-ai agent_tick::tests::save_runtime_state_serializes_persisted_driver_state`
  - `cargo test -p worldwake-ai agent_tick::tests::same_goal_same_destination_replan_preserves_intention_frame`
  - `cargo test -p worldwake-ai`
  - `cargo test --workspace`
  - `cargo clippy --workspace`
