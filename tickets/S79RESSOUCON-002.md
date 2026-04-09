# S79RESSOUCON-002: Add harvest effect to planner hypothetical state

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AI planner (`worldwake-ai`)
**Deps**: S79 spec

## Problem

The planner cannot chain harvest → consume because `PlannerOpKind::Harvest` is a no-op in `apply_planner_step()`. When the planner hypothetically expands a Harvest step, it does not update the actor's commodity quantity, so the `AcquireCommodity` satisfaction check (`commodity_quantity(actor, commodity) > Quantity(0)`) never succeeds after a Harvest step. The planner therefore cannot predict that harvesting will yield the commodity needed for subsequent eat/drink actions.

## Assumption Reassessment (2026-04-09)

1. `PlannerOpKind::Harvest` falls through to the identity arm (`=> state`) in `apply_planner_step()` at `crates/worldwake-ai/src/goal_model.rs:1034-1056`. Confirmed via read at lines 1034-1056.
2. `HarvestActionPayload` at `crates/worldwake-sim/src/action_payload.rs:321-327` has fields `output_commodity: CommodityKind` and `output_quantity: Quantity`. Accessor `as_harvest()` exists at line 91.
3. Shared boundary: `apply_planner_step()` in `goal_model.rs` is the single function being modified. The `PlanningState` type already has `with_commodity_quantity()` and `commodity_quantity()` methods used by `PlannerOpKind::Loot` (lines 911-931) and `PlannerOpKind::Bribe` (lines 1926-1943).
4. This is a planner-driven ticket. The live `GoalKind` under test is `AcquireCommodity { commodity, purpose: SelfConsume }`. The operator surface is `PlannerOpKind::Harvest` within `ACQUIRE_OPS` (confirmed at `goal_dispatch_decl.rs:67-74`). Satisfaction check at `goal_model.rs:1147`: `state.commodity_quantity(actor, *commodity) > Quantity(0)`.
5. `payload_override` is available in `apply_planner_step()` as `Option<&ActionPayload>`. The Harvest payload is accessed via `payload_override.and_then(ActionPayload::as_harvest)`. Confirmed the pattern from `PlannerOpKind::QueueForFacilityUse` at line 943-951.

## Architecture Check

1. Follows the exact pattern used by `PlannerOpKind::Loot` and `PlannerOpKind::Bribe` — extract commodity/quantity from payload, call `state.with_commodity_quantity()`. No new abstractions, no new types.
2. No backward-compatibility shims. The no-op arm is replaced with an effect-modeling arm. Old behavior (silently ignoring harvest effects) is removed, not wrapped.

## Verification Layers

1. Planner predicts commodity gain after Harvest step → focused unit test: `apply_planner_step()` with `PlannerOpKind::Harvest` and `HarvestActionPayload` returns state where `commodity_quantity(actor, commodity) > 0`
2. `AcquireCommodity` goal satisfaction after Harvest → focused unit test: `is_satisfied()` returns `true` on state after `apply_planner_step(Harvest)`
3. Single-layer ticket: planner hypothetical state only; no authoritative action changes

## What to Change

### 1. Add Harvest match arm in `apply_planner_step()`

In `crates/worldwake-ai/src/goal_model.rs`, remove `PlannerOpKind::Harvest` from the no-op identity arm (line 1037) and add a new match arm before the remaining no-op block:

```rust
PlannerOpKind::Harvest => {
    if let Some(harvest) = payload_override.and_then(ActionPayload::as_harvest) {
        let actor = state.snapshot().actor();
        let current = state.commodity_quantity(actor, harvest.output_commodity);
        state.with_commodity_quantity(
            actor,
            harvest.output_commodity,
            Quantity(current.0.saturating_add(harvest.output_quantity.0)),
        )
    } else {
        state
    }
},
```

This models "agent gains `output_quantity` units of `output_commodity`" in the hypothetical planning state, enabling the planner to chain harvest → consume.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — add Harvest match arm in `apply_planner_step()`, remove from no-op arm)

## Out of Scope

- Scenario spawning of `KnownRecipes` (ticket S79RESSOUCON-001)
- Golden E2E tests for harvest-to-consume chain (ticket S79RESSOUCON-003)
- Craft effect modeling (Craft is also a no-op in `apply_planner_step` but is not in scope for this spec)
- Changes to eat/drink or harvest action semantics
- Budget adjustments to `CognitiveProfile`

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `apply_planner_step()` with `PlannerOpKind::Harvest` and a `HarvestActionPayload { output_commodity: Apple, output_quantity: Quantity(3) }` returns a state where `commodity_quantity(actor, Apple) == Quantity(3)`
2. Unit test: `apply_planner_step()` with `PlannerOpKind::Harvest` and no payload returns unchanged state (graceful fallback)
3. Unit test: `AcquireCommodity { commodity: Apple, purpose: SelfConsume }.is_satisfied()` returns `true` on state after Harvest step
4. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Harvest effect is hypothetical only — does not change authoritative world state
2. `saturating_add` prevents overflow in hypothetical commodity quantities
3. No payload → no effect (defensive fallback, same as other payload-dependent arms)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` (test module) — test `apply_planner_step` with Harvest payload updates commodity quantity
2. `crates/worldwake-ai/src/goal_model.rs` (test module) — test `apply_planner_step` with Harvest and no payload is identity
3. `crates/worldwake-ai/src/goal_model.rs` (test module) — test `AcquireCommodity` satisfaction after hypothetical Harvest

### Commands

1. `cargo test -p worldwake-ai -- apply_planner_step`
2. `cargo test -p worldwake-ai`
3. `cargo clippy --workspace --all-targets -- -D warnings`
