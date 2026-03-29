# S35OBSACTSIG-008: Save/load round-trip for `BelievedActivity`

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None — test only
**Deps**: S35OBSACTSIG-001 (BelievedActivity type on BelievedEntityState)

## Problem

`BelievedActivity` is a new `Option` field on `BelievedEntityState` that participates in save/load via `AgentBeliefStore` serialization. We need to verify that save/load round-trips preserve `BelievedActivity` data, and that loading old saves (without the field) correctly defaults to `None`.

## Assumption Reassessment (2026-03-29)

1. `save()` / `load()` in `crates/worldwake-sim/src/save_load.rs` use bincode serialization of `SimulationState`, which includes `World` and all component tables.
2. `AgentBeliefStore` is serialized as part of agent state. `BelievedEntityState` within it is serialized per-entity.
3. After S35OBSACTSIG-001, `BelievedEntityState.believed_activity` will be `Option<BelievedActivity>` with `#[serde(default)]`.
4. Existing save/load tests verify round-trip correctness for the current field set.
5. `BelievedActivity` contains `ActionDomain` (enum), `Option<EntityId>`, and `Tick` — all already serializable.
6. No `SAVE_FORMAT_VERSION` bump is needed per spec since the new field uses `#[serde(default)]`.

## Architecture Check

1. Testing save/load round-trip for new belief fields is standard practice — ensures P11 (representation boundaries may change encoding, never world meaning).
2. `#[serde(default)]` is the correct approach for backward-compatible deserialization — no migration code needed.
3. No alternatives considered — this is a straightforward verification task.

## Verification Layers

1. Round-trip with `believed_activity: Some(...)` preserves data -> focused test
2. Round-trip with `believed_activity: None` preserves None -> focused test
3. Deserialization of old-format data (without field) yields None -> focused test (simulate by serializing without the field, or by verifying `#[serde(default)]` behavior)
4. Single-layer ticket: serialization boundary only

## What to Change

### 1. Add save/load round-trip test

In the appropriate test module (likely `crates/worldwake-sim/tests/` or within `save_load.rs` tests), add a test that:
- Creates a `SimulationState` with an agent whose belief store contains a `BelievedEntityState` with `believed_activity: Some(BelievedActivity { ... })`.
- Calls `save()` to serialize.
- Calls `load()` to deserialize.
- Asserts that the loaded `believed_activity` matches the original.

### 2. Add backward-compatibility test

Verify that `BelievedEntityState` without `believed_activity` in serialized data (or with the field missing) deserializes to `believed_activity: None`. This can be done by constructing a `BelievedEntityState` with `None` and verifying round-trip, or by testing serde default behavior directly.

## Files to Touch

- `crates/worldwake-sim/tests/save_load_tests.rs` or equivalent (modify — add round-trip tests)

## Out of Scope

- `BelievedActivity` type definition (S35OBSACTSIG-001, prerequisite)
- Any production code changes
- `SAVE_FORMAT_VERSION` changes
- Migration logic for old saves

## Acceptance Criteria

### Tests That Must Pass

1. Save/load round-trip preserves `BelievedActivity` with all fields (domain, target, tick).
2. Save/load round-trip preserves `believed_activity: None`.
3. Old-format deserialization (field absent) yields `believed_activity: None`.
4. Existing suite: `cargo test --workspace`

### Invariants

1. `BelievedActivity` survives save/load without data loss (P11).
2. Backward compatibility: old saves load without error.
3. No `SAVE_FORMAT_VERSION` bump required.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-sim/tests/save_load_tests.rs` — round-trip tests for `BelievedActivity` presence and absence.

### Commands

1. `cargo test -p worldwake-sim -- save_load`
2. `cargo test --workspace`
