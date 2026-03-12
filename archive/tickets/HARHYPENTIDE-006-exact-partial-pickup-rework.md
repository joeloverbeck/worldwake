# HARHYPENTIDE-006: Exact partial pickup planner rework and PutDownGroundLot transition

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Large
**Engine Changes**: Yes — planner transitions (`worldwake-ai`), transport handler materialization output (`worldwake-systems`)
**Deps**: HARHYPENTIDE-001 (CommitOutcome), HARHYPENTIDE-002 (PlanningEntityRef, hypothetical entities), HARHYPENTIDE-003 (carry-capacity beliefs), HARHYPENTIDE-004 (PlannedStep PlanningEntityRef targets), HARHYPENTIDE-005 (MaterializationBindings)
**Spec Reference**: HARDENING-hypothetical-entity-identity.md, Section E

## Problem

The current `apply_pick_up_transition` in `planner_ops.rs` always moves the entire lot to the actor. It has no concept of partial pickup, lot splitting, or creating a new carried lot. This means the planner lies about what will happen when carry capacity is insufficient for the full lot.

## Assumption Reassessment (2026-03-12)

1. `apply_pick_up_transition` in `crates/worldwake-ai/src/planner_ops.rs` still moves the full authoritative lot into actor possession with no partial-fit branch, no hypothetical split-off lot, and no zero-fit rejection — confirmed.
2. Authoritative `execute_pick_up` in `crates/worldwake-systems/src/transport_actions.rs` already mirrors exact transport semantics: full fit keeps the original entity, partial fit splits a new lot, zero fit fails preconditions — confirmed.
3. `PlanningState` already has the identity and cargo primitives this ticket depends on: `PlanningEntityRef`, `HypotheticalEntityId`, `spawn_hypothetical_lot`, `carry_capacity_ref`, `load_of_entity_ref`, and `remaining_carry_capacity_ref` — confirmed. This ticket must build on that surface rather than reintroducing raw-`EntityId` assumptions.
4. `PlannedStep.targets` already stores `Vec<PlanningEntityRef>`, and runtime/revalidation binding support already exists via `expected_materializations`, `MaterializationBindings`, and `resolve_planning_targets_with` — confirmed. This ticket must plug into that path rather than redoing HARHYPENTIDE-004/HARHYPENTIDE-005.
5. `apply_hypothetical_transition` still accepts `&[EntityId]`, and `search.rs` still builds steps entirely from authoritative affordance bindings — confirmed. This is a real scope item for this ticket because hypothetical `put_down` and split-off output identity cannot be represented cleanly through the current transition boundary.
6. `commit_pick_up` already returns `CommitOutcome`, but it always returns `CommitOutcome::empty()` and discards the split-off entity ID from `execute_pick_up` — confirmed.
7. Search currently enumerates candidates from `get_affordances(&node.state, ...)`, and `BeliefView` intentionally does not leak hypothetical entities. Without planner-side candidate synthesis for hypothetical possessions, a new `PutDownGroundLot` transition alone would be unreachable from search — confirmed.

## Architecture Check

1. Planner-owned transition semantics should remain the single source of truth for hypothetical state changes. Search should consume transition results, not reverse-engineer them by diffing state after the fact.
2. The planner transition surface must become typed and metadata-aware so it can carry exact post-transition targets and expected materializations. This is cleaner and more extensible than adding `search.rs`-local inference for partial pickup.
3. Exact pickup semantics must mirror authoritative transport behavior: compute remaining capacity, determine full/partial/zero fit, and create a hypothetical lot for the split-off case.
4. `PutDownGroundLot` transition is still needed so later steps can target hypothetical carried lots, but search must also synthesize planner-only `put_down` candidates for hypothetical possessions.
5. `commit_pick_up` must emit `CommitOutcome` with `SplitOffLot` only on the split path so the existing runtime binding path can bind the hypothetical lot.
6. No backward-compatibility: replace the approximate transition path rather than aliasing it.

## What to Change

### 1. Expand the planner transition surface for typed targets and transition metadata

Replace the current `apply_hypothetical_transition(..., targets: &[EntityId]) -> PlanningState` shape with a planner-owned result type that can carry:

- post-transition `PlanningState`
- final `Vec<PlanningEntityRef>` step targets
- `expected_materializations`

This keeps transition ownership in `planner_ops.rs`, which matches the hardening architecture and avoids brittle `search.rs` logic that tries to infer hypothetical outputs after the transition has already happened.

### 2. Add `PutDownGroundLot` to `PlannerTransitionKind`

```rust
pub enum PlannerTransitionKind {
    GoalModelFallback,
    PickUpGroundLot,
    PutDownGroundLot,  // new
}
```

Update `semantics_for` to assign `PutDownGroundLot` to `put_down` actions.

### 3. Rework `apply_pick_up_transition` for exact partial pickup

Replace the current implementation with:

1. Accept typed planner targets and validate co-location/target shape
2. Compute exact remaining carry capacity via `remaining_carry_capacity_ref`
3. Compute target lot load via `load_of_entity_ref`
4. If full lot fits: move the entire lot into actor possession (current behavior, using `PlanningEntityRef`)
5. If only partial quantity fits:
   - Compute `max_quantity = remaining_capacity / load_per_unit(commodity)`
   - Reduce original lot quantity in overrides by `max_quantity`
   - Call `spawn_hypothetical_lot` to create a hypothetical lot with `max_quantity` and same commodity
   - Place hypothetical lot in actor possession via override maps
   - Return the hypothetical target and `ExpectedMaterialization { tag: SplitOffLot, hypothetical_id }`
6. If nothing fits: transition is invalid (return state unchanged or signal failure)

### 4. Implement `apply_put_down_transition`

For `PutDownGroundLot`:
- If target is `PlanningEntityRef::Hypothetical(...)`: move from actor possession to ground at actor's current place
- If target is `PlanningEntityRef::Authoritative(...)`: move from actor possession to ground at actor's current place using the same typed transition surface

### 5. Synthesize planner-only `put_down` successors for hypothetical possessions

Because `BeliefView` does not expose hypothetical entities and should stay that way, search must add planner-only `put_down` candidates for hypothetical lots directly possessed by the actor in the current `PlanningState`.

Requirements:

1. Preserve authoritative affordance generation for real-world entities.
2. Add a narrow planner-side path for hypothetical direct possessions only.
3. Reuse shared action definitions and planner semantics; do not add a parallel fake action family.
4. Keep this cargo-specific and minimal. Do not generalize into a new compatibility layer.

### 6. Update `commit_pick_up` to return `CommitOutcome` with materialization

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

### 7. Wire planner transition metadata into search

When search constructs a `PlannedStep`, consume the metadata returned from the planner transition surface:

- use the transition-provided typed targets, not just the authoritative affordance bindings
- populate `expected_materializations` from the transition result
- keep step construction dumb; transition ownership remains in `planner_ops.rs`

## Files to Touch

- `crates/worldwake-ai/src/planner_ops.rs` (modify — transition result surface, `PutDownGroundLot`, exact pickup/put-down transitions)
- `crates/worldwake-ai/src/search.rs` (modify — consume transition metadata, add planner-only hypothetical put-down candidates)
- `crates/worldwake-systems/src/transport_actions.rs` (modify — `commit_pick_up` returns `CommitOutcome` with `SplitOffLot`)

## Out of Scope

- Materializing transitions for harvest, craft, trade, or loot (future work)
- New action families or action definitions
- Changes to `worldwake-core`
- Changes to carry-capacity computation (HARHYPENTIDE-003)
- Changes to `PlanningState` identity model (HARHYPENTIDE-002)
- Revalidation/binding runtime design itself (HARHYPENTIDE-005 already landed; this ticket only plugs into it)

## Acceptance Criteria

### Tests That Must Pass

1. Exact full-fit pickup: lot load <= remaining capacity → full lot moved, `CommitOutcome::empty()`.
2. Exact partial-fit pickup: lot load > remaining capacity → original lot reduced, hypothetical lot created with correct quantity, `CommitOutcome` with `SplitOffLot`.
3. Zero-fit pickup: per-unit load > remaining capacity → transition invalid.
4. `PutDownGroundLot` transition moves hypothetical lot to ground at actor's place.
5. `PutDownGroundLot` transition moves authoritative lot to ground using the same typed transition surface.
6. `commit_pick_up` returns `CommitOutcome::empty()` for full-fit path.
7. `commit_pick_up` returns `CommitOutcome` with `SplitOffLot` materialization for split path.
8. Search produces `expected_materializations` on partial pickup steps by consuming planner transition metadata.
9. Search can construct a hypothetical `put_down` step for a directly possessed hypothetical split-off lot.
10. Authoritative split test (`pick_up_splits_lot_when_only_partial_quantity_fits`) still passes.
11. Existing suite: `cargo test --workspace`
12. Existing lint: `cargo clippy --workspace`

### Invariants

1. Planner transition semantics exactly mirror authoritative `execute_pick_up` logic for quantity determination.
2. Planner-owned transition metadata is the only source of hypothetical split-off identity and expected materializations for search steps.
3. Hypothetical lots are always placed at authoritative places.
4. `CommitOutcome` is only non-empty when a split actually occurs.
5. No backward-compatibility for the old approximate `apply_pick_up_transition`.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/planner_ops.rs` — exact partial pickup transition tests (full, partial, zero fit), `PutDownGroundLot` transition tests, transition metadata tests.
2. `crates/worldwake-ai/src/search.rs` — search-level coverage for partial pickup materialization metadata and hypothetical put-down candidate generation.
3. `crates/worldwake-systems/src/transport_actions.rs` — `commit_pick_up` returns correct `CommitOutcome` for split and non-split paths.

### Commands

1. `cargo test -p worldwake-ai planner_ops`
2. `cargo test -p worldwake-ai search`
3. `cargo test -p worldwake-systems transport`
4. `cargo test --workspace`
5. `cargo clippy --workspace`

## Outcome

- Outcome amended: 2026-03-12
- Completion date: 2026-03-12
- What actually changed:
  - Replaced the planner transition boundary with typed transition results that carry post-transition targets and expected materializations.
  - Reworked planner-side `pick_up` to mirror authoritative full-fit, partial-fit, and zero-fit behavior using hypothetical split-off lots.
  - Added planner-side `PutDownGroundLot` semantics for authoritative and hypothetical lots.
  - Added planner-owned synthetic candidate generation for hypothetical targets, with `put_down` as the first concrete user, so search no longer owns cargo-specific hypothetical candidate synthesis.
  - Updated authoritative `commit_pick_up` to emit `CommitOutcome { SplitOffLot }` only on the split path.
  - Added and strengthened planner, search, and transport tests around these paths.
- Deviations from original plan:
  - Search does not infer split outputs itself. Transition metadata now comes from `planner_ops.rs`, which keeps hypothetical transition ownership in one place and is cleaner than the original ticket’s `search.rs`-local inference plan.
  - The initial completion used a narrow `search.rs` hook for hypothetical `put_down`. That was refined immediately afterward into a planner-owned synthetic candidate surface so future materializing transitions do not need one-off search hooks.
- Verification results:
  - `cargo test -p worldwake-ai planner_ops` passed.
  - `cargo test -p worldwake-ai search` passed.
  - `cargo test -p worldwake-systems transport_actions` passed.
  - `cargo test --workspace` passed.
  - `cargo clippy --workspace` passed.
