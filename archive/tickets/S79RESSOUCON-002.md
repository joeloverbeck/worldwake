# S79RESSOUCON-002: Add harvest effect to planner hypothetical state

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — AI planner (`worldwake-ai`)
**Deps**: S79 spec

## Problem

The planner cannot chain harvest → consume because `PlannerOpKind::Harvest` is a no-op in `apply_planner_step()`. When the planner hypothetically expands a Harvest step, it does not update the actor's commodity quantity, so the `AcquireCommodity` satisfaction check (`commodity_quantity(actor, commodity) > Quantity(0)`) never succeeds after a Harvest step. The planner therefore cannot predict that harvesting will yield the commodity needed for subsequent eat/drink actions.

## Assumption Reassessment (2026-04-09)

1. `PlannerOpKind::Harvest` falls through to the identity arm (`=> state`) in `apply_planner_step()` at `crates/worldwake-ai/src/goal_model.rs:1034-1056`. Confirmed via read at lines 1034-1056.
2. `HarvestActionPayload` at `crates/worldwake-sim/src/action_payload.rs:321-327` has fields `output_commodity: CommodityKind` and `output_quantity: Quantity`. Accessor `as_harvest()` exists at line 91.
3. Initial live boundary was `apply_planner_step()` in `goal_model.rs`, and `PlanningState` already had `with_commodity_quantity()` / `commodity_quantity()` used by `PlannerOpKind::Loot` and `PlannerOpKind::Bribe`.
4. This is a planner-driven ticket. The live `GoalKind` under test is `AcquireCommodity { commodity, purpose: SelfConsume }`. The operator surface is `PlannerOpKind::Harvest` within `ACQUIRE_OPS` (confirmed at `goal_dispatch_decl.rs:67-74`). Satisfaction check at `goal_model.rs:1147`: `state.commodity_quantity(actor, *commodity) > Quantity(0)`.
5. `payload_override` is available in `apply_planner_step()` as `Option<&ActionPayload>`. The Harvest payload is accessed via `payload_override.and_then(ActionPayload::as_harvest)`. Confirmed the pattern from `PlannerOpKind::QueueForFacilityUse`.
6. Reassessment after implementation attempt: adding the harvest effect alone caused six `golden_production` failures around exclusive-facility queue materialization. Live search still admitted direct `Harvest` candidates at contention-managed facilities for `AcquireCommodity(SelfConsume)` even when the actor lacked a lawful grant, so the true ticket boundary included contention-aware candidate filtering in `crates/worldwake-ai/src/search/candidates.rs`.

## Architecture Check

1. `PlannerOpKind::Harvest` now follows the same hypothetical-state pattern used by `PlannerOpKind::Loot` and `PlannerOpKind::Bribe` — extract commodity/quantity from payload, call `state.with_commodity_quantity()`.
2. The lawful boundary is wider than `goal_model.rs` alone. Exclusive-facility harvests must remain coupled to the queue/grant contention contract, so direct `Harvest` / `Craft` affordance candidates are now filtered in search unless contention status is `Unmanaged` or `Granted`.
3. No backward-compatibility shims. The old no-op harvest behavior and the planner-side contention bypass are both removed rather than wrapped.

## Verification Layers

1. Planner predicts commodity gain after Harvest step → focused unit test: `apply_planner_step()` with `PlannerOpKind::Harvest` and `HarvestActionPayload` returns state where `commodity_quantity(actor, commodity) > 0`
2. `AcquireCommodity` goal satisfaction after Harvest → focused unit test: `is_satisfied()` returns `true` on state after `apply_planner_step(Harvest)`
3. Planner/search alignment under contention → focused search tests prove `AcquireCommodity(SelfConsume)` still queues before harvest at exclusive facilities without a grant and still skips the queue when a matching grant already exists
4. No authoritative action changes

## What Changed

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

### 2. Filter direct harvest/craft search candidates by contention status

In `crates/worldwake-ai/src/search/candidates.rs`, direct `Harvest` / `Craft` affordances are now rejected when the affordance is contention-managed but not currently `Granted`. This preserves the live queue/grant contract for exclusive facilities while still allowing direct harvest when the actor already holds the lawful grant.

## Files to Touch

- `crates/worldwake-ai/src/goal_model.rs` (modify — add Harvest match arm in `apply_planner_step()`, remove from no-op arm)
- `crates/worldwake-ai/src/search/candidates.rs` (modify — prevent direct harvest/craft planning from bypassing exclusive-facility contention)
- `crates/worldwake-ai/src/search/tests.rs` (modify — add `AcquireCommodity(SelfConsume)` exclusive-orchard queue/grant coverage)

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
4. Focused search test: `AcquireCommodity(SelfConsume)` at an exclusive orchard without a grant yields `QueueForFacilityUse`
5. Focused search test: `AcquireCommodity(SelfConsume)` with a matching grant yields direct `Harvest`
6. Existing suite: `cargo test -p worldwake-ai`

### Invariants

1. Harvest effect is hypothetical only — does not change authoritative world state
2. `saturating_add` prevents overflow in hypothetical commodity quantities
3. No payload → no effect (defensive fallback, same as other payload-dependent arms)
4. Exclusive-facility contention remains state-mediated. Hypothetical harvest effects do not let the planner bypass queue/grant access rules

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/goal_model.rs` (test module) — test `apply_planner_step` with Harvest payload updates commodity quantity
2. `crates/worldwake-ai/src/goal_model.rs` (test module) — test `apply_planner_step` with Harvest and no payload is identity
3. `crates/worldwake-ai/src/goal_model.rs` (test module) — test `AcquireCommodity` satisfaction after hypothetical Harvest
4. `crates/worldwake-ai/src/search/tests.rs` — test `AcquireCommodity(SelfConsume)` queues before harvest at an exclusive facility without a grant
5. `crates/worldwake-ai/src/search/tests.rs` — test `AcquireCommodity(SelfConsume)` skips queue when a matching exclusive-facility grant is already active

### Commands

1. `cargo test -p worldwake-ai harvest_step_updates_hypothetical_commodity_quantity`
2. `cargo test -p worldwake-ai harvest_step_without_payload_is_identity`
3. `cargo test -p worldwake-ai acquire_self_consume_goal_is_satisfied_after_hypothetical_harvest`
4. `cargo test -p worldwake-ai search_acquire_self_consume_queues_before_harvest_at_exclusive_facility_without_grant`
5. `cargo test -p worldwake-ai search_acquire_self_consume_skips_queue_when_matching_grant_is_already_active`
6. `cargo test -p worldwake-ai`
7. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-09.

`AcquireCommodity(SelfConsume)` can now lawfully chain harvest into consume in planner hypothetical state, and exclusive-facility harvest planning still respects the live queue/grant contention contract instead of bypassing it.

## Verification Result

- `cargo test -p worldwake-ai harvest_step_updates_hypothetical_commodity_quantity`
- `cargo test -p worldwake-ai harvest_step_without_payload_is_identity`
- `cargo test -p worldwake-ai acquire_self_consume_goal_is_satisfied_after_hypothetical_harvest`
- `cargo test -p worldwake-ai search_acquire_self_consume_queues_before_harvest_at_exclusive_facility_without_grant`
- `cargo test -p worldwake-ai search_acquire_self_consume_skips_queue_when_matching_grant_is_already_active`
- `cargo test -p worldwake-ai`
- `cargo clippy --workspace --all-targets -- -D warnings`
