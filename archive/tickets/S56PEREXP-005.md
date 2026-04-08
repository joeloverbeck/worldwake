# S56PEREXP-005: Scenario integration for `PlaceVisibilityProfile`

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: Yes — scenario loading, PlaceDef
**Deps**: S56PEREXP-003

## Problem

After S56PEREXP-003 registers `PlaceVisibilityProfile` as a component, scenario authors need a way to set it on places via RON scenario files. Without this, all places default to zero concealment and the modulation system has no environmental input.

## Assumption Reassessment (2026-04-06)

1. `PlaceDef` at `crates/worldwake-cli/src/scenario/types.rs:36-42` has `name: String` and `tags: Vec<PlaceTag>`. Both fields are scenario-definable.
2. `build_topology()` at `crates/worldwake-cli/src/scenario/mod.rs:130-191` creates places and inserts them into `Topology`. Place entity IDs are assigned manually from slot indices (line 139), not via `txn.create_entity()`.
3. Components can be set on place entity IDs via `WorldTxn` — confirmed by `set_component_bandit_camp` usage in tests.
4. `PlaceDef` derives `Clone, Debug, Deserialize` — the `Option<PlaceVisibilityProfile>` field needs `PlaceVisibilityProfile` to also derive `Deserialize` (already required for component registration).
5. The `build_topology()` function receives `&mut Topology` but does NOT currently have a `WorldTxn` — components like `BanditCamp` on places are set elsewhere. Need to verify where place component assignment happens and follow the same pattern.
6. Reassessment correction: `spawn_entities()` in `crates/worldwake-cli/src/scenario/mod.rs` is the live place-component assignment boundary because it owns the bootstrap `WorldTxn` after `World::new(topology)`. `build_topology()` cannot set `PlaceVisibilityProfile` directly because it only builds `Topology`.
7. `PlaceVisibilityProfile` already derives `Serialize`/`Deserialize` in `crates/worldwake-core/src/observation_context.rs`; no core-type change is needed for RON parsing.
8. `worldwake-cli` already depends on `worldwake-core` in `crates/worldwake-cli/Cargo.toml`; no manifest change is needed.
9. Adding `visibility_profile` to `PlaceDef` is a shared CLI scenario-shape change, so all manual `PlaceDef { ... }` literals in CLI tests/helpers need explicit `visibility_profile: None` updates to keep the schema honest after the new field lands.

## Architecture Check

1. Optional field on `PlaceDef` with `#[serde(default)]` — places without `visibility_profile` in RON get `None`, meaning zero concealment. Clean degradation.
2. Follows the `AgentDef` pattern for optional profile fields. No backwards-compatibility shims.

## Verification Layers

1. PlaceDef deserialization with and without visibility_profile -> RON parsing test
2. Place component set when profile is present -> integration test
3. Place component absent when profile is None -> default behavior unchanged
4. Single-layer ticket (scenario loading) — no decision/action trace needed.

## What to Change

### 1. Add optional field to `PlaceDef`

In `crates/worldwake-cli/src/scenario/types.rs`:

```rust
pub struct PlaceDef {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<PlaceTag>,
    #[serde(default)]
    pub visibility_profile: Option<PlaceVisibilityProfile>,
}
```

Import `PlaceVisibilityProfile` from `worldwake_core`.

### 2. Wire component assignment in scenario loading

Add a place-component pass inside `spawn_entities()` using the bootstrap `WorldTxn` and the already-registered place names/IDs:

```rust
for place_def in &def.places {
    let place_id = resolve_name(names, &place_def.name, "place visibility_profile")?;
    if let Some(vis) = &place_def.visibility_profile {
        txn.set_component_place_visibility_profile(place_id, vis.clone())?;
    }
}
```

### 3. Add import to `Cargo.toml` if needed

No change needed after reassessment: `crates/worldwake-cli/Cargo.toml` already depends on `worldwake-core`.

## Files to Touch

- `crates/worldwake-cli/src/scenario/types.rs` (modify)
- `crates/worldwake-cli/src/scenario/mod.rs` (modify)
- `crates/worldwake-cli/src/display.rs` (modify, `PlaceDef` test-helper fallout)
- `crates/worldwake-cli/src/handlers/actions.rs` (modify, `PlaceDef` test-helper fallout)
- `crates/worldwake-cli/src/handlers/control.rs` (modify, `PlaceDef` test-helper fallout)
- `crates/worldwake-cli/src/handlers/events.rs` (modify, `PlaceDef` test-helper fallout)
- `crates/worldwake-cli/src/handlers/inspect.rs` (modify, `PlaceDef` test-helper fallout)
- `crates/worldwake-cli/src/handlers/tick.rs` (modify, `PlaceDef` test-helper fallout)
- `crates/worldwake-cli/src/handlers/world_overview.rs` (modify, `PlaceDef` test-helper fallout)

## Out of Scope

- Updating existing scenario `.ron` files to include visibility profiles — that's part of golden test scenarios (S56PEREXP-006)
- Default visibility profiles for existing places

## Acceptance Criteria

### Tests That Must Pass

1. RON deserialization of `PlaceDef` with `visibility_profile: Some(PlaceVisibilityProfile { base_concealment: Permille(400) })` succeeds
2. RON deserialization of `PlaceDef` without `visibility_profile` succeeds (defaults to `None`)
3. Place entity has `PlaceVisibilityProfile` component after spawn when profile is in RON
4. Place entity does NOT have `PlaceVisibilityProfile` component when profile is absent from RON
5. Existing suite: `cargo test -p worldwake-cli`

### Invariants

1. Existing scenario files continue to parse without changes (serde default)
2. `PlaceVisibilityProfile` is only set on Place entities

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/scenario/types.rs` — RON parsing tests with and without `visibility_profile`
2. `crates/worldwake-cli/src/scenario/mod.rs` — scenario spawn test proving place component present/absent by definition

### Commands

1. `cargo test -p worldwake-cli -- scenario::types::tests::test_place_def_deserializes_visibility_profile`
2. `cargo test -p worldwake-cli -- scenario::tests::test_spawn_applies_place_visibility_profile_when_present`
3. `cargo test -p worldwake-cli`
4. `cargo clippy --workspace --all-targets -- -D warnings`

## Outcome

Completed on 2026-04-06.

- Added optional `visibility_profile: Option<PlaceVisibilityProfile>` to `PlaceDef` in `crates/worldwake-cli/src/scenario/types.rs`.
- Wired bootstrap scenario spawning in `crates/worldwake-cli/src/scenario/mod.rs` to set `PlaceVisibilityProfile` on place entity IDs through the existing `WorldTxn` path after topology construction.
- Added focused RON parsing tests and scenario spawn tests proving profile present/absent behavior.
- Updated CLI test/helper `PlaceDef` literals to set `visibility_profile: None` explicitly so the new scenario schema is reflected across the current crate.

## Verification Result

- Passed `cargo test -p worldwake-cli -- scenario::types::tests::test_place_def_deserializes_visibility_profile`
- Passed `cargo test -p worldwake-cli -- scenario::tests::test_spawn_applies_place_visibility_profile_when_present`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
