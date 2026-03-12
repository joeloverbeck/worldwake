# HARHYPENTIDE-006: Exact partial pickup planner rework and PutDownGroundLot transition

**Status**: PENDING
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — planner transitions (`worldwake-ai`), transport handler materialization output (`worldwake-systems`)
**Deps**: HARHYPENTIDE-001 (CommitOutcome), HARHYPENTIDE-002 (PlanningEntityRef, hypothetical entities), HARHYPENTIDE-003 (carry-capacity beliefs), HARHYPENTIDE-004 (PlannedStep PlanningEntityRef targets), HARHYPENTIDE-005 (MaterializationBindings)
**Spec Reference**: HARDENING-hypothetical-entity-identity.md, Section E

## Problem

The current `apply_pick_up_transition` in `planner_ops.rs` always moves the entire lot to the actor. It has no concept of partial pickup, lot splitting, or creating a new carried lot. This means the planner lies about what will happen when carry capacity is insufficient for the full lot.

## Assumption Reassessment (2026-03-12)

1. `apply_pick_up_transition` at `planner_ops.rs:259` calls `state.move_lot_to_holder(lot, actor, commodity, quantity)` with the full quantity — confirmed, no partial logic.
2. Authoritative `execute_pick_up` at `transport_actions.rs:167` splits the lot when load exceeds remaining capacity — confirmed.
3. `PlannerTransitionKind` enum has `GoalModelFallback` and `PickUpGroundLot` — confirmed, no `PutDownGroundLot`.
4. `commit_pick_up` at `transport_actions.rs:234` currently returns `Ok(())`, discarding the split-off entity ID — confirmed.
5. `execute_pick_up` returns `Ok(moved_entity)` where `moved_entity` may be the original or the split-off lot — confirmed.

## Architecture Check

1. The planner transition must mirror authoritative semantics exactly: compute remaining capacity, determine full/partial/zero fit, and create a hypothetical lot for the split-off case.
2. `PutDownGroundLot` transition is needed so later steps can put down hypothetical lots.
3. `commit_pick_up` must return `CommitOutcome` with `SplitOffLot` materialization on the split path so the runtime can bind the hypothetical lot.
4. No backward-compatibility: the old approximate `apply_pick_up_transition` is fully replaced.

## What to Change

### 1. Add `PutDownGroundLot` to `PlannerTransitionKind`

```rust
pub enum PlannerTransitionKind {
    GoalModelFallback,
    PickUpGroundLot,
    PutDownGroundLot,  // new
}
```

Update `semantics_for` to assign `PutDownGroundLot` to `put_down` actions.

### 2. Rework `apply_pick_up_transition` for exact partial pickup

Replace the current implementation with:

1. Validate co-location and target shape (using `PlanningEntityRef` methods)
2. Compute exact remaining carry capacity via `remaining_carry_capacity_ref`
3. Compute target lot load via `load_of_entity_ref`
4. If full lot fits: move the entire lot into actor possession (current behavior, using `PlanningEntityRef`)
5. If only partial quantity fits:
   - Compute `max_quantity = remaining_capacity / load_per_unit(commodity)`
   - Reduce original lot quantity in overrides by `max_quantity`
   - Call `spawn_hypothetical_lot` to create a hypothetical lot with `max_quantity` and same commodity
   - Place hypothetical lot in actor possession via override maps
   - Record the hypothetical ID so `PlannedStep.expected_materializations` can be populated
6. If nothing fits: transition is invalid (return state unchanged or signal failure)

### 3. Implement `apply_put_down_transition`

For `PutDownGroundLot`:
- If target is `PlanningEntityRef::Hypothetical(...)`: move from actor possession to ground at actor's current place
- If target is `PlanningEntityRef::Authoritative(...)`: same as current `GoalModelFallback` for put-down

### 4. Update `commit_pick_up` to return `CommitOutcome` with materialization

In `transport_actions.rs`, change `commit_pick_up`:

```rust
fn commit_pick_up(...) -> Result<CommitOutcome, ActionError> {
    let target = require_item_lot_target(instance)?;
    let moved_entity = execute_pick_up(txn, instance.actor, target)?;
    if moved_entity != target {
        // Split occurred — report the new entity
        Ok(CommitOutcome {
            materializations: vec![Materialization {
                tag: MaterializationTag::SplitOffLot,
                entity: moved_entity,
            }],
        })
    } else {
        Ok(CommitOutcome::empty())
    }
}
```

### 5. Wire `expected_materializations` in search

When search constructs a `PlannedStep` for a `PickUpGroundLot` transition that produced a hypothetical entity, populate `expected_materializations` with the corresponding `HypotheticalEntityId` and `MaterializationTag::SplitOffLot`.

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — `PutDownGroundLot` variant, rework `apply_pick_up_transition`, add `apply_put_down_transition`)
- `crates/worldwake-ai/src/search.rs` (modify — populate `expected_materializations` for pickup steps)
- `crates/worldwake-systems/src/transport_actions.rs` (modify — `commit_pick_up` returns `CommitOutcome` with `SplitOffLot`)

## Out of Scope

- Materializing transitions for harvest, craft, trade, or loot (future work)
- New action families or action definitions
- Changes to `worldwake-core`
- Changes to carry-capacity computation (HARHYPENTIDE-003)
- Changes to `PlanningState` identity model (HARHYPENTIDE-002)
- Revalidation/binding logic (HARHYPENTIDE-005)

## Acceptance Criteria

### Tests That Must Pass

1. Exact full-fit pickup: lot load <= remaining capacity → full lot moved, `CommitOutcome::empty()`.
2. Exact partial-fit pickup: lot load > remaining capacity → original lot reduced, hypothetical lot created with correct quantity, `CommitOutcome` with `SplitOffLot`.
3. Zero-fit pickup: per-unit load > remaining capacity → transition invalid.
4. `PutDownGroundLot` transition moves hypothetical lot to ground at actor's place.
5. `PutDownGroundLot` transition moves authoritative lot to ground (fallback behavior preserved).
6. `commit_pick_up` returns `CommitOutcome::empty()` for full-fit path.
7. `commit_pick_up` returns `CommitOutcome` with `SplitOffLot` materialization for split path.
8. Search produces `expected_materializations` on partial pickup steps.
9. Authoritative split test (`pick_up_splits_lot_when_only_partial_quantity_fits`) still passes.
10. Existing suite: `cargo test --workspace`
11. Existing lint: `cargo clippy --workspace`

### Invariants

1. Planner transition semantics exactly mirror authoritative `execute_pick_up` logic for quantity determination.
2. Hypothetical lots are always placed at authoritative places.
3. `CommitOutcome` is only non-empty when a split actually occurs.
4. No backward-compatibility for the old approximate `apply_pick_up_transition`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — exact partial pickup transition tests (full, partial, zero fit), `PutDownGroundLot` transition tests.
2. `crates/worldwake-systems/src/transport_actions.rs` — `commit_pick_up` returns correct `CommitOutcome` for split and non-split paths.

### Commands

1. `cargo test -p worldwake-ai planner_ops`
2. `cargo test -p worldwake-systems transport`
3. `cargo test --workspace`
4. `cargo clippy --workspace`
