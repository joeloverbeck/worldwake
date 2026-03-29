# S35OBSACTSIG-001: Add `BelievedActivity` struct and extend `BelievedEntityState`

**Status**: PENDING
**Priority**: HIGH
**Effort**: Small
**Engine Changes**: Yes — worldwake-core belief types
**Deps**: E14 (perception & belief system, already complete)

## Problem

Agents cannot observe what other co-located agents are doing. The first step is adding the data type that represents an observed activity and extending the belief state to carry it.

## Assumption Reassessment (2026-03-29)

1. `BelievedEntityState` is defined at `crates/worldwake-core/src/belief.rs:653` with fields: `last_known_place`, `last_known_inventory`, `workstation_tag`, `resource_source`, `alive`, `wounds`, `last_known_courage`, `observed_tick`, `source`. No `believed_activity` field exists.
2. `ActionDomain` is defined at `crates/worldwake-sim/src/action_domain.rs:4` with variants: Generic, Needs, Production, Trade, Social, Epistemic, Travel, Transport, Combat, Care, Corpse. It derives Serialize/Deserialize.
3. `BelievedEntityState` derives `Clone, Debug, Eq, PartialEq, Serialize, Deserialize`.
4. `EntityId`, `Tick` are in `worldwake-core::ids`.
5. This ticket introduces a new type in core. Since `ActionDomain` lives in `worldwake-sim`, but core cannot depend on sim, the `BelievedActivity` struct must either use a local representation or `ActionDomain` must be moved/re-exported. **Correction**: The spec says `BelievedActivity.action_domain: ActionDomain`. Since core cannot depend on sim, we need to either (a) move `ActionDomain` to core, or (b) define a `BelievedActionDomain` mirror in core. Option (a) is cleaner — `ActionDomain` is a pure enum with no sim dependencies. This ticket must also move `ActionDomain` to core (re-exporting from sim for backward compatibility is forbidden by P26).
6. No existing tests reference `BelievedActivity` or `believed_activity`.

## Architecture Check

1. Adding `BelievedActivity` as an `Option` field on `BelievedEntityState` is the minimal, clean extension. It follows P3 (concrete action domain + target, not abstract busyness score) and P12 (belief state, not authoritative state).
2. Moving `ActionDomain` to core is architecturally correct — it's a classification enum with no sim logic. Core already hosts `ActionDefId` (via IDs). No shims or aliases introduced.

## Verification Layers

1. `BelievedActivity` construction and field access -> focused unit test
2. `BelievedEntityState.believed_activity` set/get -> focused unit test
3. `ActionDomain` available from core -> compilation of core crate
4. Single-layer ticket: types only, no cross-system interaction

## What to Change

### 1. Move `ActionDomain` enum to worldwake-core

Move `ActionDomain` from `crates/worldwake-sim/src/action_domain.rs` to a new `crates/worldwake-core/src/action_domain.rs`. Update all imports across workspace crates to use `worldwake_core::ActionDomain`. Remove the old file from sim and update sim's `lib.rs` to re-export from core during the transition — actually, per P26 (no backward compatibility layers), update all sim consumers directly.

### 2. Add `BelievedActivity` struct to `crates/worldwake-core/src/belief.rs`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BelievedActivity {
    pub action_domain: ActionDomain,
    pub target: Option<EntityId>,
    pub observed_tick: Tick,
}
```

### 3. Extend `BelievedEntityState`

Add `pub believed_activity: Option<BelievedActivity>` field. Use `#[serde(default)]` for backward-compatible deserialization of old saves.

### 4. Update all `BelievedEntityState` constructors

Every place that constructs `BelievedEntityState` must now include `believed_activity: None`.

## Files to Touch

- `crates/worldwake-core/src/action_domain.rs` (new — moved from sim)
- `crates/worldwake-core/src/lib.rs` (modify — register new module, export `ActionDomain`)
- `crates/worldwake-core/src/belief.rs` (modify — add `BelievedActivity` struct, extend `BelievedEntityState`)
- `crates/worldwake-sim/src/action_domain.rs` (delete — moved to core)
- `crates/worldwake-sim/src/lib.rs` (modify — remove old module, import from core)
- All files importing `ActionDomain` from sim (modify — update imports)

## Out of Scope

- Perception system changes (S35OBSACTSIG-003)
- `GoalBeliefView` extensions (S35OBSACTSIG-004)
- Ranking discount logic (S35OBSACTSIG-006)
- `UtilityProfile` changes (S35OBSACTSIG-002)
- Save/load round-trip testing (S35OBSACTSIG-008)

## Acceptance Criteria

### Tests That Must Pass

1. Unit test: `BelievedActivity` can be constructed with all `ActionDomain` variants, `Some`/`None` target, and a tick value.
2. Unit test: `BelievedEntityState` with `believed_activity: Some(...)` and `believed_activity: None` both construct and compare correctly.
3. Existing suite: `cargo test --workspace`

### Invariants

1. `ActionDomain` is now authoritative in `worldwake-core` — no duplicate definition exists in sim.
2. All existing `BelievedEntityState` construction sites initialize `believed_activity: None`.
3. `BelievedEntityState` equality comparison includes the new field.
4. `#[serde(default)]` on `believed_activity` ensures old serialized data deserializes cleanly.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-core/src/belief.rs` (or `tests/` module) — unit tests for `BelievedActivity` construction and `BelievedEntityState` field inclusion.

### Commands

1. `cargo test -p worldwake-core`
2. `cargo test --workspace`
3. `cargo clippy --workspace`
