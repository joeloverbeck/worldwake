# S89UNITWOPHA-001: Universal TravelToGoal tactical scoping

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planner search pipeline (tactical goal construction, candidate filtering, heuristic scoping)
**Deps**: S88 (completed, archived)

## Problem

32 of 34 GoalKind variants cannot benefit from the S88 two-phase planning architecture because `goal_supports_two_phase()` whitelists only `TreatWounds` and `ProduceCommodity`. When any other goal requires travel to a remote destination (AcquireCommodity, Patrol, InvestigateViolation, ShareBelief, etc.), the planner budget-exhausts due to unscoped candidate sets of 1400-2600 per expansion. This causes agent behavioral collapse (sleep-only loops), agent death (hunger/thirst deprivation), and economic stagnation. The whitelist is a backward-compatibility layer (FND-28 violation) preserving the old flat-search path alongside the new two-phase architecture.

## Assumption Reassessment (2026-04-11)

1. `goal_supports_two_phase()` exists at `crates/worldwake-ai/src/search/mod.rs:597` — returns true only for `GoalKind::TreatWounds` and `GoalKind::ProduceCommodity`. Tactical goal construction is gated by this function at line 239. Confirmed via reassess-spec session.
2. `TacticalGoal` enum at `mod.rs:67` had 3 variants: `AcquirePrerequisite`, `Explore`, `SocialQuery`. `TacticalGoal::from_strategic_step` mapped `SatisfyGoal => None`, which kept every non-whitelisted strategic destination on the old flat-search path.
3. Shared abstraction boundary: `TacticalGoal` enum and its methods (`progress_barrier_satisfied`, `goal_facts`, and the `apply_tactical_candidate_filter` free function). All are `pub(super)` or module-private within `search/`. No cross-crate impact.
4. `tactical_goal_places` in `heuristic.rs:194` matches `AcquirePrerequisite` and `Explore` to `Some(*destination)`, `SocialQuery` to `None`. Adding `TravelToGoal` to the `Some(*)` arm is a single-line pattern extension.
5. `strategic::TacticalSubGoal::SatisfyGoal` at `strategic.rs:26` is the stable live hook for a true destination-scoping tactical goal. Reassessment during implementation also showed that `strategic::TacticalSubGoal::Explore` is produced by exploration fallback waypoints, not by stable terminal destinations; scoping ordinary goal search to those fallback waypoints caused search loops in existing `worldwake-ai` tests, so exploratory fallback remains unscoped in this ticket.

## Architecture Check

1. This removes a backward-compatibility layer (the whitelist) per FND-28 rather than expanding it. The new `TravelToGoal` variant gives true strategic `SatisfyGoal` stages a dedicated tactical destination without widening public boundaries.
2. No backwards-compatibility aliasing/shims introduced. The `goal_supports_two_phase` function is deleted entirely, not replaced with a wider whitelist.
3. Search needed one extra internal state bit: once a tactical destination has been reached, later descendants must not reactivate the same tactical barrier if they lawfully leave that place for a later phase of the same plan. That retirement stays search-internal (`SearchNode.tactical_barrier_reached`) and does not widen cross-crate APIs.

## Verification Layers

1. Whitelist removal does not break existing two-phase goals → existing S88 golden tests pass unchanged
2. Unconditional tactical-goal construction compiles cleanly across the planner search pipeline → `cargo test -p worldwake-ai`
3. Dedicated `TravelToGoal` unit/behavior coverage remains owned by `S89UNITWOPHA-003`; this ticket proves the implementation slice through existing crate coverage plus CI-matching lint
4. Single-layer ticket (planner search internals only) — no cross-system or authoritative-layer mapping applicable

## What to Change

### 1. Add `TravelToGoal` variant to `TacticalGoal` enum

In `crates/worldwake-ai/src/search/mod.rs`, add to the `TacticalGoal` enum (after `SocialQuery`):

```rust
TravelToGoal {
    destination: worldwake_core::EntityId,
},
```

### 2. Implement `progress_barrier_satisfied` for `TravelToGoal`

In the `progress_barrier_satisfied` match block (line 104), add arm:

```rust
Self::TravelToGoal { destination } => state.effective_place(actor) == Some(*destination),
```

This remains the arrival test for the tactical barrier itself.

### 3. Implement `goal_facts` for `TravelToGoal`

In the `goal_facts` match block (line 119), add arm:

```rust
Self::TravelToGoal { destination } => {
    std::collections::BTreeSet::from([PlanningFact::AtPlace(*destination)])
}
```

Enables landmark extraction to recognize travel as a goal fact.

### 4. Map `SatisfyGoal` to `TravelToGoal` in `from_strategic_step`

Change line 85 from:

```rust
strategic::TacticalSubGoal::SatisfyGoal => None,
```

to:

```rust
strategic::TacticalSubGoal::SatisfyGoal => Some(Self::TravelToGoal {
    destination: step.destination,
}),
```

### 5. Delete `goal_supports_two_phase()` and simplify construction

- Delete the `goal_supports_two_phase` function (lines 597-602).
- Replace the tactical goal construction (lines 239-242) from:

```rust
let tactical_goal = goal_supports_two_phase(goal).then(|| {
    TacticalGoal::from_strategic_step(strategic_plan.as_ref().and_then(|plan| plan.steps.first()))
})
.flatten();
```

to:

```rust
let tactical_goal = TacticalGoal::from_strategic_step(
    strategic_plan.as_ref().and_then(|plan| plan.steps.first()),
);
```

### 6. Add `TravelToGoal` arm to `apply_tactical_candidate_filter`

In the `candidates.retain` match block (line 644), add arm for `TravelToGoal`. Behavior: when actor is not at destination, retain only travel candidates advancing toward destination (reuse `travel_advances_toward_destination`). When actor is at destination, retain non-travel candidates:

```rust
TacticalGoal::TravelToGoal { destination } => {
    if actor_place == Some(*destination) {
        semantics_table
            .get(&candidate.def_id)
            .is_none_or(|semantics| semantics.op_kind != crate::PlannerOpKind::Travel)
    } else {
        travel_advances_toward_destination(
            node,
            candidate,
            semantics_table,
            *destination,
        )
    }
}
```

The landed filter also preserves goal-relevant non-travel root actions before departure, so cargo/delivery plans can still perform lawful local setup before travel.

### 7. Retire completed tactical barriers across descendants

When a successor reaches the active tactical destination, later descendants must remember that the barrier has already been satisfied even if they leave that place later in the same search branch. Land this as an internal `SearchNode` boolean carried through successor construction in `transition.rs`.

### 8. Add `TravelToGoal` to `tactical_goal_places` in heuristic.rs

In `crates/worldwake-ai/src/search/heuristic.rs`, extend the match pattern at line 196:

```rust
TacticalGoal::AcquirePrerequisite { destination, .. }
| TacticalGoal::TravelToGoal { destination } => Some(*destination),
```

## Files to Touch

- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/search/heuristic.rs` (modify)
- `crates/worldwake-ai/src/search/transition.rs` (modify)

## Out of Scope

- Extending `missing_commodities()` to additional GoalKind variants
- Fixing the FreeCarryCapacity 0-step plan dispatch deadlock
- Raising `max_node_expansions` as a mitigation
- Modifying strategic planning logic or landmark extraction algorithms
- Decision trace enrichment (S89UNITWOPHA-002)
- New tests (S89UNITWOPHA-003)

## Acceptance Criteria

### Tests That Must Pass

1. `search_treat_wounds_uses_two_phase_pick_up_before_heal` — existing S88 test unchanged
2. `search_treat_wounds_with_zero_landmarks_preserves_two_phase_plan_shape` — existing S88 test unchanged
3. `search_produce_commodity_uses_two_phase_pick_up_before_craft` — existing S88 test unchanged
4. `search_produce_commodity_with_zero_landmarks_preserves_two_phase_plan_shape` — existing S88 test unchanged
5. `search_trace_metadata_records_two_phase_strategic_and_landmark_details` — existing S88 test unchanged
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `TacticalGoal::TravelToGoal` barrier logic matches `Explore` exactly — `effective_place(actor) == Some(destination)`
2. Local goals (Sleep, Relieve, Wash, ReduceDanger, FreeCarryCapacity) produce no tactical goal — `from_strategic_step(None)` returns `None`
3. Goals with commodity prerequisites (TreatWounds, ProduceCommodity) still produce `AcquirePrerequisite` tactical goals — `missing_commodities()` path unchanged
4. `goal_supports_two_phase` function is fully deleted — zero references remain
5. Exploratory fallback waypoints remain unscoped — `strategic::TacticalSubGoal::Explore` still maps to `None` in `from_strategic_step`
6. Once a branch reaches a tactical destination, later descendants do not reactivate the same barrier after leaving that place

## Test Plan

### New/Modified Tests

None — dedicated `TravelToGoal` unit/behavior tests and trace assertions are owned by `S89UNITWOPHA-002` and `S89UNITWOPHA-003`. This ticket ensures the implementation lands cleanly and existing `worldwake-ai` coverage continues to pass.

### Commands

1. `cargo test -p worldwake-ai` — all existing tests pass
2. `cargo clippy --workspace --all-targets -- -D warnings` — no new warnings

## Outcome

Completed on 2026-04-11.

- Added `TacticalGoal::TravelToGoal` and removed the `goal_supports_two_phase()` whitelist so real strategic `SatisfyGoal` stages now receive tactical destination scoping across goal families.
- Preserved lawful root-local setup by allowing goal-relevant non-travel root actions before `TravelToGoal` departure.
- Added internal `SearchNode` barrier-retirement tracking so once a branch reaches its tactical destination, later descendants do not reactivate that same travel barrier.
- Updated local search test literals for the added `SearchNode` state so existing `worldwake-ai` coverage continued to compile against the new internal planner shape.
- Kept exploration fallback unscoped after reassessment showed those `Explore` steps are probe waypoints, not stable terminal destinations; scoping generic goal search to them caused loops and regressions in existing `worldwake-ai` tests.

## Deviations

- The parent spec draft describes `Explore` as an existing tactical analogue, but the live implementation could not safely scope ordinary goal search to exploration fallback steps without broader strategic-planner changes. This ticket therefore lands universal `TravelToGoal` for true `SatisfyGoal` stages only and leaves exploratory fallback unscoped.

## Verification Result

- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
