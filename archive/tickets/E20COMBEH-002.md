# E20COMBEH-002: Instance-level body cost override and bladder cost wiring

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — worldwake-sim (ActionInstance), worldwake-systems (needs system)
**Deps**: E20COMBEH-001 (bladder_delta on BodyCostPerTick)

## Problem

Body costs are currently read from the static `ActionDef.body_cost_per_tick` field in `aggregate_body_costs()` (`crates/worldwake-systems/src/needs.rs:129`). Travel physiology requires per-agent dynamic body costs resolved from `MetabolismProfile` at action start. Additionally, `apply_action_body_cost()` (`crates/worldwake-systems/src/needs.rs:172`) does not apply `bladder_delta` — it passes `needs.bladder` through unchanged. Both must be fixed for travel exertion to work.

## Assumption Reassessment (2026-03-30)

1. **`aggregate_body_costs`** (`crates/worldwake-systems/src/needs.rs:129-148`): Reads `def.body_cost_per_tick` from `ActionDefRegistry` for each active action. No instance-level override exists. Confirmed via `Read`.
2. **`apply_action_body_cost`** (`crates/worldwake-systems/src/needs.rs:172-180`): Applies hunger, thirst, fatigue, dirtiness deltas but NOT bladder. Line 177: `needs.bladder` is passed through unchanged. This must be updated to also apply `cost.bladder_delta` (added in E20COMBEH-001).
3. **`combine_body_costs`** (`crates/worldwake-systems/src/needs.rs:150-156`): Saturating-adds all four fields. Must add `bladder_delta` combination after E20COMBEH-001 adds the field.
4. **`ActionInstance`** (`crates/worldwake-sim/src/action_instance.rs:5-17`): Has 10 fields. No body cost override field. Adding `pub body_cost_override: Option<BodyCostPerTick>` is the simplest mechanism — `aggregate_body_costs` prefers the override when `Some`, falls back to def when `None`.
5. **Cross-system boundary**: This ticket modifies `worldwake-sim` (ActionInstance struct) and `worldwake-systems` (needs system). The shared contract is `ActionInstance.body_cost_override` read by the needs system. No direct cross-system calls.
6. **Existing test**: `needs_system_applies_active_action_body_costs` (`crates/worldwake-systems/src/needs.rs:543`) tests body cost application via the def's static cost. This test must be updated to pass the new bladder_delta arg and verify bladder is affected.

## Architecture Check

1. `Option<BodyCostPerTick>` on `ActionInstance` is the minimal change. Alternative (a `BodyCostExpr` enum on `ActionDef` analogous to `DurationExpr`) is more general but adds complexity for a single use case. The override field is simpler, requires no new enum, and is future-extensible — any action that needs dynamic body costs can set the override in its start handler.
2. No backward-compatibility shims. Existing actions set `body_cost_override: None` and behavior is identical to pre-change code.

## Verification Layers

1. Bladder delta application → focused unit test (apply_action_body_cost with non-zero bladder_delta)
2. Instance override precedence → focused unit test (aggregate prefers override over def)
3. ~~Combine includes bladder~~ → already done by E20COMBEH-001
4. Existing body cost behavior unchanged → existing test updated + passes

## What to Change

### 1. Add `body_cost_override` to `ActionInstance`

In `crates/worldwake-sim/src/action_instance.rs`, add:

```rust
pub body_cost_override: Option<BodyCostPerTick>,
```

Default to `None` in all existing ActionInstance construction sites. This field is `Serialize`/`Deserialize` (both `Option` and `BodyCostPerTick` already derive these).

### 2. Update `aggregate_body_costs` to prefer instance override

In `crates/worldwake-systems/src/needs.rs`, modify `aggregate_body_costs` to use `action.body_cost_override.unwrap_or(def.body_cost_per_tick)` instead of `def.body_cost_per_tick`.

### 3. Wire `bladder_delta` into `apply_action_body_cost`

In `crates/worldwake-systems/src/needs.rs`, update `apply_action_body_cost` to apply `cost.bladder_delta`:

```rust
fn apply_action_body_cost(needs: HomeostaticNeeds, cost: BodyCostPerTick) -> HomeostaticNeeds {
    HomeostaticNeeds::new(
        needs.hunger.saturating_add(cost.hunger_delta),
        needs.thirst.saturating_add(cost.thirst_delta),
        needs.fatigue.saturating_add(cost.fatigue_delta),
        needs.bladder.saturating_add(cost.bladder_delta),
        needs.dirtiness.saturating_add(cost.dirtiness_delta),
    )
}
```

### 4. ~~Wire `bladder_delta` into `combine_body_costs`~~ (ALREADY DONE)

`combine_body_costs` already includes `bladder_delta` (line 155). Delivered by E20COMBEH-001. No change needed.

### 5. Fix all ActionInstance construction sites

Grep for `ActionInstance {` and add `body_cost_override: None` to every construction.

## Files to Touch

- `crates/worldwake-sim/src/action_instance.rs` (modify — new field)
- `crates/worldwake-systems/src/needs.rs` (modify — aggregate, apply, combine functions)
- All files constructing `ActionInstance` (modify — add `body_cost_override: None`)

## Out of Scope

- Travel handler changes (E20COMBEH-003 — that ticket sets the override)
- Wilderness relief action (E20COMBEH-004)
- Any changes to `MetabolismProfile` (done in E20COMBEH-001)
- Golden tests (E20COMBEH-006 through E20COMBEH-008)

## Acceptance Criteria

### Tests That Must Pass

1. `apply_action_body_cost` with non-zero `bladder_delta` increases `needs.bladder`
2. `aggregate_body_costs` uses `body_cost_override` when `Some`, falls back to def when `None`
3. `combine_body_costs` includes `bladder_delta`
4. Existing test `needs_system_applies_active_action_body_costs` updated and passes
5. Existing suite: `cargo test -p worldwake-systems`
6. Existing suite: `cargo test --workspace`

### Invariants

1. Actions with `body_cost_override: None` behave identically to pre-change code
2. Bladder is no longer silently excluded from action body costs
3. No existing test behavior changes (all existing actions use `BodyCostPerTick::zero()` or have zero bladder_delta)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs.rs` — `apply_action_body_cost_includes_bladder` — new test: non-zero bladder_delta increases bladder
2. `crates/worldwake-systems/src/needs.rs` — `aggregate_body_costs_prefers_instance_override` — new test: instance override takes precedence
3. `crates/worldwake-systems/src/needs.rs` — `needs_system_applies_active_action_body_costs` — update existing test to include bladder_delta

### Commands

1. `cargo test -p worldwake-systems`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-30
- **What changed**:
  - Added `body_cost_override: Option<BodyCostPerTick>` to `ActionInstance` (worldwake-sim) and set to `None` at all ~45 construction sites across 4 crates.
  - `aggregate_body_costs` now uses `action.body_cost_override.unwrap_or(def.body_cost_per_tick)`.
  - `apply_action_body_cost` now applies `cost.bladder_delta` instead of passing bladder through unchanged.
  - Added 3 new tests: `apply_action_body_cost_includes_bladder`, `aggregate_body_costs_prefers_instance_override`, `aggregate_body_costs_falls_back_to_def_when_no_override`.
  - Updated existing `needs_system_applies_active_action_body_costs` to use non-zero bladder values.
- **Deviations**: Deliverable #4 (`combine_body_costs` bladder wiring) was already done by E20COMBEH-001 — skipped as no-op.
- **Verification**: `cargo test --workspace` ✅, `cargo clippy --workspace` ✅
