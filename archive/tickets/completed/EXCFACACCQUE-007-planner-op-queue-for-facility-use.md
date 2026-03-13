# EXCFACACCQUE-007 — `QueueForFacilityUse` Planner Op + Semantics

**Status**: COMPLETED
**Spec sections**: `specs/DRAFT-exclusive-facility-access-queues.md` §10
**Crates**: `worldwake-ai`, `worldwake-sim`

## Summary

Finish the planner-side queue semantics for exclusive facilities without introducing a second abstraction layer. The planner already had a `QueueForFacilityUse` op kind and queue-aware state, but it was not wired end-to-end through goal relevance, hypothetical transition state, exclusive-facility identification, and search-time payload binding.

## Assumption Reassessment (2026-03-13)

1. `PlannerOpKind::QueueForFacilityUse` already existed in `crates/worldwake-ai/src/planner_ops.rs`. The ticket assumption that the variant still needed to be added was wrong.
2. The existing architecture does not encode per-step data inside `PlannerOpKind`. Facility identity and intended action belong in `PlannedStep.targets` and `ActionPayload::QueueForFacilityUse`, not in the enum variant itself.
3. Planning queue/grant state from EXCFACACCQUE-006 was only partially sufficient. `PlanningState` and `PlanningSnapshot` already knew about queue position and grants, but they did not preserve enough information to distinguish an exclusive facility with an empty queue from an ordinary facility.
4. Search could see `queue_for_facility_use`, but raw affordances arrived without a bound `QueueForFacilityUsePayload { intended_action }`. That made the op non-executable as a meaningful planner step.
5. Goal relevance was stale. `GoalKind::AcquireCommodity`, `ConsumeOwnedCommodity`, `ProduceCommodity`, `RestockCommodity`, and `Heal` did not all route through `QueueForFacilityUse`, so the planner could still bypass the queue model conceptually.
6. The previous out-of-scope note on decision/failure runtime was outdated in one direction: runtime/failure code already knew about `QueueForFacilityUse`, so this ticket only needed to complete planner/search semantics, not add new runtime concepts.

## Scope Correction

This ticket should not mutate the long-term planner shape by embedding facility ids or action ids into `PlannerOpKind`.

The correct scope is:

1. Make the existing op kind actually usable in search.
2. Bind queue steps to concrete intended actions through payloads.
3. Ensure the planner only queues at facilities that are explicitly exclusive in world state.
4. Ensure a matching active grant suppresses the queue step and allows direct exclusive-action planning.

This ticket does **not** own broader top-level goal discovery/ranking/runtime policy from EXCFACACCQUE-008 through EXCFACACCQUE-010.

## Architecture Reassessment

### Preferred design

Keep `PlannerOpKind` as a stable action-family classifier:

```rust
enum PlannerOpKind {
    QueueForFacilityUse,
    Harvest,
    Craft,
    // ...
}
```

Do **not** change it to:

```rust
QueueForFacilityUse { facility: EntityId, intended_action: ActionDefId }
```

That would duplicate step-local data in the type tag and make planner semantics less extensible. The current architecture is cleaner if:

1. `PlannerOpKind` stays structural.
2. `PlannedStep.targets` carries the concrete facility.
3. `ActionPayload::QueueForFacilityUse` carries `intended_action`.
4. goal-model transitions receive the payload when simulating hypothetical state.

### Missing architectural piece that mattered

Exclusive access must be explicit in the belief/read model even when no one is queued yet. The planner cannot infer exclusivity from incidental grant/queue occupancy.

The right fix was to preserve exclusive-facility policy in planning snapshot state by keeping empty facility-queue snapshot data for facilities that actually have `ExclusiveFacilityPolicy`.

### Search binding rule

Search must never admit a payload-less queue step as a valid terminal barrier. A queue step is only valid when bound to a concrete exclusive action id. The planner therefore derives queue candidates from:

1. the grounded goal,
2. the exclusive facility target,
3. the facility workstation/resource facts in the planning state,
4. the registered harvest/craft action defs.

## What Changed

### 1. Goal-model wiring completed

- Added `QueueForFacilityUse` to the relevant op sets for exclusive-facility goal families.
- Made queue steps simulate `PlanningState::simulate_queue_join(...)` using the concrete queue payload.
- Treated queue steps as progress barriers for the affected goal families.

### 2. Planner transition seam corrected

- `apply_planner_step(...)` now receives the step payload, so queue semantics can read `intended_action` from `ActionPayload::QueueForFacilityUse`.
- This keeps concrete step data in payload/targets instead of duplicating it in `PlannerOpKind`.

### 3. Exclusive-facility visibility corrected

- Added belief-view support for detecting `ExclusiveFacilityPolicy`.
- `PlanningSnapshot` now keeps facility-queue snapshot data for exclusive facilities even when queue position and grant are both empty.
- This closes the architectural hole where an empty exclusive facility looked identical to a normal workstation.

### 4. Search candidate binding completed

- Search now synthesizes fully bound queue candidates for exclusive facilities from grounded goals plus registered harvest/craft defs.
- Raw `queue_for_facility_use` affordances without a concrete intended action are rejected from search.
- Search suppresses queue steps when the actor is already queued or already holds the matching grant.

## Files Changed

- `crates/worldwake-ai/src/goal_model.rs`
- `crates/worldwake-ai/src/planner_ops.rs`
- `crates/worldwake-ai/src/planning_snapshot.rs`
- `crates/worldwake-ai/src/search.rs`
- `crates/worldwake-sim/src/belief_view.rs`
- `crates/worldwake-sim/src/omniscient_belief_view.rs`

## Tests

### New or strengthened coverage

- `goal_model::tests::restock_goal_relevant_ops_include_trade_production_and_cargo`
- `goal_model::tests::queue_for_facility_use_step_simulates_queue_join_from_payload`
- `goal_model::tests::queue_for_facility_use_is_progress_barrier_for_exclusive_goal_families`
- `planning_snapshot::tests::build_snapshot_keeps_empty_facility_queue_data_for_exclusive_facility`
- `search::tests::search_queues_before_harvest_at_exclusive_facility_without_grant`
- `search::tests::search_skips_queue_when_matching_grant_is_already_active`

### Verification

- `cargo test -p worldwake-systems queue`
- `cargo test -p worldwake-ai`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`

## Acceptance Outcome

Accepted. The planner now:

1. inserts `QueueForFacilityUse` as a real progress barrier for exclusive facilities,
2. binds that step to a concrete intended exclusive action,
3. recognizes exclusivity from explicit world/belief state rather than hidden inference,
4. skips the queue step when the matching grant is already active.

## Outcome

Originally planned:

- add a new richer `PlannerOpKind` variant
- wire queue semantics in goal model/search

Actually changed:

- kept the cleaner existing kind-only `PlannerOpKind` architecture
- passed payloads through planner goal-model transitions
- exposed exclusive-facility policy in planning beliefs/snapshots
- synthesized concrete queue candidates in search and rejected unbound queue barriers
- added missing planner/search/snapshot tests and verified the full workspace with tests and clippy
