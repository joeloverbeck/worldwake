# E20COMBEH-001: Core data model additions for travel physiology

**Status**: ✅ COMPLETED
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-core (MetabolismProfile, BodyCostPerTick, EventTag)
**Deps**: E09 (needs & metabolism), E20 spec (`specs/E20-companion-behaviors.md`)

## Problem

The travel action currently has zero physiological cost (`BodyCostPerTick::zero()`). To support travel exertion and wilderness relief, the core data model needs: (a) travel exertion multipliers on `MetabolismProfile`, (b) a wilderness relief dirtiness penalty on `MetabolismProfile`, (c) a `bladder_delta` field on `BodyCostPerTick` (currently missing — only hunger, thirst, fatigue, dirtiness are tracked), and (d) new `EventTag` variants for wilderness relief and bladder accident categorization.

## Assumption Reassessment (2026-03-30)

1. **MetabolismProfile** (`crates/worldwake-core/src/needs.rs:71-139`): Currently has 12 fields (hunger_rate, thirst_rate, fatigue_rate, bladder_rate, dirtiness_rate, rest_efficiency, and 6 tolerance/tick fields). No travel multipliers or wilderness penalty exist. Confirmed via `Read`.
2. **BodyCostPerTick** (`crates/worldwake-core/src/needs.rs:144-177`): Has 4 fields: `hunger_delta`, `thirst_delta`, `fatigue_delta`, `dirtiness_delta`. No `bladder_delta`. The spec requires travel to increase bladder, so this field must be added. `apply_action_body_cost` in `crates/worldwake-systems/src/needs.rs:172` passes `needs.bladder` through unchanged — must be updated in a later ticket (E20COMBEH-002 or E20COMBEH-003) once this field exists.
3. **EventTag** (`crates/worldwake-core/src/event_tag.rs:7-30`): 22 variants. No `WildernessRelief` or `BladderAccident`. Adding them is additive (no existing variant changes).
4. **Serialization**: `MetabolismProfile`, `BodyCostPerTick`, and `EventTag` all derive `Serialize`/`Deserialize`. New `Permille` fields are already serializable. Adding fields to `MetabolismProfile` will break save/load for existing snapshots — acceptable since no stable save format is guaranteed yet.
5. **Default backward compatibility**: `MetabolismProfile::default()` must produce `Permille(0)` for all new travel multipliers and a sensible default for `wilderness_relief_dirtiness_penalty`. The spec says `Permille(0)` for multipliers. For the dirtiness penalty, a non-zero default is reasonable (e.g., `Permille(100)`) but `Permille(0)` also works for backward compat — implementation should use `Permille(0)` default so existing tests pass, with golden tests setting explicit values.

## Architecture Check

1. All additions are additive fields on existing components or new enum variants. No structural changes, no new component types, no new relation types. This is the cleanest path — alternatives like creating separate "TravelExertionProfile" components would fragment agent configuration unnecessarily.
2. No backward-compatibility shims. New fields default to zero, which produces identical behavior to pre-change code.

## Verification Layers

1. MetabolismProfile default backward compat → focused unit test (all new fields are Permille(0))
2. BodyCostPerTick::zero() includes bladder_delta → focused unit test
3. EventTag serialization round-trip → existing serde tests (if any) or new unit test
4. Single-layer ticket (core data model only); no cross-system verification needed.

## What to Change

### 1. Add travel exertion multipliers and dirtiness penalty to `MetabolismProfile`

In `crates/worldwake-core/src/needs.rs`, add four new `Permille` fields to `MetabolismProfile`:

```rust
pub travel_fatigue_multiplier: Permille,
pub travel_thirst_multiplier: Permille,
pub travel_bladder_multiplier: Permille,
pub wilderness_relief_dirtiness_penalty: Permille,
```

Update `MetabolismProfile::new()` to accept these four additional parameters. Update `MetabolismProfile::default()` to pass `Permille(0)` for all four.

### 2. Add `bladder_delta` to `BodyCostPerTick`

In `crates/worldwake-core/src/needs.rs`, add `pub bladder_delta: Permille` to `BodyCostPerTick`. Update `BodyCostPerTick::new()` to accept 5 parameters. Update `BodyCostPerTick::zero()` to include `bladder_delta: pm(0)`. Update `Default` impl.

### 3. Add `EventTag` variants

In `crates/worldwake-core/src/event_tag.rs`, add:

```rust
WildernessRelief,
BladderAccident,
```

### 4. Fix all call sites

Every call to `MetabolismProfile::new()` and `BodyCostPerTick::new()` gains additional arguments. Grep for all call sites and update them. Most will pass `Permille(0)` or `pm(0)`.

## Files to Touch

- `crates/worldwake-core/src/needs.rs` (modify — MetabolismProfile, BodyCostPerTick)
- `crates/worldwake-core/src/event_tag.rs` (modify — EventTag enum)
- All files calling `MetabolismProfile::new()` (modify — add new args)
- All files calling `BodyCostPerTick::new()` (modify — add bladder_delta arg)

## Out of Scope

- Wiring `bladder_delta` into `apply_action_body_cost` in `crates/worldwake-systems/src/needs.rs` (that's E20COMBEH-002)
- Dynamic body cost resolution on ActionInstance (E20COMBEH-002)
- Travel action handler changes (E20COMBEH-003)
- Wilderness relief action definition/handler (E20COMBEH-004)
- Planner changes (E20COMBEH-005)
- Golden tests (E20COMBEH-006 through E20COMBEH-008)
- Any changes to `combine_body_costs` in `crates/worldwake-systems/src/needs.rs`

## Acceptance Criteria

### Tests That Must Pass

1. `MetabolismProfile::default()` has all four new fields at `Permille(0)`
2. `BodyCostPerTick::zero()` has `bladder_delta` at `Permille(0)`
3. `BodyCostPerTick::new(pm(1), pm(2), pm(3), pm(4), pm(5))` stores all five fields correctly
4. Existing suite: `cargo test -p worldwake-core`
5. Existing suite: `cargo test --workspace` (all call-site updates compile and pass)

### Invariants

1. `MetabolismProfile::default()` produces identical behavior to pre-change code (all new multipliers are zero)
2. `BodyCostPerTick::zero()` remains all-zero (no accidental non-zero bladder_delta)
3. No existing test behavior changes — only new fields with zero defaults

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/needs.rs` — `metabolism_profile_default_travel_multipliers_zero` — confirms backward-compatible defaults
2. `crates/worldwake-core/src/needs.rs` — `body_cost_per_tick_new_stores_every_field` — update existing test to include bladder_delta (5th field)
3. `crates/worldwake-core/src/needs.rs` — `body_cost_per_tick_zero_is_all_zero` — update existing test to assert bladder_delta is zero

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test --workspace`
3. `cargo clippy --workspace`

## Outcome

- **Completion date**: 2026-03-30
- **What changed**: Added 4 `Permille` fields to `MetabolismProfile` (travel_fatigue_multiplier, travel_thirst_multiplier, travel_bladder_multiplier, wilderness_relief_dirtiness_penalty), added `bladder_delta: Permille` to `BodyCostPerTick`, added `WildernessRelief` and `BladderAccident` variants to `EventTag`. Updated ~48 call sites across all 5 crates. Added `metabolism_profile_default_travel_multipliers_zero` test.
- **Deviations**: None. Implementation matched the ticket exactly.
- **Verification**: `cargo test --workspace` (all pass, 0 failures), `cargo clippy --workspace` (clean).
