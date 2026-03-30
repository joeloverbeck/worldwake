# E20COMBEH-005: Planner integration for wilderness relief

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-ai (planner_ops.rs, goal_model.rs)
**Deps**: E20COMBEH-004 (relieve_wilderness action registered)

## Problem

The GOAP planner currently maps only `toilet` to `PlannerOpKind::Relieve` and `goal_relevant_places` for `GoalKind::Relieve` returns only `PlaceTag::Latrine` locations. The planner must also recognize `relieve_wilderness` as a Relieve operation and consider outdoor places as goal-relevant for the Relieve goal, so agents can discover wilderness relief as a fallback path.

## Assumption Reassessment (2026-03-30)

1. **`classify_action_def`** (`crates/worldwake-ai/src/planner_ops.rs:72-116`): Match on `(ActionDomain::Needs, "toilet") => Some(PlannerOpKind::Relieve)`. Must add `(ActionDomain::Needs, "relieve_wilderness") => Some(PlannerOpKind::Relieve)`. Confirmed via `Read`.
2. **`goal_relevant_places`** (`crates/worldwake-ai/src/goal_model.rs:993`): `GoalKind::Relieve => places_with_place_tag(state, PlaceTag::Latrine)`. Must expand to include all `OUTDOOR_RELIEF_TAGS` places. Confirmed via `Read`.
3. **`places_with_place_tag`** helper: Used throughout `goal_relevant_places`. Available for reuse.
4. **`OUTDOOR_RELIEF_TAGS`**: Defined in E20COMBEH-004 in `worldwake-core/src/topology.rs`. The ai crate depends on core, so this constant is accessible.
5. **GoalKind::Relieve**: Already exists (`crates/worldwake-core/src/goal.rs`). No new goal kinds needed.
6. **Planner search behavior**: The planner explores plans in cost order (A* with travel heuristic). Latrines at shorter travel distance are naturally preferred. If a latrine and an outdoor place are equidistant, both may produce valid plans — the planner picks the first found, which is acceptable since both resolve the goal.

## Architecture Check

1. Adding one match arm and expanding one goal_relevant_places branch is minimal and clean. No new planner ops, no new goal kinds, no special-case logic. The planner treats `toilet` and `relieve_wilderness` as interchangeable means to the same end — the design is correct per Principle 1 (emergence through affordance evaluation, not authored fallback chains).
2. No backward-compatibility shims. The expansion is purely additive — existing Relieve plans still work.

## Verification Layers

1. PlannerOpKind classification → focused unit test (relieve_wilderness maps to Relieve)
2. goal_relevant_places expansion → focused unit test (Relieve returns latrine + outdoor places)
3. No cross-system verification needed — this is AI-layer only. Golden tests in E20COMBEH-006/007 verify end-to-end behavior.

## What to Change

### 1. Add `relieve_wilderness` to `classify_action_def`

In `crates/worldwake-ai/src/planner_ops.rs`, in the match on `(def.domain, def.name.as_str())`, add:

```rust
(ActionDomain::Needs, "relieve_wilderness") => Some(PlannerOpKind::Relieve),
```

### 2. Expand `goal_relevant_places` for `GoalKind::Relieve`

In `crates/worldwake-ai/src/goal_model.rs`, replace:

```rust
GoalKind::Relieve => places_with_place_tag(state, PlaceTag::Latrine),
```

With:

```rust
GoalKind::Relieve => {
    let mut places = places_with_place_tag(state, PlaceTag::Latrine);
    for tag in OUTDOOR_RELIEF_TAGS {
        places.extend(places_with_place_tag(state, *tag));
    }
    places.sort_unstable();
    places.dedup();
    places
}
```

Import `OUTDOOR_RELIEF_TAGS` from `worldwake_core::topology`.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — add match arm)
- `crates/worldwake-ai/src/goal_model.rs` (modify — expand goal_relevant_places)

## Out of Scope

- Wilderness relief action definition (E20COMBEH-004)
- Travel body cost changes (E20COMBEH-001 through E20COMBEH-003)
- Golden tests (E20COMBEH-006 through E20COMBEH-008)
- Planner cost modeling of dirtiness penalty (not in scope — the planner doesn't model secondary costs directly; natural preference comes from travel distance)
- New GoalKind variants (none needed)
- Changes to PlannerOpSemantics for Relieve (existing semantics apply to both toilet and relieve_wilderness)

## Acceptance Criteria

### Tests That Must Pass

1. `classify_action_def` maps `(Needs, "relieve_wilderness")` to `PlannerOpKind::Relieve`
2. `goal_relevant_places(GoalKind::Relieve)` returns places with `PlaceTag::Latrine` AND places with outdoor tags
3. Result is deduplicated (a place tagged both `Latrine` and `Road` appears once)
4. Existing suite: `cargo test -p worldwake-ai`
5. Existing suite: `cargo test --workspace`

### Invariants

1. Both `toilet` and `relieve_wilderness` map to the same `PlannerOpKind::Relieve` — they are interchangeable means to the same goal
2. No new GoalKind introduced — `GoalKind::Relieve` covers both
3. goal_relevant_places returns a sorted, deduplicated list (determinism invariant)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — `classify_relieve_wilderness_as_relieve` — new test: action classification
2. `crates/worldwake-ai/src/goal_model.rs` — `relieve_goal_relevant_places_includes_outdoor` — new test: expanded places include outdoor tags
3. `crates/worldwake-ai/src/goal_model.rs` — `relieve_goal_relevant_places_deduplicates` — new test: no duplicates

### Commands

1. `cargo test -p worldwake-ai`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
