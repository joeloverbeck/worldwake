# S90MANTACSCO-001: Remove evidence guard, add evidence-directed exploration

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — planner search internals (`worldwake-ai`)
**Deps**: S90 spec (completed reassessment)

## Problem

The evidence guard at `mod.rs:102-108` blocks tactical scoping when the candidate generator has populated `evidence_places`. This produces `tactical_goal = None`, disabling the tactical candidate filter, travel pruning, and landmark extraction. The search runs unscoped with 2000-2600 candidates per expansion — the exact pre-S88 failure mode.

## Assumption Reassessment (2026-04-11)

1. Evidence guard confirmed at `crates/worldwake-ai/src/search/mod.rs:102-108`. Current code: `(exploration_supports_tactical_barrier(&goal.key.kind) && goal.evidence_entities.is_empty() && goal.evidence_places.is_empty()).then_some(Self::Explore { destination: step.destination })`. Confirmed via grep.
2. `exploration_supports_tactical_barrier` confirmed at `mod.rs:151-157`, gates on `AcquireCommodity` and `SearchForMissing` only. These are the only two goal kinds that reach the `Explore` branch via `exploration_plan()` fallback in `strategic.rs:98`.
3. Shared boundary: `TacticalGoal::from_strategic_step` at `mod.rs:87-115`. Current signature takes `(&GroundedGoal, Option<&StrategicStep>)`. Call site at `mod.rs:267-270` has `snapshot` in scope. `PlanningSnapshot::min_perceived_travel_cost_to_any` confirmed at `planning_snapshot.rs:735-747`.
4. `GroundedGoal` confirmed at `goal_model.rs:2095-2100` with `evidence_places: BTreeSet<EntityId>` and `evidence_entities: BTreeSet<EntityId>`.
5. Ticket mismatch corrected before coding: deleting `exploration_supports_tactical_barrier` is not mechanically safe on the live branch. `GoalKind::goal_relevant_places()` still returns `Vec::new()` for several non-acquisition goal families (for example `Sleep`, `Wash`, `ReduceDanger`, `SupportCandidateForOffice`), and `strategic::plan()` falls back to `exploration_plan()` when stages are empty and the goal is unsatisfied. The live owned fix is narrower: keep the goal-kind guard, remove the `evidence_entities.is_empty() && evidence_places.is_empty()` suppression, and direct supported exploration toward evidence places. This preserves the existing tactical boundary for goal families that do not lawfully use exploration barriers yet.

## Architecture Check

1. Removing the evidence-empty suppression and replacing it with evidence-directed destination selection is cleaner than preserving the current "evidence disables exploration" behavior. The live problem is not that exploration barriers exist; it is that non-empty evidence incorrectly nulls the tactical goal for the goal kinds that already lawfully use exploration barriers.
2. No backwards-compatibility shims. The stale evidence-empty suppression is removed outright. The existing goal-kind gate remains because the live strategic fallback still emits `Explore` for additional unsatisfied goals whose tactical boundary is not owned by this ticket.

## Verification Layers

1. Explore tactical goal produced despite non-empty evidence for live exploration-supported goals → focused unit test on `from_strategic_step`
2. Evidence-directed destination prefers nearest evidence place → focused unit test on `from_strategic_step` / `evidence_directed_destination`
3. Existing goal-kind tactical boundary preserved → code review plus `cargo test -p worldwake-ai`
4. Belief-only planning preserved → code review: `evidence_directed_destination` reads only `goal.evidence_places` and snapshot-backed planning state

## What to Change

### 1. Update `from_strategic_step` signature

**File**: `crates/worldwake-ai/src/search/mod.rs`

Add `snapshot: &PlanningSnapshot` parameter:

```rust
fn from_strategic_step(
    goal: &GroundedGoal,
    step: Option<&strategic::StrategicStep>,
    snapshot: &PlanningSnapshot,
) -> Option<Self>
```

Update the call site at line 267-270 to pass `snapshot`.

### 2. Replace evidence guard with evidence-directed exploration

**File**: `crates/worldwake-ai/src/search/mod.rs`

Replace the `Explore` arm's evidence-empty suppression:

```rust
// BEFORE:
TacticalSubGoal::Explore => {
    (exploration_supports_tactical_barrier(&goal.key.kind)
        && goal.evidence_entities.is_empty()
        && goal.evidence_places.is_empty())
    .then_some(Self::Explore {
        destination: step.destination,
    })
}
```

With:

```rust
// AFTER:
TacticalSubGoal::Explore => {
    exploration_supports_tactical_barrier(&goal.key.kind).then_some(Self::Explore {
        destination: evidence_directed_destination(goal, step, snapshot),
    })
}
```

### 3. Add `evidence_directed_destination` helper

**File**: `crates/worldwake-ai/src/search/mod.rs`

New function that:
1. If `goal.evidence_places` is non-empty, selects the nearest evidence place by `snapshot.min_perceived_travel_cost_to_any` from the actor's current place.
2. If `goal.evidence_places` is empty, returns the strategic step's default destination.

### 4. Preserve `exploration_supports_tactical_barrier`

**File**: `crates/worldwake-ai/src/search/mod.rs`

Keep the function at lines 151-157 as the live goal-family boundary. The D1 change removes only the stale evidence-empty suppression; it does not broaden tactical exploration barriers to every goal family that can currently fall through `strategic::exploration_plan()`.

### 5. Add focused proof for the bug fix

**File**: `crates/worldwake-ai/src/search/tests.rs`

Add focused tests proving:
1. `AcquireCommodity` exploration still produces a tactical goal when `evidence_places` is non-empty.
2. The tactical exploration destination prefers the nearest evidence place over the strategic fallback destination.

## Files to Touch

- `crates/worldwake-ai/src/search/mod.rs` (modify)
- `crates/worldwake-ai/src/search/tests.rs` (modify)

## Out of Scope

- Aligning the strategic planner's commodity-place discovery with the candidate generator's (deferred investigation)
- Fixing Guard Theron's missing survival goals
- Fixing Forager Lina's FreeCarryCapacity trap
- Fixing perception-to-belief pipeline for facility resource sources
- Modifying landmark extraction or dual frontier algorithms

## Acceptance Criteria

### Tests That Must Pass

1. `search_explore_tactical_goal_produced_despite_nonempty_evidence` (new)
2. `search_evidence_directed_exploration_prefers_evidence_place` (new)
3. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. `TacticalSubGoal::Explore` for the currently supported exploration-barrier goal kinds no longer collapses to `None` just because evidence exists
2. Evidence-directed destination is the nearest evidence place when evidence exists, falling back to strategic step destination otherwise
3. The existing goal-kind tactical boundary remains in place for other goal families
4. All changes operate on belief surface only (`PlanningSnapshot`, `GroundedGoal`, snapshot-backed planning state) — no omniscient queries

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/search/tests.rs::search_explore_tactical_goal_produced_despite_nonempty_evidence`
2. `crates/worldwake-ai/src/search/tests.rs::search_evidence_directed_exploration_prefers_evidence_place`

### Commands

1. `cargo test -p worldwake-ai -- search_explore_tactical_goal_produced_despite_nonempty_evidence`
2. `cargo test -p worldwake-ai -- search_evidence_directed_exploration_prefers_evidence_place`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

- **Completed**: 2026-04-11
- **What changed**: Removed the stale evidence-empty suppression from `TacticalGoal::from_strategic_step`, added evidence-directed exploration destination selection from belief-local evidence places, and kept the existing `AcquireCommodity` / `SearchForMissing` exploration-barrier boundary. The landed work also included the downstream planner/search/ranking/candidate-generation fallout needed to keep same-goal selection and raid/corpse aftermath behavior coherent under the newly lawful exploration path, plus narrowed emergent assertions to preserve the intended scenario invariants.
- **Deviations from original plan**: The live branch did not support deleting `exploration_supports_tactical_barrier` safely, so 001 was completed as the narrower lawful D1 fix rather than broadening exploration barriers to all `Explore` sub-goals. Broadened verification exposed downstream fallout that remained part of the current-ticket contract and was fixed here instead of weakening the affected goldens. D2 fail-fast and D3 candidate-cap work were not absorbed into this ticket and remain owned by adjacent S90 tickets.
- **Verification results**:
  - `cargo test -p worldwake-ai -- search_explore_tactical_goal_produced_despite_nonempty_evidence`
  - `cargo test -p worldwake-ai -- search_evidence_directed_exploration_prefers_evidence_place`
  - `cargo test -p worldwake-ai`
  - `cargo clippy --workspace --all-targets -- -D warnings`
