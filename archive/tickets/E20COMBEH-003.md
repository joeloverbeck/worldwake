# E20COMBEH-003: Travel body cost wiring via MetabolismProfile multipliers

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-systems (travel_actions.rs)
**Deps**: E20COMBEH-001 (MetabolismProfile multipliers), E20COMBEH-002 (body_cost_override on ActionInstance)

## Problem

The travel action currently uses `BodyCostPerTick::zero()` (`crates/worldwake-systems/src/travel_actions.rs` action def registration). With E20COMBEH-001 adding travel multipliers to `MetabolismProfile` and E20COMBEH-002 adding `body_cost_override` to `ActionInstance`, the `start_travel` handler must resolve per-agent body costs from the actor's profile and store them in the instance override.

## Assumption Reassessment (2026-03-30)

1. **Travel action def** (`crates/worldwake-systems/src/travel_actions.rs:12-56`): Registered with `body_cost_per_tick: BodyCostPerTick::zero()`. This static zero remains correct as the fallback — per-agent costs come from the instance override.
2. **`start_travel`** (`crates/worldwake-systems/src/travel_actions.rs:104-151`): Resolves travel edge, marks in-transit. Has access to `WorldTxn` which can read `MetabolismProfile`. Must compute `BodyCostPerTick` from `MetabolismProfile` travel multipliers × basal rates and set `body_cost_override` on the `ActionInstance`.
3. **MetabolismProfile access**: `WorldTxn` provides `get_component_metabolism_profile(entity)`. The actor's profile is available during `start_travel`.
4. **Body cost formula** (from spec): `additional_cost = basal_rate * travel_multiplier / 1000`. For each need:
   - `fatigue_delta = profile.fatigue_rate * profile.travel_fatigue_multiplier / 1000`
   - `thirst_delta = profile.thirst_rate * profile.travel_thirst_multiplier / 1000`
   - `bladder_delta = profile.bladder_rate * profile.travel_bladder_multiplier / 1000`
   - `hunger_delta = Permille(0)` (no travel hunger multiplier in spec)
   - `dirtiness_delta = Permille(0)` (no travel dirtiness multiplier in spec)
5. **ActionInstance mutation**: The start handler currently does not return a modified ActionInstance — it mutates world state via `WorldTxn`. The body cost override must be set on the `ActionInstance` passed to or returned from the start handler. Check whether start handlers can mutate the instance or if the framework needs adjustment (may need to return the override or have the action execution framework set it).

## Architecture Check

1. Resolving body costs in `start_travel` from the actor's `MetabolismProfile` is the natural place — the handler already has `WorldTxn` access and actor context. The cost is resolved once at start, stored in the instance, and remains fixed for the action's duration. This is cleaner than per-tick re-reads (which would be wasteful and break the spec's "fixed for duration" constraint).
2. No backward-compatibility shims. Agents with `Permille(0)` multipliers (the default) produce `BodyCostPerTick::zero()` override, identical to pre-change behavior.

## Verification Layers

1. Travel body cost resolved from MetabolismProfile → focused unit test (start_travel sets override)
2. Correct formula application → focused unit test (basal_rate × multiplier / 1000)
3. Zero multipliers produce zero override → focused unit test (backward compat)
4. Single-layer ticket: worldwake-systems travel handler only; no cross-system verification needed beyond build.

## What to Change

### 1. Compute travel body cost in `start_travel`

In `crates/worldwake-systems/src/travel_actions.rs`, modify `start_travel` to:

1. Read the actor's `MetabolismProfile` from `WorldTxn`.
2. Compute the `BodyCostPerTick`:
   ```rust
   let fatigue = Permille(profile.fatigue_rate.0 * profile.travel_fatigue_multiplier.0 / 1000);
   let thirst = Permille(profile.thirst_rate.0 * profile.travel_thirst_multiplier.0 / 1000);
   let bladder = Permille(profile.bladder_rate.0 * profile.travel_bladder_multiplier.0 / 1000);
   let cost = BodyCostPerTick::new(pm(0), thirst, fatigue, bladder, pm(0));
   ```
3. Set `instance.body_cost_override = Some(cost)`.

### 2. Ensure start handler can set body_cost_override

If the current `StartHandler` signature does not allow mutating the `ActionInstance`, the framework must be adjusted. Check `action_handler.rs` — if start handlers receive `&mut ActionInstance`, this is straightforward. If not, the start handler may need to return the override value, and `action_execution.rs` sets it.

## Files to Touch

- `crates/worldwake-systems/src/travel_actions.rs` (modify — start_travel computes and sets body cost)
- `crates/worldwake-sim/src/action_execution.rs` (modify — if start handler API needs adjustment for override)
- `crates/worldwake-sim/src/action_handler.rs` (modify — if start handler signature needs adjustment)

## Out of Scope

- MetabolismProfile field additions (E20COMBEH-001)
- ActionInstance.body_cost_override field addition (E20COMBEH-002)
- Needs system changes (E20COMBEH-002)
- Wilderness relief action (E20COMBEH-004)
- Planner changes (E20COMBEH-005)
- Golden tests (E20COMBEH-006 through E20COMBEH-008)
- Travel duration changes (not in E20 scope)
- Hunger or dirtiness multipliers for travel (not in E20 spec)

## Acceptance Criteria

### Tests That Must Pass

1. `start_travel` with non-zero MetabolismProfile multipliers produces correct `body_cost_override`
2. `start_travel` with default (zero) multipliers produces `BodyCostPerTick::zero()` override
3. Body cost formula: `basal_rate * multiplier / 1000` for each need
4. Existing suite: `cargo test -p worldwake-systems`
5. Existing suite: `cargo test --workspace`

### Invariants

1. Agents with default MetabolismProfile experience zero travel body cost (backward compatible)
2. Body cost is resolved once at action start, not re-read per tick
3. No travel body cost applied to non-travel actions

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/travel_actions.rs` — `start_travel_sets_body_cost_from_metabolism_profile` — non-zero multipliers produce correct override
2. `crates/worldwake-systems/src/travel_actions.rs` — `start_travel_zero_multipliers_produce_zero_cost` — backward compatibility

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-30
- **What changed**:
  - `ActionStartFn` signature changed from `&ActionInstance` to `&mut ActionInstance` across ~30 handler functions in 20+ files (framework adjustment enabling start handlers to set instance fields)
  - `start_travel` in `travel_actions.rs` now reads the actor's `MetabolismProfile`, computes `BodyCostPerTick` via `basal_rate * travel_multiplier / 1000` for fatigue/thirst/bladder, and sets `instance.body_cost_override`
  - Two focused tests added: `start_travel_sets_body_cost_from_metabolism_profile`, `start_travel_zero_multipliers_produce_zero_cost`
- **Deviations**: None. The ticket anticipated the framework adjustment need; `&mut ActionInstance` was chosen over a callback-based approach for simplicity.
- **Verification**: `cargo test --workspace` all pass, `cargo clippy --workspace` clean.
